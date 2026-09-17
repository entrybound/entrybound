# EXP-INT-008: Sidecars and external references: missing, renamed and stale originals, locator resolution security, and sidecar overhead

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T9 content-free sidecar and External-reference model, T10 `ebr-int-resolver-suite`, mutator for concurrent modification, tar-split build) |
| Domain | integrity (program §23; objective HC-05, HC-09, HC-15, HC-18, HC-07, OD-05, OD-12) |
| Decisions informed | DEC-INT-013, DEC-INT-014, DEC-INT-026, DEC-INT-036, DEC-INT-045, DEC-LEG-056, DEC-MOD-046, DEC-CRY-093, DEC-LEG-025, DEC-INT-005, DEC-INT-037, DEC-MOD-001 |
| Gates, method, author | conventions C0 |
| Evidence label | Content-bearing sidecars are production behaviour (B-PROD `ebound sidecar`); content-free sidecars, External references, locators and resolvers are `MODEL` (T9, T10). |
| Specs | `spec.yaml` (Linux scenario and resolver matrix); `spec-windows.yaml` (NTFS concurrency, junction and case-fold locator cases) |

## 1. Question

For legacy artifacts with sidecars and for archives that reference external originals, how
reliably does each candidate design detect and report missing, renamed, moved, truncated,
modified, re-encoded and concurrently changing originals, can unaffected entries still be
extracted safely, do locator resolution policies stay confined under hostile locators and
origins, and what does each sidecar content model cost in bytes and verification work?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-INT-014 | Under `refuse-whole-operation`, localization precision equals the affected share of entries; `per-entry-status-partial` reaches precision 1 with recall 1 over the mutation set; `extract-mismatched-as-verified` produces HC-15 violations on every modified-original case. | `per-entry-status-partial` recall < 1 (R1) or no precision gain. |
| H2 | DEC-INT-013, DEC-INT-026 | Every automatic resolution policy (`auto-fetch-complete`, `locator-fallback` without confinement) follows at least one hostile fixture (loopback or link-local redirect, `../` escape, junction escape, `file://`), a confinement escape or origin-protocol failure; `explicit-opt-in-fetch` with digest and length verification and `local-relative-only` with confinement have zero escapes. | Zero escapes for automatic policies. |
| H3 | DEC-INT-036 | A content-free sidecar costs less than 1% of the source bytes on every INT-LEGACY item (T-01 band scale) while the content-bearing status quo costs roughly the compressed content size; both detect every byte-level modification of the legacy artifact, but only re-import-based verification distinguishes semantically equal re-encodes from content changes. | Content-free sidecar misses a modification detected by the content-bearing sidecar. |
| H4 | DEC-INT-045 | `sidecar-exact-source-digest` is cheaper than `sidecar-reimport-lai` by more than 1 band unit of T-04 on medium and large legacy artifacts, and reports every re-encode as a mismatch (false alarms for semantically equal files). | Not distinguishable in cost. |
| H5 | DEC-LEG-056 | Under a seeded concurrent mutator (rewrite, truncate, rename-over, extend), status quo `ebound sidecar` either refuses or binds one coherent version; it never binds a mixed-version snapshot. On Windows without share-mode locking the race window is observable. | Any mixed-version binding (HC-07 MVT-07(d) violation; defect). |
| H6 | DEC-CRY-093 | An attacker who replaces the legacy artifact and regenerates an unsigned sidecar passes every unsigned verification (undetected), while a signed sidecar under a caller-trusted key detects it. | Unsigned sidecar substitution detected. |
| H7 | DEC-MOD-046 | For ambiguous ZIP fixtures, strict and compatibility imports yield different LAIs or strict refuses on at least one fixture, so `describes_lai` without recorded import mode and profile is ambiguous. | Identical LAIs on all fixtures. |
| H8 | DEC-LEG-025 | tar-split side data costs under 1% of tar size and recomposes uncompressed tar byte-identically on INT-LEGACY tar members, but cannot recompose compressed tar bytes without compressor reproduction. | Recomposition fails on uncompressed tar, or succeeds on compressed tar. |

