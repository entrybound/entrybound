# EXP-PLANNER-001: Status-quo profile census (sizes, section bytes, plan usage, feature bits, identities)

| Field | Value |
|---|---|
| Domain | planner (program §9, §10) |
| Index status | READY (runnable now with existing binaries rebuilt per `research/planner/README.md` §2) |
| Document state | Phase C design. Becomes a `record.md` (decision-method §12.2) only after gate G-A holds. |
| Shared conventions | `research/planner/README.md` (build identity §2, look plan §5, frames §6, L7 disclosure §10, seeds §11) |
| Runner spec | `research/experiments/EXP-PLANNER-001/spec.yaml` (validated with `python -m ebr validate`) |
| Sample driver | `research/experiments/EXP-PLANNER-001/census_sample.py` (TP-00, written and smoke-tested at design) |

## 1. Pre-registration header

- **decision_ids:** DEC-CMP-036, DEC-CMP-034, DEC-CMP-035, DEC-CON-010, DEC-MOD-039.
- **Decision type (proposed; assignment by an independent session, §1.2):** EMPIRICAL for every listed decision's measured parts. This experiment is descriptive: it selects nothing by itself. It supplies the status-quo reference cells of every planner comparison, the deterministic size side of the profile Pareto placement, and three censuses.
- **Proposed `od_ids`:** OD-05 (artifact bytes), OD-10 (access granularity via declared requirements, secondary here), OD-23 (feature-bit census as a specification-surface input), OD-26 (minimal-reader readability, DEC-CON-010). **Proposed `hc_ids`:** HC-02 (round trip), HC-13 (repeat-hash determinism), HC-16 (research-baseline identity), HC-10 (identity change pattern across profiles, DEC-MOD-039).
- **Method:** `research/decision-method.md` at the commit that adds the record.
- **Author:** planner-domain design session (Phase C). L7 exposure disclosed in `research/planner/README.md` §10.

## 2. Question and hypotheses

- **Question.** For each current public profile and layout, what does production produce on every tuning item (validation inside planner look 1): artifact bytes, section composition, plan and codec usage, cross-file and reconstruction usage, feature bits and identities; is it deterministic and lossless; and does the pre-registration build reproduce the research-baseline archives?
- **H1 (directional, descriptive anchors).**
  1. Every valid cell round-trips (`roundtrip_mismatches` = 0) and both repetitions share one `artifact_sha256`.
  2. `baseline_9e44608_identical` is true in every INDEXED cell.
  3. Corpus-level `artifact_bytes` ratios against `balanced-indexed` order the profiles fast > balanced > dense ≥ extreme. The extreme/dense ratio is predicted inside one T-01 band (`metric_thresholds.artifact_bytes.relative`) in at least half of the applicable families.
  4. The `incompat` bitmask varies across profiles of one item only where a cross-file or reconstruction technique actually emits bytes.
  5. PCR differs between two profiles of one item exactly when their `chunker_id` differs.
- **H0.** No ordering or gap statement holds at corpus level.
- **Falsification.** Any repeat-hash, round-trip or baseline-identity mismatch falsifies H1.1-H1.2. That is an HC violation (objective §2.0 rule 2): the two outputs are committed as the violation artifact and every planner experiment stops until it is dispositioned. H1.3 fails if a corpus-level ratio has the other sign, or if extreme/dense lies outside the band in more than half of the families. H1.4-H1.5 fail on any counterexample item.

## 3. Decisions informed

| Decision | What this experiment supplies | Remaining evidence elsewhere |
|---|---|---|
| DEC-CMP-036 (four profiles, balanced default) | Deterministic size placement of every profile per item, family and tier; adjacent-profile size gaps in band units; the size inputs of the §7.3 new-profile rule | Timing, memory and default utility: EXP-PLANNER-007; human-facing parts: ecosystem §33 |
| DEC-CMP-034 (candidate tables, dead candidates) | Real-archive codec and transform usage per profile (`chunk_payload_bytes_by_codec`, `_by_transform`, `codec_usage`) | Counterfactual win frequency against the oracle: EXP-PLANNER-002/004 |
| DEC-CMP-035 (profile definition form) | Status-quo reference cells for fitted parameterized profiles | EXP-PLANNER-006, 007 |
| DEC-CON-010 (feature-bit activation) | Share of archives per profile and family that set each `incompat`/`read_only_compat`/`compat` bit, joined with whether the corresponding technique emitted bytes (dictionary, group, reconstruction, region counts) | Reader-refusal and dependency-chain analysis: container domain (§11) |
| DEC-MOD-039 (chunker binding, PCR) | Frequency of PCR divergence for identical content across profiles, and its cause (chunker policy) | Per-category chunking impact: EXP-PLANNER-005; verified-range soundness: model/formal route |

## 4. Candidates

This is a census, not a selection, so R0 completeness is not tested here. Every candidate is the status quo (R0 item 1): production at `9e44608`, whose byte identity with the pre-registration build this experiment verifies.

