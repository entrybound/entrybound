#!/usr/bin/env bash
# Smoke/determinism test for the g2-generated generators with tiny parameters (outside the corpus).
# Each generator runs twice into separate scratch directories; the logical_tree_sha256 must match.
#   wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/corpus/generators/g2-generated/smoke_test.sh [name...]
set -uo pipefail
G="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOOLS="$G/../../tools"
PY=/root/eb-research/venv/bin/python
S=/root/eb-research/scratch/g2smoke
umask 022
export LC_ALL=C.UTF-8 TZ=UTC SOURCE_DATE_EPOCH=1767225600 PYTHONHASHSEED=0 PYTHONDONTWRITEBYTECODE=1
mkdir -p "$S"
declare -A T
T[sft-sourcelike]="small_file_tree.py 11 {\"model\":\"sourcelike\",\"files\":400}"
T[sft-maildir]="small_file_tree.py 12 {\"model\":\"maildir\",\"files\":200,\"users\":3}"
T[sft-objstore]="small_file_tree.py 13 {\"model\":\"objstore\",\"files\":600}"
T[sparse-vm]="sparse_layouts.py 21 {\"model\":\"vm-raw\",\"disk_bytes\":268435456,\"fill_fraction\":0.05}"
T[sparse-db]="sparse_layouts.py 22 {\"model\":\"db-prealloc\",\"scale\":0.1}"
T[sparse-edge]="sparse_layouts.py 23 {\"model\":\"edge-cases\"}"
T[sparse-frag]="sparse_layouts.py 24 {\"model\":\"fragmented\",\"scale\":0.05}"
T[sparse-huge]="sparse_layouts.py 25 {\"model\":\"huge-hole\",\"scale\":0.02}"
T[coredump]="sparse_coredump.py 26 {\"cores\":1,\"heap_mib\":2,\"arenas\":1,\"arena_mib\":2,\"threads\":1,\"stack_mib\":1,\"wal_segments\":1,\"wal_segment_mib\":2}"
T[duptree]="duplicate_tree.py 31 {\"base_files\":120,\"exact_copies\":[\"copy-a\"],\"renamed_copies\":[\"copy-b\"],\"vendored_subtree\":{\"count\":4,\"min_files\":5},\"nested_copy\":true,\"renamed_files\":5}"
T[meta-zoo]="metadata_tree.py 41 {\"profile\":\"zoo\",\"scale\":0.1}"
T[meta-deep]="metadata_tree.py 42 {\"profile\":\"deep\",\"scale\":0.1}"
T[meta-matrix]="metadata_tree.py 43 {\"profile\":\"matrix\",\"scale\":0.1}"
T[farm-rsnap]="hardlink_farm.py 44 {\"model\":\"rsnapshot\",\"base_files\":150,\"snapshots\":4}"
T[farm-link]="hardlink_farm.py 45 {\"model\":\"linkstore\",\"packages\":6,\"versions_max\":3}"
T[csprng-files]="csprng_files.py 51 {\"mode\":\"files\",\"count\":5,\"max_mib\":1}"
T[sparsedup]="entropy_sparse_dup.py 52 {\"files\":4,\"file_mib\":4,\"blobs\":4,\"blob_kib\":[64,256,1024],\"max_copies\":3}"
T[straddle]="gear_straddle.py 53 {\"size_mib\":4}"
T[bombs-dense]="compressible_bombs.py 54 {\"mode\":\"dense\",\"scale\":0.05}"
T[bomb-variants]="bomb_variants.py 55 {\"scale\":0.02}"
T[aesctr]="aes_ctr_ciphertext.py 56 {\"plaintext_mib\":1,\"keystream_mib\":2,\"containers\":10}"
T[chacha-files]="chacha20_containers.py 57 {\"mode\":\"files\",\"count\":20,\"xm\":1024,\"sealed\":5}"
T[chacha-vol]="chacha20_containers.py 58 {\"mode\":\"volume\",\"volume_mib\":8,\"backup_mib\":4}"
names=("$@")
[[ ${#names[@]} -gt 0 ]] || names=($(printf '%s\n' "${!T[@]}" | sort))
fail=0
for n in "${names[@]}"; do
  read -r script seed params <<<"${T[$n]}"
  hashes=()
  for run in a b; do
    d="$S/$n.$run"; rm -rf "$d" "$d.scratch"; mkdir -p "$d" "$d.scratch"
    start=$(date +%s)
    ( cd "$d.scratch" && EB_SCRATCH="$d.scratch" $PY -B "$G/$script" --out "$d" --seed "$seed" --params "$params" ) > "$d.log" 2>&1
    rc=$?
    if [[ $rc -ne 0 ]]; then echo "FAIL $n ($run) rc=$rc"; tail -5 "$d.log"; fail=1; continue 2; fi
    h=$($PY -B "$TOOLS/fingerprint.py" "$d" | $PY -c 'import json,sys; f=json.load(sys.stdin); print(f["logical_tree_sha256"], f["bytes"], f["counts"]["files"])')
    hashes+=("$h")
    echo "  $n.$run $(( $(date +%s) - start ))s $h"
  done
  if [[ "${hashes[0]}" == "${hashes[1]}" ]]; then echo "OK $n"; else echo "NONDETERMINISTIC $n"; fail=1; fi
done
exit $fail
