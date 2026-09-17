# EXP-LEGACY-002: tar runtime differential and dialect matrix (primitive cases × GNU tar, libarchive, Python tarfile, Go, Rust, busybox, star, 7-Zip, .NET, Node, Commons; Entrybound tar-strict/v1)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (generator `tar_cases.py` T0-T13; writer-matrix script; drivers; GNU tar 1.34, libarchive 3.8.8 Linux, star, busybox-static, Rust crate readers, Node tar, .NET tar reader; `matrix.py --format tar`) |
| Domain / program sections | legacy / §25 (feeds §29, §30) |
| decision_ids | DEC-ECO-047, DEC-LEG-001, DEC-LEG-020, DEC-LEG-061, DEC-LEG-062, DEC-LEG-063, DEC-LEG-064, DEC-LEG-065, DEC-LEG-066, DEC-LEG-067, DEC-LEG-069, DEC-PLT-010, DEC-PLT-021, DEC-PLT-051 |
| Decision types (proposed) | EMPIRICAL with FORMAL parts (POSIX pax / GNU tar manual citations per case; triage rule) |
| OD / HC (proposed) | OD-19 (M19.2, M19.3), OD-04 (M04.2), OD-03 (M03.1 for T10); HC-07, HC-17, HC-05, HC-06 (T6 unresolved-link state) |
| Shared rules | `research/experiments/_legacy/conventions.md`; registry `_legacy/runtime-registry.json` |
| Author / L7 / gates | C§ header (L7 exposure recorded; analysis adoption by non-exposed session before G-A); C§2 gates |
| Timing | `timing: false` |

## 1. Question

How do pinned tar implementations list, read and extract each pre-registered tar construction (dialects, authority
disagreements, sparse variants, path forms, hardlink topologies, end markers, duplicates, non-UTF-8 names, metadata
keywords, special types) and tuning/validation tar archives; where do they diverge; and what does Entrybound
`tar-strict/v1` do with each?

## 2. Hypotheses

- **H1 (reproduction).** `libarchive-3.8.8-win` and `python-tarfile-3.13.5-win` reproduce the six cases of
  `tools/tar-strict/observed-outcomes-v1.json`; the GNU tar column (absent there) accepts all six except
  `bad-checksum`. *Falsified by* any differing cell.
- **H2 (strict safety floor).** `tar-strict/v1` refuses or records as a typed conflict every ambiguous case (C§7)
  (`ambiguity_refusal_fraction` = 1). *Falsified by* one ambiguous case imported silently.
- **H3 (R3 contested classes, DEC-LEG-061/062).** The EB-T007 construction is unambiguous across the tar reference
  readers (so strict refusal there is a benign false refusal), and the EB-T012 construction (GNU long name unrelated
  to the ustar name) is ambiguous (at least one reference reader selects the ustar name). *Falsified* respectively by a
  reference-reader divergence on T007 or unanimity on T012.
- **H4 (size desync, DEC-LEG-062).** At least one pax-size-versus-ustar-size case (EB-T005/T009/T010 and the
  CVE-2025-62518 class) produces a content divergence among the readers, including the pre-fix Rust async tar reader.
- **H5 (sparse, DEC-LEG-020).** GNU tar and libarchive reconstruct identical dense content for GNU sparse 0.0, 0.1 and
  1.0; Python `tarfile`, Go `archive/tar` and at least one other reader expose a different listing or content for at
  least one variant. *Falsified by* unanimity on all three variants.
- **H6 (`./` prefix, DEC-LEG-065).** Every reference reader extracts `./a` and `a` to the same destination; stripping
  `./` in strict therefore turns a (`./a`, `a`) pair into a detectable duplicate rather than a silent merge.
- **H7 (dialects, DEC-LEG-064).** GNU tar and libarchive accept v7-without-magic, old-GNU, star/xstar and Solaris `X`
  headers; Python, Go and .NET diverge on at least two of those dialects.
- **H0.** No divergence among reference readers on any pre-registered construction.

## 3. Decisions informed

