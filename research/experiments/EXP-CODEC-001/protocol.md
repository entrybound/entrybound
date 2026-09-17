# EXP-CODEC-001: Chunk-scope codec portfolio: size screen and exact portfolio cost

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CODEC-001 |
| Domain | codecs (program sections 8.6, 8.7; program Task 8.6, REQ-CMP-0124) |
| Status | NEEDS_TOOLING (tooling list in section 7). Stage A (screen) and Stage B (exact portfolio cost) are both runner-driven. |
| Stage | Stage A: tuning split only, exploratory (R3/R4 without multiplicity control). Stage B: tuning (selection of W and S), then validation confirmatory looks (§5.2 registry). Held-out: Phase D placeholder (section 15). |
| decision_ids | DEC-CMP-008 (baseline codec set), DEC-CMP-009 (extended codec set), DEC-CMP-045 (Zstandard window at chunk scope) |
| Scope boundary with other domains | Other domains' designs cover these related questions, which this experiment does not duplicate:<br>- per-profile candidate tables (DEC-CMP-034) and gating (DEC-CMP-012): planner domain, EXP-PLANNER-002/004;<br>- generation ablation (DEC-CMP-023): EXP-RECON-004;<br>- density claims (DEC-CMP-011): EXP-EVAL-002/012;<br>- dictionary/prefix modes and prefix windows (DEC-CMP-014; the prefix part of DEC-CMP-045): EXP-CROSSFILE-003/004/008.<br>For DEC-CMP-009, EXP-PLANNER-004 arm F screens brotli, bzip2 and PPMd as modelled additions inside the frozen tables. This experiment supplies the set-level chunk-scope cost of every ledger candidate, those three included (the same `ebr-codec` build), and the only measurement of lzma2-xz, bsc, context-mixing, FLAC, scientific and OpenZL. |
| Proposed decision type | EMPIRICAL for the size, speed and memory parts. The SPEC §5.8 criteria audit behind DEC-CMP-008/009 is FORMAL plus EXTERNAL (EXP-CODEC-005). Assignment belongs to an independent session (`decision-method.md` §1.2). |
| Proposed od_ids for the assigner | DEC-CMP-008/009: OD-05, OD-07, OD-08, OD-21, OD-22 (OD-21/22 as cost inputs only). DEC-CMP-045: OD-05, OD-08, OD-10. |
| Proposed hc_ids for the assigner | HC-02 (round trip of every candidate payload), HC-06 (declarable decode memory), HC-11 and HC-12 (baseline decode steps need a public specification), HC-13 (deterministic encode per planner ID), HC-16 (frozen `zstandard/v1`, `lz4/v1`, `lzma2/v1` identifiers keep decode support) |
| Method | `research/decision-method.md` and `research/methods/thresholds.json` at the commit that adds this file |
| Gates | G-A unmet at this commit (constraint crosswalk, ledger schema v2 `hc_ids`/`od_ids`/`decision_type`, validation-look registry). This file is a design pre-registration; the §12.2 `record.md` is written from it once G-A holds. Nothing here is decision-grade before G-B (thresholds test, decision tooling §14 item 2, `workload-mix.json`, frozen held-out lock). |
| Author | Phase C-design codecs-domain session (2026-09-17). It implemented no candidate and no harness code. |
| L7 exposure disclosure | See Appendix C, CC-13. Summary: this session read `research/corpus/coverage.md` and `research/PROGRESS.md`, which name held-out item identifiers and upstream sources for most families. No held-out content, listing, statistic or path was opened; identifiers are not repeated here. Candidate sets below are transcribed from the ledger; where a ledger candidate needed an operationalization, it is marked `OPERATIONALIZATION` for the R0 completeness critic and the design review. |
| Feasibility smoke (NOT_DECISION_GRADE) | 2026-09-17, WSL: `ebr-codec object-matrix` and `transform-false-positives` on four tiny generated inputs (1 MiB CSPRNG, 1 MiB float32 sine ramp, a 30,000-line synthetic log, `/usr/bin/ls`) completed in 9 s. Finding: the default release binary `/root/eb-research/target/harness/release/ebr-codec` (mtime 01:03 local) predates harness review round 1 (`5bd33f8`, 02:42 local). Its rows lack `label`, `window_bytes` and `container`, and `--candidate` filtering had no effect. Every experiment in this domain therefore rebuilds the harness first (CC-1). |

## 2. Question

At Entrybound chunk scope (production `gear-norm-v1` chunk ranges per profile, production exact-chunk dedup), how many archive bytes does each codec portfolio need? The portfolios are the candidate baseline sets, the extended-set additions and the Zstandard window policies. Per portfolio the experiment also reports how often and on how many bytes each codec configuration wins, and how much each contributes at the margin.

## 3. Hypotheses

All predicted effects are `[INFERRED]` from published whole-file benchmarks (REQ-CMP-0083, REQ-CMP-0096) and from Entrybound's 128 KiB to 8 MiB chunk sizes; none is a measurement.

