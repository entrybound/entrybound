# EXP-EVAL-002: Whole-system deterministic comparison, REF-PROD against capability-matched specialists (size, fixed overhead, determinism, round-trip, single-file self-containment)

| Field | Value |
|---|---|
| Domain | evaluation (program sections 6, 7, 34, 35) |
| Status | NEEDS_TOOLING (gates G-A and G-B of `decision-method.md` §0.4; wrapper scripts; REF-PROD build recipe) |
| Kind | Decision-relevant (EMPIRICAL parts of claim decisions). Deterministic metrics only; timing is EXP-EVAL-005 |
| Method | `research/decision-method.md` and `research/methods/thresholds.json` at `14b977c` |
| `record.md` (§12.2) | Written and committed after G-A. The decision type, `od_ids` and `hc_ids` for each decision below are assigned by an independent session (§1.2, §14 item 12), not by this designer. |

## Author, independence and exposure

Designed by the Phase C-design evaluation-domain session. It designs no candidates: REF-PROD is fixed by objective §3.0, and specialists are fixed by objective §1.3. The **L7 exposure** is recorded in `research/experiments/EXP-EVAL-001/protocol.md` ("Author, independence and exposure") and covers families F01, F04, F07, F08, F11-F17, F19 and F20. Consequence: the tuning analysis, the validation analysis and the MDE computation are run by a session with no recorded exposure (§5.4 item 3).

## Question

On tuning, and then in one confirmatory validation look, how large is the whole Entrybound system's `artifact_bytes`, relative to the named specialist incumbent for each capability combination? The system is `ebound pack` at the production research baseline, in every profile and layout, plus the encrypted and retrofit arms. The comparison runs over every applicable family and scale tier. The same run also checks round-trip exactness, repeat-hash determinism, single-file self-containment and small-tier fixed overhead, so that the size claims of SPEC §22-§23 can be kept, qualified or dropped.

## Hypotheses and falsification

The predicted effects are INFERRED from SPEC §23.4 and serve only to pre-declare the superiority metrics. Band units are those of T-01, `ln(1.01)`.

- **H1 (CC-01, declared cost).** `dense` and `extreme` INDEXED are NON_INFERIOR in `artifact_bytes` to the per-item better of `7z-mx9-solid` and tar.xz, at the CC-01 declared-cost tolerance of 1.05 (objective §1.3), at corpus level. Falsified if the corpus-level two-sided 0.99 interval's upper bound exceeds `ln(1.05)`, or if the family harm guard fails in any applicable family. The outcome is then `NOT_SERVED` for CC-01's size part, and the density marks become profile-qualified or partial (DEC-ECO-070).
- **H2 (non-solid specialist).** `balanced` INDEXED is `DISTINGUISHABLE_BETTER` than `7z-mx9-nosolid` on `artifact_bytes`, with a predicted size of 3-6 bands, on F01-F06 and F17-F18 (deduplication and cross-file grouping). H0: `EQUIVALENT` or worse. Falsified by a not-better corpus verdict. A falsification is recorded as a negative result against the "Exceed" random-access-plus-density mark.
- **H3 (CC-03, dedup).** On F17 and F18, `dense` is NON_INFERIOR to the per-item best of `tarzstd-22-ultra-long27-t1` and `7z-mx9-solid`, at band. The `zstd --patch-from` series arm and `dwarfs-l7` are reported as explicit loss workloads (DEC-ECO-031 candidate `explicit-loss-workloads`). H3 is falsified if the family-level point estimate lies more than 1 band beyond the band edge.
- **H4 (fixed overhead).** On the small tier, `fixed_overhead_bytes` of `balanced` is at most 512 B on every item. That is the CC declared-cost tolerance for "a few hundred bytes" (objective §1.3; T-21 minimum-gain cap). Falsified by any item above 512 B, and the worst item is reported.
- **H5 (retrofit).** The native `.eb` of `balanced` INDEXED is `DISTINGUISHABLE_BETTER` on `artifact_bytes` than the retrofit, where the retrofit is `tarzstd-19-t1` plus its `ebound sidecar`. H0: `EQUIVALENT`. This is size evidence for DEC-CON-022. Capability differences (random access, verification) are reported as a separate row, never folded into the size ratio.
- **Binary facts (R1-level, no statistics).** Checked on every sample:
  - `roundtrip_mismatches = 0` for every REF-PROD arm;
  - unencrypted REF-PROD arms are repeat-hash deterministic (§4.13; a mismatch is the violation artifact under objective §2.0 rule 2);
  - `single_file_self_contained = 1` (MVT-09(c): verify and unpack succeed on an isolated copy with no network and no sidecar);
  - encrypted arms are never byte-identical across repetitions (HC-14: no deterministic ciphertext).