| candidate_id | Definition | Cost tier |
|---|---|---|
| fast-indexed, balanced-indexed, dense-indexed, extreme-indexed | `ebr-pack --profile P --layout indexed` (production `plan_directory` + `ecf::encode`, planner `P-v6`) | status quo (T0 reference) |
| fast-stream, balanced-stream, dense-stream, extreme-stream | `ebr-pack --profile P --layout stream --stream-window auto` | status quo |

- **Reference candidate:** `balanced-indexed` (REF-PROD, objective §3.0).
- **Identity arm:** inside each INDEXED sample, `ebound pack` built at `9e44608` (`/root/eb-research/target/baseline/release/ebound`) packs the same item, and archive SHA-256 equality is recorded.
- **Excluded:** encrypted cells. Padding and keyed boundaries make them a crypto-domain matter; encrypted planner-ID behaviour is in EXP-PLANNER-010.

## 5. Corpus

- **Split:** tuning (every item in `research/corpus/manifest.json` with `split = tuning`: 112 items, all 20 families, all tiers). Validation (91 items) runs once, inside planner look 1, with the same spec and `splits: [validation]`.
- **Selector:** manifest plus split; no item selection by content.
- **Families and tiers:** all 20 families. Strata are scale tier and layout. There are no evaluation conditions.
- **Minimum support (§4.9):** tuning has 20 families with at least 2 groups each; validation also meets 10 groups across 5 families.
- **Family weights:** `research/methods/workload-mix.json` (gate G-B) for L4; equal weights as the §6.5 arm.
- **Known refusals:** baseline probes found `ebound pack` refuses non-UTF-8 filenames and some POSIX ACL masks. Such samples fail with the production diagnostic. They are reported by item and excluded from size aggregates, never retried with a modified item (§5.7 L4).

## 6. Environment and platform requirements

- `wsl-ubuntu` (x86-64, ext4 under `/root/eb-research`). Timing is off, so the `VIRTUALIZED` qualifier does not matter for these deterministic metrics.
- Builds per `research/planner/README.md` §2. `lock_parity.py` and `harness-cli-parity.sh` must pass first.
- Platforms: Linux x86-64 only. No network, no privileges beyond WSL root, no external review. Storage: transient scratch of about 2× the largest concurrently packed item plus extraction.

## 7. Commands

```sh
# once per campaign (Windows host first: C:/Python313/python.exe research/orchestration/disk_guard.py)
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && CARGO_TARGET_DIR=/root/eb-research/target/planner-harness bash research/harness/build.sh build --release'
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && CARGO_TARGET_DIR=/root/eb-research/target/planner-cli /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli'
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && python3 research/harness/lock_parity.py && bash research/harness/harness-cli-parity.sh'
# experiment
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && \
  /root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-PLANNER-001/spec.yaml && \
  /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-PLANNER-001/spec.yaml && \
  /root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-PLANNER-001/spec.yaml --refingerprint && \
  /root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-PLANNER-001/spec.yaml && \
  /root/eb-research/venv/bin/python -m ebr report --spec research/experiments/EXP-PLANNER-001/spec.yaml'
# census tables (decision tooling, §14 item 2) -> research/normalized/EXP-PLANNER-001/decision/
#   profile_size_placement.csv, adjacent_profile_gaps.csv, codec_usage_by_profile.csv,
#   feature_bit_share.csv, pcr_divergence.csv, baseline_identity.csv
```

Throughput: when TP-17 exists, `ebr run --shard i/K` with `research/planner/tools/shard_launch.py --max-concurrent-item-gib 12`. Sequential execution gives identical results.

## 8. Seeds, warmup, replication

- Seeds: order 260917011, bootstrap 260917012, command 260917013.
- Warmups: 0 (deterministic metrics).
- Repetitions: 2 in different rounds and processes (§4.6), with `order: randomized_blocks`. There is no adaptive rule: the repeat-hash check (§4.13) replaces it.
- A repeat-hash mismatch is not re-run. It is the violation artifact (§4.13); both archives are copied to `research/raw/EXP-PLANNER-001/violations/`.

## 9. Metrics

| Metric (canonical, Appendix A.2) | OD | Role here | Family | Unit | Dir | Threshold | Source |
|---|---|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | descriptive anchor (primary in consuming decisions) | size | B | min | T-01 | `ebr-pack` `artifact_bytes` |
| `metadata_bytes` (= artifact − chunk payload, INDEXED) | OD-05 | diagnostic | size | B | min | T-02 | TP-00 |
| `metadata_bytes_per_entry` | OD-05 | secondary | size | B/entry | min | T-02b | TP-00 |
| `index_bytes`, `dictionary_bytes`, `side_data_bytes` | OD-05 | diagnostic | size | B | min | T-02 | `byte_accounting` |
| `dedup_stored_fraction` | OD-05 | diagnostic | size | ratio | min | T-01 | unique plaintext / logical |
| `roundtrip_mismatches` | OD-02 | binary (item-level indicator: 1 per mismatching item; file-level detail from `ebr-unpack` output on failure) | correctness | count | = 0 | §3.3 | fingerprint comparison |
| `roundtrip_ok` | OD-02 | binary gate of every size sample | correctness | – | = 1 | §3.3 | check |
| `artifact_sha256` (repeat-hash) | OD-04/HC-13 | binary | correctness | string | single value | §4.13 | TP-00 |
| `baseline_9e44608_identical` | HC-16 | binary | correctness | bool | = 1 | objective MVT-16(d) | TP-00 |
| `diag_*` counts, feature bitmasks, `identity_*`, `planner_id`, `chunker_id` | – | descriptive census inputs | other/correctness | – | – | none (secondary only, §0.2 item 4) | TP-00 |

