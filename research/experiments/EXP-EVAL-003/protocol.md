# EXP-EVAL-003: Measurement-instrument validation (R1-16 RSS accounting, Windows commit, write accounting, runner additions, harness versus CLI timing parity)

| Field | Value |
|---|---|
| Domain | evaluation (program sections 6, 7, 34, 35) |
| Status | NEEDS_TOOLING (runner fix for harness review R1-16; the runner additions of `decision-method.md` §14 item 3; known-answer helper programs; one timing arm after EXP-EVAL-004 calibration) |
| Kind | Protocol-validity experiment: known-answer tests of the instruments every decision-grade memory, scratch and timing result depends on (gate G-B for timing and memory). It selects no product candidate. The instrument arms are informational method evidence (§4.7 analogue); the P arm is evidence for DEC-MOD-045. |
| Method | `research/decision-method.md` at `14b977c` (§4.2-§4.4, §4.13, §4.14, §14 item 3); `research/harness/harness-review-round1.md` use constraints 1-3 |

## Author, independence and exposure

Designed by the Phase C-design evaluation-domain session. No corpus family is analysed for a decision here (known-answer inputs are generated), so the recorded L7 exposure (EXP-EVAL-001 protocol) has no consequence for this experiment except arm P, whose analysis runs in an unexposed session.

## Question

Do the runner and harness instruments report what their names say, within the absolute floors of the thresholds they feed? The instruments are: Linux `max_rss_bytes`, Windows peak commit, write accounting, scratch polling, multi-process peak and the §14 item 3 runner controls. Separately, is harness in-process timing of production code interchangeable with the production `ebound` binary (harness review R1-06)?

## Hypotheses and falsification

- **H-RSS (R1-16).** After the runner fix, `resource.max_rss_bytes` equals the child's own `VmHWM` within 4 MiB (T-06 process floor) for every requested allocation K in {16, 64, 256, 1024} MiB, and does so regardless of runner ballast B in {0, 512} MiB, when GNU time is present. When GNU time is disabled, `max_rss_bytes` is null and the row carries an error. Falsified by any |difference| > 4 MiB, or any non-null fallback value.
  - **Negative control.** The pre-fix runner at commit `3c084ae` with GNU time disabled and B = 512 MiB is expected to report at least 512 MiB for K = 16 (reproducing R1-16). If the control does *not* reproduce it, the probe lacks power, and the positive result is not accepted until the probe is fixed.
- **H-COMMIT (Windows).** The job peak commit (`peak_job_commit_bytes`) equals the child's own `PeakPagefileUsage` within 4 MiB for every K. The sampled peak working set is at most the true peak (lower bound), and for a child that allocates and frees within 50 ms it undercounts. That undercount is the expected negative control, and it shows why sampled working set is informational only (§4.14). Falsified by a commit difference > 4 MiB.
- **H-WRITE (§4.14 write accounting).** For a child that writes exactly N bytes (N in {0, 1, 4096, 1,048,583}) to the scratch directory, calls fsync and deletes the file, Linux `/proc/<pid>/io` `write_bytes` summed over the process tree equals N exactly (tolerance 0, `thresholds.json` `protocol.scratch_write_tolerance_bytes`). On Windows, the job I/O write transfer count on a dedicated scratch volume equals N exactly.
  - Falsified by any nonzero difference. On falsification, write accounting cannot decide binary zero-scratch or streaming claims on that OS until fixed.
  - **Negative control.** Polled `scratch_peak_bytes` reports 0 for the 50 ms write-and-delete child, showing that polling cannot decide those claims (T-07).
- **H-TREE.** For a child that runs two grandchildren sequentially with K1 = 64 MiB and K2 = 256 MiB, `max_rss_bytes` equals max(VmHWM) over the processes within 4 MiB, not their sum. `tree_peak_rss_sum_bytes` is informational. Falsified by a value within 4 MiB of the sum.
- **H-CTRL (runner additions, §14 item 3).** Each control in the table below behaves as specified on its injected trigger. Falsified by any control that does not fire on its trigger, or fires without one, in 20 trials per control.
- **H-P (harness versus CLI timing parity; DEC-MOD-045).** On `wsl-ubuntu`, `st-v`, `encode_wall_s` of `ebr-pack` (harness build, `research-internals` on) and `ebound pack` (production build, feature off) is `EQUIVALENT` at corpus level (two-sided 0.99) at the `balanced` band of T-03, over the arm P item set. The same holds for `decode_wall_s` of `ebr-unpack` versus `ebound unpack` at T-04.
  - H0: not equivalent.
  - If H-P is not supported, harness timers never stand in for production timing in any claim or decision (use constraint 2 stays binding), and DEC-MOD-045 records that experiments must time whole-archive production binaries.

