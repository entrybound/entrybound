# EXP-CON-002: Index data structure and validation granularity at 1e3 to 1e7 Chunks

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: CT-1c `ebr-index-repr`, CT-1a models, CT-2, CT-3, CT-4 (conventions §4) |
| Domain / program section | container / §11 (with §12, §13) |
| decision_ids | DEC-CON-015, DEC-ACC-019, DEC-CMP-002 (Index and chunk-table overhead component only), DEC-ACC-018 (Index bytes per Chunk, whole-vs-paged local fetch); input to DEC-CON-002 |
| Division of labour | Container domain owns Index bytes, local read logs, resident memory, rebuild cost and correctness; remote requests and latency per Index structure are EXP-REMOTE-004's (it consumes the layouts exported here); hostile-Index behaviour of production readers is EXP-CONF-004 (F-INDEX cases), EXP-INT-001 and EXP-REMOTE-008 |
| Decision type (proposal) | EMPIRICAL (bytes, requests, memory, latency); FORMAL for the non-authority property of each validation strategy (I10) |
| OD / HC (proposal) | OD-05, OD-08, OD-10, OD-12, OD-14 (rebuild), OD-21, OD-24; HC-01, HC-03, HC-06, HC-09, HC-15, HC-16 |
| Specs | `spec.yaml` (tuning real, 4 profiles), `spec-synthetic.yaml` (shared EXP-CON-001 models), `spec-timing.yaml`; `validation-items.txt` |
| Method, author, L7, gates | `../_index/container-conventions.md` §1-§2 |

## 1. Question

Which Index structure (status-quo digest-sorted 100-byte records, fanout plus sorted digest table, paged two-level index, dual-coordinate seek table, sizes-only sister list, delta-compressed locators, per-ContentObject locator runs, or no Index) and which validation granularity (lazy, always cross-check, digest-bound, per-record frame check) resolve entries to Chunk-frame ranges with the fewest bytes, requests, resident memory and CPU, locally and remotely, cold and warm, from 1e3 to 1e7 Chunks, while the Index remains a small, rebuildable, non-authoritative cache; and how does Index and chunk-table overhead vary with the four profiles' chunk-size policies?

## 2. Hypotheses

- **H1.** At ≥ 1e5 Chunks, `i2-paged-16k` or `i1-fanout-oid-table` is `DISTINGUISHABLE_BETTER` than `i0-sorted-records-sq` on `open_bytes_read__resolve_entry_cold_mean` and on `alloc_peak_bytes__index_resident` beyond the minimum gain of its tier, and `NON_INFERIOR` on `artifact_bytes`. Predicted: more than 5 band units on T-11 at 1e6 Chunks, because the status quo fetches the whole section (about 100 B per Chunk; smoke sizing) to resolve one entry.
- **H2.** Small-archive equivalence: below about 160 Chunks (a status-quo Index of at most 16 KiB, the T-11 floor) no candidate beats the status quo by 1 band unit on `open_bytes_read__resolve_entry_cold_mean`, and below about 1e4 Chunks (a status-quo Index of about 1 MiB, the T-06 allocator floor) none beats it on `alloc_peak_bytes__index_resident` [both thresholds `FORMALLY_DERIVED` from the 100-byte record width; checked empirically].
- **H3 (DEC-ACC-019).** `v1-sq-always-cross-check` costs a full CHUNK_DATA header scan (`open_bytes_read__first_verified_read_mean` proportional to Chunk count) and is `DISTINGUISHABLE_WORSE` than lazy validation at ≥ 1e4 Chunks; `v2-sq-digest-bound-index` is `NON_INFERIOR` to lazy validation on bytes and requests.
- **H4 (DEC-CMP-002).** Index bytes scale with Chunk count, so `extreme` produces at least 8× the `index_bytes` of `fast` on real items (ratio of presets, `FORMALLY_DERIVED` bound checked empirically), and no tuning item approaches the 4,000,000 Chunk limit under `fast` or `balanced`.
- **H0 / falsification.** Every alternative is `EQUIVALENT` or `INCONCLUSIVE` with demonstrated power on every primary metric in every Chunk-count stratum ≥ 1e5, or fails a correctness screen.

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CON-015 | Index bytes per Chunk; local bytes to resolve an entry cold and warm; resident memory; rebuild bytes; page cacheability under `WL-ZIPF`; prototype decoder LoC (C2 input); exported layouts | Remote requests and latency: EXP-REMOTE-004; hostile-Index outcomes of production readers: EXP-CONF-004 and EXP-REMOTE-008; encrypted Index (record type 27 frozen) interplay: EXP-CON-005 encrypted cell and EXP-SCALE-010; fuzz robustness: conformance domain |
| DEC-ACC-019 | Cost of each validation granularity on first verified read; non-authority check per strategy | Hostile-Index suite on production openers: EXP-CONF-004, EXP-REMOTE-008; Index header damage: EXP-INT-001 |
| DEC-ACC-018 | Index bytes per Chunk and local bytes and memory of whole versus paged Index fetch | Discovery strategy, requests and latency: EXP-REMOTE-004 |
| DEC-CMP-002 | `index_bytes` and Chunk counts per profile preset and headroom to the 4,000,000 limit | Size, dedup and CPU sweeps: chunking domain (EXP-CHUNK-001, EXP-CHUNK-004, EXP-CHUNK-008) |
| DEC-CON-002 | Index-structure input to the revision synthesis | EXP-CON-008 |

