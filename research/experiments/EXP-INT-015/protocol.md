# EXP-INT-015: Merkle and range-verification soundness counterexample search and structure cost model

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T14 `ebr-int-merkle` independent model and counterexample generators; T12 for disagreement archives; prototype outboard structures for model validation: Bao via the `bao` or `abao` crate, paged subtree prototype; `ebr-origin` logs) |
| Domain | integrity (program §24, cross-reference container §11 and remote §12; objective HC-10, HC-12, HC-15, OD-11, OD-12, OD-23) |
| Decisions informed | DEC-INT-024, DEC-MOD-007, DEC-INT-029, DEC-CON-020 |
| Decision type expectation | DEC-INT-024 and the soundness parts of DEC-MOD-007 and DEC-INT-029 are FORMAL (§1.2): a derivation plus the committed counterexample-search protocol of this experiment, checked and signed off by a reviewer who did not produce it; cost parts are EMPIRICAL |
| Gates, method, author | conventions C0 |
| Spec | `spec.yaml` (arm A vectors and counterexample search, arm B cost model and its validation, arm C disagreement corpus) |

## 1. Question

Is the status quo SHA-256 named-domain tree (largest-power-of-two split, no padding, length and
count bound through descriptors) free of known Merkle pitfalls, can a range reader be made to
accept truncated or extended leaf lists or a node posing as a leaf under each chunk-root binding
candidate, and what proof bytes, requests, hash operations and memory does each candidate
range-verification structure need per verified range from 10^3 to 10^9 chunks?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-INT-024 | Within the committed search budget, no second preimage, leaf/node confusion, odd-count duplication (CVE-2012-2459 class) or empty-tree confusion is found for the named-domain construction, and production and the independent model agree on every vector (leaf counts 0 to 65, 2^k - 1, 2^k, 2^k + 1 for k up to 20). The shape equals the RFC 9162 Merkle Tree Hash shape; only domain encodings differ, so an RFC 9162 interop mapping is a re-encoding of leaves and nodes, not a shape change. | Any counterexample (committed artifact) or vector disagreement. |
| H2 | DEC-MOD-007 | With `status-quo-bare-root-via-pcr`, a range reader that verifies against one object's chunk_root without the PCR descriptor list accepts a truncated leaf list whose root was separately authenticated only if the count is unauthenticated; `object-descriptor-root` and `length-in-root-node` reject every truncated or extended list in the search. | A candidate claimed sound accepts a truncated or extended list. |
| H3 | DEC-INT-029, DEC-CON-020 | Flat per-chunk digests with per-object leaf lists need O(n) digest bytes per object for a verified range inside a large object, while balanced outboards need O(log n); at 10^6 chunks in one object the difference exceeds the T-11 band and floor, and at 10^3 chunks it does not. Paged subtree outboards give the most cache-aligned ranges (all fetch ranges fixed-size aligned) at a proof-byte cost within the T-11 band of Bao. | Measured prototype counts disagree with the model at 10^3-10^6 (then the model is not used for extrapolation), or no size crossover exists. |
| H4 | DEC-INT-029 | When flat digests and a tree both exist and disagree, the status quo recomputes the root from the leaf list and reports `CORRUPT` with `EB_INTEGRITY_CHUNK_ROOT_MISMATCH`; a reader that trusts either one alone accepts some disagreement archive. | Status quo accepts a disagreement archive. |

## 3. Candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-INT-024 | `status-quo-named-domain-tree`, `rfc9162-exact`, `bind-count-in-chunk-root-descriptor`, `blake3-internal-tree`, `document-only` | Counterexample search (arm A) against the status quo and `rfc9162-exact` (model); `bind-count-in-chunk-root-descriptor` evaluated with DEC-MOD-007; `blake3-internal-tree` is a digest-root and identity change of tier T4 under frozen identity/v1 SHA-256 (`docs/crypto-suite-v1.md` L20 as cited in the ledger); its soundness is covered by its own published analysis, and the ledger's "rejecting BLAKE3's internal tree documented" is an analytical deliverable (primary sources cited in the FORMAL derivation), not a measurement. Interop needs assessment (COSE Receipts, transparency logs) is a literature arm with primary-source citations. |
| DEC-MOD-007 | `status-quo-bare-root-via-pcr`, `object-descriptor-root`, `length-in-root-node`, `manifest-carried-counts`, `merkle-outboard-with-signed-count` | Model range readers per candidate in T14; attacks: truncated leaf list, extended list (appended leaves), leaf reorder, count field lies, logical length lies, subtree-as-leaf. Proof bytes and requests for remote range reads from arm B; wire cost per ContentObject at 10^3-10^7 entries from the record-size model (MODEL). |
| DEC-INT-029 | `status-quo-flat-plus-leaf-lists`, `merkle-outboard-balanced`, `per-object-outboard-large`, `sister-list-dag`, `two-stage-index-then-chunk`, `blake3-bao`, `hash-chain` | Arm B cost model at 10^3-10^9 chunks and arm C disagreement corpus. |
| DEC-CON-020 | `no-outboard-status-quo`, `level-order-array`, `bao-preorder-outboard`, `bab-rfc9162-shape`, `paged-subtree-blocks`, `per-content-object-outboard`, `fs-verity-bottom-up-levels`, `combined-interleaved-encoding` | Arm B: proof bytes per verified range, outboard size fraction, range requests under the declared fetch model (one request per contiguous needed range; coalescing threshold 64 KiB as a sensitivity parameter), fraction of fixed-size aligned ranges (cacheability proxy), rebuild cost in hash operations. Independent reproduction of roots from arm A's model. Real CDN cacheability is EXP-INT-020 (BLOCKED). |

