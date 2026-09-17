# EXP-CON-003: Merkle outboard layouts, tree construction and verified range proofs at scale

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: CT-1d `ebr-outboard`, `research/tools/container/merkle_ref.py` (second implementation), CT-1a models, CT-2, CT-3, CT-4 |
| Domain / program section | container / §11 (with §12, §24) |
| decision_ids | DEC-CON-020, DEC-INT-029, DEC-ACC-024, DEC-MOD-034, DEC-MOD-007, DEC-INT-024, DEC-INT-025 |
| Decision type (proposal) | EMPIRICAL for bytes, requests, digest operations, memory and time; FORMAL for construction soundness (DEC-INT-024, DEC-MOD-007 soundness part) with the committed counterexample-search protocol of section 13 and reviewer sign-off; EXTERNAL_REVIEW_REQUIRED for the security-proof part of DEC-INT-024 (§8.3) |
| OD / HC (proposal) | OD-05, OD-08, OD-11, OD-12, OD-21, OD-23, OD-24; HC-10, HC-12, HC-14 (signed-root binding), HC-15, HC-16 |
| Specs | `spec.yaml` (tuning real), `spec-synthetic.yaml` (per-object Chunk lists up to 1e7 leaves), `spec-timing.yaml`; `validation-items.txt` |
| Division of labour | Container domain owns outboard bytes (OD-05 primary), local verified-range bytes and digest operations, verifier memory, layouts, the construction counterexample search, attack-suite refusal and independent reproduction. Remote requests, cacheability under coalescing and remote latency per structure are `EXP-REMOTE-007`'s, which consumes the proof layouts exported here; `http_requests__*` keys here are structure-level cross-checks |
| Method, author, L7, gates | `../_index/container-conventions.md` §1-§2 |

## 1. Question

For verified byte-range reads against the published identities (chunk_root per ContentObject, PCR, the manifest tree), which verification structure (status-quo flat digests with whole leaf-list recomputation, two-stage leaf list, balanced binary outboard, per-object outboards above a threshold, Bao slices, segment-aligned proofs, sizes-sister DAG, hash chain, k-ary trees, content-defined hashsplit tree, tree-native fixed leaves), which outboard layout (level-order, Bao pre-order, RFC 9162 shape, paged subtree blocks, fs-verity bottom-up levels, per-object), which node hash construction (status-quo named-domain SHA-256 structured hash, RFC 9162 byte prefixes, C2SP sequencehash) and which count/length binding give the fewest fetched bytes, requests and digest operations per verified range at 1e3 to 1e7 Chunks per object, with small cacheable proofs, sound refusal of malformed proofs and byte-exact independent reproduction; and what does exporting a proof cost in each external format?

## 2. Hypotheses

