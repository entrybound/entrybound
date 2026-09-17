# EXP-LEGACY-003: 7z implementation differential and coder/filter matrix

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (generator `sevenz_cases.py` S1-S9 incl. hand-built headers; acquisition of 7-Zip 26.x Windows/Linux, p7zip 16.02 build, 7-Zip-zstd fork, py7zr; Commons `SevenZFile` driver; `matrix.py --format 7z`) |
| Domain / program sections | legacy / §25 (with §26 coder scope, §31 dependency questions) |
| decision_ids | DEC-ECO-047, DEC-LEG-012, DEC-LEG-031, DEC-LEG-032, DEC-LEG-049, DEC-LEG-050, DEC-LEG-052, DEC-LEG-054, DEC-LEG-055, DEC-PLT-010, DEC-PLT-021 |
| Decision types (proposed) | EMPIRICAL with FORMAL parts (7-Zip `7zFormat.txt`/`7zAes.cpp` at pinned source; triage rule) |
| OD / HC (proposed) | OD-19, OD-04, OD-03, OD-17 (AES KDF cost, dictionary sizes); HC-07, HC-17, HC-06, HC-08 (backslash interpretation independent of host) |
| Shared rules / L7 / gates | C§ header, C§2 |
| Timing | `timing: false` (KDF decode cost recorded as informational wall time only) |

## 1. Question

How do 7-Zip (23.01 Linux, 26.x Windows and Linux), p7zip 16.02, the 7-Zip-zstd fork, py7zr, libarchive and Commons
Compress read, and Entrybound strict 7z import treat, pre-registered 7z constructions: names with literal
backslashes, Windows versus Unix attribute encodings, missing timestamps, anti items, SFX/trailing/multi-volume
layouts, CRC-less streams, every upstream and fork coder and filter, and AES with varying KDF cost?

## 2. Hypotheses

- **H1 (strict floor).** Strict 7z import refuses or records every ambiguous case. *Falsified by* one silent import.
- **H2 (backslash, DEC-LEG-054).** For a Linux-created literal-backslash name, 7-Zip on Windows extracts it as a path
  separator while at least one Linux reader keeps a literal backslash, so the construct is ambiguous. *Falsified by*
  identical interpretation across all readers.
- **H3 (0x8000 gating, DEC-LEG-055).** At least one Windows-origin attribute value with a non-zero high word and no
  `0x8000` flag (for example `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` 0x00400000, `FILE_ATTRIBUTE_PINNED` 0x00080000) is
  interpreted as POSIX mode bits by at least one reader and yields `core.executable = true` under the ungated status
  quo. *Falsified by* no reader and no Entrybound projection interpreting it.
- **H4 (missing MTime, DEC-LEG-049).** No reader restores the epoch for an entry without MTime (readers use
  extraction time or leave it unset), so the status-quo epoch placeholder matches no runtime.
- **H5 (coders, DEC-LEG-052).** Upstream 7-Zip decodes ARM64, RISCV, BCJ2 and PPMd; libarchive 3.8.8 and py7zr each
  fail at least one of them; fork codec ids (`04F71101` Zstd, `04F71102` Brotli, `04F71104` LZ4) are refused by every
  non-fork reader.
- **H6 (AES KDF, DEC-LEG-012/052).** The pinned 7-Zip source defines the key derivation as `2^NumCyclesPower` SHA-256
  iterations with default power 19; 7-Zip accepts power 24 fixtures (16.7 M iterations) with no cap.
  *Falsified by* the source defining a different construction or 7-Zip refusing high powers.
- **H0.** All readers agree on every pre-registered construction.

## 3. Decisions informed

