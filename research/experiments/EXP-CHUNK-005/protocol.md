# EXP-CHUNK-005: Chunking selection rule, parameter granularity and categoriser

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CHUNK-005 |
| Status | NEEDS_TOOLING: `ebr-cdcpack` selection and granularity modes, per-object range selector (research-internals second entry point, EXP-CHUNK-004 section 18), categorisers C1/C2, `fit_constants.py`, and `size-sets.json` from EXP-CHUNK-004 arm B |
| Arms | **P** proxy accuracy (analysis of EXP-CHUNK-004 SQ/B byte accounting plus one STREAM-layout pass); **S** selection rules (`spec.yaml`, `sel-*`); **G** granularity (`spec.yaml`, `gran-*`); **C** categoriser accuracy and determinism (`spec-categoriser.yaml`, generated); **V1** validation look (generated); **H** Phase D placeholder |
| decision_ids | DEC-CMP-004 (selection rule), DEC-CMP-003 (granularity and categoriser), DEC-MOD-039 (multiple chunker parameter sets in one archive; uniform-across-profiles), DEC-CMP-002 (number of size candidates per profile) |
| Proposed decision types | EMPIRICAL (DEC-CMP-003, DEC-CMP-004); DEC-MOD-039 EMPIRICAL (cost) + FORMAL (PCR binding soundness, EXP-CHUNK-012) |
| Proposed od_ids | DEC-CMP-004: OD-05, OD-06. DEC-CMP-003: OD-05, OD-06, OD-23 (cost input). DEC-MOD-039: OD-05, OD-11, OD-23. |
| Proposed hc_ids | HC-10 (PCR binds chunker parameters), HC-11 (decoders never categorise, I30), HC-13 (deterministic planner and categoriser), HC-16 |
| Gates / author / L7 | As EXP-CHUNK-001 section 1; G-B for decision-grade verdicts |

## 2. Question

Which chunking-parameter selection rule minimizes complete archive bytes at bounded selection work? And which parameter granularity does? And what does mixed-parameter chunking cost in cross-category dedup and identity binding?

**Selection rules:**

- frozen plaintext proxy (164 B per unique chunk + 40 B per reference);
- constants corrected to measured v6 framing and Index bytes;
- end-to-end complete cost;
- sampled compressed proxy;
- first member;
- weighted objective.

**Granularity:**

- global single;
- archive-wide per profile (status quo);
- per file-size class;
- per content category (two deterministic categorisers);
- per ContentObject.

## 3. Hypotheses

- **H-P (proxy accuracy, DEC-CMP-004).** The frozen constants (64 B frame + 100 B Index record per unique chunk; 40 B per reference) differ from the measured v6 INDEXED per-unique-chunk overhead by more than the T-02 band (relative), and from STREAM by more still. **Falsified if** the fitted per-unique-chunk and per-reference coefficients (tuning, least absolute deviation on byte accounting) lie inside T-02 of the frozen constants for both layouts.
- **H-S (selection).** The frozen proxy selects a different list member than end-to-end selection on ≥ 5% of tuning archives for `extreme`. Yet at corpus level, end-to-end selection is not `DISTINGUISHABLE_BETTER` than the frozen proxy on `artifact_bytes` against the T1 minimum-gain band (k = 2).
  - **Falsified (in favour of change) if** end-to-end or corrected-proxy selection is `DISTINGUISHABLE_BETTER` against the k = 2 band and `NON_INFERIOR` on `encode_cpu_s` (EXP-CHUNK-008).
- **H-G (granularity).** Per-category and per-object granularity are not `DISTINGUISHABLE_BETTER` than archive-wide per-profile selection against their tier band: T2 at least, because per-object parameter binding adds a field (§7.2); T4 if PCR identity participation changes. Per-file-size-class is within T-01 of archive-wide.
  - **Falsified if** any finer granularity is `DISTINGUISHABLE_BETTER` against its computed tier's band in ≥ 2 families with ≥ 2 groups, `NON_INFERIOR` elsewhere and on OD-06.
