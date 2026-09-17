# EXP-EVAL-004: Timing and memory protocol calibration per environment (A/A, A/A', known effect, background load, speedup A/A) and measurement-protocol comparison

| Field | Value |
|---|---|
| Domain | evaluation (program sections 6, 7, 34, 35) |
| Status | NEEDS_TOOLING (runner additions `decision-method.md` §14 item 3, validated by EXP-EVAL-003; known-effect injector; A/A' build pair; Windows corpus staging; protocol-comparison drivers) |
| Kind | Protocol calibration (§4.7). Required before the first decision-grade timing or memory analysis in each environment (§14 item 4, gate G-B for timing and memory). Calibration results are not decision results. The protocol-comparison arms are the empirical evidence for DEC-ECO-082. |
| Method-mandated ids | The §4.7 names map onto this experiment's arm ids: `EXP-CAL-AA-<env>` is `EXP-EVAL-004-AA-<env>`; likewise `-AAP-`, `-KE-`, `-BG-` and `-SPD-`. A strengthening re-run after a failure (§4.7 "on failure") appends `-r<k>`, which gives a new experiment id for each step. |
| Method | `research/decision-method.md` §4.2-§4.7, §4.10-§4.12 at `14b977c`; `thresholds.json` `protocol.aa_calibration`, `quiet_machine_guard`, `protocol.windows_affinity_configurations`, `protocol.wsl_affinity_configurations` |

## Author, independence and exposure

Designed by the Phase C-design evaluation-domain session. Calibration compares identical or known-effect arms, so no candidate is involved. The recorded L7 exposure (EXP-EVAL-001 protocol) does not bear on calibration.

## Question

1. In each environment (`windows-host`, `wsl-ubuntu`), for each timing and memory family that decisions will use, does the §4 protocol meet the §4.7 pass criteria? The families are `time_wall` (process and in-process), `time_cpu`, `memory_peak` (RSS, commit and allocator), `throughput` (derived), T-08 in-process per-operation latency, and T-14 speedup within a core class. The criteria are: identical arms judged `EQUIVALENT` with at most one item-level `DISTINGUISHABLE`; known effects of +2r detected; injected background load at the guard limits tolerated; median item half-width at the cap at most 0.5 x `log1p(r)`.
2. Which measurement protocol candidate for DEC-ECO-082 meets those same criteria, at what machine cost, and with what rank agreement on a pilot incumbent pair?

## Hypotheses and falsification

- **H-CAL.** `wsl-ubuntu` passes A/A, A/A', known effect, background load and speedup A/A for `time_wall`, `time_cpu` and `memory_peak` in `st-v` and `mt-v-n` (n in {2, 4, 8}) within the §4.7 strengthening ladder. `windows-host` passes in `st-p` and `mt-p-n`. `windows-host` E-core configurations and `os-scheduled` are expected to fail the half-width criterion; that expectation is `INFERRED` from the hybrid-core and Defender caveats.
  - Falsified per (environment, family, configuration) by any failed criterion after the full ladder. That (environment, family) is then **not decision-grade**, and every decision needing it uses the other environment or stays `INSUFFICIENT_EVIDENCE` (§4.7). Bands are never widened.
- **H-PROT (DEC-ECO-082).** Protocols are compared on `wsl-ubuntu` `st-v`, decode `time_wall`, on the A/A and known-effect candidate sets. P1 (the ebr §4 protocol) passes all criteria. P3 (fixed median-of-5, sequential, unguarded) exceeds one item-level `DISTINGUISHABLE` on A/A or misses the +2r detection fraction. P2 (hyperfine-style sequential per command) has more A/A false positives than P1, because it does not interleave candidates. P4 (stopping on bootstrap-CI width) reaches the half-width target in fewer rounds but with lower A/A coverage than P1.
  - Falsified if P3 or P2 passes every criterion. The result is then recorded as evidence that interleaving or guarding is not needed on this host, and it is reported against the method's choice.
  - Rank agreement between P1 and each other protocol on the pilot pair is expected to be Kendall tau-b ≥ 0.8 over items; lower agreement is reported as protocol dependence.

## Decisions informed

`DEC-ECO-082` ("A/A noise-floor distributions per host and metric family"; "rank agreement between protocols on a pilot candidate pair"). Every decision with a timing or memory primary metric depends on this experiment's pass as a gate, not as evidence.

## Candidates

### Calibration arms (per environment, family, affinity configuration)

| Arm | Candidates | Construction |
|---|---|---|
| AA | `aa-a`, `aa-b` | identical binary and arguments, distinct names |
| AAP | `aap-l1`, `aap-l2` | two functionally identical but byte-different REF-PROD `ebound` builds: (l1) default link order; (l2) `-C link-arg` reversed object order plus an unused 64 KiB padded section and a different `-C symbol-mangling-version`, verified byte-different with archive-output-identical. Each sample runs with a seeded random environment block of 0-4096 bytes |
| KE | `ke-0`, `ke-0p5`, `ke-1`, `ke-2` | `ke-k` appends a deterministic busy loop whose iteration count per item is pre-calibrated in a separate, non-analysed calibration run to k x r x (median `ke-0` wall of that item), r = the family reference band (T-04 0.05 for decode, T-03 `balanced` 0.05 for encode, T-06 0.10 memory via a deterministic extra allocation of k x r x median peak) |
| BG | `bg-a`, `bg-b` | as AA, with the §4.4 injected load: Windows 3.0% of all logical CPUs on E-cores plus 2.0% on the SMT sibling of the pinned CPU; WSL 3.0% on unpinned vCPUs |
| SPD | `spd-a`, `spd-b` | identical T-14 ratio arms: `st-p` with `mt-p-n` (n in {2, 4, 7}), `st-e` with `mt-e-n` (n in {2, 4, 8, 16}), `mt-psmt-14` with `mt-p-7`; WSL `st-v` with `mt-v-n` (n in {2, 4, 8}) |

### Workloads (fixed, pre-registered)

| Family | Workload | Measurement |
|---|---|---|
| decode `time_wall`, `time_cpu`, `memory_peak` | `ebound unpack <prebuilt balanced INDEXED archive> {scratch}/out` (REF-PROD binary of EXP-EVAL-002) | process (runner resource; GNU time `max_rss_bytes` on Linux, job `peak_commit_bytes` on Windows) |
| encode `time_wall`, `time_cpu` | `ebound pack {item_path} {scratch}/a.eb --profile balanced` | process |
| in-process `time_wall` (T-03/T-04 in-process floor) and T-08 | `ebr-access` INDEXED random-entry batch, 1,000 entries per round, seeded entry order, over the prebuilt archive | in-process batch timer |
| `alloc_peak_bytes` | harness counting allocator (§14 item 17) around `ebr-unpack` | in-process; arm skipped and family recorded not calibrated if item 17 does not exist |
| T-14 speedup | `zstd -19 -T{n}` compression of the item's tar stream (deterministic workload with real thread scaling) | process |

### Protocol-comparison candidates (DEC-ECO-082), `wsl-ubuntu`, `st-v`, decode workload, AA and KE candidate sets

| id | Protocol | Implementation |
|---|---|---|
| P1 | status quo plus the §4 controls: ebr randomized blocks, guard, cooldown, canary, precision-based adaptive rounds (§4.6) | `ebr run --timing` |
| P2 | hyperfine-style per-command timing: `--warmup 2 --runs 10`, candidates in sequential blocks, no guard | `hyperfine 1.18` (apt, pinned in run meta) `--export-json`; verdicts computed with the same §4.10/§4.11 procedures |
| P3 | fixed median-of-5 sequential, no guard, no cooldown | driver `research/experiments/EXP-EVAL-004/protocols/fixed_median.py` |
| P4 | sequential stopping when the ebr bootstrap CI half-width is at most 0.5 x `log1p(r)`, cap 30 | driver `.../protocols/ci_width_stop.py` |
| P7 | P1 but `os-scheduled` (unpinned) | `ebr run --timing` without `--cpus` |
| P8 | end-to-end scenario benchmark: scripted "CI artifact publish" (`ebound pack`, then `ebound convert --to tar.zst`, then `ebound verify`) timed as a whole | `ebr run --timing`, scenario script `.../protocols/scenario_publish.sh` |

- **Excluded.**
  - P5 (modeled cost metrics only): not a timing protocol, so no run. It is recorded analytically as unable to serve OD-06, OD-07 or OD-13.
  - P6 (hardware counters): `PLATFORM_BLOCKED`, because WSL2 exposes no PMU to the guest and Windows PMC sampling requires admin.
- **Pilot pair** for rank agreement: `zstd -3 -T1` versus `zstd -9 -T1` compression on the calibration items. This is an incumbent-only pair, so the result is not decision-relevant.

## Corpus selectors

Tuning split only; 24 items, 16 families, all 3 tiers (§4.7: at least 10 families and all tiers):

- small: `f01-tuning-zstd-v1-5-7-git`, `f02-tuning-lz4-intree-make-build`, `f05-tuning-gutenberg-books`, `f07-tuning-silesia-xray`, `f08-tuning-silesia-osdb`, `f10-gen-redundant-binary-blobs`, `f12-tuning-nasa-ivl-photos-small`, `f14-apache-maven-3916-bin-tarball`
- medium: `f01-tuning-curl-8-19-0-git`, `f03-tuning-ripgrep-cargo-vendor`, `f04-tuning-maildir`, `f05-tuning-loghub-openstack`, `f06-tuning-gharchive-2015-01-01-h15-h16`, `f07-tuning-sdss-dr17-frames`, `f09-silesia-mozilla-ooffice`, `f11-ubuntu-noble-debs`, `f16-alpine-virt-3241-iso`, `f18-tuning-zstd-releases-1-5-x`
- large: `f02-tuning-ripgrep-cargo-target`, `f06-tuning-gharchive-2024-01-15-h15`, `f07-tuning-pythia-160m-safetensors-step100000`, `f08-tuning-wikimedia-simplewiki-sql-dump`, `f12-tuning-nasa-ivl-photos-large`, `f13-tuning-bbb-video-large`

SPD uses the 10 medium items only. Windows runs use NTFS copies under `D:\eb-research\corpus-win\tuning\<item_id>`, staged by `research/experiments/EXP-EVAL-004/stage_windows_items.py` (to be written). The script copies from WSL, verifies `content_tree_sha256` against the manifest, and records items whose POSIX metadata cannot be represented on NTFS (the content timing workload is unaffected).

## Environment

- `wsl-ubuntu`: ext4 under `/root/eb-research`, `VIRTUALIZED`; affinity `st-v` = {2}, `mt-v-n` = {2..n+1}; guard `max_loadavg_1m` = 1.0 + n, `max_other_cpu_percent` 3.0, `max_wait_s` 300; host-side sampler active.
- `windows-host`: local NTFS on D: outside the repository; AC power; affinity configurations per `thresholds.json`; PDH frequency covariate; drift canary; throttling-onset probe; `defender_on` label; MsMpEng and SearchIndexer covariates.
- Exclusive timing windows, scheduled by the orchestrator with no agents, builds, provisioning or Docker. `disk_guard.py` passes first.
- Cache state `hot` (§4.5). A `vm-cold` sub-arm runs on WSL decode AA only, labelled "VM cache dropped", and makes no cold-cache claim.

## Commands

```sh
# prep (not timed): REF-PROD archives for the calibration items
#   research/experiments/EXP-EVAL-004/prep_archives.sh  -> /root/eb-research/cache/eval004/<item_id>.balanced.eb (+ sha256 list)
# known-effect iteration calibration (not analysed): research/experiments/EXP-EVAL-004/ke_calibrate.py --spec spec.yaml
# A/A' builds: research/experiments/EXP-EVAL-004/build_aap.sh -> /root/eb-research/target/aap-l1/ebound, aap-l2/ebound (+ Windows .exe)
# arm specs generated from this protocol: research/experiments/EXP-EVAL-004/make_arm_specs.py -> arms/EXP-EVAL-004-{AA,AAP,KE,BG,SPD}-{wsl-ubuntu,windows-host}-{workload}.yaml
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-EVAL-004/spec.yaml'
# WSL run of one arm (spec.yaml is the WSL AA decode arm):
#   python -m ebr run --timing --gzip --cpus 2 --spec research/experiments/EXP-EVAL-004/spec.yaml
#   python -m ebr verify-raw --spec ...; python -m ebr normalize --spec ...
# Windows: pwsh -File research/experiments/EXP-EVAL-004/run_windows_arm.ps1 -Spec arms/<spec>.yaml   (TO BE WRITTEN)
# background-load injector: research/experiments/EXP-EVAL-004/bgload.py --percent 3.0 --cpus <set> --duty-period-ms 100
# protocol comparison drivers: research/experiments/EXP-EVAL-004/protocols/{fixed_median.py,ci_width_stop.py,scenario_publish.sh}; hyperfine
# analysis: research/tools/decide calibrate --experiment EXP-EVAL-004-<arm>-<env> (decision tooling §14 item 2; TO BE WRITTEN)
#   -> research/normalized/EXP-EVAL-004/<arm>-<env>/calibration.json with every §4.7 criterion and the A/A item half-widths at the cap
```

## Seeds

- `seeds: {order: 20260923, bootstrap: 20260923, command: 20260923}`.
- Environment-block padding: 20260924.
- Random-entry order: 20260925.
- Strengthening re-runs use seed + k.

## Warmup

2 rounds for the small and medium tiers and 1 for the large tier (§4.5), recorded and excluded. If the warmup-adequacy check fails (median per-item Kendall tau outside ±0.3), one round is added for that environment and family, and the change is logged as strengthening.

## Measurement method and metrics

| Canonical metric | Threshold ID | Source | Env |
|---|---|---|---|
| `decode_wall_s`, `encode_wall_s` | T-04, T-03 (`balanced`) | runner resource `wall_s` (process floor 5 ms) | both |
| `decode_cpu_s`, `encode_cpu_s` | T-05 | runner `cpu_s` (process tree) | both |
| `decode_throughput_bps` | T-05b | derived | both |
| `peak_rss_bytes` | T-06 | GNU time `max_rss_bytes` (rows with `measurement.method = gnu-time-v+wait4` only; R1-16) | WSL |
| `peak_commit_bytes` | T-06 | job peak commit | Windows |
| `alloc_peak_bytes` | T-06 | counting allocator | both, if §14 item 17 exists |
| `random_entry_latency_s` | T-08 | in-process batch of 1,000, per-op = batch / N | both |
| `parallel_speedup` | T-14 | derived within one core class | both |

## Replication and adaptive rule

Timing rounds per `thresholds.json` `protocol.timing_rounds`: small 10/10/30, medium 10/10/20, large 10/5/20, with the minimum raised to n_min(0.99) = 8 where lower (it never is here). The precision-based adaptive rule of §4.6 applies per item and never looks at the effect direction. P2 and P3 use their fixed counts by definition.

## Normalization

Item level: the median of per-round paired log ratios (`pairing: item_repetition`). Group, family and corpus levels use **equal family weights** (calibration is not a workload claim). Tier strata are reported.

## Statistical analysis (pass criteria, `thresholds.json` `protocol.aa_calibration`)

- **AA, AAP, BG, SPD.**
  - The corpus-level Welch-Satterthwaite ratio interval at 0.99 is `EQUIVALENT`.
  - At most 1 item-level `DISTINGUISHABLE`.
  - At most 10% invalid samples.
  - Median item half-width at the cap at most 0.5 x `log1p(r)`.
- **KE.**
  - At +2r, at least 90% of items and the corpus verdict are `DISTINGUISHABLE_WORSE`.
  - At +1r, at least 80% of items have the correct sign.
  - No item is `DISTINGUISHABLE_BETTER` at any level.
- **Outputs used later.** A/A item half-widths at the cap feed the item-level MDE (§4.12) and the threshold-halving resolvability condition (§6.4).
- **Protocol comparison.** Per protocol, the same criteria apply, plus machine hours and Kendall tau-b (`stats.kendall_tau`) of item paired log ratios on the pilot pair against P1. The result is a table with no verdict beyond pass or fail per criterion. DEC-ECO-082's analysis session applies the decision rule; this experiment supplies the evidence.

## Practical-significance thresholds

The bands of T-03, T-04, T-05, T-05b, T-06, T-08 and T-14, as registered in `thresholds.json` `metric_thresholds`. Calibration verifies them and never widens them (§3.4, §4.7).

## Sensitivity analysis

Pass or fail is reported per tier and per affinity configuration, and with and without the `vm-cold` sub-arm (WSL). Leave-one-family-out applies to the corpus-level A/A verdict.

## Expected negative results worth recording

- `windows-host` E-core and `os-scheduled` configurations failing the half-width criterion.
- Defender scan-on-close making encode A/A' fail on Windows (distinct-binary reputation effects). If that happens, external comparisons need the harness-timer and exclusion-directory arms of §4.2.
- WSL `mt-v-8` speedup A/A failing, because vCPU placement is uncontrolled.
- P2 or P3 passing: the method's interleaving requirement not being empirically necessary on this host.

## Threats to validity

- **WSL2 virtualization.** Host load shows only as steal time; the host-side sampler mitigates this, but the `VIRTUALIZED` qualifier stays.
- **Thermal and power state.** Temperatures cannot be read without admin. The PDH frequency covariate and throttling-onset probe are the only proxies.
- **Known-effect construction.** A busy loop adds CPU time, not I/O time, so KE demonstrates sensitivity for CPU-bound differences only, and I/O-bound effects remain uncalibrated. A secondary KE-IO arm adds fsync'd writes of k x r x median bytes and is reported informationally.
- **Calibration items.** Tuning-only items may be unrepresentative of validation variance. The MDE gate (§4.12) re-estimates variance from each decision's own tuning data.

## Platform requirements

Windows 11 host (non-admin, AC power, exclusive window); WSL2 root (`drop_caches`, taskset within the VM); x86-64. No network. Windows incumbent tools are not needed (REF-PROD `ebound.exe` and `zstd.exe` only; the pinned `zstd` Windows binary is a download recorded with SHA-256).

## Estimates

- Machine time: about 20 h per environment for the calibration arms, about 15 h for the WSL protocol-comparison arms, and a 1.5x allowance for strengthening re-runs. About 80 h total, all in exclusive quiet windows.
- Disk: about 20 GiB (prebuilt archives and NTFS item copies).
- Agent effort: none during runs; scripted analysis; judgment only when a strengthening step must be chosen, and the ordered ladder of §4.7 makes even that mechanical.
