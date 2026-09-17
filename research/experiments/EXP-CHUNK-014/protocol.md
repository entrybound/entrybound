# EXP-CHUNK-014: Sparse files and zero runs: hole representation

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CHUNK-014 |
| Status | NEEDS_TOOLING. Needed: `sparse_model.py`; a hole census (SEEK_DATA/SEEK_HOLE preceded by `posix_fadvise(POSIX_FADV_DONTNEED)`, the page-cache fix of `research/corpus/tools/fingerprint.py`, PROGRESS 2026-09-17); the generated sparse probe item; EXP-CHUNK-004 arm SQ occurrence tables for `balanced` and `extreme`; timing arm infrastructure from EXP-CHUNK-008. |
| Arms | **Z** bytes, census, headroom and WSL extraction allocation (`spec.yaml`). **T** encode CPU share of zero extents and extraction wall time, `--sparse logical` vs `--sparse restore`, status quo only (`spec-timing.yaml`, generated). **G** generated 8 GiB sparse probe (`spec-probe.yaml`, generated). **V1** validation look. **H** Phase D placeholder. |
| decision_ids | DEC-MOD-026 (hole representation), DEC-CMP-002 (chunk-count headroom on F15/F16 large files) |
| Proposed decision types | DEC-MOD-026: EMPIRICAL (bytes, time) + FORMAL (single size authority, I19/I20; hole geometry authentication) |
| Proposed od_ids | DEC-MOD-026: OD-01 (one authority per fact), OD-03 (hole restore fidelity; platform domain), OD-05, OD-06, OD-07, OD-18 |
| Proposed hc_ids | HC-01 (no second size authority), HC-02, HC-06, HC-07 (declared loss when holes are not restored), HC-10 (LAI independence from representation) |
| Coordination | The platform domain (§15) owns cross-platform hole restore fidelity (NTFS sparse, ext4/XFS/btrfs). This experiment measures WSL ext4 extraction allocation only, as a secondary check. |
| Gates / author / L7 | As EXP-CHUNK-001 section 1 |

## 2. Question

How should holes be represented? The options are:

- ordinary zero-filled chunks plus the `posix.sparse-map` AUX record (status quo);
- a registered hole TransformPlan storing no bytes (SPEC);
- a hole plan plus an AUX hint;
- relying on dedup of a canonical zero chunk without a sparse map.

The comparison covers archive bytes, chunking and hashing work, extraction cost and fidelity, and chunk-count headroom, on sparse and disk-image corpora.

## 3. Hypotheses

- **H1.** On F15 and F16 tuning items, the modelled `hole-transform-plan` is within the T-01 band of the status quo on `artifact_bytes`: zero chunks deduplicate and compress to a few bytes each. It therefore cannot meet the T3 minimum-gain band for a new plan type (§7.3). **Falsified if** it is `DISTINGUISHABLE_BETTER` against the T3 band on F15 at family level (validation).
- **H2.** Under the status quo, `bytes_hashed_by_chunker` / `logical_bytes` = 1 (holes are read and hashed as zeros) on every item, while `hole_bytes` / `logical_bytes` ≥ 0.5 on at least half of F15 items. This is secondary descriptive evidence that hole-aware capture could save work. The OD-06 verdict comes from arm T timing of the status quo only; an unimplemented design cannot be timed.
- **H3.** `dedup-single-zero-chunk` (dropping the sparse map) saves less than T-02's absolute threshold of metadata bytes per item. On restore it loses hole allocation fidelity (`extracted_allocated_bytes` > source allocated bytes), which is a declared-loss matter under HC-07 unless reported. **Falsified if** metadata savings exceed the T-02 threshold, or restore fidelity is unchanged.
- **H4 (headroom, DEC-CMP-002).** `max_chunks_in_one_object` stays below 1% of the 4,000,000-chunk limit for every tuning item and for the 8 GiB probe at the `extreme` preset. Zero runs cut at the forced maximum, `[INFERRED from docs/chunking-v1.md]`. **Falsified if** any object exceeds 40,000 chunks.
- **H0.** All representations are equivalent in bytes.
- **Model gate (binary).** For `sq-zero-chunks-aux-map-*`, `sparse_model.py` must reproduce the actual `ebr-pack` `artifact_bytes` exactly (`model_validity_exact = 1`) before any modelled candidate is reported.