- **H-X (cross-category loss, DEC-MOD-039).** On items with identical byte regions embedded under different categories (F03 vendored copies, F14 archives-in-archives, F18 series), `cross_category_duplicate_bytes_lost` under C1 per-category chunking exceeds 1% of logical bytes on at least one item. **Falsified if** it is ≤ 1% on every such item.
- **H-C (categoriser).** C1 and C2 are deterministic (identical category assignment across 3 runs and thread counts {1, 8, 32}). Their agreement with libmagic MIME major type (reference labels, not ground truth) is ≥ 0.8. **Determinism failure is binary** (HC-13) and eliminates the categoriser at R1.
- **H0.** All selection rules and granularities are `EQUIVALENT` on `artifact_bytes` at corpus level.

## 4. Decisions informed

| Decision | Arm | Contribution | Elsewhere |
|---|---|---|---|
| DEC-CMP-004 | P, S | Modelled vs actual bytes per unique chunk and reference (INDEXED, STREAM); how often each rule changes the selection and the size delta; selection work (`selection_bytes_encoded`, `selection_candidates_evaluated`); regret vs the best list member | Pack CPU of 1-4 candidate evaluations (EXP-CHUNK-008); regret vs exhaustive planner search (planner domain) |
| DEC-CMP-003 | G, C | Per-category vs archive-wide Pareto; categoriser accuracy, determinism and work; regret of staged vs per-object selection; SPEC §7.3 per-category requirement tested against measured cost | Descriptor/PCR encoding cost as a spec-complexity cost input (§7.1 C1, computed by cost scripts) |
| DEC-MOD-039 | G | Cost of mixed parameter sets; uniform-across-profiles (`gran-global-single`) vs per-profile chunking; modelled per-object binding bytes | PCR binding soundness and divergence census (EXP-CHUNK-012) |
| DEC-CMP-002 | S | Whether multi-candidate lists (dense 2, extreme 4) earn their selection work | Size values (EXP-CHUNK-004) |

## 5. Candidates

All use `gear-norm-v1` (so the granularity question is not confounded by construction). The construction winner of EXP-CHUNK-004 is substituted by amendment if it is not `gear-norm-v1`.

### Selection rules (`sel-*`, profiles `dense` and `extreme`, frozen candidate lists)

| Candidate | Definition | Ledger id | Preliminary tier |
|---|---|---|---|
| `sel-proxy-frozen-*` | Status quo `select_parameters` | `status-quo-plaintext-proxy` (**reference**) | T0 |
| `sel-proxy-corrected-*` | Same rule with constants fitted on tuning (arm P) in `proxy-corrected-constants.json`, committed before arm S runs | `corrected-constants-by-format-version` | T1 (new selection version and planner ID) |
| `sel-end-to-end-*` | Plan and encode under every list member; keep the smallest `artifact_bytes`; earlier member wins ties | `end-to-end-complete-cost` | T1 |
| `sel-sampled16-*` | Compress a deterministic 1-in-16 sample of unique chunks (by digest order) with the profile's codec plans, scale by 16, add corrected framing constants | `sampled-compressed-proxy` | T1 |
| `sel-first-member-*` | No selection: first list member | `fixed-policy-no-selection` | T1 |
| `sel-weighted-*` | Corrected proxy + w_access × `access_granularity_bytes` per unique chunk, weights from `weighted-objective.json` (pre-registered: w_index = 1, w_access = 64 B per MiB of granularity) `[INFERRED weights; sensitivity 16/256]` | `weighted-objective` | T1 |

### Granularity (`gran-*`, profiles `balanced` and `extreme`)

