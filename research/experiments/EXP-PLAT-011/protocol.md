# EXP-PLAT-011: ACL dialects, bounds and canonicalization (Linux filesystems, knfsd NFSv4, FreeBSD ZFS under QEMU)

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-002, DEC-PLT-052, DEC-PLT-054, DEC-PLT-029, DEC-PLT-036
- **Program sections:** 15, 21
- **Decision type (proposed, not assigned):** EMPIRICAL (maxima, kernel canonicalization, capture outcomes, observed NFS4 ACE bits); FORMAL cross-check of masks in EXP-PLAT-021. Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03 (M03.1), OD-17 (M17.3 for bound candidates; M17.6 descriptive maxima); HC-06, HC-07, HC-01.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** Do real filesystems produce ACLs that approach or exceed the frozen bounds (at most 3 ACLs per entry, 65,536 ACEs per ACL), can POSIX1E and NFS4 dialects coexist on one entry anywhere reachable, how does the Linux kernel canonicalize trivial and malformed POSIX ACL xattrs and what does status-quo capture do with them, and do NFSv4 ACLs produced by knfsd and FreeBSD ZFS fit the frozen NFS4 registry masks?
- **H1:** Per-filesystem ACE maxima are bounded by xattr value limits far below 65,536 on ext4 without ea_inode and within it elsewhere; no reachable platform attaches both dialects to one entry; the kernel refuses malformed ACL xattrs at set time and stores no trivial access ACL written through `setfacl` (so the status quo's abort on unprojectable values is unreachable from kernel-stored data except through raw filesystem images); FreeBSD ZFS NFSv4 ACLs use only mask bits inside 0x001f01ff and flag bits inside 0x000000ff.
- **H0 / equivalence:** Maxima reach the frozen ACE bound on some filesystem; malformed values can be stored by root; observed NFSv4 ACEs use bits outside the registry masks.
- **Falsification of H1:** Any stored malformed ACL xattr on a mounted Linux filesystem, any filesystem exceeding the 65,536-ACE bound, or any observed NFSv4 mask/flag bit outside the frozen registry.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-002 | V1_IMPORTANT | Are the frozen ACL bounds (at most 3 ACLs per Entry, 65,536 ACEs per ACL) justified, and may POSIX1E and NFS4 ACLs coexist on one Entry, and if so with what precedence... | Measured ACE maxima per filesystem, coexistence of dialects, parse behaviour at maximal ACLs (outcome only; cost informational). | Real ACE distribution (EXP-PLAT-003); primary-source maxima (EXP-PLAT-021). |
| DEC-PLT-052 | V1_IMPORTANT | Should trivial POSIX1E ACCESS ACLs (only USER_OBJ, GROUP_OBJ, OTHER) be elided at capture, forbidden, or kept as source fidelity? | Whether trivial access ACLs are stored by the kernel, how production capture records them, AUX churn frequency input. | Observable restore differences (EXP-PLAT-008). |
| DEC-PLT-054 | V1_IMPORTANT | Under feature 0x10000, what happens to a Linux POSIX ACL xattr that cannot be projected (malformed, unknown tag, unsupported version, non-rwx permission bits): abort t... | Kernel storage of non-canonical ACL xattrs (seeded fuzz, root), production capture outcome on each stored variant, raw-image-planted variants. | Prevalence (EXP-PLAT-003). |
| DEC-PLT-029 | V1_IMPORTANT | Should the NFS4 ACL dialect remain frozen without any producing or consuming implementation, and which non-Linux ACL capture targets (macOS extended ACLs, FreeBSD/ZFS... | Linux knfsd NFSv4 ACL behaviour through nfs4_getfacl/nfs4_setfacl; FreeBSD ZFS NFSv4 ACL capture under QEMU; which non-Linux targets can produce NFS4 data locally. | macOS ACL mapping (EXP-PLAT-021 formal, EXP-PLAT-022 empirical, BLOCKED); safe ACL crates (EXP-PLAT-016). |
| DEC-PLT-036 | V1_IMPORTANT | Are the frozen platform-security-v1 registry values correct and complete: Windows attribute mask 0x005af9a7, macOS flag mask 0x40bf80ef, NFS4 ACE4 permission mask 0x00... | Observed NFS4 ACE mask/flag distributions versus the frozen masks. | Header/RFC cross-check (EXP-PLAT-021); Windows ACE types and attribute bits (EXP-PLAT-013/021). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo (`status-quo-bounds-coexistence-allowed`, `status-quo-accept`, `status-quo-abort-nonconforming`, `status-quo-frozen-unvalidated`, `status-quo-frozen-masks`).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-002** — candidates: `status-quo-bounds-coexistence-allowed`, `forbid-mixed-dialects`, `platform-derived-ace-bounds`, `budget-only-bounds`, `keep-bounds-define-precedence` (status quo: `status-quo-bounds-coexistence-allowed`). Treatment: Scored against measured maxima, kernel behaviour and observed ACE bits; status quo measured directly.
- **DEC-PLT-052** — candidates: `status-quo-accept`, `elide-on-capture`, `keep-as-source-fidelity` (status quo: `status-quo-accept`). Treatment: Scored against measured maxima, kernel behaviour and observed ACE bits; status quo measured directly.
  - excluded before execution (ledger): `forbid-in-validator` — The frozen ACL_V1_POSIX_ACCESS vector is itself a trivial ACCESS ACL; forbidding it breaks a frozen vector without a new feature version. (violates docs/security-metadata-v1-vectors.txt L2-3 frozen vector); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-054** — candidates: `status-quo-abort-nonconforming`, `retain-exact-xattr`, `drop-with-entry-scoped-loss`, `skip-entry-policy` (status quo: `status-quo-abort-nonconforming`). Treatment: Scored against measured maxima, kernel behaviour and observed ACE bits; status quo measured directly.
- **DEC-PLT-029** — candidates: `status-quo-frozen-unvalidated`, `validate-via-linux-nfs-client`, `macos-acl-adapter-v1`, `separate-macos-acl-dialect`, `defer-nfs4-post-v1`, `freebsd-zfs-adapter` (status quo: `status-quo-frozen-unvalidated`). Treatment: Scored against measured maxima, kernel behaviour and observed ACE bits; status quo measured directly.
  - `macos-acl-adapter-v1`: Not measurable here (EXP-PLAT-022).
  - `separate-macos-acl-dialect`: Not measurable here (EXP-PLAT-022).
- **DEC-PLT-036** — candidates: `status-quo-frozen-masks`, `expand-masks-from-current-headers`, `opaque-unknown-bits-preserved`, `refuse-with-declared-loss` (status quo: `status-quo-frozen-masks`). Treatment: Scored against measured maxima, kernel behaviour and observed ACE bits; status quo measured directly.

## Corpus, fixtures and case sets

- **Fixtures:** per seed (tuning 20260917 `plat011-fixture-tuning`, validation 20260918 `plat011-fixture-validation`): ACL ladders (named users/groups added until failure), default plus access ACLs, trivial ACLs set through `setfacl -m u::rw-,g::r--,o::---` and through raw `setfattr -n system.posix_acl_access -v 0x...`, 10,000 seeded fuzzed ACL xattr byte strings (regression evidence), raw-image-planted non-canonical values written with `debugfs -w` into an unmounted ext4 image.
- **Corpus:** `f19-tuning-real-ext4-acl-selinux`, `f19-validation-real-xfs-acl-selinux` (real ACLs, including mask/mode disagreement).
- **Arm D:** a loopback NFSv4.2 export of an ext4 directory from knfsd in WSL; a FreeBSD VM with a ZFS pool on a virtual disk, ACLs set with `setfacl -a` and read with `getfacl -q`, archived by `tar --acls` (bsdtar) and copied out for decoding.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_ids:** `wsl-ubuntu` (root); FreeBSD VM recorded as its own environment capture (`freebsd-qemu-kvm`, qualifier VIRTUALIZED, correctness only); `windows-host` for the NTFS ACE-count arm.

| Requirement | Needed? |
|---|---|
| Linux (WSL2, root) | yes |
| Windows host (non-admin) | yes (NTFS ACE maxima on own files) |
| FreeBSD (QEMU/KVM VM) | yes for arm D |
| macOS | no (EXP-PLAT-022) |
| x86-64 | yes |
| ARM64 | no |
| Network | pinned VM image and apt packages (approved) |
| Privileged | WSL root; VM root |
| Large storage | about 30 GB (VM image and pool) |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Linux):** `rm -rf /root/eb-research/src/entrybound-plat && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/entrybound-plat && git -C /root/eb-research/src/entrybound-plat checkout <record-commit> && cd /root/eb-research/src/entrybound-plat && CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound` → `/root/eb-research/target/production/release/ebound` (production workspace, no `research-internals`; harness review R1-06).
- **Production CLI (Windows):** `git clone D:/Projects/entrybound/entrybound D:/eb-research/src/entrybound-plat; git -C D:/eb-research/src/entrybound-plat checkout <record-commit>; $env:CARGO_TARGET_DIR='D:/eb-research/target/production-win'; & "$env:USERPROFILE/.cargo/bin/cargo.exe" +1.98.1 build --release -p entrybound-cli --bin ebound` → `D:/eb-research/target/production-win/release/ebound.exe`.
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-011/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-011/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-011/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-011/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-011/spec.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-011/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Scripts:** `scripts/acl_arms.sh --arm <a> --target <t> --item <path> --item-id <id> --scratch <dir>`; `scripts/acl_xattr_codec.py` (independent POSIX ACL xattr encoder/decoder, version 2 format); `scripts/acl_fuzz.py --seed <s> --count 10000`; `scripts/nfs4_knfsd.sh` (`exportfs -o rw,no_root_squash,fsid=0 127.0.0.1:<dir>`, `mount -t nfs4 127.0.0.1:/ <mnt>`, `nfs4_getfacl`, `nfs4_setfacl`, then `ebound pack` of the NFS mount); `scripts/freebsd_vm.sh` (boot, run the ACL script over serial console, copy results out); `scripts/acl_maxima.ps1` (icacls grant loop on a file under D:\eb-research until failure, `Get-Acl` byte size).

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Maxima:** largest ACE count accepted and read back exactly per target.
- **Canonicalization:** per variant: set outcome (errno), stored bytes (getfattr hex), production capture outcome (pack success, retained, dropped, abort with reason code).
- **NFS4:** observed ACE types, masks and flags; bits outside the frozen masks; production capture of NFS mounts (unavailable/declared?).

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `scale_limits` | OD-17 | descriptive (M17.6): per-filesystem ACE maxima | other | count; B | report | - (`metric_thresholds.scale_limits`; A.2 role `descriptive`) | script |
| `benign_false_refusal_fraction` | OD-17 | primary for ACL bound candidates (declared-bound refusals on benign fixtures and F19 items) | other | fraction | minimize | T-20 (`metric_thresholds.benign_false_refusal_fraction`; A.2 role `banded`) | script |
| `capture_fidelity_fraction` | OD-03 | primary for trivial/unprojectable ACL capture candidates | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | script |
| `hc07_mvt07a_undeclared_differences` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-07 MVT-07(a) | script |

## Normalization

ACLs normalized to canonical (tag, qualifier, perms) lists via the independent codec; bytes compared as hex. Reference: status quo.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- A kernel that never stores malformed ACLs makes DEC-PLT-054's retain/drop candidates matter only for raw images and imported archives; that is recorded as a scope limit, not a null.
- knfsd maps POSIX ACLs to NFSv4 ACEs lossily; its output validates only the mapped subset of the registry.

## Threats to validity

- FreeBSD under QEMU runs the real FreeBSD kernel and ZFS; virtualization does not change ACL semantics but the VM is not a production NFSv4 server.
- knfsd in WSL is a loopback configuration; client-server idmapping is not exercised (EXP-PLAT-024).

## Estimated cost and effort

- **Compute:** about 8 machine-hours; **disk:** about 30 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-011/scripts/{acl_arms.sh,acl_xattr_codec.py,acl_fuzz.py,nfs4_knfsd.sh,freebsd_vm.sh,acl_maxima.ps1} (mechanical); arm D prerequisites: apt nfs-kernel-server nfs4-acl-tools; FreeBSD 14.x amd64 VM image (pinned release and SHA-256) run with qemu-system-x86_64 and /dev/kvm (approved downloads)
- **Agent effort:** execution none (scripted); tooling scripted (VM provisioning mechanical); judgment for registry-exception classification together with EXP-PLAT-021.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
