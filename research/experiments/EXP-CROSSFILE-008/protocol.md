# EXP-CROSSFILE-008: Decode-side latency and DAG-parallel decode speedup per lookback topology and dictionary policy

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: `ebr-crossfile decode-bench` (in-process batch timers, iterative-bounded and DAG-parallel research decoders; `research/harness/ebr-crossfile-DESIGN.md`) is not built. Also gated: §14 item 3 runner additions and §4.7 calibration including `EXP-CAL-SPD-wsl-ubuntu` (speedup A/A) for `time_wall`; emulated-network corroboration needs §14 item 18 (`EXP-NETEM-CAL` regenerated in a quiet window). |
| Domain / program sections | crossfile / 8.5 (8.4) |
| decision_ids | DEC-CMP-027 (OD-07 first-byte latency, OD-13 parallelism loss); DEC-ACC-025 (latency of last members of long groups; iterative-bounded decoder); DEC-CMP-020 (decode time per dictionary); DEC-CMP-014 (decode throughput of post-v1 dictionary arms); DEC-CON-004 (decode peak memory under timing conditions, corroboration) |
| Evidence type | timing (in-process batches, §3.2 T-08), speedup within one core class (T-14); emulated remote corroboration of EXP-CROSSFILE-004's modelled latency (§4.15) |
| Method | `research/decision-method.md`, pre-registered |
| Record | Design only; `record.md` (§12.2) instantiated once gate G-A holds; exclusive quiet windows only. |
| Design session and disclosure | Phase C-design crossfile designer; L7 disclosure as in EXP-CROSSFILE-001. |

## 1. Question

How much first-byte and random-entry latency does each lookback depth, dependency topology and
dictionary policy add for a local reader, how much parallel-decode speedup does the dependency
structure forfeit when a DAG-respecting decoder uses n threads within one core class, and does the
emulated network agree in direction with the modelled remote latency of EXP-CROSSFILE-004?

## 2. Hypotheses

- **H1a.** `random_entry_latency_s` p50 of dependent members grows approximately linearly with the
  transitive closure bytes; for chained k = 8 in 128-member groups the last member is more than
  2 band units of T-08 slower than k = 0, and `restart-n4` / `cap-group-len8` stay within 1 band
  unit of k = 1.
- **H1b.** Measured `parallel_speedup` at n = 8 on `mt-v-8` follows the EXP-CROSSFILE-004 work-span
  bound in rank order across topologies; chained groups lose more than the T-14 band against
  k = 0 on cohort-heavy items (F18, F04).
- **H1c.** Dictionary-coded member reads add a Dictionary load cost below the T-08 band for
  Dictionaries <= 32 KiB.
- **H1d.** The `iterative-bounded-decoder` has latency non-inferior to the production decoder and a
  lower decode `alloc_peak_bytes` on long groups.
- **H0.** No topology or dictionary policy changes latency or speedup beyond the bands.
- **Falsification.** Confirmatory verdicts on validation contrary to H1a-H1d; for H1b, a Kendall
  tau between measured speedup and the bound below 0.5 across topologies (secondary).

## 3. Decisions informed; proposed OD and HC sets

OD-07 (`first_entry_latency_s` primary), OD-10 (`random_entry_latency_s` secondary; OD-10 primary is
`access_amplification` from EXP-CROSSFILE-004 unless a reviewed second-primary justification is
accepted, R3), OD-13 (`parallel_speedup` primary), OD-08 (`alloc_peak_bytes` corroboration of
EXP-CROSSFILE-006), OD-12 (emulated `remote_random_entry_latency_s`, corroboration only). HC-11
floor for the parallel decoder: decoded output identical for n in {1, 2, 8, 32} (objective MVT-11,
Appendix C.1), checked per sample.

## 4. Candidates

Archive configurations (production-encodable only, so the production decoder can be timed):
`prod-{balanced,dense,extreme}`, `forced-chain-k{0,1,2,4,8}-w1m-extreme`,
`forced-cap-group-len{4,8,16}-extreme`, `forced-dict-sq-abs128-balanced`,
`forced-dict-charge-access-k1-balanced`, `forced-no-dictionaries-balanced`, plus the tuning
survivors S of EXP-CROSSFILE-003/-004 that are production-encodable (added by rule before the
campaign). Research-only topologies (`restart-n*`, `anchor-k*`, `bsolid-*`, `leader-only-star`,
`ldm-*`, k >= 16, non-1 MiB windows) are timed with the research decoder only, labelled
`research-decoder`, and compared only among research-decoder rows (never paired with production
decoder rows).

