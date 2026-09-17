# EXP-SCALE-011: Codec and dictionary-trainer output invariance under worker counts, job sizes and library versions

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** |
| Domain | scale (program §14; interfaces with codecs §8.4 and dependencies §31) |
| Platforms | Linux x86-64 (WSL2); Windows host secondary (cross-host repeat of the production-pin arm) |
| Requires timing | false |
| Compute estimate | about 12 machine-hours (plus about 1 h of source builds) |
| Disk estimate | about 15 GB |
| Agent effort | scripted (source builds and flags are mechanical); judgment only for the lzma-rust2 and JPEG XL threading audit |
| Tooling to build | (1) pinned source builds (approved downloads; release tarball SHA-256 recorded in `research/experiments/EXP-SCALE-011/tool-pins.json` before use) of zstd v1.5.0, v1.5.2, v1.5.4, v1.5.5, v1.5.6, v1.5.7 CLI into `/root/eb-research/tools/zstd-<v>/` and xz 5.8.1 into `/root/eb-research/tools/xz-5.8.1/`; (2) `ebr-codec` harness feature `codec-mt` adding `--zstd-workers <n> --zstd-job-size <B> --zstd-overlap-log <l>` through the pinned `zstd` crate 0.13.3 with its `zstdmt` feature, and `--lzma-threads <n>`/`--jxl-threads <n>` where the pinned `lzma-rust2` 0.20.0 and JPEG XL crates expose threading (the audit records "not exposed" otherwise); built in a separate target dir with the lock-parity exception documented (the feature changes `zstd-sys` build flags); (3) `research/tools/scale/codec_invariance.py` (driver) |

## Pre-registration

- **decision_ids:** DEC-ACC-030, DEC-CMP-021, DEC-MOD-011, DEC-MOD-013.
- **Decision type:** EMPIRICAL (byte identity observations).
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-13 floor; HC-11, HC-13 (output identity across worker counts), HC-16 (frozen construction strings and planner ids pin encoder behaviour).
- **Method, gate, author, L7:** as EXP-SCALE-001.

## Question

At pinned versions, does compressed output of zstd (library and CLI), xz/LZMA2 (liblzma CLI and the pure-Rust `lzma-rust2` used in production) and JPEG XL encoding depend on the number of worker threads, on single-thread versus multi-thread mode, or on job and block size; and does zstd dictionary training output change across libzstd 1.5.x releases and trainer thread counts?

## Hypotheses

- **H1a (`[EXTERNALLY_SOURCED]` expectation to be verified locally, SPEC §12.2 and zstd documentation for multithreaded mode):** zstd output with `nbWorkers ≥ 1` is identical for every worker count at a fixed job size and overlap log, and differs from `nbWorkers = 0` (single-thread mode); xz output with `-T n` and a fixed `--block-size` is identical for every n ≥ 2 (block-split mode) and differs from `-T1` without block size.
- **H1b:** zstd dictionary training (`--train` legacy cover and `--train-fastcover`) is identical across thread counts at a fixed version, and differs across at least one pair of 1.5.x releases on at least one sample set.
- **H1c:** production codec paths used by planners (`zstd` crate single-thread, `lzma-rust2`, JPEG XL `with_num_threads(1)`) are byte-identical across WSL and Windows at the production pins.
- **H0 / falsification:** one differing output within a claimed-invariant group falsifies the corresponding claim; the two outputs are committed (≤ 16 MiB) or their digests plus reproduction command (larger).

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-ACC-030 | Whether zstd 1.5.7 and xz 5.8 outputs vary with worker count or only between single and multi mode; the `codec-internal-mt-pinned` candidate's precondition | parallel candidate evaluation prototypes: EXP-SCALE-012; speedups: EXP-SCALE-013 |
| DEC-CMP-021 | Trainer output diff across libzstd 1.5.x releases; trainer and archive identity across thread counts; cross-host identity at the production pin | x86-64 feature-level identity: EXP-SCALE-014; dependency audit: ecosystem domain |
| DEC-MOD-011 | Encoder output stability study for zstd and lzma versions and thread settings | preflate-rs and jixel version stability: codecs domain |
| DEC-MOD-013 | zstd/xz multithreaded output vs single thread at pinned parameters | — |

## Candidates

