# EXP-PLAT-020: Private-key file permission checks: incumbent behaviour, secret-provisioning modes and Windows default ACLs

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-CRY-108, DEC-PLT-042
- **Program sections:** 21, 22.4, 33
- **Decision type (proposed, not assigned):** EMPIRICAL (tool behaviour, observed modes and ACLs) with HUMAN-FACING parts (warning comprehension routed to external review, §4.16) and EXTERNALLY_SOURCED provisioning defaults re-verified locally where reproducible. Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-25 (M25.4 `unsafe_defaults`, M25.3 `error_actionability_fraction`), OD-15 context; HC-05.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** When private identity or signing-key files are group/world-readable, how do incumbent tools behave (OpenSSH, GnuPG, age, minisign, restic on Linux; Windows OpenSSH), which legitimate secret-provisioning mechanisms present such modes (so that a refuse-by-default policy would refuse them), and which principals can read a key file newly created by `ebound key generate-signing` on the Windows host?
- **H1:** OpenSSH refuses group/world-readable keys on Linux and keys readable by other principals on Windows, GnuPG warns about unsafe home permissions, age and minisign do not check; vfat, drvfs-without-metadata and documented Kubernetes secret volumes present keys as readable by others, so a refuse-without-override policy refuses at least those workflow cases; a key created under D:\eb-research inherits ACEs granting read to principals other than the owner (for example Authenticated Users or Users).
- **H0 / equivalence:** All incumbents behave identically and no legitimate workflow presents keys readable by other principals.
- **Falsification of H1:** OpenSSH accepts world-readable keys, or no provisioning case yields other-readable modes, or new key files on D: are owner-only by inheritance.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-CRY-108 | V1_IMPORTANT | When the CLI loads an X-Wing identity or Ed25519 signing-key file readable beyond its owner, must it refuse (frozen crypto-suite-v1 behaviour), warn (current code) or... | Incumbent load-time handling survey (measured, not recalled); workflow cases with group/world-readable modes; status-quo warning behaviour of `ebound` on Linux; Windows DACL inspection route inputs. | Safe DACL inspection API (EXP-PLAT-016); crypto-document reconciliation (crypto domain); comprehension of warnings (HUMAN-FACING). |
| DEC-PLT-042 | V1_IMPORTANT | Must the CLI restrict the ACLs of generated secret key and identity files on Windows (and check them on load), and through which safe API? | Default inherited ACLs of key files created by `ebound` on D:\eb-research and observed (read-only) inheritance on user-profile folders; ssh-keygen, age and minisign practice on Windows where installed. | Safe ACL set API (EXP-PLAT-016). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** `status-quo-warn-unix-only` (DEC-CRY-108) and `status-quo-no-op-on-windows` (DEC-PLT-042).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-CRY-108** — candidates: `status-quo-warn-unix-only`, `refuse-unix-default-with-override`, `refuse-without-override`, `warn-all-platforms-with-dacl-check`, `policy-knob-default-warn`, `no-check-caller-responsibility` (status quo: `status-quo-warn-unix-only`). Treatment: Each policy is scored on the workflow case list: refusals of legitimate cases (secondary; no pre-registered band exists for workflow refusals, §0.2 item 4) and silent loads of other-readable keys (`unsafe_defaults`).
- **DEC-PLT-042** — candidates: `status-quo-no-op-on-windows`, `owner-only-dacl-safe-wrapper`, `warn-on-inherited-access`, `helper-process-icacls`, `document-risk-only` (status quo: `status-quo-no-op-on-windows`). Treatment: Status quo measured; restrict/warn candidates scored by resulting principals with read access (derived) and route feasibility (EXP-PLAT-016).

## Corpus, fixtures and case sets

- **Workflow case list** (`workflow_cases.yaml`, each with a primary-source citation for the default mode): Kubernetes secret volume default mode, Docker secrets, systemd `LoadCredential`, GitLab CI file variables, Vault Agent templates, tmpfs `/run/secrets`, vfat-mounted media (Linux mount defaults), drvfs `/mnt/c` without metadata, NTFS files created by Explorer/PowerShell under the user profile. Locally reproducible cases (vfat, drvfs, tmpfs, systemd if available in WSL) are reproduced; others are recorded as `[EXTERNALLY_SOURCED]` inputs.
- **Keys:** generated test keys only (never user keys): OpenSSH ed25519, GnuPG test home, age identity, minisign key, restic password file, `ebound key generate-signing` key; seeded modes {0600, 0640, 0644, 0660, 0666, 0755, 0777}.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_ids:** `wsl-ubuntu`, `windows-host` (non-admin). No timing.

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
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-020/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-020/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-020/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-020/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-020/spec.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-020/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Scripts:** `key_perm_incumbents.sh --tool <t> --modes all --scratch <dir>` (e.g. `ssh-keygen -y -f key` and `ssh -i key -o BatchMode=yes localhost` exit/message; `gpg --homedir`; `age -d -i`; `minisign -S -s`; `restic --password-file`; `ebound sign` with the key file), `provisioning_modes.sh` (mount vfat, drvfs, tmpfs with documented options; place keys; record observed modes), `key_perm_incumbents.ps1 -Tool <openssh|ebound|acl-census>` (Windows OpenSSH `ssh.exe -i`, `ebound.exe key generate-signing`, `Get-Acl` principals with read rights; read-only `Get-Acl` on `%USERPROFILE%` and `%USERPROFILE%\.ssh` if present).

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- Per (tool, mode or ACL case): load outcome {refuse, warn, silent}, message text (for the actionability census), exit code.
- Per workflow case: observed mode/ACL and whether each candidate policy would refuse.
- Windows: principals with read access on new key files.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `unsafe_defaults` | OD-25 | binary screen (silent load of a key readable by other local principals under a candidate's default) | correctness | count | equal_zero | HC-05 (`metric_thresholds.unsafe_defaults`; A.2 role `binary`) | script |
| `error_actionability_fraction` | OD-25 | primary for the mechanical message census (M25.3) | other | fraction | maximize | T-20 (`metric_thresholds.error_actionability_fraction`; A.2 role `banded`) | script |
| `workflow_refusals` | OD-25 | secondary only (no Appendix A.2 band; decision-method §0.2 item 4 — reported, never eliminates or selects) | other | see definition | report | none | script |

## Normalization

Outcomes normalized per tool and case. Reference: status quo.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Limitation recorded:** without a banded metric for legitimate-workflow refusals, the refuse/warn trade-off is decided by the binary screen and R10; the ledger must state this.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- If provisioning mechanisms commonly present 0644 keys, a hard refusal policy has a measurable workflow cost; if not, the warning-only status quo has no measured benefit.

## Threats to validity

- Documented provisioning defaults can be overridden by operators; cases record defaults only.
- Windows ACL inheritance depends on the parent folder; D:\eb-research may differ from a typical profile folder, which is observed read-only for comparison.

## Estimated cost and effort

- **Compute:** about 1 machine-hours; **disk:** about 1 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-020/scripts/{key_perm_incumbents.sh,key_perm_incumbents.ps1,provisioning_modes.sh,workflow_cases.yaml} (mechanical); apt: openssh-client gnupg age minisign restic (approved downloads)
- **Agent effort:** execution none (scripted); tooling scripted; judgment for the workflow case list (committed with primary-source citations before runs).

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
