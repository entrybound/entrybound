# EXP-RECON-002: Reconstruction-target census — prevalence, placement, producers and coding modes of DEFLATE and JPEG streams (and other candidate targets) under production chunking

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-RECON-002 |
| Domain | reconstruction (8.8); common provisions: `research/experiments/_index/reconstruction-common.md` |
| Status | **NEEDS_TOOLING**: new harness crate `ebr-recon` (subcommand `census`), per §8 |
| Stage | Descriptive census. Tuning is exploratory. The validation census runs only inside the first registered look of the decisions below (common §5). |
| decision_ids | DEC-CMP-018, DEC-CMP-026, DEC-CMP-042, DEC-CMP-012, DEC-CMP-016, DEC-CMP-039, DEC-CMP-023 |
| Proposed types/ODs/HCs | common §2. No HC is screened here; the census feeds MVT-02(e) path planning and the common §7 target-family rule. |
| Gates | G-A for the record; nothing in this experiment is band-compared, so G-B statistics are not needed. The census values that define target families must be computed on tuning at a commit descending from this record. |
| Author and exposure | common §3 |
| Feasibility smoke | none (tool not built) |

## 2. Question

- **Streams.** In each tuning (and, at the look, validation) item, which byte ranges are:
  - DEFLATE streams, in which container or placement;
  - JPEG bitstreams, in which coding mode and marker layout;
  - instances of other entropy-coded formats with known lossless re-coders?
- **Prevalence and producers.** How much of each family's logical bytes do they represent, and which producers made them?
- **Reach.** Which fraction lands where the status-quo recognisers can reach it under production chunking in each profile?
  - DEFLATE: one complete stream = one whole Chunk ≤ 16 MiB.
  - JPEG: a whole ContentObject, SOF0, ending exactly at EOI; one Chunk in balanced, ≤ 4096 Chunks in dense/extreme.

## 3. Hypotheses (all `[INFERRED]` predictions; the census is descriptive)

- **H1 (DEC-CMP-042, DEC-CMP-018).** In each of F11 and F14 on tuning, status-quo-reachable DEFLATE bytes are < 25 % of that family's DEFLATE stream bytes in balanced, and ZIP-entry-embedded DEFLATE is ≥ 50 %. **Falsified if** reachable ≥ 25 % in either family.
- **H2 (DEC-CMP-026).** Over the real-camera F12 groups, SOF0 files ending at EOI carry ≥ 80 % of JPEG file bytes, and progressive or post-EOI-trailer files carry < 20 %. **Falsified if** progressive plus trailer ≥ 20 %. **H2b**, on the S-JPEG-REAL phone group once provisioned: ≥ 20 % of files carry post-EOI data. **Falsified if** < 20 %.
- **H3 (other targets).** No non-DEFLATE, non-JPEG target class meets the prevalence rule of §11 step 6. **Falsified if** any class meets it; this triggers a proposal for a new experiment, recorded as an amendment, not run under this record.
- **H4 (producers, `deflate-zlib-replay`).** Replay oracles (§8) attribute ≥ 50 % of whole-file and ZIP-entry DEFLATE stream bytes to an exact (library, parameters) configuration. **Falsified if** < 50 %.
- **H5 (DEC-CMP-039).** In F12 and F19 tuning, fewer than 1 % of JPEG ContentObjects share a Chunk with a distinct ContentObject under dense chunking, so dedup-conflict suppression is rare on real items. **Falsified if** ≥ 1 %. S-PHOTO-LIBRARY items are expected to exceed it by construction.
- **Tool-validity controls (binary; the run is invalid if either fails).**
  - **Positive control.** `ebr-recon census --self-test` builds the fixed control tree of §8.3 in memory and must report exactly the constructed counts and byte totals.
  - **Negative control.** The CSPRNG item `f20-tuning-csprng` must yield 0 validated DEFLATE streams and 0 JPEG files. Any positive is a parser false positive.

## 4. Decisions informed and how

