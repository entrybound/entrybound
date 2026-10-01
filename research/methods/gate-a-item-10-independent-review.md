# Independent exact-source review: G-A item 10

Date: 2026-10-01.

## Acceptance boundary

Reviewer: `policy_review/2026-10-01`, independent of the original mechanical classifier and candidate-set design. The v2 migration was produced by `ledger_audit/2026-10-01` under source coordination by `root/2026-10-01`. This is acceptance of the exact public integration artifacts at the scope stated below. It supplies an independent review for a later committed receipt; it does not itself change any gate to PASS. The source commit containing these final artifacts and this note must exist before a receipt can be accepted, and the verifier must freshly rerun its checks on the receipt-bearing HEAD. This note does not contain its own digest or the unknown future source commit.

The governing method commit is `0674ba925c3283c2d18566c15035980c657e842f`. This is a methodology commit, distinct from the future design-freeze commit described in method section 5.5. The governing method SHA-256 is `880e319c8d16e778f856cdbc738ecda088865c4d5e2499665fe058228a29426b`. Its public section 14 timing is unchanged.

G-B, G-C and G-D execution acceptance remain OPEN. Full R0-R10 simulation remains required before the first validation look and at G-D for DEC-ECO-078. Calibration, workload-mix evidence, corpus custody and overlap, external expert/human evidence, outcome records, cost accounting, candidate HC conformance and capability-specific preconditions retain their declared timing. The full simulation and protected corpus artifacts were not reviewed here. Original round-2 checklist items 6 and 9 are not represented as passed. Internal agent review is not external expert review.

The 76 decisions with 80 missing-instrument dispositions still cannot admit decision-bearing preregistration, validation looks, results or DECIDED until their exact canonical instrument, role and band are independently registered. Reviewed no-primary dispositions do not permit graded selection or utility weight. Current experiment-to-assignment and first-result admission remain fail-closed; initial G-A artifact acceptance cannot promote those later events.

## Item 10: constraint crosswalk

All 989 exact classification, target, rationale and reviewer values are byte-identical to the signed crosswalk at the governing commit. The original per-string semantic review and authoritative SPEC comparison remain documented in `research/methods/constraint-crosswalk-independent-review.md`; this note refreshes the item 10 binding to the migrated ledger and regenerated objective. Every live distinct non-bare invariant constraint is covered; bare I1-I31 uses the separate governed Appendix A mapping. Every F target is in the actual Appendix B register. Classification sign-offs all name `policy_review/2026-10-01` and remain independent of the classifier.

The objective's only change since the governing commit is the generated Appendix C input-ledger digest. No objective requirement, HC, OD, freeze, metric, band or classification changed. The focused suite retains exact unsigned proposal pins against their governing v1 source and separately verifies original-field/history-prefix equality in live v2. It does not compare a historical v1 proposal pin to a new v2 serialized row or regenerate the proposal. Passing these checks grants no HC conformance, candidate selection, measured utility or release acceptance.

## Required current artifact identities

All digests are SHA-256 over exact file bytes. The receipt must bind every required path below and the current method.

| Required artifact | SHA-256 |
|---|---|
| `research/archetypal-objective.md` | `4fda4ca4b1b946a442d4dd72efae9944bde7f617d2a3b4cb5640fa895f308e53` |
| `research/decision-method.md` | `880e319c8d16e778f856cdbc738ecda088865c4d5e2499665fe058228a29426b` |
| `research/methods/constraint-crosswalk.csv` | `13ecfe0811e964432f2ceaba2d13bb9f36ca5b5935dfe08bb7e719c2cfbb9224` |
| `research/tools/ledger/constraint_crosswalk.py` | `8dfeba4a973c1d37b69a07f05a6dc0370ff2689d071598e338261a56018085a6` |
| `research/tools/ledger/tests/test_constraint_crosswalk.py` | `26c7129b65e9251eb833f9f68683ab48d5a695e6d044aca4c29541d4f403d0fe` |

## Executed independent verification

The reviewer invoked the actual `gate_admission.live_checks` commands for this item on the migrated source, rather than inheriting a producer receipt. Verdict digests use the verifier's stdout/stderr concatenation and duration normalization. All checks below actually passed.

| Live check | Normalized verdict SHA-256 | Observed verdict |
|---|---|---|
| `crosswalk_exact` | `298adc63d9cbf331849a2681ac806d1989fc9d4df641daced5110b27fb5266cb` | crosswalk exact: 989 rows |
| `crosswalk_signed` | `e48e32e3776481d4cb6813c0544b6dbe3ea183b06c725afe321bcda4bc218604` | 989 signed rows and registered freeze targets |
| `crosswalk_test` | `d3de929d3d84f45c5601ff0ca787d962d4b7e8b4d56506562982ad4d9feb65f5` | 8 unittest cases passed |

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