## 3. Decisions and candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-INT-013 | `no-locator`, `advisory-locator-outside-identity`, `locator-in-lai`, `typed-locator`, `multiple-locators`, `locator-in-receipt-only` | T9 identity effects (signature stability when a mirror URL or a renamed original changes the locator: `locator-in-lai` changes LAI, others do not) and T10 resolution security per locator type. |
| DEC-INT-014 | `refuse-whole-operation`, `per-entry-status-partial`, `digest-search-paths`, `locator-fallback`, `new-outcome-class`, `extract-mismatched-as-verified` | Mutation matrix (section 4) through T9 resolvers. Proposed L0 exclusion of `extract-mismatched-as-verified` (HC-15: unverified output presented as verified); negative control. `new-outcome-class` is a taxonomy change evaluated for code specificity and C1 cost; the reason-code taxonomy review is the model domain's. |
| DEC-INT-026 | `status-quo-no-fetch`, `explicit-opt-in-fetch`, `local-relative-only`, `caller-resolver-api`, `post-v1-protocol`, `auto-fetch-complete` | T10 hostile suite; bytes, requests and modelled latency (§4.15) of explicit fetches through `ebr-origin`; offline decode of Complete archives under `unshare -n` (MVT-09(c)). Proposed R1 exclusion of `auto-fetch-complete`: Complete archives must decode offline (HC-09 statement, SPEC §24 item 16 as cited in the ledger); negative control. |
| DEC-INT-036 | `status-quo-content-bearing`, `content-free-sidecar-role`, `both-modes`, `detached-digest-manifest`, `receipts-only`, `new-sidecar-format` | Size (T-01) and verification workflow over the mutation matrix. `new-sidecar-format` conflicts with the SPEC §15.4 L1718 rule cited in the ledger ("no new extension, format or parser"); proposed R1 exclusion pending sign-off (not run). |
| DEC-INT-045 | `status-quo-none`, `verify-against-lai`, `verify-against-tiered`, `sidecar-exact-source-digest`, `sidecar-reimport-lai`, `fold-into-diff` | Outcome matrix and cost. Directory `--against` cases use INT-SERIES version pairs (modified, renamed, extra, truncated files). `fold-into-diff` is measured with B-PROD `ebound diff --json` between the sidecar and a fresh strict import. |
| DEC-LEG-056 | `status-quo-content-bearing-zip-tar-7z`, `add-dry-run-and-encryption`, `normative-naming-discovery`, `lock-or-rehash-snapshot`, `content-free-sidecar-role`, `defer-tar-7z-modes` | Concurrent-modification arm (H5) on Linux and Windows for status quo and `lock-or-rehash-snapshot` (shim: share-mode lock on Windows, re-hash after read on both); preservation overhead (T-01) for ZIP `--preserve`; naming discovery survival is EXP-INT-011. Encryption composition is the crypto domain's (§22.5). |
| DEC-MOD-046 | `record-mode-and-profile`, `strict-only`, `unverified-claim`, `not-in-v1` | LAI divergence census over ambiguous ZIP fixtures: `f20-tuning-generated-malicious-structures`, `f20-tuning-real-libarchive-testsuite` ZIP members, and fixtures generated by `tools/zip-compat/generate_corpus.py` (run read-only into scratch, never regenerating tracked outputs). |
| DEC-CRY-093 | `status-quo-unsigned`, `detached-ebsig-over-receipt`, `dsse-in-toto-receipt`, `signed-sidecar-native`, `wording-correction-only`, `embed-signature-in-legacy-target` | Substitution attack demonstration (H6) and exact signature byte cost (`artifact_bytes` delta) with B-PROD `ebound sign --detached` and `--embed`; DSSE envelope size computed from its specification (MODEL). `embed-signature-in-legacy-target` conflicts with `docs/legacy-export-v1.md` L51-52 cited in the ledger (proposed R1 exclusion; not run). Forged-receipt threat model is analytical (crypto domain dossier). |
| DEC-LEG-025 | `status-quo-zip-snapshot-only`, `full-snapshot-all-formats`, `tar-split-side-data`, `external-sidecar-only`, `not-offered` | Side-data size and recomposition exactness with tar-split (downloaded source pinned by commit, BSD-3-Clause, built with the installed Go toolchain) on tar members of INT-LEGACY; full snapshot size = source bytes (exact); external sidecar size from the status quo sidecar. |
| DEC-INT-005, DEC-INT-037, DEC-MOD-001 | as EXP-INT-007 | Sidecar-role cells of the per-command matrix and the Sidecar LAI property. |

## 4. Corpus and scenario matrix

- Legacy artifacts: INT-LEGACY-T (tuning), INT-LEGACY-V (validation look). Each archive file
  inside an item (zip, jar, war, whl, tar, tar.gz, tar.zst, tar.xz, 7z, deb, apk, rpm payloads
  where `ebound convert` supports the format) is one case; unsupported formats are listed and
  skipped.
- Directory references: INT-SERIES-T and INT-SERIES-V version pairs.
- Mutations of the original: none; renamed; moved to another directory; truncated by 1 byte
  and to 50%; one byte modified inside one member's data; member added (re-packed with the same
  tool); timestamp-only change; semantically equal re-encode (same members, different
  compression level); deleted; replaced during verification (time-of-check to time-of-use).
- Hostile locators and origins: T10 fixture list, committed as `fixtures/hostile-locators.json`
  before any run.

## 5. Environment and platform requirements

`wsl-ubuntu` ext4 (Linux arms), root for `unshare -n`; `windows-host` NTFS on D: (concurrency,
junctions created with `mklink /J` without admin, case-fold aliases, UNC-form locators refused
without network access); `ebr-origin` on localhost only; no external network. `timing: false`,
except the DEC-INT-045 cost comparison, which runs as a small timing sub-spec in the EXP-INT-004
quiet windows (same controls; generated from this protocol when tooling exists). APFS arms are
EXP-INT-019 (BLOCKED).

## 6. Procedure and commands

1. Build tar-split at a pinned commit into `/root/eb-research/tools/tar-split/` and record its
   SHA-256 in `research/baselines/tool-versions.json` (probe script extension).
