# EXP-INT-005: Crash consistency and failure-path matrix for every output path

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T7 `ebr-int-kill`, T8 `ebr-int-fsfault`, transaction-pattern shim `ebr-int-txn`); sub-arms BLOCKED: Windows disk-full and ReFS (admin rights), power loss on NTFS and on real devices (EXP-INT-018), APFS (EXP-INT-019) |
| Domain | integrity (program §24, §21 cross-reference; objective HC-18 MVT-18(a)) |
| Decisions informed | DEC-INT-003, DEC-INT-031 (in-place repair crash arm), DEC-INT-039 (failure cleanup arm), DEC-LEG-038, DEC-INT-047 (journaling reference arm) |
| Gates, method, author | conventions C0 |
| Specs | `spec.yaml` (Linux kill, crash-state, I/O error, disk-full and rename-failure arms); `spec-windows.yaml` (Windows NTFS kill and rename-failure arms) |

## 1. Question

When `pack` (INDEXED file, STREAM file, STREAM to stdout redirected), `repack`, `convert --to`
(export), `publish` (native and multi-target), `sidecar`, `key add`, `key remove`,
`key change-password` (in-place mutations), `sign --embed`, `unpack`, and `read --output` are
killed at seeded points, lose unsynced writes, or hit I/O errors, a full disk or a rename
failure, does any destination object verify as `OK` while incomplete, is any multi-target
publish partially present, is any pre-existing destination object changed, what is left
behind, what do migration reports say, and which output transaction pattern removes every
violation on ext4, XFS, btrfs and NTFS?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-INT-003 | The status quo has at least one path with leftovers after a kill (the ledger records "INDEXED pack no cleanup; export direct write"), and at least one kill point in `export` leaves a destination file. Whether any such file verifies as `OK` is open; HC-18 predicts none may. | Zero leftovers and zero destination files on every status quo path. |
| H2 | DEC-INT-003 | `temp-sync-hardlink-everywhere` and `temp-rename-over` (as implemented by the shim) have zero MVT-18(a) violations under process kill on all filesystems, but differ under the crash-state model: patterns that skip the directory fsync after link or rename lose the final name in some crash states (a durability loss, not an HC-18 violation, recorded separately). | Any MVT-18(a) violation for these patterns. |
| H3 | DEC-INT-003 | `platform-atomic-replace` (`renameat2 RENAME_EXCHANGE` on Linux, `ReplaceFileW` on Windows) never leaves a mixed state for in-place mutations, where the status quo backup-rename sequence leaves a state with no file at the original name at some kill points. | Status quo in-place sequence has no such state. |
| H4 | DEC-LEG-038 | Under disk-full and rename failure during `publish`, the status quo migration report never records `FAILED` and records `lossy_approved: true` for non-LOSSY targets (as the ledger states); `report-v2-corrected-semantics` records every failure. | Status quo reports a failed state for every injected failure. |
| H5 | DEC-INT-047 | zpaq 7.15 appends (header-last journaling) killed at any syscall leave every earlier version extractable and the partial transaction ignored. | Any earlier version unreadable after a kill. |
| H6 | DEC-INT-039 | With late corruption (D-S6 in the last 5% of chunk data) or a kill during STREAM `unpack`, the status quo leaves no final destination file that looks complete; `per-entry-verified-streaming` leaves materialized entries that were individually verified but belong to an archive that fails verification. | Status quo leaves an unmarked final file; or per-entry streaming leaves none. |

