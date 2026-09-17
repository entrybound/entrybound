# EXP-INT-012: Salvage prototypes: intact-entry recovery rate and false-recovery rate versus incumbent repair tools

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T6 `ebr-int-salvage`, damage corpus from T2 and T3 of EXP-INT-001/002, incumbent adapters T4, `gzrt` build for gzip recovery reference) |
| Domain | integrity (program §24, cross-reference §30, §33, §34; objective OD-14, HC-15, HC-02, HC-06) |
| Decisions informed | DEC-INT-035, DEC-INT-041 (salvage-only-reporting candidate), DEC-INT-015 (assessment-only candidate), DEC-INT-031 (repair-then-reencode candidate), DEC-INT-034 (report content) |
| Gates, method, author | conventions C0 |
| Spec | `spec.yaml` |

## 1. Question

On damaged Entrybound archives and damaged incumbent archives, how much intact logical content
can each salvage design recover as verified, does any design report an entry as intact whose
bytes are wrong, how bounded is salvage's work on hostile damaged inputs, and how does this
compare with `zip -FF`, 7-Zip extraction past errors and gzip stream recovery?

## 2. Hypotheses and falsification

| ID | Hypothesis | Falsified by |
|---|---|---|
| H1 | `intact-entry-extraction-only` recovers every oracle-intact entry when the manifest and descriptor survive (recovery fraction 1 on R-PAY single events) and none when they do not; false-recovery count 0. | Recovery below 1 with intact authority structures, or any false recovery (R1). |
| H2 | `chunk-carving-best-effort` recovers content of oracle-intact entries after R-META and footer damage in INDEXED archives only when a copy of entry-to-chunk mapping survives (for example STREAM manifest records emitted after data, or an intact Index), and its unverified fragments are always marked; it never labels a fragment verified. | Any fragment labelled verified without a content digest match (R1). |
| H3 | ZIP `zip -FF` plus `unzip` recovers per-entry content after central-directory damage where Entrybound INDEXED salvage cannot without a manifest (a negative result for Entrybound); 7z solid loses the remainder of the solid block after payload damage. | Entrybound recovers at least as many intact bytes on every stratum. |
| H4 | Salvage scanning cost is linear in archive size on hostile inputs (`hostile_cpu_scaling_exponent` interval lower bound at most the MVT-06(d) criterion) and memory stays within a declared bound. | Superlinear scaling or bound exceedance. |

## 3. Candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-INT-035 | `status-quo-not-implemented` (B-PROD `ebound salvage` exits with `EB_CLI_NOT_IMPLEMENTED`: recovery 0), `salvage-v1-best-effort-report`, `salvage-post-v1` (v1 offers only the verify damage map of EXP-INT-001/004; recovery = verified entries readable by random access, measured as M-SQ-READ recovery), `intact-entry-extraction-only`, `chunk-carving-best-effort`, `legacy-zip-local-header-salvage`, `unpack-recover-mode` (same engine as intact-entry extraction; differs in CLI surface, C6 cost) | T6 prototypes over the damage corpus. Fuzzing of the salvage parser is the conformance domain's (§30); stable-v1 disposition review is §34's. |
| DEC-INT-041 | `salvage-only-reporting` | Truncation cases (D-T1, D-T2) through T6; reported with EXP-INT-002's mechanisms. |
| DEC-INT-015 | `assessment-only-no-parity` | Recovery achievable without parity (this experiment) versus with external parity (EXP-INT-010) at the same damage, per damage class. |
| DEC-INT-031 | `repair-then-reencode` | Salvaged verified entries re-packed into a fresh archive: identity pattern (LAI equals the source LAI only when every entry was recovered) and verify outcome. |
| DEC-INT-034 | report content | Fraction of salvage reports that enumerate every lost entry and byte range (completeness) versus the oracle. |
| Incumbent references | `zip -FF` (Info-ZIP, version per `tool-versions.json`) then `unzip`; `7z x` continuing past errors (7-Zip 23.01); `gzrecover` from gzrt (downloaded at a pinned release, GPL-2.0, reference tool only, never shipped) then `tar --ignore-zeros -x`; `zstd -d` on damaged frames with `--no-check` recorded separately | T4 adapters; per-entry comparison against the source tree. |

## 4. Corpus

