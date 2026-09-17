# EXP-INT-006: Verified extraction staging: disk and memory cost and time to first verified entry

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (extraction prototypes `ebr-int-extract` with in-process first-entry timer; crafted STREAM authority-contradiction archives via T12) plus G-B timing gates (runner additions, `EXP-CAL-*` for `wsl-ubuntu`) |
| Domain | integrity (program §24, cross-reference scale §13; objective OD-08, OD-09, HC-15) |
| Decisions informed | DEC-INT-039 |
| Gates, method, author | conventions C0 |
| Spec | `spec.yaml` (timing arm); the deterministic security arm is `spec-security.yaml` |

## 1. Question

For STREAM and crypto-v1 INDEXED extraction, how much staging disk and memory does each
candidate materialization policy need as archive size grows, how long does it take until the
first entry is materialized in a state the policy claims verified, and can a per-entry
streaming policy materialize entries that full-archive verification would later reject?

## 2. Hypotheses and falsification

| ID | Hypothesis | Falsified by |
|---|---|---|
| H1 | Status quo stage-then-materialize has `scratch_peak_bytes` growing linearly with logical size (slope near 1 above the in-memory threshold) for STREAM and `peak_rss_bytes` growing with size for encrypted unpack (encrypted unpack is in memory, per the ledger); `per-entry-verified-streaming` has scratch bounded by the largest entry. | Status quo scratch not distinguishably larger than per-entry streaming (T-07 band) on the 1 GiB and larger rungs. |
| H2 | `first_entry_latency_s` of per-entry streaming is at least 1 band unit of T-08 smaller than stage-then-materialize on every rung at or above 256 MiB. | Not `DISTINGUISHABLE_BETTER`. |
| H3 | Per-entry streaming materializes at least one entry that full verification rejects on crafted STREAM archives whose later authority records contradict earlier entries (duplicate LogicalPath emitted later, conflicting kind, FIDELITY mismatch, footer body-digest mismatch); `temp-tree-then-atomic-move` materializes none. | Zero such entries under per-entry streaming. |

## 3. Candidates

