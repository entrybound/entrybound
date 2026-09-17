# EXP-PLANNER-005: Chunking-policy selection rule, parameter granularity and per-profile chunk lists

| Field | Value |
|---|---|
| Domain | planner (program §9, §10) |
| Index status | NEEDS_TOOLING (TP-04 `chunking-matrix` including `--count-only`, TP-05, TP-03 `fit-constants`, TP-15; look-1 guards need TP-07, TP-08, runner additions and calibration) |
| Document state | Phase C design; `record.md` after gate G-A |
| Shared conventions | `research/planner/README.md` |
| Runner specs | `spec.yaml` (chunking × codec-stage × granularity matrix on PSUB-256 tuning sub-items); `spec-chunkcount.yaml` (chunk-count headroom on full tuning items) |

## 1. Pre-registration header

- **decision_ids:** DEC-CMP-004 (selection rule among chunking candidates), DEC-CMP-003 (parameter granularity), DEC-CMP-002 (per-profile chunk-size candidate lists; the planner consequence of chunk sizes), DEC-MOD-039 (size and dedup impact of uniform against per-category chunking; PCR divergence joined with EXP-PLANNER-001).
- **Decision type (proposed):** EMPIRICAL. For DEC-MOD-039, verified-range soundness when the chunker id is absent is FORMAL and belongs to the model domain.
- **Proposed `od_ids`:** OD-05 (superiority), OD-06, OD-07, OD-08, OD-10 (guards); OD-23 (declaration encoding cost, a C1 input). **Proposed `hc_ids`:** HC-02, HC-06 (the chunk-count limit of 4,000,000 at `archive.rs:41` as a declared bound), HC-10 (PCR binds chunker parameters), HC-13, HC-16 (frozen FAST/BALANCED/DENSE/EXTREME_V2 policies are never redefined; new policies need new chunker and planner IDs).
- **Frame:** profile scope, constrained form (Appendix B.1).
- **Author / L7:** README §10.

## 2. Question and hypotheses

- **Question.** Given each profile's codec stage, which chunking policy list, which rule for choosing among its candidates, and which granularity (archive-wide, per category, per size class, per object) gives the smallest real `artifact_bytes` within the guards? How much does the frozen plaintext proxy mis-select?
- **H1.**
  1. The status-quo proxy picks the end-to-end-best policy in at least 80% of sub-items in dense and extreme. Its corpus-level regret against end-to-end selection is below 1 band unit.
  2. `corrected-constants-by-format-version` is `NON_INFERIOR` to end-to-end selection.
  3. The modelled upper-bound gain of every per-category, per-size-class or per-object granularity fails the T4 minimum gain (identity participation change, §7.2) at corpus level, while exceeding it on at most two families.
  4. The 2K/8K/64K class breaches the chunk-count limit on at least one large tuning item (R2 elimination) and is not `NON_INFERIOR` on `artifact_bytes` elsewhere, because index and plan overhead dominate.
  5. `single-global-policy` costs more than 1 band unit in dense and extreme against their best lists.
- **H0.** Every selection rule and granularity is `EQUIVALENT` to the status quo.
- **Falsification.** H1.1 fails if agreement is below 80% or the regret interval lies wholly above 1 band unit. H1.2 fails if not non-inferior at look 1. H1.3 fails if any granularity's modelled upper bound clears the T4 band at corpus level. H1.4 fails if no breach occurs and the class is non-inferior. H1.5 fails if the global policy is within 1 band unit.

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CMP-004 | Modelled against actual container bytes per unique Chunk and per reference (v6 INDEXED and STREAM); how often each rule picks a different policy, and the size delta; work units of 1-4 candidate evaluations; regret against end-to-end selection | Timing of end-to-end selection: look-1 guard in EXP-PLANNER-007 windows |
| DEC-CMP-003 | Per-category against archive-wide Pareto from the matrix; cross-category dedup loss; categoriser determinism and work; descriptor/PCR encoding cost (C1 counter); regret of staged against per-object selection | Categoriser accuracy against format ground truth: descriptive here; conformance complexity: container and conformance domains |
| DEC-CMP-002 | End-to-end archive bytes per policy per profile codec stage; access granularity per policy; chunk-count headroom on F15/F16 large items; the min/max ratio and window-matched variants | Dedup ratio, boundary stability and CDC algorithm sweeps against restic, Borg, casync and zpaq: chunking domain (§8.1) and baselines (EXP-BASE-SIZE); remote random-read latency by policy: remote domain (§12) |
| DEC-MOD-039 | Size and dedup cost of uniform chunking across profiles (`global-single` arm); multi-chunker declaration bytes per candidate encoding | PCR divergence frequency: EXP-PLANNER-001; verified-range soundness: model domain (formal) |

