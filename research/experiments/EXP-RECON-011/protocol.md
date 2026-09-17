# EXP-RECON-011: Permanent cost and implementation burden of reconstruction — computed C1–C3 census, independent C4–C7 assessment, R2/HC-11 gated-step compliance audit, specification-feasibility and independent-implementation dossier

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-RECON-011 |
| Domain | reconstruction (8.8); common provisions: `research/experiments/_index/reconstruction-common.md` |
| Status | **NEEDS_TOOLING**. Required: the program cost-accounting scripts (§14 item 7, `research/tools/cost/`); reconstruction-specific scripts in §8; pinned `tokei`, `cargo-geiger` (or an equivalent syn-based unsafe counter) and a `cargo-audit` advisory-database snapshot (installs and downloads approved). Script-driven, not runner-driven (no `spec.yaml`). |
| Stage | Cost records must exist before the first validation look of every reconstruction decision (§14 item 7; §7.2 tiers computed, C4–C7 assessed by a non-involved session). The compliance audit (§C) must precede any R2 screen. |
| decision_ids | DEC-CMP-023, DEC-CMP-038, DEC-CON-007, DEC-CMP-018, DEC-CMP-026, DEC-INT-030, DEC-CMP-037, DEC-MOD-040, DEC-CON-010; informs DEC-ECO-016 |
| Proposed types/ODs/HCs | common §2. OD-21–OD-24 cost inputs (A.2 `cost_input`); binary `license_compatibility`, `baseline_codec_public_spec`, `unspecified_baseline_decode_steps`; HC-11 (gated-step admission conditions), HC-12(c), HC-16 (frozen identifiers) |
| Gates | G-A. The tier computation is gated by §14 item 7. C4–C7 need an assessor not involved in any reconstruction candidate (§7.2). |
| Author and exposure | common §3 |
| Feasibility smoke | none |

## 2. Question

For every reconstruction candidate in common §6:
- What is its computed permanent cost: C1 specification and wire items, C2 implementation and reader LoC, C3 dependencies (unsafe and native code, licences, maintenance, advisories, MSRV), C4 attack surface, C5 independent implementability, C6 user concepts, C7 longevity and churn?
- What cost tier results (§7.2), including the maintenance penalty?

For the two frozen reconstruction steps:
- Do they meet every HC-11 admission condition for gated library-version-defined decode steps?
- How large is an implementation-independent specification of preflate corrections, and does any independent implementation of either reconstruction format exist?

## 3. Hypotheses

All hypotheses are `[INFERRED]`.

- **H1 (R2 compliance).** At least one HC-11 admission condition fails for each frozen step (§C). Expected failures: (C.6) no permanent reconstruction vectors in `docs/`, and (C.7) emission by the default profile `balanced`.
  - **Falsified if** every condition passes for both steps.
  - Consequence if H1 holds: default-path emission candidates are eliminated at R2, and the R0 item 7 corrective candidates stand in (common §2).
- **H2 (C3 decode path).**
  - The decode-path dependency closure added by `jxl-oxide` and `preflate-rs` (crates reachable only through them) is ≥ 20 unique `name@version`.
  - At least one of those crates contains `unsafe` code.
  - **Falsified if** the closure is < 20 crates, or no added crate contains `unsafe`.
- **H3 (C5).**
  - No implementation-independent specification of `preflate-rs-0.7.6/deflate-reconstruct-v1` corrections exists. The recreation path's reachable dependency source is ≥ 3,000 non-blank, non-comment lines.
  - JPEG XL JPEG reconstruction has an ISO/IEC standard. Whether it is freely available is recorded, not assumed.
  - **Falsified if** a specification of the correction format exists, or the reachable source is < 3,000 lines.
- **H4 (maintenance penalty, §7.2).** At least one reconstruction reader-path dependency or writer-path pinned encoder fails a maintenance criterion:
  - release within the last 24 months, or feature-complete with a stable spec;
  - ≥ 2 publishers, or organisation ownership;
  - no unresolved RUSTSEC advisory;
  - MSRV ≤ workspace MSRV (1.94, `Cargo.toml`).

  **Falsified if** all pass.

## 4. Decisions informed and how

