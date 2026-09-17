# EXP-CODEC-008: Legacy export wrapper codec parameters: level ladders, seekable and multi-block variants, reader acceptance, decoder memory and cross-host byte identity

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CODEC-008 |
| Domain | codecs (8.6; legacy export semantics are owned by the legacy domain, sections 25-27) |
| Status | NEEDS_TOOLING: `ebr-codec export-ladder` (wraps the production tar export with the pinned production encoders at alternative parameters, including seekable and multi-block variants), `read_variant.sh` (reader matrix), and the production `ebound` build (CC-2). |
| Stage | Deterministic size ladder, identity gates and reader census on tuning, then a validation look. Timing (compress in-process; decompress by incumbent readers as whole processes) on tuning small and medium items, then a validation look. Held-out: Phase D placeholder. |
| decision_ids | DEC-LEG-027 (are the frozen export codec parameters justified, or should new profile versions choose other levels?), DEC-CMP-024 (frozen wrapper parameters, what happens to a frozen wrapper profile when its pinned encoder must be upgraded, and whether seekable or multi-block profiles are needed) |
| Scope boundary | Export semantics, fidelity and receipt behaviour belong to the legacy domain (EXP-LEGACY-*). Encoder output drift across `flate2`/`miniz_oxide`/`zlib-rs`, `lzma-rust2` and libzstd versions for the frozen export profiles is EXP-ECO-002. This experiment reuses its results for the `new-profile-on-encoder-change` cost; it does not re-run them. |
| Proposed decision type | EMPIRICAL |
| Proposed od_ids | OD-05 (`artifact_bytes`), OD-06 (`encode_wall_s`), OD-07 (`decode_wall_s`), OD-08 (`peak_rss_bytes` of readers), OD-19 (`export_acceptance_fraction`), OD-13 (parallel-decode support, descriptive) |
| Proposed hc_ids | HC-13 (MVT-13(d): export bytes are a deterministic function of EAM and target profile; identical across hosts for the same archive), HC-16 (frozen wrapper profiles byte-identical under the frozen encoders), HC-07 (export fidelity unchanged by the wrapper codec; legacy domain) |
| Method, gates, author, L7 | EXP-CODEC-001 section 1 and Appendix C. |

## 2. Question

For the compressed legacy export wrappers (ZIP DEFLATE, tar.gz, tar.zst, tar.xz, tar.bz2), measured on the production tar/ZIP export of every tuning item:

- What artifact size, compression time, decompression time and reader memory does each level produce?
- Which seekable or multi-block variants do the common consumer readers accept?
- Which of those variants decode in parallel?
- Are frozen export bytes identical across hosts?

## 3. Hypotheses

- **H1a (size ladder, DEC-LEG-027).**
  - `max-ratio-levels` (gzip 9, zstd 19, xz 9) is `DISTINGUISHABLE_BETTER` than status quo on `artifact_bytes` for xz 9 vs xz 6 (predicted 1-3 T-01 band units on F05/F07) and for zstd 19 vs zstd 9 (2-5 band units).
  - For gzip 9 vs gzip 6 it is `EQUIVALENT`.
  - `incumbent-defaults` (zstd 3) is `DISTINGUISHABLE_WORSE` than status quo zstd 9 by 1-3 band units.
- **H1b (compress time).** zstd 19 is `DISTINGUISHABLE_WORSE` than zstd 9 on `encode_wall_s` by more than 10 T-03 band units (balanced band). xz 9 against xz 6 differs by less than 3 band units.
- **H1c (reader memory).** The xz 9 export needs a 64 MiB decoder dictionary. `peak_rss_bytes` of `xz -dc` is `DISTINGUISHABLE_WORSE` than for xz 6 (8 MiB dictionary) beyond the T-06 band. zstd 19 export frames at window log 23 or higher stay within the zstd CLI default memory limit.
- **H1d (seekable, DEC-CMP-024).** Every tested reader accepts multi-member gzip (RFC 1952 allows it) and multi-block xz. BGZF is accepted by every gzip reader. The zstd seekable format (a skippable-frame seek table) is accepted by every zstd reader as ordinary frames, and parallel random access needs format-aware tools. `export_acceptance_fraction` = 1 for all variants.
- **H1e (identity).** The same `.eb` archive exported on the Windows host and in WSL gives byte-identical wrapper output for every frozen profile (HC-13).
- **H0.** Every alternative level is `EQUIVALENT` to status quo on `artifact_bytes`, `encode_wall_s` and `decode_wall_s`.
- **Falsification.** At the validation look, the verdict class differs from the one stated. H1d and H1e are binary per cell.
- **Identity gate (binary; invalidates the ladder).** For every frozen wrapper, `export-ladder` at the frozen parameters must reproduce production `ebound convert --to <target>` output byte for byte on every small and medium tuning item.