## 4. Candidates

### Chunking policy universe U (research-only policies need new chunker IDs, T1)

| policy id | min / target / max | Origin |
|---|---|---|
| FAST_V2 | 512K / 2M / 8M | frozen status quo |
| BALANCED_V2 | 128K / 512K / 2M | frozen status quo |
| DENSE_V2 | 64K / 256K / 1M | frozen status quo |
| EXTREME_V2 | 32K / 128K / 512K | frozen status quo |
| C-FASTCDC-SMALL | 2K / 8K / 64K | DEC-CMP-002 `fastcdc-reference-small` |
| C-CASYNC-64K | 16K / 64K / 256K | `casync-zpaq-64k` |
| C-BACKUP-1M | 512K / 1M / 8M | `backup-incumbent-1to2m` (the 2M variant equals FAST_V2) |
| C-LARGE-4M | 1M / 4M / 16M | `large-4m` |
| V-MIN8-MAX2 | 64K / 512K / 1M | `min-max-ratio-variants` around BALANCED_V2 |
| V-MIN8-MAX8 | 64K / 512K / 4M | same |
| V-MIN2-MAX2 | 256K / 512K / 1M | same |
| V-MIN2-MAX8 | 256K / 512K / 4M | same |
| W-FAST-1M | 256K / 1M / 1M | `window-matched-max` (maximum capped at the 1 MiB Zstandard window) |
| CD-n | from the chunking domain's tuning frontier | reserved slots; entered only before planner look 1, with the addition logged (R0) |

Chunker constraints (`chunker.rs:359-373`): the target is a power of two, and 0 < min ≤ target ≤ max. Every row satisfies them.

**Codec stages.** U's first eight rows run under all four profile codec stages. V-* and W-* rows run under balanced and extreme. The stage is the profile's v6 planner applied to forced chunk ranges (TP-04).

### DEC-CMP-004 selection rules (dense list {BALANCED_V2, DENSE_V2}; extreme list {FAST_V2, BALANCED_V2, DENSE_V2, EXTREME_V2}; also applied to every candidate list of DEC-CMP-002 below)

| candidate_id | Definition |
|---|---|
| status-quo-plaintext-proxy (reference) | unique plaintext bytes + 164 B per unique Chunk + 40 B per reference; the earlier candidate wins ties |
| corrected-constants-by-format-version | Same form, with per-unique and per-reference constants fitted by integer least-absolute-deviation regression of (`artifact_bytes` − `chunk_payload_bytes`) on (unique count, reference count) over tuning v6 archives, separately for INDEXED and STREAM |
| end-to-end-complete-cost-extreme | Pack under each candidate and keep the smallest real `artifact_bytes`; extreme only |
| end-to-end-complete-cost-all | The same for every multi-candidate profile |
| sampled-compressed-proxy | One sixteenth of unique Chunks, in SHA-256 order, compressed with the profile's cheapest non-STORE table codec, scaled with integer arithmetic, plus status-quo record constants |
| fixed-policy-no-selection | dense → DENSE_V2; extreme → EXTREME_V2 |
| weighted-objective | Status-quo proxy + max Chunk bytes / 64 (an integer access penalty); fixed a priori, not fitted |

### DEC-CMP-003 granularity

