# EXP-RECON-005: Encode, decode, verify and random-access cost of reconstruction (timing and process memory), detection cost of gates, subprocess-isolation overhead, research-build timing parity

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-RECON-005 |
| Domain | reconstruction (8.8); common provisions: `research/experiments/_index/reconstruction-common.md` |
| Status | **NEEDS_TOOLING**. Required: `ebr-recon pack-only` and `ebr-recon compare-tree` (research build), `ebr-access --mode read-archive`, the item-selection and adaptive-round scripts, and a research build with subprocess isolation. Timing gates: calibration `EXP-CAL-{AA,AAP,KE,BG}-wsl-ubuntu` must PASS for `time_wall`, `time_cpu` and `memory_peak` (§4.7). The Windows arm additionally needs the §14 item 3 runner additions. |
| Stage | Tuning timing (exploratory) for the candidates and guards of EXP-RECON-004, then **validation look 1** (same registered look as EXP-RECON-004). |
| decision_ids | DEC-CMP-018, DEC-CMP-026, DEC-CMP-023, DEC-CMP-012, DEC-CMP-037, DEC-CMP-042; informs DEC-CMP-044 (write-time verification CPU share) |
| Proposed types/ODs/HCs | common §2. OD-06 (`encode_cpu_s` primary, `encode_wall_s`), OD-07 (`decode_wall_s` primary, `verify_wall_s`), OD-10 (`random_entry_latency_s` secondary to `access_granularity_bytes`), OD-08 (process `peak_rss_bytes`, secondary; primary allocator memory in 006). HC gating of samples: round trip and archive identity (common §8.4). |
| Gates | G-A, G-B (runner additions §14 item 3; calibration §14 item 4), MDE gate. WSL timing carries `VIRTUALIZED` and a MEDIUM soft cap for Linux-native claims (§4.1). |
| Author and exposure | common §3 |
| Feasibility smoke | none |

## 2. Question

Under decision-grade timing controls, how do the EXP-RECON-004 candidates compare on:
- pack wall and CPU time (profile bands);
- unpack and verify wall time;
- random-entry latency for entries inside JPEG regions;
- process peak memory?

Four specific sub-questions:
- Does removing always-try raw DEFLATE attempts (gates) save pack CPU?
- What does subprocess isolation of reconstruction dependencies cost?
- What share of pack CPU is write-time reconstruction verification?
- Are research-build timings equivalent to the production `ebound` build (parity)?

## 3. Hypotheses

All hypotheses are `[INFERRED]`. Bands are referenced by id.

- **H1 (DEC-CMP-012).** `v6-signature-gated` is `EQUIVALENT` to `gen-v6` on `encode_cpu_s` at corpus level in every profile, because failed raw-DEFLATE parses terminate early.
  - **Falsified if** `DISTINGUISHABLE_BETTER` in any profile at the validation look. That result would establish a gating benefit.
- **H2 (guards for DEC-CMP-018/026/023).**
  - In the F12 family, `gen-v6` is `DISTINGUISHABLE_WORSE` than `v6-minus-jpeg` on `encode_cpu_s` beyond the T-05 profile band.
  - At corpus level, `gen-v6` is `NON_INFERIOR` to `gen-v4` under the profile's T-03/T-05 band.
  - **Falsified if** the corpus-level comparison is `DISTINGUISHABLE_WORSE`, or the F12 family is within band.
- **H3 (decode).** On F12, unpacking archives with JPEG regions is worse than `v6-minus-jpeg` on `decode_wall_s` by more than 1 T-04 band unit (JPEG XL reconstruction is slower than ordinary codec decode of the original bytes).
  - **Falsified if** within band.
- **H4 (access, DEC-CMP-026 multi-chunk).**
  - `random_entry_latency_s` (in-process, p50) for region-member entries of `dense-gen-v6` is worse than `dense-v6-minus-jpeg` by more than 1 T-08 band unit.
  - `balanced-jpeg-multichunk` is worse than `balanced-gen-v6` by more than 1 T-08 band unit on F12.
  - **Falsified if** either comparison is within band.
- **H5 (DEC-CMP-037).** `v6-subprocess-isolated` is `NON_INFERIOR` to `gen-v6` on `encode_wall_s` and `decode_wall_s` at corpus level, and worse beyond band only in families with many small reconstructive objects.
  - **Falsified if** the corpus level is `DISTINGUISHABLE_WORSE`.
