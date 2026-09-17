"""Helpers for the research-internals byte-identity check.

Shared by internals-identity-check.sh (WSL) and internals-identity-check.ps1
(Windows) so both hosts generate the same fixed tree, run the same
additive-only source audit, and splice their results into one report.

Subcommands (Python >= 3.10, standard library only):
  tree <dest>                         generate the fixed input tree (dest must not exist)
  tree-digest <dir>                   print "<files> <bytes> <sha256>" for a tree
  additive-check <repo> [--base REV]  prove every production change is a
                                      removable cfg(feature = "research-internals") block
  test-summary <log>                  totals, failing tests, and compile errors of a cargo test log
  clippy-summary <log>                list compiler and clippy diagnostics as "level lint path:line"
  write-section <report> <name> <section.md>
                                      replace one host section of the report
"""

import argparse
import gzip
import hashlib
import io
import math
import os
import pathlib
import re
import subprocess
import sys

FIXED_MTIME = 1_700_000_000  # 2023-11-14T22:13:20Z
FEATURE = "research-internals"
CFG_LINE = f'#[cfg(feature = "{FEATURE}")]'
RUST_FILES = [
    "crates/entrybound/src/lib.rs",
    "crates/entrybound/src/codec.rs",
    "crates/entrybound/src/ecf.rs",
    "crates/entrybound/src/jpeg_reconstruction.rs",
    "crates/entrybound/src/planner.rs",
    "crates/entrybound/src/reconstruction.rs",
    "crates/entrybound/src/similarity.rs",
    "crates/entrybound/src/transform.rs",
]
CARGO_TOML = "crates/entrybound/Cargo.toml"
RESEARCH_ONLY_FILES = {"crates/entrybound/tests/research_internals.rs"}


# ---------------------------------------------------------------------------
# Deterministic content generators
# ---------------------------------------------------------------------------


def xorshift_bytes(length, seed):
    state = seed & 0xFFFF_FFFF_FFFF_FFFF
    out = bytearray(length)
    for index in range(length):
        state ^= (state << 13) & 0xFFFF_FFFF_FFFF_FFFF
        state ^= state >> 7
        state ^= (state << 17) & 0xFFFF_FFFF_FFFF_FFFF
        out[index] = (state >> 24) & 0xFF
    return bytes(out)


def sha_stream(length, label):
    out = bytearray()
    counter = 0
    while len(out) < length:
        out += hashlib.sha256(f"{label}/{counter}".encode()).digest()
        counter += 1
    return bytes(out[:length])


def log_text(lines, seed):
    words = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"]
    out = io.StringIO()
    for index in range(lines):
        value = (index * 7919 + seed) % 50_000
        status = "open" if index % 3 == 0 else "closed"
        out.write(
            f"2026-09-{1 + index % 28:02d}T{index % 24:02d}:{index % 60:02d}:{(index * 7) % 60:02d}Z "
            f"{words[index % len(words)]} account={value:05d} status={status} latency_ms={(index * 13) % 997}\n"
        )
    return out.getvalue().encode()


def csv_rows(rows):
    return "".join(
        f"row={index:06d};value={index * 17:08x};category={index % 31}\n" for index in range(rows)
    ).encode()


def deterministic_gzip(payload):
    buffer = io.BytesIO()
    with gzip.GzipFile(filename="", mode="wb", fileobj=buffer, compresslevel=6, mtime=0) as handle:
        handle.write(payload)
    return buffer.getvalue()


# ---------------------------------------------------------------------------
# Minimal baseline JPEG encoder (YCbCr 4:4:4, ITU-T T.81 Annex K tables)
# ---------------------------------------------------------------------------

