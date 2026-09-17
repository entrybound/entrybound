# EXP-CHUNK-004: End-to-end archive cost of chunking constructions and size policies

This is the anchor experiment of the chunking domain. It produces the primary `artifact_bytes`, `access_amplification` and `access_granularity_bytes` evidence for DEC-CMP-001 and DEC-CMP-002, and carries their validation looks and held-out placeholder.

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CHUNK-004 |
| Status | NEEDS_TOOLING: research-internals `plan_directory_with_ranges`, `ebr-cdcpack`, `roundtrip_eb.py`, `make_arm_specs.py`; gate G-B tooling (§14 items 2, 13) for decision-grade analysis |
| Arms | **SQ** (`spec.yaml`): tool-validity gate and status-quo reference, 4 frozen profiles. **A** (`spec-arm-a.yaml`, generated): construction comparison at balanced planning. **B** (`spec-arm-b.yaml`, generated): size policy per profile. **V1** (`spec-validation-look1.yaml`, generated after tuning): confirmatory look. **H** (Phase D placeholder). |
| decision_ids | DEC-CMP-001, DEC-CMP-002, DEC-CMP-003 (archive-wide reference costs), DEC-CMP-004 (actual bytes per unique chunk and reference; see EXP-CHUNK-005), DEC-CMP-017 (status-quo scope costs; see EXP-CHUNK-007), DEC-MOD-039 (uniform chunking reference), DEC-ACC-029 (chunk-count headroom) |
| Proposed decision types | DEC-CMP-001 EMPIRICAL (+FORMAL parts in EXP-CHUNK-012); DEC-CMP-002 EMPIRICAL, **profile-scope constrained form** (Appendix B.1) |
| Proposed od_ids | DEC-CMP-001: OD-05, OD-06, OD-08, OD-10, OD-16, OD-21. DEC-CMP-002: OD-05, OD-06, OD-07, OD-08, OD-10. |
| Proposed hc_ids | HC-02 (round trip), HC-06 (declared bounds incl. chunk-count limit), HC-10, HC-13, HC-14, HC-16 |
| Gates | G-A unmet (draft). G-B: `workload-mix.json`, decision tooling (§14 item 2), frozen held-out lock. Until then all verdicts are informational. |
| Author / L7 | As EXP-CHUNK-001 section 1 |

## 2. Question

Once the production v6 planner and encoder run on their chunks, what complete artifact bytes do CDC constructions and size policies produce? Complete bytes include descriptor, manifest, Index, frames, dictionaries, groups, side data and footer. The same runs report access granularity and access amplification per stratum. All are reported per profile, per family and in aggregate.

Which construction (DEC-CMP-001) and which min/target/max set per profile (DEC-CMP-002) is on the measured epsilon-Pareto frontier? Does any beat the status quo by more than its minimum-gain band?

## 3. Hypotheses

- **H-SQ (binary tool validity).** For every tuning item and frozen profile, `ebr-cdcpack` with `gear-norm-v1`, the frozen candidate lists and frozen proxy selection produces an archive SHA-256-identical to `ebr-pack` (production `plan_directory` + `ecf::encode`). Every archive round-trips (content tree equal).
  - **Any mismatch** invalidates the tool for decision use until fixed and re-proved. It is logged as a harness defect, not a finding about candidates.
- **H-A (construction, DEC-CMP-001).** At identical (min, target, max) with realized means within ±5%, no surviving construction is `DISTINGUISHABLE_BETTER` than `gear-norm-v1` on `artifact_bytes` at corpus level against the T1 minimum-gain band (k = 2, §3.1, §7.2). The expected outcome is `EQUIVALENT` or `INCONCLUSIVE`, which routes the decision to R10, where status quo ranks below cost tier.
  - **Falsified if** a construction is `DISTINGUISHABLE_BETTER` against the k = 2 band, is `NON_INFERIOR` on every other primary metric, and passes the R4 condition 3 guard.
- **H-B (size, DEC-CMP-002).** For each profile, under the constrained form (superiority metric `artifact_bytes` only), at least one non-frozen size set from the EXP-CHUNK-002 Stage B frontier is `DISTINGUISHABLE_BETTER` than the frozen preset(s) against the k = 2 band in at least one family with ≥ 2 groups, while non-inferior on `access_granularity_bytes`.
  - The corpus-level superiority prediction is weak (under 1 band unit for `balanced`). The prediction is at least 1 band unit for `dense` and `extreme` on F17 and F18.
  - **Falsified if** no family shows it for any profile.
