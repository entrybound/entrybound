# EXP-PLAT-008: Dangerous restorations, default extraction policy and restore-order conformance (Linux)

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-007, DEC-PLT-043, DEC-PLT-061, DEC-PLT-026, DEC-PLT-046, DEC-PLT-001, DEC-PLT-009, DEC-PLT-052, DEC-PLT-016, DEC-MOD-021
- **Program sections:** 15, 16, 18, 21
- **Decision type (proposed, not assigned):** EMPIRICAL (restoration outcomes, order equivalence counterexamples) with HUMAN-FACING mechanical parts (§4.16: grant naming in refusal messages); security-threat judgments per class are analytical inputs, not measurements. Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-25 (M25.4 `unsafe_defaults`, M25.3 `error_actionability_fraction`), OD-03 (M03.2); HC-05, HC-07, HC-01.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** Under each candidate default extraction policy and grant set, which dangerous restorations (setuid/setgid/sticky, umask, security.capability, security.selinux, trusted.*, ownership, devices, FIFOs, absolute or escaping links, hardlinks, ACLs, immutable flags) actually occur for root, non-root and user-namespace extraction on ext4, XFS and btrfs, how do incumbents behave by default, and are the candidate restore orders equivalent on every fixture?
- **H1:** The status quo restores recorded setuid/setgid/sticky bits unmasked and ignores umask under its default policy (ledger REQ-PLT-0110 CONTRADICTED; SPEC §11.7 L1442), giving `unsafe_defaults` ≥ 1 for root extraction, while Python's `data` filter and GNU tar's non-root default give 0; the repository order (chown → xattrs → ACLs → final mode → timestamps) and SPEC's ACL-last order diverge on at least one fixture (masked named-user ACL plus group bits), and creating symlinks before the directory metadata pass perturbs parent mtimes unless times are re-applied.
- **H0 / equivalence:** The status quo default performs no dangerous restoration for any context, and all candidate orders yield identical final states on every fixture.
- **Falsification of H1:** `unsafe_defaults` = 0 for the status quo default in every context, or no final-state difference between any two orders on any fixture.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-007 | V1_BLOCKING | What is the v1 default extraction policy and the closed set of named grants for dangerous restorations: setuid/setgid/sticky and umask, privilege-bearing xattr namespa... | Line-by-line default matrix for Entrybound policies and incumbents (GNU tar root/non-root, bsdtar, Python data/tar/fully_trusted on 3.12/3.13/3.14, 7-Zip, cpio) over every restorable class and context, including user-namespace capability rootids; coverage table of class versus control and default. | Usability of named grants and simulated first-use sessions (HUMAN-FACING); .NET and node-tar rows (optional downloads); embedder API review (analytical). |
| DEC-PLT-043 | V1_BLOCKING | Must default extraction (library and CLI) strip setuid, setgid and sticky bits (and apply a umask) unless explicitly allowed, or is the current unconditional restorati... | Conformance matrix for 04755/02755/01777 entries extracted as root and non-root; verification that chown clears setuid on each filesystem and the ordering consequence. | Usability of the opt-in (HUMAN-FACING); workflow survey needing setuid preservation (analytical). |
| DEC-PLT-061 | V1_IMPORTANT | Should xattr restoration filter namespaces (security.capability, security.selinux, security.ima, trusted.*, system.*, com.apple.quarantine), relabel rather than restor... | Restore of security.capability, security.selinux, trusted.* and user.* by Entrybound and incumbents as root and non-root; GNU tar `--xattrs-include` defaults and bsdtar behaviour. | Prevalence (EXP-PLAT-003); Docker/SELinux-enforcing relabel (EXP-PLAT-024). |
| DEC-PLT-026 | V1_BLOCKING | What normative restore order does v1 freeze for hardlinks, ownership, xattrs, ACLs, mode and special bits, timestamps and directory metadata, and is the repo order (AC... | Final-state comparison of every candidate order on fixtures with masked, default and named ACLs, setuid plus chown, read-only directories containing symlinks, immutable flags; committed counterexamples. | Windows descriptor ordering once descriptor restore exists (EXP-PLAT-016/023). |
| DEC-PLT-046 | V1_BLOCKING | How are character/block devices, FIFOs and sockets handled in v1: first-class EntryKinds (SPEC §3.3), auxiliary snapshot metadata, FidelityReport-only omission with en... | Incumbent behaviour matrix for devices, FIFOs and sockets at extraction (GNU tar, bsdtar, cpio, Python data filter). | Capture/prevalence (EXP-PLAT-003/009); OCI runtime semantics (EXP-PLAT-024). |
| DEC-PLT-001 | V1_BLOCKING | For symlink members with absolute or escaping targets, should extraction refuse by default (status quo Safe, Python data filter), rewrite or resolve in-root (openat2 R... | Incumbent observed-outcome matrix for absolute and escaping symlink members (Python 3.12–3.14 filters, bsdtar, GNU tar, 7-Zip). | Frequency (EXP-PLAT-003); downstream-following threat and race behaviour (EXP-PLAT-014); .NET/node-tar optional rows. |
| DEC-PLT-009 | V1_IMPORTANT | What are the v1 semantics of the extraction root: must it be a real directory (not a symlink), may extraction merge into an existing non-empty directory by default, an... | Incumbent defaults for extraction into existing non-empty directories and through a symlinked destination root. | Adversarial root swap (EXP-PLAT-014 arm D); usability (HUMAN-FACING). |
| DEC-PLT-052 | V1_IMPORTANT | Should trivial POSIX1E ACCESS ACLs (only USER_OBJ, GROUP_OBJ, OTHER) be elided at capture, forbidden, or kept as source fidelity? | Observable restore differences (getfacl output, `ls -l` '+') of files with and without a stored trivial access ACL. | Kernel canonicalization and capture (EXP-PLAT-011). |
| DEC-PLT-016 | V1_IMPORTANT | Which file flags are captured and restored: a restorable subset of macOS UF/SF flags, exclusion of kernel-managed bits (SF_DATALESS, UF_COMPRESSED, SF_FIRMLINK), and a... | Immutable/append flags blocking later restore writes and the last-restore ordering candidate. | Capture/declaration (EXP-PLAT-004); macOS flags (EXP-PLAT-022). |
| DEC-MOD-021 | V1_BLOCKING | What is the v1 EntryKind set (implemented Directory/File/Symlink/ReparsePoint vs SPEC Fifo/Socket/BlockDevice/CharDevice, whiteouts, junctions) and how are unknown kin... | Restoration feasibility of each special kind per context (root, non-root, user namespace). | Security review of device/FIFO policy (analytical); SPEC amendment (external to program). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production default policy (`status-quo-per-class-enums`, `status-quo-restore-all-mode-bits` (ledger-excluded but measured as the defect baseline), `status-quo-repo-order`); REF-INC: Python 3.12 `tarfile` data filter and GNU tar 1.35.
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-007** — candidates: `status-quo-per-class-enums`, `python-data-filter-parity`, `bsdtar-style-p-flag`, `named-grants-closed-set`, `xattr-namespace-allow-list`, `root-aware-defaults`, `per-platform-xattr-defaults`, `status-quo-per-class-flags`, `spec-allow-list`, `named-presets`, `per-class-flags-plus-allow-for-privileged`, `python-filter-model` (status quo: `status-quo-per-class-enums`, `status-quo-per-class-flags`). Treatment: Policy candidates are realized either by Entrybound flags (status quo per-class enums) or by the incumbent that implements the same rule (Python `data` filter = python-data-filter-parity; GNU tar non-root/-p = bsdtar-style-p-flag and umask semantics); candidates without any implementation are scored by applying their written rule to the measured per-class outcomes (marked `[derived]`).
  - `status-quo-per-class-enums`: Measured directly (production default and each flag).
  - `status-quo-per-class-flags`: Measured directly (CLI flags).
  - excluded before execution (ledger): `archive-requested-restoration` — Extraction policy is caller-owned and cannot be constructed or widened by archive content (violates I16); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-043** — candidates: `strip-special-bits-allow-flag`, `restore-only-with-ownership-restore`, `umask-default-preserve-flag`, `strip-setuid-setgid-keep-directory-sticky`, `refuse-special-bits-unless-allowed` (status quo: `status-quo-restore-all-mode-bits`). Treatment: Measured through Entrybound flags where they exist and through the incumbent implementing the rule (GNU tar non-root default = umask-default-preserve-flag; GNU tar `--same-owner` = restore-only-with-ownership-restore); others `[derived]`.
  - excluded before execution (ledger): `status-quo-restore-all-mode-bits` — Restores setuid/setgid/sticky by default, which SPEC §11.7 forbids; only admissible if an authority explicitly supersedes that rule. (violates SPEC §11.7 L1442); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-061** — candidates: `status-quo-all-or-nothing`, `user-namespace-only-default`, `separate-security-namespace-flag`, `capability-as-dangerous-class`, `selinux-relabel`, `security-namespaces-capture-only` (status quo: `status-quo-all-or-nothing`). Treatment: Measured through Entrybound `--xattrs restore` (all-or-nothing) and GNU tar `--xattrs-include` patterns implementing namespace allow-lists; others `[derived]`.
- **DEC-PLT-026** — candidates: `status-quo-repo-order`, `spec-acl-last`, `mode-acl-mode-verify`, `single-call-acl-and-mode`, `status-quo-dir-metadata-before-symlinks`, `symlinks-then-directory-metadata`, `reapply-directory-times-after-symlinks`, `report-perturbation-only`, `full-per-platform-order-table` (status quo: `status-quo-repo-order`, `status-quo-dir-metadata-before-symlinks`). Treatment: Each order is executed by `restore_order_sim.py` as an explicit syscall sequence on identical fixtures.
  - `full-per-platform-order-table`: Linux rows measured; Windows/macOS rows blocked or pending prototypes.
- **DEC-PLT-046** — candidates: `status-quo-refuse-pack`, `omit-and-report-entry-scoped`, `first-class-kinds-spec`, `kinds-without-socket`, `aux-snapshot-metadata`, `policy-selectable-refuse-or-omit`, `post-v1-feature-bit` (status quo: `status-quo-refuse-pack`). Treatment: Only the extraction-behaviour column is measured here.
- **DEC-PLT-001** — candidates: `refuse-by-default-all-opt-in-status-quo`, `write-as-is-never-follow`, `rewrite-relative-in-root`, `skip-with-report`, `separate-grants-absolute-vs-escaping` (status quo: `refuse-by-default-all-opt-in-status-quo`). Treatment: Measured through incumbents implementing each rule (refuse: Python data; write as-is: GNU tar default; skip with report: 7-Zip) and Entrybound Safe/All.
- **DEC-PLT-009** — candidates: `status-quo-merge-follow-root-symlink`, `require-empty-or-new-root`, `refuse-symlink-root`, `follow-root-symlink-documented`, `single-top-level-enforced` (status quo: `status-quo-merge-follow-root-symlink`). Treatment: Status quo and incumbents measured; stricter root candidates `[derived]` from the observed hazards.
- **DEC-PLT-052** — candidates: `status-quo-accept`, `elide-on-capture`, `keep-as-source-fidelity` (status quo: `status-quo-accept`). Treatment: Observable-difference column only.
  - excluded before execution (ledger): `forbid-in-validator` — The frozen ACL_V1_POSIX_ACCESS vector is itself a trivial ACCESS ACL; forbidding it breaks a frozen vector without a new feature version. (violates docs/security-metadata-v1-vectors.txt L2-3 frozen vector); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-016** — candidates: `status-quo`, `macos-user-flag-subset-restore`, `declare-linux-flags-unavailable`, `add-linux-flags-name`, `immutable-last-privileged-opt-in` (status quo: `status-quo`). Treatment: Ordering column only.
- **DEC-MOD-021** — candidates: `four-kinds-status-quo`, `spec-seven-kinds`, `special-files-as-metadata`, `whiteout-kind`, `unknown-kind-fail-closed` (status quo: `four-kinds-status-quo`). Treatment: Restoration feasibility column only.

## Corpus, fixtures and case sets

- **Dangerous fixture** (`scripts/gen_dangerous_fixture.py --seed <seed>`, built as root on ext4 then archived as pax tar by GNU tar and as native `.eb` where `ebound pack` accepts it): 04755/02755 root-owned executables, 01777 directory, 0777 file under umask 022, `security.capability` set by `setcap cap_net_raw+ep`, `security.selinux`, `trusted.test`, `user.test`, named-user access ACL with mask, default ACL, char and block devices, FIFO, socket, absolute symlink, escaping relative symlink, hardlink member targeting an outside path (tar only), immutable and append-only files (chattr, applied last), read-only directory containing a symlink, directory with a set mtime, a pre-existing destination tree with a symlinked subdirectory. Tuning seed 20260917 (`plat008-fixture-tuning`), validation 20260918 (`plat008-fixture-validation`).
- **Corpus:** `f19-tuning-zoo` (all 4,096 modes, capabilities, devices), `f19-tuning-real-ext4-acl-selinux`, `f19-validation-real-xfs-acl-selinux` (loop-mounted read-only as capture sources), `f20-tuning-generated-malicious-structures`, `f20-validation-generated-malicious-structures` (symlink/hardlink escape members).
- **Contexts:** extraction as root, as non-root (`setpriv --reuid=65534 --regid=65534 --clear-groups`), and as user-namespace root (`unshare --user --map-root-user`); destination filesystems ext4, XFS, btrfs.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_id:** `wsl-ubuntu` (root). Windows dangerous-restoration rows (descriptors, reparse) are covered by EXP-PLAT-013 and EXP-PLAT-023. No timing.

| Requirement | Needed? |
|---|---|
| Linux (WSL2, root; non-root and user-namespace contexts) | yes |
| Windows host | no |
| macOS | no (EXP-PLAT-022) |
| x86-64 | yes |
| ARM64 | no |
| Network | pinned CPython downloads only |
| Privileged | WSL root (setcap, mknod, chattr, loop mounts) |
| Large storage | no |
| External review | no (security analysis per class is analytical input) |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Linux):** `rm -rf /root/eb-research/src/entrybound-plat && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/entrybound-plat && git -C /root/eb-research/src/entrybound-plat checkout <record-commit> && cd /root/eb-research/src/entrybound-plat && CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound` → `/root/eb-research/target/production/release/ebound` (production workspace, no `research-internals`; harness review R1-06).
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-008/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-008/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-008/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-008/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-008/spec.yaml'`
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-008/spec-order.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-008/spec-order.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-008/spec-order.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-008/spec-order.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-008/spec-order.yaml'`
- **Extraction (`scripts/extract_dangerous.sh --system <s> --flags <f> --item <path> --item-id <id> --scratch <dir>`):** loops over contexts × destination filesystems; exact flags: `ebound unpack a.eb <dst>` plus `--restore-owner`, `--xattrs restore`, `--acls restore`, `--symlinks all`, or all grants; `tar -xf` (default), `tar -xpf`, `tar -xpf --same-owner --xattrs --xattrs-include='*' --acls --selinux`; `bsdtar -xf`, `bsdtar -xpf --xattrs --acls --fflags`; `python3.X -c 'tarfile.open(a).extractall(dst, filter=F)'`; `7z x -snl`; `cpio -idm`.
- **Observation (`scripts/observe_dangerous.py <dst>`):** mode including special bits, owner, `getcap`, `getfattr -d -m - -h`, `getfacl -n`, `lsattr`, device numbers, link targets and resolution (`realpath -m`), objects created outside the root (tree diff of the destination's parent).
- **Order simulation (`scripts/restore_order_sim.py --order <o> --fixture <dir> --fs <ext4|xfs|btrfs> --context <root|nonroot>`):** applies the order as explicit calls (`os.chown`, `os.setxattr`, ACL xattr writes, `os.chmod`, `os.utime`, `os.symlink`, `ioctl` flags via `chattr`) and compares the final state with the archived intent; every divergence is written as a counterexample under `research/raw/EXP-PLAT-008/counterexamples/`.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Dangerous-restoration census:** per (system, flags, context, filesystem, fixture entry), whether each pre-registered dangerous class was restored; `unsafe_defaults` counts classes restored under a system's default flags that the pre-registered class list marks dangerous (SPEC §11.7 L1442 special bits, §17.1 L1792, §11.2 L1368).
- **Grant exactness:** under explicit grants, whether each requested class is restored exactly.
- **Refusal messages:** whether each refusal names the class and the grant that would allow it (mechanical census for `error_actionability_fraction`).
- **Order equivalence:** per (order, fixture, filesystem, context), final state equal to intent; pairwise order equivalence derived.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `unsafe_defaults` | OD-25 | binary R1 screen (M25.4) per policy candidate and context | correctness | count | equal_zero | HC-05 (`metric_thresholds.unsafe_defaults`; A.2 role `binary`) | observation |
| `restore_fidelity_fraction` | OD-03 | primary under explicit grants and per restore order | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | observation |
| `error_actionability_fraction` | OD-25 | primary for the HUMAN-FACING mechanical part (M25.3), refusal messages naming the grant | other | fraction | maximize | T-20 (`metric_thresholds.error_actionability_fraction`; A.2 role `banded`) | message census |
| `hc05_mvt05_escapes` | OD-25 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-05 MVT-05(a)/(b) | tree diff outside root |
| `hc07_mvt07a_undeclared_differences` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-07 MVT-07(a) | observation versus ExtractionReport |

## Normalization

Outcomes normalized to {restored-exact, restored-altered, not-restored-reported, not-restored-silent, refused, escaped}. Reference: status quo default.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Order decision:** an order with a silent divergence on any fixture is eliminated at R1 (HC-07); among equivalent survivors R10 applies (fewer normative words, then status quo).

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Context arm:** root, non-root and user-namespace results are separate conditions.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- The status quo's unmasked mode restoration is expected to be a recorded defect (ledger already excludes it as a candidate); measuring it anchors the default-policy comparison.
- Python's `data` filter may refuse benign archives with absolute symlinks common in container layers; the benign cost comes from EXP-PLAT-003.
- In a user namespace, `security.capability` may be restored as a v3 capability bound to the namespace root id — neither safe nor unsafe on the host, recorded as its own outcome.

## Threats to validity

- WSL has no SELinux enforcement; `security.selinux` restoration is observed only as an xattr write.
- Incumbent versions are pinned; later versions (for example CPython filter changes) can change rows.
- The dangerous-class list is a pre-registered judgment grounded in SPEC clauses; classes outside it are not scored.

## Estimated cost and effort

- **Compute:** about 5 machine-hours; **disk:** about 6 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-008/scripts/{gen_dangerous_fixture.py,extract_dangerous.sh,restore_order_sim.py,observe_dangerous.py} (mechanical); pinned CPython 3.13 and 3.14 standalone builds (approved downloads)
- **Agent effort:** execution none (scripted); tooling scripted; judgment for classifying each restoration as dangerous under the pre-registered class list and for the order-equivalence counterexamples.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
