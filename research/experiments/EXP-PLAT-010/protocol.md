# EXP-PLAT-010: Sparse layout capture accuracy, restore allocation, map overhead and GNU sparse projection

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-045, DEC-MOD-026, DEC-MOD-002, DEC-LEG-020
- **Program sections:** 15, 21, 25
- **Decision type (proposed, not assigned):** EMPIRICAL (accuracy, allocation, bytes); FORMAL for the map-byte formula from docs/posix-metadata-v1.md SparseMapV1, validated against measured accounting. Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03 (M03.1, M03.2), OD-05 (M05.1 artifact bytes guard, M05.2 metadata bytes diagnostic), OD-17 (M17.3 for the extent bound); HC-07, HC-02, HC-06, HC-13.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** How accurately do SEEK_DATA/SEEK_HOLE and NTFS allocated-range queries report sparse layout on each filesystem, what does recording sparse maps for dense files cost, do real fragmented images approach the 1,000,000-extent bound, what allocation results from Logical versus Restore policies on each target, and can GNU tar/pax sparse members be projected into native sparse maps exactly?
- **H1:** ext4, XFS and btrfs report written-extent layouts exactly after cache drop, tmpfs reports them in page granularity, vfat reports none; Restore reproduces allocation within one filesystem block per extent on ext4/XFS/btrfs while Logical allocates the full size; maps on dense files cost a measurable `metadata_bytes` difference on many-small-file items (F04) that exceeds the T-02 floor; no real item exceeds 1,000,000 extents; GNU sparse 1.0 maps project exactly onto block-aligned native maps while 0.0/0.1 need re-alignment, and the status quo imports GNU sparse members as regular files without a declaration.
- **H0 / equivalence:** Map overhead on dense files is EQUIVALENT to zero within T-02, and every GNU sparse variant projects exactly.
- **Falsification of H1:** No distinguishable `metadata_bytes` difference between status quo and the omit-dense-map model on F04 items, or a real item above 1,000,000 extents, or status-quo import declaring GNU sparse keys.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-045 | V1_IMPORTANT | For v1 sparse handling: is the 1,000,000-extent bound justified, should dense files carry a sparse map, should non-Linux capture (macOS SEEK_HOLE, Windows allocated-ra... | Extent counts versus bound; dense-map metadata overhead (measured natural experiment plus validated formula); SEEK_HOLE accuracy per filesystem; st_blocks after Restore versus Logical; NTFS allocated ranges on D:. | macOS SEEK_HOLE (EXP-PLAT-022); safe Windows/macOS APIs (EXP-PLAT-016). |
| DEC-MOD-026 | V1_IMPORTANT | Should holes be represented by a registered hole TransformPlan (SPEC), by ordinary zero-filled chunks plus AUX sparse-map metadata (repo), or by another scheme? | Extraction hole fidelity per platform for the status-quo representation. | Size/time/hash cost of hole-plan candidates (container/compression domains, research encoder). |
| DEC-MOD-002 | V1_IMPORTANT | Should AUX avoid host-dependent inputs (sparse maps from allocation state, fidelity platform/filesystem strings, Windows creation times) in deterministic mode, and sho... | AUX difference attributable to sparse maps captured from allocation state after `cp --sparse=always/never` and `fallocate --dig-holes`. | Cross-host AUX (EXP-PLAT-005). |
| DEC-LEG-020 | V1_BLOCKING | Must strict tar import refuse members carrying any GNU.sparse.* pax keys (formats 0.0, 0.1, 1.0) until sparse projection is frozen, or project them into native sparse... | Fixtures from GNU tar `--sparse` pax 0.0/0.1/1.0 and bsdtar; exactness of projection to native maps; status-quo import outcome. | Import policy decision (legacy domain). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production (`status-quo`, `zero-chunks-plus-aux-map-status-quo`, `status-quo-capture-all`, `status-quo-unknown-key` (ledger-excluded; measured as the defect baseline)).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-045** — candidates: `status-quo`, `omit-map-for-dense-files`, `lower-or-budgeted-extent-bound`, `add-windows-allocated-ranges`, `add-macos-seek-hole`, `restore-sparse-by-default`, `drop-sparse-v1` (status quo: `status-quo`). Treatment: Status quo measured; `omit-map-for-dense-files` measured by the vfat natural experiment and the validated byte model; bound candidates scored against measured extent counts; Windows/macOS capture candidates scored for feasibility.
- **DEC-MOD-026** — candidates: `zero-chunks-plus-aux-map-status-quo`, `hole-transform-plan`, `hole-plan-plus-aux-hint`, `dedup-single-zero-chunk` (status quo: `zero-chunks-plus-aux-map-status-quo`). Treatment: Only the status-quo representation's extraction hole fidelity is measured.
- **DEC-MOD-002** — candidates: `status-quo-capture-all`, `deterministic-mode-normalise`, `sparse-maps-excluded`, `structured-fidelity-vocabulary`, `reject-duplicates` (status quo: `status-quo-capture-all`). Treatment: Sparse-map host dependence measured for status quo and the omit/deterministic-normalise candidates by model.
- **DEC-LEG-020** — candidates: `refuse-gnu-sparse-keys`, `project-pax-1.0-only`, `project-all-variants` (status quo: `status-quo-unknown-key`). Treatment: Projection candidates evaluated by `gnu_sparse_project.py` against source extents; status quo measured directly.
  - excluded before execution (ledger): `status-quo-unknown-key` — Produces silent wrong content contrary to the documented refusal (violates docs/tar-import-v1.md L45-46; I13); L0/R1(b) counterexample and reviewer sign-off still required (R1).

## Corpus, fixtures and case sets

- **Items:** `f15-tuning-dbprealloc`, `f15-tuning-vmraw`, `f15-tuning-real-ubuntu-noble-ova-raw`, `f15-validation-edgecases`, `f15-validation-fragmented`, `f15-validation-real-debian-nocloud-raw`, `f16-alpine-3241-minirootfs-ext4-raw`, `f20-validation-sparsedup`, `f04-tuning-sourcelike`, `f04-validation-objstore` (F04 items are the dense-file controls).
- **Legacy fixtures:** GNU tar 1.35 `--sparse --sparse-version=0.0|0.1|1.0 --format=pax` and `bsdtar` archives of files from `f15-validation-edgecases`, `f15-validation-fragmented` and `f15-tuning-dbprealloc` (validation files for validation, tuning file for tuning).
- **Extent ground truth:** generator manifests of the sparse_layouts.py items plus SEEK_DATA after `posix_fadvise(DONTNEED)` on the source (the fingerprint page-cache fix).
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **Targets:** `ext4-restore`, `ext4-logical`, `xfs-restore`, `xfs-logical`, `btrfs-restore`, `btrfs-logical`, `tmpfs-restore`, `vfat-logical`, `ntfs3g-restore`; Windows `ntfs-d-restore`, `ntfs-d-logical` (`fsutil sparse queryrange`).
- **env_ids:** `wsl-ubuntu`, `windows-host`. No timing.

| Requirement | Needed? |
|---|---|
| Linux (WSL2, root) | yes |
| Windows host (non-admin) | yes |
| macOS | no (EXP-PLAT-022) |
| x86-64 | yes |
| ARM64 | no |
| Network | no |
| Privileged | WSL root for loop mounts |
| Large storage | about 60 GB transient (large-tier images copied per target) |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Linux):** `rm -rf /root/eb-research/src/entrybound-plat && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/entrybound-plat && git -C /root/eb-research/src/entrybound-plat checkout <record-commit> && cd /root/eb-research/src/entrybound-plat && CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound` → `/root/eb-research/target/production/release/ebound` (production workspace, no `research-internals`; harness review R1-06).
- **Production CLI (Windows):** `git clone D:/Projects/entrybound/entrybound D:/eb-research/src/entrybound-plat; git -C D:/eb-research/src/entrybound-plat checkout <record-commit>; $env:CARGO_TARGET_DIR='D:/eb-research/target/production-win'; & "$env:USERPROFILE/.cargo/bin/cargo.exe" +1.98.1 build --release -p entrybound-cli --bin ebound` → `D:/eb-research/target/production-win/release/ebound.exe`.
- **Harness (only where named):** `wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh build --release` (→ `/root/eb-research/target/harness/release/`), after `C:/Python313/python.exe research/harness/lock_parity.py` reports `RESULT PASS` (harness review R1-06).
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-010/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-010/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-010/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-010/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-010/spec.yaml'`
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-010/spec-legacy.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-010/spec-legacy.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-010/spec-legacy.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-010/spec-legacy.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-010/spec-legacy.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-010/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Scripts:** `scripts/sparse_arms.sh --target <t> --policy <restore|logical> --item <path> --item-id <id> --scratch <dir>`: copy with `cp --sparse=always` to the target, record `extent_truth.py` extents and `st_blocks`, `ebr-pack --item-path <copy> --profile balanced --layout indexed` (section accounting: `metadata_bytes`, `side_data_bytes`, `artifact_bytes`), `ebound inspect --json` sparse maps, `ebound unpack --sparse <policy>` to a fresh target directory, observe extents, `st_blocks` and content hash. `scripts/map_bytes_model.py` computes SparseMapV1 bytes for dense files and is validated against the vfat natural experiment (maps omitted where SEEK_HOLE is unsupported). `scripts/gnu_sparse_project.py` parses GNU sparse maps and projects them. `scripts/sparse_windows.ps1` covers D:.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Accuracy:** per file, reported extents versus ground truth (exact, block-rounded, coarser, missing).
- **Capture:** archived sparse map versus source extents.
- **Restore:** destination extents and allocated bytes versus archived map; content equality always.
- **Overhead:** exact section bytes from `ebr-pack`; dense-map bytes from the validated model.
- **Bounds:** maximum extents per file versus 1,000,000.
- **Projection:** projected map equals source extents (exact, block-aligned equal, unequal).

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `capture_fidelity_fraction` | OD-03 | primary for sparse-layout capture per filesystem | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | script |
| `restore_fidelity_fraction` | OD-03 | primary for allocation-exact restore per (target, policy) | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | script |
| `metadata_bytes` | OD-05 | diagnostic; primary for DEC-PLT-045 dense-map candidates (the decision encodes this component) | size | B | minimize | T-02 (`metric_thresholds.metadata_bytes`; A.2 role `diagnostic`) | `ebr-pack` accounting and model |
| `side_data_bytes` | OD-05 | diagnostic | size | B | minimize | T-02 (`metric_thresholds.side_data_bytes`; A.2 role `diagnostic`) | `ebr-pack` accounting |
| `artifact_bytes` | OD-05 | guard (T-01) | size | B | minimize | T-01 (`metric_thresholds.artifact_bytes`; A.2 role `banded`) | `ebr-pack` |
| `benign_false_refusal_fraction` | OD-17 | primary for extent-bound candidates (declared bound refusals on benign items) | other | fraction | minimize | T-20 (`metric_thresholds.benign_false_refusal_fraction`; A.2 role `banded`) | extent counts |
| `roundtrip_mismatches` | OD-02 | binary gating | correctness | count | equal_zero | HC-02 (`metric_thresholds.roundtrip_mismatches`; A.2 role `binary`) | content hash |
| `hc07_mvt07a_undeclared_differences` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-07 MVT-07(a) | status-quo import of GNU sparse members |
| `scale_limits` | OD-17 | descriptive (maximum extents) | other | count; B | report | - (`metric_thresholds.scale_limits`; A.2 role `descriptive`) | script |