| Decision | Supplied here | Remaining elsewhere |
|---|---|---|
| DEC-ECO-047, DEC-LEG-001 | Pinned tar matrix incl. GNU tar; strict/broad divergence; controls; effort to build a first tar rule model (hours, lines of `research/tools/legacy/models/tar_model.py`) | Fuzzed generalisation EXP-LEGACY-005 |
| DEC-LEG-020 | GNU sparse 0.0/0.1/1.0 and old `S` outcomes per reader and strict | Native sparse projection (platform domain §15) |
| DEC-LEG-061, DEC-LEG-062 | EB-T001..T013 replay with reason codes; authority-disagreement classes T3; Rust readers affected by CVE-2025-62518 | Prevalence of override kinds and T007/T008-like archives EXP-LEGACY-010 |
| DEC-LEG-063 | Per-behaviour divergence table across pinned runtimes; distinguishing cases per candidate profile runtime; drift GNU tar 1.34/1.35, libarchive 3.7.2/3.8.8, CPython 3.12/3.13/3.14 | Refusal rates on real corpora EXP-LEGACY-010; model maintenance cost EXP-LEGACY-005 |
| DEC-LEG-064 | Per-dialect fixtures (hand-built and real writer outputs) imported by Entrybound versus GNU tar and libarchive | Dialect prevalence EXP-LEGACY-010; per-dialect parser fuzzing (security domain §30) |
| DEC-LEG-065 | `./`, `.` root and V7 trailing-slash directory behaviour; collision analysis of `./a` versus `a` | Prevalence in release tarballs and Debian `data.tar` EXP-LEGACY-010 |
| DEC-LEG-066 | Hardlink topology matrix (forward, chains, to symlink/directory, repeated targets, data-bearing, never-appearing); worst-case unresolved-reference state size by construction | Prevalence EXP-LEGACY-010; allocator bound EXP-LEGACY-041 |
| DEC-LEG-067 | Non-UTF-8 names (raw ustar bytes, invalid UTF-8 pax path, `hdrcharset=BINARY`) per reader | Prevalence EXP-LEGACY-010; representation (model domain) |
| DEC-LEG-069 | End-marker, trailing-garbage and duplicate last-wins behaviour per reader including GNU tar | Gzip member padding EXP-LEGACY-004; prevalence EXP-LEGACY-010 |
| DEC-PLT-010 | Directory with content (typeflag `5`, size > 0) | ZIP/7z analogues EXP-LEGACY-001/003 |
| DEC-PLT-021 | Reader and strict import behaviour for SCHILY/LIBARCHIVE xattrs, ACL text formats (star vs libarchive), fflags, creation/atime/ctime, high mode bits | Prevalence EXP-LEGACY-010 |
| DEC-PLT-051 | pax fractional-second digit counts and negative decimals as written by GNU tar, bsdtar, star, Python, Go, .NET, Node (writer survey) and as read back | Diff false-change rate EXP-LEGACY-051 |

## 4. Candidates

| Decision | Candidates evaluated (as decision rules over observations; C§11) | Notes / exclusions |
|---|---|---|
| DEC-LEG-020 | status-quo-unknown-key; refuse-gnu-sparse-keys; project-pax-1.0-only; project-all-variants | — |
| DEC-LEG-061 | status-quo-unreplayed; adopt-r3-classes; entrybound-classes-with-deviations; profile-dependent-marking; unresolved-pending-evidence | — |
| DEC-LEG-062 | status-quo-any-override-refinement; validated-binding; size-mismatch-divergence; global-path-refused; r3-forbidden-classes | — |
| DEC-LEG-063 | status-quo-strict-only; gnu-tar-profile; libarchive-profile; python-tarfile-profile; go-archive-tar-profile; consensus-profile; defer-post-v1 | Rust-crate and busybox profiles added as critic-slot proposals to be confirmed by the R0 critic |
| DEC-LEG-064 | status-quo-ustar-pax-gnu; magic-dispatch; add-v7; add-oldgnu-incremental; add-star-xstar; add-volume-records; refuse-non-posix-magic | — |
| DEC-LEG-065 | status-quo-refuse; strip-leading-dot-slash-strict; compat-profile-only; v7-dirs-as-directories | — |
| DEC-LEG-066 | status-quo-whole-set; sequential-only; allow-chains; link-to-symlink-copy; data-bearing-divergence; bounded-deferred-table | — |
| DEC-LEG-067 | status-quo-refuse; opaque-bytes-declaration; charset-policy | — |
| DEC-LEG-069 | status-quo-strict-refuse; compat-profile-per-runtime; tolerate-padding-only; last-wins-with-evidence | — |
| DEC-PLT-010 | status-quo-zip-refuse-divergence (tar analogue: refuse); normalise-with-conflict-report; import-as-file; policy-selectable | silent-normalise proposed L0 exclusion (HC-07), still scored |
| DEC-PLT-021 | status-quo-report-unavailable; tar-adapter-v2-xattr-acl; gnu-pax-sparse-to-sparse-map; report-high-mode-bits; lom-evidence-only | sevenz-windows-namespaces in EXP-LEGACY-003 |
| DEC-PLT-051 | status-quo-five-classes; add-two-second-and-millisecond; add-unknown-precision; exponent-encoded-precision; refuse-out-of-registry-precision; truncate-with-declared-loss (writer-survey premise only) | selection needs EXP-LEGACY-010/051 |
| DEC-ECO-047, DEC-LEG-001 | as EXP-LEGACY-001 §4 (tar part) | — |

## 5. Corpus selectors

