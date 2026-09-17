# EXP-SCALE-002: Legacy import, export and publish at scale, and size-boundary correctness (status quo)

| Field | Value |
|---|---|
| Status | **READY** (design draft until gate G-A; see EXP-SCALE-001 "Pre-registration" for the gate and exposure disclosure, which apply verbatim) |
| Domain | scale (program §13; interfaces with legacy §25-§27) |
| Platforms | Linux x86-64 (WSL2 Ubuntu 24.04, root, cgroup-v1, loop mounts); large storage |
| Requires timing | false |
| Compute estimate | about 30 machine-hours |
| Disk estimate | about 150 GB peak (16 GiB cells: input, five legacy sources, one native archive, one extraction, spill) |
| Agent effort | scripted |
| Tooling to build | none beyond EXP-SCALE-001's driver (`research/tools/scale/scale_cell.py`, same contract) and `research/tools/scale/legacy_counts.py` (parses conflict, resolution and provenance-byte counts from `ebound inspect --json --provenance`) |

## Pre-registration

- **decision_ids:** DEC-ACC-013, DEC-ACC-017, DEC-ACC-028, DEC-LEG-002, DEC-LEG-019, DEC-LEG-021, DEC-LEG-022, DEC-LEG-030, DEC-LEG-059, DEC-LEG-108.
- **Decision type:** EMPIRICAL (memory, refusal and boundary outcomes). Assignment pending (§14 item 12).
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-08, OD-17, OD-19 (M19.1 import acceptance on legitimate large inputs); HC-06, HC-07 (declared conversion loss), HC-02 (round trip).
- **Method, gate, author, L7 exposure:** as EXP-SCALE-001. Inputs here are generated; no corpus family is applicable.

## Question

How do peak memory, scratch and completion under a memory cap scale with input size and entry count for legacy import (tar, tar.gz, tar.zst, tar.xz, ZIP/ZIP64, 7z solid, single-member gzip), legacy export (ZIP, tar/pax, tar.gz, tar.zst, file and stdout sinks) and multi-target publish, and does the status quo handle legitimate inputs at the format size boundaries (gzip ISIZE modulo 2^32, 65,535 ZIP entries, 4 GiB ZIP64 members, 8 GiB tar members, 7z dictionary and folder limits, tar observation caps) without false refusal or silent corruption?

## Hypotheses

- **H1 (memory, `[INFERRED]` from `export.rs`, `migration.rs`, `tar.rs`, `stream.rs`, `zip.rs`, `sevenz.rs`):** every import and export path and publish is PLAINTEXT-PROPORTIONAL (EXP-SCALE-001 classification rule), with multiple plaintext copies (slope k ≥ 1.5 B/B for export with strict re-import).
- **H1 (boundaries):** (a) single-member gzip import of 2^32 and 2^32 + 1 plaintext bytes is falsely refused as corrupt (saturating ISIZE comparison, DEC-LEG-021) while 2^32 − 1 succeeds; (b) tar import with 500,000 short-name entries is refused by the 8,000,000 observation cap although the 1,000,000 entry cap admits it (DEC-LEG-059); (c) 7z with a 1,536 MiB dictionary (`-mx=9 -md=1536m`) is refused by the 1 GiB dictionary cap (DEC-LEG-022 ultra presets); (d) export of a 1 GiB zero file to ZIP or tar.gz fails strict re-import on the 1000:1 wrapper expansion cap (DEC-LEG-019).
- **H0:** each boundary case completes with byte-exact round trip and each operation is BOUNDED.
- **Falsification:** any boundary outcome differing from (a)-(d), or any classification differing from H1, is reported. Each H1(a)-(d) result is a binary correctness outcome; one reproducible artifact suffices (objective §2.0 rule 2).

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-ACC-013 | Time (descriptive), peak memory and temp disk for multi-GiB exports to ZIP64 and tar.zst and full publish runs; overhead attributable to strict re-import (export vs export `--dry-run` difference) | streaming export prototypes: EXP-SCALE-009; crash atomicity of publish: EXP-SCALE-004 |
| DEC-ACC-017 | Peak RSS vs input size for 1/4/16 GiB tars (bare, gz, zst, xz), ZIP64 and solid 7z | LOM heap bytes per entry and streamed import equivalence: EXP-SCALE-009 |
| DEC-ACC-028 | Converter rows of the operation inventory | — |
| DEC-LEG-002 | Import of 100k and 999k-entry ZIPs: conflicts, provenance bytes, memory, descriptive time | aggregated-resolution prototype: EXP-SCALE-009; auditor needs: human participants (not measurable) |
| DEC-LEG-019 | Stress exports (1 GiB zero file, 5 GiB member, 999k entries) per profile target; memory and time of re-import at scale | source-derived-limit prototype: EXP-SCALE-009 |
| DEC-LEG-021 | Regression cases at 2^32 − 1, 2^32, 2^32 + 1 and 5 GiB; tar.gz export and re-import above 4 GiB | — |
| DEC-LEG-022 | Peak RSS and refusal per limit at the status-quo values on generated tails (entries, member size, dictionary, zstd window, folder size, ratio) | corpus percentiles and candidate defaults: EXP-SCALE-003; bomb corpus: legacy domain |
| DEC-LEG-030 | Status-quo behaviour and memory of `convert -` from stdin (refusal expected) | buffered and spooled stdin prototypes: EXP-SCALE-009; workflow survey and first-use study: not measurable here |
| DEC-LEG-059 | Synthetic tars with 500k and 999k entries with and without per-entry pax records under default policy; provenance growth | 100k-file 7z with unknown FilesInfo properties needs a format-building generator: EXP-SCALE-009 |
| DEC-LEG-108 | Peak memory, temp disk and wall (descriptive) of the status-quo in-memory ZIP export to files and pipes, including more than 65,535 entries | per-entry-buffered, temp-staged, always-ZIP64 prototypes: EXP-SCALE-009; reader acceptance matrix: legacy domain |

