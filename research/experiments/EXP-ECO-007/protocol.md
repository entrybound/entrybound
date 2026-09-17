# EXP-ECO-007: Verification, supply-chain composition and artifact-association workflows

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-007 |
| Domain | ecosystem (program §32; cross-cuts §22.4 signatures, §23-§24 sidecars and parity, §27 receipts) |
| Status | READY (runnable variants use the implemented CLI plus pinned `cosign`, `minisign`, `signify-openbsd`, `gpg`, `in-toto`, `pycose`, `par2`, MinIO server and client; binding prototypes are scripts over existing outputs) |
| Runner | `spec.yaml` (scenario × item); tamper and copy-channel matrices run inside each sample |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |
| Timing | none (verification cost is EXP-ECO-008) |

## 1. Question

When release artifacts leave the producer — detached signatures, sidecars, receipts, parity files, external attestations — which association and binding designs let a consumer (a) find the companion files after realistic copy and transfer channels, (b) detect substitution, replay, modification, truncation and separation, (c) keep valid bindings across benign operations (representation-only repack, index rebuild, key rotation, export and re-import) exactly as SPEC §8.1 prescribes, and (d) compose with Sigstore, in-toto/SLSA, COSE and OpenPGP tooling — and at what command, flag and byte cost?

## 2. Hypotheses and falsification

- **H1 (identity conformance).** For every benign operation in §7 (representation-only repack, `--index present|absent`, `key add`, `key remove`, `change-password`, export then strict re-import), the change pattern of LAI, PCR, AUX and each signature binding status matches the SPEC §8.1 table cell for cell. *Falsified* by any cell that differs (`migration_identity_conformance_fraction` < 1, an HC-10 violation).
- **H2 (unsigned receipts do not resist substitution).** Status-quo unsigned receipts and filename conventions fail to detect at least the replay case T3 and the valid-substitute case T1 when the attacker controls every published file. *Falsified* if they detect both.
- **H3 (signed bindings detect tampering).** Every variant whose consumer check verifies a signature by a pinned authorized key detects every tamper case T1-T6 (`undetected_tampers` = 0). *Falsified* by any undetected tamper (an HC-14 violation for that variant as specified).
- **H4 (separation through copy channels).** Filename-convention association survives `cp`, `rsync -a`, `tar` and HTTP download of each file, but at least one channel (object-store copy of a single object, `zip`/`unzip` of only the primary artifact, Windows copy of only the primary artifact) silently separates the companion files, and no status-quo command reports the separation. *Falsified* if every channel keeps association or every separation is reported.
- **H5 (external composition).** An in-toto statement whose subject is the `.eb` PCI digest stays valid under representation-only repack only if a second subject carries LAI; a statement over PCI alone becomes stale. *Falsified* if a PCI-only statement survives repack (would indicate PCI unchanged by repack).

## 3. Decisions informed

DEC-ECO-072, DEC-CRY-003, DEC-CRY-018, DEC-CRY-019 (third-party verification workflow), DEC-INT-011, DEC-INT-028, DEC-INT-045, DEC-LEG-009, DEC-LEG-045, DEC-LEG-018 (reproduction from receipt fields across builds), DEC-ECO-062 (consumer verification of sidecars).

## 4. Candidates

Runnable variants (reference = status quo at dev HEAD, with REF-PROD `9e44608` beside it):