| Decision | Contribution |
|---|---|
| DEC-CMP-023 | Per-generation reader burden: C1 wire items and words and C2 reader LoC for v4 (codecs and structural transforms), v5 (DEFLATE reconstruction) and v6 (regions). Promote or withdraw candidates get computed tiers. |
| DEC-CMP-038, DEC-CON-007 | C3/C7 dependency longevity and bus factor. §D specification-feasibility dossier and independent-implementation search. Tier and admissibility of `write-independent-spec`, `vendor-and-freeze`/`vendor-and-pin-forever` (vendored-source LoC and licence obligations), `extended-set-crate-reference`, `standard-format-only`, `exclude-from-v1`. |
| DEC-CMP-018, DEC-CMP-026 | Computed tiers of every candidate, including backends (C3 native code for libjxl, brunsli and preflate-cpp; licence for packJPG) and subsets (new identifiers). R2 compliance of the status quo. |
| DEC-INT-030 | C7 permanent-vector obligations per identifier; C5 availability of independent decoders (with EXP-RECON-008) |
| DEC-CMP-037 | C4 attack-surface counts (parser entry points, untrusted-sized allocation sites from EXP-RECON-006 attribution); `work-unit-timeouts` API review (§D.3); tier of `subprocess-isolation` (worker protocol, process surface) |
| DEC-MOD-040 | C1 words and wire items for each sentinel rule (from EXP-RECON-010 rule definitions) |
| DEC-CON-010 | C1 for `bits-minimal-activation` and split audit reasons (new reason codes, T2) |
| DEC-ECO-016 (informs) | Decode-path dependency counts of a minimal reader with and without a `reconstruction` cargo feature |

## 5. Candidates

Every candidate of common §6 gets a cost record. External backends and the status quo are costed on the same schema, and ledger exclusions are recorded without cost.

## 6. Inputs (no corpus)

- **Source trees.** The repository at the pre-registration commit (`crates/`, `docs/`, `Cargo.lock`), plus vendored sources (`cargo vendor --locked` into `/root/eb-research/src/EXP-RECON-011/vendor`, outside git) for the production lock and for the release workspaces of EXP-RECON-008.
- **External backend sources** at the EXP-RECON-003 pins.
- **Snapshots, taken once and hashed:**
  - crates.io index: git commit of `crates.io-index`, and the owners API per crate as JSON;
  - RUSTSEC advisory-db commit;
  - GitHub or GitLab release and tag dates and contributor counts at the pins (from cloned repositories, not web pages);
  - licence files.
- **Measurement outputs** used by C4: EXP-RECON-006 allocation-site attribution; EXP-RECON-007 target inventory and findings; EXP-RECON-008 independence results; EXP-RECON-010 sentinel rule definitions.

## 7. Environment and platform requirements

`wsl-ubuntu`, x86-64. Network is used for one-time snapshot downloads (approved). No timing. Storage: about 15 GB of vendored and cloned sources.

## 8. Procedures and tooling to build

### §A Computed C1–C3 (scripts; program §14 item 7 plus reconstruction extensions)

- **C3 (`research/tools/cost/deps_census.py`).**
  - Run `cargo metadata --locked --format-version 1` and `cargo tree -e normal --target all` per feature set:
    - full `entrybound`;
    - `entrybound` with reconstruction dependencies removed. This uses a research copy of `Cargo.toml` without `preflate-rs`, `jixel` and `jxl-oxide`; it is used only for counting and never built into a product.
    - decode-only closure: crates reachable from the reader entry points, derived from call-graph attribution (§A C2).
  - For each added crate: `name@version`, SPDX expression, `build.rs` compiling C/C++/asm and native source bytes, `unsafe` blocks/functions/impls (pinned `cargo-geiger` or the syn counter `research/tools/cost/unsafe_count.rs`), owners count and organisation flag, last release date, `rust-version`, and RUSTSEC advisories at the snapshot.
  - Output: `decode_path_dependency_count`, `decode_path_native_unsafe_count`, `rustsec_advisory_count`, `maintainer_count`, `msrv_gap`, `release_age_years`, `license_compatibility`.
- **C2 (`research/tools/cost/loc_attribution.py`).**
  - `tokei` (pinned) per file.
  - Function-level attribution with a syn-based parser (`research/tools/cost/fn_attrib.rs`) of `crates/entrybound/src/{reconstruction.rs, jpeg_reconstruction.rs}`, the region, ReconstructionData and audit handling in `ecf/` and `eam/`, and the planner v5/v6 stages. Each function is labelled writer, full reader or minimal reader by reachability from the reader entry points (`ecf::open*`, `archive::unpack*`) versus the writer entry points (`plan_*`, `ecf::encode*`).
  - Decode-path dependency LoC: jxl-oxide crates and the reachable preflate-rs recreate path.
  - Output: reader and writer LoC per candidate, the retained decode surface (§7.4), `independent_reader_loc` upper-bound estimate, and `untrusted_input_loc`.
- **C1 (`research/tools/cost/normative_census.py`).**
  - MVT-12(b) grammar extraction of normative statements and words in `docs/reconstructive-transform-v1.md`, `docs/jpeg-reconstruction-v1.md` and the reconstruction sections of `docs/format-v0.md`.
  - Wire-item census from code registries: record types 14–19, feature bits 0x4 and 0x8, frame version 3, reconstruction `ReasonCode` variants, transform identifiers, format strings, planner IDs `*-v5` and `*-v6`.
  - Candidate deltas come from each candidate's written definition (EXP-RECON-004/009/010). Where no normative text exists yet, the assessor (§B) estimates words, labelled `[INFERRED]`. A disputed tier resolves to the higher one (§7.2).
  - Output: `normative_words_added`, `wire_items_added`.