| Candidate | Definition | Ledger id | Preliminary tier |
|---|---|---|---|
| `gran-archive-profile-*` | Status quo (**reference**) | `status-quo-archive-wide-per-profile` | T0 |
| `gran-global-single-*` | `balanced` list for every profile | `global-single`; DEC-MOD-039 `uniform-chunking-across-profiles` | T1 |
| `gran-size-class-*` | File-size classes (≤ 64 KiB, ≤ 1 MiB, ≤ 16 MiB, larger), size set per class from `size-sets.json` | `per-file-size-class` | T1 if recorded only by planner ID; T2 if per-object parameters must be recorded |
| `gran-category-c1-*` | Categoriser C1 (`c1-magic-entropy-v0`: integer-only magic-number table + zstd-free order-0 entropy estimate over the first 64 KiB, classes text/source, executable (ELF/PE/Mach-O), JPEG/PNG/media, compressed/incompressible, numeric-array heuristic, other) | `spec-per-category` (SPEC §7.3) | T2-T4 (per-object parameters recorded; T4 if PCR participation changes; DEC-MOD-039) |
| `gran-category-c2-*` | Categoriser C2 (`c2-dwarfs-like-v0`: incompressible probe, binary by architecture, pcmaudio, reimplemented from DwarFS documentation at the pinned v0.15.7 release; cited by the implementer) | `dwarfs-style-categorizer` | as C1 |
| `gran-object-*` | Each ContentObject picks the smallest end-to-end cost among the size set | `per-contentobject-selection` | T2-T4 |

**Mixed-parameter archives.**

- The research writer emits one composite `chunker_id` string: `multi-v0/` + sorted distinct ids joined by `|`. This is allowed by the status-quo free-form grammar (DEC-CMP-005) and research-only.
- The per-object binding a real format would need is **modelled** as `modelled_binding_bytes` = a table of distinct ids + one varint index per ContentObject. It is added to `artifact_bytes` in the primary analysis and reported separately.

**Excluded:** none at L0/R1. `per-contentobject-selection` without recorded parameters is not a separate candidate: decoders do not need parameters (I30), but PCR must bind them (DEC-MOD-039), so an unrecorded variant would violate HC-10's descriptor binding. It is recorded as an R1(b) exclusion **candidate** pending independent sign-off with the written counterexample: two archives with identical chunk roots but different parameter provenance give identical PCR.

R0 item 6: critic candidate open.

## 6. Corpus

- **Tuning:** all 112 items (ids in `spec.yaml`).
- **Arm C:** 20 tuning items with the most distinct files per family (≤ 2 per family), plus every F14 and F19 tuning item.
- **Validation look V1:** registered look on all 91 validation items for W and S of DEC-CMP-003 and DEC-CMP-004, after the MDE gate. Look budget per §5.2.
- **Held-out:** Phase D placeholder per EXP-CHUNK-004 section 6 (§5.5; held-out MDE ≤ 2 band units; `identity_aware_heldout` cap).
- **Embedded-content items for H-X:** the F03, F14 and F18 tuning items (subset of the 112).

## 7. Environment and platform

`wsl-ubuntu` x86-64; deterministic metrics; no timing (selection CPU is timed in EXP-CHUNK-008 on the W/S rules). libmagic reference labels come from Ubuntu `file` (version recorded by `probe_tool_versions.py`). No privileges, no network.

## 8. Commands

```sh
# preconditions and builds as EXP-CHUNK-004 section 8
# Arm P: fit constants on tuning byte accounting (INDEXED from EXP-CHUNK-004 SQ, STREAM from a STREAM pass)
$PY research/experiments/EXP-CHUNK-005/make_stream_pass_spec.py --out research/experiments/EXP-CHUNK-005/spec-stream-pass.yaml
$PY -m ebr run --spec research/experiments/EXP-CHUNK-005/spec-stream-pass.yaml --gzip   # + verify-raw, normalize
$PY research/experiments/EXP-CHUNK-005/fit_constants.py \
    --occurrences /root/eb-research/results/EXP-CHUNK-004 --stream /root/eb-research/results/EXP-CHUNK-005/stream \
    --out research/experiments/EXP-CHUNK-005/proxy-corrected-constants.json          # commit before arm S
cp research/normalized/EXP-CHUNK-004/decision/size-sets.json research/experiments/EXP-CHUNK-005/size-sets.json  # commit
# Arms S and G
$PY -m ebr validate --spec research/experiments/EXP-CHUNK-005/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-CHUNK-005/spec.yaml --gzip
$PY -m ebr verify-raw --spec research/experiments/EXP-CHUNK-005/spec.yaml --refingerprint
$PY -m ebr normalize --spec research/experiments/EXP-CHUNK-005/spec.yaml
# Arm C (categoriser determinism and agreement)
$PY research/experiments/EXP-CHUNK-005/make_categoriser_spec.py --out research/experiments/EXP-CHUNK-005/spec-categoriser.yaml
#   runs: ebr-chunk categorise --item-path {item_path} --categoriser c1-magic-entropy-v0|c2-dwarfs-like-v0 --threads 1|8|32
#         and `file --mime-type -b` per file for reference labels
# Analysis
$PY -m decide analyze --experiment EXP-CHUNK-005 --decisions DEC-CMP-003,DEC-CMP-004,DEC-MOD-039 --out research/normalized/EXP-CHUNK-005/decision/
```

