# EXP-PLAT-017: Output commit, staging-move and spill primitives per target filesystem, with crash-replay consistency

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **NEEDS_TOOLING**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-037, DEC-INT-003, DEC-INT-039, DEC-PLT-011, DEC-ACC-053, DEC-PLT-018
- **Program sections:** 15, 21, 32, 33
- **Decision type (proposed, not assigned):** EMPIRICAL (primitive availability, crash-replay outcomes, spill permissions and residue). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-08 (M08.2 scratch peak for staging doubling), OD-03 (ACL/label preservation across staging moves), OD-25 (M25.4); HC-18, HC-05, HC-07.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** On each reachable target filesystem, which output-commit primitives (exclusive hardlink publish, no-replace rename, exclusive-create direct write, atomic replace, directory fsync, O_TMPFILE/linkat, delete-on-close) are available and crash-consistent, what does staging-then-rename cost and break (EXDEV, disk doubling, rename windows, ACL and label divergence), and are staging spill files created with safe permissions and cleaned up after kills?
- **H1:** Hardlink publish fails on vfat, exFAT and cifs without unix extensions, where status-quo publish/repack refuse without fallback; renameat2 NOREPLACE succeeds on ext4/XFS/btrfs/tmpfs but not on vfat through FUSE-less paths; without directory fsync at least one replayed crash prefix on ext4 or XFS leaves a destination that is absent after a reported success, and with it none; STREAM unpack spill files under `/tmp` are created with mode 0644 (readable by other local users) and remain after SIGKILL.
- **H0 / equivalence:** All primitives behave identically across targets, crash replay shows no difference with or without directory fsync, and spill files are private and cleaned up.
- **Falsification of H1:** Directory fsync makes no difference in any replayed prefix, or spill files are owner-only and removed after SIGKILL.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-037 | V1_IMPORTANT | What commit strategy applies where hard links are unavailable (FAT32/exFAT, some SMB/NFS shares, cloud-sync folders): refuse with a clear reason, no-replace rename, ex... | Publish matrix per target (exFAT via FUSE, cifs/NFS loopback, vfat, Linux filesystems, NTFS D:); crash-consistency with and without directory fsync; safe API availability (with EXP-PLAT-016). | OneDrive folders and native exFAT/SMB on Windows (EXP-PLAT-023). |
| DEC-INT-003 | V1_IMPORTANT | Should every CLI output (pack INDEXED, export, publish, repack, sidecar, in-place mutations) use one crash-safe transaction pattern (temp+sync+link or rename-over, pla... | Crash-replay and SIGKILL results for each transaction pattern on ext4/XFS/btrfs and NTFS; disk-full injection via small loop filesystems. | Kill-at-seeded-points for every CLI output under MVT-18(a) (integrity domain; this experiment provides the filesystem layer). |
| DEC-INT-039 | V1_BLOCKING | Does extraction always stage until full-archive verification, or may streaming extraction materialize entries once their own chunks and content digest verify, reportin... | Feasibility and cost of temp-tree-then-atomic-move (EXDEV, directory renames with open handles, ACL/label divergence). | Staging memory/disk versus archive size (scale domain); security analysis of per-entry streaming (integrity domain). |
| DEC-PLT-011 | V1_IMPORTANT | Which refusal classes must INDEXED and STREAM extraction check before creating anything (symlink policy, reparse, name encoding, target hazards, collisions, hardlink s... | Staging costs for fresh-sibling stage-and-rename: disk doubling, EXDEV, directory fsync, Windows rename windows, label/ACL divergence across rename. | Preflight refusal classes (EXP-PLAT-002/003); rollback safety analysis (EXP-PLAT-014 races). |
| DEC-ACC-053 | V1_IMPORTANT | Where and how should staging spill (location configurability, permissions, ephemeral-key encryption, crash cleanup) and should total_bytes bound concurrent rather than... | Default spill location and permissions on Linux and Windows, residue after SIGKILL/TerminateProcess, O_TMPFILE and delete-on-close availability per target. | Ephemeral encryption (crypto domain); cumulative-accounting false refusals (scale domain). |
| DEC-PLT-018 | V1_IMPORTANT | When a hardlink alias cannot be created (FAT/exFAT, some SMB/FUSE/cloud filesystems, cross-device), should extraction abort (status quo), materialize an independent ve... | Copy and reflink fallback feasibility per target for hardlink aliases during extraction. | Restore behaviour and prevalence (EXP-PLAT-009/003). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production commands (`status-quo-hardlink-only`, `status-quo-mixed`, `status-quo-stage-then-materialize`, `partial-output-status-quo`, `status-quo-temp-dir-default-perms-cumulative`, `abort-status-quo`).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-037** — candidates: `status-quo-hardlink-only`, `noreplace-rename-fallback`, `exclusive-create-direct-write`, `refuse-with-reason`, `add-directory-fsync` (status quo: `status-quo-hardlink-only`). Treatment: Each strategy is exercised as a probe primitive or production command on every target; strategies without production implementations are prototyped in `ebr-platform-probe`.
  - excluded before execution (ledger): `rename-replace` — Replaces an existing destination, violating the no-overwrite publication rule. (violates docs/repack-v1.md L34-36); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-INT-003** — candidates: `status-quo-mixed`, `temp-sync-hardlink-everywhere`, `temp-rename-over`, `platform-atomic-replace`, `leftover-cleanup-on-start`, `journal-file-recovery` (status quo: `status-quo-mixed`). Treatment: Each strategy is exercised as a probe primitive or production command on every target; strategies without production implementations are prototyped in `ebr-platform-probe`.
- **DEC-INT-039** — candidates: `status-quo-stage-then-materialize`, `per-entry-verified-streaming`, `temp-tree-then-atomic-move`, `streaming-with-unverified-marking` (status quo: `status-quo-stage-then-materialize`). Treatment: Each strategy is exercised as a probe primitive or production command on every target; strategies without production implementations are prototyped in `ebr-platform-probe`.
  - excluded before execution (ledger): `unverified-unmarked-streaming` — Presents unverified output as final verified files (violates I25); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-011** — candidates: `partial-output-status-quo`, `full-preflight-then-write`, `fresh-sibling-stage-and-rename`, `rollback-created-objects`, `document-only`, `journaled-crash-safe` (status quo: `partial-output-status-quo`). Treatment: Each strategy is exercised as a probe primitive or production command on every target; strategies without production implementations are prototyped in `ebr-platform-probe`.
- **DEC-ACC-053** — candidates: `status-quo-temp-dir-default-perms-cumulative`, `caller-chosen-directory`, `restricted-permissions`, `ephemeral-encryption`, `unlink-on-create`, `concurrent-accounting-with-reuse` (status quo: `status-quo-temp-dir-default-perms-cumulative`). Treatment: Each strategy is exercised as a probe primitive or production command on every target; strategies without production implementations are prototyped in `ebr-platform-probe`.
- **DEC-PLT-018** — candidates: `abort-status-quo`, `copy-fallback-with-policy-and-report`, `reflink-then-copy`, `skip-alias-with-report`, `caller-grant-for-fallback` (status quo: `abort-status-quo`). Treatment: Each strategy is exercised as a probe primitive or production command on every target; strategies without production implementations are prototyped in `ebr-platform-probe`.
  - excluded before execution (ledger): `unchecked-copy-fallback` — SPEC Pattern 6 and incumbent CVEs (Python gh-151987, LinkFallbackError) require policy re-application on link emulation (violates SPEC §11.6 L1432 frozen extraction pattern); L0/R1(b) counterexample and reviewer sign-off still required (R1).

## Corpus, fixtures and case sets

- **Inputs:** archives packed from `f04-tuning-sourcelike` and `f04-validation-objstore` (many small files) and `f15-validation-edgecases` (sparse), plus generated STREAM archives sized to force spill (seeded 20260917 tuning `plat017-fixture-tuning`, 20260918 validation `plat017-fixture-validation`).
- **Targets:** `ext4`, `xfs`, `btrfs`, `tmpfs`, `vfat`, `ntfs3g`, `cifs-samba-loopback`, `nfs4-knfsd-loopback`, `exfat-fuse`; Windows NTFS D:; destination-root-is-mount-point variant for EXDEV.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_ids:** `wsl-ubuntu` (root; dm-log-writes on loop devices where available), `windows-host` (non-admin; `TEMP` default observed read-only for ACL census, runs redirected to D:). No timing.

| Requirement | Needed? |
|---|---|
| Linux (WSL2, root) | yes |
| Windows 11 host (non-admin) | yes |
| macOS | no |
| x86-64 | yes |
| ARM64 | no |
| Network | loopback SMB/NFS only; apt packages (approved) |
| Privileged | WSL root (dm targets, mounts, Samba/knfsd) |
| Large storage | about 30 GB |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Linux):** `rm -rf /root/eb-research/src/entrybound-plat && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/entrybound-plat && git -C /root/eb-research/src/entrybound-plat checkout <record-commit> && cd /root/eb-research/src/entrybound-plat && CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound` → `/root/eb-research/target/production/release/ebound` (production workspace, no `research-internals`; harness review R1-06).
- **Production CLI (Windows):** `git clone D:/Projects/entrybound/entrybound D:/eb-research/src/entrybound-plat; git -C D:/eb-research/src/entrybound-plat checkout <record-commit>; $env:CARGO_TARGET_DIR='D:/eb-research/target/production-win'; & "$env:USERPROFILE/.cargo/bin/cargo.exe" +1.98.1 build --release -p entrybound-cli --bin ebound` → `D:/eb-research/target/production-win/release/ebound.exe`.
- **Harness (only where named):** `wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh build --release` (→ `/root/eb-research/target/harness/release/`), after `C:/Python313/python.exe research/harness/lock_parity.py` reports `RESULT PASS` (harness review R1-06).
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-017/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-017/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-017/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-017/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-017/spec.yaml'`
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-017/spec-crash.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-017/spec-crash.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-017/spec-crash.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-017/spec-crash.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-017/spec-crash.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-017/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Scripts:** `scripts/commit_arms.sh --target <t> --item <path> --scratch <dir>` (primitive matrix; `ebound publish --output-dir`, `ebound repack`, `ebound pack` INDEXED, `ebound convert --to tar`, `ebound unpack` into the target; leftovers census), `scripts/kill_points.py` (SIGKILL at 200 seeded points per command, MVT-18(a) oracle: no destination verifies `OK` unless complete, no partial publish set, pre-existing objects byte-identical), `scripts/logwrites_replay.sh --strategy <s> --fs <ext4|xfs|btrfs>` (record with dm-log-writes, replay every flush/FUA mark and 1,000 seeded write prefixes, mount, run the oracle), `scripts/staging_moves.sh` (EXDEV, directory fsync, default ACL inheritance and `security.selinux` divergence across rename), `scripts/spill_security.sh` / `.ps1` (spill path, mode or DACL, residue after SIGKILL/`Stop-Process -Force`).

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Crash replay:** exhaustive over flush/FUA marks; seeded interior write prefixes are regression evidence (objective MVT-18(a) detection note).

## Measurement method and metrics

- **Primitive outcome** per (target, primitive): success, typed error, silent non-atomic behaviour.
- **Crash oracle** per replayed state: destination absent or verifies `OK` complete; never partial `OK`; pre-existing objects unchanged.
- **Staging:** peak extra bytes during stage-and-rename (`scratch_peak_bytes`), EXDEV occurrences, ACL/label equality before and after the move.
- **Spill:** created path, mode/DACL principals with read access, residue after kill.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `hc18_mvt18a_crash_violations` | OD-25 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-18 MVT-18(a) | kill/replay oracle |
| `restore_fidelity_fraction` | OD-03 | primary for ACL/label preservation across staging moves | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | `staging_moves.sh` |
| `scratch_peak_bytes` | OD-08 | primary for staging disk doubling (T-07; polled lower bound) | scratch_peak | B | minimize | T-07 (`metric_thresholds.scratch_peak_bytes`; A.2 role `banded`) | ebr scratch source / script |
| `unsafe_defaults` | OD-25 | binary screen (spill readable by other principals; residue after kill) | correctness | count | equal_zero | HC-05 (`metric_thresholds.unsafe_defaults`; A.2 role `binary`) | `spill_security` |

## Normalization

Outcomes per primitive and replayed state; bytes exact. Reference: status quo.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Target arm:** results never pooled across targets; a strategy that fails on one in-scope target is scoped (R9) only if the writer can detect the target before committing (EXP-PLAT-001).

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Expected refusals without fallback on vfat/exFAT/cifs for hardlink publish — a negative result for the status quo's portability.
- If dm-log-writes is unavailable on this kernel, the crash-consistency comparison is BLOCKED here and must not be replaced by SIGKILL results (which cannot lose unflushed data).

## Threats to validity

- Loopback SMB/NFS and FUSE exFAT are not Windows or appliance servers.
- dm-log-writes replays the block layer of a loop device on a VHDX; hardware write-cache behaviour is not modelled.
- Windows crash consistency (power loss) is not reachable without elevation or VMs; only kill-based results are available there.

## Estimated cost and effort

- **Compute:** about 12 machine-hours; **disk:** about 30 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** `ebr-platform-probe commit-matrix` and `commit-strategy` subcommands (EXP-PLAT-016 crate): hardlink publish, renameat2 NOREPLACE, O_EXCL direct write, rename-over control, O_TMPFILE+linkat, directory fsync on/off, ReplaceFileW/MoveFileExW, delete-on-close; crash replay: dm-log-writes replay tool (`scripts/logwrites_replay.sh` around `dmsetup create ... log-writes` and the kernel `replay-log` utility from xfstests, pinned) — availability of the `log-writes` target is decided by EXP-PLAT-001 (`dmsetup targets`); if absent the crash arm is BLOCKED on this kernel and only SIGKILL arms run; research/experiments/EXP-PLAT-017/scripts/{commit_arms.sh,commit_arms.ps1,kill_points.py,staging_moves.sh,spill_security.sh,spill_security.ps1}; optional apt: samba cifs-utils nfs-kernel-server exfat-fuse
- **Agent effort:** tooling judgment once (crash oracle), then scripted; execution none; analysis judgment.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