### §B C4–C7 assessment (judgment; independent assessor)

The template follows §14 item 7 and is recorded in `cost-records/<candidate>.json`:

- **C4:**
  - parser entry points reachable from untrusted bytes;
  - allocation sites sized by untrusted values (from EXP-RECON-006 attribution);
  - new `unsafe`/native code reachable from input;
  - fuzz targets (EXP-RECON-007);
  - `attacker_controlled_parameters`, `validated_only_vulnerability_classes`.
- **C5:**
  - normative specification independent of implementation (yes/no, with citation);
  - external normative references stable and freely available;
  - vectors present;
  - independent minimal decoder LoC;
  - `reader_fragmentation_count`: optional features that make archives unreadable by a minimal baseline reader. Bits 0x4 and 0x8 count 1 each.
- **C6:** new flags, modes or concepts, for example an opt-in flag for `recon-opt-in-not-default`.
- **C7:**
  - planner-ID churn from pinned encoder versions;
  - count of library-version-defined decode steps;
  - permanent-vector and determinism maintenance per identifier;
  - reliance on a single upstream;
  - add-later cost for `defer`/`drop` candidates: post-v1 re-introduction needs new incompat bits and identifiers, and migration of v5/v6 archives already written (DEC-CON-013 link).
- **Maintenance penalty.** §7.2 criteria per dependency.
- **Tier.** The maximum over elements.

### §C HC-11 gated-step compliance audit (FORMAL, mechanical; `research/tools/recon/gated_step_audit.py`)

For each of `deflate-reconstruct/v1` and `jpeg-jxl-reconstruct/v1`, each condition is recorded as pass or fail with `path:line` citations at the pre-registration commit:

1. **Outside the baseline set** (SPEC §5.8 baseline and extended membership). The audit locates the registry declaration in `docs/codec-transform-v1.md` or the SPEC citation in the ledger; if no membership statement exists, the condition is recorded as *undeclared*, which counts as fail.
2. **Behind an `incompat` feature bit**, verified in code constants and `docs/*`.
3. **The registry declares that no public specification exists**, as an explicit statement.
4. **The version identity is part of the recorded plan's registry identifier.** Checked through the `DEFLATE_RECONSTRUCTION_FORMAT` and `JPEG_RECONSTRUCT_PARAMETERS` constants.
5. **Output digest-verified before release.** Checked by code-path citation plus the EXP-RECON-010 F4 result.
6. **Permanent vectors exist.** The files are `docs/*vectors*`, or an equivalent registered vector set.
7. **Not emitted on a default decode path.** Default creation profile emission is taken from EXP-RECON-001 counts; `balanced` is the default per the CLI help.

Output: `research/experiments/EXP-RECON-011/gated-step-audit.json`, plus `unspecified_baseline_decode_steps` and `baseline_codec_public_spec` for any step placed on the baseline or default path. An independent reviewer signs the audit, and the sign-off is recorded in the file.

### §D Specification feasibility and independent implementations (dossier; `research/experiments/EXP-RECON-011/dossier/`)

1. **preflate corrections** (`research/tools/recon/preflate_spec_scan.py`), over preflate-rs 0.7.6 sources:
   - functions and LoC reachable from `recreate_whole_deflate_stream` (syn call-graph approximation);
   - enumerated correction token and enum variants;
   - predictor model parameters;
   - any floating-point types or operations on the path (R2 unspecifiability test);
   - `usize`/pointer-width-dependent arithmetic;
   - SIMD or `cfg(target_feature)` branches.

   Output: a feasibility table with counts and citations. The words needed for a normative specification are estimated by the assessor and labelled `[INFERRED]`.
2. **JPEG XL reconstruction.**
   - Standard identity and edition (ISO/IEC 18181-1 and -2) with retrieval date and access terms: free or paid, recorded from the ISO catalogue page, which is the primary source for availability.
   - Mapping from the jxl-oxide `reconstruct_jpeg` path to standard clauses: citable only if the text is accessible to the program; otherwise the item is routed to EXTERNAL review (common §12).
   - The EXP-RECON-008 cross-implementation results.
3. **Dependency API review for `work-unit-timeouts` (DEC-CMP-037).** Does jixel, jxl-oxide or preflate-rs expose deterministic work, iteration or allocation limits (API items with citations)? Yes/no per crate.
4. **Independent-implementation search** (scripted, snapshot-dated):
   - reverse dependencies of `preflate-rs` in the crates.io index snapshot;
   - crates, PyPI and npm package names matching `preflate` (public registry APIs);
   - the precomp-cpp and preflate C++ correction format compared by source reading at the pins (same or different format; citations).

   Output: the query, date, results and SHA-256 of the saved API responses.