Generated case sets (`research/tools/legacy/cases/tar_cases.py`; F20; `S_T = 2509170201`, `S_V = 2509170202`):

| Set | Content | Approx. cases |
|---|---|---|
| T0 | the six `tools/tar-strict/probe.py` cases, byte-identical | 6 |
| T1 | Research III EB-T001..EB-T013 regenerated from primitive descriptions | 13 |
| T2 | Dialects: v7 without magic, v7 trailing-slash directories, old-GNU magic, GNU incremental (atime/ctime in prefix, `D` dumpdir), `M` multi-volume continuation, `V` volume label, star (`tar\0` prefix layout), xstar/exustar, Solaris `X`, pax `x`/`g`, GNU `L`/`K`, base-256 size/uid/mtime (including negative), ustar prefix split; plus **writer matrix** outputs of GNU tar `--format={v7,oldgnu,gnu,ustar,posix}`, bsdtar `--format={ustar,pax,gnutar,v7}`, star `artype={star,xstar,xustar,exustar,pax,ustar,gnutar,v7tar,suntar}`, busybox tar, Python tarfile (USTAR/GNU/PAX), Go `archive/tar` (USTAR/PAX/GNU), .NET `TarWriter` (V7/Ustar/Pax/Gnu), Node tar, Commons Compress (LONGFILE modes), 7-Zip `-ttar`, all over one fixed generated tree | ~90 |
| T3 | Authority disagreements: pax `path`/`linkpath`/`size` versus ustar (truncation, `@PaxHeader` placeholder, unrelated value), GNU `L` unrelated to ustar name, global pax `path`/`size`, local versus global conflict, duplicate keys in one header, bad pax record lengths, CVE-2025-62518 class | 40 |
| T4 | Sparse: GNU old `S` with extension headers, pax GNU.sparse 0.0, 0.1, 1.0 (GNU tar `--sparse --sparse-version`), bsdtar sparse | 12 |
| T5 | Path forms: `./` prefix, `.` root, `..`, absolute, backslash, trailing slash on non-directories, empty name, NUL in pax path, names > 100 and > 255 bytes | 30 |
| T6 | Hardlinks: forward, chain, to symlink, to directory, repeated target name, data-bearing, never-appearing target, self-link | 16 |
| T7 | End markers: single zero block, none (EOF on boundary), EOF mid-header, EOF mid-data, trailing garbage, non-zero after end, 10240-byte record padding, extra zero blocks | 16 |
| T8 | Duplicates: file/file, file/dir, dir/file, last-wins content | 8 |
| T9 | Non-UTF-8 names: raw bytes in ustar name, invalid UTF-8 pax `path`, `hdrcharset=BINARY`, ISO-8859-1 names written by GNU tar from a Latin-1 tree | 12 |
| T10 | Metadata keywords: SCHILY.xattr.*, LIBARCHIVE.xattr.* (base64), SCHILY.acl.access/default in star and libarchive text forms, SCHILY.fflags, LIBARCHIVE.creationtime, atime/ctime, SCHILY.dev/ino/nlink, mtime with 0-9 and > 9 fractional digits and negative decimals, uname/gname versus uid/gid disagreement, uid > 2097151 base-256 versus pax | 45 |
| T11 | Entry types: directory with content, char/block devices, FIFO, contiguous `7`, unknown vendor typeflags, setuid/sticky and bits above 07777 | 16 |
| T12 | OCI layer constructs: `.wh.` whiteouts, `.wh..wh..opq`, `MSWINDOWS.rawsd`/`MSWINDOWS.fileattr` pax keys (shared with EXP-LEGACY-020) | 8 |
| T13 | Header checksum variants: signed versus unsigned, bad checksum, space/NUL terminators, garbage in numeric fields | 10 |

Real tar tiers (tuning/validation items only): `f20-tuning-real-libarchive-testsuite` (tar/test fixtures),
`f14-apache-maven-3916-bin-tarball`, `f16-oci-ubuntu-noble-multiarch-index` (layer blobs) — tuning;
`f20-validation-real-commons-compress-testdata` (tar subset), `f20-validation-real-cpython-archivetestdata` (tar
subset), `f14-pyspark-420-sdist`, `f16-oci-fedora-44-zstd-layers` (layer blobs, decoded by EXP-LEGACY-004 transport
first) — validation. Held-out: placeholder (C§5).

## 6. Environment

As EXP-LEGACY-001 §6. Extraction in bubblewrap sandboxes (C§12); device-node cases extracted without privilege
escalation (bwrap without `--cap-add`), so device creation outcomes are recorded as the runtime's unprivileged
behaviour.

## 7. Commands and tooling