## 4. Candidates

### DEC-LEG-027

| candidate_id | Parameters (ZIP DEFLATE / gzip / zstd / xz / bzip2) |
|---|---|
| status-quo-frozen-levels (reference) | 6 when smaller / 6 / 9 with checksum / 6 single block CRC64 / 900k (level 9) |
| max-ratio-levels | 9 / 9 / 19 / 9 / 9 (ledger: "gzip 9, zstd 19, xz 9") |
| fast-levels | `OPERATIONALIZATION` for the R0 critic: 1 / 1 / 1 / 0 / 1 |
| incumbent-defaults | 6 / 6 / 3 / 6 / 9 (ledger: "gzip -6, zstd -3, xz -6") |
| multiple-profile-variants | an R9 partition: per-use-case variants (fast, status quo, max-ratio), evaluated as scoped winners; cost is a new profile ID per variant (T2 per §7.3 "New profile" analogue for export profiles) |

The full level ladder is measured, so any other operationalization can be evaluated without new compute: gzip 1-9; zstd 1, 3, 6, 9, 12, 15, 19 and 22 (ultra); xz 0, 3, 6, 9, 6e and 9e; bzip2 1, 6 and 9; ZIP DEFLATE 1, 6 and 9.

### DEC-CMP-024

| candidate_id | Measured as |
|---|---|
| status-quo-frozen-wrappers (reference) | single member/frame/block at frozen levels |
| new-profile-on-encoder-change | the policy cost: the number of new wrapper profile IDs per 24 months implied by the encoder drift that EXP-ECO-002 observes. Reused, not re-run. |
| add-seekable-profiles | multi-member gzip (8 MiB members), multi-block xz (8 MiB and 64 MiB blocks), zstd seekable format (8 MiB frames plus seek table) |
| retune-levels | the level ladder (shared with DEC-LEG-027) |
| bgzf-style-gzip | BGZF blocks (≤ 64 KiB uncompressed per block, EOF marker) |

## 5. Corpus

- Every tuning item (size ladder, identity gate, reader census), then validation (look).
- Timing arm: tuning small and medium items. For the large tier, only status quo and max-ratio levels are timed (a declared limitation for the other levels).
- Export inputs:
  - production `ebound pack` (balanced, INDEXED) of each item;
  - production `ebound convert <archive.eb> <out.tar> --to tar` into `/root/eb-research/cache/EXP-CODEC-008/tar/<item_id>.tar`, with SHA-256 recorded before any ladder run;
  - the ZIP-export entries from `--to zip`.
- Items that production refuses to export (the symlink export refusal and path refusals recorded in the baselines README) are recorded as `REFUSED` for every candidate and excluded from ratios.

## 6. Environment

- WSL2 x86-64 for the ladder, timing and reader census.
- Windows host for the H1e identity arm, exporting the same WSL-packed `.eb` archives with the production Windows build.
- Readers are pinned in `research/baselines/tool-versions.json` (gzip 1.12, pigz, xz 5.4.5, zstd 1.5.5, bzip2, 7-Zip 23.01, bsdtar 3.7.2, GNU tar 1.35), plus Python 3 `gzip`/`lzma`/`bz2`/`zipfile`/`tarfile` and python-zstandard (pinned). Java 21 `GZIPInputStream`/`ZipInputStream` runs on the Windows host. `bgzip` comes from htslib (apt pin recorded).

## 7. Tooling and commands

### 7.1 To build

1. `ebr-codec export-ladder --tar <path> --wrapper gzip|zstd|xz|bzip2|zip --level <l> --variant none|members:<bytes>|blocks:<bytes>|seekable:<frame-bytes>|bgzf [--timing-mode in-process --reference-reader-time true] --summary-stdout`.
   - Encodes with the exact production encoder crates and pins (`flate2 =1.1.9` miniz_oxide backend, `zstd =0.13.3`, `lzma-rust2 =0.20.0`, `bzip2 =0.6.1`).
   - Emits `artifact_bytes`, `artifact_sha256` and the in-process `encode_wall_s`/`encode_cpu_s`. With `--reference-reader-time`, it also spawns the reference reader as a child process and reports its wall time.
   - For ZIP it re-encodes each entry with DEFLATE at the level, keeping STORE when not smaller, and matches production's ZIP layout.
2. `prepare_exports.sh`: production pack and convert into the tar/ZIP cache, with SHA-256.
3. `read_variant.sh <variant-file> <reader>`:
   - runs the reader under GNU time;
   - records acceptance (the output equals the tar/ZIP bytes), `peak_rss_bytes`, `decode_wall_s` (process) and whether multi-threaded decode was used (`xz -T0`, `pigz`, `zstd -T0`, `bgzip -@`);
   - prints one JSON line.
