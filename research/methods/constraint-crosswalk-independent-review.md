# Independent constraint classification review

## Scope and result

Reviewer session: `policy_review/2026-10-01`.

Method integration item: **10** (`decision-method.md` §14).

All **989** distinct non-bare-invariant strings were read individually with their proposed class and target. The reviewer checked the governing HC definitions, Appendix A invariant mapping, all Appendix B freezes, the relevant original SPEC text, and the shared classification rationales. All 70 requested exact-string corrections were re-inspected after regeneration. Remaining unchanged strings retain the classification reviewed in the full pass.

The signed crosswalk SHA-256 is `13ecfe0811e964432f2ceaba2d13bb9f36ca5b5935dfe08bb7e719c2cfbb9224`.

Each CSV row now names `policy_review/2026-10-01; research/methods/constraint-crosswalk-independent-review.md` in `reviewer`. This accepts that row's exact string, string digest, class, target and rationale. It establishes classification review, not a passing HC test or a decided candidate. The subsequent gate receipt must bind the signed file and its committed sources, and rerun the exact regeneration check.

## Sources and independence

The authoritative original SPEC was read from the supplied local source; its verified SHA-256 is `1f881c7b13f193b41353b90066d33ca6abcb7d4c3dcd4f58e22834801f0fe29c`.

The current public `decision-method.md`, `archetypal-objective.md`, decision ledger, crosswalk generator and crosswalk were reviewed. Original SPEC excerpts were checked directly for preamble authority, decoder longevity, codec registries, profile defaults, signatures, library surface and range-addressable indexing where classification was ambiguous. Repository source references within the labels were evaluated against the exact registered freeze scope; this review does not claim a fresh implementation audit of every file cited by those labels.

This session did not design the initial crosswalk or candidates. It has read no research progress log, corpus manifest, corpus lock, fingerprints, held-out content, sample identities, protected held-out path, or raw first-round review. An identity-safe review packet supplied the first-round findings with the sensitive excerpts omitted. No omitted corpus evidence is certified by this review. This is independent internal session review and is not external expert assurance.

## Classification decisions

Only HC and registered freeze obligations enter R1 candidate screening. `NON_GOAL` uses the registered F-25 scope. `PROGRAM` remains a research, custody, evidence or release obligation with its own timing; it is not silently discarded. `DECISION_INPUT` remains context, a recommendation, a current default open to research, a threat-model exclusion or a cross-decision handoff. None of those classes provides an HC exemption.

Concrete corrections included:

- Caller-constructed policy maps to HC-05, even when its source path names platform fidelity.
- Exclusive create and no overwrite map to HC-18, even when their source path names STREAM.
- Verified staging maps to F-11; single-pass source and sink requirements map to F-09.
- Signature transcripts, token bounds, binding freshness and signer IDs map to F-13; hybrid recipient requirements and password transcripts map to F-12; recipient removal and key rotation map to F-14.
- External crypto release audit and research directory custody remain program gates.
- Creation profiles do not become decoder requirements (F-05). Fixed planner and transform rules map to the actual F-03 register; chunker rules map to F-04.
- Legacy export's exclusion of embedded Entrybound signatures belongs to F-22; ACL/mode consistency belongs to F-17.
- Advisory footer hints are non-authoritative claims under HC-01. Compact-manifest incompatibility is HC-04. The baseline decoder size budget is HC-12. Range-addressable indexing without full residency is HC-06.
- A timestamp recommendation is decision context; the supported timestamp wire remains F-13. The recommendation is not elevated into an immutable product requirement.
- In-archive decompressor VM rejection is now an explicit F-30 register entry, retaining the existing product constraint.

Bare I1–I31 strings are intentionally absent from these 989 rows. Their governed mapping is objective Appendix A and `I_TO_HC`; they must not be reported as uncrosswalked by the ledger validator. Embedded invariant references remain covered by the complete strings in this CSV.

