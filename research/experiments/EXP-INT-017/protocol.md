# EXP-INT-017: Legacy-artifact provenance verification: re-export reproducibility, strict re-import comparison, and the LOSSY predicted-difference gate

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T11(c) export-encoder mutation hook; EAM diff tool mapping differences to typed issues; driver `research/tools/integrity/export_provenance.py`); arm A (re-export reproducibility) needs only B-PROD builds and the driver |
| Domain | integrity (program §24, cross-reference legacy §27 and ecosystem §32; objective HC-07, HC-13, OD-19, OD-20) |
| Decisions informed | DEC-LEG-009, DEC-INT-023, DEC-INT-045 (re-import versus digest cost, shared with EXP-INT-008) |
| Gates, method, author | conventions C0 |
| Spec | `spec.yaml` (WSL; the Windows reproducibility arm is generated from it with Windows paths) |

## 1. Question

Can a consumer who holds an `.eb` archive verify that a legacy artifact (zip, tar, tar.gz, tar.zst,
tar.xz, tar.bz2) was produced from it, by byte-identical deterministic re-export (across builds,
hosts and source layouts), by strict re-import and LAI or issue-predicted comparison, or by a
signed receipt, and does the LOSSY export gate refuse to publish artifacts whose differences from
the source are not predicted by typed issues?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-LEG-009 | Re-export with the recorded target profile is byte-identical across repetitions, across INDEXED, STREAM and encrypted sources of the same EAM, and across WSL and Windows B-PROD builds for every LOSSLESS target (HC-13 MVT-13(d)); LOSSY targets differ across hosts only where host metadata differ in the EAM (F-0002). | Any byte difference for the same EAM and profile (HC-13 violation artifact). |
| H2 | DEC-LEG-009, DEC-INT-045 | Strict re-import plus LAI comparison detects every injected member-content change in LOSSLESS artifacts and costs more than 1 band unit of T-04 above hashing the artifact, on medium items. | Hash comparison not cheaper. |
| H3 | DEC-INT-023 | Under the status quo, at least one class of injected unpredicted difference (for example a dropped xattr or a changed mode bit in a LOSSY target) is published without refusal, because the comparator location is undocumented (ledger `status-quo-undescribed`); `predicted-issue-diff` refuses 100% of injected unpredicted differences with zero false refusals on unmutated exports. | Status quo refuses every injected class. |

## 3. Candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-LEG-009 | `status-quo-unsigned-receipt`, `reexport-byte-compare`, `strict-import-lai-compare`, `signed-receipt`, `signed-archive-embeds-result-digests` | Arm A reproducibility and arm B detection of injected artifact changes (member content byte, member added, member removed, metadata-only change, re-compression with other level). `signed-receipt` cost: exact detached signature bytes over the receipt (B-PROD `ebound sign` cannot sign arbitrary JSON; cost modelled from the Ed25519 signature record size, labelled MODEL). `signed-archive-embeds-result-digests` is a crypto wire change (T4) evaluated as a detection model only. Security review of receipt forgery is the crypto domain's (§22.4). |
| DEC-INT-023 | `status-quo-undescribed`, `predicted-issue-diff` (prototype), `field-mask-comparison` (prototype), `explicit-diff-report-gate` (same detection as `predicted-issue-diff`; differs in CLI surface, C6), `encoder-success-only` | Arm C mutation tests: the T11(c) hook injects, per export target, unpredicted differences (drop an xattr, change a mode bit, change mtime precision, rename an entry, change one content byte, omit an entry, change a symlink target, reorder entries where order is semantic) into the encoder output before the gate; the gate outcome is recorded. A code-review sub-arm locates the status quo comparator (`crates/entrybound/src/legacy`, cited as `path:line`) and records what it compares. Proposed L0 exclusion of `encoder-success-only`: `docs/compressed-tar-export-v1.md` "Verification" L44-49 as cited in the ledger (encoder completion insufficient) and HC-07; negative control. |
| DEC-INT-045 | cost part of `sidecar-reimport-lai` versus `sidecar-exact-source-digest` for native-to-legacy direction | Arm B cost (timing sub-spec in an EXP-INT-004 window). |

