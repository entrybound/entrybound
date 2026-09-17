# EXP-LEGACY-030: export fidelity and consuming-reader acceptance — frozen ZIP/tar/pax/compressed-tar profiles and candidate profile prototypes across GNU tar, bsdtar, star, busybox, Python, Go, Java, .NET, PowerShell, Explorer, Info-ZIP, 7-Zip, Rust, Node and OCI unpackers on Linux and Windows

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (export driver `export_matrix.py`; stress-tree generator; research-only prototype writers for candidate profiles; reader extraction drivers shared with EXP-LEGACY-001/002 plus OCI unpackers; independent restore observer `observe_tree.py` for ext4 and an NTFS observer `observe_tree_ntfs.ps1`) |
| Domain / program sections | legacy / §27 (export fidelity); feeds §29 and §35 |
| decision_ids | DEC-LEG-028, DEC-LEG-015, DEC-LEG-029, DEC-LEG-033, DEC-LEG-036, DEC-LEG-041, DEC-LEG-068, DEC-LEG-091, DEC-LEG-108, DEC-PLT-020, DEC-CRY-061 |
| Decision types (proposed) | EMPIRICAL (acceptance, restore fidelity); HUMAN-FACING parts of DEC-LEG-029/036 and DEC-CRY-061 (preferences, comprehension) are not addressed (EXTERNAL_REVIEW) |
| OD / HC (proposed) | OD-19 (M19.4), OD-03 (M03.2), OD-18 (M18.2), OD-20 (M20.4); HC-07 (MVT-07(c)), HC-13, HC-16 (frozen profiles untouched), HC-08 |
| Shared rules / L7 / gates | C§ header, C§2 |
| Timing | `timing: false` |

## 1. Question

Which consuming readers on Linux and Windows open each frozen export profile's output without errors or stray
artifacts and restore the EAM-equivalent tree per metadata class; how do typed LOSSLESS/LOSSY/REFUSED outcomes
distribute over real and stress sources; and how do candidate profile prototypes (ustar-only, record padding, tar/pax-v2
key families, ZIP Unix/NTFS extras, alternative timestamp encodings, Always-ZIP64 streaming, a pinned layer dialect)
change acceptance and fidelity?

## 2. Hypotheses

- **H1 (frozen acceptance, DEC-LEG-028).** Every mainstream reader (GNU tar, bsdtar Linux/Windows, Python `tarfile`
  with `filter="data"`, Go, 7-Zip, .NET, Info-ZIP `unzip`, Python `zipfile`, Java, Explorer, PowerShell 5.1/7) extracts
  every LOSSLESS output of the six frozen profiles with exit status 0, no stray `PaxHeaders.*`/`@LongLink` files and
  restore fidelity 1 on path, kind, content, executable and mtime to the profile's declared precision. *Falsified by*
  any (profile, reader) cell below 1.
- **H2 (7-Zip/older readers and pax).** At least one reader (predicted: busybox tar or PowerShell 5.1 for ZIP, 7-Zip for
  pax extended headers on long names) produces stray files or truncated names on `tar/pax-v1` outputs that need pax.
- **H3 (timestamps, DEC-PLT-020).** Readers disagree on restored mtimes for at least one of: pre-1980 ZIP entries,
  post-2107 ZIP entries, negative fractional pax times; Windows readers apply a local-time interpretation to DOS time.
- **H4 (lossy share, DEC-LEG-036).** More than half of freshly packed real tuning items export as LOSSY to
  `tar/pax-v1` solely because of construction-equivalent metadata (mode matching the executable projection, uid/gid 0,
  aligned mtimes) `[INFERRED]`. *Falsified by* a share ≤ 0.5.
- **H5 (Always-ZIP64 streaming, DEC-LEG-108).** Always-ZIP64 plus data-descriptor output is refused by at least one of
  Java `ZipInputStream`, PowerShell 5.1, Explorer or warehouse `validate_zipfile`.
- **H6 (identity, HC-07/MVT-07(c)).** Every LOSSLESS export strict-reimports to the source LAI
  (`migration_identity_conformance_fraction` = 1); every LOSSY receipt enumerates every degraded item
  (`receipt_completeness_fraction` = 1). *Falsified by* one counterexample.
- **H0.** All readers accept all profiles identically and restore the same trees.

## 3. Decisions informed