2. For each legacy case: status quo `ebound sidecar <artifact> <out.eb> --strict` (and
   `--compat=<profile>` and `--preserve --compat=<profile>` where supported); model content-free
   sidecar and detached manifest from T9; apply every mutation; run every verification candidate.
3. Resolver suite: T10 fixtures against every DEC-INT-013 and DEC-INT-026 policy.
4. Concurrency arm: the MVT-07(d) mutator at seeded delays during sidecar creation (200 delays
   per case, Linux and Windows).
5. All through ebr: `spec.yaml` (Linux) and `spec-windows.yaml`, then `verify-raw`,
   `verify_detail.py`, `normalize`, `report`.

## 7. Seeds, warmups, replication

Seeds 2026091771/72/73 (Linux), 2026091774/75/76 (Windows); mutator delays and hostile orders per
C8. Warmups 0; 2 repetitions with repeat-hash, except the concurrency arm, which is stochastic by
nature: its binary oracle counts over all 200 delays per repetition are reported per repetition
and summed (regression evidence, no detection bound claimed beyond the seeded schedules, as
MVT-07(d) states).

## 8. Metrics

| Metric | ID | OD | Role |
|---|---|---|---|
| `localization_recall` | HC-15 | OD-14 floor | binary (entries affected by a mutated original are reported) |
| `localization_precision` | T-20 | OD-14 | **primary** for DEC-INT-014 |
| `mvt15a_overclaim_cells` | C11 | HC-15 | binary |
| `confinement_escapes` | C11 | HC-05 | binary for DEC-INT-013 and DEC-INT-026 policies |
| `origin_protocol_safety` | HC-18 | OD-12 floor | binary |
| `adversarial_true_refusal_fraction` | HC-06 | OD-17 floor | binary (oversized and slow bodies refused within bounds) |
| `artifact_bytes` | T-01 | OD-05 | **primary** for DEC-INT-036, DEC-LEG-025, DEC-CRY-093 (sidecar, side data, signature bytes) |
| `side_data_bytes` | T-02 | OD-05 | diagnostic |
| `http_requests`, `http_bytes` | T-12, T-11 | OD-12 | **primary** `http_requests` for DEC-INT-026 fetch policies; `http_bytes` secondary |
| `verify_wall_s` | T-04 | OD-07 | **primary** for DEC-INT-045 cost (timing sub-spec) |
| `undetected_tampers` | HC-14 | OD-15 floor | binary for DEC-CRY-093 signed candidates; for unsigned candidates the count is reported as the demonstration |
| `import_agreement_fraction` | T-20 | OD-19 | **primary** for DEC-MOD-046 (fraction of fixtures whose strict and compat LAIs agree) |
| `receipt_completeness_fraction` | HC-07 | OD-20 floor | binary for DEC-LEG-056 concurrency (mixed version never bound without declaration) |
| recomposition exactness count | none | - | descriptive for DEC-LEG-025 |

## 9. Normalization

Sidecar and side-data bytes: ratio to the legacy artifact bytes per case, then paired log ratio
against the status quo sidecar (T-01 floors by tier of the legacy artifact). Coverage fractions:
exact per case set. Requests and bytes: paired log ratio against `status-quo-no-fetch` with a
local copy (zero requests) handled as an absolute difference (T-12 absolute floor).

## 10. Statistical analysis and sensitivity

C10. INT-LEGACY meets §4.9 support only partially (9 and 8 items over 4 families); banded
corpus-level verdicts are secondary (C3) and the decision records the gap unless the legacy
domain's larger artifact sets are added by revision before the first look. Sensitivity: mutation
mix arm (byte-level mutations only versus all mutations), format arm (ZIP family versus tar
family versus 7z, never pooled for DEC-LEG-056), host arm (Linux versus Windows for concurrency
and resolver cases).

## 11. Expected negative results worth recording

- Exact-source digest verification raises false alarms on semantically equal re-encodes.
- Automatic locator resolution is unsafe under at least one hostile fixture.
- Content-bearing sidecars roughly double storage for already-compressed artifacts.
- Unsigned sidecars do not authenticate anything against an active attacker (terminology
  correction needed regardless of the signing decision).

## 12. Threats to validity

- Content-free sidecars and resolvers are models; parser and implementation risks of a real
  External-reference reader are not measured (OD-24 cost inputs only).
- The hostile fixture list is an enumeration, not completeness (regression evidence).
- Windows junction and case-fold behaviour depends on NTFS settings of D:; the volume's case
  sensitivity flags are recorded in the environment capture.
- tar-split is one implementation of side-data recomposition; others might differ.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12.

## 14. Cost estimates

- Compute: about 150 legacy cases x 10 mutations x 8 verification candidates (small to medium
  files), resolver suite about 60 fixtures x 10 policies, concurrency 200 delays x cases: about
  40 CPU-hours Linux, 10 CPU-hours Windows.
- Disk: about 30 GB (sidecars, mutated copies); D: about 8 GB.
- Agent effort: model and fixture authoring require judgment; execution none; analysis judgment.

## 15. Deviations (append-only)

None.
