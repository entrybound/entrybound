# Entrybound Archetypal Objective

| Field | Value |
|---|---|
| Status | **PRE-REGISTRATION DRAFT** (Phase A, method gate). The adversarial review (`research/methods/method-review-round1.md`) and the revision that follows it produce the pre-registration record; this draft is not that record. |
| Date | 2026-09-16 |
| Companion | `research/decision-method.md` fixes thresholds, statistics, split discipline, sensitivity and status rules. This document fixes what is forbidden (hard constraints), what is optimized (optimization dimensions), how each is measured and aggregated, and how the two relate. Where the two documents appear to conflict on a threshold or statistical procedure, `decision-method.md` governs; where they conflict on what counts as a violation or what a metric measures, this document governs. |
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

> **GQ.** Among all designs of the Entrybound system that satisfy every hard constraint HC-01 to HC-17, which design most completely realizes the archetypal ideal of Entrybound as the definitive next-generation archive system, subject to its semantic, security, fidelity, determinism, interoperability, bounded-resource, longevity and usability invariants? Where no single design does, what is the smallest set of declared profiles, layouts and caller policies that does, and what stays out of reach, with the measured cost that keeps it out of reach?

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

GQ is answered by a lexicographic procedure. `decision-method.md` fixes its thresholds and statistics. [INFERRED operationalization of A11 and A12; to be checked against the program's decision rule order in `decision-method.md`]

1. **Feasibility (binary).** A candidate is feasible only if every applicable HC MVT (§2) reports zero violations. No optimization gain of any size offsets a violation.
2. **Non-domination (graded).** Among feasible candidates, keep those on the epsilon-Pareto frontier of the OD vector (§3) for the decision's workload scope, on the validation split, and confirm on held-out after design freeze.
3. **Combination coverage (graded, system-level).** Among non-dominated candidates, prefer the configuration set that serves more of the capability combinations CC-01 to CC-10 (§1.3). Count separately the combinations served by the **default configuration** (`ebound pack` with no flags, and the default extraction policy), because A11 says the project should not have to choose in advance.
4. **Permanent cost.** When remaining differences are not practically significant, prefer the candidate with lower permanent cost: OD-21 independent implementability, OD-22 dependency longevity, OD-23 specification complexity, OD-24 attack surface.
5. **Declared residue.** Record every CC not served, every OD on which the chosen design is practically worse than a capability-matched incumbent or specialist, and every HC whose MVT could not run (HC_UNVERIFIED), as negative results and remaining unknowns.

### 1.3 Capability combinations (CC)

These are the ten rows of SPEC §23.2. [FORMALLY_DERIVED] A configuration **serves** a CC when every HC floor listed for it passes and every listed OD is within the pre-registered practical-significance band of the best capability-matched baseline for that OD, as defined in `decision-method.md`. Otherwise the outcome is `SERVED_WITH_DECLARED_COST`, which requires the measured gap to be reported, or `NOT_SERVED`. [INFERRED]

| CC | Combination (SPEC §23.2 row) | HC floors | ODs that must be served |
|---|---|---|---|
| CC-01 | Random access and strong compression | HC-02, HC-09 | OD-10, OD-05 |
| CC-02 | Streaming creation and random access | HC-06, HC-15 | OD-09, OD-10 |
| CC-03 | Deduplication and a portable single-file archive | HC-01, HC-09 | OD-05 (dedup metrics), OD-26 (single-file portability) |
| CC-04 | Authenticated encryption, an encrypted index and multi-recipient | HC-14 | OD-15, OD-16 |
| CC-05 | Reproducible builds and preserved metadata | HC-13, HC-07 | OD-03, OD-20 |
| CC-06 | A signature that survives recompression | HC-10 | OD-20, OD-15 |
| CC-07 | Verified partial download | HC-15, HC-10 | OD-11, OD-12 |
| CC-08 | Declared resource bounds before decompression | HC-06 | OD-17 |
| CC-09 | An explicit statement of what was not preserved | HC-07 | OD-03 |
| CC-10 | Post-quantum confidentiality for long-lived archives | HC-14 | OD-15 (assurance state, `EXPERT_REVIEW_REQUIRED`) |

### 1.4 Decision variables, fixed elements, and unit of evaluation

**The research may vary:** new wire records and feature bits, allocated as new identifiers only (HC-16); planner, chunker, similarity, codec and transform registry contents under new planner or chunker IDs; profile parameters; layout choices and stream windows; access, coalescing and verification policies; padding and leakage parameters under a new crypto suite or version only (F-12); default extraction and resource policies; CLI and library defaults and surface; legacy import and export profiles; specification text; and conformance-corpus contents. [FORMALLY_DERIVED from CONTRIBUTING.md freezes, which permit change only through new identifiers]

**The research may not vary:** the hard constraints HC-01 to HC-17; the behavior of already-published identifiers (Appendix B); the SPEC §24 non-goals (not a filesystem, backup system, version-control system, single-stream compressor, transport protocol, key-management system, signing or transparency service, lossy compressor, or codec; never requires live infrastructure to verify; in-archive error correction permanently refused); and the program's standing constraints (local Windows, WSL2 and Docker only; production semantics unchanged by experiments; research features default-off and byte-identical when disabled). [FORMALLY_DERIVED from SPEC §24 and PROGRESS.md standing decisions]

**Unit of evaluation.** An observation is a tuple (candidate configuration, corpus item, operation, platform, repetition).

- **Configuration:** format version and feature set, planner ID, chunker ID, profile, layout, encryption mode, policy, and build.
- **Operation:** pack, unpack, verify, list/inspect, random entry read, random range read, HTTP-range remote read, repack (representation-only or replan), key add/remove, sign or verify signature, import (ZIP, tar family, compressed streams, 7z), export (ZIP, tar/pax, compressed tar), publish (multi-target), or salvage.
- **Platform:** Windows 11 host on NTFS, WSL2 Ubuntu 24.04 on ext4 under `/root/eb-research` (never `/mnt/d` for metadata work), Docker linux/amd64, or QEMU linux/arm64. QEMU is labeled emulated and used for correctness and determinism only.

[FORMALLY_DERIVED from PROGRESS.md execution environment]

---

## 2. Hard constraints

### 2.0 Rules that apply to every HC

1. **Binary.** An HC is satisfied or violated. No thresholds, weights, confidence intervals or practical-significance bands apply. [INFERRED from SPEC §23.5, which treats the invariant list as a gate rather than a trade-off]
2. **One reproducible violation is enough.** A violation is a committed artifact containing an input generator (format-building code, per F-18), the exact command, the build identity and the observed output. It sits under `research/raw/<EXP>/`. Absence of violations in N trials is evidence, not proof. Where an MVT is probabilistic, its detection bound is stated. [INFERRED]
3. **Two elimination levels.**
   - **L0 (inspection):** a candidate whose own specification contradicts an HC, such as deterministic ciphertext, a second length field or a fail-open unknown record, is excluded without running anything. The exclusion is recorded in `candidate_exclusion_reasons` with `invariant_violated`.
   - **L1 (mechanical):** every other candidate must pass the applicable MVTs on the conformance corpus and on the tuning and validation splits before design freeze, and on held-out after freeze.

   [FORMALLY_DERIVED from the decision-ledger schema fields]
4. **HC_UNVERIFIED.** If an applicable MVT cannot run (missing instrument, `PLATFORM_BLOCKED`, or expert review required), the candidate's status for that HC is HC_UNVERIFIED. That blocks a `DECIDED` status except for a decision whose scope explicitly excludes the unverifiable platform or property. HC_UNVERIFIED is never counted as a pass. [INFERRED]
5. **Production defects are not relaxations.** An MVT that finds a violation in the current production baseline creates a defect and ledger work item. It is not grounds to weaken the HC. An HC changes only through a SPEC revision with adversarial review, which is outside this program's authority (§4.5). [FORMALLY_DERIVED from SPEC §23.5 and the PROGRESS.md standing decision on production semantics]
6. **Outcome vocabulary.** MVT oracles use the six outcome classes of SPEC §20.5 (`OK`, `UNSUPPORTED`, `TRUNCATED`, `CORRUPT`, `NONCONFORMING`, `POLICY_REFUSED`), the `UNVERIFIED` qualifier, and stable reason codes. The production enum is `OutcomeClass` in `crates/entrybound/src/diagnostics.rs`. [FORMALLY_DERIVED]

### 2.1 Summary

| HC | Constraint | Invariants (primary; supporting) | Freezes (Appendix B) | MVT oracle in one line | Status at drafting |
|---|---|---|---|---|---|
| HC-01 | One authority per semantic fact | I1, I2, I10, I19, I23 | F-01, F-02, F-07, F-10, F-19 | Authority census finds at most one authoritative declaration per fact; cache-disagreement mutants never surface the cached value | UNRESOLVED (Appendix C.2: 2 weak rows) |
| HC-02 | Exact logical losslessness | I14, I28, I31; I30 | F-03, F-11, F-22 | Zero byte mismatches across the round-trip matrix; injected reconstruction error is never written | UNRESOLVED (6 weak rows) |
| HC-03 | Deterministic, unique native interpretation | I15, I3, I6, I26; I19, I21 | F-07, F-15, F-18 | Canonical re-encode is byte-identical; readers and layouts never disagree on (class, code, EAM) | UNRESOLVED (I15 row REQ-MOD-0110 INSUFFICIENT) |
| HC-04 | Fail-closed unknown critical semantics | I12, I21, I11 | F-03, F-12, F-15, F-17, F-26 | Every unknown critical feature yields `UNSUPPORTED` with zero output; unknown optional data is ignored but tamper-evident | UNRESOLVED (REQ-PLT-0144 MISSING) |
| HC-05 | Caller-owned extraction and security policy | I16; I25, I17 | F-11, F-17, F-18, F-23 | No archive mutation enlarges the operations performed under a fixed policy; zero escapes | UNRESOLVED (19 weak rows incl. REQ-PLT-0110 CONTRADICTED) |
| HC-06 | Declared resource bounds | I17, I20; I19, I29 | F-09, F-15, F-16, F-21 | Peak resources stay within policy plus a measured baseline; refusal happens before proportional allocation | UNRESOLVED (REQ-CON-0004, REQ-CON-0005 INSUFFICIENT) |
| HC-07 | No silent metadata or semantic loss | I13, I14, I18, I11 | F-17, F-19, F-20, F-21, F-22 | Every observed source-to-destination difference is covered by a typed declaration; strict import never resolves a Divergence | UNRESOLVED (REQ-PLT-0079, REQ-LEG-0056 CONTRADICTED) |
| HC-08 | Archive semantics independent of host filesystem interpretation | I4, I5, I27; I26 | F-17, F-27 | Identical (class, LAI, AUX, PCR, entry digests) on every host; hostile names are refused or renamed with a report, never silently merged | UNRESOLVED (13 weak rows) |
| HC-09 | Bounded dependency chains | I29; I30, I10 | F-03, F-06, F-10, F-25 | Every chunk's closure is acyclic, backward-only and within its declaration; Complete archives decode offline | UNRESOLVED (REQ-ACC-0001 CONTRADICTED) |
| HC-10 | Canonical identities | I7, I8, I9, I22, I28; I23 | F-07, F-08, F-13, F-15, F-17 | Identity change pattern equals the SPEC §8.1 table cell for cell; permanent vectors match | UNRESOLVED (28 weak rows) |
| HC-11 | No runtime-dependent decoder interpretation | I30, I15; I31 | F-03, F-05, F-12, F-24 | Decoded output and outcome are invariant under thread, core, locale, OS, memory and architecture variation | UNRESOLVED (instrument not built) |
| HC-12 | Independently implementable native specification | I30, I15, I12; I21 | F-12, F-18 | A clean-room reader passes the baseline conformance corpus at 100% with matching codes; every normative rule has a case | UNRESOLVED (REQ-ECO-0030, REQ-ECO-0021 MISSING); EXPERT_REVIEW_REQUIRED |
| HC-13 | Reproducible deterministic modes | I8, I9, I28; I15, I30 | F-03, F-04, F-06, F-12, F-22 | `--deterministic` unencrypted PCI is identical across threads, hosts, traversal order and memory; encrypted LAI is identical and ciphertext differs | UNRESOLVED (18 weak rows incl. REQ-CON-0055 CONTRADICTED) |
| HC-14 | Crypto correctness never traded for performance | I9, I24, I25; I12 | F-12, F-13, F-14, F-16, F-23 | Vector gate at 100%; zero plaintext released on any tamper; zero (key, nonce) repeats; frozen-suite edits excluded at L0 | UNRESOLVED (22 weak rows); EXPERT_REVIEW_REQUIRED |
| HC-15 | Verification honesty and distinguishable failure | I25, I21 | F-08, F-10, F-11, F-18 | Reported verification state never exceeds the oracle state; fault class equals injected class through to CLI exit status | UNRESOLVED (22 weak rows incl. REQ-INT-0025 INSUFFICIENT) |
| HC-16 | Published identifiers are immutable | I15, I22, I30 | F-03, F-04, F-06, F-12, F-13, F-15, F-17, F-20, F-22, F-26 | Golden vectors and archives per frozen ID reproduce byte for byte; no identifier value is reused | UNRESOLVED (12 weak rows) |
| HC-17 | Structural path and entry hazards are unrepresentable | I3, I4, I5, I6, I23, I26, I27 | F-17, F-19, F-21 | No constructor or encoder emits a hazard; decoders return `NONCONFORMING` with a stable code; strict legacy import refuses | UNRESOLVED (9 weak rows) |

The 14 constraints named by the program map as follows. The program's "one authority per semantic fact" is HC-01; "exact logical losslessness" is HC-02; "deterministic native interpretation" is HC-03; "fail-closed unknown critical semantics" is HC-04; "caller-owned extraction/security policy" is HC-05; "declared resource bounds" is HC-06; "no silent metadata/semantic loss" is HC-07; "archive semantics independent of host filesystem interpretation" is HC-08; "bounded dependency chains" is HC-09; "canonical identities" is HC-10; "no runtime-dependent decoder interpretation" is HC-11; "independently implementable native specification" is HC-12; "reproducible deterministic modes" is HC-13; and "crypto correctness never traded for performance" is HC-14. HC-15, HC-16 and HC-17 are added because three invariant families would otherwise be covered only incidentally:
- I21 and I25 (verification honesty) become HC-15.
- The repository's identifier freezes become HC-16.
- The unrepresentability invariants I3, I5, I6, I26 and I27 become HC-17.

[INFERRED]

"Weak rows" are ledger rows mapped to the HC whose `evidence_state` is `CONTRADICTED` or `INSUFFICIENT`, or whose `implementation_state` is `MISSING` (Appendix C.2). The keyword part of that mapping is a heuristic screen. [FORMALLY_DERIVED counts; INFERRED membership]

### 2.2 Constraint definitions and mechanical violation tests

#### HC-01 — One authority per semantic fact

**Statement.** Every semantic fact has exactly one authoritative declaration in the canonical encoding of every feature combination. Semantic facts include entry existence, LogicalPath, EntryKind, content bytes, each metadata item value, every size, offset and count, ChunkGroup membership, ReconstructionRegion extent, and a Chunk frame's stored length. Any other occurrence is either a non-authoritative cache that a conforming reader validates against the authority before use, or a *claim* that the reader checks against the authority. Declared budgets and descriptor summaries are claims. A hard link is content sharing plus the AUX metadata `posix.hardlink_group`, not a second structural record. [FORMALLY_DERIVED from SPEC §2.1(3), §3.2 E1, §3.3, §3.5, §4.5, §3.9 I1/I2/I10/I19/I23; CONTRIBUTING.md L82-83, L116-119]

**MVT-01.**
- (a) **Authority census.** For each record type and field in the ECF and EAM registries (`crates/entrybound/src/ecf`, `crates/entrybound/src/eam`, the `docs/*-v1.md` wire tables), assign a fact class and a role (AUTHORITY, CACHE or CLAIM). The census is regenerated per candidate by the `ebr-pack` section walker extended with per-field provenance. Until that exists, it is a manual table committed under `research/audit/`. **Violation:** any fact class with two or more AUTHORITY fields in any valid feature combination, or a CACHE or CLAIM field that the reader is not shown to validate.
- (b) **Cache-disagreement mutants.** For every CACHE field (Index locators, EncryptedIndex entries, any descriptor restatement of a derivable fact), generate archives where cache and authority disagree. **Oracle:** the result reflects the authority, or is a named refusal such as `RebuiltInvalid` or `CORRUPT` with a stable code. **Violation:** any output value, listing, extraction or identity that reflects the cached value.
- (c) **L0.** A candidate adding a second declaration of an existing fact is excluded without measurement.

**Instruments.** Production tests in `crates/entrybound/tests/native_ecf.rs` and `stream_layout.rs`; planned harness crate `research/harness/crates/ebr-pack`; `ebr` correctness-family metric `authority_violations` (must equal 0).

**Status.** Census completeness cannot be proven mechanically, because it depends on the fact-class taxonomy. [EXPERT_REVIEW_REQUIRED] REQ-MOD-0018 records an open tension with I1 for descriptor-level derivable values. [UNRESOLVED]

#### HC-02 — Exact logical losslessness

**Statement.** For every archive of role `Complete`, and every profile, layout, encryption mode, planner ID, chunker ID and feature, extraction writes, for every `File` entry, the exact octet sequence read at creation. For every `Symlink`, it writes the exact target octets. No lossy transform exists in any registry. Every reconstructive transform is round-trip verified at write time, with SHA-256 and byte equality and no opt-out. Content addressing over plaintext (`logical_digest`) verifies the path end to end. `Sidecar` and `Incremental` roles make no standalone extraction claim, and role is bound into LAI. [FORMALLY_DERIVED from SPEC §6.5, §8.5, §3.9 I14/I28/I31; CONTRIBUTING.md L89-98; REQ-INT-0052]

**MVT-02.**
- (a) **Round-trip matrix.** Covers every corpus item in the split under test, times profile {fast, balanced, dense, extreme}, times layout {indexed, stream}, times {unencrypted, recipient-encrypted, password-encrypted}, times candidate. Pack, fully verify and unpack into scratch. Compare each file's length and SHA-256 and each symlink's target bytes with the independent corpus fingerprint (`ebrc-fp-v1` under `research/corpus/fingerprints`). **Violation:** any mismatch. Implemented as the `ebr` correctness metric `roundtrip_ok` (binary).
- (b) **Fault injection.** In a research-only build (default-off feature, F-24), each reconstructive transform (deflate-reconstruct, jpeg-jxl-reconstruct and any candidate) returns a one-bit-wrong reconstruction. **Oracle:** the writer refuses with a stable reason code and emits no archive. **Violation:** an archive is written.
- (c) **Registry audit.** **Violation:** any registered codec or transform without an exact inverse, or any flag, profile or API that skips the write-time round trip.
- (d) **Known-hard inputs** are always included: F11 already-compressed, F12 JPEG including jpegli-produced files, F14 archive-in-archive, F20 adversarial. The SPEC cites a libjxl failure to recompress jpegli JPEGs and preflate degradation. [EXTERNALLY_SOURCED via SPEC §3.6 T2]

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

  Run them over the conformance corpus and a structure-aware mutation corpus built with cargo-fuzz 0.13.2 on nightly-2026-08-01 in WSL. Mutations cover every structural byte of preamble, descriptor, section and record headers, and footer exhaustively for single-byte flips, plus 10,000 seeded payload positions per archive. Compare the tuple (outcome class, reason code, entry-set digest, per-entry content digest, per-entry metadata digest). **Violation:** any disagreement that triage does not attribute to a bug in the independent reader. A disagreement caused by ambiguous specification text is an HC-03 violation of the *specification* until that text is fixed.
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
7. feature-bitmap bytes altered with the bitmap digest left stale.

**Oracles:** items 1-5 yield `UNSUPPORTED` with a stable code, zero output bytes or filesystem objects, and peak RSS within the HC-06 pre-validation bound. Item 7 yields `CORRUPT`. Item 6 yields `OK`, but a single-byte change inside the optional item makes the AUX binding or authentication fail. **Violation:** any other outcome.

**Instruments.** Generators as format-building code in tests (F-18); `ebr` correctness metrics.

**Status.** REQ-PLT-0144 is `MISSING`: the archive-wide Critical-metadata option is unspecified. [UNRESOLVED]

#### HC-05 — Caller-owned extraction and security policy

**Statement.** No archive field grants, widens or selects extraction behavior. That covers destination paths, overwrite and collision handling, special files, setuid/setgid/sticky, ownership, budgets above policy, link following, confinement mode, and metadata restoration classes. The archive may only declare facts that let a caller refuse earlier. Extraction resolves one component at a time against held directory handles, using the platform containment primitive. It never touches metadata by path, never follows or traverses symlinks, and never falls back to string validation. The extractor reports any confinement weaker than kernel-enforced. Dangerous restorations are off by default. Unverified content never becomes a final object in the caller's destination. [FORMALLY_DERIVED from SPEC §11.6-§11.8, §3.9 I16; CONTRIBUTING.md L40-42, L137-140]

**MVT-05.**
- (a) **Policy monotonicity.** Fix policy P. For each archive A and each single-field mutation A′ (descriptor, feature bits, metadata items, fidelity, provenance, role, hostility summaries), record the filesystem-operation trace of extracting A′ under P with an operation-trace shim (planned harness; `strace -f` in WSL and Docker; Windows uses API-level logging in a research build, because kernel-level tracing needs admin rights and this session has none). **Violation:** the trace contains an operation class that P disallows, or A′ causes an operation class that neither A under P nor P alone permits.
- (b) **Escape suite.** Cases: symlink-in-parent, symlink or junction swap races, absolute-target symlinks, case-fold collisions on NTFS, Windows reserved names, alternate data streams, and pre-existing destination objects. The race scenarios run 10,000 trials each with a concurrent attacker thread. **Violation:** any object created, modified or opened outside the extraction root or through a link. With 10,000 clean trials, a per-trial escape probability of 3 × 10⁻⁴ or more would have been detected with about 95% probability (rule of three). [FORMALLY_DERIVED]
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
  - STREAM with `BudgetDeclared = false` exceeding policy mid-stream.

  Run under `ebr` with explicit policy budget B, measuring `memory_peak` and `scratch_peak`. **Baseline R0:** the median peak RSS of the same build and platform opening a minimal valid archive. **Pre-validation bound H:** the largest structure the format permits the reader to buffer before the budget comparison (preamble, descriptor and its enclosing record), computed from the wire specification per candidate. **Oracles:** refusal cases end with `POLICY_REFUSED` or `CORRUPT`/`NONCONFORMING` for liars, with peak RSS ≤ R0 + H + the `memory_peak` noise band from `decision-method.md`. Accepted cases have peak RSS ≤ R0 + B_memory + band and scratch ≤ B_scratch + band. For KDF over the ceiling, an instrumented counter shows zero Argon2id invocations. **Violation:** any exceedance, any refusal after proportional allocation, or any KDF invocation above the ceiling.
- (b) **Declaration consistency.** For every valid archive a candidate produces, reader-derived aggregates equal the declared requirements, and actuals stay within the upper bounds (REQ-MOD-0017). **Violation:** any mismatch.
- (c) **Parse-site audit.** Every length or count read from untrusted bytes flows through checked arithmetic and a bound check before use. A static audit is committed with each candidate. [EXPERT_REVIEW_REQUIRED for completeness]

**Instruments.** `ebr` `memory_peak` (Linux `/usr/bin/time -v`; Windows Job Object plus psutil), `scratch_peak` poller; planned `ebr-access` streaming memory probe.

**Status.** The numeric value of H is not yet computed. REQ-CON-0004 (I20) and REQ-CON-0005 (I17) are `INSUFFICIENT`. [UNRESOLVED]

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
- (d) **Cross-producer.** Pack the same tree under every planner ID and profile. **Violation:** unequal LAI or AUX.

**Instruments.** `ebound inspect --json`; `crates/entrybound/tests/stream_layout.rs` equivalence assertions; planned `ebr-pack`.

**Status.** Twenty-eight mapped rows are weak. [UNRESOLVED]

#### HC-11 — No runtime-dependent decoder interpretation

**Statement.** Decoded bytes and outcome depend only on archive bytes, caller policy and keys. They do not depend on thread count, core type, CPU feature detection, OS, locale, time zone, clock, available memory above the declared requirement, build profile, planner ID or creation profile. Decoders implement the closed codec and transform registries and never invoke the planner, chunker or similarity analysis. A decode step whose output is defined by a pinned library version rather than a format definition must carry that version identity in the recorded plan's registry identifier, and must be verified by digest before release. [FORMALLY_DERIVED from SPEC §3.6 T1, §3.9 I15/I30, §12.2; CONTRIBUTING.md L67-71, L94-98; REQ-CMP-0044]

**MVT-11.**
- (a) **Decode variation matrix.** Decode each archive under:
  - threads {1, 2, 8, 32};
  - affinity on P-cores only (Windows logical 0-15) and on E-cores only (16-31);
  - locale {C, tr_TR.UTF-8, ja_JP.UTF-8} and time zones {UTC, Pacific/Chatham};
  - Windows, WSL and QEMU arm64;
  - Docker `--memory` at 1×, 2× and 8× the declared requirement.

  **Violation:** any difference in (outcome class, reason code, output digests).
- (b) **Alternate decoder.** Where a format-defined decoder exists for a reconstructive or codec step (for example a second Zstandard implementation, or libjxl JPEG reconstruction per ISO/IEC 18181-1 [EXTERNALLY_SOURCED]), decode with it as well. **Violation:** a difference not already classified as a bug in that alternate decoder.
- (c) **Static.** **Violation:** the decode call graph (`cargo tree -e normal` on the decode feature set, plus symbol inspection) reaches planner, chunker or similarity code. [EXPERT_REVIEW_REQUIRED for completeness]

**Status.** `deflate-reconstruct` (preflate class) appears to be defined by its implementation rather than by a public specification, so HC-11 and HC-12 depend on pinned-version identity and permanent vectors for that gated feature. [INFERRED; UNRESOLVED]

#### HC-12 — Independently implementable native specification

**Statement.** A competent engineer can implement a conforming decoder for the baseline feature and codec set from the specification, registries and permanent vectors alone, without Entrybound source code. Every normative rule has at least one conformance case with a decided outcome class and reason code. Every codec or transform needed for baseline decoding has a public specification or permanent vectors. Gated features that lack a public specification declare that in the registry. [FORMALLY_DERIVED from SPEC §19.2-§19.3, §20.6; REQ-ECO-0030, REQ-ECO-0021; CONTRIBUTING.md L142-149, L182-183]

**MVT-12.**
- (a) **Clean-room reader.** `research/independent-reader` is built from specification text and vectors under a provenance log. It is developed in a checkout that excludes `crates/` and `tools/`, and every consulted file is logged. It must pass the baseline conformance corpus at 100%, with identical outcome class and reason code. **Violation:** any failure not triaged as an independent-reader bug. A failure traced to missing or ambiguous specification text is a violation until the specification is fixed.
- (b) **Traceability.** A script extracts every normative MUST, MUST NOT and SHALL statement from the specification and maps each to at least one case ID. **Violation:** an unmapped statement.
- (c) **Codec provenance.** **Violation:** a baseline decode step whose only definition is library behavior.

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
- (a) **Repeat-hash.** For each tuning and validation item, profile and layout, pack with `--deterministic` and no encryption at least 3 times per variation cell: threads {1, 8, 32}; host {Windows, WSL, QEMU arm64}; source tree recreated with shuffled creation order; locale and time zone varied; Docker memory limit varied. **Violation:** any PCI difference or any LAI difference.
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

No mode is encrypted but unauthenticated, and no mode produces deterministic ciphertext. Chunk boundaries in encrypted archives are derived under a key-domain secret, so plaintext structure leaks only as the declared, quantized residual (I24 is a design objective; its residual is graded under OD-16). The implementation uses pinned, vetted primitive crates, implements no primitive locally, and offers no user-selectable algorithms. [FORMALLY_DERIVED from SPEC §9.1, §9.4-§9.7, §12.3, §3.9 I9/I24/I25; CONTRIBUTING.md L142-181; REQ-CRY-0013, REQ-CRY-0097, REQ-CRY-0124, REQ-CRY-0243]

**MVT-14.**
- (a) **Vector gate.** RFC, draft-10, V1-V7 and RFC 8032 vectors, plus RFC 8452 vectors once added (REQ-CRY-0085), run in `crates/entrybound/tests/crypto_primitives.rs` and `crypto_v1.rs`. **Violation:** any failure.
- (b) **Tamper before release.** Apply every single-bit flip in public framing, headers and stanzas, and 10,000 seeded flips in payload segments. Also apply truncation at each record boundary, record reordering, cross-archive record splicing, and stanza insertion or removal. **Violation:** any plaintext byte reaching the caller or the filesystem, or any outcome other than an authentication-class failure.
- (c) **Uniqueness.** Across 10,000 archive creations, including key add and remove sequences, log (derived key identifier, nonce) pairs from a research-only instrumented build (F-24). **Violation:** any repeated pair.
- (d) **L0 rule.** A candidate that edits an item frozen by F-12, F-13 or F-14, removes or reorders an unlock step, adds a primitive not in the pinned set, or exposes an algorithm choice is excluded without measurement and routed to `EXTERNAL_REVIEW_REQUIRED`.
- (e) **Keyed boundaries.** Chunk the same plaintext under two different file keys. **Violation:** identical boundary sets, or boundaries equal to the unkeyed `gear-norm-v1` boundaries beyond the chance rate computed for the parameters.

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

  **Violation:** any off-diagonal cell in the library result or the CLI exit status.

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

Improvements ship as new identifiers, and retired identifiers are never reused. Historical archives remain decodable with unchanged interpretation and identities. [FORMALLY_DERIVED from SPEC §12.2, §20.3-§20.4; CONTRIBUTING.md L44-54, L67-103, L142-173, L194-203]

**MVT-16.**
- (a) **Golden regression.** Every permanent vector file (`docs/descriptor-vectors-v1.txt`, `docs/posix-metadata-v1-vectors.txt`, `docs/security-metadata-v1-vectors.txt`, `docs/crypto-wire-v1-vectors.txt`) and a committed generator-plus-digest set per frozen identifier are re-decoded and re-produced by the candidate build. **Violation:** any byte or identity difference.
- (b) **Allocation audit.** A script over the registries in source and docs. **Violation:** an identifier value assigned twice with different meaning, or a candidate patch that changes frozen-ID behavior without allocating a new identifier.

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

**Status.** Nine mapped rows are weak. [UNRESOLVED]

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

**Correctness gating of performance samples.** Every performance or size sample carries the correctness checks of its operation: `roundtrip_ok`, identity equality, and the verification state. A sample whose correctness check fails is excluded from OD analysis and reported as an HC violation. It is never averaged. [INFERRED]

**References.** Each decision names its reference configuration. Two references are standard:
- **REF-PROD:** production `ebound pack` defaults (balanced profile, INDEXED, unencrypted, current planner ID) built at the research baseline SHA `9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155`.
- **REF-INC:** the capability-matched incumbent baseline for the metric, as defined in `research/baselines/`.

External-comparison claims (§23-style) report both references. [INFERRED]

**Per-item statistic.** For timing and resource metrics, the per-item value is the paired median over repetitions; repetitions, warmups and CI procedure are defined in `decision-method.md`. Deterministic byte metrics use a single value confirmed by repeat-hash. [INFERRED]

**Aggregation operators.** One or more is named per metric. [INFERRED; the choice of geometric means for ratios is standard for multiplicative quantities]

| Operator | Definition |
|---|---|
| **AGG-G** (geometric, family-balanced) | Per item i: r_i = value(candidate, i) / value(reference, i). Within family f, average ln r_i equally over items within each independence group, then equally over groups, giving ln R_f. Headline R = exp(mean over applicable families of ln R_f). Always reported with **worst-family guard**: max_f R_f for minimized metrics, min_f R_f for maximized ones. Scale tiers are reported separately in addition to the pooled value. Splits are never pooled. Families with no applicable item are listed and excluded. Metrics that can be zero use AGG-W or AGG-C instead. |
| **AGG-W** (worst case) | Maximum over all applicable (item, operation, platform) observations. For violation or event counts, the sum. |
| **AGG-C** (coverage) | Per family, the fraction of applicable cases that pass. Headline is the minimum over families; the mean is secondary. |
| **AGG-S** (per scenario) | Reported per pre-registered scenario (network profile, platform pair, user task, legacy format). Headline is the worst scenario. No averaging across scenarios. |
| **AGG-P** (per design) | Workload-independent: one value per candidate design (specification, dependency or surface census). |

Workload-mix reweighting of AGG-G belongs to the sensitivity analysis in `decision-method.md`. It is not the headline. [INFERRED]

**Type labels.**
- **BINARY:** pass or fail, identical to an HC floor, never traded.
- **FLOOR+GRADED:** a binary floor from an HC, with the graded quantity measured only above the floor.
- **GRADED:** trade-offs allowed under `decision-method.md`.

### 3.1 Summary

| OD | Dimension | Primary metric (unit) | Direction | Aggregation | Type | HC floors |
|---|---|---|---|---|---|---|
| OD-01 | Semantic coherence | Validation obligations: CACHE+CLAIM fields plus conditional decoding rules (count) | min | AGG-P | FLOOR+GRADED | HC-01, HC-03 |
| OD-02 | Exact losslessness | Round-trip mismatches (count) | = 0 | AGG-W | BINARY | HC-02 |
| OD-03 | Preservation fidelity | Exact capture-and-restore fraction per metadata class and platform pair (fraction) | max | AGG-S (min over classes) | FLOOR+GRADED | HC-07 |
| OD-04 | Deterministic interpretation | Cross-reader and cross-layout disagreements (count); reason-code specificity (fraction) | = 0; max | AGG-W; AGG-C | FLOOR+GRADED | HC-03, HC-11, HC-15 |
| OD-05 | Compression and storage efficiency | Artifact bytes relative to reference (ratio) | min | AGG-G | GRADED | HC-02, HC-09, HC-14 |
| OD-06 | Creation performance | Pack wall and CPU time relative to reference (ratio); logical throughput (MiB/s) | min; max | AGG-G | GRADED | HC-02, HC-13 |
| OD-07 | Decoding performance | Unpack, verify and list wall and CPU time relative to reference (ratio) | min | AGG-G | GRADED | HC-11, HC-15 |
| OD-08 | Memory and scratch | Peak RSS and scratch (bytes); scaling exponent (1) | min | AGG-W and AGG-G | FLOOR+GRADED | HC-06 |
| OD-09 | Streaming | Streaming capability per operation (binary); STREAM overhead (ratio) | max; min | AGG-C; AGG-G | FLOOR+GRADED | HC-06, HC-15 |
| OD-10 | Random access | p95 random-entry latency (s); access amplification (ratio) | min | AGG-G and AGG-W | FLOOR+GRADED | HC-09, HC-15 |
| OD-11 | Verified partial retrieval | Bytes and time to a verified range relative to an unverified read (ratio); verified fraction | min; max | AGG-G; AGG-C | FLOOR+GRADED | HC-15, HC-10 |
| OD-12 | Remote access | HTTP requests (count), bytes transferred (B), latency (s) per network profile | min | AGG-S of AGG-G | FLOOR+GRADED | HC-15, HC-14 |
| OD-13 | Parallelism | Speedup S(n) (1) and efficiency S(n)/n | max | AGG-G | FLOOR+GRADED | HC-11, HC-13 |
| OD-14 | Corruption locality | Blast radius: logical bytes and entries lost per damaged stored byte (ratio) | min | AGG-G and AGG-W | FLOOR+GRADED | HC-15 |
| OD-15 | Confidentiality and authenticity | Undetected tampers (count); authenticated coverage (fraction); encryption overhead (ratio); assurance state | = 0; = 1; min; max | AGG-W; AGG-G | FLOOR+GRADED | HC-14 |
| OD-16 | Metadata privacy | Presence-confirmation attack advantage (0-1); public plaintext-derived bits; padding overhead | min | AGG-W | GRADED | HC-14 |
| OD-17 | Resource-bounded decoding | Declared/actual tightness (ratio ≥ 1); refusal cost; benign false-refusal rate | min | AGG-W and AGG-G | FLOOR+GRADED | HC-06 |
| OD-18 | Cross-platform fidelity | Min over platform pairs of restore fidelity (fraction); platform coverage | max | AGG-S | FLOOR+GRADED | HC-08, HC-07 |
| OD-19 | Legacy interoperability | Import acceptance and correctness; export acceptance by incumbent readers (fraction) | max | AGG-S (per format) | FLOOR+GRADED | HC-07, HC-17, HC-05 |
| OD-20 | Deterministic migration | Identity-rule conformance of migrations (fraction = 1); migration cost (ratio) | max; min | AGG-C; AGG-G | FLOOR+GRADED | HC-10, HC-13, HC-07 |
| OD-21 | Independent implementability | Clean-room pass rate (fraction = 1); independent-reader LoC; specification defects per 1,000 words | max; min | AGG-P | FLOOR+GRADED | HC-12 |
| OD-22 | Dependency longevity | Decode-path dependency count, native/unsafe code, advisories, specification availability | min | AGG-P | FLOOR+GRADED | HC-12, HC-14 |
| OD-23 | Specification complexity | Normative words, record types, fields, feature bits, frozen identifiers added (count) | min | AGG-P | GRADED | HC-16 |
| OD-24 | Implementation attack surface | Untrusted-input parser LoC; fuzz findings per CPU-hour; "validated"-only vulnerability classes | min | AGG-P and AGG-W | FLOOR+GRADED | HC-03, HC-06 |
| OD-25 | User complexity | First-use task success (fraction); flags per task (count); unsafe defaults (count = 0) | max; min | AGG-S | FLOOR+GRADED | HC-05, HC-15 |
| OD-26 | Adoption friction | Consumable-with-preinstalled-tools fraction; migration steps; identification support | max; min | AGG-S | GRADED | HC-07, HC-16 |

Metric choice throughout §3 is `INFERRED` unless tagged otherwise. Instruments named "planned" are `UNRESOLVED` until committed.

### 3.2 Dimension definitions

Each dimension lists metrics (M-number), unit, measurement procedure, direction, aggregation, type and notes. The `ebr` metric family is given in brackets.

#### OD-01 Semantic coherence
- **M01.1** Validation obligations: the number of CACHE and CLAIM fields from the MVT-01 census. Each is a place where a reader must validate against authority. Unit: count [`other`]. **M01.2** Conditional decoding rules: normative rules whose applicability depends on kind, role, layout or feature combination, counted from the HC-12 traceability matrix. Unit: count [`other`]. **M01.3** Equivalence assertions: the fraction of INDEXED/STREAM, encrypted/plain and identity-tier equivalence assertions that pass. Unit: fraction [`correctness`].
- **Procedure.** Census and traceability scripts run on the candidate's specification and registries. Equivalence assertions run as `ebr` correctness checks over tuning items.
- **Direction.** Minimize M01.1 and M01.2; M01.3 must equal 1.
- **Aggregation.** AGG-P for M01.1 and M01.2; AGG-C for M01.3.
- **Type.** FLOOR+GRADED (floor HC-01, HC-03).
- **Note.** Counts proxy cognitive and implementation coherence. [INFERRED; EXPERT_REVIEW_REQUIRED for census completeness]

#### OD-02 Exact losslessness
- **M02.1** Round-trip mismatches (files and symlinks) from MVT-02(a). Unit: count [`correctness`].
- **Procedure.** MVT-02.
- **Direction.** Must be 0.
- **Aggregation.** AGG-W (sum over all observations).
- **Type.** BINARY.
- **Note.** Not traded. The *cost* of the guarantee (write-time verification) is visible in OD-06 M06.3. [FORMALLY_DERIVED from SPEC §6.5]

#### OD-03 Preservation fidelity
- **M03.1** Capture fidelity: the fraction of independently observed source metadata items, per class, stored exactly (equal value and precision). **M03.2** Restore fidelity: the fraction restored exactly on the target. **M03.3** Declared-gap count per class. Units: fraction, count [`correctness`, `other`].
- **Procedure.** MVT-07(a) observers over generated trees from F19 (metadata-heavy), F15 (sparse) and F17 (duplicate trees for hardlink topology). Trees are built on WSL ext4 under `/root/eb-research`, on Windows NTFS, and in Docker. Incumbent comparisons: GNU tar 1.35 `--xattrs --acls --selinux`, bsdtar 3.7.2 and 3.8.8, 7-Zip 23.01, zip. [EXTERNALLY_SOURCED tool versions from PROGRESS.md environment]
- **Direction.** Maximize M03.1 and M03.2.
- **Aggregation.** AGG-S per (class, source platform, target platform). Headline is the minimum over classes declared restorable for that pair; the per-class matrix is always reported.
- **Type.** FLOOR+GRADED. The floor HC-07 makes undeclared loss a violation; declared loss is graded here.
- **Note.** APFS, HFS+ and macOS ACLs are `PLATFORM_BLOCKED`, and ReFS and SACLs need admin rights. [UNRESOLVED]

#### OD-04 Deterministic interpretation
- **M04.1** Disagreements: the number of distinct (input, reader pair) disagreements from MVT-03(b). Unit: count [`correctness`]. **M04.2** Reason-code specificity: the fraction of negative conformance cases whose outcome carries a unique stable reason code identifying the violated rule. Unit: fraction [`correctness`]. **M04.3** Mutation-corpus reach: distinct outcome-class and reason-code pairs exercised per fuzz CPU-hour. Unit: count/h [`other`].
- **Direction.** M04.1 must be 0; maximize M04.2 and M04.3.
- **Aggregation.** AGG-W for M04.1; AGG-C for M04.2; AGG-P for M04.3.
- **Type.** FLOOR+GRADED (floors HC-03, HC-11, HC-15).

#### OD-05 Compression and storage efficiency
- **M05.1** Complete artifact bytes: the `.eb` file size including descriptor, manifest, metadata, index, dictionaries, frame headers, reconstruction side data, groups, padding, signatures and footer. Unit: B, reported as ratio to reference [`size`]. **M05.2** Section byte breakdown from `ebr-pack` exact accounting, whose sum must equal M05.1 or the sample is invalid. Unit: B [`size`]. **M05.3** Fixed overhead on the small tier: artifact bytes minus the best single-stream codec output of the concatenated content. Unit: B [`size`]. **M05.4** Dedup effectiveness: unique stored chunk bytes divided by logical bytes on F17 and F18. Unit: ratio [`size`].
- **Procedure.** `ebr run` with `ebr-pack` over tuning and validation items × profiles × layouts × encryption. The run is deterministic, with repeat-hash verification. Incumbent references use capability-matched settings.
- **Direction.** Minimize.
- **Aggregation.** AGG-G with worst-family guard for M05.1 and M05.4; AGG-W over the small tier for M05.3; M05.2 is descriptive.
- **Type.** GRADED.
- **Note.** Padding bytes in encrypted archives are also reported under OD-16 M16.3. They cannot be removed to improve M05.1 (HC-14).

#### OD-06 Creation performance
- **M06.1** Pack wall time. Unit: s, ratio to reference [`time_wall`]. **M06.2** Pack CPU time. Unit: s, ratio [`time_cpu`]. **M06.3** Mandatory-verification share of pack CPU time, from `ebr-pack` phase timers (capture, chunking, planning, encoding, verification, writing). Unit: fraction [`other`]. **M06.4** Logical throughput. Unit: MiB/s [`throughput`].
- **Procedure.** `ebr run --timing` under the quiet-machine guard, with paired randomized blocks and affinity recorded. Runs at thread counts {1, 8, all logical}, with P-core and E-core sets reported separately on the Windows host. WSL results are labeled virtualized. Repetitions, warmups and guard limits follow `decision-method.md`.
- **Direction.** Minimize M06.1 and M06.2; maximize M06.4; M06.3 is descriptive.
- **Aggregation.** AGG-G with worst-family guard, per thread setting.
- **Type.** GRADED.

#### OD-07 Decoding performance
- **M07.1** Full unpack wall and CPU time, including mandatory verification and verified staging. **M07.2** Verify-only time. **M07.3** List/inspect time. **M07.4** Time to first plaintext byte of the first entry. Units: s, ratio [`time_wall`, `time_cpu`].
- **Procedure.** As OD-06, with `ebr-unpack` and `ebr-verify` from `ebr-pack` and `ebr-access` for M07.4.
- **Direction.** Minimize.
- **Aggregation.** AGG-G with worst-family guard.
- **Type.** GRADED.
- **Note.** Comparisons against incumbents that do not verify must also report an incumbent-with-external-hash configuration, so verification cost is not scored as slowness. [INFERRED]

#### OD-08 Memory and scratch
- **M08.1** Peak RSS per operation (pack, unpack, verify, random read, import, export, repack, publish). Unit: B [`memory_peak`]. **M08.2** Peak scratch. Unit: B [`scratch_peak`]. **M08.3** Scaling exponent: the least-squares slope of ln(peak RSS) against ln(total logical bytes) across scale tiers and the `ebr-access` streaming-memory probe at 1, 4, 16 and 64 GiB. Unit: 1 [`other`].
- **Direction.** Minimize. For operations the SPEC or repository docs describe as bounded-memory, M08.3 must be practically indistinguishable from 0 under `decision-method.md`; otherwise the bounded-memory claim is recorded as false.
- **Aggregation.** AGG-W (max over items) and AGG-G (typical) for M08.1 and M08.2; AGG-W for M08.3.
- **Type.** FLOOR+GRADED (HC-06 for policy and declaration exceedance).

#### OD-09 Streaming
- **M09.1** Capability: whether each operation completes with a Write-only sink or Read-only source (pipe) and no temporary whole-input copy. Unit: binary per operation [`correctness`]. **M09.2** STREAM artifact bytes relative to INDEXED for the same EAM. Unit: ratio [`size`]. **M09.3** End-to-end streaming unpack latency, including verified staging (F-11). Unit: s [`time_wall`]. **M09.4** Dedup loss from the stream window: stored bytes at window w relative to unbounded window. Unit: ratio [`size`].
- **Procedure.** `ebr` commands with `cat item | ebound pack --layout stream -` and `ebound unpack -` pipelines; `ebr-pack --layout stream --stream-window`.
- **Direction.** Maximize M09.1; minimize the rest.
- **Aggregation.** AGG-C for M09.1; AGG-G for M09.2-M09.4.
- **Type.** FLOOR+GRADED (floors HC-06, HC-15, F-09).

#### OD-10 Random access
- **M10.1** Random-entry read latency, p50 and p95 over a seeded sample of 1,000 entries (or all entries if fewer). Unit: s [`time_wall`]. **M10.2** Access amplification: decoded bytes divided by requested bytes, for entry reads and for seeded byte-range reads within large entries. Unit: ratio [`other`]. **M10.3** Open cost: bytes read and time to open an INDEXED archive for random access. Unit: B, s [`io`, `time_wall`]. **M10.4** Declared-cost accuracy: measured over declared decode work, which must be ≤ 1 (HC-09). Unit: ratio [`correctness`].
- **Procedure.** `ebr-access` with Memory and LocalFile sources; `ebr-chunk` amplification subcommand.
- **Direction.** Minimize.
- **Aggregation.** AGG-G with worst-family guard for M10.1-M10.3; AGG-W for M10.2 and M10.4.
- **Type.** FLOOR+GRADED.

#### OD-11 Verified partial retrieval
- **M11.1** Verified-range overhead: bytes fetched and time to obtain a verified range [a, b), relative to the same range read unverified. Unit: ratio [`io`, `time_wall`]. **M11.2** Verification work: Merkle nodes and digest computations per verified range. Unit: count [`other`]. **M11.3** Verified fraction: the fraction of partial-read results in the strongest state the archive supports (chunk digest, or signed-PCR Merkle slice when signed). Unit: fraction [`correctness`].
- **Procedure.** `ebr-access` range reads on signed and unsigned INDEXED archives, with the MVT-15(a) oracle.
- **Direction.** Minimize M11.1 and M11.2; maximize M11.3.
- **Aggregation.** AGG-G; AGG-C for M11.3.
- **Type.** FLOOR+GRADED (HC-15, HC-10).

#### OD-12 Remote access
- **M12.1** HTTP requests per task: open plus list, first entry, random entry, random range, full verify. Unit: count [`io`]. **M12.2** Bytes transferred (response bodies plus headers). Unit: B [`io`]. **M12.3** Task latency. Unit: s [`time_wall`]. **M12.4** Protocol safety: refusal of a weak or missing ETag, a 200 whole-body fallback, and a mid-session revision change. Unit: binary [`correctness`].
- **Procedure.** `ebr-origin` serves archives through `ebr-netem-proxy` under pre-registered profiles. Round-trip time is {1, 20, 100, 300} ms, bandwidth is {1, 10, 100, 1000} Mbit/s, and loss is modeled as seeded stalls. The primary profile subset is fixed in `decision-method.md`. Emulation is application-level because `sch_netem` is unavailable in WSL2 and Docker, a documented threat to validity; calibration results live under `research/raw/EXP-NETEM-CAL/`. [FORMALLY_DERIVED from PROGRESS.md blockers]
- **Direction.** Minimize M12.1-M12.3; M12.4 must pass.
- **Aggregation.** AGG-S over network profiles of AGG-G over families. No cross-profile averaging.
- **Type.** FLOOR+GRADED (floor F-10, HC-15).

#### OD-13 Parallelism
- **M13.1** Speedup S(n) = T(1)/T(n) for pack, unpack and verify at n ∈ {1, 2, 4, 8, 16, 24, 32} threads. **M13.2** Efficiency S(n)/n. Unit: 1 [`time_wall`].
- **Procedure.** `ebr --timing --cpus` sets on the Windows host (physical cores), with WSL secondary. Affinity sets record the P/E split: P-cores are Windows logical 0-15, E-cores 16-31. [FORMALLY_DERIVED from PROGRESS.md environment] Thermal and hybrid-core controls follow `decision-method.md`.
- **Direction.** Maximize.
- **Aggregation.** AGG-G of S(8) and S(32) with worst-family guard.
- **Type.** FLOOR+GRADED. The floor is HC-11 and HC-13: output identity across n.

#### OD-14 Corruption locality
- **M14.1** Blast radius: logical bytes, and separately entries, not verifiable as intact per injected event, divided by damaged stored bytes. Unit: ratio [`other`]. **M14.2** Localization recall and precision: the entries `verify` reports as affected versus the oracle set derived from the closure (HC-09). Unit: fraction [`correctness`]. **M14.3** Truncation salvage: the fraction of oracle-intact entries identified as intact after truncation. Unit: fraction [`correctness`].
- **Procedure.** A seeded harness injector applies single-bit flips, 512 B, 64 KiB and 1 MiB overwrites, and truncation at section boundaries and random offsets. Targets are payload, dictionaries, manifest, index and footer, followed by `verify` and `salvage`. Incumbent references: ZIP, tar.gz, tar.zst, 7z solid and non-solid.
- **Direction.** Minimize M14.1; maximize M14.2 precision and M14.3.
- **Aggregation.** Per injection class, median and p95 per item, then AGG-G across families plus AGG-W.
- **Type.** FLOOR+GRADED. M14.2 recall must be 1: an affected entry reported intact violates HC-15.

#### OD-15 Confidentiality and authenticity
- **M15.1** Undetected tampers from MVT-14(b), extended to signatures and bindings. Unit: count [`correctness`]. **M15.2** Authenticated coverage: the fraction of archive bytes that are authenticated or bound as associated data. Unit: fraction [`correctness`]. **M15.3** Encryption overhead: bytes (envelope, framing, padding) and time relative to the unencrypted pack of the same EAM. Unit: ratio [`size`, `time_wall`]. **M15.4** Assurance state per property: vector-verified < internally reviewed < externally reviewed. Unit: ordinal [`other`]. **M15.5** Declared security level per recipient policy, classical and post-quantum. Unit: bits. [EXTERNALLY_SOURCED from primitive specifications]
- **Direction.** M15.1 must be 0 and M15.2 must be 1; minimize M15.3; maximize M15.4.
- **Aggregation.** AGG-W for M15.1 and M15.2; AGG-G for M15.3; AGG-P for M15.4 and M15.5.
- **Type.** FLOOR+GRADED (HC-14). M15.4 progress beyond "internally reviewed" requires external review. [EXPERT_REVIEW_REQUIRED]

#### OD-16 Metadata privacy
- **M16.1** Public plaintext-derived information: an inventory of fields visible without keys (counts, sizes, segment and record counts, lengths, boundaries) with an upper bound in bits per archive. Unit: bits [`other`]. **M16.2** Presence-confirmation advantage: for a passive adversary holding a candidate file, the best-attack true-positive rate minus false-positive rate at a pre-registered false-positive rate. Attacks follow the published content-defined-chunking and length-leakage literature that SPEC §7.6 cites. Unit: 0-1 [`other`]. **M16.3** Padding overhead. Unit: B, ratio [`size`]. **M16.4** Remote access-pattern leakage: distinct range-length signatures observable by the origin per accessed entry. Unit: count [`io`].
- **Procedure.** Harness leakage analyzer over encrypted packs of tuning items, attack implementations run offline, and `ebr-origin` request logs for M16.4.
- **Direction.** Minimize, with M16.3 in tension (see §4.3).
- **Aggregation.** AGG-W (worst family) for M16.2; AGG-P for M16.1; AGG-G for M16.3.
- **Type.** GRADED. The HC-14 floor covers only keyed boundary derivation. The residual leakage is the design objective I24 names, not a proven property. [FORMALLY_DERIVED from REQ-CRY-0097; EXPERT_REVIEW_REQUIRED for attack-model adequacy]

#### OD-17 Resource-bounded decoding
- **M17.1** Tightness: declared requirement divided by measured actual, for memory, decoded bytes, entries and chunks. Must be ≥ 1 (HC-06), with smaller being more useful to callers. Unit: ratio [`other`]. **M17.2** Refusal cost: peak RSS, CPU time and bytes read before refusal of hostile archives. Units: B, s, B [`memory_peak`, `time_cpu`, `io`]. **M17.3** Default-policy false-refusal rate: the fraction of benign tuning and validation items refused under the default CLI policy. Unit: fraction [`correctness`]. **M17.4** Default-policy true-refusal rate on the MVT-06 adversarial set. Must be 1. Unit: fraction [`correctness`].
- **Direction.** Minimize M17.1 (toward 1), M17.2 and M17.3; M17.4 must be 1.
- **Aggregation.** AGG-G for M17.1; AGG-W for M17.2 and M17.4; AGG-C for M17.3.
- **Type.** FLOOR+GRADED.

#### OD-18 Cross-platform fidelity
- **M18.1** Identity agreement across hosts (MVT-08(a)). Unit: binary per archive [`correctness`]. **M18.2** Cross-platform restore fidelity: OD-03 M03.2 restricted to pairs with different source and target platforms. Unit: fraction [`correctness`]. **M18.3** Hostile-name handling: the fraction of hostility-corpus cases that extract faithfully when representable, or else refuse or rename with a report. Unit: fraction [`correctness`]. **M18.4** Platform coverage: platforms tested out of the platforms the SPEC names; blocked platforms are listed. Unit: count [`other`].
- **Direction.** Maximize.
- **Aggregation.** AGG-S (minimum over platform pairs).
- **Type.** FLOOR+GRADED (floors HC-08, HC-07).
- **Note.** The M18.4 gap for macOS and native ARM64 is `UNRESOLVED`.

#### OD-19 Legacy interoperability
- **M19.1** Import acceptance per legacy format and policy (strict, compat-ID, preservation): the fraction of real-world inputs imported. Inputs are corpus F14 plus public legacy corpora recorded under `research/corpus/sources`. Unit: fraction [`correctness`]. **M19.2** Import agreement: the fraction of unambiguous inputs whose imported EAM matches the consensus of reference readers (Python zipfile/tarfile, Java, libarchive/bsdtar, GNU tar, 7-Zip 23.01). Unit: fraction [`correctness`]. **M19.3** Ambiguity refusal: the fraction of Research III ambiguity classes refused or declared, which must be 1 under strict. Unit: fraction [`correctness`]. **M19.4** Export acceptance: the fraction of exported artifacts that incumbent readers (unzip, bsdtar, GNU tar, 7z, Python, Java) read to the EAM-equivalent tree, plus the LOSSLESS/LOSSY/REFUSED distribution over corpus EAMs. Unit: fraction [`correctness`]. **M19.5** Import and export time and memory, as OD-06 to OD-08.
- **Procedure.** `ebr` specs invoking `ebound import` and `export` plus incumbent tools in WSL and Docker; `tools/zip-compat` and `tools/tar-strict` matrices.
- **Direction.** Maximize M19.1, M19.2 and M19.4, subject to M19.3 = 1.
- **Aggregation.** AGG-S per legacy format, with the headline at the minimum over formats.
- **Type.** FLOOR+GRADED (floors HC-07, HC-17, HC-05).
- **Note.** Strictness lowers M19.1 by design (§4.3).

#### OD-20 Deterministic migration
- **M20.1** Identity-rule conformance: the fraction of migrations whose LAI, PCR, AUX and signature-binding behavior matches SPEC §8.1 and F-08. Migrations are legacy → `.eb`, repack by profile, layout or index, key add and remove, export → re-import, and multi-target publish. Must be 1. Unit: fraction [`correctness`]. **M20.2** Migration output determinism: the fraction of deterministic-target migrations byte-identical across runs and hosts. Unit: fraction [`correctness`]. **M20.3** Migration cost: time, peak memory and scratch, and artifact byte delta relative to reference. Unit: ratio [`time_wall`, `memory_peak`, `size`]. **M20.4** Receipt completeness: the fraction of changed or dropped items recorded in receipts. Must be 1. Unit: fraction [`correctness`].
- **Direction.** Maximize M20.1, M20.2 and M20.4; minimize M20.3.
- **Aggregation.** AGG-C for M20.1, M20.2 and M20.4; AGG-G for M20.3.
- **Type.** FLOOR+GRADED (floors HC-10, HC-13, HC-07).

#### OD-21 Independent implementability
- **M21.1** Clean-room baseline pass rate (MVT-12(a)). Must be 1. Unit: fraction [`correctness`]. **M21.2** Independent-reader size for the baseline feature set, counted with `tokei` at a pinned version, excluding tests. Unit: LoC [`other`]. **M21.3** Specification defects raised per 1,000 normative words during clean-room implementation. Unit: count per 1,000 words [`other`]. **M21.4** Normative-rule coverage by conformance cases. Must be 1. Unit: fraction [`correctness`]. **M21.5** Baseline decode steps without a public specification or permanent vectors. Must be 0. Unit: count [`correctness`].
- **Direction.** Maximize M21.1 and M21.4; minimize M21.2, M21.3 and M21.5.
- **Aggregation.** AGG-P.
- **Type.** FLOOR+GRADED (HC-12). M21.2 and M21.3 from a same-program implementer are biased low. [EXPERT_REVIEW_REQUIRED]

#### OD-22 Dependency longevity
- **M22.1** Decode-path dependency count: direct plus transitive crates from `cargo tree -e normal` on the decode feature set with the pinned lockfile, counted separately from the create-only path. Unit: count [`other`]. **M22.2** Native (C/C++) code and `unsafe` blocks in decode-path dependencies (`cargo geiger` or equivalent). The workspace itself has `unsafe_code = "forbid"`. Unit: count [`other`]. **M22.3** RustSec advisories and license compatibility (`cargo deny`). Unit: count, binary [`correctness`]. **M22.4** Maintenance snapshot per dependency: last release age, maintainer count, MSRV. Recorded with retrieval date. Unit: years, count [`other`]. [EXTERNALLY_SOURCED at retrieval] **M22.5** Public format specification availability for every decode-path codec or transform. Unit: binary per dependency [`correctness`].
- **Direction.** Minimize M22.1-M22.4; maximize M22.5.
- **Aggregation.** AGG-P, with the decode path primary.
- **Type.** FLOOR+GRADED. M22.3 license must pass; M22.5 for baseline decode must pass (HC-12).
- **Note.** The snapshot tooling script is planned under `research/tools/`. [UNRESOLVED]

#### OD-23 Specification complexity
- **M23.1** Normative words, counted by script over the candidate specification (SPEC plus `docs/*-v1.md` normative sections). **M23.2** Record types, fields, feature bits, registry entries and frozen identifiers, from a registry census. **M23.3** Delta of M23.1 and M23.2 relative to REF-PROD: the permanent wire and specification cost of the candidate. Unit: count [`other`].
- **Direction.** Minimize.
- **Aggregation.** AGG-P.
- **Type.** GRADED. The minimum gains that justify added cost are set in `decision-method.md`.
- **Note.** Word and record counts are proxies for complexity. [INFERRED]

#### OD-24 Implementation attack surface
- **M24.1** Lines of code in functions reachable from untrusted bytes, across decode, import, remote and unlock paths. Unit: LoC [`other`]. **M24.2** Fuzz findings (crash, panic, timeout, out-of-memory) per CPU-hour per parser target, under a fixed campaign budget, plus coverage reached. Unit: count/h, fraction [`other`]. **M24.3** Attacker-controlled parameters that influence allocation or CPU, from the MVT-06(c) audit. Unit: count [`other`]. **M24.4** SPEC §11.2 vulnerability classes the candidate handles only by "validated" rather than "not representable", "kernel-enforced" or "declared and bounded". Unit: count [`other`].
- **Procedure.** cargo-fuzz campaigns in WSL; reachability via call-graph tooling; `tokei`.
- **Direction.** Minimize.
- **Aggregation.** AGG-P for M24.1, M24.3 and M24.4; AGG-W (sum over targets) for M24.2.
- **Type.** FLOOR+GRADED. Any untrusted-input crash or panic is a defect that blocks `DECIDED` for the affected component until fixed (via HC-03 and HC-06). [EXPERT_REVIEW_REQUIRED for reachability completeness]

#### OD-25 User complexity
- **M25.1** First-use task success. Tasks: pack, unpack, verify, encrypt to a recipient, extract one entry, remote-read one entry, import a ZIP, export a tar, and explain what was not preserved. Each is scored against a pre-registered completion criterion. Unit: fraction [`correctness`]. **M25.2** Flags and concepts required per task with defaults. Unit: count [`other`]. **M25.3** Error actionability: the fraction of refusal outcomes whose message names the failed requirement and a remedy. Unit: fraction [`correctness`]. **M25.4** Unsafe defaults: defaults that enable a dangerous restoration, trust or overwrite. Must be 0. Unit: count [`correctness`]. **M25.5** Task completion time. Unit: s [`time_wall`].
- **Procedure.** Human participants are unavailable, so evaluation uses simulated first-use sessions by agents given only the CLI help and public docs. They are labeled SIMULATED, and the transcripts are committed. Flag and message censuses are scripted.
- **Direction.** Maximize M25.1 and M25.3; minimize M25.2 and M25.5; M25.4 must be 0.
- **Aggregation.** AGG-S (minimum over tasks) for M25.1; mean and maximum for M25.2.
- **Type.** FLOOR+GRADED (floors HC-05, HC-15).
- **Note.** Simulated results do not generalize to humans. [EXPERT_REVIEW_REQUIRED; UNRESOLVED]

#### OD-26 Adoption friction
- **M26.1** Zero-install consumability: the fraction of pre-registered consumer scenarios served with preinstalled tools via dual publishing of legacy artifacts (OD-19 M19.4). Scenarios come from the SPEC §21 staged path. Unit: fraction [`correctness`]. **M26.2** Migration steps: commands and flags needed to replace an existing tar or zip workflow, from scripted reproductions in pinned Docker images. Unit: count [`other`]. **M26.3** Build and binary availability: successful builds and binaries on Windows x86-64, Linux x86-64 and emulated arm64; build time; MSRV. Unit: count, s [`correctness`, `time_wall`]. **M26.4** Identification support: status of the `.eb` extension, media type and magic in file(1)/libmagic, shared-mime-info and IANA, with retrieval date. Unit: count [`other`]. [EXTERNALLY_SOURCED]
- **Direction.** Maximize M26.1, M26.3 and M26.4; minimize M26.2.
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
| HC-09 | OD-05, OD-10, OD-14 |
| HC-10 | OD-11, OD-20 |
| HC-11 | OD-04, OD-07, OD-13 |
| HC-12 | OD-21, OD-22 |
| HC-13 | OD-06, OD-13, OD-20 |
| HC-14 | OD-05, OD-12, OD-15, OD-16, OD-22 |
| HC-15 | OD-04, OD-07, OD-09, OD-10, OD-11, OD-12, OD-14, OD-25 |
| HC-16 | OD-23, OD-26 |
| HC-17 | OD-19 |

### 4.3 Known tensions and where they are resolved

In every row the HC side is fixed. The trade-off among ODs is decided by `decision-method.md` (Pareto, practical significance, sensitivity, permanent cost). [INFERRED unless tagged]

| Tension | Fixed side | Traded among | Source |
|---|---|---|---|
| Solid-grade compression versus random access and corruption locality | HC-09 bounded lookback | OD-05 vs OD-10, OD-14 | SPEC §7.5, §23.4 ("a few percent versus solid") [FORMALLY_DERIVED] |
| Padding versus size | HC-14 keyed boundaries and no deterministic ciphertext | OD-16 vs OD-05, OD-12 | SPEC §9.8, §23.4 [FORMALLY_DERIVED] |
| Verified staging versus time to first output byte | HC-05 and HC-15 (F-11) | OD-07 M07.4, OD-09 M09.3 vs OD-08 scratch | CONTRIBUTING.md L137-140 [FORMALLY_DERIVED] |
| Parallel speed versus determinism | HC-11, HC-13 | OD-13 vs OD-06 | SPEC §12.2 (multithreaded codec output differs) [FORMALLY_DERIVED] |
| Import acceptance versus strictness | HC-07, HC-17, HC-05 | OD-19 M19.1 vs OD-26 | SPEC §2.1(2); CONTRIBUTING.md L185-193 [FORMALLY_DERIVED] |
| Write-time verification versus creation speed | HC-02 (I31) | OD-06 | SPEC §23.4 [FORMALLY_DERIVED] |
| Ratio gains from new codecs or transforms versus permanent cost | HC-12, HC-16 | OD-05 vs OD-21, OD-22, OD-23, OD-24 | SPEC §24 item 9, §20.6 |
| Richer preservation versus identity stability | HC-10 (LAI/AUX split) | OD-03 vs OD-20 | SPEC §8.3, §12.4 [FORMALLY_DERIVED] |
| Remote efficiency versus access-pattern privacy | HC-14 | OD-12 vs OD-16 M16.4 | SPEC §9.8 |
| Fewer user concepts versus declared honesty | HC-07, HC-15 | OD-25 vs OD-03 and OD-11 reporting detail | SPEC §23.4 ("more concepts than ZIP") [FORMALLY_DERIVED] |

### 4.4 Frozen identifiers and permanent cost

A candidate that would change frozen behavior is admissible only as a new identifier (HC-16). Its permanent cost is charged to OD-21 to OD-24 even if it is later deprecated for new archives, because decoders must support it indefinitely (SPEC §12.2, §20.4). [FORMALLY_DERIVED]

### 4.5 What may and may not move an HC

This program cannot relax an HC. A finding that an HC imposes a large cost is reported as a negative result with the measured cost, attached to the relevant CC or OD. Two situations are escalated as `UNRESOLVED`, with evidence, to a SPEC revision process outside this program:
- two HCs are shown to be mutually unsatisfiable for some required capability (for example, HC-06 declared totals versus unseekable streaming input, which the SPEC already resolves through `BudgetDeclared = false`);
- an HC's MVT is shown to be ill-defined.

[FORMALLY_DERIVED from SPEC §2.1(4), §23.5; INFERRED procedure]

### 4.6 Unverifiable HCs

An HC_UNVERIFIED status (§2.0 rule 4) limits the *scope* of a decision. It never becomes a pass. For example, a decision validated on Windows and Linux states that macOS extraction confinement and metadata fidelity are unverified, and the decision's `decision_scope` and `known_limitations` fields say so. [INFERRED]

---

## 5. Threats to this objective

1. **Taxonomy bias.** The HC set, the MVT oracles and the OD metric proxies were chosen by the program itself. The adversarial review exists to attack them. Any HC or OD added or changed after the first decision-relevant result invalidates pre-registration for the affected decisions. [INFERRED]
2. **Instrument gap.** Most MVTs and many ODs depend on instruments not built at drafting: the harness crates, the conformance corpus, the independent reader, the trace shim and leakage analyzers. Until they exist, those HCs are HC_UNVERIFIED for every candidate, including production. [FORMALLY_DERIVED from repository state at `f56dd3d`; UNRESOLVED]
3. **Ledger screen noise.** Appendix C's keyword mapping is heuristic, and false positives and negatives remain after manual review. It supplies anchors and weak-evidence pointers, not HC membership proofs. [INFERRED]
4. **Platform and participant blocks.** macOS, APFS, native ARM64, ReFS, admin-only Windows features, packet-level network emulation and human usability participants are unavailable, which affects HC-05, HC-07, HC-08, OD-03, OD-12, OD-18 and OD-25. [FORMALLY_DERIVED from PROGRESS.md; UNRESOLVED]
5. **Same-organization independence.** HC-12, OD-21 and every "independent" oracle are weakened when produced inside the program. [EXPERT_REVIEW_REQUIRED]
6. **Crypto assurance.** Conformance to the frozen suite is testable internally. Soundness of the constructions is not. [EXPERT_REVIEW_REQUIRED]

---

## Appendix A — Invariant coverage (I1-I31 to HC)

Every invariant of SPEC §3.9 and Appendix A maps to at least one HC, and every HC maps to at least one invariant or named freeze. The "Ledger anchors" column lists rows whose text names the invariant or its SPEC alias (Appendix C.1). "Enforced by" is the SPEC Appendix A column. [FORMALLY_DERIVED]

| Inv. | Invariant (SPEC Appendix A) | Enforced by (SPEC) | HCs | Ledger anchors |
|---|---|---|---|---|
| I1 | One semantic authority per fact | Model §3.2 | HC-01 | REQ-MOD-0018, REQ-MOD-0099 |
| I2 | No duplicated semantic headers | Container §4.1 | HC-01 | REQ-CON-0081 |
| I3 | Unique logical paths | Manifest §3.4 | HC-17, HC-03 | REQ-MOD-0039, REQ-MOD-0111 |
| I4 | Paths are structural sequences | Model §3.4 | HC-08, HC-17 | REQ-MOD-0075, REQ-MOD-0077, REQ-MOD-0080, REQ-PLT-0100, REQ-PLT-0132 |
| I5 | No `..` or `.` | Model §3.4 | HC-17, HC-08 | REQ-MOD-0026 |
| I6 | No duplicate entries | Manifest | HC-17, HC-03 | REQ-MOD-0039, REQ-MOD-0111 |
| I7 | Explicit entry identity | Model §3.2 | HC-10 | REQ-MOD-0031 |
| I8 | Compression independent of logical identity | Identity §8.1 | HC-10, HC-13 | REQ-MOD-0013, REQ-MOD-0060 |
| I9 | Encryption independent of logical identity | Identity §8.1 | HC-10, HC-13, HC-14 | REQ-CRY-0077, REQ-MOD-0060 |
| I10 | Indexes are reconstructible caches | Container §4.5 | HC-01 | REQ-CON-0050 |
| I11 | Typed, namespaced metadata | Metadata §10 | HC-07, HC-04 | REQ-PLT-0068 |
| I12 | Unknown critical features fail closed | Feature bitmaps §20.1 | HC-04 | REQ-CON-0096 |
| I13 | No silent metadata loss | Fidelity Report §10.9 | HC-07 | REQ-CRY-0033, REQ-PLT-0079 |
| I14 | No lossy transforms | Losslessness contract §6.5 | HC-02, HC-07 | REQ-CMP-0077, REQ-LEG-0101 |
| I15 | Deterministic, unique interpretation | Canonical form §11.4 | HC-03, HC-11, HC-16 | REQ-MOD-0110 |
| I16 | Extraction policy is caller-owned | Security §11.8 | HC-05 | REQ-MOD-0001, REQ-PLT-0007 |
| I17 | Resource limits declared and validated pre-allocation, or inability declared | Preamble §4.2, §11.5 | HC-06 | REQ-ACC-0155, REQ-CON-0005 |
| I18 | Legacy ambiguity surfaced, never normalized | Observation model §14.2 | HC-07 | REQ-LEG-0056 |
| I19 | Every size, offset and count declared once | Container §4.6 | HC-01, HC-06 | REQ-ACC-0019, REQ-CON-0081, REQ-MOD-0056, REQ-MOD-0076 |
| I20 | No unbounded list parsed from a header | Container §4.2, §10.7 | HC-06 | REQ-ACC-0155, REQ-CON-0004, REQ-MOD-0056 |
| I21 | Truncation, corruption and unsupported are distinguishable | Preamble and footer §4.2, §4.4 | HC-15, HC-04 | REQ-CON-0032, REQ-CON-0091, REQ-INT-0024 |
| I22 | Published identity binds parameters, not a bare root | Identity §8.3 | HC-10, HC-16 | REQ-MOD-0047 |
| I23 | Hard links are content sharing | Model §3.3 | HC-01, HC-17 | REQ-MOD-0043 |
| I24 | Encrypted chunk boundaries must not leak plaintext structure | Chunking policy §7.6 | HC-14 (residual graded in OD-16) | REQ-CRY-0097 |
| I25 | Unverified results reported as unverified | Reader contract §11.4, §11.6, §20.5 | HC-15 | REQ-ECO-0177, REQ-INT-0025, REQ-PLT-0014 |
| I26 | No non-Directory proper prefix; explicit ancestors | Model §3.4 P6 | HC-17, HC-03 | REQ-MOD-0039, REQ-PLT-0023 |
| I27 | `.` and `..` excluded under every encoding | Model §3.4 P7 | HC-17, HC-08 | REQ-MOD-0026, REQ-MOD-0077, REQ-PLT-0150 |
| I28 | Content addressing over plaintext; LAI chunking-independent, PCR not | Model §3.5, identity §8.0 | HC-10, HC-02 | REQ-MOD-0013, REQ-MOD-0060 |
| I29 | Chunks independently decodable unless in a declared-lookback group | Model §3.5, grouping §7.5 | HC-09, HC-06 | REQ-ACC-0040, REQ-CMP-0078 |
| I30 | Transform plans are data | Model §3.6 | HC-11, HC-12, HC-16 | REQ-CMP-0117 |
| I31 | Every reconstructive transform round-trip verified at write time | Planner §5.6 | HC-02 | REQ-CMP-0079 |

Reverse check: HC-01 {I1, I2, I10, I19, I23}; HC-02 {I14, I28, I31}; HC-03 {I3, I6, I15, I26}; HC-04 {I11, I12, I21}; HC-05 {I16}; HC-06 {I17, I19, I20, I29}; HC-07 {I11, I13, I14, I18}; HC-08 {I4, I5, I27}; HC-09 {I29}; HC-10 {I7, I8, I9, I22, I28}; HC-11 {I15, I30}; HC-12 {I30}; HC-13 {I8, I9}; HC-14 {I9, I24}; HC-15 {I21, I25}; HC-16 {I15, I22, I30}; HC-17 {I3, I4, I5, I6, I23, I26, I27}. Every HC has at least one invariant, and all 31 invariants appear. [FORMALLY_DERIVED]

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

## Appendix C — Requirement-ledger screen (generated)

**Method.**
1. Explicit references to I1-I31 and to the SPEC aliases P1-P8, C1-C2 and E1-E3 map through Appendix A to HCs. T1 and T2 are not screened, because the ledger uses "T1" for the crypto-v1 tuple transcript encoding.
2. Keyword regular expressions per HC are applied.
3. Manual assignments cover rows the screens missed, and manual exclusions remove keyword false positives found on review.
4. Rows without an HC tag are screened against OD keywords.

The rules live in `research/tools/ledger/objective_screen.py`. Everything below this paragraph is that tool's output, verbatim. [FORMALLY_DERIVED counts at the stated hashes; INFERRED keyword membership]

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
