# EXP-CRYPTO-006: Padding schedule privacy versus overhead (exact length model plus measured encrypted packs)

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

For each candidate padding schedule for encrypted archives, what is the storage and fetch overhead, and what length-based leakage (anonymity sets, min-entropy leakage, presence advantage of the committed suite) remains, per boundary mode and per family? The candidates are the quarter-octave BUCKETED status quo, power-of-two, Padmé, randomized, profile-specific, lower-minimum-payload-bucket, MAXIMUM, and NONE as reference. Relatedly, what does encrypted creation cost under padding for each dedup scope and small-object policy (DEC-CMP-017, DEC-CRY-025, DEC-CRY-026)?

## 2. Hypotheses and falsification

- **H1 (overhead).** Quarter-octave BUCKETED has `padding_overhead_pp` `DISTINGUISHABLE_BETTER` than power-of-two on every family with median record length above the minimum bucket. Padmé is within 1 band unit of BUCKETED on overhead. On the small tier, all schedules except `lower-minimum-payload-bucket` are `DISTINGUISHABLE_WORSE` on `fixed_overhead_bytes` against NONE, because of the 4 KiB PAYLOAD minimum.
- **H2 (privacy).** For deterministic schedules under a uniform prior over each family's known-file universe, `anonymity_set_median` orders MAXIMUM > power-of-two > Padmé ≈ BUCKETED > NONE. `presence_advantage` for A0/A4 under BUCKETED is `DISTINGUISHABLE` above 0 on F13 and F11 (distinctive large files), so padding is "quantised, not hidden" (SPEC §26.4 row 17 wording).
- **H3 (randomized).** Randomized padding with the same mean overhead as BUCKETED gives `presence_advantage` `NON_INFERIOR` to BUCKETED for single-archive observation, and `DISTINGUISHABLE_WORSE` when the observer sees many creations of the same content (averaging attack).
- **Falsification:** each H is tested on validation at the registered bands. For H1, BUCKETED `NON_INFERIOR` to power-of-two on overhead fails it only in the direction stated. For H2, a schedule ordering reversed by more than 1 band unit fails it.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: overhead is `EMPIRICALLY_MEASURED` (exact size accounting) plus `FORMALLY_DERIVED` (length functions). Privacy is a measured lower bound (attack suite) plus exact channel quantities for deterministic schedules. The **effectiveness of the padding schedule against leakage attacks is routed to external review** (SPEC §7.6 L854, §25.2; T-17). This experiment produces the internal evidence only.

## 4. Candidates and arms

| Arm | DEC-CRY-055 candidate | Length function (per record class CONTROL/PAYLOAD) |
|---|---|---|
| P0 (reference) | `status-quo-quarter-octave-bucketed` | `padded_len` in `crates/entrybound/src/crypto/container.rs` (buckets 2^k, 5·2^(k-2), 3·2^(k-1), 7·2^(k-2); CONTROL k=8..20, PAYLOAD k=12..26) |
| P1 | `power-of-two-buckets` | next 2^k within the same class ranges |
| P2 | `padme` | Padmé function from Nikitin et al., "Reducing Metadata Leakage from Encrypted Files and Communication with PURBs", PoPETs 2019(4), implemented from the paper's definition (locator recorded), clamped to the class range |
| P3 | `randomized-padding` | uniform random pad within [0, ⌈0.1·len⌉], plus a variant matching P0's mean overhead. The Borg `obfuscate` definition at 1.2.8 is recorded for comparison, not copied |
| P4 | `profile-specific-schedules` | P0 for `fast`/`balanced`, P1 for `dense`/`extreme` (an R9 partition; composed from the per-profile results) |
| P5 | `lower-minimum-payload-bucket` | P0 with the PAYLOAD minimum at 256 B, 512 B and 1 KiB |
| P6 | `maximum-default` | MAXIMUM (CONTROL 1 MiB, PAYLOAD 64 MiB) |
| P7 | `none-default` | NONE. **Excluded as a default at R1** (SPEC §7.6 L844; `docs/crypto-suite-v1.md` L537-543). Measured only as the leakage upper bound and overhead zero point |

