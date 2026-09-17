# EXP-CHUNK-001: CDC algorithm screen at the frozen presets (single- and few-file items)

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CHUNK-001 |
| Domain | chunking (program sections 8.1, 8.2) |
| Status | READY: runs with the existing `ebr-chunk` binary (rebuilt at the pre-registration commit) and the checked-in wrapper `screen_item.py` |
| Stage | Tuning screen (exploratory R3/R4, no multiplicity control; `decision-method.md` §2 R3). It produces no confirmatory verdict. |
| decision_ids | DEC-CMP-001 (CDC algorithm, normalization, masks); DEC-CMP-002 (size presets: realized size per preset) |
| Proposed decision type | EMPIRICAL for both (assignment belongs to an independent session, `decision-method.md` §1.2) |
| Proposed od_ids for the assigner | DEC-CMP-001: OD-05, OD-06, OD-08, OD-10, OD-16, OD-21. DEC-CMP-002: OD-05, OD-06, OD-08, OD-10 (profile-scope constrained form, Appendix B.1) |
| Proposed hc_ids for the assigner | HC-10 (PCR depends on chunking), HC-13 (deterministic boundaries), HC-14 (keyed variants required under encryption), HC-16 (frozen `gear-norm-v1`) |
| Method | `research/decision-method.md` at the commit that adds this file; thresholds from `research/methods/thresholds.json` |
| Gates | G-A (crosswalk, ledger schema v2, look registry) is unmet at this commit. Until it is met this file is a pre-registration draft, and runs are tuning-exploratory only; nothing from this experiment is decision-grade before G-B (§0.4, §14). |
| Author | Phase C-design chunking agent (session of 2026-09-17). It designed the candidate list below and did not implement any harness algorithm. |
| L7 exposure disclosure | As instructed, this session read `research/corpus/coverage.md` and `research/PROGRESS.md`, which name held-out item identifiers and upstream sources. It opened no held-out content, listing, statistic or path under the held-out data roots. This is an identity exposure (event L7, §5.7) for the §5.8(h) record; identifiers are not repeated. Under §5.4 consequence 3 the design review must decide whether an unexposed session re-signs the candidate list and analysis plan. |
| Feasibility smoke (NOT_DECISION_GRADE) | `screen_item.py` ran on a 4-file generated tree (random bytes, one duplicate, one edited copy, one empty file) for 3 candidates: exit 0; one JSON line; `deterministic_sha256` equal across two repetitions. The default harness binary `/root/eb-research/target/harness/release/ebr-chunk` predates harness review round 1: it lacks `mean_chunk_bytes`, so it carries the uncorrected R1-09..R1-11 baselines. This experiment therefore builds its own binary (section 8). |

## 2. Question

At the four frozen production presets (`FAST_V2`, `BALANCED_V2`, `DENSE_V2`, `EXTREME_V2` min/target/max), how do the research CDC algorithms already in `ebr-chunk` compare with `gear-norm-v1` on:

- within-item exact-chunk dedup;
- chunk-size distribution;
- boundary survival under small edits;
- chunk-level random-access amplification?

The comparison runs on tuning items with at most 64 regular files (small and medium tiers), and all algorithms are compared at matched realized mean chunk size.

## 3. Hypotheses

- **H1 (algorithm effect, directional).** Compared at matched realized mean chunk size, `gear-norm-v1` is not worse than the best alternative by more than 1 band unit of T-01 on `dedup_stored_fraction` in most applicable families. At least one alternative (FastCDC NC-2/NC-3 spread masks, or AE) is better by 1 band unit or more on low-entropy or periodic content: F15, F16, and the F20 `gear-extremes` item. Rationale `[INFERRED from docs/chunking-v1.md]`: `gear-norm-v1` cuts on the low `b±1` bits of a left-shifted Gear hash, so each cut decision depends only on the last `b+1` or so bytes.
- **H0 (equivalence).** For every family, the tuning group-mean log ratio of `dedup_stored_fraction` (alternative vs `gear-norm-v1`, matched mean) lies inside the T-01 band.
- **H1 is falsified if** no family has a group-mean ratio beyond the T-01 band in favour of any alternative.
- **Negative control (tool validity, binary).** `fixed-size` must show a median `shift_insert_preserved_boundary_fraction_median` of at most 0.10 on items at least 4 × target long, and every CDC candidate must show at least 0.50. If this fails, the harness or wrapper is defective: the run is invalid and no comparison is reported.
- **Identity control (binary).** `fastcdc-2020-nc2` and `fastcdc-2016-nc2` must give identical `unique_chunk_bytes`, `chunk_count` and `unique_chunk_count` on every item (the 2020 paper claims identical cut points). Any difference is a harness defect.