ZIGZAG = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5,
    12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13, 6, 7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
]
LUMA_QUANT = [
    16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55,
    14, 13, 16, 24, 40, 57, 69, 56, 14, 17, 22, 29, 51, 87, 80, 62,
    18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113, 92,
    49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
]
CHROMA_QUANT = [
    17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99,
    24, 26, 56, 99, 99, 99, 99, 99, 47, 66, 99, 99, 99, 99, 99, 99,
] + [99] * 32
DC_LUMA_BITS = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0]
DC_CHROMA_BITS = [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0]
DC_VALUES = list(range(12))
AC_LUMA_BITS = [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7D]
# ITU-T T.81 Tables K.5 and K.6 (typical AC luminance and chrominance codes).
AC_LUMA_VALUES = bytes([
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
    0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0,
    0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28,
    0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
    0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
    0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7,
    0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5,
    0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2,
    0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8,
    0xF9, 0xFA,
])
AC_CHROMA_BITS = [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77]
AC_CHROMA_VALUES = bytes([
    0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71,
    0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xA1, 0xB1, 0xC1, 0x09, 0x23, 0x33, 0x52, 0xF0,
    0x15, 0x62, 0x72, 0xD1, 0x0A, 0x16, 0x24, 0x34, 0xE1, 0x25, 0xF1, 0x17, 0x18, 0x19, 0x1A, 0x26,
    0x27, 0x28, 0x29, 0x2A, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
    0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
    0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
    0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5,
    0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3,
    0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA,
    0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8,
    0xF9, 0xFA,
])


