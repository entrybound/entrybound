# EXP-PLAT-025: Native ARM64 capture and identity parity (Linux aarch64, Windows on ARM) (BLOCKED)

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **BLOCKED** — PLATFORM_BLOCKED: requires native ARM64 hardware (a Linux aarch64 host and a Windows 11 on ARM device); only QEMU user-mode emulation is available locally (EXP-PLAT-005 EMULATED arm), which covers architecture-dependent code but not native filesystem/driver behaviour or performance. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-062, DEC-MOD-049, DEC-CON-030
- **Program sections:** 15, 19, 21
- **Decision type (proposed, not assigned):** EMPIRICAL. Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-18 (M18.1, M18.4); HC-08, HC-11, HC-13.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** On native ARM64 Linux and Windows hosts, do LAI, AUX, PCR, PCI and required feature bits for the EXP-PLAT-005 trees equal the x86-64 results on the same filesystems, and does the EMULATED QEMU arm predict every native outcome?
- **H1:** Native ARM64 identities equal x86-64 identities per filesystem, and the QEMU-user arm agreed with them (so the emulation validity check generalizes).
- **H0 / equivalence:** At least one identity or feature-bit difference appears natively that the EMULATED arm did not show.
- **Falsification of H1:** Any native ARM64 difference from x86-64 on the same filesystem.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-062 | V1_BLOCKING | How does stable v1 disposition capabilities that cannot be evaluated locally (macOS/APFS metadata, native ARM64 performance, ReFS, admin-only Windows operations): keep... | Native ARM64 evidence for the disposition of blocked capabilities and for validating the emulated-evidence candidate. | Emulated arm (EXP-PLAT-005); performance claims are out of platform-domain scope. |
| DEC-MOD-049 | V1_BLOCKING | How should identity-participating semantics that a capture platform cannot observe (e.g. POSIX executable state on NTFS) be represented so that LAI is canonical across... | Native ARM64 column of cross-host LAI stability. | x86-64 hosts (EXP-PLAT-005). |
| DEC-CON-030 | V1_BLOCKING | Which feature-tier mechanism should gate AUX-only platform metadata (e.g. platform-security-metadata-v1) so that Windows-created archives are not unreadable to readers... | Native Windows-on-ARM required feature bits for default capture. | x86-64 Windows (EXP-PLAT-005). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** EXP-PLAT-005 x86-64 results on the same filesystems.
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-062** — candidates: `status-quo-compiled-untested-plus-unavailable-declarations`, `exclude-blocked-platform-claims`, `experimental-tier-per-platform`, `declare-unavailable-until-tested`, `emulated-correctness-evidence-only`, `documentation-and-third-party-evidence`, `community-contributor-runs`, `acquire-hardware-or-admin-session` (status quo: `status-quo-compiled-untested-plus-unavailable-declarations`). Treatment: Measured natively when unblocked with the EXP-PLAT-005 treatment.
  - excluded before execution (ledger): `hosted-ci-runners` — The program owner declined hosted runners; platform experiments are local and Docker only (violates PROGRESS standing decision 2026-09-12 (no GitHub Actions or hosted runners)); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-MOD-049** — candidates: `status-quo-false`, `tri-state-unknown`, `caller-policy-default`, `exclude-from-lai`, `source-metadata-import` (status quo: `status-quo-false`). Treatment: Measured natively when unblocked with the EXP-PLAT-005 treatment.
- **DEC-CON-030** — candidates: `status-quo-incompat`, `compat-tier`, `per-record-criticality`, `capture-policy-default-off`, `profile-split` (status quo: `status-quo-incompat`). Treatment: Measured natively when unblocked with the EXP-PLAT-005 treatment.

## Corpus, fixtures and case sets

- The EXP-PLAT-005 item list and generated trees (tuning and validation only).
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_ids:** new `linux-aarch64-native` and `windows-arm64-native` captures.

| Requirement | Needed? |
|---|---|
| Linux | yes (native aarch64, BLOCKED) |
| Windows | yes (Windows 11 on ARM, BLOCKED) |
| macOS | no (Apple silicon covered by EXP-PLAT-022) |
| x86-64 | no |
| ARM64 | yes (native) |
| Network | no |
| Privileged | root on Linux for loop filesystems |
| Large storage | about 30 GB |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Run:** the EXP-PLAT-005 `spec.yaml`/`spec-windows.yaml` commands with `platform.arch: [aarch64]` (this experiment's `spec.yaml`), then `identity_compare.py` against x86-64 raw results.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

As in EXP-PLAT-005.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `cross_host_identity_agreement` | OD-18 | binary R1 screen | correctness | binary | equal_one | HC-08 (`metric_thresholds.cross_host_identity_agreement`; A.2 role `binary`) | identity comparison |
| `platform_coverage_count` | OD-18 | descriptive | other | count | report | - (`metric_thresholds.platform_coverage_count`; A.2 role `descriptive`) | analysis |

## Normalization

As in EXP-PLAT-005.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Scope rule:** decisions claiming architecture independence must either exclude native ARM64 from scope or stay `INSUFFICIENT_EVIDENCE` until this runs (§4.1 EMULATED evidence caps at SUPPORTED_WITH_LIMITATIONS, §9.1).

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Held-out evaluation (Phase D placeholder)

Same Phase D placeholder as EXP-PLAT-005, executed on native ARM64 after unlock.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Expected blocked-platform negative: no native ARM64 evidence exists in this program.

## Threats to validity

- Native hosts differ in kernel and filesystem versions from WSL; comparisons are per filesystem and version-labelled.

## Estimated cost and effort

- **Compute:** about 6 machine-hours; **disk:** about 30 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** ARM64 builds of production `ebound` (aarch64-unknown-linux-gnu, aarch64-pc-windows-msvc) at the record commit; EXP-PLAT-005 capture scripts unchanged
- **Agent effort:** blocked; when unblocked: scripted execution; judgment only if native results disagree with the EMULATED arm.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