| candidate | Command (per sample item file F, output to scratch; digest of output recorded) |
|---|---|
| `zstd157-l<L>-st` for L in {3, 19} | `zstd-1.5.7 -<L> --single-thread -c F` |
| `zstd157-l<L>-t<n>` for n in {1, 2, 4, 8, 16, 32} | `zstd-1.5.7 -<L> -T<n> -c F` (default job size) |
| `zstd157-l<L>-t<n>-b64m` | `zstd-1.5.7 -<L> -T<n> -B67108864 -c F` (fixed job size) |
| `xz581-t1` | `xz-5.8.1 -6 -T1 -c F` |
| `xz581-t<n>-bs16m` for n in {1, 2, 4, 8, 16, 32} | `xz-5.8.1 -6 -T<n> --block-size=16MiB -c F` |
| `train-<v>-<mode>-t<n>` for v in {1.5.0, 1.5.2, 1.5.4, 1.5.5, 1.5.6, 1.5.7}, mode in {cover, fastcover}, n in {1, 8, 32} | `zstd-<v> --train[-fastcover] -T<n> --maxdict=112640 -o dict SAMPLES/*` |
| `crate-zstd-workers-<n>` for n in {0, 1, 2, 4, 8, 16, 32} | `ebr-codec object-matrix --codec zstd --level 19 --zstd-workers <n> --zstd-job-size 0` (library path at the production pin) |
| `crate-zstd-dict-t<n>` | `ebr-codec` trainer path (`zstd::dict::from_samples`) under `taskset` n CPUs (library trainer is single-threaded unless configured; recorded) |
| `crate-lzma2-threads-<n>` | `ebr-codec --codec lzma2 --lzma-threads <n>` if exposed, else `applicable: 0` with the audit note |
| `crate-jxl-threads-<n>` | `ebr-codec --codec jxl --jxl-threads <n>` on F12 JPEG items if exposed |
| `sys-zstd155-t<n>` | Ubuntu `zstd` 1.5.5 as installed (secondary: the system build) |

- **Excluded:** brotli and lz4 multithreading (not used by production planners; no decision asks).
- **Stress cell:** `zstd157-l19-t32-b64m` and `xz581-t32-bs16m` additionally run 20 times with `bgload.py --procs 31` (Appendix D.5 detection bound).

## Corpus selectors

Manifest `research/corpus/manifest.json`, split `tuning`: one file per item from F01, F05, F06, F07, F09, F10, F17 small and medium items (the largest regular file of each item, chosen by `codec_invariance.py --pick largest-file`), plus JPEG files from F12 tuning items for the JPEG XL arm; dictionary sample sets: all regular files ≤ 64 KiB from F01, F03 and F05 tuning items (per item, capped at 10,000 files, sorted by path). Items above 64 MiB are split into their first 256 MiB to bound compute. **Held-out placeholder:** not applicable (invariance is binary and version-bound; any selection relying on it re-checks on held-out files under the §5.5 unlock as part of EXP-SCALE-012's held-out run).

## Environment

`env_id` `wsl-ubuntu`; no timing; `bgload.py` only in stress cells. Windows arm: the `crate-*` candidates rebuilt on Windows at the same pins, run on the same files copied to `D:\eb-research\scale\codec-items`.

## Commands

```sh
bash research/experiments/EXP-SCALE-011/build_tools.sh    # downloads pinned tarballs, verifies SHA-256 from tool-pins.json, builds into /root/eb-research/tools
wsl.exe -d Ubuntu -- bash research/harness/build.sh build --release -p ebr-codec --features codec-mt
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-SCALE-011/spec.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python research/tools/scale/codec_invariance.py report --raw research/raw/EXP-SCALE-011 \
   --out research/normalized/EXP-SCALE-011/decision/invariance.csv
```

## Seeds

`seeds.order` 1311001, `seeds.bootstrap` 1311002, `seeds.command` 1311003.

## Warmup

0.

## Measurement method and metrics

| Metric | Canonical name | Role | Family |
|---|---|---|---|
| `output_sha256` | repeat-hash (§4.13) | binary within invariance groups | correctness (string) |
| `output_bytes` | T-01 (descriptive: multithreaded modes change ratio) | descriptive | size |
| `distinct_outputs_in_group` | none | binary (claim holds iff 1) | correctness |
| `dict_sha256` | none | binary across threads; descriptive across versions | correctness (string) |
| `threads_exposed` | none | audit | other |

## Replication

3 repetitions per (candidate, item) plus the 20-repetition stress cells. No adaptive rule.

## Normalization and statistical analysis

Invariance groups are pre-registered: {zstd157 L T∈{2..32} b64m}, {zstd157 L T∈{2..32} default job}, {xz581 T∈{2..32} bs16m}, {crate-zstd-workers n ≥ 1}, {train v mode T∈{1,8,32}}, {crate-* WSL vs Windows}. A group holds iff every output digest is identical across its members and repetitions. Cross-version trainer differences are counted per sample set. No statistics beyond counts.

## Practical-significance thresholds

None: binary oracles (§3.3). `output_bytes` differences are descriptive (T-01 would govern any later size decision).

## Sensitivity analysis

Level arm (3 vs 19); job-size arm (default vs fixed); sample-set arm for training (F01 vs F03 vs F05).

## Expected negative results worth recording

zstd multi-thread output differing from single-thread at every level (so `codec-internal-mt-pinned` needs a new planner id and permanent vectors); trainer output changing between libzstd releases (construction-string pins do not guarantee cross-version reproducibility); library threading not exposed for LZMA2 or JPEG XL.

## Threats to validity

1. CLI builds and the crate's `zstd-sys` build may differ in compile flags; both are measured.
2. Invariance on this x86-64 host says nothing about other ISAs (EXP-SCALE-014, EXP-SCALE-015).
3. Sample sets are tuning-only.

## Platform requirements

Linux x86-64 WSL2; build toolchain (gcc, make, cmake); Windows host for the cross-host arm; about 15 GB.

## Estimated compute and effort

About 12 h; scripted.

## Deviations (append-only)

None.
