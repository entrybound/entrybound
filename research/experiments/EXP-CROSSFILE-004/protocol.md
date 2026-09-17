# EXP-CROSSFILE-004: Chunk-group lookback and dependency topology sweep (ratio versus access amplification, dependency bytes, declared cost, parallelism bound, corruption propagation)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: `ebr-crossfile topology` research encoder and exact dependency analytics (`research/harness/ebr-crossfile-DESIGN.md` §2-§4) are not built |
| Domain / program sections | crossfile / 8.5 (8.4 for dictionary interaction) |
| decision_ids | DEC-CMP-027 (primary); DEC-ACC-025 (primary); DEC-INT-002 (blast-radius definition); DEC-ACC-001 (cost-formula models, analytic part); DEC-CMP-013 (value of a `--lookback` override); DEC-CON-004 (lookback share of declared requirements) |
| Evidence type | deterministic bytes and exact graph analytics over the recorded dependency DAG (FORMALLY_DERIVED given the archive; §4.6, §4.13); validated against measured reads and real fault injection in EXP-CROSSFILE-006; latency and measured speedup in EXP-CROSSFILE-008; incumbent frontier in EXP-CROSSFILE-005 |
| Method | `research/decision-method.md`, pre-registered |
| Record | Design only; `record.md` (§12.2) instantiated once gate G-A holds. |
| Design session and disclosure | Phase C-design crossfile designer; L7 disclosure as in EXP-CROSSFILE-001. |

## 1. Question

Which lookback values (0, 1, 2, 4, 8, 16+) and reference-prefix window should each profile allow,
and is chained Zstandard prefix lookback (1 MiB prefix, digest-ordered cohorts) sufficient
compared with leader-only delta, group restarts or length caps, larger or long-distance windows,
and bounded-solid blocks, given the ratio gain against first-byte cost (access amplification and
decoded dependency bytes), declared-versus-transitive access cost, parallel-decode loss and
corruption propagation? And, for DEC-ACC-025: is the declared cost over direct or transitive
dependencies, and which of cap, reset or redeclaration keeps declarations honest at least cost?

## 2. Hypotheses

- **H1a (transitivity, DEC-ACC-025).** For production dense/extreme archives with ChunkGroups
  longer than `max_lookback + 1` members, the transitive decode closure of the last member
  exceeds the declared `max_preceding_bytes` (which counts only `max_lookback` direct
  predecessors), so `declared_cost_accuracy` fails (measured/declared > 1) on at least one
  tuning item: an HC-09/HC-06 violation of the status-quo declaration, matching ledger
  REQ-ACC-0001 `CONTRADICTED`.
- **H1b (ratio curve, DEC-CMP-027).** On F01, F03, F05, F06 and F18, the `artifact_bytes` gain of
  chained lookback over lookback 0 is concave in k: k = 1 or 2 captures >= 70% of the k = 8 gain
  in log units, and k = 16/32 adds less than 1 band unit of T-01 over k = 8, while
  `access_amplification` (1 B stratum) grows at least linearly in the closure length.
- **H1c (topology).** `restart-n4` or `cap-group-len8` keeps >= 80% of the chained k = 4 byte gain
  (log units) with worst-case closure bounded by the cap, and dominates chained k = 4 under an
  honest transitive declaration.
- **H1d (blast radius, DEC-INT-002).** Only the transitive-closure and dictionary-inclusive
  definitions achieve `localization_recall` = 1 for chained groups; the direct-prerequisite
  (status quo) and k-following definitions under-report.
- **H0.** Lookback > 0 gives no gain beyond the T-01 band over dictionaries plus ordering
  (`chain-k0` with dictionaries), so `lookback-zero-everywhere` is retained by R10 (lower cost,
  no transitive-declaration problem).