## 9. Seeds

- **Spec seeds:** `{order: 2026091751, bootstrap: 2026091752, command: 2026091753}`.
- The sampled proxy uses digest-order sampling (no seed).
- Categorisers are seedless.
- The selection-stability bootstrap is seeded from `seeds.bootstrap`.
- Warmups 0.

## 10. Metrics

| Metric | Canonical | OD | Role | Threshold |
|---|---|---|---|---|
| `artifact_bytes` (+ `modelled_binding_bytes` for mixed archives) | yes | OD-05 | **primary** (DEC-CMP-003, DEC-CMP-004, DEC-MOD-039) | T-01 |
| `index_bytes`, `metadata_bytes` | yes | OD-05 | diagnostic | T-02 |
| `dedup_stored_fraction` | yes | OD-05 | diagnostic | T-01 |
| `encode_cpu_s` (from EXP-CHUNK-008 on W and S) | yes | OD-06 | **primary** (DEC-CMP-003, DEC-CMP-004) | T-05 |
| `archive_regret_bytes`, `selection_candidates_evaluated`, `selection_bytes_encoded`, `distinct_parameter_sets_used`, `cross_category_duplicate_bytes_lost`, `categoriser_bytes_examined`, `modelled_binding_bytes` | no | — | secondary | none |
| Fitted constants (arm P) | no | — | secondary; the proxy-accuracy test uses T-02 as the materiality reference for byte components | T-02 (reference) |
| Categoriser determinism (arm C, identical category assignment digest across runs and threads) | HC-13 test | — | binary | §3.3 |
| Categoriser agreement with libmagic major type | no | — | secondary | none |
| `roundtrip_ok`, `archive_sha256` | `roundtrip_mismatches`; repeat-hash | OD-02; HC-13 | binary | §3.3 |

## 11. Replication

2 repetitions and repeat-hash (§4.6, §4.13). Arm C: 3 runs per thread count {1, 8, 32} (objective MVT-13(a) pattern) plus one 20-repetition stress cell at 32 threads on the largest arm C item. No adaptive rule.

## 12. Analysis

- **Arm P.** Least-absolute-deviation fit on tuning INDEXED archives:
  - model: `artifact_bytes − Σ stored payload bytes − metadata_fixed = a·unique_chunks + b·references + c`;
  - same for STREAM;
  - bootstrap intervals over independence groups (2,000 replicates, seeded).

  The proxy-accuracy verdict compares fitted a, b with the frozen 164 and 40 relative to the T-02 band. Validation checks the fitted model's prediction error (secondary).
- **Arms S and G, tuning.** L1-L4 as EXP-CHUNK-004 section 12. Exploratory R4 on (`artifact_bytes`; `encode_cpu_s` where available) forms S and W per decision:
  - DEC-CMP-004 scope: planner policy per profile;
  - DEC-CMP-003 scope: universal policy with R9 partitions by profile.

  Expressibility (R9 (i)): per-family winners are expressible only through a categoriser, whose cost is its tier.
- **Regret.** `archive_regret_bytes` is the item-level difference to the best end-to-end list member. The distribution is reported per profile and family (descriptive). For `gran-object`, per-object regret versus the per-object optimum under corrected constants uses exact enumeration on items with ≤ 8 distinct objects and ≤ 4 sizes (≤ 65,536 assignments costed by the proxy, top 3 confirmed end-to-end). This is labelled `MODELLED` except for the confirmed rows.
- **H-X.** `cross_category_duplicate_bytes_lost` (bytes of identical regions ≥ 64 KiB present under ≥ 2 categories that fail to dedup under per-category parameters but dedup under archive-wide parameters) per item, reported directly.
- **Confirmatory V1.** Per EXP-CHUNK-004 section 12, items 4-6 (MDE gate, W vs S, superiority at 1 − 0.01/k_sup, non-inferiority 0.99, harm guard 0.90, R6 minimum gain per computed tier, R10 with power).
- **Arm C.** Determinism is binary; agreement is a per-family table.

