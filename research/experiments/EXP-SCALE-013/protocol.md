# EXP-SCALE-013: Parallel speedup, efficiency and saturation within core classes (timing campaign)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** |
| Domain | scale (program §14; parallel marks of §35) |
| Platforms | Windows 11 host (decision-grade: pinned P-core and E-core configurations, AC power, NTFS local SSD); WSL2 (secondary, `VIRTUALIZED`) |
| Requires timing | true |
| Compute estimate | about 60 machine-hours of exclusive quiet windows (plus calibration experiments, counted in `EXP-CAL-*`) |
| Disk estimate | about 20 GB |
| Agent effort | scripted execution; judgment for the analysis |
| Tooling to build | (1) runner additions of `decision-method.md` §14 item 3 (AC-power check, power-throttling opt-out, SMT sibling verification, cooldown, drift canary on the affinity set, PDH frequency covariate and throttling-onset probe, per-core-class guard accounting, MsMpEng/SearchIndexer covariates, host-side sampler for WSL, invalid-rate balance, spec lint) and **per-candidate CPU affinity** in ebr (today `platform.cpu_affinity` is spec-level; until added, `research/tools/scale/gen_affinity_specs.py` generates one spec per affinity configuration, all committed before the campaign); (2) calibration experiments `EXP-CAL-AA-windows-host`, `EXP-CAL-AAP-windows-host`, `EXP-CAL-KE-windows-host`, `EXP-CAL-BG-windows-host`, `EXP-CAL-SPD-windows-host` and the WSL equivalents passing for time_wall, time_cpu and the T-14 speedup (§4.7); (3) EXP-SCALE-012 prototypes that passed byte identity, built for Windows and WSL from the production-workspace-equivalent dependency lock (`lock_parity.py` PASS) |

## Pre-registration

- **decision_ids:** DEC-ACC-030, DEC-ACC-031, DEC-ACC-032, DEC-ACC-033, DEC-ACC-034, DEC-ACC-007, DEC-ACC-002.
- **Decision type:** EMPIRICAL.
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-13 (M13.1 `parallel_speedup` primary, M13.2 `parallel_efficiency` diagnostic), OD-06/OD-07 (T-03/T-04/T-05 for absolute cost and CPU-energy proxy); HC-11, HC-13 as floors (only identity-passing prototypes enter).
- **Method, gate, author, L7:** as EXP-SCALE-001.

## Question

For pack, verify and unpack with each identity-preserving parallel design, what speedup S(n) = T(1)/T(n) and efficiency are achieved within the P-core class (n ∈ {2, 4, 7}; SMT variant 14 vs 7) and the E-core class (n ∈ {2, 4, 8, 16}), where does speedup saturate per operation, family and lookback depth, how do queue depth and worker count trade throughput against memory, what CPU time does parallelism add per wall second saved, and does parallel decode justify a capability mark against incumbents?

## Hypotheses

