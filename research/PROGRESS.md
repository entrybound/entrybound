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
| A. Source-of-truth audit | §2, §3 | RUNNING (third attempt) — `wf_da687d30-ac3` (`phase-a-audit.js`) hit the 5-hour limit; `wf_84e35026-465` (`phase-a2-ledgers-method.js`) hit the weekly limit with 0 agents done; now `wf_16094fa5-38e` (`phase-a3-ledgers-method.js`): 13 compact topic-shard merges, assembly with a mechanical extraction-key coverage check, 3 focused critics, then the method docs | DONE and committed: all 36 extraction slices (8,429 records) and merges for model (`f0cfbbc`), access (`c1869cc`), and integrity (`32e8146`). RUNNING in the continuation: 8 merge shards (container, compression, platform, crypto-a/b, legacy-a/b, ecosystem) reading `research/audit/by-cluster/` produced by `research/tools/ledger/split_by_cluster.py`; ledger assembly with stable IDs; at most 2 completeness-critic rounds; then `archetypal-objective.md`, `decision-method.md`, adversarial review, revision, and `research/methods/thresholds.json`. Every agent commits its own output. |
| B1. Infrastructure & corpus | §4, §5, §6, §7 | RUNNING (third attempt) — `wf_cf480e56-fb5` hit the 5-hour limit; `wf_69959eaa-27f` (`phase-b1c-corpus-baselines.js`) hit the weekly limit with 0 agents done; now `wf_3615a3d7-fe4` (`phase-b1d-corpus-baselines.js`, the same stages with Sonnet for mechanical agents) | DONE and committed: environment capture (`10cee5a`), `ebr` runner (`5eabf03`), corpus framework (`1320b64`), corpus g1-code 40 items (`10743d7`), g3-data 54 items (`d961869`). WIP committed (`6ef6659`): g4-binary 42 items and g5-media 26 items fingerprinted but unverified; g2-generated generators only. RUNNING: g2 resume, g4/g5 verification, assembly with the `logical_tree_sha256` fix and runner/corpus guard consolidation, one critic round with fixes, and baselines (configs, capability matrix, `EXP-BASE-SIZE` size/determinism pass). |
| Gate: method pre-registration | §3.3, §37 | RUNNING inside the A continuation | The `method:revise` agent's commit is the pre-registration record. No decision-relevant experiment result may exist before it. |
| B2. Research harness | §4, §7, §9, §12 | INTERRUPTED; relaunch deferred until the A2 or B1c workflow finishes (limits concurrent token burn) — original `wf_5a19305a-ed0` (`phase-b2-harness.js`), in which no agent completed | Partial additive production edits are preserved in `research/harness/wip/research-internals-partial.patch` (`745591b`; applies cleanly to HEAD); the production crates were restored to HEAD. The relaunch must start the internals agent from this patch and re-prove byte identity. | Stages: (1) default-off `research-internals` feature in `crates/entrybound`, exposing private codec/transform/reconstruction/JPEG/similarity/ecf helpers and planner candidate enumeration through cfg-gated child modules with no function-body changes, plus a byte-identity proof script; (2) separate cargo workspace `research/harness` with crates `ebr-common`, `ebr-pack` (exact section byte accounting), `ebr-chunk` (gear-norm-v1 plus FastCDC-2016/2020, Rabin, BuzHash, AE, RAM, fixed), `ebr-codec` (production plans plus external codec/transform candidates), `ebr-planner` (traces, exhaustive search, regret, alternative deterministic selectors), `ebr-access` (random/remote/streaming memory); (3) `ebr-netem` application-level proxy and range origin with calibration; (4) cross-host build, determinism smoke (WSL, Windows, QEMU arm64), runner wiring, and adversarial harness review. |
| C-design | §8–§33 | PLANNED | Pre-registered experiment specs per domain, committed before execution. |
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

## Open integration items (carry forward)

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
