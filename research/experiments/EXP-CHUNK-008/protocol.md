# EXP-CHUNK-008: Chunking throughput, pack CPU and pack memory (timing campaign)

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CHUNK-008 |
| Status | NEEDS_TOOLING. It needs: `ebr-chunk throughput` (in-process batch timer, thread CPU time); `gear-norm-v1-copy` (a verbatim harness copy of the production chunker, as a codegen control); a soft-AES build variant; `ebr-cdcpack` in-process phase timers and scoped memory; the counting allocator (§14 item 17). **Decision-grade timing additionally requires:** the runner additions of `decision-method.md` §14 item 3 (Windows power-throttling opt-out, PDH frequency covariate, drift canary on the affinity set, per-core-class guard, host-side sampler for WSL, invalid-rate balance, in-process/batch hooks) and calibration per §4.7 (`EXP-CAL-AA/AAP/KE/BG-<env>` for `time_wall`, `time_cpu`, `throughput`, `memory_peak`). |
| Arms | **T1** chunk-stage throughput (`spec.yaml`, WSL `st-v`; generated `spec-t1-windows-st-p.yaml` and `spec-t1-windows-st-e.yaml`). **T2** end-to-end pack CPU, wall and memory for the W/S configurations of EXP-CHUNK-004/005 (`spec-t2-*.yaml`, generated after those tunings). **P** harness/production timing parity (`spec-parity.yaml`). **V1** validation timing for confirmatory comparisons. |
| decision_ids | DEC-CMP-001 (OD-06, OD-08), DEC-CMP-002 (profile guards `encode_wall_s`, `alloc_peak_bytes`), DEC-CMP-003 and DEC-CMP-004 (selection and categoriser work, OD-06), DEC-CRY-023 (throughput per keyed mode, with and without AES-NI) |
| Proposed od_ids | as in EXP-CHUNK-004/005/006 |
| Proposed hc_ids | HC-13 (boundary identity across repetitions and builds), HC-06 (declared memory) |
| Gates / author / L7 | As EXP-CHUNK-001 section 1. Timing windows are exclusive (§4.2): no agents, builds, containers or provisioning. |

## 2. Question

- What single-thread in-process chunking throughput do the CDC constructions and keyed boundary modes achieve? It is measured on P-cores, E-cores and WSL vCPUs.
- What complete pack CPU, wall time and peak memory do the chunking size policies, selection rules and categorisers cost end to end?

## 3. Hypotheses

- **H-T1a.** Against `gear-norm-v1` on chunk-stage throughput (`chunk_stage_throughput_bps`, secondary) at corpus level:
  - `fastcdc-2020-nc2` and `gear-spread-nc2` are `DISTINGUISHABLE_BETTER` (spread masks allow skipping, 2-byte stepping);
  - Rabin, TTTD and PHTE are `DISTINGUISHABLE_WORSE`;
  - `phte-aes128-aesni` is worse than `secret-gear-table` by at least 1 band unit, and `phte-aes128-softaes` is worse than `phte-aes128-aesni` by at least 1 band unit.

  **Falsified** for each pair where the corpus verdict disagrees.
- **H-T1b (codegen control).** `gear-norm-v1-copy` is `EQUIVALENT` to `gear-norm-v1` (production code through the harness build). **If not equivalent**, harness timing of the production chunker is inlining-dependent (review R1-06), and every T1 comparison with `gear-norm-v1` is reported against both arms, with the less favourable one to the alternative governing.
- **H-T2.** For `balanced` packs, construction differences in `encode_cpu_s` are within the T-05 band at corpus level: chunking is a minor share of pack CPU. For `extreme`, end-to-end selection (EXP-CHUNK-005 `sel-end-to-end`) is `DISTINGUISHABLE_WORSE` than `sel-proxy-frozen` by at least 1 band unit. **Falsified** if the verdicts differ.
- **H-P (parity).** Harness `ebr-pack` and production `ebound pack` (production workspace build, no research-internals) are `EQUIVALENT` on `encode_wall_s` and `encode_cpu_s` for the four frozen profiles. **If not**, status-quo timing in T2 uses production `ebound`, and harness-built alternatives are flagged `mixed_build` (soft cap; the comparison is `INSUFFICIENT_EVIDENCE` if the parity gap exceeds 1 band unit).
- **H0.** No throughput or CPU difference exceeds its band.

