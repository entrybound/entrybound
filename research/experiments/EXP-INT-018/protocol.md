# EXP-INT-018: Power-loss crash consistency of output transactions on NTFS, ReFS and real block devices (BLOCKED)

| Field | Value |
|---|---|
| Status | **BLOCKED** |
| Blocked reason | (1) Windows power-loss emulation needs either Hyper-V VM checkpoints with forced resets or block-level write capture of an NTFS or ReFS volume; both need administrator rights, which this non-admin session lacks (PROGRESS known blockers; ReFS additionally needs admin). (2) Linux block-level write logging and replay (`dm-log-writes`, as used by CrashMonkey) is not available: the WSL2 kernel 5.15.167.4 has `CONFIG_DM_LOG_WRITES` and `CONFIG_DM_FLAKEY` unset (checked 2026-09-17 via `/proc/config.gz`); building and booting a custom WSL kernel is a system configuration change reserved to the program owner. (3) Device volatile-write-cache behaviour needs physical drives and a controllable power cut rig. Docker is unavailable, but it would not help (same kernel). |
| Unblocks when | any one of: owner-approved custom WSL2 kernel with `dm-log-writes`; an admin Windows session with Hyper-V; a physical test host with a switchable power supply and SATA or NVMe test drives |
| Domain | integrity (program §24, §21; objective HC-18) |
| Decisions informed | DEC-INT-003, DEC-INT-031, DEC-INT-039, DEC-INT-047 |
| Gates, method, author | conventions C0 |
| Spec | none now: the runs are VM or block-log orchestrations, not per-item runner samples; a spec is generated from EXP-INT-005's `spec.yaml` (Linux) and `spec-windows.yaml` with an added `--crash-model block-replay` arm when the platform exists |

## 1. Question

Under real write ordering and cache loss after power failure, does any Entrybound output
transaction pattern leave a destination that verifies as `OK` while incomplete, a partial
multi-target publish, a changed pre-existing destination, or a lost committed output, on NTFS,
ReFS, ext4, XFS and btrfs with real block-level write logs?

## 2. Hypotheses and falsification

| ID | Hypothesis | Falsified by |
|---|---|---|
| H1 | Patterns without directory fsync after rename or link lose committed names in some replayed prefixes (durability loss) but never produce an HC-18 violation. | An HC-18 violation for `temp-sync-hardlink-everywhere` or `temp-rename-over`. |
| H2 | The model-based crash-state arm of EXP-INT-005 (ALICE style) predicts every violation that block-level replay finds on ext4. | A replay violation not predicted by the model (then the EXP-INT-005 model arm is recorded as incomplete). |
| H3 | On NTFS, `ReplaceFileW`-based in-place replacement survives power loss with either the old or the new complete file. | A replay state with neither. |

## 3. Candidates

Identical to EXP-INT-005 section 3 (status quo CLI and the `ebr-int-txn` shim patterns for
DEC-INT-003, repair write variants for DEC-INT-031, extraction policies for DEC-INT-039, zpaq and
journal-recovery reference for DEC-INT-047), with the same proposed exclusions and negative controls.

## 4. Corpus

As EXP-INT-005 section 4 (tuning and validation items named there).

## 5. Environment and platform requirements

- Linux: kernel with `dm-log-writes`; `replay-log` from the CrashMonkey or xfstests tooling at a
  pinned commit; ext4, XFS and btrfs on the logged device; root.
- Windows: admin session with Hyper-V; a test VM with NTFS and ReFS data disks; checkpoint and
  forced-reset automation via PowerShell Hyper-V module.
- Physical arm: two drives with and without volatile write cache enabled, a controllable power
  switch, and a harness that cuts power at seeded times relative to fsync markers.

## 6. Procedure and commands (designed; not runnable here)

1. Linux: run each operation on the logged device with marks at every fsync; replay every prefix
   ending at a mark and 200 seeded interior prefixes; mount each replayed state and apply the
   MVT-18(a) oracle with B-PROD `verify`.
2. Windows: for each operation, take a checkpoint, start the operation, force a VM reset at 200
   seeded delays and at destination file-system events, boot, check the oracle.
3. Physical: 50 seeded power cuts per operation per drive configuration.

## 7. Seeds, warmups, replication

Seeds reserved: 2026091881/82/83. Replay is deterministic per prefix (1 repetition plus repeat-hash of
the replayed image); VM resets and physical cuts are stochastic (2 repetitions of the seeded schedule).

## 8. Metrics

`mvt18a_violations` (binary, C11), durability losses (descriptive), leftovers (secondary),
`receipt_completeness_fraction` (HC-07, report states), as EXP-INT-005 section 8.

## 9. Normalization

As EXP-INT-005 (exact tallies; filesystems and device configurations are evaluation conditions).

## 10. Statistical analysis and sensitivity

Exact tallies; R1 on violations; sensitivity by filesystem, mount options (`data=ordered` versus
`data=journal`, `barrier` settings), and write-cache setting.

## 11. Expected negative results worth recording

Name loss after power failure for rename-based publication without directory fsync; possible
differences between ReFS and NTFS replacement semantics.

## 12. Threats to validity

VM virtual disks may not reproduce physical cache behaviour; replay tools model only logged writes.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12.

## 14. Cost estimates

Linux replay about 60 CPU-hours; Windows VM arm about 40 hours wall; physical arm about 2 days of
unattended power cycling. Disk about 100 GB. Agent effort: harness setup judgment; execution scripted.

## 15. Deviations (append-only)

None.
