# EXP-SCALE-009: Bounded-memory legacy converter prototypes (import, export, publish, ZIP64 emission)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** |
| Domain | scale (program §13; interfaces with legacy §25-§27) |
| Platforms | Linux x86-64 (WSL2, root, cgroup-v1, loop mounts) |
| Requires timing | true (phase T for export and import throughput); phase M needs none |
| Compute estimate | about 40 machine-hours (M about 25 h, T about 15 h) |
| Disk estimate | about 150 GB peak |
| Agent effort | judgment (prototypes, byte-identity arguments against frozen export profiles); scripted execution |
| Tooling to build | research-only harness crate `research/harness/crates/ebr-scale-legacy` (binary `ebr-scale-legacy`) over `entrybound::legacy` public API and default-off `research-internals` wrappers; `ebr-forge --case 7z-unknown-filesinfo` (format-building 7z generator with unknown FilesInfo properties, 100,000 files); shared counting allocator `ebr-alloc` (EXP-CONF-007); runner additions and calibration (§14 items 3-4) for phase T |

## Pre-registration

- **decision_ids:** DEC-ACC-013, DEC-ACC-017, DEC-LEG-002, DEC-LEG-019, DEC-LEG-022, DEC-LEG-030, DEC-LEG-059, DEC-LEG-108.
- **Decision type:** EMPIRICAL; new export profile identifiers (for example always-ZIP64 streaming) carry FORMAL and reader-acceptance parts routed to the legacy domain.
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-08, OD-19 (M19.1, M19.2, M19.4), OD-20 (identity equivalence of streamed vs in-memory import), OD-06; HC-06, HC-07 (declared loss), HC-13 (export bytes are a deterministic function of EAM and target profile), HC-16 (frozen `zip/portable-v1` bytes), HC-18 (publish atomicity).
- **Method, gate, author, L7:** as EXP-SCALE-001; critic candidate and non-exposed sign-off required before G-A.

## Question

Which converter designs make legacy import, export and multi-target publish bounded in memory for inputs larger than RAM, while producing identical identities (import) or identical bytes for frozen export profiles, correct declared-loss records, safe refusals and atomic publish; how do conversion evidence and provenance scale per entry under aggregation; and what are the memory, disk and reader-acceptance-relevant properties of each ZIP64 emission policy for large and non-seekable exports?

## Hypotheses