- **H1 (structure, DEC-INT-029, DEC-ACC-024).** For the 1 B and 4 KiB range strata in objects of ≥ 1e4 Chunks, at least one outboard structure (s2, s3 or s8) is `DISTINGUISHABLE_BETTER` than `s0-flat-leaflist-sq` on `verified_range_fetch_bytes` (p95) and `verification_digest_ops` (p95) beyond the minimum gain of its tier (a new section type: at least T3), with `side_data_bytes` `NON_INFERIOR` in the sense that `artifact_bytes` stays within the T-01 band. Predicted: s0 needs the whole ordered leaf list (32 B per Chunk plus metadata) and O(n) hashing, s2 needs O(log n) nodes.
- **H2 (small objects).** For objects below 64 Chunks no structure beats s0 by 1 band unit on any primary metric (the leaf list is below the T-11 16 KiB floor).
- **H3 (layout, DEC-CON-020).** Among s2 layouts, paged subtree blocks (4 KiB) minimize `http_requests` for ranges under coalescing and give the fewest distinct range signatures (cacheability), while level-order arrays minimize `side_data_bytes`; Bao pre-order is `EQUIVALENT` to level-order on bytes.
- **H4 (construction, DEC-MOD-034, DEC-INT-024).** Byte-prefix (h1) and named-domain (h0) constructions produce `EQUIVALENT` proof bytes (differences below T-11 floors) and identical proof shapes; k-ary trees reduce requests but increase proof bytes per level; the counterexample search finds no second preimage structure for h0 or h1 within its budget.
- **H5 (binding, DEC-MOD-007).** Binding counts and length per object (b1 or b2) adds at most 40 B per ContentObject and makes every truncated/extended/count-lie proof in the attack suite refused (`adversarial_true_refusal_fraction = 1`); b0 (bare root via PCR list) refuses them only after fetching the PCR descriptor list.
- **H0 / falsification.** No structure is `DISTINGUISHABLE_BETTER` than s0 beyond its minimum gain in any stratum ≥ 1e4 Chunks; or the attack suite finds an accepted malformed proof for every non-status-quo structure; or the second implementation disagrees on any root.

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CON-020 | Proof bytes, digest ops, outboard fraction, range-signature counts, rebuild digest ops, independent reproduction for every layout | Remote requests and latency: EXP-REMOTE-007; real CDN/object-store cacheability: EXP-REMOTE-013 (BLOCKED) |
| DEC-INT-029 | Structure comparison at 1e3-1e7 Chunks measured and 1e8-1e9 computed exactly (section 7); disagreement corpus (flat vs tree) outcome of the prototype reader | Production reader outcomes on flat-vs-tree disagreement: only after a tree exists; integrity-domain corruption campaigns |
| DEC-ACC-024 | Granularity comparison per range stratum (local bytes, digest operations, memory) | Remote requests, cacheability and latency: EXP-REMOTE-007; use-case demand for sub-file ranges: analytical (integrity/remote domains); security review of proof binding: external |
| DEC-MOD-034 | h0/h1/h2 root agreement and proof bytes; k-ary and hashsplit shapes; independent reproduction n = 0..4,096 | Hash throughput of tree-native digests: crypto domain (DEC-MOD-016) |
| DEC-MOD-007 | b0-b4 wire bytes per ContentObject and attack-suite refusal | Formal soundness sketch reviewed by an independent session; crypto cross-review |
| DEC-INT-024 | Counterexample search (FORMAL route) and independent recomputation vectors incl. odd and truncated leaf lists | External cryptographic review of the construction (dossier) |
| DEC-INT-025 | Exported proof size per format | Demand from transparency workflows and standards status: analytical (EXTERNALLY_SOURCED) |

## 4. Candidates

Byte-level definitions in `representations.md` before the first run (CT-1d). All trees are built over the same leaf digests production records (chunk_id plus logical length leaves, status-quo leaf domain), so candidates differ only in shape, layout, node hash and binding.

| candidate_id | Ledger candidates covered | Definition |
|---|---|---|
| `s0-flat-leaflist-sq` | status-quo-whole-object; no-outboard-status-quo; status-quo-flat-plus-leaf-lists; external-rfc6962-status-quo (tree over leaves recomputed from the full list) | Fetch the ordered Chunk list of the ContentObject (manifest record) and every Chunk digest needed; recompute chunk_root; bytes released only after whole-object verification |
| `s1-two-stage-leaflist` | per-chunk-digest-with-leaf-list; two-stage-index-then-chunk | Verify the leaf list once against chunk_root (O(n) hashing, cached per session), then verify only covering Chunks by digest |
| `s2-outboard-level-order` | merkle-outboard-inclusion; level-order-array; merkle-outboard-balanced | Status-quo tree shape, all interior levels as contiguous arrays in a MERKLE_OUTBOARD section |
| `s2-outboard-bao-preorder` | bao-preorder-outboard | Same tree, Bao pre-order node layout |
| `s2-outboard-rfc9162-shape` | bab-rfc9162-shape | RFC 9162 shape with length proofs |
| `s2-outboard-paged-4k`, `-64k` | paged-subtree-blocks | Subtrees packed into fixed-size pages aligned for range caching |
| `s2-outboard-fsverity-4k` | fs-verity-bottom-up-levels | Bottom-up levels, fixed 4 KiB blocks (fs-verity style; note fs-verity hashes fixed data blocks, here leaves are CDC Chunks) |
| `s3-per-object-outboard-t64`, `-t1024`, `-t16384` | per-content-object-outboard; per-object-outboard-large | Outboard only for ContentObjects with at least the named Chunk count; s0 below |
| `s4-bao-slice` | bao-slice; bao-slices | Bao slice encoding of covering Chunks plus parents |
| `s5-segment-aligned-proofs` | segmented-proofs | Proof units aligned to fixed 64 MiB plaintext segments (crypto segment analogue) |
| `s6-sizes-sister-dag` | sizes-sister-list-dag; sister-list-dag | UnixFS-style DAG blocks with child sizes (offset resolution without the full list) |
| `s7-hash-chain` | hash-chain | AEA-style chain requiring a linear walk (negative control) |
| `s8-kary-4`, `-16`, `-64` | k-ary-tree | k-ary Merkle tree, largest-power split generalized, no padding |
| `s9-hashsplit-tree` | content-defined-hashsplit-tree | bup-style interior boundaries from extra rolling-hash bits of leaf digests |
| `s10-tree-native-leaves-shape` | tree-native-hash-leaves; blake3-internal-tree; blake3-bao (shape only) | Fixed 1,024-byte leaves inside Chunks with CDC Chunks referenced by offset; proof shape and bytes only; hash-function throughput is DEC-MOD-016's (crypto domain) |