## Candidates (operations; status quo only)

`EB` = production `ebound` built as in EXP-SCALE-001. `A.*` are prerequisites built per cell by the driver outside the measured region from the cell's generated input tree `IN`, with deterministic incumbent settings: `tar -C IN --format=pax --sort=name --mtime=@946684800 --owner=0 --group=0 --numeric-owner -cf A.tar .`; `gzip -6 -n`, `zstd -T1 -3 -q` (or the cell's `--long` level), `xz -T1 -6`; `zip -q -r -X A.zip .` run in `IN` (Info-ZIP 3.0, ZIP64 automatic); `7z a -t7z -mx=5 -ms=on A.7z ./*` run in `IN` (or the cell's dictionary flags); native `A.eb` = `EB pack IN A.eb --profile balanced`.

| candidate | Command under `memcap_run.sh` |
|---|---|
| `import-tar` | `EB convert A.tar DST/out.eb --from tar` |
| `import-tar-gz` | `EB convert A.tar.gz DST/out.eb --from tar.gz` |
| `import-tar-zst` | `EB convert A.tar.zst DST/out.eb --from tar.zst` |
| `import-tar-xz` | `EB convert A.tar.xz DST/out.eb --from tar.xz` |
| `import-zip` | `EB convert A.zip DST/out.eb --from zip` |
| `import-7z` | `EB convert A.7z DST/out.eb --from 7z` |
| `import-gzip-member` | `EB convert A.gz DST/out.eb --from gzip --entry-name payload.bin` (gzip boundary cells only) |
| `import-tar-stdin` | `EB convert - DST/out.eb --from tar` with stdin = A.tar |
| `export-zip-file` | `EB convert A.eb DST/out.zip --to zip --allow-lossy --receipt DST/receipt.json` |
| `export-zip-stdout` | `EB convert A.eb - --to zip --allow-lossy`, stdout to `DST/out.zip` |
| `export-tar-file` | `EB convert A.eb DST/out.tar --to tar --allow-lossy` |
| `export-tar-gz-file` | `EB convert A.eb DST/out.tar.gz --to tar.gz --allow-lossy` |
| `export-tar-zst-file` | `EB convert A.eb DST/out.tar.zst --to tar.zst --allow-lossy` |
| `export-tar-zst-stdout` | `EB convert A.eb - --to tar.zst --allow-lossy`, stdout to `DST/out.tar.zst` |
| `export-zip-dryrun` | `EB convert A.eb DST/out.zip --to zip --allow-lossy --dry-run` (subtracts preparation from strict re-import overhead) |
| `publish-three-targets` | `EB publish A.eb --output-dir DST/pub --native --target zip --target tar.zst --base-name release --allow-lossy --report DST/pub/report.json` |
| `inc-bsdtar-zip-create` | `bsdtar -C IN --format zip -cf DST/out.zip .` (REF-INC converter memory reference) |
| `inc-7z-extract` | `7z x -y -oDST/tree A.7z` (REF-INC) |

- Target names follow `crates/entrybound/src/legacy/export.rs` (`zip/portable-v1`, `tar/pax-v1`, `tar.gz/pax-v1`, `tar.zst/pax-v1`); the driver records the resolved profile id from the receipt.
- **Excluded:** `sidecar` (no §13 decision asks for it); `--preserve` import (exact-source bytes; its 4 GiB source cap is probed only if a legacy-domain decision requests it); encrypted export (EXP-SCALE-010).

## Corpus selectors (generated)

Manifest `research/experiments/EXP-SCALE-002/cells.jsonl` (rows pointing at `cells.json`, generated by `make_cells.py`). Scaling cells reuse EXP-SCALE-001 shapes and seeds (identical trees: `b1g`, `b4g`, `b16g`, `n1e5`, `n999k`); boundary cells:

| Cell | Input | Applicable operations | Expected status-quo outcome (H1) |
|---|---|---|---|
| `gz-2p32m1` | one file of 2^32 − 1 bytes, half content, seed 1401 | import-gzip-member | OK |
| `gz-2p32` | 2^32 bytes, seed 1402 | import-gzip-member | false CORRUPT (DEC-LEG-021) |
| `gz-2p32p1` | 2^32 + 1 bytes, seed 1403 | import-gzip-member | false CORRUPT |
| `gz-5g` | 5 GiB, seed 1404 | import-gzip-member, export-tar-gz-file | import refused or mismatched; export round trip recorded |
| `zip-65535` | entries shape, 65,535 files of 64 B, seed 1411 | import-zip, export-zip-file | OK (classic ZIP) |
| `zip-65536` | 65,536 files of 64 B, seed 1412 | import-zip, export-zip-file, export-zip-stdout | OK with ZIP64 end records |
| `zip64-member` | one file of 2^32 + 1 bytes, seed 1413 | import-zip, export-zip-file | OK with ZIP64 extra field |
| `tar-8g-member` | one file of 2^33 + 1 bytes, seed 1414 | import-tar, export-tar-file | OK via pax size record (GNU base-256 also generated as `A-gnu.tar` with `--format=gnu`) |
| `zero-1g` | one file of 1 GiB zeros (`--content zeros`), seed 1415 | export-zip-file, export-tar-gz-file | strict re-import refused by 1000:1 wrapper ratio (DEC-LEG-019) |
| `member-5g` | one file of 5 GiB, half content, seed 1416 | export-tar-zst-file, export-zip-file | strict re-import refused by the 4 GiB per-entry import cap |
| `tar-500k` | 500,000 files of 64 B, names under 100 bytes, seed 1421 | import-tar | POLICY_REFUSED by the observation cap (DEC-LEG-059) |
| `tar-500k-pax` | 500,000 files whose relative paths exceed 100 bytes (pax path records), seed 1422 | import-tar | POLICY_REFUSED |
| `tar-999k` | 999,000 files of 64 B, seed 1423 | import-tar, import-zip, export-zip-file, export-tar-file | refused by the observation cap on import; export outcome recorded |
| `7z-solid-4g` | b4g tree, `7z -ms=on -mx=5` (one 4 GiB solid folder) | import-7z, inc-7z-extract | boundary of the 4 GiB folder cap (outcome recorded) |
| `7z-solid-16g` | b16g tree, solid | import-7z | POLICY_REFUSED (folder cap) |
| `7z-ultra-1536m` | b1g tree, `7z -mx=9 -md=1536m -ms=on` | import-7z | POLICY_REFUSED (1 GiB dictionary cap, DEC-LEG-022) |
| `zst-long31` | b4g tar compressed with `zstd --long=31 -19 -T1` | import-tar-zst | POLICY_REFUSED by 512 MiB decoder memory, or OK (recorded) |

- Caps: scaling cells use `cap24` and `cap2` as EXP-SCALE-001 (`b1g` cap24 only); boundary cells use `cap24` only (they test correctness, not beyond-RAM completion).
- **Held-out placeholder (Phase D):** none (generated). Real legacy archives from corpus family F14 are measured in EXP-SCALE-003 (tuning/validation); held-out F14 evaluation of any selected import-limit defaults is designed by EXP-SCALE-003's Phase D placeholder.

## Environment

As EXP-SCALE-001 (cgroup-v1 caps via `memcap_run.sh`, per-cell `tmp`/`dst` loop mounts, `TMPDIR` on `tmp`, hot cache, disk guard and `fstrim` between cells). Incumbent tool versions are those pinned in `research/baselines/tool-versions.json` (GNU tar 1.35, gzip 1.12, zstd 1.5.5, xz 5.4.5, Info-ZIP 3.0, 7-Zip 23.01, bsdtar 3.7.2); the driver records `--version` output per run.

## Commands

As EXP-SCALE-001 with `research/experiments/EXP-SCALE-002/spec.yaml`; `python research/experiments/EXP-SCALE-002/make_cells.py` regenerates the cell tables. The driver additionally runs, after every import, `EB verify DST/out.eb` and `EB unpack DST/out.eb DST/check` (uncapped, unmeasured) followed by `synth_tree.py digest` comparison (content round trip), and after every export, a re-extraction with the incumbent (`bsdtar -xf`, `tar -xf`, `7z x`) and digest comparison; then `legacy_counts.py DST/out.eb` for conflict, resolution and provenance-byte counts.

## Seeds

`seeds.order` 1302001, `seeds.bootstrap` 1302002, `seeds.command` 1302003; generator seeds in the cell table.

## Warmup

0 (as EXP-SCALE-001).

## Measurement method and metrics

EXP-SCALE-001's payload fields and canonical names, plus:

| Payload key | Canonical name | Role | Family | Source |
|---|---|---|---|---|
| `side_data_bytes` | `side_data_bytes` (T-02, diagnostic) | diagnostic (provenance and conversion-record bytes) | size | `legacy_counts.py` from `inspect --json --provenance` |
| `conflict_count`, `resolution_count`, `observation_count` | none (descriptive counts) | descriptive | other | same |
| `import_ok` | `import_acceptance_fraction` (T-20) per boundary case, aggregated over the fixed case list above | primary for the boundary case list | other | outcome = OK and content round trip |
| `export_reimport_ok` | none (binary per case) | binary | correctness | strict re-import outcome reported by the receipt, plus incumbent re-extraction digest |

`import_acceptance_fraction` is computed over the pre-registered boundary list only (n = 17 cases; band `0.5/n` per T-20), as the status-quo row that candidate defaults and prototypes are later compared against.

## Replication and adaptive rule

3 repetitions per (operation, cell) (M08.3 count; repeat-hash of native outputs, which must be deterministic, and of export bytes, which are deterministic functions of EAM plus target profile per HC-13). No stage 2: no converter is expected to be bounded; a 64 GiB converter probe is added only if the EXP-SCALE-001 classification rule labels a converter BOUNDED at 16 GiB, in which case `select_stage2.py --experiment EXP-SCALE-002` generates and commits `spec-stage2.yaml` under the same rule.

## Normalization and statistical analysis

- Scaling classification and beyond-RAM completion: EXP-SCALE-001 rule, per converter, axes logical bytes (`b1g`, `b4g`, `b16g`) and entries (`n1e5`, `n999k`).
- Strict re-import overhead (DEC-ACC-013): per cell, `peak_rss_bytes(export-*-file) − peak_rss_bytes(export-zip-dryrun)` and the same for `op_cpu_s` (descriptive).
- Provenance growth (DEC-LEG-002, DEC-LEG-059): `side_data_bytes` and counts per entry on `n1e5`, `n999k`, `zip-65536`, `tar-500k*`; linear fit per entry reported.
- Boundary outcomes: a table of (case, expected H1 outcome, observed outcome and reason code, round trip) with every mismatch and every silent corruption (outcome OK but digest mismatch, an HC-02/HC-07 violation) as a committed artifact.
- No banded comparison and no selection is made in this experiment.

## Practical-significance thresholds

Referenced only: objective M08.3 tolerance (classification); T-20 for `import_acceptance_fraction`; T-06 for later graded memory comparisons in EXP-SCALE-009.

## Sensitivity analysis

- tar format arm (`pax` vs `gnu`) on `tar-8g-member` and `tar-999k`.
- Compression level arm on `zst-long31` (level 19 with `--long=31` vs level 3 without).
- `cgroup_max_usage_bytes` in place of `peak_rss_bytes` (as EXP-SCALE-001).

## Expected negative results worth recording

False corrupt refusal of legitimate gzip members at or above 4 GiB; observation-cap refusal of legitimate 500k-entry tars; refusal of 7-Zip ultra archives; strict re-import refusing exports the product itself produced (a self-inconsistency); export memory several times the archive's plaintext; OOM kills instead of typed refusals under the 2 GiB cap.

## Threats to validity

EXP-SCALE-001 threats 1-4 and 6 apply. Additionally: (1) incumbent-generated sources depend on incumbent versions and flags, recorded per run; (2) synthetic trees lack legacy metadata variety (ACL/xattr/pax extended records), so observation counts are lower bounds for real archives; F14 real archives are measured in EXP-SCALE-003; (3) boundary cases are a pre-registered enumeration and claim nothing beyond it.

## Platform requirements

Linux x86-64 WSL2, root (cgroups, loops), about 150 GB free, incumbent tools installed per `research/baselines/tool-versions.json`. No network, no Windows arm (legacy converters are platform-independent code; Windows memory scaling of converters is a documented gap).

## Estimated compute and effort

About 30 h (the 16 GiB cells dominate: five sources times two caps times three repetitions); disk about 150 GB; scripted execution; judgment for the boundary table only.

## Deviations (append-only)

None.
