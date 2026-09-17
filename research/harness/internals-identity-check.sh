#!/usr/bin/env bash
# Byte-identity proof for the default-off `research-internals` cargo feature
# (WSL, primary host).
#
# 1. Additive-only audit: every production change is a removable
#    #[cfg(feature = "research-internals")] block (internals_identity_tools.py).
# 2. Builds the CLI with and without --features entrybound/research-internals,
#    packs one fixed generated tree under all four profiles in INDEXED and
#    STREAM layouts, and requires identical SHA-256 for every output.
# 3. cargo test -p entrybound -p entrybound-cli without the feature, and the
#    same packages with --features entrybound/research-internals (a superset of
#    cargo test -p entrybound --features research-internals).
# 4. cargo clippy --workspace --all-targets (lints reported, and -D warnings)
#    both ways, and cargo fmt --all --check.
# Gates report PASS, FAIL, or PRE-EXISTING (fails identically without the
# feature, on source the additive audit proves equal to the base commit).
# 5. Rewrites the WSL section of internals-identity-check.md.
#
# Usage (from Windows):
#   wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/internals-identity-check.sh
# Overrides: EB_TARGET_DIR, EB_WORK_DIR, EB_BASE_REV (additive-audit base),
#   EB_SKIP_DISK_GUARD=1 (only when the caller already ran disk_guard.py).
# Not decision-grade timing: no timings are recorded.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../.." && pwd)"
TOOLCHAIN=1.98.1
export RUSTUP_TOOLCHAIN="$TOOLCHAIN"
export PATH="/root/.cargo/bin:$PATH"
CARGO=(/root/.cargo/bin/cargo "+$TOOLCHAIN")
export CARGO_TARGET_DIR="${EB_TARGET_DIR:-/root/eb-research/target/b2-internals}"
WORK="${EB_WORK_DIR:-/root/eb-research/scratch/b2-internals-identity}"
REPORT="$REPO/research/harness/internals-identity-check.md"
TOOLS="$REPO/research/harness/internals_identity_tools.py"
PY=(python3 -B)
WINPY=/mnt/c/Python313/python.exe
PROFILES=(fast balanced dense extreme)
LAYOUTS=(indexed stream)