Per-sample analysis dimensions (reported as `desc_*` keys, not separate runs): node hash constructions `h0_named_domain` (status quo), `h1_rfc9162_prefix` (rfc9162-exact; rfc6962-byte-prefix), `h2_c2sp_sequencehash`; count bindings `b0_bare_root_pcr` (status quo), `b1_object_descriptor` (object-descriptor-root; bind-count-in-chunk-root-descriptor), `b2_length_in_root` (length-in-root-node), `b3_manifest_counts` (manifest-carried-counts), `b4_outboard_header` (merkle-outboard-with-signed-count); export formats `cose_receipt` (RFC 9942 inclusion proof over RFC 9162 tree), `custom`, `bao_slice`, `scitt_statement`, plus `no-export` (zero bytes). `document-only` (DEC-INT-024) and `merkle-over-tree-hash-chunk-ids` (DEC-MOD-034) are covered by h0 plus the search record and by s10 respectively.

Exclusions: `combined-interleaved-encoding` (DEC-CON-020): excluded before execution, SPEC §8.6 L975-977 (interleaving rewrites payload and breaks signature survival across outboard rebuild). `flat-digest-list` without a tree (DEC-MOD-034): excluded before execution by the ledger (cannot give O(log n) verified range reads against a signed root, SPEC §8.6 L977); it remains measured implicitly as the leaf-list part of s0/s1, which keep the root. Reviewer sign-off per R1(b) by the completeness critic.

Expected tiers: s1 T1 (reader policy); s2-s6, s8, s9 T4 (new structure bound to identity roots) or T3 if the assessor classes an outboard as a section type without identity change; h1/h2 and b1/b2 T4 (identity/digest-root construction change); export formats T1-T2 (external artifacts, no archive wire change).

## 5. Corpus selectors

- Tuning real: every tuning item (112) packed REF-PROD; every ContentObject is a verification target, and the manifest Entry tree is a target for entry-inclusion proofs. Range strata {1 B, 4 KiB, 1 MiB, whole entry}, 1,000 seeded ranges per stratum per item (fewer if the item has fewer distinct positions; all positions then). Items with large entries (48 tuning items have ≥ 8 MiB mean entry size) dominate the ≥ 1e3-Chunk strata.
- Tuning synthetic: per-object Chunk lists n ∈ {1, 2, 3, 63, 64, 65, 1e3, 1e4, 1e5, 1e6, 4e6, 1e7} built from the shared EXP-CON-001 models' fitted Chunk-size distributions (the spec iterates models; `ebr-outboard --chunk-list-model fitted` derives the object lists).
- 1e8 and 1e9 leaves: not simulated. Node counts, proof node counts per range and outboard bytes are exact integer functions of n and range position for every tree shape; `ebr-outboard formula --n 1e8,1e9` computes them with arbitrary-precision integers and the formulas are checked against measured values at every simulated n (any mismatch invalidates the formula, not the measurement) [FORMALLY_DERIVED].
- Validation (CT-4): `validation-items.txt`. Held-out: none (section 15).