| Decision | Supplied here | Remaining elsewhere |
|---|---|---|
| DEC-ECO-047 | 7z columns of the unified matrix; controls | — |
| DEC-LEG-012 | 7z AES header-encrypted and data-only fixtures: reader behaviour, KDF construction check, decode wall time per power (informational) | Prevalence EXP-LEGACY-010; crypto/security review external (§8.3) |
| DEC-LEG-031 | SFX, prefix, trailing data, multi-volume `-v` outcomes per reader | Prevalence EXP-LEGACY-010 |
| DEC-LEG-032 | `-snl` symlinks and `-snh` hardlinks written on Linux and Windows; reader extraction outcomes | Safety review of link semantics; prevalence EXP-LEGACY-010 |
| DEC-LEG-049 | Reader treatment of absent MTime/partial defined vectors | LAI effect of omission EXP-LEGACY-051; prevalence by producer EXP-LEGACY-010 |
| DEC-LEG-050 | Malformed-case matrix across 7z implementations; distinguishing cases per candidate profile runtime | Prevalence of each strict 7z refusal EXP-LEGACY-010; LOM exactness EXP-LEGACY-051 |
| DEC-LEG-052 | Coder/filter decode support per reader including BCJ2, ARM64, RISCV, PPMd, fork codecs; method-id cross-check against pinned 7-Zip source | Coder distribution EXP-LEGACY-010; decoder crates audit EXP-LEGACY-021; dictionary bombs EXP-LEGACY-041 |
| DEC-LEG-054 | Literal backslash names written by p7zip 16.02, 7-Zip 23.01 and 7zz 26.x on Linux; extracted on Linux and Windows | Prevalence; host-OS indicator availability (header audit here) |
| DEC-LEG-055 | Attribute fixtures with and without `0x8000`; false-executable and false-refusal counts per candidate rule | Real Windows OneDrive placeholder archives: BLOCKED in part (see §17) |
| DEC-PLT-010 | 7z entry with directory attribute and a data stream | — |
| DEC-PLT-021 | 7z CTime/ATime and Windows attribute projection per reader | Prevalence EXP-LEGACY-010 |

## 4. Candidates

| Decision | Candidates (ledger ids) | Notes |
|---|---|---|
| DEC-LEG-012 | status-quo-refuse; decrypt-winzip-aes-and-7z-aes-only; decrypt-all-including-zipcrypto; opaque-preservation | premise evidence only; selection needs crypto review |
| DEC-LEG-031 | status-quo-asymmetric; refuse-all-unexplained; record-as-omission-evidence; sfx-aware-offset-fixup; support-spanned-and-volumes | 7z part |
| DEC-LEG-032 | status-quo-refuse-zip-7z; symlinks-and-reparse; refuse-links-everywhere | zip-strict-v2-symlinks in EXP-LEGACY-001 |
| DEC-LEG-049 | status-quo-placeholder-captured; omit-and-report-unavailable; placeholder-reported-unavailable; refuse-missing-mtime | — |
| DEC-LEG-050 | status-quo-strict-only; sevenzip-26-profile; p7zip-profile; libarchive-7z-profile; defer-post-v1 | py7zr profile: critic-slot proposal |
| DEC-LEG-052 | status-quo-core-set / status-quo-subset; add-arm64-riscv-bcj2; add-bcj2-ppmd; add-ppmd; add-branch-filter-family; full-upstream-7zip-set; accept-fork-codecs / fork-codecs; aes-with-password; measured-threshold | — |
| DEC-LEG-054 | status-quo-split; refuse-literal-backslash; split-if-windows-origin; split-recorded; profile-defined | — |
| DEC-LEG-055 | status-quo-ungated; gate-on-0x8000; record-both-refuse-conflict | — |
| DEC-PLT-010 | status-quo (7z analogue); normalise-with-conflict-report; import-as-file; policy-selectable | silent-normalise proposed L0, still scored |
| DEC-PLT-021 | status-quo-report-unavailable; sevenz-windows-namespaces; lom-evidence-only | — |

## 5. Corpus selectors

Generated (`research/tools/legacy/cases/sevenz_cases.py`; F20; `S_T = 2509170301`, `S_V = 2509170302`):

