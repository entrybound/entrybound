# EXP-LEGACY-020: candidate adapter and export-profile formats — EAM fidelity mapping, hazard behaviour and writer determinism (cpio, ar, OCI layers, NAR, squashfs, WIM, RAR5, ISO9660, CAB, LHA, XAR, lzip, LZ4, lzma-alone, zlib, .Z, brotli, 7z export)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (fixture-tree builder, per-format writer/reader drivers, hazard fixture generators incl. hand-built RAR5/NAR/OCI-Windows headers, independent observation tool `observe_tree.py`, builds of lhasa/lha, xar, squashfs-tools-ng, schilytools; downloads of nix static, umoci, crane, unrar, 7-Zip 26) |
| Domain / program sections | legacy / §26 (adapter priority inputs), §27 (candidate export profiles) |
| decision_ids | DEC-LEG-005, DEC-LEG-024, DEC-LEG-029, DEC-LEG-034, DEC-LEG-035, DEC-LEG-037, DEC-LEG-039, DEC-LEG-042, DEC-LEG-047, DEC-LEG-051, DEC-LEG-057, DEC-LEG-073 |
| Decision types (proposed) | EMPIRICAL (fidelity, hazard behaviour, determinism) with FORMAL parts (format specifications: `cpio(5)`, `ar(5)`, OCI image-spec layer.md, Dolstra 2006 thesis §5.2 NAR serialisation, squashfs format docs, [MS-WIM] if retrievable, RAR 5.0 technote, ECMA-119/Rock Ridge/Joliet, [MS-CAB], XAR TOC schema); licence and legal parts EXTERNAL |
| OD / HC (proposed) | OD-19, OD-03 (M03.1), OD-20 (M20.2 for export candidates), OD-22/OD-24 cost inputs (via EXP-LEGACY-021); HC-07, HC-17, HC-05, HC-06 |
| Shared rules / L7 / gates | C§ header, C§2 |
| Timing | `timing: false` |

## 1. Question

For each candidate legacy format, what fraction of each Entrybound metadata class can the format carry through its
reference writer and readers, how do its readers handle the format's known hazards, what would an adapter have to
refuse or record, and — for candidate export profiles — are the writers deterministic and do mainstream readers
accept their output?

## 2. Hypotheses

- **H1 (fidelity ordering).** Per metadata class (path bytes, kind, content, executable, mode, uid/gid, names,
  mtime precision, xattrs, ACLs, hardlinks, symlinks, sparse, empty directories), NAR and cpio newc carry fewer classes
  than OCI layers (pax), which carry fewer than WIM on its own class set; RAR5, CAB and LHA each fail at least three
  POSIX classes `[INFERRED]`. *Falsified* per pair by the opposite ordering of `capture_fidelity_fraction`.
- **H2 (hazards).** For every format, at least one reader extracts at least one hazard fixture (traversal, symlink-then-
  file, duplicate names, inode reuse, whiteout-to-parent) outside the sandbox root or merges entries silently.
  *Falsified* per format by every reader refusing or recording every hazard.
- **H3 (NAR, DEC-LEG-039).** The NarHash of a tree computed from EAM-level inputs (sorted names, content, executable
  bit, symlink targets) equals `nix hash path` of the restored tree for every fixture without non-projectable metadata.
  *Falsified by* one mismatch.
- **H4 (7z export, DEC-LEG-051).** 7-Zip 23.01 `-mmt=1` output is byte-identical across repetitions but differs across
  thread settings or across 23.01/26.x; py7zr output differs across versions. *Falsified by* byte identity across all.
- **H5 (Windows-origin hazards in layers, DEC-LEG-042).** OCI unpackers (umoci, crane, podman rootless) disagree on at
  least one of: opaque whiteout after re-add, cross-layer hardlink, duplicate path within a layer.
- **H0.** All candidate formats are equivalent in fidelity and hazard handling.

## 3. Decisions informed

