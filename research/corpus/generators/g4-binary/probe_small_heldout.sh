#!/usr/bin/env bash
# Authoring aid: resolve candidate small held-out upstreams (names, sizes, digests only).
set -uo pipefail
echo "== ip7z/7zip latest release"
curl -fsSL https://api.github.com/repos/ip7z/7zip/releases/latest |
  jq -r '.tag_name, (.assets[] | select(.name | test("(src|extra|linux-x64|mac|lzma).*(7z|xz)$")) | [.name, .size, .digest] | @tsv)'
