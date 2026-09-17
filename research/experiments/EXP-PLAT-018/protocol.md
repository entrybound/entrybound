# EXP-PLAT-018: Source instability and local revision signals under seeded concurrent mutation

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **NEEDS_TOOLING**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-CMP-028, DEC-ACC-023, DEC-LEG-056, DEC-PLT-044
- **Program sections:** 12, 15, 21, 25
- **Decision type (proposed, not assigned):** EMPIRICAL (detection by predicate per filesystem; coherence of captured entries); regression evidence for seeded schedules (objective MVT-07(d)). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03 (M03.1), OD-06 (M06.5 descriptive retry cost); HC-07 (MVT-07(d)), HC-15 (digest verification of returned bytes).
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** Under seeded concurrent mutation of source files during pack and of archive files during local random reads, which stability predicate (length plus mtime; plus ctime or change time; plus file ID; content re-hash; deny-write share modes; filesystem snapshot) detects every mixed-version capture on ext4, XFS, btrfs, tmpfs, vfat, ntfs-3g, cifs and NTFS, what retry count suffices on live trees, and does digest verification catch every modified byte returned by random reads?
- **H1:** Same-size rewrites with restored mtime evade the length-plus-mtime predicate on every filesystem and the Windows status-quo predicate on NTFS, are detected by ctime on POSIX filesystems and by ChangeTime on NTFS, but not on vfat (2-second granularity, no ctime); the status quo therefore admits at least one mixed-version entry without declaration (HC-07 MVT-07(d) violation) on NTFS; deny-write share modes prevent the Windows writer; btrfs snapshots eliminate mixed versions; random reads never return unverified modified bytes.
- **H0 / equivalence:** The status-quo predicates detect every mutation event on every filesystem.
- **Falsification of H1:** Zero mixed-version entries for the status quo across all schedules and filesystems, or ctime/ChangeTime missing a same-size rewrite.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-CMP-028 | V1_IMPORTANT | How should pack handle sources that change during capture: retry count (default 2), stability predicate (Unix full stat set versus Windows length and mtime only), and... | Fault injection on NTFS, ext4, XFS, btrfs, vfat and cifs (SMB) with same-size rewrites; capture success versus retry count on live trees; incumbent behaviour (GNU tar 'file changed as we read it', bsdtar, 7-Zip) under the same schedules. | VSS and FAT on Windows (EXP-PLAT-023); snapshot API safety (EXP-PLAT-016). |
| DEC-ACC-023 | V1_IMPORTANT | What revision signal must local random-read sources use (length, mtime, file ID, locks, content hash of metadata) and what guarantee is documented when concurrent modi... | Windows share modes, POSIX in-place same-length writes, cifs loopback, mtime granularity; digest verification of returned bytes under archive mutation. | Network filesystems beyond loopback (EXP-PLAT-024). |
| DEC-LEG-056 | V1_IMPORTANT | Which sidecar workflow is stable v1: source formats and modes (tar/7z compat or preserve), dry-run, encryption, normative <artifact>.eb naming for discovery, snapshot... | Concurrent-modification tests of `ebound sidecar` on Windows and Linux. | Preservation overhead, discovery workflow and encryption composition (legacy/integrity domains). |
| DEC-PLT-044 | POST_V1 | Which platform snapshot mechanisms (VSS, APFS, btrfs, ZFS, LVM) does pack --snapshot support in v1, and how is snapshot-consistent versus best-effort capture recorded? | Torn-capture frequency under concurrent writers with current checks; btrfs snapshot feasibility as root on a loop filesystem. | VSS (EXP-PLAT-023); APFS (EXP-PLAT-022); ZFS (FreeBSD VM optional, EXP-PLAT-011 image). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production (`status-quo-abort`, `status-quo-length-mtime`, `status-quo-content-bearing-zip-tar-7z`, `status-quo-best-effort`).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-CMP-028** — candidates: `status-quo-abort`, `exclude-with-report`, `include-with-record`, `snapshot-integration`, `stronger-predicate` (status quo: `status-quo-abort`). Treatment: Status quo measured directly; alternative predicates evaluated by `predicate_models.py` over the logged per-event snapshots (would the predicate have fired?), and share-mode/snapshot candidates run as real arms.
  - excluded before execution (ledger): `silent-include` — Unstable content included silently (violates SPEC §5.7a L621-625); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-ACC-023** — candidates: `status-quo-length-mtime`, `file-id-plus-mtime`, `share-mode-locking`, `digest-backstop-only`, `snapshot-copy` (status quo: `status-quo-length-mtime`). Treatment: Status quo measured directly; alternative predicates evaluated by `predicate_models.py` over the logged per-event snapshots (would the predicate have fired?), and share-mode/snapshot candidates run as real arms.