| Decision | Supplied here | Remaining elsewhere |
|---|---|---|
| DEC-LEG-028 | Reader × platform × profile acceptance and stray-artifact matrix; ustar-only and record-padding prototypes | macOS readers EXP-LEGACY-080 (BLOCKED) |
| DEC-LEG-015 | Behaviour of each reader for backslash, colon, newline, reserved-name, case-fold and NFC/NFD paths present in `tar/pax-v1` (allowed) versus refused by `zip/portable-v1`; rename-with-loss prototype outcomes | Frequency in published trees EXP-LEGACY-010 |
| DEC-LEG-029 | Acceptance and fidelity of tar/pax-v2, ZIP Unix/NTFS extras, registry-ingest ZIP and standalone-stream prototypes | cpio/OCI/7z candidates EXP-LEGACY-020; consumer survey EXTERNAL |
| DEC-LEG-033 | Export outcomes and incumbent extraction of trees whose ancestors were synthesised at ZIP import | AUX marker candidates EXP-LEGACY-051 |
| DEC-LEG-036 | Share of LOSSY outcomes by issue class over real items and tar/ZIP round trips; the same share under a construction-equivalence rule | Usability of approval friction EXTERNAL |
| DEC-LEG-041 | Unpacker acceptance (umoci, crane, rootless podman) and layer-digest stability of a pinned-dialect layer prototype versus `tar/pax-v1` output | OCI roadmap status (external citation) |
| DEC-LEG-068 | Reader matrix per key family (SCHILY.xattr, LIBARCHIVE.xattr, SCHILY.acl, birthtime, MSWINDOWS, GNU sparse 1.0, base-256 vs pax ids) using GNU tar, bsdtar and star writer outputs as stand-ins for v2 variants | Round-trip fidelity per class (platform domain) |
| DEC-LEG-091 | Extractor support for Info-ZIP `0x7875`/`0x5455`, symlink attributes, NTFS `0x000a` (7-Zip `-mtc`) | macOS Archive Utility EXP-LEGACY-080 |
| DEC-LEG-108 | Reader acceptance of Always-ZIP64 plus data descriptor output | Memory/disk/time EXP-LEGACY-032 |
| DEC-PLT-020 | Restored timestamps per reader for pre-1980, 2038, post-2106/2107, negative fractional, ns fractions; DOS local vs UTC-derived + `0x5455` vs `0x000a` prototypes | macOS readers EXP-LEGACY-080 |
| DEC-CRY-061 | Mechanical facts: whether export from an authenticated encrypted source requires any explicit flag, and which receipt fields record the transition | Human comprehension EXTERNAL; flag census EXP-LEGACY-070 |

## 4. Candidates

| Decision | Candidates (ledger ids) | Realisation here |
|---|---|---|
| DEC-LEG-028 | status-quo-self-reimport-only; mainstream-matrix-gate; ustar-only-profile; pad-to-record-size; document-known-incompatibilities | frozen outputs; prototypes `proto-tar-ustar-only`, `proto-tar-pad-10240`; gate and documentation rules evaluated over the matrix |
| DEC-LEG-015 | status-quo-asymmetric; format-keyed-minimal; runtime-profile-keyed; rename-with-loss; typed-newline-issue | frozen outputs plus `proto-rename-with-loss`; rules over reader outcomes |
| DEC-LEG-029 | status-quo-six-profiles; add-tar-pax-v2; add-zip-unix-v2; add-registry-ingest-zip; add-standalone-streams (here); add-7z, add-oci-layer, add-cpio (EXP-LEGACY-020) | prototypes and incumbent-writer stand-ins |
| DEC-LEG-033 | status-quo-report-list; no-metadata-typed-issue; policy-mode-0755; inherit-from-child / inherit-child-mtime; fidelity-report-flag; aux-implicit-marker (export-effect part) | rules over export issue lists; reader behaviour for implicit directories |
| DEC-LEG-036 | status-quo-lossy-on-any-recorded-posix; construction-equivalence-lossless; pack-captures-profile-precision; per-class-approval; spec-strict-definition | outcome distributions recomputed under each rule |
| DEC-LEG-041 | defer-until-oci-pins; self-pinned-pax-profile; reuse-tar-pax-v2; not-justified | `proto-oci-layer-pinned` versus `tar/pax-v1` output |
| DEC-LEG-068 | status-quo-no-v2; v2-links-and-ownership; v2-gnu-compatible; v2-libarchive-compatible; v2-densify-sparse; v2-oci-windows-keys | GNU tar `--format=posix --xattrs --acls`, bsdtar `--format pax --xattrs --acls`, star `artype=exustar`, hand-built MSWINDOWS keys and densified sparse as stand-ins |
| DEC-LEG-091 | status-quo-extended-timestamp-only; add-unix-ids-and-symlinks; add-ntfs-times; add-xattr-extras; declare-loss-only | Info-ZIP `zip -y` (writes `0x7875`, `0x5455`), 7-Zip `-tzip -mtc=on` (`0x000a`); `add-xattr-extras` has no Linux producer (macOS ditto: BLOCKED) |
| DEC-LEG-108 | status-quo-in-memory-as-needed; always-zip64-streaming-profile; never-zip64-refuse-before-write; as-needed-fail-late (acceptance parts) | `proto-zip-always-zip64-dd` |
| DEC-PLT-020 | status-quo; pax-integer-plus-positive-fraction; new-profile-local-dos; new-profile-ntfs-000a; refuse-post-2038-zip-export | prototypes; `modify-portable-v1-in-place` is a proposed L0 exclusion (HC-16: frozen `zip/portable-v1` bytes) and is not built |
| DEC-CRY-061 | status-quo-warning-and-receipt; explicit-flag-required; declared-loss-outcome; interactive-confirmation; silent-export | mechanical status-quo facts only; `silent-export` proposed L0 exclusion (SPEC §15.2 L1697) |

