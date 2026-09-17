# EXP-CONF-003 — Normative-statement extraction with recall validation, rule-to-case traceability, and registry, reason-code and doc-code consistency census

| Field | Value |
|---|---|
| Status | `NEEDS_TOOLING` (CC§2): extraction script, recall sampler, labelling template, case mapper, reason-code census, registry audit, doc cross-reference checker |
| Domain / program sections | conformance / §29 (primary), §28 |
| Decision IDs informed | DEC-MOD-031, DEC-MOD-041, DEC-MOD-042, DEC-MOD-014, DEC-CON-003, DEC-CON-011, DEC-CON-029, DEC-ECO-012, DEC-ECO-013, DEC-ECO-045, DEC-INT-020, DEC-INT-037, DEC-CRY-012, DEC-CRY-047, DEC-CRY-077, DEC-CRY-085, DEC-LEG-004, DEC-LEG-026 |
| Requirements | REQ-ECO-0021, REQ-ECO-0136, REQ-ECO-0039 |
| HC oracles | HC-12 MVT-12(b) (extraction recall ≥ 0.95; every statement mapped); HC-03 MVT-03(c) (decided class per rule); proposal for the assigner: `hc_ids` HC-12, HC-03, HC-16 (registry reuse); `od_ids` OD-01, OD-04, OD-21, OD-23 |
| Decision type | not assigned (gate G-A) |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1 |
| Depends on | clean-room snapshot of EXP-CONF-001 §5 (same document set); `ebcc-v1` manifest of EXP-CONF-004 for rule-to-case mapping (step 4 runs after the corpus exists; steps 1-3 and 5-8 do not depend on it) |

## 1. Question

Can every normative statement of the specification be extracted mechanically with recall of at least 0.95 and mapped
to at least one conformance case with a decided outcome class and reason code; and how many normative statements,
reason codes and registry assignments lack a case, a documented definition, or agreement between documents and code?

## 2. Hypotheses and falsification

