# EXP-CROSSFILE-003: Dictionary policy sweep charging dictionary bytes, decoder memory and random/remote fetch

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: `ebr-crossfile dict-sweep` and the counting allocator (`research/harness/ebr-crossfile-DESIGN.md`) are not built |
| Domain / program sections | crossfile / 8.4 (8.5, 8.6, 8.7 for the DEC-CMP-014 arms) |
| decision_ids | DEC-CMP-020 (primary); DEC-CMP-014 (post-v1 codec/transform arms); DEC-CMP-013 (supplied-dictionary value and untrusted-dictionary handling); DEC-CON-004 (dictionary share of the declared working set) |
| Evidence type | deterministic bytes, exact fetch counts and exact allocator peaks (§4.6, §4.13, §4.14); training CPU in EXP-CROSSFILE-007; decode time in EXP-CROSSFILE-008 |
| Method | `research/decision-method.md`, pre-registered |
| Record | Design only; `record.md` (§12.2) instantiated once gate G-A holds. |
| Design session and disclosure | Phase C-design crossfile designer; L7 disclosure as in EXP-CROSSFILE-001. |

## 1. Question

What dictionary policy and exact minimum-benefit rule should v1 use (training sample count and
size, dictionary size, trainer, level, minimum cohort size or reuse, dictionary scope), and must
acceptance charge decoder memory and the random/remote fetch cost of the Dictionary in addition
to the Dictionary record bytes, the 64-byte section header and plan records under the strict
128-byte margin? For post-v1 (DEC-CMP-014): do LZMA2 preset dictionaries, LZ4 dictionaries or a
structural transform before a Zstandard dictionary produce gains that clear the minimum-gain bar
of a new registered mode?

## 2. Hypotheses

- **H1a (acceptance rule).** Charging the Dictionary's record bytes once more per cold random read
  (`charge-access-k1`) rejects a minority of dictionaries, costs less than 1 band unit of T-01
  `artifact_bytes` at corpus level, and reduces the cold-member-read `http_bytes` of
  dictionary-coded members by at least 1 band unit of T-11 on F04 and F05.
- **H1b (declaration).** The production declared working set adds the **sum** of all
  Dictionary lengths (`codec.rs` `aggregate_archive_decode_requirements`) although a reader holds
  one Dictionary per Chunk decode, so `declared_tightness_ratio` of the working set grows with
  the dictionary count and exceeds 1 by more than the T-23 band on archives with >= 4
  Dictionaries.
- **H1c (training).** COVER or fastCOVER with parameter optimisation, or a 2x larger dictionary
  at dense/extreme, lowers `artifact_bytes` by >= 1 band unit on F04 and F06 relative to the
  frozen construction; per-archive dictionaries lose to per-cohort ones on heterogeneous items.
- **H1d (post-v1, DEC-CMP-014).** No LZMA2 or LZ4 dictionary mode and no transform-before-
  dictionary composition clears the §7.3 registered-extended bar (k = 3) on its target
  families.
- **H0.** Every alternative rule, size, trainer and scope is `EQUIVALENT` to `sq-abs128` on the
  primary metrics; the status quo is retained by R10.
- **Falsification.** H1a is falsified if `charge-access-k1` is `DISTINGUISHABLE_WORSE` on
  `artifact_bytes` or not better on `http_bytes` beyond the band on validation. H1b is falsified
  if the measured decode `alloc_peak_bytes` of the largest dictionary-coded read is within the
  T-23 band of the declared working set on every archive with >= 4 Dictionaries. H1c and H1d
  are falsified by the corresponding confirmatory verdicts or minimum-gain tests.

## 3. Decisions informed; proposed OD and HC sets