## 4. Candidates

Byte-level definitions go into `representations.md` before the first run (CT-1c). Every candidate resolves locators for exactly the frames that production's rebuilt locator map produces (screen S1).

| candidate_id | Ledger candidate | Definition |
|---|---|---|
| `i0-sorted-records-sq` | sorted-record-list-status-quo | Production INDEX section: digest-sorted type-6 records (digest, absolute offset, stored length), fetched whole; lazy validation as `open_indexed_random` (status quo). Research re-encoder must match production bytes (gate G1) |
| `i1-fanout-oid-table` | fanout-plus-sorted-oid-table | Git idx v2 layout: 256 × u32 fanout, sorted 32 B digests, u32 offsets with u64 large-offset table, u32 stored lengths with overflow table |
| `i2-paged-4k`, `-16k`, `-64k` | paged-two-level-index | Fixed-size pages of i0 records; top-level directory of (first digest, page offset) |
| `i3-dual-coordinate-seek-table` | dual-coordinate-seek-table | Seekable-zstd-style table of (stored size, logical size) per frame in physical order; entry resolution through ContentObject Chunk ordinals |
| `i4-sizes-sister-list` | sizes-sister-list | Stored length per frame in physical order; offsets by prefix sums (running-sum checkpoints every 4,096 frames) |
| `i5-delta-compressed-locators` | delta-compressed-locators | Blocks of 256 i0 records: base offset plus zigzag-varint deltas; block directory |
| `i6-per-object-locator-runs` | per-content-object-locator-runs | Per ContentObject (digest-sorted): first frame offset and the stored lengths of its frames |
| `i7-no-index-scan-only` | no-index-scan-only; remove-index-v1 | No Index; locators rebuilt from frame headers (64 or 96 B each) |
| `v1-sq-always-cross-check` | always-cross-check | i0 bytes; reader rebuilds the locator map from frame headers and compares before use |
| `v2-sq-digest-bound-index` | digest-bound-index | i0 bytes; INDEX section digest bound in an authenticated root (footer or Descriptor field, new wire item); locators trusted after one digest check plus per-frame digest check at read |
| `v3-sq-per-record-frame-check` | per-record-validation | i0 bytes; each used locator validated only against its frame header (digest, stored length) |

Exclusion: `authoritative-index` (both DEC-CON-015 and DEC-ACC-019): excluded before execution, I10 (the Index may never be an authority; ledger exclusion, R1(b) sign-off by the completeness critic).

Expected tiers: i1-i6 T3 (new section type or record encoding) or T4 if the assessor classes them as container structure; v2 T2-T3 (new field with decode semantics); v1 and v3 T0-T1 (reader behaviour within existing wire); i7 T1 (writer stops emitting an optional section; decode support retained, §7.4).