Tooling: `research/tools/legacy/cases/tar_cases.py`, `research/tools/legacy/cases/tar_writer_matrix.py`, drivers
`readers/{gnutar.py, bsdtar.py, py_tar.py, goread/main.go, busybox.py, star.py, sevenzip.py, DotnetRead/,
noderead.mjs, CommonsReaders.java}`, `ebr-legacy-readers` (Rust `tar` 0.4 and astral-tokio-tar pre-/post-fix),
`ebound_import.py --format tar`, `matrix.py --format tar`, research tar rule model prototype
`research/tools/legacy/models/tar_model.py` (effort logged for DEC-LEG-001).

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
/root/eb-research/venv/bin/python research/tools/legacy/acquire_runtimes.py --registry research/experiments/_legacy/runtime-registry.json --lock research/experiments/_legacy/runtime-lock.json --only-formats tar
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-002 --generator tar_cases --sets T0-T13 --split tuning --seed 2509170201
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-002 --generator tar_cases --sets T0-T13 --split validation --seed 2509170202 --append-corpus-items research/experiments/EXP-LEGACY-002/real-items.txt
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-LEGACY-002/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-002/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-002/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-002/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/matrix.py --experiment EXP-LEGACY-002 --format tar --reference-set tar --out research/normalized/EXP-LEGACY-002/matrix/
```

`tools/tar-strict/observed-outcomes-v1.json` is read, never regenerated in place.

## 8. Seeds

`order 2509170211`, `bootstrap 2509170212`, `command 2509170213`; case seeds §5.

## 9. Warmup, replication, adaptive rule

As EXP-LEGACY-001 §9.

## 10. Measurement and metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `ambiguity_refusal_fraction` (tar-strict/v1 on ambiguous cases) | binary | OD-19 | §3.3 |
| `import_acceptance_fraction` (tar-strict/v1 on unambiguous cases, per case family) | T-20 banded | OD-19 | T-20 |
| `import_agreement_fraction` (tar-strict/v1 EAM versus unanimous reference result) | T-20 banded | OD-19 | T-20 |
| `import_majority_agreement_fraction` | descriptive | OD-19 | none |
| `reason_code_specificity_fraction` | T-20 banded | OD-04 | T-20 |
| `capture_fidelity_fraction` for T10 metadata classes (what readers restore) | T-20 banded | OD-03 | T-20 |
| divergence counts, drift counts, distinguishing cases per candidate profile runtime, model agreement of `tar_model.py` on tuning cases, model effort (hours, lines) | non-canonical descriptive (secondary) | — | none |

## 11. Normalization, 12. Statistical analysis, 13. Practical significance

As EXP-LEGACY-001 §11-§13 with format `tar`. Candidate decision rules (§4) are applied to frozen tuning observations;
the validation split confirms them once (look registry §5.2). Real tiers: F14, F16, F20 only; corpus-level support is
not reachable from real tiers here (scope limitation; EXP-LEGACY-010 supplies the real tail). T-20 `a = 0.5/n_cases`.

## 14. Sensitivity analysis

C§13 arms; plus a **reference-set arm**: recompute ambiguity with and without `7zip-23.01-linux` in the reference set
(M19.2 names it, but its tar handler is a secondary implementation) and with `busybox-tar-linux` added.

## 15. Expected negative results worth recording

GNU tar disagreeing with libarchive on common constructs (argues against a consensus profile); `tar-strict/v1`
refusing constructs every reader handles identically (benign false refusals); sparse variants reconstructed
identically everywhere (weakens refuse-gnu-sparse-keys); star/Solaris dialects unreadable by most readers
(argues for refuse-non-posix-magic); no drift across versions.

## 16. Threats to validity

Hand-built headers may exercise code paths no real writer emits (mitigated by the T2 writer matrix and EXP-LEGACY-010);
unprivileged extraction masks device-node behaviour; Windows `tar.exe` via interop reads from NTFS staging copies;
Rust async reader versions around CVE-2025-62518 must be identified from their advisories (GHSA record cited at
acquisition).

## 17. Platform requirements

Linux x86-64 (WSL2), Windows x86-64 via interop (non-admin); no privileged host operations; macOS bsdtar
PLATFORM_BLOCKED (EXP-LEGACY-080); storage ~15 GB.

## 18. Estimates

Compute ≈ 25 CPU-hours; disk ≈ 15 GB (builds of GNU tar 1.34, libarchive 3.8.8, star ≈ 2 GB; sandboxes transient).
Agent effort: scripted tooling; judgment for triage, R3-contested classes and the tar model effort log.

## 19. Expected cost tiers `[INFERRED]`

New tar compat profile id: T2; new reason codes for authority classes: T2; projecting GNU sparse into native sparse
metadata: T2-T3 (new mapping in an existing record, or new record); new dialect parser surface: T2 plus C4 attack
surface; `opaque-bytes-declaration` touching identity: T4.
