# EXP-CHUNK-006: Keyed boundary modes under encryption: HC-14 screen, dedup, locality and encrypted size

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CHUNK-006 |
| Status | NEEDS_TOOLING: keyed modes in `ebr-chunk` (`--boundary-mode`, `--key-count`), `ebr-chunk mvt14e`, `ebr-pack --chunk-boundary/--pad`, encrypted padding model |
| Arms | **D** key-averaged dedup (`spec.yaml`); **L** keyed edit locality (`spec-locality.yaml`, generated: `ebr-chunk edit-sweep` with keys); **K** HC-14 MVT-14(e) chance-rate test for new keyed constructions (`spec-mvt14e.yaml`); **E** encrypted archive bytes per padding mode with production modes (`spec-encrypted.yaml`); **N** no-dedup-under-encryption and coarse re-chunking space cost (analysis, `MODELLED`, validated against arm E). Throughput per mode with and without AES-NI is in EXP-CHUNK-008; leakage (OD-16) is owned by the crypto domain. |
| decision_ids | DEC-CRY-023 (encrypted boundary default: the non-leakage evidence), DEC-CMP-001 (keyed-variant feasibility of successor constructions, a hard constraint in the ledger: "keyed variants must exist for encrypted archives"), DEC-CRY-025 (space cost of disabling dedup under encryption), DEC-CMP-017 (encrypted creation overhead with bucketed padding on small chunks) |
| Proposed decision types | DEC-CRY-023: EMPIRICAL + EXTERNAL (attack effectiveness, proof mapping). DEC-CRY-025: EMPIRICAL + EXTERNAL. DEC-CMP-001: EMPIRICAL. |
| Proposed od_ids | DEC-CRY-023: OD-05, OD-06, OD-15, OD-16. DEC-CRY-025: OD-05, OD-16. |
| Proposed hc_ids | HC-14 (keyed boundaries; recorded padding mode; public lengths conform), HC-13 (LAI stable, ciphertext differs), HC-10 (PCR under keyed boundaries) |
| External review | Required for any claim about leakage resistance of a boundary mode (SPEC §25.2, `decision-method.md` §8.3). This experiment makes no such claim. |
| Gates / author / L7 | As EXP-CHUNK-001 section 1. The OD-16 attack suite and observer model (§14 item 16) are not a dependency of this experiment's metrics. |

## 2. Question

Under encrypted creation, consider the keyed boundary constructions:

- the status-quo `SECRET_GEAR_TABLE` default;
- `PHTE_AES128` opt-in;
- keyed variants of the successor chunker candidates.

For each: what does it cost in exact-chunk dedup and edit locality relative to unkeyed chunking at identical sizes? Does it pass the mechanical HC-14 keyed-boundary test? And what do encrypted archives cost per padding mode, including without dedup and with coarse fixed-size containerization?

## 3. Hypotheses

- **H-D (dedup cost of keying).** For `secret-gear-table` and `phte-aes128`, the key-averaged `dedup_stored_fraction` over 16 keys is within the T-01 band of `unkeyed-gear-norm-v1` at the same sizes in every non-F20 family, and the between-key max/min ratio is inside the band. `keyed-permute-ae-w1` and `keyed-permute-ram` are worse by ≥ 1 band unit in at least one family. **Falsified if** a status-quo mode is outside the band in some family, or the hashless keyed variants are inside it everywhere.
- **H-L (locality).** Keyed modes' p95 `new_unique_bytes` for 64 B and 4 KiB inserts are within a factor of 1.5 of unkeyed at the same sizes. PHTE's continuous (not-reset-at-cut) window (`docs/crypto-suite-v1.md` §Boundary modes) does not worsen resynchronization. **Falsified if** the factor is exceeded on ≥ 20% of items. The metric is secondary; a falsification is recorded as counterevidence, not an elimination.
- **H-K (HC-14 MVT-14(e), binary).** Every keyed construction passes. No two distinct keys give identical boundary sets on any item, and for no more than 1 key in 1,000 does the keyed-versus-unkeyed boundary-coincidence fraction exceed the item's 0.999 null quantile (`decision-method.md` Appendix C.1 row "Keyed-boundary chance rate"; objective MVT-14(e)).
  - **A violation is an R1 elimination** of that construction as an encrypted-archive chunker.
  - Expected positive control: `unkeyed-gear-norm-v1` fails (identical sets across keys).
  - Special case: `fixed-size` fails the mechanical test by construction (boundaries do not depend on content or key). Whether HC-14's keyed-boundary fact applies to content-independent boundaries is **not decided here**. It is routed to the constraint-crosswalk reviewer and the crypto domain. `fixed-size` is also excluded for new archives by SPEC §7.2 L792.