Cross factors:
- **Boundary mode:** `SECRET_GEAR_TABLE` and `PHTE_AES128` (evaluation condition).
- **Profile:** all four.
- **Dedup and object policy (DEC-CMP-017 / DEC-CRY-025):**
  - D0 `exact-chunk-archive-wide` (status quo);
  - D1 `whole-file-only`;
  - D2 `exact-chunk-per-contentobject`;
  - D3 `tiny-object-inline-or-aggregate` (objects below 4 KiB aggregated into one padded object before padding);
  - D4 `no-dedup-option` (DEC-CRY-025);
  - all computed from plan-level chunk lists (exact).
- **Entry-count hiding (DEC-CRY-026):** `coarser-control-padding` = CONTROL objects padded with P1 or P6 while PAYLOAD uses P0.

## 5. Corpus selectors

- **Overhead (tuning, then one validation look):** every tuning item in F01-F19 at the small and medium tiers, plus one large-tier item per family where available: `f01-tuning-llvm-project-20-1-8-git`, `f02-tuning-ripgrep-cargo-target`, `f05-tuning-loghub-hdfs-v1`, `f06-tuning-gharchive-2024-01-15-h15`, `f07-tuning-qwen2-5-1-5b-safetensors-bf16`, `f08-tuning-postgres17-pgbench-s40`, `f10-linux-firmware-20260910-full-tree`, `f12-tuning-nasa-ivl-photos-large`, `f13-tuning-bbb-video-large`, `f15-tuning-real-ubuntu-noble-ova-raw`, `f16-ubuntu-noble-cloudimg-20260911-raw`, `f18-tuning-ubuntu-noble-cloud-images`. F20 items are excluded from overhead (no real-world share, Appendix B.2) but included in the MVT-14(j) conformance screen.
- **Privacy (tuning):** the known-file universes and contexts of EXP-CRYPTO-002 (F05, F07, F09, F11, F12, F13, F18).
- **Validation:** all F01-F19 validation items for overhead, and the EXP-CRYPTO-002 validation universes for privacy. One registered look per decision, meeting §4.9 minimum support.

## 6. Environment and platform requirements

`wsl-ubuntu` Linux x86-64, `timing: false`. Private record lengths come from real encrypted packs of the status-quo writer. Alternative schedules are applied as exact length functions, which is valid because the length function acts only on the unpadded private record length and class (`FORMALLY_DERIVED` from `padded_len`). P0, P6 and P7 are also produced by real `ebound pack --pad bucketed|max|none` on every item and compared byte-exact with the model (MVT-14(j)).

## 7. Commands and tooling

Tooling to build:
- `research/tools/crypto/padding_model.py`: length functions P0-P6, the class ranges, and randomized draws seeded.
- `ebr-crypto observe`, extended with `--emit-private-lengths` (research-internals accessor `private_record_lengths`) and the dedup/object policy transforms D0-D4.
- `research/tools/crypto/leakage_metrics.py` and `research/tools/crypto/cdc_attacks/presence_eval.py` at the EXP-CRYPTO-001 suite version.
- `ebr-crypto keygen` for recipients.

`spec.yaml`: one candidate per (boundary mode × profile). Each command emits all schedule × dedup-policy results for the item as `payload.<schedule>.<policy>.{padding_overhead_pp, artifact_bytes, fixed_overhead_bytes, http_bytes_single_entry}`. Privacy scoring runs as a second command over saved traces (`check` metric source).

## 8. Seeds, warmup, repetitions and adaptive rule