## 4. Decisions informed

| Decision | Ledger required evidence covered | Elsewhere |
|---|---|---|
| DEC-MOD-026 | "Size, pack/extract time and hashing cost on sparse corpora per candidate": size (Z, modelled for unimplemented candidates); hashing work (Z); time for the status quo (T) | "Extraction hole fidelity per platform" (platform domain; WSL ext4 secondary check here) |
| DEC-CMP-002 | "Chunk-count headroom against the 4,000,000 limit on F15/F16 large files" (Z, G) | Size policies (EXP-CHUNK-004) |

## 5. Candidates

| Candidate | Definition | Ledger id | Measurement | Preliminary tier |
|---|---|---|---|---|
| `sq-zero-chunks-aux-map-{balanced,extreme}` | Status quo: holes read as zeros, chunked, deduplicated and compressed; `posix.sparse-map` AUX validated against zeros | `zero-chunks-plus-aux-map-status-quo` (**reference**) | actual | T0 |
| `hole-transform-plan-{…}` | Chunks lying entirely inside hole runs are replaced by per-hole plan records (logical length only, no stored bytes). Record bytes are modelled on the TransformPlan v2 record length (`entrybound::research::ecf::encoded_transform_plan_v2_len`) with a hole step carrying a varint length. No sparse-map AUX. | `hole-transform-plan` | modelled | T2/T3 (new plan identifier with decode semantics; T3 if a new record type) |
| `hole-plan-plus-aux-hint-{…}` | As above plus the existing AUX sparse map as a hint | `hole-plan-plus-aux-hint` | modelled | T2/T3, plus a second size-authority check (HC-01: R1(b) review of whether the hint duplicates the authority) |
| `dedup-single-zero-chunk-{…}` | Status quo archive without the sparse-map AUX item (zeros rely on dedup; holes are not restorable, declared loss) | `dedup-single-zero-chunk` | modelled from actual byte accounting (AUX item bytes removed) | T1 |

- **R1(b) screening note.** `hole-plan-plus-aux-hint` may violate HC-01 (two authoritative declarations of hole geometry). An exclusion needs a written counterexample and independent sign-off (§2 R1). Until then it is measured.
- R0 item 6: critic candidate open.

## 6. Corpus

- **Arm Z tuning:** F15 and F16 tuning items ≤ 8 GiB, 9 items (ids in `spec.yaml`), including real sparse VM disks and generated preallocation/VM-raw items.
- **Arm Z validation (V1):** F15 validation items (edge cases, fragmented, real Debian raw) and F16 validation raw and FAT images, plus F20 `f20-validation-sparsedup`, after the MDE gate.
- **Arm G:** generated item `exp-chunk-014-sparse-8g-probe`. An 8 GiB logical file has 256 MiB of seeded random data in 64 extents at seeded offsets (seed 20260917), written by `gen_sparse_probe.py` as sparse writes on ext4 scratch (seek past unwritten ranges, then truncate to the logical size; no hole punching). Allocated about 256 MiB. Generator, parameters, seed and `logical_tree_sha256` are committed.
- **Held-out:** Phase D placeholder (§5.5).
- **Families:** the sparse scope covers F15 (3 tuning independence groups) and F16 (2 tuning groups). Both meet the family-guard minimum of 2 groups, but corpus-level minimum support (≥ 10 groups across ≥ 5 families) does **not** hold for a sparse-only scope. The decision scope is therefore a family/policy partition (R9), and a universal default cannot be supported by this corpus alone. The effect on non-sparse families is checked by applying the hole model to all EXP-CHUNK-004 tuning occurrence tables. Families without holes must show exactly zero difference (binary expectation).