- **Falsification.** H1a: every archive's measured transitive closure bytes <= declared on every
  Chunk. H1b: k = 16 or 32 improves `artifact_bytes` by more than 1 band unit over k = 8 at
  corpus level on tuning, or k in {1, 2} captures < 70% of the k = 8 gain. H1c: both capped
  topologies keep < 80% of the gain. H1d: status-quo definition recall = 1 on every injected
  Chunk.

## 3. Decisions informed; proposed OD and HC sets

| Decision | Proposed type | Proposed od_ids | Proposed hc_ids |
|---|---|---|---|
| DEC-CMP-027 | EMPIRICAL (profile-scope Appendix B.1 form if so classified) | OD-05, OD-07, OD-08, OD-10, OD-12, OD-13, OD-14 | HC-02, HC-06, HC-09, HC-13, HC-15, HC-16 |
| DEC-ACC-025 | FORMAL + EMPIRICAL | OD-05, OD-10, OD-17 | HC-06, HC-09, HC-16 |
| DEC-INT-002 | FORMAL + EMPIRICAL | OD-14 | HC-09, HC-15 |
| DEC-ACC-001 (analytic part) | FORMAL + EMPIRICAL (+ HUMAN-FACING for reporting usability, not measured here) | OD-10, OD-12, OD-17 | HC-06, HC-09, HC-15 |
| DEC-CMP-013 (override value) | EMPIRICAL part only | OD-05 | HC-13 (overrides recorded), HC-09 |
| DEC-CON-004 (lookback part) | FORMAL + EMPIRICAL | OD-08, OD-17 | HC-06, HC-16 |

## 4. Candidates

Cohorts are `sq-bottomk-v1` cohorts of the dense and extreme profiles (balanced uses lookback 0
by contract and is the k = 0 reference). Dictionary candidates keep production behaviour unless
the topology says otherwise. For forced configurations the cohort level is the best of the
profile's lookback levels under the frozen 128-byte rule (production cost), applied to every
cohort; cohorts where no candidate qualifies stay independent (recorded).

**Encoding topologies (DEC-CMP-027, DEC-ACC-025 cap/reset candidates):**

| candidate_id | Definition | Production-encodable | Expected tier |
|---|---|---|---|
| `sq-profile-lookbacks` | Status quo production planner: dense {1,2,4} x levels {5,9}; extreme {1,2,4,8} x {9,15,19}; chained, final 1 MiB prefix | yes (reference) | reference |
| `chain-k0` (= `lookback-zero-everywhere`) | No ChunkGroups; dictionaries and ordering only | yes | T1 |
| `chain-k{1,2,4,8}-w1m` | Forced single lookback per profile | yes | T1 |
| `chain-k{16,32}-w1m` (`lookback-16-plus`) | Forced; new registered lookback values | research | T2 (new registered parameter values) |
| `chain-k{1,2,4,8}-w{256k,2m,4m,8m,chunkmax}` (`larger-prefix-window`) | Prefix window other than 1 MiB; `chunkmax` = the chunker's maximum Chunk length | research (256k/2m/4m/8m) | T2 |
| `ldm-w{23,27}-k{4,8}` (`zstd-ldm-long-window`) | Prefix plus long-distance matching, windowLog 23 or 27 | research | T2 |
| `leader-only-star` | Every member prefix-coded against the cohort leader only (closure <= 1) | research | T2 |
| `restart-n{2,4,8,16}` (`group-restart-interval`, `periodic-independent-members`) | Chained lookback k = min(4, n-1) whose chain restarts with an independent member every n members | research (new group semantics) | T2 |
| `cap-group-len{4,8,16,32}` (`cap-group-length`) | Cohort split into consecutive ChunkGroups of at most L members, chained k = 4 inside | yes when L <= k + 1 makes closure = declared; research otherwise (same records, different grouping) | T1 |
| `anchor-k{2,4}` (`non-transitive-prefix`) | Members at positions multiple of k+1 are independent anchors; each dependent member's prefix is built only from the preceding anchor and dependents after it (closure <= k) | research | T2 |
| `bsolid-b{1m,4m,16m,64m}` (`bounded-solid-blocks`) | One Zstandard frame per contiguous run of cohort members of at most B plaintext bytes (DwarFS/7z-style) at the profile's lookback levels | research (new record type) | T3 |
| `per-category-lookback` | Lookback chosen per extension category from the tuning oracle (only if the planner categorizes; expressibility checked at R9) | research | T1 |
| `critic-TBD` | R0 item 6 slot | | |
| `unbounded-solid` | **Excluded** before execution: L0/R1, I29 and HC-09 (cannot declare a finite lookback), SPEC §2.3 non-goal (ledger `candidate_exclusion_reasons`); independent reviewer sign-off pending | | |

