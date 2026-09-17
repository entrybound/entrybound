# EXP-INT-009: Identity reproduction and binding staleness under rewrite, re-plan and substitution operations

| Field | Value |
|---|---|
| Status | READY: driver `run_identity_ops.py` (this directory, standard library only) exists and passed a non-decision-grade feasibility smoke on a tiny generated two-version tree (both repetitions gave equal deterministic-view hashes). Preconditions are mechanical: build B-PROD on both hosts (conventions C1) and mirror the items to D: for the Windows arm. Sub-arm NEEDS_TOOLING: encrypted operations (`key add`, `key remove`, `key change-password`) need scriptable recipient identities (the CLI has no recipient key generator and `--password` prompts on a terminal); that sub-arm is deferred to EXP-INT-016's tooling and recorded as a gap here. |
| Domain | integrity (program §23, §24; objective HC-10, HC-13, HC-14, OD-20, OD-15) |
| Decisions informed | DEC-INT-016, DEC-INT-011, DEC-INT-028, DEC-CRY-033, DEC-LEG-040, DEC-LEG-046, DEC-INT-046 |
| Gates, method, author | conventions C0 |
| Specs | `spec.yaml` (WSL), `spec-windows.yaml` (Windows NTFS; cross-host comparison) |

## 1. Question

Which archive identities (LAI, AUX, PCR, PCI) and which binding carriers (PCI, chunk-level
digests, LAI plus signature, filename convention, detached signature receipt, par2 file hash,
wrapper index by digest) survive each benign rewrite operation of the production CLI, which
of them detect a rollback substitution by an older validly signed archive, and does
re-planning from extracted content reproduce PCR and PCI?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-INT-016 | LAI and AUX are unchanged by every benign operation (repack with and without `--profile`, Index removal and rebuild, layout change); PCR changes only under `--profile` replanning; PCI changes under Index and layout changes. So PCI- and PCR-based base references break on benign operations and LAI-based references do not. | Any expectation cell of the pre-registered table (`run_identity_ops.py` `EXPECTED`) mismatched (an HC-10 or HC-13 defect, not a falsification of the design question); or LAI changes under a benign operation. |
| H2 | DEC-INT-011, DEC-INT-028 | `bind-pci`, `par-native-file-hash` and `wrapper-index-artifact` stop matching after Index and layout operations (false staleness for unchanged content); `bind-lai-plus-signature` survives every benign operation; the detached signature reports `physical=STALE` rather than `INVALID` after replanning. | PCI-bound carriers survive layout or Index changes. |
| H3 | DEC-CRY-033 | A rollback to an older version signed by the same key is not detected by the status quo (`verify --signature` reports VALID) and is detected by `expected-pci-option` whenever the caller holds the new PCI. | Status quo detects the substitution. |
| H4 | DEC-INT-046 | Re-planning from an extracted tree with the same profile reproduces PCR (planner determinism) but not always PCI, because AUX depends on metadata restored by `unpack` (feasibility smoke: AUX differed between two extractions of the same archive, non-decision-grade); a `replan-compare-pcr-pci` design must therefore re-plan from the verified EAM, not from an extracted filesystem tree. | PCI reproduced on every item. |
| H5 | DEC-LEG-040, DEC-LEG-046 | Status quo `repack` copies provenance: AUX unchanged by repack; an `append-rewrite-record` or `strip-provenance` design changes AUX by construction and therefore makes every AUX-bound signature binding stale (modelled from the measured binding table). | AUX changes under status quo repack. |
| H6 | cross-host (F-0002) | PCR is identical across WSL and Windows for every item and profile; LAI and AUX differ where host metadata differ (F-0001, F-0002), so cross-host base references by LAI are not portable for trees captured on different hosts. | LAI equal across hosts on items with host-specific metadata. |

