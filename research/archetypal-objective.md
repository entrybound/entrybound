# Entrybound Archetypal Objective

| Field | Value |
|---|---|
| Status | **PRE-REGISTERED, round-1 revision** (Phase A, method gate). This revision dispositions every finding of `research/methods/method-review-round1.md` (see [Revision history](#revision-history)); its commit is the pre-registration record. Round-2 review may still revise it under `decision-method.md` §0.2 item 1, because no decision-relevant result exists. |
| Date | 2026-09-16 |
| Companion | `research/decision-method.md` fixes the decision procedure, thresholds, statistics, split discipline, sensitivity, cost and status rules. This document fixes what is forbidden (hard constraints), what is optimized (optimization dimensions), how each is measured and aggregated, and how the two relate. Precedence when the two appear to conflict: **procedure and rule order**, thresholds, statistical procedures, canonical parameter values (network profiles, affinity and thread configurations, repetition counts) and canonical metric names and roles: `decision-method.md` governs (its §2, §3, §4 and Appendices A and C). What counts as a violation and what a metric measures: this document governs. |
| Inputs (by hash, never copied) | SPEC `design/2026-08-29-entrybound-product-architecture.md` SHA-256 `1f881c7b13f193b41353b90066d33ca6abcb7d4c3dcd4f58e22834801f0fe29c` (§1-§3.9, §6.5, §8, §11, §12, §23, Appendix A read in full; §20.5 and §24 consulted for outcome classes and non-goals); `CONTRIBUTING.md` SHA-256 `ab28c1ce8e2ae114603304643167537a90ce32366506a48967caa4efa32f16cc`; `research/requirement-ledger.csv` SHA-256 `306bf48bb72a22d3de757005e25f388e237f54e8c0b82950ed85e8c3aa69b833` (157 INVARIANT and 432 REQUIREMENT rows screened by script); `research/decision-ledger.jsonl` SHA-256 `98b5401d79c621fab8db2a72e4825dcef9d74dc60de042fdb3990c46daa4068b`; repository HEAD `f56dd3d` at drafting. |
| Result state at drafting | `research/raw/` and `research/normalized/` contain only `EXP-SMOKE-000`, a pipeline smoke test with `decision_ids: []`. No decision-relevant experiment result exists at drafting. [FORMALLY_DERIVED] |
| Generated content | Appendix C is the verbatim output of `C:/Python313/python.exe research/tools/ledger/objective_screen.py --repo .` at the ledger hashes above. No count in Appendix C was typed by hand. |

## 0. Evidence tags

Every material claim ends with one tag, or its table row carries one in a Tag column. A tag covers the sentence or row it ends.

| Tag | Meaning in this document |
|---|---|
| `FORMALLY_DERIVED` | Follows by definition or deduction from normative project text (SPEC by section, repository `docs/*.md`, `CONTRIBUTING.md`), or is a mechanical computation over committed inputs at a stated hash. |
| `EMPIRICALLY_MEASURED` | Produced by a checked-in command from raw artifacts under `research/raw/`. No decision-relevant claim in this draft carries this tag, because no such artifact exists yet. |
| `EXTERNALLY_SOURCED` | Taken from a cited external primary source, such as a standard, paper, tool manual or the SPEC's own evidence-base citations about incumbents. |
| `INFERRED` | A judgement or design choice made by this document, such as picking a metric as a proxy or choosing an aggregation operator. Reviewers should challenge these first. |
| `EXPERT_REVIEW_REQUIRED` | Cannot be settled inside this program. Needs independent cryptographic, security, usability or specification expertise. |
| `UNRESOLVED` | A known gap, contradiction or missing instrument at drafting. |

Abbreviations: **HC** hard constraint; **OD** optimization dimension; **MVT** mechanical violation test; **F-nn** repository freeze (Appendix B); **CC** capability combination (§1.3); **LAI/PCR/AUX/PCI** the identities of SPEC §8.0.

---

## 1. Governing question

> **GQ.** Among all designs of the Entrybound system that satisfy every hard constraint HC-01 to HC-18, which design most completely realizes the archetypal ideal of Entrybound as the definitive next-generation archive system, subject to its semantic, security, fidelity, determinism, interoperability, bounded-resource, longevity and usability invariants? Where no single design does, what is the smallest set of declared profiles, layouts and caller policies that does, and what stays out of reach, with the measured cost that keeps it out of reach?

"Design" means the whole system under study: the semantic model (EAM), the container (ECF), the planner, chunker and codec/transform registries, profiles, layouts, the crypto envelope, verification and access policies, legacy adapters, the CLI and library defaults, the normative specification, and the conformance corpus. That scope matches SPEC §1's eight components plus the promoted corpus and verifier. [FORMALLY_DERIVED]

### 1.1 The archetypal ideal

The ideal consists of the following properties, each taken from the SPEC. The research may not redefine them. It may only measure how completely a design realizes them.

| # | Property of the ideal | Source | Tag |
|---|---|---|---|
| A1 | Every conforming archive, opened by every conforming reader, has exactly one interpretation: which entries exist, their names, kinds, bytes and metadata. No admissible-interpretation set exists, and source ambiguity is data about the conversion. | SPEC §1.2 | FORMALLY_DERIVED |
| A2 | The semantic model is authoritative and the container only encodes it. Two archives that serialize the same model are the same archive. | SPEC §2.1(1) | FORMALLY_DERIVED |
| A3 | Legacy defects stop at the adapter boundary. Compatibility comes from converting, never from accommodating. | SPEC §2.1(2) | FORMALLY_DERIVED |
| A4 | Every fact has exactly one authority. Indexes are caches. | SPEC §2.1(3), §4.5 | FORMALLY_DERIVED |
| A5 | Capability is declared before payload and never discovered mid-decode, or the archive declares that it cannot declare it. | SPEC §2.1(4) | FORMALLY_DERIVED |
| A6 | Loss is declared and never silent, at capture, at extraction and at conversion. | SPEC §2.1(5) | FORMALLY_DERIVED |
| A7 | Extracting a Complete archive reproduces every original file byte for byte, with no exempt mode, profile, flag or extension. | SPEC §6.5 | FORMALLY_DERIVED |
| A8 | Content identity (LAI), physical chunking (PCR) and declared honesty (AUX) are separate signed roots. Recompression, re-chunking, re-encryption and layout change therefore do not break signatures, and staleness is reported rather than hidden. | SPEC §8.0-§8.2 | FORMALLY_DERIVED |
| A9 | "Not representable" beats "kernel-enforced", which beats "declared and bounded", which beats "validated". | SPEC §11.1 | FORMALLY_DERIVED |
| A10 | Logical, planner and physical determinism exist. Encryption is honestly non-deterministic, and reproducibility is anchored to LAI. | SPEC §12.1-§12.3 | FORMALLY_DERIVED |
| A11 | The claim is system-level, not single-axis. For any combination of capabilities a real project needs, Entrybound provides all of them at once, without the project having to know in advance which combination it needed. | SPEC §23.1 | FORMALLY_DERIVED |
| A12 | Every future feature must answer: "can Entrybound provide this without violating an invariant in §3.9?" | SPEC §23.5, §2.2 | FORMALLY_DERIVED |

A11 is explicitly falsifiable, and so is this objective. A research outcome of "combination CC-k cannot be served without an HC violation, at measured cost X" is a legitimate answer to GQ, not a failure of the program. [FORMALLY_DERIVED from SPEC §23.1 "stated so it can be falsified"]

### 1.2 What "most completely" means operationally

**The normative decision procedure is `decision-method.md` §2 (rules R0-R10), and only that procedure selects.** This section states the ideal the procedure operationalizes and where each part of the ideal enters it. [INFERRED operationalization of A11 and A12]

| Part of the ideal | Where it enters `decision-method.md` |
|---|---|
| **Feasibility (binary).** A candidate is feasible only if every applicable HC MVT (§2) reports zero violations. No optimization gain of any size offsets a violation. | R1 (hard constraints, applicable HCs from the ledger `hc_ids`) and R2 (unboundable or unspecifiable designs) |
| **Non-domination (graded).** Among feasible candidates, keep those not epsilon-dominated on the decision's OD set for its scope, confirmed on validation and replicated on held-out after the design freeze. | R3-R5 |
| **Permanent cost.** OD-21 to OD-24 are measured as cost elements C1-C7 and cost tiers. Cost enters the procedure in exactly three places: R4 condition 5 (a more expensive candidate cannot eliminate a cheaper one by measurement alone), R6 (minimum gain across tiers) and R10 key 1 (ties). | `decision-method.md` §2.0, §7 |
| **Combination coverage (system-level).** CC coverage is a **reporting obligation, not a selection step**. It is computed at program synthesis (phase E) over the set of decided configurations, per §1.3, and counts separately the combinations served by the **default configuration** (`ebound pack` with no flags, and the default extraction policy), because A11 says the project should not have to choose in advance. The default-configuration count is a reported outcome; no R-rule uses it. | `decision-method.md` §10.1 item 5 |
| **Declared residue.** Every CC not served, every OD on which the chosen design is practically worse than its named specialist baseline, and every HC whose MVT could not run (HC_UNVERIFIED), recorded as negative results and remaining unknowns. | `decision-method.md` §10.1 |

CC coverage does not select because a single ledger decision is scoped to its own OD set, while a CC spans several decisions and several ODs; ranking one decision's candidates by a system-level count would let a decision outside a CC's ODs be decided by it. [INFERRED]

### 1.3 Capability combinations (CC)

These are the ten rows of SPEC §23.2. [FORMALLY_DERIVED] SPEC §23.2 states that no incumbent serves these combinations, so a CC has no single "capability-matched baseline". Each OD metric listed for a CC is instead compared with a **named specialist baseline for that OD alone** (the table below), using the non-inferiority test of `decision-method.md` §4.11 on the validation split and replicated on held-out. [INFERRED]

| Outcome | Condition |
|---|---|
| `SERVED` | Every HC floor listed for the CC passes (no HC_UNVERIFIED in scope), and every listed OD metric is non-inferior to its specialist baseline at the metric's band. |
| `SERVED_WITH_DECLARED_COST` | Every HC floor passes, and every listed OD metric is non-inferior at its **declared-cost tolerance**, but at least one is not non-inferior at its band. The measured gap is reported. The tolerance is the SPEC §23.4 magnitude where the SPEC states one, and otherwise equals the band, so this outcome exists only where the SPEC accepts a cost. |
| `NOT_SERVED` | Otherwise, including any HC_UNVERIFIED floor. |

Declared-cost tolerances taken from SPEC §23.4: "Independence costs ratio: a few percent versus solid" gives `artifact_bytes` ratio ≤ 1.05 against the solid specialist for CC-01, where "a few percent" is read as 5% [INFERRED]; "Chunking overhead on tiny archives: a few hundred bytes" gives `fixed_overhead_bytes` ≤ 512 B on the small tier for every CC [INFERRED reading of "a few hundred"]. SPEC §23.4 gives no other magnitude, so every other tolerance equals the band. [FORMALLY_DERIVED from SPEC §23.4 table]

| CC | Combination (SPEC §23.2 row) | HC floors | OD metrics that must be served (canonical names, `decision-method.md` Appendix A.2) | Specialist baseline per OD (configurations fixed in `research/baselines/` before any CC result) |
|---|---|---|---|---|
| CC-01 | Random access and strong compression | HC-02, HC-09 | OD-10 `random_entry_latency_s`; OD-05 `artifact_bytes` | Random access: 7z non-solid (LZMA2). Ratio: solid 7z (LZMA2) and tar.xz; the better of the two per item |
| CC-02 | Streaming creation and random access | HC-06, HC-15 | OD-09 `streaming_capability` (binary), `stream_unpack_wall_s`; OD-10 `random_entry_latency_s` | Streaming: tar+zstd pipe. Random access: 7z non-solid |
| CC-03 | Deduplication and a portable single-file archive | HC-01, HC-09 | OD-05 `artifact_bytes` on F17 and F18; OD-26 `single_file_self_contained` (binary, M26.5) | Deduplication: the best of tar+zstd with `--long` window and solid 7z. Single file: none needed (binary) |
| CC-04 | Authenticated encryption, an encrypted index and multi-recipient | HC-14 | OD-15 `undetected_tampers` (binary), `artifact_bytes` encrypted versus unencrypted; OD-10 `encrypted_random_entry_latency_s` (M10.5); OD-12 `http_requests` on encrypted archives; OD-16 `presence_advantage` per padding mode | Encryption overhead: 7z AES-256 with encrypted headers. Random access: 7z non-solid encrypted. Leakage: the length-only reference observer of OD-16 |
| CC-05 | Reproducible builds and preserved metadata | HC-13, HC-07 | OD-03 `restore_fidelity_fraction`; OD-20 `migration_determinism_fraction` (binary) | Fidelity: GNU tar 1.35 `--xattrs --acls --selinux` pax format |
| CC-06 | A signature that survives recompression | HC-10 | OD-20 `migration_identity_conformance_fraction` (binary); OD-15 `undetected_tampers` (binary) | None (binary) |
| CC-07 | Verified partial download | HC-15, HC-10, HC-18 | OD-11 `verified_range_fetch_bytes`, `verify_overhead_pp`; OD-12 `http_requests`, `http_bytes`, `remote_random_entry_latency_s` | Unverified HTTP range reads of 7z non-solid and ZIP through the same origin |
| CC-08 | Declared resource bounds before decompression | HC-06 | OD-17 `declared_tightness_ratio`, `benign_false_refusal_fraction` | None: no incumbent declares bounds; tightness is reported against 1 |
| CC-09 | An explicit statement of what was not preserved | HC-07 | OD-03 `capture_fidelity_fraction`, `restore_fidelity_fraction` | GNU tar 1.35 pax and bsdtar 3.8.8 |
| CC-10 | Post-quantum confidentiality for long-lived archives | HC-14 | OD-15 `assurance_state` (descriptive; `EXPERT_REVIEW_REQUIRED`) | None; the outcome is `NOT_SERVED` until external review (SPEC §25.2) |

The CC outcome table is a reporting obligation of program synthesis (§1.2); it never selects a candidate within a decision. [INFERRED]

### 1.4 Decision variables, fixed elements, and unit of evaluation

**The research may vary:** new wire records and feature bits, allocated as new identifiers only (HC-16); planner, chunker, similarity, codec and transform registry contents under new planner or chunker IDs; profile parameters; layout choices and stream windows; access, coalescing and verification policies; padding and leakage parameters under a new crypto suite or version only (F-12); default extraction and resource policies; CLI and library defaults and surface; legacy import and export profiles; specification text; and conformance-corpus contents. [FORMALLY_DERIVED from CONTRIBUTING.md freezes, which permit change only through new identifiers]

**The research may not vary:** the hard constraints HC-01 to HC-18; the behavior of already-published identifiers (Appendix B), including decode support for archives that use them (HC-16); the SPEC §24 non-goals (not a filesystem, backup system, version-control system, single-stream compressor, transport protocol, key-management system, signing or transparency service, lossy compressor, or codec; never requires live infrastructure to verify; in-archive error correction permanently refused); and the program's standing constraints (local Windows, WSL2 and Docker only; production semantics unchanged by experiments; research features default-off and byte-identical when disabled). [FORMALLY_DERIVED from SPEC §24 and PROGRESS.md standing decisions]

**Unit of evaluation.** An observation is a tuple (candidate configuration, corpus item, operation, platform, repetition).

- **Configuration:** format version and feature set, planner ID, chunker ID, profile, layout, encryption mode, policy, and build.
- **Operation:** pack, unpack, verify, list/inspect, random entry read, random range read, HTTP-range remote read, repack (representation-only or replan), key add/remove, sign or verify signature, import (ZIP, tar family, compressed streams, 7z), export (ZIP, tar/pax, compressed tar), publish (multi-target), or salvage.
- **Platform:** Windows 11 host on NTFS, WSL2 Ubuntu 24.04 on ext4 under `/root/eb-research` (never `/mnt/d` for metadata work), Docker linux/amd64, QEMU linux/arm64, or QEMU user-mode linux/amd64 with a restricted `-cpu` model (MVT-11). QEMU is labeled emulated and used for correctness and determinism only. WSL2 timing is labeled virtualized (`decision-method.md` §4.3).
- **Parameters.** Thread counts, affinity configurations, network profiles, repetition counts and injection counts named in this document are copies of the canonical parameter table in `decision-method.md` Appendix C.1. Where they differ, Appendix C.1 governs and this document is corrected.

[FORMALLY_DERIVED from PROGRESS.md execution environment]

---

## 2. Hard constraints

### 2.0 Rules that apply to every HC

1. **Binary.** An HC is satisfied or violated. No weights, trade-offs or practical-significance bands apply, and no HC outcome is ever compared against an optimization threshold. An MVT whose oracle is a measurement (a scaling exponent, a chance rate, a detection count) states its decision criterion as an exact, pre-registered number in the MVT itself; that criterion defines the violation and is not a band. [INFERRED from SPEC §23.5, which treats the invariant list as a gate rather than a trade-off]
2. **One reproducible violation is enough.** A violation is a committed artifact containing an input generator (format-building code, per F-18), the exact command, the build identity and the observed output. It sits under `research/raw/<EXP>/`. Absence of violations in N trials is evidence, not proof. Every probabilistic MVT states its detection bound, or is labelled **regression evidence** (no bound claimed). [INFERRED]
   - **Nondeterminism artifact rule.** Two differing outputs from identical input, configuration, build and declared resource model are themselves the reproducible violation artifact; both outputs and their metadata are committed. No confirming rerun is required. The violation is waived only when an infrastructure fault is proven: the stored artifact's re-hash differs from its recorded hash, or the environment capture logs a storage or memory error for that sample. [INFERRED]
3. **Two elimination levels.**
   - **L0 (inspection):** a candidate whose own specification contradicts an HC, such as deterministic ciphertext, a second length field or a fail-open unknown record, is excluded without running anything. An L0 exclusion, and any exclusion by formal argument (`decision-method.md` R1(b)), requires a written counterexample (archive bytes, a state sequence, or the specification clause that admits the violation) and sign-off by a session not involved in constructing the candidate set. The exclusion is recorded in `candidate_exclusion_reasons` with `invariant_violated`, the counterexample reference and the reviewer.
   - **L1 (mechanical):** every other candidate must pass the applicable MVTs on the conformance corpus and on the tuning and validation splits before design freeze, and on held-out after freeze.

   [FORMALLY_DERIVED from the decision-ledger schema fields; INFERRED sign-off rule]
4. **HC_UNVERIFIED.** If an applicable MVT cannot run (missing instrument, `PLATFORM_BLOCKED`, or expert review required), the candidate's status for that HC is HC_UNVERIFIED. HC_UNVERIFIED is never counted as a pass. It is allowed in a `DECIDED` decision only for a **platform** that the decision's `decision_scope` explicitly excludes. It is never allowed for a property: a decision cannot exclude an HC, or part of one, from its scope. A `PLATFORM_BLOCKED` platform inside the scope leaves the decision `INSUFFICIENT_EVIDENCE`. [INFERRED]
5. **Production defects are not relaxations.** An MVT that finds a violation in the current production baseline creates a defect and ledger work item. It is not grounds to weaken the HC. An HC changes only through a SPEC revision with adversarial review, which is outside this program's authority (§4.5). [FORMALLY_DERIVED from SPEC §23.5 and the PROGRESS.md standing decision on production semantics]
6. **Outcome vocabulary.** MVT oracles use the six outcome classes of SPEC §20.5 (`OK`, `UNSUPPORTED`, `TRUNCATED`, `CORRUPT`, `NONCONFORMING`, `POLICY_REFUSED`), the `UNVERIFIED` qualifier, and stable reason codes. The production enum is `OutcomeClass` in `crates/entrybound/src/diagnostics.rs`. [FORMALLY_DERIVED]
7. **Applicability.** The HCs applicable to a decision are the ledger field `hc_ids` (ledger schema v2, `decision-method.md` §14 item 12). `hc_ids` is derived from the constraint crosswalk `research/methods/constraint-crosswalk.csv` (`decision-method.md` R1), which classifies every distinct free-text ledger constraint as `HC-xx`, `F-xx`, `NON_GOAL`, `PROGRAM` or `DECISION_INPUT`, plus every HC whose invariants a candidate of the decision can affect, as assigned in the reviewed `hc_ids` assignment. Only `HC-xx` and `F-xx` classes screen. Until the crosswalk and `hc_ids` exist and are signed off, HC applicability is undefined and no decision can leave `INSUFFICIENT_EVIDENCE`. [INFERRED]
8. **Triage rule for differential oracles** (MVT-03(b), MVT-11(b), MVT-12(a)). A disagreement between the production reader and another reader counts as a bug in the other reader only if a cited normative sentence (document, section and line) unambiguously mandates the production behavior. A session involved in neither reader adjudicates. The triage log is committed with one citation per case, and the counts of cases attributed to each side are reported. A case without such a citation counts as a specification violation of the HC. [INFERRED]
9. **Research builds as oracles.** An HC result obtained with an instrumented research build (F-24) counts only while MVT-16(c) passes for that build: with the research features off it is byte- and outcome-identical to the production build. Race- or timing-sensitive HC results (MVT-05(b), MVT-13(a) stress, MVT-14(c)) are replicated black-box on the production build wherever the oracle permits, and the report says where it does not. [INFERRED]

### 2.1 Summary

| HC | Constraint | Invariants (primary; supporting) | Freezes (Appendix B) | MVT oracle in one line | Status at drafting |
|---|---|---|---|---|---|
| HC-01 | One authority per semantic fact | I1, I2, I10, I19, I23 | F-01, F-02, F-07, F-10, F-19 | Authority census finds at most one authoritative declaration per fact; cache-disagreement mutants never surface the cached value | UNRESOLVED (Appendix C.2: 2 weak rows) |
| HC-02 | Exact logical losslessness | I14, I28, I31; I30 | F-03, F-11, F-22 | Zero byte mismatches across the round-trip matrix; injected reconstruction error is never written | UNRESOLVED (6 weak rows) |
| HC-03 | Deterministic, unique native interpretation | I15, I3, I6, I26; I19, I21 | F-07, F-15, F-18 | Canonical re-encode is byte-identical; readers and layouts never disagree on (class, code, EAM) | UNRESOLVED (I15 row REQ-MOD-0110 INSUFFICIENT) |
| HC-04 | Fail-closed unknown critical semantics | I12, I21, I11 | F-03, F-12, F-15, F-17, F-26 | Every unknown critical feature yields `UNSUPPORTED` with zero output; unknown optional data is ignored but tamper-evident | UNRESOLVED (REQ-PLT-0144 MISSING) |
| HC-05 | Caller-owned extraction and security policy | I16; I25, I17 | F-11, F-17, F-18, F-23 | No archive mutation enlarges the operations performed under a fixed policy; zero escapes | UNRESOLVED (19 weak rows incl. REQ-PLT-0110 CONTRADICTED) |
| HC-06 | Declared resource bounds | I17, I20; I19, I29 | F-09, F-15, F-16, F-21 | Counting-allocator peak stays within exact declared bounds; no allocation sized from an untrusted field precedes its budget check; CPU work on hostile input scales at most linearly | UNRESOLVED (REQ-CON-0004, REQ-CON-0005 INSUFFICIENT) |
| HC-07 | No silent metadata or semantic loss | I13, I14, I18, I11 | F-17, F-19, F-20, F-21, F-22 | Every observed source-to-destination difference is covered by a typed declaration; strict import never resolves a Divergence; every packed entry comes from one coherent source version or is declared `unstable_source` | UNRESOLVED (REQ-PLT-0079, REQ-LEG-0056 CONTRADICTED) |
| HC-08 | Archive semantics independent of host filesystem interpretation | I4, I5, I27; I26 | F-17, F-27 | Identical (class, LAI, AUX, PCR, entry digests) on every host; hostile names are refused or renamed with a report, never silently merged | UNRESOLVED (13 weak rows) |
| HC-09 | Bounded dependency chains | I29; I30, I10 | F-03, F-06, F-10, F-25 | Every chunk's closure is acyclic, backward-only and within its declaration; Complete archives decode offline | UNRESOLVED (REQ-ACC-0001 CONTRADICTED) |
| HC-10 | Canonical identities | I7, I8, I9, I22, I28; I23 | F-07, F-08, F-13, F-15, F-17 | Identity change pattern equals the SPEC §8.1 table cell for cell; permanent vectors match | UNRESOLVED (28 weak rows) |
| HC-11 | No runtime-dependent decoder interpretation | I30, I15; I31 | F-03, F-05, F-12, F-24 | Decoded output and outcome are invariant under verified thread, core, CPU-feature, locale, OS, memory and architecture variation | UNRESOLVED (instrument not built) |
| HC-12 | Independently implementable native specification | I30, I15, I12; I21 | F-12, F-18 | A clean-room reader passes the baseline conformance corpus at 100% with matching codes; every normative rule has a case | UNRESOLVED (REQ-ECO-0030, REQ-ECO-0021 MISSING); EXPERT_REVIEW_REQUIRED |
| HC-13 | Reproducible deterministic modes | I8, I9, I28; I15, I30 | F-03, F-04, F-06, F-12, F-22 | `--deterministic` unencrypted PCI is identical across threads, hosts, traversal order and memory; encrypted LAI is identical and ciphertext differs | UNRESOLVED (18 weak rows incl. REQ-CON-0055 CONTRADICTED) |
| HC-14 | Crypto correctness never traded for performance | I9, I24, I25; I12 | F-12, F-13, F-14, F-16, F-23 | Vector gate at 100%; zero plaintext released on any tamper; zero (key, nonce) repeats; keyed boundaries; declared padding mode recorded and public lengths conform to it; frozen-suite edits excluded at L0 | UNRESOLVED (22 weak rows); EXPERT_REVIEW_REQUIRED |
| HC-15 | Verification honesty and distinguishable failure | I25, I21 | F-08, F-10, F-11, F-18 | Reported verification state never exceeds the oracle state; fault class equals injected class through to CLI exit status | UNRESOLVED (22 weak rows incl. REQ-INT-0025 INSUFFICIENT) |
| HC-16 | Published identifiers are immutable | I15, I22, I30 | F-03, F-04, F-06, F-12, F-13, F-15, F-17, F-20, F-22, F-26, F-28, F-29 | Golden vectors, frozen-ID differential runs against `9e44608` and archives per frozen ID reproduce byte for byte; no identifier value is reused; decode support persists | UNRESOLVED (12 weak rows) |
| HC-17 | Structural path and entry hazards are unrepresentable | I3, I4, I5, I6, I23, I26, I27 | F-17, F-19, F-21 | No constructor or encoder emits a hazard; decoders return `NONCONFORMING` with a stable code; strict legacy import refuses | UNRESOLVED (9 weak rows) |
| HC-18 | Crash-consistent output and safe remote origins | I25; I21, I16 | F-10, F-11, F-22 | A kill at any seeded point leaves no destination object that verifies as `OK`, no partial publish and nothing overwritten; origin misbehaviour is refused, never read as verified data | UNRESOLVED (instruments not built) |

The 14 constraints named by the program map as follows. The program's "one authority per semantic fact" is HC-01; "exact logical losslessness" is HC-02; "deterministic native interpretation" is HC-03; "fail-closed unknown critical semantics" is HC-04; "caller-owned extraction/security policy" is HC-05; "declared resource bounds" is HC-06; "no silent metadata/semantic loss" is HC-07; "archive semantics independent of host filesystem interpretation" is HC-08; "bounded dependency chains" is HC-09; "canonical identities" is HC-10; "no runtime-dependent decoder interpretation" is HC-11; "independently implementable native specification" is HC-12; "reproducible deterministic modes" is HC-13; and "crypto correctness never traded for performance" is HC-14. HC-15 to HC-18 are added because four invariant families or freezes would otherwise be covered only incidentally:
- I21 and I25 (verification honesty) become HC-15.
- The repository's identifier freezes become HC-16.
- The unrepresentability invariants I3, I5, I6, I26 and I27 become HC-17.
- Writer crash consistency and remote-origin safety (F-10, F-11, F-22, the ledger constraint "CLI exclusive-create no-overwrite") become HC-18; they were previously only a "must pass" metric inside OD-12 (review finding B16).

[INFERRED]

"Weak rows" are ledger rows mapped to the HC whose `evidence_state` is `CONTRADICTED` or `INSUFFICIENT`, or whose `implementation_state` is `MISSING` (Appendix C.2). The keyword part of that mapping is a heuristic screen. [FORMALLY_DERIVED counts; INFERRED membership]

### 2.2 Constraint definitions and mechanical violation tests

#### HC-01 — One authority per semantic fact

**Statement.** Every semantic fact has exactly one authoritative declaration in the canonical encoding of every feature combination. Semantic facts include entry existence, LogicalPath, EntryKind, content bytes, each metadata item value, every size, offset and count, ChunkGroup membership, ReconstructionRegion extent, and a Chunk frame's stored length. Any other occurrence is either a non-authoritative cache that a conforming reader validates against the authority before use, or a *claim* that the reader checks against the authority. Declared budgets and descriptor summaries are claims. A hard link is content sharing plus the AUX metadata `posix.hardlink_group`, not a second structural record. [FORMALLY_DERIVED from SPEC §2.1(3), §3.2 E1, §3.3, §3.5, §4.5, §3.9 I1/I2/I10/I19/I23; CONTRIBUTING.md L82-83, L116-119]

**MVT-01.**
- (a) **Authority census.** For each record type and field in the ECF and EAM registries (`crates/entrybound/src/ecf`, `crates/entrybound/src/eam`, the `docs/*-v1.md` wire tables), assign a fact class and a role (AUTHORITY, CACHE or CLAIM); the classification is made by a session not involved in designing the candidate. The census is regenerated per candidate by the `ebr-pack` section walker extended with per-field provenance. Until that exists, it is a manual table committed under `research/audit/`. **Violation:** any fact class with two or more AUTHORITY fields in any valid feature combination, or a CACHE or CLAIM field that the reader is not shown to validate.
- (b) **Cache-disagreement mutants.** For every CACHE field (Index locators, EncryptedIndex entries, any descriptor restatement of a derivable fact), generate archives where cache and authority disagree. **Oracle:** the result reflects the authority, or is a named refusal such as `RebuiltInvalid` or `CORRUPT` with a stable code. **Violation:** any output value, listing, extraction or identity that reflects the cached value.
- (c) **L0.** A candidate adding a second declaration of an existing fact is excluded without measurement (with the §2.0 rule 3 counterexample and sign-off).
- (d) **Index reconstruction (I10).** For INDEXED archives of every candidate, delete the Index section, and separately overwrite it with zero bytes. **Oracle:** the decoded EAM and every entry digest equal those of the intact archive; the reader reports `EB_ECF_INDEX_ABSENT_REBUILT` for the deleted Index and `EB_ECF_INDEX_INVALID_REBUILT` for the zeroed one (`docs/format-v0.md` L470-475); and `repack --rebuild-index` yields an Index canonically equal to the original. **Violation:** any difference, or any other code.

**Instruments.** Production tests in `crates/entrybound/tests/native_ecf.rs` and `stream_layout.rs`; planned harness crate `research/harness/crates/ebr-pack`; `ebr` correctness-family metric `authority_violations` (must equal 0).

**Status.** Census completeness cannot be proven mechanically, because it depends on the fact-class taxonomy. [EXPERT_REVIEW_REQUIRED] REQ-MOD-0018 records an open tension with I1 for descriptor-level derivable values. [UNRESOLVED]

#### HC-02 — Exact logical losslessness

**Statement.** For every archive of role `Complete`, and every profile, layout, encryption mode, planner ID, chunker ID and feature, extraction writes, for every `File` entry, the exact octet sequence read at creation. For every `Symlink`, it writes the exact target octets. No lossy transform exists in any registry. Every reconstructive transform is round-trip verified at write time, with SHA-256 and byte equality and no opt-out. Content addressing over plaintext (`logical_digest`) verifies the path end to end. `Sidecar` and `Incremental` roles make no standalone extraction claim, and role is bound into LAI. [FORMALLY_DERIVED from SPEC §6.5, §8.5, §3.9 I14/I28/I31; CONTRIBUTING.md L89-98; REQ-INT-0052]

**MVT-02.**
- (a) **Round-trip matrix.** Covers every corpus item in the split under test, times profile {fast, balanced, dense, extreme}, times layout {indexed, stream}, times {unencrypted, recipient-encrypted, password-encrypted}, times candidate. Pack, fully verify and unpack into scratch. Compare each file's length and SHA-256 and each symlink's target bytes with the independent corpus fingerprint (`ebrc-fp-v1` under `research/corpus/fingerprints`). **Violation:** any mismatch. Implemented as the `ebr` correctness metric `roundtrip_ok` (binary).
- (b) **Fault injection.** In a research-only build (default-off feature, F-24), each reconstructive transform (deflate-reconstruct, jpeg-jxl-reconstruct and any candidate) returns a one-bit-wrong reconstruction. **Oracle:** the writer refuses with a stable reason code and emits no archive. **Violation:** an archive is written.
- (c) **Registry audit.** **Violation:** any registered codec or transform without an exact inverse, or any flag, profile or API that skips the write-time round trip.
- (d) **Known-hard inputs** are always included: F11 already-compressed, F12 JPEG including jpegli-produced files, F14 archive-in-archive, F20 adversarial. The SPEC cites a libjxl failure to recompress jpegli JPEGs and preflate degradation. [EXTERNALLY_SOURCED via SPEC §3.6 T2]
- (e) **Path coverage and mutation adequacy.** Recorded plans emit a per-path counter for every registered codec, transform and feature path: deflate and JPEG reconstruction, dictionaries, lookback groups, cross-entry dedup, sparse files, empty files, and symlink targets whose bytes are not valid UTF-8, among others. A zero-mismatch claim for a (profile, layout, encryption) cell requires every path registered for that cell to be exercised at least **5** times over the split under test; generated items are added where the corpus does not reach a path. A path exercised fewer than 5 times is HC_UNVERIFIED for that cell. The write-time round-trip comparison and verification code is mutation-tested with `cargo-mutants` at a pinned version. **Violation:** a mismatch, or a surviving mutant that a reviewer not involved in the code has not shown to be equivalent.

**Instruments.** `ebr run` with the planned `ebr-pack` binaries (`ebr-pack`, `ebr-unpack`, `ebr-verify`); corpus manifest `research/corpus/manifest.json`.

**Status.** Six mapped rows are weak (Appendix C.2). [UNRESOLVED]

#### HC-03 — Deterministic, unique native interpretation

**Statement.** Every byte sequence presented as an Entrybound archive has exactly one conforming outcome. That outcome is either one outcome class with one stable reason code, or `OK` with exactly one EAM: the entry set, per-entry path, kind, content bytes and metadata. Each model has one canonical serialization. Readers parse strictly and have no recovery mode; `salvage` is the only producer of output from non-conforming input, and its output is labeled unverified. Paths are unique under byte equality, and ancestors are explicit Directory entries. [FORMALLY_DERIVED from SPEC §1.2, §11.4, §3.4 P4/P6, §3.9 I15/I3/I6/I26, §20.5]

**MVT-03.**
- (a) **Canonical re-encode.** For every archive produced by any candidate and every valid conformance case, decode to EAM, re-encode with the same identifiers, and byte-compare (`ebound verify --canonical` where available). **Violation:** any difference.
- (b) **Differential interpretation.** Compare these readers:
  - the production reader;
  - the clean-room reader (`research/independent-reader`, HC-12);
  - the production INDEXED and STREAM paths for the same EAM.

  Run them over the conformance corpus and a structure-aware mutation corpus built with cargo-fuzz 0.13.2 on nightly-2026-08-01 in WSL. Mutations cover every structural byte of preamble, descriptor, section and record headers, and footer exhaustively for single-byte flips, plus 10,000 seeded payload positions per archive. Compare the tuple (outcome class, reason code, entry-set digest, per-entry content digest, per-entry metadata digest). **Violation:** any disagreement that the triage rule (§2.0 rule 8) does not attribute to the other reader. A disagreement caused by ambiguous specification text is an HC-03 violation of the *specification* until that text is fixed. **Detection:** the structural single-byte flips are exhaustive for that class; the 10,000 seeded payload positions per archive are regression evidence, with no bound claimed for adversarially chosen positions.
- (c) **Decided-class coverage.** Every normative rule has a conformance case with a decided outcome class, so the "UNRESOLVED" class is empty. **Violation:** a rule without a case, or a case without a decided class. [FORMALLY_DERIVED from REQ-ECO-0021]

**Instruments.** `ebr` correctness metrics; `research/conformance/` (empty at drafting); `research/independent-reader/` (empty at drafting).

**Status.** I15 anchor REQ-MOD-0110 is `INSUFFICIENT`. Both readers are missing. [UNRESOLVED]

#### HC-04 — Fail-closed unknown critical semantics

**Statement.** Before emitting any output for the affected scope, a reader refuses with `UNSUPPORTED` and a stable code when it meets any of the following:
- an unknown required-incompatibility feature bit;
- a set reserved bit or nonzero reserved field;
- a codec or transform identifier outside the compiled closed registry;
- an unknown record type in a critical position;
- an unknown Critical metadata item.

A corrupt feature bitmap, detected by its digest, is `CORRUPT`, not `UNSUPPORTED`. Unknown optional features are ignored but stay covered by authentication (AUX, signature bindings, AEAD, envelope MAC). Archive-controlled strings never select executable code. Feature bits and identifiers are never reused. [FORMALLY_DERIVED from SPEC §3.9 I11/I12/I21, §4.2, §10.3, §20.1-§20.3; REQ-CMP-0023, REQ-CMP-0161, REQ-CON-0032, REQ-CON-0096]

**MVT-04.** A generator enumerates, for each candidate format version:
1. every unassigned bit in each feature-bitmap tier;
2. each reserved field set to a nonzero value;
3. codec and transform IDs outside the registry;
4. an unknown Critical metadata name at entry scope and at archive scope;
5. an unknown record type in each section;
6. unknown optional records and stanzas;
7. feature-bitmap bytes altered with the bitmap digest left stale;
8. an archive-scope metadata item placed at entry scope, and an entry-scope item placed at archive scope (I11 "explicitly scoped");
9. a registered metadata item carrying a value of the wrong type (I11 "typed");
10. a Critical item in an unregistered namespace other than `x-` (I11 "namespaced").

**Oracles:** items 8 and 9 yield `NONCONFORMING` and item 10 yields `UNSUPPORTED`, each with a stable reason code and zero output. Items 1-5 yield `UNSUPPORTED` with a stable code, zero output bytes or filesystem objects, and peak RSS within the HC-06 pre-validation bound. Item 7 yields `CORRUPT`. Item 6 yields `OK`, but a single-byte change inside the optional item makes the AUX binding or authentication fail. **Violation:** any other outcome.

**Instruments.** Generators as format-building code in tests (F-18); `ebr` correctness metrics.

**Status.** REQ-PLT-0144 is `MISSING`: the archive-wide Critical-metadata option is unspecified. [UNRESOLVED]

#### HC-05 — Caller-owned extraction and security policy

**Statement.** No archive field grants, widens or selects extraction behavior. That covers destination paths, overwrite and collision handling, special files, setuid/setgid/sticky, ownership, budgets above policy, link following, confinement mode, and metadata restoration classes. The archive may only declare facts that let a caller refuse earlier. Extraction resolves one component at a time against held directory handles, using the platform containment primitive. It never touches metadata by path, never follows or traverses symlinks, and never falls back to string validation. The extractor reports any confinement weaker than kernel-enforced. Dangerous restorations are off by default. Unverified content never becomes a final object in the caller's destination. [FORMALLY_DERIVED from SPEC §11.6-§11.8, §3.9 I16; CONTRIBUTING.md L40-42, L137-140]

**MVT-05.**
- (a) **Policy monotonicity.** Fix policy P. For each archive A and each single-field mutation A′ (descriptor, feature bits, metadata items, fidelity, provenance, role, hostility summaries), record the operations of extracting A′ under P. In WSL and Docker the trace is `strace -f` with every path argument resolved by the kernel (`/proc/<pid>/fd` targets). On Windows, kernel-level tracing needs admin rights this program does not have, and an API-level log in a research build records what the extractor *requested*, not what the kernel *resolved*, so it is secondary. The primary Windows oracle uses observations independent of the extractor: (i) sentinel directories outside the extraction root, beside it and at each ancestor, carrying deny-write ACL canaries set by the harness user; (ii) a post-run tree diff (names, sizes, timestamps, SHA-256, reparse tags) of the root's parent and sibling directories; (iii) where available without admin, final-path checks (`GetFinalPathNameByHandle`) of every handle the extractor holds. **Violation:** an operation class that P disallows, an operation class that neither A under P nor P alone permits, a canary change, a sibling-tree difference, or a final path outside the root.
- (b) **Escape suite.** Cases: symlink-in-parent, symlink or junction swap races, absolute-target symlinks, case-fold collisions on NTFS, Windows reserved names, alternate data streams, and pre-existing destination objects. **Deterministic interleaving:** a filesystem shim in a research build (F-24) performs the swap (symlink, junction, rename or directory replacement) at every path-resolution step of the extractor in turn, for step index 0 to N−1 of an N-component path, so every race window is exercised once by construction. **Random races:** 10,000 trials per scenario with a concurrent attacker thread, labelled **regression evidence**: the rule-of-three figure (a per-trial escape probability of 3 × 10⁻⁴ would be detected with about 95% probability [FORMALLY_DERIVED]) assumes a fixed per-trial probability, and adaptive attackers widen race windows with oplocks and change notifications, so it is not a security bound. **Violation:** any object created, modified or opened outside the extraction root or through a link, in either mode. The environment capture records whether the session can create NTFS symbolic links (Developer Mode or `SeCreateSymbolicLinkPrivilege`); if it cannot, the NTFS symlink cases are HC_UNVERIFIED.
- (c) **API audit.** `ExtractionPolicy` has no constructor or mutator reachable from archive-parsed types (a type-level check in a compile-fail test). **Violation:** any such path.
- (d) **Staging.** Inject corruption into the last chunk of STREAM and encrypted archives. **Violation:** any destination object appears before the failure is reported (F-11).

**Instruments.** `crates/entrybound/tests/filesystem_workflow.rs` pattern; planned harness trace shim; Docker for Linux and WSL ext4.

**Status.** macOS `openat` resolve-beneath behavior is `PLATFORM_BLOCKED`. REQ-PLT-0110 (safe-by-default primary API) is `CONTRADICTED`. [UNRESOLVED]

#### HC-06 — Declared resource bounds

**Statement.** An archive declares its resource budget before payload in eight fields:
- entry count;
- total logical bytes;
- largest single entry;
- maximum expansion ratio;
- chunk count;
- path depth;
- metadata bytes;
- maximum key-derivation cost.

`DecodeRequirements` separately declares the decode window and working memory. The reader validates all of these against caller policy before allocating anything proportional to them. Alternatively, the archive declares `BudgetDeclared = false`, and the reader enforces policy alone and reports that it did. Every repeated structure has a declared, bounded count. If reality exceeds the declaration, decoding aborts. KDF cost above policy is refused before the KDF runs. Chunk-group lookback is declared and bounded. [FORMALLY_DERIVED from SPEC §11.5, §3.9 I17/I19/I20/I29; CONTRIBUTING.md L165-181]

**MVT-06.**
- (a) **Adversarial resource corpus.** A generated set of lying and hostile archives:
  - declared counts or sizes above policy;
  - declarations below reality (lying);
  - expansion-ratio bombs;
  - path depth and metadata-byte bombs;
  - long and transitive lookback chains;
  - KDF cost above the ceiling;
  - overflowing arithmetic fields;
  - STREAM with `BudgetDeclared = false` exceeding policy mid-stream;
  - structures whose element count differs from the count declared in the descriptor, in both directions (I20 "validated against the descriptor").

  **Primary oracle: an instrumented counting allocator** (research build, F-24, with MVT-16(c)) with allocation-site attribution: every allocation records its size, call site and whether its size derives from an untrusted field. **Baseline R0_max:** the maximum allocator peak over 20 runs of the same build opening a minimal valid archive. **Pre-validation bound H:** the largest structure the format permits the reader to buffer before the budget comparison (preamble, descriptor and its enclosing record), computed in bytes from the wire specification for each candidate before that candidate's first experiment record. If H is not computed, HC-06 is HC_UNVERIFIED for that candidate. **Oracles** (exact integer arithmetic, no band): refusal cases end with `POLICY_REFUSED`, or `CORRUPT`/`NONCONFORMING` for liars, with allocator peak ≤ R0_max + H; accepted cases have allocator peak ≤ R0_max + B_memory and scratch bytes written ≤ B_scratch (write accounting, `decision-method.md` §4.14); for KDF over the ceiling, an instrumented counter shows zero Argon2id invocations. **Violation:** any allocation sized from an untrusted field before that field's budget check, any exceedance, any disagreement between declared and actual element count that is not refused, or any KDF invocation above the ceiling. **Secondary corroboration:** process peak RSS (Linux) and job peak commit (Windows) are recorded and reported, never used as the oracle. **Detection:** the adversarial set is a pre-registered enumeration; no completeness over all hostile inputs is claimed (regression evidence beyond the enumeration).
- (b) **Declaration consistency.** For every valid archive a candidate produces, reader-derived aggregates equal the declared requirements, and actuals stay within the upper bounds (REQ-MOD-0017). **Violation:** any mismatch.
- (c) **Parse-site audit.** Every length or count read from untrusted bytes flows through checked arithmetic and a bound check before use. A static audit is committed with each candidate. [EXPERT_REVIEW_REQUIRED for completeness]
- (d) **Algorithmic-complexity bounds on hostile input.** For each structure a reader or writer processes (entry lists, path components with case-fold and NFC collision maps, metadata items and names, sparse-map records, Merkle and range-verification recomputation per range read, chunk groups, legacy central directories), a generator builds an adversarial scaling series of 2^10, 2^11, ..., 2^20 elements (colliding hashes, colliding folded names, maximal nesting, pathological metadata). CPU time is measured from in-process counters (harness crates) or process accounting, 3 runs per point. The least-squares slope of ln(CPU time) on ln(element count) is fitted with a 0.99 confidence interval. **Violation:** the interval's lower bound exceeds 1 + δ with δ = 0.15, or CPU time per input byte at 2^20 elements exceeds the candidate's declared per-byte bound. The exponents are reported under OD-17 M17.5. **Detection:** limited to the enumerated structures and generators (regression evidence beyond them).

**Instruments.** `ebr` `memory_peak` (Linux `/usr/bin/time -v`; Windows Job Object plus psutil), `scratch_peak` poller; planned `ebr-access` streaming memory probe.

**Status.** H is not yet computed for any candidate, and the counting allocator is not built, so HC-06 is HC_UNVERIFIED for every candidate until both exist. REQ-CON-0004 (I20) and REQ-CON-0005 (I17) are `INSUFFICIENT`. [UNRESOLVED]

#### HC-07 — No silent metadata or semantic loss

**Statement.** Every gap is recorded as typed data:
- capture gaps in the in-band FidelityReport, bound by AUX;
- restoration gaps in the ExtractionReport;
- legacy-import resolutions in the ConversionRecord, which affects AUX only;
- export downgrades in the ExportReceipt as `LOSSLESS`, `LOSSY` (explicit approval required) or `REFUSED` (no override).

Legacy ambiguity is surfaced and never normalized: strict import refuses `Divergence` and `Irreconcilable` evidence. Entries are never silently skipped. Metadata is typed, namespaced and scoped. [FORMALLY_DERIVED from SPEC §2.1(5), §3.9 I11/I13/I14/I18, §10.9, §14.2, §15.1; CONTRIBUTING.md L44-65, L185-247]

**MVT-07.**
- (a) **Set-difference fidelity oracle.** For each metadata class, source filesystem and target filesystem:
  - Classes: timestamps with precision, mode bits, numeric and named ownership, xattrs, POSIX1E and NFS4 ACLs, Windows security descriptors, attributes and alternate data streams, reparse points, symlinks, hardlink topology, sparse layout, empty directories, zone markers.
  - Filesystems: NTFS on the host, ext4 in WSL, and overlay in Docker. APFS is `PLATFORM_BLOCKED`.

  Observe the source with tools independent of Entrybound's capture code: `stat`, `getfattr`, `getfacl`, `lsattr`, `filefrag` on Linux; `Get-Acl`, `Get-Item -Stream`, `fsutil reparsepoint query` (non-admin subset) on Windows. Pack, extract, and observe the destination the same way. **Violation:** the symmetric difference between source and destination observations contains any item not declared in the FidelityReport or ExtractionReport.
- (b) **Legacy evidence accounting.** For every import of a ZIP, tar, compressed tar or 7z input (research corpus F14 plus generated Research III ambiguity classes), each LOM conflict in the evidence either has a resolution recorded in the ConversionRecord or the import refused. **Violation:** a strict-policy import that resolves a `Divergence` or `Irreconcilable` conflict, or a source entry count that does not equal output entries plus recorded omissions.
- (c) **Export accounting.** **Violations:** a `LOSSLESS` export whose strict re-import LAI differs from the source LAI; a `LOSSY` export that does not enumerate every degraded item; LOSSY output written without approval.
- (d) **Input consistency during pack (SPEC §5.7a).** A seeded mutator process rewrites, truncates, extends, swaps (rename over) and touches the mtime of source files while `pack` runs, at seeded delays, over generated trees on NTFS and ext4. **Oracle:** each packed entry's content, size and metadata equal one coherent version of the source file taken from a single descriptor state (the mutator logs every version with its SHA-256, size and metadata); otherwise the entry is excluded with a report, or it carries a `fidelity.unstable_source` record naming it. **Violation:** a mixed-version entry without that declaration. A static audit of the capture code, committed with each candidate, finds no re-`stat` or re-open by path after the file's descriptor is open (SPEC §5.7a item 1). **Detection:** regression evidence for the seeded schedules; the static audit covers the by-path class completely. Retry cost is reported as OD-06 M06.5. [FORMALLY_DERIVED contract from SPEC §5.7a; INFERRED test]

**Instruments.** `ebr` specs with `correctness` and `other` metrics; `tools/tar-strict` and `tools/zip-compat` matrices (reference runtimes stay outside production, F-20).

**Status.** REQ-PLT-0079 (I13) and REQ-LEG-0056 (I18) are `CONTRADICTED`. [UNRESOLVED]

#### HC-08 — Archive semantics independent of host filesystem interpretation

**Statement.** An archive's meaning (outcome class, EAM, LAI, AUX, PCR) is a function of its bytes and the caller's keys only. It does not depend on host OS, filesystem case sensitivity, Unicode normalization, path-separator conventions, locale, time zone or file extensions. Paths are component sequences with exact bytes and declared encodings. Bytes hostile to some target are legal and handled by extractor policy with a hostility summary. Collisions are detected, never normalized. Content categorization never uses extensions. [FORMALLY_DERIVED from SPEC §3.4 P2/P3/P5/P7/P8, §10.8, §3.9 I4/I5/I27; REQ-CMP-0038]

**MVT-08.**
- (a) **Cross-host open.** Open the same archive set (conformance corpus plus packed tuning and validation items) on the Windows host, WSL2 ext4, Docker linux/amd64 and QEMU linux/arm64. **Violation:** any difference in (outcome class, reason code, LAI, AUX, PCR, entry-list digest, per-entry content digest).
- (b) **Hostility corpus.** Generated archives containing case-fold pairs, NFC/NFD pairs, Windows reserved names, trailing dot or space, `0x5C` and `:` bytes, control bytes, and UTF-16LE `2E 00 2E 00`. **Oracles:** the archive-level interpretation is identical on all hosts, and extraction on each target either succeeds faithfully where representable or refuses or renames under the declared policy with a report entry. **Violation:** silent merge, overwrite, normalization or skip.
- (c) **Capture equivalence.** Pack the same seeded generated tree on Windows and WSL. **Violation:** an LAI difference not explained item by item by FidelityReport entries.

**Instruments.** `ebr` platform matrix; Docker `--platform linux/arm64` (emulated).

**Status.** macOS/APFS normalization behavior is `PLATFORM_BLOCKED`. [UNRESOLVED]

#### HC-09 — Bounded dependency chains

**Statement.** Decoding any chunk requires only a declared, bounded, acyclic, backward-only dependency closure within the archive:
- its TransformPlan;
- a referenced Dictionary;
- its ReconstructionRegion and ReconstructionData;
- at most `max_lookback` preceding members of its ChunkGroup.

STREAM archives have no forward references and stay within the declared dedup window. A `Complete` archive needs no external artifact, network service, key server or live infrastructure to decode or verify. `Incremental` and `Sidecar` dependencies are declared by role. Reported access cost is an upper bound on actual decode work. [FORMALLY_DERIVED from SPEC §3.5 C1, §3.9 I29/I30, §4.9, §7.5, §24 item 7; REQ-INT-0010, REQ-CRY-0247, REQ-ACC-0161]

**MVT-09.**
- (a) **Closure computation.** From the recorded plans, groups, dictionaries and regions of every archive a candidate produces, compute each chunk's transitive closure. **Violation:** a cycle, a forward reference in STREAM, a reference beyond the declared window, a closure larger in chunks or decoded bytes than the declared access cost (`max_access_granularity` and per-region worst bytes), or any reference outside the archive for role `Complete`.
- (b) **Measured closure.** `ebr-access` random entry and range reads over a seeded sample record decoded chunk count and decoded bytes per read. **Violation:** measured exceeds declared.
- (c) **Offline decode.** Run `unpack` and `verify` for `Complete` archives, including signature verification with caller-supplied trust anchors, inside Docker with `--network none`. **Violation:** any failure attributable to network absence.

**Instruments.** Planned `ebr-access`, `ebr-planner` (closure traces); `ebr` correctness metrics.

**Status.** REQ-ACC-0001 is `CONTRADICTED`: the declared group cost understates transitive decode. [UNRESOLVED]

#### HC-10 — Canonical identities

**Statement.**
- **Descriptor digests.** LAI, PCR and AUX are digests over descriptors that bind the digest-algorithm identifier and parameters, never bare Merkle roots.
- **Entry identity.** Each Entry carries an explicit `identity_digest` and `aux_digest`.
- **LAI** is independent of compression, codec parameters, transforms, dictionaries, chunk boundaries, physical order, layout, Index presence, encryption, padding, recipients and signatures. **PCR** depends on chunking by design.
- **Metadata coverage.** Every metadata byte is under exactly one of LAI or AUX. Participation is fixed by the registry and identity profile, never by the writer, and `x-` names never participate.
- **Merkle construction.** Leaves and nodes are domain-separated, there is no padding, the split is at the largest power of two, and length is bound.
- **PCI** is over exact container bytes and is never embedded.
- **Layout equivalence.** INDEXED and STREAM encodings of one EAM agree on every identity except PCI.

[FORMALLY_DERIVED from SPEC §8.0-§8.6, §3.9 I7/I8/I9/I22/I28; CONTRIBUTING.md L104-114, L151-156]

**MVT-10.**
- (a) **Operation matrix.** Apply each SPEC §8.1 operation to each test archive, plain and signed:
  - recompress with the same chunking;
  - `repack --layout`;
  - `repack --rebuild-index`;
  - `repack --profile`;
  - explicit re-chunk;
  - `--strip-provenance`;
  - re-encrypt, `key add` and `key remove`;
  - non-identity metadata change;
  - `repack --add`;
  - content change.

  **Violation:** any cell of the observed (LAI, PCR, AUX, PCI) change pattern or the signature binding states (CONTENT, PHYSICAL, ADDRESSING), reported as `VALID` or `STALE`, differs from the SPEC §8.1 table.
- (b) **Permanent vectors.** Descriptor and Merkle vectors cover leaf counts 0 to 65, a leaf-versus-interior collision attempt, and every field-decomposition ambiguity attempt. Production and the independent reader compute them. **Violation:** any mismatch with the committed vectors.
- (c) **Participation audit.** For each registered metadata name, change its value. **Violation:** a change to both LAI and AUX, to neither, to the wrong one per the registry and profile, or any `x-` name affecting LAI.
- (d) **Cross-producer.** Pack the same tree under every planner ID and profile, then compare after `--strip-provenance`, so that creation provenance (planner and profile identifiers) which SPEC §8.1 and F-19 may bind into AUX does not cause false violations. Where the SPEC §8.1 table places that provenance outside AUX, the comparison is also run without stripping, and the MVT record cites the table cell. **Violation:** unequal LAI, or unequal AUX after stripping.
- (e) **Entry identity mutation (I7).** Mutate `identity_digest` and, separately, `aux_digest` of single entries. **Violation:** any outcome other than `CORRUPT` with a stable reason code.
- (f) **Bare roots (I22).** Construct a signature or binding over a bare Merkle root, and archives whose descriptor fields are swapped between roots. **Violation:** any such binding or archive accepted as `VALID` or `OK`.

**Instruments.** `ebound inspect --json`; `crates/entrybound/tests/stream_layout.rs` equivalence assertions; planned `ebr-pack`.

**Status.** Twenty-eight mapped rows are weak. [UNRESOLVED]

#### HC-11 — No runtime-dependent decoder interpretation

**Statement.** Decoded bytes and outcome depend only on archive bytes, caller policy and keys. They do not depend on thread count, core type, CPU feature detection, OS, locale, time zone, clock, available memory above the declared requirement, build profile, planner ID or creation profile. Decoders implement the closed codec and transform registries and never invoke the planner, chunker or similarity analysis. A decode step whose output is defined by a pinned library version rather than a format definition is **admitted only as a gated feature**: it sits outside the baseline set (SPEC §5.8) behind an `incompat` feature so that readers without it fail closed; the registry declares that no public specification exists; the version identity is part of the recorded plan's registry identifier; its output is verified by digest before release; and it carries permanent vectors (HC-12). Such a step is not "decoding depends on library version" in the sense of `decision-method.md` R2, because the version is recorded data and the output is digest-checked. It is charged the highest longevity cost (tier T4, `decision-method.md` §7.2) and is never eligible for the baseline set. [FORMALLY_DERIVED from SPEC §3.6 T1, §3.9 I15/I30, §5.8, §12.2; CONTRIBUTING.md L67-71, L94-98; REQ-CMP-0044; INFERRED admission rule]

**MVT-11.**
- (a) **Decode variation matrix.** Decode each archive under:
  - threads {1, 2, 8, 32} (correctness variation, not timing; any affinity);
  - affinity on P-cores only (Windows logical 0-15) and on E-cores only (16-31);
  - locale {C, tr_TR.UTF-8, ja_JP.UTF-8} and time zones {UTC, Pacific/Chatham}, set per process in WSL and Docker;
  - CPU features: QEMU user-mode linux/amd64 with a `-cpu` model lacking AVX2, and QEMU linux/arm64 under at least two `-cpu` models;
  - Windows, WSL, Docker and QEMU;
  - Docker memory limit = R0_max + declared decode requirement + a page-cache allowance of 64 MiB, and 2× and 8× that value.

  **Variation is verified to take effect.** Each run records `locale -a` and runs a locale canary (Turkish dotted-i case mapping must differ between `C` and `tr_TR.UTF-8`); a cell whose canary shows no difference is HC_UNVERIFIED, not a pass. The CPU model in effect is recorded from `/proc/cpuinfo` flags inside QEMU. Windows system locale and time zone are settings changes this program may not make, so Windows locale and time-zone variation is HC_UNVERIFIED unless the variation is per process. Out-of-memory kills are classified separately as `oom_killed` and are neither passes nor outcome differences; a cell with an OOM kill is re-run at the next memory multiple. **Violation:** any difference in (outcome class, reason code, output digests) among completed runs.
- (b) **Alternate decoder.** Where a format-defined decoder exists for a reconstructive or codec step (for example a second Zstandard implementation, or libjxl JPEG reconstruction per ISO/IEC 18181-1 [EXTERNALLY_SOURCED]), decode with it as well. **Violation:** a difference not already classified as a bug in that alternate decoder.
- (c) **Static.** **Violation:** the decode call graph (`cargo tree -e normal` on the decode feature set, plus symbol inspection) reaches planner, chunker or similarity code. [EXPERT_REVIEW_REQUIRED for completeness]

**Status.** `deflate-reconstruct` (planner v5, preflate class) and the v6 `jixel`/`jxl-oxide` JPEG reconstruction appear to be defined by their implementations rather than by public specifications. They are admitted under the gated-feature rule above. If a production default path (the baseline set) reaches either one, that is a defect under §2.0 rule 5. [INFERRED; UNRESOLVED]

#### HC-12 — Independently implementable native specification

**Statement.** A competent engineer can implement a conforming decoder for the baseline feature and codec set from the specification, registries and permanent vectors alone, without Entrybound source code. Every normative rule has at least one conformance case with a decided outcome class and reason code. Every codec or transform needed for baseline decoding has a public specification or permanent vectors. Gated features that lack a public specification declare that in the registry. [FORMALLY_DERIVED from SPEC §19.2-§19.3, §20.6; REQ-ECO-0030, REQ-ECO-0021; CONTRIBUTING.md L142-149, L182-183]

**MVT-12.**
- (a) **Clean-room reader.** `research/independent-reader` is built from specification text and vectors under a provenance log. It is developed in a checkout that excludes `crates/` and `tools/`, and every consulted file is logged. It must pass the baseline conformance corpus at 100%, with identical outcome class and reason code. **Violation:** any failure not attributed to the independent reader under the triage rule (§2.0 rule 8). A failure traced to missing or ambiguous specification text is a violation until the specification is fixed.
- (b) **Traceability.** The SPEC carries normative force mostly in lowercase "must", "never", "is prohibited", "only" and normative tables, not in RFC 2119 keywords (review finding B08). A script extracts normative statements under this **grammar**: sentences containing, case-insensitively, `must`, `must not`, `shall`, `required`, `never`, `always`, `prohibited`, `may not`, `only` used as a restriction, or `is rejected`/`is refused`; plus every row of a table the specification marks normative. Each statement maps to at least one case ID. **Extraction recall** is validated before HC-12 can pass: a seeded random sample of 100 sentences from the specification is labelled normative or not by a session not involved in the specification, the script or either reader, and the script's recall on the sentences labelled normative must be at least 0.95. If recall is lower, the specification must be rewritten with RFC 2119 keywords before HC-12 can pass. **Violation:** an unmapped statement.
- (c) **Codec provenance.** **Violation:** a baseline decode step whose only definition is library behavior. Gated decode steps admitted under HC-11 must declare the missing public specification in the registry and carry permanent vectors; a gated step without both is a violation.

**Status.** REQ-ECO-0030 and REQ-ECO-0021 are `MISSING`. An independent reader produced inside the same program is not organizationally independent, so final assurance needs a genuinely external implementer or reviewer. [UNRESOLVED; EXPERT_REVIEW_REQUIRED]

#### HC-13 — Reproducible deterministic modes

**Statement.**
- **Logical determinism** always holds: the same input tree and identity profile give the same LAI.
- **Planner determinism** holds in deterministic mode: the same input, planner ID and declared resource model give the same plan and chunk boundaries. Thread count, core count, available memory, locale, traversal order and wall-clock time do not affect it.
- **Physical determinism** holds for unencrypted archives under `--deterministic`: the same input, planner ID, format version and writer version give a byte-identical container.
- **Encryption** is intentionally non-deterministic, and no deterministic-ciphertext mode exists.
- **Frozen IDs** keep their planner and chunker outputs indefinitely.
- **Legacy export** bytes are a deterministic function of EAM plus target profile.

[FORMALLY_DERIVED from SPEC §12.1-§12.3, §5.7; CONTRIBUTING.md L67-81, L223-241; REQ-CRY-0128, REQ-LEG-0038]

**MVT-13.**
- (a) **Repeat-hash.** The determinism input is the pair (EAM, identity profile) captured under the SPEC §12.1 deterministic-mode rules: timestamps and ownership are fixed by those rules, so recreated trees differ only in creation order. For each tuning and validation item, profile and layout, pack with `--deterministic` and no encryption at least 3 times per variation cell: threads {1, 8, 32}; source tree recreated with shuffled creation order; locale and time zone varied per process; Docker memory limit varied. Each host class (Windows NTFS, WSL ext4, Docker overlay, QEMU arm64) records the metadata classes it captures. **Stress mode:** for every determinism claim, one cell additionally runs 20 repetitions at the host's maximum thread count with seeded background CPU load. **Violations:** within one host class, any PCI or LAI difference; across host classes, any LAI difference, or any AUX difference not mechanically explained, item by item, by FidelityReport entries for metadata classes that host does not capture. Any two differing outputs are the violation artifact (§2.0 rule 2). **Detection:** 20 stress repetitions detect a per-run mismatch probability of 0.139 or more with 95% probability (`decision-method.md` Appendix D.5); rarer interleavings are regression evidence.
- (b) **Encryption.** **Violations:** unequal LAI or AUX across two encrypted packs of the same input, or identical ciphertext bytes across them.
- (c) **Frozen-ID goldens.** For each frozen planner ID (v1, v3-v6) and `gear-norm-v1`, a committed seeded generator plus expected plan, boundary and PCI digests. **Violation:** the current build differs.
- (d) **Export.** **Violation:** a legacy artifact whose bytes differ when the same EAM is sourced from different layouts, profiles or encryption modes.

**Instruments.** `ebr` `size`/`correctness` families with repeat-hash; planned `EXP-DET-SMOKE` (Phase B2).

**Status.** REQ-CON-0055 (INDEXED/STREAM frame byte identity) is `CONTRADICTED`. [UNRESOLVED]

#### HC-14 — Crypto correctness never traded for performance

**Statement.** No candidate may weaken any of the following to gain size, speed, memory, compatibility or usability:
- confidentiality or integrity;
- authenticity or key commitment;
- nonce and key uniqueness, or domain separation;
- constant-time secret comparison;
- the hybrid post-quantum recipient policy;
- KDF cost enforcement or secret hygiene;
- the unlock and verification order;
- the rule that no plaintext is released before authentication.

No mode is encrypted but unauthenticated, and no mode produces deterministic ciphertext. Chunk boundaries in encrypted archives are derived under a key-domain secret, so plaintext structure leaks only as the declared, quantized residual. **I24 as a hard constraint is exactly three mechanically checkable facts:** (1) boundaries in encrypted archives are keyed (MVT-14(e)); (2) the padding mode (`--pad none|buckets|max`, SPEC CLI) is recorded in the archive, and `inspect` and `verify` report `pad none` as a declared leak (SPEC §23.4: "Opt-out exists and is recorded as a declared leak"); (3) every public length an observer sees conforms to the length function of the declared padding mode (MVT-14(j)). SPEC §7.6 and §9.8 describe the default bucketed padding as quantised, not hidden, so the *effectiveness* of padding against length and boundary attacks is not an HC: it is graded per padding mode under OD-16 and routed to external review (SPEC §25.2). No statistical test is part of this HC. The implementation uses pinned, vetted primitive crates, implements no primitive locally, and offers no user-selectable algorithms. [FORMALLY_DERIVED from SPEC §9.1, §9.4-§9.7, §12.3, §3.9 I9/I24/I25; CONTRIBUTING.md L142-181; REQ-CRY-0013, REQ-CRY-0097, REQ-CRY-0124, REQ-CRY-0243]

**MVT-14.**
- (a) **Vector gate.** RFC, draft-10, V1-V7 and RFC 8032 vectors, plus RFC 8452 vectors once added (REQ-CRY-0085), run in `crates/entrybound/tests/crypto_primitives.rs` and `crypto_v1.rs`. **Violation:** any failure.
- (b) **Tamper before release.** Apply every single-bit flip in public framing, headers and stanzas, and 10,000 seeded flips in payload segments. Also apply truncation at each record boundary, record reordering, cross-archive record splicing, and stanza insertion or removal. **Violation:** any plaintext byte reaching the caller or the filesystem, or any outcome other than an authentication-class failure. **Detection:** exhaustive for framing, header and stanza bit flips and for the enumerated structural operations; the payload flips are regression evidence (AEAD forgery resistance is a property of the primitive, covered by (a) and by external review).
- (c) **Uniqueness.** Across 10,000 archive creations, including key add and remove sequences, log (derived key identifier, nonce) pairs from a research-only instrumented build (F-24, with MVT-16(c)). **Violation:** any repeated pair. **Detection:** an implementation defect that repeats a pair with probability 3 × 10⁻⁴ or more per creation is detected with about 95% probability (rule of three); random-nonce collision bounds are a property of the construction, not of this test.
- (d) **L0 rule.** A candidate that edits an item frozen by F-12, F-13 or F-14, removes or reorders an unlock step, adds a primitive not in the pinned set, or exposes an algorithm choice is excluded without measurement and routed to `EXTERNAL_REVIEW_REQUIRED`.
- (e) **Keyed boundaries.** For each tuning item of at least 1 MiB, chunk the same plaintext under 1,000 seeded pairs of distinct file keys, and under each key versus the unkeyed `gear-norm-v1` chunker. **Chance rate:** the null distribution of the boundary-coincidence fraction (shared boundary offsets divided by the mean boundary count) is the empirical distribution over the 1,000 distinct-key pairs on that item. **Violation:** identical boundary sets under two distinct keys, or a keyed-versus-unkeyed coincidence fraction above the 0.999 quantile of that item's null distribution for more than 1 key in 1,000. **Detection:** a keying defect that makes boundaries unkeyed-equal for 0.3% or more of keys is detected with about 95% probability over 1,000 keys (rule of three).
- (f) **Key commitment.** Vectors and constructed archives in which one ciphertext authenticates under two different keys or recipients. **Violation:** any such archive releasing plaintext under either key instead of being rejected.
- (g) **Secret hygiene.** A compile-fail audit (`trybuild`) that secret-bearing types implement none of `Debug`, `Display` or implicit `Clone`, plus a research-build test (F-24, with MVT-16(c)) that zeroize-on-drop clears their memory. **Violation:** any such implementation, or non-zero secret bytes after drop.
- (h) **Constant-time comparison.** A code audit that every secret comparison uses the pinned constant-time primitive, plus a dudect-style leakage test (Welch t-statistic over 10^6 timed comparisons of fixed versus random inputs, threshold |t| > 10). **Violation:** the audit finds a non-constant-time secret comparison. A comparison that fails the timing test on this noisy laptop is HC_UNVERIFIED, not a violation, and is routed to external review.
- (i) **Downgrade.** Strip signature records, encryption markers or feature bits, and recompute every unkeyed digest so that the stripped archive is internally consistent. **Violation:** a reported verification or authentication state stronger than the oracle state of the stripped archive (HC-15).
- (j) **Declared padding mode.** For every encrypted archive a candidate produces, read the recorded padding mode and compute every observable object length from the public framing alone. **Violation:** a missing padding-mode record; `inspect` or `verify` not reporting `pad none` as a declared leak; or an observed public length that the declared mode's length function does not permit.

**Status.** Correctness of the constructions, as opposed to conformance to them, cannot be established internally. SPEC §25.2 and `docs/crypto-review-v1.md` require dedicated external review. [EXPERT_REVIEW_REQUIRED; UNRESOLVED]

#### HC-15 — Verification honesty and distinguishable failure

**Statement.** A result not verified against its applicable authority is reported as unverified, and `UNVERIFIED` qualifies `OK` rather than looking like verified output. Applicable authorities are the chunk digest, a Merkle slice rooted in a signed PCR, whole-archive verification, and AEAD. Truncation, corruption, unsupported features and policy refusal stay distinguishable at every layer from record decoding to CLI exit status. Random access reports `whole_archive_verified = false` unless a full reader consumed the container. PCR and PCI are never claimed from unread bytes. `salvage` alone produces output from non-conforming input and marks it unverified. Repaired data is re-verified before it is trusted. [FORMALLY_DERIVED from SPEC §3.9 I21/I25, §11.4, §20.5; CONTRIBUTING.md L110-114, L128-140; REQ-INT-0051, REQ-INT-0054, REQ-INT-0065]

**MVT-15.**
- (a) **Verification-state oracle.** Covers each read API and CLI command (list, inspect, read_entry, range read, verify, unpack, diff, salvage), each source (Memory, LocalFile, HttpRange via `ebr-origin`, STREAM pipe), each archive type (unsigned, signed, encrypted with key, encrypted without key), and each injected fault (none, fault in requested chunk, fault in unrequested chunk, Index damage, footer truncation). An oracle computes the strongest state the bytes actually verified support. In the black-box variant, states are known by construction; a research-internals build (F-24) adds a byte-verification log. **Violation:** a reported state stronger than the oracle state.
- (b) **Fault-class confusion matrix.**
  - Truncation at every section boundary and at 1,000 seeded offsets must give `TRUNCATED`.
  - Digest-detected damage must give `CORRUPT`.
  - Unknown features must give `UNSUPPORTED`.
  - Rule breaks must give `NONCONFORMING`.
  - Over-budget archives must give `POLICY_REFUSED`.

  **Precedence table for ambiguous injections**, derived from the class meanings of SPEC §20.5 ("TRUNCATED: well-formed prefix; declared length exceeds actual"; "CORRUPT: structure or digest mismatch") and pre-registered here:

  | Injection | Expected class |
  |---|---|
  | Bytes removed from the end; every present byte is a well-formed prefix of the original | `TRUNCATED` |
  | A length or offset field changed to point past end of file, where a digest or structural check available from the present bytes covers the field | `CORRUPT` |
  | The same, where no check available from the present bytes covers the field | `TRUNCATED` (indistinguishable from real truncation by construction; counted on the diagonal) |
  | Footer or end record damaged in place, file length unchanged | `CORRUPT` |
  | Footer removed by truncation | `TRUNCATED` |
  | Unknown feature bit with a valid bitmap digest | `UNSUPPORTED` |
  | Feature bitmap changed with a stale digest | `CORRUPT` |
  | Digest-valid bytes that break a normative rule | `NONCONFORMING` |
  | Conforming archive whose declared budget exceeds policy | `POLICY_REFUSED` |
  | Several injections at once | the class of the injection the reader reaches first in the specification's read order |

  **Violation:** any cell off the diagonal defined by this table, in the library result or the CLI exit status. **Detection:** exhaustive for section-boundary truncation; the 1,000 seeded offsets are regression evidence (a class-confusion defect affecting 0.3% or more of offsets is detected with about 95% probability).

**Status.** REQ-INT-0025 (I25) is `INSUFFICIENT`, and REQ-INT-0065 and REQ-INT-0057 are `MISSING`. [UNRESOLVED]

#### HC-16 — Published identifiers are immutable

**Statement.** Behavior bound to a published identifier never changes in place. Identifiers include:
- format version, feature bit, record type and version;
- Descriptor version;
- planner ID, chunker ID and layout ID;
- identity profile;
- crypto suite and wire version;
- signature record;
- legacy target profile, compatibility profile and ExportReceipt version;
- reason code.

Improvements ship as new identifiers, and retired identifiers are never reused. Historical archives remain decodable with unchanged interpretation and identities. This holds before v1 as well: the README's "not stable" status does not override `CONTRIBUTING.md` or this HC inside the program. A writer may stop *emitting* an identifier; dropping *decode* support for it is outside this program's authority and is escalated under §4.5. [FORMALLY_DERIVED from SPEC §12.2, §20.3-§20.4; CONTRIBUTING.md L44-54, L67-103, L142-173, L194-203]

**MVT-16.**
- (a) **Golden regression.** Every permanent vector file (`docs/descriptor-vectors-v1.txt`, `docs/posix-metadata-v1-vectors.txt`, `docs/security-metadata-v1-vectors.txt`, `docs/crypto-wire-v1-vectors.txt`) and a committed generator-plus-digest set per frozen identifier are re-decoded and re-produced by the candidate build. **Violation:** any byte or identity difference.
- (b) **Allocation audit.** A script over the registries in source and docs. **Violation:** an identifier value assigned twice with different meaning, or a candidate patch that changes frozen-ID behavior without allocating a new identifier.
- (c) **Research-build equivalence.** Every research build used as an HC oracle (F-24) is built with its research features off and compared with the production build at the same commit over the conformance corpus plus every tuning item. **Violation:** any artifact byte difference or any outcome difference.
- (d) **Frozen-ID differential regression.** For every frozen planner ID (v1, v3-v6) and chunker ID (`gear-norm-v1`), run the baseline binary built at `9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155` and the candidate binary over every tuning and validation item and the conformance corpus. **Violation:** any difference in plan, chunk boundaries or container bytes.

**Status.** Twelve mapped rows are weak. [UNRESOLVED]

#### HC-17 — Structural path and entry hazards are unrepresentable

**Statement.** EAM and ECF have no encoding for any of the following:
- a `.` or `..` component under any declared encoding;
- an absolute or rooted path;
- a separator inside a component;
- an empty component;
- duplicate LogicalPaths;
- a non-Directory proper prefix, or a missing ancestor;
- a Directory carrying content;
- kind inferred from absence;
- a hardlink kind with ordering or target-resolution hazards.

Legacy importers refuse these under strict policy rather than normalizing them, and compatibility profiles never bypass that. [FORMALLY_DERIVED from SPEC §3.3, §3.4 P1-P7, §11.2, §3.9 I3/I4/I5/I6/I23/I26/I27; REQ-MOD-0033, REQ-LEG-0183, REQ-LEG-0225]

**MVT-17.**
- (a) **Constructor and encoder property tests.** Attempt each hazard through every public constructor and builder, and through raw record injection into the ECF encoder. **Violation:** a successfully encoded archive containing the hazard.
- (b) **Decoder.** Hand-built ECF bytes carrying each hazard, including UTF-16LE `2E 00 2E 00` and file-as-ancestor. **Violation:** any outcome other than `NONCONFORMING` with the expected stable code, in either the production or the independent reader.
- (c) **Legacy.** ZIP, tar, pax, GNU tar and 7z inputs carrying each hazard, under strict and every compatibility and preservation profile. **Violation:** any import that succeeds with the hazard normalized away and unrecorded, or any compatibility profile that admits it (F-20).
- (d) **Hardlink inference (I23, F-17).** Pack trees containing files with equal content on distinct inodes, and restore them. **Violation:** a hardlink group recorded for them, or a restoration that links them.

**Status.** Nine mapped rows are weak. [UNRESOLVED]

#### HC-18 — Crash-consistent output and safe remote origins

**Statement.** An interrupted write never leaves a destination object that verifies as `OK`, never leaves a partial multi-target publish, and never overwrites an existing destination object that the caller did not authorize to be replaced (exclusive create). A remote reader refuses origin misbehaviour instead of reading it as verified data: a weak or missing ETag, a 200 whole-body fallback, a revision change mid-session, and a short or mismatched 206 response each yield a stable refusal code. [FORMALLY_DERIVED from CONTRIBUTING.md L128-140 (F-10, F-11) and L223-247 (F-22: multi-target publish is all-or-nothing with rollback); `docs/http-range-access-v1.md`; ledger constraint "CLI exclusive-create no-overwrite"; SPEC §3.9 I16/I21/I25]

**MVT-18.**
- (a) **Kill at seeded points.** Run `pack`, `repack`, `export` and multi-target `publish` under a harness that sends an uncatchable kill (SIGKILL; `TerminateJobObject` on Windows) at 200 seeded points per operation, including every write-phase boundary the research build reports. **Oracles:** no destination file exists that `verify` reports as `OK`; no publish target set is partially present; every pre-existing destination object is byte-identical to its state before the run. **Violation:** any of these. **Detection:** exhaustive over reported phase boundaries; seeded interior points are regression evidence.
- (b) **Origin misbehaviour suite.** `ebr-origin` serves each misbehaviour above, plus an ETag that changes between requests and a `Content-Range` that disagrees with the body. **Violation:** any read that returns bytes as verified, or any outcome other than the stable refusal code for that case.

**Instruments.** Planned harness kill injector; `ebr-origin`; `ebr` correctness metric `origin_protocol_safety` (M12.4).

**Status.** Instruments not built. [UNRESOLVED]

---

## 3. Optimization dimensions

### 3.0 Common measurement framework

**Instruments.**

| Instrument | Location and state at drafting | Role | Tag |
|---|---|---|---|
| `ebr` runner and statistics | `research/tools/ebr` (committed `5eabf03`) | Spec validation, seeded randomized-block run order, hash-chained raw rows under `research/raw/<EXP>/`, normalization to `research/normalized/<EXP>/summary.json`, reports, bootstrap CIs, epsilon-Pareto, Kendall tau-b, weight sensitivity, quiet-machine guard, held-out lock. Metric families: `time_wall`, `time_cpu`, `memory_peak`, `scratch_peak`, `size`, `throughput`, `io`, `correctness`, `other`. | FORMALLY_DERIVED |
| Corpus manifest | `research/corpus/manifest.json` (`ebrc-2026.09-v1`, 196 items, families F01-F20, splits tuning/validation/heldout, scale tiers small ≤ 16 MiB, medium ≤ 512 MiB, large ≤ 8 GiB, fingerprints `ebrc-fp-v1` with `logical_tree_sha256`) | Workload population and item identity | FORMALLY_DERIVED |
| Harness crates | Planned in `research/harness/crates/`: `ebr-common`, `ebr-pack` (exact section byte accounting), `ebr-chunk`, `ebr-codec`, `ebr-planner`, `ebr-access`, `ebr-netem` (`ebr-origin`, `ebr-netem-proxy`). Phase B2 relaunch pending; partial patch in `research/harness/wip/`. | Metric extraction beyond black-box CLI timing | UNRESOLVED (not built) |
| Baselines | `research/baselines/` (Phase B1d: ZIP/DEFLATE, tar+gzip, tar+zstd, tar+xz, 7z/LZMA2, capability-matched) | Incumbent and specialist references | UNRESOLVED (empty at drafting) |
| Conformance corpus and independent reader | `research/conformance/`, `research/independent-reader/` | HC-03, HC-04, HC-12, HC-15, HC-17 oracles; OD-04, OD-21 | UNRESOLVED (empty at drafting) |

The definitions in this document govern. A harness crate whose scope changes must still implement the metric as defined here. If it cannot, the metric definition is amended through the method-review process before any result exists, never after. [INFERRED]

**Canonical names and roles.** Every M-numbered metric below has a canonical name, a threshold ID and a **role** fixed in `decision-method.md` Appendix A.2 and transcribed into `research/methods/thresholds.json` `metric_thresholds`: `banded` (may be a primary metric), `diagnostic` (primary only when the decision is about that component), `sensitivity_arm`, `binary` (an HC floor; any failure is R1 or R2), `descriptive` (reported; never eliminates or selects), `cost_input` (feeds the cost elements and tiers of `decision-method.md` §7) or `heuristic` (simulated sessions; no CI-based verdicts). A metric without a band committed before its first decision-relevant result is secondary for every decision, permanently (`decision-method.md` §0.2). Canonical names are cited in brackets after each M-number. [INFERRED]

**Correctness gating of performance samples.** Every performance or size sample carries the correctness checks of its operation: `roundtrip_ok`, identity equality, and the verification state. A sample whose correctness check fails is excluded from OD analysis and reported as an HC violation. It is never averaged. [INFERRED]

**References.** Each decision names its reference configuration. Two references are standard:
- **REF-PROD:** production `ebound pack` defaults (balanced profile, INDEXED, unencrypted, current planner ID) built at the research baseline SHA `9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155`.
- **REF-INC:** the capability-matched incumbent baseline for the metric, as defined in `research/baselines/`.

External-comparison claims (§23-style) report both references. [INFERRED]

**Per-item statistic.** For timing and resource metrics, the per-item value is the paired median over repetitions; repetitions, warmups and CI procedure are defined in `decision-method.md`. Deterministic byte metrics use a single value confirmed by repeat-hash. [INFERRED]

**Aggregation operators.** One or more is named per metric. [INFERRED; the choice of geometric means for ratios is standard for multiplicative quantities]

| Operator | Definition |
|---|---|
| **AGG-G** (geometric, family-weighted) | Per item i: r_i = value(candidate, i) / value(reference, i). Within family f, average ln r_i equally over items within each independence group, then equally over groups, giving ln R_f. Headline R = exp(weighted mean over applicable families of ln R_f), with family weights from `decision-method.md` §4.9 (the cited workload mix; equal weights as a sensitivity arm). Always reported with the **worst-family guard** (max_f R_f for minimized metrics, min_f R_f for maximized ones) and per scale tier; `decision-method.md` R4 and R8 apply their not-worse guards per family **and** per tier. The byte-weighted ratio of totals is reported as a sensitivity arm. Splits are never pooled. Families with no applicable item are listed and excluded. Metrics that can be zero use AGG-W or AGG-C instead. Absolute-only metrics (no relative band in `decision-method.md` Appendix A.2, for example percentage points and bits) use the same group and family weighting on item differences in metric units, with arithmetic means (`decision-method.md` §3.1, §4.9). |
| **AGG-W** (worst case) | Maximum over all applicable (item, operation, platform) observations. For violation or event counts, the sum. |
| **AGG-C** (coverage) | Per family, the fraction of applicable cases that pass. Headline is the minimum over families; the mean is secondary. |
| **AGG-S** (per evaluation condition) | Reported per pre-registered evaluation condition (network profile, platform pair, user task, legacy format, padding mode). Headline is the worst condition. No averaging across conditions; `decision-method.md` R4 and R8 apply per condition. Evaluation conditions are not the workload mixes WM-* of `decision-method.md` Appendix B, which reweight families. |
| **AGG-P** (per design) | Workload-independent: one value per candidate design (specification, dependency or surface census). |

Workload-mix reweighting of AGG-G belongs to the sensitivity analysis in `decision-method.md`. It is not the headline. [INFERRED]

**Type labels.**
- **BINARY:** pass or fail, identical to an HC floor, never traded.
- **FLOOR+GRADED:** a binary floor from an HC, with the graded quantity measured only above the floor.
- **GRADED:** trade-offs allowed under `decision-method.md`.

### 3.1 Summary

| OD | Dimension | Primary metric (unit) | Direction | Aggregation | Type | HC floors |
|---|---|---|---|---|---|---|
| OD-01 | Semantic coherence | Equivalence assertions pass (fraction = 1); CACHE+CLAIM fields and conditional decoding rules (count, descriptive) | = 1; report | AGG-C; AGG-P | FLOOR+GRADED | HC-01, HC-03 |
| OD-02 | Exact losslessness | Round-trip mismatches (count) | = 0 | AGG-W | BINARY | HC-02 |
| OD-03 | Preservation fidelity | Exact capture-and-restore fraction per metadata class and platform pair (fraction) | max | AGG-S (min over classes) | FLOOR+GRADED | HC-07 |
| OD-04 | Deterministic interpretation | Cross-reader and cross-layout disagreements (count); reason-code specificity (fraction); decided-class reach (fraction, descriptive) | = 0; max; report | AGG-W; AGG-C | FLOOR+GRADED | HC-03, HC-11, HC-15 |
| OD-05 | Compression and storage efficiency | Artifact bytes relative to reference (ratio) | min | AGG-G | GRADED | HC-02, HC-09, HC-14 |
| OD-06 | Creation performance | Pack wall and CPU time relative to reference (ratio); logical throughput (MiB/s) | min; max | AGG-G | GRADED | HC-02, HC-13 |
| OD-07 | Decoding performance | Unpack, verify and list wall and CPU time relative to reference (ratio) | min | AGG-G | GRADED | HC-11, HC-15 |
| OD-08 | Memory and scratch | Peak memory and scratch (bytes); bounded-memory growth from 1 to 64 GiB (binary claim) | min | AGG-W and AGG-G | FLOOR+GRADED | HC-06 |
| OD-09 | Streaming | Streaming capability per operation (binary); STREAM overhead (ratio) | max; min | AGG-C; AGG-G | FLOOR+GRADED | HC-06, HC-15 |
| OD-10 | Random access | p95 random-entry latency (s); access amplification (ratio) | min | AGG-G and AGG-W | FLOOR+GRADED | HC-09, HC-15 |
| OD-11 | Verified partial retrieval | Bytes and time to a verified range relative to an unverified read (ratio); verified fraction | min; max | AGG-G; AGG-C | FLOOR+GRADED | HC-15, HC-10 |
| OD-12 | Remote access | HTTP requests (count), bytes transferred (B), modelled latency (s) per network profile NP-LAN, NP-METRO, NP-CONT, NP-INTER | min | AGG-S of AGG-G | FLOOR+GRADED | HC-15, HC-14, HC-18 |
| OD-13 | Parallelism | Speedup S(n) (1) within one core class, and efficiency S(n)/n | max | AGG-G | FLOOR+GRADED | HC-11, HC-13 |
| OD-14 | Corruption locality | Blast radius: logical bytes and entries not verifiable as intact per injected event, region-stratified (B, count) | min | AGG-G and AGG-W | FLOOR+GRADED | HC-15 |
| OD-15 | Confidentiality and authenticity | Undetected tampers (count); authenticated coverage (fraction); encryption overhead (ratio); assurance state | = 0; = 1; min; max | AGG-W; AGG-G | FLOOR+GRADED | HC-14 |
| OD-16 | Metadata privacy | Per padding mode: presence-confirmation advantage (0-1); public plaintext-derived bits; padding overhead | min | AGG-W; AGG-S over padding modes | GRADED | HC-14 |
| OD-17 | Resource-bounded decoding | Declared/actual tightness (ratio ≥ 1); refusal cost; benign false-refusal rate; hostile-input CPU scaling and scale limits (descriptive) | min | AGG-W and AGG-G | FLOOR+GRADED | HC-06 |
| OD-18 | Cross-platform fidelity | Min over platform pairs of restore fidelity (fraction); platform coverage | max | AGG-S | FLOOR+GRADED | HC-08, HC-07 |
| OD-19 | Legacy interoperability | Import acceptance and correctness; export acceptance by incumbent readers (fraction) | max | AGG-S (per format) | FLOOR+GRADED | HC-07, HC-17, HC-05 |
| OD-20 | Deterministic migration | Identity-rule conformance of migrations (fraction = 1); migration cost (ratio) | max; min | AGG-C; AGG-G | FLOOR+GRADED | HC-10, HC-13, HC-07 |
| OD-21 | Independent implementability | Clean-room pass rate (fraction = 1); independent-reader LoC and reader fragmentation (cost inputs); specification defects (descriptive) | = 1; cost | AGG-P | FLOOR+GRADED | HC-12 |
| OD-22 | Dependency longevity | Decode-path dependency count, native/unsafe code, advisories, maintainers (cost inputs); licence and specification availability (binary) | cost; = 1 | AGG-P | FLOOR+GRADED | HC-12, HC-14 |
| OD-23 | Specification complexity | Normative words and wire items added (cost inputs, R10 key); words per normative rule (descriptive) | cost | AGG-P | GRADED | HC-16 |
| OD-24 | Implementation attack surface | Untrusted-input parser LoC, attacker-controlled parameters, "validated"-only vulnerability classes (cost inputs); unresolved fuzz findings at a fixed coverage target (= 0) | cost; = 0 | AGG-P and AGG-W | FLOOR+GRADED | HC-03, HC-06 |
| OD-25 | User complexity | Flags per task (count, cost input); error actionability census (fraction); unsafe defaults (count = 0); simulated task success (heuristic) | min; max; = 0 | AGG-S | FLOOR+GRADED | HC-05, HC-15 |
| OD-26 | Adoption friction | Single-file self-containment (binary); consumable-with-preinstalled-tools fraction, migration steps, identification support (descriptive) | = 1; report | AGG-S | GRADED | HC-07, HC-16, HC-09 |

Metric choice throughout §3 is `INFERRED` unless tagged otherwise. Instruments named "planned" are `UNRESOLVED` until committed.

### 3.2 Dimension definitions

Each dimension lists metrics (M-number), unit, measurement procedure, direction, aggregation, type and notes. The `ebr` metric family is given in brackets; it is the family of the canonical metrics in `decision-method.md` Appendix A.2, which governs where the two differ (checked by script in `decision-method.md` Appendix A.3).

#### OD-01 Semantic coherence
- **M01.1** Validation obligations: the number of CACHE and CLAIM fields from the MVT-01 census. Each is a place where a reader must validate against authority. Unit: count [`other`]. **M01.2** Conditional decoding rules: normative rules whose applicability depends on kind, role, layout or feature combination, counted from the HC-12 traceability matrix. Unit: count [`other`]. **M01.3** [`equivalence_assertion_pass_fraction` (binary)] Equivalence assertions: the fraction of INDEXED/STREAM, encrypted/plain and identity-tier equivalence assertions that pass. Unit: fraction [`correctness`].
- **Procedure.** Census and traceability scripts run on the candidate's specification and registries. Equivalence assertions run as `ebr` correctness checks over tuning items.
- **Direction.** M01.3 must equal 1 (binary). M01.1 [`validation_obligations_count`] and M01.2 [`conditional_decoding_rules_count`] are **descriptive**: minimizing CACHE and CLAIM fields would penalize indexes (OD-10), and the classification is gameable by the designer, so the census is classified by a session not involved in the candidate and the counts never eliminate or select (review finding D02).
- **Aggregation.** AGG-P for M01.1 and M01.2; AGG-C for M01.3.
- **Type.** FLOOR+GRADED (floor HC-01, HC-03).
- **Note.** Counts proxy cognitive and implementation coherence. [INFERRED; EXPERT_REVIEW_REQUIRED for census completeness]

#### OD-02 Exact losslessness
- **M02.1** [`roundtrip_mismatches` (binary)] Round-trip mismatches (files and symlinks) from MVT-02(a). Unit: count [`correctness`].
- **Procedure.** MVT-02.
- **Direction.** Must be 0.
- **Aggregation.** AGG-W (sum over all observations).
- **Type.** BINARY.
- **Note.** Not traded. The *cost* of the guarantee (write-time verification) is visible in OD-06 M06.3. [FORMALLY_DERIVED from SPEC §6.5]

#### OD-03 Preservation fidelity
- **M03.1** Capture fidelity: the fraction of independently observed source metadata items, per class, stored exactly (equal value and precision). **M03.2** Restore fidelity: the fraction restored exactly on the target. **M03.3** Declared-gap count per class. Units: fraction, count [`other`].
- **Procedure.** MVT-07(a) observers over generated trees from F19 (metadata-heavy), F15 (sparse) and F17 (duplicate trees for hardlink topology). Trees are built on WSL ext4 under `/root/eb-research`, on Windows NTFS, and in Docker. Incumbent comparisons: GNU tar 1.35 `--xattrs --acls --selinux`, bsdtar 3.7.2 and 3.8.8, 7-Zip 23.01, zip. [EXTERNALLY_SOURCED tool versions from PROGRESS.md environment]
- **Direction.** Maximize M03.1 and M03.2.
- **Aggregation.** AGG-S per (class, source platform, target platform). For each platform pair, the **class list is fixed before any OD-03 result** from independently observed platform capability: a class is on the list when the MVT-07(a) observers can read and write it on both platforms. Every listed class counts, and a class the candidate does not restore counts as 0 whether or not it is declared non-restorable. The headline is the minimum over listed classes. Declared-non-restorable classes are reported against the capability of the incumbent specialists (GNU tar 1.35 pax, bsdtar 3.8.8). The per-class matrix is always reported. M03.1 [`capture_fidelity_fraction`], M03.2 [`restore_fidelity_fraction`]; M03.3 [`declared_gap_count`] is descriptive.
- **Type.** FLOOR+GRADED. The floor HC-07 makes undeclared loss a violation; declared loss is graded here.
- **Note.** APFS, HFS+ and macOS ACLs are `PLATFORM_BLOCKED`, and ReFS and SACLs need admin rights. [UNRESOLVED]

#### OD-04 Deterministic interpretation
- **M04.1** Disagreements: the number of distinct (input, reader pair) disagreements from MVT-03(b). Unit: count [`correctness`]. **M04.2** Reason-code specificity: the fraction of negative conformance cases whose outcome carries a unique stable reason code identifying the violated rule. Unit: fraction [`other`]. **M04.3** Decided-class reach [`decided_class_reach_fraction`]: the fraction of the pre-registered decided (outcome class, reason code) pairs of the conformance corpus that the mutation corpus reaches. Unit: fraction [`other`]. Descriptive: it measures the fuzzing campaign, and a per-CPU-hour count would reward reason-code proliferation (OD-23).
- **Direction.** M04.1 [`reader_disagreements`] must be 0; maximize M04.2 [`reason_code_specificity_fraction`]; M04.3 is reported.
- **Aggregation.** AGG-W for M04.1; AGG-C for M04.2 and M04.3.
- **Type.** FLOOR+GRADED (floors HC-03, HC-11, HC-15).

#### OD-05 Compression and storage efficiency
- **M05.1** Complete artifact bytes: the `.eb` file size including descriptor, manifest, metadata, index, dictionaries, frame headers, reconstruction side data, groups, padding, signatures and footer. Unit: B, reported as ratio to reference [`size`]. **M05.2** [`metadata_bytes` (diagnostic), `index_bytes` (diagnostic), `dictionary_bytes` (diagnostic), `side_data_bytes` (diagnostic), `metadata_bytes_per_entry`] Section byte breakdown from `ebr-pack` exact accounting, whose sum must equal M05.1 or the sample is invalid. Unit: B [`size`]. **M05.3** Fixed overhead on the small tier [`fixed_overhead_bytes`]: artifact bytes minus the smallest output, over the following single-stream codec configurations, of the item's content concatenated in manifest order: zstd levels 3 and 19; xz presets 6 and 9e; brotli quality 11 with window 24; gzip level 9. Codec versions are those pinned in `research/baselines/` at the experiment's pre-registration. Unit: B [`size`]. **M05.4** Dedup effectiveness [`dedup_stored_fraction`, diagnostic]: unique stored chunk bytes divided by logical bytes on F17 and F18. Unit: ratio [`size`]. **M05.5** Byte-weighted stored total [`total_stored_bytes`, sensitivity arm]: the sum of artifact bytes over all items of the split relative to the reference's sum, which is what storage cost is. Unit: ratio [`size`].
- **Procedure.** `ebr run` with `ebr-pack` over tuning and validation items × profiles × layouts × encryption. The run is deterministic, with repeat-hash verification. Incumbent references use capability-matched settings.
- **Direction.** Minimize.
- **Aggregation.** AGG-G with worst-family and per-tier guards for M05.1 [`artifact_bytes`] and M05.4; AGG-W over the small tier for M05.3; ratio of totals for M05.5; M05.2 [`metadata_bytes` and components] is diagnostic.
- **Type.** GRADED.
- **Note.** Padding bytes in encrypted archives are also reported under OD-16 M16.3. The padding mode is a declared parameter (`--pad none|buckets|max`); `pad none` is an admissible, recorded opt-out that reports a declared leak (HC-14). M05.1 comparisons of encrypted archives are made within one padding mode, and a candidate cannot claim a size gain by changing the mode without that change being evaluated under OD-16.

#### OD-06 Creation performance
- **M06.1** Pack wall time. Unit: s, ratio to reference [`time_wall`]. **M06.2** Pack CPU time. Unit: s, ratio [`time_cpu`]. **M06.3** Mandatory-verification share of pack CPU time [`verification_cpu_share`, descriptive], from `ebr-pack` phase timers (capture, chunking, planning, encoding, verification, writing). Unit: fraction [`other`]. **M06.4** Logical throughput [`encode_throughput_bps`]: logical input bytes divided by pack wall time. Unit: B/s [`throughput`]. **M06.5** Unstable-source retry cost [`unstable_source_retry_cost_s`, descriptive]: extra pack wall time under the MVT-07(d) mutator relative to the same pack without mutation. Unit: s [`other`].
- **Procedure.** `ebr run --timing` under the quiet-machine guard, with paired randomized blocks. Decision-grade runs use only the affinity configurations of `decision-method.md` §4.2 and §4.3 (Windows `st-p`, `mt-p-n`, `mt-psmt-14`, `st-e`, `mt-e-n`; WSL `st-v`, `mt-v-n`); unpinned `os-scheduled` runs are informational. WSL results carry the `VIRTUALIZED` qualifier. Repetitions, warmups and guard limits follow `decision-method.md`.
- **Direction.** Minimize M06.1 [`encode_wall_s`] and M06.2 [`encode_cpu_s`]; maximize M06.4; M06.3 and M06.5 are descriptive.
- **Aggregation.** AGG-G with worst-family guard, per thread setting.
- **Type.** GRADED.

#### OD-07 Decoding performance
- **M07.1** Full unpack wall and CPU time, including mandatory verification and verified staging. **M07.2** Verify-only time. **M07.3** List/inspect latency [`list_latency_s`]. **M07.4** Time to first plaintext byte of the first entry [`first_entry_latency_s`], with first verified and first unverified byte both reported (I25). Units: s, ratio, B/s [`time_wall`, `time_cpu`, `throughput`]. M07.1 [`decode_wall_s`, `decode_cpu_s`, `decode_throughput_bps`]; M07.2 [`verify_wall_s`, `verify_throughput_bps`].
- **Procedure.** As OD-06, with `ebr-unpack` and `ebr-verify` from `ebr-pack` and `ebr-access` for M07.4. M07.3 and M07.4 are per-operation latencies: they are measured only with in-process timers or as batches of operations inside one process (`decision-method.md` §3.2 T-08), never from whole-process samples. The small tier is reported separately.
- **Direction.** Minimize.
- **Aggregation.** AGG-G with worst-family guard.
- **Type.** GRADED.
- **Note.** Comparisons against incumbents that do not verify must also report an incumbent-with-external-hash configuration, so verification cost is not scored as slowness. [INFERRED]

#### OD-08 Memory and scratch
- **M08.1** Peak RSS per operation (pack, unpack, verify, random read, import, export, repack, publish). Unit: B [`memory_peak`]. **M08.2** Peak scratch [`scratch_peak_bytes`], and scratch bytes written [`scratch_write_bytes`, binary, for zero-scratch claims]. Unit: B [`scratch_peak`, `correctness`]. **M08.3** Bounded-memory growth [`bounded_memory_growth_bytes`, binary claim check]: the counting-allocator peak minus the baseline R0 (the allocator peak for the same operation on a minimal valid input) at 1, 4, 16 and 64 GiB of logical input from the `ebr-access` streaming-memory probe, 3 runs per size. A regression of ln(peak RSS) on ln(bytes) is not used, because the constant R0 dominates at small and medium sizes and biases the slope toward 0 (review finding D04). Unit: B [`correctness`].
- **Direction.** Minimize M08.1 [`peak_rss_bytes`, `peak_commit_bytes`, `alloc_peak_bytes`] and M08.2 [`scratch_peak_bytes`]. For operations the SPEC or repository docs describe as bounded-memory, the claim holds only if the growth from 1 GiB to 64 GiB is at most max(4 MiB, 0.10 × R0) in every run, and the upper bound of the exact order-statistic interval over runs (`decision-method.md` §4.10) is inside that limit; otherwise the bounded-memory claim is recorded as false. Zero-scratch or streaming claims are checked by write accounting with tolerance 0 (`scratch_write_bytes`, `decision-method.md` §4.14), not by polled scratch peaks.
- **Aggregation.** AGG-W (max over items) and AGG-G (typical) for M08.1 and M08.2; AGG-W for M08.3.
- **Type.** FLOOR+GRADED (HC-06 for policy and declaration exceedance).

#### OD-09 Streaming
- **M09.1** Capability [`streaming_capability`, binary]: whether each operation completes with a Write-only sink or Read-only source (pipe) and no temporary whole-input copy (write accounting, tolerance 0). Unit: binary per operation [`correctness`]. M09.2 and M09.4 are [`artifact_bytes`] comparisons (STREAM versus INDEXED of the same EAM; window w versus unbounded); M09.3 is [`stream_unpack_wall_s`]. **M09.2** STREAM artifact bytes relative to INDEXED for the same EAM. Unit: ratio [`size`]. **M09.3** End-to-end streaming unpack latency, including verified staging (F-11). Unit: s [`time_wall`]. **M09.4** Dedup loss from the stream window: stored bytes at window w relative to unbounded window. Unit: ratio [`size`].
- **Procedure.** `ebr` commands with `cat item | ebound pack --layout stream -` and `ebound unpack -` pipelines; `ebr-pack --layout stream --stream-window`.
- **Direction.** Maximize M09.1; minimize the rest.
- **Aggregation.** AGG-C for M09.1; AGG-G for M09.2-M09.4.
- **Type.** FLOOR+GRADED (floors HC-06, HC-15, F-09).

#### OD-10 Random access
- **M10.1** Random-entry read latency [`random_entry_latency_s`], p50 and p95 over a seeded sample of 1,000 entries (or all entries if fewer), in-process timers only. Unit: s [`time_wall`]. **M10.2** Access amplification [`access_amplification`]: decoded bytes divided by requested bytes, reported per pre-registered range-size stratum {1 B, 4 KiB, 1 MiB, whole entry}, with seeded byte-range reads within large entries; strata are never pooled, because amplification for 1-byte ranges equals the decoded unit size. Unit: ratio [`other`]. **M10.3** Open cost: bytes read [`open_bytes_read`] and time [`open_latency_s`] to open an INDEXED archive for random access. Unit: B, s [`size`, `time_wall`]. **M10.4** Declared-cost accuracy [`declared_cost_accuracy`, binary]: measured over declared decode work, which must be ≤ 1 (HC-09). Unit: ratio [`correctness`]. **M10.5** Encrypted-index random access [`encrypted_random_entry_latency_s`]: M10.1 on encrypted INDEXED archives with the key supplied (CC-04). Unit: s [`time_wall`].
- **Procedure.** `ebr-access` with Memory and LocalFile sources; `ebr-chunk` amplification subcommand.
- **Direction.** Minimize.
- **Aggregation.** AGG-G with worst-family guard for M10.1-M10.3; AGG-W for M10.2 and M10.4.
- **Type.** FLOOR+GRADED.

#### OD-11 Verified partial retrieval
- **M11.1** Verified-range overhead: bytes fetched to obtain a verified range [a, b) [`verified_range_fetch_bytes`, compared with the same range read unverified], and time overhead [`verify_overhead_pp`] = 100 × (verified wall / unverified wall − 1). Unit: B, pp [`size`, `other`]. **M11.2** Verification work [`verification_digest_ops`, diagnostic]: Merkle nodes and digest computations per verified range. Unit: count [`other`]. **M11.3** Verified fraction [`verified_fraction`]: the fraction of partial-read results in the strongest state the archive supports (chunk digest, or signed-PCR Merkle slice when signed). Unit: fraction [`other`].
- **Procedure.** `ebr-access` range reads on signed and unsigned INDEXED archives, with the MVT-15(a) oracle.
- **Direction.** Minimize M11.1 and M11.2; maximize M11.3.
- **Aggregation.** AGG-G; AGG-C for M11.3.
- **Type.** FLOOR+GRADED (HC-15, HC-10).

#### OD-12 Remote access
- **M12.1** HTTP requests per task [`http_requests`]: open plus list, first entry, random entry, random range, full verify. Unit: count [`other`]. **M12.2** Bytes transferred [`http_bytes`] (response bodies plus headers). Unit: B [`size`]. **M12.3** Task latency [`remote_first_entry_latency_s`, `remote_random_entry_latency_s`, `remote_task_latency_s`]. Unit: s [`time_wall`]. **M12.4** Protocol safety [`origin_protocol_safety`, binary; HC-18 MVT-18(b)]. Unit: binary [`correctness`].
- **Procedure.** `ebr-origin` serves archives through `ebr-netem-proxy` under the canonical network profiles of `decision-method.md` §4.15 (NP-LAN 2 ms / 1000 Mbit/s, NP-METRO 20 ms / 200 Mbit/s, NP-CONT 80 ms / 50 Mbit/s, NP-INTER 200 ms / 20 Mbit/s), on plain and encrypted INDEXED archives. Requests and bytes are counted exactly. The primary latency value is **modelled** from the exact request and byte log (round trips of the declared connection model plus transfer time and TCP initial-window rounds, `decision-method.md` §4.15); the emulated measurement corroborates it after the emulator passes its pre-registered calibration (`EXP-NETEM-CAL`). Loss is injected only to test failure behaviour, and no performance claim is made under loss. Emulation is application-level because `sch_netem` is unavailable in WSL2 and Docker, a documented threat to validity. [FORMALLY_DERIVED from PROGRESS.md blockers]
- **Direction.** Minimize M12.1-M12.3; M12.4 must pass.
- **Aggregation.** AGG-S over network profiles of AGG-G over families. No cross-profile averaging.
- **Type.** FLOOR+GRADED (floors F-10, HC-15, HC-18).

#### OD-13 Parallelism
- **M13.1** Speedup [`parallel_speedup`] S(n) = T(1)/T(n) for pack, unpack and verify, **within one core class**: P-core without SMT, T(1) on `st-p` and T(n) on `mt-p-n` for n ∈ {2, 4, 7}; E-core, T(1) on `st-e` and T(n) on `mt-e-n` for n ∈ {2, 4, 8, 16}; the SMT variant `mt-psmt-14` against `mt-p-7`, reported separately. **M13.2** Efficiency [`parallel_efficiency`, diagnostic] S(n)/n. Unit: 1 [`other`].
- **Procedure.** `ebr --timing` with the pinned affinity configurations of `decision-method.md` §4.2 on the Windows host, with WSL `mt-v-n` secondary and `VIRTUALIZED`. Speedup is the one derived comparison across affinity configurations that `decision-method.md` §4.2 permits; it has its own A/A-calibrated noise estimate. Speedups across core classes, and any `os-scheduled` scaling, are informational. [FORMALLY_DERIVED from PROGRESS.md environment]
- **Direction.** Maximize.
- **Aggregation.** AGG-G of S(7) on P-cores and S(16) on E-cores, each with worst-family guard; never pooled across core classes.
- **Type.** FLOOR+GRADED. The floor is HC-11 and HC-13: output identity across n.

#### OD-14 Corruption locality
- **M14.1** Blast radius **per injected event** [`blast_logical_bytes_p95`, `blast_logical_bytes_max`, `blast_entries_p95`]: logical bytes, and separately entries, not verifiable as intact after one injection event. It is not divided by damaged stored bytes. Unit: B, count [`other`]. **M14.2** Localization precision [`localization_precision`] and recall [`localization_recall`, binary = 1]: the entries `verify` reports as affected versus the oracle set derived from the closure (HC-09). Unit: fraction [`other`, `correctness`]. **M14.3** Truncation salvage [`truncation_salvage_fraction`]: the fraction of oracle-intact entries identified as intact after truncation. Unit: fraction [`other`].
- **Procedure.** A seeded harness injector applies, per archive, at least 200 single-byte corruptions in each of five **region strata** (payload, Index, metadata and manifest, dictionaries, integrity records and footer), which is the primary class, plus 512 B, 64 KiB and 1 MiB overwrites and truncation at section boundaries and random offsets as secondary classes. Each injection is followed by `verify` and `salvage`. Incumbent references: ZIP, tar.gz, tar.zst, 7z solid and non-solid.
- **Direction.** Minimize M14.1; maximize M14.2 precision and M14.3.
- **Aggregation.** Per injection class and region stratum, p95 and max per item. The headline combines strata with **equal pre-registered weights** (0.2 each), so that Index and manifest damage, rare under uniform byte sampling, is not outvoted by payload damage; the **worst-region guard** (the maximum over strata) applies in `decision-method.md` R4 and R8; the byte-proportional combination is a sensitivity arm. Then AGG-G across families plus AGG-W.
- **Type.** FLOOR+GRADED. M14.2 recall must be 1: an affected entry reported intact violates HC-15.

#### OD-15 Confidentiality and authenticity
- **M15.1** [`undetected_tampers` (binary)] Undetected tampers from MVT-14(b), extended to signatures and bindings. Unit: count [`correctness`]. **M15.2** Authenticated coverage [`authenticated_coverage_fraction`, binary]: the fraction of archive bytes that are authenticated or bound as associated data, **scoped per archive type**: for encrypted archives every byte after the public preamble must be under AEAD or bound as associated data; for signed unencrypted archives every byte covered by the CONTENT, PHYSICAL and ADDRESSING transcripts must be bound; for unsigned plaintext archives the metric is not applicable and integrity is graded under OD-14 and HC-15. Unit: fraction [`correctness`]. **M15.3** [`artifact_bytes`, `encode_wall_s`, `decode_wall_s`] Encryption overhead: bytes (envelope, framing, padding), and pack and unpack time, relative to the unencrypted pack of the same EAM. Unit: ratio [`size`, `time_wall`]. **M15.4** [`assurance_state` (descriptive)] Assurance state per property: vector-verified < internally reviewed < externally reviewed. Unit: ordinal [`other`]. **M15.5** [`declared_security_level_bits` (descriptive)] Declared security level per recipient policy, classical and post-quantum. Unit: bits. [EXTERNALLY_SOURCED from primitive specifications]
- **Direction.** M15.1 must be 0 and M15.2 must be 1; minimize M15.3; maximize M15.4.
- **Aggregation.** AGG-W for M15.1 and M15.2; AGG-G for M15.3; AGG-P for M15.4 and M15.5.
- **Type.** FLOOR+GRADED (HC-14). M15.4 progress beyond "internally reviewed" requires external review. [EXPERT_REVIEW_REQUIRED]

#### OD-16 Metadata privacy
- **M16.1** Public plaintext-derived information [`leak_min_entropy_bits`, `leak_shannon_bits`, `anonymity_set_median`]: an inventory of fields visible without keys (counts, sizes, segment and record counts, lengths, boundaries) with bounds in bits per archive, per padding mode. Unit: bits [`other`]. **M16.2** Presence-confirmation advantage [`presence_advantage`]: for a passive adversary holding a candidate file, the maximum over the committed attack suite of true-positive rate minus false-positive rate, at a false-positive rate of 0.01, per padding mode. The attack suite and the observer model are committed to the repository before any OD-16 result: the length-only reference attack, the published content-defined-chunking and length-leakage attacks that SPEC §7.6 cites, and at least one **adaptive** attack that is tuned on tuning-split archives of the candidate under test. Until the suite and observer model are committed, OD-16 is not decision-grade. Because the metric is a lower bound on true leakage, a lower value is never evidence of safety beyond the suite, and effectiveness claims are routed to external review (SPEC §25.2). Unit: 0-1 [`other`]. **M16.3** [`padding_overhead_pp`] Padding overhead: 100 × padding bytes / artifact bytes without padding, compared within one padding mode; per-artifact padding bytes are also reported (`decision-method.md` T-16). Unit: pp [`other`]. **M16.4** [`range_length_signatures`] Remote access-pattern leakage: distinct range-length signatures observable by the origin per accessed entry. Unit: count [`other`].
- **Procedure.** Harness leakage analyzer over encrypted packs of tuning items, attack implementations run offline, and `ebr-origin` request logs for M16.4.
- **Direction.** Minimize, with M16.3 in tension (see §4.3).
- **Aggregation.** AGG-W (worst family) for M16.2; AGG-P for M16.1; AGG-G for M16.3.
- **Type.** GRADED. The HC-14 floor covers keyed boundary derivation, the recorded padding mode and conformance of public lengths to it. The residual leakage per padding mode is the design objective I24 names, not a proven property. [FORMALLY_DERIVED from REQ-CRY-0097, SPEC §7.6, §9.8, §23.4; EXPERT_REVIEW_REQUIRED for attack-model adequacy]

#### OD-17 Resource-bounded decoding
- **M17.1** Tightness: declared requirement divided by measured actual, for memory, decoded bytes, entries and chunks. Must be ≥ 1 (HC-06), with smaller being more useful to callers. Unit: ratio [`other`]. **M17.2** Refusal cost: peak RSS, CPU time and bytes read before refusal of hostile archives. Units: B, s, B [`memory_peak`, `time_cpu`, `size`]. **M17.3** Default-policy false-refusal rate: the fraction of benign tuning and validation items refused under the default CLI policy. Unit: fraction [`other`]. **M17.4** Default-policy true-refusal rate on the MVT-06 adversarial set [`adversarial_true_refusal_fraction`, binary]. Must be 1. Unit: fraction [`correctness`]. **M17.5** Hostile-input CPU scaling [`hostile_cpu_scaling_exponent`, descriptive]: the fitted exponents from MVT-06(d) per structure. Unit: 1 [`other`]. **M17.6** Scale limits [`scale_limits`, descriptive]: the largest entry count, path count, path depth and archive size, probed in powers of two beyond the 8 GiB tier, at which each operation still completes within declared bounds, and the first size at which it fails or scales superlinearly. Unit: count, B [`other`]. M17.1 is [`declared_tightness_ratio`]; M17.2 is [`refusal_alloc_peak_bytes`, `refusal_cpu_s`, `refusal_bytes_read`]; M17.3 is [`benign_false_refusal_fraction`].
- **Direction.** Minimize M17.1 (toward 1), M17.2 and M17.3; M17.4 must be 1.
- **Aggregation.** AGG-G for M17.1; AGG-W for M17.2 and M17.4; AGG-C for M17.3.
- **Type.** FLOOR+GRADED.

#### OD-18 Cross-platform fidelity
- **M18.1** [`cross_host_identity_agreement` (binary)] Identity agreement across hosts (MVT-08(a)). Unit: binary per archive [`correctness`]. **M18.2** [`cross_platform_restore_fidelity_fraction`] Cross-platform restore fidelity: OD-03 M03.2 restricted to pairs with different source and target platforms. Unit: fraction [`other`]. **M18.3** [`hostile_name_handling_fraction` (binary)] Hostile-name handling: the fraction of hostility-corpus cases that extract faithfully when representable, or else refuse or rename with a report. Unit: fraction [`correctness`]. **M18.4** [`platform_coverage_count` (descriptive)] Platform coverage: platforms tested out of the platforms the SPEC names; blocked platforms are listed. Unit: count [`other`].
- **Direction.** Maximize.
- **Aggregation.** AGG-S (minimum over platform pairs).
- **Type.** FLOOR+GRADED (floors HC-08, HC-07).
- **Note.** The M18.4 gap for macOS and native ARM64 is `UNRESOLVED`.

#### OD-19 Legacy interoperability
- **M19.1** [`import_acceptance_fraction`] Import acceptance per legacy format and policy (strict, compat-ID, preservation): the fraction of real-world inputs imported. Inputs are corpus F14 plus public legacy corpora recorded under `research/corpus/sources`. Unit: fraction [`other`]. **M19.2** Import agreement [`import_agreement_fraction`]: the fraction of unambiguous inputs whose imported EAM matches the **unanimous** result of the reference readers (Python zipfile/tarfile, Java, libarchive/bsdtar, GNU tar, 7-Zip 23.01); inputs on which the reference readers disagree are not "unambiguous" and are excluded. The fraction matching the reference majority is reported separately [`import_majority_agreement_fraction`, descriptive]. Unit: fraction [`other`]. **M19.3** [`ambiguity_refusal_fraction` (binary)] Ambiguity refusal: the fraction of Research III ambiguity classes refused or declared, which must be 1 under strict. Unit: fraction [`correctness`]. **M19.4** [`export_acceptance_fraction`, `export_outcome_distribution` (descriptive)] Export acceptance: the fraction of exported artifacts that incumbent readers (unzip, bsdtar, GNU tar, 7z, Python, Java) read to the EAM-equivalent tree, plus the LOSSLESS/LOSSY/REFUSED distribution over corpus EAMs. Unit: fraction [`other`]. **M19.5** [`encode_wall_s`, `decode_wall_s`, `peak_rss_bytes`] Import and export time and memory, as OD-06 to OD-08. Units: s, B [`time_wall`, `memory_peak`].
- **Procedure.** `ebr` specs invoking `ebound import` and `export` plus incumbent tools in WSL and Docker; `tools/zip-compat` and `tools/tar-strict` matrices.
- **Direction.** Maximize M19.1, M19.2 and M19.4, subject to M19.3 = 1.
- **Aggregation.** AGG-S per legacy format, with the headline at the minimum over formats.
- **Type.** FLOOR+GRADED (floors HC-07, HC-17, HC-05).
- **Note.** Strictness lowers M19.1 by design (§4.3).

#### OD-20 Deterministic migration
- **M20.1** [`migration_identity_conformance_fraction` (binary)] Identity-rule conformance: the fraction of migrations whose LAI, PCR, AUX and signature-binding behavior matches SPEC §8.1 and F-08. Migrations are legacy → `.eb`, repack by profile, layout or index, key add and remove, export → re-import, and multi-target publish. Must be 1. Unit: fraction [`correctness`]. **M20.2** [`migration_determinism_fraction` (binary)] Migration output determinism: the fraction of deterministic-target migrations byte-identical across runs and hosts. Unit: fraction [`correctness`]. **M20.3** [`artifact_bytes`, `encode_wall_s`, `peak_rss_bytes`, `scratch_peak_bytes`] Migration cost: time, peak memory and scratch, and artifact byte delta relative to reference. Unit: ratio [`time_wall`, `memory_peak`, `scratch_peak`, `size`]. **M20.4** [`receipt_completeness_fraction` (binary)] Receipt completeness: the fraction of changed or dropped items recorded in receipts. Must be 1. Unit: fraction [`correctness`].
- **Direction.** Maximize M20.1, M20.2 and M20.4; minimize M20.3.
- **Aggregation.** AGG-C for M20.1, M20.2 and M20.4; AGG-G for M20.3.
- **Type.** FLOOR+GRADED (floors HC-10, HC-13, HC-07).

#### OD-21 Independent implementability
- **M21.1** [`cleanroom_pass_fraction` (binary)] Clean-room baseline pass rate (MVT-12(a)). Must be 1. Unit: fraction [`correctness`]. **M21.2** Independent-reader size for the baseline feature set [`independent_reader_loc`, cost input C2], counted with `tokei` at a pinned version, excluding tests. Unit: LoC [`other`]. **M21.3** Specification defects raised per 1,000 normative words during clean-room implementation [`spec_defects_per_1000_words`, descriptive]: minimizing an in-program count would reward not reporting defects. Unit: count per 1,000 words [`other`]. **M21.4** [`normative_rule_coverage_fraction` (binary)] Normative-rule coverage by conformance cases. Must be 1. Unit: fraction [`correctness`]. **M21.5** [`unspecified_baseline_decode_steps` (binary)] Baseline decode steps without a public specification or permanent vectors. Must be 0. Unit: count [`correctness`]. **M21.6** Reader fragmentation [`reader_fragmentation_count`, cost input C5]: the number of optional or extended features a candidate's archives may use that make them unreadable by a minimal baseline reader. Unit: count [`other`].
- **Direction.** M21.1 [`cleanroom_pass_fraction`] and M21.4 [`normative_rule_coverage_fraction`] must be 1 and M21.5 [`unspecified_baseline_decode_steps`] must be 0 (binary); M21.2 and M21.6 are cost inputs; M21.3 is descriptive.
- **Aggregation.** AGG-P.
- **Type.** FLOOR+GRADED (HC-12). M21.2 and M21.3 from a same-program implementer are biased low. [EXPERT_REVIEW_REQUIRED]

#### OD-22 Dependency longevity
- **M22.1** [`decode_path_dependency_count` (cost input)] Decode-path dependency count: direct plus transitive crates from `cargo tree -e normal` on the decode feature set with the pinned lockfile, counted separately from the create-only path. Unit: count [`other`]. **M22.2** [`decode_path_native_unsafe_count` (cost input)] Native (C/C++) code and `unsafe` blocks in decode-path dependencies (`cargo geiger` or equivalent). The workspace itself has `unsafe_code = "forbid"`. Unit: count [`other`]. **M22.3** [`license_compatibility` (binary), `rustsec_advisory_count` (cost input)] RustSec advisories and license compatibility (`cargo deny`). Unit: count, binary [`other`, `correctness`]. **M22.4** Maintenance snapshot per dependency: maintainer count [`maintainer_count`, cost input, maximized], MSRV gap [`msrv_gap`, cost input] and last release age [`release_age_years`, descriptive, because feature-complete crates legitimately stop releasing]. Recorded with retrieval date. Unit: count, versions, years [`other`]. [EXTERNALLY_SOURCED at retrieval] **M22.5** Public format specification availability for every decode-path codec or transform [`baseline_codec_public_spec`, binary for the baseline set]. Unit: binary per dependency [`correctness`].
- **Direction.** Minimize M22.1 [`decode_path_dependency_count`], M22.2 [`decode_path_native_unsafe_count`] and the advisory count of M22.3 [`rustsec_advisory_count`]; licence compatibility [`license_compatibility`] must pass; maximize maintainer count; maximize M22.5. All except the binary items are cost inputs to `decision-method.md` §7 (C3 and the maintenance penalty).
- **Aggregation.** AGG-P, with the decode path primary.
- **Type.** FLOOR+GRADED. M22.3 license must pass; M22.5 for baseline decode must pass (HC-12).
- **Note.** The snapshot tooling script is planned under `research/tools/`. [UNRESOLVED]

#### OD-23 Specification complexity
- **M23.1** Normative words per normative rule [`normative_words_per_rule`, descriptive], counted by script over the candidate specification (SPEC plus `docs/*-v1.md` normative sections) with the MVT-12(b) grammar. A raw word count is not minimized, because it would reward terse, underspecified text (HC-12). **M23.2** Record types, fields, feature bits, registry entries, reason codes and frozen identifiers, from a registry census [`wire_items_added`]. **M23.3** Delta of normative words [`normative_words_added`] and of M23.2 relative to REF-PROD: the permanent wire and specification cost of the candidate. Unit: count [`other`].
- **Direction.** M23.2 and M23.3 are cost inputs to `decision-method.md` §7 C1 and R10 key 2; M23.1 is descriptive.
- **Aggregation.** AGG-P.
- **Type.** GRADED. The minimum gains that justify added cost are set in `decision-method.md`.
- **Note.** Word and record counts are proxies for complexity. [INFERRED]

#### OD-24 Implementation attack surface
- **M24.1** [`untrusted_input_loc` (cost input)] Lines of code in functions reachable from untrusted bytes, across decode, import, remote and unlock paths. Unit: LoC [`other`]. **M24.2** Unresolved fuzz findings at a fixed coverage target [`unresolved_fuzz_findings_at_coverage_target`, binary = 0]: each parser target is fuzzed until it reaches a pre-registered edge-coverage target (or a 72 CPU-hour ceiling, reported); every crash, panic, timeout or out-of-memory finding is a defect under HC-03 or HC-06, not a graded rate, because a per-CPU-hour rate rewards weak fuzzing. Unit: count [`correctness`]. M24.1 [`untrusted_input_loc`], M24.3 [`attacker_controlled_parameters`] and M24.4 [`validated_only_vulnerability_classes`] are cost inputs to C4. **M24.3** [`attacker_controlled_parameters` (cost input)] Attacker-controlled parameters that influence allocation or CPU, from the MVT-06(c) audit. Unit: count [`other`]. **M24.4** [`validated_only_vulnerability_classes` (cost input)] SPEC §11.2 vulnerability classes the candidate handles only by "validated" rather than "not representable", "kernel-enforced" or "declared and bounded". Unit: count [`other`]. **M24.5** Superlinear structures [`superlinear_structure_count`, descriptive]: structures whose MVT-06(d) exponent interval upper bound exceeds 1 + δ. Unit: count [`other`].
- **Procedure.** cargo-fuzz campaigns in WSL; reachability via call-graph tooling; `tokei`.
- **Direction.** Minimize.
- **Aggregation.** AGG-P for M24.1, M24.3, M24.4 and M24.5; AGG-W (sum over targets) for M24.2.
- **Type.** FLOOR+GRADED. Any untrusted-input crash or panic is a defect that blocks `DECIDED` for the affected component until fixed (via HC-03 and HC-06). [EXPERT_REVIEW_REQUIRED for reachability completeness]

#### OD-25 User complexity
- **M25.1** [`task_success_rate` (heuristic), `task_steps` (heuristic), `task_errors` (heuristic)] First-use task success. Tasks: pack, unpack, verify, encrypt to a recipient, extract one entry, remote-read one entry, import a ZIP, export a tar, and explain what was not preserved. Each is scored against a pre-registered completion criterion. Unit: fraction, count [`other`]. **M25.2** [`flags_per_task` (cost input)] Flags and concepts required per task with defaults. Unit: count [`other`]. **M25.3** [`error_actionability_fraction`] Error actionability: the fraction of refusal outcomes whose message names the failed requirement and a remedy. Unit: fraction [`other`]. **M25.4** [`unsafe_defaults` (binary)] Unsafe defaults: defaults that enable a dangerous restoration, trust or overwrite. Must be 0. Unit: count [`correctness`]. **M25.5** [`task_completion_time_s` (heuristic)] Task completion time. Unit: s [`other`].
- **Procedure.** Human participants are unavailable. M25.2 [`flags_per_task`, cost input C6], M25.3 [`error_actionability_fraction`: a scripted census of refusal messages for a named failed requirement and a named remedy] and M25.4 [`unsafe_defaults`, binary] are mechanically checkable and may decide. M25.1 [`task_success_rate`, `task_steps`, `task_errors`] and M25.5 [`task_completion_time_s`] come from simulated first-use sessions by agents given only the CLI help and public docs; they are labeled SIMULATED, are **heuristic** (per-task success counts and qualitative failure modes; no CI-based verdicts), and never decide a claim about human comprehension, task success or preference, which requires external or human review (`decision-method.md` §4.16). Completion criteria for every task are committed before any session. Transcripts are committed.
- **Direction.** Maximize M25.3; minimize M25.2; M25.4 must be 0; M25.1 and M25.5 are reported.
- **Aggregation.** AGG-S (minimum over tasks) for M25.1; mean and maximum for M25.2.
- **Type.** FLOOR+GRADED (floors HC-05, HC-15).
- **Note.** Simulated results do not generalize to humans. [EXPERT_REVIEW_REQUIRED; UNRESOLVED]

#### OD-26 Adoption friction
- **M26.1** Zero-install consumability [`zero_install_scenario_fraction`, descriptive]: the fraction of consumer scenarios served with preinstalled tools via dual publishing of legacy artifacts (OD-19 M19.4). The scenarios are the consumer roles of the SPEC §21.1 staged path, enumerated: (S1) auditing a legacy ZIP or tar with `convert --strict --dry-run`, `verify --canonical`, `inspect --fidelity` and `diff --logical` (Stage 1); (S2) a build pipeline that packs to `.eb` internally and exports `.zip` or `.tar.zst` at publication (Stage 2); (S3) a consumer without Entrybound taking the legacy artifact of a dual publish (Stage 3); (S4) an Entrybound-aware installer or CI runner taking `release.eb` with partial retrieval and verified ranges (Stage 4); (S5) a consumer requesting the optional legacy fallback (Stage 5). Unit: fraction [`other`]. **M26.2** [`migration_steps_count` (descriptive)] Migration steps: commands and flags needed to replace an existing tar or zip workflow, from scripted reproductions in pinned Docker images. Unit: count [`other`]. **M26.3** [`build_availability` (descriptive)] Build and binary availability: successful builds and binaries on Windows x86-64, Linux x86-64 and emulated arm64; build time; MSRV. Unit: count, s [`other`]. **M26.4** Identification support [`identification_support_count`, descriptive]: status of the `.eb` extension, media type and magic in file(1)/libmagic, shared-mime-info and IANA, with retrieval date. Unit: count [`other`]. [EXTERNALLY_SOURCED] **M26.5** Single-file self-containment [`single_file_self_contained`, binary]: a `Complete` archive is one file that decodes and verifies offline with no sidecar, external dictionary or network access (MVT-09(c)); this is the portability half of CC-03. Unit: binary [`correctness`]. M26.2 is [`migration_steps_count`, descriptive] and M26.3 is [`build_availability`, descriptive].
- **Direction.** M26.5 must be 1; M26.1-M26.4 are reported (no adoption data exists, so they are proxies and never select).
- **Aggregation.** AGG-S (minimum over scenarios).
- **Type.** GRADED.
- **Note.** No real adoption data exists; this dimension is largely proxy. [INFERRED; UNRESOLVED]

---

## 4. Relationship between hard constraints and optimization dimensions

### 4.1 Lexicographic priority

HCs define the feasible set and ODs rank within it. The relationship is lexicographic, not weighted:
- No OD gain, however large, admits a candidate with a violation.
- No weight vector, sensitivity perturbation or workload reweighting touches an HC.
- HC outcomes are never inputs to Pareto or aggregate computations.

This follows from SPEC §23.5, where the invariant list is the gate for every feature, and from A11, where superiority is claimed only as the combination of properties, which fails if any invariant is traded. [FORMALLY_DERIVED]

### 4.2 Floors and graded remainders

Six ODs share a name with an HC: OD-01 and HC-01/03, OD-02 and HC-02, OD-04 and HC-03/11/15, OD-15 and HC-14, OD-17 and HC-06, and OD-18 and HC-08. In each pair the HC is the floor. The OD grades only what lies above it: the cost of providing the guarantee, the tightness of a declaration, the assurance level, or coverage and precision. It never grades *whether* to satisfy the floor. OD-02 has no remainder above its floor, so it is BINARY and appears only so the cost of the guarantee (M06.3) has a named home. [INFERRED]

| HC | ODs whose graded remainder sits above this floor |
|---|---|
| HC-01 | OD-01, OD-05 (dedup without a second authority) |
| HC-02 | OD-02, OD-05, OD-06 |
| HC-03 | OD-01, OD-04, OD-24 |
| HC-04 | OD-04, OD-15 |
| HC-05 | OD-19, OD-25 |
| HC-06 | OD-08, OD-09, OD-17, OD-24 |
| HC-07 | OD-03, OD-18, OD-19, OD-20, OD-26 |
| HC-08 | OD-18 |
| HC-09 | OD-05, OD-10, OD-14, OD-26 |
| HC-10 | OD-11, OD-20 |
| HC-11 | OD-04, OD-07, OD-13 |
| HC-12 | OD-21, OD-22 |
| HC-13 | OD-06, OD-13, OD-20 |
| HC-14 | OD-05, OD-12, OD-15, OD-16, OD-22 |
| HC-15 | OD-04, OD-07, OD-09, OD-10, OD-11, OD-12, OD-14, OD-25 |
| HC-16 | OD-23, OD-26 |
| HC-17 | OD-19 |
| HC-18 | OD-12, OD-20 |

### 4.3 Known tensions and where they are resolved

In every row the HC side is fixed. The trade-off among ODs is decided by `decision-method.md` (Pareto, practical significance, sensitivity, permanent cost). [INFERRED unless tagged]

| Tension | Fixed side | Traded among | Source |
|---|---|---|---|
| Solid-grade compression versus random access and corruption locality | HC-09 bounded lookback | OD-05 vs OD-10, OD-14 | SPEC §7.5, §23.4 ("a few percent versus solid") [FORMALLY_DERIVED] |
| Padding versus size | HC-14 keyed boundaries, no deterministic ciphertext, recorded padding mode with conforming public lengths | OD-16 (per padding mode) vs OD-05, OD-12 | SPEC §7.6, §9.8, §23.4 [FORMALLY_DERIVED] |
| Verified staging versus time to first output byte | HC-05 and HC-15 (F-11) | OD-07 M07.4, OD-09 M09.3 vs OD-08 scratch | CONTRIBUTING.md L137-140 [FORMALLY_DERIVED] |
| Parallel speed versus determinism | HC-11, HC-13 | OD-13 vs OD-06 | SPEC §12.2 (multithreaded codec output differs) [FORMALLY_DERIVED] |
| Import acceptance versus strictness | HC-07, HC-17, HC-05 | OD-19 M19.1 vs OD-26 | SPEC §2.1(2); CONTRIBUTING.md L185-193 [FORMALLY_DERIVED] |
| Write-time verification versus creation speed | HC-02 (I31) | OD-06 | SPEC §23.4 [FORMALLY_DERIVED] |
| Ratio gains from new codecs or transforms versus permanent cost | HC-12, HC-16 | OD-05 vs OD-21, OD-22, OD-23, OD-24 | SPEC §24 item 9, §20.6 |
| Richer preservation versus identity stability | HC-10 (LAI/AUX split) | OD-03 vs OD-20 | SPEC §8.3, §12.4 [FORMALLY_DERIVED] |
| Remote efficiency versus access-pattern privacy | HC-14 | OD-12 vs OD-16 M16.4 | SPEC §9.8 |
| Fewer user concepts versus declared honesty | HC-07, HC-15 | OD-25 vs OD-03 and OD-11 reporting detail | SPEC §23.4 ("more concepts than ZIP") [FORMALLY_DERIVED] |

### 4.4 Frozen identifiers and permanent cost

A candidate that would change frozen behavior is admissible only as a new identifier (HC-16); an in-place change is excluded at L0. Its permanent cost is charged even if it is later deprecated for new archives, because decoders must support it indefinitely (SPEC §12.2, §20.4). [FORMALLY_DERIVED] OD-21 to OD-24 are measured as the cost elements C1-C7 of `decision-method.md` §7.1 (crosswalk in its §7.6), and it is the resulting cost **tiers**, not OD-21 to OD-24 values, that drive R4, R6 and R10. [INFERRED]

### 4.5 What may and may not move an HC

This program cannot relax an HC. A finding that an HC imposes a large cost is reported as a negative result with the measured cost, attached to the relevant CC or OD. Three situations are escalated as `UNRESOLVED`, with evidence, to a SPEC revision process outside this program:
- two HCs are shown to be mutually unsatisfiable for some required capability (for example, HC-06 declared totals versus unseekable streaming input, which the SPEC already resolves through `BudgetDeclared = false`);
- an HC's MVT is shown to be ill-defined;
- a candidate proposes to drop decode support for an already-published identifier (HC-16).

[FORMALLY_DERIVED from SPEC §2.1(4), §23.5; INFERRED procedure]

### 4.6 Unverifiable HCs

An HC_UNVERIFIED status (§2.0 rule 4) can only narrow the *platform* scope of a decision. It never becomes a pass, and it never removes a property from scope. For example, a decision validated on Windows and Linux excludes macOS from `decision_scope` and states in `known_limitations` that macOS extraction confinement and metadata fidelity are unverified; a decision whose scope includes macOS stays `INSUFFICIENT_EVIDENCE`. [INFERRED]

---

## 5. Threats to this objective

1. **Taxonomy bias.** The HC set, the MVT oracles and the OD metric proxies were chosen by the program itself. The adversarial review exists to attack them. Any HC or OD added or changed after the first decision-relevant result invalidates pre-registration for the affected decisions. [INFERRED]
2. **Instrument gap.** Most MVTs and many ODs depend on instruments not built at drafting: the harness crates, the conformance corpus, the independent reader, the trace shim and leakage analyzers. Until they exist, those HCs are HC_UNVERIFIED for every candidate, including production. [FORMALLY_DERIVED from repository state at `f56dd3d`; UNRESOLVED]
3. **Ledger screen noise.** Appendix C's keyword mapping is heuristic, and false positives and negatives remain after manual review. It supplies anchors and weak-evidence pointers, not HC membership proofs. [INFERRED]
4. **Platform and participant blocks.** macOS, APFS, native ARM64, ReFS, admin-only Windows features, packet-level network emulation and human usability participants are unavailable, which affects HC-05, HC-07, HC-08, OD-03, OD-12, OD-18 and OD-25. [FORMALLY_DERIVED from PROGRESS.md; UNRESOLVED]
5. **Same-organization independence.** HC-12, OD-21 and every "independent" oracle are weakened when produced inside the program. [EXPERT_REVIEW_REQUIRED]
6. **Crypto assurance.** Conformance to the frozen suite is testable internally. Soundness of the constructions is not. [EXPERT_REVIEW_REQUIRED]
7. **Held-out identity exposure.** Held-out item identities, sources and generator inputs are readable in the repository, `research/PROGRESS.md` change-log entries name the upstream sources of several held-out items, and at least three sessions (the round-1 reviewer and both round-1 revision sessions, `decision-method.md` §0.1) have seen held-out item names. `decision-method.md` §5.4 therefore adopts a result-blind, identity-aware threat model with a confidence cap, plus a sealing route that lifts the cap. [FORMALLY_DERIVED from the review's disclosure and `decision-method.md` §0.1; UNRESOLVED]
8. **Shared base model.** The adversarial reviewer and the drafting sessions share a base model, so internal review is not independent expert review, and correlated blind spots are likely. [INFERRED]

---

## Appendix A — Invariant coverage (I1-I31 to HC)

Every invariant of SPEC §3.9 and Appendix A maps to at least one HC, and every HC maps to at least one invariant or named freeze. **The §2.1 invariant column is the single source:** the "HCs" column below (P primary, S supporting) and the reverse check are generated from it, and the consistency check at the end of this appendix must report no difference before any revision is committed. The "Ledger anchors" column lists rows whose text names the invariant or its SPEC alias (Appendix C.1). "Enforced by" is the SPEC Appendix A column. [FORMALLY_DERIVED]

| Inv. | Invariant (SPEC Appendix A) | Enforced by (SPEC) | HCs | Ledger anchors |
|---|---|---|---|---|
| I1 | One semantic authority per fact | Model §3.2 | HC-01 (P) | REQ-MOD-0018, REQ-MOD-0099 |
| I2 | No duplicated semantic headers | Container §4.1 | HC-01 (P) | REQ-CON-0081 |
| I3 | Unique logical paths | Manifest §3.4 | HC-03 (P), HC-17 (P) | REQ-MOD-0039, REQ-MOD-0111 |
| I4 | Paths are structural sequences | Model §3.4 | HC-08 (P), HC-17 (P) | REQ-MOD-0075, REQ-MOD-0077, REQ-MOD-0080, REQ-PLT-0100, REQ-PLT-0132 |
| I5 | No `..` or `.` | Model §3.4 | HC-08 (P), HC-17 (P) | REQ-MOD-0026 |
| I6 | No duplicate entries | Manifest | HC-03 (P), HC-17 (P) | REQ-MOD-0039, REQ-MOD-0111 |
| I7 | Explicit entry identity | Model §3.2 | HC-10 (P) | REQ-MOD-0031 |
| I8 | Compression independent of logical identity | Identity §8.1 | HC-10 (P), HC-13 (P) | REQ-MOD-0013, REQ-MOD-0060 |
| I9 | Encryption independent of logical identity | Identity §8.1 | HC-10 (P), HC-13 (P), HC-14 (P) | REQ-CRY-0077, REQ-MOD-0060 |
| I10 | Indexes are reconstructible caches | Container §4.5 | HC-01 (P), HC-09 (S) | REQ-CON-0050 |
| I11 | Typed, namespaced metadata | Metadata §10 | HC-04 (P), HC-07 (P) | REQ-PLT-0068 |
| I12 | Unknown critical features fail closed | Feature bitmaps §20.1 | HC-04 (P), HC-12 (P), HC-14 (S) | REQ-CON-0096 |
| I13 | No silent metadata loss | Fidelity Report §10.9 | HC-07 (P) | REQ-CRY-0033, REQ-PLT-0079 |
| I14 | No lossy transforms | Losslessness contract §6.5 | HC-02 (P), HC-07 (P) | REQ-CMP-0077, REQ-LEG-0101 |
| I15 | Deterministic, unique interpretation | Canonical form §11.4 | HC-03 (P), HC-11 (P), HC-12 (P), HC-16 (P), HC-13 (S) | REQ-MOD-0110 |
| I16 | Extraction policy is caller-owned | Security §11.8 | HC-05 (P), HC-18 (S) | REQ-MOD-0001, REQ-PLT-0007 |
| I17 | Resource limits declared and validated pre-allocation, or inability declared | Preamble §4.2, §11.5 | HC-06 (P), HC-05 (S) | REQ-ACC-0155, REQ-CON-0005 |
| I18 | Legacy ambiguity surfaced, never normalized | Observation model §14.2 | HC-07 (P) | REQ-LEG-0056 |
| I19 | Every size, offset and count declared once | Container §4.6 | HC-01 (P), HC-03 (S), HC-06 (S) | REQ-ACC-0019, REQ-CON-0081, REQ-MOD-0056, REQ-MOD-0076 |
| I20 | No unbounded list parsed from a header | Container §4.2, §10.7 | HC-06 (P) | REQ-ACC-0155, REQ-CON-0004, REQ-MOD-0056 |
| I21 | Truncation, corruption and unsupported are distinguishable | Preamble and footer §4.2, §4.4 | HC-04 (P), HC-15 (P), HC-03 (S), HC-12 (S), HC-18 (S) | REQ-CON-0032, REQ-CON-0091, REQ-INT-0024 |
| I22 | Published identity binds parameters, not a bare root | Identity §8.3 | HC-10 (P), HC-16 (P) | REQ-MOD-0047 |
| I23 | Hard links are content sharing | Model §3.3 | HC-01 (P), HC-17 (P), HC-10 (S) | REQ-MOD-0043 |
| I24 | Encrypted chunk boundaries must not leak plaintext structure | Chunking policy §7.6 | HC-14 (P) (residual graded in OD-16 per padding mode) | REQ-CRY-0097 |
| I25 | Unverified results reported as unverified | Reader contract §11.4, §11.6, §20.5 | HC-14 (P), HC-15 (P), HC-18 (P), HC-05 (S) | REQ-ECO-0177, REQ-INT-0025, REQ-PLT-0014 |
| I26 | No non-Directory proper prefix; explicit ancestors | Model §3.4 P6 | HC-03 (P), HC-17 (P), HC-08 (S) | REQ-MOD-0039, REQ-PLT-0023 |
| I27 | `.` and `..` excluded under every encoding | Model §3.4 P7 | HC-08 (P), HC-17 (P) | REQ-MOD-0026, REQ-MOD-0077, REQ-PLT-0150 |
| I28 | Content addressing over plaintext; LAI chunking-independent, PCR not | Model §3.5, identity §8.0 | HC-02 (P), HC-10 (P), HC-13 (P) | REQ-MOD-0013, REQ-MOD-0060 |
| I29 | Chunks independently decodable unless in a declared-lookback group | Model §3.5, grouping §7.5 | HC-09 (P), HC-06 (S) | REQ-ACC-0040, REQ-CMP-0078 |
| I30 | Transform plans are data | Model §3.6 | HC-11 (P), HC-12 (P), HC-16 (P), HC-02 (S), HC-09 (S), HC-13 (S) | REQ-CMP-0117 |
| I31 | Every reconstructive transform round-trip verified at write time | Planner §5.6 | HC-02 (P), HC-11 (S) | REQ-CMP-0079 |

Reverse check, generated from the §2.1 invariant column by the command below (P = primary, S = supporting): HC-01 primary {I1, I2, I10, I19, I23}; HC-02 primary {I14, I28, I31}, supporting {I30}; HC-03 primary {I3, I6, I15, I26}, supporting {I19, I21}; HC-04 primary {I11, I12, I21}; HC-05 primary {I16}, supporting {I17, I25}; HC-06 primary {I17, I20}, supporting {I19, I29}; HC-07 primary {I11, I13, I14, I18}; HC-08 primary {I4, I5, I27}, supporting {I26}; HC-09 primary {I29}, supporting {I10, I30}; HC-10 primary {I7, I8, I9, I22, I28}, supporting {I23}; HC-11 primary {I15, I30}, supporting {I31}; HC-12 primary {I12, I15, I30}, supporting {I21}; HC-13 primary {I8, I9, I28}, supporting {I15, I30}; HC-14 primary {I9, I24, I25}, supporting {I12}; HC-15 primary {I21, I25}; HC-16 primary {I15, I22, I30}; HC-17 primary {I3, I4, I5, I6, I23, I26, I27}; HC-18 primary {I25}, supporting {I16, I21}. Every HC has at least one primary invariant, and all 31 invariants appear. [FORMALLY_DERIVED]

Consistency check (run from the repository root; output at this revision: `differences: none | HCs 18 | invariants 31`):

```sh
C:/Python313/python.exe - <<'EOF'
import re, collections
t = open('research/archetypal-objective.md', encoding='utf-8').read().splitlines()
s21 = {}
for l in t:
    m = re.match(r'\| (HC-\d\d) \| [^|]+\| ([^|]+)\| F-', l)
    if m:
        parts = m.group(2).split(';')
        s21[m.group(1)] = (set(re.findall(r'I\d+', parts[0])), set(re.findall(r'I\d+', parts[1])) if len(parts) > 1 else set())
appA = collections.defaultdict(lambda: (set(), set()))
for l in t:
    m = re.match(r'\| (I\d+) \| [^|]+\| [^|]+\| ([^|]+)\|', l)
    if m and 'HC' in m.group(2):
        for h, role in re.findall(r'(HC-\d\d) \((P|S)\)', m.group(2)):
            appA[h][0 if role == 'P' else 1].add(m.group(1))
bad = [(h, s21[h], appA[h]) for h in sorted(set(s21) | set(appA)) if s21.get(h) != appA.get(h)]
print('differences:', bad if bad else 'none', '| HCs', len(s21), '| invariants', len(set().union(*[p | q for p, q in s21.values()])))
EOF
```

## Appendix B — Repository freeze register

Freezes are cited as F-nn by the HCs. Line numbers refer to `CONTRIBUTING.md` at SHA-256 `ab28c1ce…16cc` unless another file is named. A freeze may be extended only by allocating a new identifier (HC-16). [FORMALLY_DERIVED]

| F | Freeze | Source |
|---|---|---|
| F-01 | Single-authority framing: a Chunk frame's `stored_length` is the sole declaration of its extent; no second length on the enclosing item or a trailing record; STREAM has no Index to reconcile | L116-119 |
| F-02 | ChunkGroup membership authority only on `Chunk.group_ref`; never a duplicated authoritative member list | L82-83 |
| F-03 | Planner freezes by planner ID: v1 candidate levels, probe, cost rule and parameter encoding; v3 `bottom-k-shingle-v1`, cohort bounds, dictionary sampling and caps, trainer IDs, dictionary and lookback candidates, strict gain rule, bounded access calculation; v4 closed codec/transform registries, parameters, structural transforms, eligibility probe, complete-cost rule, candidate sets (`docs/codec-transform-v1.md`); v5 Structural/Reconstructive classes, preflate version and bounds, DEFLATE wrapper eligibility, mandatory dual round trip, ReconstructionData encoding (`docs/reconstructive-transform-v1.md`); v6 whole-ContentObject ReconstructionRegion, `jixel`/`jxl-oxide` versions, single-thread encoder, dedup eligibility, mandatory exact round trip, access declarations (`docs/jpeg-reconstruction-v1.md`) | L67-98 |
| F-04 | Chunker frozen by `chunker_id`: `gear-norm-v1` table-generation formula, masks, min/target/max; exact dedup equality is plaintext SHA-256 | L72-77; `docs/chunking-v1.md` |
| F-05 | Decoder behavior depends only on recorded TransformPlans, never on the creation profile | L69-71 |
| F-06 | `stream-layout-v1`: STREAM_BODY item tags and order, record-item and Chunk-frame framing, footer layout, dedup-window definition; changes need a new layout feature bit | L99-103; `docs/stream-layout-v1.md` |
| F-07 | Layout equivalence: INDEXED and STREAM agree on entries, ContentObjects, Chunks, declared bounds, LAI, PCR and AUX; only PCI and `ContentStore::physical_order` may differ; assertions live in `tests/stream_layout.rs` | L104-109 |
| F-08 | Identity tiers in tooling: representation-only repack keeps LAI, AUX and PCR equal; explicit replanning keeps LAI and AUX equal; `NOT_VERIFIED`/`NOT_COMPUTED` for unfetched bytes; explanation claims classed RECORDED, DERIVED, AUDIT or NOT_RECORDED | L110-114 |
| F-09 | Sequential I/O bounds: writer usable with a `Write`-only sink, reader with a `Read`-only source; no whole-source buffering; no API that looks random-access while hiding a full scan | L121-126 |
| F-10 | Random INDEXED access: Index and EncryptedIndex are locator caches; bounded dependencies computed before release; `whole_archive_verified=false` unless fully read; PCR and PCI never claimed from unread bytes; one strong ETag; exact 206/Content-Range; no hidden 200 fallback | L128-135; `docs/random-access-v1.md`, `docs/http-range-access-v1.md` |
| F-11 | Verified staging: sequential extraction stages in bounded spilling storage and materializes only after whole-archive verification; no plaintext from an encrypted archive before full authentication and EAM/identity verification; encrypted STREAM fails before any output | L137-140, L177-180 |
| F-12 | Crypto v1 closed suite: T1 fields, HKDF labels, recipient wrapping, segment nonces, associated data, padding buckets, boundary derivations, feature and record assignments; `entrybound::crypto` has no locally written primitives and no user-selectable algorithms; pinned RustCrypto, X-Wing, Argon2 and zeroize; RFC, draft-10, V1-V7 and RFC 8032 vectors are the external-conformance gate | L142-149; `docs/crypto-suite-v1.md`, `docs/crypto-wire-v1.md` |
| F-13 | Signatures: canonical type 26 with three independent CONTENT/PHYSICAL/ADDRESSING transcripts; never PCI for PCR; cryptographic validity never merged with binding freshness; embedded only in encrypted EBCS kind 7; detached `.ebsig` for unencrypted archives; offline SHA-256/Ed25519 timestamp verification bounded to 64 KiB DER with caller trust anchors | L151-156 |
| F-14 | Recipient mutation: add preserves AFK, archive ID and PAYLOAD ciphertext and staleness is ADDRESSING only; removal or password change rotates the AFK, re-runs keyed chunking and planning, and self-verifies a replacement; stale signatures are never silently deleted or re-signed | L158-163 |
| F-15 | Descriptor: v1 is type 1/version 1 with tags 1-8 permanently; v2 (tags 1-19) only for corrected encrypted INDEXED with feature `0x1000`; no second resource or decode record; Descriptor v2 authenticated and caller-checked before dependent objects; legacy encrypted Descriptor-v1 input is compatibility-only and writers may not emit it | L165-173; `docs/descriptor-vectors-v1.txt` |
| F-16 | Secret hygiene and pre-work bounds: no `Debug`/`Display` for secrets; attacker-controlled KDF, stanza, segment, record and allocation bounds checked before expensive work | L175-177 |
| F-17 | POSIX metadata v1 (`0x8000`) and platform security metadata v1 (`0x10000`, requires `0x8000`): Entry/MetadataItem version bytes permanent; symlink targets and executable state are LAI; mode, ownership, hardlink topology, xattrs and sparse layout are AUX; ACL, Windows and macOS metadata are AUX; opaque ReparsePoint tag and data are LAI; never infer hardlinks from equal content, never serialize device or inode numbers, never infer holes from zero runs, never follow source symlinks; no unsafe code and no partial-fidelity claims | L44-65; `docs/posix-metadata-v1.md`, `docs/security-metadata-v1.md` |
| F-18 | Conformance discipline: generated inputs as format-building code, not opaque fixtures; each negative case asserts a stable reason code; filesystem tests use generated trees; extraction changes need collision, containment, preflight-integrity and resource-policy tests | L40-42, L182-183 |
| F-19 | Legacy import boundary: LOM evidence before EAM projection; foreign declarations never directly authoritative; strict refuses Divergence and Irreconcilable; automatic Omission and Refinement recorded as canonical provenance affecting AUX only, never LAI or PCR; caller-owned limits enforced | L185-193 |
| F-20 | ZIP policies: strict, compatibility and preservation consume one immutable `ZipObservation`; compatibility IDs include the exact probed runtime version; reference runtimes only in `tools/zip-compat`; compatibility never bypasses path, extent, collision, special-file or resource safety; preservation `0x4000` requires `0x2000` and may change AUX and PCI, never LAI or PCR | L194-202 |
| F-21 | Tar, transport and 7z: composition of one wrapper observer and one tar observer, no `tar.gz` semantic parser; layer-relative evidence bound by decoded-child SHA-256; limits enforced during decode; 7z structure parsed by production code, solid folders decoded once with each output byte assigned once; new coders require a frozen method contract, pre-allocation resource derivation, adversarial cases and differential evidence | L204-221; `docs/7z-import-v1.md` |
| F-22 | Legacy export and publishing: export consumes EAM; representability preflight before output; exact versioned target profiles (`zip/portable-v1`, `tar/pax-v1`, compressed variants); LOSSLESS, LOSSY (approval) or REFUSED (no override); strict re-import before publication; ExportReceipt v1 bytes frozen; multi-target publish is all-or-nothing with rollback | L223-247; `docs/legacy-export-v1.md`, `docs/migration-workflows-v1.md` |
| F-23 | Workspace lint `unsafe_code = "forbid"`; new dependencies require a concrete format or implementation need | `Cargo.toml` L11-12; CONTRIBUTING.md L26-27 |
| F-24 | Research isolation: experiments do not change production semantics; research features in production crates are default-off and proven byte-identical when disabled | PROGRESS.md standing decision 2026-09-12 |
| F-25 | SPEC §24 non-goals: no Entrybound transport protocol; not a key-management system; verification never requires live infrastructure; no lossy mode; in-archive error correction permanently refused | SPEC §24 |
| F-26 | Identifiers never reused; retired features and suites keep their identifiers forever | SPEC §20.3-§20.4 |
| F-27 | Exact name bytes stored and never normalized; target-hostile bytes legal in the archive and handled by the extractor | SPEC §3.4 P5/P8, §10.8 |
| F-28 | Native tooling v1 adds no wire records or feature bits; representation-only repack preserves LAI, AUX and PCR; explicit v6 replanning preserves LAI and AUX; inspection and diff reports are external versioned JSON, not archive authorities | `docs/format-v0.md` L477-481 |
| F-29 | Verified random access v1 is an access API and verification doctrine, not an ECF wire feature | `docs/random-access-v1.md` L3-4 |

The register is extended whenever the constraint crosswalk (`decision-method.md` R1) classifies a ledger constraint string as a freeze not listed here; each extension is a revision under `decision-method.md` §0.2 and adds the new F-number to the affected HC rows. [INFERRED]

## Appendix C — Requirement-ledger screen (generated)

**Method.**
1. Explicit references to I1-I31 and to the SPEC aliases P1-P8, C1-C2 and E1-E3 map through Appendix A to HCs. T1 and T2 are not screened, because the ledger uses "T1" for the crypto-v1 tuple transcript encoding.
2. Keyword regular expressions per HC are applied.
3. Manual assignments cover rows the screens missed, and manual exclusions remove keyword false positives found on review.
4. Rows without an HC tag are screened against OD keywords.

The rules live in `research/tools/ledger/objective_screen.py`. Everything below this paragraph is that tool's output, verbatim. **Known limitation (review finding C03):** C.1 counts only ledger constraint strings that are exactly an `I<n>` token, so strings such as "I15 unique deterministic interpretation" are not counted; the "Decisions citing it" column is a lower bound. The screen's pattern is corrected to `I([1-9]|[12][0-9]|3[01])` inside strings when the constraint crosswalk tooling is built (`decision-method.md` §14 item 11), and this appendix is regenerated then. HC membership for decisions comes from the crosswalk and the ledger `hc_ids` field, never from this screen. [FORMALLY_DERIVED counts at the stated hashes; INFERRED keyword membership]

<!-- BEGIN GENERATED: objective_screen.py -->
Inputs: `research/requirement-ledger.csv` SHA-256 `306bf48bb72a22d3de757005e25f388e237f54e8c0b82950ed85e8c3aa69b833`; `research/decision-ledger.jsonl` SHA-256 `98b5401d79c621fab8db2a72e4825dcef9d74dc60de042fdb3990c46daa4068b`.

C.1 Explicit invariant references (rows whose text names the invariant or its SPEC alias) and decision-ledger rows listing the invariant in `hard_constraints`.

| Invariant | Ledger rows naming it | Decisions citing it | HCs (Appendix A) |
|---|---|---|---|
| I1 | REQ-MOD-0018, REQ-MOD-0099 | 36 | HC-01 |
| I2 | REQ-CON-0081 | 2 | HC-01 |
| I3 | REQ-MOD-0039, REQ-MOD-0111 | 14 | HC-17, HC-03 |
| I4 | REQ-MOD-0075, REQ-MOD-0077, REQ-MOD-0080, REQ-PLT-0100, REQ-PLT-0132 | 11 | HC-08, HC-17 |
| I5 | REQ-MOD-0026 | 10 | HC-17, HC-08 |
| I6 | REQ-MOD-0039, REQ-MOD-0111 | 5 | HC-17, HC-03 |
| I7 | REQ-MOD-0031 | 8 | HC-10 |
| I8 | REQ-MOD-0013, REQ-MOD-0060 | 17 | HC-10, HC-13 |
| I9 | REQ-CRY-0077, REQ-MOD-0060 | 20 | HC-10, HC-13, HC-14 |
| I10 | REQ-CON-0050 | 20 | HC-01 |
| I11 | REQ-PLT-0068 | 14 | HC-07, HC-04 |
| I12 | REQ-CON-0096 | 36 | HC-04 |
| I13 | REQ-CRY-0033, REQ-PLT-0079 | 83 | HC-07 |
| I14 | REQ-CMP-0077, REQ-LEG-0101 | 19 | HC-02, HC-07 |
| I15 | REQ-MOD-0110 | 94 | HC-03, HC-11, HC-16 |
| I16 | REQ-MOD-0001, REQ-PLT-0007 | 70 | HC-05 |
| I17 | REQ-ACC-0155, REQ-CON-0005 | 60 | HC-06 |
| I18 | REQ-LEG-0056 | 75 | HC-07 |
| I19 | REQ-ACC-0019, REQ-CON-0081, REQ-MOD-0056, REQ-MOD-0076 | 29 | HC-01, HC-06 |
| I20 | REQ-ACC-0155, REQ-CON-0004, REQ-MOD-0056 | 43 | HC-06 |
| I21 | REQ-CON-0032, REQ-CON-0091, REQ-INT-0024 | 56 | HC-15, HC-04 |
| I22 | REQ-MOD-0047 | 35 | HC-10, HC-16 |
| I23 | REQ-MOD-0043 | 10 | HC-01, HC-17 |
| I24 | REQ-CRY-0097 | 20 | HC-14 |
| I25 | REQ-ECO-0177, REQ-INT-0025, REQ-PLT-0014 | 131 | HC-15 |
| I26 | REQ-MOD-0039, REQ-PLT-0023 | 5 | HC-17, HC-03 |
| I27 | REQ-MOD-0026, REQ-MOD-0077, REQ-PLT-0150 | 6 | HC-17, HC-08 |
| I28 | REQ-MOD-0013, REQ-MOD-0060 | 33 | HC-10, HC-02 |
| I29 | REQ-ACC-0040, REQ-CMP-0078 | 7 | HC-09, HC-06 |
| I30 | REQ-CMP-0117 | 19 | HC-11, HC-12, HC-16 |
| I31 | REQ-CMP-0079 | 8 | HC-02 |

C.2 HC screen. Weak = evidence_state CONTRADICTED or INSUFFICIENT, or implementation_state MISSING.

| HC | Rows (INVARIANT / REQUIREMENT) | INVARIANT rows | Weak rows |
|---|---|---|---|
| HC-01 | 17 / 10 | REQ-ACC-0023, REQ-ACC-0044, REQ-ACC-0129, REQ-CMP-0017, REQ-CMP-0018, REQ-CMP-0138, REQ-CMP-0146, REQ-CON-0050, REQ-CON-0081, REQ-INT-0005, REQ-LEG-0096, REQ-LEG-0126, REQ-LEG-0128, REQ-MOD-0033, REQ-MOD-0043, REQ-MOD-0076, REQ-MOD-0099 | 2: REQ-ECO-0183, REQ-MOD-0018 |
| HC-02 | 13 / 12 | REQ-CMP-0077, REQ-CMP-0079, REQ-CMP-0153, REQ-INT-0037, REQ-INT-0049, REQ-INT-0052, REQ-LEG-0014, REQ-LEG-0047, REQ-LEG-0101, REQ-MOD-0013, REQ-MOD-0024, REQ-MOD-0060, REQ-MOD-0066 | 6: REQ-CMP-0130, REQ-LEG-0116, REQ-LEG-0119, REQ-LEG-0281, REQ-MOD-0004, REQ-PLT-0141 |
| HC-03 | 17 / 9 | REQ-ACC-0128, REQ-ACC-0129, REQ-ACC-0161, REQ-CMP-0018, REQ-CON-0051, REQ-CON-0076, REQ-ECO-0021, REQ-INT-0031, REQ-LEG-0152, REQ-LEG-0184, REQ-LEG-0229, REQ-MOD-0031, REQ-MOD-0039, REQ-MOD-0110, REQ-MOD-0111, REQ-PLT-0002, REQ-PLT-0099 | 8: REQ-CRY-0057, REQ-ECO-0021, REQ-LEG-0100, REQ-LEG-0169, REQ-MOD-0008, REQ-MOD-0063, REQ-MOD-0110, REQ-PLT-0002 |
| HC-04 | 13 / 6 | REQ-CMP-0161, REQ-CON-0032, REQ-CON-0054, REQ-CON-0076, REQ-CON-0096, REQ-CRY-0040, REQ-CRY-0079, REQ-CRY-0243, REQ-INT-0024, REQ-LEG-0193, REQ-MOD-0017, REQ-PLT-0068, REQ-PLT-0144 | 2: REQ-PLT-0064, REQ-PLT-0144 |
| HC-05 | 13 / 30 | REQ-ACC-0044, REQ-ACC-0138, REQ-CON-0005, REQ-CRY-0243, REQ-LEG-0126, REQ-MOD-0001, REQ-MOD-0075, REQ-PLT-0007, REQ-PLT-0030, REQ-PLT-0071, REQ-PLT-0105, REQ-PLT-0110, REQ-PLT-0128 | 19: REQ-CON-0005, REQ-INT-0061, REQ-MOD-0062, REQ-PLT-0014, REQ-PLT-0015, REQ-PLT-0017, REQ-PLT-0031, REQ-PLT-0035, REQ-PLT-0056, REQ-PLT-0076, REQ-PLT-0083, REQ-PLT-0092, REQ-PLT-0093, REQ-PLT-0097, REQ-PLT-0110, REQ-PLT-0112, REQ-PLT-0115, REQ-PLT-0148, REQ-PLT-0156 |
| HC-06 | 11 / 29 | REQ-ACC-0149, REQ-CMP-0078, REQ-CON-0004, REQ-CON-0005, REQ-CON-0035, REQ-CON-0081, REQ-CRY-0139, REQ-CRY-0155, REQ-INT-0041, REQ-MOD-0076, REQ-PLT-0007 | 11: REQ-CMP-0033, REQ-CMP-0054, REQ-CON-0004, REQ-CON-0005, REQ-CON-0006, REQ-CON-0035, REQ-CON-0079, REQ-LEG-0117, REQ-MOD-0062, REQ-MOD-0088, REQ-PLT-0157 |
| HC-07 | 38 / 44 | REQ-ACC-0014, REQ-CMP-0077, REQ-CMP-0102, REQ-CON-0051, REQ-CON-0067, REQ-CRY-0033, REQ-ECO-0011, REQ-ECO-0075, REQ-INT-0022, REQ-INT-0036, REQ-INT-0037, REQ-INT-0049, REQ-LEG-0014, REQ-LEG-0047, REQ-LEG-0048, REQ-LEG-0056, REQ-LEG-0096, REQ-LEG-0101, REQ-LEG-0127, REQ-LEG-0149, REQ-LEG-0152, REQ-LEG-0171, REQ-LEG-0184, REQ-LEG-0205, REQ-LEG-0210, REQ-LEG-0225, REQ-LEG-0269, REQ-MOD-0005, REQ-MOD-0039, REQ-MOD-0093, REQ-PLT-0002, REQ-PLT-0033, REQ-PLT-0068, REQ-PLT-0071, REQ-PLT-0079, REQ-PLT-0099, REQ-PLT-0105, REQ-PLT-0128 | 26: REQ-CMP-0162, REQ-CON-0011, REQ-CON-0067, REQ-CRY-0004, REQ-CRY-0033, REQ-ECO-0074, REQ-ECO-0128, REQ-ECO-0130, REQ-ECO-0133, REQ-INT-0036, REQ-INT-0067, REQ-LEG-0056, REQ-LEG-0062, REQ-LEG-0112, REQ-LEG-0116, REQ-LEG-0207, REQ-LEG-0281, REQ-MOD-0062, REQ-MOD-0068, REQ-PLT-0002, REQ-PLT-0011, REQ-PLT-0036, REQ-PLT-0039, REQ-PLT-0079, REQ-PLT-0123, REQ-PLT-0136 |
| HC-08 | 9 / 17 | REQ-CMP-0038, REQ-LEG-0149, REQ-LEG-0192, REQ-LEG-0205, REQ-LEG-0240, REQ-MOD-0026, REQ-MOD-0075, REQ-MOD-0080, REQ-PLT-0053 | 13: REQ-CRY-0164, REQ-ECO-0054, REQ-LEG-0117, REQ-LEG-0120, REQ-LEG-0192, REQ-MOD-0018, REQ-MOD-0077, REQ-MOD-0079, REQ-PLT-0036, REQ-PLT-0074, REQ-PLT-0100, REQ-PLT-0132, REQ-PLT-0150 |
| HC-09 | 6 / 10 | REQ-ACC-0101, REQ-ACC-0161, REQ-CMP-0017, REQ-CMP-0078, REQ-CRY-0247, REQ-INT-0010 | 6: REQ-ACC-0001, REQ-CMP-0039, REQ-CMP-0127, REQ-CMP-0176, REQ-ECO-0033, REQ-ECO-0103 |
| HC-10 | 32 / 42 | REQ-ACC-0096, REQ-ACC-0144, REQ-CMP-0106, REQ-CON-0050, REQ-CRY-0033, REQ-CRY-0077, REQ-CRY-0129, REQ-INT-0002, REQ-INT-0005, REQ-INT-0022, REQ-INT-0036, REQ-INT-0039, REQ-INT-0041, REQ-INT-0049, REQ-INT-0051, REQ-INT-0070, REQ-LEG-0014, REQ-LEG-0047, REQ-LEG-0048, REQ-LEG-0261, REQ-MOD-0005, REQ-MOD-0013, REQ-MOD-0030, REQ-MOD-0031, REQ-MOD-0047, REQ-MOD-0048, REQ-MOD-0060, REQ-MOD-0066, REQ-MOD-0067, REQ-MOD-0081, REQ-MOD-0093, REQ-PLT-0106 | 28: REQ-ACC-0077, REQ-ACC-0111, REQ-ACC-0168, REQ-CON-0065, REQ-CON-0074, REQ-CON-0075, REQ-CON-0099, REQ-CRY-0033, REQ-CRY-0060, REQ-CRY-0150, REQ-ECO-0090, REQ-INT-0020, REQ-INT-0028, REQ-INT-0036, REQ-INT-0061, REQ-INT-0067, REQ-LEG-0017, REQ-LEG-0119, REQ-LEG-0124, REQ-LEG-0160, REQ-LEG-0174, REQ-MOD-0004, REQ-MOD-0046, REQ-MOD-0052, REQ-MOD-0063, REQ-MOD-0068, REQ-PLT-0124, REQ-PLT-0150 |
| HC-11 | 7 / 3 | REQ-CMP-0023, REQ-CMP-0044, REQ-CMP-0117, REQ-CMP-0146, REQ-CMP-0153, REQ-CMP-0161, REQ-MOD-0110 | 2: REQ-MOD-0063, REQ-MOD-0110 |
| HC-12 | 3 / 5 | REQ-CMP-0117, REQ-ECO-0021, REQ-ECO-0030 | 7: REQ-CMP-0030, REQ-CMP-0124, REQ-CRY-0252, REQ-ECO-0021, REQ-ECO-0030, REQ-ECO-0072, REQ-ECO-0136 |
| HC-13 | 11 / 31 | REQ-CMP-0023, REQ-CMP-0038, REQ-CON-0055, REQ-CRY-0077, REQ-CRY-0128, REQ-LEG-0038, REQ-LEG-0047, REQ-LEG-0128, REQ-MOD-0013, REQ-MOD-0030, REQ-MOD-0060 | 18: REQ-ACC-0082, REQ-CMP-0001, REQ-CMP-0033, REQ-CMP-0054, REQ-CMP-0108, REQ-CMP-0128, REQ-CMP-0131, REQ-CON-0055, REQ-ECO-0038, REQ-ECO-0058, REQ-ECO-0131, REQ-ECO-0190, REQ-INT-0072, REQ-INT-0074, REQ-LEG-0100, REQ-MOD-0008, REQ-MOD-0020, REQ-MOD-0063 |
| HC-14 | 32 / 25 | REQ-ACC-0167, REQ-CRY-0001, REQ-CRY-0008, REQ-CRY-0013, REQ-CRY-0016, REQ-CRY-0027, REQ-CRY-0033, REQ-CRY-0040, REQ-CRY-0077, REQ-CRY-0079, REQ-CRY-0096, REQ-CRY-0097, REQ-CRY-0111, REQ-CRY-0114, REQ-CRY-0124, REQ-CRY-0125, REQ-CRY-0126, REQ-CRY-0128, REQ-CRY-0129, REQ-CRY-0139, REQ-CRY-0144, REQ-CRY-0155, REQ-CRY-0172, REQ-CRY-0199, REQ-CRY-0225, REQ-CRY-0243, REQ-CRY-0245, REQ-CRY-0247, REQ-ECO-0029, REQ-INT-0034, REQ-MOD-0030, REQ-MOD-0060 | 22: REQ-CRY-0001, REQ-CRY-0016, REQ-CRY-0033, REQ-CRY-0035, REQ-CRY-0060, REQ-CRY-0082, REQ-CRY-0085, REQ-CRY-0086, REQ-CRY-0097, REQ-CRY-0111, REQ-CRY-0164, REQ-CRY-0166, REQ-CRY-0167, REQ-CRY-0168, REQ-CRY-0195, REQ-CRY-0211, REQ-CRY-0212, REQ-CRY-0213, REQ-CRY-0252, REQ-CRY-0255, REQ-CRY-0256, REQ-INT-0014 |
| HC-15 | 18 / 37 | REQ-ACC-0014, REQ-ACC-0096, REQ-ACC-0128, REQ-ACC-0138, REQ-CON-0032, REQ-INT-0024, REQ-INT-0025, REQ-INT-0039, REQ-INT-0051, REQ-INT-0052, REQ-INT-0054, REQ-INT-0057, REQ-INT-0063, REQ-INT-0065, REQ-INT-0068, REQ-LEG-0257, REQ-PLT-0030, REQ-PLT-0105 | 22: REQ-CON-0042, REQ-CRY-0035, REQ-ECO-0130, REQ-INT-0025, REQ-INT-0032, REQ-INT-0046, REQ-INT-0047, REQ-INT-0054, REQ-INT-0056, REQ-INT-0057, REQ-INT-0064, REQ-INT-0065, REQ-INT-0069, REQ-INT-0074, REQ-LEG-0113, REQ-LEG-0132, REQ-LEG-0207, REQ-MOD-0062, REQ-MOD-0071, REQ-PLT-0014, REQ-PLT-0126, REQ-PLT-0136 |
| HC-16 | 7 / 17 | REQ-CMP-0117, REQ-CRY-0016, REQ-LEG-0048, REQ-MOD-0034, REQ-MOD-0047, REQ-MOD-0048, REQ-MOD-0110 | 12: REQ-CON-0011, REQ-CON-0023, REQ-CON-0074, REQ-CRY-0016, REQ-CRY-0082, REQ-CRY-0159, REQ-CRY-0255, REQ-ECO-0147, REQ-ECO-0189, REQ-LEG-0113, REQ-LEG-0116, REQ-MOD-0110 |
| HC-17 | 15 / 14 | REQ-LEG-0034, REQ-LEG-0149, REQ-LEG-0183, REQ-LEG-0192, REQ-LEG-0225, REQ-LEG-0240, REQ-LEG-0269, REQ-MOD-0026, REQ-MOD-0033, REQ-MOD-0039, REQ-MOD-0043, REQ-MOD-0075, REQ-MOD-0080, REQ-MOD-0111, REQ-PLT-0128 | 9: REQ-LEG-0012, REQ-LEG-0104, REQ-LEG-0192, REQ-MOD-0063, REQ-MOD-0077, REQ-PLT-0093, REQ-PLT-0100, REQ-PLT-0132, REQ-PLT-0150 |

C.3 REQUIREMENT rows with no HC tag, screened to ODs (a row may match several ODs).

| OD | Rows |
|---|---|
| OD-01 | 8 |
| OD-02 | 11 |
| OD-03 | 39 |
| OD-04 | 5 |
| OD-05 | 66 |
| OD-06 | 8 |
| OD-07 | 3 |
| OD-08 | 22 |
| OD-09 | 38 |
| OD-10 | 18 |
| OD-11 | 7 |
| OD-12 | 12 |
| OD-13 | 4 |
| OD-14 | 5 |
| OD-15 | 30 |
| OD-16 | 12 |
| OD-17 | 16 |
| OD-18 | 20 |
| OD-19 | 69 |
| OD-20 | 24 |
| OD-21 | 8 |
| OD-22 | 9 |
| OD-23 | 5 |
| OD-24 | 12 |
| OD-25 | 50 |
| OD-26 | 11 |

METHOD rows (obligations on the research method, discharged by this document and `decision-method.md`): REQ-ECO-0184, REQ-ECO-0185.

Totals: 589 rows screened (157 INVARIANT, 432 REQUIREMENT); 406 carry at least one HC tag; 181 carry only OD tags; 2 METHOD; 0 untagged; INVARIANT rows without an HC tag: 0.
<!-- END GENERATED: objective_screen.py -->

## Revision history

| Date | Change |
|---|---|
| 2026-09-16 | Pre-registration draft (Phase A method stage): governing question, 17 hard constraints with mechanical violation tests, 26 optimization dimensions with metrics, procedures, directions and aggregation, HC/OD relationship, invariant coverage, freeze register, and generated ledger screen. Awaits `research/methods/method-review-round1.md` and revision. |
| 2026-09-16 | Round-1 revision (pre-registered) against `research/methods/method-review-round1.md`. §1.2 replaced by a pointer to the single procedure of `decision-method.md` §2 with the cost entry points and CC coverage as a report; §1.3 specialist baselines, SPEC §23.4 tolerances and corrected OD assignments; precedence clause extended to procedure, parameters and canonical names; §2.0 rules on measured criteria, the nondeterminism artifact rule, L0 counterexample and sign-off, HC_UNVERIFIED for platforms only, applicability via the constraint crosswalk, the triage rule and research-build equivalence; I24 as three mechanical facts; gated library-version decode steps; new MVTs (input consistency, complexity bounds, allocator oracle, independent escape oracle and interleaving, normative grammar and recall, path coverage and mutation, verified variation, determinism input and stress mode, research-build equivalence, frozen-ID differential regression, fault-class precedence, HC-14 (f)-(j), invariant clauses I7, I10, I11, I20, I22, I23); new HC-18 (crash consistency and origin safety); metric roles and canonical names for every M-number, with redefined or new metrics M01.1, M04.3, M05.3, M05.5, M06.4, M06.5, M08.3, M10.2, M10.5, M11.1, M13.1, M14.1, M15.2, M16.1, M16.2, M17.5, M17.6, M19.2, M21.3, M21.6, M22.4, M23.1, M24.2, M24.5, M25.x, M26.1, M26.5; Appendix A regenerated from §2.1 with roles and a check; freezes F-28 and F-29; threats 7 and 8. No decision-relevant result existed. |
| 2026-09-16 | Round-1 revision completed by a resumed session after the WIP checkpoint `3fda801` (`decision-method.md` §0.1). ebr-family brackets in §3.2 aligned with `decision-method.md` Appendix A.2 (fractions over case lists, heuristic usability metrics and remote access-pattern counts are family `other`; binary checks `correctness`), with the governing rule stated in the §3.2 preamble and checked by `decision-method.md` Appendix A.3; AGG-G states the difference-space aggregation of absolute-only metrics; M08.2 names `scratch_write_bytes`; M15.3 names pack and unpack time; M16.3 is defined in percentage points as T-16; M19.5 units; M21 and M24 definitions in numeric order; §4.2 table in HC order; §4.5 counts three escalation situations; threat 7 records the third identity exposure. No HC, MVT oracle or metric direction changed. No decision-relevant result existed. |

### Round-1 dispositions

Commit: the commit with subject "research: pre-register decision method and archetypal objective". This table lists the findings whose fix changes this document; `decision-method.md` Revision history holds the complete table for all 85 findings, including those fixed only there. `REJECTED` and `ACCEPTED_RISK` rows, and partial rejections, are argued in the review file's "Review disposition" section.

| ID | Sev. | Disposition | Where in this document |
|---|---|---|---|
| A01 | BLOCKER | FIXED | Companion row; §1.2 |
| A02 | BLOCKER | FIXED | HC-14 statement, MVT-14(e), MVT-14(j), §2.1 HC-14 row; OD-05 note; OD-16 (M16.1, M16.2, Type); §4.3 padding row; Appendix A I24 row |
| A03 | BLOCKER | FIXED | HC-11 statement and status; MVT-12(c) |
| A04 | BLOCKER | FIXED | §1.4; HC-16 statement; §4.4; §4.5 |
| A05 | HIGH | FIXED | AGG-G (family weights per `decision-method.md` §4.9) |
| A06 | HIGH | FIXED | §1.4 parameters clause; §3.2 preamble and ebr-family brackets (checked by `decision-method.md` Appendix A.3); OD-06 procedure; OD-12 procedure; OD-13; OD-14 procedure; MVT-11(a); MVT-13(a) |
| A07 | HIGH | FIXED | OD-13 |
| B01 | BLOCKER | FIXED (rule; artifact gate) | §2.0 rule 7; Appendix B F-28, F-29 and extension rule |
| B02 | HIGH | FIXED | MVT-07(d); §2.1 HC-07 row; M06.5 |
| B03 | HIGH | FIXED | MVT-06(d); M17.5; M24.5 |
| B04 | HIGH | FIXED | MVT-06(a); HC-06 status; §2.1 HC-06 row |
| B05 | HIGH | FIXED | §2.0 rule 2; MVT-13(a) |
| B06 | HIGH | FIXED | MVT-05(a), MVT-05(b) |
| B07 | HIGH | FIXED | §2.0 rule 8; MVT-03(b); MVT-12(a) |
| B08 | HIGH | FIXED | MVT-12(b) |
| B09 | MEDIUM | FIXED | MVT-13(a) |
| B10 | MEDIUM | FIXED | MVT-11(a); §1.4 platform list |
| B11 | MEDIUM | FIXED | MVT-02(e) |
| B12 | MEDIUM | FIXED | §2.0 rule 9; MVT-16(c) |
| B13 | MEDIUM | FIXED | MVT-15(b) precedence table |
| B14 | MEDIUM | FIXED | MVT-14(f)-(i) |
| B15 | MEDIUM | FIXED | MVT-16(d); §2.1 HC-16 row |
| B16 | MEDIUM | FIXED | HC-18, MVT-18; §2.1; §4.2; M12.4 |
| B17 | MEDIUM | FIXED | MVT-10(d) |
| B18 | LOW | FIXED | MVT-03(b), MVT-06(a), MVT-14(b), MVT-14(c), MVT-15(b) detection statements |
| B19 | MEDIUM | FIXED | §2.0 rule 3; MVT-01(c) |
| C01 | MEDIUM | FIXED | Appendix A (HC column with roles and reverse check generated from §2.1; embedded check) |
| C02 | MEDIUM | FIXED | MVT-01(d); MVT-04 items 8-10; MVT-06(a) count disagreement; MVT-10(e), (f); MVT-17(d) |
| C03 | LOW | ACCEPTED_RISK | Appendix C limitation note |
| D01 | BLOCKER | FIXED | §3.0 canonical names and roles; canonical names at every M-number |
| D02 | HIGH | FIXED | OD-01 direction; M04.3; M21.2; M21.3; M22.4; M23.1; M24.2 |
| D03 | HIGH | FIXED | OD-03 aggregation |
| D04 | HIGH | FIXED | M08.3 and OD-08 direction |
| D05 | MEDIUM | FIXED | M05.3; M10.2; M14.1 procedure and aggregation; M15.2; M16.2; M19.2; OD-25 procedure; M26.1 |
| D06 | HIGH | FIXED | §1.3; M10.5; M26.5 |
| D07 | MEDIUM | FIXED | M17.6; M05.5; M21.6; HC-18 |
| D08 | MEDIUM | FIXED | AGG-G per-tier guards and byte-weighted arm |
| E01 | HIGH | FIXED | OD-07 procedure; M10.1 |
| E02 | HIGH | FIXED | OD-14 procedure and aggregation |
| E06 | MEDIUM | FIXED | M08.3 direction (write accounting); M09.1 |
| E07 | MEDIUM | FIXED | OD-25 procedure |
| F06 | HIGH | FIXED | §1.4 platform list; OD-06 procedure |
| F10 | MEDIUM | FIXED | OD-12 procedure |
| G01 | BLOCKER | FIXED (threat model and route) | §5 threat 7 (three sessions) |
| I04 | MEDIUM | FIXED | §4.4 |
| J01 | BLOCKER | FIXED | §2.0 rule 4; §4.6 |
| J07 | MEDIUM | FIXED | OD-25 procedure and direction |
| K01 | BLOCKER | FIXED (field defined; artifact gate) | §2.0 rule 7 |