| Set | Content | Approx. cases |
|---|---|---|
| S1 | Names: literal backslash (p7zip 16.02, 7-Zip 23.01, 7zz 26 on Linux ext4), colon, unpaired UTF-16 surrogates (hand-built), `..`, absolute, empty | 20 |
| S2 | Attributes: `0x8000` with high-word mode 0o100755/0o100644/0o120777; high word without `0x8000` (0x00400000, 0x00080000, 0x00100000, arbitrary); Windows-only attributes | 20 |
| S3 | Times: MTime absent (`-mtm=off`), partial defined vectors, CTime/ATime present, out-of-range FILETIME | 12 |
| S4 | Structure: anti items (`7z u -up...`), empty files/dirs, SFX (Windows `7z.sfx`/`7zCon.sfx` modules), prefix and trailing data, multi-volume `-v1m`, CRC-less streams, encoded and plain headers, unknown FilesInfo property ids, `kDummy` padding, start-header CRC error, next-header offset beyond EOF, large NumFolders | 40 |
| S5 | Coders and filters: COPY, LZMA, LZMA2 (dictionaries 64 KiB..1536 MiB), PPMd (orders, memory), BZip2, Deflate, Deflate64, Delta, BCJ x86/PPC/IA64/ARM/ARMT/ARM64/SPARC/RISCV, BCJ2 (4 streams), chains (BCJ+LZMA2, Delta+LZMA2), multiple folders, solid; fork Zstd/Brotli/LZ4/LZ5/Lizard | 60 |
| S6 | AES-256: data-only and header-encrypted; NumCyclesPower 0, 8, 19, 22, 24 (hand-built from the pinned 7-Zip source construction) | 10 |
| S7 | Directory attribute with data stream; file without data stream but size > 0 | 6 |
| S8 | Links: `-snl` symlinks (relative, absolute, dangling) and `-snh` hardlinks, Linux and Windows writers | 12 |
| S9 | Writer matrix over one fixed tree: 7-Zip 23.01, 7zz 26, 7-Zip 26 Windows, p7zip 16.02, py7zr, 7-Zip-zstd fork, bsdtar `--format 7zip`, Commons `SevenZOutputFile`; presets `-mx0..9`, `-ms=on/off`, `-mhc=on/off`, `-mtc/-mta` | 60 |

Real tiers (tuning/validation only): `f14-legacy-writer-matrix-tuning` (7z members), `f20-tuning-real-libarchive-testsuite`
(7z fixtures) — tuning; `f14-legacy-writer-matrix-validation` (7z members),
`f20-validation-real-commons-compress-testdata` (7z subset) — validation. Held-out: placeholder (C§5).

**Source citation arm (FORMAL).** The 7-Zip source release matching `7zz-26-linux` is downloaded, hashed, and the
method-id table (`CPP/7zip/Archive/7z/7zHeader.h`, `Methods.txt`) and AES key derivation (`CPP/7zip/Crypto/7zAes.cpp`)
are cited as `path:line` for H5/H6; p7zip 16.02 source likewise for attribute handling of `0x8000`.

## 6. Environment

As EXP-LEGACY-001 §6. Windows-side writers (7-Zip 26, fork) run through interop on NTFS staging; symlink writing on
Windows without Developer Mode is recorded as not available (not a runtime outcome).

## 7. Commands and tooling