## Decisions informed

`DEC-CMP-011`, `DEC-ECO-070`, `DEC-ECO-031`, `DEC-ECO-069`, `DEC-CON-022`, `DEC-ECO-043` (measured-benefit evidence against the displacement base rate), `DEC-CRY-042` (encryption size overhead row only; the tamper and privacy rows are EXP-EVAL-006).

## Candidates

| candidate_id | Definition | Applicable families |
|---|---|---|
| `refprod-{fast,balanced,dense,extreme}-indexed` | `ebound pack {item_path} {scratch}/artifact.eb --profile P --layout indexed`. `ebound` is built from the production workspace at research baseline `9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155` with `cargo +1.98.1 build --release --locked -p entrybound-cli`, default features, `CARGO_TARGET_DIR=/root/eb-research/target/refprod`; the binary SHA-256 is recorded in each run's meta (harness review use constraint 2) | F01-F20 |
| `refprod-{fast,balanced,dense,extreme}-stream` | as above, `--layout stream --stream-window auto` | F01-F20 |
| `refprod-balanced-indexed-enc-{bucketed,none,max}` | as `balanced` INDEXED, plus `--recipient {scratch}/recipient.pub --pad {pad}` with a fresh X-Wing recipient per sample (`ebound key` generation step in the wrapper). Byte comparisons are made only within one padding mode (objective M05 note) | F01-F20 |
| `retrofit-tarzstd19-sidecar` | `tarzstd-19-t1` artifact (EXP-EVAL-001 command) plus `ebound sidecar artifact.tar.zst sidecar.eb --strict`; `artifact_bytes` = sum of both files | F01-F19 |
| `concat-stream-min` (reference for T-21 only) | minimum over zstd 3, zstd 19, xz 6, xz 9e, brotli q11 w24 and gzip 9 of the item's file contents concatenated in manifest order (objective M05.3) | small tier |
| `zstd-patch-from-series` | `research/baselines/patch_from.py`: first version compressed with `zstd -19 --long=27`, each later version as `zstd -19 --patch-from=<previous version tar> --long=27`; `artifact_bytes` = sum | F18 versioned series, F01 full-history clones (tag snapshots) |
| **REF-INC specialists** (not re-run: joined from EXP-EVAL-001 `best_incumbent_per_item.csv` by item) | CC-01 ratio: per-item better of `7z-mx9-solid` and `tarxz-9e` (`tarxz-6` where `tarxz-9e` was capped); CC-01 random-access partner: `7z-mx9-nosolid`; CC-03: better of `tarzstd-22-ultra-long27-t1` and `7z-mx9-solid`; defaults-versus-defaults: `targz-gzip-6`, `zip-6`, `7z-mx5-solid`, `tarzstd-3-t1`; upper bound (report only): `zpaq-m5` | per config |

- **Reference candidate:** `refprod-balanced-indexed` (REF-PROD, objective §3.0).
- **Status quo:** the REF-PROD arms are the status quo.
- **Frozen-design rerun:** at Commit A the same spec is re-run on validation with `ebound` built at the design-freeze commit (candidate ids suffixed `-frozen`). That run is the pre-unlock record EXP-EVAL-012 compares against.
- **Excluded before execution:**
  - `--password` arms: the Argon2id recipient needs an interactive or password-file path that the CLI does not script (`ebound pack --password`), and the password versus recipient envelope size is a crypto-domain question.
  - Encrypted STREAM: unsupported by the build ("Encrypted STREAM is unsupported").
  - Harness `ebr-pack` as the REF-PROD producer: not excluded for bytes (`harness-cli-parity.sh` 8/8 identical), but production `ebound` is used so that the identical binary serves EXP-EVAL-005.