Decoder variants: `production` (entrybound `open_indexed_random` + `read_entry`),
`iterative-bounded` (DEC-ACC-025 candidate), `dag-parallel` (research decoder that decodes the
archive's Chunks honouring the dependency DAG with n worker threads; used only for OD-13).

Reference: `prod-<profile>` with the `production` decoder; for OD-13 the same configuration at
n = 1 in the same core class.

## 5. Corpus selectors

- Manifest `ed28b1b4…`, tuning only: the 20 small items and the 10 medium items of the
  EXP-CROSSFILE-007 §5 rule, plus the three long-group stress items of EXP-CROSSFILE-004 §5.
- Reads: shared access sample (EXP-CROSSFILE-005) and T2 draw (EXP-CROSSFILE-002); >= 1,000
  operations per batch (repeating the sampled reads in seeded order), >= 40 sampled operations
  per item per round for p95 (Appendix C.1).
- Emulated remote corroboration: 10 items (one per target family) through `ebr-netem-proxy` at
  NP-METRO and NP-CONT only (configured RTT below 20 ms is avoided, harness review use constraint
  7), driven by `research/experiments/EXP-CROSSFILE-008/run_remote_corroboration.sh` (to be
  written with the tooling: starts `ebr-origin` and `ebr-netem-proxy`, reads bound addresses,
  runs `ebr-crossfile decode-bench --source http`, stops both, prints one JSON line).
- Validation look: validation counterpart by the same rules, W and S only, after the MDE gate.

## 6. Environment and platform requirements

- `wsl-ubuntu`: `st-v` {2} for single-thread latency; `mt-v-n` n in {2, 4, 8} for speedup
  (`VIRTUALIZED`, soft cap MEDIUM, §4.1); `max_loadavg_1m` = 1.0 + n per configuration;
  `max_other_cpu_percent` 3.0; `max_wait_s` 300; cache `hot`.
- Windows host (`st-p`, `mt-p-{2,4,7}`, `st-e`, `mt-e-{2,4,8,16}`) is the decision-grade venue for
  speedup within core classes once §14 item 3 exists; `spec-windows.yaml` is generated then. Until
  then Windows rows are informational.
- Emulated network: application-level proxy only (no `sch_netem`), labelled `EMULATED`; no slow
  start (R1-14): corroboration only, never a claim on its own.
- No privilege, no ARM64 (native ARM64 speedup is PLATFORM_BLOCKED and outside this experiment's
  scope; any scope including it stays `INSUFFICIENT_EVIDENCE`, §4.1).

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
PY=/root/eb-research/venv/bin/python
$PY -m ebr validate --spec research/experiments/EXP-CROSSFILE-008/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-CROSSFILE-008/spec.yaml --timing
$PY -m ebr verify-raw --spec research/experiments/EXP-CROSSFILE-008/spec.yaml
$PY -m ebr normalize  --spec research/experiments/EXP-CROSSFILE-008/spec.yaml
$PY -m ebr report     --spec research/experiments/EXP-CROSSFILE-008/spec.yaml
```

Speedup cells use `research/experiments/EXP-CROSSFILE-008/spec-speedup.yaml` (affinity {2..9},
`max_loadavg_1m` 9.0): each sample runs, in seeded order inside one process, a T(1) batch pinned
to vCPU {2} and a T(k) batch pinned to {2..k+1} (k in {2, 4, 8} as a candidate parameter), so the
T-14 pair is formed within the round and within the core class.

Per-sample command (archives are pre-built by an untimed setup pass,
`ebr-crossfile decode-bench --prepare`, into `/root/eb-research/scratch/EXP-CROSSFILE-008/`, and
their SHA-256 recorded; the timed command only reads):

```text
/root/eb-research/target/harness/release/ebr-crossfile decode-bench
  --item-id {item_id} --experiment-id EXP-CROSSFILE-008 --env-name wsl-ubuntu
  --archive /root/eb-research/scratch/EXP-CROSSFILE-008/{item_id}/{config}.eb
  --decoder {decoder} --threads {threads} --reads access-sample-v1,t2 --batch-min 1000
  --timers in-process --check-output-identity true
```

## 8. Seeds, warmups, repetitions

Seeds `order` 20260917081, `bootstrap` 20260917082, `command` 20260917083; batch order seeded per
round from the command seed. Warmups 2 (§4.5). Rounds: minimum 10, step 10, cap 30 (small) / 20
(medium) with the precision-only adaptive rule (§4.6). Speedup: rounds pair n = 1 and n = k within
the same round and core class (T-14 derived comparison).

## 9. Metrics

| Metric | OD | Role | ebr family | Threshold | Source |
|---|---|---|---|---|---|
| `first_entry_latency_s` (to first verified byte; unverified also reported, I25) | OD-07 | **primary** | time_wall | T-08 (in-process only) | `payload.first_entry_latency_s` |
| `random_entry_latency_s` p50 and p95 | OD-10 | secondary (see §3) | time_wall | T-08 | `payload.*` |
| `parallel_speedup` T(1)/T(n) within one core class | OD-13 | **primary** | other | T-14 | derived from `payload.decode_wall_s_n*` |
| `parallel_efficiency` | OD-13 | diagnostic | other | T-14 | derived |
| `decode_wall_s`, `decode_cpu_s` (full archive decode, in-process) | OD-07 | secondary | time_wall / time_cpu | T-04 / T-05 | `payload.*` |
| `decode_throughput_bps` | OD-07 | primary for DEC-CMP-014 arms only | throughput | T-05b | derived |
| `alloc_peak_bytes` (decode) | OD-08 | corroboration of EXP-CROSSFILE-006 | memory_peak | T-06 | `payload.*` |
| `remote_random_entry_latency_s` (emulated) | OD-12 | corroboration of the modelled value (§4.15) | time_wall | T-10 | remote script |
| output identity across n | HC-11 | binary | correctness | §3.3 | `payload.output_identity_ok` |

## 10. Normalization and statistical analysis

Timing protocol of decision-method §4: paired rounds, exact order-statistic item intervals, L2-L4
aggregation, Welch-Satterthwaite corpus intervals, confirmatory confidences (§4.12), family harm
guard, invalid-sample balance (§4.4). Speedup: the one permitted cross-configuration comparison,
within a core class only, with its own A/A calibration (`EXP-CAL-SPD`). Emulated remote: direction
agreement with EXP-CROSSFILE-004's modelled latency is required for any remote claim (§4.15); a
disagreement is recorded as counterevidence. Profile-scope (Appendix B.1): `decode_wall_s` is a
non-inferiority guard; both roles pre-declared.

## 11. Practical-significance thresholds

T-04, T-05, T-05b, T-06, T-08, T-10, T-14 by reference.

## 12. Sensitivity analysis

decision-method §6 where a selection uses these metrics; plus (a) p50 versus p95 read latency;
(b) access-sample reads versus T2 entries; (c) WSL `mt-v-n` versus Windows P-core and E-core
classes when available (environment arm: same-direction support required, §4.1); (d) session cache
cold (fresh session per read) versus warm.

## 13. Validation look and held-out placeholder

As §5; held-out placeholder `EXP-CROSSFILE-008-HO` at Commit A (§5.5), re-timed once post-unlock in
quiet windows.

## 14. Expected negative results worth recording

- Lookback latency penalties are invisible at p50 for typical cohorts but dominate p95.
- The DAG-parallel decoder barely scales on chained groups (speedup near 1 for grouped Chunks), so
  SPEC §13.2's parallel-decode claim holds only for lookback 0.
- Emulated remote latency disagrees with the model for small-range-heavy topologies because the
  emulator has no slow start (R1-14).

## 15. Threats to validity

- WSL2 virtualization (soft cap MEDIUM); hybrid P/E cores make Windows cross-class speedups
  meaningless (only within-class, §4.2).
- The DAG-parallel decoder is a research artifact; production has no parallel decoder, so OD-13
  measures the format's parallelism potential, not the shipped reader.
- Harness-built decoder codegen (R1-06): comparisons are among decoders in the same binary; no
  incumbent timing comparison is made here.
- Archives pre-built and hot in the page cache; latency excludes disk I/O by design (`hot`, §4.5).

## 16. Cost

About 120 h of exclusive wall time (single-thread latency about 60 h; speedup cells about 60 h at
up to 8 vCPUs), about 300 CPU-hours; disk: prepared archives about 10 GB, raw about 2 GB. Agent
effort: none for execution; judgment for analysis.
