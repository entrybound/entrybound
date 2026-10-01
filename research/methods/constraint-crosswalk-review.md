# B01/K01 classification and assignment proposal record

Date: 2026-10-01. Branch at preparation: `codex/research-method-gates`, baseline HEAD `6a6aaa3c1b823b5f659d715552b9a92218b405d2`. The constraint classifications have independent row sign-off in `constraint-crosswalk-independent-review.md`; the 596 assignment records remain **unapproved proposals**. Neither artifact alone is a G-A pass or permission to record a decision-grade experiment. Live v2 assignments, the strict validator, committed exact-source gate receipts and applicable later gates remain required.

## Source and output identity

All digests are SHA-256 over file bytes at generation. `string_sha256` is SHA-256 over each exact UTF-8 constraint string; `source_row_sha256` is SHA-256 over the exact ledger JSON line excluding its newline. The generator preserves exact source text rather than normalized text.

| File | SHA-256 |
|---|---|
| `research/decision-ledger.jsonl` | `7ceb7c8c4d934c3cb5747b1dc6965657b09551cbbd3ceadf7e2f1644c3d77d90` |
| `research/requirement-ledger.csv` | `14ea66685680d18a58bfda5689be5698013f3f1b03b5df343c2badaf138d3d82` |
| `research/decision-method.md` | `880e319c8d16e778f856cdbc738ecda088865c4d5e2499665fe058228a29426b` |
| `research/archetypal-objective.md` | `f81627d0171931aec960baa1ce0e1999f8d81802d207480803c575c692b30036` |
| `research/tools/ledger/objective_screen.py` | `3e035d700aee7abe769072ecf037c05f422c81f839df9b33b98a6610f845d566` |
| `D:/Projects/entrybound/design/2026-08-29-entrybound-product-architecture.md` (authoritative original SPEC) | `1f881c7b13f193b41353b90066d33ca6abcb7d4c3dcd4f58e22834801f0fe29c` |
| `research/methods/constraint-crosswalk.csv` | `13ecfe0811e964432f2ceaba2d13bb9f36ca5b5935dfe08bb7e719c2cfbb9224` |
| `research/methods/decision-assignments-proposal.jsonl` | `479d877e5247e00be3f03ab7e8a29945c9969c899efafb9306d0076cfe2e7446` |

The original SPEC was read outside the linked checkout at the path and exact hash above. Terse section and line labels were crosschecked against its text, particularly §§11, 17, 20, 22, 24–26. The original is intentionally not copied into the checkout. The independent classifier review repeated the relevant original-source comparisons, as recorded in its separate note.

## Classification and assignment procedure

`python research/tools/ledger/constraint_crosswalk.py --write` generates one row for every distinct non-bare `I1`–`I31` value in `hard_constraints`, sorted by exact string. It found 989 rows: 445 `F-xx`, 245 `HC-xx`, 21 `NON_GOAL`, 108 `PROGRAM`, and 170 `DECISION_INPUT`. A `NON_GOAL` points to F-25; `PROGRAM` and `DECISION_INPUT` have no R1 target. Existing Appendix B entries are used for `F-xx`; explicit invariant labels are linked to the primary and supporting HCs of objective Appendix A. Seventy source-specific semantic corrections from the independent review were applied before broad source and keyword rules; each exact string carries its own rationale. All 989 rows are signed by `policy_review/2026-10-01` with the independent review note reference. Regeneration retains a reviewer's reference only when the exact classification, target, rationale, classifier, source text and digest are unchanged.

`python research/tools/ledger/decision_assignment_proposal.py --write` generated 596 JSONL proposals. Each points to its exact ledger row and the source file digests; `hc_ids` includes the mapped bare invariants and crosswalk HC targets, `freeze_ids_from_crosswalk` captures F screens separately, and `od_ids` records a question/requirement-screen rationale with cluster fallback only where neither directly selects an OD. There is no arbitrary four-OD cap. For example, DEC-ACC-002 retains OD-11 alongside four other capability dimensions, while DEC-ACC-013 does not inherit an unrelated random-access OD from its cluster. The type proposal follows §1.2's forced human/external routes and treats measured claims as empirical; its default FORMAL inference is a review item, not a grant to skip measurement. Canonical metric names are transcribed from method Appendix A.2; no threshold was changed. At most one proposed primary metric is named per OD. `metric_registration_blockers_by_od` defaults to an empty map in this mechanical proposal; the independent review must name any actual missing instrument, role or band rather than insert a proxy metric. Metric selection, applicability families, experiment record, and final type remain to be accepted by an independent reviewer and, for L7-overlapping families, an eligible analysis designer.

