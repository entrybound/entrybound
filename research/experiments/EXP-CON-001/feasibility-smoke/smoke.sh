#!/usr/bin/env bash
# NON-DECISION-GRADE feasibility smoke for Phase C-design (container domain).
# Generates a tiny synthetic many-small-file tree, packs it with the dev-HEAD
# baseline ebound build, and records what inspect/list expose. Not a result.
set -euo pipefail
S=/root/eb-research/scratch/con-design-smoke
rm -rf "$S" && mkdir -p "$S/tree"
cd "$S"
python3 - <<'EOF'
import os, random
random.seed(20260917)
root = '/root/eb-research/scratch/con-design-smoke/tree'
n = 0
for d in range(40):
    dp = os.path.join(root, f"pkg{d:03d}", "src", "module")
    os.makedirs(dp, exist_ok=True)
    for f in range(500):
        with open(os.path.join(dp, f"file_{f:05d}_{random.getrandbits(32):08x}.rs"), 'wb') as fh:
            fh.write(os.urandom(random.randint(0, 64)))
        n += 1
print("generated_files", n)
EOF
E=/root/eb-research/target/baseline/release/ebound
/usr/bin/time -f "pack maxrss_kib=%M wall_s=%e" "$E" pack tree out.eb --profile balanced > pack.out 2> pack.time || { echo "pack failed"; cat pack.time; }
tail -n 3 pack.time
stat -c "archive_bytes=%s" out.eb
/usr/bin/time -f "inspect maxrss_kib=%M wall_s=%e" "$E" inspect out.eb --json > insp.json 2> insp.time; tail -n 1 insp.time
/usr/bin/time -f "list maxrss_kib=%M wall_s=%e" "$E" list out.eb > list.out 2> list.time; tail -n 1 list.time
python3 - <<'EOF'
import json
d = json.load(open('/root/eb-research/scratch/con-design-smoke/insp.json'))
print("top_keys", sorted(d.keys()))
def walk(o, p=''):
    if isinstance(o, dict):
        for k, v in o.items():
            key = p + k
            if isinstance(v, dict):
                walk(v, key + '.')
            elif not isinstance(v, list):
                if any(s in k for s in ('feature', 'section', 'manifest', 'index', 'bytes', 'count', 'layout', 'budget', 'window', 'working')):
                    print(key, v)
walk(d)
EOF
/root/eb-research/target/harness/release/ebr-inspect-bytes --archive-path out.eb --experiment-id EXP-CON-SMOKE --env-name wsl-ubuntu --layout indexed > ib.json 2>/dev/null || echo "inspect-bytes failed"
python3 - <<'EOF'
import json
row = json.loads(open('/root/eb-research/scratch/con-design-smoke/ib.json').read().splitlines()[0])
p = row['payload']
def walk(o, pre=''):
    if isinstance(o, dict):
        for k, v in o.items():
            if isinstance(v, dict):
                walk(v, pre + k + '.')
            elif isinstance(v, (int, float)) and 'bytes' in k:
                print('ib.' + pre + k, v)
walk(p)
EOF
file out.eb || true
ls /usr/share/misc/magic.mgc /usr/share/file/magic.mgc 2>/dev/null || true
dpkg-query -W -f='${Package} ${Version}\n' libmagic1t64 file shared-mime-info 2>/dev/null || dpkg -l | awk '/ (file|libmagic1t64|shared-mime-info) /{print $2, $3}'
head -c 8 out.eb | od -An -tx1
rm -rf "$S/tree"