- **H-E (encrypted size).** Within bucketed padding, `padding_overhead_pp` of `dense`-sized chunks exceeds that of `balanced` by more than T-16's absolute threshold on F04 and F19 small-file trees. `artifact_bytes` for PHTE and secret-table modes is `EQUIVALENT` (T-01) at the same profile and padding mode. **Falsified if** the padding difference is inside T-16 on both families, or the modes are distinguishable.
- **H-N (no dedup under encryption).** Disabling dedup under encryption increases modelled `artifact_bytes` by more than the T-01 band on F17 and F18 at corpus family level, and by less than the band on F07, F12 and F13. **Falsified if** F17/F18 are within the band.
- **H0.** Keying, padding mode and dedup disabling are all within their bands everywhere.

## 4. Decisions informed

| Decision | Arms | Contribution | Not here |
|---|---|---|---|
| DEC-CRY-023 | D, L, E, K (SQ modes as controls), N (coarse re-chunking, fixed-size) | "Dedup ratio and boundary stability under insertions/edits per mode" (ledger); encrypted artifact bytes per mode and padding; fixed-size and containerization costs | Attack reproduction (Alexeev et al., Truong et al.) and leakage metrics (crypto domain, T-17); proof-mapping review (external); throughput (EXP-CHUNK-008); ARM64 (EXP-CHUNK-013, BLOCKED) |
| DEC-CMP-001 | D, L, K | Whether each successor construction has a keyed variant that passes HC-14 at acceptable dedup and locality cost. A construction without one needs a second chunker for encrypted archives (a cost, §7.1 C1/C7). | — |
| DEC-CRY-025 | N, E | "Space cost of disabling dedup on corpora" (ledger) under the production padding modes | "Within-archive equality leakage measurements" (crypto domain OD-16); multi-archive key domains (integrity domain) |
| DEC-CMP-017 | E, N | "Encrypted creation overhead with bucketed padding on small chunks" (ledger) | Unencrypted scopes (EXP-CHUNK-007) |

## 5. Candidates

| Candidate | Construction | Source | Status |
|---|---|---|---|
| `unkeyed-gear-norm-v1-*` | Frozen public chunker | DEC-CRY-023 `public-unkeyed-cdc` | **Excluded under encryption** before execution (R1: I24; `docs/crypto-wire-v1.md` L56-57; ledger exclusion). Measured only as the unkeyed reference and the MVT-14(e) positive control. |
| `secret-gear-table-*` | `gear-norm-secret-table-v1`. Table from 16 seeded research keys. The production HMAC derivation is replaced by uniformly random tables, equivalent under the PRF assumption `[FORMALLY_DERIVED under that premise]`. Exactness of production derivation is checked in arm E via `ebound inspect --chunks` on real encrypted archives. | **Status quo default** (DEC-CRY-023 `status-quo-secret-gear-default-phte-opt-in`); **reference** for arms D, L, E | frozen |
| `phte-aes128-*` | `phte-aes128-norm-v1` via `EncryptedBoundaryKey::PhteAes128 { polynomial, aes_key }` from seeded research keys | Status quo opt-in; `phte-default-everywhere` and `phte-default-for-selected-profiles` are policy recombinations of the per-profile results (no extra run) | frozen |
| `keyed-gear-spread-nc2-*` | Spread-mask NC-2 Gear with a secret table | DEC-CMP-001 keyed variant of `gear-spread-mask-nc2` | new (research construction; EXPERT_REVIEW_REQUIRED for security) |
| `keyed-buzhash-w64-*` | Buzhash with a keyed 256-entry u64 table, window 64 | DEC-CRY-023 `widened-keyed-rolling-hash` (research instance; the "256-bit" wording in the ledger is ambiguous and is recorded as an open definition question) | new; EXPERT_REVIEW_REQUIRED |
| `keyed-rabin-w64-*` | Rabin over a per-key random irreducible polynomial of degree 53 (irreducibility verified by the tool) | DEC-CMP-001 keyed variant of `rabin-fingerprint` | new; EXPERT_REVIEW_REQUIRED |
| `keyed-permute-ae-w1-*`, `keyed-permute-ram-*` | Keyed byte permutation (Fisher-Yates from the key) applied before the hashless rule | DEC-CMP-001 keyed variants of `ae-extremum`, `ram-hashless` (research constructions; hashless CDC has no published keyed construction known to this design; the implementer searches the primary literature and cites any) | new; EXPERT_REVIEW_REQUIRED |
| `fixed-size-*` | Content-independent blocks | DEC-CRY-023 `fixed-size-chunks-under-encryption` | Measured. HC-14 applicability routed (H-K). SPEC §7.2 L792 excludes it for new archives. |
| `keyed-cdc-coarse-rechunk-{1m,4m}` | Keyed chunks packed into fixed 1 MiB / 4 MiB padded containers (arm N, `MODELLED` from arm E chunk tables and the container fill) | DEC-CRY-023 `keyed-cdc-with-coarse-rechunking` | new wire structure (T4 preliminary) |
| `no-dedup-under-encryption` | Every chunk occurrence stored and padded (arm N, `MODELLED` via the EXP-CHUNK-007 scope model plus the bucketed padding function, validated against arm E real bytes) | DEC-CRY-025 `no-dedup-option` | T2 preliminary (new flag) |

