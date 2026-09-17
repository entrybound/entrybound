# ebr-netem calibration

**Status: PROVISIONAL.** This machine was not confirmed quiet when these
numbers were captured (Defender real-time protection and Hyper-V/VBS stay
on throughout -- see `research/PROGRESS.md`'s execution-environment notes),
and no other Phase B2 agent activity was excluded. Treat every row below as
a smoke measurement confirming the emulator is roughly calibrated, not as
decision-grade timing. **Re-run this binary in a confirmed quiet window
before citing these numbers in an experiment analysis or a Decision Ledger
entry.**

Run id: `20260917T053802Z-dff529`. Raw JSONL: `/mnt/d/Projects/entrybound/entrybound/research/raw/EXP-NETEM-CAL/20260917T053802Z-dff529.jsonl`.

## RTT sweep

Bandwidth unmetered, jitter off, loss off, a 1-byte range per request, 15 repetitions per setting. Isolates per-request RTT
from bandwidth pacing (see `src/proxy.rs`'s doc comment for the one-way
delay split this measures the sum of).

| Configured RTT (ms) | Achieved mean (ms) | Achieved median (ms) | Achieved p90 (ms) | Error % (mean) | N |
|---:|---:|---:|---:|---:|---:|
| 1 | 14.397 | 3.709 | 4.868 | +1339.7% | 15 |
| 20 | 22.831 | 22.836 | 23.082 | +14.2% | 15 |
| 100 | 102.931 | 102.977 | 103.179 | +2.9% | 15 |
| 300 | 302.622 | 302.317 | 303.505 | +0.9% | 15 |

## Bandwidth sweep

RTT fixed at 1 ms, jitter off, loss off, `--burst-down-bytes 4096`, a range sized to take about 1.0s at the configured rate, 3 repetitions per setting.

The fixed token-bucket burst allowance is included in every transfer's byte count; at the lowest setting (1 Mbit/s) it is a non-negligible fraction of the sampled bytes and biases the achieved rate slightly high. This is a real property of the token-bucket model (an initial burst is *supposed* to be free), not a measurement bug, but it means the 1 Mbit/s row is the least representative of the sustained rate alone.

| Configured (Mbit/s) | Achieved mean (Mbit/s) | Achieved median (Mbit/s) | Achieved p90 (Mbit/s) | Error % (mean) | N |
|---:|---:|---:|---:|---:|---:|
| 1 | 1.01 | 1.00 | 1.03 | +0.9% | 3 |
| 10 | 9.97 | 9.97 | 9.98 | -0.3% | 3 |
| 100 | 61.70 | 61.96 | 62.29 | -38.3% | 3 |
| 1000 | 106.70 | 107.87 | 108.24 | -89.3% | 3 |

At high configured rates, expect the *achieved* rate to fall increasingly short of the configured one: [`TokenBucket::consume`](src/bandwidth.rs) is called once per `--chunk-size` (1s of bytes / many small chunks at a high rate), and each call's fixed overhead -- a mutex lock, an `Instant::now()`, and the async scheduling and one syscall-sized socket write around it -- does not shrink as the configured rate grows, while the *time budget* between chunks (what the token bucket would otherwise make the proxy sleep for) does. Once that overhead is comparable to or larger than the sleep it's supposed to replace, it dominates and the achieved rate plateaus below the configured one. If this run's 100/1000 Mbit/s rows show a large negative error, that is almost certainly this effect (confirm by re-running `ebr-netem-proxy` by hand with a much larger `--chunk-size` at the same rate and checking whether the gap shrinks), not a bug in the token bucket's math (`src/bandwidth.rs`'s own unit tests check that in isolation, without any real I/O in the loop).

## Threats to validity

See `README.md`'s "Threats to validity" section for the general application-level-emulation caveats (no real TCP congestion control, no real packet loss, HOL blocking differences). This calibration additionally: runs both legs (origin, proxy, and this measuring client) on one host over loopback, so it cannot detect emulator behavior that only manifests over a real NIC or a real multi-hop path; and shares that host's CPU with whatever else is running, which is exactly why every number above is provisional.
