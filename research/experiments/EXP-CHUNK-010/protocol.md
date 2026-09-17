# EXP-CHUNK-010: Chunking × reconstruction: stream containment, stream-aligned chunking and region conflicts

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CHUNK-010 |
| Status | NEEDS_TOOLING: `ebr-chunk stream-containment` (stream detectors, forced-cut chunking, reconstruction upper-bound probe through research-internals `reconstruction::try_forward` / `jpeg_reconstruction::verified_forward`); `ebr-cdcpack --align`; `ebr-planner region-order` (research-only planner orderings); generated photo-library item |
| Arms | **A** containment census (`spec.yaml`, SQ presets); **B** alignment alternatives: census (`spec.yaml`) plus end-to-end `artifact_bytes` (`spec-arm-b-pack.yaml`, generated); **C** dedup loss from forced cuts on version series (inside `spec.yaml` on F18); **D** region vs dedup/cohort conflicts (`spec-arm-d.yaml`, generated); **V1**; **H** placeholder |
| decision_ids | DEC-CMP-042 (align chunk boundaries or regions to embedded streams), DEC-CMP-039 (region vs shared/cohort chunk conflicts; audit reasons) |
| Proposed decision types | EMPIRICAL (both); DEC-CMP-039 audit-reason split has a FORMAL part (reason-code semantics) |
| Proposed od_ids | DEC-CMP-042: OD-05, OD-06, OD-08. DEC-CMP-039: OD-05, OD-06, OD-04 (audit truthfulness via `reason_code_specificity_fraction`-like census, secondary). |
| Proposed hc_ids | HC-02 (exact round trip of reconstruction), HC-11 (gated library-version decode steps: preflate and jixel are T4, never baseline), HC-01 (one representation per chunk), HC-16 (frozen v5/v6 eligibility) |
| Coordination | The reconstruction domain (section 8.8) owns whether DEFLATE and JPEG reconstruction ship (DEC-CMP-018, DEC-CMP-026) and their prevalence and net-savings measurements. This experiment measures only the chunking interaction. Its reconstruction probe is an upper bound. |
| Gates / author / L7 | As EXP-CHUNK-001 section 1 |

## 2. Question

- How much recognised DEFLATE content survives content-oblivious CDC as complete single chunks, which is the frozen v5 eligibility condition? Recognised DEFLATE means gzip members, zlib streams, and raw DEFLATE inside ZIP entries, PNG IDAT, PDF FlateDecode and OOXML. The same census covers JPEG files.
- What do stream-aligned chunking alternatives recover in net bytes?
- What do those alternatives cost in forced-cut dedup loss on version series?
- How often do dedup sharing and similarity cohorts suppress whole-object JPEG regions?
- Would region-first or joint complete-cost ordering produce smaller archives?

## 3. Hypotheses

