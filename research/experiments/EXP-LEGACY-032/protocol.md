# EXP-LEGACY-032: export and multi-target publish at scale — memory, time, scratch, ZIP64 emission strategies, strict re-import cost and transactional crash safety

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (research-only export prototypes in `entrybound::research::legacy::export` behind `research-internals` with identity proof; harness binary `ebr-legacy-export` with counting allocator and sink modes; streaming scale-input generator; crash-injection driver; runner write accounting and calibration prerequisites of C§2) |
| Domain / program sections | legacy / §27 (with §13 beyond-RAM operation) |
| decision_ids | DEC-ACC-013, DEC-LEG-019, DEC-LEG-108, DEC-LEG-018, DEC-LEG-010, DEC-LEG-017 |
| Decision types (proposed) | EMPIRICAL (time, memory, scratch, bytes) with binary HC-06/HC-18 claims |
| OD / HC (proposed) | OD-08 (M08.1-M08.3), OD-06 (encode wall/CPU as export cost), OD-05 (receipt and ZIP64 byte cost), OD-20 (M20.3); HC-06, HC-18, HC-07 |
| Shared rules / L7 / gates | C§ header, C§2 (timing and memory: calibration `EXP-CAL-AA/AAP/KE/BG-wsl-ubuntu` for families `time_wall`, `time_cpu`, `memory_peak`, `scratch_peak`; runner additions §14 item 3 incl. write accounting) |
| Timing | `timing: true` (WSL, `VIRTUALIZED`) |

## 1. Question

How do peak memory, scratch writes, wall and CPU time of legacy export and multi-target publish scale with source size
and entry count under the status quo (whole target prepared in memory, strict re-import with default import limits)
and under streaming, staging and alternative ZIP64 strategies; which strategies preserve byte identity with the frozen
profiles; how large do receipts become at 1,000,000 lossy entries; and do export and publish leave no partial or
overwritten outputs when killed at any seeded point?

## 2. Hypotheses

- **H1 (status-quo growth, DEC-ACC-013).** Status-quo export peak RSS grows at least linearly with target size (slope
  ≥ 0.9 bytes per target byte over the 1-16 GiB ladder) `[INFERRED from docs/legacy-export-v1.md: target fully prepared
  before creation]`. *Falsified by* a slope below 0.1.
- **H2 (streaming prototypes).** `streaming-export-sink` and `temp-file-staging-as-needed` keep
  `bounded_memory_growth_bytes` within their declared constant bound over 1-64 GiB and produce byte-identical output to
  the frozen profiles when written to a file. *Falsified by* any exceedance (binary) or byte difference.
- **H3 (ZIP64 policy, DEC-LEG-108).** `always-zip64-streaming-profile` differs in bytes from `zip/portable-v1` (by
  construction) and costs ≤ 1% `artifact_bytes` on the tuning real items; `per-entry-buffered-as-needed` is byte-identical
  to `zip/portable-v1` with peak memory bounded by the largest entry.
- **H4 (re-import cost, DEC-LEG-019).** Strict re-import accounts for more than 30% of export wall time on large items,
  and default import limits refuse at least one legitimate stress export (1,000,000 entries or a 5 GiB member),
  producing a non-typed failure under the status quo. *Falsified by* no refusal or a typed per-target issue.
- **H5 (receipts, DEC-LEG-018).** A receipt enumerating 1,000,000 lossy entries exceeds 100 MiB, while per-class counts
  with samples stay under 1 MiB `[INFERRED]`.
- **H6 (crash safety, DEC-LEG-010/017; HC-18).** For kills at 200 seeded points per operation (export to file, export
  to stdout pipe, publish with 3 targets), no destination name exists that verifies as a complete target, no existing
  file is overwritten, no partial publish set is visible, and stdout receives either zero bytes or the complete target.
  *Falsified by* one counterexample.
- **H0.** Export resources are independent of size and all strategies are equivalent.

## 3. Decisions informed

| Decision | Supplied here |
|---|---|
| DEC-ACC-013 | Time, peak memory and temp disk for multi-GiB exports to ZIP64 and `tar.zst` and full publish runs; overhead of strict re-import; crash-consistent multi-target commit |
| DEC-LEG-019 | Stress exports (1 GiB zero file, 5 GiB member, 1,000,000 entries) per profile under default versus source-derived re-import limits; memory and time of re-import |
| DEC-LEG-108 | Peak memory, temp disk and wall time of each ZIP64 emission strategy for multi-GiB and > 65,535-entry exports to files and pipes; byte identity with `zip/portable-v1` (reader acceptance in EXP-LEGACY-030) |
| DEC-LEG-018 | Receipt size at 1,000,000 lossy entries for per-entry enumeration versus per-class counts with samples |
| DEC-LEG-010 | Failure-mode evidence for the publish transaction (crash injection) |
| DEC-LEG-017 | Rollback behaviour for stdout targets and whether refused exports leave receipts |