- **Pre-registered refusal handling.** Items that `ebound pack` refuses (for example non-UTF-8 names, `EB_EAM_INVALID_PATH_COMPONENT`, or an ACL mask mismatch, `EB_EAM_INVALID_ACL`) are **not** dropped from the incumbent side (fair-comparison rule 1). Each refusal is recorded with its reason code and counted per family as `refused_items`. Ratio comparisons use the intersection of items both sides completed, and the refused share of logical bytes is reported beside every ratio.

## Corpus selectors

- **Tuning look.** `splits: [tuning]`, all 112 tuning items. The spec's shard specs are generated by `research/experiments/EXP-EVAL-002/make_specs.py` (to be written; same sharding rule as EXP-EVAL-001).
- **Confirmatory look.** `splits: [validation]`, all 91 validation items: one look, registered in `research/decisions/validation-looks.jsonl` before execution, after the MDE gate of §4.12 passes on tuning variance.
- `spec.yaml` is the tuning pilot shard over the small tuning items; the generator emits the rest.
- **Families.** F01-F20. **Tiers.** small, medium, large. **Evaluation conditions.** Padding mode, for the encrypted arms only.
- **Minimum support (§4.9).** The tuning group counts per family ({2:6, 3:6, 4:5, 5:2, 6:1}) and validation counts ({1:1, 2:9, 3:9, 4:1}) meet 10 groups across at least 5 families. The single-group validation family is labelled `single-group`.
- **Family weights.** `research/methods/workload-mix.json` at its Commit A SHA-256 (gate G-B). Equal weights are the sensitivity arm.

## Environment

`wsl-ubuntu` (ext4, `/root/eb-research`). Determinism across hosts is not claimed here; EXP-DET-SMOKE and the platform domain own it. Timing is off.

## Commands

```sh
# build REF-PROD (once; binary sha256 recorded)
wsl.exe -d Ubuntu -- bash -lc 'rm -rf /root/eb-research/src/refprod && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/refprod && cd /root/eb-research/src/refprod && git checkout --detach 9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155 && CARGO_TARGET_DIR=/root/eb-research/target/refprod /root/.cargo/bin/cargo +1.98.1 build --release --locked -p entrybound-cli && sha256sum /root/eb-research/target/refprod/release/ebound'
# preconditions (harness review use constraint 1)
C:/Python313/python.exe research/harness/lock_parity.py
wsl.exe -d Ubuntu -- bash research/harness/harness-cli-parity.sh
# spec validation, then per shard:
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-EVAL-002/spec.yaml'
#   ebr run --gzip --spec <shard>; ebr verify-raw --refingerprint --spec <shard>; ebr normalize --spec <shard>
# decision analysis (research/tools/decide/, decision-method.md §14 item 2 -- TO BE WRITTEN):
#   python -m decide analyze --experiment EXP-EVAL-002 --split tuning --reference refprod-balanced-indexed \
#       --join-incumbents research/normalized/EXP-EVAL-001/merged/best_incumbent_per_item.csv \
#       --workload-mix research/methods/workload-mix.json --out research/normalized/EXP-EVAL-002/decision/
```

Scripts **to be written**:
- `research/experiments/EXP-EVAL-002/refprod_sample.sh`: pack, then `ebr-inspect-bytes` accounting, `ebound verify`, `ebound unpack` into `{scratch}/extracted`, a content fingerprint comparison using `research/baselines/probes/roundtrip_check.py`'s fingerprint definitions, and the isolated-copy self-containment check (copy only `artifact.eb` into an empty directory, run `unshare -n` verify and unpack). It prints one JSON line with keys `artifact_bytes`, `metadata_bytes`, `index_bytes`, `dictionary_bytes`, `side_data_bytes`, `named_bucket_total_bytes`, `roundtrip_ok`, `verify_outcome`, `self_contained_ok`, `refusal_reason_code`.
- `research/experiments/EXP-EVAL-002/retrofit_sample.sh`
- `research/baselines/probes/concat_stream_min.py`
- `research/baselines/patch_from.py`
- `research/experiments/EXP-EVAL-002/make_specs.py`

