# Reconstruction domain (program section 8.8): provisions common to EXP-RECON-001 … EXP-RECON-012

This file is part of the pre-registration record of every `EXP-RECON-*` experiment. Each `protocol.md` cites the sections below by number (`common §N`) instead of repeating them. If a protocol and this file disagree, the protocol governs for that experiment, and `research/decision-method.md` governs both.

## 1. Decisions in scope

The ledger query was `research/decision-ledger.jsonl`, filtered to rows whose `program_sections` contain `8.8`. It returned 15 rows. Every row is covered by at least one experiment, so `reconstruction-uncovered.jsonl` is empty. Parts of some decisions are routed elsewhere (§12).

| Decision | Short question | Release | Experiments |
|---|---|---|---|
| DEC-CMP-012 | Gating of transform, reconstruction and codec attempts. This domain covers only the reconstruction slice; structural-transform and codec gating belong to 8.7. | V1_IMPORTANT | 001, 002, 003, 004, 005, 006 |
| DEC-CMP-016 | Decoder ceilings and reconstruction limits; whether the planner mirrors validation bounds | V1_IMPORTANT | 001, 002, 003, 004, 006, 007, 010, 012 |
| DEC-CMP-018 | DEFLATE reconstruction in v1: profiles, input subsets, max-chain | V1_IMPORTANT | 001, 002, 003, 004, 005, 006, 011, 012 |
| DEC-CMP-023 | Which experimental-frozen generations (v4/v5/v6) enter stable v1 | V1_BLOCKING | 001, 002, 004, 005, 008, 011 |
| DEC-CMP-026 | JPEG reconstruction in v1: backend, subset, multi-chunk regions in balanced | V1_IMPORTANT | 001, 002, 003, 004, 005, 006, 008, 009, 011, 012 |
| DEC-CMP-037 | In-process execution of reconstruction dependencies: containment | V1_BLOCKING | 005, 006, 007, 011 |
| DEC-CMP-038 | Crate-version-bound reconstruction formats versus independent specification | V1_BLOCKING | 003, 004, 007, 008, 011 |
| DEC-CMP-039 | Region representation versus dedup, dictionary and ChunkGroup conflicts | V1_IMPORTANT | 002, 009 |
| DEC-CMP-042 | Chunk or region alignment to recognised embedded streams | V1_IMPORTANT | 002, 004, 005 |
| DEC-CON-004 | What DecodeRequirements declare (this domain: reconstruction buffers) | V1_BLOCKING | 001, 006, 010 |
| DEC-CON-005 | Default decoder policy ceilings | V1_BLOCKING | 001, 004, 006, 012 |
| DEC-CON-007 | Decodability that depends on dependency behaviour | V1_BLOCKING | 003, 004, 007, 008, 011, 012 |
| DEC-CON-010 | Feature-bit activation (this domain: bits 0x4 and 0x8, audit and fallback records) | V1_BLOCKING | 001, 004, 009, 010, 011 |
| DEC-INT-030 | Test vectors or differential evidence for reconstructive transforms | V1_BLOCKING | 008, 010, 011, 012 |
| DEC-MOD-040 | `plan_ref` sentinel for region-member chunks | V1_IMPORTANT | 010, 011 |

Some experiments also list rows outside 8.8 that they measure directly: DEC-CMP-034 (JPEG post-codec candidate lists, 003), DEC-CMP-044 (write-time verification cost, 005) and DEC-ECO-016 (crate partition for a minimal reader, 011). They list them as `informs` only.

## 2. Gates, status semantics and decision attributes

- **Status field.** `READY` means that the experiment can execute with binaries that exist at design time, plus scripts committed in its directory, once those binaries are rebuilt at the pre-registration commit. `NEEDS_TOOLING` names the tools to build. `BLOCKED` names the missing platform or access.
- **Program gates (decision-method §0.4, §14) apply on top of the status.**
  - **G-A** must hold before these records count as decision-relevant pre-registrations: the constraint crosswalk, ledger schema v2 with `hc_ids`, `od_ids` and `decision_type`, and the look registry `research/decisions/validation-looks.jsonl`.
  - **G-B** must hold before any result is decision-grade: the frozen held-out lock, decision tooling (§14 item 2), `workload-mix.json`, runner additions for timing, and calibration for timing and memory.
  - Until both hold, READY runs are tuning-exploratory, and they are labelled `NOT_DECISION_GRADE` wherever a gate they depend on is unmet.
- **Decision type, `od_ids`, `hc_ids`.** These are assigned by an independent session (§1.2, §14 item 12). Protocols give *proposals for the assigner* only.
  - The designer expects DEC-CMP-018, 026, 016, 042 and CON-005 to be EMPIRICAL.
  - DEC-CMP-038, CON-007, INT-030 and MOD-040 are expected to be mixed FORMAL + EMPIRICAL. Their specification-feasibility and standard-availability parts may need EXTERNAL review (§8.3; ISO/IEC 18181 access, licence interpretation).
  - DEC-CMP-023 is expected to be EMPIRICAL + FORMAL (cost).
