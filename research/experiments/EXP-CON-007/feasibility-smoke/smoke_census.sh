#!/usr/bin/env bash
# NON-DECISION-GRADE feasibility smoke for EXP-CON-007/census_row.sh on a tiny generated tree
# (pack plain INDEXED, pack plain STREAM, pack encrypted, convert strict, convert preserve).
set -euo pipefail
S=/root/eb-research/scratch/con-design-smoke2
rm -rf "$S" && mkdir -p "$S/tree" "$S/c1" "$S/c2" "$S/c3" "$S/c4" "$S/c5"
python3 - <<'EOF'
import os, random, zipfile
random.seed(7)
root = '/root/eb-research/scratch/con-design-smoke2/tree'
for d in range(4):
    p = os.path.join(root, f"d{d}")
    os.makedirs(p, exist_ok=True)
    for f in range(25):
        open(os.path.join(p, f"f{f}.txt"), 'wb').write(os.urandom(random.randint(0, 4096)))
with zipfile.ZipFile(os.path.join(root, 'd0', 'bundle.zip'), 'w', zipfile.ZIP_DEFLATED) as z:
    for f in range(5):
        z.writestr(f"member{f}.txt", ("member %d " % f) * 200)
EOF
R=/mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-CON-007/census_row.sh
E=/root/eb-research/target/baseline/release/ebound
IB=/root/eb-research/target/harness/release/ebr-inspect-bytes
bash "$R" --mode pack --item-path "$S/tree" --out "$S/c1/a.eb" --profile balanced --layout indexed --encrypt none --ebound "$E" --inspect-bytes "$IB" | tail -n 1
bash "$R" --mode pack --item-path "$S/tree" --out "$S/c2/a.eb" --profile fast --layout stream --encrypt none --ebound "$E" --inspect-bytes "$IB" | tail -n 1
bash "$R" --mode pack --item-path "$S/tree" --out "$S/c3/a.eb" --profile balanced --layout indexed --encrypt password --ebound "$E" --inspect-bytes "$IB" | tail -n 1
bash "$R" --mode convert-strict --item-path "$S/tree" --out "$S/c4/a.eb" --profile balanced --layout indexed --ebound "$E" --inspect-bytes "$IB" | tail -n 1
bash "$R" --mode convert-preserve --item-path "$S/tree" --out "$S/c5/a.eb" --profile balanced --layout indexed --ebound "$E" --inspect-bytes "$IB" | tail -n 1
rm -rf "$S"
