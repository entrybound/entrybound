# EXP-INT-013: Stored-payload canonicality and legacy integrity-check strictness

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T12 `ebr-int-reseal`, T15 `ebr-int-fixtures`, reader-option prototypes for single-frame enforcement and stored-bytes checks, prevalence scanner) |
| Domain | integrity (program §24, cross-reference §25, §29, §30; objective HC-03, HC-06, HC-07, HC-15, OD-17, OD-19) |
| Decisions informed | DEC-INT-008, DEC-INT-022, DEC-LEG-078, DEC-LEG-021 |
| Gates, method, author | conventions C0 |
| Spec | `spec.yaml` (arm A native payloads and arm B legacy fixtures and prevalence scan as candidates) |

## 1. Question

Does the status quo reader accept non-canonical stored payloads (extra frames, skippable frames,
trailing bytes) as valid, how much work does it do before rejecting damaged or crafted payloads,
what would stored-bytes digests or per-frame checksums add, and, for legacy import, which
integrity checks that formats make optional (zstd content checksums, xz check types, CRC-32 and
declared-size agreement, gzip ISIZE) should strict and compatibility profiles require, at what
benign false-refusal rate on real corpora?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-INT-008 | With section and body digests recomputed (T12), a zstd payload followed by a skippable frame or trailing bytes is accepted by the status quo bulk decoder when the decoded plaintext still matches the chunk digest (non-unique interpretation of stored bytes, an HC-03 concern); `enforce-single-frame-no-trailing` refuses all such cases. | Status quo refuses every crafted non-canonical payload. |
| H2 | DEC-INT-008 | For STREAM chunk frames with damaged stored bytes, the status quo decodes before any stored-byte check, spending decoder CPU and memory proportional to the declared decode requirement before `CORRUPT`; a stored-bytes digest or CRC32C rejects before decode with `refusal_cpu_s` lower by more than 1 band unit of T-05, at an exact wire cost of 32 B or 4 B per frame. | Refusal cost not distinguishable. |
| H3 | DEC-INT-022 | Checksum-less zstd frames with corrupted literals and xz streams with `check=None` import as valid content under the status quo (silent wrong bytes, an HC-07 and I21 gap at the transport layer); `strict-require-checksums` refuses them; on real corpora the share of such streams is small but non-zero (for example zstd frames written without checksums by some packagers), giving a measurable `benign_false_refusal_fraction`. | No checksum-less stream exists in INT-LEGACY, or the status quo refuses corrupted checksum-less streams. |
| H4 | DEC-LEG-078 | CRC-collision size cases import with different content selections under different compatibility profiles; `refuse-all-size-disagreement` refuses all of them with `ambiguity_refusal_fraction` = 1. | All profiles agree. |
| H5 | DEC-LEG-021 | The status quo gzip ISIZE comparison (`unwrap_or(u32::MAX)`, per the ledger) refuses or misreports a valid gzip member whose decoded size exceeds 4 GiB; the modulo 2^32 comparison accepts it and still detects a size mismatch. | Status quo accepts the > 4 GiB member. |

## 3. Candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-INT-008 | `status-quo-bulk-decode`, `enforce-single-frame-no-trailing` (T11(d) reader option), `stream-stored-bytes-digest` (research per-frame stored-bytes digest map as stand-in; wire cost exact, MODEL), `per-frame-crc32c` (same stand-in with CRC32C), `hardened-decoders-and-budgets` (status quo plus the conformance domain's fuzzing evidence, cited), `accept-trailing-bytes-silently` | Crafted payloads per codec and plan mode (zstd independent, dictionary and prefix modes; LZ4; LZMA2; STORE) with T12 resealing: appended skippable frame, appended data frame decoding to 0 and to n bytes, trailing garbage 1 B and 4,096 B, frame content-size field disagreement, checksum flag toggled, dictionary id mismatch. Proposed L0 exclusion of `accept-trailing-bytes-silently`: it gives the same stored bytes two admissible encodings (HC-03 unique interpretation; HC-01 stored length authority); negative control. Decoder CVE history review is a primary-source survey arm (`survey/decoder-cves.md`: zstd, xz, lz4, LZMA SDK, brotli, flate2 and miniz_oxide advisories with CVE, RUSTSEC or GHSA identifiers). |
| DEC-INT-022 | `status-quo-present-checks-only`, `strict-require-checksums`, `report-unverified-layers`, `refuse-xz-check-none`, `crc-safety-all-profiles`, `crc-plus-size-invariant` | Fixture outcomes under `ebound convert --strict` and every `--compat=<profile>`; the non-status-quo candidates are rule vectors applied to fixture properties recorded by T15, plus a prototype strict flag where the rule is a pure refusal. Prevalence scan over INT-LEGACY and INT-LEGACY-V real items: count and byte share of zstd frames without content checksum, xz streams by check type, gzip members with ISIZE disagreement, ZIP entries with CRC or size disagreement. |
| DEC-LEG-078 | `status-quo-mixed`, `faithful-truncation`, `refuse-all-size-disagreement`, `import-with-conflict-report`, `dual-projection-preserve-only`, `per-profile-reviewed-override` | CRC-collision size fixtures (STORE and DEFLATE; declared size shorter and longer) through every profile; pinned runtime outcomes (CPython zipfile, Java ZipFile, libarchive) are the legacy domain's differential campaign (§25) and are cited, not re-measured. |
| DEC-LEG-021 | `status-quo-saturating`, `modulo-2-32`, `modulo-plus-declared-size-cap` | Generated gzip member of 4.5 GiB zeros (and one of 4 GiB + 1 B random-seeded text) imported with `ebound convert --from gzip --entry-name big.bin`; ISIZE-corrupted variant; tar.gz with a > 4 GiB entry exported with `convert --to tar.gz` and re-imported. The fix candidates are evaluated by a research reader option implementing each comparison on the same streams. |