Tooling: `cases/sevenz_cases.py`, `cases/sevenz_writer_matrix.py`, drivers `readers/{sevenzip.py, py7zr_read.py,
bsdtar.py, CommonsReaders.java}`, `ebound_import.py --format 7z`, `matrix.py --format 7z`,
`research/tools/legacy/source_cite.py` (extracts and hashes cited source lines).

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
/root/eb-research/venv/bin/python research/tools/legacy/acquire_runtimes.py --registry research/experiments/_legacy/runtime-registry.json --lock research/experiments/_legacy/runtime-lock.json --only-formats 7z
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-003 --generator sevenz_cases --sets S1-S9 --split tuning --seed 2509170301
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-003 --generator sevenz_cases --sets S1-S9 --split validation --seed 2509170302 --append-corpus-items research/experiments/EXP-LEGACY-003/real-items.txt
/root/eb-research/venv/bin/python research/tools/legacy/source_cite.py --lock research/experiments/_legacy/runtime-lock.json --runtime 7zz-26-linux --paths CPP/7zip/Archive/7z/7zHeader.h,CPP/7zip/Crypto/7zAes.cpp,DOC/Methods.txt --out research/normalized/EXP-LEGACY-003/source-citations.json
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-003/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-003/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-003/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/matrix.py --experiment EXP-LEGACY-003 --format 7z --reference-set 7z --out research/normalized/EXP-LEGACY-003/matrix/
```

## 8. Seeds

`order 2509170311`, `bootstrap 2509170312`, `command 2509170313`; case seeds §5.

## 9. Warmup, replication

As EXP-LEGACY-001 §9.

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `ambiguity_refusal_fraction` | binary | OD-19 | §3.3 |
| `import_acceptance_fraction` (unambiguous cases, per case family) | T-20 | OD-19 | T-20 |
| `import_agreement_fraction` | T-20 | OD-19 | T-20 |
| `reason_code_specificity_fraction` | T-20 | OD-04 | T-20 |
| `capture_fidelity_fraction` (S2/S3 attributes and times) | T-20 | OD-03 | T-20 |
| false-executable count and false-refusal count per DEC-LEG-055 rule; coder support table; KDF decode wall per power (informational, not decision-grade timing) | non-canonical descriptive | — | none |

## 11-13. Normalization, analysis, practical significance

As EXP-LEGACY-001 §11-§13 with format `7z` and reference set C§7. Real 7z tiers are few (F14, F20): corpus-level
verdicts from real tiers are unreachable; recorded as a scope limitation. T-20 `a = 0.5/n_cases`.

## 14. Sensitivity analysis

C§13 arms; reference-set arm with py7zr replaced by `7zz-26-linux`; writer arm (drop each S9 writer in turn).

## 15. Expected negative results worth recording

Strict 7z import refusing archives from mainstream writer defaults; `gate-on-0x8000` breaking p7zip archives whose
writers omit the flag; no divergence for backslash names on one platform pair (weakens split-if-windows-origin);
fork codecs present in real samples (EXP-LEGACY-010) though every upstream reader refuses them.

## 16. Threats to validity

Real Windows OneDrive placeholder attributes cannot be produced non-interactively here (hand-built attributes stand in;
S2 is FORMAL on the attribute encoding, EMPIRICAL on readers only); 7-Zip SFX modules on Linux differ from Windows;
p7zip 16.02 builds with a current compiler may differ from distribution binaries (build flags recorded).

## 17. Platform requirements

Linux x86-64, Windows x86-64 (interop, non-admin); OneDrive placeholder capture: **BLOCKED part** (needs a signed-in
OneDrive client and interactive session; recorded, not required for the hand-built S2 fixtures); storage ~20 GB
(1536 MiB dictionary fixtures); no privileged operations.

## 18. Estimates

Compute ≈ 10 CPU-hours (AES power-24 fixtures dominate); disk ≈ 20 GB. Agent effort: scripted; judgment for source
citation review and triage.

## 19. Expected cost tiers `[INFERRED]`

Adding BCJ2/ARM64/RISCV/PPMd decoders: T2 (registered extended import scope) or T3 if any dependency contains
`unsafe`/native code; fork codecs: T3 plus maintenance penalty risk; AES support: T4 (cryptographic material) and
EXTERNAL review; `gate-on-0x8000`: T1 if it changes only projection, T4 if it changes identity participation.