## 6. Environment and platform requirements

As EXP-CON-001 section 6. `merkle_ref.py` (Python 3.12 stdlib `hashlib` only, written by a session other than the `ebr-outboard` author) provides the second implementation. Linux x86-64; Windows timing replicate for universal scope; no privileged operations.

## 7. Commands

Pipeline as EXP-CON-001 section 7 with EXP-CON-003 specs, plus:

```sh
# formulas beyond simulation
/root/eb-research/target/harness/release/ebr-outboard formula --structures all --n 100000000,1000000000 --out research/normalized/EXP-CON-003/decision/formula-1e8-1e9.json
# counterexample search (FORMAL route; section 13)
/root/eb-research/target/harness/release/ebr-outboard search --constructions h0,h1,h2 --bindings b0,b1,b2,b3,b4 --max-leaves 12 --alphabet 4 --budget-cpu-hours 20 --seed 13031 --out research/experiments/EXP-CON-003/adversarial-search/search-results.jsonl
# independent reproduction vectors
python research/tools/container/merkle_ref.py vectors --constructions h0,h1,h2 --n 0-4096 --seed 13032 > research/experiments/EXP-CON-003/adversarial-search/ref-vectors.jsonl
/root/eb-research/target/harness/release/ebr-outboard vectors --constructions h0,h1,h2 --n 0-4096 --seed 13032 > research/experiments/EXP-CON-003/adversarial-search/impl-vectors.jsonl
cmp research/experiments/EXP-CON-003/adversarial-search/ref-vectors.jsonl research/experiments/EXP-CON-003/adversarial-search/impl-vectors.jsonl
```

