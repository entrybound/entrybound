# EXP-PLAT-007: Ownership and identity across user namespaces, idmapped mounts, account databases and hosts

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-032, DEC-MOD-028, DEC-PLT-007
- **Program sections:** 15, 19, 21
- **Decision type (proposed, not assigned):** EMPIRICAL for stability and restore outcomes; HUMAN-FACING for any claim about what users expect cross-machine (routed to external review, §4.16). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03 (M03.1/M03.2 for ownership classes), OD-18 (M18.1 for strict-profile candidates), OD-25 (M25.4 `unsafe_defaults`); HC-05, HC-07, HC-10.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** How stable are numeric ids, user/group names and Windows SIDs across capture contexts (same host, user namespace, idmapped mount, different account databases, cross-OS), and which restore policy (ignore, numeric, map-by-name with numeric fallback, refuse) restores the intended owner exactly without an unsafe default, compared with GNU tar and bsdtar ownership modes?
- **H1:** Numeric ids are stable only within one account database and namespace; names are stable across databases but not across user namespaces and idmapped mounts (captured ids shift, names may vanish); map-by-name restores the intended owner in strictly more cross-machine scenarios than numeric restore, while numeric restore combined with retained setuid produces at least one unsafe default when restoring as root into a different database.
- **H0 / equivalence:** Numeric and name-mapped restore are EQUIVALENT across scenarios, and no policy default is unsafe.
- **Falsification of H1:** Map-by-name not DISTINGUISHABLE_BETTER on the cross-database scenarios, or zero unsafe outcomes for numeric restore with setuid across all scenarios.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-032 | V1_IMPORTANT | How is ownership represented and restored: numeric uid/gid only, plus user/group names, Windows owner/group SIDs, idmap or user-namespace context; does it live in port... | Capture stability and restore outcomes per representation and policy across S1–S7. | Docker rootless/userns-remap and NFS idmap (EXP-PLAT-024); SID capture API (EXP-PLAT-016). |
| DEC-MOD-028 | V1_IMPORTANT | What must identity/strict-v1 contain (mode bits, ownership names vs numeric ids vs SIDs, mtime precision and normalisation, hardlink grouping, xattrs, ACLs, platform m... | Stability of names, ids, SIDs and mode bits across contexts for each strict-profile content candidate (via re-projection). | Consumer requirement survey (HUMAN-FACING, no participants); cost of missing uname/gname names (C1). |
| DEC-PLT-007 | V1_BLOCKING | What is the v1 default extraction policy and the closed set of named grants for dangerous restorations: setuid/setgid/sticky and umask, privilege-bearing xattr namespa... | Ownership-restoration grant behaviour for root, non-root and user-namespace extraction, including capability rootids. | Full dangerous-restoration matrix (EXP-PLAT-008); usability of named grants (HUMAN-FACING). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** `status-quo-numeric-aux-ignore` (production default Ignore; `--restore-owner` numeric).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-032** — candidates: `status-quo-numeric-aux-ignore`, `add-names-aux`, `names-with-map-by-name-restore`, `sids-via-security-descriptor`, `idmap-context-recording`, `ownership-in-strict-identity`, `override-ids-on-pack`, `remove-ownership` (status quo: `status-quo-numeric-aux-ignore`). Treatment: Representations measured through capture stability; restore policies measured directly (status quo) or through the incumbent implementing the same rule (GNU tar default = map by name).
  - `override-ids-on-pack`: Measured through GNU tar `--owner/--group` as the reference implementation.
  - `remove-ownership`: Scored as capture fraction 0 by construction; its benefit is identity stability (re-projection).
- **DEC-MOD-028** — candidates: `spec-set`, `numeric-ids`, `names-and-ids`, `tagged-union-ownership`, `plus-xattrs-acls`, `mtime-second-precision`, `not-in-v1`, `multiple-strict-profiles`. Treatment: Evaluated by re-projection of captured facts across contexts.
- **DEC-PLT-007** — candidates: `status-quo-per-class-enums`, `python-data-filter-parity`, `bsdtar-style-p-flag`, `named-grants-closed-set`, `xattr-namespace-allow-list`, `root-aware-defaults`, `per-platform-xattr-defaults`, `status-quo-per-class-flags`, `spec-allow-list`, `named-presets`, `per-class-flags-plus-allow-for-privileged`, `python-filter-model` (status quo: `status-quo-per-class-enums`, `status-quo-per-class-flags`). Treatment: Only the ownership and setuid-with-ownership grants are measured here.
  - excluded before execution (ledger): `archive-requested-restoration` — Extraction policy is caller-owned and cannot be constructed or widened by archive content (violates I16); L0/R1(b) counterexample and reviewer sign-off still required (R1).

## Corpus, fixtures and case sets

- **Fixture:** `scripts/gen_ownership_fixture.py --seed <seed>`: files and directories with uids/gids {0, 1, 1000, 65534, 2^31, 4294967294}, with and without entries in the account database, setuid/setgid combinations, group-writable directories; tuning seed 20260917 (`plat007-fixture-tuning`), validation 20260918 (`plat007-fixture-validation`).
- **Corpus:** `f19-tuning-zoo` (varied uids/gids), `f19-validation-rsnapshot-b` (varied owners).
- **Scenarios:** S1 same host, capture and restore as root; S2 capture as root, restore as non-root; S3 capture inside `unshare --user --map-root-user` (unmapped ids appear as the overflow id); S4 capture through an idmapped bind mount (`mount --bind -o X-mount.idmap=u:1000:2000:100,g:1000:2000:100`); S5 restore inside a mount namespace whose `/etc/passwd` and `/etc/group` are bind-mounted from `alt_accountdb/{same-names-different-ids,same-ids-different-names,missing-names}`; S6 capture of an OCI layer tree unpacked with numeric ids (`f16-oci-ubuntu-noble-multiarch-index`, `f16-oci-fedora-44-zstd-layers`); S7 capture on NTFS D: (owner/group SIDs via `Get-Acl`) restored in WSL. Intended owner per scenario is written in `intended_owner.yaml` before any run.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_ids:** `wsl-ubuntu` (root; `unshare`, idmapped mounts), `windows-host` (S7 capture, non-admin). No timing.

| Requirement | Needed? |
|---|---|
| Linux (WSL2 Ubuntu 24.04, kernel 5.15.167.4, root) | yes |
| Windows 11 host (non-admin, NTFS D:) | yes |
| macOS | no (macOS counterpart is EXP-PLAT-022, BLOCKED) |
| x86-64 | yes |
| ARM64 | no (native ARM64 parity is EXP-PLAT-025, BLOCKED) |
| Network | no (downloads of pinned tools only, approved) |
| Privileged | WSL root for loop mounts; Windows non-admin only |
| Large storage | no |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Linux):** `rm -rf /root/eb-research/src/entrybound-plat && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/entrybound-plat && git -C /root/eb-research/src/entrybound-plat checkout <record-commit> && cd /root/eb-research/src/entrybound-plat && CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound` → `/root/eb-research/target/production/release/ebound` (production workspace, no `research-internals`; harness review R1-06).
- **Production CLI (Windows):** `git clone D:/Projects/entrybound/entrybound D:/eb-research/src/entrybound-plat; git -C D:/eb-research/src/entrybound-plat checkout <record-commit>; $env:CARGO_TARGET_DIR='D:/eb-research/target/production-win'; & "$env:USERPROFILE/.cargo/bin/cargo.exe" +1.98.1 build --release -p entrybound-cli --bin ebound` → `D:/eb-research/target/production-win/release/ebound.exe`.
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-007/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-007/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-007/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-007/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-007/spec.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-007/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Scripts:** `scripts/ownership_scenarios.sh --system <eb|gnutar|bsdtar|py312> --policy <p> --item <path> --item-id <id> --scratch <dir>` runs S1–S6 for the item (creating namespaces and mounts, never modifying the real `/etc/passwd`), extracts with the system's policy flags (`ebound unpack [--restore-owner]`; `tar -xpf [--numeric-owner|--no-same-owner]`; `bsdtar -xpf [--numeric-owner]`; `tarfile.extractall(filter=...)`), observes `stat -c '%u %g %U %G %a'`, and prints `restore_fidelity_fraction`, `capture_fidelity_fraction`, `unsafe_defaults`, `observation_sha256`. `scripts/sid_capture.ps1` covers S7 capture.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Capture:** recorded numeric ids and names (where the format records names) versus the source observation in the capture context.
- **Restore:** destination owner versus the pre-registered intended owner for the scenario and policy.
- **Unsafe default:** a default-policy extraction that yields a setuid/setgid object owned by a principal other than the intended one, or a privileged capability xattr effective for an unintended owner.
- **Strict-profile stability:** re-projected strict-profile fact sets (SPEC set, numeric ids, names and ids, tagged union, mtime second precision) compared across S1/S3/S4/S5 and S7.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `restore_fidelity_fraction` | OD-03 | primary per (policy, scenario) | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | script |
| `capture_fidelity_fraction` | OD-03 | primary per (representation, scenario) | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | script |
| `unsafe_defaults` | OD-25 | binary R1 screen (M25.4) | correctness | count | equal_zero | HC-05 (`metric_thresholds.unsafe_defaults`; A.2 role `binary`) | script |
| `cross_host_identity_agreement` | OD-18 | binary screen for strict-profile candidates | correctness | binary | equal_one | HC-08 (`metric_thresholds.cross_host_identity_agreement`; A.2 role `binary`) | re-projection |

## Normalization

Owners compared as (uid, gid, uname, gname) tuples against the intended tuple; SIDs as strings. Reference: status quo.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Scenario arm:** leave-one-scenario-out.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Numeric ownership is not meaningful across account databases; recording it is still useful for same-host restores — both effects are reported.
- User namespaces cannot represent unmapped ids on capture (overflow id), a declared-loss requirement.
- Windows SIDs have no POSIX mapping here; S7 restores are expected to be report-only.

## Threats to validity

- Alternate account databases are simulated by bind mounts, not by real remote machines; NSS modules (LDAP, SSSD) are not covered.
- Docker userns-remap, rootless containers and NFS idmap are not measured here (EXP-PLAT-024).
- Windows domain accounts are unavailable (local account only).

## Estimated cost and effort

- **Compute:** about 3 machine-hours; **disk:** about 6 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-007/scripts/{gen_ownership_fixture.py,ownership_scenarios.sh,alt_accountdb/,sid_capture.ps1} (mechanical); reuses `identity_reproject.py` from EXP-PLAT-005 for strict-profile candidates
- **Agent effort:** execution none (scripted); tooling scripted; judgment for the intended-owner definition per scenario (committed before runs).

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