- **Profile-scope decisions.** "Which profiles" in DEC-CMP-018 and 026 and the per-profile ceilings in DEC-CMP-016 are profile-scope decisions. They use the constrained form of Appendix B.1: `artifact_bytes` is the sole superiority metric, and `encode_wall_s`, `decode_wall_s`, `alloc_peak_bytes` and `access_granularity_bytes` are non-inferiority guards. They cannot become `DECIDED` until DEC-CMP-035 fixes numeric profile bounds (R2 "Profile bounds").
- **Gated library-version-defined decode steps (R2, objective HC-11).** `deflate-reconstruct/v1` (preflate-rs 0.7.6 corrections) and `jpeg-jxl-reconstruct/v1` (jixel 0.2.19 / jxl-oxide 0.12.6) are admitted only if every HC-11 condition holds. EXP-RECON-011 §C checks them mechanically.
  - Status quo emits both from `balanced`, the default creation profile. R2 says: "A candidate that places such a step on a baseline or default decode path is eliminated".
  - Every decision whose candidates include emitting these steps from the default profile therefore carries the **R0 item 7 corrective candidate** `recon-opt-in-not-default` (§6). The R2 screen of the status quo is evaluated before any comparison.
  - Every candidate built on a library-version-defined step is tier **T4** (§7.2); R6 minimum gains use the T4 multiplier.

## 3. Author, independence and exposure disclosure

- **Author.** The Phase C-design reconstruction agent (session of 2026-09-17). It designed the candidate lists, hypotheses and analyses in `EXP-RECON-*`. It implemented no production or harness code. It wrote only `EXP-RECON-001/recon_sample.py` and `prepare.sh` (measurement wrappers).
- **L7 exposure (§5.7).** As instructed, this session read `research/corpus/coverage.md` and `research/PROGRESS.md`, which list held-out item identifiers and upstream sources. It opened no held-out content, listing, statistic or path under `/root/eb-research/heldout` or `/root/eb-research/fingerprints/heldout`. The identifiers are not repeated in any `EXP-RECON-*` file.
  - Under §5.4 consequence 3, the design review must decide whether an unexposed session re-signs the candidate sets and analysis plans for decisions whose families include exposed items (F12, F13, F16 and others).
  - **R0 item 6** (a critic-proposed candidate) is **open** for every decision. This designer is not an independent critic, and the candidates it added (§6, marked `designer-added`) do not satisfy item 6.
- **Feasibility smoke (NOT_DECISION_GRADE).** `EXP-RECON-001/recon_sample.py` ran on a generated 5-file tree for 3 cells in under 10 s (gzip/zlib/raw DEFLATE of generated words; baseline and progressive JPEG from a generated gradient; stale pre-review harness binaries). Every cell exited 0 and produced one JSON line with a zero-mismatch round trip.
  - These observations shaped the hypotheses. They are not evidence:
    - the encrypted path recorded planner id `dense-enc-v1`;
    - `feature.incompat` was `0x800F` with and without selections;
    - the declared working set was 256 MiB when a JPEG region was selected.
  - No corpus item was touched.

## 4. Build identity and measurement preconditions (all experiments)

