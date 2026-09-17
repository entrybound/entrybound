# EXP-SCALE-016: Parallel and sequential extraction and verification on slow and network storage

| Field | Value |
|---|---|
| Status | **BLOCKED** |
| Blocked reason | This host has only its local NVMe-class SSD (NTFS; the WSL ext4 VHDX sits on it). There is no rotational HDD and no SATA SSD attached; creating an SMB share requires Windows administrator rights, which this non-admin session lacks; there is no second host to serve NFS or SMB over a real network; `sch_netem` is unavailable in WSL2, so link shaping of a filesystem mount cannot be emulated (PROGRESS.md known blockers). A loopback NFS or SMB server on the same SSD would not exhibit network or seek latency and is therefore not accepted as a substitute. |
| Domain | scale (program §14) |
| Platforms | Linux x86-64 and Windows 11 with: a rotational HDD (7,200 rpm class), a SATA SSD, an SMB 3 share and an NFSv4 export on a separate host over 1 Gbit/s Ethernet, and a bandwidth-limited link (for example 100 Mbit/s) to the same share |
| Requires timing | true |
| Compute estimate | about 25 machine-hours once the storage exists |
| Disk estimate | about 100 GB on each storage class |
| Agent effort | scripted (reuses EXP-SCALE-013 specs with a storage-class dimension) |
| Tooling to build (when unblocked) | `research/tools/scale/gen_affinity_specs.py --storage-root <mount>` to place inputs and outputs on each storage class; storage fingerprinting (`smartctl -i`, `lsblk -d -o ROTA`, `Get-PhysicalDisk MediaType`, share protocol and dialect) into `research/environment`; incumbent parallel extractors pinned (`ripunzip` release binary, `pigz`, `pixz`, `zstd -T`) |

## Pre-registration

- **decision_ids:** DEC-ACC-033, DEC-ACC-034, DEC-ACC-002.
- **Decision type:** EMPIRICAL.
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-13, OD-07; HC-11, HC-13 floors.
- **Method, gate, author, L7:** as EXP-SCALE-001.

## Question

On HDD, SATA SSD, SMB and NFS (and a bandwidth-limited share), how do sequential and identity-passing parallel extraction and verification designs perform as worker counts grow, where does parallel I/O become a slowdown, and would a storage-probing adaptive worker policy or separate CPU and I/O pools avoid it (the ripunzip-style negative result the ledger cites)?

## Hypotheses

- **H1:** parallel per-entry extraction (`parallel-entry-extraction`) on HDD is `DISTINGUISHABLE_WORSE` than sequential extraction for many-small-file archives (F04, F03) beyond T-04's band, while on NVMe (EXP-SCALE-013) it is better; separate CPU and I/O pools with an I/O pool of 1-2 on HDD are `NON_INFERIOR` to the best fixed setting on every storage class.
- **H0 / falsification:** confirmatory verdicts per §4.11 (validation), per storage class (an evaluation condition, never averaged).

## Decisions informed

DEC-ACC-033 (extraction on HDD/network storage vs NVMe), DEC-ACC-034 (speedup and slowdown curves on SATA SSD, HDD, SMB/NFS and bandwidth-limited links; adaptive and separate-pool candidates), DEC-ACC-002 (parallel-decode marks qualified by storage class). Until unblocked, these decisions' scopes exclude non-local storage classes explicitly, or they stay `INSUFFICIENT_EVIDENCE`.

## Candidates

`status-quo-serial`, `fixed-default-workers` (physical cores), `user-flag-only` sweeps {1, 2, 4, 8, 16}, `io-probe-adaptive` (prototype: probe write latency, choose I/O workers), `separate-cpu-io-pools` (CPU pool = cores, I/O pool ∈ {1, 2, 4}); incumbents `ripunzip`, `unzip`, `zstd -dc | tar -x`, `pigz -d -p n`.

## Corpus selectors

Tuning then validation medium items of F03, F04, F05, F13 (few large files vs many small files); held-out placeholder as EXP-SCALE-013 per storage class (§5.5).

## Environment

§4.2/§4.3 timing controls plus storage covariates (device queue depth, SMB dialect, NFS mount options, link bandwidth measured with `iperf3` before each session); cache state `vm-cold` where available on Linux (no cold-cache claim, §4.5) and `hot`.

## Commands

As EXP-SCALE-013 with `--storage-root` per class; spec template `research/experiments/EXP-SCALE-016/spec.yaml` (Linux, storage root parameterized as a candidate parameter) validated at design time.

## Seeds

`seeds.order` 1316001, `seeds.bootstrap` 1316002, `seeds.command` 1316003.

## Warmup

§4.5.

## Measurement method and metrics

`decode_wall_s`, `stream_unpack_wall_s`, `verify_wall_s` (T-04), `parallel_speedup` (T-14, within core class, per storage class), `decode_cpu_s` (T-05), `fs_write_ops` (T-19, secondary), identity gating.

## Replication

§4.6 with the precision rule; storage classes are evaluation conditions.

## Normalization and statistical analysis

As EXP-SCALE-013, per storage class; R9 partition by storage class only if the product can express it (policy expressibility, R9 (i)).

## Practical-significance thresholds

Referenced: T-04, T-05, T-14, T-19; §7.3.

## Sensitivity analysis

§6 as EXP-SCALE-013; cache-state arm; link-bandwidth arm.

## Expected negative results worth recording

Parallel extraction slower than sequential on HDD and SMB; adaptive probing mis-classifying storage; no single default safe across classes.

## Threats to validity

Specific device models and share servers; network variability (covariates recorded); cold-cache limits on Windows (admin required).

## Platform requirements

As in the header; Windows administrator rights (or a pre-created share) for the SMB class; a second host.

## Estimated compute and effort

About 25 h once available; scripted.

## Deviations (append-only)

None.
