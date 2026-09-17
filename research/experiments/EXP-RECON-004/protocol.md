# EXP-RECON-004: Archive-level net-savings ablation — planner generations, reconstruction filters and gates, chunk alignment, audit activation (REAL archives) and exact-cost-modelled backends and subsets

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-RECON-004 |
| Domain | reconstruction (8.8); common provisions: `research/experiments/_index/reconstruction-common.md` |
| Status | **NEEDS_TOOLING**: `ebr-recon variants`, `ebr-recon model`, per-chunk byte accounting, and research planner and chunker overrides in default-off research features (§8) |
| Stage | **Tuning** (exploratory R1–R4: forms W and S per decision and profile), then **validation look 1** (confirmatory W vs S, R4–R10) for DEC-CMP-018, 026, 023, 042, 012 (reconstruction slice) and DEC-CON-010 (activation) |
| decision_ids | DEC-CMP-018, DEC-CMP-026, DEC-CMP-023, DEC-CMP-042, DEC-CMP-012, DEC-CON-010, DEC-CON-005, DEC-CMP-016, DEC-CMP-038, DEC-CON-007 |
| Proposed types/ODs/HCs | common §2. Profile-scope constrained form (Appendix B.1) for "which profiles" (DEC-CMP-018, 026). Non-profile decisions (backend and subset choice in DEC-CMP-026/018, DEC-CMP-042, DEC-CMP-023) propose OD-05, OD-06, OD-07, OD-08, OD-10 with equal base weights. HCs: HC-02 (round trip of every variant), HC-09 (`declared_cost_accuracy`), HC-11/R2 (gated steps), HC-13 (repeat-hash), HC-16 (frozen v3–v6 IDs reproduced by the variants tool: identity controls). |
| Gates | G-A for the record. G-B (decision tooling, frozen held-out lock, `workload-mix.json`) before any verdict is decision-grade. MDE gate (§4.12) before the look. Computed cost tiers (EXP-RECON-011) before the look (§14 item 7). |
| Author and exposure | common §3 |
| Feasibility smoke | none (tool not built) |

## 2. Question

- **Artifact bytes and access.** For every item, per profile, what are the exact `artifact_bytes` and access declarations of each archive-level candidate: planner generations v3–v6, v6 with reconstruction selections filtered, gated or audit-free, alignment variants, and modelled backends or subsets?
- **Verdicts.** Under decision-method R1–R10, which reconstruction design per decision and scope survives and is selected when compared against the no-reconstruction reference and against each other? Guards come from EXP-RECON-005 (time) and EXP-RECON-006 (memory).

## 3. Hypotheses

All hypotheses are `[INFERRED]` predictions. Direction and size are stated in band units of the pre-registered metric (thresholds referenced by id).

- **H1 (DEC-CMP-018, balanced).** At corpus level, `balanced-v6-minus-deflate` is `EQUIVALENT` to `balanced-gen-v6` on `artifact_bytes` (T-01): the |corpus log ratio| is below 1 band unit.
  - **Falsified if** `balanced-gen-v6` is `DISTINGUISHABLE_BETTER` at validation look 1.
- **H2 (DEC-CMP-018, target families).** In dense and extreme, `gen-v6` is better than `v6-minus-deflate` by more than 1 T-01 band unit in the family mean of at least one DEFLATE target family (common §7 rule).
  - **Falsified if** no target family exceeds 1 band unit on tuning.
- **H3 (DEC-CMP-026, R6).** In dense and extreme, `gen-v6` is better than `v6-minus-jpeg` beyond the T-01 band on the F12 family mean. At corpus level the gain does **not** reach the T4 minimum-gain band (§3.1, §7.3, Appendix D.6), so R6 would not retain `jpeg-v6-status-quo` in universal profile scope.
  - **Falsified if** the corpus-level tuning point estimate lies beyond the T4 minimum-gain band.
- **H4 (DEC-CMP-023).** The savings share of v5+v6 (`gen-v4 → gen-v6`) is below 10 % of the share of v4 (`gen-v3 → gen-v4`) at corpus level, per profile, in log units under the cited mix.
  - **Falsified if** ≥ 10 % in any profile.