| Decision | Contribution | Not settled here |
|---|---|---|
| DEC-CMP-042 | Fraction of DEFLATE and JPEG bytes in F11, F12, F14 (and every family) that land as one complete Chunk per profile policy; bytes reachable only with member-aligned cuts, single-chunk policy or embedded scanning | Net savings and dedup loss (004) |
| DEC-CMP-018 | Prevalence by container class (whole-file gzip single/multi-member, zlib, raw, ZIP entry, PNG IDAT, PDF Flate, git loose/pack, nested in tar/zip); producer classes; stream size distribution | Round-trip success and correction size (003); archive savings (004) |
| DEC-CMP-026 | JPEG prevalence by SOF class, trailer presence and kind, marker sets, megapixels, file size; embedded JPEG (PDF DCT, ZIP/OOXML, MPF secondary, EXIF thumbnails) | Backend success and savings (003, 004) |
| DEC-CMP-012 | Signature-gate truth set: the chunks that contain a validated stream. Enables offline false-negative/false-positive counts for signature and entropy gates against the always-try oracle of EXP-RECON-003 §5. | Pack time saved (005) |
| DEC-CMP-016 | Distributions of DEFLATE plaintext sizes and expansion ratios, JPEG megapixels and bytes, versus the frozen limits (16/16/64 MiB, 64×; 64 MiB, 100 MP, 4096 chunks, 64×) | Peak memory (006) |
| DEC-CMP-039 | Frequency of JPEG objects whose chunks are shared with other ContentObjects, or fall inside similarity cohorts (predictor) | Size effect of conflict policies (009) |
| DEC-CMP-023 | Prevalence denominator for "share of savings attributable to each generation" | Savings (004) |

## 5. Candidates

Not applicable: one measurement tool, no alternatives. The census classifies bytes, and the placement columns evaluate reach for these common §6 candidates:
- `deflate-v5-status-quo`, `deflate-signature-gated`, `deflate-multi-member-gzip`, `deflate-embedded-streams`, `deflate-whole-object-regions`;
- `jpeg-v6-status-quo`, `jpeg-add-progressive`, `jpeg-add-trailing`, `jpeg-nested`, `jpeg-balanced-multichunk`;
- `align-member-cdc`, `align-single-chunk-recognised`.

## 6. Corpus

- **Tuning.** All 112 tuning items of `research/corpus/manifest.json`.
- **Supplements** (common §7). S-JPEG-REAL, S-JPEG-PRODUCERS and S-DEFLATE-PRODUCERS are added by amendment as soon as they are provisioned. This is allowed before any result for those items exists.
- **Validation.** All 91 validation items plus validation supplements, run only inside the first registered look (common §5).
- **Held-out.** No census before unlock. The Phase D placeholder runs the census on held-out after Commit C, report-only (prevalence claims in product text must cite held-out prevalence).
- **Minimum support.** Descriptive, so not applicable. Group counts per family are reported with every prevalence number.

## 7. Environment and platform requirements

`wsl-ubuntu`, x86-64, no timing. Linux only: parsers are platform-independent. No network. Reads corpus items. Replay oracles need pinned `libdeflate` and `zlib-ng` builds (§8). Storage: detail JSONL (zstd) about 5–15 GB outside git.

## 8. Commands and tooling to build

### 8.1 `ebr-recon census`

New crate `research/harness/crates/ebr-recon`: safe Rust, workspace lints, harness held-out guard on every path.

```text
ebr-recon census --item-path <dir> --experiment-id EXP-RECON-002 --env-name wsl-ubuntu
                 --profiles fast,balanced,dense,extreme
                 --replay zlib-1.3,libdeflate-<pin>,miniz_oxide-0.8,zlib-ng-<pin>
                 --max-replay-plaintext-bytes 67108864 --nest-depth 4
                 --detail-out /root/eb-research/results/EXP-RECON-002/detail/<item_id>.jsonl.zst
                 [--self-test]
```

Behaviour, all detection by bytes only, never by extension (SPEC §5.2):