- **DEC-LEG-056** — candidates: `status-quo-content-bearing-zip-tar-7z`, `add-dry-run-and-encryption`, `normative-naming-discovery`, `lock-or-rehash-snapshot`, `content-free-sidecar-role`, `defer-tar-7z-modes` (status quo: `status-quo-content-bearing-zip-tar-7z`). Treatment: Status quo measured directly; alternative predicates evaluated by `predicate_models.py` over the logged per-event snapshots (would the predicate have fired?), and share-mode/snapshot candidates run as real arms.
- **DEC-PLT-044** — candidates: `status-quo-best-effort`, `btrfs-zfs-helpers`, `vss-helper`, `record-best-effort-flag-only`, `defer-post-v1` (status quo: `status-quo-best-effort`). Treatment: Status quo measured directly; alternative predicates evaluated by `predicate_models.py` over the logged per-event snapshots (would the predicate have fired?), and share-mode/snapshot candidates run as real arms.

## Corpus, fixtures and case sets

- **Generated trees** (seeded 20260917 tuning `plat018-schedules-tuning`, 20260918 validation `plat018-schedules-validation`): 200 files from 10 KiB to 64 MiB; mutation schedules: 200 seeded events per run at offsets relative to pack start, plus strace-synchronized events at each file's first `read` (Linux) for deterministic interleaving.
- **Live-tree arm:** `f05-tuning-gen-logs-a-small` and `f05-validation-gen-logs-b` copied and continuously appended at seeded rates.
- **Archive-mutation arm:** INDEXED archives of `f04-tuning-sourcelike` read with `ebound read` while rewritten in place.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **Targets:** `ext4`, `xfs`, `btrfs`, `btrfs-snapshot`, `tmpfs`, `vfat`, `ntfs3g`, `cifs-samba-loopback`; Windows `ntfs-d` and `ntfs-d-sharemode` (writer opened with deny-write sharing by the reader probe).
- **env_ids:** `wsl-ubuntu`, `windows-host`. No timing claims (retry cost is descriptive).

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
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-018/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-018/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-018/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-018/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-018/spec.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-018/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Scripts:** `scripts/instability_arms.sh --target <t> --schedules-from <item_id> --scratch <dir>` (mutator plus `ebound pack`, `ebound sidecar`, GNU tar and bsdtar under the same schedule; `coherence_oracle.py` compares each captured entry with the version log; `predicate_models.py` scores alternative predicates), `scripts/instability_arms.ps1` (NTFS; share-mode arm), `scripts/retry_cost.py` (retry counts {0,1,2,4,8}).

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Schedules:** each repetition uses a different seeded mutation schedule recorded in the raw row; determinism is required of the oracle outcome per schedule, not of the schedule itself.
- **Detection bound:** seeded schedules are regression evidence; the static audit of capture code for by-path re-stat/re-open (objective MVT-07(d)) is committed alongside and covers that class completely.

## Measurement method and metrics

- **Mixed-version entry:** content, size or metadata of a captured entry not equal to one logged version from a single descriptor state, without exclusion report or `fidelity.unstable_source` record.
- **Predicate detection:** per event, whether each predicate would have fired between open and close snapshots.
- **Share modes:** writer outcome while the reader holds the file.
- **Random reads:** returned bytes versus versions; verification status reported.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `hc07_mvt07d_mixed_version_entries` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-07 MVT-07(d) | `coherence_oracle.py` |
| `capture_fidelity_fraction` | OD-03 | primary for disposition candidates (entries captured coherently and not excluded) | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | oracle |
| `unstable_source_retry_cost_s` | OD-06 | descriptive (M06.5) | other | s | report | - (`metric_thresholds.unstable_source_retry_cost_s`; A.2 role `descriptive`) | `retry_cost.py` |
| `roundtrip_mismatches` | OD-02 | binary gating for non-mutated files | correctness | count | equal_zero | HC-02 (`metric_thresholds.roundtrip_mismatches`; A.2 role `binary`) | oracle |

## Normalization

Events and entries normalized to {coherent, excluded-with-report, recorded-unstable, mixed-silent}. Reference: status quo.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Schedule arm:** leave-one-mutation-type-out.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- vfat cannot support any metadata predicate for sub-2-second rewrites; only content re-hash or snapshots detect them.
- If the status quo already re-hashes content, predicate differences matter only for cost; recorded either way.

## Threats to validity

- Seeded schedules do not model adversarial writers timed by oplocks or inotify.
- cifs loopback timestamps come from Samba's view of ext4.

## Estimated cost and effort

- **Compute:** about 16 machine-hours; **disk:** about 20 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-018/scripts/mutator.py (seeded rewrite-same-size with restored mtime, truncate, extend, rename-over, touch-back; logs every version's SHA-256, size and full stat/`GetFileInformationByHandleEx` snapshot); research/experiments/EXP-PLAT-018/scripts/{instability_arms.sh,instability_arms.ps1,coherence_oracle.py,predicate_models.py,strace_sync.py,sharemode_writer.ps1,retry_cost.py}; `ebr-inspect-meta` (EXP-PLAT-004) for per-entry content digests
- **Agent effort:** tooling judgment once (oracle and synchronization), then scripted; execution none; analysis judgment.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