## Seeds

`seeds: {order: 20260918, bootstrap: 20260918, command: 20260918}`. X-Wing recipient keys are fresh per sample by design; the key seed is not controlled, and non-determinism of ciphertext is the expected property. The selection-stability bootstrap uses seed 20260919.

## Warmup

0 (deterministic metrics, §4.5 not applicable).

## Measurement method and metrics

| Metric (canonical) | OD / HC | Role | Family | Unit | Threshold ID | Source |
|---|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | primary (the single superiority metric) | size | B | T-01 | `path_size` / wrapper |
| `fixed_overhead_bytes` | OD-05 (M05.3) | primary on small tier for H4 | size | B | T-21 | derived: REF-PROD `artifact_bytes` minus `concat-stream-min` |
| `metadata_bytes`, `index_bytes`, `dictionary_bytes`, `side_data_bytes` | OD-05 (M05.2) | diagnostic | size | B | T-02 | `ebr-inspect-bytes` named buckets; must sum to `artifact_bytes` or the sample is invalid (§4.13) |
| `total_stored_bytes` | OD-05 (M05.5) | sensitivity arm | size | B | T-01 | ratio of totals |
| `dedup_stored_fraction` | OD-05 (M05.4) | diagnostic (F17, F18) | size | ratio | T-01 | inspect chunk table |
| `padding_overhead_pp` | OD-16 (M16.3) | secondary, reported within mode | other | pp | T-16 | encrypted arm bytes minus unpadded plaintext envelope |
| `roundtrip_mismatches` | HC-02 | binary | correctness | count | §3.3 | wrapper `roundtrip_ok` |
| repeat-hash equality | HC-13 | binary where determinism is claimed | correctness | - | §4.13 | `path_sha256` of `artifact.eb` |
| `single_file_self_contained` | HC-09 / OD-26 (M26.5) | binary | correctness | binary | §3.3 | wrapper `self_contained_ok` |

- **At most one primary metric per OD** (R3): OD-05 uses `artifact_bytes` on the medium and large tiers and for the corpus verdict; `fixed_overhead_bytes` is the small-tier primary. The two do not overlap by tier, which is why T-01's small tier is relative-only (T-01 justification). A reviewer not involved in the decision signs this pairing off in `record.md` (R3 second-metric rule).
- **Correctness gating** (objective §3.0): a sample with `roundtrip_ok = 0` or an unbalanced byte accounting is excluded from OD analysis and reported as an HC violation.

## Replication and adaptive rule

2 repetitions (§4.6 deterministic), plus the repeat-hash check. No adaptive extension. A determinism mismatch on an unencrypted REF-PROD arm is an R1 artifact without a confirming rerun (§4.13). Both outputs are kept (`--keep-scratch` rerun of that one cell to capture the bytes) and committed under `research/raw/EXP-EVAL-002/violations/`.

## Normalization

`ebr normalize`, then the decision tooling:
- item level: the exact value (L1);
- group level: mean of item log ratios (L2);
- family level: mean over groups (L3);
- corpus level: `workload-mix.json` weights (L4).

The small tier uses relative bands only for `artifact_bytes` (T-01). Absolute floors are applied at item level (§3.1).

## Statistical analysis

