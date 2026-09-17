# EXP-PLAT-002: Native path representation, hazard and collision behaviour matrix

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-028, DEC-PLT-048, DEC-MOD-038, DEC-PLT-059, DEC-PLT-011, DEC-MOD-010, DEC-MOD-036, DEC-LEG-015, DEC-LEG-054, DEC-LEG-077, DEC-LEG-079
- **Program sections:** 15, 16
- **Decision type (proposed, not assigned):** EMPIRICAL (native behaviour ground truth, extraction outcomes and rule-set accuracy); FORMAL parts for the pinned Unicode and NTFS upcase tables (their content is checked in EXP-PLAT-021). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-18 (M18.1, M18.2, M18.3), OD-03 (M03.2), OD-19 for the legacy rows; HC-08, HC-17, HC-05, HC-07.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** For each native filesystem and OS reachable here, what does the platform do with every LogicalPath hazard class (create, refuse, transform, alias, collide), and which extraction rule set (per-target refusal table, portable superset, declared escape renaming, native bytes, WTF-8/UTF-16 code units, verbatim long paths) and which collision-detection rule set (exclusive create only, pinned full case folding plus NFC, probed rules, per-filesystem tables, conservative superset plus probe) handles every case with no silent merge, overwrite, normalization or skip, at the highest exact-restore fraction on benign trees?
- **H1:** Every target silently transforms or aliases at least one hazard class that at least one other target preserves (for example vfat and the NTFS Win32 layer strip trailing dots, NTFS merges case pairs by default), so the status quo (no target checks, exclusive create only) yields at least one HC-08 MVT-08(b) violation on vfat and on NTFS; a per-target refusal table derived from this matrix yields zero violations on every target and a strictly higher `restore_fidelity_fraction` on the benign corpora of EXP-PLAT-003 than the portable superset on ext4/XFS/btrfs by more than the T-20 absolute rule.
- **H0 / equivalence:** Status quo produces no violation on any target (exclusive create and the kernel already refuse every hazard) and the portable superset is EQUIVALENT to the per-target table on benign trees.
- **Falsification of H1:** Zero violations for the status quo on vfat and NTFS; any violation for the per-target table; or no distinguishable exact-restore difference between the per-target table and the portable superset on Linux targets.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-028 | V1_BLOCKING | What exact rules map LogicalPath components to native names on each target (ext4, XFS, btrfs, APFS, NTFS, ReFS), and which components are refused or renamed: invalid U... | Ground-truth behaviour matrix per target for invalid UTF-8, surrogates, reserved names (version-aware list), trailing dot/space, backslash/colon/control, stream syntax, component and path length; extraction outcomes of each candidate rule set. | Hazard prevalence (EXP-PLAT-003); APFS and ReFS rows (EXP-PLAT-022/023); escape-scheme interop with Cygwin (optional arm here) and macOS (blocked). |
| DEC-PLT-048 | V1_BLOCKING | Which collision classes must extraction detect (case-fold, NFC/NFD normalization, Windows reserved aliases, trailing dot/space, 8.3 short names, confusables), against... | Collision behaviour per target (case, NFC/NFD, compatibility forms, ignorables, reserved aliases, trailing dot/space); miss and false-alarm counts of every collision rule set against ground truth. | Detection cost at large entry counts (EXP-PLAT-015); 8.3 short names and per-directory case sensitivity where elevation is needed (EXP-PLAT-023); APFS (EXP-PLAT-022); false-positive rates on real corpora (EXP-PLAT-003). |
| DEC-MOD-038 | V1_BLOCKING | Do non-UTF-8 / non-Unicode native names (raw POSIX bytes, unpaired UTF-16 surrogates, legacy code pages) need LogicalPath representation in v1, and if so with which en... | Round-trip and extraction outcome of non-UTF-8 byte names on Linux targets and unpaired-surrogate names on NTFS for each representation candidate (native creation, escape mappings, WSL drvfs private-use mapping). | Real-tree frequency (EXP-PLAT-003); traversal-safety fuzzing of decoded forms (EXP-PLAT-014 arm H); identity impact (EXP-PLAT-005). |
| DEC-PLT-059 | V1_IMPORTANT | Should pack and strict legacy import warn about, flag in the archive, or refuse names that collide or are unrepresentable on mainstream targets (SPEC: writers SHOULD w... | Which hazard classes are unrepresentable or colliding on which mainstream targets, which defines the warning classes a pack-time diagnostic would use. | Prevalence and warning volume (EXP-PLAT-003); usability of warnings is HUMAN-FACING (§4.16). |
| DEC-PLT-011 | V1_IMPORTANT | Which refusal classes must INDEXED and STREAM extraction check before creating anything (symlink policy, reparse, name encoding, target hazards, collisions, hardlink s... | Which refusal classes are archive-determinable before writing versus only target-probed, per target. | Staging costs (EXP-PLAT-017); share of real failures preventable (EXP-PLAT-003 × this matrix). |
| DEC-MOD-010 | V1_IMPORTANT | Should the ArchiveDescriptor carry path hostility and collision-class summaries, and if so are they advisory caches or validated authorities, with which enumerated cla... | Stability of Unicode case-fold/normalization outcomes across the pinned table versions and targets; defines the class vocabulary a summary would need. | Collision prevalence and reader-side cost (EXP-PLAT-003, EXP-PLAT-015); container hostility slot (container domain). |
| DEC-MOD-036 | V1_IMPORTANT | Which structural bounds belong in the model versus budgets and extraction policy: LogicalPath component length and depth, symlink target size (currently 1 MiB), and ca... | Measured component-length, depth and path-length limits per target. | Corpus distributions (EXP-PLAT-003); primary-source limits (EXP-PLAT-021). |
| DEC-LEG-015 | V1_IMPORTANT | Is export path refusal keyed to target format or declared extraction runtime, is the ZIP/tar portability asymmetry intentional, are backslash/colon/newline paths refus... | Extractor behaviour matrix (GNU tar, bsdtar, 7-Zip, Info-ZIP unzip, Python tarfile/zipfile, Expand-Archive, tar.exe) for case collisions, reserved names, backslash, colon and newline on Linux and Windows. | Frequency in published trees (EXP-PLAT-003); profile semantics (legacy domain). |
| DEC-LEG-054 | V1_IMPORTANT | How are 7z names with literal backslashes handled: split as separators (status quo), refused as ambiguous, split only for Windows-origin archives, split with a recorde... | Linux and Windows extraction of 7z archives with literal backslash names by 7-Zip 23.01 (Linux) and 7-Zip for Windows. | Host-OS indicators in 7z headers and prevalence (legacy domain, EXP-PLAT-003). |
| DEC-LEG-077 | V1_IMPORTANT | Should ZIP import keep refusing every backslash in names (status quo), interpret backslash as a separator (as 7z import does), do so only under a named compat profile,... | Runtime extraction of ZIP names containing backslash and colon on Windows and Linux. | Producer prevalence (EXP-PLAT-003, legacy domain). |
| DEC-LEG-079 | V1_IMPORTANT | Under compat and preserve, which duplicate member does each profile project (central vs local order, first vs last), should duplicates be refused instead, and what exa... | Extraction-collision outcomes for case-fold and normalization duplicates on case-insensitive targets (NTFS, vfat, cifs). | Duplicate-member order probes through runtimes (legacy domain). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo of each decision (`status-quo-utf8-only-no-target-checks`, `no-detection-status-quo`, `utf8-only-refuse-status-quo`, `silent-status-quo`, `partial-output-status-quo`); REF-INC for extraction: GNU tar 1.35 and bsdtar 3.7.2 (Linux), tar.exe/bsdtar 3.8.8 (Windows).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-028** — candidates: `status-quo-utf8-only-no-target-checks`, `refuse-per-target-hazard-list`, `refuse-portable-superset`, `declared-escape-renaming`, `native-bytes-on-posix`, `wtf8-surrogate-roundtrip`, `verbatim-long-path-prefix` (status quo: `status-quo-utf8-only-no-target-checks`). Treatment: Implemented as a rule predicate in `rulesets.py` and as an extraction arm; scored against ground truth.
  - `verbatim-long-path-prefix`: Measured on NTFS D: with `\\?\` paths for paths beyond MAX_PATH and component limits.
  - excluded before execution (ledger): `silent-sanitize` — Silent mangling makes faithful and relocated extraction indistinguishable; SPEC P8 requires refusal or a declared, reported renaming policy (violates SPEC §3.4 P8 L224 (frozen)); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-048** — candidates: `no-detection-status-quo`, `exclusive-create-only`, `pinned-unicode-full-casefold-nfc`, `probed-target-rules`, `filesystem-specific-tables`, `conservative-superset-plus-probe`, `confusables-advisory`, `confusables-refuse`, `enable-per-directory-case-sensitivity` (status quo: `no-detection-status-quo`). Treatment: Implemented as a collision predicate in `rulesets.py` with its pinned table; scored for misses (HC-08 violations) and false alarms (lost exact restores).
  - `enable-per-directory-case-sensitivity`: Measured only if EXP-PLAT-001 shows the attribute settable without elevation; otherwise routed to EXP-PLAT-023.
- **DEC-MOD-038** — candidates: `utf8-only-refuse-status-quo`, `opaque-bytes-declaration`, `utf16le-code-units`, `wtf8`, `legacy-charset-registry`, `transcode-on-import`, `escape-mapping`, `refuse-with-fidelity-report` (status quo: `utf8-only-refuse-status-quo`). Treatment: Scored on native round trip per target; wire-level representation cost is not measurable here (C1 cost scripts).
- **DEC-PLT-059** — candidates: `silent-status-quo`, `warn-on-stderr-and-report`, `record-hazard-summary`, `strict-import-refuse-option` (status quo: `silent-status-quo`). Treatment: Scored only as the hazard-class definition each diagnostic would use; volume and usability are elsewhere.
  - excluded before execution (ledger): `default-refuse-hazardous-names` — Target-hostile and colliding names are legal archive content so POSIX trees archive faithfully; default refusal at write would contradict the frozen P5/P8 model (violates SPEC §3.4 P5/P8 (frozen); I18); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-011** — candidates: `partial-output-status-quo`, `full-preflight-then-write`, `fresh-sibling-stage-and-rename`, `rollback-created-objects`, `document-only`, `journaled-crash-safe` (status quo: `partial-output-status-quo`). Treatment: Only the preflight-determinability part is measured here.
- **DEC-MOD-010** — candidates: `not-implemented-status-quo`, `validated-summary`, `advisory-cache`, `reader-computed-only`, `extractor-target-specific` (status quo: `not-implemented-status-quo`). Treatment: Only the class vocabulary and table-version stability are measured here.
- **DEC-MOD-036** — candidates: `status-quo-bounds`, `platform-derived-bounds`, `budget-declared-only`, `format-limit-registry` (status quo: `status-quo-bounds`). Treatment: Platform maxima only.
- **DEC-LEG-015** — candidates: `status-quo-asymmetric`, `format-keyed-minimal`, `runtime-profile-keyed`, `rename-with-loss`, `typed-newline-issue` (status quo: `status-quo-asymmetric`). Treatment: Status quo measured through `ebound convert --to zip|tar` refusal outcomes on the same hostile names; runtime-keyed profiles scored against the extractor matrix.
- **DEC-LEG-054** — candidates: `refuse-literal-backslash`, `split-if-windows-origin`, `split-recorded`, `profile-defined` (status quo: `status-quo-split`). Treatment: Scored against observed 7-Zip extraction outcomes on both OSes.
  - excluded before execution (ledger): `status-quo-split` — Splits POSIX literal backslashes into path components without recording the normalisation (violates I18); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-LEG-077** — candidates: `status-quo-refuse`, `normalise-as-separator`, `compat-profile-only`, `literal-component`, `harmonise-refuse-both`, `refuse-colon-components` (status quo: `status-quo-refuse`). Treatment: Scored against observed runtime extraction outcomes on both OSes.
  - excluded before execution (ledger): `normalise-without-record` — Silent normalisation hides legacy ambiguity (violates I18); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-LEG-079** — candidates: `status-quo-last-central-order`, `per-profile-order`, `refuse-duplicates-in-compat`, `first-member-profiles`, `collision-byte-equality-only`, `collision-casefold-nfc` (status quo: `status-quo-last-central-order`). Treatment: Only the extraction-collision class definition is scored here.
  - excluded before execution (ledger): `import-both-duplicates` — Projecting both members under one LogicalPath is unrepresentable (violates I6); L0/R1(b) counterexample and reviewer sign-off still required (R1).

## Corpus, fixtures and case sets

- **Hostility case list:** `scripts/gen_hostility_cases.py --seed <seed> --out cases-<split>.jsonl`; tuning seed 20260917 (`plat002-cases-tuning`), validation seed 20260918 (`plat002-cases-validation`), held-out seed sealed. Each seed draws concrete strings from the same pre-registered class list:
  - case pairs (ASCII; simple versus full folding: `ß`/`SS`/`ẞ`, `ſ`/`s`, `K`/U+212A, `Σ`/`σ`/`ς`, `İ`/`i̇`, Cherokee; code points whose folding differs between Unicode 12.1 and 16.0);
  - NFC/NFD pairs (precomposed versus combining sequences, Hangul), compatibility forms (U+FB01), default-ignorables (U+200B, U+200D), confusables (Cyrillic `а`);
  - reserved device names across Windows versions (`CON`, `PRN`, `AUX`, `NUL`, `COM0`–`COM9`, `COM¹²³`, `LPT0`–`LPT9`, `LPT¹²³`, `CONIN$`, `CONOUT$`), each bare, with an extension, and with trailing spaces;
  - trailing dot, trailing space, leading space, `\`, `:`, `*?"<>|`, bytes 0x01–0x1F and 0x7F, `name:stream`, `name::$DATA`, `PROGRA~1`-style names;
  - invalid UTF-8 (lone continuation, overlong `C0 AE`, encoded surrogates `ED A0 80`), unpaired UTF-16 surrogates (Windows only);
  - length boundaries: 255-byte UTF-8 components, 255 UTF-16 units that exceed 255 bytes, depth 1,024, total path 4,096 bytes and 260/32,767 UTF-16 units.
- **Benign exact-restore population:** the name sets (bytes only) recorded per item by EXP-PLAT-003 over tuning/validation items; no content is read here.
- **Hostile archives:** `scripts/build_hostile_archives.py` writes, per case, a pax tar (Python `tarfile`), a ZIP with the UTF-8 flag (Python `zipfile`), a 7z (`7z a` 23.01 on Linux) and, where `ebound pack` accepts the tree on ext4, a native `.eb` (both layouts).
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **Linux targets:** `ext4`, `xfs`, `btrfs`, `tmpfs`, `vfat-utf8`, `vfat-default`, `ntfs3g-windows-names`, `ntfs3g-posix-names`, `cifs-samba-mangling`, `ext4-casefold` (all via `EXP-PLAT-001/scripts/fsimg.sh`; `ext4-casefold` expected HC_UNVERIFIED; `cifs-samba-mangling` needs Samba from apt and is an optional arm).
- **Windows targets:** `ntfs-win32` (normal paths), `ntfs-verbatim` (`\\?\` paths), `ntfs-casesensitive-dir` (only if settable per EXP-PLAT-001), `drvfs-view` (the same NTFS directory created from WSL through `/mnt/d`, to observe WSL's private-use escape mapping), `git-for-windows-checkout` (host git with `core.protectNTFS` default).
- **env_ids:** `wsl-ubuntu`, `windows-host`. Non-admin on Windows.

| Requirement | Needed? |
|---|---|
| Linux (WSL2 Ubuntu 24.04, kernel 5.15.167.4, root) | yes |
| Windows 11 host (non-admin, NTFS D:) | yes |
| macOS | no (macOS counterpart is EXP-PLAT-022, BLOCKED) |
| x86-64 | yes |
| ARM64 | no (native ARM64 parity is EXP-PLAT-025, BLOCKED) |
| Network | no (downloads of pinned tools only, approved) |
| Privileged | WSL root for loop mounts; Windows non-admin only |
| Large storage | no |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Linux):** `rm -rf /root/eb-research/src/entrybound-plat && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/entrybound-plat && git -C /root/eb-research/src/entrybound-plat checkout <record-commit> && cd /root/eb-research/src/entrybound-plat && CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound` → `/root/eb-research/target/production/release/ebound` (production workspace, no `research-internals`; harness review R1-06).
- **Production CLI (Windows):** `git clone D:/Projects/entrybound/entrybound D:/eb-research/src/entrybound-plat; git -C D:/eb-research/src/entrybound-plat checkout <record-commit>; $env:CARGO_TARGET_DIR='D:/eb-research/target/production-win'; & "$env:USERPROFILE/.cargo/bin/cargo.exe" +1.98.1 build --release -p entrybound-cli --bin ebound` → `D:/eb-research/target/production-win/release/ebound.exe`.
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-002/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-002/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-002/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-002/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-002/spec.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-002/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Scripts to be written (mechanical):**
  - `scripts/path_matrix.py --target <t> --cases <jsonl> --extractors ebound,gnutar,bsdtar,sevenzip,unzip,py312,py313,py314 --scratch <dir> --detail-dir research/raw/EXP-PLAT-002/detail` — per case: create via bytes-level syscalls, list back raw names (`os.scandir` on bytes paths), test aliasing (open the partner name, compare `st_dev/st_ino`), second-create outcome; then extract each hostile archive with each extractor into a fresh root, observing with `strace -f -e trace=%file` and a before/after tree diff of the root's parent (MVT-05(a) Linux oracle); prints one JSON line with `hostile_name_handling_fraction`, `restore_fidelity_fraction`, `hc05_mvt05_escapes`, `hc17_encoded_hazards`, `observation_sha256`.
  - `scripts/path_matrix.ps1 -Mode <m> -Cases <jsonl> -Scratch <dir>` — the Windows counterpart using `CreateFileW` semantics through .NET and `\\?\` paths, `Get-ChildItem -LiteralPath` listings, `fsutil file queryfileid` aliasing, canary directories beside and above the root for extraction (MVT-05(a) Windows oracle), extractors `ebound.exe unpack`, `tar.exe`, 7-Zip portable, `Expand-Archive`, CPython 3.12/3.13/3.14.
  - `scripts/rulesets.py --cases <jsonl> --ground-truth <csv> --benign-names research/normalized/EXP-PLAT-003/decision/names.parquet` — implements every DEC-PLT-028/DEC-PLT-048 candidate predicate with pinned tables: CPython 3.12.3 `unicodedata` (Unicode 15.0.0), UCD 12.1.0 CaseFolding (ext4 casefold), UCD 16.0.0, an NTFS `$UpCase` snapshot read from a fresh `mkntfs` image (`ntfscat -i 10`), and the union for the conservative superset.
  - `scripts/extract_matrix.py` and `scripts/observe_tree.py` — normalizers shared with EXP-PLAT-014.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Arm A — native behaviour ground truth:** per (target, case): outcome ∈ {created-exact, refused (errno/Win32 error), created-transformed (stored bytes differ), aliased (partner resolves to the same object), collided (second create hits the first)}.
- **Arm B — extraction outcomes:** per (target, extractor, archive format, case): faithful (exact name bytes and content), refused with a report, renamed with a report, or silent merge/overwrite/normalization/skip (the MVT-08(b) violation classes). Objects created outside the root count as `hc05_mvt05_escapes`.
- **Arm C — archive-level interpretation:** `ebound inspect --json` of every native hostility archive on WSL and on Windows; any difference in outcome class, reason code, LAI, AUX, PCR or entry-list digest is a `cross_host_identity_agreement` failure (MVT-08(a)/(b) first oracle).
- **Arm D — rule-set accuracy:** each candidate predicate versus Arm A ground truth: *miss* = the rule lets through a case the target would alias, merge or transform (would be an HC-08 violation if extraction proceeded); *false alarm* = the rule refuses a case the target preserves exactly (reduces exact restores). False alarms are evaluated both on the hostility case list and on the benign name sets of EXP-PLAT-003.
- **Arm E — limits:** maximum component bytes/units, depth and path length per target (DEC-MOD-036).

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `hostile_name_handling_fraction` | OD-18 | binary R1 screen (M18.3), per (candidate, target) | correctness | fraction | equal_one | HC-08 (`metric_thresholds.hostile_name_handling_fraction`; A.2 role `binary`) | stdout_json |
| `cross_host_identity_agreement` | OD-18 | binary R1 screen (M18.1) for native hostility archives across WSL and Windows | correctness | binary | equal_one | HC-08 (`metric_thresholds.cross_host_identity_agreement`; A.2 role `binary`) | analysis join of Linux and Windows inspect outputs |
| `restore_fidelity_fraction` | OD-03 | primary (entries restored with exact name bytes on the target, over benign name sets and representable hostility cases) | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | stdout_json and `rulesets.py` |
| `cross_platform_restore_fidelity_fraction` | OD-18 | primary for cross-OS pairs (archives packed on WSL extracted on Windows and vice versa) | other | fraction | maximize | T-20 (`metric_thresholds.cross_platform_restore_fidelity_fraction`; A.2 role `banded`) | analysis |
| `hc05_mvt05_escapes` | OD-18 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-05 MVT-05(a)/(b) | strace/canary oracle |
| `hc17_encoded_hazards` | OD-18 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-17 MVT-17 | native archives that encode a structural hazard (should be 0) |
| `platform_coverage_count` | OD-18 | descriptive (M18.4) | other | count | report | - (`metric_thresholds.platform_coverage_count`; A.2 role `descriptive`) | analysis |

## Normalization

Name bytes are recorded as hex; Windows names as UTF-16 code-unit hex. Every extraction observation is reduced to the per-case outcome vocabulary above; the reference for relative comparisons is the status quo of each decision. Unicode table versions are part of each predicate's identifier (`casefold-full-nfc@UCD15.0.0`).

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Scopes:** evaluation conditions are (target) for native behaviour and (source host, target) for cross-OS pairs. A rule set with any miss on an in-scope target is eliminated at R1 for that scope; survivors are compared on `restore_fidelity_fraction` over benign name sets (R4), then R6 by computed tier (reason codes and tables are T2; a user-visible renaming mode is T2; a new encoding declaration is T4 via identity participation).
- **R9 expressibility:** a per-target winner is admissible only if the extractor can determine the target before writing (EXP-PLAT-001 predictor accuracy); otherwise the partition is recorded as inexpressible.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Table-version arm:** every collision predicate is re-scored with UCD 12.1.0, 15.0.0 and 16.0.0 tables; a selection that depends on the table version is flagged in `known_limitations` (reopen trigger: a Unicode release that changes the folding of any case in the list).

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Status quo is expected to leave silent case merges on NTFS (exclusive create detects only the second alias after partial output) and silent trailing-dot stripping on vfat — defects to record, not to excuse.
- Linux vfat accepts `CON` while Windows refuses it (smoke): a Linux-probed rule cannot predict Windows behaviour, which bounds the probed-rules candidate.
- The portable superset may refuse a measurable share of real benign trees (for example source trees containing `aux.c` or `con.h`), which is the cost to report against the per-target table.
- Expand-Archive or other incumbents may overwrite silently on collisions; such results enter the incumbent comparison and the SPEC §22 claim review (EXP-PLAT-004 DEC-PLT-024).

## Threats to validity

- Windows reserved-name behaviour changed across Windows releases; results are specific to build 10.0.26200 and are re-run when the OS build changes.
- The NTFS `$UpCase` table of D: is not readable without elevation; the snapshot from `mkntfs` may differ from the table written when D: was formatted. The risk is reported and the table version is part of the predicate identity.
- ntfs-3g `windows_names`, drvfs and Samba mangling are emulation layers with their own rules; they are separate conditions, never pooled with native NTFS.
- APFS normalization-insensitive behaviour and ReFS are not measured (EXP-PLAT-022/023); any decision scope that includes them stays blocked.
- Hostile names generated from classes may miss real-world combinations; EXP-PLAT-003 supplies the real-tree complement.

## Estimated cost and effort

- **Compute:** about 8 machine-hours; **disk:** about 6 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-002/scripts/{gen_hostility_cases.py,path_matrix.py,path_matrix.ps1,build_hostile_archives.py,rulesets.py,extract_matrix.py,observe_tree.py} (mechanical); pinned downloads: Unicode UCD 12.1.0/15.0.0/16.0.0 CaseFolding.txt and DerivedNormalizationProps; 7-Zip for Windows portable (7zr/7z extra, pinned version and SHA-256); CPython 3.13 and 3.14 standalone builds for Linux and Windows (pinned); optional Samba (apt) for the cifs arm
- **Agent effort:** execution none (scripted); tooling scripted (mechanical); judgment for the rule-set screening and the interpretation of any silent transformation.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
