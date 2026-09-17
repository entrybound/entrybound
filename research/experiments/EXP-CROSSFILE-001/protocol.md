# EXP-CROSSFILE-001: Similarity method and parameter sweep (cohort quality and complete-cost archive bytes)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: `ebr-crossfile similarity` (`research/harness/ebr-crossfile-DESIGN.md`) is not built |
| Domain / program sections | crossfile / 8.3 (with 8.4 and 8.5 cohort consumers) |
| decision_ids | DEC-CMP-041 (primary); DEC-CMP-006 (cheap-ordering and no-clustering arms only) |
| Evidence type | deterministic byte metrics (decision-method §4.6, §4.13); planning CPU and memory are measured in EXP-CROSSFILE-007 |
| Method | `research/decision-method.md`, pre-registered (commit resolved by the Appendix A.1 command) |
| Record | This protocol is the design. The decision-method §12.2 `record.md` is instantiated from it, and `spec.yaml` re-validated, once gate G-A holds (ledger schema v2 `od_ids`, `hc_ids`, `decision_type` assigned by an independent session; constraint crosswalk signed off; validation-look registry exists). No run may start before that commit. |
| Design session and disclosure | Phase C-design crossfile designer. L7 disclosure: as instructed, this session read `research/corpus/coverage.md` (lists held-out item ids and groups) and `research/PROGRESS.md` (names upstream sources of some held-out items) and saw the directory name `/root/eb-research/heldout` in a listing of `/root/eb-research`. No held-out content, file listing, statistic or fingerprint was opened. Recorded for the §5.8(h) L7 record; the gate audit decides whether an independent countersignature of this design is required (§5.4 item 3). |

## 1. Question

Which similarity method and parameters should form cross-file cohorts so that the cohorts'
downstream dictionary and lookback plans minimise complete-cost archive bytes, compared with the
frozen `bottom-k-shingle-v1` greedy leader clustering, and how much of the achievable gain does
each method's cohort recall and precision explain?

## 2. Hypotheses

- **H1 (directional).** On the dense profile, at least one alternative in the pre-registered set
  (expected: content-defined sampling or one-permutation MinHash-LSH at matched sketch budget)
  reduces corpus-level `artifact_bytes` against `sq-bottomk-v1` by at least 1 band unit of T-01
  in the cross-file target families (F01, F03, F04, F05, F06, F17, F18). Predicted effect: 0.5
  to 2 band units at corpus level, larger (2 to 5 band units) on F18 and F04; the gap to the
  `oracle-exact-jaccard` ceiling is predicted to exceed 1 band unit on at least one family.
- **H1b (mechanism).** For Chunks of 256 KiB to 8 MiB, `bottom-k-shingle-v1` (fixed 16-byte
  stride, at most 4,096 scanned shingles, so only about the first 64 KiB is sampled) retains a
  lower sketch intersection after a 1-byte or 16-byte insertion than content-defined sampling.
- **H0 / equivalence.** Every alternative is `EQUIVALENT` to `sq-bottomk-v1` on `artifact_bytes`
  at corpus level on validation; R10 then retains the status quo (lower tier, status quo key).
- **Falsification of H1.** On tuning, no alternative's corpus-level ratio point estimate lies
  beyond the T-01 band on the favourable side, or on validation every confirmatory comparison is
  `EQUIVALENT` or `DISTINGUISHABLE_WORSE`. H1b is falsified if content-defined sampling's median
  sketch retention is not above bottom-k's on the 256 KiB+ Chunk stratum.

## 3. Decisions informed; proposed OD and HC sets

The binding `od_ids`, `hc_ids` and `decision_type` come from ledger schema v2 (gate G-A). The
designer's proposal, used to pre-declare primary metrics, is:

| Decision | Proposed type | Proposed od_ids | Proposed hc_ids |
|---|---|---|---|
| DEC-CMP-041 | EMPIRICAL | OD-05, OD-06, OD-08 | HC-02, HC-06 (MVT-06(d) writer CPU on hostile input), HC-13, HC-16 |
| DEC-CMP-006 (arms here) | EMPIRICAL | OD-05 (here); OD-06, OD-12 in EXP-CROSSFILE-002/-007 | HC-09, HC-13, HC-16 |

If the assigned OD set differs, the record adds or removes primary metrics before execution;
an OD in the assigned set that this family of experiments leaves unmeasured is listed as a gap
(R3).

## 4. Candidates

Every non-status-quo candidate is a **new** similarity policy identifier and planner identifier
(HC-16, objective §4.4; `CONTRIBUTING.md` frozen v3 similarity). Cohort bounds are the frozen
profile bounds unless the candidate varies them. Cohort selection, dictionary training, lookback
candidates, cost rule and encoding are production (v6 running the frozen v5 stages).