- **H5 (DEC-CMP-012).** `v6-signature-gated` is `NON_INFERIOR` to `gen-v6` on `artifact_bytes` at corpus level. No family's point estimate lies beyond the band on the unfavourable side.
  - **Falsified if** any family is `DISTINGUISHABLE_WORSE` on tuning.
- **H6 (DEC-CON-010).** `v6-no-audits` makes bits 0x4 and 0x8 not required on ≥ 90 % of archives that emit no reconstructive byte. On ≥ 50 % of small-tier items it is smaller than `gen-v6` by more than the T-21 floor on `fixed_overhead_bytes`.
  - **Falsified if** either share falls below its threshold.
- **H7 (DEC-CMP-042).** On F18, `align-member-cdc` is worse than `gen-v6` beyond the T-01 band in at least one group (dedup loss from forced cuts). On F11 and F14 it is not better beyond the band (the reach gain is too small).
  - **Falsified if** it is better beyond the band on the F11 or F14 family mean and within the band on every F18 group.
- **H8 (model admissibility; binary, common §8.3).** The exact cost model reproduces `gen-v5` from `gen-v4`, and `gen-v6` from `gen-v5`, with 0 bytes of error on every tuning item and profile.
  - **Falsified by** any nonzero error. Modelled candidates of the failing class are then `NOT_DECISION_GRADE`.
