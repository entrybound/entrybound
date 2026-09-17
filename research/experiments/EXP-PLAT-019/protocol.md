# EXP-PLAT-019: Fidelity and extraction report census, report size under per-entry failures, and fail-closed unknown platform metadata

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **NEEDS_TOOLING**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-015, DEC-PLT-013, DEC-MOD-009, DEC-PLT-025, DEC-PLT-038, DEC-MOD-044, DEC-PLT-036, DEC-MOD-002, DEC-PLT-022
- **Program sections:** 15, 17, 18, 21, 29
- **Decision type (proposed, not assigned):** EMPIRICAL (census, bytes, reader outcomes) with FORMAL parts (schema byte models derived from the candidate definitions; expected fail-closed outcomes derived from SPEC §20 and docs/security-metadata-v1.md). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-04 (M04.2), OD-05 (M05.2 metadata bytes, T-02b per entry), OD-25 (M25.3 mechanical part); HC-04, HC-07, HC-03, HC-01, HC-17.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** What loss, refusal and restoration classes and reason strings does production actually emit, how large do FidelityReport encodings grow under per-entry capture failures for each schema candidate, and do readers fail closed (with a stable, specific reason code) on unknown Critical metadata, unknown reparse tags, ACE types and attribute bits, symlink-tagged ReparsePoint encodings, ReparsePoint descendants or content, and duplicate fidelity declarations?
- **H1:** The status-quo archive-wide free-form FidelityReport stays constant-size under per-entry failures but cannot say which entries lost a class (so `declared_gap_count` is exact only per class), bounded entry-scope candidates grow linearly until their declared cap and then summarize; readers return NONCONFORMING with specific codes for unknown bits and ACE types, but at least one noncanonical case (symlink tag encoded as ReparsePoint, or a duplicate fidelity item silently collapsed) is accepted as OK (HC-03/HC-01 finding).
- **H0 / equivalence:** All report candidates are EQUIVALENT in `metadata_bytes` within T-02, and every injected case fails closed with a specific code.
- **Falsification of H1:** No injected case accepted as OK, and no distinguishable byte difference between report candidates at 10^5 failing entries.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-015 | V1_BLOCKING | What FidelityReport schema does v1 freeze: free-form class/reason strings or closed MetadataClass and reason registries, degraded {from, to}, per-entry scope or bounde... | Catalogue of every emitted class/reason string (static and dynamic); report bytes per schema candidate at N ∈ {1, 10, 10^3, 10^5, 10^6} entries with failure fractions {0.001, 0.1, 1}; partial-capture semantics observed. | AUX determinism across hosts (EXP-PLAT-005); simulated first-use interpretation (HUMAN-FACING); independent-reader ambiguity log (conformance domain). |
| DEC-PLT-013 | V1_IMPORTANT | What structured, versioned form must the extraction-side restoration report take (per-entry reason codes, metadata class identifiers from a registry, confinement mode,... | Inventory of every loss and refusal message currently emitted by extraction, as the basis for typed issue schemas. | Usability (HUMAN-FACING); schema stability (access domain). |
| DEC-MOD-009 | V1_IMPORTANT | How is metadata criticality exercised in v1 (which names are Critical, how readers treat unknown Critical items) and is restorability a per-name constant validated by... | Fail-closed tests with constructed unknown Critical items and wire cost of per-item fields. | Which losses must fail per platform (EXP-PLAT-004). |
| DEC-PLT-025 | V1_BLOCKING | Does v1 keep a closed numeric metadata-name registry (15 names) or add SPEC's open reverse-DNS x- namespace with unknown-item carriage, criticality and safe-to-copy ru... | Interop and security behaviour of opaque unknown items (restore gating, identity participation) through injected x-namespace-like and unknown-id items. | Gap sizing (EXP-PLAT-004); precedent survey (analytical). |
| DEC-PLT-038 | V1_IMPORTANT | Which reparse tags are 'recognized' (mapped to Symlink entries) versus opaque ReparsePoints, is a recognized tag encoded as ReparsePoint noncanonical, does tag-2 data... | Acceptance of symlink-tagged ReparsePoint entries (canonical uniqueness) and GUID-header payload variants. | Real tag layouts (EXP-PLAT-013); tag enumeration (EXP-PLAT-021). |
| DEC-MOD-044 | V1_IMPORTANT | Can ReparsePoint entries have descendant Entries or carry content (directory junctions, cloud placeholders, dedup files), and is Entry-v3 tag 9 forbidden on non-Repars... | EntrySet validation outcomes for tag 9 on non-ReparsePoint kinds, ReparsePoint descendants and ReparsePoint content. | NTFS semantics survey (EXP-PLAT-013). |
| DEC-PLT-036 | V1_IMPORTANT | Are the frozen platform-security-v1 registry values correct and complete: Windows attribute mask 0x005af9a7, macOS flag mask 0x40bf80ef, NFS4 ACE4 permission mask 0x00... | Reader behaviour for unknown attribute bits, macOS flag bits and ACE types outside the frozen registries. | Header cross-check (EXP-PLAT-021). |
| DEC-MOD-002 | V1_IMPORTANT | Should AUX avoid host-dependent inputs (sparse maps from allocation state, fidelity platform/filesystem strings, Windows creation times) in deterministic mode, and sho... | Whether duplicate fidelity/conversion declarations are rejected or silently collapsed. | Host-dependent AUX (EXP-PLAT-005). |
| DEC-PLT-022 | V1_IMPORTANT | Should pack and unpack expose --metadata <class> selection, what are the default captured and restored class sets, and what are the stable CLI class names? | Consistency of class names across registry, FidelityReport vocabulary and CLI flags (census). | Overhead and churn (EXP-PLAT-004/005). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production (`status-quo-free-form-archive-wide`, `free-text-strings-status-quo`, `all-optional-per-name-status-quo`, `closed-registry-status-quo`, `undefined-status-quo`, `leaf-no-content-status-quo`, `status-quo-frozen-masks`, `status-quo-capture-all`).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-015** — candidates: `status-quo-free-form-archive-wide`, `closed-registry-archive-wide`, `closed-registry-bounded-entry-scope`, `closed-registry-per-class-counts`, `per-entry-fidelity-metadata`, `spec-10-9-full`, `attempted-versus-succeeded-split` (status quo: `status-quo-free-form-archive-wide`). Treatment: Status quo measured directly; schema and criticality candidates evaluated by byte models over observed issue lists and by injected-record reader tests.
- **DEC-PLT-013** — candidates: `free-text-strings-status-quo`, `fidelityreport-shaped-record`, `typed-issue-enum-with-registry`, `json-report-file-option`, `free-form-fidelity-strings` (status quo: `free-text-strings-status-quo`). Treatment: Status quo measured directly; schema and criticality candidates evaluated by byte models over observed issue lists and by injected-record reader tests.
- **DEC-MOD-009** — candidates: `all-optional-per-name-status-quo`, `registry-defined-critical-names`, `per-item-override`, `drop-per-item-fields`, `restorability-runtime-capability` (status quo: `all-optional-per-name-status-quo`). Treatment: Status quo measured directly; schema and criticality candidates evaluated by byte models over observed issue lists and by injected-record reader tests.
- **DEC-PLT-025** — candidates: `closed-registry-status-quo`, `spec-x-namespace-reverse-dns`, `x-namespace-plus-safe-to-copy`, `registry-only-growth`, `central-iana-style-registry`, `fill-spec-v1-membership` (status quo: `closed-registry-status-quo`). Treatment: Status quo measured directly; schema and criticality candidates evaluated by byte models over observed issue lists and by injected-record reader tests.
- **DEC-PLT-038** — candidates: `undefined-status-quo`, `symlink-only-recognized`, `symlink-and-junction-recognized`, `registry-of-tags`, `remove-known-safe-option`, `guid-included-in-data`, `guid-excluded-separate-field` (status quo: `undefined-status-quo`). Treatment: Status quo measured directly; schema and criticality candidates evaluated by byte models over observed issue lists and by injected-record reader tests.
- **DEC-MOD-044** — candidates: `leaf-no-content-status-quo`, `directory-reparse-with-children`, `data-bearing-reparse`, `map-to-directory-plus-metadata` (status quo: `leaf-no-content-status-quo`). Treatment: Status quo measured directly; schema and criticality candidates evaluated by byte models over observed issue lists and by injected-record reader tests.
- **DEC-PLT-036** — candidates: `status-quo-frozen-masks`, `expand-masks-from-current-headers`, `opaque-unknown-bits-preserved`, `refuse-with-declared-loss` (status quo: `status-quo-frozen-masks`). Treatment: Status quo measured directly; schema and criticality candidates evaluated by byte models over observed issue lists and by injected-record reader tests.
- **DEC-MOD-002** — candidates: `status-quo-capture-all`, `deterministic-mode-normalise`, `sparse-maps-excluded`, `structured-fidelity-vocabulary`, `reject-duplicates` (status quo: `status-quo-capture-all`). Treatment: Status quo measured directly; schema and criticality candidates evaluated by byte models over observed issue lists and by injected-record reader tests.
- **DEC-PLT-022** — candidates: `status-quo-capture-all-restore-per-flag`, `spec-metadata-class-flags`, `profile-based-classes`, `capture-all-restore-selection-only`, `exclude-volatile-classes-by-default` (status quo: `status-quo-capture-all-restore-per-flag`). Treatment: Status quo measured directly; schema and criticality candidates evaluated by byte models over observed issue lists and by injected-record reader tests.

## Corpus, fixtures and case sets

- **Static census:** production sources at the record commit (`crates/entrybound`, `crates/entrybound-cli`) — FidelityReport class/reason literals, ExtractionReport `metadata_not_restored` formats, `ReasonCode` variants, CLI status lines — with `path:line`.
- **Dynamic census:** every stdout/stderr and report string captured in raw outputs of EXP-PLAT-002, 004, 006–014 (tuning/validation runs).
- **Report-size trees:** `mixed_fs_tree.sh` builds ext4 trees with N entries where a fraction lives on bind-mounted vfat or ntfs-3g subtrees (so xattrs, ACLs and sparse maps are unavailable per entry), seeded 20260917 (`plat019-trees-tuning`) and 20260918 (`plat019-trees-validation`).
- **Fail-closed cases:** `failclosed_cases.py` enumerates injected records (unknown Critical item, unknown metadata name id, symlink tag as ReparsePoint, tag 9 on File/Directory/Symlink, ReparsePoint with children, ReparsePoint with ContentObject, attribute bits outside 0x005af9a7, macOS flags outside 0x40bf80ef, ACE type 0x16+, duplicate fidelity item, duplicate conversion declaration), each built on top of a canonical archive.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_id:** `wsl-ubuntu`. No timing.

| Requirement | Needed? |
|---|---|
| Linux (WSL2, root) | yes |
| Windows host | no |
| macOS | no |
| x86-64 | yes |
| ARM64 | no |
| Network | no |
| Privileged | root for bind mounts |
| Large storage | no |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Linux):** `rm -rf /root/eb-research/src/entrybound-plat && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/entrybound-plat && git -C /root/eb-research/src/entrybound-plat checkout <record-commit> && cd /root/eb-research/src/entrybound-plat && CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound` → `/root/eb-research/target/production/release/ebound` (production workspace, no `research-internals`; harness review R1-06).
- **Harness (only where named):** `wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh build --release` (→ `/root/eb-research/target/harness/release/`), after `C:/Python313/python.exe research/harness/lock_parity.py` reports `RESULT PASS` (harness review R1-06).
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-019/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-019/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-019/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-019/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-019/spec.yaml'`
- **Scripts:** `scripts/emission_census.py --src <clone> --out research/normalized/EXP-PLAT-019/decision/emission-catalogue.csv`; `scripts/dynamic_strings.py --raw research/raw`; `scripts/report_arms.sh --arm <size|failclosed> --from <item_id> --scratch <dir>` (size arm: `ebr-pack` accounting and `report_schema_models.py`; fail-closed arm: forge each case, then `ebound verify`, `inspect --json`, `unpack`, `repack`, recording outcome class and reason code).

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Census:** unique strings, their class vocabulary, and dynamic coverage of the static catalogue.
- **Report size:** exact FidelityReport bytes (status quo) and modelled bytes per candidate; per-entry bytes.
- **Fail-closed:** outcome class and reason code per case and command; accepted-as-OK counts as `hc04_fail_open_outcomes` (unknown critical) or a canonicality finding (noncanonical encodings).

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `metadata_bytes` | OD-05 | diagnostic; primary for DEC-PLT-015 schema candidates (the decision encodes this component) | size | B | minimize | T-02 (`metric_thresholds.metadata_bytes`; A.2 role `diagnostic`) | `ebr-pack` and models |
| `metadata_bytes_per_entry` | OD-05 | banded (T-02b) secondary guard | size | B/entry | minimize | T-02b (`metric_thresholds.metadata_bytes_per_entry`; A.2 role `banded`) | `ebr-pack` and models |
| `reason_code_specificity_fraction` | OD-04 | primary for fail-closed cases (M04.2) | other | fraction | maximize | T-20 (`metric_thresholds.reason_code_specificity_fraction`; A.2 role `banded`) | fail-closed arm |
| `hc04_fail_open_outcomes` | OD-04 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-04 | fail-closed arm |
| `error_actionability_fraction` | OD-25 | primary for the mechanical message census (M25.3) | other | fraction | maximize | T-20 (`metric_thresholds.error_actionability_fraction`; A.2 role `banded`) | census |
| `declared_gap_count` | OD-03 | descriptive | other | count | report | - (`metric_thresholds.declared_gap_count`; A.2 role `descriptive`) | census |

