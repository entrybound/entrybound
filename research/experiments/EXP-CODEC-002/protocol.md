# EXP-CODEC-002: In-process encode/decode throughput of codec, transform and C-reference decoder configurations at chunk scope

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CODEC-002 |
| Domain | codecs (program sections 8.6, 8.7) |
| Status | NEEDS_TOOLING. Needs `ebr-codec bench` and `prepare-batches` (section 7.2), the C-reference decoder bindings, and, for decision-grade timing, the §14 item 3 runner additions: adaptive rounds, cooldown and drift canary, the WSL host-side sampler, and on Windows the power-throttling opt-out, PDH frequency covariate and SMT sibling verification. Calibration under §4.7 must pass for `time_wall`, `time_cpu` and `throughput` in `wsl-ubuntu` and `windows-host`. |
| Stage | Tuning (all candidates), then validation confirmatory looks (W and S only). Held-out: Phase D placeholder. |
| decision_ids | DEC-CMP-008 and DEC-CMP-009 (the `decode_throughput_bps` branch of the §7.3 minimum-gain rows; decode and encode guards for the portfolios of EXP-CODEC-001), DEC-CMP-045 (decode time against window), DEC-CMP-043 (decode-speed impact of structural transforms; trial encode cost on non-target families, the §7.3 k = 2 condition), DEC-CMP-007 (decode throughput of the `c-reference-everywhere` decoders against the status-quo decoders) |
| Scope boundary | These related measurements belong to other domains' designs:<br>- pure-Rust Zstandard decoding (`ruzstd`), its correctness and throughput: EXP-ECO-003;<br>- decode timing of frozen table candidates and gates (DEC-CMP-034, DEC-CMP-012): EXP-PLANNER-004/007;<br>- dictionary and prefix decode (DEC-CMP-014): EXP-CROSSFILE-008;<br>- whole-archive production decode against incumbents (DEC-CMP-011): EXP-EVAL-005. |
| Proposed decision type | EMPIRICAL (timing parts) |
| Proposed od_ids | OD-06 (`encode_wall_s`, `encode_cpu_s`, `encode_throughput_bps`), OD-07 (`decode_wall_s`, `decode_cpu_s`, `decode_throughput_bps`) |
| Proposed hc_ids | HC-02 (round trip checked after every timed round), HC-11 (a decoded output difference between a C-reference decoder and the status-quo decoder is an R1 artifact, triaged per objective §2.0 rule 8) |
| Method, gates, author, L7 | As EXP-CODEC-001 section 1 and Appendix C (CC-4, CC-13). Timing results are not decision-grade before §14 items 3 and 4 hold in the environment used. |

## 2. Question

Measured with in-process timers under pinned single-thread affinity on WSL2 and on the Windows host, what encode and decode wall time, CPU time and throughput does each codec configuration, structural transform and C-reference decoder have? Each is timed on a fixed per-item batch of production chunks, to judge the speed branches of the codec-portfolio, window, transform and backend decisions.

## 3. Hypotheses

Effect sizes are `[INFERRED]` from REQ-CMP-0083/0096 (whole-file results on other hardware) and are predictions only.

- **H1a (LZ4 speed tier, DEC-CMP-008).** `lz4(production)` decode is `DISTINGUISHABLE_BETTER` than `zstd(production,level=1)` on `decode_throughput_bps` at corpus level. In both environments the interval's lower bound clears the §7.3 "New baseline-set codec" throughput band (predicted ratio 1.8-2.5).
- **H1b (LZMA2 decode cost).** `lzma2(production,preset=6,dict=4194304)` decode is `DISTINGUISHABLE_WORSE` than `zstd(production,level=19)` by more than 10 T-04 band units.
- **H1c (extended codecs, DEC-CMP-009).** PPMd decode is at least as slow as its encode and is `DISTINGUISHABLE_WORSE` than LZMA2 at the same profile. Brotli decode lies between zstd and LZMA2.
- **H1d (windows, DEC-CMP-045).** Zstandard decode throughput at window logs 21-27 is `EQUIVALENT` to log 20 on independent chunks of at most 2 MiB.
- **H1e (transforms, DEC-CMP-043).** Structural inverse transforms add less than 1 T-04 band unit of decode time relative to the same codec without the transform. Trying the status-quo transform set on non-target families costs `encode_wall_s` beyond the §7.3 k = 2 band at dense and extreme.
- **H1f (C-reference decoders, DEC-CMP-007).** liblz4 block decode is `EQUIVALENT` to `lz4_flex` or within 2 band units. liblzma raw LZMA2 decode is within 2 band units of `lzma-rust2`. Every C-reference decode is byte-identical to the status-quo decode.
- **H0.** Every comparison is `EQUIVALENT` at its band.
- **Falsification.** At the validation look, the verdict class of a named comparison differs from the one stated.
- **Parity control (harness review R1-06).** Decode of identical Zstandard frames through `entrybound::research::codec::decode_payload`, and directly through the `zstd =0.13.3` crate on the same libzstd build, must be `EQUIVALENT` at the T-04 band on every validation independence group. If not, production-codec timings here are informational only, and production decode claims move to whole-archive `ebound` timing (CC-2; EXP-EVAL-005).
- **A/A negative control.** Two identically configured candidates under distinct names must satisfy the §4.7 A/A criteria inside this run. Otherwise the run is not decision-grade.

