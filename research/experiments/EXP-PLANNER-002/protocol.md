# EXP-PLANNER-002: Exhaustive planner oracle traces and status-quo regret by workload class

| Field | Value |
|---|---|
| Domain | planner (program §9, §10) |
| Index status | NEEDS_TOOLING (TP-01 `ebr-planner trace-tree`, TP-02 `materialize`, TP-05 `categorise`, TP-15 `make_psub.py`; TP-17 for throughput) |
| Document state | Phase C design; `record.md` after gate G-A |
| Shared conventions | `research/planner/README.md` §2 build, §3 PSUB-k, §5 look plan, §6 frames, §7 modelled vs materialized, §9 tooling |
| Runner spec | `research/experiments/EXP-PLANNER-002/spec.yaml` |

## 1. Pre-registration header

- **decision_ids:** DEC-CMP-032 (regret part), DEC-CMP-034 (win frequency and marginal contribution inputs), DEC-CMP-009 (win frequency of density codecs under exhaustive search).
- **Decision type (proposed):** EMPIRICAL. The trace-mirror check (`drift_differences` = 0) is a FORMAL-style exact test.
- **Proposed `od_ids`:** OD-05 (primary), OD-07 and OD-08 (reported per selected plan through declared decode requirements, as guards consumed by EXP-PLANNER-003/004). **Proposed `hc_ids`:** HC-02, HC-06 (declared decode requirements of oracle choices), HC-11 (no library-version-defined decode step on a baseline path), HC-13, HC-16.
- **Author / L7:** `research/planner/README.md` §10.

## 2. Question and hypotheses

- **Question.** Against an exhaustive oracle over a strict superset of every registered candidate, how much `artifact_bytes` does the status-quo v6 planner leave unrealized (regret, in T-01 band units)? The answer is broken down per profile, per stage (Chunk, cohort, JPEG region), and per family, scale tier, workload mix (Appendix B.2) and TP-05 content category. The experiment also asks which candidates the oracle selects.
- **H1.**
  1. Corpus-level status-quo regret against the **bound-respecting** oracle (§4) is below 1 band unit for fast and balanced, and at most 2 band units for dense and extreme.
  2. At least one family per profile shows regret above 2 band units: structured numeric (F07), databases (F08) or executables (F09), where transform and LZMA2 shapes matter.
  3. Against the **unbounded** oracle, regret for fast and balanced exceeds 2 band units at corpus level, because LZMA2 and high Zstandard levels are outside their tables.
  4. External density codecs (modelled-only) win under 5% of Chunk bytes in dense and extreme.
- **H0 (equivalence).** Status-quo regret is inside the T-01 band (`EQUIVALENT`) at corpus level for every profile.
- **Falsification.** H1.1 fails if the corpus-level interval of the regret log ratio lies wholly above 1 band unit (fast, balanced) or 2 band units (dense, extreme). H1.2 fails if no family exceeds 2 band units in any profile. H1.3 fails if the corpus-level unbounded regret for fast or balanced is at most 2 band units. H1.4 fails if external codecs win at least 5% of Chunk bytes in dense or extreme. A non-zero `drift_differences` on any item invalidates that item's traces: they are excluded, and the experiment stops for a research-API mirror defect.

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CMP-032 | Regret distributions of the status quo against the oracle per corpus group (ledger required evidence item 1); the oracle cost tables every strategy replay uses | Strategy candidates, pack time, memory: EXP-PLANNER-003, 007; determinism across platforms: 009; held-out: 012 |
| DEC-CMP-034 | Per-candidate win share and the marginal size contribution inputs per profile table and under the oracle | Table variants and their R4-R10 analysis: EXP-PLANNER-004 |
| DEC-CMP-009 | Win frequency of LZMA2 and of external brotli, bzip2 and PPMd under exhaustive search in dense and extreme (ledger required evidence item 4) | §5.8 criteria audit, decoder memory, portability: codecs domain (§8.6) |

## 4. Candidates

| candidate_id | Definition | Materializable | Expected tier |
|---|---|---|---|
| status-quo-v6-<profile> (reference) | `trace_archive(archive, profile, V6)`; must equal `PlannerVersion::V6.plan` (`drift_differences` = 0) | yes (production archive) | status quo |
| oracle-bounded-<profile> | Chunk stage: minimum `complete_cost` over the `SearchCaps::full()` shapes whose declared decode requirement does not exceed the status-quo profile's **implied bounds**, meaning the maximum working set and window over that profile's own v6 table (balanced: Zstandard 1 MiB window and 4 MiB working set, LZ4, STORE, transforms on those codecs). Cohort stage: `search_cohort` over every `SUPPORTED_LEVELS` × `SUPPORTED_LOOKBACKS` inside the same bounds, including lookback for balanced (counterfactual). Region stage: every `jpeg_region_candidates` entry. Stages are composed greedily in production order | yes, via TP-02 | T1 (new planner ID) |
| oracle-unbounded | The same without the bound filter: every `SUPPORTED_LEVELS` Zstandard level, every `SUPPORTED_LZMA2_CONFIGURATIONS`, every TransformKind | yes | T1 |
| oracle-unbounded-plus-external | oracle-unbounded plus `brotli-q11-w24`, `bzip2-9`, `ppmd-o6-m16` payloads from `ebr-codec`'s pinned external codecs. Plan record bytes are charged as a TransformPlan record of the same length as a Zstandard plan (a hypothetical registered-extended codec id) | **modelled-only** (§7 of the README) | T2 (registered extended codec) |

