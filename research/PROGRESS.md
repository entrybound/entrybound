# Entrybound Comprehensive Research Program — Progress Log

This is the living, resumable log of the Comprehensive Entrybound Evidence and
Product-Decision Research Program. It is updated and committed at every
atomically meaningful change. If work is interrupted, start at
[How to resume](#how-to-resume).

Status: **IN PROGRESS** — `COMPREHENSIVE RESEARCH PROGRAM: INCOMPLETE` until the
completion gate (program §43) is audited and met.

## Program baseline

| Item | Value |
|---|---|
| Program start | 2026-09-12 |
| Research baseline SHA (`dev` = `origin/dev` at start) | `9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155` |
| `origin/main` at start (never modified) | `bf5b533f68d5a9afa8303178432d40f8b411a4dd` |
| Worktree at start | clean |
| Baseline test run | `cargo test --workspace --release` on Windows host: 286 tests passed, 0 failed |

### External (unpublished) inputs — referenced by hash, never copied into this repo

| Input | SHA-256 |
|---|---|
| `design/2026-08-29-entrybound-product-architecture.md` (the "SPEC") | `1f881c7b13f193b41353b90066d33ca6abcb7d4c3dcd4f58e22834801f0fe29c` |
| `design/research-appendix/entrybound-architecture-research-appendix.tgz` | `47a3240a685300206353c84c348ac504266b2e3e335bc05c441369a9e7d4db99` |
| `research/2026-08-29-archive-category-research.md` (Research I) | `bc524da26e85e20bce515b81a9e84ba1a412599ca66b08bdda7ab3005c4b8cfe` |
| `research/2026-08-29-entrybound-opportunity-validation.md` (Research II) | `8c3575a463e492d99298400c2bf77f0f8b6fa260368671b23398e00c9c1bd301` |
| `research/2026-08-29-entrybound-research-iii-final-gate.md` (Research III) | `bb209e0c74bf3aac4067efdf54aeb70efd0cee232945903e6b11d1bdfee495c3` |
| `research/entrybound-research-iii-instrumentation.tgz` | `d221f434dd92641156003fe4b1fa9d50ba7fa10d4c848374cb37cd14017810c8` |

Paths are relative to the parent directory `D:\Projects\entrybound` (not a git
repository). `research/tools/sources/extract_external_sources.ps1` verifies the
hashes and extracts the two tarballs to a stable directory.

## Standing decisions and constraints (from the program owner)

| Date | Decision | Consequence |
|---|---|---|
| 2026-09-12 | No GitHub Actions / hosted runners; platform work is local only (Windows host, WSL2, Docker images). | macOS/APFS and native ARM64 evidence is `PLATFORM_BLOCKED`; ARM64 via QEMU-in-Docker is labeled *emulated*. ReFS requires admin and is blocked on this host. |
| 2026-09-12 | Research checkpoints are committed to `dev` and pushed to public `origin/dev`. `main` is never modified. | Ledgers paraphrase and cite the unpublished inputs by section and hash; the documents themselves are not committed. |
| 2026-09-12 | Downloads of public third-party corpora and tools are approved. | Large inputs live outside git under `/root/eb-research` (WSL ext4); only source definitions, manifests, and hashes are committed. |
| 2026-09-12 | Commit and document progress at every atomically meaningful change. | This log plus per-phase commits; workflow scripts are copied into `research/orchestration/workflows/`. |
| 2026-09-16 | Usage-budget pacing (orchestrator decision after two limit interruptions: the 5-hour limit on 2026-09-13 and the weekly limit on 2026-09-13, reset 2026-09-16). | Check plan usage (`get_usage`) between workflows and keep weekly use at or below about 14% per day. Use Sonnet for mechanical stages (provisioning, verification, harness boilerplate, experiment execution) and the session model for judgment stages (merges, method, experiment design, analysis, adversarial review). Merge agents read compact topic shards (`research/tools/ledger/make_merge_shards.py`). Workflows stay small so progress can be checkpointed and usage re-checked between them. |
| 2026-09-16 | **Never consume C: drive space** (program owner, urgent). | All research storage lives on D:: the WSL Ubuntu distro disk (move to `D:\WSL\Ubuntu`), the Docker Desktop data disk (`D:\Docker\wsl`), `/root/eb-research` (inside the D:-hosted distro), and cargo targets (`D:\eb-research\target`). `research/orchestration/disk_guard.py` must pass (C: free ≥ 25 GB, distro BasePath on D:, no Docker data disk on C:) before any workflow launches. |
| 2026-09-12 | Production semantics are not changed by experiments. Alternate algorithms/parameters stay in research-only tooling or default-off research features until a Decision Ledger entry is accepted. | Any research feature in production crates must be default-off and proven byte-identical when disabled. |

## Execution environment

- **Windows host:** Windows 11 Pro 10.0.26200, Intel i9-14900HX (8 P-cores + 16 E-cores, 32 logical), 64 GB RAM, NTFS, non-admin session, balanced power scheme. Tools: Rust 1.97.1 at `%USERPROFILE%\.cargo\bin` (not on the default PATH), Python 3.13.5 at `C:\Python313\python.exe`, Java 21, PowerShell 7, bsdtar 3.8.8, Docker Desktop 29.7.2.
- **WSL2 Ubuntu 24.04 (root):** ext4 with about 769 GB free, 31 GB RAM visible, 8 GiB swap. Rust stable 1.98.1 plus nightly-2026-08-01 and cargo-fuzz 0.13.2. Installed via apt: GNU tar 1.35, bsdtar 3.7.2, 7-Zip 23.01 (`7z`/`7za`/`7zr` from Ubuntu package `7zip` 23.01+dfsg-11; `p7zip-full` is only a transitional package, so there is no p7zip 16.02 binary and no `7zz` on PATH), zstd 1.5.5, xz 5.4.5, gzip 1.12, pigz, lz4, lzip, brotli, pixz, par2, hyperfine, cpio, squashfs-tools, xfsprogs, btrfs-progs, attr, acl, sqlite3, jq.
- **Pinned research Rust toolchain: `1.98.1` on both Windows and WSL** (installed explicitly on 2026-09-13). Floating `stable` differs between the hosts (Windows 1.97.1/LLVM 22.1.6, WSL 1.98.1/LLVM 22.1.8). A WSL `bash script.sh` does not source `~/.cargo/env`, so plain `rustc` resolves to Ubuntu's `/usr/bin/rustc` 1.75.0. Research scripts must invoke `/root/.cargo/bin/cargo +1.98.1` (WSL) or `%USERPROFILE%\.cargo\bin\cargo.exe +1.98.1` (Windows) explicitly.
- **Timing and fidelity caveats:**
  - Microsoft Defender real-time protection is on, and Hyper-V/VBS runs the Windows host under the hypervisor; both affect Windows-side I/O timing.
  - `/mnt/d` in WSL is mounted without `metadata`, so metadata-fidelity experiments must run on ext4 under `/root/eb-research`.
  - Inside WSL the quiet-machine guard sees only VM CPU accounting (host load appears as steal time), and `taskset` cannot target host P- or E-cores.
  - Temperatures and throttling cannot be read without admin.
- **Docker:** Linux containers run on the same WSL2 kernel. `sch_netem` is absent in both WSL2 and Docker, so network emulation must be application-level (documented threat to validity). QEMU binfmt ARM64 emulation works.
- **Fingerprints:** machine-readable fingerprints are in `research/environment/` (`index.json`); usernames and hostnames are redacted.
- **Data root outside git (WSL):** `/root/eb-research/{corpus,heldout,cache,venv,src,target}`.

## Phase plan and status

Legend: DONE / RUNNING / NEXT / PLANNED / BLOCKED.

| Phase | Scope (program sections) | Status | Artifacts / notes |
|---|---|---|---|
| A. Source-of-truth audit | §2, §3 | DONE (2026-09-17) — final run `wf_16094fa5-38e` (`phase-a3-ledgers-method.js`); earlier attempts `wf_da687d30-ac3` and `wf_84e35026-465` were interrupted by usage limits | Extraction: 36 slices, 8,429 records. 13 compact-shard merges plus the earlier model/access/integrity merges. Ledgers after critic round 1 (`f56dd3d`): `research/requirement-ledger.csv` has 1,518 requirements and `research/decision-ledger.jsonl` has 593 decisions (all INSUFFICIENT_EVIDENCE or EXTERNAL_REVIEW_REQUIRED); mechanical extraction-key coverage is 8,429/8,429. Also `research/supersession-ledger.md`, `research/source-inventory.md`, `research/audit/coverage-report.md`. |
| B1. Infrastructure & corpus | §4, §5, §6, §7 | DONE WITH CONDITIONS (2026-09-17) — closure `wf_a924ee34-6e0` (`phase-b1e-corpus-closure.js`) completed; the detached size pass continues | Corpus `ebrc-2026.09-v1`: 300 items (manifest `ed28b1b4…`), `provision --check` 300/300 OK. The round-2 critic `research/corpus/critique-round2.md` rates it **ADEQUATE_FOR_TUNING_AND_VALIDATION = YES, WITH CONDITIONS**: all 7 round-1 blockers are closed or substantially closed. Conditions: G02 DB dumps partial (no pg_dump -Fc/-Fd without Docker); G03 registry source packages absent; G17 F02 CI artifacts and validation large tier open; cross-split overlaps (F03 yq/fzf, bat/ripgrep; new F17 kernel headers) documented as covariates rather than removed. **HELD-OUT = NOT YET ADEQUATE** (draft `heldout-lock.json`, no design freeze, G01 sealed-supplement decision pending). Fixes: ebr backslash-path bug; fingerprint extent map now drops the page cache before SEEK_DATA/SEEK_HOLE (a root cause of earlier flip-flopping identities); upstream-digest cache checks. Baselines `f516306`; EXP-BASE-SIZE small tier complete, medium sample running detached. |
| Gate: method pre-registration | §3.3, §37 | DONE — pre-registration commit `14b977c`; round-1 review `research/methods/method-review-round1.md` recorded 85 findings (10 BLOCKER): 84 fixed, 1 accepted risk (RD-3) | `research/archetypal-objective.md`, `research/decision-method.md`, `research/methods/thresholds.json` (status `pre-registered`). No decision may reach DECIDED until the RD-4 gates listed under Open integration items are closed and a round-2 method review has re-checked every RD-4 row. |
| B2. Research harness | §4, §7, §9, §12 | DONE (2026-09-17) — `wf_b2e35b8c-f65` (`phase-b2r-harness.js`) | research-internals `c9a2d3a` (default-off, additive-only, 8/8 CLI outputs identical with and without the feature); harness workspace `research/harness` with 7 crates: ebr-common, ebr-pack `ab31252`, ebr-chunk `50b6a1c` (gear-norm-v1 + 8 CDC algorithms), ebr-codec `9d80208`, ebr-planner `f144878` (exhaustive search, regret, alternative selectors), ebr-access `ee5118e`, ebr-netem `4593671`. Integration `68dfa5d`: 249/249 tests on WSL, Windows builds clean. Adversarial review `5bd33f8` (`research/harness/harness-review-round1.md`): 16 HIGH/MEDIUM findings, 14 fixed. **Binding use constraints:** decision-grade timing uses `ebound` built from the production workspace (research-internals may change inlining); the netem calibration must be regenerated in a quiet window before citation; R1-16 (ebr `execute.py` RSS includes the Python/GNU time process when GNU time is missing) and R1-14 (netem has no TCP slow start, so range-coalescing decisions need a slow-start sensitivity analysis) remain open; emulated arm64 is deferred (Docker). Findings F-0001 and F-0002. |
| C-design | §6–§35 | RUNNING — `wf_47347bc4-611` (script `research/orchestration/workflows/phase-c-design.js`; resume with `resumeFromRunId`) | 15 domain design agents (chunking, crossfile, codecs, reconstruction, planner, container, remote, scale, platform, crypto, integrity, legacy, conformance, ecosystem, evaluation) write `research/experiments/<EXP-ID>/{protocol.md,spec.yaml}` and index fragments. A Sonnet agent then builds `research/experiments/index.csv` and `decision-coverage.csv`, followed by an adversarial design review and a revision; the revision commit is the experiment pre-registration record. Blocked evidence routes (macOS/ARM64/Docker/humans/external review) are designed and marked BLOCKED. |
| C-exec | §8–§33 | PLANNED | Size/determinism/platform/security experiments in parallel; timing only in quiet windows via runner `--timing` guard. |
| C-analyze | §8–§33, §38, §39 | PLANNED | Normalization, domain reports, Decision Ledger updates, negative results. |
| D. Design freeze + held-out | §35 | PLANNED | Freeze commit recorded in `research/decisions/design-freeze.json`; held-out unlock only after it. |
| E. Synthesis | §34, §36, §40–§42 | PLANNED | product-decisions, stable-v1-scope, final-system-evaluation, remaining-unknowns, reproducibility, threats-to-validity, README. |
| F. Gate audit + completion report | §43, §45 | PLANNED | Adversarial gate audit; report states COMPLETE only if every applicable gate is met. |

## Known blockers and limitations (so far)

- macOS/APFS/HFS+ fork, FinderInfo, birthtime, and macOS ACL behavior: `PLATFORM_BLOCKED` (no local macOS; hosted runners declined).
- Native ARM64: `PLATFORM_BLOCKED` for performance claims; emulated ARM64 is usable only for determinism/correctness checks.
- ReFS, and Windows operations needing admin (e.g. SACLs, VHD creation, some reparse types): blocked in this non-admin session.
- Packet-level loss/latency emulation (`netem`) is unavailable; remote-access experiments use an application-level proxy.
- Human usability participants are unavailable; §33 will use independently simulated first-use evaluations, labeled as such.
- Independent external cryptographic review cannot be performed internally; crypto final selections remain `EXTERNAL_REVIEW_REQUIRED` with a dossier.

## Storage incident 2026-09-16 (C: drive exhausted) — RESOLVED 2026-09-17

- **Cause:** the WSL Ubuntu distro disk (`ext4.vhdx`, default location under `C:\Users\<user>\AppData\Local\Packages\...\LocalState`) grew from about 188 GB to 342.4 GB as `/root/eb-research` accumulated (corpus 50 GB, held-out 42 GB, download cache 54 GB, scratch/staging about 6 GB). Together with Docker Desktop's 30.5 GB data disk, this left C: with 0.38 GB free.
- **Resolution:**
  - Stopped the A3 and B1d workflows.
  - Moved Docker's data disk to `D:\Docker\wsl\disk`.
  - After the owner rebooted to clear a hung WSL service, relocated the distro with `wsl --manage Ubuntu --move D:\WSL\Ubuntu` (43 minutes) and enabled sparse mode (`--set-sparse true`).
  - Relocated the `docker-desktop` distro with `wsl --manage docker-desktop --move D:\Docker\wsl\main` and set Docker Desktop `CustomWslDistroDir = D:\Docker\wsl`; the settings backup is at `D:\Docker\settings-store.json.bak-20260916`.
  - Afterwards C: had about 375 GB free and D: about 485 GB free. `research/orchestration/disk_guard.py` (C: ≥ 25 GB, D: ≥ 75 GB, distro on D:, no Docker data disk on C:) passes.
- **Docker Desktop is currently BLOCKED (pre-existing host issue, not caused by the relocation).** Startup crashes because stale AF_UNIX socket reparse points (e.g. `%LOCALAPPDATA%\Docker\run\sailor-ingest.sock`, `%LOCALAPPDATA%\docker-secrets-engine\engine.sock`) return Win32 error 1920 on every access. The same failure occurred on 2026-09-05 (`*.codex-backup-20260905` entries).
  - Directories renamed non-destructively: `%LOCALAPPDATA%\Docker\run.claude-backup-20260916` and `%LOCALAPPDATA%\docker-secrets-engine.claude-backup-20260916`. Each failed start creates new stale sockets, so start attempts were stopped.
  - Likely fix: a reboot, then start Docker once (without force-killing it). Do NOT use Docker's "Reset to factory defaults".
  - Docker-dependent research steps are deferred until Docker is verified to start with its relocated disks: QEMU arm64 determinism, pinned-container environment captures, and Docker-built corpus items.
- **Resume:** A3 from `method:revise` (resume `wf_16094fa5-38e`), B1d from corpus reassembly r1 and baselines (resume `wf_3615a3d7-fe4`), then B2 (`phase-b2r-harness.js`). Run `disk_guard.py` before each launch.
## Open integration items (carry forward)

- **Baseline probe findings to add to the ledger at the next assembly** (from `research/baselines/README.md`; probed at dev HEAD):
  - `ebound pack` refuses trees with non-UTF-8 filenames (`EB_EAM_INVALID_PATH_COMPONENT`) and refuses a tree whose POSIX ACL mask disagrees with the mode group bits.
  - Legacy tar export refuses symlink entries.
  - Incumbents: Info-ZIP `zip` and `7z` dereference symlinks unless given `-y`/`-snl`; zpaq 7.15 silently drops symlinks.
- ~~**ebr fingerprint bug** (follow-up task `task_bb8ca4e8`)~~ FIXED 2026-09-17 (`4aef874`): `_walk()` no longer corrupts backslash-byte filenames; `f19-tuning-zoo` un-excluded from `KNOWN_BROKEN_ITEMS` in `run_baselines.py`.
- ~~The corpus cache's self-consistency check cannot detect a corrupt cached blob that matches its own SHA-256-named path~~ FIXED 2026-09-17 (`7aab572`): `check_magic_bytes` (offline, runs on every cache read) plus an opt-in `upstream_digest` recipe field (cross-checks against a digest the source host itself publishes; wired onto the 4 real Wikimedia dump inputs that have one). Wikimedia pageviews (the historically-affected item) has no published per-file digest, so it relies on the magic-byte check only.
- **Extent-map page-cache bug found and fixed 2026-09-17** (`f132128`, task_bb8ca4e8 follow-up, superseding the "settling" diagnosis in the 2026-09-16T21:30Z entry below): `fingerprint.py`'s SEEK_DATA/SEEK_HOLE-based `logical_tree_sha256`/`tree_sha256` component depended on ambient page-cache warmth, not just on-disk content, on this ext4/WSL2 host -- not a one-time "settling" as first thought (`f16-alpine-3241-minirootfs-ext4-raw`'s pin history shows the same two values flip-flopping across four separate "fixes" on 2026-09-13 and 2026-09-16). Fixed with `posix_fadvise(..., POSIX_FADV_DONTNEED)` before reading extents, forcing a deterministic cold read every time. All 300 items now provision clean (0 failed) with the fixed tool; a 10% fingerprint spot-check (30 items, held-out fingerprint-only) found 0 identity/content mismatches. Residual risk: the same class of bug could in principle recur for any other sparse/fragmented item not yet exercised by a full re-verify; if `provision --verify` ever reports a mismatch with an unchanged `content_tree_sha256` again, re-fingerprint a few times first (now deterministic) before assuming corruption.
- Round-1 corpus assembly regenerated 2026-09-17 (`250f7a3`): `manifest.json` (300 items, `complete: true`), `statistics.json`, `coverage.md`, `licenses.md`, `heldout-lock.json` (draft, 97 items) all current against the closed critique-round1.md gaps. `research/corpus/methodology.md` updated to round-1 status and documents `upstream_digest` and both cache-corruption defenses.