## 4. Decisions informed and how

| Decision | What this experiment contributes | What it cannot settle |
|---|---|---|
| DEC-CMP-001 | Tuning ranking of existing algorithm constructions on dedup, variance proxy (`max_file_p99_chunk_bytes`), edit stability and chunk-level amplification at the frozen presets. Forms the algorithm survivor set passed to EXP-CHUNK-002 and EXP-CHUNK-004. | Primary `artifact_bytes` (EXP-CHUNK-004); throughput (EXP-CHUNK-008); keyed variants (EXP-CHUNK-006); new constructions (EXP-CHUNK-002); implementability (EXP-CHUNK-012) |
| DEC-CMP-002 | Realized mean chunk size and chunk count per preset and algorithm, and the dedup curve over 4 preset sizes | Off-preset sizes and min/max ratios (EXP-CHUNK-002, EXP-CHUNK-004) |

## 5. Candidates

Every algorithm is sized from the same preset via `--profile`, so candidates within a preset share min/target/max. 12 algorithm variants × 4 presets = 48 candidates.

| Candidate prefix | Definition (ebr-chunk flags) | Role and status | Preliminary cost tier `[INFERRED; computed tier governs, §7.2]` |
|---|---|---|---|
| `gear-norm-v1` | `--algorithm gear-norm-v1` (passthrough to `entrybound::chunker::chunk_ranges`) | **Status quo** and SPEC-proposed design (frozen, `docs/chunking-v1.md`). **Reference candidate** at each preset. | T0 (existing) |
| `fixed-size` | `--algorithm fixed-size` | Measurement baseline and negative control. **Excluded from selection** before execution: SPEC §7.2 L792 rejects fixed blocking for new archives (ledger `candidate_exclusion_reasons`). | n/a |
| `fastcdc-2016-nc1/2/3` | `--algorithm fastcdc-2016 --normalization 1|2|3` (FastCDC Gear table, spread masks, sub-minimum cut-point skipping) | Ledger alternative `fastcdc-2016`; NC levels span the normalization question | T1 (new chunker ID + vectors, C7) |
| `fastcdc-2020-nc2` | `--algorithm fastcdc-2020 --normalization 2` | Ledger alternative `fastcdc-2020`; identity control against 2016 | T1 |
| `rabin-w48` | `--algorithm rabin` (window 48, mean-matched modulo cut, R1-09) | Ledger alternative `rabin-fingerprint` (restic-style) | T1 |
| `buzhash-w64` | `--algorithm buzhash` (window 64) | Ledger alternative `buzhash`. Borg's 4095-byte window is in EXP-CHUNK-002 (L-06). | T1 |
| `ae-w1`, `ae-w8` | `--algorithm ae --value-width 1|8` | Ledger alternative `ae-extremum`, both published calibrations (R1-11) | T1 |
| `ram` | `--algorithm ram` (Widodo et al. fixed-window rule) | Ledger alternative `ram-hashless` | T1 |
| `tttd` | `--algorithm tttd` (backup-last, R1-10) | Ledger alternative `tttd` | T1 |

- **Not in this screen.** Each is covered elsewhere:
  - `gear-spread-mask-nc2`, `seqcdc`, `vectorcdc-simd` (scalar rule) and `chonkers` need new implementations (EXP-CHUNK-002).
  - Incumbent parameterizations (restic, Borg, casync, kopia, Duplicacy, FastCDC paper sizes) are in EXP-CHUNK-002.