| candidate_id | Definition | Profiles | Expected cost tier (to be computed, §7.2) |
|---|---|---|---|
| `sq-bottomk-v1` | Status quo: FNV-1a-64 over 32-byte shingles, stride 16, <= 4,096 scanned, bottom-k, greedy leader cohorts, frozen k/threshold/min/max/cap per profile | balanced, dense, extreme | reference (T0 status quo) |
| `bk-k{4,8,16,32,64}-t{125,250,375,500,625,750}` | Bottom-k grid (30 points) at the profile's frozen bounds and cap | dense (grid); balanced, extreme at their frozen k with t grid only | T1 (new planner ID) |
| `bk-bounds-min{2,3,4,8}` / `-max{16,32,64,128,256}` / `-cap{32,64,128,256,1024}` | One-factor-at-a-time around the frozen dense and extreme points | dense, extreme | T1 |
| `bk-scan{16384,unbounded}-stride{8,16,32}` | Remove or raise the 4,096-shingle scan cap and vary stride (large-Chunk recall) | dense | T1 |
| `minhash-oph{64,128}-lsh-b{8,16,32}` | One-permutation MinHash with optimal densification; banded LSH with (bands x rows) tuned to thresholds 0.25/0.375/0.5; same greedy leader assignment and bounds | dense | T1 |
| `minhash-khash-64` | k independent 64-bit hashes (k = 64), exact Jaccard estimate, bucketed candidate search | dense | T1 |
| `simhash-64-h{3,6,10}` | 64-bit SimHash over shingle hashes; Hamming-distance buckets (pigeonhole tables) | dense | T1 |
| `tlsh-t{30,60,100}` | TLSH digests (reference definition at a pinned commit), distance threshold | dense | T1 |
| `nilsimsa-dwarfs` | Nilsimsa digests with DwarFS v0.15.7 recursive similarity ordering; cohorts are maximal adjacent runs within a Nilsimsa distance cut | dense | T1 |
| `sf-finesse-3x4`, `sf-ntransform-3x4` | Super-feature resemblance (Finesse; N-transform), cohort when >= 1 super-feature matches | dense | T1 |
| `cds-gear-m{6,8}-k{8,16,32}` | Content-defined shingle sampling: shingles anchored where a gear hash masked by 2^m-1 is zero, no scan cap, bottom-k over anchored shingles | dense | T1 |
| `cheap-type-path` | No sketches: cohorts are contiguous runs of Chunks grouped by (extension category, parent directory), ordered by size, cut at the profile's cohort max (SPEC balanced cheap ordering) | balanced, dense | T1 |
| `no-clustering` | No cohorts (digest order; no dictionaries or groups) | balanced, dense, extreme | T1 (a policy change) |
| `oracle-exact-jaccard` | Reference ceiling only: exact shingle-set Jaccard over all Chunk pairs sharing a shingle, greedy leader at the same bounds | dense | not a selectable candidate |
| `critic-TBD` | Slot for at least one candidate proposed by the design-review critic (R0 item 6), added before execution | per critic | per critic |

- **Reference candidate:** `sq-bottomk-v1` for each profile.
- **R0 coverage:** status quo; SPEC-proposed (`cheap-type-path` for balanced, full similarity for
  dense/extreme = status quo); every ledger-named alternative (MinHash-LSH, SimHash, Nilsimsa,
  TLSH, super-features, content-defined sampling, cheap ordering, no clustering); strongest
  incumbent technique (DwarFS Nilsimsa ordering, measured as a candidate here and as a whole
  incumbent in EXP-CROSSFILE-005); defer (= keep status quo, no new ID); critic slot.
- **Excluded before execution:**
  - Any in-place change to `bottom-k-shingle-v1` or the v3-v6 planner IDs: L0 / R1, HC-16
    (published identifiers immutable). Counterexample: `docs/cross-file-compression-v1.md`
    frozen v3 table; reviewer sign-off pending (objective §2.0 rule 3).
  - `oracle-exact-jaccard` as a product candidate: R2 (no CPU-per-input-byte bound computable
    from declared parameters; all-pairs work is superlinear on many-similar inputs). It is
    measured only as the headroom ceiling.
  - Candidates whose creation-time computation uses floating point are admissible only if the
    determinism MVT (objective MVT-13(a), repeat-hash here) passes; TLSH and SimHash are
    implemented integer-only.

## 5. Corpus selectors