- **H-A.** Under the four frozen presets, less than 50% of recognised DEFLATE stream bytes in F11 and F14 lie in chunks that equal exactly one complete stream (v5-eligible). JPEG containment is irrelevant because v6 uses whole-object regions. **Falsified if** ≥ 50% on either family at `balanced`.
- **H-B1.** `member-aligned-balanced` raises `deflate_single_chunk_eligible_fraction` above 0.9 on F11 and F14. Its reconstruction upper-bound saving is below the T3 minimum-gain band on `artifact_bytes` at corpus level (a forced-cut rule is a new planner behaviour, T1; region-spanning DEFLATE regions add a record type, T3). **Falsified if** the upper bound exceeds the band of the candidate's computed tier.
- **H-B2.** `single-chunk-recognised-16m-balanced` has a higher eligible fraction than `member-aligned` but a larger `access_granularity_bytes` (reported from arm B pack). **Falsified if** it is not higher.
- **H-C.** On F18 version series, `member-aligned-balanced` changes `dedup_stored_fraction` by less than the T-01 band compared with `sq-balanced`. **Falsified if** outside the band on the F18 family mean.
- **H-D1.** On photo-library content with exact duplicates and metadata-only edits, `RegionDedupConflict` suppresses a region for ≥ 5% of JPEG objects. **Falsified if** < 5%.
- **H-D2.** `joint-complete-cost` is not `DISTINGUISHABLE_BETTER` than status-quo cohort-first ordering on `artifact_bytes` against the T1 band at corpus level, and region-first is `NON_INFERIOR`. **Falsified if** joint is better against T1.
- **H0.** Alignment and ordering alternatives are `EQUIVALENT` to the status quo.
- **Binary:** every reconstruction probe result used in a saving estimate round-trips exactly (the writer's frozen verification rule). An inexact attempt counts as zero saving and is recorded.

## 4. Decisions informed

| Decision | Ledger required evidence covered | Not covered |
|---|---|---|
| DEC-CMP-042 | Fraction of DEFLATE and JPEG bytes in F11, F12 and F14 that land as one complete chunk per CDC policy (A); net savings and CPU of each alignment candidate (B, with CPU from EXP-CHUNK-008 arm T2 on W/S); dedup loss from forced cuts on F18 (C); encode/verify memory for 16 MiB-bounded streams (B pack, `peak_rss_bytes` scoped, informational until timing and memory calibration) | Whether reconstruction ships (reconstruction domain) |
| DEC-CMP-039 | Frequency of region suppression by dedup and by cohort conflicts on F12, F17 and F18 photo libraries with duplicates and edits (D); archive-size delta for region-first, cohort-first and joint selection (D); audit schema cost (D, modelled record bytes) | Explain accuracy of split reasons as a user claim (human-facing, external) |

## 5. Candidates

### DEC-CMP-042 (arms A-C)

| Candidate | Definition | Ledger id | Preliminary tier |
|---|---|---|---|
| `sq-{fast,balanced,dense,extreme}` | CDC ignores streams; DEFLATE reconstruction only when one chunk is one stream; JPEG via v6 whole-object regions | `status-quo-per-chunk-and-jpeg-regions` (**reference**: `sq-balanced`) | T0 (reconstruction steps themselves are T4-gated, HC-11) |
| `member-aligned-balanced` | Force cuts at recognised stream start and end offsets, CDC within | `member-aligned-cdc` | T1 |
| `single-chunk-recognised-16m-balanced` | Recognised compressed streams ≤ 16 MiB stored as exactly one chunk; CDC elsewhere | `single-chunk-for-recognised-compressed-files` | T1 (maximum chunk size exception interacts with declared limits, HC-06) |
| `whole-object-region-model-balanced` | DEFLATE regions spanning chunks: reconstruction applied to the whole stream; region record bytes modelled with `encoded_reconstruction_region_len` | `whole-object-deflate-regions` | T3 (new record usage; T4 because of the preflate decode step) |
| Embedded-stream scanning | Detectors for PNG, ZIP, PDF and OOXML are part of every census; the region model uses them | `embedded-stream-scanning` | T3/T4 |

**Detectors** (content-blind parsers, bounded):

- gzip member header plus trailer validation;
- zlib header check (CMF/FLG, no preset dictionary) with inflate-to-end success;
- ZIP local-file-header method 8;
- PNG IDAT concatenation (zlib);
- PDF `/FlateDecode` stream objects;
- OOXML (ZIP container);
- JPEG SOI..EOI with a marker walk.

Every stream is capped at `--max-stream-bytes` = 16 MiB (the reconstruction input limit).

### DEC-CMP-039 (arm D)

| Candidate | Definition | Ledger id | Preliminary tier |
|---|---|---|---|
| `cohorts-first-conservative` | Production v6 (**status quo; reference**) | `status-quo-cohorts-first-conservative` | T0 |
| `region-first` | Regions selected before similarity cohorts; region members excluded from cohorts (research ordering in `ebr-planner region-order`) | `region-first` | T1 |
| `joint-complete-cost` | For each conflicting object, compare region savings against the displaced cohort and dedup savings using production cost primitives; choose the lower total | `joint-complete-cost` | T1 |
| `split-audit-reasons` | Same selection as status quo; audit reason split into dedup-sharing vs dictionary/group conflict (modelled record-length delta; reason-code census) | `split-audit-reasons` | T2 (new reason code) |

- **Excluded:** `duplicate-representation-for-shared-chunk`. R1 per ledger: one frame per chunk digest and single region membership (`ecf/container.rs:1856-1858`, I1).
- R0 item 6: critic candidates open for both decisions.

## 6. Corpus

- **Arms A-C tuning:** all tuning items of F09, F11, F12, F13, F14 and F18, 34 items (ids in `spec.yaml`). F09 contributes embedded compressed sections; F18 contributes forced-cut dedup loss.
- **Arm D tuning:**
  - F12 tuning items;
  - F17 tuning items;
  - F18 tuning items;
  - plus **one generated item** `exp-chunk-010-photo-library-gen`: a seeded generator `research/experiments/EXP-CHUNK-010/gen_photo_library.py` over the F12 tuning JPEGs (Kodak-derived and NASA IVL). It creates exact duplicates, EXIF-only edits and lossless crops at pre-registered rates (duplicates 20%, EXIF edits 20%, crops 10%, seed 20260917):
    - EXIF-only edits rewrite APP1 segment bytes in Python;
    - lossless crops use `jpegtran -crop` from Ubuntu `libjpeg-turbo-progs`, version recorded (an approved apt download).

    It is committed with the generator, parameters, seed and output SHA-256, is tuning split only, and is derived from tuning upstreams only (§5.4 no same-upstream issue).
- **Validation V1:** F11, F12, F14 and F18 validation items for W and S of both decisions. No generated validation photo library exists; creating one from validation upstreams is a pre-registered option, generated and committed before the look.
- **Held-out:** Phase D placeholder (§5.5; EXP-CHUNK-004 section 6).

## 7. Environment and platform

WSL x86-64. Deterministic metrics (bytes, fractions); CPU and memory for W/S come from EXP-CHUNK-008. The reconstruction probe uses the production preflate and jixel paths through research-internals, at the pinned crate versions (recorded). No privileges, network or external review.

## 8. Commands

```sh
# preconditions/builds as EXP-CHUNK-004 section 8
$PY -m ebr validate --spec research/experiments/EXP-CHUNK-010/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-CHUNK-010/spec.yaml --gzip
$PY -m ebr verify-raw --spec research/experiments/EXP-CHUNK-010/spec.yaml --refingerprint
$PY -m ebr normalize --spec research/experiments/EXP-CHUNK-010/spec.yaml
python3 research/experiments/EXP-CHUNK-010/gen_photo_library.py --seed 20260917 \
    --sources f12-tuning-kodak-jpeg-matrix-medium,f12-tuning-nasa-ivl-photos-small \
    --out /root/eb-research/generated/EXP-CHUNK-010/exp-chunk-010-photo-library-gen --record research/experiments/EXP-CHUNK-010/generated-item.json
$PY research/experiments/EXP-CHUNK-010/make_specs.py --arms b-pack,d --out-dir research/experiments/EXP-CHUNK-010
#   arm B pack: ebr-cdcpack --align member-aligned|single-chunk-recognised:16777216 ... (production v6 planner)
#   arm D: ebr-planner region-order --item-path {item_path} --order cohorts-first|region-first|joint --profile balanced
#          --audit-census true   (archive bytes from the research ordering + production encoder)
$PY -m decide analyze --experiment EXP-CHUNK-010 --decisions DEC-CMP-042,DEC-CMP-039 --out research/normalized/EXP-CHUNK-010/decision/
```

## 9. Seeds

- **Spec seeds:** `{order: 2026091801, bootstrap: 2026091802, command: 2026091803}`.
- **Generator seed:** 20260917.
- Detectors are deterministic.
- Warmups 0.

## 10. Metrics

| Metric | Canonical | OD | Role | Threshold |
|---|---|---|---|---|
| `artifact_bytes` (arm B pack; arm D) | yes | OD-05 | **primary** (both decisions) | T-01 |
| `side_data_bytes` | yes | OD-05 | diagnostic | T-02 |
| `dedup_stored_fraction` (arm C) | yes | OD-05 | diagnostic | T-01 |
| `access_granularity_bytes` (arm B pack) | yes | OD-10 | secondary guard reporting | T-09 |
| `encode_cpu_s`, `peak_rss_bytes` (W/S via EXP-CHUNK-008) | yes | OD-06, OD-08 | **primary** (DEC-CMP-042 OD-06/OD-08) | T-05, T-06 |
| `deflate_single_chunk_eligible_fraction`, `jpeg_single_chunk_fraction`, `recognised_*_bytes`, `reconstruction_upper_bound_saved_bytes`, `reconstruction_exact_fraction`, `forced_cut_count`, `bytes_scanned_by_detectors` | no | — | secondary | none |
| Arm D: region suppression counts by cause (dedup sharing, cohort dictionary, chunk group), audit record bytes | no | — | secondary | none |
| `result_sha256`, `detail_sha256` | no | — | repeat-hash, provenance | §3.3 |
| `roundtrip_ok` (arm B pack, arm D) | `roundtrip_mismatches` | OD-02 | binary R1 | §3.3 |

## 11. Replication

2 repetitions and repeat-hash for all arms (deterministic). No adaptive rule.

## 12. Analysis

- **Arm A:** per family and per preset, byte-weighted and group-mean eligible fractions (descriptive). This is the prevalence input for the reconstruction domain.
- **Arm B:**
  - L1-L4 on `artifact_bytes` versus `sq-balanced` (EXP-CHUNK-004 section 12 procedure);
  - the upper-bound saving is reported next to actual `artifact_bytes`;
  - tuning W and S for DEC-CMP-042; MDE gate; V1 with R6 minimum gain at computed tiers (T3/T4 alternatives need large gains, §7.3 "New reversible transform" row applies to region-spanning DEFLATE regions: target families F11/F14, non-target families non-inferior including detection cost on `encode_wall_s`).
- **Arm C:** `dedup_stored_fraction` log ratio `member-aligned` vs `sq-balanced` on F18 (tuning; the family harm guard is evaluated on validation).
- **Arm D:**
  - suppression frequency per object by cause;
  - `artifact_bytes` deltas for the orderings;
  - tuning W and S for DEC-CMP-039; V1 per EXP-CHUNK-004 section 12.

  `split-audit-reasons` is a zero-byte-selection-difference candidate, so its evaluation is cost (T2) versus a truthfulness census. Explanation accuracy for users is a human-facing claim and is not decided here (§4.16).

## 13. Thresholds

T-01, T-02, T-05, T-06, T-09 as transcribed in `research/methods/thresholds.json`; `cost_tiers` multipliers and the §7.3 minimum-gain table for transforms. Not restated.

## 14. Sensitivity

- §6 arms for both decisions.
- Detector set ablation (without PDF, without OOXML).
- Stream cap 16 MiB versus 4 MiB.
- Photo-library generator rates halved and doubled (tuning only; reported, not used for selection).
- Profile `balanced` versus `extreme` for arm D.

## 15. Expected negative results

- Alignment recovers eligible bytes, but reconstruction savings on already-compressed families are too small for T3/T4 bands.
- Region conflicts are rare on real F12 items and occur mostly on the generated library.
- Joint ordering ties cohort-first.
- Forced cuts cost nothing measurable on F18.

## 16. Threats to validity

- **The reconstruction probe is an upper bound** (it ignores side-data framing and planner qualification margins). Mitigation: arm B pack measures actual bytes where the design is expressible.
- **Detector false positives and negatives:** unit-tested on synthetic streams; detection recall is not measured against a ground truth. Limitation recorded.
- **Generated photo library:** its conflict rates are generator choices; real F12 items are reported separately.
- **Library-version-defined decode steps (preflate, jixel)** are T4 and never baseline (HC-11); their use in any winner carries that cost.
- **Research planner orderings** replicate production cost primitives (research-internals rule 4) but not a production writer; a winner needs implementation before freeze.

## 17. Estimates

| Arm | Compute |
|---|---|
| A-C | 7 candidates × 34 items (about 12 GiB) × 2 reps; detectors + chunking + reconstruction probes (preflate is slow, about 5-20 MB/s on eligible streams): about **40 CPU-hours** |
| B pack | 3 candidates × about 12 GiB × 2 reps under v6 `balanced`: about **15 CPU-hours** |
| D | 3 orderings × about 5 GiB × 2 reps: about **20 CPU-hours** |
| V1 | about **30 CPU-hours** |

- **Disk:** stream detail files about 2 GB; generated item ≤ 2 GB.
- **Agent effort:** scripted; judgment for the detectors, the generator and `region-order` (planner replication).

## 18. Tooling to build

1. **`ebr-chunk stream-containment`:** detectors listed in section 5; `--align none|member-aligned|single-chunk-recognised:<n>|whole-object-region-model`; `--reconstruction-probe` through `entrybound::research::reconstruction::try_forward` and `jpeg_reconstruction::verified_forward` (exact round trip required); payload fields as named in `spec.yaml`.
2. **`ebr-cdcpack --align`** (forced cuts passed through the range-injection wrapper).
3. **`ebr-planner region-order`:** research-only orderings over `trace_archive` state, using production cost primitives only; production encoder for bytes; audit census.
4. **`research/experiments/EXP-CHUNK-010/{gen_photo_library.py, make_specs.py}`.**

## 19. Deviations

(append-only)