## 13. Thresholds

T-01, T-02, T-05 as transcribed in `research/methods/thresholds.json` `metric_thresholds` (`artifact_bytes`, `index_bytes`, `metadata_bytes`, `encode_cpu_s`), and the tier multipliers in `cost_tiers`. Not restated.

## 14. Sensitivity

- §6 in full for DEC-CMP-003 and DEC-CMP-004. The weight grid applies to OD-05/OD-06 weights; threshold ×0.5/×2 with the resolvability condition; §6.5 family and mix arms; selection-stability bootstrap.
- `sel-weighted` access weights {16, 64, 256}.
- Size-class thresholds shifted ×0.5 and ×2.
- Categoriser C1 entropy cut ±10%.
- Binding-cost model: with vs without `modelled_binding_bytes`.
- Profiles `dense` vs `extreme` for arm S.

## 15. Expected negative results

- End-to-end selection costs 2-4× encode work for sub-band gains.
- Per-category chunking loses on mixed trees because embedded duplicates stop deduplicating.
- The SPEC §7.3 per-category requirement is not supported by measured bytes.
- Multi-candidate lists for `dense` almost never select the second member.
- libmagic disagreement shows categoriser classes are not meaningful for chunking.

## 16. Threats to validity

- **Mixed-parameter archives are research-only** (composite `chunker_id`). Their binding cost is modelled, not encoded, so the true wire cost may differ. Mitigation: sensitivity with 2× modelled binding bytes.
- **Proxy constants fitted on tuning and tested on the same tuning archives are optimistic.** Mitigation: the prediction error is reported on validation.
- **Categorisers designed by this program may be weaker than DwarFS's own.** C2 is a reimplementation, not the DwarFS binary. Its disagreement with `mkdwarfs --categorize` output on the same items is reported as secondary evidence (the DwarFS binary is available in WSL).
- **End-to-end selection interacts with planner cohort stages:** the measured effect is intended to be complete cost.
- **Profile coverage:** only `dense` and `extreme` have multi-member lists today; `balanced` list variants are covered in EXP-CHUNK-004 arm B(v).

## 17. Estimates

| Item | Estimate |
|---|---|
| Arm P | 1 STREAM pass × 4 profiles on tuning: about **30 CPU-hours** |
| Arms S and G | 24 candidates × 36.1 GiB × 2 reps; end-to-end rules multiply work by list length (extreme 4×): about **250 CPU-hours** |
| Arm C | about 2 CPU-hours |
| V1 | about **80 CPU-hours** |
| Disk | selection traces about 5 GB; scratch as EXP-CHUNK-004 |
| Agent effort | scripted runs; judgment for the categoriser implementations (C1 design, C2 from DwarFS docs), the R1(b) counterexample write-up, and analysis |

## 18. Tooling to build

1. `ebr-cdcpack` modes: `--selection {proxy-frozen, proxy-corrected:<json>, end-to-end, end-to-end-per-object, sampled:<n>, first-member, weighted:<json>}`, `--granularity {archive, size-class:<thresholds>, category:<categoriser>, object}`, `--size-set file:<json>#<profile>`, `--selection-trace-out`; payload `selection.*` fields as named in `spec.yaml`.
2. The research-internals per-object range-selector entry point (EXP-CHUNK-004 section 18 item 1).
3. `ebr-chunk categorise` subcommand with categorisers `c1-magic-entropy-v0` and `c2-dwarfs-like-v0` (integer-only; category digest output; `--threads` for the determinism test).
4. `research/experiments/EXP-CHUNK-005/{fit_constants.py, make_stream_pass_spec.py, make_categoriser_spec.py}`, `weighted-objective.json` (pre-registered weights above) and `proxy-corrected-constants.json` (generated on tuning, committed before arm S).

## 19. Deviations

(append-only)