- **H1a (DEC-CMP-008, LZMA2 in the baseline).** At dense and extreme, `B_ZSTD_LZMA2` is `DISTINGUISHABLE_BETTER` than `B_ZSTD` on `artifact_bytes` at corpus level, by 1 to 3 T-01 band units. Its upper interval bound does **not** clear the §7.3 "New baseline-set codec" minimum-gain band at corpus level. It does clear that band in at least two families (predicted F05 and F07).
- **H1b (DEC-CMP-008, LZ4).** `B_ZSTD_LZ4` is `EQUIVALENT` to `B_ZSTD` on `artifact_bytes` at every profile. LZ4's case therefore rests only on `decode_throughput_bps` (EXP-CODEC-002).
- **H1c (DEC-CMP-008, DEFLATE).** `B_DEFLATE` is `DISTINGUISHABLE_WORSE` than `B_ZSTD` at balanced, dense and extreme, by at least 3 band units. `B_DEFLATE_ZSTD` is `EQUIVALENT` to `B_ZSTD`.
- **H1d (DEC-CMP-009, extended codecs).** No single extended addition to `B_ZSTD_LZMA2` clears the §7.3 "New registered-extended codec" band at corpus level. `E_PPMD` clears it on F05 and F06 text at dense and extreme; `E_BROTLI` clears it on no family. Context-mixing configurations beat every other configuration on the small-tier text items where they are measurable (the size of what the R2 screens exclude).
- **H1e (DEC-CMP-045).** For independent chunks, Zstandard window logs above 20 are `EQUIVALENT` to log 20 at balanced, dense and extreme, because chunks are at most 2 MiB. At fast (maximum chunk 8 MiB), window log 23 is better by less than 1 band unit.
- **H0 (equivalence).** Every portfolio's corpus-level ratio against the status-quo tables `B_SQ` lies inside the T-01 band.
- **What falsifies H1a-H1e.** The corresponding verdict class differs from the stated one at the validation look. A contradicted hypothesis is a recorded negative result (section 16), not a failed experiment.
- **Reconciliation control (binary; invalidates the run).** For every production-expressible portfolio, the modelled `artifact_bytes` (Appendix C, CC-6) must equal the bytes of a real harness-encoded archive, on every small and medium tuning item. Separately, `zstd(external,level=L,window_log=20)` must produce payloads byte-identical to `zstd(production,level=L)` on every sampled chunk. Otherwise the external/production comparison is confounded by frame flags, and the window series is reported only within the external family.
- **Round-trip control (binary, HC-02).** Every configuration's payload must decode to its chunk plaintext. Any mismatch is an R1 violation artifact for that configuration (objective §2.0 rule 2), and the configuration is excluded from every portfolio.

## 4. Decisions informed and what this experiment supplies

| Decision | Evidence item from the ledger `required_evidence` | Supplied here | Supplied elsewhere |
|---|---|---|---|
| DEC-CMP-008 | "Ratio, speed and memory on program corpora at chunk scope"; "Share of corpus bytes each baseline set would leave uncompressed or poorly compressed" | `artifact_bytes` per baseline portfolio; the diagnostic share of plaintext bytes whose best configuration in the portfolio is STORE or saves under 1% | speed EXP-CODEC-002; memory EXP-CODEC-003; criteria audit and decoder size EXP-CODEC-005; public-spec census and reader size per baseline set EXP-ECO-001/003; ARM64 speed EXP-CODEC-009 (BLOCKED) |
| DEC-CMP-009 | "Ratio, speed and decode memory versus Zstandard with dictionaries and long windows"; "Win frequency under exhaustive search in dense and extreme" | `artifact_bytes` per extended addition at chunk scope; win frequency and marginal contribution within each portfolio | exhaustive-search win frequency and the modelled R6 screen for brotli, bzip2 and PPMd EXP-PLANNER-002/004; dictionaries and long prefix windows EXP-CROSSFILE-003/004; decode footprint EXP-CODEC-003; float and VM portability EXP-CODEC-005 |
| DEC-CMP-045 | "Ratio versus window log at Entrybound chunk sizes per corpus group" | independent-chunk window series | prefix/LDM windows EXP-CROSSFILE-004; decoder memory and tool refusals EXP-CODEC-003 |

## 5. Candidates

### 5.1 Configuration universe (measurement, not candidates)

`configs/universe-stageA.json` lists every codec configuration measured in Stage A, pinned by backend and version. Production configurations go through `entrybound::research::codec` with the exact `TransformPlan`. They are:

- `store`;
- `zstandard/v1` at all seven `SUPPORTED_LEVELS`;
- `lz4/v1`;
- `lzma2/v1` at the three `SUPPORTED_LZMA2_CONFIGURATIONS`.

External configurations cover:

- Zstandard with an explicit window log (17, 19, 20, 21, 23, 24, 27), long-distance matching on and off, and negative levels;
- brotli (qualities 1/5/9/11, lgwin 22/24, and the large-window extension at lgwin 30);
- bzip2;
- DEFLATE, with three encoders for one format: `miniz_oxide`, libdeflate, zopfli;
- LZ4-HC levels (standard LZ4 block format, so an encoder-only alternative);
- raw LZMA2 at additional preset and literal-context parameters;
- PPMd var.H at 7-Zip levels 5/7/9;
- libbsc BWT+QLFC;
- context-mixing (small tier only);
- FLAC on derived PCM;
- typed numeric codecs (pcodec, fpzip) on declared typed layouts;
- OpenZL whole-object profiles.

Every row records `window_bytes`, `threads` (1), `container` and the pinned backend (harness review use constraint 5). Structural transforms are out of scope here: EXP-CODEC-004 reuses this experiment's untransformed rows as its reference.

### 5.2 Portfolios per decision (candidates)

Portfolios are defined in `configs/portfolios.json`. A portfolio is a set of configurations per profile. Its archive cost is the Chunk-stage portfolio cost model (CC-6) under the production v4 selection rule. The oracle minimum-stored-bytes rule is a sensitivity arm.

**DEC-CMP-008 (baseline set).** Every candidate is evaluated at each profile. The profile level list is intersected with the set, and fast keeps its SPEC §6.1 single-codec restriction.

| candidate_id (ledger) | Portfolio | Notes |
|---|---|---|
| status-quo-all-registered-mandatory (status quo, reference) | `B_SQ` | Codec entries of the frozen v4 tables (REQ-CMP-0165), transforms removed; `B_SQ_T` (with transforms, from EXP-CODEC-004 rows) is a sensitivity arm |
| store-only-baseline | `B_STORE` | |
| store-zstd | `B_ZSTD` | Profile level lists as frozen |
| store-zstd-lz4 | `B_ZSTD_LZ4` | |
| store-zstd-lzma2 | `B_ZSTD_LZMA2` | Frozen LZMA2 pairs per profile |
| store-deflate-zstd | `B_DEFLATE_ZSTD` | DEFLATE encoder per profile: `OPERATIONALIZATION` balanced `miniz_oxide` 6, dense libdeflate 12, extreme zopfli. Only the encoder differs; the decoded format is RFC 1951 in all three. |
| store-deflate | `B_DEFLATE` | same DEFLATE encoders |
| vm-bytecode-decoder | excluded | L0 exclusion recorded in the ledger (SPEC §20.7 frozen rejection). Not measured. |

R0 gaps flagged for the completeness critic, not added by this session (L7): the SPEC-proposed design (SPEC §25.1 row 3 leaves the set open, so "status quo" may be the only SPEC anchor); the strongest incumbent (xz/7z LZMA2 at whole-archive scope, measured by EXP-EVAL); "defer" (not applicable to a V1_BLOCKING baseline); the critic's own candidate. Any subset of the measured universe can be added later at no extra compute; an added candidate enters at R1 and is logged.

**DEC-CMP-009 (extended set).** Each addition `E_x` is `B_ZSTD_LZMA2` plus the configurations of x eligible at dense and extreme. `B_ZSTD` plus x is a sensitivity arm, because DEC-CMP-008 may drop LZMA2.

| candidate_id | Portfolio | Applicability |
|---|---|---|
| status-quo-lzma2-only (reference) | `B_ZSTD_LZMA2` | all |
| none | `B_ZSTD` (chunk scope); its "dictionaries and long windows" component is measured by EXP-CROSSFILE-003/004 | all |
| lzma2-xz | `B_ZSTD` plus LZMA2 frozen pairs plus `lzma2(external-raw,preset=9e)` | all |
| brotli | `E_BROTLI` | all |
| bzip2 | `E_BZIP2` | all |
| ppmd | `E_PPMD` | all |
| bsc-bwt | `E_BSC` | all |
| context-mixing | `E_CM` | small-tier items only (throughput); expected R2 screens are licence (GPL implementations) and unspecifiable floating-point or VM decoders. These are recorded as exclusions by an independent reviewer, not by this session, and the sizes are measured anyway as a negative-results bound. |
| domain-flac-audio | `E_FLAC` | F13 PCM only. No PCM item exists in the corpus, so experiment-local PCM is derived from the F13 FLAC tuning and validation items (CC-5) and is single-group per split. It cannot support a scoped winner alone (§4.9). |
| domain-scientific | `E_SCI` (pcodec, fpzip) | F07 items with a declared typed layout (`configs/typed-layouts.json`, written by the tooling session from item provenance documents before any Stage A result) |
| openzl-format-aware | `E_OPENZL` | whole-object, F06 and F07 items for which an OpenZL profile exists |

**DEC-CMP-045 (window).** At chunk scope, independent plans only.

| candidate_id | Portfolio change | Notes |
|---|---|---|
| status-quo-log20 (reference) | zstd window log 20 | |
| per-profile-windows | fast 20/23, balanced 20/21, dense 20, extreme 20 at chunk scope (logs above the chunk maximum give nothing to independent chunks) | full profile windows in prefix/LDM mode: EXP-CROSSFILE-004 |
| long-mode-dense-extreme | LDM on at log 24/27 for dense/extreme, with the log-24 LDM-off control | window-matched control per harness review R1-12 |
| window-sized-to-chunk | log = ceil(log2(profile maximum chunk)): fast 23, balanced 21, dense 20, extreme 19 | |
| rely-on-cross-file-dictionaries | status quo at chunk scope; cohort-stage part in EXP-CROSSFILE-003/004 | |