- **Controls (binary; the item's samples are invalid on failure).**
  - **Identity control.** The unfiltered materialisation `P-gen-v6` equals `ebound pack --profile P` byte for byte.
  - **Generation controls.** `P-gen-v{3,4,5}` equal the frozen generation outputs of the baseline binary at `9e44608` on the same item (objective MVT-16(d) differential, run on tuning items).
  - **Reconciliation control.** `P-v6-minus-jpeg` minus `P-gen-v5` must equal exactly the planner-ID string, required-bit, audit-record and section-header differences reported by `ebr-inspect-bytes`. Any unexplained byte is a tooling defect.

## 4. Decisions informed and how

| Decision | Candidates compared here (common §6) | Primary metric (proposal) | Also needed |
|---|---|---|---|
| DEC-CMP-018 | `deflate-v5-status-quo` (= `gen-v6`) vs `deflate-drop`, `deflate-dense-extreme-only`, `deflate-signature-gated`, `deflate-entropy-gated-*`, `recon-opt-in-not-default` (REAL); `deflate-embedded-streams`, `deflate-multi-member-gzip`, `deflate-whole-object-regions`, `deflate-preflate-rs-latest`, `deflate-preflate-cpp`, `deflate-zlib-replay`, best max-chain (MODELLED) | `artifact_bytes` (superiority, profile scope) | guards: `encode_wall_s`, `decode_wall_s` (005); `alloc_peak_bytes` (006); `access_granularity_bytes` (here) |
| DEC-CMP-026 | `jpeg-v6-status-quo` vs `jpeg-drop`, `jpeg-balanced-multichunk`, `jpeg-opt-in-not-default` (REAL); `jpeg-add-progressive`, `jpeg-add-trailing`, `jpeg-jixel-latest`, `jpeg-libjxl`, `jpeg-lepton`, `jpeg-brunsli`, `jpeg-nested` (MODELLED; `jpeg-packjpg` reference only) | `artifact_bytes`; OD-10 `access_granularity_bytes` for multi-chunk | as above |
| DEC-CMP-023 | `gen-promote-all` (= `gen-v6`), `gen-promote-v4-only` (= `gen-v4` emission), `gen-default-stable-subset` (balanced `gen-v4`, dense/extreme `gen-v6`), `gen-withdraw-reconstruction` (= `gen-v4` emission, decode retained, §7.4) | `artifact_bytes`; generation share decomposition (descriptive) | tiers (011), R2 gated-step screen (011), reader burden (conformance domain) |
| DEC-CMP-042 | `align-status-quo` (= `gen-v6`) vs `align-member-cdc`, `align-single-chunk-recognised` (REAL); `align-whole-object-deflate`, `align-embedded-scan` (MODELLED) | `artifact_bytes`; `dedup_stored_fraction` diagnostic on F17/F18 | encode CPU (005); 16 MiB-bounded stream memory (006) |
| DEC-CMP-012 (reconstruction slice) | `gen-v6` (always-try raw) vs `deflate-signature-gated`, `deflate-entropy-gated-{7000,7500,7800}` | size regret: `artifact_bytes` non-inferiority | superiority metric `encode_cpu_s` (005) |
| DEC-CON-010 (bits 0x4/0x8, audits) | `bits-status-quo` (= `gen-v6`) vs `bits-minimal-activation` (= `v6-no-audits`) | `fixed_overhead_bytes` (small tier), `artifact_bytes` | `benign_false_refusal_fraction` of a minimal reader lacking reconstruction (here); explain accuracy loss (009) |
| DEC-CON-005, DEC-CMP-016 | ceiling policies (EXP-RECON-001 §11 step 7) applied to every candidate's declarations | `benign_false_refusal_fraction` (T-20) per candidate × ceiling | measured peaks (006) |
| DEC-CMP-038, DEC-CON-007 | `standard-format-only` (= `v6-minus-deflate`: only the ISO/IEC 18181 payload remains), `exclude-from-v1` (= `gen-v4`), `extended-set-with-refusal-profile` (= status quo bytes plus a refusal count for minimal readers) | `artifact_bytes` consequence of each | decode independence (008), cost and spec (011) |

## 5. Candidates (runner cells)

Per profile P ∈ {balanced, dense, extreme}: 13 cells. Plus 1 balanced-only cell and 2 fast controls. 42 cells in total.

| Cell | Definition | Tier est. |
|---|---|---|
| `P-gen-v3`, `P-gen-v4`, `P-gen-v5` | `entrybound::research::planner::PlannerVersion::{V3,V4,V5}::plan` on the captured unplanned archive, then production `ecf::encode` (INDEXED) | existing frozen IDs |
| `P-gen-v6` | `PlannerVersion::V6::plan`, identity-controlled against `ebound pack` | status quo |
| `P-v6-minus-deflate` | v6 trace, then every DEFLATE-reconstructive Chunk assignment replaced by that Chunk's v4 winner. The cohort stage and JPEG region stage are re-run on the resulting state with production rules. Encoded. | T1 |
| `P-v6-minus-jpeg` | v6 with the JPEG region stage disabled (regions not selected; audits retained) | T1 |
| `P-v6-signature-gated` | v6 with raw-wrapper DEFLATE attempts removed (gzip/zlib only); later stages re-run | T1 |
| `P-v6-entropy-gated-{7000,7500,7800}` | v6 with raw-wrapper attempts only on Chunks whose integer order-0 entropy estimate (§8.1) ≥ threshold (milli-bits per byte); later stages re-run | T1 |
| `P-v6-no-audits` | v6 selections unchanged. No ReconstructionFallback (type 16) or ReconstructionAudit (type 19) records. Required bits 0x4/0x8 set only when a ReconstructionData object or region is emitted. Research writer override. | T2 (bit semantics) |
| `P-align-member-cdc` | research chunker: gear-norm-v1 with forced cuts at the start and end of every census-located whole-file or embedded gzip/zlib/JPEG stream ≥ the profile minimum chunk size; then v6 planning | T1 (new chunker ID) |
| `P-align-single-chunk-recognised` | research chunker: census-recognised gzip/zlib/JPEG files ≤ 16 MiB as one Chunk; others gear-norm-v1; then v6 planning | T1 |
| `balanced-jpeg-multichunk` | v6 balanced with dense region eligibility (≤ 4096 Chunks) and the balanced post-codec list | T1 |
| `fast-gen-v4`, `fast-gen-v6` | fast controls (no reconstruction; bit and audit behaviour) | — |

- **Modelled candidates** are not runner cells. `ebr-recon model` computes them after the runs (§8.2) from `P-v6-minus-deflate` or `P-v6-minus-jpeg` real archives as bases, plus EXP-RECON-003 stream results for the backend survivor set (EXP-RECON-003 §11 step 5).
- **Reference candidate** per decision: the no-reconstruction arm of the class (`P-v6-minus-deflate` for DEFLATE, `P-v6-minus-jpeg` for JPEG, `P-gen-v4` for generations), or `P-gen-v6` for gates, alignment and activation.
- **Exclusions:** ledger exclusions recorded in common §6. **R0 item 6** critic candidates are open (common §3).

## 6. Corpus

- **Tuning.** All 112 tuning items (manifest), plus supplements S-DEFLATE-PRODUCERS, S-JPEG-PRODUCERS and S-JPEG-REAL when provisioned (amendment before their first run).
- **Validation (look 1).** All 91 validation items plus validation supplements. Registered in `research/decisions/validation-looks.jsonl` with decision ids, cells, primary metrics and item ids before execution. W, S, superiority metrics and MDEs are appended in §18 in a commit before the look.
- **Applicable families.** All families (common §7). Target families come from EXP-RECON-002 `target-families.json` (tuning), which is read before any size analysis.
- **Minimum support (§4.9).** A corpus-level verdict needs ≥ 10 groups across ≥ 5 families; validation has 76 groups over 20 families, so it is met. Family-scoped partitions for F12 (2 groups), the F05 logrotate class (1 group) and F04 objstore (1 group) are `single-group`, or 2-group with pooled variance. They cannot on their own support a scoped winner without S-JPEG-REAL and new DEFLATE groups (common §7).
- **Held-out.** Phase D placeholder (common §5): every cell and modelled candidate that reaches the freeze.
- **Public tuning data sources:** common §5.

## 7. Environment and platform requirements

- `wsl-ubuntu`, x86-64, no timing. Linux results for size are platform-independent for PCR/payload. Cross-host byte differences (F-0001/F-0002 metadata and bits) are out of scope: packing on WSL is the declared environment, a limitation noted for Windows-created archives.
- Research builds with default-off features (common §4 item 4).
- Storage: scratch peak about 2 × the largest item plus the trace cache (≤ 20 GiB per concurrent shard). Results about 10 GB outside git.

## 8. Commands and tooling to build

### 8.1 `ebr-recon variants`

The tool is in `research/harness/crates/ebr-recon`, with production research APIs behind default-off features. New research APIs needed in `crates/entrybound` (Internals agent, additive, byte-identical when disabled, MVT-16(c)):
- `research::planner::materialize(assignment)`: encode an arbitrary `PlannedAssignment` with production writers and write-time verification;
- `research::planner::plan_v6_with(overrides)`: overrides `{minus_deflate, minus_jpeg, gate: None|Signature|Entropy(u32), no_audits, region_eligibility: Balanced|DenseLike}`;
- `research::chunker::ranges_with_forced_cuts(bytes, params, cuts)`;
- `research::archive::capture_unplanned(path, options)`.

```text
ebr-recon variants --item-path <dir> --experiment-id EXP-RECON-004 --env-name wsl-ubuntu
                   --profile <fast|balanced|dense|extreme> --variant <cell suffix>
                   --census /root/eb-research/results/EXP-RECON-002/detail/<item_id>-rep0.jsonl.zst
                   --trace-cache /root/eb-research/cache/EXP-RECON-004/<item_id>-<profile>-rep<rep>
                   --identity-bin /root/eb-research/tools/EXP-RECON-004/bin/ebound
                   --baseline-bin /root/eb-research/tools/EXP-RECON-004/bin/ebound-9e44608
                   --archive-out <scratch>/a.eb --roundtrip true --access-sample 200 --access-seed 20260922
```

- **Trace cache.** The trace for (item, profile, repetition) is computed once and reused by the filter cells of the same repetition. The cache key includes the tool binary SHA-256 and the repetition, so repetition 2 recomputes and the repeat-hash check stays meaningful.
- **Per sample, the tool prints one summary line and performs:**
  - the identity or generation control applicable to the cell;
  - `ebr-inspect-bytes` accounting with `--per-chunk`, a new flag that attributes stored bytes to each Chunk frame, region and ReconstructionData object, and splits reconstruction sections into data, region, fallback and audit record bytes;
  - `ebound inspect --json --reconstruction --access`;
  - full `ebound unpack` plus per-file SHA-256 comparison (as EXP-RECON-001);
  - `ebr-access` seeded range reads (200 per archive, strata {1 B, 4 KiB, 1 MiB, whole entry}, entries drawn with probability ∝ logical bytes among region-member entries when regions exist, else all entries), recording decoded bytes per read against the declaration.
- **Entropy estimate (fixed now).** `H_milli = floor(1000 · Σ_c −(n_c/N)·log2(n_c/N))`, computed with integer arithmetic via a 16-bit fixed-point log2 table over byte counts n_c of the whole Chunk. Ties at the threshold count as "attempt". No floating point in the gate (planner-v1 no-float rule).

### 8.2 `ebr-recon model`

```text
ebr-recon model --real-results research/normalized/EXP-RECON-004 --stream-results research/normalized/EXP-RECON-003
                --census research/normalized/EXP-RECON-002 --candidate <modelled id> --profile <P>
                --base <cell> --out research/normalized/EXP-RECON-004/modelled/<candidate>-<P>.csv
ebr-recon model --admissibility --pairs gen-v4:gen-v5,gen-v5:gen-v6,gen-v5:balanced-jpeg-multichunk
                --out research/normalized/EXP-RECON-004/modelled/admissibility.csv
```

Formula and admissibility: common §8.3. Every modelled row carries the base cell, the per-object terms and the record arithmetic, so it can be recomputed (§12.3).

### 8.3 Runner and analysis

```sh
C:/Python313/python.exe research/orchestration/disk_guard.py
wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-RECON-004/prepare.sh   # to be written: as EXP-RECON-001/prepare.sh plus ebr-recon, ebr-access and a 9e44608 ebound build
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools &&
  PY=/root/eb-research/venv/bin/python && S=research/experiments/EXP-RECON-004/spec.yaml &&
  $PY -m ebr validate --spec $S && $PY -m ebr run --spec $S --gzip && $PY -m ebr verify-raw --spec $S --refingerprint &&
  $PY -m ebr normalize --spec $S && $PY -m ebr report --spec $S'
# fixed overhead reference (per item, small tier; objective M05.3; pinned tools from research/baselines/tool-versions.json):
#   python3 research/tools/recon/fixed_overhead_reference.py --manifest research/corpus/manifest.json --split tuning --out research/normalized/EXP-RECON-004/fixed-overhead-reference.csv
# decision analysis (decision tooling, §14 item 2):
#   python3 -m decide --experiment EXP-RECON-004 --decision DEC-CMP-018 --scope profile --record research/experiments/EXP-RECON-004/protocol.md
```

For the validation look, the spec is copied to `spec-validation-look1.yaml` (`splits: [validation]`) in the look-registration commit. No other change is allowed.

## 9. Seeds, warmups, replication

- Seeds: order, bootstrap and command 20260922; access sample seed 20260922; selection-stability bootstrap seed 20260922.
- Warmups: 0.
- Repetitions: 2 per cell and item (§4.6), in different rounds and processes, with the repeat-hash check (§4.13).
- Determinism is claimed for every cell (planner determinism, HC-13), so a hash mismatch is an R1 artifact for that cell (common §8.4) with both archives committed. No adaptive rule applies (deterministic).

## 10. Metrics

| Metric (canonical) | Role | OD | Threshold | Source |
|---|---|---|---|---|
| `artifact_bytes` | **primary** (superiority for profile scope; R4/R6) | OD-05 | T-01 (small tier relative only) | file size |
| `total_stored_bytes` | sensitivity arm | OD-05 | T-01 | sum over split |
| `fixed_overhead_bytes` | primary for DEC-CON-010 small-tier activation | OD-05 | T-21 | `artifact_bytes` − M05.3 reference |
| `side_data_bytes`, `metadata_bytes`, `index_bytes`, `dictionary_bytes` | diagnostic | OD-05 | T-02 | `ebr-inspect-bytes --per-chunk` |
| `dedup_stored_fraction` | diagnostic (F17, F18) | OD-05 | T-01 | unique stored chunk bytes ÷ logical |
| `access_granularity_bytes` | primary guard (OD-10) | OD-10 | T-09 | max declared worst reconstructed bytes per accessed byte |
| `access_amplification` (4 strata) | secondary | OD-10 | T-22 | `ebr-access` sample |
| `declared_cost_accuracy` | binary (HC-09 / M10.4) | — | §3.3 | measured decoded bytes ≤ declared |
| `roundtrip_mismatches` | binary (HC-02) | OD-02 | §3.3 | unpack comparison |
| `benign_false_refusal_fraction` | primary for ceiling and minimal-reader analyses | OD-17 | T-20 | declarations × policies |
| repeat-hash, identity, generation and reconciliation controls | binary | — | §3.3, §4.13 | tool |
| required bits, audit record bytes, region/data counts, declared window and working set | descriptive | — | — | inspect |
| planning CPU seconds | informational only | — | — | tool (decision-grade in 005) |

## 11. Normalization and statistical analysis

1. **Screens first (R1/R2).**
   - Any `roundtrip_mismatches` > 0, repeat-hash mismatch, identity or generation control failure, or `declared_cost_accuracy` violation → R1 for that cell (all its items), recorded with artifacts.
   - The R2 gated-step compliance result from EXP-RECON-011 §C is applied to every cell that emits `deflate-reconstruct/v1` or `jpeg-jxl-reconstruct/v1` from `balanced` (default path). If non-compliant, those cells are eliminated in the balanced scope, and the R0 item 7 corrective `recon-opt-in-not-default` stands in.
2. **Model admissibility** (H8) before any modelled analysis.
3. **Aggregation (§4.9).**
   - L1: exact item values; log ratio versus the reference; floors per tier (§3.1).
   - L2: group mean. L3: family mean over groups. L4: corpus mean with `workload-mix.json` weights; equal weights as an arm.
   - Strata: scale tier (small, medium, large). The profile is the scope, never averaged.
4. **Tuning (exploratory R3/R4).** Per decision and profile scope:
   - compute the L4 estimate and interval for every candidate against its reference;
   - apply R4 without multiplicity control to form S;
   - W = the smallest-`artifact_bytes` survivor meeting guards (profile scope, B.1), or the continuous band-unit utility winner (§6.2) for non-profile decisions, using OD-05/06/07/08/10 primary metrics with equal base weights (§6.3). Guards come from EXP-RECON-005/006 tuning outputs.
   - Declare per pair (W, s) the superiority metrics (k_sup) from tuning.
5. **MDE gate (§4.12).** For each confirmatory comparison, MDE from the tuning group-level variance and the validation group structure. Every MDE must be ≤ 1 band unit, and the A/A item-level condition applies only to timing metrics (from 005). A failing comparison is **not looked at**: the decision stays `INSUFFICIENT_EVIDENCE`, with "more groups needed" and the numeric MDE recorded.
6. **Validation look 1 (confirmatory).** For W versus each s ∈ S:
   - superiority at two-sided 1 − 0.01/k_sup;
   - non-inferiority on every primary metric at 0.99;
   - family, tier and condition harm guard at 0.90 (pooled variance) plus the 1-band point bound;
   - cost condition from computed tiers.

   Then R6 minimum gain per computed tier (T4 for library-version-defined steps; `recon-opt-in-not-default` T2; filters, gates and chunker variants T1).
7. **R8/R9.**
   - A universal (profile-scope) default only if R8 holds.
   - Otherwise R9 partitions in order: policy (encrypted or plain, where applicable, per EXP-RECON-001 H1d), workload mix (WM-MEDIA, WM-DIST, WM-BACKUP), family.
   - Expressibility: family partitions are expressible only in non-fast profiles through byte categorisation. Partition cost ≥ T1.
   - Scoped winners must beat the best single candidate within the part against the tier's minimum-gain band.
   - Compromise per R9 (iii).
8. **R10** tie handling with the power requirement.
9. **Generation share (DEC-CMP-023, descriptive).** Per profile and family, `ln(bytes(gen-v3)/bytes(gen-v4))`, `ln(bytes(gen-v4)/bytes(gen-v5))` and `ln(bytes(gen-v5)/bytes(gen-v6))`, aggregated per §4.9 with intervals. Reported as shares, never used to select.
10. **Minimal-reader refusals (DEC-CON-010, DEC-CON-007 `extended-set-with-refusal-profile`).** `benign_false_refusal_fraction` of a reader lacking bits 0x4 and 0x8, per cell. n_cases = archives per cell.
11. **Ceiling refusals (DEC-CON-005/016).** `benign_false_refusal_fraction` per (cell × ceiling policy), with the policies of EXP-RECON-001 §11 step 7.
12. **Byte identity of claims.** Every reported number comes from `ebr normalize` or decision-tooling outputs (§12.3).

## 12. Practical-significance thresholds

T-01 (`artifact_bytes`, `total_stored_bytes`, `dedup_stored_fraction`), T-02 (diagnostics), T-09 (`access_granularity_bytes`), T-21 (`fixed_overhead_bytes`), T-22 (`access_amplification`) and T-20 (refusal fractions), with §3.3 for binaries. Minimum gains per §3.1 and §7.3 with the computed tier multiplier (Appendix D.6). No value is restated here.

## 13. Sensitivity analysis

- common §10 in full.
- Per profile scope: threshold perturbation (global and single-metric ×0.5 and ×2, resolvability condition), all §6.5 family arms (equal weights, leave-one-family-out, leave-three-families-out, family Dirichlet, WM-*, leave-one-mix-out, tier-stratified, byte-weighted), and the selection-stability bootstrap.
- Non-profile decisions additionally get the weight grid, Dirichlet and SMAA report (§6.3).
- Reconstruction-specific arms: without derived producer matrices; without F20; with and without S-JPEG-REAL; modelled candidates re-analysed with record-overhead terms ×1.5 (a model-structure stress test, reported only).

## 14. Expected negative results worth recording

- DEFLATE reconstruction is equivalent to dropping it in balanced (H1), and possibly in every profile at corpus level. The v5 generation contributes a negligible savings share (H4).
- JPEG regions win on F12 but fail T4 minimum gain at corpus scope (H3). The family-scoped winner is not supportable with 2 F12 groups (common §7).
- Status-quo required bits on archives without reconstructive bytes (H6) are a pure compatibility cost.
- Member-aligned CDC loses dedup on version series (H7).
- Exact cost model inadmissible for a class (H8), so those modelled candidates stay `NOT_DECISION_GRADE`.
- Every family where the selected candidate loses at R9 or R5 is listed by name with magnitudes (§10.1 item 4).

## 15. Threats to validity

- **Research overrides are not production.** Filters re-run later stages with production rules, but the interaction with cohort selection could differ from a planner designed without reconstruction. Mitigations: reconciliation and identity controls; filter cells are new T1 planner IDs whose real implementation must reproduce these bytes before freeze (reopen trigger).
- **WSL-only packing** misses Windows metadata effects on bits and bytes (F-0001). Size conclusions are scoped to Linux-captured trees; a Windows arm is a known limitation for DEC-CON-010.
- **Trace cache.** Stale-cache risk is controlled by the binary-hash key and a per-repetition cache.
- **Modelled candidates:** common §8.3 limitation; H8.
- **Small F12, F05 and F04 group counts:** §6; MDE gate.
- **Guards from other experiments** are joined at aggregate level. Pairing for timing guards exists only within EXP-RECON-005.
- All other threats: common §13.

## 16. Held-out placeholder

At Commit A, `spec-heldout.yaml` (`splits: [heldout]`) is committed with the held-out MDE of every supporting comparison, computed from validation group variance and held-out group counts per family. Every frozen cell and modelled candidate runs once after Commit C (R5 (a)–(f)); held-out modelled candidates also need held-out stream runs of EXP-RECON-003. Held-out F12 has groups from real photo sources (counts only are used). The `identity_aware_heldout` cap applies unless sealed tranches exist (common §5).

## 17. Cost and effort estimates

- **Compute.** About 300 CPU-hours for tuning plus validation: about 18 full planning passes per (item, repetition) over 62.7 GiB × 2 repetitions, extreme dominating; filter cells are encode-only; unpack and hash of 42 archive kinds. Item sharding (runner addition `ebr run --shard i/n`) is recommended; without it, wall time ≈ CPU time.
- **Disk.** Scratch peak ≤ 20 GiB per shard; trace cache ≤ 40 GiB transient; results about 10 GB.
- **Tooling.** Research APIs in production (Internals agent, reviewed), `ebr-recon variants` and `model`, and `ebr-inspect-bytes --per-chunk`: two to three harness sessions.
- **Agent effort.** Execution: none. Analysis: judgment (decision tooling runs are scripted; interpretation, R9 partition review and CB triggers need a judgment session plus the independent re-analysis required by CB-7).

## 18. Looks, deviations and outcome (append-only)

- Look 1 (validation): to be registered. W/S/k_sup/MDE are appended here before the look.
- Look 2: only on new validation groups (§5.2).
- Deviations: none.
- Outcome: `outcome.json`.
