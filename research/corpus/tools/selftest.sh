#!/usr/bin/env bash
# End-to-end self-test of the corpus tools in a sandbox.  Never touches research/corpus
# or the real corpus: EB_CORPUS_DIR / EB_DATA_ROOT point under /root/eb-research/selftest.
#
#   wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/corpus/tools/run.sh selftest
#
# Rerunnable: the sandbox is recreated on every run.  Exits non-zero on the first failed check.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export HERE
PY=/root/eb-research/venv/bin/python
ST=/root/eb-research/selftest
case "$ST" in /root/eb-research/selftest) ;; *) echo "bad sandbox path"; exit 2 ;; esac
rm -rf "$ST"
mkdir -p "$ST/corpus/sources" "$ST/upstream"
export EB_CORPUS_DIR="$ST/corpus" EB_DATA_ROOT="$ST/data" EB_SELFTEST_ALLOW_FILE_URLS=1 PYTHONDONTWRITEBYTECODE=1
mkdir -p "$ST/data/heldout" && chmod 700 "$ST/data/heldout"

T() { "$PY" -B "$HERE/$1.py" "${@:2}"; }
pass() { echo "PASS: $*"; }
fail() { echo "SELFTEST FAIL: $*"; exit 1; }
expect_rc() {  # expect_rc <code> <grep-pattern|-> cmd...
  local want=$1 pat=$2; shift 2
  set +e; "$@" > "$ST/last.out" 2>&1; local rc=$?; set -e
  if [[ $rc -ne $want ]]; then cat "$ST/last.out"; fail "expected exit $want, got $rc: $*"; fi
  if [[ "$pat" != "-" ]] && ! grep -q -- "$pat" "$ST/last.out"; then cat "$ST/last.out"; fail "output lacks '$pat': $*"; fi
}
jget() { "$PY" -c 'import json,sys; d=json.load(open(sys.argv[1]));
for k in sys.argv[2].split("."): d=d[k]
print(d)' "$1" "$2"; }

# ---- upstream fixtures: a local "download" blob and a local git repo ----------------------
printf 'upstream blob v1\n' > "$ST/upstream/blob.txt"
tar -C "$ST/upstream" -cf "$ST/upstream/bundle.tar" blob.txt
git init -q "$ST/upstream/repo"
( cd "$ST/upstream/repo" && mkdir -p sub && printf 'int main(void){return 0;}\n' > sub/main.c && printf 'readme\n' > README \
  && git add -A && GIT_AUTHOR_DATE='2026-01-01T00:00:00Z' GIT_COMMITTER_DATE='2026-01-01T00:00:00Z' \
     git -c user.name=selftest -c user.email=selftest@invalid commit -qm init )
COMMIT=$(git -C "$ST/upstream/repo" rev-parse HEAD)

GEN=research/corpus/generators/selftest_tree.py
LIC='"license":{"spdx_or_name":"CC0-1.0","redistributable":true,"attribution":"","notes":"selftest"}'
write_sources() {  # $1 = heldout seed, $2 = tuning output_pin JSON fragment
cat > "$ST/corpus/sources/selftest.json" <<EOF
{"schema":"ebrc-sources-v1","items":[
 {"item_id":"st-gen-tuning","family":"F19","split":"tuning","scale":"small","kind":"generate","real_or_generated":"generated",
  "recipe":{"generator":{"script":"$GEN","interpreter":"python","seed":1,"params":{"files":16}}$2},
  $LIC,"independence_group":"st-gen-a","description":"selftest generated tree (tuning)"},
 {"item_id":"st-dup-tuning","family":"F17","split":"tuning","scale":"small","kind":"derive","real_or_generated":"generated",
  "recipe":{"from_items":["st-gen-tuning"],"steps":[{"op":"copy-item","from_item":"st-gen-tuning","dest":"a"},
            {"op":"copy-item","from_item":"st-gen-tuning","dest":"b"},{"op":"remove","paths":["b/empty.txt"]}]},
  $LIC,"independence_group":"st-gen-a","description":"selftest duplicate tree derived from st-gen-tuning"},
 {"item_id":"st-dl-tuning","family":"F05","split":"tuning","scale":"small","kind":"download","real_or_generated":"real",
  "recipe":{"inputs":[{"name":"blob","url":"file://$ST/upstream/blob.txt","sha256":"TOFU"},
                      {"name":"bundle","url":"file://$ST/upstream/bundle.tar","sha256":"TOFU"}],
            "steps":[{"op":"copy","input":"blob","dest":"copied/blob.txt"},{"op":"extract","input":"bundle","dest":"x"}]},
  $LIC,"independence_group":"st-local-blob","description":"selftest TOFU download"},
 {"item_id":"st-git-validation","family":"F01","split":"validation","scale":"small","kind":"git-archive","real_or_generated":"real",
  "recipe":{"git":{"repo":"file://$ST/upstream/repo","commit":"$COMMIT"}},
  $LIC,"independence_group":"st-local-repo","description":"selftest git archive"},
 {"item_id":"st-gen-validation","family":"F19","split":"validation","scale":"small","kind":"generate","real_or_generated":"generated",
  "recipe":{"generator":{"script":"$GEN","interpreter":"python","seed":2,"params":{"files":10}}},
  $LIC,"independence_group":"st-gen-b","description":"selftest generated tree (validation)"},
 {"item_id":"st-gen-heldout","family":"F19","split":"heldout","scale":"small","kind":"generate","real_or_generated":"generated",
  "recipe":{"generator":{"script":"$GEN","interpreter":"python","seed":$1,"params":{"files":10}}},
  $LIC,"independence_group":"st-gen-c","description":"selftest generated tree (heldout)"}
]}
EOF
}
write_sources 3 ""

# ---- 1. validation -------------------------------------------------------------------------
expect_rc 0 "OK: 6 item(s)" T provision --check
pass "source definitions validate"
mkdir -p "$ST/bad/sources"
sed -e 's/"independence_group":"st-gen-c"/"independence_group":"st-gen-b"/' "$ST/corpus/sources/selftest.json" > "$ST/bad/sources/bad.json"
expect_rc 2 "spans splits" T provision --check --corpus-dir "$ST/bad"
sed -e 's/"item_id":"st-gen-heldout"/"item_id":"Bad_ID"/' "$ST/corpus/sources/selftest.json" > "$ST/bad/sources/bad.json"
expect_rc 2 "kebab-case" T provision --check --corpus-dir "$ST/bad"
pass "group/split conflict and bad item_id rejected"

# ---- 2. materialize + idempotence ------------------------------------------------------------
expect_rc 0 "6 ok, 0 failed" T provision --jobs 2
cat "$ST/last.out"
R="$ST/corpus/fingerprints"
L1=$(jget "$R/st-gen-tuning.json" fingerprint.logical_tree_sha256)
expect_rc 0 "up-to-date (full fingerprint)" T provision
grep -q "materialized" "$ST/last.out" && { cat "$ST/last.out"; fail "second run rematerialized"; }
expect_rc 0 "up-to-date (quick stat check)" T provision --verify quick
pass "second run skips (full and quick verification)"
[[ -f "$ST/corpus/pins/st-dl-tuning.json" ]] || fail "TOFU pins not written"
[[ "$(jget "$R/st-gen-tuning.json" output_pin.status)" == "tofu-first-pin" ]] || fail "output TOFU pin status"
"$PY" - "$R/st-gen-heldout.json" <<'PY' || fail "held-out record exposes more than hashes/bytes/counts"
import json, sys
r = json.load(open(sys.argv[1]))
fp = r["fingerprint"]
assert "structure" not in fp and set(fp["counts"]) == {"objects", "files", "dirs", "symlinks", "hardlink_groups"}, fp
PY
pass "held-out record is fingerprint-only"

# ---- 3. determinism: rebuild matches TOFU pin -------------------------------------------------
expect_rc 0 "materialized" T provision --item st-gen-tuning --rebuild
[[ "$(jget "$R/st-gen-tuning.json" output_pin.status)" == "matched-tofu" ]] || fail "rebuild did not match TOFU pin"
[[ "$(jget "$R/st-gen-tuning.json" fingerprint.logical_tree_sha256)" == "$L1" ]] || fail "rebuild changed logical hash"
pass "generator rebuild reproduces logical_tree_sha256 $L1"

# ---- 4. fingerprint semantics ------------------------------------------------------------------
ITEM="$ST/data/corpus/tuning/st-gen-tuning"
[[ "$(T fingerprint "$ITEM" | "$PY" -c 'import json,sys; print(json.load(sys.stdin)["logical_tree_sha256"])')" == "$L1" ]] \
  || fail "standalone fingerprint differs from record"
zcat "$ST/data/fingerprints/tuning/st-gen-tuning.lines.gz" > "$ST/lines.txt"
"$PY" - "$ST/lines.txt" <<'PY' || fail "canonical lines missing object kinds"
import sys
lines = open(sys.argv[1], encoding="ascii").read().splitlines()
kinds = {f.split("=",1)[1] for l in lines for f in l.split("\t") if f.startswith("t=")}
need = {"file", "dir", "symlink", "fifo", "hardlink-group"}
assert need <= kinds, kinds
assert any("\tg=h0\t" in l for l in lines)
assert any("\td=e:1048576-" in l for l in lines), "sparse extent map missing"
assert any("\tx=757365722e656272632e6e6f7465:" in l for l in lines), "user xattr missing"
assert any("\tx=" in l and "73797374656d2e706f7369785f61636c5f616363657373" in l for l in lines), "ACL xattr missing"
assert any(l.startswith("p=") and "fffe2e62696e\t" in l for l in lines), "non-UTF-8 name missing"
print("\n".join(lines[:6]))
PY
C="$ST/copy"
rm -rf "$C"; cp -a "$ITEM" "$C"
"$PY" - "$ITEM" "$C" <<'PY' || fail "fingerprint sensitivity checks"
import json, os, subprocess, sys
sys.path.insert(0, os.environ["HERE"])
import fingerprint as f
a, c = sys.argv[1], sys.argv[2]
fa = f.fingerprint_path(a); fc = f.fingerprint_path(c)
assert fa["logical_tree_sha256"] == fc["logical_tree_sha256"], "cp -a copy should keep logical hash"
assert fa["content_tree_sha256"] == fc["content_tree_sha256"]
print("cp -a copy: logical equal; tree_sha256 equal:", fa["tree_sha256"] == fc["tree_sha256"])
def fp(): return f.fingerprint_path(c)
base = fp()
os.utime(os.path.join(c, "README"), ns=(1, 1))
x = fp(); assert x["logical_tree_sha256"] == base["logical_tree_sha256"] and x["extended_tree_sha256"] != base["extended_tree_sha256"], "mtime"
os.chmod(os.path.join(c, "empty.txt"), 0o600)
y = fp(); assert y["logical_tree_sha256"] != x["logical_tree_sha256"] and y["content_tree_sha256"] == x["content_tree_sha256"], "mode"
os.setxattr(os.path.join(c, "README"), "user.ebrc.note", b"changed")
z = fp(); assert z["logical_tree_sha256"] != y["logical_tree_sha256"], "xattr"
os.unlink(os.path.join(c, "data", "hardlink-a")); os.link(os.path.join(c, "empty.txt"), os.path.join(c, "data", "hardlink-a"))
w = fp(); assert w["content_tree_sha256"] != z["content_tree_sha256"], "hardlink regroup"
with open(os.path.join(c, "data", "sparse.img"), "r+b") as fh:
    fh.seek(0); fh.write(b"\0")
v = fp(); assert v["content_tree_sha256"] == w["content_tree_sha256"] and v["logical_tree_sha256"] != w["logical_tree_sha256"], "extent map"
print("sensitivity: mtime->extended only, mode->logical, xattr->logical, hardlink->content, holes->logical")
PY
pass "fingerprint semantics"

# ---- 5. regression: logical_tree_sha256 excludes st_blocks (ext4 delayed-allocation defect) ------
# tree_sha256 records raw st_blocks, which ext4 can settle to a different value after delayed
# allocation flushes (same content, same logical layout, different allocated block count). Corpus
# identity, held-out locking and verification must key on logical_tree_sha256, which must be
# unaffected. This constructs two fabricated stat_results that agree on every field except
# st_blocks and drives fingerprint.finalize() directly, so the check does not depend on coaxing a
# real filesystem into delayed allocation.
"$PY" - <<'PY' || fail "st_blocks regression"
import os, stat, sys
sys.path.insert(0, os.environ["HERE"])
import fingerprint as f

def make(blocks):
    seq = (stat.S_IFREG | 0o644, 424242, 7, 1, 1000, 1000, 4096, 0, 0, 0)
    st = os.stat_result(seq, {"st_blocks": blocks, "st_mtime_ns": 1700000000000000000,
                              "st_ctime_ns": 1700000000000000000, "st_atime_ns": 0})
    entry = f.Entry(b"", st, "file", None, (), 0)
    sc = f.Scan(b"/selftest-synthetic", [entry], "file")
    contents = {(st.st_dev, st.st_ino): ("ab" * 32, [(0, 4096)])}  # single "full" extent, no holes
    return f.finalize(sc, contents)

a, b = make(8), make(999999)  # same content/layout, wildly different allocated-block counts
assert a["logical_tree_sha256"] == b["logical_tree_sha256"], "st_blocks must not affect logical_tree_sha256"
assert a["content_tree_sha256"] == b["content_tree_sha256"], "st_blocks must not affect content_tree_sha256"
assert a["tree_sha256"] != b["tree_sha256"], "tree_sha256 is documented as allocation-bound (includes st_blocks)"
assert a["extended_tree_sha256"] != b["extended_tree_sha256"]
print("st_blocks 8 vs 999999: logical/content_tree_sha256 stable, tree/extended_tree_sha256 differ as documented")
PY
pass "logical_tree_sha256 is st_blocks-independent (regression for the ext4 delayed-allocation defect)"

# ---- 6. tamper and pin mismatch fail loudly ----------------------------------------------------
printf 'tamper' >> "$ITEM/empty.txt"
expect_rc 1 "HASH MISMATCH" T provision --item st-gen-tuning --verify full
expect_rc 0 "materialized" T provision --item st-gen-tuning --rebuild
pass "tampered tree detected; rebuild restores"
write_sources 3 ',"output_pin":"0000000000000000000000000000000000000000000000000000000000000000"'
expect_rc 1 "HASH MISMATCH" T provision --item st-gen-tuning
expect_rc 1 "HASH MISMATCH" T provision --item st-gen-tuning --rebuild
write_sources 3 ""
expect_rc 0 "up-to-date" T provision --item st-gen-tuning
pass "declared output_pin mismatch fails"
printf 'upstream blob v2 (drifted)\n' > "$ST/upstream/blob.txt"
rm -rf "$ST/data/cache"
expect_rc 1 "HASH MISMATCH" T provision --item st-dl-tuning --rebuild
printf 'upstream blob v1\n' > "$ST/upstream/blob.txt"
expect_rc 0 "materialized" T provision --item st-dl-tuning --rebuild
pass "TOFU-pinned download drift fails loudly"

# ---- 7. stats ------------------------------------------------------------------------------------
expect_rc 0 "5 ok, 0 failed" T stats --all --jobs 2
cat "$ST/last.out"
expect_rc 0 "up-to-date" T stats --item st-gen-tuning
cp "$ST/corpus/stats/st-gen-tuning.json" "$ST/stats1.json"
expect_rc 0 "stats written" T stats --item st-gen-tuning --force
cmp "$ST/stats1.json" "$ST/corpus/stats/st-gen-tuning.json" || fail "stats not deterministic"
pass "stats deterministic and idempotent"
expect_rc 3 "REFUSED" T stats --item st-gen-heldout
expect_rc 3 "REFUSED" T stats --path "$ST/data/heldout/st-gen-heldout"
[[ ! -f "$ST/corpus/stats/st-gen-heldout.json" ]] || fail "held-out stats were written"
T fingerprint "$ST/data/heldout/st-gen-heldout" | grep -q '"structure"' && fail "fingerprint CLI leaked held-out structure"
pass "held-out content access refused"
"$PY" - "$ST/corpus/stats/st-dup-tuning.json" "$ST/corpus/stats/st-gen-tuning.json" <<'PY' || fail "stats plausibility"
import json, sys
d = json.load(open(sys.argv[1])); g = json.load(open(sys.argv[2]))
assert d["exact_duplicates"]["duplicate_byte_ratio"] > 0.45, d["exact_duplicates"]
assert d["block_duplicates"]["duplicate_byte_ratio"] > 0.45, d["block_duplicates"]
m = g["metadata"]
assert m["symlinks"]["total"] == 3 and m["symlinks"]["absolute_target"] == 1 and m["symlinks"]["dangling_relative"] == 1, m["symlinks"]
assert m["hardlinks"]["groups"] == 1 and m["hardlinks"]["extra_paths"] == 2, m["hardlinks"]
assert m["sparse"]["sparse_files"] == 1 and m["xattrs"]["acl_access_objects"] == 1 and m["xattrs"]["acl_default_dirs"] == 1, m
assert m["names"]["non_utf8"] == 1 and m["names"]["case_insensitive_collisions"] == 1, m["names"]
assert m["modes"]["setgid"] == 1 and m["object_types"]["fifo"] == 1
print("dup tree exact/block dup ratios:", d["exact_duplicates"]["duplicate_byte_ratio"], d["block_duplicates"]["duplicate_byte_ratio"])
print("gen tree entropy/zstd/xz:", g["entropy"]["stream_order0_bits_per_byte"], g["compressibility"]["zstd -3"]["ratio"], g["compressibility"]["xz -6"]["ratio"])
print("mime types:", g["content_types"]["by_count"])
PY
pass "stats plausibility"
"$PY" - <<'PY' || fail "sampler"
import os, sys
sys.path.insert(0, os.environ["HERE"])
import stats
s = stats.Sampler(10 * (1 << 20) + 5, 3 * (1 << 20))
got = 0
for i in range(0, 10 * (1 << 20) + 5, 777777):
    n = min(777777, 10 * (1 << 20) + 5 - i)
    got += sum(len(p) for p in s.pieces(memoryview(bytearray(n))))
assert got == 3 * (1 << 20), got
PY
pass "compressibility sampler takes exactly the budget"

# ---- 8. assemble, determinism, freeze ---------------------------------------------------------------
expect_rc 0 "status=draft" T assemble --require-all --require-stats
cat "$ST/last.out"
cp "$ST/corpus/manifest.json" "$ST/m1.json"; cp "$ST/corpus/statistics.json" "$ST/s1.json"
cp "$ST/corpus/coverage.md" "$ST/c1.md"; cp "$ST/corpus/licenses.md" "$ST/lic1.md"
expect_rc 0 "manifest" T assemble
cmp "$ST/m1.json" "$ST/corpus/manifest.json" && cmp "$ST/s1.json" "$ST/corpus/statistics.json" \
  && cmp "$ST/c1.md" "$ST/corpus/coverage.md" && cmp "$ST/lic1.md" "$ST/corpus/licenses.md" \
  || fail "assemble not deterministic"
"$PY" - "$ST/corpus/manifest.json" <<'PY' || fail "manifest_sha256"
import json, sys, hashlib
m = json.load(open(sys.argv[1])); h = m.pop("manifest_sha256")
assert hashlib.sha256(json.dumps(m, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest() == h
assert m["coverage"]["family_split_scale"]["F19"]["heldout"]["small"]["items"] == 1
PY
pass "assemble deterministic; manifest_sha256 verifies"
grep -q "Independence groups" "$ST/corpus/coverage.md" || fail "coverage.md missing independence groups section"
grep -q "Real vs. generated" "$ST/corpus/coverage.md" || fail "coverage.md missing real-vs-generated section"
grep -q "st-gen-a" "$ST/corpus/coverage.md" || fail "coverage.md missing independence group ids"
grep -q "^# Corpus licenses" "$ST/corpus/licenses.md" || fail "licenses.md missing header"
grep -q "CC0-1.0" "$ST/corpus/licenses.md" || fail "licenses.md missing selftest license"
grep -q "every item is marked .redistributable: true." "$ST/corpus/licenses.md" || fail "licenses.md non-redistributable section wrong"
pass "coverage.md and licenses.md generated and deterministic"
expect_rc 0 "status=frozen" T assemble --freeze
expect_rc 0 "OK" T assemble --verify-heldout
write_sources 4 ""
expect_rc 0 "materialized" T provision --item st-gen-heldout
expect_rc 1 "HELD-OUT SET CHANGED" T assemble
write_sources 3 ""
expect_rc 0 "materialized" T provision --item st-gen-heldout
expect_rc 0 "status=frozen" T assemble
pass "frozen held-out set guarded; restored set accepted"
echo "fingerprint of held-out lock: $(jget "$ST/corpus/heldout-lock.json" heldout_set_sha256)"

# ---- 9. optional docker-export (network): SELFTEST_DOCKER_IMAGE=name:tag@sha256:<index digest> ----
if [[ -n "${SELFTEST_DOCKER_IMAGE:-}" ]]; then
  mkdir -p "$ST/dcorpus/sources"
  printf '%s\n' "{\"schema\":\"ebrc-sources-v1\",\"items\":[
 {\"item_id\":\"st-docker-tuning\",\"family\":\"F09\",\"split\":\"tuning\",\"scale\":\"small\",\"kind\":\"docker-export\",
  \"real_or_generated\":\"real\",
  \"recipe\":{\"docker\":{\"image\":\"$SELFTEST_DOCKER_IMAGE\",\"platform\":\"linux/amd64\"},
            \"steps\":[{\"op\":\"extract\",\"input\":\"docker-rootfs\",\"format\":\"tar\"},{\"op\":\"remove\",\"paths\":[\".dockerenv\"]}],
            \"output_pin\":\"TOFU\"},
  $LIC,\"independence_group\":\"st-docker\",\"description\":\"selftest docker export\"}]}" > "$ST/dcorpus/sources/docker.json"
  expect_rc 0 "1 ok, 0 failed" T provision --corpus-dir "$ST/dcorpus"
  cat "$ST/last.out"
  expect_rc 0 "materialized" T provision --corpus-dir "$ST/dcorpus" --rebuild
  [[ "$(jget "$ST/dcorpus/fingerprints/st-docker-tuning.json" output_pin.status)" == "matched-tofu" ]] \
    || fail "docker export not reproducible under logical_tree_sha256"
  expect_rc 0 "stats written" T stats --corpus-dir "$ST/dcorpus" --all
  "$PY" - "$ST/dcorpus/fingerprints/st-docker-tuning.json" "$ST/dcorpus/stats/st-docker-tuning.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1])); s = json.load(open(sys.argv[2]))
print("docker provenance:", json.dumps(r["provenance"]["docker"]))
print("docker counts:", r["fingerprint"]["counts"], "hardlinks:", s["metadata"]["hardlinks"])
PY
  pass "docker-export reproducible (TOFU matched on rebuild)"
fi

echo "SELFTEST PASS"
