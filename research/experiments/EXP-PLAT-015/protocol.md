# EXP-PLAT-015: Containment and hazard-detection cost at depth, breadth and link count (timing)

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **NEEDS_TOOLING**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-005, DEC-PLT-041, DEC-PLT-048, DEC-MOD-010
- **Program sections:** 15, 16, 21
- **Decision type (proposed, not assigned):** EMPIRICAL (timing). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-07 (M07.1 decode wall/CPU as extraction cost); HC-05 floors come from EXP-PLAT-014 (a backend that escapes is not timed for selection).
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** What does each containment backend cost per extracted entry at path depths up to 1,024 and on many-small-file trees on ext4 and NTFS, and what do collision/hazard detection and planned-tree symlink resolution cost at large entry and link counts, relative to the status quo?
- **H1:** Per-component openat2 resolution costs less than a userspace no-follow walk at depth ≥ 64 on ext4 by more than the T-04 band, cap-std is within the band of openat2 on Linux, NTFS per-component opens dominate extraction time at depth ≥ 256; Unicode full folding plus NFC detection adds less than the T-04 band on F04 many-small-file items; planned-tree resolution with a hop bound stays within the band until link counts exceed 10^5.
- **H0 / equivalence:** All backends and detectors are EQUIVALENT to the status quo within the T-04 band on every item and ladder point.
- **Falsification of H1:** No DISTINGUISHABLE difference between openat2 and the no-follow walk at depth 1,024, or Unicode detection DISTINGUISHABLE_WORSE on F04 items.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-005 | V1_BLOCKING | Which containment primitive does Entrybound extraction (and capture) use on Linux, macOS and Windows, given cap-std 4.0.3 uses openat2 RESOLVE_BENEATH only on Linux (w... | Per-component cost of each containment backend at depth and breadth on ext4 (VIRTUALIZED) and NTFS. | Correctness screens (EXP-PLAT-014). |
| DEC-PLT-041 | V1_BLOCKING | Should the Safe symlink policy remain a per-link lexical check, or resolve each target against the planned extracted tree including other archived symlinks (and case-i... | Cost of planned-tree resolution for large link counts. | Correctness (EXP-PLAT-014); compatibility cost (EXP-PLAT-003). |
| DEC-PLT-048 | V1_BLOCKING | Which collision classes must extraction detect (case-fold, NFC/NFD normalization, Windows reserved aliases, trailing dot/space, 8.3 short names, confusables), against... | Detection cost for large entry counts per rule set. | Accuracy (EXP-PLAT-002/003). |
| DEC-MOD-010 | V1_IMPORTANT | Should the ArchiveDescriptor carry path hostility and collision-class summaries, and if so are they advisory caches or validated authorities, with which enumerated cla... | Cost of reader-side hazard computation at large entry counts (the alternative to a stored summary). | Prevalence (EXP-PLAT-003). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** `statusquo-capstd-inprocess` (parity-checked against production `ebound unpack`; harness review R1-06: decision-grade timing uses production builds or demonstrated parity).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-005** — candidates: `cap-std-4-status-quo`, `rustix-openat2-in-root-no-symlinks`, `manual-openat-nofollow-walk`, `macos-o-resolve-beneath`, `windows-ntcreatefile-reparse-walk`, `mount-namespace-or-chroot-sandbox`, `landlock-seccomp-complement`, `refuse-unsupported-platforms` (status quo: `cap-std-4-status-quo`). Treatment: Timed as a backend or detector configuration of the in-process benchmark; status quo additionally timed as production `ebound unpack`.
  - excluded before execution (ledger): `string-prefix-validation` — SPEC §11.6 Pattern 1 forbids string validation followed by ordinary file operations; R1 and R2 classify prefix checking as a defect class (violates SPEC §11.6 L1413 frozen extraction doctrine); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-041** — candidates: `lexical-per-link-status-quo`, `planned-tree-resolution`, `no-symlink-components-in-targets`, `post-extraction-physical-verify`, `case-insensitive-aware-resolution`, `refuse-all-symlinks-default` (status quo: `lexical-per-link-status-quo`). Treatment: Timed as a backend or detector configuration of the in-process benchmark; status quo additionally timed as production `ebound unpack`.
- **DEC-PLT-048** — candidates: `no-detection-status-quo`, `exclusive-create-only`, `pinned-unicode-full-casefold-nfc`, `probed-target-rules`, `filesystem-specific-tables`, `conservative-superset-plus-probe`, `confusables-advisory`, `confusables-refuse`, `enable-per-directory-case-sensitivity` (status quo: `no-detection-status-quo`). Treatment: Timed as a backend or detector configuration of the in-process benchmark; status quo additionally timed as production `ebound unpack`.
- **DEC-MOD-010** — candidates: `not-implemented-status-quo`, `validated-summary`, `advisory-cache`, `reader-computed-only`, `extractor-target-specific` (status quo: `not-implemented-status-quo`). Treatment: Timed as a backend or detector configuration of the in-process benchmark; status quo additionally timed as production `ebound unpack`.

## Corpus, fixtures and case sets

- **Items:** `f04-tuning-sourcelike`, `f04-tuning-real-netbsd-pkgsrc`, `f04-tuning-maildir`, `f03-tuning-typescript-npm-node-modules`, `f04-validation-real-gentoo-repo-20260915`, `f04-validation-objstore`, `f03-validation-bootstrap-npm-node-modules`, `f19-validation-deep`, `f19-tuning-zoo` (tuning and validation).
- **Generated ladders** (seeded 20260917 tuning, 20260918 validation): depth ladder directories, breadth ladder (10^2–10^6 entries in one directory), link-count ladder with chains of length {1, 8, 40}.
- **Minimum support:** timing verdicts are family-scoped (F03, F04, F19) plus generated ladders; no corpus-level L4 claim is made (§4.9 support is not met by three families).
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **WSL:** `st-v` {2} and `mt-v-8` {2..9} (§4.3), ext4 under `/root/eb-research`, `hot` cache (warmups read inputs), qualifier VIRTUALIZED (MEDIUM soft cap for Linux-native performance claims, §4.1).
- **Windows:** `st-p` {2} (§4.2), NTFS D: outside the repository, AC power required, power-throttling opt-out, drift canary, PDH frequency covariate, `defender_on` label and MsMpEng covariates.
- **Guard:** thresholds.json `quiet_machine_guard` with spec tightening `max_loadavg_1m = 1.0 + threads`, `max_wait_s = 300` (§4.4); exclusive timing windows (no agents, builds or provisioning).
- **Calibration prerequisite:** EXP-CAL-AA/AAP/KE/BG for `time_wall` and `time_cpu` in both environments must have passed (§4.7) before any decision-grade run.

| Requirement | Needed? |
|---|---|
| Linux (WSL2) | yes (VIRTUALIZED timing) |
| Windows 11 host | yes (non-admin; AC power) |
| macOS | no |
| x86-64 | yes |
| ARM64 | no |
| Network | no |
| Privileged | no (loop mounts not needed; ext4 native) |
| Large storage | about 35 GB |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Linux):** `rm -rf /root/eb-research/src/entrybound-plat && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/entrybound-plat && git -C /root/eb-research/src/entrybound-plat checkout <record-commit> && cd /root/eb-research/src/entrybound-plat && CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound` → `/root/eb-research/target/production/release/ebound` (production workspace, no `research-internals`; harness review R1-06).
- **Production CLI (Windows):** `git clone D:/Projects/entrybound/entrybound D:/eb-research/src/entrybound-plat; git -C D:/eb-research/src/entrybound-plat checkout <record-commit>; $env:CARGO_TARGET_DIR='D:/eb-research/target/production-win'; & "$env:USERPROFILE/.cargo/bin/cargo.exe" +1.98.1 build --release -p entrybound-cli --bin ebound` → `D:/eb-research/target/production-win/release/ebound.exe`.
- **Harness (only where named):** `wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh build --release` (→ `/root/eb-research/target/harness/release/`), after `C:/Python313/python.exe research/harness/lock_parity.py` reports `RESULT PASS` (harness review R1-06).
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **Timing runs:** `ebr run --spec research/experiments/EXP-PLAT-015/spec.yaml --timing --cpus 2` (WSL) and `--spec spec-windows.yaml --timing --cpus 2` (Windows) inside scheduled exclusive windows, then `verify-raw`, `normalize`, `report`, and the §14 item 2 decision tooling.
- **Benchmark:** `ebr-platform-probe bench-extract --backend <b> --detector <d> --item <path> --scratch <dir> --timer in-process` materializes the verified item into a fresh destination and reports in-process wall and CPU for planning, detection and materialization separately; `--parity` mode runs production `ebound unpack` on the same input for the parity check.

## Seeds, warmups and replication

- **Seeds:** order/bootstrap/command 20260917; ladder seeds as above.
- **Warmups:** §4.5 — 2 rounds for small and medium items, 1 for large, recorded and excluded.
- **Rounds:** §4.6 tiers (small 10/10/30, medium 10/10/20, large 10/5/20), minimum also ≥ n_min(c) for the comparison confidence (Appendix D.4).
- **Adaptive rule:** after the minimum rounds, add one step for any item whose exact order-statistic median interval half-width exceeds 0.5 × log1p(r) (§4.6), never looking at effect direction; items still imprecise at the cap are flagged `imprecise`.
- **Order:** `randomized_blocks` with pairing inside rounds (`pairing: item_repetition`).
- **Parity check:** before comparisons, `statusquo-capstd-inprocess` and production `ebound unpack` must be EQUIVALENT on `decode_wall_s` (in-process versus process floors reconciled), otherwise all in-process comparisons are informational.

## Measurement method and metrics

- **Timers:** in-process timers around planning/detection and materialization (T-04 in-process floor); whole-process `decode_wall_s` for the production parity arm (process floor).
- **Correctness gating:** each sample verifies content and entry count (objective §3.0); failing samples are excluded and reported.
- **Covariates:** per §4.2/§4.3.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `decode_wall_s` | OD-07 | primary (extraction cost; in-process timer excluding verification, process timer for the parity arm) | time_wall | s | minimize | T-04 (`metric_thresholds.decode_wall_s`; A.2 role `banded`) | benchmark / ebr resource |
| `decode_cpu_s` | OD-07 | secondary | time_cpu | s | minimize | T-05 (`metric_thresholds.decode_cpu_s`; A.2 role `banded`) | benchmark / ebr resource |
| `roundtrip_mismatches` | OD-02 | binary gating | correctness | count | equal_zero | HC-02 (`metric_thresholds.roundtrip_mismatches`; A.2 role `binary`) | benchmark |

## Normalization

Paired per-round log ratios against the reference within each (environment, affinity, item) (§4.9 L1); no cross-environment pairing (§4.1).

## Statistical analysis

- **Item level:** exact order-statistic interval of the median paired log ratio (§4.10) at the comparison's confidence; verdicts per §4.11.
- **Aggregation:** L2 group, L3 family; ladders reported per ladder point (never pooled). No L4 claim (support rule, §4.9).
- **Confirmatory set:** W versus S on validation items with k_sup-adjusted confidence (§4.12); MDE gate before the look using tuning variance and the A/A item half-widths (§4.7).
- **Rules:** only candidates that pass EXP-PLAT-014 screens are compared (R1 precedes R3); R4 with the family/tier guard; R6 by computed tier; environment scope requires same-direction support in both environments for a cross-platform claim (§4.1).

## Practical-significance thresholds

`decode_wall_s` and `decode_cpu_s`: T-04/T-05 (`metric_thresholds.decode_wall_s`, `metric_thresholds.decode_cpu_s`), including the in-process versus process floor rule and the small-tier rule (§3.2). No other band is used.

## Sensitivity analysis

- §6.4 threshold perturbation (×0.5 only if the A/A half-width at the cap is ≤ 0.25 × log1p(r), else recorded as not resolvable).
- §6.5 environment arm (Windows versus WSL), tier-stratified arm, leave-one-family-out.
- §6.7 selection-stability bootstrap over groups.
- Affinity arm: `st-v`/`st-p` versus `mt-v-8` reported separately (no cross-configuration pairing).

## Held-out evaluation (Phase D placeholder)

After unlock the timing specs run once on held-out F03/F04/F19 items with the frozen backends; held-out MDE ≤ 2 band units per supporting comparison is stated in Commit A (R5).

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Timing differences may be inside the T-04 band everywhere; a null result supports choosing on correctness and cost tier alone (R10), with power demonstrated.
- NTFS results may be dominated by Defender scan-on-open, which is part of the realistic cost and labelled.

## Threats to validity

- WSL timing is VIRTUALIZED and vCPU placement is uncontrolled (§4.1); Linux claims are soft-capped.
- In-process prototypes may differ from a production implementation; parity is shown only for the status-quo backend.
- Page cache warmth changes directory lookup cost; `hot` state is declared and `vm-cold` is informational only.

## Estimated cost and effort

- **Compute:** about 36 machine-hours; **disk:** about 35 GB peak (transient scratch unless stated).
- **Timing required:** yes.
- **Tooling to build:** in-process extraction-planning and materialization benchmark `ebr-platform-probe bench-extract` with pluggable containment backends and detectors (EXP-PLAT-014/016 prototypes) plus a timing-parity check of its cap-std backend against production `ebound unpack`; runner additions of decision-method §14 item 3 (affinity, cooldown, drift canary, frequency covariate, host-side sampler, invalid-rate balance); calibration experiments per environment and family (§14 item 4: EXP-CAL-AA/AAP/KE/BG/SPD, owned by the evaluation domain); generated depth ladder {1, 16, 64, 256, 1024} and link-count ladder {10^2 .. 10^6} built inside the command from seeds
- **Agent effort:** execution none (unattended timing windows, about 36 machine-hours); tooling judgment once (benchmark fairness review), then scripted; analysis judgment.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
