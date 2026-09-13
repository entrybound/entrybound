#!/usr/bin/env python3
"""Per-item content statistics for the tuning and validation splits (format ebrc-stats-v1).

Held-out items are refused unless a two-key unlock is present (see corpuslib.check_heldout_unlock):
  --unlock-heldout <design-freeze-commit>  AND  a committed research/corpus/heldout-unlock.json
naming the same commit, which must be an ancestor of HEAD, with heldout-lock.json frozen.

For each item the tree is re-read once; while reading, the fingerprint is recomputed
and must equal the recorded logical_tree_sha256 (fails loudly otherwise).

Statistics (all deterministic; no timings are collected):
  content types   libmagic via `file --mime-type -b -N` (unique inodes; by count and bytes)
  extensions      lowercased final suffix of the basename (by count and bytes)
  duplicates      exact-duplicate files by SHA-256 (non-empty unique inodes) and 4 KiB
                  fixed-block duplicates (per-file offsets 0,4096,...; tail blocks included,
                  keyed by BLAKE2b-128 + length)
  entropy         order-0 Shannon entropy of the whole byte stream, mean over full 4 KiB
                  blocks, byte-weighted mean per-file entropy, and histograms
  compressibility single-shot `zstd -3 -T1` and `xz -6 -T1` over the concatenated file
                  contents in path order (unique inodes); streams larger than
                  --compress-max-bytes are sampled with evenly spaced 1 MiB windows.
                  Indicators only, not benchmarks.
  metadata        symlinks, hardlinks, xattrs, POSIX ACLs, sparse files, modes, owners,
                  timestamps, name encodings and portability hazards
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import subprocess
import sys
import threading
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import corpuslib as cl  # noqa: E402
import fingerprint as fpm  # noqa: E402

try:
    import numpy as np
except ImportError:  # pragma: no cover
    np = None

FORMAT = "ebrc-stats-v1"
BS = 4096
SEG = 1 << 20
SAFE_ENV = {"PATH": "/usr/local/bin:/usr/bin:/bin", "LC_ALL": "C.UTF-8", "LANG": "C.UTF-8", "TZ": "UTC"}
WIN_RESERVED = {"con", "prn", "aux", "nul", *(f"com{i}" for i in range(1, 10)), *(f"lpt{i}" for i in range(1, 10))}
WIN_BAD_CHARS = set(b'<>:"\\|?*')


def r6(x) -> float:
    return round(float(x), 6)


def entropy_of_counts(counts) -> float:
    total = counts.sum()
    if total == 0:
        return 0.0
    p = counts[counts > 0].astype(np.float64) / float(total)
    return float(-(p * np.log2(p)).sum())


# ---------------------------------------------------------------------------
# compression sinks


class CompressSink:
    def __init__(self, cmd):
        self.cmd = cmd
        self.proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.DEVNULL, env=SAFE_ENV)
        self.out_bytes = 0
        self.in_bytes = 0
        self.thread = threading.Thread(target=self._drain, daemon=True)
        self.thread.start()

    def _drain(self):
        while True:
            b = self.proc.stdout.read(1 << 16)
            if not b:
                break
            self.out_bytes += len(b)

    def write(self, mv):
        self.proc.stdin.write(mv)
        self.in_bytes += len(mv)

    def close(self):
        self.proc.stdin.close()
        self.thread.join()
        rc = self.proc.wait()
        if rc != 0:
            raise cl.CorpusError(f"{' '.join(self.cmd)} exited {rc}")


class Sampler:
    """Deterministic evenly spaced 1 MiB windows over a stream of known length."""

    def __init__(self, total: int, limit: int):
        self.total = total
        if limit <= 0 or total <= limit:
            self.windows = None
        else:
            n = max(1, limit // SEG)
            stride = total / n
            self.windows = [(int(i * stride), min(total, int(i * stride) + SEG)) for i in range(n)]
        self.pos = 0
        self.wi = 0

    def describe(self) -> dict:
        if self.windows is None:
            return {"sampled": False}
        return {"sampled": True, "method": "evenly spaced windows", "window_bytes": SEG,
                "windows": len(self.windows), "stream_bytes": self.total}

    def pieces(self, mv):
        n = len(mv)
        if self.windows is None:
            self.pos += n
            return [mv]
        start, end = self.pos, self.pos + n
        out = []
        while self.wi < len(self.windows):
            ws, we = self.windows[self.wi]
            if ws >= end:
                break
            s, e = max(ws, start), min(we, end)
            if s < e:
                out.append(mv[s - start:e - start])
            if we <= end:
                self.wi += 1
            else:
                break
        self.pos = end
        return out


# ---------------------------------------------------------------------------
# content analyzer


class Analyzer:
    def __init__(self, stream_bytes: int, compress_max_bytes: int, compress: bool):
        self.global_counts = np.zeros(256, dtype=np.uint64)
        self.file_counts = None
        self.block_hist = np.zeros(16, dtype=np.uint64)  # 0.5-bit bins over [0, 8]
        self.block_entropy_sum = 0.0
        self.full_blocks = 0
        self.zero_blocks = 0
        self.block_keys: set = set()
        self.blocks_total = 0
        self.block_bytes = 0
        self.dup_blocks = 0
        self.dup_block_bytes = 0
        self.file_entropy_hist_bytes = np.zeros(8, dtype=np.uint64)
        self.file_entropy_hist_count = np.zeros(8, dtype=np.uint64)
        self.file_entropy_weighted = 0.0
        self.sampler = Sampler(stream_bytes, compress_max_bytes)
        self.sinks = []
        if compress:
            self.sinks = [("zstd -3", CompressSink(["zstd", "-3", "-T1", "-c", "-q", "--no-progress"])),
                          ("xz -6", CompressSink(["xz", "-6", "-T1", "-c", "-q"]))]

    def start_file(self):
        self.file_counts = np.zeros(256, dtype=np.uint64)

    def _block_key(self, piece) -> bytes:
        return hashlib.blake2b(piece, digest_size=16).digest()

    def feed(self, mv):
        arr = np.frombuffer(mv, dtype=np.uint8)
        n = arr.shape[0]
        self.file_counts += np.bincount(arr, minlength=256).astype(np.uint64)
        nfull = n // BS
        if nfull:
            blocks = arr[: nfull * BS].reshape(nfull, BS)
            idx = (np.arange(nfull, dtype=np.int64)[:, None] * 256 + blocks).ravel()
            cnt = np.bincount(idx, minlength=nfull * 256).reshape(nfull, 256).astype(np.float64)
            p = cnt / BS
            with np.errstate(divide="ignore", invalid="ignore"):
                ent = -np.where(cnt > 0, p * np.log2(np.where(cnt > 0, p, 1.0)), 0.0).sum(axis=1)
            bins = np.minimum((ent * 2.0).astype(np.int64), 15)
            self.block_hist += np.bincount(bins, minlength=16).astype(np.uint64)
            self.block_entropy_sum += float(ent.sum())
            self.full_blocks += nfull
            self.zero_blocks += int((~blocks.any(axis=1)).sum())
            keys = self.block_keys
            for i in range(nfull):
                k = self._block_key(mv[i * BS:(i + 1) * BS])
                if k in keys:
                    self.dup_blocks += 1
                    self.dup_block_bytes += BS
                else:
                    keys.add(k)
            self.blocks_total += nfull
            self.block_bytes += nfull * BS
        tail = n - nfull * BS
        if tail:
            k = self._block_key(mv[nfull * BS:]) + tail.to_bytes(2, "little")
            if k in self.block_keys:
                self.dup_blocks += 1
                self.dup_block_bytes += tail
            else:
                self.block_keys.add(k)
            self.blocks_total += 1
            self.block_bytes += tail
        if self.sinks:
            for piece in self.sampler.pieces(mv):
                for _, s in self.sinks:
                    s.write(piece)

    def end_file(self, size: int):
        self.global_counts += self.file_counts
        if size:
            e = entropy_of_counts(self.file_counts)
            b = min(int(e), 7)
            self.file_entropy_hist_bytes[b] += size
            self.file_entropy_hist_count[b] += 1
            self.file_entropy_weighted += e * size
        self.file_counts = None

    def finish(self) -> dict:
        comp = {}
        for name, s in self.sinks:
            s.close()
            comp[name] = {"input_bytes": s.in_bytes, "output_bytes": s.out_bytes,
                          "ratio": r6(s.out_bytes / s.in_bytes) if s.in_bytes else None}
        total = int(self.global_counts.sum())
        return {
            "entropy": {
                "stream_order0_bits_per_byte": r6(entropy_of_counts(self.global_counts)),
                "mean_full_block_entropy_bits_per_byte": r6(self.block_entropy_sum / self.full_blocks)
                if self.full_blocks else None,
                "byte_weighted_mean_file_entropy_bits_per_byte": r6(self.file_entropy_weighted / total) if total else None,
                "file_entropy_histogram": {
                    "bins_bits_per_byte": [f"[{i},{i + 1})" if i < 7 else "[7,8]" for i in range(8)],
                    "files": [int(x) for x in self.file_entropy_hist_count],
                    "bytes": [int(x) for x in self.file_entropy_hist_bytes]},
                "block_entropy_histogram": {
                    "block_bytes": BS, "note": "full 4 KiB blocks only",
                    "bins_bits_per_byte": [f"[{i / 2:g},{(i + 1) / 2:g})" if i < 15 else "[7.5,8]" for i in range(16)],
                    "blocks": [int(x) for x in self.block_hist]},
            },
            "block_duplicates": {
                "block_bytes": BS, "blocks": self.blocks_total, "bytes": self.block_bytes,
                "duplicate_blocks": self.dup_blocks, "duplicate_bytes": self.dup_block_bytes,
                "duplicate_block_ratio": r6(self.dup_blocks / self.blocks_total) if self.blocks_total else 0.0,
                "duplicate_byte_ratio": r6(self.dup_block_bytes / self.block_bytes) if self.block_bytes else 0.0,
                "full_blocks": self.full_blocks, "zero_full_blocks": self.zero_blocks,
                "zero_full_block_ratio": r6(self.zero_blocks / self.full_blocks) if self.full_blocks else 0.0,
                "unique_block_keys": len(self.block_keys),
                "method": "per-file fixed 4 KiB blocks from offset 0; tail block keyed with its length; BLAKE2b-128",
            },
            "compressibility": dict(comp, **self.sampler.describe(),
                                    note="single-shot ratios over concatenated unique file contents in path order; "
                                         "indicator only, not a benchmark"),
        }


# ---------------------------------------------------------------------------
# libmagic, extensions, metadata


def mime_types(paths: list) -> list:
    out = []
    batch, blen = [], 0

    def flush():
        nonlocal batch, blen
        if not batch:
            return
        r = subprocess.run(["file", "--mime-type", "-b", "-N", "-E", "--"] + batch, capture_output=True, env=SAFE_ENV)
        lines = r.stdout.decode("utf-8", "replace").splitlines()
        if r.returncode != 0 or len(lines) != len(batch):
            raise cl.CorpusError(f"file(1) failed (exit {r.returncode}, {len(lines)}/{len(batch)} lines): "
                                 f"{r.stderr.decode('utf-8', 'replace')[:500]}")
        out.extend(ln.strip() for ln in lines)
        batch, blen = [], 0

    for p in paths:
        batch.append(p)
        blen += len(p) + 1
        if len(batch) >= 512 or blen > 256_000:
            flush()
    flush()
    return out


def extension_of(rel: bytes) -> str:
    name = rel.rsplit(b"/", 1)[-1]
    stem = name.lstrip(b".")
    if b"." not in stem:
        return "(none)"
    ext = stem.rsplit(b".", 1)[-1]
    if not ext:
        return "(none)"
    try:
        s = ext.decode("utf-8")
    except UnicodeDecodeError:
        return "(non-utf8)"
    if len(ext) > 16:
        return "(long)"
    return s.lower()


def top(d: dict, n: int) -> dict:
    items = sorted(d.items(), key=lambda kv: (-kv[1], kv[0]))
    out = dict(items[:n])
    rest = sum(v for _, v in items[n:])
    if rest:
        out["(other)"] = rest
    return out


def metadata_stats(sc: fpm.Scan, contents: dict) -> dict:
    groups = fpm.hardlink_groups(sc)
    in_item_links: dict = {}
    for e in sc.entries:
        if e.kind != "dir":
            k = (e.st.st_dev, e.st.st_ino)
            in_item_links[k] = in_item_links.get(k, 0) + 1
    types: dict = {}
    sym = {"total": 0, "absolute_target": 0, "relative_escaping_root": 0, "dangling_relative": 0,
           "non_utf8_target": 0}
    xattr = {"objects_with_xattrs": 0, "names_by_namespace": {}, "total_value_bytes": 0, "max_names_per_object": 0,
             "acl_access_objects": 0, "acl_default_dirs": 0}
    sparse = {"sparse_files": 0, "apparent_bytes": 0, "allocated_bytes": 0, "data_extent_bytes": 0,
              "hole_bytes": 0, "all_hole_files": 0}
    modes: dict = {}
    special_bits = {"setuid": 0, "setgid": 0, "sticky": 0, "world_writable_non_symlink": 0, "executable_files": 0}
    uids, gids = {}, {}
    names = {"non_utf8": 0, "control_chars": 0, "newline": 0, "windows_hostile": 0, "case_insensitive_collisions": 0,
             "max_name_bytes": 0}
    mt = {"min_s": None, "max_s": None, "distinct": 0, "subsecond_nonzero": 0, "before_1970": 0, "after_2038_01_19": 0}
    mtimes = set()
    children: dict = {}
    casefold: dict = {}
    external_links = 0
    seen_inodes = set()
    for e in sc.entries:
        types[e.kind] = types.get(e.kind, 0) + 1
        s = e.st
        if e.rel:
            parent, _, name = e.rel.rpartition(b"/")
            children[parent] = children.get(parent, 0) + 1
            names["max_name_bytes"] = max(names["max_name_bytes"], len(name))
            try:
                text = name.decode("utf-8")
                key = (parent, text.casefold())
            except UnicodeDecodeError:
                names["non_utf8"] += 1
                text = None
                key = (parent, name.lower())
            casefold[key] = casefold.get(key, 0) + 1
            if any(c < 0x20 or c == 0x7F for c in name):
                names["control_chars"] += 1
            if b"\n" in name:
                names["newline"] += 1
            base = (text if text is not None else name.decode("latin-1")).split(".")[0].lower()
            if any(c in WIN_BAD_CHARS or c < 0x20 for c in name) or name.endswith((b".", b" ")) or base in WIN_RESERVED:
                names["windows_hostile"] += 1
            mtimes.add(s.st_mtime_ns)
            sec = s.st_mtime_ns // 1_000_000_000
            mt["min_s"] = sec if mt["min_s"] is None else min(mt["min_s"], sec)
            mt["max_s"] = sec if mt["max_s"] is None else max(mt["max_s"], sec)
            if s.st_mtime_ns % 1_000_000_000:
                mt["subsecond_nonzero"] += 1
            if s.st_mtime_ns < 0:
                mt["before_1970"] += 1
            if sec > 2_147_483_647:
                mt["after_2038_01_19"] += 1
        perm = stat.S_IMODE(s.st_mode)
        if e.kind != "symlink":
            key = f"{e.kind}:{perm:04o}"
            modes[key] = modes.get(key, 0) + 1
            if perm & 0o002:
                special_bits["world_writable_non_symlink"] += 1
        if perm & stat.S_ISUID:
            special_bits["setuid"] += 1
        if perm & stat.S_ISGID:
            special_bits["setgid"] += 1
        if perm & stat.S_ISVTX:
            special_bits["sticky"] += 1
        uids[s.st_uid] = uids.get(s.st_uid, 0) + 1
        gids[s.st_gid] = gids.get(s.st_gid, 0) + 1
        if e.xattrs:
            xattr["objects_with_xattrs"] += 1
            xattr["max_names_per_object"] = max(xattr["max_names_per_object"], len(e.xattrs))
            for n, _, vlen in e.xattrs:
                ns = n.split(b".", 1)[0].decode("ascii", "replace") if b"." in n else "(none)"
                xattr["names_by_namespace"][ns] = xattr["names_by_namespace"].get(ns, 0) + 1
                xattr["total_value_bytes"] += vlen
                if n == b"system.posix_acl_access":
                    xattr["acl_access_objects"] += 1
                if n == b"system.posix_acl_default":
                    xattr["acl_default_dirs"] += 1
        if e.kind == "symlink":
            sym["total"] += 1
            t = e.target
            try:
                t.decode("utf-8")
            except UnicodeDecodeError:
                sym["non_utf8_target"] += 1
            if t.startswith(b"/"):
                sym["absolute_target"] += 1
            else:
                parts = [p for p in e.rel.split(b"/")[:-1]]
                depth_ok = True
                for comp in t.split(b"/"):
                    if comp in (b"", b"."):
                        continue
                    if comp == b"..":
                        if not parts:
                            depth_ok = False
                            break
                        parts.pop()
                    else:
                        parts.append(comp)
                if not depth_ok:
                    sym["relative_escaping_root"] += 1
                elif not os.path.exists(fpm.join(sc.root, e.rel)):
                    sym["dangling_relative"] += 1
        if e.kind == "file":
            if stat.S_IMODE(s.st_mode) & 0o111:
                special_bits["executable_files"] += 1
            k = (s.st_dev, s.st_ino)
            if s.st_nlink > in_item_links.get(k, 1) and k not in seen_inodes:
                external_links += 1
            if k not in seen_inodes:
                seen_inodes.add(k)
                _, ext = contents[k]
                data = sum(b - a for a, b in ext)
                if s.st_size and (s.st_blocks * 512 < s.st_size or data < s.st_size):
                    sparse["sparse_files"] += 1
                    sparse["apparent_bytes"] += s.st_size
                    sparse["allocated_bytes"] += s.st_blocks * 512
                    sparse["data_extent_bytes"] += data
                    sparse["hole_bytes"] += s.st_size - data
                    if data == 0:
                        sparse["all_hole_files"] += 1
    names["case_insensitive_collisions"] = sum(v - 1 for v in casefold.values() if v > 1)
    mt["distinct"] = len(mtimes)
    group_sizes = {}
    for gid, n, _ in groups.values():
        group_sizes[gid] = n
    dir_counts = list(children.values())
    return {
        "object_types": dict(sorted(types.items())),
        "symlinks": sym,
        "hardlinks": {"groups": len(group_sizes), "extra_paths": sum(n - 1 for n in group_sizes.values()),
                      "max_group_size": max(group_sizes.values()) if group_sizes else 0,
                      "files_with_links_outside_item": external_links},
        "xattrs": xattr,
        "sparse": sparse,
        "modes": {"distinct": len(modes), "top": top(modes, 20), **special_bits},
        "ownership": {"distinct_uids": len(uids), "distinct_gids": len(gids),
                      "top_uids": {str(k): v for k, v in top(uids, 10).items()},
                      "top_gids": {str(k): v for k, v in top(gids, 10).items()}},
        "names": names,
        "mtimes": mt,
        "directories": {"max_entries": max(dir_counts) if dir_counts else 0,
                        "mean_entries": r6(sum(dir_counts) / len(dir_counts)) if dir_counts else 0.0},
    }


# ---------------------------------------------------------------------------
# per item


def tool_info() -> dict:
    magic = None
    for p in ("/usr/lib/file/magic.mgc", "/usr/share/misc/magic.mgc", "/usr/share/file/magic.mgc"):
        if os.path.exists(p):
            magic = {"path": os.path.realpath(p), "sha256": cl.sha256_file(os.path.realpath(p))[0]}
            break
    return {
        "stats_sha256": cl.sha256_file(Path(__file__).resolve())[0],
        "fingerprint_sha256": cl.sha256_file(Path(fpm.__file__).resolve())[0],
        "file_version": cl.command_version(["file", "--version"]),
        "magic_mgc": magic,
        "zstd_version": cl.command_version(["zstd", "--version"]),
        "xz_version": cl.command_version(["xz", "--version"]),
        "numpy_version": np.__version__,
        "python": sys.version.split()[0],
    }


def analyze_tree(root, compress_max_bytes: int, compress: bool = True) -> tuple[dict, dict]:
    sc = fpm.scan(root)
    files = fpm.unique_files(sc)
    stream_bytes = sum(e.st.st_size for e in files)
    an = Analyzer(stream_bytes, compress_max_bytes, compress)
    contents = {}
    exact: dict = {}
    for e in files:
        an.start_file()
        digest, ext = fpm.hash_file(fpm.join(sc.root, e.rel), e.st, sink=an.feed)
        an.end_file(e.st.st_size)
        contents[(e.st.st_dev, e.st.st_ino)] = (digest, ext)
        if e.st.st_size:
            exact.setdefault(digest, []).append(e.st.st_size)
    content = an.finish()
    fp = fpm.finalize(sc, contents)

    mimes = mime_types([fpm.join(sc.root, e.rel) for e in files])
    by_count, by_bytes, ext_count, ext_bytes = {}, {}, {}, {}
    for e, m in zip(files, mimes):
        by_count[m] = by_count.get(m, 0) + 1
        by_bytes[m] = by_bytes.get(m, 0) + e.st.st_size
        x = extension_of(e.rel)
        ext_count[x] = ext_count.get(x, 0) + 1
        ext_bytes[x] = ext_bytes.get(x, 0) + e.st.st_size
    nonempty = sum(len(v) for v in exact.values())
    nonempty_bytes = sum(sum(v) for v in exact.values())
    dup_files = sum(len(v) - 1 for v in exact.values())
    dup_bytes = sum(sum(v) - v[0] for v in exact.values())
    stats = {
        "totals": {"unique_file_inodes": len(files), "bytes": stream_bytes,
                   "empty_files": len(files) - nonempty, "objects": len(sc.entries)},
        "content_types": {"distinct": len(by_count), "by_count": top(by_count, 40), "by_bytes": top(by_bytes, 40)},
        "extensions": {"distinct": len(ext_count), "by_count": top(ext_count, 40), "by_bytes": top(ext_bytes, 40)},
        "exact_duplicates": {
            "nonempty_files": nonempty, "nonempty_bytes": nonempty_bytes, "distinct_contents": len(exact),
            "duplicate_files": dup_files, "duplicate_bytes": dup_bytes,
            "duplicate_file_ratio": r6(dup_files / nonempty) if nonempty else 0.0,
            "duplicate_byte_ratio": r6(dup_bytes / nonempty_bytes) if nonempty_bytes else 0.0,
            "note": "unique inodes (hardlinks counted once); empty files excluded"},
        "block_duplicates": content["block_duplicates"],
        "entropy": content["entropy"],
        "compressibility": content["compressibility"],
        "metadata": metadata_stats(sc, contents),
        "structure": fp["structure"],
    }
    return fp, stats


def stats_for_item(item: dict, layout_args: tuple, compress_max_bytes: int, force: bool, unlock: str | None) -> str:
    os.umask(0o022)
    layout = cl.Layout(*layout_args)
    iid = item["item_id"]
    heldout = item["split"] == "heldout"
    if heldout:
        cl.check_heldout_unlock(layout, unlock)
    record = cl.load_json_if_exists(layout.record_path(iid))
    if not record:
        raise cl.CorpusError(f"{iid}: no fingerprint record; run provision first")
    path = layout.item_path(item, allow_heldout=heldout)
    if not heldout:
        cl.assert_not_heldout(path, layout)
    if not path.is_dir():
        raise cl.CorpusError(f"{iid}: {path} is not materialized")
    logical = record["fingerprint"]["logical_tree_sha256"]
    out_path = layout.stats_path(iid)
    info = tool_info()
    params = {"block_bytes": BS, "compress_max_bytes": compress_max_bytes, "zstd": "-3 -T1", "xz": "-6 -T1",
              "mime": "file --mime-type -b -N"}
    old = cl.load_json_if_exists(out_path)
    if old and not force and old.get("logical_tree_sha256") == logical and old.get("params") == params \
            and old.get("tool", {}).get("stats_sha256") == info["stats_sha256"] \
            and old.get("tool", {}).get("fingerprint_sha256") == info["fingerprint_sha256"]:
        return "up-to-date"
    fp, stats = analyze_tree(path, compress_max_bytes)
    if fp["logical_tree_sha256"] != logical:
        raise cl.HashMismatch(f"HASH MISMATCH: {iid} on disk {fp['logical_tree_sha256']} != recorded {logical}")
    doc = {
        "format": FORMAT, "corpus_version": cl.CORPUS_VERSION, "item_id": iid, "family": item["family"],
        "split": item["split"], "scale": item["scale"], "logical_tree_sha256": logical,
        "tool": info, "params": params, **stats,
    }
    cl.write_json_atomic(out_path, doc)
    return (f"stats written: {len(stats['content_types']['by_count'])} mime types, "
            f"zstd {stats['compressibility'].get('zstd -3', {}).get('ratio')}, "
            f"xz {stats['compressibility'].get('xz -6', {}).get('ratio')}")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--corpus-dir")
    ap.add_argument("--data-root")
    ap.add_argument("--item", action="append")
    ap.add_argument("--family", action="append")
    ap.add_argument("--split", action="append", help="tuning|validation (heldout needs unlock)")
    ap.add_argument("--all", action="store_true", help="all materialized tuning+validation items")
    ap.add_argument("--path", help="ad hoc: analyze an arbitrary non-held-out directory")
    ap.add_argument("--out", help="with --path: output JSON file (default stdout)")
    ap.add_argument("--jobs", type=int, default=2, help="items analyzed concurrently (processes)")
    ap.add_argument("--compress-max-bytes", type=int, default=1 << 30,
                    help="sample compressibility streams above this many bytes (0 = never sample); default 1 GiB")
    ap.add_argument("--force", action="store_true")
    ap.add_argument("--unlock-heldout", metavar="DESIGN_FREEZE_COMMIT")
    args = ap.parse_args(argv)
    cl.require_linux()
    if np is None:
        print("ERROR: numpy is required; run research/tools/setup_venv.sh and use /root/eb-research/venv/bin/python")
        return 2
    layout = cl.Layout(args.corpus_dir, args.data_root)

    if args.path:
        if cl.is_heldout_path(args.path, layout):
            try:
                cl.check_heldout_unlock(layout, args.unlock_heldout)
            except cl.HeldoutAccessError as e:
                print(f"REFUSED: {e}")
                return 3
        fp, stats = analyze_tree(args.path, args.compress_max_bytes)
        doc = {"format": FORMAT, "path": args.path, "logical_tree_sha256": fp["logical_tree_sha256"],
               "tool": tool_info(), **stats}
        if args.out:
            cl.write_json_atomic(args.out, doc)
        else:
            json.dump(doc, sys.stdout, indent=2)
            print()
        return 0

    items, errors, warnings = cl.load_sources(layout)
    if errors:
        for e in errors:
            print(f"ERROR: {e}")
        return 2
    fams = {x for v in args.family or [] for x in v.split(",")}
    splits = {x for v in args.split or [] for x in v.split(",")}
    ids = {x for v in args.item or [] for x in v.split(",")}
    unknown = ids - set(items)
    if unknown:
        print(f"ERROR: unknown item(s) {sorted(unknown)}")
        return 2
    if not (args.all or ids or fams or splits):
        ap.error("select items with --item/--family/--split or --all")
    selected = []
    refused = []
    for iid in sorted(items):
        it = items[iid]
        if ids and iid not in ids:
            continue
        if fams and it["family"] not in fams:
            continue
        if splits and it["split"] not in splits:
            continue
        if it["split"] == "heldout":
            explicit = iid in ids or "heldout" in splits
            if not explicit:
                continue
            try:
                cl.check_heldout_unlock(layout, args.unlock_heldout)
            except cl.HeldoutAccessError as e:
                refused.append((iid, str(e)))
                continue
        if not (ids and iid in ids) and not cl.load_json_if_exists(layout.record_path(iid)):
            continue  # not materialized; skip silently in bulk mode
        selected.append(it)
    for iid, why in refused:
        print(f"[{iid}] REFUSED (held-out): {why}")
    if refused:
        return 3
    failed = 0
    with ProcessPoolExecutor(max_workers=max(1, args.jobs)) as ex:
        futs = {ex.submit(stats_for_item, it, (args.corpus_dir, args.data_root), args.compress_max_bytes,
                          args.force, args.unlock_heldout): it["item_id"] for it in selected}
        for fut in as_completed(futs):
            iid = futs[fut]
            try:
                print(f"[{iid}] {fut.result()}", flush=True)
            except Exception as e:  # noqa: BLE001
                failed += 1
                print(f"[{iid}] FAILED: {e}", flush=True)
    print(f"stats: {len(selected) - failed} ok, {failed} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