## 4. Candidates

| Decision | Candidates (ledger ids) | Realisation |
|---|---|---|
| DEC-ACC-013 | status-quo-all-in-memory; streaming-export-sink; sequential-target-preflight; streamed-strict-reimport; declared-ceilings | `ebound` CLI (status quo); prototypes via `ebr-legacy-export --strategy stream-sink`, `--publish sequential-preflight`, `--reimport streamed`; `declared-ceilings` evaluated as a rule over measured ceilings |
| DEC-LEG-019 | status-quo-import-defaults; source-derived-limits; typed-target-limit-issue; skip-reimport-above-threshold | `--reimport default|source-derived|typed-issue|skip`; `skip-reimport-above-threshold` is a proposed L0 exclusion (freeze: strict re-import before publication, `docs/legacy-export-v1.md`) and is measured only as a cost reference |
| DEC-LEG-108 | status-quo-in-memory-as-needed; per-entry-buffered-as-needed; temp-file-staging-as-needed; always-zip64-streaming-profile; never-zip64-refuse-before-write; as-needed-fail-late; refuse-non-seekable-large-zip | `--strategy` values of the same names |
| DEC-LEG-018 | status-quo-v1-v2; per-entry-enumeration; per-class-counts-with-samples; receipt-v3-unified (size only) | `--receipt-format` prototypes |
| DEC-LEG-010, DEC-LEG-017 | status quo (publish, stdout preparation) versus `refusal-receipts` and `receipt-to-stderr-or-fd` (receipt presence only) | crash driver over CLI and prototypes |

## 5. Corpus selectors

Generated streaming scale inputs (`research/tools/legacy/cases/scale_inputs.py`, packed to `.eb` once with
`ebound-head pack --profile fast`; F20/F04; `S_T = 2509173201`, `S_V = 2509173202`): one 1 GiB zero file; one 5 GiB
`random-v1` file; 1,000,000 files of 1-100 B; 70,000 files; a logical-size ladder 1, 4, 16, 64 GiB (zeros and `text-v1`
mix); a 1,000,000-entry tree whose export is LOSSY per entry (ns mtimes, uid 1000). Real large items (tuning):
`f01-tuning-llvm-project-20-1-8-git`, `f05-tuning-loghub-hdfs-v1`, `f02-tuning-ripgrep-cargo-target`; (validation):
`f01-validation-cpython-3-13-15-full-clone`, `f03-validation-bat-cargo-vendor`, `f13-validation-sintel-media-large`.
The 16 and 64 GiB rungs are a beyond-RAM stratum (program §13) reported separately from the §4.6 tiers. Held-out:
placeholder (C§5).

## 6. Environment

`wsl-ubuntu`, timing window exclusive (§4.2), `st-v` = {2} for single-threaded export, `mt-v-8` = {2..9} arm where the
build uses threads; cache `hot`; per-sample scratch on a dedicated loop-mounted ext4 volume for write accounting
(§4.14); sinks: file on that volume, pipe to `cat > /dev/null`, and `/dev/null`. Windows arms are informational only
(§14 item 3) and not run here.

## 7. Commands and tooling