- **H1a:** `streaming-decode-spill` import produces LAI and AUX identical to the in-memory import for tar, tar.gz, tar.zst, tar.xz, ZIP and 7z on every EXP-SCALE-002 scaling cell, with M08.3-bounded growth on the bytes axis.
- **H1b:** `compact-lom-evidence` plus `aggregate-resolutions` reduce `side_data_bytes` per entry on `n999k` ZIP import by more than the T2 minimum gain on `metadata_bytes_per_entry` (T-02b, k = 3), and let `tar-500k` import complete under the default observation budget.
- **H1c:** `per-entry-buffered-as-needed` ZIP export is byte-identical to `zip/portable-v1` on every cell (so it is a T0 implementation change) with memory bounded by the largest encoded entry; `temp-file-staging-as-needed` is byte-identical with one archive of temporary disk; `always-zip64-streaming` differs in bytes (a new profile identifier, T2 or higher).
- **H1d:** `streaming-export-sink` plus `sequential-target-preflight` plus `streamed-strict-reimport` bound publish memory and keep publish all-or-nothing under kills (MVT-18(a)).
- **H1e:** `source-derived-limits` for strict re-import remove the EXP-SCALE-002 self-refusals (1 GiB zero file, 5 GiB member) without admitting the bomb fixtures.
- **H0 / falsification:** per candidate, a binary counterexample (identity or byte mismatch where claimed, unbounded growth, partial publish, admitted bomb) or a non-supporting confirmatory verdict.

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-ACC-013 | Time, peak memory and temporary disk per export/publish design; overhead of strict re-import per design; atomic multi-target commit under kill | status quo: EXP-SCALE-002, EXP-SCALE-004 |
| DEC-ACC-017 | Peak RSS vs input size per import design; LOM heap bytes per entry at 100k and 1M entries; equivalence of streamed vs in-memory import identities | — |
| DEC-LEG-002 | Conflicts, provenance bytes and AUX time under exclude-structural-absence, aggregate-resolutions and external-evidence-object at 100k/300k/1M-entry ZIPs | auditor needs: human participants (not measurable) |
| DEC-LEG-019 | Stress exports under source-derived limits and typed TARGET_LIMIT_EXCEEDED; re-import memory and time | — |
| DEC-LEG-022 | Peak RSS and time per limit at candidate values (from EXP-SCALE-003's candidate table) under the bounded import designs; bomb fixture refusal | corpus distributions: EXP-SCALE-003 |
| DEC-LEG-030 | Peak memory of `buffer-stdin-under-budget` and disk of `spool-to-temp` for stdin import, tar-only streaming for tar and compressed tar | workflow survey and first-use study: not measurable here |
| DEC-LEG-059 | 500k and 1M-entry tars with and without pax under add-provenance-byte-bound, derive-observation-cap, count-shared-observations; 100k-file 7z with unknown FilesInfo properties | fuzzing for unbounded observation growth: conformance domain |
| DEC-LEG-108 | Peak memory, temporary disk and wall per ZIP64 policy for multi-GiB and >65,535-entry exports to files and pipes; byte identity of per-entry-buffered and temp-staged output with `zip/portable-v1` vectors | reader acceptance matrix (Info-ZIP, Python, Java, Go, .NET, 7-Zip, Explorer, PyPI validator): legacy domain; demand evidence: ecosystem domain |

## Candidates

| Group | candidate | Definition |
|---|---|---|
| Import | `sq-import` | production `convert` (reference) |
| | `streaming-decode-spill` | streamed decode with plaintext spill and per-entry LOM evidence |
| | `compact-lom-evidence` | observation model without duplicate header copies |
| | `import-to-stream-producer` | import directly into EXP-SCALE-007's incremental STREAM producer |
| | `declared-ceilings` | status quo with declared per-format ceilings and pre-read typed refusal |
| Evidence | `exclude-structural-absence`, `aggregate-resolutions`, `external-evidence-object`, `add-provenance-byte-bound`, `derive-observation-cap`, `count-shared-observations` | as named in DEC-LEG-002 and DEC-LEG-059 |
| stdin | `sq-file-only`, `buffer-stdin-under-budget`, `stream-tar-only`, `spool-to-temp` | as named in DEC-LEG-030 |
| Export | `sq-export` | production (reference) |
| | `streaming-export-sink` | each target streamed to a temp file with incremental digest |
| | `sequential-target-preflight` | prepare and verify one target at a time, keeping only digests |
| | `streamed-strict-reimport` | strict re-import from the temp file with bounded memory |
| | `source-derived-limits`, `typed-target-limit-issue`, `skip-reimport-above-threshold` | as named in DEC-LEG-019 (`skip-reimport-above-threshold` must report an explicit unverified state, HC-15) |
| ZIP64 | `sq-in-memory-as-needed`, `per-entry-buffered-as-needed`, `temp-file-staging-as-needed`, `always-zip64-streaming-profile`, `never-zip64-refuse-before-write`, `as-needed-fail-late`, `refuse-non-seekable-large-zip` | as named in DEC-LEG-108 |
| References | `inc-bsdtar`, `inc-7z` | incumbent create/extract memory references |

- **Excluded:** `unlimited-when-unset` (R2 unboundable: no bound exists before payload; one counterexample is EXP-SCALE-002 `tar-999k` observation growth); `as-needed-fail-late` is kept as a candidate but its partial pipe output is an HC-18-relevant outcome recorded, not excluded a priori.

## Corpus selectors

Generated EXP-SCALE-002 cells (scaling and boundary) plus `n300k` ZIP (300,000 files of 64 B, seed 1901) and the forged `7z-unknown-filesinfo` (seed 1902); bomb fixtures from tuning items `f20-tuning-bombs-dense` and validation `f20-validation-bombs-compressed` (manifest splits tuning, validation) for admission checks; F14 tuning items for phase T and one registered validation look on F14 validation items. **Held-out placeholder (Phase D):** provisional selections re-run once on held-out F14 and F20 items after unlock (§5.5).

## Environment

As EXP-SCALE-002 (caps, loop mounts, incumbent versions) for phase M; phase T as EXP-SCALE-007 (timing guard, `st-v`, calibration). Kill oracles reuse EXP-SCALE-004's `kill-publish` and `kill-export-zip` scenarios with `--converter ebr-scale-legacy --design <d>`.

## Commands

```sh
wsl.exe -d Ubuntu -- bash research/harness/build.sh build --release -p ebr-scale-legacy -p ebr-pack --features ebr-alloc
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-SCALE-009/spec.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --timing --spec research/experiments/EXP-SCALE-009/spec-timing.yaml
```

Binary contract: `ebr-scale-legacy --design <candidate> --cell <id> --cells research/experiments/EXP-SCALE-002/cells.json --source <path|-> --target <profile> --out <path|-> --seed <n> --experiment-id EXP-SCALE-009 --env-name wsl-ubuntu`; every output checked (import: `ebound verify` and identity comparison with `sq-import`; export: byte comparison with `sq-export` where identity is claimed, and incumbent re-extraction digest).

## Seeds

`seeds.order` 1309001, `seeds.bootstrap` 1309002, `seeds.command` 1309003.

## Warmup

M: 0. T: §4.5.

## Measurement method and metrics

| Metric | Canonical name | Role | Family |
|---|---|---|---|
| `alloc_peak_bytes`, `bounded_memory_growth_bytes` | T-06; M08.3 | primary; binary | memory_peak; correctness |
| `scratch_write_bytes`, `scratch_peak_bytes` | §4.14; T-07 | primary (temporary disk) | correctness; scratch_peak |
| `side_data_bytes`, `metadata_bytes_per_entry` | T-02; T-02b | primary for evidence designs | size |
| `identity_lai_equal`, `identity_aux_equal`, `export_bytes_equal_profile` | HC-10/HC-13; HC-16 | binary | correctness |
| `import_acceptance_fraction`, `export_acceptance_fraction` | T-20 over the EXP-SCALE-002 boundary list and F14 items | primary | other |
| `adversarial_true_refusal_fraction` | HC-06 over the bomb fixtures | binary (= 1) | correctness |
| `partial_publish_sets`, `verified_ok_partial_outputs` | HC-18 | binary | correctness |
| `encode_wall_s`, `decode_wall_s`, `encode_throughput_bps` | T-03, T-04, T-05b | primary (phase T) | time_wall, throughput |

## Replication

M08.3 probes 3 runs per size; deterministic outputs 2 repetitions plus repeat-hash; kill scenarios 3 × 25; phase T §4.6.

## Normalization and statistical analysis

As EXP-SCALE-007 (R3/R4 tuning, MDE gate, one validation look, §4.10-§4.12, guards, computed tiers: byte-identical export implementations T0; new export profile identifiers T2 plus reader-acceptance evidence; new evidence records T2-T3), binary oracles by counterexample.

## Practical-significance thresholds

Referenced by ID: T-02, T-02b, T-03, T-04, T-05b, T-06, T-07, T-20; M08.3; §7.3.

## Sensitivity analysis

Full §6 for decisions reaching R8/R9; format arm (tar, compressed tar, ZIP, 7z reported separately, never averaged, as legacy formats are evaluation conditions per §1.1); size-tier arm.

## Expected negative results worth recording

Streamed import changing AUX (evidence ordering) and therefore identities; aggregated evidence failing auditor-level itemization (reported as a limitation for the human-facing part); per-entry buffering still unbounded for a single huge member; always-ZIP64 output rejected by validators (legacy domain evidence); streamed strict re-import slower than the in-memory version by several bands.

## Threats to validity

Prototype constant factors; incumbent-generated sources; bomb fixtures are an enumeration (regression evidence); reader-acceptance conclusions are out of scope here.

## Platform requirements

Linux x86-64 WSL2 root; about 150 GB; calibration and runner additions for phase T.

## Estimated compute and effort

About 40 h after tooling; tooling judgment.

## Deviations (append-only)

None.