- **H0.** Every surviving candidate is `EQUIVALENT` to the frozen status quo on `artifact_bytes` in every profile scope.

## 4. Decisions informed

| Decision | Arm | What is settled here | Elsewhere |
|---|---|---|---|
| DEC-CMP-001 | A, V1, H | OD-05 primary `artifact_bytes`; OD-10 primary `access_amplification` | OD-06/OD-08 (EXP-CHUNK-008); OD-16 keyed feasibility (EXP-CHUNK-006, crypto domain); OD-21 (EXP-CHUNK-012); locality (EXP-CHUNK-003) |
| DEC-CMP-002 | B, V1, H | Superiority `artifact_bytes`; guard `access_granularity_bytes`; chunk-count headroom; window-matched maximum; min/max ratio variants; incumbent size classes; single- vs multi-candidate lists at frozen proxy selection | Guards `encode_wall_s`, `alloc_peak_bytes` (EXP-CHUNK-008), `decode_wall_s` (EXP-CHUNK-009); remote bytes (EXP-CHUNK-009); R2 profile-bound screen pending DEC-CMP-035 |
| DEC-CMP-003, DEC-CMP-004, DEC-MOD-039 | SQ, B | Archive-wide per-profile reference costs and per-occurrence tables consumed by EXP-CHUNK-005 | EXP-CHUNK-005 |
| DEC-CMP-017 | SQ | Status-quo archive-wide scope bytes and occurrence tables consumed by the scope model | EXP-CHUNK-007 |
| DEC-ACC-029 | SQ, B | `max_chunks_in_one_object` and archive chunk counts against the 4,000,000 limit per size policy | memory beyond RAM (scale domain) |

## 5. Candidates

### Arm SQ (`spec.yaml`)

`sq-gear-norm-v1-{fast,balanced,dense,extreme}`: status quo (research baseline `9e44608` behaviour, frozen `docs/chunking-v1.md` policies and proxy selection). These are the **reference candidates** for arms A and B within each profile.

### Arm A (generated from EXP-CHUNK-002 Stage A and EXP-CHUNK-003 screens)

- **Constructions:** the survivor set S_A (≤ 4 non-status-quo constructions plus `gear-norm-v1`).
- **Sizes:** for each construction, targets {64 KiB, 128 KiB, 256 KiB, 512 KiB, 1 MiB, 2 MiB} at (t/4, 4t), single size per run (no candidate list), balanced codec and cohort policy (`--codec-profile balanced`, planner v6 stages).
- **Exclusions:**
  - `quickcdc-feature-acceptance` (R1, SPEC §7.3 L803);
  - `fixed-size` (SPEC §7.2 L792; included once at 512 KiB as a measured baseline, never selectable);
  - any construction recorded `R2 unspecifiable` in EXP-CHUNK-002.

### Arm B (generated from EXP-CHUNK-002 Stage B)