- **Manifest:** `research/corpus/manifest.json`, `manifest_sha256`
  `ed28b1b47333c186a8c80781d378b4c149eaaceea09a65098182f4c062380392`. Tuning split only in
  this spec; no held-out selector exists anywhere in this design.
- **Stage A and B (screening and complete cost), `spec.yaml`:** all 50 tuning items of scale
  small or medium in families F01, F02, F03, F04, F05, F06, F08, F17, F18, F19, the small F07
  items, and the three F20 screening items (`f20-tuning-gear-extremes`,
  `f20-tuning-bombs-dense`, `f20-tuning-csprng`), listed explicitly in `spec.yaml` (about
  4.07 GiB, 44 family-group pairs). Selection rule: manifest `split = tuning`,
  `scale != large`, family in the list above, F07 only `scale = small`.
- **Stage C (finalists, family and tier guards):** every tuning item of every family (small,
  medium and large), for `sq-bottomk-v1` and the tuning survivor set S; its spec
  (`spec-stage-c.yaml`) is generated from the Stage B outcome by the rule "status quo plus every
  candidate not epsilon-dominated on tuning (R4 exploratory)" and committed before it runs.
- **Validation look (at most 2 per decision, §5.2):** validation split, every family, small and
  medium, plus the large items `f01-validation-cpython-3-13-15-full-clone`,
  `f03-validation-bat-cargo-vendor`, `f18-validation-cpython-3-12-point-releases`,
  `f06-validation-noaa-ghcn-daily-csv-2023`; only W and S; only after the MDE gate (§4.12)
  passes on tuning variance; registered in `research/decisions/validation-looks.jsonl`.
- **Minimum support check (§4.9):** validation small+medium has at least 2 groups in 17 of 20
  families (well above 10 groups across 5 families); single-group families are labelled.
- **Items expected to fail pack for every candidate:** `f19-tuning-zoo` (non-UTF-8 names are
  refused by `ebound pack`, `research/baselines/README.md`). Such samples are recorded as
  `SOURCE_REFUSED`, excluded from OD analysis identically for all candidates, and reported.
- **Public tuning data sources** declared per candidate for the §5.4 item 2 same-upstream check:
  the upstream projects of the items above (manifest `provenance`), plus the DwarFS and TLSH
  reference sources used to implement candidates (code, not data).

## 6. Environment and platform requirements

- `env_id` `wsl-ubuntu` (WSL2 Ubuntu 24.04, ext4 under `/root/eb-research`), x86-64, Linux.
  Deterministic byte metrics only, so no timing guard and no `VIRTUALIZED` qualifier applies
  to the verdicts; `timing: false`.
- No network, no privilege, no Windows, no macOS, no ARM64 requirement. Cross-host determinism
  of cohorts is part of EXP-CROSSFILE-009/-010.
- Scratch: per-sample archives in the runner scratch on ext4; memo cache under
  `/root/eb-research/cache/crossfile/EXP-CROSSFILE-001/rep{rep}`.
- Prerequisites: harness release build at the pre-registration commit;
  `python research/harness/lock_parity.py` and `research/harness/harness-cli-parity.sh`
  print `RESULT PASS`; `ebr-crossfile` validity controls 1-3 and 5 pass
  (`ebr-crossfile-DESIGN.md` §4); `research/orchestration/disk_guard.py` passes.

## 7. Commands

```sh
# inside WSL, repository at /mnt/d/Projects/entrybound/entrybound (specs and code only; data on ext4)
cd /mnt/d/Projects/entrybound/entrybound
export PYTHONPATH=research/tools
PY=/root/eb-research/venv/bin/python
$PY -m ebr validate   --spec research/experiments/EXP-CROSSFILE-001/spec.yaml
$PY -m ebr plan       --spec research/experiments/EXP-CROSSFILE-001/spec.yaml
$PY -m ebr run        --spec research/experiments/EXP-CROSSFILE-001/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-CROSSFILE-001/spec.yaml
$PY -m ebr normalize  --spec research/experiments/EXP-CROSSFILE-001/spec.yaml
$PY -m ebr report     --spec research/experiments/EXP-CROSSFILE-001/spec.yaml
# decision analysis: decision-method §14 item 2 tooling (not built; planned location research/tools/decide).
# Its invocation is fixed in record.md when the tool exists; it reads research/normalized/EXP-CROSSFILE-001/
# and writes research/normalized/EXP-CROSSFILE-001/decision/ (§12.1).
```

Per-sample command (from `spec.yaml`):

