# EXP-LEGACY-080: macOS-produced and macOS-consumed legacy archives, and native ARM64 export determinism (BLOCKED)

| Field | Value |
|---|---|
| Status | **BLOCKED** — missing platforms: (1) a macOS host with APFS (current macOS release; Finder/Archive Utility, `ditto`, `/usr/bin/zip`, `/usr/bin/unzip`, `/usr/bin/tar`); (2) native ARM64 hardware (Apple Silicon macOS and a native Linux arm64 host). Both are `PLATFORM_BLOCKED` on this program's host (PROGRESS standing decision 2026-09-12: no hosted runners; local Windows/WSL2/Docker only). Emulated arm64 determinism is covered by EXP-LEGACY-031 arm C. |
| Domain / program sections | legacy / §27 (export fidelity on macOS), §25 (macOS-produced imports) |
| decision_ids | DEC-LEG-028, DEC-LEG-068, DEC-LEG-076, DEC-LEG-082, DEC-LEG-091, DEC-PLT-020, DEC-LEG-008 |
| Decision types (proposed) | EMPIRICAL |
| OD / HC (proposed) | OD-19, OD-18, OD-03, OD-20; HC-08, HC-07, HC-13 |
| Shared rules / L7 / gates | C§ header, C§2; a decision whose scope includes macOS cannot be `DECIDED` while this experiment is blocked (`decision-method.md` §4.1, §8.4); decisions must exclude macOS explicitly instead |
| Timing | `timing: false` |

## 1. Question

Do macOS consumers (Archive Utility, `ditto`, macOS `unzip`, macOS bsdtar) accept and faithfully restore Entrybound's
frozen and candidate export profiles on APFS; what do macOS producers (Finder Compress, `ditto -c -k --sequesterRsrc`,
macOS `zip`, macOS bsdtar with copyfile AppleDouble) put into ZIP and tar archives and how does Entrybound import it;
and are export bytes identical on native ARM64?

## 2. Hypotheses

- **H1.** Archive Utility restores `zip/portable-v1` outputs with fidelity 1 on core classes and ignores `tar/pax-v1` pax
  keys without stray files; macOS bsdtar reads SCHILY/LIBARCHIVE xattr keys into `com.apple.*`-capable xattrs.
- **H2 (DEC-LEG-076).** `ditto --sequesterRsrc` ZIPs carry `__MACOSX/._*` AppleDouble members for every file with
  resource forks or xattrs; strict import keeps them as ordinary entries (status quo) and no reference reader outside
  macOS folds them.
- **H3 (DEC-PLT-020, DEC-LEG-091).** Archive Utility interprets DOS times as local time and prefers `0x5455` when present;
  macOS `unzip` honours `0x7875` uid/gid only as root.
- **H4 (DEC-LEG-008).** Export bytes on native ARM64 (macOS and Linux) equal x86-64 bytes for every frozen profile.
- **H0.** macOS behaves like libarchive on Linux in every cell.

## 3. Decisions informed

| Decision | Evidence this experiment would supply |
|---|---|
| DEC-LEG-028 | macOS reader column of the export acceptance matrix |
| DEC-LEG-068 | macOS bsdtar and Archive Utility behaviour for pax key families |
| DEC-LEG-076 | Real Finder/ditto-created ZIP corpus; fold/report/drop candidates evaluated |
| DEC-LEG-082 | `add-macos-archive-utility` runtime column in the ZIP differential matrix |
| DEC-LEG-091 | macOS extractor support for Info-ZIP Unix, NTFS and xattr extras |
| DEC-PLT-020 | macOS timestamp interpretation for exported ZIP and pax times |
| DEC-LEG-008 | Native ARM64 byte identity of exports |

## 4. Candidates

As in EXP-LEGACY-001 §4 (DEC-LEG-082), EXP-LEGACY-030 §4 (DEC-LEG-028, 068, 091, PLT-020), EXP-LEGACY-031 §4
(DEC-LEG-008), and for DEC-LEG-076: status-quo-plain-entries; fold-into-forks-xattrs; opt-in-fold; report-and-drop;
aux-opaque.