1. **Walk.** Walk the item exactly as production capture does: regular files, no following of symlinks, sparse files as logical bytes. Compute production chunk ranges per profile with `entrybound::chunker::select_parameters` + `chunk_ranges` (public API; production's per-object preset choice in dense/extreme, L-09), and per-chunk SHA-256 to find chunks shared across ContentObjects within the item.
2. **DEFLATE location and validation.** Every located stream must fully inflate with `flate2 =1.1.9` (miniz_oxide) within the bound and pass its wrapper checksum. Otherwise it is recorded as class `*.malformed`. Classes:
   - whole-file gzip, walking every member (`single` / `multi`, trailing garbage length);
   - whole-file zlib (CMF/FLG check, FDICT flag);
   - whole-file raw DEFLATE, attempted only for files ≤ 16 MiB that inflate completely with no trailing bytes;
   - ZIP local entries, method 8, validated against the central directory; method 9 counted separately;
   - PNG IDAT, concatenated per image;
   - PDF `stream … endstream` objects with `/FlateDecode` as the first filter (bounded tokenizer; object streams expanded once);
   - git loose objects (whole-file zlib whose plaintext starts with `<type> <len>\0`) and pack objects (`PACK` v2/v3 walk, including delta objects' zlib streams);
   - nested: gzip/zlib/ZIP found inside inflated gzip, tar or ZIP payloads, to `--nest-depth`.
3. **JPEG location.** Classes: whole-file (SOI at 0); PDF `/DCTDecode` streams; JPEG inside ZIP entries (after inflate); MPF secondary images; EXIF APP1 thumbnails. Per JPEG record:
   - SOF marker and class (SOF0 baseline, SOF1 extended, SOF2 progressive, SOF3 lossless, SOF9–SOF15 arithmetic/hierarchical);
   - dimensions and megapixels, components, sampling factors, restart interval, scan count;
   - APPn/COM marker list with lengths; DQT fingerprint (IJG-quality estimate by least-squares fit of standard tables); DHT standard versus optimised;
   - bytes after EOI and their class (zeros/0xFF padding, second SOI, ISO-BMFF `ftyp`, other);
   - structural anomaly flags (truncated, missing EOI, marker errors);
   - EXIF `Make`/`Model`/`Software` strings (producer evidence only).
4. **Other targets.** Byte counts only, per class: MP3 frames, GIF LZW image data, bzip2 streams, base64 MIME bodies, WebP (VP8/VP8L), HEIF/AVIF items, ISO-BMFF video `mdat`, FLAC frames, xz/zstd/brotli frames. Brotli is detected only inside HTTP-archive-like or known containers.
5. **Producer replay** (DEFLATE streams whose plaintext ≤ `--max-replay-plaintext-bytes`):
   - **Stage A (prefix).** Re-deflate the first 256 KiB of plaintext with every configuration of:
     - zlib 1.3: levels 1–9 × memLevel {1…9} × windowBits {9…15} × strategy {default, filtered, huffman-only, rle};
     - libdeflate: levels 1–12;
     - miniz_oxide: levels 0–10;
     - zlib-ng: levels 1–9.

     Keep configurations whose output is a byte prefix of the stream up to the last complete block boundary.
   - **Stage B (full stream).** Full-stream confirmation for Stage A survivors.
   - **Result.** `producer_class` = the first exact match in the fixed list order above, or `unmatched`. Container metadata is recorded as supplementary evidence: gzip OS/XFL, ZIP version-made-by, PNG `Software` text, PDF `/Producer`.
6. **Reach per profile:**
   - `status_quo_deflate_reachable` = (whole-file single-member gzip, zlib or raw) ∧ file ≤ 16 MiB ∧ file is exactly one Chunk in that profile;
   - `status_quo_jpeg_reachable_balanced` = whole-file ∧ SOF0 ∧ ends at EOI ∧ ≤ 64 MiB ∧ ≤ 100 MP ∧ one Chunk;
   - `…_dense`, `…_extreme` = the same with ≤ 4096 Chunks;
   - `member_aligned_reachable` = the stream starts and ends at forced cuts, which is always true for located streams, recorded with the extra cut count;
   - `shares_chunk_with_other_object` per JPEG object.
7. **Output.** One summary JSON row (schema `ebr.harness.raw.v1`, payload below) on stdout, plus one detail row per stream in `--detail-out`, including `detail_sha256`.

### 8.2 Payload keys (summary)

`input_bytes`, `input_files`, `deflate_stream_count`, `deflate_stream_bytes`, and bytes by class:
- `deflate_gzip_single_file_bytes`, `deflate_gzip_multi_file_bytes`, `deflate_zlib_file_bytes`, `deflate_raw_file_bytes`;
- `deflate_zip_entry_bytes`, `deflate_png_bytes`, `deflate_pdf_bytes`, `deflate_git_bytes`, `deflate_nested_bytes`, `deflate_malformed_bytes`.

Reach, producers and plaintext:
- `deflate_status_quo_reachable_bytes_{balanced,dense,extreme}`;
- `deflate_replay_matched_bytes`, `deflate_replay_unmatched_bytes`;
- `deflate_plaintext_bytes_max`, `deflate_expansion_ratio_milli_max`.

JPEG:
- `jpeg_file_count`, `jpeg_file_bytes`, `jpeg_sof0_bytes`, `jpeg_sof1_bytes`, `jpeg_sof2_bytes`, `jpeg_arith_or_lossless_bytes`;
- `jpeg_trailer_files`, `jpeg_trailer_bytes`, `jpeg_embedded_bytes`;
- `jpeg_status_quo_reachable_bytes_{balanced,dense,extreme}`;
- `jpeg_megapixels_max`, `jpeg_shared_chunk_objects_dense`.

Other targets and validity:
- `other_target_bytes` (object by class);
- `self_test_exact` (0/1), `detail_sha256`.

### 8.3 Positive-control tree (`--self-test`, fixed, in-memory, seed 20260918)

One each of:
- gzip single member (zlib level 6);
- gzip 3 members;
- zlib level 9;
- raw DEFLATE;
- a ZIP with 2 deflated and 1 stored entries;
- a PNG (2 IDAT);
- a PDF with 1 Flate and 1 DCT stream;
- a git loose object;
- a `PACK` with 3 objects;
- a tar.gz containing a zip;
- JPEG SOF0, SOF2 and SOF0 + 1 KiB trailer, from a fixed 64×48 RGB buffer via the `jpeg-encoder` crate already in production dev-dependencies (for SOF2 a hand-assembled fixture);
- an MP3 frame run;
- a 4 KiB CSPRNG blob.

Expected counts are hard-coded in the tool's test and compared on every run.

### 8.4 Runner

```sh
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools &&
  PY=/root/eb-research/venv/bin/python && S=research/experiments/EXP-RECON-002/spec.yaml &&
  $PY -m ebr validate --spec $S && $PY -m ebr run --spec $S --gzip && $PY -m ebr verify-raw --spec $S --refingerprint &&
  $PY -m ebr normalize --spec $S && $PY -m ebr report --spec $S'
# analysis (to be written): research/tools/recon/census_tables.py --detail /root/eb-research/results/EXP-RECON-002/detail \
#   --manifest research/corpus/manifest.json --split tuning --out research/normalized/EXP-RECON-002/decision/
```

## 9. Seeds, warmups, replication

- Seeds: 20260919 (order, bootstrap, command).
- Warmups: 0.
- Repetitions: 2 (deterministic; repeat-hash on `detail_sha256`, §4.13). A mismatch is a tool defect. The census claims no product determinism, so the metric is reclassified and investigated, not an R1 matter.

## 10. Metrics

All census payload keys are descriptive (common §8.2). The only canonical input is `input_bytes`. The census defines the target families (common §7) and the MVT-02(e) generated-item needs.

## 11. Normalization and analysis

1. `verify-raw`, then `normalize`. Samples with `self_test_exact = 0`, or a negative-control violation, invalidate the run.
2. **Prevalence tables.** Per item, then per group (mean of item fractions), per family (mean over groups), and corpus. Report weighted by `workload-mix.json` and with equal family weights (§4.9 conventions, descriptive), plus byte-weighted ratio of totals. Tables are stratified by scale tier.
3. **Reach tables.** Per profile, reachable ÷ located bytes per class (H1, H2), per family and group.
4. **Producer tables.** Bytes by `producer_class` × container class (H4), and JPEG quality-estimate and marker-set histograms by group.
5. **Size distributions.** Distributions against the frozen limits (DEC-CMP-016), including counts above each limit: the planner-eligibility loss attributable to each limit.
6. **Other-target prevalence rule (fixed now).** A class qualifies for a future reconstruction experiment if its tuning bytes are ≥ 1 % of logical bytes in ≥ 3 families, or ≥ 10 % in any family with ≥ 2 tuning groups. Qualifying classes are listed as an amendment proposal (H3).
7. **Target-family determination** by the common §7 rule for each candidate class. Written to `research/normalized/EXP-RECON-002/decision/target-families.json` before any EXP-RECON-004 analysis reads sizes. EXP-RECON-004 and 005 read that file.
8. **Gate truth set.** Write the per-chunk truth table `(item, profile, chunk_index, contains_validated_stream_class)` for EXP-RECON-003/004 gating analyses.

## 12. Practical-significance thresholds

Not applicable (descriptive). The 1 % / 10 % prevalence cut-offs in §11 steps 6–7 and common §7 are design rules for scoping follow-up work. They are not decision bands and never eliminate a candidate.

## 13. Sensitivity analysis

- Prevalence reported with and without derived-producer supplements, with and without F20, and per tier.
- Target-family sets recomputed at 0.5 % and 2 % cut-offs and reported. The 1 % rule governs.

## 14. Expected negative results worth recording

- Status-quo DEFLATE reach below 25 % in F11 and F14 (H1): most real DEFLATE is embedded or larger than one Chunk.
- A non-trivial share of progressive, arithmetic or post-EOI-trailer JPEG bytes in web and phone groups (H2b): the status-quo subset misses real content.
- Low replay match rates for non-zlib producers: `deflate-zlib-replay` has little reach.
- No other target class qualifies (H3): recorded as the reason no MP3, GIF or bzip2 reconstruction is designed.

## 15. Threats to validity

- **Parser recall.** Bounded, hand-written parsers can miss unusual containers (encrypted or linearised PDFs, ZIP64 edge cases). Mitigated by the positive control, by cross-checking whole-file gzip/zlib/JPEG counts against `file(1)`/libmagic (`file --mime-type`) on tuning items, reported as agreement counts, and by F20 structurally adversarial items.
- **Raw DEFLATE false positives.** Arbitrary bytes rarely inflate completely. Complete consumption is required, and the negative control guards the rule.
- **Replay-oracle incompleteness.** An `unmatched` result does not prove an unknown producer. Stage A may miss encoders whose early blocks match one configuration and later ones another. Reported as a lower bound.
- **Production chunk placement.** The census computes production chunk ranges. Planner-level effects (dedup across items, cohorts) are not modelled here; they are measured in 004.
- **Corpus representativeness:** common §13 item 1.

## 16. Held-out placeholder

Census on held-out after Commit C, report-only, to back any prevalence statement used in product claims (§10.1 item 3).

## 17. Cost and effort estimates

- **Compute.** About 40 CPU-hours: 62.7 GiB parse + inflate × 2 repetitions. The replay grid is bounded by Stage A prefixes, and the full zlib grid (9 × 9 × 7 × 4) is used only on 256 KiB prefixes.
- **Disk.** 15 GB detail outside git; < 20 MB normalized.
- **Tooling effort.** One harness agent session, scripted.
- **Agent effort.** Execution: none. Analysis: scripted tables plus a short judgment pass.

## 18. Looks, deviations and outcome (append-only)

- Looks: the validation census is part of look 1 of DEC-CMP-018/026/042, registered with EXP-RECON-004.
- Deviations: none.
- Outcome: `outcome.json`.