- **H1:** `parallel-chunk-hashing` plus `parallel-final-encode-only` reach S(7) on P-cores beyond the T-14 band's upper edge relative to 1 (that is, distinguishably above 1.10 with the 0.2 floor) for balanced and dense pack on medium and large items of F01, F05, F07; `parallel-independent-chunks` verify reaches S(7) distinguishable from 1 on lookback-0 archives and not on dense archives with long ChunkGroup lookback.
- **H0:** S(n) is `EQUIVALENT` to 1 (no practical speedup) for a design and operation.
- **Falsification:** confirmatory verdicts on validation per §4.11-§4.12 against the T-14 band.

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-ACC-030 | Planner wall time vs workers per profile (regret unchanged is EXP-SCALE-012's identity result) | — |
| DEC-ACC-031 | Speedup curves vs worker count on P and E cores for CDC and hashing per corpus family | identity: EXP-SCALE-012 |
| DEC-ACC-032 | Throughput vs queue depth and worker count | memory bound: EXP-SCALE-012 |
| DEC-ACC-033 | Decode/verify speedup vs workers and lookback depth; memory overhead of queues (allocator peak recorded beside timing) | HDD/network storage: EXP-SCALE-016 (blocked) |
| DEC-ACC-034 | Speedup and slowdown curves on local NVMe-class SSD; CPU-time vs wall trade-off on P/E cores | SATA SSD, HDD, SMB/NFS, bandwidth-limited links: EXP-SCALE-016 (blocked) |
| DEC-ACC-007 | Throughput of each concurrency-model shell for parallel verify | dependency cost and determinism: EXP-SCALE-012 |
| DEC-ACC-002 | Parallel-decode capability mark: measured speedups vs `zstd -T0`/`pixz` decode baselines at matched thread counts | other marks: evaluation domain |

## Candidates

Identity-passing EXP-SCALE-012 designs (the list is fixed when EXP-SCALE-012's identity table is committed), the sequential status quo (T(1) reference and the n = 1 arm of every design), and incumbent references `inc-zstd-T<n>` (`zstd -T<n> -19` and `-d`), `inc-xz-T<n>` (`xz -T<n> -6 --block-size=16MiB`), `inc-pixz-p<n>`, `inc-pigz-p<n>`, run under the same affinity configurations.

Affinity configurations (`decision-method.md` §4.2, §4.3, Appendix C.1): Windows `st-p` {2}; `mt-p-2`, `mt-p-4`, `mt-p-7`; `mt-psmt-14` {2..15}; `st-e` {18}; `mt-e-2`, `mt-e-4`, `mt-e-8`, `mt-e-16`; WSL `st-v` {2}, `mt-v-2`, `mt-v-4`, `mt-v-8`. Worker count equals the logical CPUs of the configuration (§4.2). Queue-depth arm: reorder capacity {1, 2, 4, 8} × workers on `mt-p-7`.

**Excluded:** `os-scheduled` configurations (informational only, §4.2); cross-core-class speedups (informational, T-14).

## Corpus selectors

Manifest `research/corpus/manifest.json`. Tuning (exploratory, names W and S per decision): medium and large items of all families with ≥ 2 independence groups (§4.9 minimum support), plus dense-profile archives of F17 and F18 for lookback depth. Validation (one registered look after the §4.12 MDE gate): the same families' validation items. **Held-out placeholder (Phase D):** the frozen provisional selections re-timed once on held-out medium and large items after unlock under the same affinity configurations (§5.5), R5 (b)-(f).

## Environment

Windows host per §4.2 in full (AC power, throttling opt-out, affinity with SMT verification, cooldown, drift canary, frequency covariate and throttling onset, Defender covariates and the owner-excluded directory arm where available, exclusivity). WSL per §4.3 with the host-side sampler; `VIRTUALIZED` soft cap. Guard: `max_other_cpu_percent` 3.0, pinned-set and sibling 2.0, `max_loadavg_1m` = 1.0 + n (WSL), `max_wait_s` 300. Cache state `hot`. Inputs and outputs on local NTFS outside the repository (Windows) and ext4 under `/root/eb-research` (WSL).

## Commands

```powershell
C:/Python313/python.exe research/tools/scale/gen_affinity_specs.py --template research/experiments/EXP-SCALE-013/spec.yaml --out research/experiments/EXP-SCALE-013/affinity/
# commit the generated specs, then in an exclusive window per configuration:
$env:PYTHONPATH='research/tools'; C:/Python313/python.exe -m ebr run --timing --spec research/experiments/EXP-SCALE-013/affinity/mt-p-7.yaml
C:/Python313/python.exe -m ebr verify-raw --spec research/experiments/EXP-SCALE-013/affinity/mt-p-7.yaml
C:/Python313/python.exe -m ebr normalize --spec research/experiments/EXP-SCALE-013/affinity/mt-p-7.yaml
# decision tooling (decision-method §14 item 2) computes T-14 speedups within core classes from paired configurations
```

## Seeds

`seeds.order` 1313001, `seeds.bootstrap` 1313002, `seeds.command` 1313003; one order seed per affinity spec derived as 1313001 + index.

## Warmup

§4.5: 2 rounds for small and medium, 1 for large; warmup adequacy checked by Kendall τ.

## Measurement method and metrics

| Metric | Canonical name | Role | Family |
|---|---|---|---|
| `parallel_speedup` | T-14 (M13.1) | primary | other |
| `parallel_efficiency` | T-14 (M13.2) | diagnostic | other |
| `encode_wall_s`, `decode_wall_s`, `verify_wall_s` | T-03, T-04 | primary for absolute cost; inputs to T-14 | time_wall |
| `encode_cpu_s`, `decode_cpu_s` | T-05 | primary for the CPU-energy trade-off | time_cpu |
| `encode_throughput_bps`, `decode_throughput_bps`, `verify_throughput_bps` | T-05b | secondary | throughput |
| `alloc_peak_bytes` / `peak_commit_bytes` | T-06 | secondary (queue overhead) | memory_peak |
| identity digests | repeat-hash (§4.13) | gating: any mismatch invalidates the sample and is reported to EXP-SCALE-012 | correctness |

## Replication and adaptive rule

§4.6: small 10/10/30, medium 10/10/20, large 10/5/20, with the precision-based adaptive rule (half-width ≤ 0.5 × log1p(r) for T-14's r = 0.10) and n ≥ n_min(c) for the comparison's confidence (Appendix D.4). No other adaptive step.

## Normalization

T-14 per item: median over rounds of paired log ratios log T(1) − log T(n) with T(1) and T(n) from the same round block of the paired configuration specs (the one permitted cross-configuration comparison, §4.2, calibrated by `EXP-CAL-SPD-*`); L2-L4 aggregation per §4.9 with the cited workload mix (gate G-B) and equal weights as the sensitivity arm.

## Statistical analysis

R3/R4 on tuning; MDE gate; confirmatory W vs S on validation at corpus level (§4.10 Welch-Satterthwaite, §4.11 verdicts including `NON_INFERIOR`, §4.12 confidences); family, tier and condition guards (§4.9); computed cost tiers (§7.2: implementation-only parallelism with unchanged bytes T0; new planner id for `codec-internal-mt-pinned` T1); R6 minimum gains; R7 sensitivity; saturation point per operation reported as the smallest n whose S(n) is not `DISTINGUISHABLE_BETTER` than S(n_prev) (descriptive).

## Practical-significance thresholds

Referenced by ID: T-14 (r = 0.10, a = 0.2), T-03, T-04, T-05, T-05b, T-06; §7.3 minimum gains; §4.7 calibration criteria.

## Sensitivity analysis

§6 in full (weights over OD-13, OD-06, OD-07, OD-08 for decisions with those ODs; threshold ×0.5 subject to the resolvability condition; leave-one/three-families-out; family Dirichlet; WM-* mixes; tier-stratified; byte-weighted; environment arm Windows vs WSL requiring same-direction support; selection-stability bootstrap). Core-class arm (P vs E reported separately, never pooled).

## Expected negative results worth recording

No practical speedup for dense/extreme archives with long lookback; E-core speedups saturating at 4-8 workers; SMT adding nothing (`mt-psmt-14` vs `mt-p-7` equivalent); parallel verify slower than sequential on small items; CPU time per wall second saved exceeding incumbents'; WSL results not agreeing in direction with Windows.

## Threats to validity

Hybrid-core scheduling and power limits (§4.2 controls; `power_limit_confound` flag); Defender scanning of outputs (`defender_on` label and in-process timers); WSL virtualization (soft cap); prototype codegen vs a production implementation (R1-06; the campaign uses production-workspace dependency builds, and any adopted design must re-time as production code before `DECIDED`).

## Platform requirements

Windows 11 host with AC power and no concurrent workloads (exclusive windows scheduled by the orchestrator); WSL2; runner additions and passing calibration; about 20 GB. Admin not required. Storage classes other than the host's local SSD are EXP-SCALE-016.

## Estimated compute and effort

About 60 h of exclusive windows (13 configurations × about 25 design-operation pairs × tuning medium/large items × 10-20 rounds, plus the validation look). Scripted execution; judgment for analysis.

## Deviations (append-only)

None.