`status-quo-stage-then-materialize` (B-PROD `ebound unpack`), `per-entry-verified-streaming`,
`temp-tree-then-atomic-move`, `streaming-with-unverified-marking` (marking carrier per
EXP-INT-011's survivor; default: marker manifest file), `unverified-unmarked-streaming`
(proposed L0 exclusion, conventions C0 and EXP-INT-005 section 3; negative control in the
security arm only; no timing claims are made for it). The three alternatives are research
prototypes in `ebr-int-extract` built from B-RES; a timing parity check between B-PROD
`ebound unpack` and the prototype's `stage-then-materialize` mode must be `EQUIVALENT` on
`stream_unpack_wall_s` before prototype-versus-B-PROD comparisons are made (otherwise only
prototype-versus-prototype).

## 4. Corpus

- Size ladder (tuning): ebr generated items `text-v1` single files of 16 MiB, 256 MiB,
  1 GiB, 4 GiB and 16 GiB (seed 2026091751; SHA-256 recorded in `spec.yaml` after first
  generation, before any timing run), plus many-entry items `f18-tuning-curl-8-x-release-series`
  (115,028 files), `f05-tuning-loghub-hdfs-v1` (large logs), `f01-tuning-llvm-project-20-1-8-git`.
- Validation look: `f18-validation-cpython-3-12-point-releases`,
  `f06-validation-noaa-ghcn-daily-csv-2023`, `f04-validation-real-gentoo-repo-20260915`, and a
  second generated ladder with seed 2026091752.
- Security arm: crafted archives from INT-SMALL-T and INT-SMALL-V (section 6).
- The size ladder is not a corpus family; generated items are reported as a separate scaling
  stratum and never enter family-weighted L4 aggregates.

## 5. Environment and platform requirements

`wsl-ubuntu`, ext4 scratch under `/root/eb-research/scratch` on a dedicated per-sample
directory (write accounting §4.14), affinity `st-v`, `timing: true`, exclusive quiet window,
guard `max_loadavg_1m` 2.0, `max_other_cpu_percent` 3.0, `max_wait_s` 300; cache `hot`.
Windows arm (`windows-host`, D: only) generated from this spec when §14 item 3 exists. Atomic
tree moves on APFS are EXP-INT-019 (BLOCKED).

## 6. Procedure and commands

1. Pre-pack each ladder item and corpus item once per layout (`indexed-crypto` with a test
   recipient, `stream`) into `/root/eb-research/int-archives/EXP-INT-006/`.
2. Timing arm: `python -m ebr run --spec research/experiments/EXP-INT-006/spec.yaml --timing`
   per adaptive step, then `verify-raw`, `normalize`, `report`.
3. Security arm: `ebr run` of `spec-security.yaml` (to be generated from this protocol when the
   crafting tool exists; not timing): T12 crafts STREAM archives where item records after the
   first materialized entry contradict it; each policy extracts; the destination is compared
   with the full-verification verdict.

## 7. Seeds, warmups, replication

Seeds 2026091753/54/55. Warmups 2 (16 MiB, 256 MiB), 1 (1 GiB and above). Rounds per §4.6
(minimum 10, adaptive steps and caps by tier); the 16 GiB rung is large tier (step 5, cap 20).
Deterministic security arm: 2 repetitions with repeat-hash.

## 8. Metrics

| Metric | ID | OD | Role |
|---|---|---|---|
| `scratch_peak_bytes` | T-07 | OD-08 | **primary** (polled lower bound, T-07) |
| `scratch_write_bytes` | T-07 binary | OD-08 | binary for any zero-scratch claim (write accounting) |
| `peak_rss_bytes` | T-06 | OD-08 | secondary (second OD-08 primary needs justification) |
| `first_entry_latency_s` | T-08 | OD-07 | **primary**; in-process timer only (§3.2 T-08) |
| `stream_unpack_wall_s` | T-04 | OD-09 | **primary** |
| `unsafe_defaults` | HC-05 | floor | binary (security arm: entries materialized as final and later rejected, unmarked) |
| materialized-then-rejected entries | none | - | descriptive count per policy (security arm) |

## 9. Normalization

Per-round paired log ratios against the status quo within rounds; floors per T-06, T-07,
T-08, T-04; scaling slopes (ln metric on ln size) reported descriptively per policy.

## 10. Statistical analysis and sensitivity

C10 timing plan. Size rung is a stratum; the corpus items give the family-level verdicts
(three items do not meet §4.9 minimum support, so banded corpus-level verdicts from this
experiment are secondary; the scale domain's streaming experiments on INT-CORE-sized sets are
cited as confirmatory sources when they cover the same policies; otherwise the decision records
the gap). Sensitivity: layout arm (STREAM versus crypto INDEXED, never pooled), entry-count arm
(many small entries versus few large), cache arm (`vm-cold` on the 1 GiB rung).

## 11. Expected negative results worth recording

- Stage-then-materialize needs scratch comparable to the archive's logical size.
- Per-entry streaming leaks entries that later fail archive-level verification unless marked.
- Temp-tree atomic move doubles transient disk use on the same volume.

## 12. Threats to validity

- Scratch polling is a lower bound (T-07); write accounting is exact but covers only zero-scratch claims.
- Generated text ladders are compressible and single-file; the corpus items cover entry structure.
- WSL timing is `VIRTUALIZED`; a Windows arm is needed for any cross-platform claim.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12 (ladder regenerated with a held-out seed committed only as its SHA-256 at the freeze, §5.4 sealing route).

## 14. Cost estimates

- Timing: 4 policies x 8 inputs x 10-20 rounds; unpack of 16 GiB about 60-120 s plus cooldown:
  about 18 hours of exclusive wall time (tuning), about 10 hours (validation look).
- Disk: 16 GiB input archive, up to 16 GiB staging, 16 GiB output: about 60 GB peak.
- Security arm: about 2 CPU-hours.
- Agent effort: tooling scripted with review; execution none; analysis judgment.

## 15. Deviations (append-only)

None.