- **H6 (parity, binary gate for research-build timings).** `pack-gen-v6-research` is `EQUIVALENT` to `pack-gen-v6-ebound` on `encode_wall_s` and `encode_cpu_s` at corpus level, and no family is `DISTINGUISHABLE`.
  - **Falsified** otherwise. Every research-build pack timing is then informational only, and OD-06 guards for non-status-quo candidates stay `INSUFFICIENT_EVIDENCE` (common §4 item 5).
- **Descriptive (DEC-CMP-044).** `verification_cpu_share` of reconstruction write-time verification in `gen-v6` dense/extreme on F12 is reported. There is no hypothesis test (descriptive role).

## 4. Decisions informed and how

| Decision | Contribution |
|---|---|
| DEC-CMP-018, DEC-CMP-026, DEC-CMP-023 | OD-06/OD-07 guards (B.1 constrained form: `encode_wall_s`, `decode_wall_s`) and primary metrics for non-profile decisions; OD-10 latency for regions |
| DEC-CMP-012 | The superiority metric for gates (`encode_cpu_s`), paired with 004's size non-inferiority |
| DEC-CMP-037 | Subprocess-isolation overhead: the ledger's "subprocess overhead on pack throughput" |
| DEC-CMP-042 | Pack CPU of `align-member-cdc` and `align-single-chunk-recognised` (non-inferiority guard) |
| DEC-CMP-044 (informs) | Verification CPU share of reconstructive transforms |

## 5. Candidates (runner cells)

Operations `op` ∈ {pack, unpack, verify, access}. Cells are `op-variant-profile` for profile ∈ {balanced, dense, extreme}.

| Variant | pack tool (timed) | unpack/verify tool (timed) | Notes |
|---|---|---|---|
| `gen-v6-ebound` | `ebound pack --profile P` (production build) | `ebound unpack` / `ebound verify` | status quo, decision-grade build |
| `gen-v6-research` | `ebr-recon pack-only --variant gen-v6` | — | parity arm (H6) |
| `gen-v4`, `gen-v5`, `v6-minus-deflate`, `v6-minus-jpeg`, `v6-signature-gated` | `ebr-recon pack-only --variant <v>` | `ebound unpack` / `ebound verify` on the pre-built archive of that variant | decoders are self-describing, so decode timing uses the production build for every variant |
| `v6-subprocess-isolated` | `ebr-recon pack-only --variant gen-v6 --isolate-reconstruction subprocess` | `ebr-recon unpack-isolated` (research reader with subprocess inverse) | DEC-CMP-037 candidate `subprocess-isolation` |
| `align-member-cdc`, `align-single-chunk-recognised` | `ebr-recon pack-only --variant <v>` | `ebound unpack` | DEC-CMP-042 |
| `balanced-jpeg-multichunk` | `ebr-recon pack-only` | `ebound unpack`; access | DEC-CMP-026 |
| `S_bytes` additions | as above | as above | Tuning survivors of EXP-RECON-004 (at most 4 per profile). Added by amendment, rule fixed here, before any timing result for them exists. |

- **Access cells.** `access-{gen-v6-ebound, v6-minus-jpeg}-{balanced,dense}` and `access-balanced-jpeg-multichunk` use `ebr-access --mode read-archive` (new mode): one process, 1,000 seeded reads per round of whole region-member entries (all entries if no regions), in-process timers, p50 and p95.
- **Pre-built archives.** Built untimed by `prepare.sh` with the same tools, and verified byte-identical to EXP-RECON-004 `artifact_sha256` for the same (item, cell). Stored under `/root/eb-research/cache/EXP-RECON-005/archives/`.

## 6. Corpus (item-selection rule fixed now; executed by `select_items.py`)

- **Target set T (all ops).** Every tuning item for which either:
  - (a) an EXP-RECON-001 `{balanced,dense,extreme}-indexed-plain` archive has ≥ 1 ReconstructionData object or JPEG region; or
  - (b) the EXP-RECON-002 census reports status-quo-reachable DEFLATE or JPEG bytes > 0 in any profile.