- **Oracle status.** The oracles are regret references and T1/T2 candidates of DEC-CMP-032. Their pack cost makes them unlikely to survive R2 compress-budget screens once DEC-CMP-035 fixes budgets, which is analysed in EXP-PLANNER-003.
- **Stage-composition limitation.** Composing stage oracles greedily is not a joint global optimum, because cohort choices override Chunk plans. Regret is therefore claimed against the staged oracle only, and the true joint regret is at least this large. The claim "the status quo is near-optimal" is limited accordingly.
- **Excluded:** none at this stage. `float-heuristic-selector` (DEC-CMP-032) is excluded in EXP-PLANNER-003.

## 5. Corpus

- **Items:** PSUB-256 sub-items (`research/planner/derived/psub-256.json`) of every tuning item. Validation sub-items run once, in planner look 1.
- **Chunk coverage:** every unique Chunk of the sub-item under each profile's realized chunking. There is no Chunk subsampling, so archive-level bounds are complete.
- **Families:** all 20. Strata: parent scale tier; TP-05 content category (descriptive); WM-* mixes as §6.5 arms.
- **Minimum support:** as EXP-PLANNER-001. Weights: `workload-mix.json` (G-B).
- **F20:** included. Adversarial inputs test that oracle search terminates within `SearchCaps.max_candidates` and that no decode requirement is undeclared (R2 boundedness).

## 6. Environment and platform

- `wsl-ubuntu`, x86-64, deterministic metrics only, no timing.
- Memory: production capture is in memory. TP-01 processes one sub-item per worker (at most 256 MiB of plaintext plus traces), so up to 16 concurrent workers fit in 31 GiB.
- Platforms: Linux x86-64. No network, no privileges, no external review.

## 7. Commands

```sh
# derive sub-items (TP-15) and categories (TP-05)
python3 research/planner/tools/make_psub.py --k-mib 256 --splits tuning --manifest research/corpus/manifest.json \
  --out-root /root/eb-research/derived/planner/psub-256 --out-manifest research/planner/derived/psub-256.json
# traces (runner)
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-PLANNER-002/spec.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-PLANNER-002/spec.yaml --refingerprint
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-PLANNER-002/spec.yaml
# regret tables and oracle assignments (TP-03)
python3 research/planner/tools/replay.py regret --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces \
  --manifest research/planner/derived/psub-256.json --out research/normalized/EXP-PLANNER-002/replay/
# reconciliation of oracle upper-bound assignments (TP-02), seeded 10% sample, seed 260917992
python3 research/planner/tools/replay.py reconcile-sample --experiment EXP-PLANNER-002 --fraction 0.10 --seed 260917992 \
  --materialize-bin /root/eb-research/target/planner-harness/release/ebr-planner
```

Trace shards are large side outputs written to `/root/eb-research/raw-side/EXP-PLANNER-002/traces/<item_id>/`, outside git. Their SHA-256 values are recorded in the hash-chained raw rows (`shards_sha256`), so `verify-raw` covers them by hash. A compact `trace-index.csv` (item, shard hashes, sizes) is committed under `research/raw/EXP-PLANNER-002/`.

## 8. Seeds, warmup, replication

- Seeds: order 260917021, bootstrap 260917022, command 260917023; reconciliation sample 260917992.
- Warmups 0. Repetitions 2 (§4.6 deterministic). `shards_sha256` must be identical across repetitions (repeat-hash, §4.13). A mismatch is a determinism violation of the research mirror and is committed as an artifact.
- The oracle cache (`/root/eb-research/cache/planner-oracle/`) is keyed by Chunk SHA-256 and caps hash. Repetition 2 runs with `--cache-dir` pointing to an empty directory, so repeat-hash checks recomputation, not the cache.

## 9. Metrics

| Metric | OD | Role | Family | Unit | Dir | Threshold | Source |
|---|---|---|---|---|---|---|---|
| `artifact_bytes`, as regret log ratio status-quo/oracle (archive-level bounds; non-inferiority uses the **upper** regret bound = selected − oracle lower bound; superiority uses the **lower** regret bound = selected − oracle upper bound, materialized) | OD-05 | primary for the regret comparisons | size | B | min | T-01 | TP-01 `archive_regret`, TP-02 |
| per-stage regret (Chunk, cohort, region) | OD-05 | diagnostic | size | B | min | T-01 | TP-01 shards via TP-03 |
| win share per candidate shape (by Chunk count and by stored bytes) | – | descriptive | other | fraction | – | none | TP-03 |
| `drift_differences` | HC-16 | binary gate | correctness | count | = 0 | exact | TP-01 |
| declared decode working set and window of oracle choices | OD-08 | diagnostic (feeds bound filter audit) | other | B | – | none | research `aggregate_archive_decode_requirements` |
| `work.candidate_evaluations`, `work.bytes_examined` | – | descriptive (consumed by EXP-PLANNER-003 budgets) | other | count; B | – | none | TP-01 |