## 4. Candidates (timed configurations)

The pre-registered minimum set is below. Before any timing run, `make_candidates.py` appends the Stage B survivors of EXP-CODEC-001 and EXP-CODEC-004 mechanically: every surviving configuration, timed at the chunking of each profile whose portfolio uses it. The appended list is committed before the run.

| Group | Configurations | Decisions |
|---|---|---|
| Production codecs (status quo; reference `zstd(production,level=3)`) | `store`; `zstd(production,level∈{1,3,5,9,15,19,22})`; `lz4(production)`; `lzma2(production,{4/1M, 6/4M, 9/8M})` | 008, 009, 045 |
| External codecs (EXP-CODEC-001 ledger candidates) | `brotli(external,quality=11,lgwin=24)`; `bzip2(external,level=9)`; `ppmd7(external,7zip-level=5)`; `bsc(external,bwt,qlfc-adaptive)`; `zstd-window(external,level=19,window_log∈{23,27},ldm=off)`; `zstd-ldm(external,level=19,window_log=24)`; `deflate(external,level=6)` (decode of DEFLATE, which is encoder-independent) | 008, 009, 045 |
| Production transforms (status quo) | `zstd+delta8` and `zstd+byte-shuffle-{2,4,8}` at levels 3, 9 and 15; `lzma2+delta8`, `lzma2+byte-shuffle-{2,4,8}` at 6/4M and 9/8M | 043 |
| Parity pair | `zstd(production,level=L)` against `zstd-window(external,level=L,window_log=20)` for L ∈ {1, 3, 19} | control |
| A/A negative control | `zstd-prod-l3-aa1`, `zstd-prod-l3-aa2` | control |
| DEC-CMP-007 decoders | status-quo-mixed (`lz4_flex`, `lzma-rust2`, libzstd via `zstd-sys`) against c-reference-everywhere (liblz4 block decode via `lz4 =1.28.1`; liblzma raw LZMA2 decode via the `liblzma` crate, pin recorded). libzstd is already the status quo, so it has no C-reference twin. pure-rust-default-c-optional and pure-rust-decode-c-encode (ruzstd) are EXP-ECO-003. vendored-frozen-encoders has no decode-speed difference (EXP-ECO-002). | 007 |

## 5. Corpus

- **Splits.** `tuning` (all candidates), then `validation` (W and S only, one registered look per decision).
- **Items.** Every tuning and validation item; minimum support per CC-9.
- **Batch per item.** An object-preserving seeded sample (seed 20260917) of production chunks (CC-5), at the chunking of the profile under test, capped at 8 MiB of unique plaintext (the whole item if smaller). Each batch file and its SHA-256 are fixed before any timed run (`/root/eb-research/cache/EXP-CODEC-002/batches/<item_id>-<profile>.ebrb`). The same batch is used by every candidate for that (item, profile).
- **Tiers.** Small, medium and large are reported as strata. The timing unit is the batch; the adaptive-rule tier is the item's scale tier.

## 6. Environment and platform requirements