## Normalization

Extents normalized to (offset, length) in bytes; allocation in bytes; byte metrics exact. Reference: status quo on ext4.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Byte metrics** follow T-01/T-02 semantics with tier-specific floors (§3.2); item-level values are exact, aggregated L2–L4 across F04/F15/F16/F20 groups where §4.9 support holds, otherwise family-scoped.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Byte-weighted arm (§6.5)** for metadata bytes.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Unwritten (fallocated) extents are reported inconsistently by SEEK_DATA depending on page-cache state (generator note); items avoid fallocate, but `fallocate --dig-holes` copies may still show the effect and are recorded.
- tmpfs and ntfs-3g may not preserve holes on restore; that limits Restore's guarantee per target.

## Threats to validity

- WSL ext4 on a VHDX: host-side sparseness of the VHDX does not affect guest extent reports but can affect cache behaviour.
- Large items are copied per target; copy tools themselves can alter layout, which is why extents are measured on each copy, not assumed.

## Estimated cost and effort

- **Compute:** about 10 machine-hours; **disk:** about 60 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-010/scripts/{sparse_arms.sh,extent_truth.py,gnu_sparse_project.py,sparse_windows.ps1,map_bytes_model.py} (mechanical); harness `ebr-pack` section accounting (exists)
- **Agent effort:** execution none (scripted); tooling scripted; judgment only for the dense-map byte model check.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