| candidate_id | Definition | Materializable |
|---|---|---|
| status-quo-archive-wide-per-profile (reference) | one policy per archive from the profile list | yes |
| spec-per-category | TP-05 category → policy from the profile's list, fitted on tuning by end-to-end cost per category | research-only multi-chunker archive plus modelled declaration bytes → **modelled-only** |
| per-file-size-class | Buckets: < 64 KiB, < 16 MiB, ≥ 16 MiB → policy, fitted on tuning | modelled-only |
| per-contentobject-selection | Each ContentObject takes the list policy with the smallest single-object end-to-end cost; realized jointly | modelled-only |
| global-single | One policy for every profile and archive; the best single on tuning (status-quo tables unchanged) | yes (new chunker/planner IDs) |
| dwarfs-style-categorizer | Fixed a priori mapping: incompressible and media → FAST_V2; executable → BALANCED_V2; everything else → the profile's own selection | modelled-only |

### DEC-CMP-002 per-profile lists

| candidate_id | Definition |
|---|---|
| status-quo-four-policies (reference) | frozen lists |
| each single class of U | the profile uses that one policy |
| single-global-policy | as `global-single` |
| adaptive-by-archive-size | target = the smallest power of two in {8K … 4M} ∩ U-targets such that total logical bytes / target ≤ 2^20; min and max from the U row with that target (BALANCED_V2-shaped ratios when several exist) |
| window-matched-max | W-FAST-1M replaces FAST_V2 in fast |
| min-max-ratio-variants | V-* rows replacing BALANCED_V2 in balanced |

- **Excluded:** none at design. Policies with a target that is not a power of two are not proposed; the chunker constraint would refuse them.
- **Expected tiers:** new policy or rule is T1 (new chunker or planner ID); multi-chunker granularities are T4 (identity participation assignment: PCR binding of several chunker declarations, DEC-MOD-039); `split` declaration encodings are C1 counted.

## 5. Corpus

- **Matrix:** PSUB-256 tuning sub-items, all 20 families, in planner look 1 for the confirmatory cells (W ∪ S only).
- **Chunk-count arm:** **full** tuning items (not sub-items), every item. Counting-only CDC passes (TP-04 `--count-only`) under every U policy, reporting archive-level `logical_chunk_references` and unique counts against the 4,000,000 limit. The count is analytically exact for the chunker and does not depend on planning.
- **Strata:** parent tier; TP-05 category.
- **Weights:** `workload-mix.json`.

## 6. Environment and platform

`wsl-ubuntu` x86-64 for deterministic cells. The look-1 encode guard (end-to-end selection work) is timed in EXP-PLANNER-007 windows on WSL `st-v` and Windows `st-p`. No network, privileges or external review.

## 7. Commands

```sh
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-PLANNER-005/spec.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-PLANNER-005/spec-chunkcount.yaml
for s in spec.yaml spec-chunkcount.yaml; do
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-PLANNER-005/$s --refingerprint
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-PLANNER-005/$s
done
python3 research/planner/tools/replay.py fit-constants --normalized research/normalized/EXP-PLANNER-001 --out research/normalized/EXP-PLANNER-005/replay/constants.json
python3 research/planner/tools/replay.py chunking-rules --matrix research/normalized/EXP-PLANNER-005 --constants research/normalized/EXP-PLANNER-005/replay/constants.json \
  --out research/normalized/EXP-PLANNER-005/replay/
```

## 8. Seeds, warmup, replication

Seeds: order 260917051, bootstrap 260917052, command 260917053; selection bootstrap 260917994. Deterministic: 0 warmups, 2 repetitions, repeat-hash. Fits (constants, category maps, size classes) use tuning only and are frozen before look 1.

## 9. Metrics

| Metric | OD | Role | Family | Threshold | Source |
|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | primary superiority | size | T-01 | TP-04 cell (materialized for archive-wide and global; research-only archive plus modelled declaration bytes for multi-chunker rows) |
| `access_granularity_bytes` | OD-10 | guard | size | T-09 | declared access cost |
| `encode_wall_s` | OD-06 | guard; work units on tuning, timed at look 1 | time_wall | T-03 per profile | TP-07 |
| `decode_wall_s` | OD-07 | guard | time_wall | T-04 | look 1 |
| `alloc_peak_bytes` | OD-08 | guard | memory_peak | T-06 | TP-08, look 1 |
| `index_bytes`, `metadata_bytes`, `metadata_bytes_per_entry` | OD-05 | diagnostic | size | T-02, T-02b | TP-04 byte accounting |
| `dedup_stored_fraction` | OD-05 | diagnostic | size | T-01 | TP-04 |
| proxy error per unique Chunk and per reference; rule agreement rate; cross-category duplicate bytes | – | descriptive | other | none | TP-03 |
| archive chunk count against 4,000,000 | HC-06 | binary R2 screen | correctness | exact | TP-04 `--count-only` |
| `roundtrip_mismatches`, repeat-hash | OD-02, HC-13 | binary | correctness | §3.3 | TP-04 |