`ebr-outboard` contract (one JSON line): builds the candidate structure over the archive's (or model's) leaves; checks that the root under h0/b0 equals production chunk_root for every ContentObject (gate G1); replays the range workloads over a logging byte source (covering Chunk frames plus the structure's proof bytes; coalescing per the connection model); counts digest computations; runs the attack suite (truncate, extend, reorder, leaf-as-node, node-as-leaf, count-lie, length-lie) against the candidate verifier and records acceptances; invokes `merkle_ref.py` on a seeded 1% of objects and compares roots and proofs (`reader_disagreements`); and exports per-range proof layouts (extents fetched, with purpose labels) to `research/normalized/EXP-CON-003/layouts/` for EXP-REMOTE-007.

## 8. Seeds, warmups, repetitions

Seeds: 13001-13003, 13011-13013, 13021-13023; search seed 13031; vectors seed 13032. Deterministic: 2 repetitions with repeat-hash. Timing: warmups 2; §4.6 rounds; 1,000 verified ranges per round (T-08 batching).

## 9. Measurement method and metrics

| Metric | OD | Role | Family | T-ID |
|---|---|---|---|---|
| `verified_range_fetch_bytes__<stratum>__p95` | OD-11 | **primary** (per stratum; strata never pooled) | size | T-11 |
| `http_requests__<stratum>__p95` | OD-12 | secondary cross-check (OD-12 primary in EXP-REMOTE-007) | other | T-12 |
| `verification_digest_ops__<stratum>__p95` | OD-11 | diagnostic; primary only if the OD-11 byte metric ties (second OD-11 primary needs a reviewed justification, R3) | other | T-12 |
| `side_data_bytes` | OD-05 | **primary** (decision about the outboard component) | size | T-02 |
| `artifact_bytes` | OD-05 | guard | size | T-01 |
| `alloc_peak_bytes__verify_range` | OD-08 | **primary** | memory_peak | T-06 |
| `verify_overhead_pp__4k`, `random_entry_latency_s__verified_range_4k` | OD-11, OD-10 | timing arm, secondary until filter | other / time_wall | T-13 / T-08 |
| `adversarial_true_refusal_fraction` | OD-15 | binary screen | correctness | HC-14/HC-15 |
| `reader_disagreements` | OD-21 | binary screen (independent reproduction) | correctness | HC-03/HC-12 |
| p50, max, `desc_range_signature_count__*`, `desc_rebuild_digest_ops`, `desc_export_proof_bytes__*`, `desc_count_binding_bytes__*`, `desc_hash_construction_root_agreement__*` | various | descriptive | other | none |

Screens: gate G1 (root equality with production for h0/b0); attack-suite refusal = 1 (R1); independent reproduction (R1); R2 boundability of verifier memory (proof size bound formula checked on F20 items).

## 10. Normalization and statistical analysis

As EXP-CON-001 section 10. Strata: range size (4, never pooled) × Chunk-count stratum of the target object (< 64, 64-1e3, 1e3-1e4, ≥ 1e4 per object). The primary comparison family for DEC-INT-029/DEC-ACC-024 is structure (s-candidates with their best layout from tuning); DEC-CON-020 compares layouts within s2; DEC-MOD-034 compares h0/h1/h2 and k (descriptive keys evaluated as derived candidates: the per-construction proof bytes enter the same analysis as candidate variants of the tuning winner W). Validation look via CT-4 on W and S only.

## 11. Practical-significance thresholds

`research/methods/thresholds.json` entries T-01, T-02, T-06, T-08, T-11, T-12, T-13; minimum gains per computed tier (§3.1, §7.3; §7.3 row "new record or section type" gives the T3 example for `http_requests`). Not restated.

## 12. Sensitivity analysis

Conventions §8 defaults plus: (a) chunk-size preset arm (dense and fast packs of 20 seeded tuning items: more or fewer leaves per object); (b) coalescing gap arm (0, 16 KiB, 256 KiB); (c) range position distribution arm (uniform vs file-head-biased vs Zipf over offsets); (d) session cache arm (cold vs warm leaf-list/outboard cache, which favours s1); (e) signed-root arm: proofs verified against PCR (requires the PCR descriptor list) vs against a per-object root assumed already authenticated.

## 13. Adversarial search and expected negative results

- `adversarial-search/` holds: the **counterexample-search protocol** (FORMAL route, §1.2): generator enumerating all leaf lists of length ≤ 12 over a 4-symbol leaf-digest alphabet and all proof shapes, plus 1e7 seeded random structures at larger n; budget 20 CPU-hours; target properties: (P1) no two distinct (leaf list, length, count) tuples share a root; (P2) no interior node verifies as a leaf and vice versa; (P3) no truncated or extended list verifies against a bound root; (P4) no field decomposition ambiguity in structured hashes. Results (`search-results.jsonl`) and the independent reproduction vectors (n = 0-4,096, odd and truncated lists) are committed with the procedure; the sign-off is by a session not involved in constructing the candidates. Absence of counterexamples is evidence within the budget, not proof (objective §2.0 rule 2), and the security-proof claim goes to external review.
- Negative results worth recording: outboards whose gain vanishes under coalescing or for whole-entry reads; k-ary trees losing on bytes; hashsplit trees giving unstable proof sizes; the status quo winning for typical object sizes (median Chunks per object is small in most families), which would favour a per-object threshold (s3) or no outboard.

## 14. Threats to validity

Prototype structures only (no production outboard exists); gate G1 anchors the status-quo root. Proof bytes are exact given the definitions, but a production implementation might add framing; definitions are committed first. Remote request counts depend on the connection model and coalescing (sensitivity b; slow-start arms applied in EXP-REMOTE-007). s10 changes the digest function and cannot be judged on throughput here. Construction soundness is not provable by search (external review). L7 exposure (conventions §1).

## 15. Phase D held-out placeholder

As EXP-CON-001 section 15.

## 16. Cost and effort

Compute: real pass about 16 h (20 candidates × 112 items × 2 repetitions × 4,000 ranges plus proofs, over cached archives), synthetic about 14 h (1e7-leaf trees hashed in memory, about 2e7 SHA-256 per build), counterexample search 20 CPU-hours (parallel, not timing), timing about 16 h WSL plus 16 h Windows, validation about 25 h: about 110 machine hours. Disk: 1e7-leaf outboards up to about 1 GB each transient; peak about 30 GB beyond the shared pack cache. Agent effort: judgment for `representations.md`, the formal sign-off review (separate session), and analysis records; scripted otherwise.