- **Sizes:** `balanced` (128/512 KiB/2 MiB) and `dense` (64/256 KiB/1 MiB) presets in arm D. Arm K uses `balanced` only. Arm E uses the four production profiles.
- **Excluded, not measured:** `cross-tenant-convergent` (DEC-CRY-025) and `convergent-cross-archive-dedup` (DEC-CMP-017), per the ledger (SPEC §9.7, §7.2 L790; I24).
- R0 item 6: critic candidate open.

## 6. Corpus

- **Arm D:** all 112 tuning items (ids in `spec.yaml`).
- **Arm L:** EXP-CHUNK-003's item and file selection, cells {insert, overwrite} × {64, 4096}, version pairs.
- **Arm K:** tuning items with a regular file ≥ 1 MiB, using the first 16 MiB of the largest file. That prefix yields enough boundaries at `balanced` for the coincidence statistic; items where it gives < 8 boundaries are reported as `underpowered` for the chance-rate test, not passed.
  - All new keyed constructions run on all qualifying items.
  - The status-quo modes run on a 10-item positive/negative-control subset only, because the crypto domain owns the full HC-14 verdict for production modes. If the crypto domain's pre-registration does not include MVT-14(e) on all tuning and validation items for the status-quo modes, the design review extends this arm to all items (amendment).
- **Arm E:** all tuning items ≤ 512 MiB (encrypted pack holds the plan in memory).
- **HC-14 on validation:** the MVT is required on tuning and validation before freeze (objective §2.0 rule 3). `spec-mvt14e-validation.yaml` runs the same test on validation. It is registered in the look registry as an HC screen, not a confirmation look.
- **Confirmatory look for DEC-CRY-023 non-leakage metrics:** coordinated with the crypto domain's decision record, where W and S are declared.
- **Held-out:** Phase D placeholder (§5.5; EXP-CHUNK-004 section 6).

## 7. Environment and platform

- **WSL x86-64** for arms D, L, K, N.
- **Arm E** on WSL using the production CLI build (`ebound pack --recipient <file> --pad bucketed|none|max --chunk-boundary default|keyed-prf`). Test recipients come from `ebr-pack`'s generator; their public key files are written to scratch. Arm E test keys are throwaway research material, never real credentials.
- AES-NI presence is recorded (`/proc/cpuinfo` flags). ARM64 without AES acceleration is EXP-CHUNK-013 (BLOCKED).
- No privileges, network or external review for the measurements.

## 8. Commands

```sh
# preconditions/builds as EXP-CHUNK-004 section 8
$PY -m ebr run --spec research/experiments/EXP-CHUNK-006/spec.yaml --gzip     # arm D (+ verify-raw, normalize)
$PY research/experiments/EXP-CHUNK-006/make_specs.py --arms L,K,K-validation,E \
    --edit-items research/experiments/EXP-CHUNK-003/spec.yaml --out-dir research/experiments/EXP-CHUNK-006   # commit specs
#   arm K command: ebr-chunk mvt14e --item-path {item_path} --algorithm {algo} --boundary-mode {mode}
#                  --min 131072 --target 524288 --max 2097152 --prefix-bytes 16777216
#                  --key-pairs 1000 --key-seed-base 20260917 --null-quantile 0.999
#   arm E command: ebr-pack --item-path {item_path} --encrypt true --chunk-boundary {mode} --pad {pad}
#                  --profile {profile} --archive-out {scratch}/enc.eb ; ebr-inspect-bytes --encrypted true ...
$PY research/experiments/EXP-CHUNK-006/padding_model.py --arm-e research/normalized/EXP-CHUNK-006/arm-e \
    --scope-model research/experiments/EXP-CHUNK-007/scope_model.py --out research/normalized/EXP-CHUNK-006/decision/arm-n.csv
```