| Decision | Variants run | Not run here (reason; where measured) |
|---|---|---|
| DEC-CRY-018 | status-quo-single-record-default-path (`ebound sign --detached`); per-signer-suffix-files (naming convention `<artifact>.<signer-id>.ebsig`, three signers); sigstore-bundle-compatible-sidecar (`cosign sign-blob --key <scratch> --bundle <artifact>.sigstore.json --tlog-upload=false`) | multi-record-ebsig-v1, bundle-new-version, embed-uniqueness-per-transcript, media-type-registration: not implemented (synopses in EXP-ECO-018; extension collisions in EXP-ECO-012) |
| DEC-ECO-072 | status-quo-native; in-toto-subjects (in-toto v1 statement with subjects PCI digest and LAI, signed with a scratch key, verified with the `in-toto` Python package); sigstore-bundles (as above); minisign-compat (`minisign -S` over the `.eb`); verify-third-party-attestations (consumer runs `cosign verify-blob`/`in-toto` itself) | registry-attestations: requires registry accounts (EXP-ECO-010/EXP-ECO-022) |
| DEC-CRY-003 | status-quo-none; dsse-in-toto-member emulated as an external DSSE envelope over LAI; sigstore-bundle-sidecar; cose-sign1-export (`pycose` COSE_Sign1 over LAI); external-digest-signing (`minisign` over a digest file listing PCI, LAI, PCR); openpgp-signatures (`gpg --detach-sign` over the `.eb`) | keyless-sole-path: needs an OIDC identity and public transparency-log upload (publishing; excluded) |
| DEC-CRY-019 | status-quo-detached-default; embedded-default-for-encrypted (`ebound sign --embed` on an encrypted archive); third-party verifier without keys | warn-on-detached, refuse-detached-without-flag, new-version forms: behaviour changes not implemented (EXP-ECO-016 records status-quo default; EXP-ECO-018 counts) |
| DEC-INT-011 | filename-convention-only; bind-pci (SHA-256 manifest of the `.eb`); bind-lai-plus-signature (`ebound sign`); par-native-file-hash (`par2 create` over the `.eb`, association by PAR2 internal file hash); receipt-or-detached-signature | bind-section-chunk-digests, wrapper-index-artifact: not implemented; design cost assessed by the integrity domain |
| DEC-INT-028 | status-quo-compute-only (`ebound verify`); expected-pci-option emulated (`sha256sum -c` of a published digest before `ebound verify`); signed-pci-binding (`ebound sign` with physical binding) | digest-before-parse as an in-tool option: not implemented; parser-differential analysis belongs to the integrity domain |
| DEC-INT-045 | status-quo-none; sidecar-exact-source-digest (compare artifact SHA-256 with the sidecar provenance digest); sidecar-reimport-lai (strict re-import of the artifact and LAI comparison with the sidecar); fold-into-diff (`ebound diff` of re-imported and sidecar archives) | verify-against-lai/tiered as in-tool options: not implemented (counts in EXP-ECO-018) |
| DEC-LEG-009 | status-quo-unsigned-receipt; reexport-byte-compare (consumer re-exports from the `.eb` and compares bytes); strict-import-lai-compare; signed-receipt emulated (`minisign` over the receipt) | signed-archive-embeds-result-digests: not implemented |
| DEC-LEG-045 | status-quo-no-chain; embed-upstream-provenance (inspect whether export receipts carry the import provenance); receipt-chain-command prototype (`chain_assemble.py` over supplied receipts and provenance, tamper-checked); in-toto-layout-chain (layout with steps `import` and `export`) | defer-post-v1: no measurement |
| DEC-LEG-018 | reproduction of an export from receipt fields alone with REF-PROD, dev HEAD and the EXP-ECO-004 toolchain builds; receipt size for a 1,000,000-entry lossy export | receipt-v3-unified and in-toto-attestation: not implemented |
| DEC-ECO-062 | status-quo-append-eb (`sidecar` default naming); distinct-sidecar-suffix emulated by renaming; checksum-manifest-style (`SHA256SUMS` + signature) | attestation-carriage, registry-metadata: need registries |

Signing keys, identities and passwords are generated in per-sample scratch and deleted after the sample; nothing is uploaded.

## 5. Corpus

- Tuning items: `f01-tuning-ripgrep-14-1-1-git` and `f18-tuning-lz4-releases-1-9-to-1-10` (release trees; the second provides a genuine "other release" for replay and substitution), `f19-tuning-real-home-snapshot` (metadata-rich; AUX-only changes), `f14-apache-maven-3916-bin-tarball` and `f14-jenkins-25683-war` (legacy artifacts for zip/tar → `.eb` → tar.zst chains).
- Generated: `gen-1m-entries` — 1,000,000 files of 1-64 bytes in a 3-level fan-out, produced by `research/tools/ecosystem/verification/gen_many_entries.py --seed 2026091707` under `/root/eb-research/generated/EXP-ECO-007/` (streaming generator; the PROGRESS note on in-memory hashing applies), used only for the receipt-size measurement.
- Validation look (one): `f01-validation-redis-7-2-16-git`, `f18-validation-cjson-releases`, `f19-validation-real-home-snapshot`, `f14-pyspark-420-sdist`, `f14-gen-office-documents`.

## 6. Environment and platform requirements

WSL2 Ubuntu 24.04 (`wsl-ubuntu`) for all scenarios; Windows host for the Windows copy channels (`robocopy /COPY:DAT` of the primary artifact only; `Copy-Item` from `\\wsl.localhost\Ubuntu\...`). Pinned downloads: `cosign` release binary (SHA-256 recorded), MinIO server and `mc` binaries, `in-toto` and `pycose` in the research venv; apt: `minisign`, `signify-openbsd`, `gnupg`, `par2`, `rsync`, `nginx`. Local network only.

## 7. Procedure and commands

Scripts in `research/tools/ecosystem/verification/` (to be written):