Prototype writers live in `research/tools/legacy/export_protos/` (Python, reading the source EAM through
`ebound inspect --json --entries` and `ebound read`) and are research-only; frozen profiles are never modified (HC-16).

## 5. Corpus selectors

Real sources (packed with `ebound-head pack --profile balanced --layout indexed`; STREAM layout as a byte-identity check
in EXP-LEGACY-031): tuning `f01-tuning-ripgrep-14-1-1-git`, `f01-tuning-zstd-v1-5-7-git`, `f04-tuning-sourcelike`,
`f05-tuning-logrotate-from-loghub-hdfs`, `f06-tuning-gen-structured-a-small`, `f12-tuning-kodak-jpeg-edge-cases-small`,
`f18-tuning-lz4-releases-1-9-to-1-10`, `f19-tuning-real-home-snapshot`, `f03-tuning-fzf-go-mod-vendor`,
`f09-alpine-324-rootfs-binaries` (10 groups, 8 families); validation `f01-validation-redis-7-2-16-git`,
`f01-validation-bootstrap-5-3-8-git`, `f04-validation-objstore`, `f05-validation-logrotate-from-loghub-ssh`,
`f06-validation-gen-structured-b`, `f12-validation-fsa-jpeg-matrix-medium`, `f18-validation-cjson-releases`,
`f19-validation-real-home-snapshot`, `f03-validation-yq-go-mod-vendor`, `f09-fedora-44-usr-binaries` (10 groups, 8
families). Minimum support (§4.9) is met per split.

Generated sources (`research/tools/legacy/cases/export_stress_trees.py`; F19; `S_T = 2509173001`, `S_V = 2509173002`):
timestamps (1970-01-01, pre-1980, 1980-01-01, 2038-01-19 ± 1 s, 2106-02-07, 2107-12-31, 2108, negative fractional,
1 ns, 100 ns); paths (backslash, colon, newline, trailing dot/space, `CON`/`AUX`, case-fold and NFC/NFD pairs, names of
100/101/255/256 bytes, depth 4096); kinds (symlinks relative/absolute/dangling — REFUSED by status quo, measured; hardlinks;
empty directories; executables 0755/0744/0700); ownership (uid/gid 0, 1000, 2097152, names); sizes (one 8 GiB + 1 B
file of zeros; 70,000 entries); xattrs and ACLs.

Round-trip sources: GNU-tar and bsdtar tars from EXP-LEGACY-002 T2 writer matrix (tuning/validation) and ZIPs from
EXP-LEGACY-001 C5 (tar→eb→tar, zip→eb→zip); ZIPs with implicit ancestors (C9); an encrypted source (`pack --recipient`
with a generated X-Wing recipient file) and a signed source (`ebound sign`). Held-out: placeholder (C§5).

## 6. Environment

`wsl-ubuntu` (ext4) and `windows-host` (NTFS, non-admin) extraction; Windows readers via interop with NTFS staging;
Windows symlink creation without Developer Mode is recorded as a platform limitation, not a reader outcome. OCI
unpackers rootless in WSL. No timing.

## 7. Commands and tooling

