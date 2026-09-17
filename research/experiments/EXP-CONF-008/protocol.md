# EXP-CONF-008 — Fuzz target inventory, harness modes and coverage-adequacy criterion calibration with injected defects

| Field | Value |
|---|---|
| Status | `NEEDS_TOOLING` (CC§2): cargo-fuzz crate `research/harness/fuzz` with the target inventory, digest-bypass patch, injected-defect patch sets, campaign manager, coverage checkpoint replayer, reason-code reach replayer |
| Domain / program sections | conformance / §30 (primary) |
| Decision IDs informed | DEC-ECO-028, DEC-ECO-029, DEC-CRY-017 |
| Requirements | REQ-ECO-0053, REQ-ECO-0054, REQ-ECO-0134 |
| HC oracles | objective M24.2 (`unresolved_fuzz_findings_at_coverage_target`) defines the campaign stop rule this experiment calibrates; proposal for the assigner: `hc_ids` HC-03, HC-06; `od_ids` OD-24, OD-04 |
| Decision type | not assigned (gate G-A) |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1 |
| Depends on | EXP-CONF-004 (seed cases; reason-code set per boundary from EXP-CONF-003 step 5) |

## 1. Question

For each attacker-facing parsing boundary, which coverage-adequacy criterion among the ledger candidates of DEC-ECO-028,
and which harness mode (production checks on, or digest/CRC bypass in fuzz builds), would stop a campaign at a point
that has detected the most pre-registered injected defects per CPU-hour, reproducibly across seeds, and with the widest
reach of decided reason codes?

## 2. Hypotheses and falsification

- **H1 (bypass reach, REQ-ECO-0053).** For digest-gated targets, bypass mode detects injected defects located behind
  digest or CRC checks that verify-on mode does not detect within the same CPU budget. *Falsified* if, at the 4 CPU-hour
  checkpoint, verify-on mode detects every such defect that bypass mode detects on every seed.
- **H2 (criterion stability).** At least one criterion defined for every calibration target stops within a factor of 2
  of its median stop time on all three seeds. *Falsified* if no criterion does.
