# Independent exact-source review: G-A item 11

Date: 2026-10-01.

## Acceptance boundary

Reviewer: `policy_review/2026-10-01`, independent of the original mechanical classifier and candidate-set design. The v2 migration was produced by `ledger_audit/2026-10-01` under source coordination by `root/2026-10-01`. This is acceptance of the exact public integration artifacts at the scope stated below. It supplies an independent review for a later committed receipt; it does not itself change any gate to PASS. The source commit containing these final artifacts and this note must exist before a receipt can be accepted, and the verifier must freshly rerun its checks on the receipt-bearing HEAD. This note does not contain its own digest or the unknown future source commit.

The governing method commit is `0674ba925c3283c2d18566c15035980c657e842f`. This is a methodology commit, distinct from the future design-freeze commit described in method section 5.5. The governing method SHA-256 is `880e319c8d16e778f856cdbc738ecda088865c4d5e2499665fe058228a29426b`. Its public section 14 timing is unchanged.

G-B, G-C and G-D execution acceptance remain OPEN. Full R0-R10 simulation remains required before the first validation look and at G-D for DEC-ECO-078. Calibration, workload-mix evidence, corpus custody and overlap, external expert/human evidence, outcome records, cost accounting, candidate HC conformance and capability-specific preconditions retain their declared timing. The full simulation and protected corpus artifacts were not reviewed here. Original round-2 checklist items 6 and 9 are not represented as passed. Internal agent review is not external expert review.

The 76 decisions with 80 missing-instrument dispositions still cannot admit decision-bearing preregistration, validation looks, results or DECIDED until their exact canonical instrument, role and band are independently registered. Reviewed no-primary dispositions do not permit graded selection or utility weight. Current experiment-to-assignment and first-result admission remain fail-closed; initial G-A artifact acceptance cannot promote those later events.

## Item 11: objective screen and Appendix C

The corrected embedded-invariant pattern and each-invariant-once-per-decision counting remain in the governing source. Fresh `objective_screen.py` output on the actual migrated ledger matches the generated Appendix C block exactly after the verifier's newline handling. Independent comparison against the governing objective proves the entire document changed only by replacing the generated input-ledger SHA-256; the manual objective and all remaining generated counts/text are unchanged.

Appendix C remains a descriptive heuristic screen. The signed crosswalk and exact independently reviewed assignment, including reviewed zero-HC explanations, determine actual initial applicability. This acceptance is not permission to omit an applicable HC, and does not discharge the METHOD rows' later execution evidence merely because they appear in the generated screen.

## Required current artifact identities

All digests are SHA-256 over exact file bytes. The receipt must bind every required path below and the current method.

| Required artifact | SHA-256 |
|---|---|
| `research/archetypal-objective.md` | `4fda4ca4b1b946a442d4dd72efae9944bde7f617d2a3b4cb5640fa895f308e53` |
| `research/decision-ledger.jsonl` | `ea6a7e4f5810e399cfd10ed86d3393ae054248e5cafaf770f245ee0cb6fae727` |
| `research/decision-method.md` | `880e319c8d16e778f856cdbc738ecda088865c4d5e2499665fe058228a29426b` |
| `research/requirement-ledger.csv` | `14ea66685680d18a58bfda5689be5698013f3f1b03b5df343c2badaf138d3d82` |
| `research/tools/ledger/objective_screen.py` | `3e035d700aee7abe769072ecf037c05f422c81f839df9b33b98a6610f845d566` |
| `research/tools/ledger/tests/test_objective_screen.py` | `5350e3258d7dc8ce530580b74b686ec01fb21736a6ee890713ab549617569cda` |

## Executed independent verification

The reviewer invoked the actual `gate_admission.live_checks` commands for this item on the migrated source, rather than inheriting a producer receipt. Verdict digests use the verifier's stdout/stderr concatenation and duration normalization. All checks below actually passed.

| Live check | Normalized verdict SHA-256 | Observed verdict |
|---|---|---|
| `objective_generated` | `3a469699802f0afdd017fba66032e25646b75e3d1dbb5fdd7ed735cdbadfcfb6` | Appendix C matches generator |
| `objective_test` | `b9f3084ee11b9993db4a3a82b9a377e73e6ce6c053f64c2ea9d721dc821f99dd` | 4 unittest cases passed |

Independent all-row comparison additionally verified every original field and history prefix, exact ten-field assignment equality/digest and reviewer identity, all applicable HC split states, method-source ancestry and unchanged prospective artifacts. The regenerated objective differs only in its v2 input-ledger digest. These are source and initial-state checks, not research experiment results.

## Governing source and preserved provenance

| Identity | SHA-256 |
|---|---|
| Original v1 ledger at governing commit | `7ceb7c8c4d934c3cb5747b1dc6965657b09551cbbd3ceadf7e2f1644c3d77d90` |
| Current migrated v2 ledger | `ea6a7e4f5810e399cfd10ed86d3393ae054248e5cafaf770f245ee0cb6fae727` |
| Original objective at governing commit | `f81627d0171931aec960baa1ce0e1999f8d81802d207480803c575c692b30036` |
| Current regenerated objective | `4fda4ca4b1b946a442d4dd72efae9944bde7f617d2a3b4cb5640fa895f308e53` |
| Preserved unsigned prospective proposal | `479d877e5247e00be3f03ab7e8a29945c9969c899efafb9306d0076cfe2e7446` |
| Preserved prospective independent review register | `a9a29e492c3f3d4b7feda0989b9602b1b147c0216fe6cae1824f6a1f9d493472` |
| Preserved round-2 review history | `8479c127de4492fe642580c08ca516c8e6ddd19adcaab4ad461d9784bf44ee10` |
| Current gate admission source | `06defeb2db50c0970e817c2d2b21a47846d688facd60566a5447116ea42b965b` |

No protected corpus, manifests, locks, fingerprints, raw first-round review, research progress log or held-out payload was read for this acceptance. Original product SPEC and execution gates retain authority. The current gate register remains OPEN pending real committed receipts; this note performs no register update, staging, commit or publication.
