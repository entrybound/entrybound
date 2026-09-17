#!/usr/bin/env python3
"""Build research/baselines/capability-matrix.csv from:
  * research/baselines/probes/capability-probe-results.json (PROBE rows: what
    each representative config preserved after create+extract on the fixture
    tree, research/baselines/probes/make_fixtures.py, run as root on ext4)
  * research/normalized/EXP-BASE-SIZE/summary.json if present (PROBE rows for
    deterministic_output, reusing the real size/determinism pass instead of
    re-probing)
  * DOC rows, hand-cited to each tool's own documentation, for capabilities the
    fixture round-trip cannot exercise (encryption, authentication, streaming,
    random access / partial retrieval) -- see DOC_ROWS below for citations.

One representative config per incumbent family, plus Entrybound (ebound),
per research/baselines/README.md's fair-comparison rules: a baseline lacking a
capability is stated here, never used to exclude Entrybound's own cost for it.
"""
from __future__ import annotations

import csv
import hashlib
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
RESULTS = HERE / "probes" / "capability-probe-results.json"
RAW_DIR = HERE.parents[0] / "raw" / "EXP-BASE-SIZE"
OUT = HERE / "capability-matrix.csv"

FIELDS = ["system_config", "capability", "support", "method", "evidence_ref", "notes"]

FAMILY_OF = {
    "zip-9": "zip-deflate", "7z-mx9-solid": "7z-lzma2", "tar-gzip-9": "tar-gzip",
    "tar-zstd-19": "tar-zstd", "tar-xz-9e": "tar-xz", "tar-lz4": "tar-lz4",
    "tar-brotli-q11": "tar-brotli", "squashfs-zstd": "squashfs", "wim-lzms-solid": "wim",
    "borg": "borg-dedup", "restic": "restic-dedup", "dwarfs": "dwarfs", "zpaq": "zpaq",
    "entrybound-ebound": "entrybound",
}


def support(v: bool | None) -> str:
    if v is None:
        return "N/A"
    return "YES" if v else "NO"