- **H1 (MVT-12(b) extraction recall).** The pre-registered grammar extracts ≥ 0.95 of the sentences an independent
  labeller marks normative in a seeded sample of 100 snapshot sentences. *Falsified* by recall < 0.95 (point estimate;
  the objective's criterion). Consequence pre-registered by the objective: HC-12 cannot pass until the specification
  is rewritten with RFC 2119 keywords.
- **H2 (MVT-12(b) traceability, MVT-03(c)).** Every extracted statement maps to ≥ 1 `ebcc-v1` case with a decided class
  (`normative_rule_coverage_fraction` = 1). *Falsified* by one unmapped statement or one mapped statement whose cases are
  all `UNRESOLVED`.
- **H3 (reason-code registry closure).** Every reason code production emits is defined in a normative document and is
  asserted by at least one test or conformance case, and every documented code is emitted by some path. *Falsified* by
  one code outside the intersection.
- **H4 (registry consistency, HC-16 MVT-16(b)).** No identifier value (record type, record version, TLV tag, feature
  bit, section type, STREAM item tag, hash domain, codec/transform id, metadata item name, reason code) is assigned two
  meanings across documents and code. *Falsified* by one collision.

## 3. Contribution per decision

| Decision | Contribution |
|---|---|
| DEC-MOD-031 | Traceability matrix from invariants I1-I31 and P1-P8 (and the omitted rules P2, P5, P8, directory-without-content, hole semantics) to statements and cases (candidate `traceability-matrix`) |
| DEC-MOD-041, DEC-INT-020, DEC-CRY-085, DEC-MOD-014 | Reason-code census: emitted ∖ documented, documented ∖ emitted, emitted ∖ asserted, per prefix and class; signature codes unasserted; diagnostic emission sites with available entry/offset/section context |
| DEC-MOD-042, DEC-CON-003, DEC-CON-029 | Registry audit (collisions, gaps, doc-only, code-only, value mismatches) per registry source — evidence for `code-as-registry`, `consolidated-normative-registry-doc` and `machine-readable-registry` candidates; element-by-element freeze table checked against vectors and code; doc-code contradiction count |
| DEC-ECO-012 | Fraction of statements whose case mapping is computable mechanically under each candidate ID scheme (inputs to EXP-CONF-005 arm A) |
| DEC-ECO-013 | Rule-to-case audit: statements whose cases would be `UNRESOLVED` |
| DEC-ECO-045 | Defect counts found by the automated cross-reference checker (candidate `automated-crossref-checker`) |
| DEC-CON-011 | Operation-by-tier matrix: which (command, ro_compat/compat tier) cells have a governing statement |
| DEC-CRY-047, DEC-CRY-012, DEC-CRY-077 | Specification text audit of `max_key_derivation_cost` across format-v0, crypto-wire and SPEC; provenance of each security-reporting field; error-surface inventory of secret-dependent failures (codes and message templates per failure) |
| DEC-INT-037 | Statements and cases per archive role (Complete, Sidecar, Incremental): the conformance and independent-reader surface each role adds (descriptive) |
| DEC-LEG-004, DEC-LEG-026 | Enumeration of emitted conversion-record string values per adapter (from source); refusal paths per legacy format versus fixtures that reach them |

## 4. Candidates

No design candidate is executed; the decisions' ledger candidates are informed by counts (CC§1: none added). Extraction
grammar is fixed by the objective (MVT-12(b)) and is not a candidate here.

## 5. Inputs

- The clean-room snapshot document set of EXP-CONF-001 §5 (by manifest SHA-256), including the SPEC copy; plus, for
  steps 5-8 only, production source (`crates/**`) and tests at the recorded `ebound-head` commit, read by this
  experiment's scripts (not by any reader builder).
- `research/conformance/corpus/ebcc-v1/manifest.json` (step 4).
- No corpus item of any split.

## 6. Environment

Windows host Python 3.13.5 or WSL Python 3.12.3, standard library only; scripts produce byte-identical outputs on both
(checked in step 9). No timing, no network, no privileged operation.

## 7. Procedure and commands

All scripts live in `research/conformance/traceability/` and write outputs with a provenance header (command, input
SHA-256s, script SHA-256; §12.3).

1. **Extraction.** `extract_normative.py --snapshot <snap> --grammar mvt12b-v1 --out statements-v1.jsonl`. Grammar
   `mvt12b-v1`, exactly the objective's MVT-12(b) grammar: sentences containing, case-insensitively, `must`,
   `must not`, `shall`, `required`, `never`, `always`, `prohibited`, `may not`, `is rejected`, `is refused`, or `only`
   used as a restriction (operationalized as `only` followed within four tokens by `if`, `when`, `after`, `before`,
   `for`, `by`, `from`, `as`, or a verb in a closed list recorded in the script); plus every row of a table that the
   document marks normative (a table under a heading containing `normative`, `frozen`, `registry`, `format`, `layout`,
   `wire`, `vectors`, or a table whose preceding paragraph says it is normative). Statement ID: `NS-<doc-stem>-<first
   10 hex of SHA-256 of the whitespace-normalized sentence>` (stable under line moves). Sentence segmentation rules
   are part of the script and pinned by its SHA-256.
2. **Recall sample.** `recall_sample.py --snapshot <snap> --seed 2809170002 --n 100 --out recall-sample-v1.jsonl`
   draws 100 sentences uniformly from **all** snapshot sentences (normative or not), hiding extraction results.
   Pre-registered augmentation: if fewer than 20 sampled sentences are labelled normative, 100 further sentences are
   drawn with seed `2809170002 + 1` and both samples are pooled.
3. **Labelling.** Labeller L1 (CC§11) labels each sampled sentence `NORMATIVE` or `NOT_NORMATIVE` using written
   criteria (`labelling-criteria-v1.md`, committed before labelling: a sentence is normative if an implementation could
   conform or fail to conform to it). Labeller L2 independently labels the same sentences. Recall is computed against
   L1; L2 labels give Cohen's kappa and a recall under L2 (sensitivity). `recall.py` reports recall, precision and exact
   Clopper-Pearson 95% intervals (descriptive; the H1 criterion is the point estimate).
4. **Rule-to-case mapping.** `map_cases.py --statements statements-v1.jsonl --manifest ebcc-v1/manifest.json` joins on
   `ns_ids`; outputs `traceability-v1.csv` (statement, cases, classes, codes), the unmapped list, statements covered only
   by positive cases, and statements whose cases are all `UNRESOLVED`. Invariants I1-I31 and P1-P8 are mapped to
   statements by `invariant_map.py` using the objective's Appendix A anchors and SPEC section references.
5. **Reason-code census.** `reason_code_census.py --src crates --tests crates --docs <snap>/docs --manifest ebcc-v1/manifest.json`:
   codes as string literals and enum variants in source (with emission site file:line), codes asserted in tests, codes
   defined in documents, codes expected in `ebcc-v1`, codes reached by the fuzz corpus (EXP-CONF-009 replay output,
   when available). Set differences per prefix and class.
6. **Registry audit.** `registry_audit.py --src crates --docs <snap>/docs`: extracts assignments for record types,
   record versions, TLV tags, feature bits (per tier), section types per feature, STREAM item tags, hash-domain strings,
   codec and transform identifiers, metadata item names, reason codes; detects collisions, gaps, doc-only, code-only
   and value mismatches; emits `registry-audit-v1.csv` and the element freeze table `freeze-table-v1.csv`
   (element, frozen claim location, vector present, code location).
7. **Cross-reference checker.** `doc_crossref.py --snapshot <snap>`: broken section references, counts stated in text
   that disagree with the table they describe, identifiers named in text but absent from registries, "frozen" claims
   without vectors, contradictory statements (same subject, opposite modality) detected by a pre-registered pattern
   list; each finding with document and line.
8. **Special inventories.** `op_tier_matrix.py` (DEC-CON-011), `role_surface.py` (DEC-INT-037),
   `crypto_text_audit.py` (DEC-CRY-047 field meaning across three documents; DEC-CRY-012 field provenance;
   DEC-CRY-077 failure → code/message table), `legacy_string_census.py` (DEC-LEG-004 emitted strings per adapter;
   DEC-LEG-026 refusal paths versus fixtures in tests).
9. **Determinism.** `make_all.sh` runs steps 1, 2, 4-8 twice (Windows host and WSL); outputs must be byte-identical
   (§4.13 repeat-hash); a difference is a script defect fixed before any number is cited.

## 8. Seeds, warmup, replication

Recall sample seed `2809170002`. No warmup. Scripts: two runs on two hosts (step 9). Labelling: two labellers.

## 9. Metrics (CC§9)

| Metric | Role | Unit | Definition |
|---|---|---|---|
| `normative_rule_coverage_fraction` | binary (HC-12) | fraction | mapped statements with ≥ 1 decided case ÷ extracted statements |
| `conditional_decoding_rules_count` | descriptive (M01.2) | count | statements whose applicability depends on kind, role, layout or feature combination (pattern list in the script) |
| `normative_words_per_rule` | descriptive (M23.1) | words/rule | normative words ÷ extracted statements, per document and total |
| `spec_defects_per_1000_words` | descriptive (M21.3) | count/1000 words | cross-reference defects + registry collisions + doc-code mismatches per 1,000 normative words (reported separately from EXP-CONF-001's ambiguity-based value) |
| Extraction recall and precision (MVT-12(b) criterion ≥ 0.95 on recall); kappa; unmapped statements; reason-code set differences; registry collisions, gaps and mismatches; cross-reference defects; operation-tier cells without a rule | binary criterion (recall) / descriptive, non-canonical | fraction, count | steps 3-8 |

## 10. Normalization and statistical analysis

Deterministic counts (AGG-P). The recall criterion is the objective's pre-registered number applied to the point
estimate; the Clopper-Pearson interval is reported so readers see its width (with ≈ 20-40 normative sentences in the
sample it is wide; this is a stated limitation of MVT-12(b), not a reason to change the criterion). Analysis tables per
decision (§3). The analysis plan needs adoption by a non-exposed session (CC§1).

## 11. Practical-significance thresholds

`normative_rule_coverage_fraction`: §3.3 (binary). Recall criterion: objective MVT-12(b) (0.95). Descriptive counts: no
band.

## 12. Sensitivity analysis

- Recall and coverage under L2 labels; with table rows excluded; with the `only` restriction rule disabled.
- Coverage with SPEC-only statements versus repository-document-only statements.
- Registry audit restricted to frozen identifiers (CONTRIBUTING freezes) versus all identifiers.

## 13. Split discipline and held-out placeholder

No corpus split is used. The analysis is re-run on the frozen documents and build at Commit A (§5.5) so that Phase D
decisions cite the frozen traceability matrix. No held-out placeholder is needed.

## 14. Expected negative results worth recording

- Recall below 0.95 because normative force sits in lowercase prose and tables (objective review finding B08).
- Reason codes present in source but not in documents, and emitted codes without asserting tests.
- Feature-bit and section-number drift between documents (the ledger cites `0x200` and `0x1000` drift).
- Normative statements covered only by positive cases (no negative case).

## 15. Threats to validity

- **Grammar validity**: the grammar is fixed by the objective; its false positives inflate the unmapped count, its
  false negatives hide rules (measured by recall).
- **Labelling judgment** and shared base model between labellers (CC§11); kappa is reported.
- **Sentence segmentation** of Markdown tables and lists is heuristic; pinned by script hash and checked by step 9.
- **Source-code census** relies on string and enum patterns; an emission site built dynamically may be missed (each
  missed-code finding in EXP-CONF-004/-009 is fed back as a census defect).
- **Snapshot drift**: documents are pinned by the snapshot manifest.

## 16. Cost estimates

| Item | Estimate |
|---|---|
| Compute | 0.5 CPU-hours |
| Disk | 0.5 GB |
| Agent effort | **scripted** extraction and censuses; **judgment, medium**: two labelling sessions (100-200 sentences each), one review session for registry and cross-reference findings |
| Platform | Windows host or Linux (both used for the determinism check); no timing; no network |

## 17. Outcome obligations

`outcome.json` (§10.1). Unmapped statements become cases in `ebcc-v1.1` or specification findings; registry
collisions and undocumented codes are recorded as specification defects and `counterevidence` for the decisions in §3.