## 10. Normalization and statistical analysis

- **R2:** any full tuning or validation item whose archive chunk count exceeds 4,000,000 under policy x eliminates x for every profile scope containing that item's tier. The limit is recorded as the bound.
- **Selection rules (DEC-CMP-004):** per profile with a multi-candidate list, the rule's chosen policy per sub-item maps to the real `artifact_bytes` of that matrix cell. Comparisons against end-to-end selection and among rules follow B.1 with look-1 confirmation. Agreement rate and proxy residuals are descriptive.
- **Granularity (DEC-CMP-003) and DEC-MOD-039:** the modelled-only rows use R6 elimination. If the modelled corpus-level gain interval against the reference does not lie beyond the T4 minimum-gain band (`cost_tiers.multipliers.T4` on T-01), the row is eliminated (README §7). A row that passes cannot be selected; the decision stays `INSUFFICIENT_EVIDENCE`, with an implementation recommendation for a materializable candidate. `global-single` is a normal T1 candidate.
- **Per-profile lists (DEC-CMP-002):** B.1 per profile, R3-R10, with the access-granularity guard mattering most for small-chunk rows.
- **Aggregation, intervals and confidences:** §4.9-§4.12 via the thresholds.json `statistics` and `bootstrap` keys.

## 11. Sensitivity analysis

- §6.4 threshold perturbation.
- §6.5 arms: equal weights, leave-one-family-out, leave-three-families-out, family Dirichlet, WM-* mixes, tier-stratified (important: small-chunk rows are tier-sensitive), byte-weighted.
- §6.7 bootstrap.
- Layout arm: INDEXED against STREAM constants for the corrected-constants rule.
- Categoriser arm: `spec-per-category` recomputed with each TP-05 category merged into `unknown` in turn.

## 12. Expected negative results worth recording

- The frozen proxy rarely mis-selects: end-to-end evaluation buys under one band and is not justified.
- Per-category chunking's best-case gain is below the T4 minimum gain, a negative for SPEC §7.3's per-category requirement. It is recorded as counterevidence to the SPEC.
- Small-chunk classes lose on index overhead and breach the chunk limit.
- Uniform chunking across profiles costs little, which supports the DEC-MOD-039 `uniform-chunking-across-profiles` candidate.

## 13. Threats to validity

- PSUB-256 limits dedup opportunities on large items. Dedup-heavy families (F17, F18) are the most affected, which is reported per tier.
- Research-only multi-chunker archives mis-declare their chunker. Their bytes plus modelled declaration costs are an approximation, labelled modelled-only.
- The categoriser is research code (TP-05) with no production equivalent.
- The chunking domain's frontier additions (CD-n) may arrive late. Additions after results are visible enter as logged additions (R0) and are secondary for that look.
- Designer L7 exposure.

## 14. Cost and effort

- **Compute:** the matrix has 8 × 4 + 5 × 2 = 42 chunking × stage cells per sub-item plus 5 granularity cells × 4 profiles (62 runner candidates), on PSUB-256 tuning × 2 repetitions: about 180 CPU-hours, dominated by extreme and dense stages. The chunk-count arm is about 5 CPU-hours. Look-1 W ∪ S validation cells are about 50 CPU-hours.
- **Disk:** about 3 GB raw; about 20 GB transient.
- **Agent effort:** scripted tooling and execution; judgment for the analysis write-up.

## 15. Gates and status

- **Tooling:** TP-03 (`fit-constants`, `chunking-rules`), TP-04, TP-05, TP-15; look 1 needs TP-07, TP-08 and `make_timing_spec.py`.
- **Gates:** G-A, G-B, cost scripts and rule simulation before look 1, runner additions and calibration for guards, DEC-CMP-035 bounds before `DECIDED`.
- **Held-out:** EXP-PLANNER-012.