- **Tuning (exploratory).** Descriptive intervals. The superiority and non-inferiority declarations for the validation look are fixed from tuning. They are recorded in `record.md` before the look: H2 and H5 are superiority claims (`k_sup = 1` per pair); H1 and H3 are non-inferiority claims at the stated tolerances.
- **MDE gate (§4.12).** Per comparison, MDE is computed from tuning group-level variance and the validation group structure. Every MDE must be at most 1 band unit before the look. Otherwise the look is not taken, and the decision stays `INSUFFICIENT_EVIDENCE` with the MDE recorded.
- **Validation (confirmatory).**
  - Corpus-level Welch-Satterthwaite intervals (§4.10): superiority at 1 - 0.01/k_sup; non-inferiority and equivalence at two-sided 0.99.
  - Family harm guard: pooled interval at two-sided 0.90, plus the 1-band point bound (§4.9).
  - Tier and padding-mode strata are reported separately and never averaged.
- **CC outcome rows** (objective §1.3). `SERVED`, `SERVED_WITH_DECLARED_COST` or `NOT_SERVED` for the size parts of CC-01 and CC-03, and the small-tier overhead part of every CC. Each row is a report, never a selection (§2.0).
- **Program level.** Every decision-supporting comparison is listed for the Benjamini-Yekutieli adjustment in Phase E (§4.12).

## Practical-significance thresholds

As registered, with no restatement:
- `artifact_bytes`: T-01 (`thresholds.json` `metric_thresholds.artifact_bytes`);
- `fixed_overhead_bytes`: T-21;
- metadata components: T-02;
- `padding_overhead_pp`: T-16.

Declared-cost tolerances come from objective §1.3 (CC-01 ratio 1.05; small-tier overhead 512 B). Binary metrics follow §3.3.

## Sensitivity analysis (§6, applied to the claim verdicts)

- equal-weight arm;
- leave-one-family-out and leave-three-families-out;
- family Dirichlet (10,000 samples, concentration 4.0);
- WM-* mixes (Appendix B.2);
- tier-stratified;
- byte-weighted ratio of totals (`total_stored_bytes`);
- selection-stability bootstrap (2,000 group resamples, seed 20260919);
- threshold perturbation x0.5 and x2 on T-01 and T-21, where the resolvability condition does not apply because the metrics are deterministic;
- REF-PROD versus REF-INC reference arm (§6.5).

A claim verdict that flips under any arm is reported as scope-limited, with the arm named.

## Expected negative results worth recording

- `extreme` loses to `zpaq-m5` (upper bound) and to `7z-mx9-solid` on text-heavy families (F05, F06).
- `dwarfs-l7` and `zstd --patch-from` beat `dense` on F17 and F18 duplicate or versioned trees.
- `fast` loses to `tarzstd-3-t1` on already-compressed families (F11-F13) because of framing overhead.
- Small-tier fixed overhead above 512 B on many-small-file items (F04).
- Whole-tree refusals on F19 and F20 items where every incumbent completes.

Every such outcome goes into the negative-results register (§10.1) and the final evaluation's section on workloads where Entrybound is not best (EXP-EVAL-012).

## Threats to validity

- **REF-PROD is the research baseline, not the frozen design.** Claims are re-evaluated on the frozen build at Commit A and on held-out (EXP-EVAL-012).
- **Specialist choice.** The objective §1.3 specialists were fixed before results. Per-item "better of" selection favours incumbents (conservative for Entrybound).
- **Retrofit fairness.** The sidecar's `--strict` mode may refuse some legacy artifacts. Refusals are reported, not dropped.
- **Version skew.** Apt incumbent versions are pinned (EXP-EVAL-001).
- **Corpus overlap.** Cross-split covariates (F03 shared packages, F17 kernel headers; corpus critique round 2) are reported, and a with/without arm excludes the flagged items.
- **Public-benchmark contamination.** Silesia, enwik8 and Kodak items are tagged. A with/without arm runs for L5 hygiene even on validation.

## Platform requirements

Linux x86-64 (WSL2), root for `unshare -n`, no network, large storage.

## Estimates

- Machine time: about 180 CPU-sequential hours (tuning and validation; `extreme` dominates on large items); about 45 h wall with 4 shards.
- Disk: about 80 GiB peak scratch (archive, extraction and isolated copy on large items).
- Agent effort: scripted execution; judgment for the analysis (an unexposed analysis session) and for the independent R3 sign-off.