## 4. Corpus

- Tuning: `f01-tuning-ripgrep-14-1-1-git`, `f04-tuning-sourcelike`, `f05-tuning-gutenberg-books`,
  `f09-alpine-324-rootfs-binaries`, `f12-tuning-kodak-jpeg-edge-cases-small`, `f19-tuning-zoo`,
  `f19-tuning-real-home-snapshot`.
- Validation look: `f01-validation-redis-7-2-16-git`, `f05-validation-rfc-texts`,
  `f09-cpython-3147-embed-win-amd64`, `f12-validation-fsa-jpeg-edge-cases-small`,
  `f19-validation-deep`, `f19-validation-real-home-snapshot`.
- Targets: every `convert --to` target and every exact target profile the CLI lists at the
  pre-registration commit (the list is captured into `targets.json` before any run).

## 5. Environment and platform requirements

`wsl-ubuntu` and `windows-host` (D:) for arm A; WSL for arms B and C; emulated aarch64 via
`qemu-user-static` optional for arm A (EMULATED, correctness only). `timing: false` except the arm B
cost sub-spec.

## 6. Procedure and commands

1. Capture `targets.json` from `ebound convert --help` and the profile registry.
2. `ebr run` of `spec.yaml` (candidates are arms), `verify-raw`, `verify_detail.py`, `normalize`,
   `report`; the Windows arm A spec runs the same driver with Windows paths and its outputs are
   compared by `research/tools/integrity/export_provenance.py compare-hosts`.

## 7. Seeds, warmups, replication

Seeds 2026091871/72/73; mutation positions per C8. Warmups 0; 2 repetitions per host with
repeat-hash on artifact SHA-256 sets (arm A) and outcome vectors (arms B and C).

## 8. Metrics

| Metric | ID | OD | Role |
|---|---|---|---|
| `migration_determinism_fraction` | HC-13 | OD-20 floor | binary (arm A) |
| `cross_host_identity_agreement` (artifact bytes for LOSSLESS targets) | HC-08 | OD-18 floor | binary for LOSSLESS targets; reported for LOSSY |
| `receipt_completeness_fraction` | HC-07 | OD-20 floor | binary (arm C: every published difference predicted by a typed issue) |
| `export_outcome_distribution` | descriptive | OD-19 | descriptive |
| `undetected_tampers` (arm B, per verification candidate) | HC-14 extended to bindings | OD-15 floor | binary for candidates claiming provenance verification |
| `verify_wall_s` | T-04 | OD-07 | **primary** for DEC-INT-045 cost (sub-spec) |
| `artifact_bytes` (receipt and signature bytes) | T-01 | OD-05 | secondary |

## 9. Normalization

Exact tallies; cost as paired log ratios within rounds (sub-spec).

## 10. Statistical analysis and sensitivity

Exact binary tallies; R1 for violations; selection among violation-free candidates by cost
(sub-spec) and R10. Sensitivity: target arm (per target, never pooled; AGG-S by legacy format),
source-layout arm, host arm.

## 11. Expected negative results worth recording

- Some LOSSY targets are not reproducible across hosts because the source EAM differs (F-0002).
- The status quo LOSSY gate is not locatable or not complete for some difference classes.
- Strict re-import is costly on large artifacts compared with digest comparison.

## 12. Threats to validity

- Mutation hooks inject differences at the encoder boundary; bugs earlier in the pipeline are
  not covered.
- Cross-host comparison depends on mirrored items being identical (verified by
  `logical_tree_sha256`) but captured metadata differs by host.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12.

## 14. Cost estimates

- Arm A: 7 items x about 10 target profiles x 3 source layouts x 2 hosts x 2 repetitions: about 8
  CPU-hours per host.
- Arms B and C: about 12 CPU-hours.
- Disk: about 20 GB per host.
- Agent effort: hook and diff tool judgment with review; code-review sub-arm judgment; execution none.

## 15. Deviations (append-only)

None.
