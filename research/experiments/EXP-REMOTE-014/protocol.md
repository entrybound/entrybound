# EXP-REMOTE-014: Local random-read source revision signal under concurrent modification

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T10 mutator harness with deterministic interleaving via a `PausingSource` wrapper; candidate revision-signal sources in `ebr-remote` for Linux and Windows builds; loop-mounted XFS, btrfs and FAT images in WSL). Sub-arms BLOCKED: macOS/APFS (PLATFORM_BLOCKED), SMB share on the Windows host (creating a share needs admin), ReFS (admin). |
| Domain / program section | remote / §12 (§21 safe-Rust/platform API) |
| decision_ids | DEC-ACC-023 |
| Decision type (provisional) | EMPIRICAL (detection matrix, cost) + FORMAL (documented guarantee when modification is undetectable) |
| OD / HC (provisional until G-A) | OD-04 (M04.2), OD-17 (M17.3), OD-10 guard (M10.3 open cost), OD-08 (scratch for snapshot copies); HC-15 (bytes never reported verified unless verified in the session; unstable source distinguishable from corruption), HC-08 (behaviour must not depend on host filesystem interpretation beyond declared platform limits), HC-05 |
| Author / L7 / gates | As EXP-REMOTE-001. Windows timing (open cost) is not decision-grade until the §14 item 3 runner additions exist; detection outcomes are deterministic and are unaffected. |

## 1. Question

Which revision signal should local random-read sources use (length + mtime, adding file ID and change time, deny-write share modes or advisory locks, content verification only, or a private snapshot copy), what does each detect across in-place same-length writes, mtime restoration, sub-granularity writes, truncate-extend, rename-over and metadata-only changes on NTFS, ext4, XFS, btrfs, FAT, tmpfs and network-style filesystems, and what guarantee must be documented when concurrent modification is undetectable?

## 2. Hypotheses

- **H1 (safety).** For every candidate and mutation, no read returns bytes reported verified that differ from a single consistent archive revision's verified content: digest verification catches every undetected modification of bytes inside the read's closure (`wrong_bytes_reported_verified` = 0).
- **H2 (attribution).** `status-quo-length-mtime` misattributes same-length in-place writes with restored mtime (and writes within the filesystem's mtime granularity, notably FAT's 2 s) as `CORRUPT` digest mismatches rather than `SOURCE_UNSTABLE`, and misses modifications outside the read closure entirely; `file-id-plus-mtime` (adding change time) raises `reason_code_specificity_fraction` on ext4/XFS/btrfs/NTFS but not on FAT or 9P.
- **H3 (locking).** Windows deny-write share mode prevents in-place writers (they fail with a sharing violation) and makes every case detected-by-construction; Linux advisory locks do not stop non-cooperating writers (no detection gain).
- **H4 (snapshot cost).** `snapshot-copy` detects nothing new beyond atomic-copy consistency but costs scratch bytes equal to the archive size and open time linear in size, `DISTINGUISHABLE_WORSE` on `open_latency_s` for medium and large archives.
- **H5 (benign refusals).** Candidates using change time refuse sessions after metadata-only changes (chmod/ACL/xattr) that do not alter content: `benign_false_refusal_fraction` > 0 for them and = 0 for the status quo.
- **H0 / falsification.** H1 falsified by one wrong-bytes-verified event (an HC-15 violation artifact); H2 by a filesystem where the stated attribution differs; H3 if Windows share modes fail to block a writer or Linux locks block one; H4 if snapshot open cost is within band; H5 if change-time candidates never refuse metadata-only changes.

## 3. Decisions informed

| Decision | Supplied here | Remaining |
|---|---|---|
| DEC-ACC-023 | Platform tests: Windows share modes, POSIX in-place same-length writes, network filesystems (9P via `/mnt/d` and `\\wsl.localhost`), mtime granularity (FAT, 9P); confirmation that digest verification catches every undetected modification for returned bytes | SMB/NFS network shares: SMB BLOCKED (admin), NFS only if a WSL kernel NFS server is available (else unmeasured); macOS/APFS PLATFORM_BLOCKED; the documented-guarantee wording: FORMAL from these results |