- **Non-target set N (pack ops only; detection cost, §7.3).** For every tuning independence group not represented in T, one item: the smallest-bytes item of that group among small and medium tiers, or else its smallest item.
- **Manifests.** `select_items.py` writes `items-target-tuning.jsonl` and `items-nontarget-tuning.jsonl`. Each row carries `item_id`, `split`, `family`, `materialized_relpath` and `logical_tree_sha256`. Both files are committed before the first timing run. `spec.yaml` (T × all cells) and `spec-nontarget.yaml` (N × pack cells) read them.
- **Validation look 1.** The same rule applied to validation items, using the validation outputs of EXP-RECON-001 and 002 produced inside the same look. Registered together with EXP-RECON-004 look 1.
- **Held-out.** Phase D placeholder (common §5).
- **Minimum support.** The rule keeps every tuning independence group represented (T ∪ N), so group counts for corpus-level verdicts equal the split's.

## 7. Environment and platform requirements

- **Primary.** `wsl-ubuntu` with the decision-grade WSL configuration `st-v` = vCPU {2} (§4.3). Reconstruction encoders are pinned to one thread (frozen v6), and the planner is single-threaded.
  - Cache state `hot` (§4.5). Inputs and scratch on ext4 under `/root/eb-research`.
  - Host-side sampler on Windows during every sample (§4.3; runner addition).
  - Exclusive timing window (§4.2 exclusivity).
- **Guard (spec).** `max_loadavg_1m` = 2.0 (1.0 + 1 thread), `max_other_cpu_percent` = 3.0, `sample_interval_s` 1, `max_wait_s` 600. Cooldown and drift canary per §4.2/§4.3 (runner additions).
- **Secondary arm (Windows host, NEEDS_TOOLING).** `st-p` = logical CPU {2}. A copy of the spec with Windows commands (`ebound.exe`, research build for Windows) runs only after §14 item 3 (AC power, power-throttling opt-out, PDH frequency covariate, per-core-class guard, MsMpEng covariates) and Windows calibration. It is required for any Windows-scoped timing claim.
  - Defender is on, so scan-on-close is included and `defender_on` labelled. External-comparison rules do not apply: all candidates here are Entrybound builds.
- **Not used.** Docker (unavailable) and QEMU (never for timing).

## 8. Commands and tooling to build

```sh
C:/Python313/python.exe research/orchestration/disk_guard.py
# tooling (to be written): research/experiments/EXP-RECON-005/prepare.sh -> builds ebound (production), ebr-recon (research),
#   ebr-access (read-archive mode); runs lock_parity + harness-cli-parity; pre-builds and hash-checks archives against EXP-RECON-004
python3 research/experiments/EXP-RECON-005/select_items.py --split tuning \
   --status-quo research/normalized/EXP-RECON-001 --census research/normalized/EXP-RECON-002 \
   --manifest research/corpus/manifest.json --out-dir research/experiments/EXP-RECON-005
# calibration prerequisite check (to be written): research/tools/recon/check_calibration.py --env wsl-ubuntu --families time_wall,time_cpu,memory_peak
# run inside a WSL shell (PowerShell may strip $-variables passed through wsl.exe):
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-RECON-005/spec.yaml --timing --cpus 2 --gzip
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-RECON-005/spec-nontarget.yaml --timing --cpus 2 --gzip
# adaptive rounds (§9): python3 research/experiments/EXP-RECON-005/adaptive_rounds.py --normalized research/normalized/EXP-RECON-005 \
#   --spec research/experiments/EXP-RECON-005/spec.yaml --out research/experiments/EXP-RECON-005/spec-step-<k>.yaml  (then ebr run --timing again)
# then verify-raw, normalize, report; decision tooling as EXP-RECON-004 §8.3
```

- **Timed regions.** The runner's process wall and CPU cover exactly the timed command. Correctness checks run as `check` metrics after the command and are not timed:
  - pack cells: `ebr-recon check-archive --archive {scratch}/a.eb --expect-sha256-table research/normalized/EXP-RECON-004/artifact-sha256.csv --item {item_id} --cell <variant>-<profile>`;
  - unpack cells: `ebr-recon compare-tree --source {item_path} --dest {scratch}/out`.

## 9. Seeds, warmups, replication and the adaptive rule