## 5. Corpus selectors

EXP-LEGACY-030 export artifacts (tuning/validation sources); EXP-LEGACY-001 C0-C14 cases; macOS-produced archives of
the EXP-LEGACY-020 portable fixture tree with added resource forks, `com.apple.quarantine`/`com.apple.FinderInfo`
xattrs and ACLs, created on APFS by each macOS writer (seeds `S_T = 2509178001`, `S_V = 2509178002`). Held-out:
placeholder (C§5).

## 6. Environment (required, unavailable)

macOS (current release) on Apple Silicon and on Intel, APFS case-insensitive and case-sensitive volumes; native Linux
arm64 host; each with the pinned `ebound` build for its architecture.

## 7. Commands (to run when the platform exists)

```sh
# on macOS, repository checkout at the record commit
python3 research/tools/legacy/acquire_runtimes.py --registry research/experiments/_legacy/runtime-registry.json --lock research/experiments/_legacy/runtime-lock-macos.json --only-runtime macos-archive-utility-ditto-bsdtar
PYTHONPATH=research/tools python3 -m ebr run --spec research/experiments/EXP-LEGACY-080/spec.yaml
PYTHONPATH=research/tools python3 -m ebr verify-raw --spec research/experiments/EXP-LEGACY-080/spec.yaml
PYTHONPATH=research/tools python3 -m ebr normalize --spec research/experiments/EXP-LEGACY-080/spec.yaml
```

Drivers: `research/tools/legacy/readers/macos.sh` (Archive Utility through `open -W -a "Archive Utility"` on a copy,
`ditto -x -k`, `/usr/bin/unzip`, `/usr/bin/tar`), `macos_writers.sh` (Finder Compress via AppleScript `System Events`,
`ditto -c -k --sequesterRsrc --keepParent`, `/usr/bin/zip -r -y`, `/usr/bin/tar -c` with and without `--no-mac-metadata`),
`observe_tree_macos.py` (`stat -f`, `xattr -l`, `ls -le`, `GetFileInfo`).

## 8-9. Seeds and replication

`order 2509178011`, `bootstrap 2509178012`, `command 2509178013`; 2 repetitions (3 plus a 20-repetition stress cell for
the determinism arm, as EXP-LEGACY-031).

## 10. Metrics

`export_acceptance_fraction`, `restore_fidelity_fraction`, `cross_platform_restore_fidelity_fraction` (T-20);
`import_acceptance_fraction`, `import_agreement_fraction` (T-20); `migration_determinism_fraction` (binary);
`cross_host_identity_agreement` (binary).

## 11-14. Normalization, analysis, significance, sensitivity

As EXP-LEGACY-001 §11-§14, EXP-LEGACY-030 §11-§14 and EXP-LEGACY-031 §11-§14; APFS case-sensitivity arm added.

## 15. Expected negative results worth recording

Archive Utility rejecting pax outputs or applying normalisation that merges NFC/NFD names; AppleDouble members carrying
metadata Entrybound cannot represent; ARM64 byte differences.

## 16. Threats to validity

Archive Utility automation through `open` differs from interactive double-click; macOS versions change unzip behaviour.

## 17. Platform requirements

macOS/APFS and native ARM64: **PLATFORM_BLOCKED**. No workaround on this host; results stay blocked until the owner
provides the platforms. `remaining-unknowns` cites this experiment for every macOS-scoped and native-ARM64-scoped claim
in the legacy decisions.

## 18. Estimates (if unblocked)

Compute ≈ 30 CPU-hours; disk ≈ 30 GB. Agent effort: scripted drivers; judgment for triage.

## 19. Expected cost tiers `[INFERRED]`

As the referenced experiments; AppleDouble folding into native forks/xattrs: T3-T4 (metadata mapping with identity
participation).