## 9. Seeds

- **Spec seeds:** `{order: 2026091761, bootstrap: 2026091762, command: 2026091763}`.
- **Research keys:** derived as SHA-256(`"EXP-CHUNK-006/key/v1" ‖ key_seed_base ‖ index`), expanded per construction. They are candidate-independent where constructions share a key type.
- **Arm K:** key pairs (2i, 2i+1) for i < 1000.
- **Arm E:** fresh production randomness (encrypted output is intentionally non-deterministic, HC-13). Its size metrics are therefore stochastic in padding-bucket placement only. Arm E uses 3 repetitions and reports the range; a byte difference between repetitions of the same candidate beyond what the declared padding function allows is an HC-14 (j) violation artifact, routed to the crypto domain.
- Warmups 0.

## 10. Metrics

| Metric | Canonical | OD | Role | Threshold |
|---|---|---|---|---|
| `dedup_stored_fraction` (mean over 16 keys; min/max secondary) | yes | OD-05 | diagnostic; primary for the keyed-dedup component (DEC-CRY-023 non-leakage part) | T-01 |
| `artifact_bytes` (arm E real; arm N modelled) | yes | OD-05 | **primary** (DEC-CRY-025; DEC-CRY-023 size) | T-01 |
| `padding_overhead_pp` (arm E; within one padding mode) | yes | OD-16 | **primary** for the padding-overhead part (DEC-CMP-017 encrypted overhead) | T-16 |
| MVT-14(e) outcome per construction and item | HC-14 binary | — | binary R1 | §3.3 |
| `new_unique_bytes_p95`, `preserved_boundary_fraction_median` (arm L) | no | — | secondary | none |
| `boundary_coincidence_with_reference`, `forced_max_fraction`, `mean_chunk_bytes`, `keyed_boundary_sets_distinct` | no | — | secondary; the last one is a sanity binary (must equal the key count) | none |
| `result_sha256`, `detail_sha256`, `algorithm_id` | no | — | repeat-hash, provenance | §3.3 |

## 11. Replication

- **Arms D, L, K:** 2 repetitions and repeat-hash; keys are seeded, so outputs are deterministic.
- **Arm E:** 3 repetitions (stochastic padding placement and fresh keys); size metrics use the median of 3, and a range above the declared padding function's permitted variation is flagged (section 9).
- No timing, so no adaptive rule.

## 12. Analysis

- **Arm D.**
  - **L1:** log ratio of key-mean `dedup_stored_fraction` versus `unkeyed-gear-norm-v1` at identical sizes. Between-key variation is reported as the within-item max/min ratio.
  - **L2-L4:** as EXP-CHUNK-004 section 12 (equal family weights until G-B).
  - **Verdicts:** informational (tuning).
  - **Keyed-feasibility table** for DEC-CMP-001: construction × {passes K, D inside band, L factor}.
- **Arm K.** Per item, the null distribution is the 1,000 distinct-key-pair coincidence fractions. The per-key keyed-versus-unkeyed coincidence is compared with the 0.999 quantile, and the count of keys above it (≤ 1 allowed). Identical boundary sets under any two distinct keys is an immediate violation. Output is one pass/fail row per (construction, item) plus the violation artifacts (key pair, boundary lists) committed under `research/raw/EXP-CHUNK-006/`.
- **Arm E.** `artifact_bytes` and `padding_overhead_pp` within each padding mode:
  - mode comparisons (secret table vs PHTE) at T-01;
  - padding comparisons across sizes at T-16, absolute threshold;
  - verification that `inspect` reports the padding mode and `pad none` as a declared leak (HC-14 fact 2, binary; also owned by the crypto domain).
