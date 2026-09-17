#!/usr/bin/env python3
"""Run-step script (F20 structurally-adversarial archives, corpus group g2-generated): decode every
*.uu (uuencoded) file already extracted into the staging tree into its real binary sibling (same
path, .uu suffix stripped), then remove the .uu text form. libarchive's test suite stores many
binary archive fixtures (malformed/edge-case tar, cpio, zip, 7z files that would otherwise not
survive a text-oriented git diff/checkout cleanly) uuencoded; this step is what "with *.uu decoded"
means in this item's source-of-truth entry.

Uses `binascii.a2b_uu` directly rather than the stdlib `uu` module (deprecated since 3.11, removed
in 3.13) so this keeps working on newer interpreters.

Params (EB_PARAMS_JSON): {} (walks the whole staging tree; no parameters needed).
"""

import binascii
import os
from pathlib import Path


def decode_uu(src: Path, dst: Path) -> None:
    lines = src.read_bytes().splitlines()
    out = bytearray()
    in_body = False
    for line in lines:
        if not in_body:
            if line.startswith(b"begin "):
                in_body = True
            continue
        if line in (b"end", b"`\nend", b""):
            if line == b"end":
                break
            continue
        if line == b"`":
            continue
        try:
            out += binascii.a2b_uu(line)
        except binascii.Error:
            # Some encoders pad the last short line differently; retry with an explicit
            # length byte recomputed from the encoded character count (a2b_uu is picky).
            n = (((line[0] - 0x20) & 0x3F) * 8 + 5) // 6
            out += binascii.a2b_uu(line[: n + 1].ljust(len(line), b" "))
    dst.parent.mkdir(parents=True, exist_ok=True)
    dst.write_bytes(bytes(out))


def main() -> None:
    out = Path(os.environ["EB_OUT"])
    n = 0
    for src in sorted(out.rglob("*.uu")):
        dst = src.with_suffix("")
        decode_uu(src, dst)
        src.unlink()
        n += 1
    print(f"decode_uu_files: decoded {n} .uu file(s)")


if __name__ == "__main__":
    main()