Tooling: `research/tools/legacy/export_matrix.py` (exports each source with every frozen profile, `--dry-run` and
`--allow-lossy --receipt`, then runs prototype writers, then extraction by every reader via `run_matrix.py --ops extract`),
`cases/export_stress_trees.py`, `export_protos/{tar_ustar_only.py, tar_pad_10240.py, tar_pax_v2_variants.py,
zip_unix_extras.py, zip_ntfs_times.py, zip_always_zip64_dd.py, zip_local_dos.py, pax_positive_fraction.py,
oci_layer_pinned.py, rename_with_loss.py, standalone_stream.py, registry_ingest_zip.py}`, `observe_tree.py`,
`observe_tree_ntfs.ps1`, `readers/oci_unpack.py`, `export_analysis.py` (rule evaluation, restore fidelity per class,
receipt completeness, strict re-import LAI check).

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-030 --generator export_stress_trees --split tuning --seed 2509173001 --append-corpus-items research/experiments/EXP-LEGACY-030/real-items-tuning.txt
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-030 --generator export_stress_trees --split validation --seed 2509173002 --append-corpus-items research/experiments/EXP-LEGACY-030/real-items-validation.txt
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-030/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-030/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-030/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/export_analysis.py --experiment EXP-LEGACY-030 --out research/normalized/EXP-LEGACY-030/analysis/
```

## 8. Seeds

`order 2509173011`, `bootstrap 2509173012`, `command 2509173013`; tree seeds §5.

## 9. Warmup, replication

No warmup; 2 repetitions per (source, profile or prototype, reader, platform) with repeat-hash of export bytes and
restore observations.

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `export_acceptance_fraction` (per profile × reader × platform) | T-20 banded | OD-19 | T-20 |
| `restore_fidelity_fraction` (per metadata class) | T-20 banded | OD-03 | T-20 |
| `cross_platform_restore_fidelity_fraction` (Linux-exported, Windows-restored and reverse) | T-20 banded | OD-18 | T-20 |
| `export_outcome_distribution` (LOSSLESS/LOSSY/REFUSED by issue class, per candidate rule) | descriptive | OD-19 | none |
| `migration_identity_conformance_fraction` (LOSSLESS export → strict re-import LAI equals source LAI) | binary | OD-20 | §3.3 |
| `receipt_completeness_fraction` (LOSSY receipts enumerate every observed degradation) | binary | OD-20 | §3.3 |
| stray-artifact counts; unpacker layer-digest stability; reader error classes | non-canonical descriptive | — | none |

## 11. Normalization

Restore observations per class by independent tools (C§6); expected values per class derive from the source EAM and
the profile's declared representation (for LOSSY cases, from the receipt's declared degradation, so a correctly
declared loss is not counted as a fidelity failure but is counted in `export_outcome_distribution`).

## 12. Statistical analysis

T-20 fractions per evaluation condition (profile × reader × platform) with AGG-S worst-reader headline; real-source
items aggregate L1 → L2 groups → L3 families → L4 corpus with arithmetic means of differences between candidates
(absolute-only metrics, §4.9), Welch-Satterthwaite corpus interval (§4.10) and the family harm guard; confirmatory
comparisons (for example `proto-tar-ustar-only` versus `tar/pax-v1` on `export_acceptance_fraction`) are declared from
tuning results before the validation look with confidences per §4.12. Binary failures are counterexamples.

## 13. Practical significance

T-20 `a = 0.5/n_cases` (`decision-method.md` §3.2); binary §3.3.

## 14. Sensitivity analysis

C§13 arms; reader-set arm (mainstream set of R1 §11.1 — GNU tar, bsdtar, Info-ZIP, 7-Zip, Windows Explorer — versus all
readers); Python filter arm (`data` versus `tar` versus `fully_trusted`); layout arm (INDEXED versus STREAM sources);
equal-family-weight arm (§6.5).

## 15. Expected negative results worth recording

A frozen profile rejected or mis-restored by a mainstream reader (a frozen-profile limitation to document, new profile
id if changed); ustar-only or padding prototypes giving no acceptance gain; ZIP extras prototypes breaking Explorer or
PowerShell; pinned layer dialect not accepted by an unpacker; construction-equivalence changing few outcomes.

## 16. Threats to validity

Windows readers exercised through interop on staged copies; Explorer COM automation may not match interactive use; no
macOS readers (BLOCKED); stand-in writers for v2 variants differ from what an Entrybound profile would emit (the
reader-support premise is what is measured); 8 GiB stress files exercise size fields but not realistic content.

## 17. Platform requirements

Linux x86-64 (WSL2), Windows x86-64 non-admin via interop; macOS readers **PLATFORM_BLOCKED** (EXP-LEGACY-080); storage
~60 GB transient; no privileged host operations.

## 18. Estimates

Compute ≈ 70 CPU-hours (≈ 3,800 export artifacts × ≈ 30 reader columns × 2 repetitions); disk ≈ 60 GB transient, ≈ 20
GB retained artifacts. Agent effort: scripted tooling; judgment for rule evaluation and triage.

## 19. Expected cost tiers `[INFERRED]`

Each new export profile: T2 (new profile) with permanent golden vectors (C7); timestamp encoding change requires a new
profile id (in-place change excluded, HC-16); construction-equivalence redefinition of LOSSLESS: T2 new profile
versions; Always-ZIP64 streaming profile: T2.
