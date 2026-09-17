# F-0002 — The same logical tree yields a different LAI depending on the packing OS

- **Found:** 2026-09-17, Phase B2 cross-host determinism smoke `EXP-DET-SMOKE` (`research/normalized/EXP-DET-SMOKE/summary.csv`, raw inspect JSON under `research/raw/EXP-DET-SMOKE/{wsl-x86_64,windows-x86_64}/inspect/`).
- **Baseline:** production behavior at `9e44608`; the harness only reports it.
- **Evidence class:** EMPIRICALLY_MEASURED; the interpretation is INFERRED.

## Observation

One checked-in generated tree (52 entries) was packed under all four profiles in both layouts on WSL2 Ubuntu 24.04 (ext4)
and on the Windows 11 host (NTFS):

| Identity | Cross-host result |
|---|---|
| PCR | **identical** in all 8 cells (chunking, dedup, and planning are platform-independent for this tree) |
| LAI | **differs** in all 8 cells |
| AUX | **differs** in all 8 cells |
| PCI / bytes | differ (follows from AUX and feature bits; see F-0001) |

A per-entry field diff of `balanced-indexed` inspect output (the generating command is in this document's review
history; comparison by path over identical path sets) shows:

- `core_executable` differs in **8 of 52** entries: `false` on Windows, `true` on Linux, where the files have mode 0755.
  CONTRIBUTING classifies executable state as LAI semantics, so this is the entire LAI difference.
- `core_mtime` differs in 52 entries only in the recorded **precision** (`Hectonanosecond` on Windows vs `Nanosecond` on
  Linux) for identical seconds/nanoseconds values.
- `posix` (mode/uid/gid) is populated only on Linux, and `platform_security` (Windows attributes, creation time) only
  on Windows.

## Interpretation (INFERRED)

1. NTFS cannot represent POSIX execute permission, and Windows capture records `executable=false` rather than
   *unknown*. A tree whose logical content and intended executable state are identical therefore receives a different
   Logical Archive Identity when packed on Windows. That conflicts with the archetypal expectation that LAI is a
   canonical, host-independent identity of logical content (hard constraints on canonical identities and on archive
   semantics independent of host filesystem interpretation).
2. Recording timestamp precision from the capturing filesystem makes AUX host-dependent even for identical instants.
   Whether precision is semantic (a restorability promise) or incidental needs a decision.
3. PCR equality confirms that the physical content pipeline is deterministic across these two x86-64 hosts for this
   input. Emulated arm64 was deferred (Docker unavailable).

## Ledger action

Addendum rows in `research/audit/addenda/orchestrator-findings.jsonl` (cluster `model`):
- requirement `cross-host-lai-executable-unobservable`, decision `lai-unobservable-semantics`;
- requirement `aux-timestamp-precision-host-dependent`, decision `timestamp-precision-semantics`.