## 3. Decisions and candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-INT-003 | `status-quo-mixed` (B-PROD CLI, black box), `temp-sync-hardlink-everywhere`, `temp-rename-over`, `platform-atomic-replace`, `leftover-cleanup-on-start`, `journal-file-recovery` | Production code is not changed by experiments. The alternatives are implemented once in a research shim `ebr-int-txn` that obtains the final archive or export bytes from the production library (`prepare_*` and pack APIs, B-RES) and writes them to the destination using exactly the candidate's syscall sequence. The same kill, crash-state, I/O-error, disk-full and rename-failure arms are applied to the shim and to the CLI. `leftover-cleanup-on-start` is measured as a second invocation after each kill (leftovers remaining afterwards). `journal-file-recovery` writes a journal record before each step and a recovery pass runs before the oracle check. |
| DEC-INT-031 | `out-file-only`, `in-place-atomic`, `in-place-non-atomic`, `repair-then-reencode`, `dry-run-assessment`, `trust-parity-output` | Only the crash behaviour of the write step: `ebr-int-txn` repair writers for the first four (the repair engine itself is EXP-INT-010). `in-place-non-atomic` is expected to violate "pre-existing destination byte-identical"; kept as a negative control. Success rates and wrong-reconstruction refusal are EXP-INT-010. |
| DEC-INT-039 | `status-quo-stage-then-materialize`, `per-entry-verified-streaming`, `temp-tree-then-atomic-move`, `streaming-with-unverified-marking`, `unverified-unmarked-streaming` | Destination state after a kill and after late detected corruption, per candidate (T6-style extraction prototypes in `ebr-int-txn --extract-mode`). Proposed L0 exclusion of `unverified-unmarked-streaming`: it emits final files before verification without marking, contradicting HC-15 (I25) and the verified-before-write staging rule cited in the ledger; counterexample = the late-corruption case; kept as a negative control. Cost is EXP-INT-006. |
| DEC-LEG-038 | `status-quo`, `report-v2-corrected-semantics`, `remove-failed-state`, `fix-in-place` | Status quo reports parsed from `publish --report` under each failure; the alternatives are evaluated by a report-semantics model applied to the recorded event sequence (which state each schema would record). Report reproducibility: the same successful publish on WSL and Windows, with relative and absolute output paths, compared field by field. |
| DEC-INT-047 | `status-quo-deferred`, `never`, `journal-extension-header-last`, `incremental-as-substitute`, `external-repository-tools`, `append-in-place` | Empirical input only: crash safety of an existing header-last journal (zpaq 7.15 `a` append) under the same kill arm, and of the `journal-file-recovery` shim. Demand evidence and complexity cost are analytical and are recorded as gaps of this experiment. `append-in-place` is rejected by the SPEC §22.4 L2419 freeze cited in the ledger (R1; sign-off pending). |

## 4. Corpus

- Tuning: `f04-tuning-sourcelike` (many small files), `f05-tuning-gutenberg-books` (few files),
  `f19-tuning-zoo` (many entries and metadata), `f01-tuning-curl-8-19-0-git` (long write phase);
  legacy sources for `convert` and `sidecar`: `f14-apache-maven-3916-bin-tarball`,
  `f14-gen-nested-archives-py`.
- Validation look: `f19-validation-real-home-snapshot`, `f05-validation-rfc-texts`,
  `f04-validation-objstore`, `f14-gen-office-documents`.
- These are binary HC tests over pre-registered operation and kill-point cases; §4.9 minimum
  support does not apply (C3).

## 5. Environment and platform requirements

- Linux (`wsl-ubuntu`, root inside WSL): ext4, XFS and btrfs loop images under
  `/root/eb-research/scratch/int-005/` (T8); `dm-linear`/`dm-error` on loop devices; ptrace
  (T7). Destination filesystems are never `/mnt/*`.
- Windows (`windows-host`, non-admin): NTFS on D: only (never C:, program standing
  decision); job-object kills; rename failure via an open handle without `FILE_SHARE_DELETE`.
- BLOCKED sub-arms: Windows disk-full (no VHD creation or quota without admin), ReFS (admin),
  power loss with real write caches (EXP-INT-018), APFS (EXP-INT-019).
- `timing: false`.

## 6. Procedure and commands

1. Recording pass per (operation, item, filesystem): `ebr-int-kill record` enumerates the
   file-mutating syscall sequence and the write-phase boundaries (opens with `O_CREAT`,
   renames, links, unlinks, fsyncs).
2. Kill arm: kill at every boundary syscall (exhaustive) and at 200 seeded interior write
   indices (C8), each followed by the oracle check (section 8).
3. Crash-state arm (Linux): `ebr-int-kill crashstates --model ext4-ordered` over the recorded
   trace, at most 2,000 states per run (all if fewer), each materialized and checked.
4. I/O error arm: T8 EIO on a seeded sector range hit by the output (5 ranges per run).
5. Disk-full arm: loop filesystems with free space {0, 4 KiB, 1 MiB, 50% of expected output}.
6. Rename-failure arm: destination name pre-created as a directory (Linux and Windows); held
   open without delete sharing (Windows); parent directory made immutable with `chattr +i`
   inside the loop filesystem (Linux).
7. Pre-existing destination sub-arm: the destination exists with sentinel bytes before every
   run of an exclusive-create command.
8. All through ebr: `spec.yaml` on WSL, `spec-windows.yaml` on Windows; then `verify-raw`,
   `verify_detail.py`, `normalize`, `report`.