## 10. Normalization and statistical analysis

- **Correctness gating.** A sample with `roundtrip_ok` = 0, a repeat-hash mismatch or a failed pack is excluded from every size aggregate and reported as an HC outcome (objective §3.0).
- **Item level (L1).** Exact values (§4.13). Paired log ratio `ln(artifact_bytes(P, L) / artifact_bytes(balanced, indexed))`. The T-01 absolute floor per parent tier (`metric_thresholds.artifact_bytes.absolute_by_tier`) applies before aggregation (§3.1).
- **L2-L4 (§4.9).** Group means, family means with equal group weights, corpus mean with `workload-mix.json` weights. Corpus intervals use Welch-Satterthwaite; family intervals use pooled variance (§4.10). These intervals express between-group heterogeneity only.
- **Outputs (all generated, §12.3):**
  1. `profile_size_placement.csv`: corpus, family and tier ratios with intervals for every profile × layout against the reference, and adjacent-profile gaps in continuous band units (§6.2).
  2. `codec_usage_by_profile.csv`: stored and logical bytes per codec and transform, and the share of Chunks.
  3. `feature_bit_share.csv`: per profile × family, the fraction of archives setting each bit, cross-tabulated with technique-emitted indicators.
  4. `pcr_divergence.csv`: per item, pairwise PCR equality across profiles, with the `chunker_id` pair.
  5. `baseline_identity.csv`.
- **Verdicts.** None of this experiment's outputs eliminates or selects a candidate. The H1.3 statements are checked with the §4.11 verdict classes at the §4.12 secondary confidence (`bootstrap.secondary_confidence`, labelled `unadjusted`).
- **Practical significance.** T-01 and T-02 bands as transcribed in `research/methods/thresholds.json` `metric_thresholds`, never restated.

## 11. Sensitivity analysis

§6.5 arms on the size placement: equal-weight arm, leave-one-family-out, tier-stratified, and byte-weighted `total_stored_bytes` (ratio of totals). They are reported as descriptive stability of the ordering. No selection is made here, so no stability bar is applied.

## 12. Expected negative results worth recording

- Extreme not distinguishable from dense on most families, which feeds the DEC-CMP-036 merge candidate.
- Codec or transform candidates with zero stored bytes in every archive of a profile, which are dead-candidate evidence for DEC-CMP-034.
- Feature bits set although the technique emitted nothing, for example cross-file bits with zero dictionaries and groups (DEC-CON-010 status-quo defect evidence).
- PCR divergence for identical content across profiles on most multi-policy items (DEC-MOD-039).
- Production refusals on corpus items (non-UTF-8 names, ACL masks). These are recorded as product limitations with item ids.

## 13. Threats to validity

- The harness `ebr-pack` links `research-internals`. Byte identity with `ebound` is proven for 8 cells by `harness-cli-parity.sh`, and the identity arm extends that proof per item for INDEXED only. STREAM relies on the parity script.
- The census covers only production planner v6. Historical generations are in EXP-PLANNER-010.
- Tuning items only: the corpus is representative only as far as its conditions allow (`research/corpus/critique-round2.md`: G02 database dumps partial, G03 registry sources absent).
- `metadata_bytes` is undefined (null) for STREAM, whose byte accounting is a single lump (harness review L-03).
- `ebound inspect --json` output can be large on trees with many entries. Parse memory is harness-side and not measured.

## 14. Cost and effort

- **Compute (estimate, from a non-decision-grade design smoke on a 2.3 MiB tree plus the corpus byte totals; uncertainty about 5×):** tuning about 240 CPU-hours (dominated by dense and extreme packs over 36.1 GiB × 2 layouts × 2 repetitions, plus extraction and fingerprinting); validation about 180 CPU-hours. Sequential wall time is about 18 days; with TP-17 sharding at 12 concurrent workers, about 1.5 days.
- **Disk:** transient scratch at most about 40 GB under `/root/eb-research/scratch/ebr`; raw JSONL about 1 GB (gzip).
- **Agent effort:** scripted execution (mechanical model); judgment only to read the generated census tables for the consuming experiments.

## 15. Gates and status

- **Runnable now:** yes (READY). Existing binaries (`ebr-pack`, `ebr-unpack`, production `ebound`, baseline `ebound`) plus TP-00.
- **Decision-grade use requires:** G-A (for the record) and G-B (decision tooling, frozen held-out lock, `workload-mix.json`). There is no timing gate.
- **Held-out:** EXP-PLANNER-012 (Phase D placeholder, §5.5).