## 7. Environment and platform

- WSL x86-64, ext4 under `/root/eb-research` (never `/mnt/*` for extent work, PROGRESS caveat).
- Arm T timing per EXP-CHUNK-008 section 7 (calibration and runner-addition gates).
- Hole census needs no privileges (SEEK_HOLE and fadvise are unprivileged).
- Windows NTFS sparse restore: platform domain.

## 8. Commands

```sh
# EXP-CHUNK-004 arm SQ must have produced occurrence tables for balanced and extreme
$PY -m ebr validate --spec research/experiments/EXP-CHUNK-014/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-CHUNK-014/spec.yaml --gzip
$PY -m ebr verify-raw --spec research/experiments/EXP-CHUNK-014/spec.yaml --refingerprint
$PY -m ebr normalize --spec research/experiments/EXP-CHUNK-014/spec.yaml
python3 research/experiments/EXP-CHUNK-014/gen_sparse_probe.py --seed 20260917 --logical-bytes 8589934592 \
    --data-bytes 268435456 --extents 64 --out /root/eb-research/generated/EXP-CHUNK-014/exp-chunk-014-sparse-8g-probe \
    --record research/experiments/EXP-CHUNK-014/generated-item.json
$PY research/experiments/EXP-CHUNK-014/make_specs.py --arms probe,timing,all-families-null --out-dir research/experiments/EXP-CHUNK-014
#   timing (status quo only): ebr-pack --profile P (in-process phase timers) and
#   ebr-unpack with production `--sparse logical|restore` semantics (tooling flag), exclusive timing window
$PY -m decide analyze --experiment EXP-CHUNK-014 --decisions DEC-MOD-026 --out research/normalized/EXP-CHUNK-014/decision/
```

## 9. Seeds

- **Spec seeds:** `{order: 2026091841, bootstrap: 2026091842, command: 2026091843}`.
- **Probe generator seed:** 20260917.
- **Warmups:** 0 (arm Z); per EXP-CHUNK-008 (arm T).

## 10. Metrics

| Metric | Canonical | OD | Role | Threshold |
|---|---|---|---|---|
| `artifact_bytes` (actual SQ; `MODELLED` others) | yes | OD-05 | **primary** | T-01 |
| `side_data_bytes`, `metadata_bytes` | yes | OD-05 | diagnostic | T-02 |
| `encode_cpu_s` (arm T, SQ `--profile` packs; in-process) | yes | OD-06 | **primary** for the SQ cost baseline; an unimplemented candidate has no measured value, which is recorded as an R3 coverage gap | T-05 |
| `decode_wall_s` (arm T, `ebr-unpack`, logical vs restore) | yes | OD-07 | **primary** (SQ modes) | T-04 |
| `hole_bytes`, `hole_runs`, `zero_bytes_in_data_extents`, `bytes_hashed_by_chunker`, `max_chunks_in_one_object`, `extracted_allocated_bytes` | no | — | secondary | none |
| `sparse_restore_hole_fidelity` (extracted hole map equals source hole map on ext4) | related to `restore_fidelity_fraction` (T-20; platform domain owns the case list) | OD-03 | secondary here | — |
| `roundtrip_ok` (content equality) | `roundtrip_mismatches` | OD-02 | binary | §3.3 |
| `model_validity_exact` | no | — | binary gate | — |

## 11. Replication

- **Arm Z:** 2 repetitions and repeat-hash on `result_sha256`. The census uses the fadvise-cold extent read, so it is deterministic by construction (PROGRESS 2026-09-17 root-cause fix).
- **Arm T:** §4.6 rounds, tiers and adaptive rule (large items: 10 / 5 / 20, warmups 1).

## 12. Analysis

