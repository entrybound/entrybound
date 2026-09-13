# ebr: experiment runner and statistics

`ebr` runs every Entrybound research experiment: it executes a spec, appends
hash-chained raw rows, verifies them, normalizes them into CSV and summary JSON,
and generates Markdown tables and PNG figures. Every generated file records the
command that produced it, so no aggregate number is ever typed by hand.

## Setup and tests (WSL)

```sh
wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/tools/setup_venv.sh   # idempotent, --require-hashes
wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/tools/run_tests.sh
pwsh -File research/tools/run_tests_windows.ps1   # Windows job-object/psutil measurement path
```

`requirements.txt` pins exact versions and SHA-256 hashes, including transitive
dependencies. `requirements-downloads.json` records each wheel's URL, size, hash
and retrieval date. `lock_requirements.sh --relock` re-resolves the pins, which
changes them, so run it deliberately.

## Pipeline

```sh
export PYTHONPATH=/mnt/d/Projects/entrybound/entrybound/research/tools
cd /mnt/d/Projects/entrybound/entrybound
PY=/root/eb-research/venv/bin/python
$PY -m ebr validate   --spec research/experiments/<EXP>/spec.yaml
$PY -m ebr plan       --spec ...          # seeded run order, nothing executed
$PY -m ebr run        --spec ... [--gzip] [--cpus 2,3] [--timing --max-load X --max-other-cpu Y]
$PY -m ebr verify-raw --spec ... [--refingerprint]
$PY -m ebr normalize  --spec ...
$PY -m ebr report     --spec ...
```

Exit codes: 0 ok, 1 failed samples or verification, 2 spec error, 3 refused
(held-out lock, quiet-machine guard, timing flags, platform).

| Output | Location |
| --- | --- |
| Raw rows (one per sample, hash-chained) and run meta | `research/raw/<EXP>/<run_id>.jsonl[.gz]`, `<run_id>.meta.json` |
| Normalized tables and `summary.json` | `research/normalized/<EXP>/` |
| Markdown tables | `research/reports/tables/<EXP>/` |
| Figures (PNG, plus the CSV that was plotted) | `research/reports/figures/<EXP>/` |
| Generated items, scratch space, fingerprint cache | `/root/eb-research/{generated,scratch/ebr,cache/ebr}` (outside git) |

For a complete example, see `research/experiments/EXP-SMOKE-000/` (`spec.yaml`, `run_smoke.sh`).

## Rules the tool enforces

* **Held-out data.** `split: heldout` is refused unless `EB_HELDOUT_UNLOCK`
  equals the SHA in `research/decisions/design-freeze.json`. Locked runs never
  list the held-out directory. They also reject items that resolve inside it
  and templates that hard-code its path. `EB_HELDOUT_UNLOCK` is stripped from
  child processes.
* **Thresholds.** Thresholds are read from `research/methods/thresholds.json`, a
  transcription of `research/decision-method.md`. That file is authoritative and
  every value starts as `null`. A verdict that needs an unset value is reported
  as `threshold_unset:<key>`. Only specs with no `decision_ids` may carry
  `thresholds_override`, and their outputs are labelled as spec overrides.
* **Timing.** A `timing: true` spec runs only with `--timing`. The quiet-machine
  guard checks the 1-minute load average and other-process CPU before the first
  sample and between samples. It refuses to start if either limit is unset or
  exceeded, and it marks a sample invalid if the guard fails afterwards.
  Timing-family numbers from runs without `--timing` (`time_wall`, `time_cpu`,
  `throughput`) are labelled informational and never get a verdict. In WSL the
  guard sees only the Linux VM.
* **Run order.** Every round visits each (candidate, item) pair once, and warmup
  rounds come first. Items are shuffled per round and candidates are shuffled
  within each item block, so paired runs stay close in time. The shuffle is
  seeded by `seeds.order`.
* **Measurement.** On Linux, `[taskset] /usr/bin/time -v` wraps each command:
  `wait4` supplies user/sys time at microsecond resolution, the `-v` report
  supplies max RSS, faults, context switches and FS in/out, and
  `perf_counter_ns` measures wall time. On Windows the command runs inside a Job
  Object, which accounts CPU and I/O; peak working set is sampled with psutil;
  CPU affinity is set through psutil. On both, a poller tracks the scratch
  directory's peak size and enforces the timeout.
* **Statistics** (`ebr.stats`). Percentiles are type 7 and MAD is unscaled.
  Bootstrap CIs are percentile intervals seeded with PCG64 and use at least
  10000 resamples. Paired ratios resample whole pairs. The noise rule calls a
  difference distinguishable only when the CI lies entirely outside the band.
  Also provided: epsilon-Pareto dominance and frontier, Kendall tau-b, and
  weight sensitivity (a ±25/±50 % grid plus Dirichlet samples).

Every number in `summary.json`, the CSVs, the tables and the figures traces back
to a verified raw file whose SHA-256 is listed in the output header.