def rows_from_probe(cfg: str, r: dict) -> list[dict]:
    ev = f"research/baselines/probes/capability-probe-results.json#{cfg}"
    out = []

    def add(cap, sup, notes=""):
        out.append({"system_config": cfg, "capability": cap, "support": sup, "method": "PROBE", "evidence_ref": ev, "notes": notes})

    strict = r.get("strict_refusal_findings", {})

    # non-UTF-8 paths
    if "nonutf8" in strict:
        add("non_utf8_paths", "NO", f"whole input tree refused: {strict['nonutf8']['stderr'][:200]}")
    else:
        add("non_utf8_paths", support(r.get("nonutf8_present")))

    # Unicode paths
    if r.get("config") == "entrybound-ebound" and "nonutf8" in strict:
        pass  # unicode row still meaningful (probed against a fixture that has it)
    add("unicode_paths", support(r.get("unicode_dir_present") and r.get("unicode_file_present")))

    # mtime / atime (ctime and birthtime are handled as fixed N/A rows below, not per-probe)
    mt = r.get("mtime")
    add("mtime_precision", support(mt["match"]) if mt else "N/A",
        f"got={mt['got']} want={mt['want']}" if mt else "not probed (extraction produced no output)")
    at = r.get("atime")
    add("atime_precision", support(at["match"]) if at else "N/A",
        "most incumbents intentionally do not restore atime" if at else "not probed")

    # permissions: PARTIAL if some but not all of the 4 probe files matched
    perms = r.get("permissions") or {}
    vals = [v["match"] for v in perms.values() if v]
    if not vals:
        add("permissions", "N/A", "not probed")
    elif all(vals):
        add("permissions", "YES")
    elif any(vals):
        add("permissions", "PARTIAL", json.dumps({k: v["match"] for k, v in perms.items() if v}))
    else:
        add("permissions", "NO")

    own = r.get("ownership")
    add("ownership", support(own["match"]) if own else "N/A",
        f"got=uid:{own['uid']}/gid:{own['gid']}" if own else "not probed")

    acl = r.get("acl") or {}
    if "acl_mode_mismatch" in strict:
        add("posix_acls", "NO", f"whole input tree refused: {strict['acl_mode_mismatch']['stderr'][:200]}")
    elif not acl.get("present"):
        add("posix_acls", "N/A", "extraction produced no output to check")
    else:
        add("posix_acls", support(acl.get("has_entry")))

    xattr = r.get("xattr") or {}
    if not xattr.get("present"):
        add("xattrs", "N/A", "extraction produced no output to check")
    else:
        add("xattrs", support(xattr.get("match")), "" if xattr.get("match") else xattr.get("value", "")[:200])

    sparse = r.get("sparse") or {}
    if not sparse.get("present"):
        add("sparse_files", "N/A", "extraction produced no output to check")
    else:
        still = sparse.get("still_sparse")
        add("sparse_files", "YES" if still else "PARTIAL",
            f"apparent={sparse.get('apparent_bytes')} allocated={sparse.get('allocated_bytes')}"
            + ("" if still else " (materialized as a fully-allocated file: content preserved, sparseness not)"))

    sym = r.get("symlinks") or {}
    if "symlink_export" in strict:
        add("symlinks", "NO", f"export refused for the entry kind: {strict['symlink_export']['stderr'][:200]}")
    elif not sym:
        add("symlinks", "N/A", "extraction produced no output to check")
    else:
        rel = sym.get("symlinks/rel-link.txt", {})
        dangling = sym.get("symlinks/dangling-link.txt", {})
        ok = rel.get("is_symlink") and rel.get("target_match") and dangling.get("is_symlink") and dangling.get("target_match")
        add("symlinks", "YES" if ok else ("PARTIAL" if rel.get("is_symlink") else "NO"),
            "relative and dangling symlink targets preserved" if ok else json.dumps(sym)[:200])
        abslink_note = r.get("extract_stderr", "")
        if "Dangerous link path" in abslink_note or "abs-link" in abslink_note:
            add("symlinks_absolute_target", "PARTIAL", "an absolute-path symlink target was refused/skipped at extract time: " + abslink_note[:200])

    hl = r.get("hardlinks") or {}
    if not hl.get("present"):
        add("hardlinks", "N/A", "extraction produced no output to check")
    else:
        add("hardlinks", support(hl.get("same_inode")), f"nlink={hl.get('nlink1')}/{hl.get('nlink2')}")

    # birth/ctime: not meaningfully testable as an archive-preservation capability.
    add("ctime_precision", "N/A", "ctime is the inode's last-metadata-change time, always set by the extraction "
        "operation itself; no format or tool can restore an earlier ctime, so this is not a capability to compare")
    birth = r.get("birthtime_raw")
    if birth in (None, 0):
        add("birthtime_precision", "N/A", "stat(1) %W reported 0/unset for the extracted file on this ext4 mount; "
            "ext4 stores a crtime but no probed tool attempted to set it, and GNU coreutils' own birth-time read "
            "support is inconsistent across kernels")
    else:
        add("birthtime_precision", "NO", f"raw stat %W={birth} (the extraction moment, not the original fixture's creation time; "
            "no probed tool has a mechanism to declare a birth time on extraction)")
    return out