## 4. Decisions informed

| Decision | Arm | Contribution |
|---|---|---|
| DEC-CMP-001 | T1, T2, P | "Single-core throughput and end-to-end pack time per algorithm on the reference i9-14900HX (P-core and E-core)" (ledger); OD-06 primary `encode_cpu_s`, OD-08 primary `peak_rss_bytes`/`alloc_peak_bytes` (pack) |
| DEC-CMP-002 | T2 | Profile-scope non-inferiority guards `encode_wall_s` and `alloc_peak_bytes` (Appendix B.1) for W_P versus S_P per profile |
| DEC-CMP-003, DEC-CMP-004 | T2 | Selection work: "Pack CPU of evaluating 1-4 candidates (full chunking plus SHA-256 per candidate) and of end-to-end evaluation"; categoriser CPU |
| DEC-CRY-023 | T1 | "Chunking throughput (MiB/s) per mode on x86-64 with/without AES-NI" (ledger). ARM64 is EXP-CHUNK-013 (BLOCKED). |

## 5. Candidates

### Arm T1 (`spec.yaml`, balanced sizes 128 KiB / 512 KiB / 2 MiB)

| Candidate | Definition | Role |
|---|---|---|
| `gear-norm-v1` | Production chunker through the harness | **Status quo; reference** |
| `gear-norm-v1-copy` | Verbatim copy of `chunk_ranges_with_table` in the harness crate | Codegen control (not a decision candidate) |
| `gear-spread-nc2`, `fastcdc-2016-nc2`, `fastcdc-2020-nc2`, `rabin-w64`, `buzhash-w64`, `ae-w1`, `ram`, `tttd`, `seqcdc`, `vram-scalar`, `chonkers` | As EXP-CHUNK-002 section 5 | Ledger alternatives. The survivor subset is chosen after EXP-CHUNK-002/003; this spec deliberately times all, because timing does not feed the dedup screens. |
| `fixed-size` | Baseline | Excluded from selection (SPEC §7.2 L792) |
| `secret-gear-table`, `phte-aes128-aesni` | Production keyed modes via `chunk_ranges_encrypted` | DEC-CRY-023 status quo default and opt-in |
| `phte-aes128-softaes` | Same, built with the `aes` crate forced to its software backend (`RUSTFLAGS="--cfg aes_force_soft"`, per the pinned `aes` crate documentation, verified by the builder) | DEC-CRY-023 "without AES-NI" arm on x86-64 |
| `keyed-gear-spread-nc2` | As EXP-CHUNK-006 | DEC-CMP-001 keyed successor |
| `sha256-chunk-digests-only` | SHA-256 over `gear-norm-v1` chunks only (no boundary search timed) | Descriptive hashing share (M06.3-like), not a candidate |

- **`quickcdc-feature-acceptance`:** excluded (R1).
- **`vectorcdc-simd`:** the accelerated path cannot be built under `unsafe_code = "forbid"` on stable Rust, so its throughput is **not measured here**. It is recorded as an evidence gap (EXP-CHUNK-013). `vram-scalar` timing says nothing about SIMD throughput.

### Arm T2 (generated after EXP-CHUNK-004 and EXP-CHUNK-005 tuning)

W and every s ∈ S for:

- DEC-CMP-001 (universal, at `balanced` and `extreme`);
- DEC-CMP-002 (per profile, sizes);
- DEC-CMP-003 and DEC-CMP-004 (selection and granularity policies).

These are run through `ebr-cdcpack` with in-process timers, plus production `ebound pack` for the status quo.

### Arm P

`ebound pack` (production build `/root/eb-research/target/exp-chunk-prod`) versus `ebr-pack` (harness) × 4 profiles, INDEXED.

## 6. Corpus