- **Arm N.** The model's validity gate: applied to arm E's `archive` scope, the model must reproduce real encrypted `artifact_bytes` exactly for plan-independent chunks, or else within T-01 at item level on every item. Otherwise arm N is reported as `MODELLED, unvalidated` and cannot support a decision. Then no-dedup and coarse-rechunk deltas are reported at L1-L4.
- **Confirmatory looks** are declared by the owning decision records (DEC-CRY-023 with the crypto domain, since leakage is an OD there). This experiment supplies tuning S and W candidates for the dedup/size ODs and runs their validation arms when those records request them.

## 13. Thresholds

T-01 (`artifact_bytes`, `dedup_stored_fraction`) and T-16 (`padding_overhead_pp`) as transcribed in `research/methods/thresholds.json`. The MVT-14(e) criterion is the objective's exact pre-registered HC criterion (not a band), with parameters from `decision-method.md` Appendix C.1. None restated.

## 14. Sensitivity

- Key count 16 versus 64 (a 10-item subsample).
- Arm K prefix 16 MiB versus 64 MiB (10 items).
- Arm K null quantile computed per item versus pooled per family (reported only; the HC criterion stays per item).
- Arm N model with and without cohort/dictionary-coupled chunks.
- Profile `balanced` versus `dense` for arm D.
- F20 in versus out of aggregation.

## 15. Expected negative results

- PHTE costs nothing in dedup, so the default choice rests on throughput (EXP-CHUNK-008) and external leakage review alone.
- Keyed byte permutation does not rescue hashless CDC.
- The mechanical MVT-14(e) test cannot distinguish a strong PRF-keyed construction from a weak secret-table one (both pass), which confirms that HC-14's mechanical fact is not a leakage measure (objective HC-14 statement).
- Bucketed padding dominates dedup savings on small-file trees.

## 16. Threats to validity

- **Random tables stand in for HMAC-derived tables** (PRF premise). Mitigated by arm E's real production archives.
- **New keyed constructions are research designs without security review.** Passing MVT-14(e) is necessary, not sufficient (EXPERT_REVIEW_REQUIRED).
- **Arm E uses WSL ext4 capture:** metadata sizes differ from Windows (F-0001), equally across candidates.
- **Arm N is modelled:** gated as above.
- **The 16 MiB prefix in arm K** may miss periodic structure deeper in files.
- **Ownership overlap with the crypto domain** (HC-14 on production modes; OD-16): reconciled at the cross-domain index and review. Duplicate runs are acceptable; conflicting verdicts are a review finding.

## 17. Estimates

| Arm | Compute |
|---|---|
| D | 18 candidates × 36.1 GiB × 16 keys × 2 reps ≈ 20 TB of keyed chunking; PHTE at about 50-150 MB/s and gear at about 0.5-1 GB/s: about **60-120 CPU-hours** (PHTE dominates) |
| L | about 15 CPU-hours |
| K | ≤ 9 constructions × about 100 items × 2,000 chunkings × 16 MiB ≈ 29 TB: about **50-150 CPU-hours** (PHTE and Rabin dominate; status-quo modes only on 10 items) |
| E | 3 modes × 3 pad modes × 4 profiles × tuning ≤ 512 MiB (about 25 GiB) × 3 reps: about **60 CPU-hours** |
| N | minutes |

- **Disk:** detail files about 20 GB; arm K violation artifacts small.
- **Agent effort:** scripted; judgment for the new keyed constructions (literature search, port review), the HC-14 applicability write-up for `fixed-size`, and analysis.

## 18. Tooling to build

1. **`ebr-chunk` keyed modes:** `--boundary-mode {none, secret-gear-table, phte-aes128, keyed-table, keyed-polynomial, keyed-byte-permutation}`, `--key-count`, `--key-seed-base`, in `tree-dedup` and `edit-sweep`.
   - Production modes go through `entrybound::chunker::chunk_ranges_encrypted` with `EncryptedBoundaryKey` (public enum).
   - New modes are harness modules.
   - Payload `keys.*` aggregates as named in `spec.yaml`.
2. **`ebr-chunk mvt14e` subcommand:** the objective MVT-14(e) procedure with the Appendix C.1 parameters; violation artifact output.
3. **`ebr-pack` flags `--chunk-boundary default|keyed-prf` and `--pad bucketed|none|max`,** passed to production encrypted-pack options. The production API supports both, per `ebound pack --help`.
4. **`research/experiments/EXP-CHUNK-006/{make_specs.py, padding_model.py}`.** The padding function is implemented from `docs/crypto-suite-v1.md` (bucketed, maximum and none classes) and cross-checked against arm E.

## 19. Deviations

(append-only)