## 4. Corpus

- Arm A: INT-SMALL-T items packed under balanced, dense (dictionaries, prefix groups) and extreme
  profiles, INDEXED and STREAM; one crafted mutation per (frame class, mutation type) for up to
  200 frames per archive. Validation: INT-SMALL-V.
- Arm B fixtures: generated by T15 (seed 2026091831), committed as a fixture manifest with SHA-256
  before any run; prevalence scan: INT-LEGACY-T (tuning) and INT-LEGACY-V (validation), all
  archive and compressed-stream files inside those items.
- ISIZE arm: generated streams in `/root/eb-research/scratch/int-013/` (about 9 GB decoded
  transiently).

## 5. Environment and platform requirements

`wsl-ubuntu`, `timing: false` for outcomes; refusal CPU and allocation are in-process counters in
the research reader (deterministic enough for ordering; decision-grade timing of refusal cost, if
needed, is scheduled in an EXP-INT-004 quiet window). zstd 1.5.5 and xz 5.4.5 CLIs for fixture
generation (pinned). No Windows arm.

## 6. Procedure and commands

1. Commit the fixture manifest and `survey/decoder-cves.md` before runs.
2. `ebr run` of `spec.yaml` (candidates: `arm-a-native`, `arm-b-legacy-fixtures`,
   `arm-b-prevalence`, `arm-isize`), then `verify-raw`, `verify_detail.py`, `normalize`, `report`.

## 7. Seeds, warmups, replication

Seeds 2026091831/32/33; frame selection per C8. Warmups 0; 2 repetitions with repeat-hash of
outcome vectors; refusal CPU is reported as the median of the 2 repetitions and is secondary.

## 8. Metrics

| Metric | ID | OD | Role |
|---|---|---|---|
| accepted non-canonical payloads | none (HC-03 oracle count) | OD-04 floor | binary for DEC-INT-008 candidates claiming canonicality |
| `reader_disagreements` | HC-03 | OD-04 floor | binary: two admissible stored encodings accepted for one chunk |
| `refusal_cpu_s`, `refusal_alloc_peak_bytes` | T-05, T-06 | OD-17 | **primary** `refusal_alloc_peak_bytes` for DEC-INT-008 (OD-17); `refusal_cpu_s` secondary |
| `artifact_bytes` | T-01 | OD-05 | **primary** for DEC-INT-008 wire candidates (exact per-frame cost, MODEL) |
| wrong bytes imported silently | `roundtrip_mismatches` (HC-02) | OD-02 floor | binary for DEC-INT-022 candidates claiming integrity |
| `benign_false_refusal_fraction` | T-20 | OD-17 | **primary** for DEC-INT-022 |
| `ambiguity_refusal_fraction` | HC-07 | OD-19 floor | binary for DEC-LEG-078 |
| `import_acceptance_fraction` | T-20 | OD-19 | **primary** for DEC-LEG-078 and DEC-LEG-021 (valid fixtures accepted) |

## 9. Normalization

Coverage fractions exact per fixture set; refusal cost as paired log ratios against the status
quo on identical crafted inputs (T-05 and T-06 floors).

## 10. Statistical analysis and sensitivity

Exact tallies for binaries and fractions; refusal-cost verdicts are secondary here (INT-SMALL
support, C3) unless re-run on INT-CORE in an EXP-INT-004 window. Sensitivity: codec arm (never
pooled), prevalence corpus arm (tuning versus validation items), profile arm (strict versus each
compat profile).

## 11. Expected negative results worth recording

- The status quo accepts some non-canonical stored payloads.
- Strict checksum requirements refuse a real, non-zero share of benign packages.
- The > 4 GiB gzip member case fails under the status quo.

## 12. Threats to validity

- Resealed crafted archives are adversarial constructions; accidental damage rarely produces them.
- Prevalence counts depend on the corpus's packager mix (Ubuntu, Alpine, Fedora, PyPI, Maven, OCI).
- Stored-bytes digest and CRC stand-ins do not capture wire placement effects.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12 (prevalence re-scan on held-out legacy items after unlock).

## 14. Cost estimates

- Compute: arm A about 10 CPU-hours; arm B fixtures about 2 CPU-hours; prevalence scan about
  10 CPU-hours (decompressing INT-LEGACY streams); ISIZE arm about 1 CPU-hour.
- Disk: about 30 GB transient.
- Agent effort: fixture and reseal tooling judgment with review; survey judgment (primary sources);
  execution none; analysis judgment.

## 15. Deviations (append-only)

None.
