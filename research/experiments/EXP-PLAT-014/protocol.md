# EXP-PLAT-014: Extraction confinement: deterministic-interleaving races, random races, symlink chains, mode reporting and hostile-path replay

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **NEEDS_TOOLING**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-005, DEC-PLT-006, DEC-PLT-004, DEC-PLT-017, DEC-PLT-041, DEC-PLT-009, DEC-PLT-039, DEC-PLT-047, DEC-PLT-034, DEC-PLT-001, DEC-INT-004, DEC-MOD-038
- **Program sections:** 15, 16, 21, 29, 30
- **Decision type (proposed, not assigned):** EMPIRICAL (escapes, refusals, reported modes); FORMAL parts (planned-tree resolution rules and decoded-form validation rules as derivations with counterexample search); security claims about races beyond regression evidence need external review (§8.3). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-25 (M25.4), OD-18 (M18.3), OD-04 (M04.2 reason-code specificity); HC-05, HC-15, HC-17, HC-08.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** Does each containment candidate keep every create, open, link and metadata operation of extraction beneath the root under deterministic-interleaving swaps at every path-resolution step, 10,000-trial random races, symlink-chain and case-aliasing corpora, root replacement, mount substitution and legacy hostile-path replays; does each correctly detect and report its achieved confinement mode (including ENOSYS/EPERM fallback and absent /proc); and do candidate overwrite implementations and decoded-form path validations stay race- and traversal-safe?
- **H1:** Status-quo cap-std extraction has zero escapes on Linux (openat2 RESOLVE_BENEATH present) but reports `KernelEnforced` even when openat2 is forced to fail with ENOSYS and the manual fallback runs (an HC-05/I25 misreport); the lexical Safe symlink check admits the two-link chain `p -> .`, `q -> p/..` whose kernel resolution escapes after all links exist (planned-tree resolution and post-extraction verification refuse it); on Windows the status quo has at least one escape or misreport under oplock-paused junction swaps unless every component is opened with reparse awareness; rename-over overwrite races escape while unlinkat-plus-O_EXCL with bounded retry does not.
- **H0 / equivalence:** All candidates, including the status quo, have zero escapes and correct mode reports in every scenario, and lexical and planned-tree symlink checks refuse the same cases.
- **Falsification of H1:** No misreport under ENOSYS injection, no escape by the two-link chain under Safe, or zero Windows escapes for the status quo under oplock-driven swaps.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-005 | V1_BLOCKING | Which containment primitive does Entrybound extraction (and capture) use on Linux, macOS and Windows, given cap-std 4.0.3 uses openat2 RESOLVE_BENEATH only on Linux (w... | Race-injection results per containment candidate on ext4/XFS/btrfs and NTFS; mechanism table for WSL2 (openat2, Landlock, mount namespaces, seccomp) and for Windows. | Per-component cost (EXP-PLAT-015); safe-Rust/unsafe audit (EXP-PLAT-016); Docker/Android sandboxes (EXP-PLAT-024); macOS (EXP-PLAT-022). |
| DEC-PLT-006 | V1_IMPORTANT | What bounded retry count applies when kernel resolution returns EAGAIN (RESOLVE_BENEATH/RESOLVE_IN_ROOT), is exhaustion a distinct reason code, and is mount crossing (... | Adversarial rename-race liveness versus DoS for bounded EAGAIN retries; bind-mount false positives for NO_XDEV. | cap-primitives retry audit (EXP-PLAT-016 source audit). |
| DEC-PLT-004 | V1_BLOCKING | How is the achieved confinement mode detected and reported, and must a weaker-than-kernel mode refuse, require explicit opt-in, or proceed with a printed report? | Honesty audit of the reported confinement mode under ENOSYS/EPERM injection and absent /proc. | Usability of opt-in versus report-only (HUMAN-FACING); macOS/BSD/WASI (not reachable). |
| DEC-PLT-017 | V1_BLOCKING | Which handle-relative, no-follow operations must capture and extraction use for directory, file, symlink and special-file creation and metadata (mode, owner, xattrs, t... | Race tests swapping parents during symlink xattr restore and during capture; behaviour without /proc for the O_PATH bridge. | API coverage table (EXP-PLAT-016); macOS (EXP-PLAT-022). |
| DEC-PLT-041 | V1_BLOCKING | Should the Safe symlink policy remain a per-link lexical check, or resolve each target against the planned extracted tree including other archived symlinks (and case-i... | Adversarial chain corpus (including the CVE-2025-59825 two-link escape, dangling, loops, case aliasing on NTFS) versus kernel resolution after extraction. | Compatibility cost (EXP-PLAT-003); resolution cost (EXP-PLAT-015); APFS (EXP-PLAT-022). |
| DEC-PLT-009 | V1_IMPORTANT | What are the v1 semantics of the extraction root: must it be a real directory (not a symlink), may extraction merge into an existing non-empty directory by default, an... | Root replaced by a symlink/junction between create and open; merge into a non-empty root containing pre-existing links. | Incumbent defaults (EXP-PLAT-008); usability (HUMAN-FACING). |
| DEC-PLT-039 | V1_IMPORTANT | Which safe-extraction and path-safety claims may v1 make (SPEC §22.2 'safe extraction guaranteed by format: full'; traversal structurally impossible; kernel-enforced c... | Per-platform, per-mode escape and misreport results that bound every safe-extraction claim. | Wording review against SPEC and docs (analytical). |
| DEC-PLT-047 | V1_IMPORTANT | What minimum Linux kernel, macOS and Windows versions does v1 support for metadata capture and kernel-enforced confinement, and what fallback or report applies below t... | Fallback behaviour when kernel mechanisms are unavailable (ENOSYS/EPERM injection simulating older kernels and seccomp profiles). | Release-note timeline (EXP-PLAT-021); installed-base data (not measurable). |
| DEC-PLT-034 | V1_BLOCKING | What is Entrybound's REQUIRED/FORBIDDEN outcome (accept, refuse, report conflict) for each path, link and reparse corpus case: EB-Z014 backslash, Z015 absolute, Z016 t... | Replay of every EB-Z/EB-T case and F20 malicious archive through Entrybound import/unpack and incumbents on Linux and Windows under whole-filesystem write monitoring; proposed REQUIRED/FORBIDDEN outcome inputs. | Conformance corpus schema and class assignment (conformance domain). |
| DEC-PLT-001 | V1_BLOCKING | For symlink members with absolute or escaping targets, should extraction refuse by default (status quo Safe, Python data filter), rewrite or resolve in-root (openat2 R... | Downstream-following behaviour: after `--symlinks all`, whether consumers following written links leave the root (post-extraction resolution census). | Frequency (EXP-PLAT-003); incumbent matrix (EXP-PLAT-008). |
| DEC-INT-004 | V1_IMPORTANT | Is 'never overwrite, no --force' the frozen v1 CLI policy, or does v1 adopt SPEC's caller-selected --on-collision policy (default refuse, overwrite via parent-descript... | TOCTOU results for overwrite implementations (unlinkat via parent handle plus O_EXCL with bounded retry, renameat2 NOREPLACE temp-rename, plain rename-over) under swap attackers on ext4 and NTFS. | Usability of refuse-only outputs (HUMAN-FACING). |
| DEC-MOD-038 | V1_BLOCKING | Do non-UTF-8 / non-Unicode native names (raw POSIX bytes, unpaired UTF-16 surrogates, legacy code pages) need LogicalPath representation in v1, and if so with which en... | Traversal-safety fuzzing of decoded-form `.`/`..`/separator checks per encoding candidate, including target normalization from EXP-PLAT-002 (for example NTFS trailing-dot stripping). | Representation cost and identity (EXP-PLAT-002/005). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production `ebound` (cap-std 4.0.3, lexical Safe symlinks, hardcoded `KernelEnforced`, exclusive create without overwrite).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-005** — candidates: `cap-std-4-status-quo`, `rustix-openat2-in-root-no-symlinks`, `manual-openat-nofollow-walk`, `macos-o-resolve-beneath`, `windows-ntcreatefile-reparse-walk`, `mount-namespace-or-chroot-sandbox`, `landlock-seccomp-complement`, `refuse-unsupported-platforms` (status quo: `cap-std-4-status-quo`). Treatment: Implemented as a backend and run through every scenario.
  - excluded before execution (ledger): `string-prefix-validation` — SPEC §11.6 Pattern 1 forbids string validation followed by ordinary file operations; R1 and R2 classify prefix checking as a defect class (violates SPEC §11.6 L1413 frozen extraction doctrine); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-006** — candidates: `status-quo-cap-std-internal`, `fixed-small-bound-distinct-reason`, `configurable-bound`, `no-xdev-default-on`, `no-xdev-default-off` (status quo: `status-quo-cap-std-internal`). Treatment: Implemented as retry/NO_XDEV variants of the openat2 backend.
  - excluded before execution (ledger): `string-validation-fallback` — SPEC forbids falling back to string validation when the kernel cannot prove containment. (violates SPEC §11.6 L1423); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-004** — candidates: `hardcoded-kernel-enforced-status-quo`, `runtime-detect-and-report`, `weak-mode-requires-opt-in`, `refuse-weak-platforms`, `static-platform-table` (status quo: `hardcoded-kernel-enforced-status-quo`). Treatment: Status quo measured; detection candidates implemented in the prototype backends' mode reporting.
  - excluded before execution (ledger): `silent-weaker-mode` — SPEC §11.6 L1436 says silence about a weaker confinement mode is an I25 violation (violates I25); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-017** — candidates: `status-quo-mixed`, `rustix-at-nofollow`, `o-path-proc-self-fd-bridge`, `cap-std-ext-apis`, `audited-unsafe-ffi-boundary`, `decline-and-report-only`, `status-quo`, `handle-based-capture`, `kernel-resolve-beneath-macos`, `landlock-sandboxed-extraction`, `declare-concurrent-mutation-out-of-scope`, `private-mount-destination` (status quo: `status-quo-mixed`, `status-quo`). Treatment: Implemented in prototypes where an API exists (EXP-PLAT-016); status quo measured.
  - excluded before execution (ledger): `path-based-with-checks` — SPEC §11.6 Pattern 2 forbids setting metadata by path; silently doing so is a conformance violation (violates SPEC §11.6 L1424-1426 frozen extraction pattern); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-041** — candidates: `lexical-per-link-status-quo`, `planned-tree-resolution`, `no-symlink-components-in-targets`, `post-extraction-physical-verify`, `case-insensitive-aware-resolution`, `refuse-all-symlinks-default` (status quo: `lexical-per-link-status-quo`). Treatment: Implemented as reference models (`planned_tree_resolve.py`, `verify_beneath.py`) applied to the chain corpus; status quo measured through `ebound unpack --symlinks safe`.
- **DEC-PLT-009** — candidates: `status-quo-merge-follow-root-symlink`, `require-empty-or-new-root`, `refuse-symlink-root`, `follow-root-symlink-documented`, `single-top-level-enforced` (status quo: `status-quo-merge-follow-root-symlink`). Treatment: Status quo measured; stricter root candidates implemented in prototypes.
- **DEC-PLT-039** — candidates: `spec-full-guarantee-status-quo`, `per-platform-tested-claims`, `model-level-only-claim`, `baseline-guarantee-no-marketing` (status quo: `spec-full-guarantee-status-quo`). Treatment: Claims scored against measured escapes and misreports.
- **DEC-PLT-047** — candidates: `status-quo-unstated`, `linux-5-6-openat2-minimum`, `linux-lts-5-10-minimum`, `macos-11-minimum`, `macos-latest-two`, `tiered-support-weaker-reported` (status quo: `status-quo-unstated`). Treatment: Scored on fallback behaviour under injection.
- **DEC-PLT-034** — candidates: `refuse-at-import-status-quo`, `accept-and-refuse-at-extract`, `report-conflict-preservation-mode`, `regenerate-t014-true-non-utf8` (status quo: `refuse-at-import-status-quo`). Treatment: Outcome candidates scored against the replay matrix.
  - excluded before execution (ledger): `strip-or-normalise-like-incumbents` — Legacy ambiguity must be surfaced, never silently normalised (violates I18); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-001** — candidates: `refuse-by-default-all-opt-in-status-quo`, `write-as-is-never-follow`, `rewrite-relative-in-root`, `skip-with-report`, `separate-grants-absolute-vs-escaping` (status quo: `refuse-by-default-all-opt-in-status-quo`). Treatment: Status quo Safe/All measured; downstream-following analysis per written link.
- **DEC-INT-004** — candidates: `status-quo-refuse-only`, `on-collision-policy`, `force-flag`, `interactive-prompt` (status quo: `status-quo-refuse-only`). Treatment: Each overwrite implementation prototyped in `ebr-platform-probe`.
  - excluded before execution (ledger): `archive-directed-overwrite` — Extraction policy is caller-owned and cannot be overridden by the archive (violates I16); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-MOD-038** — candidates: `utf8-only-refuse-status-quo`, `opaque-bytes-declaration`, `utf16le-code-units`, `wtf8`, `legacy-charset-registry`, `transcode-on-import`, `escape-mapping`, `refuse-with-fidelity-report` (status quo: `utf8-only-refuse-status-quo`). Treatment: Each encoding candidate implemented as a reference decoder in `encoding_models.py`.

## Corpus, fixtures and case sets

- **Scenario generator** (`scripts/chain_corpus.py` and race scenario definitions), per seed (tuning 20260917 `plat014-scenarios-tuning`, validation 20260918 `plat014-scenarios-validation`): symlink-in-parent, directory→symlink swap, junction swap (Windows), root rename, parent rename, bind-mount substitution (Linux), pre-existing destination links, two-link chain `p -> .` and `q -> p/..`, dangling and loop chains, links resolving through other archived links, NTFS case aliasing (`Link -> A/../..` versus `a`), overwrite collisions.
- **Replay inputs:** `f20-tuning-generated-malicious-structures`, `f20-validation-generated-malicious-structures`, `f20-tuning-real-libarchive-testsuite`, `f20-validation-real-cpython-archivetestdata`, `f20-validation-real-commons-compress-testdata`; EB-Z014, Z015, Z016, Z017, Z018, Z022, Z023, Z024, EB-T003, T004, T011 and regenerated T014 (conformance cases, not corpus splits).
- **Encoding fuzz:** 1,000,000 seeded component byte strings per encoding candidate plus targeted forms (UTF-16LE `2E 00 2E 00`, overlong `C0 AE`, U+FF0E, U+2024, trailing-dot variants).
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **Linux:** WSL2 ext4, loop XFS, loop btrfs; injection via strace (`delay_enter`, `error=ENOSYS|EPERM`); `/proc` absent via `unshare --mount --propagation private` and `umount /proc` inside the namespace; write monitoring via `strace -f -y` and `fatrace`.
- **Windows:** NTFS D: non-admin; oplock-paused swaps; MVT-05(a) Windows oracle (sentinel directories with deny-write canaries beside and above the root, post-run tree diff, `GetFinalPathNameByHandle` checks where possible). NTFS symlink cases are HC_UNVERIFIED here (no privilege) and routed to EXP-PLAT-023.
- **Qualifiers:** random-race results are regression evidence (objective MVT-05(b)); timing is irrelevant and not recorded as a metric.

| Requirement | Needed? |
|---|---|
| Linux (WSL2, root) | yes |
| Windows 11 host (non-admin) | yes |
| macOS | no (EXP-PLAT-022) |
| x86-64 | yes |
| ARM64 | no |
| Network | pinned tool downloads only |
| Privileged | WSL root (mount substitution, namespaces, fanotify); Windows non-admin |
| Large storage | no |
| External review | only if a race-safety claim beyond regression evidence is made (§8.3) |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Linux):** `rm -rf /root/eb-research/src/entrybound-plat && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/entrybound-plat && git -C /root/eb-research/src/entrybound-plat checkout <record-commit> && cd /root/eb-research/src/entrybound-plat && CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound` → `/root/eb-research/target/production/release/ebound` (production workspace, no `research-internals`; harness review R1-06).
- **Production CLI (Windows):** `git clone D:/Projects/entrybound/entrybound D:/eb-research/src/entrybound-plat; git -C D:/eb-research/src/entrybound-plat checkout <record-commit>; $env:CARGO_TARGET_DIR='D:/eb-research/target/production-win'; & "$env:USERPROFILE/.cargo/bin/cargo.exe" +1.98.1 build --release -p entrybound-cli --bin ebound` → `D:/eb-research/target/production-win/release/ebound.exe`.
- **Harness (only where named):** `wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh build --release` (→ `/root/eb-research/target/harness/release/`), after `C:/Python313/python.exe research/harness/lock_parity.py` reports `RESULT PASS` (harness review R1-06).
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-014/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-014/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-014/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-014/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-014/spec.yaml'`
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-014/spec-replay.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-014/spec-replay.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-014/spec-replay.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-014/spec-replay.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-014/spec-replay.yaml'`
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-014/spec-encodings.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-014/spec-encodings.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-014/spec-encodings.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-014/spec-encodings.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-014/spec-encodings.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-014/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Orchestrators:** `scripts/confinement_suite.py --backend <b> --scenarios-from <item_id> --arms A,B,C,D,E,G --scratch <dir> --detail-dir <dir>` (Linux) and `scripts/confinement_suite.ps1 -Backend <b> -ScenariosFrom <item_id> -Scratch <dir> -DetailDir <dir>` (Windows) dispatch the arms below and print one JSON line per sample.
- **Arm A (deterministic interleaving):** `scripts/interleave.py --backend <b> --scenario <s> --steps all` counts the extractor's path-resolution syscalls (`openat`, `openat2`, `mkdirat`, `symlinkat`, `linkat`, `fchownat`, `fchmodat`, `utimensat`, `setxattr` family) in a dry run, then for each step index i re-runs with `strace -f -e inject=<syscall>:delay_enter=200000:when=i` while the attacker performs the swap during the delay.
- **Arm B (random races):** `scripts/race_trials.py --backend <b> --scenario <s> --trials 10000 --seed <seed>`.
- **Arm C (chains):** `ebound unpack --symlinks safe|all` on chain archives; `scripts/verify_beneath.py <root>` resolves every created link with openat2 RESOLVE_BENEATH (Linux) or final-path queries (Windows); `planned_tree_resolve.py` evaluates the candidate rules.
- **Arm D (root semantics):** root swap between `create_dir` and open via Arm A step targeting; merge into non-empty roots.
- **Arm E (mode reporting):** `strace -f -e inject=openat2:error=ENOSYS` and `error=EPERM` around `ebound unpack`; parse the `confinement:` status line and compare with the mechanism actually used (strace log).
- **Arm F (replay):** `scripts/replay_cases.py --extractor <e> --item <path> --scratch <dir>` runs `ebound convert --strict|--compat=<profile>` then `ebound unpack`, or the incumbent, under `fs_write_monitor.sh` (fatrace plus tree diff outside the root).
- **Arm G (overwrite TOCTOU):** `ebr-platform-probe overwrite --impl <unlinkat-excl-retry|noreplace-rename|rename-over>` under race attackers.
- **Arm H (encodings):** `scripts/encoding_models.py --encoding <e> --seed <seed> --count 1000000`.

## Seeds, warmups and replication

- **Seeds:** ebr seeds 20260917; scenario seeds 20260917 (tuning) and 20260918 (validation); race trial seeds derived per (backend, scenario) as SHA-256 of the seed and names.
- **Warmups:** 0.
- **Deterministic arms (A, C, D, E, F, H):** 2 repetitions plus repeat-hash of the outcome document (§4.13). Arm A covers every step index once per repetition by construction.
- **Random races (B, G):** 10,000 trials per (backend, scenario); labelled regression evidence (objective MVT-05(b): rule-of-three detection bound assumes a fixed per-trial probability and is not a security bound).
- **Violations:** a single reproducible escape is the violation artifact (objective §2.0 rule 2); both the trace and the resulting tree are committed.

## Measurement method and metrics

- **Escape:** any object created, modified or opened outside the root or through a link, detected by the write monitor, canaries or tree diff (`hc05_mvt05_escapes`).
- **Misreport:** reported confinement mode differs from the mechanism observed in the trace (`hc15_confinement_misreports`).
- **Refusal specificity:** refusals carry a stable reason code naming the violated rule (`reason_code_specificity_fraction`).
- **Hostile names:** replay outcomes per case (`hostile_name_handling_fraction`).
- **Liveness:** extractions that fail to complete under adversarial rename load with bounded retries (secondary `retry_exhaustion_count`).

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `hc05_mvt05_escapes` | OD-25 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-05 MVT-05(a)/(b) | write monitor, canaries, tree diff |
| `hc15_confinement_misreports` | OD-25 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-05/HC-15 (SPEC §11.6 L1436 reporting) | status line versus trace |
| `hostile_name_handling_fraction` | OD-18 | binary R1 screen (M18.3) | correctness | fraction | equal_one | HC-08 (`metric_thresholds.hostile_name_handling_fraction`; A.2 role `binary`) | replay |
| `reason_code_specificity_fraction` | OD-04 | primary for refusal-class decisions (M04.2) | other | fraction | maximize | T-20 (`metric_thresholds.reason_code_specificity_fraction`; A.2 role `banded`) | replay |
| `unsafe_defaults` | OD-25 | binary screen (defaults that write links later followed out of root) | correctness | count | equal_zero | HC-05 (`metric_thresholds.unsafe_defaults`; A.2 role `binary`) | post-extraction census |
| `hc17_encoded_hazards` | OD-18 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-17 MVT-17 | Arm H accepted traversal components |
| `restore_fidelity_fraction` | OD-03 | guard: benign link restores lost by stricter symlink rules (benign chains and EXP-PLAT-003 link tables) | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | Arm C |

## Normalization

Outcomes per trial/case normalized to {contained-success, refused-with-code, escaped, misreported, liveness-failure}. Reference: status quo.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Screens dominate:** any escape or misreport eliminates the candidate for the scope at R1; survivors are compared on benign restore losses (EXP-PLAT-003) and cost (EXP-PLAT-015), then R6 by computed tier (an audited unsafe boundary is at least T3 through §7.2 dependency rules and F-23 applies to workspace crates).

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Scenario arm:** leave-one-scenario-out; **filesystem arm:** ext4/XFS/btrfs separately.

## Held-out evaluation (Phase D placeholder)

Confinement correctness is an HC screen, not a graded selection: after unlock the replay arm runs once on held-out F20 items and on the sealed-seed scenarios (§5.4 sealing route); any held-out escape is an R1 failure under R5 (a).

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- The hardcoded `KernelEnforced` report is expected to be falsified under ENOSYS injection (a defect).
- The lexical Safe check is expected to admit at least one chain that escapes after extraction completes.
- Random-race zero results are not proofs; they are reported with the MVT-05(b) caveat.

## Threats to validity

- strace injection changes timing and may hide races that depend on speed; deterministic step targeting compensates for path-resolution races but not for races inside the kernel.
- Windows oplock-based pausing covers opens that break oplocks, not all resolution steps.
- Prototype backends are research code; a production implementation must repeat the suite (objective §2.0 rule 9).

## Estimated cost and effort

- **Compute:** about 40 machine-hours; **disk:** about 12 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** containment backend prototypes in `ebr-platform-probe` (EXP-PLAT-016): rustix openat2 IN_ROOT/BENEATH(+NO_SYMLINKS, NO_XDEV, bounded EAGAIN retry), manual openat O_NOFOLLOW walk, mount-namespace sandbox launcher, Landlock layer; Windows per-component NtCreateFile reparse-aware walk in the quarantined FFI probe; research/experiments/EXP-PLAT-014/scripts/interleave.py: strace `-e inject=<syscall>:delay_enter=<us>:when=<n>` step targeting plus a synchronized attacker performing the swap at each path-resolution step (black-box on the production binary); Windows oplock attacker `ebr-oplock-attacker` (quarantined research FFI crate: FSCTL_REQUEST_OPLOCK) and canary/tree-diff oracle; research/experiments/EXP-PLAT-014/scripts/{race_trials.py,planned_tree_resolve.py,verify_beneath.py,chain_corpus.py,replay_cases.py,encoding_models.py,fs_write_monitor.sh}; Research III EB-Z/EB-T case files extracted from the hash-verified instrumentation bundle (`research/tools/sources/extract_external_sources.ps1`), EB-T014 regenerated with genuine non-UTF-8 bytes; pinned downloads: CPython 3.13/3.14 builds, Go toolchain, 7-Zip Windows; apt `fatrace` (fanotify write monitor)
- **Agent effort:** tooling: judgment-heavy once (interleaving orchestration and prototype backends need review), then scripted; execution none (about 40 machine-hours, mostly random-race trials); analysis judgment for escape triage.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