```text
/root/eb-research/target/harness/release/ebr-crossfile similarity
  --item-path {item_path} --item-id {item_id} --experiment-id EXP-CROSSFILE-001 --env-name wsl-ubuntu
  --profile {profile} --method {method} --method-params {method_params}
  --ground-truth {ground_truth} --archive-out {scratch}/out.eb
  --memo-dir /root/eb-research/cache/crossfile/EXP-CROSSFILE-001/rep{rep} --verify-roundtrip true
```

Sharding: Stage B is split into disjoint item shards run as separate `ebr run` invocations with
distinct `--run-id` values (no timing, so shards may run concurrently on the 32 vCPUs); the
runner's randomized block order is kept within each shard.

## 8. Seeds, warmups, repetitions

- Seeds: `order` 20260917001, `bootstrap` 20260917002, `command` 20260917003. Ground-truth pair
  sampling on items with more than 20,000 unique Chunks uses a 10% Chunk sample seeded by
  SHA-256(`EXP-CROSSFILE-001 gt\0` || item_id).
- Warmups: 0 (deterministic metrics).
- Repetitions: 2 in different rounds and processes, plus the §4.13 repeat-hash check on
  `archive_sha256`. A candidate that claims determinism (every candidate here) and shows two
  hashes is an R1 elimination without a confirming rerun (§4.13), unless an infrastructure fault
  is proven.
- Screening economy (pre-registered): Stage A quality metrics are computed in the same run.
  Stage B covers the grid on dense only; balanced and extreme run the frozen point, the
  threshold grid, `cheap-type-path`, `no-clustering` and Stage C finalists.

## 9. Metrics

| Metric | OD | Role in this record | ebr family | Threshold | Source |
|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | **primary** (superiority and non-inferiority) | size | T-01 | `payload.artifact_bytes`, exact, reconciled by `ebr_pack::bytes` |
| `dictionary_bytes` | OD-05 | diagnostic | size | T-02 | `payload.dictionary_bytes` |
| `side_data_bytes` | OD-05 | diagnostic (ChunkGroup and plan records) | size | T-02 | `payload.side_data_bytes` |
| `encode_cpu_s` | OD-06 | primary, measured in EXP-CROSSFILE-007 | time_cpu | T-05 | EXP-CROSSFILE-007 |
| `alloc_peak_bytes` (planning) | OD-08 | primary, measured in EXP-CROSSFILE-007 | memory_peak | T-06 | EXP-CROSSFILE-007 |
| `pair_recall_j{250,375,500}`, `pair_precision_j{250,375,500}` | - | secondary (not canonical; mechanism only) | other | none (no band: secondary for every decision, §0.2 item 4) | `payload.*` |
| `sketch_retention_insert_{1b,16b,256b}` | - | secondary (H1b) | other | none | `payload.*` |
| `cohort_count`, `cohorts_dictionary_selected`, `cohorts_lookback_selected`, `cohorts_rejected`, `rejected_candidate_encodes` | - | descriptive (false-group cost) | other | none | `payload.*` |
| `roundtrip_ok`, `drift_ok` | HC-02 / validity | binary gate | correctness | §3.3 | `payload.*` |
| `archive_sha256` | HC-13 | repeat-hash string metric | correctness | §4.13 | `payload.archive_sha256` |

At most one primary metric per OD (R3). OD-06 and OD-08 primaries are declared here and measured
in EXP-CROSSFILE-007 on the same candidates; a candidate eliminated at R1/R2 here does not enter
007.

## 10. Normalization and statistical analysis

- Correctness gating: samples with `roundtrip_ok != 1` or a failed validity control are HC
  violation records, excluded from OD analysis, never averaged.
- Item level (L1): exact values after the repeat-hash check; paired log ratio against
  `sq-bottomk-v1` of the same profile; T-01 absolute floor per tier (small tier relative only).
- L2-L4: group mean, family mean with equal group weights, corpus mean with base weights from
  `research/methods/workload-mix.json` (gate G-B; until it exists no L4 value is
  decision-grade); strata per scale tier.
- Tuning (exploratory): R4 without multiplicity control forms S and W (continuous band-unit
  utility, §6.2); the Stage B to Stage C rule of §5 above.
- Validation (confirmatory): W against each s in S on `artifact_bytes` (and the 007 primaries),
  corpus level, Welch-Satterthwaite interval (§4.10) at `1 - 0.01/k_sup` for superiority and
  0.99 two-sided for non-inferiority (§4.12); family harm guard at 0.90 pooled (§4.9); R4
  condition 5 with computed tiers (§7.2); R6 minimum gain for tiers above the status quo (a new
  planner ID is T1, k = 2); R8 to R10.
- MDE gate before the validation look: every confirmatory MDE <= 1 band unit from tuning
  group-level variance and the validation group structure, else redesign (more groups) or the
  decision stays `INSUFFICIENT_EVIDENCE`.