| Decision | Proposed type | Proposed od_ids | Proposed hc_ids |
|---|---|---|---|
| DEC-CMP-020 | EMPIRICAL (profile-scope form of Appendix B.1 if so classified) | OD-05, OD-06, OD-08, OD-12, OD-17 | HC-06, HC-09, HC-13, HC-16 |
| DEC-CMP-014 | EMPIRICAL + cost (C1-C5) | OD-05, OD-07, OD-08 (OD-21, OD-22 as cost inputs) | HC-06, HC-12, HC-16 |
| DEC-CMP-013 (this experiment's part) | HUMAN-FACING for its user-facing claims; EMPIRICAL for the supplied-dictionary value | OD-05, OD-17 (OD-25 cost input) | HC-06, HC-13 |
| DEC-CON-004 (this experiment's part) | FORMAL + EMPIRICAL | OD-08, OD-17 | HC-06, HC-16 |

## 4. Candidates

Per profile (balanced, dense, extreme; fast has no cross-file stage and is out of scope), over
the cohorts `sq-bottomk-v1` forms. Every non-status-quo rule or construction is a new planner ID
and, where it changes the construction string, a new construction identifier (HC-16).

| candidate_id | Definition | Encodable by production reader | Expected tier |
|---|---|---|---|
| `sq-abs128` | Status quo: frozen v3 training caps (16/32/64 samples, 16/32/64 KiB each, 8/16/32 KiB dictionary), train-buffer construction, dictionary wins only if complete cost + 128 < independent | yes | reference |
| `margin-max32-1pct`, `margin-max256-2pct` | Margin `max(32 B, 1%)` (v4 rule) or `max(256 B, 2%)` (v5 reconstruction rule) of the independent cost | yes | T1 |
| `charge-access-k1`, `charge-access-k2` | Accept only if independent - candidate > 128 + k x Dictionary record bytes | yes | T1 |
| `charge-memcap-{8,16,32}k` | Reject Dictionaries longer than the cap (decoder-memory charge as a bound) | yes | T1 |
| `charge-access-k1-memcap16k` | Both charges (ledger "decoder-and-access-cost-charged") | yes | T1 |
| `min-reuse-{2,4,8,16,32}` | Require >= N referencing Chunks | yes | T1 |
| `min-reuse-bytes-{64k,256k,1m}` | Require >= B logical bytes referencing the Dictionary | yes | T1 |
| `size-{4,8,16,32,64,112}k` x `samples-{8,16,32,64,128,256}` x `samplecap-{4,16,64}k` | Screening: one-factor-at-a-time around each profile's frozen point, plus a 24-point Latin hypercube on dense (seed 20260917031, points listed in `spec.yaml`); balanced and extreme hypercubes are added in Stage C only if a dense point beats `sq-abs128` beyond the band on tuning | construction strings outside the nine registered ones are **not** production-decodable: rows are research-decoded and marked | T1 (new construction ID) |
| `trainer-cover-opt`, `trainer-fastcover-opt`, `trainer-legacy` | `ZDICT_optimizeTrainFromBuffer_cover` / `_fastCover` with default parameter search, and `ZDICT_trainFromBuffer_legacy`, at the frozen size | research-decoded (new construction) | T1 |
| `raw-content-leader-{8,16,32}k` | Raw-content Zstandard dictionary: the first N bytes of the cohort leader (first Chunk in digest order) | research-decoded (new format identifier) | T2 (new Dictionary format value) |
| `per-archive-{16,32,64,112}k` | One Dictionary per archive over all Chunks <= 64 KiB (digest-order samples), used by every cohort and by non-cohort small Chunks where it wins | research-decoded if the plan structure differs | T1 |
| `per-category` | One Dictionary per extension category (categories of `cheap-type-path`, EXP-CROSSFILE-001) | research-decoded | T1 |
| `exact-marginal-accounting` (designer-proposed) | Status-quo training, but the 64-byte section header is charged once per archive and shared plan records once, not per cohort | yes | T1 |
| `supplied-family` | Caller-supplied Dictionary (SPEC `pack --dictionary`): trained with the frozen construction on a disjoint tuning item of the same family (pairs listed in `spec.yaml` params) | research path (no CLI flag exists) | T2 (new user-visible flag) |
| `supplied-mismatched` | Negative control: supplied Dictionary trained on a different family | research path | T2 |
| `no-dictionaries` | Dictionaries disabled; cohorts may still use lookback where the profile has it | yes | T1 |
| DEC-CMP-014 arms: `lzma2-preset-dict`, `lz4-block-dict`, `delta8-then-zstd-dict`, `shuffle-then-zstd-dict` | LZMA2 raw at the registered (preset, dictionary) pairs with the trained Dictionary as preset dictionary; LZ4 block compression with the Dictionary; delta8 or byte-shuffle (production structural transforms) before Zstandard-with-Dictionary; on F06, F07 and the cohort families | research-decoded | T2 registered-extended, T3 if baseline |
| `critic-TBD` | R0 item 6 slot | | |

- **Reference candidate:** `sq-abs128` per profile; for DEC-CMP-014 arms the reference is the
  best production plan on the same cohort (status-quo Zstandard dictionary or independent).
- **R0 coverage:** status quo; SPEC `dictionary_policy none|supplied|train-if-measured-to-pay`
  (`no-dictionaries`, `supplied-family`, and the trained rules); every ledger-named alternative;
  strongest incumbent technique (zstd trained dictionaries at larger sizes via COVER/fastCOVER;
  incumbents do not embed per-cohort dictionaries, recorded); defer (`no-dictionaries` doubles as
  the defer arm for dictionary-less v1, with add-later cost C7 recorded); critic slot.
- **Excluded before execution:** in-place change of the frozen v3-v5 construction strings or
  rule: L0, HC-16. Any construction whose decode depends on the libzstd trainer version rather
  than stored bytes: not applicable (Dictionary bytes are stored; training drift is a writer
  determinism question, EXP-CROSSFILE-009/-010).
- **Adversarial supplied dictionaries (DEC-CMP-013, HC-06 screen):** a pre-registered set of 8
  hostile Dictionary inputs (truncated header, bad magic, declared content size above the
  archive budget, 1 GiB length declaration, Dictionary whose window exceeds the declared
  decoder window, zero-length, random bytes, a valid Dictionary with a colliding identity claim)
  run through `entrybound::research::codec::validate_dictionary` and the research embed path.
  Metric: `adversarial_true_refusal_fraction` (binary) and `refusal_alloc_peak_bytes`.

## 5. Corpus selectors

- Manifest `ed28b1b4…`, tuning split only.
- **Tuning (`spec.yaml`):** the 50-item cross-file target set of EXP-CROSSFILE-001 §5 (covers the
  required F01, F04, F05, F06 plus F02, F03, F08, F17, F18, F19, small F07 and F20 screening).
  DEC-CMP-014 transform arms additionally use every tuning F07 item of scale small or medium
  (listed in `spec.yaml` under the `f07` shard).
- **Supplied-dictionary pairs (rule, `--supplied-from auto-family-pair`):** the supplying item is
  the first tuning item, by item id, of the same family and a different independence group
  (`supplied-family`), or the first tuning item by item id of the next family in F01..F20 order
  (`supplied-mismatched`); the tool records the supplier item id in every row; families with one
  tuning group skip `supplied-family` (recorded).
- **Cold member reads:** the T2 draw of EXP-CROSSFILE-002 (32 seeded entries), restricted to
  entries with at least one dictionary-coded Chunk; plus the EXP-CROSSFILE-005 access sample.
- **Validation look:** validation split, all families small and medium, plus the four large items
  of EXP-CROSSFILE-001 §5; W and S only, after the MDE gate.

## 6. Environment and platform requirements

`wsl-ubuntu`, x86-64 Linux, ext4; loopback `ebr-origin` child for exact HTTP counts; `timing:
false`; in-process counting allocator (exact, §4.14) for `alloc_peak_bytes`, one decode per
process for per-read peaks (harness review use constraint 3). Prerequisites as EXP-CROSSFILE-001
§6.

## 7. Commands

`ebr validate|plan|run|verify-raw|normalize|report --spec research/experiments/EXP-CROSSFILE-003/spec.yaml`
as in EXP-CROSSFILE-001 §7. Per-sample command:

```text
/root/eb-research/target/harness/release/ebr-crossfile dict-sweep
  --item-path {item_path} --item-id {item_id} --experiment-id EXP-CROSSFILE-003 --env-name wsl-ubuntu
  --profile {profile} --rule {rule} --construction {construction} --scope {scope}
  --supplied-from auto-family-pair --codec-arm {codec_arm}
  --archive-out {scratch}/out.eb --memo-dir /root/eb-research/cache/crossfile/EXP-CROSSFILE-003/rep{rep}
  --cold-reads t2 --origin-bin /root/eb-research/target/harness/release/ebr-origin
  --alloc-isolate true --verify-roundtrip true
```

The adversarial dictionary screen is one extra candidate `adversarial-dict-screen` run on one
generated item (`spec.yaml` generated item `xf003-adversarial-anchor`), command
`ebr-crossfile dict-sweep --adversarial-set v1`.

## 8. Seeds, warmups, repetitions

Seeds `order` 20260917031, `bootstrap` 20260917032, `command` 20260917033; LHS seed as above.
Warmups 0. Repetitions 2 plus repeat-hash on `archive_sha256` and on `dictionary_set_sha256`.
Allocator peaks are deterministic for a fixed binary and input under the single-threaded decode
path; if the two repetitions differ, the metric is reclassified stochastic (§4.13) and moves to
EXP-CROSSFILE-008's timing-style repetitions.

## 9. Metrics

| Metric | OD | Role | ebr family | Threshold | Source |
|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | **primary** | size | T-01 | `payload.artifact_bytes` |
| `http_bytes` (cold read of one dictionary-coded member, metadata-first open included) | OD-12 | **primary** | size | T-11 | `payload.http_bytes_cold_member_read` |
| `http_requests` (same read) | OD-12 | secondary | other | T-12 | `payload.http_requests_cold_member_read` |
| `alloc_peak_bytes` (decode, one member read) | OD-08 | **primary** | memory_peak | T-06 | `payload.alloc_peak_bytes_member_read` |
| `declared_tightness_ratio` (declared `working_set_bytes` / measured decode peak) | OD-17 | **primary** | other | T-23 | `payload.declared_tightness_ratio_working_set` |
| `declared_cost_accuracy` (measured / declared <= 1) | HC-06 / HC-09 | binary | correctness | §3.3 | derived |
| `dictionary_bytes` | OD-05 | diagnostic (primary only for the size/sample sub-question, declared as such for that sub-analysis) | size | T-02 | `payload.dictionary_bytes` |
| `encode_cpu_s` (training + candidate encodes) | OD-06 | primary, measured in EXP-CROSSFILE-007 | time_cpu | T-05 | EXP-CROSSFILE-007 |
| `decode_throughput_bps` | OD-07 | primary for DEC-CMP-014 arms, measured in EXP-CROSSFILE-008 | throughput | T-05b | EXP-CROSSFILE-008 |
| `adversarial_true_refusal_fraction` | HC-06 | binary | correctness | §3.3 | adversarial screen |
| `refusal_alloc_peak_bytes` | OD-17 | secondary | memory_peak | T-06 | adversarial screen |
| `dictionaries_accepted`, `dictionary_chunks`, `net_savings_bytes`, `accept_flips_margin_x0_5`, `accept_flips_margin_x2` | - | descriptive | other | none | `payload.*` |
| `roundtrip_ok`, `archive_sha256`, `encodable_by_production` | HC-02, HC-13 | binary / repeat-hash | correctness | §3.3, §4.13 | `payload.*` |

Derived, generated by the analysis (never hand-computed, §12.3): break-even reuse count and
cohort size per profile (smallest N whose median `net_savings_bytes` is positive).

## 10. Normalization and statistical analysis

As EXP-CROSSFILE-001 §10. Specifics:
- Profile-scope: DEC-CMP-020 asks for a per-profile policy. Under Appendix B.1 the superiority
  metric is `artifact_bytes` only and `alloc_peak_bytes`, `access_granularity_bytes`,
  `encode_wall_s` and `decode_wall_s` are non-inferiority guards; `http_bytes` and
  `declared_tightness_ratio` then act as secondary and R9 policy-partition evidence (local
  versus remote access, R9 partition 2). Both analyses are pre-declared; the one matching the
  assigned decision type governs.
- `http_bytes` and `alloc_peak_bytes` are per-read quantities: item value is the median over the
  item's cold reads (deterministic), and the p95 is reported.
- DEC-CMP-014 arms are judged with the §7.3 minimum-gain rows: registered-extended codec mode
  (k = 3) and, if proposed for the baseline set, k = 5, in >= 2 applicable families with >= 2
  groups each; the other metric non-inferior.
- DEC-CON-004 part: the ratio of declared to measured is compared across three declaration
  formulas computed from the same archive: production (max codec + sum of Dictionaries + max
  preceding bytes), `max-dictionary` (max codec + largest Dictionary + max preceding bytes) and
  `per-group` (per-group requirement maximum). Formulas that under-declare on any read are R1
  (HC-06); the survivors are compared on `declared_tightness_ratio`.

## 11. Practical-significance thresholds

T-01, T-02, T-06, T-11, T-12, T-23, T-05, T-05b by reference; §7.3 minimum-gain rows and caps.

## 12. Sensitivity analysis

decision-method §6 in full, plus (a) margin sensitivity: accept/reject flips under x0.5 and x2
margins, reported per profile; (b) cold-read set: T2 draw versus the EXP-CROSSFILE-005 access
sample; (c) coalescing gap x0.5 and x2 for `http_bytes`; (d) per-cohort 64-byte section and
shared-plan charge bias (`exact-marginal-accounting` versus `sq-abs128`).

## 13. Validation look and held-out placeholder

As EXP-CROSSFILE-001 §13 (`EXP-CROSSFILE-003-HO` at Commit A; §5.5 single freeze).

## 14. Expected negative results worth recording

- Trained dictionaries win rarely under the strict margin on real corpora, and the win is below
  the band at corpus level (dictionary machinery not justified for balanced).
- Dictionary-coded members cost more to fetch remotely than they save in bytes for NP-LAN and
  NP-METRO random reads.
- COVER/fastCOVER optimisation changes the Dictionary bytes but not the archive bytes beyond band.
- The production working-set declaration is loose by a large factor on dictionary-heavy archives
  (declaration defect), or, if any read exceeds it, an HC-06 violation.
- LZMA2/LZ4 dictionary modes do not clear the registered-extended minimum gain.

## 15. Threats to validity

- Research-decoded constructions are not production-decodable: their `artifact_bytes` are real
  encoder output but their decode path is the research decoder (validity control 2).
- Supplied dictionaries simulate an expert choice; a real user's supply may be better or worse.
- Loopback counts are exact, but the value of saved fetch bytes depends on the network profile
  (modelled latency reported in EXP-CROSSFILE-004 for the same reads).
- Allocator peaks exclude kernel and mapped memory (the §4.14 in-process oracle is exact only for
  heap allocations).
- Corpus and L7 threats as EXP-CROSSFILE-001.

## 16. Cost

About 300 CPU-hours (dictionary training dominates; 177 screening candidates in `spec.yaml`: one-factor-at-a-time plus 24 LHS
points on dense), wall about 15 h sharded; disk: scratch about 3 GB, memo about 5 GB, raw about
2 GB. Agent effort: none for execution; scripted tooling with judgment for the trainer bindings
and codec arms; judgment for analysis.

## 17. Evidence these decisions need that this experiment does not produce

- **DEC-CMP-013:** the SPEC first-use comparison of profile-only versus override flags is a
  HUMAN-FACING claim (decision-method §1.2, §4.16): routed to the ecosystem/evaluation domain's
  simulated sessions (heuristic only) and to `EXTERNAL_REVIEW_REQUIRED` for any comprehension
  claim; flag-count cost C6 comes from the §14 item 7 cost scripts; "recording of overrides for
  reproducible replan and explain" is a FORMAL check of the recorded policy identity (HC-13) by the
  planner domain. This experiment supplies only the measured value of a supplied Dictionary and the
  hostile-input screen; EXP-CROSSFILE-004 supplies the value of a lookback override.
- **DEC-CMP-014:** decoder complexity, dependency cost and independent implementability of LZMA2/LZ4
  dictionary modes (ledger: Tasks 28, 31) are cost elements C2-C5 assessed by the codecs,
  conformance and ecosystem domains; this experiment supplies only size, memory and (via
  EXP-CROSSFILE-008) decode throughput.
- **DEC-CON-004:** reader behaviour on all-ones flag and budget vectors and the constrained-reader
  use-case study for per-group declarations belong to the container domain (program §11); this
  experiment supplies the measured declared-versus-actual working set for dictionaries.