## Decisions informed

- `DEC-ECO-082`: the "peak-memory method cross-check (Windows job object, GNU time, psutil)" and "reconciliation of reported sizes with container section accounting" required evidence, plus instrument validity for every protocol candidate.
- `DEC-MOD-045`: whether a research-internals feature can serve measurements, or experiments must use whole-archive production binaries.

## Candidates

- **Instrument arms (known-answer).** The status quo runner (ebr at `3c084ae`) versus the fixed runner (after the R1-16 fix and §14 item 3 additions). Spec candidates are the known allocations and write sizes; the runner version is a run-level factor recorded in raw `meta`.
- **Arm P.** `harness-ebr-pack-balanced-indexed` (`/root/eb-research/target/harness/release/ebr-pack --profile balanced --layout indexed`) versus `refprod-ebound-balanced-indexed` (the EXP-EVAL-002 REF-PROD binary). Decode: `harness-ebr-unpack` versus `refprod-ebound-unpack`. Reference: the REF-PROD binary.
- **Arm P-access.** EXP-EVAL-005 O5 needs this arm, because no process-level production equivalent of an in-process random-read batch exists. It compares `ebr-access` INDEXED random-entry batches (1,000 entries per round) built two ways: `access-internals-on`, the harness workspace build, and `access-internals-off`, the same crate built through a research-only Cargo profile that depends on `entrybound` without `research-internals` (`research/harness/build-feature-off.sh`, to be written; it touches no production file). The metric is `random_entry_latency_s` at T-08, tested for `EQUIVALENT`. If equivalence is not shown, EXP-EVAL-005 uses only the feature-off build.
- **Excluded.** Hardware performance counters are not validated here: WSL2 exposes no PMU to the guest, and Windows ETW PMC sampling needs admin (a `PLATFORM_BLOCKED` note for DEC-ECO-082's `hardware-counter-metrics` candidate).

## Runner-control acceptance table (H-CTRL)

| Control (§14 item 3) | Injected trigger | Pass criterion |
|---|---|---|
| AC power check | Simulated `GetSystemPowerStatus` AC line = 0 (test double) | every sample of the run invalid with reason `on_battery` |
| Windows power-throttling opt-out | none (state probe) | `GetProcessInformation(ProcessPowerThrottling)` on the measured child shows the execution-speed control bit set and the state bit cleared |
| SMT sibling verification | none; then a fabricated topology with a mismatched pair | real topology: pairs (0,1)..(14,15) accepted; fabricated: run refused |
| Cooldown | none | gap before each sample equals min(60, max(2, previous wall)) s ±0.5 s |
| Drift canary | busy-loop injector at +5% on the pinned set after block 10 | pause 120 s, repeat, abort; samples since the last good canary marked `drift_canary` |
| PDH frequency covariate | none; then a counter read failure (test double) | covariate stored per sample; on failure, multi-threaded and >20 s Windows samples labelled not decision-grade |
| Throttling-onset probe | none | `t_on` or "none within 300 s" recorded in run meta |
| Per-core-class guard accounting | 2.0% plus 0.5% busy loop on the SMT sibling of the pinned CPU; 3.5% system-wide on E-cores | sample start refused until load removed; `max_wait_s` abort if not removed |
| MsMpEng and SearchIndexer covariates | none | CPU seconds per sample stored |
| Host-side sampler for WSL runs | busy loop on the Windows host during a WSL sample | host other-process CPU above §4.4 limits marks the sample invalid |
| Invalid-rate balance | synthetic invalid-sample injection: 12% on one cell; 8% versus 1% on a paired item | cell not decision-grade; item excluded from the comparison; >20% items excluded makes the comparison not decision-grade |
| `vm-cold` cache mode | read of a 1 GiB file after `drop_caches` | major faults and `io_read_bytes` at least 0.9 GiB on the cold sample versus under 1% on the hot sample |
| Environment-block padding (A/A') | seeded padding size 0-4096 | the child sees the recorded padding length; distribution uniform (chi-square p > 0.01 over 1,000 draws) |
| In-process and batch timing hooks | harness busy-loop op of exactly 10^7 iterations, batch of 1,000 | batch time / N within 2% of single-op median; hook schema fields present |
| Clean-worktree and pre-registration ancestry | dirty tracked file; spec committed after run HEAD | both runs refused |
| Spec lint | `thresholds_override` in a decision spec; guard looser than file; `max_loadavg_1m` not 1.0 + threads; sensitivity-sample override | each refused with the named rule |
| Size reconciliation (§4.13) | archive with tampered section table (test double) | named buckets not summing to `artifact_bytes` invalidates the sample; `cross_check_indexed` mismatch fails |

## Corpus selectors

- **Instrument arms.** Generated inputs only (`corpus.generated`, `zeros-v1` with 0 bytes as a dummy item; the helpers generate their own allocation and write patterns). No corpus family is involved.
- **Arm P.** Tuning split only: `f01-tuning-zstd-v1-5-7-git`, `f02-tuning-lz4-intree-make-build`, `f04-tuning-sourcelike`, `f05-tuning-gutenberg-books`, `f07-tuning-silesia-xray`, `f08-tuning-silesia-osdb`, `f09-alpine-324-rootfs-binaries`, `f12-tuning-nasa-ivl-photos-small`, `f13-tuning-nasa-ntrs-pdf-small`, `f18-tuning-lz4-releases-1-9-to-1-10` (small tier), plus `f01-tuning-curl-8-19-0-git`, `f05-tuning-loghub-openstack`, `f06-tuning-gharchive-2015-01-01-h15-h16`, `f09-silesia-mozilla-ooffice`, `f17-tuning-duptree-mixed`, `f18-tuning-zstd-releases-1-5-x` (medium tier). That is 16 items across 11 families. Parity is a property of builds, not of content, so no validation look is needed.

## Environment

`wsl-ubuntu` (instrument arms Linux; arm P) and `windows-host` (H-COMMIT, Windows H-WRITE, Windows controls). Arm P runs only after EXP-EVAL-004 calibration has passed for `wsl-ubuntu` `time_wall` in `st-v`, in an exclusive quiet window.

## Commands

```sh
# known-answer helpers (TO BE WRITTEN):
#   research/experiments/EXP-EVAL-003/tools/alloc_child.py   --mib K [--touch] [--grandchildren 64,256] [--free-after-ms 50]
#        prints {"self_vmhwm_bytes":..,"self_peak_pagefile_bytes":..,"requested_bytes":..,"children_vmhwm_bytes":[..]}
#   research/experiments/EXP-EVAL-003/tools/write_child.py   --bytes N --dir {scratch} [--delete-after-ms 50]
#        prints {"written_bytes":N}
#   research/tools/ebr selftest hooks (TO BE WRITTEN in execute.py/run.py, active only with EBR_SELFTEST=1):
#        EBR_SELFTEST_RUNNER_BALLAST_MIB, EBR_DISABLE_GNU_TIME; resource metric peak_commit_bytes (Windows)
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-EVAL-003/spec.yaml'
# four runs of the RSS spec (runner factor grid), fixed runner:
#   EBR_SELFTEST=1 EBR_SELFTEST_RUNNER_BALLAST_MIB={0,512} [EBR_DISABLE_GNU_TIME=1] python -m ebr run --spec research/experiments/EXP-EVAL-003/spec.yaml --run-id rss-b{B}-t{0,1}
# negative control: same grid with the runner checked out at 3c084ae in /root/eb-research/src/ebr-prefix (git worktree), run-id prefix rss-prefix-
# Windows: pwsh -File research/experiments/EXP-EVAL-003/run_windows.ps1   (TO BE WRITTEN; runs spec-windows.yaml, generated from spec.yaml with os: windows)
# controls: research/tools/tests/test_runner_controls.py (TO BE WRITTEN; pytest; 20 trials per control; JSON report to research/raw/EXP-EVAL-003/controls/)
# arm P (timing; after EXP-EVAL-004 passes): research/experiments/EXP-EVAL-003/spec-parity.yaml (TO BE WRITTEN from this protocol), ebr run --timing --cpus 2
# analysis: research/experiments/EXP-EVAL-003/analyze.py (TO BE WRITTEN): per-arm pass/fail table + decision tooling equivalence test for arm P
```

## Seeds

- `seeds: {order: 20260920, bootstrap: 20260920, command: 20260920}`.
- Control trials: seed 20260921.
- A/A' padding: seed 20260922.

## Warmup

- Instrument arms: 1 warmup per candidate (interpreter start-up paging), recorded and excluded.
- Arm P: §4.5 (2 small, 2 medium).

## Measurement method and metrics

| Measured quantity | Canonical metric | Role here | Oracle |
|---|---|---|---|
| runner `max_rss_bytes` | `peak_rss_bytes` (T-06) | instrument under test | child `VmHWM` |
| runner `peak_commit_bytes` (Windows) | `peak_commit_bytes` (T-06) | instrument under test | child `PeakPagefileUsage` |
| harness scoped memory | `alloc_peak_bytes` (T-06) | instrument under test (arm P rows) | counting allocator (§14 item 17), where available |
| `io_write_bytes` | `scratch_write_bytes` (binary, §3.3) | instrument under test | `written_bytes` |
| `scratch_peak_bytes` | `scratch_peak_bytes` (T-07) | negative control | `written_bytes` |
| arm P `encode_wall_s`, `decode_wall_s` | T-03, T-04 | primary for H-P equivalence | none (paired comparison) |

The pass criteria use each metric's registered absolute floor (T-06: 4 MiB for process RSS and commit; `scratch_write_tolerance_bytes` 0), not a new tolerance.

## Replication and adaptive rule

- **Instrument arms.** 20 samples per (K or N, runner factor cell): a deterministic oracle comparison with a stated detection bound. 20 trials detect a per-sample failure probability of at least 0.1391 with 95% confidence (Appendix D.5).
- **Controls.** 20 trials each.
- **Arm P.** §4.6 timing rounds (small 10/10/30; medium 10/10/20) with the precision-based adaptive rule.

## Normalization

- **Instrument arms.** The per-sample difference, measured minus oracle, is the unit. There is no aggregation beyond the max |difference| per cell.
- **Arm P.** Levels L1-L4 with **equal family weights**, because this is a build-property equivalence test and not a workload claim; the workload mix is not used.

## Statistical analysis

- **Instrument arms.** Exact pass/fail on every sample (worst case, AGG-W). No intervals.
- **Arm P.** Exact order-statistic item intervals (§4.10), corpus-level Welch-Satterthwaite interval at two-sided 0.99, and `EQUIVALENT` per §4.11. The MDE is computed from the calibration A/A half-widths (§4.12); if the item-level MDE exceeds 1 band unit, arm P is `INCONCLUSIVE`, and use constraint 2 stays binding.

## Practical-significance thresholds

- T-06 floors: 4 MiB for RSS and commit, 1 MiB for the allocator.
- T-07 and §4.14: write tolerance 0.
- T-03: `balanced` r = 0.05, process floor 5 ms.
- T-04: r = 0.05.

All as in `thresholds.json` `metric_thresholds`.

## Sensitivity analysis

- **Instrument arms.** K and N sweep over four magnitudes; runner ballast 0 and 512 MiB. The pass must hold in every cell.
- **Arm P.** Per tier (small and medium), and leave-one-family-out.

## Expected negative results worth recording

- The R1-16 reproduction on the pre-fix runner (the expected positive control of the defect).
- Windows sampled working set underestimates short-lived peaks.
- Polled scratch misses short writes.
- Arm P non-equivalence, if `research-internals` inlining changes codegen enough to exceed the 5% band.

## Threats to validity

- Python interpreter paging adds a few MiB of noise. It is absorbed by the 4 MiB floor, and the oracle is the child's own `VmHWM`, not the requested K.
- `/proc/<pid>/io` `write_bytes` counts page-cache writeback attribution. The helper uses `fsync`, and short files are tested explicitly.
- Windows job accounting includes loader commit. The oracle is the child's own peak pagefile usage, which includes it too.
- Arm P covers `balanced` INDEXED only. Other profiles are assumed to behave alike, and the protocol states that assumption as a limitation.

## Platform requirements

Linux x86-64 (WSL2 root: `drop_caches`, `/proc/<pid>/io`); Windows 11 host non-admin (job objects, PDH); arm P needs a quiet exclusive window. No network.

## Estimates

- Machine time: instrument arms about 3 h (both OSes); controls about 2 h; arm P about 8 h (16 items x 2 candidates x up to 30 rounds, with cooldown). Total about 13 h.
- Disk: under 10 GiB.
- Agent effort: scripted (tests and runs); judgment only for the arm P analysis.