**Declaration models (DEC-ACC-025; evaluated on the same encodings, no re-encode):**
`declare-max-lookback` (status quo `max_lookback` + direct `max_preceding_bytes`),
`declare-transitive-closure` (worst transitive closure Chunks and bytes per group),
`iterative-bounded-decoder` (declarations unchanged; decoder rewritten iteratively; its memory and
latency are measured in EXP-CROSSFILE-006/-008, closure is identical by definition).

**Blast-radius definitions (DEC-INT-002):** `status-quo-direct-prerequisite`, `k-following`,
`transitive-closure`, `whole-group`, `byte-bounded` (declared `max_preceding_bytes`),
`dictionary-inclusive` (transitive closure plus every user of a damaged Dictionary).

**Access-cost models (DEC-ACC-001, analytic part):** `status-quo-chunks-and-lookback`,
`decode-work-model`, `fetch-model`, `dual-local-remote-model`, `measured-calibration`
(constants fitted on tuning only, evaluated on validation).

**Override value (DEC-CMP-013):** per item, the oracle-best forced lookback among the
production-registered values under Appendix B.1 guards, against the profile default.

- **Reference candidate:** `sq-profile-lookbacks` per profile (dense, extreme); `chain-k0` is the
  secondary reference for gain curves.
- **Incumbent comparison (R0 item 4):** 7z LZMA2 solid with bounded solid block, tar.xz with
  blocks, `zstd --long=27`, squashfs and DwarFS from EXP-CROSSFILE-005 on the same items and the
  same access sample; SPEC §23.4 "a few percent versus solid" (objective CC-01 declared-cost
  tolerance) is evaluated there by joining the two experiments' normalized outputs.

## 5. Corpus selectors

- Manifest `ed28b1b4…`, tuning only.
- **Tuning (`spec.yaml`):** the 50-item cross-file target set of EXP-CROSSFILE-001 §5 (covers the
  ledger-required F01, F03, F05, F06, F18). Large-tier tier-guard items for finalists
  (`spec-stage-c.yaml`, generated by rule): `f18-tuning-curl-8-x-release-series`,
  `f05-tuning-loghub-hdfs-v1`, `f01-tuning-llvm-project-20-1-8-git`.
- **Long-group stress (DEC-ACC-025 required evidence "last members of 8/64/128-Chunk groups"):**
  three generated items (`spec.yaml` generated entries built by `ebr-crossfile topology
  --generate long-groups`, seeds 20260917041-43): 8, 64 and 128 near-identical 64 KiB files with
  seeded 1% byte edits, sized so that dense and extreme form one maximal cohort. These are
  tuning-role generated inputs, not corpus items.
- **Validation look:** validation split, all families small and medium, plus the four large items
  of EXP-CROSSFILE-001 §5; W and S only, after the MDE gate.

## 6. Environment and platform requirements

`wsl-ubuntu`, x86-64 Linux, ext4, `timing: false`, no network (remote quantities are modelled from
exact byte layouts, and validated by measured HTTP counts in EXP-CROSSFILE-006), no privilege.
Memory: the tool holds one item's Chunk plaintext and DAG; the largest target item needs about
2x its size in RAM (31 GB available). Prerequisites as EXP-CROSSFILE-001 §6.

