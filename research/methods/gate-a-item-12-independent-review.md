# Independent exact-source review: G-A item 12

Date: 2026-10-01.

## Acceptance boundary

Reviewer: `policy_review/2026-10-01`, independent of the original mechanical classifier and candidate-set design. The v2 migration was produced by `ledger_audit/2026-10-01` under source coordination by `root/2026-10-01`. This is acceptance of the exact public integration artifacts at the scope stated below. It supplies an independent review for a later committed receipt; it does not itself change any gate to PASS. The source commit containing these final artifacts and this note must exist before a receipt can be accepted, and the verifier must freshly rerun its checks on the receipt-bearing HEAD. This note does not contain its own digest or the unknown future source commit.

The governing method commit is `0674ba925c3283c2d18566c15035980c657e842f`. This is a methodology commit, distinct from the future design-freeze commit described in method section 5.5. The governing method SHA-256 is `880e319c8d16e778f856cdbc738ecda088865c4d5e2499665fe058228a29426b`. Its public section 14 timing is unchanged.

G-B, G-C and G-D execution acceptance remain OPEN. Full R0-R10 simulation remains required before the first validation look and at G-D for DEC-ECO-078. Calibration, workload-mix evidence, corpus custody and overlap, external expert/human evidence, outcome records, cost accounting, candidate HC conformance and capability-specific preconditions retain their declared timing. The full simulation and protected corpus artifacts were not reviewed here. Original round-2 checklist items 6 and 9 are not represented as passed. Internal agent review is not external expert review.

The 76 decisions with 80 missing-instrument dispositions still cannot admit decision-bearing preregistration, validation looks, results or DECIDED until their exact canonical instrument, role and band are independently registered. Reviewed no-primary dispositions do not permit graded selection or utility weight. Current experiment-to-assignment and first-result admission remain fail-closed; initial G-A artifact acceptance cannot promote those later events.

## Item 12: actual v2 ledger and assignments

All 596 original v1 rows were compared independently against their bytes in the governing commit. Every original field value is exactly unchanged, except that `history` retains its complete original list as an exact prefix and has explicit integration entries appended. Candidate sets, exclusions, questions, required evidence, result/evidence bodies, blockers, scopes, costs and status are preserved. The schema-v2 additions exactly match the checked-in generated schema.

All ten governed assignment fields equal the accepted prospective review payload, including exact array order and rationale bytes, and every canonical digest matches. Actual per-record reviewer identity is retained: 391 `policy_review/2026-10-01` and 205 `qualification_audit/2026-10-01`. Producer/reviewer separation and committed, SHA-pinned public review bytes are required and verified by the live validator. The original 52 empty-HC dispositions and only four narrow empty-product-OD method-admission exceptions remain governed by their exact reviewed rationale.

Statuses remain 580 INSUFFICIENT_EVIDENCE and 16 EXTERNAL_REVIEW_REQUIRED, with zero DECIDED. All 11,772 applicable HC split states are UNVERIFIED with empty evidence; zero are PASS. Cost tier/assessment, actual look/freeze/gate-audit references and new outcome evidence remain unassigned. Every newly appended R0-R10 outcome explicitly states UNVERIFIED; migration did not evaluate these rules.

The three additional public-section-14 history notes were read individually. DEC-CMP-035 binds the constrained profile form (SPEC section 5.4 / method section 2.0) and cited fitted mix with equal-arm sensitivity (SPEC section 25.3 / method section 4.9), leaving bounds and fitted evidence open. DEC-ECO-076 records one future program-wide design freeze/unlock and fresh confirmation for unfrozen decisions, explicitly distinguishing that future freeze from this governing methodology commit. DEC-ECO-078 limits Appendix D to named synthetic arithmetic checks and retains full rule simulation, false-selection and power evidence before first validation and G-D. None claims a candidate outcome or DECIDED.

The preserved prospective proposal and review register remain byte-identical to the governing commit. Their v1 source hashes are historical source bindings, not claims that the current v2 ledger has those bytes. Re-generating either to erase its governing-source provenance was not part of migration. A Gate-A validator PASS here means exact prospective integration is valid; it does not mean G-D, HC conformance, costs, simulation or result admission passed.

## Required current artifact identities

All digests are SHA-256 over exact file bytes. The receipt must bind every required path below and the current method.

| Required artifact | SHA-256 |
|---|---|
| `research/decision-ledger.jsonl` | `ea6a7e4f5810e399cfd10ed86d3393ae054248e5cafaf770f245ee0cb6fae727` |
| `research/decision-method.md` | `880e319c8d16e778f856cdbc738ecda088865c4d5e2499665fe058228a29426b` |
| `research/methods/constraint-crosswalk.csv` | `13ecfe0811e964432f2ceaba2d13bb9f36ca5b5935dfe08bb7e719c2cfbb9224` |
| `research/methods/thresholds.json` | `9cd4b5be3486ae671c54987df2c5ee4f8650e77f464145fe48fe829fe93ddc90` |
| `research/tools/experiments/validation_looks.py` | `ebdd8c46030788808f4fcd332c73fedb3df922e5844f6806579d10caff26a29e` |
| `research/tools/ledger/assemble.py` | `ddae63c0e3ecb8df3f8a7a469b0ce2c8caa4877906e494b2aaabcf97fe39e3f4` |
| `research/tools/ledger/decision-ledger.schema.json` | `bce3c615e1d1affd9b8c35c5fa95e9dee6769a91769cc0eacf55d752e1acec0a` |
| `research/tools/ledger/validate_decisions.py` | `d4ea286b93c2e795f3a205d1c09ee7d05c2b731936f0f898319de8687bb2f48e` |
| `research/tools/tests/test_decision_method_schema.py` | `919cd6e46e06f6febde9e3324ca7fbce3e0add0a6ca7f65ff6020aa7810cc8e2` |

## Executed independent verification

The reviewer invoked the actual `gate_admission.live_checks` commands for this item on the migrated source, rather than inheriting a producer receipt. Verdict digests use the verifier's stdout/stderr concatenation and duration normalization. All checks below actually passed.

| Live check | Normalized verdict SHA-256 | Observed verdict |
|---|---|---|
| `ledger_gate_a` | `cecce0853992221d727214592025c002cabe154c30fc70195793b4261d55e5a1` | 596 rows, zero DECIDED, zero errors, PASS |
| `ledger_schema_test` | `29b17033248b7ac086649b3c2ec1e4be65a2d4bd6ed45cbfa86395a0f1fa966c` | 24 pytest cases passed; cache disabled |

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