Tooling: `entrybound::research::legacy::export` prototypes (strategies and receipt formats) behind `research-internals`
with the C§3 identity proof; `research/harness/crates/ebr-legacy` binary `ebr-legacy-export --source <eb> --target
<profile> --strategy <s> --reimport <r> --sink file|pipe|null --receipt-format <f>` printing one `ebr.harness.raw.v1` row
(alloc peak, stage timings, bytes); `cases/scale_inputs.py`; `research/tools/legacy/crash_inject.py --op export-file|
export-stdout|publish --points 200 --seed <s>` (SIGKILL at seeded byte/syscall counts, then oracle checks);
`research/tools/legacy/scratch_volume.sh` (per-sample loop ext4).

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
python3 research/harness/lock_parity.py && bash research/harness/harness-cli-parity.sh
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-032 --generator scale_inputs --split tuning --seed 2509173201 --pack-with /root/eb-research/target/legacy-head/release/ebound --append-corpus-items research/experiments/EXP-LEGACY-032/real-items-tuning.txt
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-032 --generator scale_inputs --split validation --seed 2509173202 --pack-with /root/eb-research/target/legacy-head/release/ebound --append-corpus-items research/experiments/EXP-LEGACY-032/real-items-validation.txt
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-032/spec.yaml --timing
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-032/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-032/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/crash_inject.py --experiment EXP-LEGACY-032 --ops export-file,export-stdout,publish --points 200 --seed 2509173214 --out research/raw/EXP-LEGACY-032/crash/
```

## 8. Seeds

`order 2509173211`, `bootstrap 2509173212`, `command 2509173213`; crash points `2509173214`; input seeds §5.

## 9. Warmup, replication, adaptive rule

Warmups 1 (large tier) or 2 (small/medium real items); §4.6 table (large: minimum 10 rounds, step 5, cap 20; the
beyond-RAM rungs use the large-tier rule) with the precision-based adaptive rule; byte outputs checked by repeat-hash;
binary bounded-memory and streaming claims decided per sample (any exceedance is a counterexample). Crash injection:
200 seeded kill points per operation, each once, plus a rerun of any point whose oracle fails (the failing artifact is
kept either way, objective §2.0 rule 2).

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `peak_rss_bytes` (process) | banded T-06 | OD-08 | T-06 |
| `alloc_peak_bytes` (counting allocator) | banded T-06 | OD-08 | T-06 |
| `scratch_peak_bytes` | banded T-07 | OD-08 | T-07 |
| `scratch_write_bytes` (zero-scratch claims) | binary | OD-08 | §3.3 |
| `bounded_memory_growth_bytes` (streaming and bounded claims over the 1-64 GiB ladder) | binary | OD-08 | §3.3 |
| `streaming_capability` (pipe sinks) | binary when claimed | OD-09 | §3.3 |
| `encode_wall_s`, `encode_cpu_s` (export as encode operation; process floors; small-tier in-process rule) | banded T-03/T-05 (profile band `balanced`) | OD-06 | T-03, T-05 |
| `artifact_bytes` (ZIP strategy outputs) and `side_data_bytes` (receipts) | banded T-01 / T-02 (diagnostic→primary for DEC-LEG-018, which is about that component) | OD-05 | T-01, T-02 |
| `migration_determinism_fraction` (strategy output equals frozen-profile bytes where identity is claimed) | binary | OD-20 | §3.3 |
| HC-18 crash oracle outcomes (verifying partial targets, overwrites, partial publish sets, partial stdout) | binary HC-18 test | — | objective HC-18 |

## 11. Normalization

Memory and time per sample; ratios candidate/status quo per item for banded comparisons; growth slopes by least squares
over the ladder (descriptive) and the binary bound check per rung.

## 12. Statistical analysis

§4.9-§4.12: item-level exact order-statistic intervals of paired log ratios; the real-item set has 3 groups per split
across 3 families, below the §4.9 corpus-level minimum, so decisions from this experiment are scoped to **item-level
and ladder-level evidence** (recorded limitation) and binary claims; generated rungs are strata, never averaged with
real items. Confirmatory comparisons (W against each tuning survivor per primary metric) are declared after tuning with
§4.12 confidences and the MDE gate.

## 13. Practical significance

T-01, T-02, T-03, T-05, T-06, T-07 per `decision-method.md` §3.2; binary metrics §3.3.

## 14. Sensitivity analysis

Sink arm (file, pipe, null); affinity arm (`st-v` versus `mt-v-8`); profile arm (`zip/portable-v1`, `tar.zst/pax-v1`,
`tar.xz/pax-v1`); threshold perturbation §6.4 on banded metrics; ladder-rung leave-one-out for slopes.

## 15. Expected negative results worth recording

Streaming prototypes unable to keep byte identity for `zip/portable-v1` (needs a new profile id); strict re-import
dominating export time; receipts growing beyond practical size; any crash counterexample (a production defect).

## 16. Threats to validity

WSL memory accounting under a VM (`VIRTUALIZED`); prototypes are research implementations whose memory may exceed a
production-quality version (reported as upper bounds); 64 GiB inputs of compressible content under-represent I/O
throughput of real data; kill points at syscall granularity may miss narrow windows (regression evidence, no bound).

## 17. Platform requirements

Linux x86-64 WSL2 in an exclusive timing window; ≥ 90 GB free for staging arms and the 64 GiB rung; loop-mounted scratch
volume (root inside WSL); Windows timing not decision-grade here; macOS **PLATFORM_BLOCKED**.

## 18. Estimates

Compute ≈ 150 compute-hours (timing window); disk ≈ 90 GB peak. Agent effort: prototype implementation judgment
(research-only code), execution none, analysis judgment.

## 19. Expected cost tiers `[INFERRED]`

Streaming export keeping frozen bytes: T0-T1 (implementation); Always-ZIP64 streaming profile: T2 (new profile);
source-derived re-import limits: T1; typed target-limit issue: T2 (new issue class); receipt v3: T2; sequential publish
preflight: T0-T1.