- **Per profile P ∈ {fast, balanced, dense, extreme}, with P's codec and cohort policy:**
  - (i) each Stage B frontier size set (≤ 12) under the provisional construction winner W_A from arm A tuning, and under `gear-norm-v1`;
  - (ii) the incumbent size classes (`fastcdc-reference-small`, `casync-zpaq-64k`, `backup-incumbent-1to2m` as restic and Borg, `large-4m` as kopia and Duplicacy) with their own algorithms;
  - (iii) min/max ratio variants at P's frozen target (`min-max-ratio-variants`);
  - (iv) `window-matched-max` (max capped at 1 MiB, the frozen `zstandard/v1` window, where P's frozen maximum exceeds it);
  - (v) candidate-count policies: P's frozen list, each single member, and the 2-member and 4-member lists (`single-global-policy` = the `balanced` list for every P);
  - (vi) `adaptive-by-archive-size`: the target from the total plaintext bucket (≤ 64 MiB → 128 KiB; ≤ 1 GiB → 512 KiB; else 2 MiB; thresholds pre-registered here), deterministic.
- **Selection** among list members uses the frozen proxy in this arm. The corrected-constants, end-to-end and sampled rules are EXP-CHUNK-005.
- **"Window raised to match maximum"** (the other half of `window-matched-max`) needs a new codec identifier (frozen 1 MiB window). It is costed by the codecs domain and represented here only by `ebr-codec chunk-matrix --window` sizes on the same chunks (secondary, `MODELLED`).

### Preliminary cost tiers `[INFERRED; computed by §14 item 7 scripts]`

| Candidate type | Tier |
|---|---|
| frozen presets | T0 |
| new size set or list under `gear-norm-v1` (new planner/chunker ID, permanent vectors) | T1 |
| new construction | T1 (T2 if a new reason code or field is needed) |
| adaptive-by-archive-size | T1 |

R0 item 6 (critic candidate) is open.

## 6. Corpus

- **Tuning (arms SQ, A, B):** all 112 tuning items, 20 families.
- **Validation (V1):** all 91 validation items. The look is registered in `research/decisions/validation-looks.jsonl` with decision ids, experiment id, W, S, primary metrics, item ids, commit and time (§5.2). At most 2 looks per decision; a second look needs new validation groups.
- **Minimum support (§4.9):** validation has ≥ 2 independence groups in all 20 families (F10 and F12 have exactly 2), so corpus-level verdicts are supportable. Family guards on 2-group families are weak (§4.9 limitation).
- **Held-out (arm H, Phase D):**
  - **Placeholder only.** `spec-heldout.yaml` is written at the design freeze (Commit A, §5.5) with `splits: [heldout]` and every candidate that reached the freeze. It runs once with `EB_HELDOUT_UNLOCK` = `design_freeze_sha`, after Commits A, B and C.
  - **MDE:** held-out MDE per supporting comparison is computed before unlock from validation group variance and held-out group counts (fingerprint-level fields only). A comparison above 2 band units routes the decision to `INSUFFICIENT_EVIDENCE` before unlock.
  - **Cap:** the `identity_aware_heldout` soft cap applies unless sealed tranches exist.
- **Public tuning sources:** the item upstreams (manifest provenance), checked by a held-out-aware session before V1 (§5.4 consequence 2).

## 7. Environment and platform

- `wsl-ubuntu`, x86-64, ext4 under `/root/eb-research`. Deterministic byte metrics only (timing guards live in EXP-CHUNK-008/009).
- Windows-host byte identity for the same archives is checked in EXP-CHUNK-012 (PCR); LAI and AUX differ by host by design (F-0001, F-0002).
- Large storage: transient scratch ≤ 2 × the largest item per shard.

## 8. Commands

```sh
# preconditions: disk_guard, lock_parity (PASS), harness-cli-parity (PASS)
# production CLI for round-trip oracle and comparisons (use constraint 2)
CARGO_TARGET_DIR=/root/eb-research/target/exp-chunk-prod /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli
# harness build incl. new research-internals wrapper; identity re-proof is mandatory after the wrapper lands
bash research/harness/internals-identity-check.sh            # must PASS (additive audit + 8/8 archive identity)
CARGO_TARGET_DIR=/root/eb-research/target/exp-chunk bash research/harness/build.sh build --release -p ebr-pack -p ebr-chunk
export PYTHONPATH=research/tools; PY=/root/eb-research/venv/bin/python
# Arm SQ
$PY -m ebr run --spec research/experiments/EXP-CHUNK-004/spec.yaml --gzip
$PY -m ebr verify-raw --spec research/experiments/EXP-CHUNK-004/spec.yaml --refingerprint
$PY -m ebr normalize --spec research/experiments/EXP-CHUNK-004/spec.yaml
# Arms A and B (specs generated, committed, then run with the same chain)
$PY research/experiments/EXP-CHUNK-004/make_arm_specs.py --arm A \
    --stage-a research/normalized/EXP-CHUNK-002/decision/stage-a.csv \
    --edits research/normalized/EXP-CHUNK-003/decision/edits.csv --out research/experiments/EXP-CHUNK-004/spec-arm-a.yaml
$PY research/experiments/EXP-CHUNK-004/make_arm_specs.py --arm B \
    --stage-b research/normalized/EXP-CHUNK-002/decision/stage-b.csv \
    --arm-a research/normalized/EXP-CHUNK-004/decision/arm-a-tuning.csv \
    --incumbents research/experiments/EXP-CHUNK-002/incumbent-presets.json --out research/experiments/EXP-CHUNK-004/spec-arm-b.yaml
# Decision analysis (G-B tooling, research/tools/decide/, section 14 item 2)
$PY -m decide analyze --experiment EXP-CHUNK-004 --decisions DEC-CMP-001,DEC-CMP-002 \
    --thresholds research/methods/thresholds.json --workload-mix research/methods/workload-mix.json \
    --out research/normalized/EXP-CHUNK-004/decision/
# V1 look (after MDE gate passes): generated spec with splits [validation], W and S only
$PY research/experiments/EXP-CHUNK-004/make_arm_specs.py --arm V1 --w-s research/normalized/EXP-CHUNK-004/decision/w-s.json \
    --out research/experiments/EXP-CHUNK-004/spec-validation-look1.yaml
```

## 9. Seeds

- **Spec seeds:** `{order: 2026091741, bootstrap: 2026091742, command: 2026091743}`.
- **Access sampling:** `--access-seed 20260917`, candidate-independent, so ranges pair across candidates. Offsets are drawn uniformly over logical bytes of regular files.
- **Selection-stability bootstrap:** 2,000 replicates seeded from `seeds.bootstrap` (§6.7).
- Warmups 0 (deterministic).

## 10. Metrics

| Metric (spec name) | Canonical metric | OD | Role in DEC-CMP-001 | Role in DEC-CMP-002 (profile scope) | Threshold |
|---|---|---|---|---|---|
| `artifact_bytes` (file size; must equal `artifact_bytes_accounted`) | `artifact_bytes` | OD-05 | **primary** | **sole superiority metric** | T-01 |
| `access_amplification_{1b,4k,1m,entry}` | `access_amplification` (strata never pooled) | OD-10 | **primary** (guards per stratum) | secondary | T-22 |
| `access_granularity_bytes` | `access_granularity_bytes` | OD-10 | secondary (one primary per OD) | **non-inferiority guard** (Appendix B.1) | T-09 |
| `index_bytes`, `metadata_bytes`, `dictionary_bytes`, `side_data_bytes` | same | OD-05 | diagnostic | diagnostic | T-02 |
| `metadata_bytes_per_entry` | same | OD-05 | secondary | secondary | T-02b |
| `fixed_overhead_bytes` (small tier; computed by `fixed_overhead_refs.py` from the item's content concatenated in manifest order under the M05.3 codec set) | same | OD-05 | secondary | secondary | T-21 |
| `dedup_stored_fraction` | same | OD-05 | diagnostic | diagnostic | T-01 |
| `chunk_count`, `max_chunks_in_one_object`, `selected_chunker_id` | no | — | secondary (headroom; HC-06 declared limit) | secondary | none |
| `roundtrip_ok` (check) | `roundtrip_mismatches` = 0 | OD-02 | binary R1 | binary R1 | §3.3 |
| `sq_gate_identical` (check, arm SQ) | tool validity | — | binary gate | binary gate | — |
| `archive_sha256` | repeat-hash (§4.13) | HC-13 | binary R1 | binary R1 | §3.3 |

**Definitions.**

- `access_granularity_bytes`: the worst-case decoded bytes needed to read one arbitrary byte, summed over the chunk's decode closure (chunk logical length plus group lookback prefix plus dictionary bytes), maximized over chunks. It is cross-checked against production `inspect` "worst-access-bytes"; any disagreement is a defect of one side, adjudicated per objective §2.0 rule 8.
- `access_amplification`: decoded bytes over requested bytes per seeded range, as the mean over 1,000 ranges per stratum (whole-entry stratum: all entries equally weighted). p95 is reported as secondary.

## 11. Replication

2 repetitions with the repeat-hash check on `archive_sha256` (§4.6, §4.13). Planning and encoding are claimed deterministic (SPEC §5.7), so a mismatch is an R1 violation artifact without a confirming rerun, unless an infrastructure fault is proven. No adaptive rule.

## 12. Statistical analysis

1. **L1.** Exact values; floors per §3.1 (T-01 small tier relative-only). Log ratio versus the profile's SQ reference.
2. **L2-L4.** Group means, family means (equal groups), corpus weighted by `research/methods/workload-mix.json` (gate G-B). The equal-weight arm is a sensitivity (§6.5). Strata: scale tier; `access_amplification` stratum.
3. **Tuning (A, B).** Exploratory R4 to form S and W per decision scope:
   - DEC-CMP-001: universal scope; W_A = continuous band-unit utility winner (§6.2) with equal per-OD weights over its measured ODs;
   - DEC-CMP-002: per profile; W_P = the smallest-`artifact_bytes` candidate meeting guards.

   R9 partitions are tried in order profile → policy → workload mix → family. Per-family winners are expressible only via categorisation (EXP-CHUNK-005, expressibility constraint (i)).
4. **MDE gate** before V1 (§4.12): MDE ≤ 1 band unit per confirmatory comparison, using tuning group variance with the validation group structure. On failure the look does not happen; add validation groups or keep `INSUFFICIENT_EVIDENCE`.
5. **Confirmatory (V1).** W against each s ∈ S at corpus level:
   - superiority at two-sided 1 − 0.01/k_sup on the declared superiority metrics;
   - `NON_INFERIOR` at 0.99 on all primary metrics;
   - family harm guard at 0.90 per family, tier and stratum;
   - Welch-Satterthwaite corpus intervals and pooled family intervals (§4.10); verdict classes per §4.11;
   - R4 condition 5 via computed tiers;
   - R6 minimum-gain bands per tier (log-consistent, §3.1).
6. **R10.** Tie set and keys; demonstrated power required under `INCONCLUSIVE`.
7. **Profile scope (DEC-CMP-002).** The constrained form: R2 profile-bound screen (inactive until DEC-CMP-035 fixes bounds), superiority `artifact_bytes` only, guards `encode_wall_s`, `decode_wall_s`, `alloc_peak_bytes` and `access_granularity_bytes`. The timing and memory guards come from EXP-CHUNK-008 and EXP-CHUNK-009 on the same candidates. **DEC-CMP-002 cannot become `DECIDED` before DEC-CMP-035** (§2 R2 "Profile bounds").
8. **Pareto reporting.** Per-family and aggregate epsilon-Pareto frontiers over (`artifact_bytes`, `access_amplification_4k`, `access_granularity_bytes`), plus timing and memory from EXP-CHUNK-008 once available. Required by the ledger for DEC-CMP-001 and DEC-CMP-002; informational, never selecting.
9. **Program-level.** Supporting comparisons are listed for the phase E Benjamini-Yekutieli report (§4.12).

## 13. Practical-significance thresholds

T-01 (`artifact_bytes`, `dedup_stored_fraction`), T-02/T-02b (metadata components), T-09 (`access_granularity_bytes`), T-21 (`fixed_overhead_bytes`) and T-22 (`access_amplification`), exactly as in `research/methods/thresholds.json` `metric_thresholds`, with tier multipliers from `cost_tiers`. Not restated.

## 14. Sensitivity analysis

§6 in full, per decision scope:

- weight grid and Dirichlet over the OD weights (DEC-CMP-001 only; profile scope has no weights);
- SMAA whole-simplex report;
- threshold perturbation ×0.5 (subject to the resolvability condition) and ×2;
- equal-weight arm; leave-one- and leave-three-families-out; family Dirichlet;
- WM-* mixes; leave-one-mix-out;
- tier-stratified;
- byte-weighted (`total_stored_bytes`);
- selection-stability bootstrap (2,000).

Additional pre-registered arms:

- realized-mean-matched interpolation versus identical-parameter comparison (DEC-CMP-001);
- F20 excluded from aggregation (F20 remains a screen);
- `window-matched-max` on/off.

## 15. Expected negative results worth recording

- No construction beats `gear-norm-v1` on `artifact_bytes` against the T1 band. The chunker choice then rests on R10, and possibly on OD-06 throughput or OD-16 keyed feasibility.
- The frozen `balanced` preset is not on the aggregate frontier.
- Multi-candidate lists (`dense`, `extreme`) rarely select a non-first member.
- `window-matched-max` changes nothing (most maxima are rarely reached).
- Incumbent size classes are dominated on F17 but win on F16 or F18 large images.

## 16. Threats to validity

- **Research pack path versus production:** controlled by the binary SQ gate, then only candidate chunk ranges differ. Non-frozen chunker IDs have no production writer, so their archives are research-only and cannot be confirmed with the production CLI beyond decode (which never runs the chunker, I30).
- **Planner coupling:** v6 cohort and dictionary stages depend on chunk boundaries, so a construction's measured effect includes planner interaction. This is the intended complete-cost measurement.
- **Deterministic metrics hide timing trade-offs:** paired with EXP-CHUNK-008.
- **Workload mix not yet cited (G-B):** until then only the equal-weight arm is computed, and nothing is decision-grade.
- **F03/F17/F18 cross-family covariates and the identity-aware held-out** (§5.4): reported caps.
- **Metadata on WSL ext4 differs from Windows capture** (F-0001/F-0002): affects metadata bytes equally across candidates; not a comparison confound.

## 17. Estimates

| Item | Estimate |
|---|---|
| Arm SQ | 4 profiles × 36.1 GiB × 2 reps, plus the `ebr-pack` reference pack and production unpack per sample. Planning at about 5-100 MB/s (extreme slowest): about **40 CPU-hours** |
| Arm A | ≤ 5 constructions × 6 sizes, balanced planning, 2 reps: about **25 CPU-hours** |
| Arm B | about 4 profiles × (≤ 24 size or incumbent candidates + list variants) ≈ 110 configs, dense and extreme dominating: about **200 CPU-hours** |
| V1 | W plus ≤ 4 S per scope on 26.6 GiB: about **60 CPU-hours** |
| Disk | transient scratch ≤ 20 GB per shard (6 shards ≈ 120 GB); occurrence tables about 15 GB |
| Agent effort | scripted runs; judgment for spec generation review, analysis interpretation, cost-tier assessment (C4-C7 by an independent session) |

## 18. Tooling to build

1. **research-internals wrapper** (default-off, additive child module; `research/harness/research-internals.md` rules 1-5):
   - signature: `entrybound::research::archive::plan_directory_with_ranges(input: &Path, options: PackOptions, chunker_id: String, ranges: &mut dyn FnMut(&[u8]) -> Result<Vec<Range<usize>>>) -> Result<Archive>`;
   - replays the `finish_with` loop of `crates/entrybound/src/archive/filesystem.rs` with the injected range function and chunker_id, then runs the production planner stages of `PackOptions`'s profile;
   - a second entry point takes a per-ContentObject range selector (for EXP-CHUNK-005);
   - **proof obligations:** `internals-identity-check.sh/.ps1` PASS, additive audit PASS, and a new test that the wrapper with `chunk_ranges` + frozen selection equals `plan_directory` for every profile.
2. **`ebr-cdcpack` binary** in `research/harness/crates/ebr-pack`:
   - **flags:** `--algorithm`, `--size-set frozen:<profile>|<min>/<target>/<max>[,…]`, `--selection proxy-frozen|proxy-corrected:<constants.json>|end-to-end|sampled:<rate>`, `--codec-profile`, `--planner v6`, `--layout`, `--stream-window`, `--granularity archive|size-class:<t1,t2,t3>|category:<categoriser>|object`, `--archive-out`, `--access-strata`, `--access-samples`, `--access-seed`, `--occurrence-out`;
   - **reuse:** `ebr_pack::bytes` and `ebr_pack::counts` for exact accounting;
   - **occurrence rows:** chunk digest, object digest, logical offset, logical length, stored frame bytes, frame header bytes, plan ref, group ref;
   - held-out guard.
3. **`research/experiments/EXP-CHUNK-004/roundtrip_eb.py`:** production `ebound unpack`, then `fpshim.fingerprint_path` content-tree comparison (the pattern of `research/baselines/probes/roundtrip_check.py`, which has no `.eb` tag).
4. **`fixed_overhead_refs.py`** (M05.3 single-stream references at pinned codec versions from `research/baselines/tool-versions.json`).
5. **`make_arm_specs.py`.**
6. **G-B dependencies:** `research/tools/decide/` (§14 item 2), `research/methods/workload-mix.json` (§14 item 13), `research/decisions/validation-looks.jsonl` (§14 item 14), cost scripts (§14 item 7).

## 19. Deviations

(append-only)