case "$WORK" in
    /root/eb-research/*|/tmp/*) ;;
    *) [[ -n "${EB_WORK_DIR:-}" ]] || { echo "refusing unexpected work dir $WORK" >&2; exit 2; } ;;
esac

cd "$REPO" || exit 2
rm -rf "$WORK"
mkdir -p "$WORK/logs" "$WORK/bin" "$WORK/out/default" "$WORK/out/research"
SECTION="$WORK/section.md"
: > "$SECTION"
GATES=()
FAILED=0

md() { printf '%s\n' "$@" >> "$SECTION"; }
PREEXISTING=0
gate() {
    GATES+=("| $1 | $2 | $3 |")
    case "$2" in
        PASS) ;;
        PRE-EXISTING) PREEXISTING=1 ;;
        *) FAILED=1 ;;
    esac
}
quote_cmd() { local out="" arg; for arg in "$@"; do out+="$(printf '%q' "$arg") "; done; printf '%s' "${out% }"; }
# run_logged <log> <cmd...>: records the command line, returns its exit code.
run_logged() {
    local log="$1"; shift
    printf '$ %s\n' "$(quote_cmd "$@")" > "$log"
    "$@" >> "$log" 2>&1
}
fence() { md '```text'; cat "$1" >> "$SECTION"; md '```'; }
sha() { sha256sum "$1" | cut -d' ' -f1; }

# ---------------------------------------------------------------------------
# Disk guard
# ---------------------------------------------------------------------------
if [[ "${EB_SKIP_DISK_GUARD:-0}" == 1 ]]; then
    guard_status="skipped (EB_SKIP_DISK_GUARD=1; caller ran disk_guard.py)"
elif [[ -x "$WINPY" ]]; then
    if (cd /mnt/c && "$WINPY" "$(wslpath -w "$REPO/research/orchestration/disk_guard.py")") > "$WORK/logs/disk-guard.log" 2>&1; then
        guard_status="PASS"
    else
        cat "$WORK/logs/disk-guard.log" >&2
        echo "disk guard failed; not building" >&2
        exit 2
    fi
else
    echo "cannot run disk_guard.py via $WINPY; set EB_SKIP_DISK_GUARD=1 after running it on the host" >&2
    exit 2
fi

git_ro() { GIT_OPTIONAL_LOCKS=0 git -c core.autocrlf=true -c safe.directory="$REPO" -C "$REPO" "$@"; }
head_sha="$(git_ro rev-parse HEAD)"
worktree="$(git_ro status --porcelain -- crates research/harness | sed 's/^/    /')"

md "## WSL (primary)" ""
md "- Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ) on $(uname -srm) (host name redacted)"
md "- Repository HEAD: \`$head_sha\`"
md "- Worktree changes under \`crates/\` and \`research/harness/\` at run time:" ""
if [[ -n "$worktree" ]]; then md '```text' "$worktree" '```'; else md "    (clean: the feature is committed)"; fi
md "" "- Toolchain: \`$("${CARGO[@]}" --version)\`, \`$(rustc "+$TOOLCHAIN" --version)\` ($(rustc "+$TOOLCHAIN" -vV | sed -n 's/^LLVM version: /LLVM /p'))"
md "- \`CARGO_TARGET_DIR=$CARGO_TARGET_DIR\`, work directory \`$WORK\` (both outside the repository)"
md "- Disk guard (\`research/orchestration/disk_guard.py\` via Windows Python): $guard_status"
md "- Timing: none recorded (not decision-grade)." ""

# ---------------------------------------------------------------------------
# 1. Additive-only source audit
# ---------------------------------------------------------------------------
audit_cmd=("${PY[@]}" "$TOOLS" additive-check "$REPO")
[[ -n "${EB_BASE_REV:-}" ]] && audit_cmd+=(--base "$EB_BASE_REV")
if run_logged "$WORK/logs/additive-check.log" "${audit_cmd[@]}"; then audit=PASS; else audit=FAIL; fi
gate "Additive-only production diff" "$audit" "every changed production line is inside a removable research-internals block"

# ---------------------------------------------------------------------------
# 2. Builds and output byte identity
# ---------------------------------------------------------------------------
build_default=("${CARGO[@]}" build --release --locked -p entrybound-cli)
build_research=("${CARGO[@]}" build --release --locked -p entrybound-cli --features entrybound/research-internals)
builds_ok=PASS
if run_logged "$WORK/logs/build-default.log" "${build_default[@]}"; then
    cp "$CARGO_TARGET_DIR/release/ebound" "$WORK/bin/ebound-default"
else
    builds_ok=FAIL
fi
if run_logged "$WORK/logs/build-research.log" "${build_research[@]}"; then
    cp "$CARGO_TARGET_DIR/release/ebound" "$WORK/bin/ebound-research"
else
    builds_ok=FAIL
fi
gate "CLI builds (default and research-internals)" "$builds_ok" "release, --locked"

tree_line="$("${PY[@]}" "$TOOLS" tree "$WORK/tree")"
read -r tree_files tree_bytes tree_digest <<< "$tree_line"

pack_rows=()
identical_count=0
pack_total=0
pack_ok=PASS
if [[ "$builds_ok" == PASS ]]; then
    for profile in "${PROFILES[@]}"; do
        for layout in "${LAYOUTS[@]}"; do
            # Dense/Extreme lookback groups need a STREAM dedup window; the CLI
            # default ceiling is 0, so STREAM packs declare an automatic window.
            window=()
            [[ "$layout" == stream ]] && window=(--stream-window auto)
            declare -A hashes=()
            for build in default research; do
                out="$WORK/out/$build/$profile-$layout.eb"
                if ! run_logged "$WORK/logs/pack-$build-$profile-$layout.log" \
                    "$WORK/bin/ebound-$build" pack "$WORK/tree" "$out" --layout "$layout" "${window[@]}" --profile "$profile"; then
                    hashes[$build]="PACK-FAILED"
                    continue
                fi
                hashes[$build]="$(sha "$out")"
            done
            pack_total=$((pack_total + 1))
            size="$(stat -c %s "$WORK/out/default/$profile-$layout.eb" 2>/dev/null || echo "?")"
            if [[ "${hashes[default]}" == "${hashes[research]}" && "${hashes[default]}" != PACK-FAILED ]]; then
                same=yes
                identical_count=$((identical_count + 1))
            else
                same=NO
                pack_ok=FAIL
            fi
            pack_rows+=("| $profile | $layout | $size | \`${hashes[default]}\` | \`${hashes[research]}\` | $same |")
            unset hashes
        done
    done
    for profile in "${PROFILES[@]}"; do
        "$WORK/bin/ebound-default" inspect "$WORK/out/default/$profile-indexed.eb" > "$WORK/logs/inspect-$profile.log" 2>&1 || true
    done
else
    pack_ok=FAIL
fi
gate "Pack output SHA-256 identical with and without the feature" "$pack_ok" "$identical_count/$pack_total profile x layout outputs identical"

# ---------------------------------------------------------------------------
# 3. Tests. --no-fail-fast so every test binary runs even when an unrelated
# environment-dependent test fails. The research run covers the same packages
# plus research_internals (a superset of cargo test -p entrybound --features
# research-internals), so failing-test sets are directly comparable.
# ---------------------------------------------------------------------------
test_default=("${CARGO[@]}" test --release --locked --no-fail-fast -p entrybound -p entrybound-cli)
test_research=("${CARGO[@]}" test --release --locked --no-fail-fast -p entrybound -p entrybound-cli --features entrybound/research-internals)
run_logged "$WORK/logs/test-default.log" "${test_default[@]}"; test_default_rc=$?
run_logged "$WORK/logs/test-research.log" "${test_research[@]}"; test_research_rc=$?
for name in test-default test-research; do
    "${PY[@]}" "$TOOLS" test-summary "$WORK/logs/$name.log" > "$WORK/logs/$name.summary"
    grep '^FAILED ' "$WORK/logs/$name.summary" > "$WORK/logs/$name.failed"
done
if [[ $test_default_rc -eq 0 ]]; then
    t_default=PASS
elif [[ "$audit" == PASS && -s "$WORK/logs/test-default.failed" ]] && grep -q '^compile errors: no' "$WORK/logs/test-default.summary"; then
    t_default=PRE-EXISTING
else
    t_default=FAIL
fi
if grep -q '^compile errors: no' "$WORK/logs/test-research.summary" \
    && grep -Eq '^research_internals .*: ok, [1-9][0-9]* passed, 0 failed' "$WORK/logs/test-research.summary" \
    && cmp -s "$WORK/logs/test-default.failed" "$WORK/logs/test-research.failed" \
    && { [[ $test_research_rc -eq 0 ]] || [[ $test_default_rc -ne 0 ]]; }; then
    t_research=PASS
else
    t_research=FAIL
fi
gate "cargo test without the feature" "$t_default" "exit $test_default_rc; $(head -1 "$WORK/logs/test-default.summary"); $(wc -l < "$WORK/logs/test-default.failed") failing"
gate "cargo test with research-internals: research_internals passes, failing tests identical to the run without the feature" "$t_research" "exit $test_research_rc; $(head -1 "$WORK/logs/test-research.summary")"

# ---------------------------------------------------------------------------
# 4. Clippy and rustfmt. The warning run (lints reported, not denied) checks
# every target, so its findings are the complete comparison; the -D warnings
# run stops at the first failing crate.
# ---------------------------------------------------------------------------
clippy_warn_default=("${CARGO[@]}" clippy --locked --workspace --all-targets)
clippy_warn_research=("${CARGO[@]}" clippy --locked --workspace --all-targets --features entrybound/research-internals)
clippy_default=("${clippy_warn_default[@]}" -- -D warnings)
clippy_research=("${clippy_warn_research[@]}" -- -D warnings)
run_logged "$WORK/logs/clippy-warn-default.log" "${clippy_warn_default[@]}"; warn_default_rc=$?
run_logged "$WORK/logs/clippy-warn-research.log" "${clippy_warn_research[@]}"; warn_research_rc=$?
run_logged "$WORK/logs/clippy-strict-default.log" "${clippy_default[@]}"; strict_default_rc=$?
run_logged "$WORK/logs/clippy-strict-research.log" "${clippy_research[@]}"; strict_research_rc=$?
for name in clippy-warn-default clippy-warn-research clippy-strict-default clippy-strict-research; do
    "${PY[@]}" "$TOOLS" clippy-summary "$WORK/logs/$name.log" > "$WORK/logs/$name.summary"
done
if [[ $warn_default_rc -eq 0 && $warn_research_rc -eq 0 ]] && cmp -s "$WORK/logs/clippy-warn-default.summary" "$WORK/logs/clippy-warn-research.summary"; then
    c_same=PASS
else
    c_same=FAIL
fi
warn_count="$(grep -c -v '^(no diagnostics)$' "$WORK/logs/clippy-warn-default.summary")"
gate "clippy findings identical with and without the feature (all targets, lints reported)" "$c_same" "exit $warn_default_rc/$warn_research_rc; $warn_count findings without the feature"
if [[ $strict_default_rc -eq 0 && $strict_research_rc -eq 0 ]]; then
    c_strict=PASS
elif [[ "$audit" == PASS && $strict_default_rc -eq $strict_research_rc ]] && cmp -s "$WORK/logs/clippy-strict-default.summary" "$WORK/logs/clippy-strict-research.summary"; then
    c_strict=PRE-EXISTING
else
    c_strict=FAIL
fi
gate "clippy --all-targets -D warnings, both ways" "$c_strict" "exit $strict_default_rc/$strict_research_rc"
fmt_cmd=("${CARGO[@]}" fmt --all --check)
if run_logged "$WORK/logs/fmt.log" "${fmt_cmd[@]}"; then fmt=PASS; else fmt=FAIL; fi
gate "cargo fmt --all --check" "$fmt" "exit 0 required"

# ---------------------------------------------------------------------------
# 5. Report section
# ---------------------------------------------------------------------------
verdict=PASS
if [[ $FAILED -ne 0 ]]; then
    verdict=FAIL
elif [[ $PREEXISTING -ne 0 ]]; then
    verdict="PASS (pre-existing failures recorded; identical with and without the feature)"
fi
md "### Verdict: $verdict" "" "| Gate | Result | Detail |" "|---|---|---|"
for row in "${GATES[@]}"; do md "$row"; done
md "" "PRE-EXISTING: the command also fails without the feature, and its results are identical with the feature. The additive-only audit proves that the source compiled without the feature is byte-identical to the base commit, so these failures exist at the base commit in this environment and are not caused by research-internals."
md "" "### Additive-only production diff" "" "\`$(quote_cmd "${audit_cmd[@]}" | sed "s#$REPO#\$REPO#g")\`" ""
fence "$WORK/logs/additive-check.log"
md "" "### Builds" ""
md "- \`$(quote_cmd "${build_default[@]}")\` -> \`ebound-default\` SHA-256 \`$(sha "$WORK/bin/ebound-default" 2>/dev/null || echo n/a)\`"
md "- \`$(quote_cmd "${build_research[@]}")\` -> \`ebound-research\` SHA-256 \`$(sha "$WORK/bin/ebound-research" 2>/dev/null || echo n/a)\`"
md "- Binary hashes are informational; the gate is the output identity below." ""
md "### Fixed input tree" ""
md "- \`$(quote_cmd "${PY[@]}" "$TOOLS" tree "$WORK/tree" | sed "s#$REPO#\$REPO#g")\`"
md "- $tree_files files, $tree_bytes bytes, tree digest \`$tree_digest\` (sorted path, length and SHA-256 of every file; mtimes fixed at $(date -u -d @1700000000 +%Y-%m-%dT%H:%M:%SZ))"
md "- Contents: log text, Markdown, an empty file and directory, counters, zeros, SHA-256 counter-mode random bytes, a gzip DEFLATE stream, a baseline JPEG, 12 similar text records, 24 shared-noise samples." ""
md "### Pack outputs" ""
md "Command per cell: \`\$WORK/bin/ebound-<build> pack \$WORK/tree \$WORK/out/<build>/<profile>-<layout>.eb --layout <layout> [--stream-window auto for stream] --profile <profile>\`" ""
md "| Profile | Layout | Bytes | SHA-256 without feature | SHA-256 with research-internals | Identical |" "|---|---|---|---|---|---|"
for row in "${pack_rows[@]}"; do md "$row"; done
md "" "Coverage of the default build's INDEXED archives (\`ebound inspect\`, selected lines):" ""
: > "$WORK/logs/inspect.summary"
for profile in "${PROFILES[@]}"; do
    {
        echo "[$profile]"
        grep -E '^(planner|cross-file compression|chunk groups|reconstruction|whole-object reconstruction):' "$WORK/logs/inspect-$profile.log"
    } >> "$WORK/logs/inspect.summary" 2>/dev/null
done
fence "$WORK/logs/inspect.summary"
md "" "### Tests" ""
md "- \`$(quote_cmd "${test_default[@]}")\`: $t_default"
fence "$WORK/logs/test-default.summary"
md "- \`$(quote_cmd "${test_research[@]}")\`: $t_research"
fence "$WORK/logs/test-research.summary"
md "" "### Clippy and rustfmt" ""
md "- \`$(quote_cmd "${clippy_warn_default[@]}")\`: exit $warn_default_rc"
fence "$WORK/logs/clippy-warn-default.summary"
md "- \`$(quote_cmd "${clippy_warn_research[@]}")\`: exit $warn_research_rc"
fence "$WORK/logs/clippy-warn-research.summary"
md "- \`$(quote_cmd "${clippy_default[@]}")\`: exit $strict_default_rc"
fence "$WORK/logs/clippy-strict-default.summary"
md "- \`$(quote_cmd "${clippy_research[@]}")\`: exit $strict_research_rc"
fence "$WORK/logs/clippy-strict-research.summary"
md "- \`$(quote_cmd "${fmt_cmd[@]}")\`: $fmt"
md "" "Full logs were written to \`$WORK/logs\` (outside the repository)."

"${PY[@]}" "$TOOLS" write-section "$REPORT" wsl "$SECTION"
echo "research-internals identity check (WSL): $verdict"
printf '%s\n' "${GATES[@]}"
exit "$FAILED"