## 3. Decisions and candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-INT-016 | `base-lai`, `base-pcr`, `base-pci`, `base-archive-id`, `content-digests-only`, `lai-plus-pci`, `signed-base-reference` | Robustness = identity pattern after each benign operation (measured). `base-archive-id` is not observable (no archive_id in inspect output at the baseline); recorded as not measured here, evaluated in EXP-INT-007's model. |
| DEC-INT-011 | `bind-pci`, `bind-section-chunk-digests` (proxied by PCR equality, labelled PROXY), `bind-lai-plus-signature`, `filename-convention-only` (always "matches", never detects staleness), `receipt-or-detached-signature` (B-PROD detached `.ebsig` statuses), `par-native-file-hash` (par2cmdline 0.8.1 verify), `wrapper-index-artifact` (digest of PCI) | Survival over benign operations; separation through copy tools is EXP-INT-011. Authentication analysis of parity-induced substitution: an attacker regenerates a par2 set for a substituted archive; par2 file hashes are unkeyed MD5 values, so the binding authenticates nothing (FORMALLY_DERIVED from the PAR 2.0 specification's packet definitions, cited in the analysis with section locators). |
| DEC-INT-028 | `status-quo-compute-only`, `expected-pci-option`, `digest-before-parse`, `signed-pci-binding`, `external-receipt-pci`, `drop-pci-validation-wording` | Staleness of PCI-bound claims under Index rebuild and repack (measured); cost is EXP-INT-004. |
| DEC-CRY-033 | `status-quo-none`, `expected-pci-option`, `expected-addressing-signature`, `min-timestamp-policy`, `signed-version-counter-new-version`, `tuf-style-external-metadata`, `self-contained-freshness-claim` | Rollback arm on version series: measured for `status-quo-none`, `expected-pci-option` and `self-contained-freshness-claim`; `expected-addressing-signature` is not applicable to plaintext archives (addressing binding exists only for encrypted archives, feasibility smoke) and is measured in EXP-INT-016's encrypted arm; timestamp, version-counter and TUF candidates are MODEL entries (no timestamp authority or counter exists at the baseline). Proposed R1 exclusion of `self-contained-freshness-claim`: `docs/crypto-threat-model-v1.md` L59-63 (cited in the ledger) states freshness is not claimed from archive bytes; the rollback arm is its counterexample. |
| DEC-LEG-040 | `status-quo-copy-only`, `append-rewrite-record`, `external-rewrite-receipt`, `no-rewrite-provenance` | Measured AUX invariance under status quo rewrites; the AUX and signature effects of the alternatives follow from the measured binding table (an AUX change makes every AUX-dependent binding stale), labelled MODEL. Auditor value of rewrite history is analytical (gap). |
| DEC-LEG-046 | `status-quo-always-embed-no-strip`, `strip-provenance-repack`, `no-embed-with-external-receipt`, `redacted-provenance` | As DEC-LEG-040; privacy needs are analytical and human-facing (gap). |
| DEC-INT-046 | `no-reproducible-verify`, `replan-compare-pcr-pci`, `record-adaptive-flag`, `repack-check-reproducible`, `external-rebuild-tooling` | Re-plan reproduction from extracted content and via `repack --profile` round trips (measured); re-planning cost is recorded as the wall time of the re-pack step (secondary, not decision-grade timing). Cross-thread and cross-platform determinism runs are the scale and platform domains' (§14); the Windows arm here adds cross-host PCR agreement. |

## 4. Corpus

- Tuning: `f01-tuning-ripgrep-14-1-1-git`, `f04-tuning-sourcelike`, `f05-tuning-gutenberg-books`,
  `f06-tuning-wikimedia-tnwiki-20260901`, `f07-tuning-silesia-xray`,
  `f09-alpine-324-rootfs-binaries`, `f12-tuning-kodak-jpeg-edge-cases-small`,
  `f17-tuning-duptree-mixed`, `f19-tuning-zoo`, and the series items
  `f18-tuning-lz4-releases-1-9-to-1-10`, `f18-tuning-zstd-releases-1-5-x` (rollback arm runs
  automatically when the item's top level is one directory per version, conventions C3).
- Validation look: `f01-validation-redis-7-2-16-git`, `f04-validation-objstore`,
  `f05-validation-rfc-texts`, `f06-validation-osm-monaco-20250101-xml`,
  `f07-validation-gen-numeric-b-small`, `f09-cpython-3147-embed-win-amd64`,
  `f12-validation-fsa-jpeg-edge-cases-small`, `f17-validation-duptree-vendor`,
  `f19-validation-real-home-snapshot`, `f18-validation-cjson-releases`.
- Items the production packer refuses (known refusals: non-UTF-8 names, ACL mask disagreement;
  `research/baselines/README.md`) are recorded with `payload.refused = true` and excluded with
  the refusal line; no replacement item is chosen after results are visible.

## 5. Environment and platform requirements

`wsl-ubuntu` (ext4 scratch) and `windows-host` (NTFS on D:), `timing: false`. par2cmdline 0.8.1
on WSL (pinned to one thread, `-t1`); the Windows arm skips par2 unless a par2cmdline build is
installed (recorded). No network, no root, no Docker.

## 6. Procedure and commands

1. Build B-PROD on both hosts (conventions C1); record SHA-256.
2. WSL:
   ```sh
   wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools &&
     P=/root/eb-research/venv/bin/python; S=research/experiments/EXP-INT-009/spec.yaml;
     $P -m ebr validate --spec $S && $P -m ebr run --spec $S && $P -m ebr verify-raw --spec $S --refingerprint &&
     $P -m ebr normalize --spec $S && $P -m ebr report --spec $S'
   ```
3. Windows: the same sequence with `spec-windows.yaml` from PowerShell using the research venv
   on Windows (`research/tools/run_tests_windows.ps1` environment) and `--data-root D:/eb-research`.
4. Analysis: `research/tools/integrity/identity_tables.py --experiment EXP-INT-009` (to be
   written; reads detail files) builds the operation x identity pattern table, the binding
   survival table, the rollback detection table and the cross-host PCR, LAI and AUX agreement
   table; every number in them is generated from detail files (§12.3).

Operations per sample (pre-registered, with expectations in `run_identity_ops.py` `EXPECTED`):
`pack-again`, `repack-preserve`, `repack-index-absent`, `repack-index-present-from-absent`,
`repack-layout-stream`, `repack-layout-indexed-from-stream`, `repack-profile-other`
(balanced to extreme, extreme to fast), `replan-from-extract` (unpack then pack with the same
profile); detached signature with the physical binding verified against every output; par2
5% set verified against every output; rollback arm on series items.

## 7. Seeds, warmups, replication

Seeds 2026091791/92/93 (WSL), 2026091794/95/96 (Windows). No random choices in the driver
(signing keys are fresh per repetition and excluded from the deterministic view). Warmups 0;
2 repetitions; repeat-hash on `detail_sha256` (deterministic view: identities for scored cells,
patterns, signature and binding statuses, par2 outcomes, rollback outcomes). A mismatch on a
scored identity cell under identical input is an HC-13 violation artifact (§4.13).

## 8. Metrics

| Metric | ID | OD | Role |
|---|---|---|---|
| `migration_identity_conformance_fraction` | HC-10 | OD-20 floor | binary: every pre-registered identity expectation cell matched |
| `migration_determinism_fraction` | HC-13 | OD-20 floor | binary for `pack-again` (PCI equality) |
| `binding_survival_fraction` (per carrier) | C11 proposed | OD-20 or as assigned | secondary until named; **primary candidate** for DEC-INT-011 and DEC-INT-016 if the review adds it |
| `undetected_tampers` (per freshness candidate, rollback arm) | HC-14 | OD-15 floor | binary for candidates claiming rollback detection; for the status quo it is the demonstration |
| `cross_host_identity_agreement` | HC-08 | OD-18 floor | reported per identity; binding only for PCR (LAI and AUX differences are explained by F-0001 and F-0002 and handled by the platform domain) |
| re-plan wall time | T-03 `encode_wall_s` | OD-06 | secondary, not decision-grade (no quiet window) |

## 9. Normalization

None for binary and coverage values (exact per item). Survival fractions: per item over the
benign operation set, then mean over groups and families (AGG-C with per-family minimum as the
headline).

## 10. Statistical analysis and sensitivity

Exact tallies; no intervals. R1 for binary cells. Carrier and reference selections use the
survival table with R10 cost keys (C1 words, wire items) where survival ties. Sensitivity:
profile arm (balanced versus extreme), host arm (WSL versus Windows), operation-set arm
(layout and Index operations only versus all).

## 11. Expected negative results worth recording

- PCI-bound bindings (including par2 file hashes and wrapper indexes) break on benign container
  rewrites.
- Status quo signatures cannot detect rollback.
- Re-planning from an extracted tree does not reproduce AUX or PCI.
- LAI is not portable across hosts for host-metadata-bearing trees (F-0002).

## 12. Threats to validity

- `bind-section-chunk-digests` is proxied by PCR equality (labelled PROXY); a real
  chunk-digest binding could survive operations that reorder but do not re-chunk.
- The operation set covers the CLI at the pre-registration commit; future operations need
  re-measurement (reopen trigger).
- Timestamp, counter and TUF freshness candidates are MODEL entries only.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12.

## 14. Cost estimates

- Compute: about 12 CLI operations plus inspections per (item, profile), items up to about
  250 MB: about 6 CPU-hours per host per phase.
- Disk: about 15 GB scratch per host (deleted per sample); detail about 50 MB.
- Agent effort: none to execute (scripted); analysis judgment.

## 15. Deviations (append-only)

None.