## 7. Seeds, warmups, replication

Seeds 2026091741/42/43 (Linux), 2026091744/45/46 (Windows); interior kill indices per C8.
Warmups 0. Repetitions 2 per (operation, item, filesystem, arm). Multi-threaded writers can
order syscalls differently between runs; the recording pass is repeated per repetition, and
kill indices address the boundary *kind and ordinal* (for example "second rename") rather
than raw syscall numbers. A state difference between repetitions is reported, not averaged.
Stress cell: `pack` INDEXED with 32 threads, 20 repetitions at the three boundary kill points
nearest the final rename (race regression evidence, detection bound per Appendix D.5).

## 8. Metrics and oracles

| Metric | ID | Role | Oracle |
|---|---|---|---|
| `mvt18a_violations` | C11 (HC-18) | binary (R1; defect for status quo) | after the kill or failure: (a) no destination file exists that B-PROD `verify` reports `OK` unless the command had already reported success; (b) no publish target set is partially present; (c) every pre-existing destination object is byte-identical to its state before the run |
| durability losses | none (descriptive) | descriptive | crash states in which a command that reported success has no complete destination |
| leftovers (`scratch_final_bytes` and count of temp siblings) | ebr scratch metric | secondary | bytes and files other than inputs and complete outputs, before and after `leftover-cleanup-on-start` |
| `receipt_completeness_fraction` | HC-07 | binary for DEC-LEG-038 report candidates | fraction of injected failures whose report records the failed state and every target outcome |
| report reproducibility mismatches | none | binary-style count for DEC-LEG-038 | fields that differ between hosts or path syntaxes for the same successful publish |
| zpaq version integrity | none | descriptive (DEC-INT-047 input) | every earlier version extracts byte-identically after a kill |
| `unsafe_defaults` | HC-05 | binary for DEC-INT-039 | unverified entries left as final files without marking |

## 9. Normalization

Binary counts are summed over cases (AGG-W). Leftover bytes are reported per operation and
filesystem, never averaged across filesystems (filesystem is an evaluation condition).

## 10. Statistical analysis and sensitivity

No intervals: every metric is a binary count or a descriptive tally over enumerated cases;
exhaustive boundary kills give exhaustive evidence for reported boundaries, and interior
seeded kills are regression evidence (objective MVT-18(a) detection statement). R1 eliminates
any shim pattern with a violation; for the status quo each violation is a production defect
with its artifact (the recorded trace, kill point and destination state). Selection among
violation-free patterns uses cost (EXP-INT-006 timing for fsync cost, C1-C2 counts) and R10.
Sensitivity: filesystem arm (ext4, XFS, btrfs, NTFS separately); thread-count arm (1 versus
32); crash-state model arm (ext4 ordered versus a stricter "only fsynced data and metadata
persist" model).

## 11. Expected negative results worth recording

- Status quo export and INDEXED pack leave partial files or temp siblings after kills.
- Hard-link publish is unavailable on some filesystems or mounts (fallback behaviour recorded).
- Crash-state model shows name loss without a directory fsync for every rename-based pattern.
- Migration reports never record `FAILED` (DEC-LEG-038 status quo).

## 12. Threats to validity

- Process kills do not model power loss; the crash-state arm uses an abstract persistence
  model (ALICE style), not real device behaviour; real power-loss evidence is EXP-INT-018
  (BLOCKED).
- The shim implements candidate patterns outside production; integration may differ.
- WSL2 ext4 sits on a VHDX over NTFS; fsync semantics of the virtual disk are not those of a
  physical disk.
- Windows kills are time- and event-based, not syscall-exact; boundary coverage there is
  weaker and reported as regression evidence only.
- ptrace changes timing and may hide races; the stress cell runs without ptrace (time-based
  kills) to partially offset this.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12; HC tests are re-run on held-out items after unlock.

## 14. Cost estimates

- Linux: about 12 operations x 6 items x 3 filesystems x about 500 kill points x 2 repetitions
  x about 2 s, plus crash-state and fault arms: about 110 CPU-hours; wall about 6 hours at 20
  workers.
- Windows: about 12 operations x 4 items x 200 points x 2 repetitions x 3 s: about 16 CPU-hours;
  wall about 3 hours at 6 workers on D:.
- Disk: about 30 GB on WSL scratch; about 10 GB on D: for the Windows arm.
- Agent effort: tooling build with careful review of the kill injector (judgment); execution
  none; analysis of violation artifacts judgment.

## 15. Deviations (append-only)

None.