## 4. Corpus

- Arm A: generated leaf lists (random 32-byte digests, seeds per C8) and every archive packed from
  INT-SMALL-T (production roots compared with the model).
- Arm B model validation at real scale: `f04-tuning-real-netbsd-pkgsrc` (234,505 files),
  `f01-tuning-llvm-project-20-1-8-git` (154,022 files), `f18-tuning-curl-8-x-release-series`
  (115,028 files), `f07-tuning-qwen2-5-1-5b-safetensors-bf16` (one large object, many chunks);
  validation `f04-validation-real-gentoo-repo-20260915`, `f19-validation-deep`,
  `f03-validation-bootstrap-npm-node-modules`, `f07-validation-smollm2-360m-safetensors`.
  Beyond 10^6 chunks the model uses generated leaf lists only (no archive of that size is packed).
- Arm C: disagreement archives generated from INT-SMALL-T with T12.

## 5. Environment and platform requirements

`wsl-ubuntu`, `timing: false`; `ebr-origin` on localhost for exact request and byte logs of the
prototype structures. CPU per verified range is an in-process secondary measurement (T-08-style
batch timing of 1,000 proofs) scheduled in an EXP-INT-004 quiet window only if a selection depends
on it.

## 6. Procedure and commands

1. Commit the counterexample-search protocol file `adversarial-search/protocol.md` (generators,
   budgets, acceptance) before any search run; outputs go to `adversarial-search/` in this
   directory (§8.2 item 5).
2. `ebr run` of `spec.yaml` (candidates are arms), `verify-raw`, `verify_detail.py`, `normalize`,
   `report`.
3. FORMAL derivation document `formal/merkle-soundness.md` written by the designer of the model,
   reviewed and signed off by a session that did not produce it (§1.2).

Search budgets (pre-registered): exhaustive structural cases for leaf counts 0-65; 10^8 random
framings for leaf/node confusion; 10^6 random truncation and extension trials per reader candidate
at list sizes 2^1-2^20; all duplication patterns for n <= 4,096.

## 7. Seeds, warmups, replication

Seeds 2026091851/52/53. Warmups 0; 2 repetitions of vectors and model outputs with repeat-hash;
the random search is split by seed into two disjoint halves (one per repetition) so that
repetitions add coverage; its outcome (counterexamples found) must be identical (zero) or each
counterexample is committed.

## 8. Metrics

| Metric | ID | OD | Role |
|---|---|---|---|
| `reader_disagreements` | HC-03 | OD-04 floor | binary (production versus independent model roots) |
| counterexamples found | none (FORMAL search output) | HC-10 | binary: any counterexample eliminates the construction or reader candidate (R1(b), with artifact and sign-off) |
| `verified_range_fetch_bytes` | T-11 | OD-11 | **primary** for DEC-INT-029 and DEC-CON-020 (per verified range, per range-size stratum 1 B, 4 KiB, 1 MiB, whole object) |
| `http_requests` | T-12 | OD-12 | **primary** for DEC-CON-020 |
| `verification_digest_ops` | T-12 | OD-11 | diagnostic |
| `index_bytes`, `side_data_bytes` | T-02 | OD-05 | diagnostic (outboard size) |
| `metadata_bytes_per_entry` | T-02b | OD-05 | **primary** for DEC-MOD-007 wire cost (MODEL) |
| `access_amplification` | T-22 | OD-10 | secondary |
| `normative_words_added`, `wire_items_added`, `untrusted_input_loc` | C1, C4 | OD-23, OD-24 | cost inputs |

## 9. Normalization

Model outputs are exact counts; per stratum and chunk-count rung, ratios against the status quo
structure (T-11, T-12 floors); rungs and strata are never pooled (T-22 rule).

## 10. Statistical analysis and sensitivity

Deterministic counts; model admitted for extrapolation only if prototype measurements at
10^3-10^6 chunks match model counts exactly (pre-registered acceptance). Selection per R4-R10 on
the exact values per rung, with rung as a stratum; a winner that differs by rung is reported as
scale-dependent (R9 partition by size is expressible only if the format allows per-object choice,
as `per-object-outboard-large` does). Sensitivity: chunk-size arm (64 KiB, 256 KiB, 1 MiB),
coalescing-threshold arm, slow-start arm of the §4.15 latency model when latency is derived
(harness constraint R1-14).

## 11. Expected negative results worth recording

- The status quo needs whole leaf lists for one verified range in a large object, which is
  prohibitive at 10^6+ chunks per object.
- RFC 9162 byte-exact interop would require re-encoding identity digests (frozen), so interop is
  only a mapping.
- Hash chains (`hash-chain`) need linear walks and are eliminated on OD-11 at every large rung.

## 12. Threats to validity

- The independent model is written in the same program; organizational independence is limited
  (HC-12 note); external review of the FORMAL derivation is recommended.
- Random search is not proof; the FORMAL derivation carries the soundness claim and the search is
  its counterexample protocol.
- Cost model assumptions (fetch model, coalescing) are declared parameters; real clients differ.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12; FORMAL parts skip R5 with the reason recorded (§1.2); cost-model validation is
re-run on held-out archives after unlock.

## 14. Cost estimates

- Arm A search: about 20 CPU-hours (10^8 framings at SHA-256 speed plus truncation trials).
- Arm B: packing validation items and prototype measurement about 15 CPU-hours; model evaluation
  to 10^9 chunks analytic, under 1 CPU-hour.
- Arm C: about 1 CPU-hour.
- Disk: about 15 GB.
- Agent effort: model and derivation judgment; independent sign-off judgment; execution none.

## 15. Deviations (append-only)

None.
