#!/usr/bin/env bash
# Capture the WSL Ubuntu environment fingerprint.
#
# Invoke from Windows exactly the way research jobs are invoked:
#   wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/tools/env/capture_wsl.sh
#
# Deliberately does NOT source ~/.profile or ~/.cargo/env: the fingerprint must
# describe the non-login shell that research scripts actually get (in which a
# bare `rustc` resolves to /usr/bin/rustc, not the rustup proxy).
#
# Requires the windows-host fingerprint to be captured first (its platform_id
# is embedded as stable.platform.parent). Idempotent: an unchanged fingerprint
# is not rewritten.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
OUT="$REPO/research/environment"
PY=/usr/bin/python3

"$PY" "$HERE/capture_env.py" capture \
  --name wsl-ubuntu \
  --repo "$REPO" \
  --work-dir "$REPO" \
  --work-dir /root/eb-research \
  --work-dir /tmp \
  --parent-from-index windows-host \
  --out-dir "$OUT"