- Tuning: INT-CORE-T under `eb-indexed-balanced`, `eb-indexed-dense`, `eb-stream-balanced`, and
  incumbents `inc-zip-deflate-6`, `inc-tar-gz-6`, `inc-tar-zst-3`, `inc-7z-lzma2-solid`,
  `inc-7z-lzma2-nonsolid`; damage classes D-P1 (all strata), D-S2, D-S3, D-S4, D-S6, D-T1 (at
  most 500 cuts per archive, seeded subset of EXP-INT-002's cuts), D-T2 (200) with C8 seeds shared
  with EXP-INT-001 and EXP-INT-002 so that events are identical across experiments.
- Hostile: INT-HOSTILE-T plus generated worst cases (archives filled with forged `EBCH` magic
  every 64 bytes; frames whose declared lengths overlap).
- Validation look: INT-CORE-V and INT-HOSTILE-V for W and S.

## 5. Environment and platform requirements

`wsl-ubuntu`, `timing: false`, parallel workers; downloads under the approved policy with
SHA-256 recorded. No Windows arm (salvage logic is platform-independent; marker survival on
Windows is EXP-INT-011).

## 6. Procedure and commands

`ebr run` of `spec.yaml`; per sample the driver packs or reuses the archive, regenerates the
shared events, runs every salvage candidate and incumbent tool into fresh destinations, compares
recovered entries with the source tree and the C5 oracle, runs the hostile scaling series (3 runs
per point, 2^10 to 2^24 bytes of forged frames), writes detail rows. Then `verify-raw`,
`verify_detail.py`, `normalize`, `report`.

## 7. Seeds, warmups, replication

Seeds 2026091821/22/23; events per C8 (seed domain of EXP-INT-001 and EXP-INT-002). Warmups 0;
2 repetitions with repeat-hash on the recovery vector.

## 8. Metrics

| Metric | ID | OD | Role |
|---|---|---|---|
| `salvage_recovered_fraction` | C11 proposed | OD-14 | secondary until named; **primary candidate** for DEC-INT-035 non-truncation damage |
| `truncation_salvage_fraction` | T-20 | OD-14 | **primary** for truncation events |
| `roundtrip_mismatches` | HC-02 | OD-02 floor | binary: entries reported intact whose bytes differ (false recovery) |
| `mvt15a_overclaim_cells` | C11 | HC-15 | binary: fragments or entries labelled verified without the applicable digest |
| `bounded_memory_growth_bytes` | HC-06 | OD-08 floor | binary on hostile inputs |
| `hostile_cpu_scaling_exponent` | descriptive | OD-17 | descriptive |
| report completeness | `receipt_completeness_fraction` (HC-07) | OD-20 floor | binary: every lost entry and range enumerated |

## 9. Normalization

Recovered fractions per event (oracle-intact logical bytes recovered verified / oracle-intact
logical bytes), mean over events per item and stratum, equal stratum weights (C4); item-level
differences against `salvage-post-v1` (the verify-only reference) as absolute-only metrics (T-20).

## 10. Statistical analysis and sensitivity

C10. Sensitivity: damage-class arms (as EXP-INT-001 section 10), stratum weighting arm, layout
arm (INDEXED versus STREAM, never pooled), incumbent reference arm (REF-INC comparisons reported
beside REF-PROD, §6.5).

## 11. Expected negative results worth recording

- Without a surviving manifest, INDEXED Entrybound recovers less than ZIP with `zip -FF`, because
  ZIP repeats metadata in local headers.
- STREAM manifest-after-data helps carving but not after tail truncation.
- Chunk carving recovers fragments only as unverified data for dictionary- or group-dependent chunks.

## 12. Threats to validity

- Salvage designs are research prototypes; production salvage could recover more or less.
- Incumbent repair tools have version-specific heuristics (pinned).
- Oracle-intact definitions follow C5; alternative definitions would change recovery denominators
  (the definition is pre-registered to prevent that choice after results).

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12.

## 14. Cost estimates

- Compute: 8 configurations x 21 items x about 1,700 events x 2 repetitions, salvage passes about
  0.1-2 s each: about 80 CPU-hours; hostile series about 5 CPU-hours.
- Disk: about 40 GB scratch (recovered trees); detail about 3 GB.
- Agent effort: prototype build requires judgment (carving rules) with review; execution none;
  analysis judgment.

## 15. Deviations (append-only)

None.