All 596 assignment records have `assignment_status: PROPOSED_UNAPPROVED` and empty `assignment_review_ref`. Proposed type counts are 176 EMPIRICAL, 358 FORMAL, 21 EXTERNAL and 41 HUMAN-FACING. Ninety-seven rows have no mapped HC ID from their current labels; their freeze screens and decision-input labels still need row-specific review and a signed applicability rationale. Four hundred forty-one rows have no proposed banded primary metric. Every OD in a route without EMPIRICAL has a no-primary proposal; a human or external evidence route does not acquire a product timer or byte metric by lexical match. DEC-ACC-063 proposes a no-primary OD-12 disposition because its remote-emulation validity is governed by §4.15 calibration and rank-stability criteria. The four narrow method-admission questions DEC-ECO-077/080/081/082 propose empty product OD sets with individual `od_applicability_rationale` values under §1.2; their HC/F, evidence-route, cost, status and execution gates remain. DEC-ECO-068/070 retain actual product ODs with reviewed no-primary explanations sought for their governed claims. DEC-ACC-013 proposes export acceptance for its export/publish operation but leaves OD-09 without a primary because Appendix A.2 has no stream-export primary; both choices remain subject to review. No decision status was changed.

## Reviewer focus before G-A

1. The independent crosswalk reviewer accepted all 989 exact classification/target/rationale rows, including F-30 for the original SPEC §20.7 bytecode/VM rejection. This grants no HC test PASS, selection, metric or release approval. Any changed row must be re-reviewed.
2. The independent assignment reviewer must inspect all 596 HC/OD/type and metric-disposition proposals against each decision question, required evidence, linked requirements and the canonical metric roles. In particular, no-primary and empty-OD rationales must be accepted for their exact scope; neither is permission for a late graded primary metric.
3. A separate session not involved in candidate design must assign/review `decision_type` (§1.2). The live v2 ledger, exact reviewed assignment digest, gate validator, method SHA and independent review refs must be integrated before G-A can pass. The method-admission exception does not satisfy the later simulation, calibration, G-D or result gates.

## Role and exposure disclosure

The classifier/assignment session did not design the ledger candidates and made no candidate changes. It previously saw held-out identifiers in an L7-type exposure while doing baseline source routing; no held-out payload, path under the protected root, statistic or result was opened in this task. Under `decision-method.md` §5.4(3) and §5.7, this session must not design candidates or analyses for decisions whose applicable families include the exposed items. The metric names here are **unapproved mechanical proposals** from existing canonical method text, not an experiment analysis plan or a held-out result. Applicability families are not yet established per decision, so eligible independent analysis design remains a separate gate. No held-out identities are repeated in this record.

## Verification

From the worktree root:

```text
python research/tools/ledger/constraint_crosswalk.py --check
  crosswalk exact: 989 rows
python research/tools/ledger/decision_assignment_proposal.py --check
  decision assignment proposal exact: 596 rows
python -m unittest research.tools.ledger.tests.test_constraint_crosswalk -q
  Ran 8 tests; OK
```

The test checks exact source coverage and digests, deterministic regeneration, reviewer invalidation on classification changes, representative semantic routes including HC-18 and a SPEC non-goal, invalid screening targets, and the 596 exact row links. It does not substitute for independent semantic review.

## Controlled LF source rebinding

On 2026-10-01, `policy_review/2026-10-01` normalized only CRLF to LF in `research/decision-method.md` and `research/archetypal-objective.md`, after verifying exact decoded-text equality. The canonical proposal was regenerated with only those two file-byte source hashes changed. Every reviewed assignment field, canonical assignment digest, status and per-record reviewer identity remained unchanged; only the raw proposal-row source pins were rebound. This is source-byte rebinding of the accepted prospective review, not a new candidate, experiment, outcome, gate PASS or live ledger migration.

The preceding method/objective byte identities were `fe27bcb0c078d700463916c12ef5f07b9c7cd5870ae2c25894e02fe2590dbb7c` / `ab00418cdfb40cb3ca283324d30d01e10a3dac4f6cb733b11d104dc0d567664a`; the preceding proposal identity was `4099090f862a03490ff19fac279bce217b72badcc4e028b108a03552f1efe7bd`. These are superseded byte identities, not current-source receipts. Their retention here does not assert that historical bytes are durably archived in the public repository.