| Decision | Supplied here | Remaining elsewhere |
|---|---|---|
| DEC-LEG-005 (cpio, ar) | Fidelity per class for newc/odc/bin/crc and GNU/BSD ar; hazards (hardlink inode reuse, device nodes, member-name tables, duplicate members); `.deb` member-order mapping | Prevalence EXP-LEGACY-010; scoring EXP-LEGACY-021 |
| DEC-LEG-035 (ISO9660, CAB, LHA, XAR, RAR5 long tail) | Reader agreement and hazard outcomes of libarchive vs format-native tools; divergent namespaces (Joliet vs Rock Ridge) | Sandboxing cost, CVE history EXP-LEGACY-021 |
| DEC-LEG-037 (lzip, LZ4 frame, lzma-alone, zlib, `.Z`, brotli; tar.lz/tar.lz4) | Writer determinism and reader acceptance of tar.lz/tar.lz4 import and export candidates | Integrity semantics EXP-LEGACY-004; scoring EXP-LEGACY-021 |
| DEC-LEG-039 (NAR) | NAR to EAM mapping round trips; NarHash-from-EAM feasibility; hazard fixtures | Nix demand evidence (external) |
| DEC-LEG-042 (OCI layer import) | Whiteout, opaque, Windows `Files/`/`Hives/` + `MSWINDOWS.*` pax keys, hardlink and duplicate-path handling by unpackers | Registry prevalence EXP-LEGACY-010 |
| DEC-LEG-047 (RAR5) | Hazard fixtures (unknown extra records, PUA-mapped names, dictionary sizes, header encryption, service headers) across unrar, libarchive 3.8.8, 7-Zip 26 | Licence/legal review EXTERNAL; CVE history EXP-LEGACY-021 |
| DEC-LEG-051 (7z export) | LZMA2 writer determinism and header-writer ordering; reader matrix for produced archives | Demand evidence (external) |
| DEC-LEG-057 (squashfs) | uid/gid table, name and inode limits; traversal fixtures (CVE-2021-40153 and CVE-2021-41072 classes) through unsquashfs 4.6.1, squashfs-tools-ng, 7-Zip, `backhand` | Demand (AppImage/snap) EXP-LEGACY-010 prevalence |
| DEC-LEG-073 (WIM/ESD) | wimlib capture/apply fidelity (reparse, ADS, security descriptors, hardlinks), LZX/XPRESS/LZMS/solid decode agreement wimlib vs 7-Zip; LZMS decoder memory (informational) | Encrypted ESD: **BLOCKED** (no producer or fixture available without Microsoft media and licence acceptance); DISM capture: **BLOCKED** (admin) |
| DEC-LEG-029 (additional export profiles) | Fidelity, determinism and reader acceptance for cpio, OCI layer (pinned dialect proxy), standalone streams, tar.lz/tar.lz4, 7z candidates | tar/pax-v2 and ZIP-extras candidates EXP-LEGACY-030; human survey EXTERNAL |
| DEC-LEG-034 | Construct inventories per format that the LOM would need to represent (inputs to the paper mapping) | Mapping prototypes EXP-LEGACY-051 |
| DEC-LEG-024 | EAM fidelity mapping per format (evidence dimension of the scoring rule) | EXP-LEGACY-021 |

## 4. Candidates

Per decision the ledger candidates are carried to EXP-LEGACY-021 (classification). Measured objects here are formats,
writers and readers, not candidates. Exclusions: none. Blocked arms named in §3.

## 5. Corpus selectors

- **Fixture trees** (`research/tools/legacy/cases/adapter_fixture_tree.py`; F19; `S_T = 2509172001`,
  `S_V = 2509172002`): portable tree (regular files, empty files and dirs, executables 0755/0744, symlinks relative and
  dangling, hardlink pairs, mtimes with ns fractions and pre-1970, uid/gid 0/1000/70000, user xattrs, access and
  default ACLs, a sparse file, names > 255 bytes, non-ASCII and non-UTF-8 names, case-fold pair); corpus items
  `f19-tuning-zoo`, `f04-tuning-sourcelike` (tuning) and `f19-validation-deep`, `f19-validation-real-home-snapshot`
  (validation).
- **Hazard fixtures** (`research/tools/legacy/cases/adapter_hazards.py`): per format as listed in §3, hand-built from
  the cited specifications.
- **Real tiers (tuning/validation only):** `f11-ubuntu-noble-debs` (ar), `f16-oci-ubuntu-noble-multiarch-index`
  (OCI), `f16-alpine-virt-3241-iso` (ISO9660), `f14-legacy-writer-matrix-tuning` (WIM, CAB, cpio members),
  `f20-tuning-real-libarchive-testsuite` (RAR, RAR5, LHA, XAR, CAB, ISO, cpio fixtures) — tuning;
  `f11-fedora-44-rpms` (cpio payloads), `f10-fedora-44-static-libraries` (ar), `f16-oci-fedora-44-zstd-layers` (OCI),
  `f16-debian-13-netinst-iso` (ISO9660), `f14-legacy-writer-matrix-validation`,
  `f20-validation-real-commons-compress-testdata` (ar, cpio, arj, dump) — validation.
- Held-out: placeholder (C§5).

## 6. Environment

`wsl-ubuntu` (root for loop mounts, bubblewrap sandboxes, rootless podman); Windows 7-Zip 26 via interop; no Docker.
Independent observation of source and extracted trees with `stat`, `getfattr`, `getfacl`, `filefrag`, `lsattr`
(`observe_tree.py`), never with Entrybound.