## 6. Corpus

- **Manifest.** `research/corpus/manifest.json` (`ebrc-2026.09-v1`, manifest_sha256 recorded in the raw meta at run time). Splits `tuning` (Stage A, Stage B tuning) and `validation` (Stage B looks). Never `heldout`.
- **Items.** Every tuning item (112 items across F01-F20) and every validation item (91). Per family, groups on tuning range from 2 to 9 and on validation from 2 to 7; the minimum support of §4.9 (≥ 10 groups across ≥ 5 families) holds in both splits. Group counts were generated from the manifest `split`/`family`/`independence_group` fields by the command in CC-9.
- **Stage A sample.** Per item, every production chunk if the item has at most 64 MiB of plaintext. Otherwise the tool draws ContentObjects without replacement (seed 20260917) and takes all their chunks until 64 MiB of plaintext is reached, so the sample preserves object structure. Chunking is `balanced` and `extreme`; `fast` is screened for fast-eligible configurations only.
- **Stage B.** Complete items, all four profiles, the Stage B universe only (section 12.1).
- **Derived experiment-local inputs** (CC-5): PCM from F13 FLAC items (`flac -d` at the pinned version, 16-bit little-endian); typed-layout extraction for F07 (safetensors headers, npy/npz headers, FITS BITPIX, and raw float32/float64 files whose provenance documents declare the layout). Every derived file is written under `/root/eb-research/generated/EXP-CODEC-001/`, with its generator command, seed (none needed) and SHA-256 recorded in `configs/derived-inputs.lock.json` before any Stage A result.
- **Silesia whole-object arm (descriptive; claim re-verification of REQ-CMP-0083 for EXP-EVAL's DEC-CMP-011 work).** The five tuning items derived from the Silesia corpus are compressed whole-object by every configuration in the universe. Published Silesia results are corpus-level over twelve files, so this arm can only report per-file values. The expected outcome is "not reproducible at corpus level" (section 16).
- **Workload-mix weights.** `research/methods/workload-mix.json` at its frozen SHA-256 (gate G-B).

## 7. Tooling and commands

### 7.1 Existing tools used

`ebr-codec` (`research/harness/crates/ebr-codec`), `ebr-planner` (`production_chunk_ranges`, `archive_regret`), `entrybound::research::{codec, planner, ecf}`, and the ebr runner. Preflight is CC-1.

### 7.2 Tooling to build (harness only; no production change)

1. `ebr-codec chunk-matrix` extensions:
   - `--universe <json>` (configuration list, replacing the fixed `MATRIX_*` constants);
   - `--production-chunking object` (per-ContentObject `chunker::select_parameters` + `chunk_ranges` over a directory item in production capture order, with exact-chunk dedup by plaintext digest);
   - `--chunk-sample-bytes` and `--chunk-sample-seed` (Stage A object-preserving sample);
   - `--detail-out <path.jsonl.gz>` (one row per unique chunk and configuration);
   - `--summary-stdout` (one `ebr.harness.raw.v1` summary line on stdout even when detail goes to a file);
   - `matrix_digest_sha256` over the sorted (chunk digest, label, payload SHA-256) triples, for the repeat-hash check.
2. New external configurations in `ebr-codec/src/external.rs`, each exact-pinned in `Cargo.toml`:
   - `ruzstd` is not needed here (pure-Rust decoding is EXP-ECO-003 and EXP-CODEC-002);
   - zstd explicit `window_log` series and negative levels (already-pinned `zstd =0.13.3`);
   - brotli large-window (`brotli =9.0.0` `large_window` parameter);
   - libdeflate (`libdeflater`, pin resolved and recorded at build time);
   - zopfli (`zopfli` crate, pin recorded);
   - raw LZMA2 with explicit `lc/lp/pb` and the extreme flag (`lzma-rust2 =0.20.0`);
   - libbsc through a checked-in C shim `research/harness/shims/bsc/` against libbsc 3.3.x built from source (download approved);
   - PPMd var.H (existing `ppmd-rust =1.5.0`).
3. Whole-object external CLI runner `research/tools/codecs/whole_object_cli.py` for configurations with no library binding: context-mixing (paq8px at a pinned release tag; zpaq 7.15 from `tool-versions.json`), FLAC (`flac` pinned apt version), OpenZL (`zli` built from the pinned release tag), fpzip (pinned tag). It records exit status, output size, round trip and GNU time `max_rss_bytes` (harness review R1-16: only rows with `measurement.method` = `gnu-time-v+wait4`).
4. Typed-layout extractor `ebr-codec/src/typed.rs`: safetensors, npy/npz, FITS, declared raw. It feeds pcodec (`pco` crate, pin recorded) and the fpzip CLI.
5. `ebr-planner portfolio` subcommand implementing CC-6:
   - reads the matrix detail rows for (item, profile, rep) and a portfolio definition;
   - applies the production v4 qualification rule (`planner::research::qualifies_v4` against the complete candidate cost, with external plan-record lengths from `configs/plan-record-model.json`);
   - charges distinct plan records once (`ecf::research::encoded_transform_plan_len`);
   - for production-expressible portfolios, encodes a real Chunk-stage archive through the public encoder with the selected plans and reports both the exact and the modelled `artifact_bytes`;
   - prints one summary line;
   - reuses a per-(item, profile, rep) matrix cache so that each repetition recomputes the matrix exactly once.
6. `research/experiments/EXP-CODEC-001/screen.py`: applies the Stage A screening rule (section 12.1) to the Stage A detail rows and writes `configs/universe-stageB.json`. It is stdlib-only and deterministic, and its output is committed before any Stage B run.

### 7.3 Commands

```sh
# Preflight (CC-1), then Stage A screen (tuning only):
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools &&
  PY=/root/eb-research/venv/bin/python &&
  $PY -m ebr validate --spec research/experiments/EXP-CODEC-001/spec-screen.yaml &&
  $PY -m ebr run --spec research/experiments/EXP-CODEC-001/spec-screen.yaml &&
  $PY -m ebr verify-raw --spec research/experiments/EXP-CODEC-001/spec-screen.yaml &&
  $PY -m ebr normalize --spec research/experiments/EXP-CODEC-001/spec-screen.yaml &&
  $PY research/experiments/EXP-CODEC-001/screen.py --raw research/raw/EXP-CODEC-001-screen \
      --out research/experiments/EXP-CODEC-001/configs'
# Stage B tuning (after the screen output is committed):
#   same sequence with --spec research/experiments/EXP-CODEC-001/spec.yaml
# Validation look: spec-validation-look1.yaml is derived mechanically from spec.yaml
#   (splits: [validation]; candidates restricted to W and S), registered in
#   research/decisions/validation-looks.jsonl and committed before the run.
# Decision analysis: research/tools/decide/ (decision-method §14 item 2) over
#   research/normalized/EXP-CODEC-001/ plus the joined guard metrics of EXP-CODEC-002/003.
```

## 8. Seeds, warmups, cache

- Order seed 20260917; bootstrap seed 20260917; chunk-sample seed 20260917, the same for both repetitions so that the repeat-hash is meaningful.
- Warmups 0 (deterministic metrics, §4.6). Cache state is irrelevant because nothing is timed.

## 9. Metrics

| Metric | OD | Role here | ebr family | Threshold ID | Source |
|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | primary (superiority metric for every decision in section 4) | size | T-01 | `ebr-planner portfolio` summary (`payload.artifact_bytes`); exact for production-expressible portfolios, CC-6 model otherwise |
| `metadata_bytes`, `index_bytes` | OD-05 | diagnostic | size | T-02 | Chunk-stage archive byte accounting (`ebr-pack` buckets) |
| `roundtrip_mismatches` | OD-02 | binary (HC-02; R1) | correctness | §3.3 | per configuration, summed over chunks |
| `fixed_overhead_bytes` | OD-05 | not computed here | size | T-21 | evaluation domain (M05.3) |
| win bytes fraction, win count, leave-one-out marginal band units, STORE-fallback plaintext fraction, `window_bytes` | OD-05 | descriptive, non-canonical, report-only (§0.2 item 4: never primary) | other | none | detail rows |
| `alloc_peak_bytes` (decode) | OD-08 | guard, joined from EXP-CODEC-003 | memory_peak | T-06 | |
| `decode_throughput_bps`, `encode_wall_s` | OD-07, OD-06 | guard / minimum-gain alternative, joined from EXP-CODEC-002 | throughput, time_wall | T-05b, T-03 | |

At most one primary metric per OD (R3). The joined guard metrics are primary for their own ODs in the decision analysis, which joins experiments on (item, candidate). This experiment supplies only OD-05.

## 10. Replication

Deterministic metrics (§4.6): 2 repetitions in different rounds and processes, then the repeat-hash check (§4.13) on `matrix_digest_sha256` and on the portfolio `portfolio_digest_sha256`. No encoder in the universe is claimed to be deterministic across builds. Within one build, a digest mismatch reclassifies the affected configuration as stochastic (§4.13), and its sizes are then summarised by the median over 10 repetitions. A mismatch on a production configuration is an HC-13 violation artifact (production claims deterministic unencrypted output) and is committed without a confirming rerun.

## 11. Normalization and aggregation

- L1 item value: exact `artifact_bytes`, with floors per tier (T-01: relative-only on the small tier). The candidate/reference log ratio is taken against the decision's reference portfolio.
- L2 to L4: CC-9.
- Strata: scale tier, and profile (a scope, not an averaged condition).
- Applicability per section 5.2. Inapplicable (item, portfolio) cells are reported as `NOT_APPLICABLE`. A domain codec is still charged its trial cost on non-target families, through EXP-CODEC-002.

## 12. Statistical analysis

### 12.1 Stage A screening rule (pre-registered, mechanical)

A universe configuration advances to Stage B if any of these holds on the tuning sample:

- (a) it is in a status-quo table;
- (b) at any profile it wins at least 1% of the sampled plaintext bytes in some family;
- (c) adding it to `B_ZSTD_LZMA2` improves that family's group-mean `artifact_bytes` log ratio by at least 1 T-01 band unit;
- (d) it is the best configuration of a ledger candidate family (brotli, bzip2, PPMd, bsc, context-mixing, FLAC, scientific, OpenZL, DEFLATE encoder), best meaning the smallest total sampled stored bytes over applicable items, so that every ledger candidate keeps at least one configuration;
- (e) it is a DEC-CMP-045 window-series point.

The cap is 40 configurations. Beyond the cap, (b)-qualified configurations are ranked by win bytes and the lowest are dropped; (a), (d) and (e) are never dropped. The Stage A verdicts are exploratory and are never cited as decision evidence.

### 12.2 Stage B

- Per decision, per scope: R1 (round-trip and reconciliation failures), R2 (DEC-CMP-008/009 criteria screens from EXP-CODEC-005 and EXP-ECO-001, and bounds from EXP-CODEC-003), R3 on tuning (W as the band-unit utility winner; for DEC-CMP-045 per profile, the smallest `artifact_bytes` candidate meeting guards), R4 on validation (W against each member of S).
- Confidences, MDE gate before each look and look budget: CC-10.
- R6 minimum gains use the computed tier (§7.2, from EXP-CODEC-005 and EXP-ECO-001 cost records plus the §14 item 7 scripts) and the §7.3 rows "New baseline-set codec" (DEC-CMP-008), "New registered-extended codec" (DEC-CMP-009) and the T1 or T2 rows for DEC-CMP-045 (a planner parameter within the existing record, or a new window field). Decode support of already-published identifiers is common to every candidate and is not a differential cost (§7.4). **Pre-v1 emission gets no sunk-cost credit**, so `lz4/v1` and `lzma2/v1` in a v1 baseline are costed as new baseline codecs.
- The §7.3 "≥ 2 applicable families with ≥ 2 groups each" condition for a new baseline codec is evaluated with the family harm guard's pooled intervals.

## 13. Practical-significance thresholds

Referenced, never restated: T-01 (`metric_thresholds.artifact_bytes`, including the small-tier relative-only rule), T-02, T-03/T-04/T-05b/T-06 for joined guards, §7.3 minimum-gain rows as named in 12.2, and the R10 tie rule.

## 14. Sensitivity analysis

§6 in full (CC-11). The profile-scope decision (DEC-CMP-045 per profile) uses Appendix B.1: no weights, threshold perturbation, the §6.5 family arms and the selection-stability bootstrap. Experiment-specific arms:

- selection rule: oracle minimum stored bytes versus the production v4 qualification rule;
- `B_SQ_T` (with transforms) versus `B_SQ` (codec-only), for DEC-CMP-008/009;
- extended additions over `B_ZSTD` versus over `B_ZSTD_LZMA2`;
- the byte-weighted arm with `total_stored_bytes` (sensitivity_arm role);
- the harness `archive_regret` bounds replacing per-chunk minima.

## 15. Held-out (Phase D placeholder)

At Commit A (§5.5), `spec-heldout.yaml` is written with `splits: [heldout]` and exactly the frozen candidates (every candidate that reached the freeze, not only W, per R5). It states the held-out MDE of every supporting comparison, computed from validation group-level variance and held-out group counts (fingerprint-level fields only). It runs once after unlock, with `EB_HELDOUT_UNLOCK` set to Commit A. `identity_aware_heldout` applies unless sealed tranches exist.

## 16. Expected negative results worth recording

- LZ4 adds no bytes saved at any profile (H1b); its retention depends on decode throughput.
- Window logs above 20 are useless for independent chunks (H1e). This is a negative result for "per-profile-windows" at chunk scope.
- Context-mixing wins on text but is excluded by R2: the measured size gap is the price of the licence and specification criteria.
- No typed or domain codec generalises beyond its target family, and the FLAC arm is single-group.
- Published Silesia figures are not reproducible from five of twelve files (REQ-CMP-0083 stays `SUPPORTED_WITH_LIMITATIONS`).

## 17. Threats to validity

- **Chunk-stage model.** The v6 cohort stage (dictionaries, ChunkGroups) and JPEG/DEFLATE reconstruction change which chunks use which codec. Portfolio effects under those stages are measured in EXP-RECON-004 (production generations) and EXP-CROSSFILE-003/004 (cohort modes), not here.
- **External plan-record model.** For codecs production cannot express, archive bytes are modelled. The model is validated exactly only on production-expressible portfolios (control in section 3). A new codec's real record length could differ from `configs/plan-record-model.json`; that file is committed before results and any real implementation is re-measured.
- **Encoder-only alternatives** (libdeflate, zopfli, LZ4-HC) change no decoder, but they are new pinned writer dependencies (C3/C7 churn), which EXP-ECO-001/002 and EXP-CODEC-005 charge.
- **Typed-layout declarations** are inputs written by a session; wrong declarations penalise typed codecs. They are committed before results and cited to provenance.
- **Context-mixing and FLAC** coverage is restricted to the small tier or to derived PCM; conclusions are scoped accordingly.
- **Cross-split content overlap** (critique round 2, for example F17 kernel headers) is a covariate. The §5.3 overlap audit governs.
- **Stale binaries.** Addressed by CC-1.

## 18. Platform and cost

| Item | Value |
|---|---|
| Platforms | Linux x86-64 (WSL2 Ubuntu 24.04, ext4 under `/root/eb-research`). No timing, so WSL qualifies without `VIRTUALIZED` caveats for size. Codec output is expected platform-independent; cross-host encoder identity is the planner and ecosystem domains' (EXP-PLANNER-009, EXP-ECO-002). No network, no privileges. |
| Compute | Stage A ≈ 30 CPU-h (≈ 2 h wall at 16 workers). Stage B tuning+validation ≈ 150 CPU-h (≈ 10-12 h wall at 16 workers). Total ≈ 180 CPU-h. |
| Disk | Persistent detail ≈ 3 GB gzip under `/root/eb-research/raw-detail/EXP-CODEC-001/`, SHA-256 in raw rows. Peak scratch ≈ 20 GB (reconciliation archives are deleted after hashing). Derived PCM and typed arrays ≈ 6 GB. |
| Agent effort | Scripted execution. Judgment only at: typed-layout declaration review, R2 screen sign-off (independent reviewer), analysis (a session without the L7 exposure, CC-13). |

## 19. Deviations (append-only)

None.

---

## Appendix C. Codecs-domain common conventions (CC-1 to CC-15)

EXP-CODEC-002 to EXP-CODEC-009 cite these by number.

- **CC-1 Preflight and build identity.**
  - `C:/Python313/python.exe research/orchestration/disk_guard.py` passes.
  - `python research/harness/lock_parity.py` prints `RESULT PASS`, and `research/harness/harness-cli-parity.sh` prints `RESULT PASS` (harness review use constraint 1).
  - The harness is rebuilt at the pre-registration commit: `CARGO_TARGET_DIR=/root/eb-research/target/harness /root/.cargo/bin/cargo +1.98.1 build --release --workspace --locked` (Windows: `D:/eb-research/target/harness`, `cargo.exe +1.98.1`).
  - The SHA-256 of every binary used goes into the spec's raw `meta`.
  - A run refuses to start if `ebr-codec object-matrix --candidate 'store(production)'` on a 1 KiB generated file emits a row without `label` and `window_bytes` (the stale-binary finding of section 1).
- **CC-2 Production build.** Timing against incumbents, and any "production" whole-archive decode claim, uses `ebound` built from the production workspace (`cargo +1.98.1 build --release --locked -p entrybound-cli`, default features) into `/root/eb-research/target/production` (harness review use constraint 2). Harness in-process codec timing is valid only for comparisons inside one harness build, and only after the parity arm of EXP-CODEC-002 passes.
- **CC-3 Held-out.** Specs select only `tuning`, `validation` or experiment-local generated sets. Every harness binary refuses held-out paths (R1-01). Detail and cache paths live under `/root/eb-research/{raw-detail,cache,generated}/EXP-CODEC-*`, never under a held-out root.
- **CC-4 Gates.**
  - G-A precedes each §12.2 `record.md`.
  - G-B precedes any decision-relevant result: thresholds test, §14 item 2 decision tooling, `workload-mix.json`, frozen held-out lock.
  - Timing and memory additionally need §14 item 3 (runner additions) and item 4 (calibration: `EXP-CAL-AA-<env>`, `EXP-CAL-AAP-<env>`, `EXP-CAL-KE-<env>`, `EXP-CAL-BG-<env>`, `EXP-CAL-SPD-<env>` for families `time_wall`, `time_cpu`, `throughput`, `memory_peak`).
  - HC-06 oracles need §14 item 17 (counting allocator, bound H).
  - Cost tiers need §14 item 7.
- **CC-5 Chunk scope and derived inputs.**
  - Production `gear-norm-v1` ranges per ContentObject via `chunker::select_parameters` and `chunk_ranges`, in production capture order, with archive-wide exact-chunk dedup (a chunk's bytes are counted once).
  - Experiment-local derived inputs are generated by a committed script, hashed, and recorded in the experiment's `configs/derived-inputs.lock.json` before any result. They carry the independence group of their source item.
- **CC-6 Chunk-stage portfolio cost model (PCM).**
  - For portfolio P at profile p: A(P) = A_enc(R) − S_R − Π_R + Σ_unique chunks s_P(chunk) + Π_P.
    - A_enc(R) is the exact size of a harness-encoded Chunk-stage archive using the production-expressible reference assignment R.
    - S is the summed stored payload bytes of the selected plans.
    - Π is the bytes of the distinct TransformPlan records.
    - s_P(chunk) is the payload size of the configuration the production v4 qualification rule selects from P (ties broken by universe order).
    - External record lengths come from `configs/plan-record-model.json`.
  - Frame header length is constant within an archive (`ecf::research::chunk_frame_header_len`), so it cancels.
  - Validation is exact equality with a real encode for every production-expressible P on small and medium tuning items. A failure invalidates PCM for all portfolios until fixed.
- **CC-7 Deterministic metrics.**
  - 2 repetitions in different rounds and processes.
  - Output digests per (item, candidate) are recorded, and the repeat-hash check follows §4.13.
  - Size reconciliation follows §4.13: section buckets sum exactly to `artifact_bytes`.
- **CC-8 Timing protocol.**
  - WSL: affinity `st-v` {2}, with `guard.max_loadavg_1m` = 2.0 (1.0 + 1 thread).
  - Windows: `st-p` {2} decision-grade, `st-e` {18} report-only.
  - Common to both: `guard.max_other_cpu_percent` 3.0, `max_wait_s` 300, cache `hot`, warmups 2 (small/medium tier) or 1 (large tier), `record_warmups: true`.
  - Rounds follow §4.6 by the item's scale tier, and the adaptive rule is applied by the §14 item 3 runner addition.
  - In-process monotonic timers around batch loops only. Process wall time is informational.
  - Exclusive timing windows (§4.2). WSL results carry `VIRTUALIZED`.
- **CC-9 Aggregation.**
  - Levels follow §4.9: L1 exact or median paired log ratio; L2 group mean; L3 family mean with equal group weights; L4 weighted by `research/methods/workload-mix.json`.
  - Strata are reported per scale tier and evaluation condition. The family harm guard applies.
  - Group counts come from `C:/Python313/python.exe -c "import json,collections;m=json.load(open('research/corpus/manifest.json',encoding='utf-8'));g=collections.defaultdict(set);[g[(i['split'],i['family'])].add(i['independence_group']) for i in m['items'] if i['split'] in ('tuning','validation')];print({k:len(v) for k,v in sorted(g.items())})"`.
- **CC-10 Statistics.**
  - Exact order-statistic item intervals and Welch-Satterthwaite corpus intervals (§4.10).
  - Confirmatory comparisons are W against each member of S. Superiority confidence is 1 − 0.01/k_sup, non-inferiority and equivalence 0.99, family guard 0.90, held-out one-sided 0.95 (§4.12).
  - MDE ≤ 1 band unit before each validation look, and ≤ 2 band units on held-out.
  - At most 2 validation looks per decision, each registered in `research/decisions/validation-looks.jsonl`. Cross-decision looks count when decisions share a candidate, a primary metric or the ledger cluster (all `compression`-cluster decisions here are related).
  - Rules R4-R10, cost tiers §7.2, minimum gains §7.3.
  - Only the §14 item 2 decision tooling produces decision-grade verdicts; ebr `normalize` verdicts are informational (§3.6).
- **CC-11 Sensitivity.** §6.3 weight grid and Dirichlet (non-profile decisions, per-OD base weights, equal by default); §6.4 threshold perturbation with the resolvability condition; §6.5 arms (equal-weight, leave-one/three-families-out, family Dirichlet, WM-* mixes, tier-stratified, byte-weighted, environment arm, reference arm); §6.7 selection-stability bootstrap; §6.6 bars.
- **CC-12 Held-out placeholder.** §5.5 and R5. `spec-heldout.yaml` is authored at Commit A and run once after Commit C.
- **CC-13 L7 disclosure and role separation.**
  - This design session read `research/corpus/coverage.md` (its independence-group table names held-out item identifiers and upstream groups in most families) and `research/PROGRESS.md` (whose change log names held-out upstream sources). This is an L7 identity exposure (§5.7) for the §5.8(h) record. No held-out content, listing, statistic or path under `/root/eb-research/heldout` or `/root/eb-research/fingerprints/heldout` was opened, and identifiers are not repeated in any file of this domain.
  - Under §5.4 consequence 3 this session may not design candidates or analyses for decisions whose families include exposed items. Accordingly:
    - (a) candidate sets are transcribed from the ledger, and every operationalization is marked for the R0 completeness critic;
    - (b) analysis plans invoke only the generic procedures of `decision-method.md` with no discretionary choices beyond the pre-registered screening, pruning and fitting rules stated in each protocol;
    - (c) the Phase C design review must decide whether a session without the exposure re-signs these candidate operationalizations and rules. Decision analyses are to be executed by sessions without the exposure.
- **CC-14 Outcome records.** `outcome.json` per experiment (§10.1). Negative, null and inconclusive outcomes go to the `research/reports/` register and to the `counterevidence` of each informed decision. Adversarial-search artifacts go under `research/experiments/<ID>/adversarial-search/` (§8.2 item 5).
- **CC-15 Number provenance.** §12.3. Every number comes from `ebr run` → `verify-raw` → `normalize` → `report` → decision tooling. Detail files outside git are cited by SHA-256 from committed raw rows.