- **Excluded, not measured:** `quickcdc-feature-acceptance`. R1 exclusion recorded in the ledger (SPEC §7.3 L803 / §25.1 row 6: boundary acceptance on a partial feature match).
- **R0 item 5 ("defer / not in v1"):** not applicable. A v1 chunker must exist (SPEC §25.1 row 6).
- **R0 item 6 (critic candidate):** open. The design review adds it by amendment before execution.
- **Validation looks:** none in this experiment. W and S for DEC-CMP-001 are declared in EXP-CHUNK-004.

## 6. Corpus

- **Split:** tuning only. Manifest `research/corpus/manifest.json` (`ebrc-2026.09-v1`, manifest_sha256 `ed28b1b4…` at design time).
- **Selection rule (mechanical):** tuning items with scale small or medium and `file_count` ≤ 64. This gives 63 items in 15 families (F05-F16, F18-F20) and 46 independence groups, 6.76 GiB. The item ids are listed in `spec.yaml`. Many-file families (F01-F04, F17) are not screened here. They enter at EXP-CHUNK-002 with the tree tool.
- **Minimum support (§4.9):** tuning, exploratory. ≥ 10 groups across ≥ 5 families is met. Families with one group here (F14 has several; F18 has only `sqlite`) are labelled `single-group`.
- **Generated items:** none beyond the corpus.
- **Held-out:** none. The held-out run of the frozen DEC-CMP-001 selection is a Phase D placeholder under `decision-method.md` §5.5 (Commit A/B/C, `EB_HELDOUT_UNLOCK`), specified in EXP-CHUNK-004.
- **Public tuning data sources (§5.4 consequence 2):** the upstreams of the listed items as recorded in their manifest `provenance`. They are checked against held-out lineages by a held-out-aware session before any validation look.

## 7. Environment and platform requirements

- `env_id` `wsl-ubuntu` (WSL2 Ubuntu 24.04, ext4 under `/root/eb-research`), x86-64.
- No timing claims are made, so the `VIRTUALIZED` qualifier is irrelevant to the metrics used.
- No privileges, network or external review are needed. Large storage is not needed: scratch holds one ≤ 16 MiB prefix copy and symlinks.
- Not run on Windows: the metrics are deterministic functions of input bytes. Cross-host boundary identity is EXP-CHUNK-012's job.

## 8. Commands

```sh
# 0. Preconditions (WSL, repo root /mnt/d/Projects/entrybound/entrybound)
C:/Python313/python.exe research/orchestration/disk_guard.py         # Windows side, must pass
python3 research/harness/lock_parity.py                              # must print RESULT PASS (use constraint 1)
bash research/harness/harness-cli-parity.sh                          # must print RESULT PASS (use constraint 1)
# 1. Build the binary at the pre-registration commit (clean tree), dedicated target dir
CARGO_TARGET_DIR=/root/eb-research/target/exp-chunk bash research/harness/build.sh build --release -p ebr-chunk
CARGO_TARGET_DIR=/root/eb-research/target/exp-chunk bash research/harness/build.sh test -p ebr-chunk
# 2. Run, verify, normalize, report
export PYTHONPATH=research/tools; PY=/root/eb-research/venv/bin/python
$PY -m ebr validate   --spec research/experiments/EXP-CHUNK-001/spec.yaml
$PY -m ebr run        --spec research/experiments/EXP-CHUNK-001/spec.yaml --gzip
$PY -m ebr verify-raw --spec research/experiments/EXP-CHUNK-001/spec.yaml --refingerprint
$PY -m ebr normalize  --spec research/experiments/EXP-CHUNK-001/spec.yaml
$PY -m ebr report     --spec research/experiments/EXP-CHUNK-001/spec.yaml
# 3. Screen analysis (to be written, stdlib + ebr.stats only)
$PY research/experiments/EXP-CHUNK-001/analyze_screen.py \
    --normalized research/normalized/EXP-CHUNK-001 --manifest research/corpus/manifest.json \
    --out research/normalized/EXP-CHUNK-001/decision/screen.csv
```