| Environment | Affinity | Status | Qualifier |
|---|---|---|---|
| `wsl-ubuntu` | `st-v` {2} | primary for Linux scope | `VIRTUALIZED` (soft cap MEDIUM for Linux-native claims) |
| `windows-host` | `st-p` {2} decision-grade; `st-e` {18} report-only | primary for Windows scope | Defender on; MsMpEng covariate |
| Native ARM64 | — | PLATFORM_BLOCKED (EXP-CODEC-009) | decisions exclude ARM64 explicitly or stay blocked |

Timing windows are exclusive (§4.2). Inputs and scratch live on ext4 under `/root/eb-research` (WSL) or on local NTFS outside the repository (Windows).

## 7. Tooling and commands

### 7.1 Existing

`ebr-codec` production and external round-trip functions; ebr runner with `timing: true` and the quiet-machine guard.

### 7.2 To build

1. `ebr-codec prepare-batches`: writes the per-(item, profile) chunk batch file (untimed) and prints its SHA-256.
2. `ebr-codec bench --batch <file> --candidate-exact <label> --phase encode|decode|both --summary-stdout`.
   - Pre-encodes untimed.
   - Times with `ebr_common::timing::Stopwatch` (monotonic): the encode loop over all chunks, and the decode loop over all stored payloads. All I/O and input-buffer allocation are excluded.
   - Reads per-thread CPU time (Linux `RUSAGE_THREAD`; Windows `GetThreadTimes`) around each loop.
   - Verifies the round trip after timing (untimed). A failed round trip marks the sample invalid and HC-02 failed.
   - Emits `encode_wall_s`, `decode_wall_s`, `encode_cpu_s`, `decode_cpu_s`, `encode_throughput_bps` and `decode_throughput_bps` (logical batch bytes per wall second), plus `batch_sha256` and `payload_digest_sha256`.
   - Runs one configuration per process.
3. C-reference decoders in `ebr-codec/src/external.rs`: `lz4` block decode (existing pin) and `liblzma` raw LZMA2 decode (pin recorded). The decoded bytes are compared with the status-quo decoder's output before timing; any difference is an R1 artifact.
4. `research/experiments/EXP-CODEC-002/make_candidates.py`: the mechanical candidate append of section 4.
5. The §14 item 3 runner additions. These are not codec-specific; the runner work item owns them.

### 7.3 Commands

```sh
PY=/root/eb-research/venv/bin/python; S=research/experiments/EXP-CODEC-002/spec.yaml   # Windows: spec-windows.yaml
$PY -m ebr validate --spec $S && $PY -m ebr run --spec $S && $PY -m ebr verify-raw --spec $S \
  && $PY -m ebr normalize --spec $S && $PY -m ebr report --spec $S
# Decision analysis: research/tools/decide over research/normalized/EXP-CODEC-002 joined with EXP-CODEC-001/004.
```

## 8. Seeds, warmups, cache

- Seeds: order 20260917, bootstrap 20260917, batch sample 20260917.
- Warmups: 2 for small and medium items, 1 for large, all recorded. Cache is `hot`. Warmup adequacy follows the Kendall τ rule of §4.5.

## 9. Metrics

| Metric | OD | Role | Family | Threshold | Source |
|---|---|---|---|---|---|
| `decode_throughput_bps` | OD-07 | primary for the DEC-CMP-008/009 throughput branch and for DEC-CMP-007 | throughput | T-05b | in-process batch |
| `decode_wall_s` | OD-07 | guard: portfolio decisions of EXP-CODEC-001, DEC-CMP-045 per profile, and the decode impact of transforms (DEC-CMP-043) | time_wall | T-04 (in-process floor; small-tier rule) | in-process batch |
| `encode_wall_s` | OD-06 | guard at the profile's T-03 band; the §7.3 non-target condition of the "New reversible transform" row | time_wall | T-03 (per profile) | in-process batch |
| `encode_cpu_s`, `decode_cpu_s` | OD-06, OD-07 | secondary (single-thread, so approximately equal to wall time) | time_cpu | T-05 | in-process thread CPU |
| `encode_throughput_bps` | OD-06 | secondary | throughput | T-05b | derived |
| `roundtrip_mismatches` | OD-02 | binary | correctness | §3.3 | post-timing check |
| `wall_s` (process) | — | informational only | time_wall | — | ebr resource |

At most one primary metric per OD per decision. For DEC-CMP-008/009 the OD-07 primary is `decode_throughput_bps`, the metric the §7.3 rows name.