1. **Pre-registration commit.** Every run descends from the commit that adds the experiment's `protocol.md` and `spec.yaml`, with `dirty = false` for `crates/`, `research/harness/`, `Cargo.lock`, `Cargo.toml` and the experiment directory (§5.8(d)).
2. **Rebuild.** Every binary is rebuilt from a fresh clone at that commit with the pinned toolchain `+1.98.1` (`research/PROGRESS.md` environment). The binaries at `/root/eb-research/target/harness/release` on 2026-09-17 predate harness review round 1 and must not be used. Binary SHA-256s are recorded in `/root/eb-research/results/<EXP>/build.json` and in every sample row.
3. **Parity.** `python3 research/harness/lock_parity.py` and `research/harness/harness-cli-parity.sh` must both print `RESULT PASS` before any campaign (harness review use constraint 1).
4. **Research variants.** Planner generations v3–v5, filtered or overridden assignments, and fault injection exist only in research builds (default-off features, F-24). Such a build is an oracle only while objective MVT-16(c) passes for it: with research features off it is byte- and outcome-identical to production on the conformance corpus plus every tuning item. Each variant tool also runs an **identity control** in every item's sample: its unfiltered v6 materialisation must be byte-identical to `ebound pack` for the same item and profile. Any difference invalidates that item's variant samples.
5. **Timing.** Decision-grade timing of the status quo uses `ebound` from the production workspace (use constraint 2). Research-build timing (v3–v5 and variants) is admissible only after a parity arm shows `EQUIVALENT` encode and decode timing between `ebr-recon variants --generation v6` and `ebound pack` on the same items (EXP-RECON-005 §5).
6. **Memory.** Memory claims use scoped or allocator measurements only (use constraint 3). Decision-grade decode memory comes from the counting allocator of §14 item 17 (EXP-RECON-006).
7. **Environment guards.** Before any workflow, `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass. WSL scratch lives on ext4 under `/root/eb-research`, never under `/mnt/*` (§4.1).

## 5. Split discipline (§5)

- **Tuning** is exploratory and unrestricted: search, sweeps, variant refinement, and forming W and S per decision and scope (R3).
- **Validation looks.**
  - Each look is registered in `research/decisions/validation-looks.jsonl` before it is opened. The record lists decision ids, experiment id, candidates, primary metrics, item ids, commit and UTC time.
  - W, S, the declared superiority metrics per pair and the MDE computation (§4.12) are appended to the experiment's `protocol.md` "Looks" section in a commit **before** the look.
  - At most two looks per decision. A second look needs new validation items from new independence groups (§5.2).
  - Validation runs of *descriptive* experiments (EXP-RECON-002 census, EXP-RECON-001 status quo on validation) are part of the first look of the decisions they inform and are registered with it. They are not run earlier.
- **Held-out (Phase D placeholder).** Every surviving candidate at the single program-wide freeze runs once on held-out under §5.5 (Commit A/B/C, `EB_HELDOUT_UNLOCK` equal to `design_freeze_sha`).
  - Specs are copies of the validation specs with `splits: [heldout]`, committed in Commit A with the held-out MDE of every supporting comparison, computed from validation group variance and held-out group counts only.
  - A decision with a held-out MDE above 2 band units is routed to `INSUFFICIENT_EVIDENCE` before unlock (R5).
  - Held-out evidence from unsealed items carries the `identity_aware_heldout` cap.
  - **Sealed-tranche recommendation.** For F12, F11 and F14, a sealed supplementary tranche (§5.4) with producer-diverse JPEG and DEFLATE content, generated or acquired after the freeze, is the only route that lifts the cap for DEC-CMP-018 and 026.
- **Public tuning data sources (§5.4 consequence 2).** These are every corpus item's `provenance` plus the corpus supplements of §7. A held-out-aware session checks them against held-out lineages before the first validation look and records only pass or fail.

## 6. Candidate registry (stable ids used across EXP-RECON-*)

**Expressibility**:
- `REAL`: real archives from production or research writers.
- `MODELLED`: artifact bytes from the exact cost model of §8.3, allowed only where no wire support exists.
- `SPEC`: a policy or specification change evaluated by counterexample search, cost census or refusal counts.

The tiers shown are designer estimates `[INFERRED]`. The computed tier (§7.2, EXP-RECON-011) governs.

### 6.1 DEFLATE reconstruction (DEC-CMP-018; with DEC-CMP-012, 042, 023)

| id | Ledger candidate | Definition | Expr. | Est. tier |
|---|---|---|---|---|
| `deflate-v5-status-quo` | status-quo-v5 | v5/v6 whole-Chunk gzip (single member), zlib and raw via preflate-rs 0.7.6, max-chain 512/2048/4096 for balanced/dense/extreme; gain rule > 256 B and > 2 % | REAL | T4 (existing; no sunk-cost credit, §7.4) |
| `deflate-drop` | drop-deflate-reconstruction | v6 assignment with every DEFLATE-reconstructive selection replaced by its v4 winner (`--filter minus-deflate`) | REAL | T1 (planner ID) |
| `deflate-dense-extreme-only` | dense-extreme-only | `deflate-drop` in balanced, status quo in dense/extreme (composition of per-profile results) | REAL | T1 |
| `deflate-signature-gated` | signature-gated-only | status quo with raw-wrapper attempts removed (`--filter signature-gated`) | REAL | T1 |
| `deflate-entropy-gated-{7000,7500,7800}` | (DEC-CMP-012 entropy-estimate-gate, reconstruction slice) | raw attempts only on chunks whose integer order-0 entropy estimate ≥ threshold milli-bits/byte; thresholds fixed now | REAL | T1 |
| `deflate-embedded-streams` | add-embedded-png-zip-pdf | reconstruct DEFLATE streams inside ZIP entries, PNG IDAT, PDF FlateDecode and git packfile objects as whole-object regions | MODELLED | T4 |
| `deflate-multi-member-gzip` | (ledger question: multi-member gzip) *designer-added* | concatenated gzip members (e.g. `.apk`) reconstructed per member with a member skeleton | MODELLED | T4 |
| `deflate-whole-object-regions` | whole-object-deflate-regions (DEC-CMP-042) | v6-style regions spanning chunks for gzip/zlib objects ≤ 64 MiB intermediate | MODELLED | T4 |
| `deflate-preflate-rs-latest` | *designer-added* | newest preflate-rs release published on or before 2026-09-17 (resolved and pinned by the tooling session before any run), same bounds, max-chain ∈ {512, 2048, 4096} | MODELLED | T4 |
| `deflate-preflate-rs-maxchain-sweep` | (ledger question: max-chain settings) | preflate-rs 0.7.6 with max-chain ∈ {256, 512, 1024, 2048, 4096, 8192, 32768} | MODELLED (stream level) | T2 (new parameter values) / T4 |
| `deflate-preflate-cpp` | precomp-preflate-cpp | preflate C++ library at the pinned precomp-cpp commit (C++, native code) | MODELLED | T4 (+ native code) |
| `deflate-zlib-replay` | *designer-added* | store only (library, level, memLevel, windowBits, strategy) when re-deflating the plaintext with a pinned zlib reproduces the stream exactly; else not eligible | MODELLED | T4 (library-version-defined) |
| `recon-opt-in-not-default` | R0 item 7 corrective | reconstruction never emitted by the default profile; emitted only on explicit opt-in (flag or non-default profile) | REAL (composition) + SPEC | T2 (user-visible flag) |
| — excluded | substitutive-recompress | not bit-exact (I14; SPEC §5.5), ledger exclusion; recorded, not measured | — | — |

### 6.2 JPEG reconstruction (DEC-CMP-026; with DEC-CMP-038, 039, 042)

| id | Ledger candidate | Definition | Expr. | Est. tier |
|---|---|---|---|---|
| `jpeg-v6-status-quo` | status-quo-v6 | SOF0 only, whole ContentObject ending exactly at EOI, ≤ 64 MiB, ≤ 100 MP, jixel 0.2.19 + jxl-oxide 0.12.6, single thread; balanced one-Chunk regions, dense/extreme ≤ 4096 Chunks | REAL | T4 (existing) |
| `jpeg-drop` | drop-jpeg-reconstruction | v6 with regions removed (`--filter minus-jpeg`); expected bytes = v5 plus audit and bit differences | REAL | T1 |
| `jpeg-balanced-multichunk` | balanced-multi-chunk | balanced post-codec list with dense region eligibility (≤ 4096 chunks) | REAL (research planner override) | T1 |
| `jpeg-add-progressive` | add-progressive-and-trailing (progressive part) | SOF2 admitted where the backend round-trips exactly | MODELLED | T4 |
| `jpeg-add-trailing` | add-progressive-and-trailing (trailing part) | bytes after EOI stored as suffix side data, JPEG part reconstructed | MODELLED | T4 |
| `jpeg-jixel-latest` | *designer-added* | newest jixel and jxl-oxide releases on or before 2026-09-17, pinned before any run | MODELLED | T4 |
| `jpeg-libjxl` | libjxl-backend | libjxl pinned release: `cjxl --lossless_jpeg=1`, reconstruction by `djxl` to `.jpg` | MODELLED | T4 + native C++ |
| `jpeg-lepton` | lepton-class | Microsoft `lepton_jpeg` Rust crate at the newest release on or before 2026-09-17. **Fallback rule, fixed now:** if it fails to build or run, dropbox/lepton C++ at its final commit represents the class. | MODELLED | T4 |
| `jpeg-brunsli` | brunsli | google/brunsli pinned commit (`cbrunsli`/`dbrunsli`) | MODELLED | T4 + native C++ |
| `jpeg-packjpg` | *designer-added* (reference) | packJPG pinned commit. Its licence is read from the LICENSE file at the pin; a strong-copyleft licence makes it a measured upper-bound reference only (R2, §7.5), never selectable. | MODELLED | reference |
| `jpeg-nested` | (ledger question: nested JPEGs) | JPEG streams embedded in PDF DCTDecode, ZIP/OOXML members and MPF secondary images, via container skeleton + region | MODELLED | T4 |
| `jpeg-opt-in-not-default` | R0 item 7 corrective | as `recon-opt-in-not-default`, JPEG only | REAL (composition) + SPEC | T2 |
| — excluded | pixel-reencode | lossy or non-bit-exact (I14; docs/jpeg-reconstruction-v1.md recognition rule); ledger exclusion | — | — |

### 6.3 Generations, conflicts, alignment and bits (DEC-CMP-023, 039, 042; DEC-CON-010)

| id | Ledger candidate(s) | Definition | Expr. |
|---|---|---|---|
| `gen-v3`, `gen-v4`, `gen-v5`, `gen-v6` | (ablation arms for "share of savings per generation"); status-quo-experimental-default = `gen-v6` | `PlannerVersion::{V3,V4,V5,V6}::plan` per profile | REAL |
| `gen-promote-v4-only`, `gen-default-stable-subset`, `gen-withdraw-reconstruction`, `gen-promote-all` | DEC-CMP-023 candidates | Compositions of the arms above plus SPEC or cost records: `promote-v4-only` = `gen-v4` emission; `withdraw-reconstruction` = `gen-v4` emission with decode support for v5/v6 retained (§7.4); `default-to-stable-subset` = `gen-v4` for default profiles plus opt-in v5/v6 | REAL + SPEC |
| `conflict-status-quo` | status-quo-cohorts-first-conservative | production v6 | REAL |
| `conflict-region-first` | region-first | regions selected before cohorts; members excluded from cohorts | REAL (research planner override) |
| `conflict-joint-cost` | joint-complete-cost | the lower total of region savings and displaced cohort/dedup savings | REAL (research planner override) |
| `conflict-split-audit-reasons` | split-audit-reasons | status-quo bytes plus separate audit reasons for dedup sharing and dictionary/group conflicts (new audit version) | REAL (writer override) |
| — excluded | duplicate-representation-for-shared-chunk | ledger exclusion (I1; unique Chunk frames). An L0 exclusion needs counterexample and reviewer sign-off (objective §2.0 rule 3), pending. | — |
| `align-status-quo` | status-quo-per-chunk-and-jpeg-regions | production v6 | REAL |
| `align-member-cdc` | member-aligned-cdc | forced cuts at recognised stream starts and ends, gear-norm-v1 within (new chunker ID) | REAL (research chunker) |
| `align-single-chunk-recognised` | single-chunk-for-recognised-compressed-files | recognised gzip/zlib/JPEG files ≤ 16 MiB stored as one Chunk | REAL (research chunker) |
| `align-whole-object-deflate` | whole-object-deflate-regions | = `deflate-whole-object-regions` | MODELLED |
| `align-embedded-scan` | embedded-stream-scanning | = `deflate-embedded-streams` ∪ `jpeg-nested` | MODELLED |
| `bits-status-quo` | status-quo-always-set-cumulative | production v6 | REAL |
| `bits-minimal-activation` | minimal-activation | no audit or fallback records; bits 0x4/0x8 only when a reconstructive byte is emitted (`--filter no-audits`) | REAL (writer override) |
| `bits-independent`, `bits-aux-ro-compat`, `bits-derived-mask`, `bits-documented-implication` | other DEC-CON-010 candidates | policy and specification; this domain supplies reader-refusal counts only (EXP-RECON-004 §10) | SPEC |

### 6.4 Limits, ceilings, containment, dependency-definition, vectors, sentinel

These candidates are listed with their ledger ids in the protocols that evaluate them: DEC-CMP-016 and DEC-CON-004/005 in EXP-RECON-006; DEC-CMP-037 in EXP-RECON-007; DEC-CMP-038, DEC-CON-007 and DEC-INT-030 in EXP-RECON-008 and 011; DEC-MOD-040 in EXP-RECON-010. Ledger exclusions (`trust-declared-without-ceiling`, `planner-defined-decoding`, `trust-encoder-no-verify`) are recorded, not measured.

## 7. Corpus

- **Manifest.** `research/corpus/manifest.json` (`ebrc-2026.09-v1`; manifest sha256 `ed28b1b47333c186a8c80781d378b4c149eaaceea09a65098182f4c062380392` at design). Splits `tuning` (112 items, 36.1 GiB, 89 groups) and `validation` (91 items, 26.6 GiB, 76 groups). These counts were computed from manifest fields only (`split`, `bytes`, `independence_group`); the command is in EXP-RECON-001 §6.
- **Applicability.** Reconstruction is content-triggered and never excluded by design for any family, so every family is applicable to every candidate. Non-target families are measured to charge detection cost (§4.9, §7.3).
- **Target families** are defined mechanically from the **tuning** census (EXP-RECON-002), by a rule fixed now. A family f is a target family for a candidate class (DEFLATE whole-chunk, DEFLATE embedded, JPEG whole-object, JPEG embedded) if the class's eligible stream bytes are ≥ 1 % of f's logical bytes in ≥ 2 tuning independence groups of f, or in its only group when f has one. Non-target families are all others.
- **Minimum support risks, known now from group counts alone.**
  - F12 has 2 groups in tuning (`g5-kodak-photocd-suite`, `g5-nasa-ivl-photos`) and 2 in validation (`g5-commons-cc0-photos`, `g5-loc-fsa-owi-color`).
  - The F05 rotated-`.gz` items are one group per split. F04 `objstore` (git loose objects) is one validation group.
  - A JPEG-scoped or DEFLATE-scoped winner (R9 partition 4) therefore rests on 1–2 groups per split. It is pre-registered as **unlikely to pass the §4.12 MDE gate** without supplements. If the MDE gate fails, the look is not spent and the decision stays `INSUFFICIENT_EVIDENCE` with the gap recorded.
- **Corpus supplements requested before execution.** They are built by a corpus-construction session, which is held-out-aware and designs no candidate (§5.4 consequence 3). Each supplement is committed and fingerprinted before first use (L4), with held-out counterparts from independent sources (preferably sealed).
  - **S-JPEG-REAL.** ≥ 3 new independence groups per split of real JPEG populations not already in the corpus: phone cameras (files carrying MPF or post-EOI trailers such as motion-photo payloads), web-published images (progressive and optimised), and document scans. Each group has ≥ 40 files and records producer evidence (EXIF `Make`/`Software`, quantisation-table fingerprint).
  - **S-JPEG-PRODUCERS.** Derived producer matrices built from each split's own source images:
    - libjpeg-turbo 2.1.5: baseline, optimised Huffman, restart intervals, `-progressive`, `-arithmetic`;
    - mozjpeg and jpegli (`cjpegli`), each at a pinned release, baseline and progressive;
    - Pillow 12.3.0 (`optimize`, `progressive`);
    - guetzli at a pinned release (small images only);
    - chroma 4:4:4, 4:2:2 and 4:2:0, grayscale, and CMYK/Adobe APP14;
    - EXIF, ICC and XMP-heavy marker sets;
    - an MPF primary + secondary structure; zero padding after EOI; an appended MP4-like trailer.

    jpegli output is mandatory (objective MVT-02(d) known-hard inputs).
  - **S-DEFLATE-PRODUCERS.** Derived gzip, zlib and raw streams built from each split's own plaintexts (text from F05/F06, binaries from F09), sizes 1 KiB–16 MiB, ≥ 20 plaintexts per producer setting:
    - zlib 1.3 via CPython: levels 1–9, memLevel {8, 9}, windowBits {9, 12, 15}, strategies default/filtered/huffman-only/rle;
    - GNU gzip 1.12 at -1…-9, and pigz at `-p1` and `-p8`;
    - libdeflate at a pinned release, levels 1–12; zopfli at a pinned release;
    - 7-Zip 23.01 `-tgzip -mx=5/9`;
    - Java 21 `Deflater` levels 1–9;
    - miniz_oxide (flate2 1.1.9) levels 0–10; zlib-ng at a pinned release;
    - .NET `DeflateStream` at `Fastest`, `Optimal` and `SmallestSize` (Windows host);
    - Go `compress/flate` at a pinned toolchain.

    Embedded forms: Info-ZIP `zip` at -1…-9, Python `zipfile`, `jar`, and PNG via Pillow `compress_level` 0–9, with optipng and zopflipng at pinned releases.
  - **S-PHOTO-LIBRARY** (EXP-RECON-009). Photo-library series derived from each split's F12 sources, with seeds.
  - **S-RESOURCE** (EXP-RECON-006). Generated scaling and hostile series. These are experiment-local generated case sets, not corpus families, fingerprinted in the experiment's case manifest.
- **Independence.** Derived supplements inherit the independence group of their source lineage (corpus methodology §3). They add producer strata, not groups. Only S-JPEG-REAL adds groups.

## 8. Metrics and measurement conventions

### 8.1 Canonical metrics (decision-method Appendix A.2; thresholds by T-id, never restated)

| OD (proposal) | Primary metric | Band | Other canonical metrics used |
|---|---|---|---|
| OD-05 | `artifact_bytes` | T-01 | `total_stored_bytes` (T-01 sensitivity arm), `side_data_bytes`, `metadata_bytes`, `dictionary_bytes`, `index_bytes` (T-02 diagnostic), `dedup_stored_fraction` (T-01 diagnostic), `fixed_overhead_bytes` (T-21) |
| OD-06 | `encode_cpu_s` (wall reported) | T-03/T-05 per profile | `encode_wall_s`, `encode_throughput_bps` (T-05b), `verification_cpu_share` (descriptive) |
| OD-07 | `decode_wall_s` | T-04 | `verify_wall_s`, `decode_cpu_s`, `decode_throughput_bps` |
| OD-08 | `alloc_peak_bytes` (decode and encode variants) | T-06 | `peak_rss_bytes` (secondary corroboration) |
| OD-10 | `access_granularity_bytes` | T-09 | `access_amplification` (T-22 per stratum), `random_entry_latency_s` (T-08, in-process) |
| OD-17 | `declared_tightness_ratio` | T-23 | `benign_false_refusal_fraction` (T-20), `refusal_cpu_s`, `refusal_alloc_peak_bytes`, `hostile_cpu_scaling_exponent` (descriptive) |
| OD-02 / HC-02 | binary | §3.3 | `roundtrip_mismatches` |
| OD-04 / HC-03, HC-11 | binary | §3.3 | `reader_disagreements`, `cross_host_identity_agreement` |
| HC-06 | binary | §3.3 | `bounded_memory_growth_bytes` (n/a here), `adversarial_true_refusal_fraction` |
| OD-24 / HC-03, HC-06 | binary | §3.3 | `unresolved_fuzz_findings_at_coverage_target` |
| OD-21–OD-24 | cost inputs | §7 | `decode_path_dependency_count`, `decode_path_native_unsafe_count`, `rustsec_advisory_count`, `maintainer_count`, `msrv_gap`, `normative_words_added`, `wire_items_added`, `untrusted_input_loc`, `attacker_controlled_parameters`, `independent_reader_loc`, `reader_fragmentation_count`; `license_compatibility`, `baseline_codec_public_spec`, `unspecified_baseline_decode_steps` (binary) |

- **At most one primary metric per OD** per decision (R3). Which ODs apply is the assigner's `od_ids`.
- **Profile-scope constrained form** (common §2): `artifact_bytes` is the only superiority metric.

### 8.2 Non-canonical census and diagnostic quantities

Census quantities are prevalence fractions, stream counts, producer classes, attempt outcome classes, audit counts by reason, and modelled-vs-real model error. They are **descriptive only** (§0.2 item 4). They are reported, they define target families by the rule of §7, and they can trigger review. They never eliminate or select.

### 8.3 Exact cost model (for `MODELLED` candidates)

- **Definition.**

  ```text
  artifact_bytes_model(c, item, profile) = artifact_bytes_real(base, item, profile)
                                           − Σ_{o ∈ O_c} cost_base(o)
                                           + Σ_{o ∈ O_c} cost_c(o)
                                           + Δrecords(c)
  ```

  - `base` is a REAL archive of the same item and profile.
  - `O_c` is the set of objects or streams whose representation c changes, from the EXP-RECON-002 stream index.
  - `cost_base(o)` is o's exact stored bytes in `base`, from per-Chunk frame accounting (`ebr-inspect-bytes --per-chunk`, tooling).
  - `cost_c(o)` is c's representation bytes after the profile's post-codec list, plus side-data bytes, from the EXP-RECON-003 stream results.
  - `Δrecords(c)` is the exact record, section-header and plan-record length change, computed with the `entrybound::research::ecf::encoded_*_len` functions (or the candidate's written wire definition for new records).
  - Objects whose chunks are shared with another ContentObject keep their base representation, and this is recorded.
- **Admissibility (pre-registered).** Before use, the model must reproduce, with **0 bytes of error on every tuning item and profile**:
  - `gen-v5` from `gen-v4`, for DEFLATE;
  - `gen-v6` from `gen-v5`, for JPEG;
  - `jpeg-balanced-multichunk` from `gen-v5`, as a multi-chunk check.

  The same check runs on validation at the look. A candidate class whose model check fails anywhere is not decision-grade. Its modelled bytes are reported as `NOT_DECISION_GRADE` only.
- **Limitation (pre-registered).** A selection that rests on a MODELLED candidate stays `INSUFFICIENT_EVIDENCE` until a research writer produces REAL archives under new identifiers and they are re-measured before the design freeze. The reopen trigger is "real `artifact_bytes` differs from the model by any byte".

### 8.4 Correctness gating

Every size, time and memory sample carries its round-trip result, identity and verification status. A sample whose check fails is excluded from OD analysis and reported as an HC violation, with both artifacts committed (objective §2.0 rule 2). It is never averaged.

## 9. Statistics (all comparison experiments)

- **Rules.** Unit and aggregation follow §4.9: item → group → family → corpus with the cited `workload-mix.json` weights, and equal weights as a sensitivity arm. Floors per §3.1. Intervals per §4.10. Verdicts per §4.11.
- **Confidences** follow §4.12: superiority at 1 − 0.01/k_sup; non-inferiority and equivalence at two-sided 0.99; family harm guard at 0.90; held-out at one-sided 0.95.
- **MDE gate.** No look without it (§4.12): MDE ≤ 1 band unit on validation, ≤ 2 on held-out.
- **Deterministic metrics** get 2 repetitions in different rounds and processes plus the repeat-hash check (§4.6, §4.13). Timing and memory rounds follow §4.6, with the adaptive precision rule and tier caps.
- **Rule order.** R1/R2 screens before R4. R4 epsilon-dominance includes the cost condition. R6 minimum gain uses the computed tier (T4 for library-version-defined steps). R8/R9 scope, where R9 partition order is profile → policy → workload mix → family, with expressibility. `fast` performs no categorisation (SPEC §6.1), so family-scoped reconstruction winners are expressible only in balanced, dense and extreme. R10 ties.
- **Tooling.** Decision-grade verdicts come only from the §14 item 2 decision tooling. ebr `normalize` output is informational (§3.6).
- **Multiple comparisons.** Only W against each s ∈ S is confirmatory. Every other comparison is secondary and labelled `unadjusted`.

## 10. Sensitivity (all comparison experiments)

- **§6 in full, per decision and scope:**
  - weight grid and Dirichlet (non-profile decisions only);
  - threshold perturbation ×0.5 and ×2, with the resolvability condition;
  - equal-weight arm, leave-one-family-out, leave-three-families-out, family Dirichlet, WM-* mixes (WM-MEDIA, WM-DIST and WM-BACKUP are the reconstruction-relevant mixes), leave-one-mix-out;
  - tier-stratified, byte-weighted (`total_stored_bytes`) and environment arms;
  - the selection-stability bootstrap (2,000 replicates, seed = spec `seeds.bootstrap`).
- **Bars.** §6.6.
- **Reconstruction-specific arms.** These are reported, and they flag `band_dependent`-style dependence without changing the rule:
  - (a) with and without the F12/F14 derived producer matrices;
  - (b) with and without F20 items;
  - (c) with and without items carrying the `public-benchmark` tag.

## 11. Negative results, outcome records and adversarial search

- **Outcome records.** Every experiment writes `outcome.json` (§10.1) whatever its result. Abandoned runs keep raw data.
- **Pre-registered negative results.** Each protocol's "Expected negative results worth recording" section lists them. Each one found is added to the negative-results register under `research/reports/` and to the `counterevidence` of the decisions it constrains.
- **Adversarial search.** A committed adversarial-search artifact under `research/experiments/<EXP>/adversarial-search/` is required for §8.2 item 5 and CB-8:

| Decision(s) | Artifact home | Search |
|---|---|---|
| DEC-CMP-018, 026 | EXP-RECON-003 | inputs where reconstruction loses net bytes or fails exactness, seeded from producer matrices, F20, jpegli and fuzz corpora |
| DEC-CMP-016, CON-004, CON-005, CMP-012 | EXP-RECON-006 | memory- and CPU-maximising inputs within declared bounds; near-DEFLATE chunks that maximise failed-attempt work |
| DEC-CMP-037 | EXP-RECON-007 | fuzz corpora and minimised findings |
| DEC-CMP-038, CON-007, INT-030 | EXP-RECON-008 | seeded mutations of corrections and JPEG XL representations looking for cross-version or cross-implementation disagreement |
| DEC-CMP-039 | EXP-RECON-009 | photo-library generator search for the largest status-quo regret |
| DEC-CMP-042, 023, CON-010 | EXP-RECON-004 | items that maximise dedup loss from forced cuts; archives where bits are required with no reconstructive byte |
| DEC-MOD-040 | EXP-RECON-010 | exhaustive sentinel and flag mutation enumeration |

## 12. Evidence routes outside these experiments (partial coverage)

| Decision | Part not settled by EXP-RECON-* | Route |
|---|---|---|
| DEC-CMP-012 | structural-transform and codec gating, and the 4096-sample probe thresholds | codecs domain (8.7) |
| DEC-CMP-038 | whether a normative specification of preflate corrections is *acceptable* as a v1 obligation; ISO/IEC 18181 availability terms | EXP-RECON-011 produces a feasibility dossier; the acceptability judgement is FORMAL review by a non-involved reader and, for standards access and licence terms, EXTERNAL review (§8.3) |
| DEC-CON-004 | codec-state, lookback-closure and per-group declaration semantics for non-reconstruction codecs; the all-ones flag vector outside reconstruction | container (11) and crossfile (8.3–8.5) domains |
| DEC-CON-005, DEC-CMP-016 | behaviour on physical low-memory devices | EXP-RECON-012 (BLOCKED) |
| DEC-CON-007 | lzma-rust2 memory estimates across versions (only the reconstruction-adjacent check is in EXP-RECON-008 §5d); old readers against new dictionary construction strings | codecs (8.6) and crossfile domains |
| DEC-CON-010 | AUX verification impact of skippable auxiliary records; ro_compat/compat tiers for metadata bits | container (11) and platform (15) domains |
| DEC-INT-030 | whether vectors plus one in-program differential suffice as independent assurance | EXTERNAL review if the selected route claims independence beyond the program (§8.3); independent reader in the conformance domain (28) |
| DEC-MOD-040 | independent-reader agreement on sentinel cases | conformance domain (28–29), with this domain's case table as input |

## 13. Common threats to validity

1. **Corpus representativeness.** JPEG and DEFLATE prevalence and producer mix in `ebrc-2026.09-v1` are not a sample of real archives. Two JPEG groups per split, and derived matrices that add strata but not groups, make family-scoped claims fragile (§7). Mitigation: supplements, the `workload-mix.json` weights, WM-MEDIA/WM-DIST arms, and reporting per producer stratum.
2. **Research builds versus shipped build.** Mitigated by MVT-16(c), the per-item identity controls, `harness-cli-parity.sh` and the timing parity arm (§4).
3. **Model error for MODELLED candidates** (§8.3). Admissibility requires 0 bytes of error, and decisions resting on a model stay provisional.
4. **Dependency drift.** "Latest" backends are pinned at record time. A later upstream fix can change eligibility (reopen rule §11 item 4).
5. **Virtualized timing.** WSL timing is `VIRTUALIZED` and soft-capped at MEDIUM. Windows timing needs the §14 item 3 runner additions.
6. **Hybrid host.** P-core and E-core placement and Defender scanning (§4.2) matter. External C++ backends differ in binary reputation, so A/A′ applies.
7. **Emulation.** QEMU arm64 serves correctness only. Native ARM64 and macOS are `PLATFORM_BLOCKED` (EXP-RECON-012).
8. **Exposure.** Held-out identities were seen (§3). Sealed tranches are the remedy.
9. **Designer bias.** R0 item 6 critic candidates are open, and CB-7 independent re-analysis applies to every selection.
