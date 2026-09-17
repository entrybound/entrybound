# EXP-CRYPTO-002: Boundary leakage of encrypted archives: public CDC, secret Gear, PHTE-AES128 and alternatives, by padding, layout and compression

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

For the committed observer model and attack suite (EXP-CRYPTO-001), how much does each chunk-boundary construction for encrypted archives leak about which known files an archive contains? The constructions are the status-quo secret Gear table, PHTE-AES128, a widened keyed rolling hash, fixed-size chunks, and keyed CDC with coarse re-packing, with public unkeyed CDC as the negative control. The analysis is per padding mode, segment layout, compression setting and profile.

## 2. Hypotheses and falsification

- **H1.** Under observers O2 (known plaintext) and O4 (adaptive), `gear-norm-secret-table-v1` with `NONE` padding has a `presence_advantage` that is not `NON_INFERIOR` to `phte-aes128-norm-v1` at the T-17 band (secret Gear leaks more). Predicted: at least 2 band units worse on F13 and F11, the families with distinctive large files.
- **H2.** With `BUCKETED` padding, the gap between secret Gear and PHTE shrinks by at least half in band units, but stays `DISTINGUISHABLE_WORSE` for O2 in at least one family.
- **H3.** Under O5 (key-holding), every keyed construction's advantage is `EQUIVALENT` to public unkeyed CDC under the same padding. The boundary secret protects nothing against a key holder.
- **H4.** Compression reduces A1/A2 advantage relative to stored-only chunks (Alexeev et al. recommendation).
- **H0 / equivalence.** Secret Gear and PHTE are `EQUIVALENT` on `presence_advantage` under `BUCKETED` padding for every observer.
- **Falsification.** H1 is falsified if secret Gear is `NON_INFERIOR` to PHTE at corpus level for O2 and O4 under `NONE`. H3 is falsified if a keyed construction beats public CDC for O5 by more than the band (which would point to an implementation defect in the observation pipeline). The directional claims are evaluated only on validation, after tuning selects W and S.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` leakage lower bounds per padding mode (graded under OD-16, never R1). Interpretation and any claim of effectiveness go to external review (§8.3, T-17). HC-14's mechanical parts (keyed boundaries, recorded padding mode, conforming public lengths) are checked here as binary preconditions on every produced archive: MVT-14(j) on every archive, and MVT-14(e) in EXP-CRYPTO-003.

## 4. Candidates and arms

**Boundary constructions (DEC-CRY-023 candidate ids):**

| Arm | Candidate | Implementation for this experiment |
|---|---|---|
| B0 | `public-unkeyed-cdc` | `chunk_ranges` (gear-norm-v1). **Excluded at R1** (I24, `docs/crypto-wire-v1.md` L56-57); measured only as the upper-bound negative control |
| B1 | `status-quo-secret-gear-default-phte-opt-in` (default mode) | production `pack_directory_encrypted` with `BoundaryMode::SecretGearTable`; harness-direct `chunk_ranges_encrypted` for key-controlled arms |
| B2 | `phte-default-everywhere` (PHTE mode) | `BoundaryMode::PhteAes128` |
| B3 | `widened-keyed-rolling-hash` | research-only 256-bit keyed buzhash chunker in `ebr-crypto` (new chunker id `research-buzhash256-keyed-v0`), normalized to the same min/target/max |
| B4 | `fixed-size-chunks-under-encryption` | research fixed-size chunker at the profile target size |
| B5 | `keyed-cdc-with-coarse-rechunking` | B1 or B2 boundaries, then objects packed into fixed-size padded containers (container sizes 4, 16 and 64 MiB) as an exact observation transform |
| — | `phte-default-for-selected-profiles` | not a separate arm: an R9 partition of B1 and B2 over profiles, evaluated from per-profile results |

**Evaluation conditions (never averaged; objective AGG-S):**

- Padding modes: `NONE`, `BUCKETED` (quarter-octave status quo), `MAXIMUM`, plus the alternative schedules of EXP-CRYPTO-006 applied as exact length functions (power-of-two, Padmé, lower-minimum-payload-bucket). EXP-CRYPTO-006 owns their selection; here they appear as conditions.
- Segment layouts (DEC-CRY-081, as exact observation transforms of the same private objects): L0 `status-quo-one-object-per-segment`, L1 `batch-objects-to-target-segment-size` (16 MiB and 64 MiB targets), L2 `fixed-size-padded-segments` (64 MiB), L3 `group-by-chunkgroup`, L4 `dummy-segments` (segment count quantized to quarter-octave buckets).
- Compression: planner as-is (`*-enc-v1` inheriting the v6 candidate sets), stored-only (research planner flag forcing the stored codec), both measured for DEC-CRY-028's `mandatory-compression-under-encryption`.
- Profiles: `fast`, `balanced`, `dense`, `extreme` (min/target/max from `crates/entrybound/src/chunker.rs`).
- Observers O1-O5 and attacks A0-A4 as committed by EXP-CRYPTO-001, at the attack-suite version recorded in the run metadata.

**Reference candidate:** B1 (status quo).

## 5. Corpus selectors

- **Tuning** (attack tuning, W and S selection): target universes from F05, F07, F09, F11, F12, F13 and F18 tuning items (files of at least 64 KiB; multi-chunk files of at least 1 MiB flagged). Archive contexts are (a) the natural item tree that contains the target, and (b) a synthetic context that inserts the target into a tuning item of another family (seeded choice). Negatives are the same contexts without the target, or with a size-matched decoy.
  - Item ids: `f05-tuning-gutenberg-books`, `f05-tuning-enwik8`, `f07-tuning-pythia-160m-safetensors-main`, `f07-tuning-mnist-npz`, `f09-ubuntu-noble-usr-binaries`, `f09-silesia-mozilla-ooffice`, `f11-tuning-ubuntu-noble-debs`, `f11-alpine-324-apks`, `f11-tuning-pypi-binary-wheels-cp312`, `f12-tuning-nasa-ivl-photos-medium`, `f12-tuning-kodak-jpeg-matrix-medium`, `f13-tuning-musopen-flac-medium`, `f13-tuning-librivox-audiobook-medium`, `f13-tuning-nasa-ntrs-pdf-small`, `f13-tuning-bbb-video-medium`, `f18-tuning-sqlite-amalgamation-series`, `f18-tuning-zstd-releases-1-5-x`, `f18-tuning-curl-8-x-release-series`.
- **Validation** (at most two looks per decision, each registered in `research/decisions/validation-looks.jsonl` before running): F05, F07, F09, F11, F12, F13 and F18 validation items, meeting §4.9 minimum support (at least 10 independence groups across at least 5 families). The look spec `spec-validation-look1.yaml` is written only after tuning names W and S.
- Contexts for F04 and F17 (many small files, duplicate trees) are added as a robustness stratum for the layout conditions.

## 6. Environment and platform requirements

`wsl-ubuntu`, Linux x86-64, `timing: false` (leakage is computed from deterministic observations for given keys). Windows is not needed: public lengths are platform-independent for identical plans. EXP-CRYPTO-007 checks the host-dependent feature bits of F-0001 separately.

## 7. Commands and tooling

Tooling to build:

1. `ebr-crypto observe`: for an item, boundary arm, key seed, profile and compression flag, it plans (production planner, or research chunkers for B3 and B4), records private record lengths and classes, and emits observation traces for every padding function and layout transform. On a seeded 2% sample of (item, key) cells, it also packs for real with `pack_directory_encrypted` and asserts that the predicted public length sequence equals the parsed real one (MVT-14(j) oracle; any mismatch is an R1 finding and invalidates the transform model for that arm).
2. Research-internals accessor (production crate, default-off, additive; MVT-16(c) identity proof re-run): `entrybound::research::crypto::{boundary_key_for_afk, plan_directory_encrypted_with_key, private_record_lengths}`, plus a stored-only planner switch.
3. `research/tools/crypto/cdc_attacks/*`, frozen at the EXP-CRYPTO-001 suite version.
4. `research/tools/crypto/leakage_metrics.py`: `leak_min_entropy_bits`, `leak_shannon_bits` and `anonymity_set_median` for deterministic channels (B0, B4, and O5 for keyed arms). For keyed arms under O1-O4 it reports plug-in estimates over K keys, labelled estimator-based.

Execution (after tooling):

```sh
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CRYPTO-002/spec.yaml
```

Each spec candidate is one (boundary arm × profile × compression) cell. Every command emits all padding × layout × observer observations for that cell, so padding and layout are conditions inside the row (keys `payload.presence_advantage.<padding>.<layout>.<observer>`).

## 8. Seeds, warmup, repetitions and adaptive rule

- K = 16 independent keys per (item context, arm, profile, compression) for presence scoring, and K = 64 for the plug-in channel estimates on 8 designated contexts. Key seeds derive from `seeds.command`.
- Deterministic outputs: 2 repetitions in different processes plus the repeat-hash check of observation traces (§4.6, §4.13). Given the key seed, the traces must be byte-identical.
- Warmups: 0. Timing: none.
- Sample sizes: at least 300 positive targets and at least 1,000 negatives per family in each split. The FPR-0.01 threshold is set on tuning negatives and applied unchanged to validation.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role | Unit | Measurement |
|---|---|---|---|---|---|
| `presence_advantage` | T-17 | OD-16 | **primary** (per padding mode and per observer; the headline is the maximum over the committed suite, per M16.2) | fraction | `presence_eval.py` over attack scores |
| `leak_min_entropy_bits` | T-17 | OD-16 | secondary; primary only for deterministic channels (B0, B4, O5) | bits | `leakage_metrics.py` |
| `leak_shannon_bits` | T-17 | OD-16 | secondary | bits | same |
| `anonymity_set_median` | T-17 | OD-16 | secondary | count | same |
| `padding_overhead_pp` | T-16 | OD-16/OD-05 | diagnostic (owned by EXP-CRYPTO-006) | pp | observation traces |
| `artifact_bytes` | T-01 | OD-05 | diagnostic (size cost of B4 and B5 layouts) | B | real packs on the sample, exact model elsewhere |
| key-recovery success and work | descriptive | — | reported per arm | fraction; core-s | attack scripts |

Every produced encrypted archive also carries its correctness gates: `roundtrip_mismatches` = 0 on the 2% real-pack sample, and MVT-14(j) conformance.

## 10. Normalization and statistical analysis

- **Unit and aggregation (§4.9):**
  - L1: `presence_advantage` for one target group and arm is TPR − FPR over that group's targets against the family negative pool.
  - L2: independence group.
  - L3: family (equal groups).
  - L4: corpus weights from `research/methods/workload-mix.json` (gate G-B), with equal weights as a sensitivity arm.
  - Absolute-only metric: arithmetic means of differences (§4.9).
- **Pairing:** arms are compared on the same targets, contexts and negatives, as paired differences per group.
- **Intervals:** corpus-level Welch-Satterthwaite on group-level differences (§4.10), and family-level pooled intervals for the harm guard.
- **Verdicts (§4.11):** three-way plus `NON_INFERIOR`.
- **Confirmatory set (§4.12):** W against each s in S on `presence_advantage` for each (padding mode, observer) condition that the decision scopes. Superiority metrics are declared per pair from tuning. The MDE gate must pass before the validation look.
- **Decision tooling (§14 item 2)** writes `research/normalized/EXP-CRYPTO-002/decision/`. ebr verdicts are informational.
- **Cross-experiment integration for DEC-CRY-023:** the decision analysis combines this experiment's OD-16 primary with EXP-CRYPTO-003's OD-06 primary (`encode_wall_s`) and OD-05 guard (`artifact_bytes`). Weights follow §6.3, with per-OD equal base weights unless the record declares otherwise before the look.

## 11. Practical significance

Bands and floors come from `research/methods/thresholds.json` `metric_thresholds` for `presence_advantage`, `leak_min_entropy_bits`, `leak_shannon_bits` and `anonymity_set_median` (T-17), `padding_overhead_pp` (T-16) and `artifact_bytes` (T-01). A default change between existing boundary modes is costed at the tier an independent assessor computes (§7.2; a policy/default change is at least T1). A new boundary mode (B3, B4, B5) is a crypto construction (T4), and the minimum-gain band of §7.3 applies to it.

## 12. Sensitivity analysis

§6 in full on validation results:
- weight grid and Dirichlet over the OD set of DEC-CRY-023;
- threshold perturbation ×0.5 and ×2, with the resolvability condition;
- leave-one-family-out, leave-three-families-out, family Dirichlet and the WM-* mixes;
- tier stratification;
- the selection-stability bootstrap with 2,000 replicates.

Experiment-specific arms:
- context type (natural tree versus synthetic insertion);
- known-plaintext fraction for O2 (1%, 10%, 50%);
- attack-suite version: re-score with the frozen version plus any later version, with both reported.

## 13. Expected negative results worth recording

- Bucketed padding erases most of the secret-Gear versus PHTE gap. This supports keeping the cheaper default, and is recorded against the stronger-mode claim.
- PHTE shows non-zero advantage above A0 under some layout. That would be a construction or implementation leak, escalated to external review.
- Compression does not reduce advantage for already-compressed families (F11, F12, F13), so H4 is scoped.
- Layout L2 (fixed-size padded segments) nearly removes the boundary signal, so segment layout rather than the boundary mode is the effective mitigation.
- Fixed-size chunking (B4) leaks least but loses shift-resistant dedup (measured in EXP-CRYPTO-003).

## 14. Threats to validity

- **Attack-suite lower bound:** absence of advantage is not safety.
- **Synthetic contexts:** insertion contexts may be easier or harder than real archives, so both context types are reported.
- **Key sampling:** 16 keys per context may miss weak-key classes. A weak-key search over 10^4 keys runs for the secret-Gear low-bit structure on 4 contexts.
- **Model-based layout transforms:** they are exact only if the research writer's assignment matches the frozen wire rules. The 2% real-pack check covers L0 only, so L1-L4 need the research writer of EXP-CRYPTO-008 for their own check.
- **Held-out identity awareness (§5.4)** caps confidence.
- **Shared base model** between attack designer and construction designers.

## 15. Held-out placeholder (Phase D)

After the single program-wide freeze (decision-method.md §5.5), every candidate that reaches Commit A is run once on the held-out split with the frozen attack-suite version and `EB_HELDOUT_UNLOCK` set. The held-out spec is written in Commit A with its held-out MDE (§4.12, at most 2 band units). R5 criteria apply. No held-out item is named here.

## 16. Cost estimate and agent effort

- Compute: about 300 core-hours (planning and observation for about 18 tuning contexts × 6 arms × 4 profiles × 2 compression settings × 16 keys, plus a similar validation look, plus attack scoring). Disk: about 200 GB transient scratch and about 20 GB retained raw and traces.
- Agent effort: tooling **judgment** (observation transforms, accessor); execution **scripted**; analysis **judgment** (decision tooling outputs only).

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