- **Seeds.** Order, bootstrap and command 20260923. Access read sequence seed = 20260923 + round.
- **Warmups.** 2 rounds for all tiers. This is stronger than the §4.5 large-tier minimum of 1, which is allowed as a strengthening. Warmups are recorded and excluded.
- **Rounds (§4.6).** Initial `repetitions: 10`. This meets n_min at 0.99 (8) and at 0.995 (9), the superiority confidence for k_sup = 2. For k_sup = 3 (0.9967) the minimum is 10.
- **Adaptive precision rule** (§4.6; `adaptive_rounds.py`, to be written):
  1. After each batch, for every primary comparison and item, compute the exact order-statistic interval of the median paired log ratio at the comparison's confidence.
  2. If its half-width exceeds 0.5 × `log1p(r)` for the metric's band, add one tier step of rounds (small +10, medium +10, large +5) for that item and **every** cell of the comparison.
  3. Stop at the tier cap (30/20/20). Items still above target are flagged `imprecise` (soft cap).
  4. The rule never looks at effect direction.
- **Operations per round (access cells).** 1,000 reads, which is at least the 40 required for p95 (§4.6).
- **Invalid-sample balance.** §4.4: > 10 % invalid in a cell means not decision-grade; an invalid-rate difference > 5 pp between compared cells excludes the item; > 20 % of items excluded means the comparison is not decision-grade.

## 10. Metrics

| Metric (canonical) | Realised as | Role | OD | Threshold |
|---|---|---|---|---|
| `encode_cpu_s` | `cpu_s` of pack cells | primary (OD-06) | OD-06 | T-05 (profile r), process floor |
| `encode_wall_s` | `wall_s` of pack cells | B.1 guard / secondary | OD-06 | T-03 (profile r), process floor; small tier: in-process timer primary (`payload.pack_seconds` of `ebr-recon pack-only`; for `ebound`, process samples are informational on the small tier) |
| `encode_throughput_bps` | derived `input_bytes / wall_s` | secondary | OD-06 | T-05b |
| `decode_wall_s` | `wall_s` of unpack cells | primary (OD-07) / B.1 guard | OD-07 | T-04 |
| `decode_cpu_s` | `cpu_s` of unpack cells | secondary | OD-07 | T-05 |
| `verify_wall_s` | `wall_s` of verify cells | secondary | OD-07 | T-04 |
| `random_entry_latency_s` | `payload.latency_p50_s` (and p95) of access cells, in-process | secondary for DEC-CMP-026 (OD-10 primary is `access_granularity_bytes` in 004) | OD-10 | T-08 |
| `peak_rss_bytes` | `max_rss_bytes` (GNU time rows with `measurement.method = gnu-time-v+wait4` only, R1-16) | secondary corroboration | OD-08 | T-06 |
| `verification_cpu_share` | `payload.verification_cpu_share` (research build phase timers) | descriptive | OD-06 | — |
| correctness checks | `archive_identity_ok`, `roundtrip_mismatches` | sample gating (common §8.4) | — | §3.3 |

## 11. Normalization and statistical analysis

1. `verify-raw`, then `normalize`. Apply the invalid-sample balance rules (§4.4). Samples failing correctness checks are excluded and reported as HC findings.
2. **Parity gate (H6).**
   - Corpus-level `EQUIVALENT` at 0.99 on `encode_wall_s` and `encode_cpu_s` per profile between `pack-gen-v6-research` and `pack-gen-v6-ebound`, and no family `DISTINGUISHABLE` at 0.90.
   - On failure, research-build pack timings are labelled `NOT_DECISION_GRADE` and the affected guards are unmet.
   - This gate is protocol calibration, not a decision comparison.
3. **Item level (§4.9 L1).** Median per-round paired log ratio (pairing within rounds, `item_repetition`), with floors applied per §3.1, including the small-tier in-process rule of T-03/T-04.
4. **Aggregation.** L2–L4 per §4.9 with the cited mix. Strata: tier; environment (WSL only unless the Windows arm runs).
5. **Tuning.**
   - Exploratory intervals for every candidate against its reference.
   - Provides the OD-06/07 guard and primary inputs to EXP-RECON-004's W/S formation.
   - Declares k_sup for gate pairs (DEC-CMP-012: `encode_cpu_s` superiority) and for `v6-subprocess-isolated` versus `gen-v6` (non-inferiority only; DEC-CMP-037 selection also uses 007 and 011).