def scaled_quant(base, quality):
    scale = 5000 // quality if quality < 50 else 200 - 2 * quality
    return [min(255, max(1, (value * scale + 50) // 100)) for value in base]


def huffman_codes(bits, values):
    if sum(bits) != len(values) or len(set(values)) != len(values):
        raise ValueError("Huffman table size mismatch")
    if len(values) == 162:
        expected = {0x00, 0xF0} | {(run << 4) | size for run in range(16) for size in range(1, 11)}
        if set(values) != expected:
            raise ValueError("AC Huffman table does not cover every (run, size) symbol")
    codes = {}
    code = 0
    index = 0
    for length in range(1, 17):
        for _ in range(bits[length - 1]):
            codes[values[index]] = (code, length)
            code += 1
            index += 1
        code <<= 1
    return codes


class BitWriter:
    def __init__(self):
        self.out = bytearray()
        self.accumulator = 0
        self.count = 0

    def write(self, value, length):
        self.accumulator = (self.accumulator << length) | (value & ((1 << length) - 1))
        self.count += length
        while self.count >= 8:
            self.count -= 8
            byte = (self.accumulator >> self.count) & 0xFF
            self.out.append(byte)
            if byte == 0xFF:
                self.out.append(0x00)
        self.accumulator &= (1 << self.count) - 1

    def flush(self):
        if self.count:
            self.write((1 << (8 - self.count)) - 1, 8 - self.count)


COSINES = [
    [
        (math.sqrt(0.5) if u == 0 else 1.0) * 0.5 * math.cos((2 * x + 1) * u * math.pi / 16)
        for x in range(8)
    ]
    for u in range(8)
]


def forward_dct(block):
    rows = [[sum(row[x] * COSINES[u][x] for x in range(8)) for u in range(8)] for row in block]
    return [
        sum(rows[y][u] * COSINES[v][y] for y in range(8)) for v in range(8) for u in range(8)
    ]


def encode_block(writer, block, quant, previous_dc, dc_codes, ac_codes):
    coefficients = forward_dct(block)
    quantized = [int(math.floor(coefficients[i] / quant[i] + 0.5)) for i in range(64)]
    zigzag = [quantized[index] for index in ZIGZAG]

    def emit(value, codes, symbol_high):
        magnitude = abs(value).bit_length()
        code, length = codes[(symbol_high << 4) | magnitude]
        writer.write(code, length)
        if magnitude:
            writer.write(value if value > 0 else value + (1 << magnitude) - 1, magnitude)

    difference = zigzag[0] - previous_dc
    magnitude = abs(difference).bit_length()
    code, length = dc_codes[magnitude]
    writer.write(code, length)
    if magnitude:
        writer.write(
            difference if difference > 0 else difference + (1 << magnitude) - 1, magnitude
        )
    run = 0
    for value in zigzag[1:]:
        if value == 0:
            run += 1
            continue
        while run > 15:
            code, length = ac_codes[0xF0]
            writer.write(code, length)
            run -= 16
        emit(value, ac_codes, run)
        run = 0
    if run:
        code, length = ac_codes[0x00]
        writer.write(code, length)
    return zigzag[0]


def baseline_jpeg(width, height, rgb, quality):
    if width % 8 or height % 8:
        raise ValueError("dimensions must be multiples of 8")
    luma_quant = scaled_quant(LUMA_QUANT, quality)
    chroma_quant = scaled_quant(CHROMA_QUANT, quality)
    dc_luma = huffman_codes(DC_LUMA_BITS, DC_VALUES)
    dc_chroma = huffman_codes(DC_CHROMA_BITS, DC_VALUES)
    ac_luma = huffman_codes(AC_LUMA_BITS, AC_LUMA_VALUES)
    ac_chroma = huffman_codes(AC_CHROMA_BITS, AC_CHROMA_VALUES)

    planes = ([], [], [])
    for index in range(width * height):
        r, g, b = rgb[3 * index], rgb[3 * index + 1], rgb[3 * index + 2]
        planes[0].append(0.299 * r + 0.587 * g + 0.114 * b - 128)
        planes[1].append(-0.168736 * r - 0.331264 * g + 0.5 * b)
        planes[2].append(0.5 * r - 0.418688 * g - 0.081312 * b)

    writer = BitWriter()
    previous = [0, 0, 0]
    tables = [(luma_quant, dc_luma, ac_luma), (chroma_quant, dc_chroma, ac_chroma), (chroma_quant, dc_chroma, ac_chroma)]
    for block_y in range(0, height, 8):
        for block_x in range(0, width, 8):
            for component in range(3):
                plane = planes[component]
                block = [
                    [plane[(block_y + y) * width + block_x + x] for x in range(8)] for y in range(8)
                ]
                quant, dc_codes, ac_codes = tables[component]
                previous[component] = encode_block(
                    writer, block, quant, previous[component], dc_codes, ac_codes
                )
    writer.flush()

    def segment(marker, payload):
        return bytes([0xFF, marker]) + (len(payload) + 2).to_bytes(2, "big") + payload

    out = bytearray(b"\xff\xd8")
    out += segment(0xE0, b"JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00")
    out += segment(0xDB, bytes([0]) + bytes(luma_quant[i] for i in ZIGZAG))
    out += segment(0xDB, bytes([1]) + bytes(chroma_quant[i] for i in ZIGZAG))
    out += segment(
        0xC0,
        bytes([8]) + height.to_bytes(2, "big") + width.to_bytes(2, "big")
        + bytes([3, 1, 0x11, 0, 2, 0x11, 1, 3, 0x11, 1]),
    )
    for table_class_id, bits, values in [
        (0x00, DC_LUMA_BITS, DC_VALUES),
        (0x10, AC_LUMA_BITS, AC_LUMA_VALUES),
        (0x01, DC_CHROMA_BITS, DC_VALUES),
        (0x11, AC_CHROMA_BITS, AC_CHROMA_VALUES),
    ]:
        out += segment(0xC4, bytes([table_class_id]) + bytes(bits) + bytes(values))
    out += segment(0xDA, bytes([3, 1, 0x00, 2, 0x11, 3, 0x11, 0, 63, 0]))
    out += writer.out
    out += b"\xff\xd9"
    return bytes(out)


def photo_pixels(width, height, seed):
    noise = xorshift_bytes(width * height * 3, seed)
    out = bytearray(width * height * 3)
    for y in range(height):
        for x in range(width):
            index = 3 * (y * width + x)
            base = (
                (x * 255) // (width - 1),
                (y * 255) // (height - 1),
                ((x + y) * 255) // (width + height - 2),
            )
            for channel in range(3):
                value = base[channel] + (noise[index + channel] % 49) - 24
                out[index + channel] = min(255, max(0, value))
    return bytes(out)


# ---------------------------------------------------------------------------
# Tree
# ---------------------------------------------------------------------------


def tree_files():
    files = {
        "text/service.log": log_text(4_000, 11),
        "text/notes.md": b"# Fixed research-internals identity tree\n\n" + log_text(40, 3),
        "text/empty.txt": b"",
        "binary/counters.bin": b"".join((value * 3).to_bytes(4, "little") for value in range(65_536)),
        "binary/zeros.bin": bytes(512 * 1024),
        "binary/random.bin": sha_stream(256 * 1024, "entrybound/research-internals/random"),
        "compressed/rows.csv.gz": deterministic_gzip(csv_rows(12_000)),
        "media/photo.jpg": baseline_jpeg(256, 160, photo_pixels(256, 160, 0x6A09_E667_F3BC_C909), 85),
    }
    for index in range(12):
        header = f"record={index:04d}\nkind=entrybound-research-internals\n"
        rows = "".join(
            f"row={row:04d},field=shared-value,status=active,owner=ops\n" for row in range(160)
        )
        files[f"similar/records/record-{index:02d}.txt"] = f"{header}{rows}tail={index:04d}\n".encode()
    base = xorshift_bytes(64 * 1024, 0xBB67_AE85_84CA_A73B)
    for index in range(24):
        sample = bytearray(base)
        first = 128 + (index * 977) % (len(sample) - 512)
        for offset in range(256):
            sample[first + offset] = (index * 31 + offset) & 0xFF
        files[f"similar/samples/sample-{index:03d}.bin"] = bytes(sample)
    return files


def generate_tree(dest):
    root = pathlib.Path(dest)
    if root.exists():
        raise SystemExit(f"{root} already exists; remove it first")
    for relative, payload in tree_files().items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(payload)
    (root / "empty-dir").mkdir()
    for directory, _dirs, names in sorted(os.walk(root), key=lambda item: -len(item[0])):
        for name in names:
            os.utime(pathlib.Path(directory) / name, (FIXED_MTIME, FIXED_MTIME))
        os.utime(directory, (FIXED_MTIME, FIXED_MTIME))
    print(tree_digest(root))


def tree_digest(directory):
    root = pathlib.Path(directory)
    digest = hashlib.sha256()
    count = 0
    total = 0
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        payload = path.read_bytes()
        relative = path.relative_to(root).as_posix()
        digest.update(f"{relative}\0{len(payload)}\0{hashlib.sha256(payload).hexdigest()}\n".encode())
        count += 1
        total += len(payload)
    return f"{count} {total} {digest.hexdigest()}"


# ---------------------------------------------------------------------------
# Additive-only source audit
# ---------------------------------------------------------------------------


def git(repo, *args):
    env = dict(os.environ, GIT_OPTIONAL_LOCKS="0")
    result = subprocess.run(
        ["git", "-c", "core.autocrlf=true", "-c", f"safe.directory={repo}", "-C", repo, *args],
        capture_output=True,
        env=env,
    )
    if result.returncode != 0:
        raise SystemExit(f"git {' '.join(args)} failed: {result.stderr.decode(errors='replace')}")
    return result.stdout


def normalized(data):
    return data.replace(b"\r\n", b"\n").decode("utf-8")


def strip_rust_research_blocks(text):
    lines = text.split("\n")
    out = []
    index = 0
    removed = 0
    while index < len(lines):
        is_block = (
            lines[index].strip() == CFG_LINE
            and index + 1 < len(lines)
            and re.fullmatch(r"(\s*)pub mod research \{", lines[index + 1])
        )
        if not is_block:
            out.append(lines[index])
            index += 1
            continue
        indent = re.fullmatch(r"(\s*)pub mod research \{", lines[index + 1]).group(1)
        while out and out[-1].strip().startswith("///"):
            out.pop()
        if out and out[-1] == "":
            out.pop()
        end = index + 2
        while end < len(lines) and lines[end] != f"{indent}}}":
            end += 1
        if end == len(lines):
            raise SystemExit("unterminated research module")
        index = end + 1
        removed += 1
    return "\n".join(out), removed


def strip_cargo_feature(text):
    lines = text.split("\n")
    out = []
    index = 0
    removed = 0
    while index < len(lines):
        if lines[index] == "[features]":
            end = index + 1
            while end < len(lines) and lines[end].startswith("#"):
                end += 1
            if end < len(lines) and lines[end] == f"{FEATURE} = []" and end + 1 < len(lines) and lines[end + 1] == "":
                index = end + 2
                removed += 1
                continue
        out.append(lines[index])
        index += 1
    return "\n".join(out), removed


def additive_check(repo, base):
    repo = str(pathlib.Path(repo).resolve())
    if base is None:
        intro = git(repo, "log", "-1", "--format=%H", f"-G{FEATURE} = \\[\\]", "--", CARGO_TOML).decode().strip()
        committed_feature = intro and FEATURE in normalized(git(repo, "show", f"HEAD:{CARGO_TOML}"))
        base = f"{intro}^" if committed_feature else "HEAD"
    base_sha = git(repo, "rev-parse", base).decode().strip()
    print(f"base: {base} ({base_sha})")
    failures = []
    for relative in [CARGO_TOML, *RUST_FILES]:
        current = normalized((pathlib.Path(repo) / relative).read_bytes())
        original = normalized(git(repo, "show", f"{base_sha}:{relative}"))
        if relative == CARGO_TOML:
            stripped, removed = strip_cargo_feature(current)
        else:
            stripped, removed = strip_rust_research_blocks(current)
        identical = stripped == original
        leftover = FEATURE in stripped
        status = "PASS" if identical and removed == 1 and not leftover else "FAIL"
        if status == "FAIL":
            failures.append(relative)
        print(f"{status} {relative}: removed {removed} gated block(s); remainder identical to base: {identical}")
    changed = git(repo, "diff", "--name-only", base_sha, "--", "crates").decode().split()
    untracked = git(repo, "ls-files", "--others", "--exclude-standard", "--", "crates").decode().split()
    audited = {CARGO_TOML, *RUST_FILES}
    for relative in sorted(set(changed) | set(untracked)):
        if relative in audited:
            continue
        if relative in RESEARCH_ONLY_FILES:
            text = normalized((pathlib.Path(repo) / relative).read_bytes())
            gated = text.startswith(f'#![cfg(feature = "{FEATURE}")]')
            print(f"{'PASS' if gated else 'FAIL'} {relative}: research-only test gated at crate level: {gated}")
            if not gated:
                failures.append(relative)
        else:
            print(f"FAIL {relative}: production path changed outside the audited research blocks")
            failures.append(relative)
    print("RESULT " + ("PASS" if not failures else "FAIL " + " ".join(failures)))
    return 0 if not failures else 1


# ---------------------------------------------------------------------------
# Log summaries and report splicing
# ---------------------------------------------------------------------------


def test_summary(log):
    """Totals, the research_internals result, failing tests, and compile errors."""
    text = pathlib.Path(log).read_text(encoding="utf-8", errors="replace")
    totals = [0, 0, 0]
    results = 0
    label = None
    research = []
    failed = set()
    pattern = re.compile(
        r"^\s*(?:Running (?:unittests )?(\S+) \(.*?([^/\\]+?)-[0-9a-f]+(?:\.exe)?\)"
        r"|Doc-tests (\S+)"
        r"|test (.+) \.\.\. FAILED$"
        r"|test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored)"
    )
    for line in text.splitlines():
        match = pattern.match(line)
        if not match:
            continue
        if match.group(1):
            label = f"{match.group(1).replace(chr(92), '/')}[{match.group(2)}]"
        elif match.group(3):
            label = f"doc-tests[{match.group(3)}]"
        elif match.group(4):
            failed.add(f"FAILED {label}::{match.group(4)}")
        else:
            results += 1
            for slot in range(3):
                totals[slot] += int(match.group(slot + 6))
            if label and "research_internals" in label:
                research.append(
                    f"research_internals {label}: {match.group(5)}, {match.group(6)} passed, "
                    f"{match.group(7)} failed, {match.group(8)} ignored"
                )
    compile_errors = bool(re.search(r"(?m)^error: could not compile", text))
    print(
        f"{results} test result lines (test binaries and doc-test runs): {totals[0]} passed, "
        f"{totals[1]} failed, {totals[2]} ignored"
    )
    for line in research:
        print(line)
    for line in sorted(failed):
        print(line)
    print(f"compile errors: {'yes' if compile_errors else 'no'}")


def clippy_summary(log):
    text = pathlib.Path(log).read_text(encoding="utf-8", errors="replace")
    findings = set()
    for block in re.split(r"(?m)^(?=(?:error|warning)(?:\[\w+\])?: )", text):
        head = re.match(r"(error|warning)(?:\[(\w+)\])?: ([^\n]*)", block)
        if not head:
            continue
        message = head.group(3)
        if message.startswith(("could not compile", "build failed", "aborting due to")) or re.search(
            r"\d+ (?:warnings?|errors?) emitted|generated \d+ warnings?", message
        ):
            continue
        location = re.search(r"-->\s+(\S+?):(\d+):\d+", block)
        lint = re.search(r"rust-clippy/\S*#(\w+)", block)
        name = f"clippy::{lint.group(1)}" if lint else (head.group(2) or message)
        where = f"{location.group(1).replace(chr(92), '/')}:{location.group(2)}" if location else "(no location)"
        findings.add(f"{head.group(1)} {name} {where}")
    for finding in sorted(findings):
        print(finding)
    if not findings:
        print("(no diagnostics)")


HEADER = """# research-internals byte-identity check

Generated by `research/harness/internals-identity-check.sh` (WSL, primary) and
`research/harness/internals-identity-check.ps1` (Windows). Each script rewrites
only its own host section. Do not edit by hand; re-run the scripts instead.
See `research/harness/research-internals.md` for what the feature exposes.
"""
ORDER = ["wsl", "windows"]


def write_section(report, name, section_file):
    path = pathlib.Path(report)
    body = pathlib.Path(section_file).read_text(encoding="utf-8").strip("\n")
    sections = {}
    if path.exists():
        text = path.read_text(encoding="utf-8")
        for match in re.finditer(r"<!-- section:(\w+):begin -->\n(.*?)\n<!-- section:\1:end -->", text, re.S):
            sections[match.group(1)] = match.group(2)
    sections[name] = body
    parts = [HEADER]
    for key in ORDER + sorted(set(sections) - set(ORDER)):
        if key in sections:
            parts.append(f"<!-- section:{key}:begin -->\n{sections[key]}\n<!-- section:{key}:end -->\n")
    path.write_text("\n".join(parts), encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("tree").add_argument("dest")
    sub.add_parser("tree-digest").add_argument("dir")
    check = sub.add_parser("additive-check")
    check.add_argument("repo")
    check.add_argument("--base")
    sub.add_parser("test-summary").add_argument("log")
    sub.add_parser("clippy-summary").add_argument("log")
    section = sub.add_parser("write-section")
    section.add_argument("report")
    section.add_argument("name")
    section.add_argument("section")
    args = parser.parse_args()
    if args.command == "tree":
        generate_tree(args.dest)
    elif args.command == "tree-digest":
        print(tree_digest(args.dir))
    elif args.command == "additive-check":
        sys.exit(additive_check(args.repo, args.base))
    elif args.command == "test-summary":
        test_summary(args.log)
    elif args.command == "clippy-summary":
        clippy_summary(args.log)
    elif args.command == "write-section":
        write_section(args.report, args.name, args.section)


if __name__ == "__main__":
    main()