1. `produce.sh <item> <scratch>` — pack the item (balanced INDEXED), create encrypted copy (X-Wing recipient generated in scratch), `publish` legacy targets with receipts, `sidecar` for legacy items, and every signature/attestation variant of §4.
2. `benign_ops.py` — apply each benign operation (§2 H1) and record LAI, PCR, AUX, PCI and each binding status from `ebound inspect --json` and `ebound verify --signatures`; compare with `research/experiments/EXP-ECO-007/spec-8-1-expected.yaml`, a transcription of the SPEC §8.1 table (by identifier, not text; authored and checked by two sessions against the SPEC at its recorded SHA-256 before any run).
3. `tamper_matrix.py` — cases: T1 valid substitute (artifact from the other release, receipts and signatures unchanged); T2 one-byte modification; T3 replay (receipt and signature of release N with artifacts of release N+1); T4 AUX-only change (metadata changed, re-packed); T5 representation-only repack; T6 attacker-produced archive with an attacker-key detached signature; T7 companion file deleted; T8 artifacts renamed; T9 index rebuild. For each (variant, case): consumer check script `consumer_<variant>.sh` (committed; it is also the counted command sequence) → outcome detected / not detected / false alarm.
4. `copy_channels.py` — channels C1 `cp`, C2 `cp -a`, C3 `rsync -a`, C4 `rsync -rt`, C5 `zip -r` then `unzip` of the directory, C6 `zip` of the primary artifact only, C7 `tar -cf` then extract, C8 `curl` download of the primary artifact from nginx, C9 MinIO `mc cp` of the primary artifact, C10 MinIO `mc mirror` of the directory, C11 Windows `robocopy` of the primary artifact, C12 Windows `Copy-Item` of the directory; after each, run each consumer check and record: companions present, association discoverable by the convention, verification outcome, whether any tool reports separation.
5. `receipt_reproduce.py` — for each export receipt, reconstruct the export command and options from receipt fields only, rerun with REF-PROD, dev HEAD and EXP-ECO-004 toolchain builds, compare bytes.
6. `receipt_size.sh` — export `gen-1m-entries` to `tar/pax-v1` and `zip/portable-v1` with `--allow-lossy` where required; receipt and migration-report bytes.
7. `ebr run --spec spec.yaml`; verify-raw; normalize; `matrix_report.py` writes `research/normalized/EXP-ECO-007/{identity-conformance,tamper,copy-channels}.csv`.

## 8. Seeds, warmup, replication

Seeds `order` 2026091707, `bootstrap` 2026091707, `command` 2026091707 (key generation uses OS randomness; outcomes, not bytes, are compared across repetitions). 2 repetitions per (scenario, item); outcome tables must be identical across repetitions, else the cell is reported nondeterministic. No warmups.

## 9. Metrics and measurement

| Metric | Canonical | Role |
|---|---|---|
| `migration_identity_conformance_fraction` | yes (HC-10, M20.1) | binary: H1 cells matching SPEC §8.1 |
| `undetected_tampers` | yes (HC-14, M15.1) | binary for signed variants; reported (not screened) for unsigned variants |
| `migration_determinism_fraction` | yes (HC-13, M20.2) | binary: receipt-based re-export reproducibility |
| `receipt_completeness_fraction` | yes (HC-07, M20.4) | binary: every lossy item of the 1M export recorded |
| `migration_steps_count`, `flags_per_task` | yes (M26.2, C6) | descriptive / cost input: consumer check scripts |
| `artifact_bytes` | yes (T-01) | descriptive: companion file bytes (signatures, bundles, receipts, parity) |
| `binding_survival`, `separation_detected`, `association_discoverable`, `false_alarm` | no | record fields per (variant, case or channel) |

## 10. Normalization and aggregation

AGG-C over the pre-registered case lists (tamper cases, benign operations, channels) per variant; headline the minimum over cases; AGG-S per evaluation condition (encrypted versus plain archives reported separately). Byte costs AGG-P per variant.

## 11. Statistical analysis and use in the decision rule

Deterministic outcomes, no inference. H1 failures are HC-10 violations of the status quo (defect, objective §2.0 rule 5). Signed variants with `undetected_tampers` > 0 are R1-eliminated for the tamper model they claim; unsigned variants are not screened by HC-14 but their undetected cases are counterevidence for any claim of substitution resistance (§10.1). Counts feed C6; companion bytes feed the cost record.

## 12. Practical-significance thresholds

Binary metrics per §3.3; `flags_per_task` and `migration_steps_count` have no band (Appendix A.2); `artifact_bytes` is descriptive here.

## 13. Sensitivity analysis

Tamper matrix with the attacker controlling (a) every published file, (b) everything except a key pinned out of band, (c) only the primary artifact; copy channels with and without extended attributes enabled on the destination.

## 14. Held-out (Phase D placeholder)

Conventions §8; the tamper and identity-conformance matrices are re-run once on held-out release items after unlock.

## 15. Expected negative results worth recording

Detached `.ebsig` files of encrypted archives exposing plaintext-derived roots to third parties; representation-only repack invalidating PCI-bound claims; object-store single-object copies separating every companion file silently; receipts insufficient to reproduce exports across builds.

## 16. Threats to validity

The consumer scripts encode one competent verifier's procedure; real consumers may check less (EXP-ECO-019 interprets outputs). The SPEC §8.1 transcription can contain errors (two-session check). Emulated candidate forms (external DSSE, signed receipts) approximate but are not the ledger designs.

## 17. Cost estimate

About 8 h compute (5 tuning items × about 30 variants × 9 tamper cases × 12 channels, most operations seconds; 1M-entry generation and export about 2 h); about 30 GB disk transient; agent effort scripted plus judgment for the SPEC §8.1 transcription (two sessions).

## 18. Gates and exposure

Conventions §1 and §2. No exposed family is applicable except through generated items; analysis may be performed by any session not involved in the candidates.