## 4. Candidates

| candidate_id | Definition | Realization |
|---|---|---|
| `status-quo-length-mtime` (reference) | `SourceRevision::Local { length, modified_nanos }` checked at session start and in `check_stable` (`random_access.rs:110-171, 666-676`) | production `LocalFileRandomReadSource` |
| `file-id-plus-mtime` | adds inode/device (Linux) or volume serial + file index (Windows) and ctime (Linux `st_ctime`) / ChangeTime (Windows, via safe std metadata where available; otherwise recorded as not obtainable without `unsafe`, which harness and production forbid) | `ebr-remote` source variant |
| `share-mode-locking` | Windows: open without `FILE_SHARE_WRITE` (`std::os::windows::fs::OpenOptionsExt::share_mode`); Linux: `flock(LOCK_SH)`-equivalent advisory lock via a safe crate if one exists without `unsafe` in harness code, else recorded as not implementable in safe Rust | source variant |
| `digest-backstop-only` | no revision check; rely on per-read content verification and document detection limits | source variant |
| `snapshot-copy` | copy the archive to private staging before reading (copy verified by length and SHA-256 at copy end) | source variant |

No candidate excluded.

## 5. Case selectors

- Archives: 6 tuning archives from EXP-REMOTE-002 (`balanced-plain`), 2 small, 2 medium, 2 large-tier items (seeded draw across ≥ 4 families), plus 1 `balanced-enc-bucketed` small archive.
- Mutations (`mutations.json`, committed before runs): M1 in-place same-length overwrite of payload bytes inside the requested closure (mtime updated); M2 as M1 with mtime restored to the original value; M3 in-place write within 1 ms of open (sub-granularity on coarse filesystems); M4 truncate then extend to the original length with different bytes; M5 rename-over by a different valid archive of equal length; M6 rename-over by a different-length archive; M7 metadata-only change (mode, xattr); M8 in-place write outside the requested closure; M9 file deleted (unlinked) while held open.
- Interleavings: mutation applied (a) between open and `read_entry`, (b) after the k-th source read inside `read_entry` for k seeded in [1, reads−1] (deterministic via `PausingSource`), (c) after `read_entry` returns and before a second read.
- Filesystems: Windows host NTFS (local, non-admin); WSL ext4 (`/root/eb-research`), XFS and btrfs loop images, FAT32 loop image (`dosfstools`), tmpfs; 9P `/mnt/d` from WSL (mutated from Windows) and `\\wsl.localhost\Ubuntu\root\eb-research\...` from the Windows host (mutated from WSL, outside any held-out root); NFS loopback only if available; SMB, ReFS, APFS BLOCKED.

## 6. Environment and platform requirements

Windows 11 host (non-admin) and WSL2 Ubuntu (root inside the VM for loop mounts); deterministic outcome matrix (no timing); `open_latency_s` for `snapshot-copy` vs status quo in-process batches in a quiet window on WSL ext4 (decision-grade) and on NTFS (informational until §14 item 3); scratch on a dedicated directory with write accounting for `snapshot-copy` (§4.14). Linux and Windows x86-64; macOS PLATFORM_BLOCKED; no large storage beyond copies of the large-tier archives (removed after use).

**Platform matrix.** Linux x86-64 (WSL2, root inside the VM for loop mounts) and Windows x86-64 (non-admin NTFS) both required; macOS/APFS: PLATFORM_BLOCKED; ARM64: not required; network: 9P filesystem paths only; privileged: SMB share and ReFS BLOCKED (admin); large storage: ~20 GB; external review: no.

## 7. Commands

```sh
# WSL
cd /mnt/d/Projects/entrybound/entrybound; PY=/root/eb-research/venv/bin/python; export PYTHONPATH=research/tools
python research/orchestration/disk_guard.py
bash research/experiments/EXP-REMOTE-014/setup_filesystems.sh        # loop images under /root/eb-research/fsimg (ext4 host dir, XFS, btrfs, FAT32, tmpfs)
$PY -m ebr run --spec research/experiments/EXP-REMOTE-014/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-REMOTE-014/spec.yaml && $PY -m ebr normalize --spec research/experiments/EXP-REMOTE-014/spec.yaml
```

