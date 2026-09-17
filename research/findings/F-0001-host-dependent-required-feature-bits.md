# F-0001 — Required feature bits depend on the packing host

- **Found:** 2026-09-17, during the Phase B2 `research-internals` identity check (`research/harness/internals-identity-check.md`), and reproduced with `research/harness/repro_cross_file_linux.sh`.
- **Baseline:** `9e44608` (pre-existing; independent of the research-internals feature).
- **Evidence class:** EMPIRICALLY_MEASURED.

## Observation

`crates/entrybound/tests/cross_file_compression.rs::small_cohort_stays_independent_and_v2_v3_remain_readable`
passes on the Windows 11 host (all 286 workspace tests passed at program start) and fails on WSL2 Ubuntu 24.04 ext4
(284 passed, 1 failed):

```
crates/entrybound/tests/cross_file_compression.rs:271:5
assertion `left == right` failed
  left: 32769   (0x08001 = CROSS_FILE_COMPRESSION_V1 | POSIX_METADATA_V1)
 right: 98305   (0x18001 = ... | PLATFORM_SECURITY_METADATA_V1)
```

The test packs a generated directory with the filesystem workflow and asserts the archive's **incompatible (required)**
feature set, including `FEATURE_PLATFORM_SECURITY_METADATA_V1` (`0x10000`). On Windows, filesystem capture records a
security descriptor for every entry, so `0x10000` is always required. On Linux ext4 without POSIX ACLs, no platform
security metadata is captured and the bit is absent.

## Interpretation (INFERRED)

1. The test asserts a host-specific capture outcome. It is a test portability defect, not archive nondeterminism: the
   same host produces the same bytes.
2. It surfaces a product question: an archive packed on Windows with default options always sets a *required* feature
   bit for platform security metadata, which is AUX (non-identity) metadata. A reader or extractor that does not
   implement platform-security-metadata-v1 must fail closed on essentially every Windows-created archive, even when the
   caller would ignore that metadata. This affects independent implementability, minimal-reader scope, cross-platform
   interoperability, and adoption friction. The decision is whether AUX-only metadata records should gate archive
   readability through incompatible bits, or through per-record criticality or a compatible/read-only tier.

## Ledger action

Recorded as addendum rows in `research/audit/addenda/orchestrator-findings.jsonl` (requirement + decision, cluster
`container`) for inclusion at the next ledger assembly. Production code and the test are unchanged, per program
discipline; a test-portability fix is proposed in the decision record.