- `screen_item.py` is checked in with this protocol.
- `analyze_screen.py` is **to be written** before the run is analysed. It implements section 12 and is committed before `normalize` outputs are opened.

## 9. Seeds, warmups, cache

- `seeds: {order: 2026091701, bootstrap: 2026091702, command: 2026091703}`.
- The edit and range seeds are the fixed literal `--seed 20260917` for every candidate, so all candidates see identical edits and ranges (paired). Shift seeds are `20260917000 + s` for s = 1..4.
- Warmups 0: the metrics are deterministic byte counts, not timing (§4.5 applies to timing only). Cache state is irrelevant.

## 10. Metrics

| Metric (spec name) | Canonical? | Role here | Family, unit, direction | Threshold ID | Source |
|---|---|---|---|---|---|
| `dedup_stored_fraction` | yes (M05.4) | diagnostic; screening primary for the chunker component | size, ratio, min | T-01 (`metric_thresholds.dedup_stored_fraction`) | wrapper over `ebr-chunk dedup`: unique chunk plaintext bytes / total logical bytes of all regular files (duplicates included in the denominator) |
| `unique_chunk_bytes`, `unique_chunk_count`, `chunk_count`, `mean_chunk_bytes` | no | secondary (non-canonical; §0.2 item 4) | size/other | none | `ebr-chunk dedup`, `boundaries` |
| `modelled_cost_bytes_frozen_constants` | no | secondary: the frozen v2 proxy (164 B/unique chunk + 40 B/reference), itself under question in DEC-CMP-004 | size, B, min | none | `ebr-chunk dedup` `estimated_cost_bytes` |
| `max_file_p99_chunk_bytes` | no | secondary (size-variance proxy) | other, B | none | `boundaries` |
| `shift_{insert,delete,overwrite}_preserved_boundary_fraction_median`, `shift_*_changed_bytes_median` | no | secondary (boundary stability; no canonical band exists) | other | none | `shift-stability`, 64-byte edits, 4 seeds, first 16 MiB of the largest file |
| `chunk_ra_4k_mean_amplification` | no (chunk-plaintext proxy, not T-22 `access_amplification`, which counts decoded archive bytes including dependencies) | secondary | other, ratio, min | none | `random-access`, 4096-byte ranges, 200 samples |
| `algorithm_id`, `deterministic_sha256` | no | repeat-hash check (§4.13) | correctness, string | §3.3 | wrapper |

- **Primary metrics for DEC-CMP-001 and DEC-CMP-002 decisions** are declared in EXP-CHUNK-004 (`artifact_bytes`, `access_amplification`) and EXP-CHUNK-008 (`encode_cpu_s`, `peak_rss_bytes`), at most one per OD.
- This screen's ranking uses `dedup_stored_fraction` only (a diagnostic role justified because the component under decision is the chunker). The other columns are reported and can trigger review, never elimination.

## 11. Replication and adaptive rule

- 2 repetitions in different processes and rounds (§4.6 deterministic metrics) with the repeat-hash check on `deterministic_sha256`.
- If the two digests of a (candidate, item) cell differ, the cell is a determinism violation artifact (§4.13; objective §2.0 rule 2). Both raw payloads are kept, the candidate is flagged for R1 review, and the infrastructure-fault test of §4.13 is applied.
- No adaptive rule is needed (exact values).

## 12. Normalization and statistical analysis

