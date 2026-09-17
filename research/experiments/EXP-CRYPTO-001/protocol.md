# EXP-CRYPTO-001: Observer model and CDC attack-suite validation against reference keyed-chunking schemes

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** |
| Kind | instrument (calibrates an instrument; informs no decision directly) |
| Domain | crypto (program §22.1, §22.2, §22.3, §22.4, §22.5) |
| Decisions informed | none (instrument) |
| Platforms | Linux/x86-64 |
| Requires timing | False |
| Estimates | 150 machine-hours; 50 GB disk |
| Tooling to build | research/tools/crypto/cdc_attacks/*; ebr-crypto attack + reference schemes |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns no validation look |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
<!-- END AUTO:meta -->

## 1. Question

Do the program's own implementations of the published content-defined-chunking (CDC) attacks reproduce the published key-recovery and file-fingerprinting outcomes against faithful targets (the real `borg` 1.2.8 and `restic` 0.16.4 binaries installed in WSL, plus reference re-implementations of the attacked keyed-chunking schemes), so that the suite and the observer model can be committed as the OD-16 attack suite that `decision-method.md` §14 item 16 requires before any OD-16 result?

This experiment calibrates an instrument. It produces no OD-16 result for an Entrybound candidate. Its outputs are the committed observer model, the attack registry and the pass/fail record of each attack against its positive and negative controls.

## 2. Hypotheses and falsification

- **H1 (positive controls).** Each attack that a primary source reports as successful against a scheme succeeds against the program's target for that scheme, within the work budget of §8, on at least 9 of 10 seeded keys at the published parameters. Where the published work exceeds the budget, it succeeds at scaled parameters, and the fitted slope of log2(work) against the scaled security parameter agrees with the published asymptotic order.
- **H2 (negative control).** Against `phte-aes128-norm-v1`, no attack in the suite recovers key material within the same budget, and the presence advantage of every boundary-exploiting attack is not greater than the advantage of the length-only reference attack A0 on the same observations (both at T-17 resolution).
- **H3 (null control).** On archives that do not contain the target file, every attack's false-positive rate at its tuned threshold is at most 0.01 on held-back negatives. Its measured presence advantage on a pure-null set is `EQUIVALENT` to 0 at the T-17 band.
- **Falsification.** H1 fails if any published successful attack fails against its faithful target (the suite is then a strawman for that attack class, and the attack is repaired and re-validated under a new attack version id before use). H2 fails if an attack recovers PHTE key material or beats A0 on PHTE. Because that would contradict the Truong et al. security claim, it is first checked for implementation bugs and then recorded as a finding for external review (EXP-CRYPTO-021 dossier). H3 fails if false-positive control fails, and the scoring rule is then repaired.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
This experiment informs no ledger decision directly (instrument or calibration). Its outputs are inputs to the decisions of the experiments that cite it; no candidate table applies.
<!-- END AUTO:candidates -->

Evidence route: instrument calibration (`EMPIRICALLY_MEASURED`, correctness-type) plus `EXTERNALLY_SOURCED` attack descriptions re-implemented from primary sources. The attack suite's adequacy is an `EXPERT_REVIEW_REQUIRED` item (objective OD-16 M16.2) and goes into the dossier (EXP-CRYPTO-021).

## 4. Arms: targets, attacks and observer model

### 4.1 Observer model (committed by this experiment as `observer-model.md`)

| Observer | Sees | Knows | Capability |
|---|---|---|---|
| O1 passive known-file | Every public byte of an encrypted archive or repository: public object and record lengths in file order, record class (CONTROL/PAYLOAD), segment and record counts, feature bits, envelope public fields | Public algorithms and profile parameters; a set of candidate files | Scores "file F is inside" |
| O2 known-plaintext | As O1, for archives that contain some files whose plaintext the observer knows | As O1 plus those plaintexts | Recovers boundary secrets, then fingerprints |
| O3 chosen-plaintext co-packer | As O1, across repeated archive creations | Chooses content packed next to the victim content | Adaptive queries |
| O4 adaptive | As O1/O2 | May tune a classifier on tuning-split archives made by the candidate under test | Learned scoring |
| O5 key-holding | As O1 | Holds the boundary key (for example a removed recipient under boundary-preserving rotation) | Computes keyed boundaries directly |

Remote access-pattern observation (range requests, M16.4) belongs to EXP-CRYPTO-008 and is outside this model.

### 4.2 Targets

| Target id | Scheme | Implementation |
|---|---|---|
| `tgt-borg-1.2.8-none` | Borg buzhash with secret 32-bit table, compression `none` | real `borg` 1.2.8 (apt), `repokey` encryption, repository segment files observed |
| `tgt-borg-1.2.8-lz4` | same, compression `lz4` | real `borg` |
| `tgt-borg-1.2.8-obfuscate` | same, with the `obfuscate` pseudo-compressor at the level documented for 1.2.8 | real `borg` |
| `tgt-restic-0.16.4` | Rabin fingerprint with a secret degree-53 polynomial (pre-0.18 mitigation) | real `restic` 0.16.4 (apt), local repository, pack files observed |
| `ref-tarsnap-like` | Tarsnap chunking as described in the primary source | research re-implementation (no Tarsnap service is contacted) |
| `ref-buzhash-secret-{8,16,32}` | Borg-style buzhash at scaled table width | research re-implementation, for scaling checks |
| `ref-rabin-secret-deg{24,32,40,53}` | restic-style Rabin at scaled degree | research re-implementation, for scaling checks |
| `eb-gear-norm-v1` | Entrybound public Gear CDC | `entrybound::chunker::chunk_ranges` |
| `eb-gear-secret-table-v1` | Entrybound SECRET_GEAR_TABLE | `entrybound::chunker::chunk_ranges_encrypted` with `EncryptedBoundaryKey::SecretGearTable` |
| `eb-phte-aes128-v1` | Entrybound PHTE_AES128 | `chunk_ranges_encrypted` with `EncryptedBoundaryKey::PhteAes128` |

Before use, each reference re-implementation is checked against its real counterpart where one exists: identical chunk boundaries for 100 seeded keys on 20 generated inputs. Tarsnap has no local counterpart, so its re-implementation is checked against the primary source's pseudocode by a second session.

### 4.3 Attack registry (`attack-suite.json`, versioned attack ids)

| Attack id | Class | Source (implementer opens the primary source and records section and page locators in the registry) |
|---|---|---|
| `A0-length-only-v1` | Matches the target's padded object-length multiset and sequence under the declared padding function | Reference observer of objective OD-16 and CC-04 |
| `A1-unkeyed-cdc-fingerprint-v1` | Computes public CDC boundaries of the target and matches the padded chunk-length subsequence | Standard, and the unkeyed baseline in both 2025 papers |
| `A2-apz-kpa-buzhash-v1`, `A2-apz-cpa-buzhash-v1`, `A2-apz-kpa-rabin-v1`, `A2-apz-cpa-tarsnap-v1` | Key recovery from chunk sizes, then fingerprinting | Alexeev, Percival and Zhang, "Chunking Attacks on File Backup Services using Content-Defined Chunking", 2025 (PDF at daemonology.net), each attack section |
| `A3-tmsgp-*-v1` | Each attack on keyed-CDC constructions in Truong et al., "Breaking and Fixing Content-Defined Chunking", ACM CCS 2025 (IACR ePrint 2025/558), one id per attack described | Paper, attack sections |
| `A4-adaptive-gear-lowbits-v1` | Constraint-solving key recovery for secret-Gear low bits (z3/SAT, or linear algebra over Z/2^k), tuned on tuning archives | Program-designed. Motivated by an `[INFERRED]` observation: the cut test of a Gear hash on its low k bits depends only on the last k input bytes and on the low bits of the table entries |
| `A4-adaptive-classifier-v1` | Gradient-boosted classifier over padded length sequences, tuned per candidate on tuning archives | Program-designed (objective M16.2 adaptive requirement) |

No attack is implemented from an agent's recollection (§9.3). The registry records, for every attack, the primary-source locator, the parameters and a SHA-256 of the source PDF as fetched.

## 5. Corpus selectors

- **Generated plaintexts:** `random-v1` and `text-v1` items of 1, 4, 16 and 64 MiB, seeds recorded in `spec.yaml`, used for key-recovery known plaintexts and for boundary equivalence checks.
- **Known-file universes (tuning split only):** file sets drawn from `f13-tuning-musopen-flac-medium`, `f13-tuning-librivox-audiobook-medium`, `f13-tuning-nasa-ntrs-pdf-small`, `f11-tuning-ubuntu-noble-debs`, `f11-tuning-pypi-binary-wheels-cp312`, `f09-ubuntu-noble-usr-binaries`, `f05-tuning-gutenberg-books`, `f07-tuning-pythia-160m-safetensors-main`. Files of at least 1 MiB are targets (multi-chunk). Smaller files feed A0 only.
- **Negative pools:** files of similar size class from other tuning items of the same family that are not in the target set, at least 1,000 per family.
- **Validation split:** not used. This is instrument calibration, so no validation look is spent. **Held-out:** not used (see the placeholder section).

## 6. Environment and platform requirements

- `wsl-ubuntu` (Linux x86-64, ext4 under `/root/eb-research`), `timing: false`. Attack work is reported as core-seconds, informational only.
- Real `borg` 1.2.8 and `restic` 0.16.4 at the versions in `research/baselines/tool-versions.json`. Before the run, the runner records `borg --version` and `restic version`, and it refuses to start on a mismatch.
- Entrybound targets are built from the harness workspace at a commit where `research/harness/lock_parity.py` prints `RESULT PASS`.
- No network access. Tarsnap is re-implemented, never contacted.

## 7. Commands and tooling

Tooling to build (none of it exists at this commit):

1. `research/tools/crypto/cdc_attacks/` (Python 3, numpy/scipy/pandas from `/root/eb-research/venv`; `z3-solver` and `lightgbm` pinned in `research/tools/crypto/requirements.txt`): `observe_repo.py` (extracts length observations from borg segments and restic packs), `a0_length_only.py`, `a1_unkeyed.py`, `a2_apz.py`, `a3_tmsgp.py`, `a4_adaptive.py`, `presence_eval.py`, and `attack-suite.json` (the registry).
2. Harness crate `research/harness/crates/ebr-crypto`, binary `ebr-crypto`, with subcommands:
   - `chunk-trace --scheme <target> --key-seed <n> --item-path <p>`: emits chunk lengths as JSON;
   - `kr-accel --scheme ... --work-budget ...`: Rust inner loops for key-recovery search.
   Reference schemes live in `ebr-crypto/src/refschemes/{buzhash,rabin,tarsnap}.rs`.
3. `research/experiments/EXP-CRYPTO-001/run_controls.sh`: builds the borg and restic repositories under `/root/eb-research/scratch/EXP-CRYPTO-001/`, one repository per seeded key, and drives the attacks.

Execution:

```sh
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && python3 research/harness/lock_parity.py && \
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CRYPTO-001/spec.yaml && \
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CRYPTO-001/spec.yaml && \
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-CRYPTO-001/spec.yaml && \
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-CRYPTO-001/spec.yaml && \
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr report --spec research/experiments/EXP-CRYPTO-001/spec.yaml'
```

Each spec candidate is one (target, attack) cell. The command runs `run_controls.sh --target {target} --attack {attack} --keys 10 --key-seed {seed}` and prints one JSON row (success fraction, work core-seconds, TPR, FPR, presence advantage).

## 8. Seeds, warmup, repetitions and adaptive rule

- Ten seeded keys per (target, parameter point). Key seeds are derived from `seeds.command` with a keyed stream (`sha256(seed || target || i)`). They are research test keys and never production randomness.
- Work budget per key-recovery attempt: 2^40 simple operations, or 24 core-hours, whichever comes first. If a published attack's predicted work exceeds the budget, scaled parameter points (for example Rabin degrees 24, 32 and 40) are run and the scaling slope is fitted.
- Warmups: none (no timing). Repetitions: each (target, key) cell runs once. Its deterministic outputs are then re-run once in a separate process and hash-checked (§4.13).
- Adaptive rule: none, because the counts are fixed in advance. Presence evaluation uses at least 300 positive targets and at least 1,000 negatives per family, and the A4 classifier is trained on a 50% seeded partition of the tuning targets and scored on the other half.

## 9. Measurement method and metrics

| Metric | Canonical name | Role here | Source |
|---|---|---|---|
| Presence-confirmation advantage at FPR 0.01 | `presence_advantage` (T-17) | Instrument output (not a decision metric in this experiment) | `presence_eval.py` |
| Key-recovery success fraction per (target, attack) | descriptive, not canonical | Pass criterion for H1 and H2 | attack scripts |
| Work to recovery | descriptive (core-seconds, simple-operation count) | Scaling check | attack scripts |
| False-positive rate at the tuned threshold | descriptive | H3 | `presence_eval.py` |
| Boundary equivalence of re-implementations against the real tools | binary (identical boundary sets) | Precondition | `chunk-trace` versus observed repositories |

## 10. Normalization and statistical analysis

- Success fractions get exact Clopper-Pearson 95% intervals. H1 passes when the lower bound is at least 0.5 and the point estimate is at least 0.9.
- TPR at FPR 0.01 uses a threshold set on the training half's negatives and applied to the scoring half. The presence advantage interval is a cluster bootstrap over independence groups (2,000 replicates, PCG64 seeded from `seeds.bootstrap`). This bootstrap is informational (§4.10: bootstrap only for informational intervals).
- The scaling fit is least squares of log2(work) on the parameter, with a 95% slope interval compared to the published order.
- ebr `normalize` verdicts are informational (§3.6). Pass/fail is computed by `research/tools/crypto/cdc_attacks/validate_suite.py`, which writes `research/normalized/EXP-CRYPTO-001/decision/suite-validation.json`.

## 11. Practical significance

H2 and H3 use the T-17 bands of `presence_advantage` from `research/methods/thresholds.json` (`metric_thresholds.presence_advantage`) exactly as registered. Nothing is restated here. H1 criteria are instrument acceptance criteria, not decision thresholds.

## 12. Sensitivity analysis

- Negative-pool composition: same-family versus cross-family negatives.
- Known-plaintext fraction for O2: 1%, 10% and 50% of repository bytes.
- Classifier seed: 5 seeds for A4.
- borg compression: `none`, `lz4` and `obfuscate`, reported separately. The Alexeev et al. claim that compression matters is itself checked.

## 13. Expected negative results worth recording

- An A2 attack fails against the real tool even though it succeeds against the reference re-implementation. This would reveal a re-implementation fidelity gap or undocumented tool behaviour.
- `obfuscate` or `lz4` defeats known-plaintext recovery on borg at realistic plaintext fractions.
- A4 recovers secret-Gear low bits faster than any published attack on the analogous schemes. This counts against the status-quo default of DEC-CRY-023.
- A4 finds no advantage over A0 on secret Gear once bucketed padding applies. This supports "defense in depth" wording.

## 14. Threats to validity

- **Strawman attacks:** mitigated by requiring reproduction on real tools, a second-session source check, and versioned attack ids.
- **Lower bound only:** a suite that fails to break a scheme is not evidence of security (objective M16.2). Such an outcome is recorded as "not broken by suite vX", never as "secure".
- **Scaled parameters:** extrapolation from reduced degrees may miss constant factors, so the report states the published point separately.
- **Shared base model:** the attack implementer and the Entrybound designers share a base model, which can produce correlated blind spots (objective §5 item 8). External review of suite adequacy is required.
- **Real-tool drift:** apt package upgrades are prevented by the version check.

## 15. Held-out placeholder (Phase D)

Not applicable. Instrument calibration uses tuning items and generated inputs only (decision-method.md §5.1). No held-out item is named or used.

## 16. Cost estimate and agent effort

- Compute: about 150 core-hours, dominated by restic polynomial and borg table recovery at published parameters. Disk: about 50 GB transient repositories under `/root/eb-research/scratch`, and under 1 GB retained raw.
- Agent effort: **judgment** to implement attacks from the primary sources (tooling stage) and to adjudicate H1 failures. Execution is **scripted**. The analysis is a short **judgment** pass over `suite-validation.json`.

<!-- BEGIN AUTO:common -->
**Shared provisions (binding; `research/experiments/_index/crypto-conventions.md`).**

- **Author and L7 exposure (CR1):** designed by the Phase C-design crypto session, which read `research/corpus/coverage.md` and `research/PROGRESS.md` under the shared design instructions and is recorded as L7-exposed for all families (`research/experiments/_program/l7-exposures-design-phase.jsonl`). Its candidate operationalizations, exclusions and analysis plans are re-signed by an unexposed session before G-A (PA-01); decision analyses are executed by unexposed sessions only.
- **Gates (CR0):** runs before G-A/G-B are `NOT_DECISION_GRADE`; specs with non-empty `decision_ids` launch only through `research/tools/experiments/run_guarded.py` (PA-13).
- **Candidates (CR2):** the tables above are generated; R0 item 6 is open until the crypto-cluster critic pass (PA-12).
- **Security claims (CR3):** attack, leakage and tamper results are lower bounds; selections resting on a security claim, sufficiency of a mitigation or acceptance of leakage are provisional until the EXP-CRYPTO-021 external review is received.
- **Metrics (CR6):** component throughput uses `__component_<name>` strata; chunk-stage throughput is owned by EXP-CHUNK-008 T1 (PA-17); HC oracle counts are binary under their MVT ids pending the PA-02 metric-registry disposition.
- **Timing (CR5):** affinity and guard per §4.2-§4.4 (lint-checked), staged helpers (PA-16), calibration identity and instrument-class calibration (PA-04, PA-15).
- **Split discipline (CR7):** committed specs are tuning-only or binary HC screens; graded looks use look specs derived at look time with candidates restricted to W ∪ S, registered by the owner in `research/experiments/_program/look-plan.csv` (PA-03, PA-09).
- **Held-out (CR8):** generated only by EXP-EVAL-012 at Commit A (PA-20).
- **Power (PA-06):** a banded comparison enters its full tuning run only after `research/tools/experiments/mde_feasibility.py` projects an MDE of at most 1 band unit on a tuning proxy.

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-001; edit the inputs, not this block._
<!-- END AUTO:common -->