## 7. Commands and tooling

Tooling: `cases/adapter_fixture_tree.py`, `cases/adapter_hazards.py`, drivers `readers/{cpio_ar.py, oci_unpack.py,
nar.py, squashfs.py, wim.py, longtail.py, sevenzip.py}`, `research/tools/legacy/observe_tree.py`,
`research/tools/legacy/narhash_from_eam.py` (computes NarHash from `ebound inspect --json --entries` plus content
digests), `research/tools/legacy/adapter_matrix.py`.

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
/root/eb-research/venv/bin/python research/tools/legacy/acquire_runtimes.py --registry research/experiments/_legacy/runtime-registry.json --lock research/experiments/_legacy/runtime-lock.json --only-formats cpio,ar,oci-layer,nar,squashfs,wim,rar5,iso9660,cab,lha,xar,lzip,lz4,brotli,7z
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-020 --generator adapter_fixture_tree,adapter_hazards --split tuning --seed 2509172001
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-020 --generator adapter_fixture_tree,adapter_hazards --split validation --seed 2509172002 --append-corpus-items research/experiments/EXP-LEGACY-020/real-items.txt
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-020/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-020/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-020/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/adapter_matrix.py --experiment EXP-LEGACY-020 --out research/normalized/EXP-LEGACY-020/
```

## 8. Seeds

`order 2509172011`, `bootstrap 2509172012`, `command 2509172013`; fixture seeds §5.

## 9. Warmup, replication

No warmup; writer determinism uses 3 repetitions per cell plus a 20-repetition stress cell at maximum threads for each
writer claimed deterministic (objective MVT-13(a) pattern); all other cells 2 repetitions with repeat-hash.

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `capture_fidelity_fraction` per (format, metadata class, writer→reader) | T-20 | OD-03 | T-20 |
| `export_acceptance_fraction` per candidate export profile over the reader set | T-20 | OD-19 | T-20 |
| `migration_determinism_fraction` (candidate export writers) | binary when determinism is claimed | OD-20 | §3.3 |
| `declared_gap_count` per format | descriptive | OD-03 | none |
| hazard outcome table (refuse / record / normalise / escape per reader), NarHash equality per fixture, reader agreement | non-canonical descriptive (NarHash equality is a FORMAL feasibility check) | — | none |

## 11-12. Normalization and analysis

Trees compared by independent observation per class (C§6 extraction fingerprint plus class-specific observations);
format fidelity is reported per (format, class) under AGG-S (worst reader) and AGG-C (per family). Writer determinism
follows §4.13 (differing outputs are the violation artifact). No candidate selection happens here.

## 13. Practical significance

T-20 `a = 0.5/n_cases`; binary §3.3; descriptive none.

## 14. Sensitivity analysis

Reader-set arm (libarchive-only versus format-native tools); fixture-tree arm (portable tree versus F19 items);
writer-option arm (e.g. mksquashfs `-all-root`, wimlib `--unix-data`, xorriso RR/Joliet on/off).

## 15. Expected negative results worth recording

A format expected to be low-fidelity carrying more than predicted (e.g. WIM with `--unix-data`); readers of popular
formats (cpio, squashfs) escaping the sandbox on hazards; NarHash-from-EAM infeasible for any tree (argues against a NAR
export conformance check); 7z writers deterministic (weakens the determinism objection to 7z export).

## 16. Threats to validity

Writers configured by designers (options recorded); rootless podman may differ from rootful containerd; WIM fidelity
without Windows-native capture (DISM blocked) under-represents Windows metadata; RAR5 fixtures limited to libarchive's
test suite and hand-built headers (no RAR5 writer licence acquired).

## 17. Platform requirements

Linux x86-64 WSL2 (root inside the VM for loop mounts; rootless podman); Windows 7-Zip via interop; **BLOCKED parts:**
Windows DISM capture (admin), encrypted ESD fixtures (no public producer; Microsoft media licence acceptance is an owner
action), macOS-created `.pkg`/XAR (PLATFORM_BLOCKED; Linux `xar` stands in); storage ~30 GB.

## 18. Estimates

Compute ≈ 15 CPU-hours; disk ≈ 30 GB. Agent effort: scripted drivers; judgment for hazard fixture construction from
specifications and for the LOM construct inventory.

## 19. Expected cost tiers `[INFERRED]`

Any new import adapter: T2 (new registered import format, new reason codes) plus C4 attack surface; adapters needing
`unsafe`/native dependencies (libarchive wrapping, unrar): T3 and licence screen; export profiles: T2 each (new
profile) with permanent golden vectors (C7).