## Mechanical verification

Before signing, the independent reviewer ran:

```text
C:/Python313/python.exe research/tools/ledger/constraint_crosswalk.py --check
crosswalk exact: 989 rows
```

The objective-screen, crosswalk and method-admission unittest modules passed together: **18 tests**. The decision-schema and validation-look pytest modules passed together: **22 tests**. The pytest run emitted only a cache-directory permission warning; all test cases passed. These counts describe that observed source state; later source changes need their own exact verification.

Signed-file regeneration and the gate receipt are required after this note is committed. A changed class, target, rationale, classifier, string or digest invalidates its carried reviewer field and requires a new semantic review.

## Exact-source binding for item 10

The final classification and timing changes were reviewed again against the following current sources. The item 10 receipt must bind these exact bytes and this independent review; path existence alone does not satisfy the gate.

| Required source | SHA-256 |
|---|---|
| `research/methods/constraint-crosswalk.csv` | `13ecfe0811e964432f2ceaba2d13bb9f36ca5b5935dfe08bb7e719c2cfbb9224` |
| `research/archetypal-objective.md` | `f81627d0171931aec960baa1ce0e1999f8d81802d207480803c575c692b30036` |
| `research/tools/ledger/constraint_crosswalk.py` | `8dfeba4a973c1d37b69a07f05a6dc0370ff2689d071598e338261a56018085a6` |
| `research/tools/ledger/tests/test_constraint_crosswalk.py` | `b1a7ac5ec30cd3d130ce0a21a079e0059bec8d8c09333e08277630153ffce851` |
| `research/decision-method.md` | `880e319c8d16e778f856cdbc738ecda088865c4d5e2499665fe058228a29426b` |

At this final source state the reviewer reran the three unittest modules (**19 passed**) and the decision-schema/validation-look pytest modules (**27 passed**, cache disabled). These checks exercise synthetic fixtures; no corpus evidence was read or admitted. Later source changes require a refreshed semantic review and exact-source receipt.

The subsequent exact-question assignment repair and instrument-gap amendment were reviewed independently. The current method permits initial assignment acceptance with a digest-bound missing canonical instrument/role/band while blocking the entire affected decision's decision-bearing preregistration, look, result and DECIDED. It preserves every public §14 timing and the four narrowly scoped method-admission OD exceptions. No crosswalk class/target/rationale changed. Against the current source table above, the reviewer reran the three unittest modules (**20 passed**) and the decision-schema/validation-look modules (**31 pytest cases passed**, cache disabled), and exact crosswalk regeneration again returned **989 rows**. These checks are synthetic; all unaccepted execution gates remain OPEN. This paragraph refreshes the current-source review; the earlier counts record historical runs.

## Controlled LF source rebinding

On 2026-10-01, `policy_review/2026-10-01` normalized only CRLF to LF in `research/decision-method.md` and `research/archetypal-objective.md`, after verifying exact decoded-text equality. The canonical proposal was regenerated with only those two file-byte source hashes changed. Every reviewed assignment field, canonical assignment digest, status and per-record reviewer identity remained unchanged; only the raw proposal-row source pins were rebound. This is source-byte rebinding of the accepted prospective review, not a new candidate, experiment, outcome, gate PASS or live ledger migration.

The preceding method/objective byte identities were `fe27bcb0c078d700463916c12ef5f07b9c7cd5870ae2c25894e02fe2590dbb7c` / `ab00418cdfb40cb3ca283324d30d01e10a3dac4f6cb733b11d104dc0d567664a`; the preceding proposal identity was `4099090f862a03490ff19fac279bce217b72badcc4e028b108a03552f1efe7bd`. These are superseded byte identities, not current-source receipts. Their retention here does not assert that historical bytes are durably archived in the public repository.

All 989 signed crosswalk classification, target, rationale and reviewer values remain byte-identical. Item 10 still requires a committed exact-source receipt at its public timing.