- **T1/T2 tuning (40 items):** two tuning items per family from distinct independence groups, scale small or medium, ≥ 1 MiB. Selection is mechanical (first two by item id with distinct groups); ids in `spec.yaml`. This gives 20 families, 40 groups and 4.86 GiB, which satisfies §4.9 minimum support at tuning. Large-tier timing (warmups 1, cap 20) is `spec-t2-large.yaml` for DEC-CMP-002 tier guards: 1 large tuning item per family where one exists.
- **V1 (validation timing):** after the MDE gate, W versus S on validation, two items per family from distinct groups by the same rule (all 20 families have ≥ 2 validation groups), registered as the look of the owning decision. Timing and byte evidence of one decision share one look.
- **Held-out:** Phase D placeholder (§5.5; EXP-CHUNK-004 section 6).

## 7. Environment and platform requirements

| env_id | Affinity (§4.2/§4.3) | Status | Qualifiers |
|---|---|---|---|
| `wsl-ubuntu` | `st-v` {2}; `taskset` via `platform.cpu_affinity: [2]` | Decision-grade only after WSL calibration passes (§4.7) and the host-side sampler exists | `VIRTUALIZED`: soft cap MEDIUM for Linux-native claims |
| `windows-host` | `st-p` {2} (sibling 3 idle), `st-e` {18} | Not decision-grade until §14 item 3 runner additions exist and Windows calibration passes | `defender_on`; frequency covariate; throttling onset probe |

- **Cache:** `hot` (preloaded into memory before the in-process timer; `--preload true`).
- **Power:** AC required (checked per run). Agents never change power or security settings.
- ARM64 native and macOS: out of scope here (EXP-CHUNK-013).
- **Build identity:** harness binaries in `/root/eb-research/target/exp-chunk` (and `…-softaes`), production in `/root/eb-research/target/exp-chunk-prod`. `lock_parity.py` and `harness-cli-parity.sh` must PASS.

## 8. Commands

```sh
# Builds (WSL; Windows equivalents via build.ps1 with D:/eb-research/target/exp-chunk-win)
CARGO_TARGET_DIR=/root/eb-research/target/exp-chunk bash research/harness/build.sh build --release -p ebr-chunk -p ebr-pack
CARGO_TARGET_DIR=/root/eb-research/target/exp-chunk-softaes RUSTFLAGS="--cfg aes_force_soft" \
    bash research/harness/build.sh build --release -p ebr-chunk
CARGO_TARGET_DIR=/root/eb-research/target/exp-chunk-prod /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli
# Calibration must have passed for this env and families (EXP-CAL-*-wsl-ubuntu), else stop.
export PYTHONPATH=research/tools; PY=/root/eb-research/venv/bin/python
$PY -m ebr validate --spec research/experiments/EXP-CHUNK-008/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-CHUNK-008/spec.yaml --timing --gzip      # exclusive timing window
$PY research/experiments/EXP-CHUNK-008/precision_step.py --spec research/experiments/EXP-CHUNK-008/spec.yaml \
    --run-id <first> --out research/experiments/EXP-CHUNK-008/spec-step-<k>.yaml          # direction-blind (section 11)
$PY -m ebr run --spec research/experiments/EXP-CHUNK-008/spec-step-<k>.yaml --timing --gzip
$PY -m ebr verify-raw --spec research/experiments/EXP-CHUNK-008/spec.yaml
$PY -m ebr normalize --spec research/experiments/EXP-CHUNK-008/spec.yaml --run-id <first> --run-id <step runs...>
$PY research/experiments/EXP-CHUNK-008/make_specs.py --arms t1-windows-st-p,t1-windows-st-e,parity,t2,t2-large \
    --w-s research/normalized/EXP-CHUNK-004/decision/w-s.json research/normalized/EXP-CHUNK-005/decision/w-s.json
$PY -m decide analyze --experiment EXP-CHUNK-008 --out research/normalized/EXP-CHUNK-008/decision/
```

## 9. Seeds, warmups, cache