## 10. Normalization and statistical analysis

- **L1:** per sub-item exact log ratio `ln(selected_cost / oracle_cost)` using the archive-level bound selected by the comparison type (§9). The T-01 floor applies by parent tier.
- **L2-L4:** §4.9 with `workload-mix.json` weights. Corpus interval: Welch-Satterthwaite; family interval: pooled variance (§4.10).
- **Comparisons (tuning, exploratory).** For each profile scope: status-quo against oracle-bounded, and status-quo against oracle-unbounded, on `artifact_bytes`. Verdict classes §4.11, with `bootstrap.noninferiority_confidence` for non-inferiority and `bootstrap.superiority_confidence_rule` for superiority. These are tuning results and cannot eliminate. The confirmatory versions run in EXP-PLANNER-003 at planner look 1.
- **Regret distributions (the focus deliverable).** Per profile × family, tier, WM-* mix and TP-05 category: median, p90 and maximum of item-level regret in continuous band units (§6.2), with item counts. Percentiles use type 7 (§4.8); p90 needs n ≥ 10, otherwise it is flagged `low_n`.
- **Win frequency.** Per profile and oracle: share of Chunks and of stored bytes won by each shape. The drop-one marginal contribution (regret increase when a shape is removed from the oracle universe) is computed by TP-03 and consumed by EXP-PLANNER-004.
- **MDE computation (for look 1).** From tuning group-level variance of the regret log ratio with the validation group structure (§4.12). The result goes into EXP-PLANNER-003's record.
- **Practical significance:** `metric_thresholds.artifact_bytes` (T-01) only.

## 11. Sensitivity analysis

- §6.4 threshold perturbation on the regret verdicts (T-01 band ×0.5 and ×2; the ×0.5 resolvability condition cannot fail for deterministic metrics, so it runs).
- §6.5 arms: equal weights, leave-one-family-out, leave-three-families-out, family Dirichlet, WM-* mixes, tier-stratified, byte-weighted `total_stored_bytes`.
- §6.7 selection-stability bootstrap (seed 260917994) on the regret class of each profile (below 1, 1-2, above 2 band units).
- **Stage-composition arm:** regret against the Chunk-stage oracle alone versus the composed staged oracle.
- **Bound-definition arm:** the bounded oracle recomputed with bounds taken from EXP-PLANNER-006's fitted values once they exist (secondary; results are visible by then, so it is labelled post-hoc, CB-3).

## 12. Expected negative results worth recording

- Status-quo regret inside one band for every profile at corpus level: the planner is near-optimal within its bounds, which argues against complex strategies (DEC-CMP-032 negative for the alternatives).
- Frozen candidates with zero oracle wins (dead candidates).
- External codecs never winning beyond LZMA2 in dense and extreme (DEC-CMP-009 negative for adding them).
- Families where the bounded oracle gains nothing but the unbounded oracle gains much, which shows the bound, not the search, limits size.

## 13. Threats to validity

- PSUB-256 sampling truncates cross-file structure on large items (README §3). Cohort-stage regret on F17 and F18 large items is a lower bound of whole-item behaviour.
- The oracle universe is limited to registered codecs, supported levels and structural transforms, plus three external codecs. "Exhaustive" means exhaustive over that universe (harness review verdict (e)).
- The v5-basis recosting of selections (`selected_recosted_cost`) differs from the generation's own basis. Only the v5 basis is used. DEFLATE-reconstruction selections are out of scope (`in_scope == false`) and reported separately.
- Bounded-oracle bounds are inferred from status-quo tables, not fixed values. That is circular for decisions about the bounds, hence the bound-definition arm.
- External-codec plan-record bytes are assumed, not implemented.
- The designer's L7 exposure (README §10).

## 14. Cost and effort

- **Compute (estimate):** exhaustive search at about 10 s of CPU per MiB (design smoke, non-decision-grade) over PSUB-256 tuning (about 14.3 GiB) × up to 4 realized chunkings, plus traces and external codecs: about 240 CPU-hours tuning and about 240 CPU-hours validation. With TP-17 at 16 workers, about 30 hours of wall time in total.
- **Disk:** PSUB-256 sub-items about 29 GB (tuning and validation); trace shards about 15 GB gzip; oracle cache about 2 GB; transient scratch about 10 GB.
- **Agent effort:** tooling build is scripted (mechanical model; TP-01/02/05/15 are Rust and Python with tests); execution is scripted; analysis is judgment (reading generated regret tables).

## 15. Gates and status

- **Tooling:** TP-01, TP-02, TP-03 (`regret`, `reconcile-sample`), TP-05, TP-15; TP-17 recommended.
- **Gates:** G-A for the record; G-B for decision-grade verdicts; no timing gate; §14 item 9 rule simulation before look 1.
- **Held-out:** EXP-PLANNER-012.