1. **Gates:** the model gate, then the all-families null check (zero difference where there are no holes).
2. **L1-L4 on `artifact_bytes`** as EXP-CHUNK-004 section 12, restricted to the sparse scope (F15, F16, plus F20 `sparsedup` on validation), with the family harm guard. R9 partition: policy "sparse-bearing content" is expressible only if a capture-time hole detector exists, which the status quo already has (SEEK_HOLE).
3. **Tuning W/S → MDE gate → V1** with R6 minimum gains at computed tiers (T2/T3 bands for the hole plan).
4. **Modelled-evidence rule** (as EXP-CHUNK-007 section 12 item 5): a modelled winner must be implemented as a research writer and re-measured (actual bytes and timing) before freeze. Otherwise the decision stays `INSUFFICIENT_EVIDENCE` with a provisional selection.
5. **Headroom:** per item, `max_chunks_in_one_object` and extrapolation to the 8 GiB tier bound at each frozen preset (descriptive, HC-06 declared limit check).
6. **Arm T:** SQ encode CPU share attributable to hole extents is estimated as the time for chunking and hashing zeros, from phase timers on the probe item. Descriptive.

## 13. Thresholds

T-01, T-02, T-04, T-05 as transcribed in `research/methods/thresholds.json`; `cost_tiers` multipliers. Not restated.

## 14. Sensitivity

- Plan record cost model ±50%.
- `balanced` vs `extreme` profile.
- Probe data fraction 3% vs 25% (a second generated probe, tuning only).
- Zero runs inside data extents (not holes) counted as holes or not (`zero_bytes_in_data_extents` arm).
- §6 arms within the sparse scope.

## 15. Expected negative results worth recording

- The hole plan saves only framing bytes, so the SPEC hole-plan design is not justified by size.
- The status quo's zero hashing is a measurable but small share of pack CPU.
- Dropping the sparse map loses restore fidelity for negligible savings.
- The corpus cannot support a universal decision (minimum support); the decision is family-scoped or `INSUFFICIENT_EVIDENCE`.

## 16. Threats to validity

- **Modelled plan bytes** depend on a hypothetical record encoding. Mitigated by ± sensitivity and the implementation requirement.
- **Extent maps** of qcow2, vhdx and ISO images (F16) are container-internal, not filesystem holes: those items test "zero runs in data", not holes. They are reported separately.
- **Generated probe:** hole and data geometry are generator choices.
- **Page-cache-dependent extent reads** (fixed by fadvise). Any census mismatch between repetitions is a tool defect and invalidates the run.
- **The status-quo in-memory pack** limits probe size to fit RAM (8 GiB logical); larger probes are the scale domain's.

## 17. Estimates

| Arm | Compute |
|---|---|
| Z | 8 candidates × 9 items (about 8 GiB logical) × 2 reps. SQ packs come from EXP-CHUNK-004 tables plus one `ebr-pack` pass per profile for the byte-accounting cross-check and extraction; modelled candidates take minutes. About **10 CPU-hours** |
| G | about 2 CPU-hours |
| T | about **20 wall-hours** |
| V1 | about 10 CPU-hours |

- **Disk:** extraction scratch ≤ 8 GiB logical (sparse, about 4 GiB allocated worst case); probe about 256 MiB allocated.
- **Agent effort:** scripted; judgment for the plan-record model, the HC-01 R1(b) write-up for the AUX hint candidate, and analysis.

## 18. Tooling to build

1. **`research/experiments/EXP-CHUNK-014/sparse_model.py`:**
   - hole census via `os.posix_fadvise(DONTNEED)` then `os.lseek(SEEK_DATA/SEEK_HOLE)`;
   - joins EXP-CHUNK-004 occurrence tables to hole runs;
   - models the candidates;
   - runs production `ebound unpack --sparse restore` and measures `st_blocks` allocation plus the hole map of the extracted file;
   - enforces the model gate.
2. **`gen_sparse_probe.py`** and **`make_specs.py`.**
3. **`ebr-unpack` flag for production sparse mode** (`--sparse logical|restore`, passed to `UnpackOptions`) and phase timers for the timing arm.

## 19. Deviations

(append-only)
