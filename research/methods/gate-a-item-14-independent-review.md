# Independent exact-source review: G-A item 14

Date: 2026-10-01.

## Acceptance boundary

Reviewer: `policy_review/2026-10-01`, independent of the original mechanical classifier and candidate-set design. The v2 migration was produced by `ledger_audit/2026-10-01` under source coordination by `root/2026-10-01`. This is acceptance of the exact public integration artifacts at the scope stated below. It supplies an independent review for a later committed receipt; it does not itself change any gate to PASS. The source commit containing these final artifacts and this note must exist before a receipt can be accepted, and the verifier must freshly rerun its checks on the receipt-bearing HEAD. This note does not contain its own digest or the unknown future source commit.

The governing method commit is `0674ba925c3283c2d18566c15035980c657e842f`. This is a methodology commit, distinct from the future design-freeze commit described in method section 5.5. The governing method SHA-256 is `880e319c8d16e778f856cdbc738ecda088865c4d5e2499665fe058228a29426b`. Its public section 14 timing is unchanged.

G-B, G-C and G-D execution acceptance remain OPEN. Full R0-R10 simulation remains required before the first validation look and at G-D for DEC-ECO-078. Calibration, workload-mix evidence, corpus custody and overlap, external expert/human evidence, outcome records, cost accounting, candidate HC conformance and capability-specific preconditions retain their declared timing. The full simulation and protected corpus artifacts were not reviewed here. Original round-2 checklist items 6 and 9 are not represented as passed. Internal agent review is not external expert review.

The 76 decisions with 80 missing-instrument dispositions still cannot admit decision-bearing preregistration, validation looks, results or DECIDED until their exact canonical instrument, role and band are independently registered. Reviewed no-primary dispositions do not permit graded selection or utility weight. Current experiment-to-assignment and first-result admission remain fail-closed; initial G-A artifact acceptance cannot promote those later events.

## Item 14: actual validation-look registry

The actual registry is the same empty file committed in the governing source. Fresh validation reports zero registered looks, zero executed looks, zero confirmatory or tuning-only registrations, and zero registered-unexecuted looks. This is an accepted initial empty registry and its tested append/audit machinery; a plan row, synthetic fixture, named artifact or this review is never actual validation execution.

The focused tests cover actual registration/execution distinction, planned owner and candidate restrictions, exact primary-map equality including empty maps, unresolved instrument blockers, related-look charging, new-group confirmation and committed Git/spec/preregistration binding. No actual validation group, result or protected held-out identity was inspected or admitted. First-validation simulation/cost and subsequent G-D requirements remain open.

## Required current artifact identities

All digests are SHA-256 over exact file bytes. The receipt must bind every required path below and the current method.

| Required artifact | SHA-256 |
|---|---|
| `research/decision-ledger.jsonl` | `ea6a7e4f5810e399cfd10ed86d3393ae054248e5cafaf770f245ee0cb6fae727` |
| `research/decision-method.md` | `880e319c8d16e778f856cdbc738ecda088865c4d5e2499665fe058228a29426b` |
| `research/decisions/validation-looks.jsonl` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `research/experiments/_program/look-plan.csv` | `b57c06fb848f0dd6f7e1ea3790e4ec18a04cc66db4ec1da18e0c31bbb535708b` |
| `research/tools/experiments/validation_looks.py` | `ebdd8c46030788808f4fcd332c73fedb3df922e5844f6806579d10caff26a29e` |
| `research/tools/tests/test_validation_looks.py` | `3ee3f2df5c05511826b8ca40ad3d46adaf7666557d823cbb5f6602149840aff3` |

## Executed independent verification

The reviewer invoked the actual `gate_admission.live_checks` commands for this item on the migrated source, rather than inheriting a producer receipt. Verdict digests use the verifier's stdout/stderr concatenation and duration normalization. All checks below actually passed.

| Live check | Normalized verdict SHA-256 | Observed verdict |
|---|---|---|
| `look_registry` | `942ce8d0dd86e9ee5b3a2d740f5b9e3c7686960412df7637f054513df71cf178` | Zero registered and executed looks |
| `look_registry_test` | `4643374efcfd1da6cc31ae854600f20d84609a9de53b469f5aa556a91ace3e68` | 7 pytest cases passed; cache disabled |

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