- **RD-4 gates from the method review (block DECIDED):**
  - (B01) Build a crosswalk mapping the ledger's free-text hard constraints (989 distinct) to HC/freeze IDs.
  - (K01) Extend the decision-ledger schema with optimization dimensions, HC IDs, decision type, cost tier, validation looks, and method commit.
  - (C03, accepted risk) Apply the objective-screen fix in `objective_screen.py`.
  - Run a round-2 method review that re-checks every RD-4 row.
- **(G01) Held-out exposure:** held-out item identities appear in committed corpus sources, `PROGRESS.md`, and git history, and most items are public upstream data. Agents in this program have seen held-out item names (no content), as disclosed in `decision-method.md` §0.1/§5.4/§5.8. Before Phase D, decide on and build a sealed supplementary held-out set (for example items selected after the design freeze by a committed seeded process from sources never named in the repo, plus secret-seeded generated items) per `decision-method.md` §5.

- `research/methods/thresholds.json` is a null template; fill it from the pre-registered `decision-method.md` (Phase A output) before any timing run. The runner refuses timing runs while quiet-machine limits are unset.
- `research/decisions/design-freeze.json` must use one of the key names `design_freeze_sha`, `commit_sha`, or `sha`; the `ebr` held-out unlock reads those.
- `fingerprint.py`/`stats.py` memory grows roughly 0.5–1 KB per filesystem object; generated items are hashed in memory. Streaming generators are needed for multi-GB synthetic items.
- Some corpus build trees are not bit-reproducible (e.g. cJSON's CMake build), and the Docker database data directories use `output_pin: null`. Experiments must reference the committed fingerprint of the materialized bytes, not assume a rebuild reproduces them.
- `tools/zip-compat/observed-outcomes-v1.json` (tracked production tooling) was transiently modified by an agent during extraction and was verified byte-identical to `HEAD` afterwards. Agents must not run regeneration tools against tracked files.

## How to resume

1. `git -C D:\Projects\entrybound\entrybound log --oneline -20` and read this file's phase table; the latest commit message names the last completed step.
2. Re-extract the external inputs if needed: `pwsh research/tools/sources/extract_external_sources.ps1` (the Phase A/B1 workflow scripts reference an older session scratchpad path for the extracted appendix; replace it with `D:/eb-research/sources` when re-running).
3. Check the data root in WSL: `wsl -d Ubuntu -- ls /root/eb-research`. Corpus items can be re-materialized from `research/corpus/sources/*.json` with `research/corpus/tools/provision.py` (hash-verified, idempotent).
4. For a phase marked RUNNING whose workflow is no longer alive, re-run its script from `research/orchestration/workflows/`. Stages whose outputs are already committed can be skipped by editing the script to start at the first missing stage. Extraction/merge agents overwrite their own output files. Continuation scripts (`*-a2-*`, `*-b1c-*`) show the pattern: run only uncommitted work, point agents at committed WIP, and require `checkpoint.py` commits.
4a. After a usage-limit interruption, check `git status` for uncommitted production edits under `crates/`. Save them as a patch under `research/harness/wip/`, restore the crates to HEAD, and commit partial research outputs only after validating them (JSON parses, fingerprints present), labeled WIP.
5. Never read `/root/eb-research/heldout` before the design-freeze commit.

## Change log

- 2026-09-12 — Program started; repository discipline verified (baseline SHA above). WSL toolchain and archivers installed; Docker verified; netem absence and ARM64 emulation confirmed.
- 2026-09-13T01:59Z — Checkpoint 1 (commits `a966630`..`1320b64`): progress log, research `.gitignore`/`.gitattributes` (LF), external-source extraction script, orchestration scripts, Phase A extraction records (35 of 36 slices final; `code-cli` pending), Phase B1 environment fingerprints, `ebr` runner/stats framework with smoke experiment, corpus framework tools.
- 2026-09-13 — Decision: pin research Rust builds to toolchain 1.98.1 on both hosts, invoked by explicit path. Recorded environment corrections (WSL 7-Zip is 23.01, not p7zip; Defender/VBS timing caveats; `/mnt/d` lacks metadata).
- 2026-09-13T02:04Z — Added research/orchestration/checkpoint.py (serialized commit+log+push) and watch_journals.py (per-agent completion events). All later workflows commit stage outputs through checkpoint.py.
- 2026-09-13T02:08Z — Launched Phase B2 (wf_5a19305a-ed0); script at research/orchestration/workflows/phase-b2-harness.js. Journal watcher now covers A, B1, B2.
- 2026-09-13T02:28Z — Corpus g3-data committed: 54 items F05-F08 (tuning/validation/held-out); gaps: no large validation items, F08 validation lacks real-content DBs, F07/F08 held-out have 2 real + 1 generated groups; several upstreams are unstable (pinned bytes cached).
- 2026-09-13T02:30Z — Phase A: code-cli extraction final (36/36 slices); integrity cluster merged.
- 2026-09-13T02:32Z — Phase A: model cluster merged.
- 2026-09-13T02:32Z — Corpus g1-code committed: 40 items F01/F02/F03/F18. Carry-forward: tree_sha256 includes ext4 st_blocks and is unstable under delayed allocation (use logical_tree_sha256); build trees are not all bit-reproducible; vendor trees share common packages across splits; held-out large tier is all linux-kernel group.
- 2026-09-13T02:34Z — Phase A: access cluster merged.
- 2026-09-13T05:43Z — INTERRUPTION (usage limit, 2026-09-13 ~00:0x local): all three workflows stopped. Production crates restored to HEAD; partial research-internals edits saved as research/harness/wip/research-internals-partial.patch (git apply --check OK).
- 2026-09-13T05:43Z — Corpus WIP checkpoint: g4-binary 42 items and g5-media 26 items fingerprinted (group verification pending); g2-generated generators only.
- 2026-09-13T05:44Z — Resume prep: research/tools/ledger/split_by_cluster.py (crypto 985/543, legacy 850/499 shards); external inputs extracted to D:/eb-research/sources (hash-verified).
- 2026-09-13T05:49Z — Launched continuations wf_84e35026-465 (A2) and wf_69959eaa-27f (B1c); B2 relaunch deferred; resume procedure extended with interruption recovery steps.
- 2026-09-16T21:02Z — RESUMED 2026-09-16 after weekly limit (A2 wf_84e35026-465 and B1c wf_69959eaa-27f failed with 0 agents done; no partial outputs). Added make_merge_shards.py (15 topic shards, 100-263 KB compact) and usage pacing policy.
- 2026-09-16T21:05Z — Relaunched A3 (wf_16094fa5-38e) and B1d (wf_3615a3d7-fe4); B2 relaunch deferred pending usage check.
- 2026-09-16T21:20Z — g5-media (F12/F13) verified and finalized: 26 items hash-identical (logical_tree_sha256) and smoke-tested; F12 held-out/validation independence-group gap remains (commons-usgov-photos/commons-cc0-photos selected but not fetched -- Wikimedia rate-limited this session).
- 2026-09-16T21:22Z — compression-chunking merged: 62 requirements, 16 decisions, 427/427 keys mapped
- 2026-09-16T21:23Z — legacy-zip merged: 82 requirements, 30 decisions, 422/422 keys
- 2026-09-16T21:24Z — ecosystem-conformance-deps merged: 75 requirements, 24 decisions, 475/475 keys
- 2026-09-16T21:27Z — platform-metadata merge: 71 requirements, 41 decisions, 351/351 keys covered
- 2026-09-16T21:30Z — Corpus g4-binary (42 items, F09/F10/F11/F14/F16) verified and closed out: found and fixed a logical_tree_sha256 extent-map settling defect (SEEK_DATA/SEEK_HOLE, ext4/WSL2) affecting sparse VM-image outputs, one field deeper than the documented tree_sha256/st_blocks issue; repinned 3 F16 items to their settled, content-identical state; idempotence confirmed (2x full verify, 42/42 ok); coverage meets requirements; ~11.93 GiB total (budget ~12 GiB, thin margin).
- 2026-09-16T21:30Z — crypto-leakage-suite merge: 99 requirements, 27 decisions, 572/572 keys mapped
- 2026-09-16T21:30Z — Phase A: ecosystem cli-adoption shard merged (109 requirements, 53 decisions).
- 2026-09-16T21:31Z — crypto-signatures-trust merged: 75 requirements, 51 decisions, 1 key deferred to model cluster
- 2026-09-16T21:32Z — container cluster merged: 98 requirements, 29 decisions, 756/756 keys
- 2026-09-16T21:33Z — compression-codecs-planner merge: 114 requirements, 29 decisions, 539/539 keys
- 2026-09-16T21:33Z — Phase A: legacy-export-migration shard merged (116 requirements, 57 decisions)
- 2026-09-16T21:35Z — g2-generated F04 provisioned and fingerprinted (6 items); sources.json written for all 5 families (34 items); Docker unavailable this session (com.docker.service needs admin).
- 2026-09-16T21:35Z — Phase A3: platform-paths-extraction merged (91 requirements, 24 decisions)
- 2026-09-16T21:35Z — g2-generated F15 provisioned and fingerprinted (6 items: 2 tuning, 2 validation, 2 heldout).
- 2026-09-16T21:35Z — crypto-keys-recipients merged: 86 requirements, 28 decisions, 528/528 keys mapped
- 2026-09-16T21:37Z — g2-generated F17 provisioned and fingerprinted (6 items: 2 tuning, 2 validation, 2 heldout).
- 2026-09-16T21:38Z — g2-generated F19 provisioned and fingerprinted (6 items: 2 tuning, 2 validation, 2 heldout).
- 2026-09-16T21:40Z — g2-generated F20 provisioned and fingerprinted (10 items: 3 tuning, 3 validation, 4 heldout across 3 independence groups); all 5 g2-generated families now provisioned. gear-norm-v1 reference generators cross-checked against entrybound::chunker (0 mismatches).
- 2026-09-16T21:45Z — legacy-tar-7z-streams merged: 98 requirements, 23 decisions, 429/429 keys, 41 cross-referenced committed decisions
- 2026-09-16T22:12Z — Phase A assembly round 0: 1501 requirements, 581 decisions, 0 uncovered extraction keys, 0 validation errors (29+9 near-duplicate merges, 20 integrity addenda).
- 2026-09-16T22:32Z — Round 1 spec/research critic addenda: 1 new decision, 2 decision and 26 requirement corrections.
- 2026-09-16T22:33Z — Corpus assembly round 0: manifest/statistics/coverage/licenses generated for all 196 items (131 tuning+validation stats, 0 missing/stale); heldout-lock draft (65 items, not frozen); 27/196 fingerprints (13.8%) spot-verified by recomputation; added an st_blocks/logical_tree_sha256 regression test; consolidated ebr's held-out guard onto corpuslib.assert_not_heldout().
- 2026-09-16T22:33Z — round 1 appendix/docs critic addendum: 3 requirements + 1 decision added, 9 corrections
- 2026-09-16T22:34Z — round1 program-code critic: 14 req + 10 dec added, 9 corrections; dry-run assembly 0 errors
- 2026-09-16T22:41Z — Phase A assembly round 1: 1518 requirements, 593 decisions, 0 uncovered extraction keys, 0 validation errors (8 addenda near-duplicate candidates kept distinct; 70 primary keys made visible).
- 2026-09-16T22:54Z — Corpus round-1 critic: INCOMPLETE, 25 gaps (7 blockers: ML weights, DB dumps, supply-chain, home backups, cross-platform metadata, encrypted private archives, F12 held-out groups)
- 2026-09-16T23:01Z — archetypal-objective.md pre-registration draft: 17 HCs (all I1-I31 mapped, MVTs), 26 ODs, generated ledger screen
- 2026-09-16T23:05Z — Prepared research/orchestration/workflows/phase-b2r-harness.js; launch after A3/B1d finish and usage permits (weekly 10% at 2026-09-16 ~23:05Z).
- 2026-09-16T23:07Z — g2-generated F17: added 3 real kernel-headers duplicate-tree items (tuning/validation/heldout), closing the F17 real-duplicate-tree gap; F20 scale label fixed (bombs-compressed -> small).
- 2026-09-16T23:10Z — g4-binary F11 gap-closing: 3 items added (cosign signing assets, cpython release set, firefox signing metadata), F09 Silesia licence fixed; group now 45 items / ~11.994 GiB. F09 arch-skew, F14, F16, F10 large-tier, and remaining F11 sub-items deferred (budget).
- 2026-09-16T23:13Z — Phase A method: decision-method.md pre-registration draft (round 0) committed; review and revision pending
- 2026-09-16T23:22Z — g2-generated F15: added 3 real sparse VM-disk/partition items (tuning/validation/heldout) via qemu-img convert + fallocate --dig-holes, closing the F15 real-layout gap; validation now has a large-tier item.
- 2026-09-16T23:34Z — Method review round 1 committed: NOT READY (10 BLOCKER, 36 HIGH); revision of objective and method pending.
- 2026-09-16T23:35Z — g1-code gap-closing: F01 6 new items (3 full-history git clones incl. CPython validation large tier, 3 JS/TS source-tree archives), F18 3 new items (Ubuntu cloud images, CPython embeddable series, Firefox series), F02 1 new item (Node PDB); F03 lockfile-provenance + per-package license audits added, 5/7 measurable vendor items flipped to redistributable=true with evidence, yq annotated with shared_packages covariate, 4 heldout items pre-registered for post-freeze; group now ~13.9 GiB (over the ~12 GiB target -- flagged, not fixed). provision.py/corpuslib.py gained git.history=full support (new acquire_git_full), selftest still green.
- 2026-09-16T23:35Z — g5-media round-1: F12 held-out independence fixed (commons-usgov-photos, now 2 groups); F13 non-benchmark large held-out item (NASA video) + Elephants Dream public-benchmark retag + Sintel large (same group) + new phone-style x265/libheif pipeline in validation and held-out + pipeline covariate tags on every F13 item; 35 items materialized, about 10.45 GiB, 0 failed.
- 2026-09-16T23:45Z — g3-data gap closure (F05/F06/F07/F08): 33 items added, 30 materialized and verified (pythia, qwen2.5 x2, mnist, smollm2 x2, bert-base-uncased x3, phi-3-mini x2, fineweb-edu, sdss, ann-benchmarks, cmip6, 1000genomes x2, simplewiki, ensembl, stackexchange, rfam, google-clusterdata, gen-journal, backblaze, evtx); 3 F05 items (wikimedia-pageviews, 3x logrotate-derive) interrupted mid-run by a host C: drive full incident (WSL vhdx grew to ~342 GiB while host free space was already critically low from unrelated data; host had ~5.6 GiB free after, WSL became unresponsive).
- 2026-09-17T00:00Z — g2-generated F04: added 3 real many-small-file items (NetBSD pkgsrc tuning, Gentoo snapshot validation, FreeBSD ports heldout), tagged the 2 byte-identical F03-derived items duplicate-of, closing the F04 real-tree gap. Relabeled pkgsrc/Gentoo scale large->medium to match materialized bytes.
- 2026-09-17T00:04Z — g2-generated F20 (structurally-adversarial archives, MAJOR): WIP only -- 4 new generator scripts + a drafted 8-item block saved at wip/f20-structural-items-pending.py, NOT provisioned, because the WSL2 VM became unresponsive mid-session (systemic resource exhaustion, all agents affected). Needs a fresh session once WSL recovers: run provision --check, fix any schema errors, provision, verify scale tiers, then merge into make_sources.py and checkpoint.
- 2026-09-17T00:05Z — g4-binary: F16 AlmaLinux fingerprint re-verified (matched-tofu, no content change) as a side effect of the g2-generated F15 gap closure.
- 2026-09-17T00:48Z — STORAGE INCIDENT: C: at 0.38 GB free. Stopped A3/B1d, moved Docker data disk to D: (C: now 30.8 GB free); WSL service hung, distro move to D:\WSL\Ubuntu awaits owner WSL service restart. Added research/orchestration/disk_guard.py.
- 2026-09-17T02:09Z — Storage incident RESOLVED: Ubuntu distro at D:\WSL\Ubuntu (sparse), Docker disks at D:\Docker\wsl; C: 375 GB free; disk_guard passes (C>=25, D>=75 GB). Docker Desktop startup BLOCKED by pre-existing AF_UNIX socket error 1920 (also seen 2026-09-05); Docker-dependent steps deferred.
- 2026-09-17T02:09Z — WIP: interrupted method revision preserved (archetypal-objective.md, decision-method.md, method-review-round1.md, thresholds.json); pre-registration commit still pending.
- 2026-09-17T02:10Z — Added research/orchestration/disk_alarm.py (C: < 50 GB, D: < 100 GB alarms), armed during workflows.
- 2026-09-17T02:22Z — Method pre-registration committed: objective and decision method round-1 revision complete (85 findings dispositioned), thresholds.json transcribed; ebr tests pass except the template-state test (decision-method.md section 14 item 1).
- 2026-09-17T02:24Z — Phase A DONE: pre-registration commit 14b977c (85 review findings dispositioned: 84 fixed, 1 accepted risk; RD-4 gates pending). Ledgers: 1518 requirements, 593 decisions (f56dd3d). ebr thresholds test updated (83 passed).
- 2026-09-17T02:25Z — Launching B2r (harness) with storage/Docker constraints; weekly usage 13%, disk guard passing.
- 2026-09-17T03:15Z — B2 internals: default-off research-internals feature (crates/entrybound, additive cfg-gated research modules plus planner candidate enumeration), tests/research_internals.rs (9 tests), research/harness/{research-internals.md, internals-identity-check.sh/.ps1/.md, internals_identity_tools.py}; WSL and Windows: 8/8 pack outputs SHA-256 identical, additive audit PASS, tests and clippy identical both ways (pre-existing clippy 1.98 lints and one WSL-only test recorded); wip patch removed.
- 2026-09-17T03:19Z — Finding F-0001: host-dependent required feature bits (Windows always requires 0x10000 platform-security feature; baseline test fails on Linux). research-internals feature verified (c9a2d3a): additive-only, 8/8 outputs identical.
- 2026-09-17T03:32Z — Phase B1 baselines: configs.json + tool-versions.json (14 incumbent families), run_baselines.py/EXP-BASE-SIZE size+determinism pass (140/2240 small-scale samples this session, all rep=0), capability-matrix.csv (283 rows, incl. Entrybound probed via a fresh dev-HEAD build). Flagged (not fixed): ebr/corpus.py backslash-path bug (task_bb8ca4e8).
- 2026-09-17T03:34Z — Corpus WIP: late fix outputs committed (4 F05 items' fingerprints/pins, 11 stats files); corpus assembly round 1 still incomplete (manifest not regenerated).
- 2026-09-17T03:34Z — Launched detached EXP-BASE-SIZE size pass (small tier + medium sample) via research/baselines/run_size_pass.sh; log /root/eb-research/logs/baselines/size-pass.log.
- 2026-09-17T03:37Z — Watcher now quiet (failures/disk/markers only) to cut orchestrator usage; watching B2r and size-pass markers.
- 2026-09-17T04:06Z — research/harness: Cargo workspace skeleton committed -- ebr-common/ebr-pack/ebr-chunk/ebr-codec/ebr-planner/ebr-access/ebr-netem, each with README.md; 44 tests pass on both WSL and Windows (build.sh/build.ps1), clippy/fmt clean; production cargo metadata identical before/after and cargo test --workspace unaffected (one pre-existing WSL-only failure, cross_file_compression::small_cohort_stays_independent_and_v2_v3_remain_readable, already documented in research-internals.md).
- 2026-09-17T04:34Z — ebr-access: INDEXED random access (policy-configurable, 3 source kinds, byte-range sampling, full verification accounting) + STREAM encode/open/verify/list/unpack + streaming memory probe (pack/verify/unpack/repack/export over a synthetic generator with interval memory sampling); 17 tests pass on WSL+Windows, clippy/fmt clean; README updated with ebr spec examples.
- 2026-09-17T04:34Z — ebr-planner: exhaustive chunk/cohort search + regret + greedy/DP/tree strategies + CSV/JSONL output (research/harness/crates/ebr-planner/{src/exhaustive.rs,src/strategies.rs,src/lib.rs,src/main.rs,README.md}); 15/15 tests pass, clippy/fmt clean, on both WSL (CARGO_TARGET_DIR=/root/eb-research/target/b2-planner) and Windows (D:/eb-research/target/b2-planner), toolchain 1.98.1; binary smoke-tested end to end (JSONL rows + candidate-regret CSV verified).
- 2026-09-17T04:41Z — ebr-codec: production.rs/external.rs extended, new transform.rs + matrix.rs, main.rs subcommands (object-matrix/chunk-matrix/transform-false-positives), tests/production_archive_equality.rs. 28 tests pass on WSL and Windows (pinned toolchain 1.98.1, CARGO_TARGET_DIR=b2-codec); clippy -D warnings clean, cargo fmt --check clean; CLI smoke-tested end to end on WSL. New pins: bzip2 =0.6.1, flate2 =1.1.9, lzma-rust2 =0.20.0, brotli =9.0.0, lz4 =1.28.1, ppmd-rust =1.5.0 (matching production's exact pin where shared). README documents module map, candidate rationale, ebr spec command templates, and the plan_ref/ChunkLocation findings.
- 2026-09-17T04:43Z — ebr-chunk implemented: 8 CDC algorithms (fixed-size, fastcdc-2016/2020 cross-checked against pinned fastcdc crate, rabin, buzhash, ae, ram, tttd) + boundaries/dedup/shift-stability/random-access subcommands; 58 tests pass on WSL (+1.98.1, CARGO_TARGET_DIR=/root/eb-research/target/b2-chunk) and Windows (+1.98.1, D:/eb-research/target/b2-chunk-win); clippy/fmt clean; README documents flags and ebr spec command templates. Cargo.lock already carries the fastcdc dev-dependency from a concurrent commit (9d80208), so it needed no change here.
- 2026-09-17T04:43Z — ebr-pack crate complete: 4 binaries (ebr-pack, ebr-unpack, ebr-verify, ebr-inspect-bytes), 8 new/rewritten src files (bytes.rs, counts.rs, encrypt.rs, pack.rs, scratch.rs, unpack.rs, verify.rs, lib.rs/main.rs rewritten), 29 unit tests passing on both WSL (CARGO_TARGET_DIR=/root/eb-research/target/b2-pack) and Windows (CARGO_TARGET_DIR=D:/eb-research/target/b2-pack-win) with pinned cargo +1.98.1, clippy clean on both; manual end-to-end smoke test on WSL confirmed byte accounting balances exactly and cross-validates against production's own codec_usage for indexed/stream/encrypted archives, including the cross-process encrypted verify/unpack/inspect-bytes flow via the .testpassword sidecar. README documents all 4 binaries' flags and ebr runner spec command templates. No workspace Cargo.toml/Cargo.lock changes needed (no new dependencies; Cargo.lock currently clean against HEAD).
- 2026-09-17T05:41Z — ebr-netem crate complete: origin (range/ETag/multipart/churn contract, verified against the real HttpRangeSource), netem proxy (RTT/jitter/bandwidth/loss/cache/max-conn), calibration binary+report (research/harness/crates/ebr-netem/calibration.md, raw JSONL research/raw/EXP-NETEM-CAL/, PROVISIONAL/non-decision-grade, re-run in quiet window). 76 tests pass (56 unit + 12 origin_contract + 8 proxy_netem), clippy -D warnings clean, cargo fmt --check clean, on WSL (CARGO_TARGET_DIR=/root/eb-research/target/b2-netem, toolchain +1.98.1); production crates/ untouched. Fixed a TokenBucket infinite-loop bug (n > capacity) found via testing; README documents chunk-size-vs-bandwidth tradeoff confirmed by spot check.
- 2026-09-17T06:23Z — B2 harness integration validation: workspace build+test clean on WSL/Windows (249 tests, 0 harness fixes needed); internals-identity-check re-run PASS both hosts; EXP-DET-SMOKE cross-host determinism (PCR identical, LAI/AUX differ by design per F-0001, arm64 deferred) in research/raw|normalized/EXP-DET-SMOKE; ebr runner wired via research/experiments/EXP-HARNESS-SMOKE-{PACK,CHUNK,CODEC,PLANNER,ACCESS,NETEM} (all run->normalize->report clean); research/harness/README.md updated with verified commands and known limitations.
- 2026-09-17T07:42Z — Harness review round 1: research/harness/harness-review-round1.md (16 HIGH/MEDIUM + 14 LOW findings; 14 fixed, 2 open with use constraints) plus fixes across all ebr-* crates, realigned harness Cargo.lock, lock_parity.py, harness-cli-parity.sh, review-round1-smoke.sh; verified WSL 277 tests + Windows 274 tests pass, clippy -D warnings and fmt clean, lock parity PASS, ebr-pack vs production ebound archives SHA-256 identical 8/8, end-to-end smoke PASS (non-decision-grade).
- 2026-09-17T07:45Z — Finding F-0002: cross-host LAI differs (executable bit unobservable on NTFS), AUX precision host-dependent, PCR identical (EXP-DET-SMOKE). checkpoint.py: forward-slash absolute paths fixed; --co-author added.
- 2026-09-17T07:47Z — Ledgers round 2: 1521 requirements, 596 decisions (F-0001/F-0002 addenda: DEC-CON-030, DEC-MOD-049, DEC-MOD-050). Phase B2 DONE with review use constraints.
- 2026-09-17T07:48Z — Launched B1e corpus closure wf_a924ee34-6e0 (Sonnet); weekly usage 18%.
- 2026-09-17T07:49Z — Prepared Phase C-design workflow (research/orchestration/workflows/phase-c-design.js); launch after B1e + usage check.
- 2026-09-17T07:57Z — g2-generated F20 (structurally-adversarial archives, MAJOR): wired WIP block into make_sources.py, provisioned+fingerprinted all 9 items (libarchive testsuite, cpython/go archive testdata derives, commons-compress testdata, fastzip/malo, Fifield zipbomb regen, 2 generated malicious-structure sets); libarchive item relabeled medium->small (fits_smaller_tier). Gap closed.
- 2026-09-17T08:18Z — g2-generated F20 (encrypted private archives, BLOCKER): added private_vault.py + private_vault_v2.py, provisioned+fingerprinted 3 items (tuning/validation/heldout) covering ZipCrypto, WinZip/7z AES-256, 7z encrypted headers, GPG symmetric+pubkey, age passphrase+recipient(+ssh-recipient), KeePass KDBX4, and held-out LUKS2. Gap closed.
- 2026-09-17T08:30Z — g2-generated F19 (real cross-platform metadata, BLOCKER): added ntfs_metadata_image.py (real mkntfs+ntfs-3g NTFS images: ADS/DOS attrs/security descriptors/reparse points, one distinct configuration per split) and posix_metadata_image.py (real ext4/XFS/btrfs loop images with real POSIX ACLs, SELinux xattr labels, and security.capability, one fstype per split). 6 items provisioned+fingerprinted. macOS/APFS stays PLATFORM_BLOCKED (not faked). Gap closed.
- 2026-09-17T08:40Z — g4-binary: closed F09/F10/F11/F14/F16 MAJOR gaps from critique-round1.md (G09/G10/G13/G25) -- 19 new items (arm64+riscv64 ELF/Mach-O via skopeo no-Docker OCI export, linux-firmware full tree + Fedora large RPM set, OCI multi-arch/zstd/whiteout layouts + 3 ISO9660 + 1 VHDX, and a 3-split 7z/zip/tar/cpio/wim/cab/Windows-host legacy-writer matrix); provisioned, fingerprinted, stats'd; provision.py --check clean (0 errors, 291 items).
- 2026-09-17T08:43Z — g4-binary: stats.py finished for 8/16 tuning+validation items from the F09/F10/F11/F14/F16 gap closure (all 7 F09 items plus f14-legacy-writer-matrix-tuning); the remaining 8 (F10/F11/F16 large items, F14 validation) are still computing.
- 2026-09-17T08:51Z — g2-generated F19 (real home/project-directory backups, BLOCKER): added home_snapshot.py; 3 items combine a real full git clone (isaacs/node-mkdirp, chalk/ansi-styles, jonschlinkert/is-number) + a real dotfiles repo (mathiasbynens, holman, necolas) + a reused real F13 government PDF into a two-snapshot home-directory backup series with real edits/deletion/addition and hardlinked-unchanged files. Fixed a hardlink-corruption bug found in testing. Gap closed.
- 2026-09-17T08:58Z — ebr fingerprint bug fixed (task_bb8ca4e8): _walk() no longer corrupts backslash-byte filenames; regression test added; f19-tuning-zoo un-excluded from baselines.
- 2026-09-17T09:24Z — Integrity fixes: f05 journal fingerprint re-verified and corrected (extent-map settling, content unchanged); cache verification strengthened with magic-byte sniff + upstream-digest cross-check (wired to 4 real Wikimedia dump inputs); selftest.sh covers both.
- 2026-09-17T10:27Z — stats.py run for all 46 previously-missing tuning+validation items; 2 more stale F19 (ext4/xfs ACL+SELinux) fingerprints found and corrected via the same extent-map-settling diagnosis as f05; methodology.md updated (round-1 status, upstream_digest schema, cache-defense + extent-settling notes).
- 2026-09-17T11:50Z — Root cause found and fixed for the recurring 'stale fingerprint' issue: fingerprint.py's SEEK_DATA/SEEK_HOLE extent map depended on ambient page-cache warmth on this host, not just on-disk content (confirmed via f16-alpine-3241-minirootfs-ext4-raw's pin history flip-flopping across two prior 'fixes'). Fixed with posix_fadvise DONTNEED before reading extents; corrected the 3 records this exposed (2 F19, 1 F16 TOFU pin) plus its F16 dependents; full unfiltered provision.py now reports 300/300 ok, 0 failed.
- 2026-09-17T11:51Z — Round-1 corpus assembly complete: manifest.json/statistics.json/coverage.md/licenses.md/heldout-lock.json (draft) regenerated over all 300 items, complete=true, 0 missing/stale stats. 10% fingerprint spot-check (30 items, held-out fingerprint-only) found 0 identity/content mismatches.
- 2026-09-17T11:52Z — PROGRESS.md brought current: closed items marked with commit refs, stale blocker mentions corrected, round-2 critic flagged as the remaining open item for B1.
- 2026-09-17T12:08Z — Round-2 corpus critic complete: critique-round2.md written with per-gap verdicts, new-leakage finding (F17), and both required verdicts (tuning/validation adequate-with-conditions; held-out not yet adequate). B1e corpus-closure phase's remaining open item is now this critique's own conditions list, not a missing critic.
- 2026-09-17T12:10Z — Phase B1 DONE WITH CONDITIONS: corpus 300 items (ed28b1b4), critic round 2 adequate for tuning/validation; held-out not yet adequate. Launching Phase C-design next (weekly usage 20%).
- 2026-09-17T12:10Z — Launched Phase C-design wf_47347bc4-611 (15 domain designers, index, review, revise).
- 2026-09-17T12:45Z — Pre-registered EXP-EVAL-001..013 (1 READY, 10 NEEDS_TOOLING, 2 BLOCKED); 13 uncovered decisions routed.
- 2026-09-17T12:55Z — Scale-domain experiment designs EXP-SCALE-001..016 (5 READY, 9 NEEDS_TOOLING, 2 BLOCKED; 62/63 decisions covered; DEC-CRY-096 uncovered).