1. **L1 (item).** Exact values. For each algorithm and item, interpolate `ln dedup_stored_fraction` against `ln mean_chunk_bytes` piecewise-linearly across its 4 presets. Evaluate at `gear-norm-v1`'s realized mean for each preset (use constraint 4, matched realized means). Points outside an algorithm's realized range are labelled `extrapolated` and excluded. The raw same-preset ratio is also reported.
2. **L2-L4.** Item log ratios (alternative / `gear-norm-v1`) are averaged within independence groups, then families (equal group weights). Corpus level uses equal family weights: the cited workload mix (`research/methods/workload-mix.json`, gate G-B) does not exist, so no L4 value here is decision-grade.
3. **Verdicts (informational, tuning).** Floors per §3.1 at item level. Family and corpus Welch-Satterthwaite intervals per §4.10 at 0.99. Verdict classes per §4.11. Labelled `PRE_FREEZE`, `UNADJUSTED`, exploratory.
4. **Tuning survivor rule (pre-registered).** An algorithm variant passes to EXP-CHUNK-002/004 if any of these hold:
   - (a) it is `gear-norm-v1`;
   - (b) it is not dominated under R4 conditions 1, 2 and 4 (exploratory, tuning, no multiplicity control) on `dedup_stored_fraction` at corpus level;
   - (c) it has the best family point estimate in at least one family.

   At most 4 non-status-quo variants pass, ranked by corpus point estimate; ties go to the lower preliminary tier, then the lexicographic id. `fixed-size` never passes.
5. **Negative and identity controls** (section 3) are evaluated first; a failure invalidates the run.

## 13. Practical-significance thresholds

T-01 applies to `dedup_stored_fraction` (relative band only; no absolute floor for this diagnostic metric) exactly as transcribed in `research/methods/thresholds.json` `metric_thresholds.dedup_stored_fraction`. Nothing is restated here. Secondary metrics have no band and produce no verdicts.

## 14. Sensitivity analysis

Exploratory only:

- equal-weight versus byte-weighted aggregation (§6.5 byte-weighted arm);
- leave-one-family-out survivor stability;
- same-preset (unmatched) ratios versus matched-mean ratios;
- the tier-stratified survivor set (small versus medium).

A survivor set that changes under the matched-versus-unmatched arm is reported as a limitation, and both sets pass to EXP-CHUNK-002 (union, cap 6).

## 15. Expected negative results worth recording

- `gear-norm-v1` ties the best alternative within the band on most families: CB-2 bookkeeping for the compression cluster.
- Normalization levels NC-1..3 are indistinguishable on dedup at matched means.
- AE or RAM degenerate (mostly forced-maximum chunks) on low-entropy F15, F16 and F20 content.
- Dedup differences vanish on F11, F12 and F13 already-compressed media (dedup_stored_fraction ≈ 1 for every algorithm).

## 16. Threats to validity

- The chunk-level proxy ignores compression, so dedup gains can shrink after codecs. EXP-CHUNK-004 measures `artifact_bytes`.
- `dedup` reads all distinct files into memory. Items above 512 MiB are excluded, so large-tier behaviour is untested here.
- Shift stability uses only a 16 MiB prefix and 64-byte edits. EXP-CHUNK-003 sweeps edit sizes and positions.
- Realized-mean interpolation over only 4 presets is coarse. EXP-CHUNK-002 uses a 6-point target grid.
- Selection bias: the rule picks many-file-poor families. F01-F04 and F17 are absent.
- Stale binary risk: mitigated by the dedicated build plus `build.sh test`.
- Research implementations of AE, RAM and TTTD are best-effort reproductions (R1-10, R1-11). A poor result may reflect the port, and must not be recorded as a property of the published algorithm without a second implementation.

## 17. Estimates

| Item | Estimate |
|---|---|
| Compute | About 48 candidates × 6.76 GiB × about 4 chunking passes × 2 repetitions ≈ 2.6 TB of chunking at 0.15-1 GB/s. About **10 CPU-hours**; about 3 h wall with 4 concurrent runner shards. |
| Disk | < 2 GB (scratch prefix copies; gzip raw rows) |
| Agent effort | scripted run; judgment only for writing `analyze_screen.py` (small) and interpreting the survivor set |

## 18. Tooling to build

- `research/experiments/EXP-CHUNK-001/analyze_screen.py`: matched-mean interpolation, L2-L4 aggregation, survivor rule. It is the only new tool.

## 19. Deviations

(append-only; UTC time, author, reason, results-visible yes/no)
