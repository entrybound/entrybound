# EXP-LEGACY-040: legacy import at scale — peak memory, time, spill, conversion-evidence growth and streaming import candidates

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (streaming scale-input generators for tar/ZIP/7z; `ebr-legacy-import` with counting allocator and site attribution; research-only import prototypes in `entrybound::research::legacy::import` with identity proof; provenance field-set size calculator; calibration and runner prerequisites C§2) |
| Domain / program sections | legacy / §25 (with §13 beyond-RAM operation, §28 record schema) |
| decision_ids | DEC-ACC-017, DEC-LEG-002, DEC-LEG-003, DEC-LEG-022, DEC-LEG-059, DEC-LEG-093, DEC-LEG-101, DEC-LEG-096 (timing arm) |
| Decision types (proposed) | EMPIRICAL with binary HC-06 claims |
| OD / HC (proposed) | OD-08 (M08.1-M08.3), OD-06 (import as encode cost), OD-05 (provenance bytes), OD-17 (M17.6 scale limits), OD-20 (M20.1 identity equality of candidates); HC-06, HC-07, HC-10 |
| Shared rules / L7 / gates | C§ header, C§2 (calibration for `time_wall`, `time_cpu`, `memory_peak`, `scratch_peak`; runner write accounting) |
| Timing | `timing: true` (WSL, `VIRTUALIZED`) |

## 1. Question

How do peak memory, wall and CPU time, spill and conversion-evidence size of legacy import (ZIP, tar, compressed tar,
7z solid folders) scale with input size and entry count under the status quo and under streaming, compact-evidence,
aggregated-resolution and bounded-provenance candidates; where do current caps bind; do candidates preserve import
identities; and can ZIP import be driven by local headers without changing results?

## 2. Hypotheses

- **H1 (status-quo growth, DEC-ACC-017).** Status-quo import peak memory grows with input size for at least one format
  (predicted: 7z solid folders and compressed tar) with slope ≥ 0.5 byte per input byte over 1-16 GiB `[INFERRED]`.
  *Falsified by* slopes below 0.05 for every format.
- **H2 (evidence growth, DEC-LEG-002).** For 1,000,000-entry ZIPs with data descriptors, per-entry conflicts make
  conversion evidence exceed the 256 MiB cap under the status quo; `exclude-structural-absence` plus
  `aggregate-resolutions` keeps it below 1 MiB with identical LAI. *Falsified by* status-quo evidence under the cap, or a
  candidate changing LAI.
