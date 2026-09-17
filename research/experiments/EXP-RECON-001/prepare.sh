#!/usr/bin/env bash
# EXP-RECON-001 build preparation (WSL2 Ubuntu, root). Run once before `ebr run`.
#
# Builds, from a fresh clone at the checked-out commit (which must descend from the
# pre-registration commit of this experiment), exactly the binaries the spec invokes:
#   ebound            production workspace, --locked, default features (harness use constraint 2)
#   ebr-pack, ebr-unpack, ebr-inspect-bytes   research harness workspace, --locked
# then enforces harness review use constraint 1 (lock parity + CLI parity must PASS),
# installs the binaries into /root/eb-research/tools/EXP-RECON-001/bin and records
# their SHA-256 plus toolchain identity in /root/eb-research/results/EXP-RECON-001/build.json.
#
# Usage: wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-RECON-001/prepare.sh
# Preconditions (orchestrator): disk_guard.py passes; no held-out path is involved.
set -euo pipefail

REPO=/mnt/d/Projects/entrybound/entrybound
EXP=EXP-RECON-001
CLONE=/root/eb-research/src/$EXP
CLI_TARGET=/root/eb-research/target/$EXP-cli
HARNESS_TARGET=/root/eb-research/target/$EXP-harness
BIN=/root/eb-research/tools/$EXP/bin
RES=/root/eb-research/results/$EXP
CARGO=(/root/.cargo/bin/cargo +1.98.1)

COMMIT="$(git -C "$REPO" rev-parse HEAD)"
if [ -n "$(git -C "$REPO" status --porcelain -- crates research/harness Cargo.toml Cargo.lock "research/experiments/$EXP")" ]; then
  echo "refusing: working tree is dirty in measured paths (decision-method §5.8(d))" >&2
  exit 3
fi

rm -rf "$CLONE"
git clone --quiet --no-local "$REPO" "$CLONE"
git -C "$CLONE" checkout --quiet --detach "$COMMIT"

(cd "$CLONE" && CARGO_TARGET_DIR="$CLI_TARGET" "${CARGO[@]}" build --locked --release -p entrybound-cli --bin ebound >&2)
(cd "$CLONE/research/harness" && CARGO_TARGET_DIR="$HARNESS_TARGET" bash build.sh build --locked --release -p ebr-pack >&2)

mkdir -p "$BIN" "$RES"
python3 -B "$CLONE/research/harness/lock_parity.py" | tee "$RES/lock_parity.txt" >&2
grep -q "RESULT PASS" "$RES/lock_parity.txt"
HARNESS_TARGET="$HARNESS_TARGET" CLI_TARGET="$CLI_TARGET" bash "$CLONE/research/harness/harness-cli-parity.sh" | tee "$RES/cli_parity.txt" >&2
grep -q "RESULT PASS" "$RES/cli_parity.txt"

install -m 0755 "$CLI_TARGET/release/ebound" "$BIN/ebound"
for b in ebr-pack ebr-unpack ebr-inspect-bytes; do install -m 0755 "$HARNESS_TARGET/release/$b" "$BIN/$b"; done

python3 - "$BIN" "$RES/build.json" "$COMMIT" <<'EOF'
import hashlib, json, os, subprocess, sys
bin_dir, out, commit = sys.argv[1:4]
def sha(p):
    h = hashlib.sha256()
    with open(p, "rb") as fh:
        for b in iter(lambda: fh.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()
rustc = subprocess.run(["/root/.cargo/bin/rustc", "+1.98.1", "-Vv"], capture_output=True, text=True).stdout
json.dump({"experiment_id": "EXP-RECON-001", "commit": commit, "rustc": rustc,
           "binaries": {n: sha(os.path.join(bin_dir, n)) for n in sorted(os.listdir(bin_dir))}},
          open(out, "w"), indent=2, sort_keys=True)
EOF
echo "prepared $EXP at $COMMIT"