DOC_ROWS = [
    # (system_config, capability, support, evidence_ref, notes)
    ("zip-9", "encryption", "PARTIAL", "Info-ZIP zip(1) manual, -e/--password", "legacy ZipCrypto only (weak, not authenticated); no AES in stock Info-ZIP zip"),
    ("zip-9", "authentication", "NO", "PKWARE APPNOTE.TXT 4.4.7 (CRC-32 only)", "CRC-32 per entry only, not a cryptographic MAC (REQ-CRY-0014/0102)"),
    ("zip-9", "random_access", "YES", "PKWARE APPNOTE.TXT central directory", "central directory enables listing and per-entry extraction without full decompression"),
    ("zip-9", "partial_retrieval", "YES", "PROBE: unzip -p <archive> <member>", "single named member extracted without extracting the rest"),
    ("zip-9", "streaming_creation", "PARTIAL", "PKWARE APPNOTE.TXT data descriptors", "supports streamed writes via data descriptors, but the central directory (needed for random access) is only finalized at the end"),
    ("zip-9", "streaming_extraction", "YES", "PKWARE APPNOTE.TXT local file headers", "a conforming reader can stream-extract via local headers without the central directory"),

    ("7z-mx9-solid", "encryption", "YES", "7-Zip help (7z i / -mhe=on)", "AES-256-CBC via -p, optionally -mhe=on to also encrypt filenames; CBC only, no AEAD"),
    ("7z-mx9-solid", "authentication", "NO", "7-Zip format docs", "CRC-32 per stream only when present; AES-CBC mode itself provides no integrity (REQ-CRY-0014/0102)"),
    ("7z-mx9-solid", "random_access", "PARTIAL", "7-Zip format docs (solid blocks)", "solid mode (used here, mx=9 default) requires decoding a whole solid block to reach one member; -ms=off (a separate config) restores per-file access at a ratio cost"),
    ("7z-mx9-solid", "partial_retrieval", "PARTIAL", "PROBE: 7z e <archive> <member>", "extracts the named member, but solid blocking still requires decoding other members sharing its block"),
    ("7z-mx9-solid", "streaming_creation", "NO", "7-Zip format docs", "the format's header/folder graph is written after all data, so 7z cannot create output before it has scanned/planned the whole input"),
    ("7z-mx9-solid", "streaming_extraction", "PARTIAL", "7-Zip format docs", "non-solid single-file streams can be decoded in order; the solid mode used here cannot be decoded from a stdin stream member-by-member"),

    ("tar-gzip-9", "encryption", "NO", "GNU tar / gzip manuals", "neither tar nor gzip has a native encryption mode; would require piping through a separate cipher"),
    ("tar-gzip-9", "authentication", "PARTIAL", "RFC 1952 (gzip format, CRC-32)", "gzip's trailer CRC-32 detects accidental corruption of the compressed stream only, not a cryptographic authentication tag"),
    ("tar-gzip-9", "random_access", "NO", "GNU tar manual; RFC 1952", "both tar and gzip are sequential formats with no index; reaching any member requires decompressing from the start"),
    ("tar-gzip-9", "partial_retrieval", "PARTIAL", "PROBE: tar xzf <archive> <member>", "tar can select one member by name, but the gzip layer beneath it must still be decompressed sequentially up to that point"),
    ("tar-gzip-9", "streaming_creation", "YES", "GNU tar manual; RFC 1952", "both layers are single-pass streams; this is exactly the tar|gzip pipe used to create the artifact"),
    ("tar-gzip-9", "streaming_extraction", "YES", "GNU tar manual; RFC 1952", "both layers decode sequentially from a stream with no seeking"),

    ("tar-zstd-19", "encryption", "NO", "Zstandard Format RFC 8878", "no native encryption in the zstd frame format"),
    ("tar-zstd-19", "authentication", "PARTIAL", "RFC 8878 §3.1.1 (optional content checksum)", "an optional 32-bit content checksum (used by the zstd CLI by default) detects corruption, not a cryptographic MAC"),
    ("tar-zstd-19", "random_access", "PARTIAL", "RFC 8878 Appendix (seekable format); Facebook seekable-zstd spec", "the plain frame used here has no index; the related seekable-zstd container format (not used by this config) adds one -- see tarxz-pixz for the indexed analogue this suite does run"),
    ("tar-zstd-19", "partial_retrieval", "PARTIAL", "PROBE: tar --zstd -xf <archive> <member>", "tar can select one member, but the zstd frame must still be decompressed sequentially up to it"),
    ("tar-zstd-19", "streaming_creation", "YES", "RFC 8878", "single-pass streaming frame format; this is exactly the tar|zstd pipe used to create the artifact"),
    ("tar-zstd-19", "streaming_extraction", "YES", "RFC 8878", "decodes sequentially from a stream with no seeking"),

    ("tar-xz-9e", "encryption", "NO", "XZ Format spec (xz-file-format.txt)", "no native encryption in the xz container"),
    ("tar-xz-9e", "authentication", "PARTIAL", "XZ Format spec (per-block/stream CRC)", "CRC-32/64 integrity check per block, not a cryptographic MAC"),
    ("tar-xz-9e", "random_access", "NO", "XZ Format spec", "plain xz (this config) has no index; pixz (a separate config in this suite) adds a block index for exactly this reason"),
    ("tar-xz-9e", "partial_retrieval", "PARTIAL", "PROBE: tar xJf <archive> <member>", "tar selects one member, but the xz stream must still be decompressed sequentially up to it"),
    ("tar-xz-9e", "streaming_creation", "YES", "XZ Format spec", "single-pass streaming format; this is exactly the tar|xz pipe used to create the artifact"),
    ("tar-xz-9e", "streaming_extraction", "YES", "XZ Format spec", "decodes sequentially with no seeking"),

    ("tar-lz4", "encryption", "NO", "LZ4 Frame Format spec", "no native encryption"),
    ("tar-lz4", "authentication", "PARTIAL", "LZ4 Frame Format spec (optional per-block/content checksum)", "detects corruption, not a cryptographic MAC"),
    ("tar-lz4", "random_access", "NO", "LZ4 Frame Format spec", "sequential frame format, no index"),
    ("tar-lz4", "partial_retrieval", "PARTIAL", "PROBE: tar xf <archive> -I lz4 <member>", "tar selects one member, but the lz4 stream decodes sequentially up to it"),
    ("tar-lz4", "streaming_creation", "YES", "LZ4 Frame Format spec", "single-pass streaming format"),
    ("tar-lz4", "streaming_extraction", "YES", "LZ4 Frame Format spec", "decodes sequentially with no seeking"),

    ("tar-brotli-q11", "encryption", "NO", "RFC 7932 (Brotli format)", "no native encryption"),
    ("tar-brotli-q11", "authentication", "NO", "RFC 7932", "the brotli stream format has no built-in checksum or MAC at all"),
    ("tar-brotli-q11", "random_access", "NO", "RFC 7932", "sequential format, no index"),
    ("tar-brotli-q11", "partial_retrieval", "PARTIAL", "PROBE: tar xf <archive> --use-compress-program brotli -d <member>", "tar selects one member, but the brotli stream decodes sequentially up to it"),
    ("tar-brotli-q11", "streaming_creation", "YES", "RFC 7932", "single-pass streaming format"),
    ("tar-brotli-q11", "streaming_extraction", "YES", "RFC 7932", "decodes sequentially with no seeking"),

    ("squashfs-zstd", "encryption", "NO", "squashfs-tools / kernel squashfs.txt", "no native encryption (some vendor forks add it; not in mainline squashfs-tools 4.6.1)"),
    ("squashfs-zstd", "authentication", "NO", "kernel Documentation/filesystems/squashfs.txt", "no per-block or whole-image cryptographic authentication"),
    ("squashfs-zstd", "random_access", "YES", "kernel Documentation/filesystems/squashfs.txt", "block-indexed read-only filesystem image; any file is independently locatable and decompressible"),
    ("squashfs-zstd", "partial_retrieval", "YES", "PROBE: unsquashfs -f -d <dest> <archive> <member-path>", "extracts one named member directly, without extracting the rest"),
    ("squashfs-zstd", "streaming_creation", "NO", "squashfs-tools manual", "mksquashfs needs to see and sort the whole input tree to build the image and its indexes before writing any output"),
    ("squashfs-zstd", "streaming_extraction", "PARTIAL", "kernel Documentation/filesystems/squashfs.txt", "the on-disk image is randomly seekable (typically loop-mounted or read via unsquashfs), but is not designed to be consumed as a single forward-only stream the way tar is"),

    ("wim-lzms-solid", "encryption", "NO", "wimlib documentation (Encryption)", "wimlib does not support WIM encryption (Microsoft's own imagex/DISM support EFS-wrapped WIMs in restricted scenarios not exposed by wimlib)"),
    ("wim-lzms-solid", "authentication", "PARTIAL", "MS-WIM / wimlib docs (per-chunk SHA-1 and image hash)", "integrity hashes catch corruption; not a keyed authentication scheme"),
    ("wim-lzms-solid", "random_access", "PARTIAL", "MS-WIM specification; wimlib docs", "non-solid WIM resources are independently seekable; --solid (used in this config, matching the task's requested LZMS-solid variant) groups streams into solid resource chunks, reducing per-file random access"),
    ("wim-lzms-solid", "partial_retrieval", "YES", "PROBE: wimextract <archive> 1 <member-path>", "extracts one named file from the image directly"),
    ("wim-lzms-solid", "streaming_creation", "NO", "wimlib documentation", "wimcapture scans the whole source tree, deduplicates streams and writes the lookup table/XML info after data, so it is not a single forward pass"),
    ("wim-lzms-solid", "streaming_extraction", "PARTIAL", "wimlib documentation", "wimlib can apply an image from a pipable WIM created for that purpose; the archive created by this config is not built pipable, so ordinary extraction needs random access to the file"),

    ("borg", "encryption", "YES", "borgbackup documentation, --encryption", "repokey/keyfile modes use AES-CTR with an HMAC-SHA256 authentication tag per chunk (this config uses --encryption=none for the size/determinism pass; see notes)"),
    ("borg", "authentication", "YES", "borgbackup documentation (AEAD chunk format)", "each chunk is authenticated (HMAC) when encryption is enabled; with --encryption=none (this config) only a checksum, not authentication, is present"),
    ("borg", "random_access", "YES", "borgbackup Internals documentation (chunk index)", "content-defined chunking with a repository-wide chunk index; any file's chunks are independently locatable"),
    ("borg", "partial_retrieval", "YES", "PROBE: borg extract <repo>::<archive> <path>", "restores one named path from the archive without restoring the rest"),
    ("borg", "streaming_creation", "YES", "borgbackup documentation", "borg create chunks and uploads/writes data as it walks the source tree"),
    ("borg", "streaming_extraction", "PARTIAL", "borgbackup documentation", "extract can process files in archive order without a separate random-access pass, but reading the repository's chunk index first is required, so it is not a pure single-pass stream"),

    ("restic", "encryption", "YES", "restic design documentation", "always encrypted (AES-256-CTR + Poly1305-AES MAC per pack blob); this config still supplies a password, since restic has no unencrypted mode"),
    ("restic", "authentication", "YES", "restic design documentation", "every blob is individually authenticated (Poly1305-AES MAC)"),
    ("restic", "random_access", "YES", "restic design documentation (index)", "content-defined chunking with a repository index mapping blobs to pack files and offsets"),
    ("restic", "partial_retrieval", "YES", "PROBE: restic restore --include <path> <snapshot>", "restores one named path without restoring the rest of the snapshot"),
    ("restic", "streaming_creation", "YES", "restic design documentation", "restic backup chunks and uploads data as it walks the source tree"),
    ("restic", "streaming_extraction", "PARTIAL", "restic design documentation", "restore reads the repository index before writing files, so it is not a pure single-pass stream, though no full-archive decode is needed to reach one file"),

    ("dwarfs", "encryption", "NO", "DwarFS documentation", "no native encryption in the DwarFS image format"),
    ("dwarfs", "authentication", "PARTIAL", "DwarFS documentation (per-block checksums)", "block checksums (xxhash/SHA) detect corruption; not a keyed authentication scheme"),
    ("dwarfs", "random_access", "YES", "DwarFS documentation", "designed for FUSE mounting with random file access; per-category codec selection is itself built on independently addressable blocks"),
    ("dwarfs", "partial_retrieval", "YES", "PROBE: dwarfsextract -i <archive> -o <dest> --pattern <member>", "extracts a matched subset of the image directly"),
    ("dwarfs", "streaming_creation", "NO", "DwarFS documentation", "mkdwarfs needs the whole input tree to perform its similarity/ordering and deduplication analysis before writing output"),
    ("dwarfs", "streaming_extraction", "PARTIAL", "DwarFS documentation", "dwarfsextract can extract in image order without mounting, but the image itself is a random-access structure, not a forward-only stream"),

    ("zpaq", "encryption", "YES", "zpaq documentation, -key", "AES-256-CTR via -key (not used in this config, which is unencrypted for the size/determinism pass)"),
    ("zpaq", "authentication", "NO", "zpaq documentation", "no keyed authentication tag on the archive/journal"),
    ("zpaq", "random_access", "PARTIAL", "zpaq documentation (block/fragment index)", "zpaq indexes fragments for dedup and versioning, but is primarily designed for whole-archive journaled add/extract, not general random access"),
    ("zpaq", "partial_retrieval", "PARTIAL", "PROBE: zpaq x <archive> <member> -to <dest>", "can extract a named file, but journaling/versioning bookkeeping is read regardless"),
    ("zpaq", "streaming_creation", "NO", "zpaq documentation", "zpaq add computes a journal entry against prior archive state, which is not a pure forward-only stream operation"),
    ("zpaq", "streaming_extraction", "NO", "zpaq documentation", "extraction resolves the most recent version of each file across the journal's update history before writing"),
    ("zpaq", "symlinks", "NO", "PROBE: zpaq add/extract on a tree containing a symlink", "confirmed by capability probe: the symlink fixture is silently dropped on add, not merely on extract (zpaq has no symlink entry kind in this version, 7.15)"),

    ("entrybound-ebound", "encryption", "YES", "`ebound pack --help` (this build, dev HEAD)", "--recipient (X-Wing draft-10, repeatable) or --password; encryption is INDEXED-layout-only per the CLI's own help text"),
    ("entrybound-ebound", "authentication", "YES", "docs/crypto-threat-model-v1.md (unpublished design doc, cited by REQ-CRY-0014/0102 in research/requirement-ledger.csv)", "the ledger records Entrybound's encrypted container as authenticated (AEAD), unlike 7z/RAR5/zpaq; not independently re-verified by this probe pass (would need --recipient/--password exercised here)"),
    ("entrybound-ebound", "random_access", "YES", "`ebound pack --help` (--layout indexed)", "the indexed layout is the format's random-access organization; a regular-file output defaults to it"),
    ("entrybound-ebound", "partial_retrieval", "PARTIAL", "this build's CLI surface (pack/convert/publish only)", "no direct 'extract one member to a path' subcommand exists yet in this experimental bootstrap CLI; retrieving one file today means exporting to a legacy format (convert --to tar) and extracting from that, which is not single-member-direct"),
    ("entrybound-ebound", "streaming_creation", "PARTIAL", "`ebound pack --help` (--layout stream, --stream-window)", "a stream-layout output (writing to `-`) is designed for bounded-lookback streaming creation; the indexed layout used by this probe's default is not"),
    ("entrybound-ebound", "streaming_extraction", "PARTIAL", "`ebound pack --help` (--layout stream)", "the stream layout is designed for streaming decode; not exercised by this probe pass, which used the default indexed layout"),
    ("entrybound-ebound", "symlinks", "NO", "PROBE: ebound pack + convert --to tar on a tree containing a symlink (capability-probe-results.json#entrybound-ebound.strict_refusal_findings.symlink_export)", "the tar export path in this dev-HEAD build refuses any symlink entry (\"the frozen target profile has no native symlink representation\"); unknown whether the native .eb format itself represents symlinks, since there is no direct extract-to-filesystem command yet to test that independently of the tar export path"),
]


