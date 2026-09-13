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
| A. Source-of-truth audit | §2, §3 | RUNNING — workflow `wf_da687d30-ac3`, script `research/orchestration/workflows/phase-a-audit.js` | Stage 1 extraction: 36 source slices → `research/audit/extract/*.jsonl` (8,429 records validated, 0 malformed lines, as of 2026-09-13T01:59Z; `code-cli` still finalizing). Then per-cluster merge → `research/audit/merged/`, ledger assembly (`research/tools/ledger/assemble.py`, stable IDs in `research/audit/id-map.json`), completeness critics looping until a round finds nothing new, then `archetypal-objective.md` + `decision-method.md` with adversarial review. |
| B1. Infrastructure & corpus | §4, §5, §6, §7 | RUNNING — workflow `wf_cf480e56-fb5`, script `research/orchestration/workflows/phase-b1-infra-corpus.js` | DONE and committed: environment capture (`research/tools/env`, `research/environment`; commit `10cee5a`), the `ebr` runner/stats package with tests and the end-to-end smoke `EXP-SMOKE-000` (`5eabf03`), and the corpus framework (`research/corpus/tools`, `1320b64`). RUNNING: 5 corpus family groups (g1-code, g2-generated, g3-data, g4-binary, g5-media), followed by corpus assembly/critic, baseline tooling, the capability matrix, and the size/determinism pass `EXP-BASE-SIZE`. |
| Gate: method pre-registration | §3.3, §37 | NEXT | Commit `decision-method.md` and `archetypal-objective.md` **before** any decision-relevant result is observed. |
| B2. Research harness | §4, §7, §9, §12 | RUNNING — workflow `wf_5a19305a-ed0`, script `research/orchestration/workflows/phase-b2-harness.js`; each agent commits its own outputs via `checkpoint.py` | Stages: (1) default-off `research-internals` feature in `crates/entrybound`, exposing private codec/transform/reconstruction/JPEG/similarity/ecf helpers and planner candidate enumeration through cfg-gated child modules with no function-body changes, plus a byte-identity proof script; (2) separate cargo workspace `research/harness` with crates `ebr-common`, `ebr-pack` (exact section byte accounting), `ebr-chunk` (gear-norm-v1 plus FastCDC-2016/2020, Rabin, BuzHash, AE, RAM, fixed), `ebr-codec` (production plans plus external codec/transform candidates), `ebr-planner` (traces, exhaustive search, regret, alternative deterministic selectors), `ebr-access` (random/remote/streaming memory); (3) `ebr-netem` application-level proxy and range origin with calibration; (4) cross-host build, determinism smoke (WSL, Windows, QEMU arm64), runner wiring, and adversarial harness review. |
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
- The `ebr` corpus module and `research/corpus/tools/corpuslib.py` both implement held-out guards. Consolidate them so `ebr` calls `corpuslib.assert_not_heldout()` and reads `research/corpus/manifest.json` (Phase B2 task).
- An extra directory, `/root/eb-research/generated`, was created by a framework agent and may overlap the corpus framework; reconcile it during corpus assembly.
- `fingerprint.py`/`stats.py` memory grows roughly 0.5–1 KB per filesystem object; generated items are hashed in memory. Streaming generators are needed for multi-GB synthetic items.
- `tools/zip-compat/observed-outcomes-v1.json` (tracked production tooling) was transiently modified by an agent during extraction and was verified byte-identical to `HEAD` afterwards. Agents must not run regeneration tools against tracked files.

## How to resume

1. `git -C D:\Projects\entrybound\entrybound log --oneline -20` and read this file's phase table; the latest commit message names the last completed step.
2. Re-extract the external inputs if needed: `pwsh research/tools/sources/extract_external_sources.ps1` (the Phase A/B1 workflow scripts reference an older session scratchpad path for the extracted appendix; replace it with `D:/eb-research/sources` when re-running).
3. Check the data root in WSL: `wsl -d Ubuntu -- ls /root/eb-research`. Corpus items can be re-materialized from `research/corpus/sources/*.json` with `research/corpus/tools/provision.py` (hash-verified, idempotent).
4. For a phase marked RUNNING whose workflow is no longer alive, re-run its script from `research/orchestration/workflows/`. Stages whose outputs are already committed can be skipped by editing the script to start at the first missing stage. Extraction/merge agents overwrite their own output files.
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