6. **MDE gate (§4.12).**
   - Corpus-level MDE ≤ 1 band unit from tuning group variance and validation group structure.
   - The item-level MDE implied by the A/A half-widths at the cap must be ≤ 1 band unit.
   - A failure means no look for that comparison.
7. **Validation look 1** (same registered look as EXP-RECON-004), for W vs each s ∈ S:
   - confidences per common §9;
   - family harm guard;
   - `VIRTUALIZED` soft cap;
   - a `power_limit_confound` check does not apply in WSL; a host-side sampler anomaly invalidates samples.
8. **Joint use.** EXP-RECON-004's decision analysis consumes these verdicts as guards (B.1) or as OD-06/07 primary metrics. Pairing never crosses experiments. Only verdicts are combined.

## 12. Practical-significance thresholds

T-03, T-04, T-05, T-05b, T-06, T-08 (by id; profile-specific r for T-03/T-05), process and in-process floors per §3.2, and §3.3 for correctness gating. Not restated.

## 13. Sensitivity analysis

- common §10.
- Threshold perturbation ×0.5 applies only where the A/A median item half-width at the cap is ≤ 0.25 × `log1p(r)` (resolvability, §6.4).
- Environment arm: Windows versus WSL, where the Windows arm runs (same-direction support required for cross-environment claims, §4.1).
- Tier-stratified results are reported. Items flagged `imprecise` are analysed with and without them (CB-5).

## 14. Expected negative results worth recording

- Gating saves no measurable pack time (H1 holds). Gate candidates then have no superiority metric, and R10 prefers the status quo or the simpler candidate.
- JPEG regions make unpack and random entry reads of photo libraries slower beyond band (H3, H4). Recorded as a cost against JPEG reconstruction on read-many workloads (WM-MEDIA).
- Research-build parity fails (H6), leaving non-status-quo encode guards unmet. That is a program-level tooling gap, recorded.
- Subprocess isolation is costly on many-small-object families (H5).

## 15. Threats to validity

- **WSL2 virtualization.** vCPU placement is uncontrolled (P/E cores) and host caches are warm, so results carry `VIRTUALIZED` and a MEDIUM cap. Mitigation: `st-v` pinning, host-side sampler, drift canary, calibration.
- **Codegen differences between research and production builds** (R1-06). Mitigated by the parity gate (H6).
- **Pre-built archives decoded repeatedly from the page cache.** This is the declared hot cache state (§4.5); no cold-cache claim is made.
- **Scratch writes during unpack.** Hot and warm VHDX I/O noise is controlled by randomized blocks and invalid-sample balance.
- **Only single-thread configurations** are decision-grade here. Parallel speedup belongs to other domains (OD-13).
- All other threats: common §13.

## 16. Held-out placeholder

At Commit A, spec copies with the held-out manifests from `select_items.py --split heldout` are committed. That script reads only fingerprint-level fields after unlock: item ids and families from the frozen lock; census and status-quo outputs come from the held-out runs of EXP-RECON-001 and 002 after Commit C. They carry held-out MDEs from validation variance and held-out group counts, and run once after Commit C in an exclusive timing window (R5).

## 17. Cost and effort estimates

- **Compute.** About 260 machine-hours in exclusive timing windows (tuning plus validation): about 30 target items × about 70 cells × 12 rounds plus warmups, and about 60 non-target items × 21 pack cells × 12. Adaptive steps may add up to 50 %.
- **Disk.** Pre-built archives about 60 GB (cache); scratch peak ≤ 12 GiB; raw rows < 200 MB.
- **Tooling.** Runner additions (§14 item 3; program-wide), calibration (§14 item 4; program-wide), `ebr-recon pack-only`, `check-archive`, `compare-tree` and `unpack-isolated`, `ebr-access --mode read-archive`, `select_items.py`, `adaptive_rounds.py`.
- **Agent effort.** Execution: none (scheduled overnight windows). Analysis: judgment.

## 18. Looks, deviations and outcome (append-only)

- Look 1: registered jointly with EXP-RECON-004 look 1.
- Deviations: none.
- Outcome: `outcome.json`.