## Normalization

Bytes exact; outcomes as (class, reason code). Reference: status quo.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Size arm:** reported per N and failure fraction; no pooling across N.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- If every injected case already fails closed with a specific code, the fail-closed candidates differ only in cost — a null result worth recording.
- A silent collapse of duplicate fidelity items is a canonicality defect relative to HC-03, recorded as a work item.

## Threats to validity

- The forge could encode differently from a future writer; only canonical-base-plus-single-mutation cases are used, and the forge is validated byte-identical on canonical inputs.
- Static census by string literals can miss formatted messages; the dynamic census compensates partially.

## Estimated cost and effort

- **Compute:** about 4 machine-hours; **disk:** about 12 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research encoder for raw record injection (default-off `research-internals` constructor or a harness `ebr-forge` binary writing ECF records directly), proven byte-identical to production for canonical inputs before use (objective §2.0 rule 9); research/experiments/EXP-PLAT-019/scripts/{emission_census.py,dynamic_strings.py,mixed_fs_tree.sh,report_schema_models.py,failclosed_cases.py,run_failclosed.sh}; `ebr-pack` section accounting (exists) and `ebr-inspect-meta` (EXP-PLAT-004)
- **Agent effort:** tooling judgment (the forge must not diverge from production encoding; review required), then scripted; analysis judgment for schema-candidate cost interpretation.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