## 7. Commands

`ebr validate|plan|run|verify-raw|normalize|report --spec research/experiments/EXP-CROSSFILE-004/spec.yaml`
as EXP-CROSSFILE-001 §7. Per-sample command:

```text
/root/eb-research/target/harness/release/ebr-crossfile topology
  --item-path {item_path} --item-id {item_id} --experiment-id EXP-CROSSFILE-004 --env-name wsl-ubuntu
  --profile {profile} --topology {topology} --synthetic-from-item-id true --declarations all --blast-definitions all
  --cost-models all --access-sample v1 --coalesce-gap default
  --network-profiles NP-LAN,NP-METRO,NP-CONT,NP-INTER --slow-start both
  --archive-out {scratch}/out.eb --memo-dir /root/eb-research/cache/crossfile/EXP-CROSSFILE-004/rep{rep}
  --verify-roundtrip true
```

Exact analytics (no sampling except the shared access sample):
- **Closure.** For every Chunk: direct predecessors per topology; transitive closure (Chunks and
  decoded logical bytes) by DAG traversal; Dictionaries in the closure.
- **Access amplification** per stratum {1 B, 4 KiB, 1 MiB, whole entry} on the shared sample:
  decoded bytes = closure predecessors' full plaintext + the streaming prefix of the requested
  Chunk up to the last requested byte (the same streaming model as `solid_access.py`), divided
  by requested bytes; the whole-Chunk model is also reported.
- **Blast radius, payload and dictionary strata, exact:** single-byte corruption uniform over the
  stratum's stored bytes; for a byte in Chunk c the unreadable set is c plus every Chunk whose
  closure contains c (for a Dictionary byte, every Chunk whose plan references it, plus their
  dependents); entries touching the set are unreadable. The distribution over injected bytes is
  computed exactly (weights = stored bytes), giving p95 and max without sampling. Index,
  metadata, integrity and footer strata come from EXP-CROSSFILE-006 injections.
- **Parallelism bound:** work W = sum of decoded bytes over all Chunks, span S = maximum closure
  work along any dependency chain; `workspan_speedup_bound_n` = min(n, W/S) (Brent bound)
  for n in {2, 4, 8, 16}.