- **H3 (crash-only blind spot).** Defects of class D3 (wrong outcome class without a crash) are not detected by
  single-reader crash fuzzing within budget. *Falsified* by detection of a D3 defect without a class oracle (this is a
  negative-result hypothesis: confirming it motivates EXP-CONF-010's differential oracles).

## 3. Target inventory (pre-registered; exact entry functions are resolved from the public API by the tooling builder and printed in each target's header)

| Target | Boundary (REQ-ECO-0054) | Digest-gated | Calibration subset |
|---|---|---|---|
| T01 | preamble, Descriptor v1/v2, feature bitmaps | yes | — |
| T02 | INDEXED full open and verify from memory | yes | ✓ |
| T03 | canonical records and sequences (manifest TLV decoder) | yes | — |
| T04 | STREAM sequential reader from a Read-only source | yes | ✓ |
| T05 | Index parse and random-access open | yes | ✓ |
| T06 | random entry and range reads on the opened archive | yes | — |
| T07 | HTTP range response handling (status, `Content-Range`, ETag, multipart) against an in-process fake origin | no | — |
| T08 | encrypted public framing: sentinels, CONTROL directory, recipient stanzas (no key) | no | ✓ |
| T09 | encrypted full open with a public test password at minimum KDF cost: EBPO/EBCS private objects, fragment reassembly | yes (AEAD) | ✓ |
| T10 | encrypted range opener | yes (AEAD) | — |
| T11 | signature records: embedded and detached `.ebsig` parse and verify | no | — |
| T12 | RFC 3161 timestamp verification (DER/CMS tokens) | no | ✓ |
| T13 | codec and transform decode dispatch with parameter records (zstd, lz4, lzma2, delta8, byte-shuffle) under declared window limits | yes (chunk digest) | ✓ |
| T14 | DEFLATE reconstruction v5 decode | yes | — |
| T15 | JPEG reconstruction v6 decode | yes | — |
| T16 | POSIX metadata records, xattrs, POSIX ACL xattr projection, sparse maps | no | ✓ |
| T17 | platform security metadata (Windows security descriptors, ACEs, reparse data) | no | — |
| T18 | LogicalPath decode per declared encoding and hazard checks | no | — |
| T19 | identifier strings: `chunker_id`, planner id, codec and transform ids | no | — |
| T20 | ZIP adapter observation (strict) | yes (CRC) | ✓ |
| T21 | ZIP compatibility-profile projections (each frozen profile) | yes (CRC) | — |
| T22 | tar adapter observation (ustar, pax, GNU) | no | ✓ |
| T23 | 7z adapter observation and folder decode | yes (CRC) | ✓ |
| T24 | compressed-stream wrappers: detection and decode of gzip, xz, bzip2, zstd | yes (CRC) | ✓ |
| T25 | ConversionProvenance and preservation records (types 28-36) | no | — |
| T26 | `inspect`/`diff` library report generation on untrusted archives | yes | — |

Calibration subset (12 targets, stratified across container, crypto, codec, metadata and legacy boundaries): T02, T04,
T05, T08, T09, T12, T13, T16, T20, T22, T23, T24. All 26 targets run in EXP-CONF-009.

## 4. Candidates

- **Adequacy criteria** (ledger candidates of DEC-ECO-028, operationalized as stop rules evaluated on recorded run
  traces; CC§1: none added; the R0 item 6 critic slot is open):
  - `status-quo-none`: no stop rule; reported only.
  - `r1-80-percent-tar-zip`: stop when line coverage of the target's parser modules reaches 80%; defined for T20, T22
    only (recorded `UNDEFINED` elsewhere).
  - `per-boundary-region-thresholds`: stop when region coverage reaches 0.95 × |U_b|, where U_b is the union of regions
    reached by all calibration runs of that target (all modes, builds, seeds) plus the seed corpus.
  - `coverage-plateau-criterion`: stop when no new edge appears for W CPU-hours, W ∈ {0.5, 1, 2} (the 4 CPU-hour runs
    cannot evaluate larger W; W = 4 and 8 are evaluated on the long runs).
  - `reason-code-reachability`: stop when every reason code that EXP-CONF-003's census attributes to that boundary is
    reached by the corpus (replayed through a code-recording build); `NEVER` if not reached within the run.
  - `cpu-hours-budget-only`: fixed budgets {1, 4} CPU-hours (and {16, 72} on the long runs).
  - `digest-bypass-harness-mode`: evaluated as the harness-mode factor (§5), not as a stop rule.
- **Engine candidates of DEC-ECO-029** executed here: `cargo-fuzz-libfuzzer-per-boundary` and
  `structure-aware-arbitrary-generators` (T02, T04, T13 each get an `Arbitrary`-driven variant building valid-then-mutated
  archives); `afl-plus-plus-campaigns` is run on T20 and T22 only (AFL++ built from source at a pinned tag via
  `cargo-afl`, same budget) as an engine comparison; `oss-fuzz-integration` is BLOCKED (EXP-CONF-014);
  `internal-differential-fuzzing`, `incumbent-differential-fuzzing` → EXP-CONF-010, EXP-CONF-011;
  `regression-promotion-ledger`, `property-based-tests` → EXP-CONF-009, EXP-CONF-010.

## 5. Harness modes and builds

- **M-ON**: `ebound-fuzz` with production checks.
- **M-BYPASS**: `ebound-fuzz` plus `research/harness/fuzz/patches/bypass.patch`, which under `#[cfg(fuzzing)]` (set by
  cargo-fuzz only) skips comparison of chunk, section, footer and legacy CRC digests (still computing them) and, for
  T09/T10, treats authenticated-private plaintext as given (fuzzing the private-object grammar directly). The patch is
  applied only to the scratch source copy (CC§4).
- **Clean build** and **defect build** per (target, mode). The defect build applies
  `research/harness/fuzz/defects/<target>.patch`, which injects five defects, each behind `#[cfg(fuzzing)]` and each
  emitting a unique panic or abort signature `EBCC-DEFECT-<target>-D<k>` when triggered:
  - D1 allocation sized from an untrusted length without its budget check (detected as OOM under `-rss_limit_mb=2048`
    or as the signature);
  - D2 off-by-one in a length validation (panic);
  - D3 wrong outcome class on a deep rejection branch, no crash (detectable only with a class oracle; §2 H3);
  - D4 a panic placed on a path reachable only after a digest or CRC check passes;
  - D5 a loop whose iteration count is attacker-controlled without bound (timeout under `-timeout=10`).
  Defect locations are chosen by a session that does not run or analyse the campaigns, from the target's parse code, and
  committed (patch SHA-256) before any run; each patch is checked to compile and to leave the clean seed corpus passing.

## 6. Seeds and inputs

- Seed corpora: every valid `ebcc-v1` case applicable to the target; `ebound-head` deterministic packs of the 14 small
  tuning items of EXP-CONF-002 §5.1 (format targets); for legacy targets the small and medium tuning items
  `f20-tuning-real-libarchive-testsuite`, `f20-tuning-generated-malicious-structures`, `f14-gen-nested-archives-py`,
  `f14-apache-maven-3916-bin-tarball`, `f11-alpine-324-apks`, `f14-legacy-writer-matrix-tuning`, `f14-jenkins-25683-war`,
  and scratch regenerations of `tools/zip-compat` and `tools/tar-strict` cases (CC§7). Files larger than 1 MiB are split
  into members or skipped by `seed_prep.py` (recorded). No validation or held-out input.
- libFuzzer seeds: `2809171001`, `2809171002`, `2809171003`.

## 7. Environment and commands

WSL2 Ubuntu 24.04 x86-64; toolchain `nightly-2026-08-01`, cargo-fuzz 0.13.2, `--sanitizer address`; libFuzzer fork mode
`-fork=4 -ignore_crashes=1 -ignore_ooms=1 -ignore_timeouts=1 -rss_limit_mb=2048 -timeout=10`, 4 CPU-hours per run =
1 wall hour with 4 workers; 8 runs in parallel (32 vCPUs). Must not overlap timing windows (CC§3).

```sh
bash research/harness/fuzz/tools/prepare_source.sh --commit <ebound-head sha> --out /root/eb-research/src/conf-fuzz-<sha12>
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CONF-008/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-008/spec.yaml        # raw id EXP-CONF-008
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-008/spec-long.yaml   # raw id EXP-CONF-008-LONG
python3 research/harness/fuzz/tools/criteria_eval.py --exp EXP-CONF-008 --criteria research/harness/fuzz/criteria-v1.json
```

Each sample (`campaign.py`) builds (cached per source SHA, mode, defect set), runs one campaign, replays the corpus
snapshot at checkpoints {0.25, 0.5, 1, 2, 4} CPU-hours through a coverage build (`cargo +nightly-2026-08-01 fuzz
coverage`, `llvm-cov export -format=text` region and line counts) and through a reason-code recording build, records
first-detection times of each defect signature, and prints the summary. Long runs: T02, T13, T20 × M-ON × defect build
× seed `2809171001` extended to 72 CPU-hours with checkpoints every 4 CPU-hours.

## 8. Replication

Three seeds per (target, mode, build); no warmup; campaign traces are inherently stochastic, so every criterion is
evaluated per seed and summarized across seeds; no repeat-hash applies to fuzzing traces (§4.13 is for deterministic
metrics), but coverage replays of a fixed corpus snapshot are repeat-hashed.

## 9. Metrics (CC§9)

| Quantity | Role | Unit |
|---|---|---|
| `decided_class_reach_fraction` per target at each checkpoint and at each criterion's stop point | descriptive (M04.3) | fraction |
| Injected-defect detection at a criterion's stop point (detected ÷ injected, per class D1-D5) | descriptive, non-canonical | fraction |
| CPU-hours to stop; stop-time ratio max/min across seeds; criterion defined (binary) | descriptive, non-canonical | h, ratio, binary |
| Region and line coverage curves; edges (`cov:`) over time | descriptive, non-canonical | count |
| Findings in clean builds (crash, panic, OOM, timeout) | carried into EXP-CONF-009 triage; counted there as `unresolved_fuzz_findings_at_coverage_target` | count |

**Metric gap.** Appendix A.2 has no banded canonical metric for injected-defect detection, so under §0.2 item 4 these
quantities are secondary for every decision. The design review is asked to add `injected_defect_detection_fraction`
(T-20 coverage fraction over a pre-registered defect set, `a = 0.5 / n_defects`) before any result exists; if it is not
added, DEC-ECO-028 can be decided only through its R1/R2 aspects (criteria undefined for some targets are unusable as a
universal rule) and R10.

## 10. Analysis

For each criterion c and mode m: per target and seed, the stop point s(c, m); detection fraction and reason-code reach at
s; CPU-hours at s; stability across seeds. Pre-registered comparison: a criterion is **dominated** if another criterion,
defined on every calibration target, has detection fraction ≥ and CPU-hours ≤ on every target (median over seeds), with
at least one strict inequality. The non-dominated set, the H1 bypass comparison per digest-gated target, and the D3
results are reported. No confidence intervals are computed on three seeds; per-seed values are shown. The analysis plan
needs adoption by a non-exposed session (CC§1).

## 11. Practical-significance thresholds

None applies (descriptive quantities; §9 metric gap). The adequacy rule adopted for M24.2 remains the objective's
pre-registered form (coverage target or 72 CPU-hour ceiling) until DEC-ECO-028 is decided.

## 12. Sensitivity analysis

- U_b computed without the defect builds; region threshold 0.9 and 0.99 instead of 0.95.
- Detection counting OOM and timeout signatures only when the defect signature is present in the stack.
- Engine comparison libFuzzer versus AFL++ on T20 and T22.
- Structure-aware variants versus byte-level variants on T02, T04, T13.

## 13. Split discipline and held-out placeholder

Seeds from tuning items and generated cases only (CC§7). Fuzzing has no validation or held-out role; the adopted
criterion is confirmed by applying it to the 14 non-calibration targets in EXP-CONF-009 (a pre-registered out-of-sample
check), not by a corpus split.

## 14. Expected negative results worth recording

- Crash-only fuzzing missing wrong-class defects (H3).
- The 80% line-coverage rule undefined or unreachable for format targets.
- Plateau criteria stopping before defects behind digest checks are reached in verify-on mode.
- Reason codes unreachable from any fuzzed input (dead or mis-attributed codes).

## 15. Threats to validity

- **Injected defects are synthetic** and placed by one session; real defects may be distributed differently.
- **Fork-mode interaction**: several defects in one build can mask each other's reach; detection time per defect is
  recorded, and the clean build provides uncontaminated coverage curves.
- **Three seeds** give limited stability evidence.
- **Coverage replays** at checkpoints lose within-interval ordering (edges are attributed to the checkpoint).
- **Sanitizer choice**: AddressSanitizer on safe Rust mainly affects native dependencies (libzstd); throughput differs from
  `--sanitizer none`, which is noted but not varied.

## 16. Cost estimates

| Item | Estimate |
|---|---|
| Compute | 900 CPU-hours: libFuzzer 40 configurations (8 digest-gated targets × 2 modes × 2 builds, 4 other targets × 1 mode × 2 builds) × 3 seeds × 4 h = 480 h; AFL++ arm 3 configurations × 3 seeds × 4 h = 36 h; long runs 3 × 72 h = 216 h (`spec-long.yaml`); coverage replays and builds ≈ 170 h. ≈ 28 wall hours on 32 vCPUs |
| Disk | 40 GB (corpora snapshots, coverage exports, builds) |
| Agent effort | **scripted** campaigns; **judgment, low**: defect placement (one uninvolved session), criteria evaluation review |
| Platform | Linux x86-64 (WSL); CPU-saturating, never concurrent with timing windows; network once for AFL++ source |

## 17. Outcome obligations

`outcome.json` (§10.1). Findings in clean builds are handed to EXP-CONF-009's triage and regression ledger; the
calibration table is evidence for DEC-ECO-028 and DEC-ECO-029, and the target inventory with its acceptance-item mapping
(crypto targets T08-T12 against `docs/crypto-review-v1.md` fuzzing acceptance items) is evidence for DEC-CRY-017.