def determinism_from_raw() -> dict:
    """{candidate: {item_key: {"reps": n, "deterministic": bool}}} from every
    *.jsonl under RAW_DIR. "reps" counts actual valid rows observed for that
    (candidate, item) pair; "deterministic" is only meaningful once reps >= 2
    (see call site), so it is computed from a distinct-digest count, not from a
    deduplicated set alone, which would always look like size 1 with one rep."""
    if not RAW_DIR.is_dir():
        return {}
    per_pair: dict[str, dict[str, list]] = {}
    for jf in RAW_DIR.glob("*.jsonl"):
        with open(jf, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    row = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if not row.get("valid", row.get("ok")):
                    continue
                cand = row.get("candidate")
                item = row.get("item_id")
                dig = (row.get("metrics") or {}).get("artifact_tree_sha256")
                if not cand or not item or not dig:
                    continue
                per_pair.setdefault(cand, {}).setdefault(item, []).append(dig)
    return {
        cand: {
            item: {"reps": len(dig_list), "deterministic": len(set(dig_list)) == 1}
            for item, dig_list in items.items()
        }
        for cand, items in per_pair.items()
    }


def entrybound_determinism_row() -> dict:
    """Entrybound's own docs claim unencrypted output is deterministic (`ebound
    pack --help`); probe it directly (two packs of the same fixture, compare
    sha256) rather than only citing the claim."""
    fixture = Path("/root/eb-research/cache/baselines/fixture-lenient")
    scratch = Path("/root/eb-research/scratch/baselines/capability")
    ebound = "/root/eb-research/target/baseline/release/ebound"
    if not fixture.is_dir() or not Path(ebound).exists():
        return {"system_config": "entrybound-ebound", "capability": "deterministic_output", "support": "N/A",
                "method": "DOC", "evidence_ref": "ebound pack --help", "notes": "not probed this build (fixture/binary unavailable); "
                "docs state: \"Unencrypted output is deterministic\""}
    out1, out2 = scratch / "determinism1.eb", scratch / "determinism2.eb"
    for out in (out1, out2):
        subprocess.run(["bash", "-c", f"{ebound} pack {fixture} {out} --profile balanced"], capture_output=True, timeout=120)
    if not (out1.exists() and out2.exists()):
        return {"system_config": "entrybound-ebound", "capability": "deterministic_output", "support": "N/A",
                "method": "PROBE", "evidence_ref": "research/baselines/build_capability_matrix.py:entrybound_determinism_row",
                "notes": "probe pack failed; see ebound pack --help claim instead"}
    h1 = hashlib.sha256(out1.read_bytes()).hexdigest()
    h2 = hashlib.sha256(out2.read_bytes()).hexdigest()
    return {"system_config": "entrybound-ebound", "capability": "deterministic_output", "support": "YES" if h1 == h2 else "NO",
            "method": "PROBE", "evidence_ref": "research/baselines/build_capability_matrix.py:entrybound_determinism_row",
            "notes": f"two unencrypted ebound pack runs on the same fixture: sha256 {'match' if h1 == h2 else 'DIFFER: ' + h1 + ' vs ' + h2}"}


def main() -> int:
    if not RESULTS.exists():
        print(f"missing {RESULTS}; run research/baselines/probes/run_capability_probes.py first", file=sys.stderr)
        return 1
    probe_results = json.loads(RESULTS.read_text(encoding="utf-8"))

    rows: list[dict] = []
    for cfg, r in probe_results.items():
        rows.extend(rows_from_probe(cfg, r))

    for cfg, cap, sup, ev, notes in DOC_ROWS:
        rows.append({"system_config": cfg, "capability": cap, "support": sup, "method": "DOC", "evidence_ref": ev, "notes": notes})

    # deterministic_output: reuse the real EXP-BASE-SIZE size/determinism pass
    # (research/baselines/run_baselines.py) instead of re-probing here. Read the
    # raw JSONL directly (not summary.json's string_metrics, which only records
    # the deduplicated set of digests per (candidate,item) -- with a single
    # repetition observed so far that set always has size 1, which would
    # misread as "confirmed deterministic" when it is really "not yet enough
    # repetitions to tell"). A pair only counts once repetitions >= 2.
    by_cfg = determinism_from_raw()
    if by_cfg:
        for cand, pairs in sorted(by_cfg.items()):
            decidable = {k: v for k, v in pairs.items() if v["reps"] >= 2}
            n, k = len(decidable), sum(1 for v in decidable.values() if v["deterministic"])
            insufficient = len(pairs) - len(decidable)
            if n == 0:
                sup, note = "N/A", (f"{insufficient} (candidate,item) pairs measured so far but none yet has "
                                     "2+ repetitions in this partial EXP-BASE-SIZE pass; rerun once it has more coverage")
            else:
                sup = "YES" if k == n else ("NO" if k == 0 else "PARTIAL")
                note = f"{k}/{n} (candidate,item) pairs with >=2 repetitions had identical artifact_tree_sha256"
                if insufficient:
                    note += f" ({insufficient} more pairs measured but with only 1 repetition so far, not counted either way)"
            rows.append({
                "system_config": cand, "capability": "deterministic_output", "support": sup, "method": "PROBE",
                "evidence_ref": "research/raw/EXP-BASE-SIZE/*.jsonl (grouped by candidate+item+rep)",
                "notes": note,
            })
    else:
        for cfg in FAMILY_OF:
            if cfg == "entrybound-ebound":
                continue
            rows.append({
                "system_config": cfg, "capability": "deterministic_output", "support": "N/A", "method": "PROBE",
                "evidence_ref": "research/raw/EXP-BASE-SIZE/ (no raw rows yet at capability-matrix build time)",
                "notes": "EXP-BASE-SIZE had not produced any raw rows yet; rerun build_capability_matrix.py once it has",
            })
    rows.append(entrybound_determinism_row())

    rows.sort(key=lambda r: (r["system_config"], r["capability"]))
    with open(OUT, "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=FIELDS)
        w.writeheader()
        w.writerows(rows)
    print(f"wrote {OUT} ({len(rows)} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