- **Spec seeds:** `{order: 2026091781, bootstrap: 2026091782, command: 2026091783}`. Keyed modes use key seed 20260917 (fixed across rounds so work is identical).
- **Warmups:** 2 for small and medium (`spec.yaml`), 1 for large (`spec-t2-large.yaml`), recorded (`record_warmups: true`). Warmup adequacy is checked in calibration (Kendall τ, §4.5).
- **Cooldown and canary:** per §4.2/§4.3 (runner additions).

## 10. Metrics

| Metric | Canonical | OD | Role | Measurement source | Threshold |
|---|---|---|---|---|---|
| `encode_cpu_s` (T2, whole pack, in-process thread CPU) | yes | OD-06 | **primary** (DEC-CMP-001, 003, 004) | in-process | T-05 (profile-specific r) |
| `encode_wall_s` (T2) | yes | OD-06 | **guard** for DEC-CMP-002 (profile scope); secondary otherwise | in-process (small tier primary per T-03); process samples informational | T-03 |
| `encode_throughput_bps` (T2) | yes | OD-06 | secondary (reciprocal view) | in-process | T-05b |
| `peak_rss_bytes` (T2, scoped `pack_memory`, use constraint 3) | yes | OD-08 | **primary** for OD-08 on Linux until the counting allocator exists | scoped kernel high-water reset | T-06 |
| `alloc_peak_bytes` (T2, counting allocator) | yes | OD-08 | **guard** for DEC-CMP-002; replaces `peak_rss_bytes` as primary once built | in-process exact | T-06 |
| `peak_commit_bytes` (Windows T2) | yes | OD-08 | Windows primary (§4.14) | job accounting | T-06 |
| `chunk_stage_throughput_bps`, `chunk_stage_seconds_per_pass`, `chunk_stage_cpu_seconds_per_pass` (T1) | no (component timings) | OD-06 | secondary (they eliminate nothing; they explain T2) | in-process batch ≥ 1 s | none |
| `boundary_set_sha256` | no | — | per-round identity check (HC-13); a mismatch across rounds is a determinism violation artifact | — | §3.3 |
| `wall_s`, `cpu_s`, `max_rss_bytes` (runner resource) | yes (families) | — | informational (R1-16: runner RSS includes the Python process when GNU time is missing) | process | — |

## 11. Replication and adaptive rule

- **Minimum rounds:** small 10 / step 10 / cap 30; medium 10 / 10 / 20; large 10 / 5 / 20 (§4.6). The minimum is also ≥ n_min(c) for the comparison's confidence (§4.6, Appendix D.4).
- **Adaptive rule (§4.6, direction-blind):** after the minimum, `precision_step.py` computes, per item and primary comparison, the exact order-statistic interval of the median paired log ratio. Where the half-width exceeds 0.5 × log1p(r), it writes a step spec for those items covering every candidate. Stop at the target or the cap; `imprecise` items are flagged.
- **Pairing:** `item_repetition` within rounds; `randomized_blocks` order.
- **Invalid-sample balance:** §4.4.

## 12. Statistical analysis

- **L1:** median of per-round paired log ratios; exact order-statistic intervals (§4.10); in-process floors (T-03/T-05 in-process values) at item level.
- **L2-L4:** as EXP-CHUNK-004 section 12. Equal-family-weight arm until `workload-mix.json` exists.
- **Environment rule (§4.1):** a claim spanning Windows and Linux needs same-direction decision-grade support in both; `INCONCLUSIVE` supports nothing. P-core and E-core results are reported separately and never pooled; cross-class speedups are informational.
- **Confirmatory:** T2 comparisons are part of the owning decisions' confirmatory sets (EXP-CHUNK-004/005): superiority at 1 − 0.01/k_sup, non-inferiority 0.99, family harm guard 0.90, MDE ≤ 1 band unit, including the A/A item half-width condition (§4.12).
- **T1:** secondary, `UNADJUSTED`, 0.99, used to explain T2 and to populate the DEC-CRY-023 throughput table.
- **Parity (P):** `EQUIVALENT` required as in H-P.
- **Covariates reported with every verdict:** MsMpEng and SearchIndexer CPU (Windows), steal time and host sampler (WSL), frequency covariate and `power_limit_confound` flags.

