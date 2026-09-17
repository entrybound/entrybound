# EXP-PLAT-012: Named streams, forks, origin markers and salvage markers: representation cost, restorability and survival

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-027, DEC-PLT-031, DEC-MOD-035, DEC-INT-034, DEC-PLT-060, DEC-CON-001
- **Program sections:** 15, 17, 20, 21, 24
- **Decision type (proposed, not assigned):** EMPIRICAL (bytes, restorability, survival, propagation); security review of attacker-supplied streams and of origin-marker policy is EXTERNAL/analytical input (§8.3 applies only if a security proof is required). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03 (M03.1, M03.2), OD-05 (M05.1, M05.2), OD-25 (M25.4); HC-07, HC-05, HC-10, HC-06.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** What do NTFS alternate data streams (including Zone.Identifier) cost and do under each classification candidate (declared unavailable, AUX named values like xattrs, ContentObject-backed named streams participating in LAI, per-class split), how do they restore onto non-NTFS targets, how do Windows extractors propagate the archive's mark of the web, and which in-band salvage/origin marker forms survive common copy, sync and archive transports on Linux and Windows?
- **H1:** Representing large streams as AUX named values fails restoration above filesystem xattr value limits (ext4 without ea_inode, 16 MiB frozen bound) and forfeits dedup, while the ContentObject proxy keeps `artifact_bytes` within the T-01 band of plain files; adding a Zone.Identifier to an unchanged file changes AUX under the AUX candidate and LAI under the LAI candidate; Explorer propagates MOTW to extracted members while tar.exe and the status-quo `ebound unpack` do not (an `unsafe_defaults` outcome for extraction of marked archives); xattr/ADS markers are lost by at least half of common transports, filename suffixes and marker manifests survive all of them.
- **H0 / equivalence:** Stream representation candidates are EQUIVALENT in bytes and restorability; all extractors propagate MOTW; all marker forms survive the same transports.
- **Falsification of H1:** AUX named-value representation restores every stream size on every Linux target, or `ebound unpack`/tar.exe propagate MOTW, or xattr markers survive every transport.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-027 | V1_BLOCKING | How are NTFS alternate data streams (including Zone.Identifier), macOS resource forks, FinderInfo, AppleDouble sidecars and xattr analogues classified: LAI-participati... | Stream size/count cost and restorability per candidate; restore onto ext4/XFS/btrfs/tmpfs via ntfs-3g xattr mapping; identity churn when streams change; status-quo undeclared ADS loss. | Resource fork sizes (EXP-PLAT-022, BLOCKED); security review of attacker-supplied streams (analytical/external); AppleDouble reconciliation (legacy domain). |
| DEC-PLT-031 | V1_IMPORTANT | How does Entrybound handle origin/provenance markers: preserve raw member markers (Zone.Identifier, com.apple.quarantine/provenance) only, define a canonical cross-pla... | MOTW propagation by Explorer (Shell COM), 7-Zip (default and -snz), tar.exe, Expand-Archive, Python zipfile/tarfile and `ebound unpack` on the Windows host; survival of raw member markers. | quarantine/provenance payloads (EXP-PLAT-022, BLOCKED); safe ADS write API (EXP-PLAT-016); archiver MOTW-bypass advisory survey (analytical, primary sources). |
| DEC-MOD-035 | V1_BLOCKING | Which metadata classes and model objects participate in which identity root (LAI, AUX, PCR, PCI or none), including pending classes such as ownership names, NTFS ADS/f... | Identity-root churn for forks/streams and origin markers under each assignment candidate. | Other classes (EXP-PLAT-005). |
| DEC-INT-034 | V1_IMPORTANT | Which in-band unverified marker is used per destination type for salvage output (report only, filename suffix, xattr/ADS, marker manifest, quarantine layout, new archi... | Marker survival through cp, rsync, tar, zip, 7z, git, robocopy, Copy-Item and Explorer copy for xattr/ADS markers, filename suffixes, marker manifests and quarantine layouts on ext4 and NTFS. | APFS (EXP-PLAT-022); cloud sync (owner consent, EXP-PLAT-023); simulated first-use visibility (HUMAN-FACING). |
| DEC-PLT-060 | V1_IMPORTANT | Are the frozen xattr bounds (at most 4096 attributes per entry, values at most 16 MiB, names 1-255 bytes) justified, or should they be derived from filesystem limits,... | Restore reports for oversize named values (XFS or NTFS stream to ext4) and values near the frozen 16 MiB bound. | Distributions (EXP-PLAT-003); per-filesystem maxima (EXP-PLAT-021). |
| DEC-CON-001 | V1_IMPORTANT | Do the eight ResourceBudget fields bound every attacker-controllable allocation (dictionaries, Merkle or outboard data, recipients, signatures, preserved legacy source... | Stream/xattr count and size ladders that a budget axis would bound; metadata-byte growth under the AUX representation. | Allocation-path audit (container domain). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo (`status-quo` NTFS ADS neither captured nor declared; `status-quo-none` origin handling; `report-only` for salvage markers; `status-quo-4096-16mib`).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-027** — candidates: `status-quo`, `declare-ads-unavailable`, `ads-as-xattr-analogue`, `named-content-streams-in-lai`, `per-class-split`, `appledouble-materialize-on-export`, `refuse-files-with-streams` (status quo: `status-quo`). Treatment: `ads-as-xattr-analogue` realized by Linux capture of ntfs-3g `streams_interface=xattr` (production records `user.*` xattrs as AUX); `named-content-streams-in-lai` approximated by sibling files (byte costs only; identity semantics differ, stated as a proxy limit); `declare-ads-unavailable` and status quo measured on Windows capture; `per-class-split` derived from the two; others `[derived]`.
- **DEC-PLT-031** — candidates: `status-quo-none`, `preserve-raw-only`, `propagate-archive-marker-default`, `canonical-origin-metadata`, `caller-policy-flag`, `propagate-when-source-marked` (status quo: `status-quo-none`). Treatment: Status quo measured; propagation candidates scored against the measured behaviour of the incumbent implementing each rule (Explorer = propagate-archive-marker-default; 7-Zip -snz = propagate-when-source-marked; tar.exe = none).
- **DEC-MOD-035** — candidates: `status-quo-assignment`, `named-content-streams-in-lai`, `forks-in-aux`, `origin-markers-none`, `origin-markers-aux`, `refuse-unassigned` (status quo: `status-quo-assignment`). Treatment: Evaluated by re-projection on measured facts.
  - excluded before execution (ledger): `unauthenticated-hints` — Leaves asserted archive bytes outside LAI/AUX, allowing silent alteration in signed archives (violates SPEC §8.0 L885 metadata coverage; I13); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-INT-034** — candidates: `report-only`, `filename-suffix`, `xattr-or-ads-marker`, `marker-manifest-file`, `quarantine-directory`, `salvage-to-new-archive`. Treatment: Each marker form is measured for survival; report-only is the zero-survival reference; salvage-to-new-archive survival equals native archive transport (always survives as bytes).
  - excluded before execution (ledger): `no-marking` — Salvage artifacts must be marked unverified (violates I25); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-060** — candidates: `status-quo-4096-16mib`, `linux-vfs-limits`, `budget-declared-only`, `per-platform-limit-table`, `status-quo-16mib-4096`, `linux-vfs-64kib`, `per-platform-maximum`, `large-values-as-content-objects` (status quo: `status-quo-4096-16mib`, `status-quo-16mib-4096`). Treatment: Bound candidates scored against measured restore outcomes and EXP-PLAT-003 distributions.
- **DEC-CON-001** — candidates: `status-quo-eight-fields`, `add-declared-fields`, `caller-policy-only-new-limits`, `fixed-format-bounds-registry`, `hole-logical-len-bound` (status quo: `status-quo-eight-fields`). Treatment: Only the stream/xattr axis inputs are measured.

## Corpus, fixtures and case sets

- **Stream fixture:** `gen_stream_fixture.ps1 -Seed <s>` on D: and `gen_stream_fixture.py --seed <s>` on an ntfs-3g image: stream sizes {0, 1, 26, 4 KiB, 64 KiB, 1 MiB, 16 MiB, 64 MiB} × counts {1, 8, 256}, Zone.Identifier with ZoneId 3 and HostUrl, duplicate stream content across files; tuning seed 20260917 (`plat012-fixture-tuning`), validation 20260918 (`plat012-fixture-validation`).
- **Corpus:** `f19-tuning-real-ntfs-metadata`, `f19-validation-real-ntfs-metadata` (real Zone.Identifier and multi-stream files in NTFS images).
- **Marked archives (Windows):** ZIP, 7z and tar archives of the fixture given a Zone.Identifier (ZoneId=3) with `Set-Content -Stream` (simulating a download; no network), plus 7z archives storing member ADS (`-sns`).
- **Transports:** Linux `cp`, `cp -a`, `cp --preserve=xattr`, `mv` across filesystems, `rsync -a`, `rsync -aX`, GNU tar default and `--xattrs`, bsdtar, zip/unzip, 7z, `git add/commit/clone`; Windows `Copy-Item`, `robocopy /COPY:DAT`, Explorer copy (Shell.Application `CopyHere`), 7-Zip `-sns`, tar.exe, host git; cross ext4→NTFS through `\\wsl.localhost`.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_ids:** `wsl-ubuntu` (root), `windows-host` (non-admin). Explorer shell automation runs in the interactive user session via COM (no UI clicks). OneDrive and cloud-sync transports need owner consent and are routed to EXP-PLAT-023. No timing.

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
- **Harness (only where named):** `wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh build --release` (→ `/root/eb-research/target/harness/release/`), after `C:/Python313/python.exe research/harness/lock_parity.py` reports `RESULT PASS` (harness review R1-06).
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-012/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-012/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-012/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-012/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-012/spec.yaml'`
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-012/spec-markers.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-012/spec-markers.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-012/spec-markers.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-012/spec-markers.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-012/spec-markers.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-012/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-012/spec-windows-markers.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Scripts:** `stream_repr_arms.sh --repr <statusquo-windows-interface|xattr-analogue|sibling-content-proxy> --item <path> --item-id <id> --scratch <dir>` (mount the image with the matching ntfs-3g option or materialize siblings, `ebr-pack` accounting, `ebound pack/unpack` onto ext4/XFS/btrfs/tmpfs/ntfs-3g, observe); `marker_transports.sh --transport <t>` and `marker_transports.ps1 -Transport <t>` (apply every marker form to a salvaged-output fixture, run the transport, observe survival); `motw_propagation.ps1 -Extractor <e>` (extract marked archives, `Get-Item -Stream Zone.Identifier` on every output).

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Representation:** `artifact_bytes`, `metadata_bytes`, `dedup_stored_fraction` per candidate; capture and restore per target; restore reports for oversize values.
- **Identity churn:** entries whose LAI or AUX facts change when a Zone.Identifier is added.
- **MOTW:** per extractor, fraction of members carrying a Zone.Identifier after extraction from a marked archive; default extraction that drops the archive's mark counts toward `unsafe_defaults` (pre-registered as a MOTW-bypass class).
- **Survival:** per (marker form, transport), marker present and intact after the transport.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | primary for representation cost (T-01) | size | B | minimize | T-01 (`metric_thresholds.artifact_bytes`; A.2 role `banded`) | `ebr-pack` |
| `metadata_bytes` | OD-05 | diagnostic; primary when the decision encodes streams as metadata | size | B | minimize | T-02 (`metric_thresholds.metadata_bytes`; A.2 role `diagnostic`) | `ebr-pack` |
| `dedup_stored_fraction` | OD-05 | diagnostic | size | ratio | minimize | T-01 (`metric_thresholds.dedup_stored_fraction`; A.2 role `diagnostic`) | `ebr-pack` |
| `capture_fidelity_fraction` | OD-03 | primary for stream capture | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | observation |
| `restore_fidelity_fraction` | OD-03 | primary for stream restore and for marker survival (marker class through transport case list) | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | observation |
| `unsafe_defaults` | OD-25 | binary screen for MOTW-dropping extraction defaults | correctness | count | equal_zero | HC-05 (`metric_thresholds.unsafe_defaults`; A.2 role `binary`) | `motw_propagation.ps1` |
| `hc07_mvt07a_undeclared_differences` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-07 MVT-07(a) | observation versus reports |

## Normalization

Byte metrics exact; survival binary per case. Reference: status quo (streams) and report-only (markers).

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Proxy limit:** sibling-file results support only byte and restorability claims about ContentObject-backed streams; identity-semantics claims come from the re-projection model.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Size-ladder arm:** leave-one-size-class-out.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- `ebound unpack` has no marker handling, so extraction from a marked archive leaves unmarked files: expected `unsafe_defaults` ≥ 1 under the MOTW class.
- Git drops xattr and ADS markers; only filename or manifest markers survive a commit.

## Threats to validity

- Shell COM extraction may differ from interactive Explorer drag-and-drop in MOTW handling; recorded as a separate condition.
- ntfs-3g stream mapping is a transport representation, not Windows' own API.
- Defender SmartScreen reputation is not exercised (no execution of extracted files).

## Estimated cost and effort

- **Compute:** about 5 machine-hours; **disk:** about 12 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-012/scripts/{gen_stream_fixture.py,gen_stream_fixture.ps1,stream_repr_arms.sh,marker_transports.sh,marker_transports.ps1,motw_propagation.ps1} (mechanical); 7-Zip for Windows portable (pinned); reuse `identity_reproject.py` (EXP-PLAT-005) for identity churn
- **Agent effort:** execution none (scripted); tooling scripted; judgment for the sibling-file proxy validity note and for classifying MOTW-bypass outcomes.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