```powershell
# Windows host (NTFS and \\wsl.localhost cases), generated spec with shell: pwsh
C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-REMOTE-014/spec-windows.yaml
```

## 8. Seeds, warmups, repetitions

Seeds `order` 20261023, `bootstrap` 20261024, `command` 20261025 (mutation offsets, interleaving k, archive draw). Each (filesystem, candidate, mutation, interleaving, archive) cell: 3 repetitions; an outcome that differs across repetitions is committed as a nondeterminism artifact and counted as a failed attribution. Open-cost timing: warmups 2, rounds min 10, tier step/cap, in-process batches of ≥ 1,000 opens for small archives.

## 9. Metrics

| Metric | OD | Role | Threshold |
|---|---|---|---|
| `wrong_bytes_reported_verified` | – | binary (HC-15) | §3.3 |
| `reason_code_specificity_fraction` (mutation cases reported with the cause-specific class/code: `SOURCE_UNSTABLE` for detected revision change, digest mismatch only when no revision signal could see it) | OD-04 | **primary** | T-20 |
| `benign_false_refusal_fraction` (M7 and no-mutation controls refused) | OD-17 | **primary** | T-20 |
| `undetected_mutation_fraction` (mutations inside the closure that neither the revision signal nor verification reported) | – | reported (must be 0 by H1 for in-closure bytes) | – |
| `open_latency_s` | OD-10 | guard | T-08 |
| `scratch_write_bytes`, `scratch_peak_bytes` (snapshot copy) | OD-08 | secondary | T-07 |
| `writer_blocked` (share mode or lock prevented the mutation) | – | descriptive | – |

## 10. Normalization and statistical analysis

Deterministic coverage fractions over the fixed case list (AGG-C with the minimum over filesystems as headline, AGG-S per platform pair; never pooled across filesystems); R1 on the binary; R4 on the two primaries plus the guard; R9 partition by platform (expressible: sources are platform-specific code) with the §4.1 rule that a claim covering both Windows and Linux needs same-direction support in both. Tiers (independent assessment): source-internal changes T0; a new reason code T2; share-mode behaviour that can make other applications' writes fail is a user-visible behaviour change (T2, C6).

## 11. Practical significance

T-20 (a = 0.5 / n_cases per filesystem), T-08, T-07 by reference; binary §3.3.

## 12. Sensitivity analysis

Interleaving point (a/b/c) as strata; mtime restoration precision (exact vs rounded to 1 s); writer using `O_DIRECT`/unbuffered writes vs buffered; Windows writer opened before vs after the reader; archive tier.

## 13. Expected negative results worth recording

- Length + mtime cannot see same-length writes with restored mtime or sub-granularity writes; attribution then falls back to digest mismatch (safe but less specific).
- Change-time signals raise benign refusals after metadata-only changes.
- Advisory locks give no protection against ordinary writers; share modes can break other applications.
- No signal detects modifications outside the read closure (by design: unread bytes are not claimed, HC-15).

## 14. Threats to validity

Finite mutation list; filesystem behaviour on loop images and 9P inside WSL2 may differ from physical disks and real network shares; Windows ChangeTime may not be reachable through safe std APIs (then that part of `file-id-plus-mtime` is recorded as unimplementable in safe Rust rather than approximated).

## 15. Held-out (Phase D placeholder)

The case list is not corpus-derived; at Phase D the frozen candidate is rerun once with held-out archives as payloads (§5.5), report-only per §5.6.

## 16. Estimates

- Compute: about 5 machine-hours (case matrix × filesystems × candidates; large-tier snapshot copies dominate).
- Disk: about 20 GB peak (loop images and copies of large archives; removed after the run).
- Agent effort: scripted; judgment for the FORMAL guarantee wording and analysis.