- **Remote, modelled (§4.15):** per random entry read, fetched ranges = metadata-first open
  (production value for encodable archives; the reference archive's value otherwise) + stored
  extents of closure Chunks and Dictionaries, coalesced with the default gap; latency per network
  profile with and without the slow-start term.
- **Definitions and models:** each blast definition's predicted affected set and each cost model's
  prediction per read versus the exact oracle above.

## 8. Seeds, warmups, repetitions

Seeds `order` 20260917044, `bootstrap` 20260917045, `command` 20260917046; access sample seed per
item as `solid_access.py`; generated long-group items 20260917041-43. Warmups 0. Repetitions 2 plus
repeat-hash on `archive_sha256` and `analytics_sha256` (canonical JSON of every analytic output).

## 9. Metrics

| Metric | OD | Role (DEC-CMP-027 unless noted) | ebr family | Threshold | Source |
|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | **primary** | size | T-01 | `payload.artifact_bytes` (+ `artifact_bytes_source`) |
| `access_amplification` (strata 1b, 4k, 1m, entry; never pooled) | OD-10 | **primary** (also DEC-ACC-025) | other | T-22 | `payload.access_amplification_*` |
| `access_granularity_bytes` | (no OD; Appendix B.1 guard) | guard / R2 screen for profile bounds | size | T-09 | `payload.access_granularity_bytes` |
| `first_entry_latency_s` | OD-07 | primary, measured in EXP-CROSSFILE-008 | time_wall | T-08 | EXP-CROSSFILE-008 |
| `alloc_peak_bytes` (decode) | OD-08 | primary, measured in EXP-CROSSFILE-006 | memory_peak | T-06 | EXP-CROSSFILE-006 |
| `remote_random_entry_latency_s` (modelled, per network profile) | OD-12 | **primary** (per network-profile condition) | time_wall | T-10 | `payload.remote_random_entry_latency_s_*` |
| `http_bytes`, `http_requests` (random entry) | OD-12 | secondary | size / other | T-11 / T-12 | `payload.*` |
| `parallel_speedup` | OD-13 | primary, measured in EXP-CROSSFILE-008; the work-span bound here is FORMALLY_DERIVED corroboration | other | T-14 | EXP-CROSSFILE-008; `payload.workspan_speedup_bound_*` |
| `blast_logical_bytes_p95` (strata equal-weight headline; worst-stratum guard) | OD-14 | **primary** | other | T-15 | `payload.blast_*` + EXP-CROSSFILE-006 strata |
| `blast_logical_bytes_max`, `blast_entries_p95` | OD-14 | secondary | other | T-15 | `payload.*` |
| `declared_tightness_ratio` (declared access bytes / transitive closure bytes) | OD-17 | **primary for DEC-ACC-025 and DEC-ACC-001** | other | T-23 | `payload.declared_tightness_ratio_access`, `cost_model_tightness_*` |
| `declared_cost_accuracy` | HC-09 | binary (R1) | correctness | §3.3 | `payload.declared_cost_accuracy`, `cost_model_accuracy_*` |
| `localization_recall` per definition | HC-15 | binary (DEC-INT-002) | correctness | §3.3 | `payload.localization_recall_*` |
| `localization_precision` per definition | OD-14 | **primary for DEC-INT-002** | other | T-20 (case set = injected byte classes of the exact distribution) | `payload.localization_precision_*` |
| `side_data_bytes` | OD-05 | diagnostic | size | T-02 | `payload.side_data_bytes` |
| `closure_chunks_max`, `closure_decoded_bytes_max` (dependency bytes) | - | descriptive (the M10.4 inputs) | other | none | `payload.*` |
| `roundtrip_ok`, `archive_sha256`, `analytics_sha256` | HC-02, HC-13 | binary / repeat-hash | correctness | §3.3, §4.13 | `payload.*` |

At most one primary per OD per decision; `first_entry_latency_s` (OD-07) and
`random_entry_latency_s` (OD-10, secondary here; a second OD-10 primary needs a reviewed
justification, R3) are measured in EXP-CROSSFILE-008.

## 10. Normalization and statistical analysis

As EXP-CROSSFILE-001 §10, with these specifics:
- **R1/R2 first (§13 batching).** Declaration models and topologies whose `declared_cost_accuracy`
  fails on any Chunk of any item (including F20 and the long-group stress items) are eliminated
  for that (candidate, declaration) pair at R1 (HC-09) or R2 (unboundable, §2 R2), before any
  graded comparison. Blast definitions with `localization_recall` < 1 are eliminated (HC-15).
- **Modelled values.** Rows with `artifact_bytes_source = modelled` are decision-grade only after
  validity control 4 (EXP-CROSSFILE-006 exact agreement for production-encodable configurations)
  passes; until then they are informational and labelled so.
- **Profile scope.** DEC-CMP-027 is a per-profile question; under Appendix B.1 superiority is
  `artifact_bytes` only, with `access_granularity_bytes`, `alloc_peak_bytes`, `encode_wall_s`,
  `decode_wall_s` as non-inferiority guards and R2 screens against declared profile bounds once
  DEC-CMP-035 fixes them (until then declared requirements are reported, not screened). The
  OD-form analysis is pre-declared as well; the assigned decision type selects which governs.
- **Evaluation conditions.** Network profile (NP-*) and access stratum are conditions: never
  averaged; R4 and R8 guards apply per condition.
- **Minimum gain.** New registered lookback values, window sizes, restart semantics and anchors
  are T2 (k = 3); bounded-solid blocks are T3 (k = 5, §7.3 new record type row).
- **Override value (DEC-CMP-013).** Distribution over items of the band-unit regret of the profile
  default against the per-item oracle forced lookback; generated table only.
- **Curves.** The SPEC §25.3 curve (ratio gain versus worst-case and mean decoded bytes per random
  read for lookback 0, 1, 2, 4, 8, 16) is produced by `ebr report` from the normalized outputs.

## 11. Practical-significance thresholds

T-01, T-02, T-06, T-08, T-09, T-10, T-11, T-12, T-14, T-15, T-20, T-22, T-23 by reference
(`metric_thresholds.<name>`); §7.2/§7.3 minimum gains.

## 12. Sensitivity analysis

decision-method §6 in full; plus (a) modelled record sizes for research topologies at 0 and 2x
the DESIGN §3 assumption; (b) slow start on/off (R1-14); (c) coalescing gap x0.5 and x2; (d)
blast headline with byte-proportional stratum weights (objective OD-14 sensitivity arm) and
worst-stratum only; (e) access amplification whole-Chunk model versus streaming model; (f)
exclusion of the generated long-group stress items from aggregates (they are screens, not
workload).

## 13. Validation look and held-out placeholder

As EXP-CROSSFILE-001 §13 (`EXP-CROSSFILE-004-HO` at Commit A, single freeze §5.5; every frozen
candidate; held-out MDE <= 2 band units; report-only on failure; identity-aware cap).

## 14. Expected negative results worth recording

- The status-quo declaration under-states transitive decode work (HC-09 defect at baseline; the
  corrective candidate is required by R0 item 7).
- Lookback beyond 2 buys less than a band over dictionaries on most families; extreme's k = 8 is
  not justified.
- Larger prefix windows and LDM help only F18 version series, where exact dedup already captures
  most redundancy.
- Bounded-solid blocks beat chained lookback on ratio at equal granularity (a result against the
  current design) or lose because per-block framing and loss of dedup interaction cost more.
- Corruption blast radius of chained groups exceeds the T-15 band relative to lookback 0 on the
  payload stratum, so dense/extreme defaults trade integrity locality visibly.

## 15. Threats to validity

- Analytic closure and blast radius assume the decoder decodes exactly the recorded DAG; validated
  against production reads and real injections in EXP-CROSSFILE-006 (validity control 4).
- Research topologies use modelled record bytes; sensitivity (a).
- Forced configurations apply one lookback to every cohort, unlike production's per-cohort choice;
  the status quo keeps production's choice, and the oracle arm shows the per-cohort headroom.
- Generated long-group items are adversarial constructions, excluded from workload aggregates.
- Modelled remote latency depends on the §4.15 connection model and on the default coalescing
  policy (open remote-domain decision); sensitivities (b) and (c).
- Corpus and L7 threats as EXP-CROSSFILE-001.

## 16. Cost

About 260 CPU-hours (prefix encodes at levels 9-19 over every topology; memoised per topology and
cohort), wall about 12-15 h sharded; disk: scratch about 4 GB, memo about 8 GB, raw about 3 GB.
Agent effort: none for execution; judgment for the research encoder and analytic definitions;
judgment for analysis.

## 17. Evidence these decisions need that this experiment does not produce

- **DEC-ACC-025:** "planner version impact analysis (new planner version needed?)" is FORMAL
  (HC-16: any change to frozen v3-v6 group semantics or declarations allocates new identifiers);
  recorded in the record's cost section, not measured.
- **DEC-ACC-001:** the pack-time and inspect reporting usability check is HUMAN-FACING (§4.16) and
  routed to the evaluation domain; the formula's soundness and tightness are measured here and in
  EXP-CROSSFILE-006.
- **DEC-CMP-013:** see EXP-CROSSFILE-003 §17.
- **DEC-INT-002:** the formal dependency-closure definition checked against the implementation
  needs an independent reviewer's sign-off (FORMAL route, §1.2); this experiment supplies the
  exact closure computation it is checked against.