4. `identity_windows.ps1`: exports the same archives on Windows and compares SHA-256 values.

### 7.2 Commands

```sh
PY=/root/eb-research/venv/bin/python
bash research/experiments/EXP-CODEC-008/prepare_exports.sh
for S in spec.yaml spec-readers.yaml spec-timing.yaml; do $PY -m ebr validate --spec research/experiments/EXP-CODEC-008/$S; done
pwsh -File research/experiments/EXP-CODEC-008/identity_windows.ps1
```

## 8. Seeds and warmups

- Order and bootstrap seed: 20260917.
- Timing warmups follow CC-8. Reader timing uses the `hot` cache, and every candidate reads the same file.

## 9. Metrics

| Metric | OD | Role | Family | Threshold |
|---|---|---|---|---|
| `artifact_bytes` | OD-05 | primary | size | T-01 |
| `encode_wall_s` | OD-06 | primary (in-process encoder; exports are read-many distribution artifacts, so the T-03 "balanced" row is the pre-registered mapping) | time_wall | T-03 (balanced) |
| `decode_wall_s` | OD-07 | primary (incumbent readers, process timing; external tools, so no in-process timer exists; process floor applies) | time_wall | T-04 (process floor) |
| `peak_rss_bytes` | OD-08 | primary (reader memory; GNU time only, R1-16) | memory_peak | T-06 |
| `export_acceptance_fraction` | OD-19 | primary for the variants | other | T-20 |
| identity (Windows = WSL; ladder = production at frozen parameters) | — | binary (HC-13, HC-16) | correctness | §3.3 |
| parallel-decode used, reader-reported block or member counts | — | descriptive | other | — |

## 10. Replication

- Size ladder and census: deterministic; 2 repetitions plus repeat-hash (CC-7).
- Timing: §4.6 rounds with the adaptive rule (CC-8).

## 11. Normalization

CC-9. Readers are an evaluation condition: acceptance and decode time are reported per reader and never averaged across readers (AGG-S). The headline is the worst reader.

## 12. Statistical analysis

- **R1.** Identity-gate or HC-13 failures come first. An identity-gate failure invalidates the ladder. A cross-host difference is an HC-13 violation artifact of production, and is recorded as a defect.
- **DEC-LEG-027.** R4 on `artifact_bytes`, `encode_wall_s`, `decode_wall_s` and `peak_rss_bytes`, with equal base OD weights (§6.3). R9 partitions (`multiple-profile-variants`) are subject to expressibility (a new export profile ID) and to cost. Frozen profiles stay decodable, and a new level choice needs a new versioned profile ID (HC-16).
- **DEC-CMP-024.** For the seekable variants: `export_acceptance_fraction` = 1 is required per reader (an R1-like floor for a "widely readable" export). Then `artifact_bytes` overhead against single-stream, and `decode_wall_s` with parallel readers, are compared under R4.
- CC-10.

## 13. Practical-significance thresholds

T-01, T-03 (balanced row, pre-registered mapping), T-04, T-06, T-20; §7.3 "New profile" row as the analogue for new export profile IDs.

## 14. Sensitivity

- CC-11.
- Reader-subset arms: drop Java and drop the Python readers.
- ZIP-only and tar-only strata.
- Member and block sizes of 8 against 64 MiB.

## 15. Held-out (Phase D placeholder)

CC-12.

## 16. Expected negative results

- gzip 9 buys nothing over 6.
- xz 9 exports raise reader memory for little size gain.
- Seekable variants cost size and are only useful to format-aware readers.
- The frozen levels are a reasonable, non-dominated choice, a negative result for `retune-levels`.

## 17. Threats to validity

- `export-ladder` re-wraps the production tar stream. The identity gate proves equivalence only at the frozen parameters.
- Process timing of external readers includes process start-up. The process floor (T-04) applies.
- Reader versions are pinned to this environment; newer readers may differ.
- The large-tier timing coverage is partial (declared).

## 18. Platform and cost

| Item | Value |
|---|---|
| Platforms | Linux x86-64 WSL2 (ladder, readers, timing); Windows host (identity arm and Java readers). No network, no privileges. |
| Compute | Production pack and convert ≈ 25 CPU-h (balanced over 62.7 GiB). Size ladder ≈ 32 configurations × 62.7 GiB at a mean of about 1.5 s/MB ≈ 90 CPU-h (xz 9e and zstd 22 dominate). Reader census ≈ 5 CPU-h. Timing arm ≈ 60 h exclusive (small and medium tiers). |
| Disk | Tar cache ≈ 65 GB (transient, deleted after the run); variant outputs deleted after hashing; raw < 1 GB. |
| Agent effort | Scripted; analysis by a session without the L7 exposure. |

## 19. Deviations (append-only)

None.