## 10. Replication and adaptive rule

- Rounds follow §4.6 by the item's scale tier: minimum rounds max(tier minimum, n_min(c)) at the comparison's confidence, then step and cap.
- Item level: exact order-statistic intervals of the median paired log ratio (§4.10).
- Adaptive rule: add rounds while the half-width exceeds 0.5 × log1p(r); items still imprecise at the cap are flagged `imprecise`.
- Pairing is within rounds (`item_repetition`), with randomized blocks.
- Until the adaptive-rule runner addition exists, the committed specs use the small-tier cap (30) as fixed repetitions. This is conservative for every tier.

## 11. Normalization and aggregation

- CC-9, per environment separately; environments are never paired.
- The small tier uses in-process timers as primary (the T-03/T-04 small-tier rule, which holds throughout here).
- Invalid-sample balance follows §4.4.

## 12. Statistical analysis

- CC-10.
- DEC-CMP-008/009: the §7.3 alternative, "`decode_throughput_bps` ratio interval lower bound beyond the tier band", with `artifact_bytes` from EXP-CODEC-001 required to be `NON_INFERIOR`.
- DEC-CMP-007: R4 among decoder backends on `decode_throughput_bps`, with cost tiers from EXP-ECO-001 (native or unsafe code raises the tier; §7.2 maintenance penalty) and R1 on any output difference.
- DEC-CMP-043: the §7.3 non-target `encode_wall_s` condition, evaluated on the always-try trial cost (the sum of transform-trial encode times per batch) at the k = 2 band.
- A claim covering both environments requires same-direction decision-grade support in both (§4.1).

## 13. Practical-significance thresholds

T-03 (per profile), T-04, T-05, T-05b; §7.3 rows "New baseline-set codec", "New registered-extended codec" and "New reversible transform".

## 14. Sensitivity

CC-11, plus these arms:

- `st-e` (E-core) results as a low-end hardware arm (report-only);
- batch cap of 2 MiB against 8 MiB on the small and medium tiers (cache-size sensitivity);
- the environment arm, Windows against WSL.

## 15. Held-out (Phase D placeholder)

CC-12: `spec-heldout.yaml` and `spec-heldout-windows.yaml` at Commit A, with every frozen candidate.

## 16. Expected negative results

- LZMA2 and PPMd decode are an order of magnitude slower than Zstandard, a cost on every read.
- C-reference decoders give no material speed gain over the pure-Rust status quo, which favours the lower-tier status quo.
- Long windows give no decode benefit for independent chunks.
- Parity may fail. Harness timings of production codecs would then be informational only.

## 17. Threats to validity

- **WSL2.** vCPU placement is uncontrolled, and VHDX I/O is not exercised because batches are in memory. Results carry the `VIRTUALIZED` qualifier.
- **Windows hybrid cores and Defender.** In-memory batches keep scan effects out of the timed loops. The frequency covariate and throttling-onset probe are required (§4.2).
- **Batch representativeness.** 8 MiB per item under-represents very large objects; the cache-size arm is reported.
- **Codegen.** research-internals may change inlining. The parity control covers Zstandard only; LZ4 and LZMA2 production paths have no direct-crate twin with identical bytes, so their parity is untested (a recorded limitation).
- **Compiler differences.** MSVC-built and gcc-built C libraries are part of the environment and are not separated.

## 18. Platform and cost

| Item | Value |
|---|---|
| Platforms | Linux x86-64 WSL2 (timing, virtualized); Windows 11 x86-64 host (timing; non-admin); native ARM64 BLOCKED (EXP-CODEC-009). No network, no privileges. |
| Compute | About 50 candidate configurations × 203 items × a mean of about 15 rounds (adaptive rule) × about 1.5 s per round, plus warmups: about 65 h exclusive on WSL and about 65 h on Windows `st-p`, plus an `st-e` report arm of about 20 h on a seeded 30% item sample. Total ≈ 150 h of exclusive timing windows; the fixed-cap fallback (30 repetitions) roughly doubles this. |
| Disk | Batches ≈ 203 × 4 × 8 MiB ≈ 6.5 GB (cache); raw rows < 1 GB. |
| Agent effort | Scripted execution in scheduled quiet windows. Judgment only for the analysis, by a session without the L7 exposure. |

## 19. Deviations (append-only)

None.