- **H3 (observation caps, DEC-LEG-059).** tar's per-header observations make the default observation cap bind before the
  default entry cap (predicted near 420,000 entries for pax-bearing tars, per the ledger's arithmetic, to be measured),
  and unknown 7z FilesInfo bytes copied per file grow provenance linearly with no byte bound.
- **H4 (streaming candidates).** `streaming-decode-spill` and `import-to-stream-producer` keep
  `bounded_memory_growth_bytes` within their declared bounds over 1-16 GiB, with `scratch_write_bytes` equal to their
  declared spill, and produce LAI/AUX/PCR identical to status-quo imports where both complete.
- **H5 (LFH streaming, DEC-LEG-101).** On EXP-LEGACY-001 C0-C14 cases, local-header-driven projection differs from
  central-directory-driven projection on at least 10% of cases, so `lfh-streaming-untrusted` must label results
  untrusted until reconciliation; on benign real ZIPs the two agree.
- **H0.** Import memory, time and evidence are independent of size and entry count.

## 3. Decisions informed

| Decision | Supplied here |
|---|---|
| DEC-ACC-017 | Peak RSS versus input size for 1/4/16 GiB tars (bare, gz, zst, xz), ZIP64 and solid 7z; LOM heap bytes per entry at 100k/1M entries; identity equivalence of streamed versus in-memory import |
| DEC-LEG-002 | Imports of 100k/300k/1M-entry ZIPs: conflicts, provenance bytes, AUX computation time per candidate |
| DEC-LEG-003 | Record size of each candidate field set on large and adversarial imports (computed exactly from observations) |
| DEC-LEG-022, DEC-LEG-093 | Peak RSS and time at the status-quo and candidate limit values for large legitimate inputs (defaults themselves are selected in EXP-LEGACY-041) |
| DEC-LEG-059 | Synthetic tars with 500k and 1M entries with and without pax; 100k-file 7z with unknown FilesInfo properties under default policy; provenance byte growth |
| DEC-LEG-101 | Memory of spool-then-CD versus LFH streaming prototypes on pipes; projection agreement LFH versus CD |

## 4. Candidates

| Decision | Candidates | Realisation |
|---|---|---|
| DEC-ACC-017 | status-quo-in-memory; streaming-decode-spill; compact-lom-evidence; import-to-stream-producer; declared-ceilings | `ebound convert` (status quo); prototypes via `ebr-legacy-import --strategy`; `declared-ceilings` evaluated as a rule over measured ceilings |
| DEC-LEG-002 | status-quo-per-entry; exclude-structural-absence; aggregate-resolutions; external-evidence-object | `--evidence` prototypes |
| DEC-LEG-003 | status-quo-provenance; spec-field-set; minimal-additions; external-receipt-json; preserve-only-detail | exact serialized-size computation per field set (`provenance_sizes.py`) from the same observations |
| DEC-LEG-059 | status-quo-independent-caps; add-provenance-byte-bound; derive-observation-cap; count-shared-observations; aggregate-resolutions; unlimited-when-unset | `unlimited-when-unset` is a proposed L0 exclusion (`docs/legacy-preservation-v1.md` L86-90: absence of an override never means unlimited); run only as a growth reference with a 64 GiB hard memory guard |
| DEC-LEG-101 | status-quo-complete-source-cd; spool-then-cd; lfh-streaming-untrusted; lfh-streaming-compat-profile | prototypes on pipe inputs |
| DEC-LEG-022, DEC-LEG-093 | status quo constants (read from code with `path:line`) and the candidate default sets of EXP-LEGACY-041 §4 | `--policy-file` per candidate |
| (timing arm for EXP-LEGACY-050, DEC-LEG-096) | compat versus compat+preserve import time and memory on the ZIP inputs | candidates `compat-libarchive`, `preserve-libarchive` |

## 5. Corpus selectors

Generated (`research/tools/legacy/cases/scale_import_inputs.py`, streaming; F20/F04; `S_T = 2509174001`,
`S_V = 2509174002`): tar 1/4/16 GiB × {bare, gz, zst, xz} with lognormal member sizes of `text-v1`/`random-v1`;
tars with 100k/500k/1M entries with and without pax long names; ZIPs 1 GiB and 4 GiB (one member > 4 GiB, ZIP64),
100k/300k/1M entries, each with and without data descriptors; 7z solid 1/4 GiB with 64 MiB and 1536 MiB dictionaries;
7z with 100k files, with and without unknown FilesInfo property bytes. Real (tuning): GNU tar of
`f01-tuning-llvm-project-20-1-8-git`, Info-ZIP ZIP of `f02-tuning-ripgrep-cargo-target`, 7-Zip 7z of
`f05-tuning-loghub-hdfs-v1`; (validation): GNU tar of `f01-validation-cpython-3-13-15-full-clone`, Info-ZIP ZIP of
`f03-validation-bat-cargo-vendor`, 7-Zip 7z of `f13-validation-sintel-media-large`. LFH-versus-CD arm: EXP-LEGACY-001
C0-C14 tuning/validation cases and ZIP-family real items there. The 16 GiB rung is the beyond-RAM stratum. Held-out:
placeholder (C§5).

## 6. Environment

`wsl-ubuntu` exclusive timing window; `st-v` = {2}; cache `hot`; dedicated loop-mounted scratch volume per sample for
write accounting; out-of-memory kills classified as `oom_killed` (neither pass nor outcome difference) and recorded as
scale limits.

## 7. Commands and tooling

Tooling: `cases/scale_import_inputs.py`; `ebr-legacy-import --input <path> --from <fmt> --mode strict|compat:<p>
--strategy status-quo|streaming-decode-spill|compact-lom-evidence|import-to-stream-producer|spool-then-cd|lfh-streaming-untrusted
--evidence per-entry|exclude-structural-absence|aggregate-resolutions|external-evidence-object --policy-file <json>
--out <eb> --experiment-id EXP-LEGACY-040 --env-name wsl-ubuntu` (JSON row: allocator peak by site class, stage timings,
conflicts, observations, provenance bytes, LAI/AUX/PCR); `provenance_sizes.py`; `lfh_vs_cd.py`.

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
python3 research/harness/lock_parity.py && bash research/harness/harness-cli-parity.sh
bash research/harness/internals-identity-check.sh   # legacy import cells, prototypes off
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-040 --generator scale_import_inputs --split tuning --seed 2509174001 --append-corpus-items research/experiments/EXP-LEGACY-040/real-items-tuning.txt
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-040 --generator scale_import_inputs --split validation --seed 2509174002 --append-corpus-items research/experiments/EXP-LEGACY-040/real-items-validation.txt
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-040/spec.yaml --timing
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-040/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-040/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/provenance_sizes.py --experiment EXP-LEGACY-040 --out research/normalized/EXP-LEGACY-040/provenance-sizes.csv
/root/eb-research/venv/bin/python research/tools/legacy/lfh_vs_cd.py --cases EXP-LEGACY-001 --out research/normalized/EXP-LEGACY-040/lfh-vs-cd.csv
```

## 8. Seeds

`order 2509174011`, `bootstrap 2509174012`, `command 2509174013`; input seeds §5.

## 9. Warmup, replication, adaptive rule

Warmups 1 (large and beyond-RAM), 2 (smaller); §4.6 rounds (large: 10, step 5, cap 20) with the precision rule;
deterministic byte and identity outputs by repeat-hash; binary bound checks per sample.

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `peak_rss_bytes` | banded T-06 | OD-08 | T-06 |
| `alloc_peak_bytes` (with site attribution: evidence, decode, planning) | banded T-06 | OD-08 | T-06 |
| `scratch_peak_bytes` / `scratch_write_bytes` | banded T-07 / binary (spill claims) | OD-08 | T-07 / §3.3 |
| `bounded_memory_growth_bytes` | binary | OD-08 | §3.3 |
| `encode_wall_s`, `encode_cpu_s` (import+pack) | banded T-03/T-05 (`balanced` row) | OD-06 | T-03, T-05 |
| `side_data_bytes` (conversion provenance) and `metadata_bytes_per_entry` | banded T-02 / T-02b (primary for DEC-LEG-002/003/059, which are about that component) | OD-05 | T-02, T-02b |
| `migration_identity_conformance_fraction` (candidate import identities equal status quo where both complete) | binary | OD-20 | §3.3 |
| `import_agreement_fraction` (LFH-driven vs CD-driven projection) | T-20 | OD-19 | T-20 |
| `scale_limits` (entries and bytes at which caps bind or OOM occurs), `hostile_cpu_scaling_exponent` (CPU vs entries) | descriptive | OD-17 | none |

## 11. Normalization

Paired log ratios candidate / status quo per (item, round); slopes over ladders (descriptive); evidence bytes exact.

## 12. Statistical analysis

§4.9-§4.12; generated ladders and entry-count series are strata reported separately; the real-item set (3 groups per
split) is below corpus-level support, so selections rest on item/stratum-level verdicts and binary claims, recorded as a
scope limitation; confirmatory comparisons declared after tuning with the MDE gate.

## 13. Practical significance

T-02, T-02b, T-03, T-05, T-06, T-07, T-20 per `decision-method.md` §3.2; binary §3.3.

## 14. Sensitivity analysis

§6.4 threshold perturbation; format arm (per format, never pooled); compression arm; policy arm (status-quo versus each
EXP-LEGACY-041 default set); affinity arm (`st-v` versus `mt-v-8`).

## 15. Expected negative results worth recording

Status quo already bounded for some formats; aggregated evidence losing auditor-relevant detail (recorded with the
field-set sizes); streaming prototypes changing identities (then rejected under R1).

## 16. Threats to validity

Synthetic entry-count series lack real name/metadata diversity; prototype memory is an upper bound for production-quality
implementations; `VIRTUALIZED` memory accounting; OOM behaviour depends on the WSL VM memory limit (31 GiB visible).

## 17. Platform requirements

Linux x86-64 WSL2 exclusive window; ≥ 200 GB free for inputs, outputs and spill; loop devices (root inside WSL); Windows
and macOS arms not run (informational / BLOCKED).

## 18. Estimates

Compute ≈ 250 compute-hours; disk ≈ 200 GB peak. Agent effort: prototype implementation judgment; execution none;
analysis judgment.

## 19. Expected cost tiers `[INFERRED]`

Streaming import keeping identities: T0-T1; aggregated evidence or excluded structural absence changing ConversionProvenance
records: T3 (record semantics) and never in place for version-1 bytes (HC-16); provenance byte bound: T2 (new policy field
and reason code); LFH streaming untrusted mode: T2 (new mode) plus C4 surface.