## 5. Corpus selectors

- Tuning real (`spec.yaml`): every tuning item (112), each packed by `ebound-prod` INDEXED under `fast`, `balanced`, `dense`, `extreme` (pack cache; primary analysis on `balanced` = REF-PROD; the other presets feed `index_bytes__<profile>` and Chunk-count keys for DEC-CMP-002).
- Tuning synthetic (`spec-synthetic.yaml`): the 42 tuning models of `../EXP-CON-001/synthetic-items.json`; Chunk tables generated from each model's fitted chunks-per-file distribution under the balanced chunker; strata reach 1e7 Chunks (above the 4,000,000 production limit, labelled `beyond-limit`, prototype-only).
- Timing: synthetic models ≤ 1e6 entries.
- Validation (CT-4): `validation-items.txt`.
- Held-out: none (section 15).

## 6. Environment and platform requirements

As EXP-CON-001 section 6 (WSL primary; Windows timing replicate and 10% hash cross-check). Linux x86-64 required; no privileged operations; storage section 16.

## 7. Commands

Pipeline identical to EXP-CON-001 section 7 with `EXP-CON-002` and its specs. `ebr-index-repr` contract (one JSON line per sample):

1. Load the balanced archive's rebuilt locator map through production `open_indexed_random` over a memory source (status-quo truth) and the other three presets' Chunk counts and `index_bytes` from the pack cache JSON.
2. Encode the candidate Index; decode it; compare every locator with the rebuilt map (`reader_disagreements`, screen S1).
3. Replay read plans over a logging byte source using `research/experiments/EXP-REMOTE-002/workloads.json`: `resolve_entry_cold` (fresh session, 1,000 seeded regular-file entries), `resolve_entry_warm` (same session, next 1,000), `first_verified_read` (resolve plus fetch and verify the first Chunk of the entry, with the candidate's validation strategy), `rebuild` (Index absent or invalid), and `WL-ZIPF` page-cache hit fraction with a 64 KiB range cache.
4. CT-2 resident memory of the loaded Index structure.
5. `desc_decoder_loc`: non-blank, non-comment lines of the candidate's decoder module counted by a pinned `tokei` (C2 input; the §14 item 7 script supersedes it when available).
6. Export the exact Index layout (extents with purpose labels) to `research/normalized/EXP-CON-002/layouts/<item_id>/<candidate>.json` for EXP-REMOTE-004.

## 8. Seeds, warmups, repetitions

Seeds: 12001-12003 (`spec.yaml`), 12011-12013 (`spec-synthetic.yaml`), 12021-12023 (`spec-timing.yaml`). Deterministic: 2 repetitions with repeat-hash. Timing: warmups 2, §4.6 rounds, batch 1,000 resolutions per round.

## 9. Measurement method and metrics

| Metric | OD | Role | Family | T-ID |
|---|---|---|---|---|
| `open_bytes_read__resolve_entry_cold_mean` | OD-10 | **primary** | size | T-11 |
| `http_requests__resolve_entry_cold_mean` | OD-12 | secondary cross-check (OD-12 primary in EXP-REMOTE-004) | other | T-12 |
| `alloc_peak_bytes__index_resident` | OD-08 | **primary** | memory_peak | T-06 |
| `index_bytes` | OD-05 | **primary** (decision about the Index component; `diagnostic` role justified) | size | T-02 |
| `artifact_bytes` | OD-05 | guard | size | T-01 |
| `open_bytes_read__first_verified_read_mean`, `http_requests__first_verified_read_mean` | OD-11 | primary for DEC-ACC-019 comparisons (v-candidates) | size / other | T-11 / T-12 |
| `open_bytes_read__rebuild` | OD-14 | secondary (rebuild cost) | size | T-11 |
| `http_requests__resolve_entry_warm_mean` | OD-12 | guard | other | T-12 |
| `random_entry_latency_s__resolve`, `open_latency_s__index_load` | OD-10, OD-07 | timing arm, secondary until the filter | time_wall | T-08 |
| `index_bytes__<profile>`, `desc_chunk_count__<profile>`, `desc_chunk_count_limit_fraction__<profile>` | OD-05 | descriptive for DEC-CMP-002 | size / other | T-02 / none |
| `reader_disagreements` | OD-04 | binary screen S1 | correctness | HC-03 |
| `desc_index_bytes_per_chunk`, `desc_page_cache_hit_fraction__wl_zipf`, `desc_decoder_loc` | OD-05, OD-12, OD-21 | descriptive / cost input | other | none |

Screens: S1 locator agreement (R1); S2 non-authority (FORMAL, R1(b)): for each validation strategy a written argument that a hostile Index can at worst cause a named refusal or a rebuild, never a different EAM, checked by the F-INDEX hostile cases of EXP-CONF-004 on production readers and by the same mutations replayed against the prototype strategies here (`--hostile-suite` in `ebr-index-repr`); S3 R2 boundability of every decoder allocation against CT-2.

## 10. Normalization and statistical analysis

As EXP-CON-001 section 10 (units, strata, tuning, confirmatory, R6/R8/R9). Chunk-count strata for real items: < 1e3, 1e3-1e5, ≥ 1e5 Chunks (balanced). DEC-ACC-019 candidates are compared among `i0`, `v1`, `v2`, `v3` only (same Index bytes, different reader behaviour); DEC-CON-015 compares `i0`-`i7`. DEC-CMP-002: descriptive per-profile tables and the limit fraction (AGG-W, maximum over items), handed to the chunking domain's profile-scope analysis (constrained form, Appendix B.1).

## 11. Practical-significance thresholds

`research/methods/thresholds.json` entries for T-01, T-02, T-06, T-08, T-11, T-12; minimum gains per computed tier (§3.1, §7.3). Not restated.

## 12. Sensitivity analysis

Conventions §8 defaults plus: (a) range-cache size arm (0, 64 KiB, 1 MiB) for the warm metrics; (b) Chunk-size preset arm (the primary comparison repeated on `dense` packs, which have more Chunks per entry); (c) workload arm `WL-RAND1` vs `WL-ZIPF` vs `WL-SUBTREE`; (d) coalescing gap arm (0 and 16 KiB) because paged structures benefit from coalescing (harness review use constraint 7: a coalescing-sensitive conclusion also needs the slow-start arms of `EXP-REMOTE-001/connection-model.json` as applied in EXP-REMOTE-004).

## 13. Adversarial search and expected negative results

- `adversarial-search/`: worst-case Chunk tables for each structure (all frames in one ContentObject; offsets straddling the u32 boundary for i1; highly irregular stored lengths for i4 and i5; digests clustered in one fanout bucket for i1 and i2 page skew), and the hostile-Index replay of screen S2.
- Negative results worth recording: paged structures that lose to the status quo below 1e5 Chunks; `v2` needing a wire change for a gain below the T2-T3 minimum; `i7` (no Index) being `EQUIVALENT` locally for small archives (which would favour dropping the Index from small archives by policy).

## 14. Threats to validity

Prototype-vs-production (gate G1 and timing among prototypes only); synthetic Chunk tables ignore physical dedup order effects (real items dominate L4); remote conclusions depend on the connection model (slow-start arms applied in EXP-REMOTE-004); encrypted archives use frozen record type 27 and segment scanning, which none of these candidates changes (a new encrypted Index would be a crypto-v1 successor, T4, out of scope here); L7 exposure (conventions §1).

## 15. Phase D held-out placeholder

As EXP-CON-001 section 15.

## 16. Cost and effort

Compute: real pass about 14 h (4 presets packed once each, shared cache; 13 candidates × 112 items × 2 repetitions over cached archives), synthetic about 10 h, timing about 16 h WSL plus 16 h Windows, validation about 20 h: about 75 machine hours. Disk: pack cache for 4 presets about 150 GB total over tuning and validation (shared with EXP-CON-001, EXP-CON-003, EXP-CON-005, EXP-CON-007); raw under 1 GB. Agent effort: judgment for `representations.md`, the S2 formal arguments (reviewed by a separate session), and analysis records.