- Deterministic schedules: 2 repetitions plus repeat-hash of the per-item length traces. Recipient keys are seeded test keys.
- Randomized schedule P3: 32 draws per item, treated as a stochastic metric with order-statistic item intervals (§4.6).
- No warmups, no timing.
- Presence scoring sample sizes as EXP-CRYPTO-002 (at least 300 positives and 1,000 negatives per family).

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `padding_overhead_pp` | T-16 | OD-16 (M16.3) | **primary** (compared within one padding mode per T-16; schedule-to-schedule comparisons use `artifact_bytes`) |
| `artifact_bytes` | T-01 | OD-05 | **primary** across schedules and dedup policies |
| `fixed_overhead_bytes` | T-21 | OD-05 | **primary**, small tier |
| `presence_advantage` | T-17 | OD-16 (M16.2) | **primary**, per padding mode |
| `anonymity_set_median` | T-17 | OD-16 | secondary (primary for deterministic-schedule channel comparison) |
| `leak_min_entropy_bits`, `leak_shannon_bits` | T-17 | OD-16 | secondary |
| `http_bytes` for a single-entry range fetch (modelled: padded bytes of the entry's objects plus CONTROL prefetch) | T-11 | OD-12 | diagnostic (full remote study in EXP-CRYPTO-008) |
| `metadata_bytes` (CONTROL bytes) | T-02 | OD-05 | diagnostic for coarser CONTROL padding |

## 10. Normalization and statistical analysis

- **Size metrics:** exact item values (§4.13). Item comparison against band and floor. Aggregation L2-L4 (§4.9) with the cited workload mix; byte-weighted arm per §6.5.
- **Privacy metrics:** as EXP-CRYPTO-002.
- **Padding mode is an evaluation condition** (AGG-S). Nothing is averaged across padding modes. R4 and R8 guards apply per condition.
- **Trade-off:** resolved at R4 with `padding_overhead_pp` (or `artifact_bytes`) and `presence_advantage` both primary (T-16 note). Per-OD base weights are declared in the record before the look.
- **Confirmatory:** W against S on validation with k_sup-adjusted confidence. MDE gate.

## 11. Practical significance

T-16, T-01, T-21, T-17, T-11 and T-02 bands from `research/methods/thresholds.json`. A new padding schedule needs a new padding mode identifier with decode-visible semantics under the frozen registry (DEC-CRY-055 required evidence). An independent assessor computes its tier (§7.2), at least T2, and possibly T4 as a crypto-construction change. §7.3 minimum gains apply on held-out-replicated results.

## 12. Sensitivity analysis

§6 arms, plus:
- prior over the known-file universe (uniform versus size-weighted);
- observer sees one versus many creations (for P3);
- class ranges (P1 with PAYLOAD minimum 4 KiB versus 1 KiB);
- tier stratification (the small tier is expected to reverse orderings).

## 13. Expected negative results worth recording

- Padmé beats the status quo on privacy at equal overhead. That would mean the frozen BUCKETED default was suboptimal, which is recorded even though changing it costs a new mode id.
- BUCKETED overhead on F04 many-small-file trees exceeds the SPEC's "bounded, declared cost" expectations because of the 4 KiB PAYLOAD minimum.
- Coarser CONTROL padding hides entry count only up to quarter-octave classes, with a measurable CONTROL overhead on tiny archives.
- Disabling dedup under encryption (D4) costs little on most families but a lot on F17 and F18. That would make DEC-CRY-025's option profile- or workload-scoped.

## 14. Threats to validity

- Modelled schedules assume the writer applies them exactly as the length function does. Fragmentation of objects near the class maximum is modelled from the frozen fragmentation rule and checked for P0 against real packs.
- Uniform-prior channel metrics understate leakage for skewed real priors, so both priors are reported.
- The attack suite is a lower bound.
- Randomized padding against repeated observation depends on the observer model (O1 versus multi-snapshot storage adversary).

## 15. Held-out placeholder (Phase D)

After Commit A, the frozen candidates run once on held-out items (overhead and privacy) with `EB_HELDOUT_UNLOCK`. The held-out MDE is at most 2 band units per supporting comparison. No held-out item is named here.

## 16. Cost estimate and agent effort

- Compute: about 150 core-hours (encrypted packs of all tuning and validation items at 2 boundary modes × 4 profiles, with dense and extreme dominating, plus privacy scoring). Disk: about 120 GB transient and about 15 GB retained traces.
- Agent effort: tooling **scripted** (length functions, accessor) plus **judgment** for Padmé fidelity; execution **scripted**; analysis **judgment**.

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