## 13. Thresholds

T-03, T-05, T-05b, T-06 as transcribed in `research/methods/thresholds.json` (`encode_wall_s`, `encode_cpu_s`, `encode_throughput_bps`, `peak_rss_bytes`, `alloc_peak_bytes`, `peak_commit_bytes`), including profile-specific relative bands and in-process floors. Guard limits from `quiet_machine_guard` are tightened in the spec only (`max_loadavg_1m` = 1.0 + 1 thread). Not restated.

## 14. Sensitivity

- §6.4 threshold perturbation, subject to the resolvability condition from A/A half-widths.
- §6.5 family arms.
- Environment arm (Windows vs WSL).
- Core-class arm (`st-p` vs `st-e`).
- Batch-length arm (min batch 1 s vs 5 s on 10 items).
- Codegen arm (`gear-norm-v1` vs `-copy`).
- Build arm (harness vs production for the status quo).

## 15. Expected negative results

- Chunker throughput differences vanish in end-to-end pack CPU for `balanced` and heavier profiles.
- PHTE's AES cost is a small share of `dense`/`extreme` packs, weakening the throughput argument against a PHTE default.
- E-core results reverse P-core rankings for table-lookup-heavy algorithms.
- WSL calibration fails for some families (timing not decision-grade there).
- Harness/production parity fails (R1-06 materialized).

## 16. Threats to validity

- Hybrid-core scheduling, Defender scanning, VBS: controlled per §4.2 and reported as covariates.
- WSL virtualization: `VIRTUALIZED` qualifier and cap.
- Harness codegen versus the shipped build (R1-06): arms P and codegen control.
- Soft-AES build flag correctness: the builder verifies the backend with the `aes` crate's documented detection (or a symbol check) and records it.
- In-process preload excludes I/O, which is intended for the chunk-stage component; T2 includes file reading on hot cache.
- Runner RSS contamination (R1-16): `max_rss_bytes` is informational only.
- Thermal and power limits on a laptop: throttling-onset probe; `power_limit_confound` flag.

## 17. Estimates

| Arm | Estimate |
|---|---|
| T1 per environment | 19 candidates × 40 items × about 12-32 rounds, plus cooldown (§4.2: at least 2 s per sample) ≈ 15,000-25,000 samples: about **25-40 wall-hours per environment**, × 3 environments (WSL `st-v`, Windows `st-p`, `st-e`) |
| T2 | about 15-25 configurations × 40 items × 12-22 rounds, packs of 1-60 s plus cooldown: about **60-120 wall-hours per environment** (Windows `st-p` and WSL; E-core T2 only for DEC-CMP-001 W vs S) |
| P | about 15 wall-hours |
| V1 | about 40 wall-hours |

- **Disk:** < 10 GB.
- **Agent effort:** none during execution (scheduled exclusive windows, unattended); judgment for `precision_step.py`, the soft-AES build verification and analysis.

## 18. Tooling to build

1. **`ebr-chunk throughput` subcommand:** preload files; batch passes until `--min-batch-seconds`; report seconds and thread CPU per pass (`getrusage(RUSAGE_THREAD)` on Linux, `GetThreadTimes` on Windows via std-compatible safe API or an existing safe crate; `unsafe_code = "forbid"`); boundary digest per round; CPU flags.
2. **`gear-norm-v1-copy`** algorithm module (verbatim copy with a unit test proving boundary identity with production).
3. **Soft-AES build profile** and its verification note in `research/harness/README.md`.
4. **`ebr-cdcpack`:** in-process phase timers (chunk, hash, select, plan, encode), thread CPU, scoped memory; counting allocator (§14 item 17).
5. **`research/experiments/EXP-CHUNK-008/{precision_step.py, make_specs.py}`.**
6. **Program dependencies:** §14 item 3 runner additions; §4.7 calibration experiments per env and family.

## 19. Deviations

(append-only)