- Mechanism analysis (secondary, cannot select): Spearman correlation between pair recall at the
  profile threshold and item-level `artifact_bytes` log ratio; headroom = status quo versus
  `oracle-exact-jaccard` per family.

## 11. Practical-significance thresholds

Referenced, never restated: T-01 (`metric_thresholds.artifact_bytes`), T-02
(`metric_thresholds.dictionary_bytes`, `side_data_bytes`), T-05
(`metric_thresholds.encode_cpu_s`, per profile), T-06 (`metric_thresholds.alloc_peak_bytes`),
minimum-gain multipliers §7.2/§7.3 (Appendix D.6). Secondary quality metrics have no band.

## 12. Sensitivity analysis

decision-method §6 in full at the selected scope: weight grid and Dirichlet over the assigned
ODs (or, if DEC-CMP-041 is classified profile-scope, Appendix B.1 constrained form with no
weights), SMAA whole-simplex report, threshold perturbation with the resolvability condition,
equal-weight arm, leave-one-family-out, leave-three-families-out, family Dirichlet, WM-* mixes
(expected to matter: WM-DIST and WM-BACKUP contain the target families), tier-stratified,
byte-weighted (`total_stored_bytes`), selection-stability bootstrap (2,000 replicates, seed
20260917004). Experiment-specific arms: (a) ground truth on the full Chunk set versus the 10%
sample for items where both are feasible; (b) `f19-tuning-zoo` and other `SOURCE_REFUSED` items
excluded versus counted as ratio 1.

## 13. Validation look and held-out placeholder

- Validation look spec `spec-validation-look1.yaml`: generated from W and S, committed with the
  record addendum (W, S, superiority metrics per pair, MDEs) before the look.
- **Held-out (Phase D placeholder):** `EXP-CROSSFILE-001-HO`, written at Commit A of the single
  program-wide design freeze (decision-method §5.5), runs every candidate that reached the freeze
  on the held-out split exactly once after Commit C, with `EB_HELDOUT_UNLOCK` equal to the
  freeze SHA; held-out MDE <= 2 band units computed from validation variance and held-out group
  counts only (§5.4); report-only on failure (§5.6); carries `identity_aware_heldout` unless a
  sealed tranche is used. No held-out selector is part of this pre-registration.

## 14. Expected negative results worth recording

- No alternative beats bottom-k by a band at corpus level (similarity quality is not the
  bottleneck; the cohort cost rule is).
- High pair recall does not translate into bytes because most extra cohorts fail the 128-byte
  gain rule (false-group cost without gain).
- `cheap-type-path` matches full similarity for balanced (supports SPEC's cheap ordering) or is
  distinguishably worse on F18 (contradicts it).
- Bottom-k's scan cap loses cohorts for >= 256 KiB Chunks with insertions but the byte effect is
  within band because such Chunks rarely win dictionary or lookback plans.
- F20 many-similar inputs drive cohort search to the candidate cap on every method.

## 15. Threats to validity

- Research-path planning differs from production: controlled by validity control 1 (byte
  identity for the status quo) on every item.
- Memoisation hides nondeterminism within a repetition: memo is per repetition, and the second
  repetition recomputes all cohorts.
- Planner-ID string lengths differ for new IDs: the exact length difference is recorded and
  removed from `artifact_bytes` comparisons (a new ID would ship with its own string length;
  both values are reported).
- Ground-truth sampling on large Chunk sets biases recall estimates: secondary metrics only.
- Corpus composition is curated (`critique-round2.md` conditions: G02 partial DB dumps, F03/F17
  covariates); the §6.5 arms and per-family reporting expose it.
- Identity-aware held-out threat model and this session's L7 disclosure (header).
- Similarity reimplementations may be strawmen: each must cite its primary reference definition
  at a pinned version and pass the reference's published test vectors where they exist (TLSH);
  the harness review checks this like R1-09..R1-11 for CDC.

## 16. Cost

- Compute: about 300 CPU-hours (Stage A+B about 230 over the 116 screening candidates in `spec.yaml`, Stage C about 70), wall about 15-20 h
  sharded over 24 vCPUs; the smoke measurement in this design session (non-decision-grade:
  about 0.3 MB/s for dense and 0.1 MB/s for extreme on 3.7 MB of 300 small files) sets the
  order of magnitude. Disk: scratch peak about 3 GB, memo cache about 5 GB, raw JSONL about 1 GB.
- Agent effort: none for execution (scripted); scripted for tooling boilerplate; judgment for
  similarity-method implementations, analysis, and the critic candidate.