## 9. Seeds, warmups, replication

- **Scripts** are deterministic. They are run twice from the same snapshots, and output files must be byte-identical (§12.3 regeneration).
- **Snapshots** are taken once and hashed. A re-run with a new snapshot is a new record version, recorded as a deviation.
- **Assessor judgments** are made once by one assessor. A second non-involved assessor independently re-derives the tier for every T3/T4 candidate (CB-7 analogue for cost). Disagreements resolve to the higher tier (§7.2).

## 10. Metrics

Cost inputs (A.2 `cost_input`):
- `decode_path_dependency_count`, `decode_path_native_unsafe_count`, `rustsec_advisory_count`, `maintainer_count`, `msrv_gap`;
- `normative_words_added`, `wire_items_added`;
- `untrusted_input_loc`, `attacker_controlled_parameters`, `validated_only_vulnerability_classes`;
- `independent_reader_loc`, `reader_fragmentation_count`.

Binary: `license_compatibility`, `baseline_codec_public_spec`, `unspecified_baseline_decode_steps`.

Descriptive: `release_age_years`.

Derived: the computed tier T0–T4 per candidate.

## 11. Analysis

1. **Cost records per candidate.** Each record carries every element, the computed tier, citations, and the generating command outputs (§7.6 ledger fields `complexity_cost`, `dependency_cost`, `security_cost`, `wire_cost`).
2. **R2 screens.**
   - Licence: strong copyleft in shipped production code fails; weak copyleft or unusual licences go to external review (§7.5).
   - HC-11 audit (§C): a candidate whose emission places a failing gated step on a default or baseline path is eliminated.
   - Unspecifiable decode (floating point or environment dependence found in §D.1) fails R2 for any baseline placement.
3. **Tiers into rules.** Tiers feed R4 condition 5, R6 minimum gains (Appendix D.6 multipliers) and R10 key 1 for every reconstruction decision analysed in EXP-RECON-004/005/006/007/009/010.
4. **Dossier (§D).** The dossier is the evidence package for the FORMAL and EXTERNAL parts of DEC-CMP-038, DEC-CON-007 and DEC-INT-030. Acceptability of `write-independent-spec` needs a non-involved reader (FORMAL sign-off) and, for standards access, external review.

## 12. Practical-significance thresholds

Not applicable. Cost enters only through tiers (§7.2, §7.3). Tier multipliers are cited, not restated.

## 13. Sensitivity analysis

- Each candidate's tier is recomputed with the maintenance penalty toggled per criterion.
- Selections in EXP-RECON-004/009 are re-run with each T3/T4 candidate moved one tier down and one tier up. The result is reported as a cost-sensitivity arm; it is not a §6.6 bar.

## 14. Expected negative results worth recording

- The status quo fails HC-11 admission (H1): default-path emission of library-version-defined steps is a production defect under objective §2.0 rule 5.
- No independent implementation or specification exists for preflate corrections (H3). `ci-differential-independent` and `independent-spec-plus-second-implementation` are infeasible without new work of measured size.
- JPEG XL standard text is not freely available, so C5 "freely available normative reference" fails and the question goes to external review.
- A maintenance penalty applies (H4).

## 15. Threats to validity

- **Call-graph approximations** (syn-based, no monomorphisation or dynamic dispatch resolution) over- or under-attribute reader LoC. Reported as bounds, with a manual spot check of 20 functions by the assessor.
- **Registry and API snapshots** age. They are dated, and reopen rule §11 item 2 applies.
- **Assessor subjectivity** in C4–C7. Mitigated by a second assessor for T3/T4 and the higher-tier resolution rule.
- **Normative-word estimates** for candidates without text are `[INFERRED]` and cannot carry a decision alone (§9.2). They only set tiers conservatively (higher on dispute).

## 16. Held-out placeholder

Not applicable (no corpus). Cost records are frozen in Commit A (§5.5), and tiers used at R5 re-evaluation are those records.

## 17. Cost and effort estimates

- **Compute.** About 6 CPU-hours (vendoring, geiger/unsafe counting, tokei, scripts).
- **Disk.** About 15 GB of sources and snapshots.
- **Tooling.** Program-wide cost scripts (§14 item 7) plus 4 reconstruction scripts: about one to two sessions.
- **Agent effort.** Scripted census. Judgment: C4–C7 assessor, second assessor for T3/T4, and the independent reviewer of the HC-11 audit and dossier.

## 18. Deviations and outcome (append-only)

- Deviations: none.
- Outcome: `outcome.json` (with the list of cost records, audit and dossier paths).
