# EXP-CONF-005 — Conformance corpus schema, expectation taxonomy, release form and verifier interface

| Field | Value |
|---|---|
| Status | `NEEDS_TOOLING` (CC§2): schema converters, taxonomy crosswalk and rating kit, regeneration checker, `ebcc-verify` with adapter prototypes, golden-fixture diff harness, `qemu-user-static` and `aarch64` cross toolchain for the emulated arm |
| Domain / program sections | conformance / §29 (primary), §28 |
| Decision IDs informed | DEC-ECO-012, DEC-ECO-013, DEC-ECO-014, DEC-ECO-042, DEC-ECO-046, DEC-ECO-010 |
| Requirements | REQ-ECO-0014, REQ-ECO-0015, REQ-ECO-0016, REQ-ECO-0136 |
| HC oracles | HC-12 MVT-12(b) (mechanical traceability under a schema); proposal for the assigner: `hc_ids` HC-12, HC-16; `od_ids` OD-21, OD-26 |
| Decision type | not assigned (gate G-A); the taxonomy agreement arm is `SIMULATED`-style heuristic evidence (agents as raters) |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1 |
| Depends on | EXP-CONF-003 (statements), EXP-CONF-004 (corpus), EXP-CONF-001 (`ebir` as the non-Rust consumer), EXP-CONF-011 (legacy case slice for arms B and D) |

## 1. Question

Which corpus case schema lets rule-to-case traceability be computed mechanically without information loss; how
reproducibly do independent raters assign expectation classes under each candidate taxonomy, and which taxonomies
collapse safety-relevant distinctions; is the exported corpus byte-reproducible across hosts and architectures and
consumable without Entrybound's Rust code; and which verifier interface lets any implementation, including incumbents,
produce a machine-readable conformance report with verification scope?

## 2. Hypotheses and falsification

- **H1 (schema, DEC-ECO-012).** Under candidate `spec-clause-derived-ids` (and any candidate carrying explicit
  `spec_refs`) the rule-to-case coverage audit is computable mechanically for 100% of statements; under
  `status-quo-ecf-string-ids`, `malo-directory-class-layout` and `annotated-vector-text-format` it is not.
  *Falsified* for a schema by a mechanical audit result that differs from the reference audit of EXP-CONF-003 step 4.
- **H2 (taxonomy, DEC-ECO-013).** No native case receives `UNRESOLVED` from either rater under the five-class taxonomy,
  and collapsing taxonomies (`malo-four-classes`, `three-classes-research-iii`, `binary-accept-reject`) merge at least
  one `FORBIDDEN` legacy case with an accepted class. *Falsified* respectively by one native `UNRESOLVED` rating
  confirmed by both raters, or by zero merges over the rated sample.
- **H3 (release form, DEC-ECO-042).** `ebcc-gen` regeneration is byte-identical for every case across two runs on each of
  WSL x86-64, the Windows host and QEMU-user emulated linux/arm64. *Falsified* by one differing case SHA-256.
- **H4 (consumption).** A non-Rust reader (`ebir`) runs the exported corpus from bytes and JSON manifest alone, with no
  generator execution for cases ≤ 256 MiB. *Falsified* by one case that requires Rust code to consume.
- **H5 (verifier interface, DEC-ECO-014).** The subprocess adapter protocol expresses outcome class, reason code,
  verification scope and `PROFILE_DEPENDENT` verdicts for production, `ebir`, and two incumbent legacy readers.
  *Falsified* by one required field the protocol cannot carry for one implementation.

## 3. Arms

### A. Schema traceability (DEC-ECO-012)

Converters `research/conformance/schema/convert_<candidate>.py` render `ebcc-v1/manifest.json` into each ledger
candidate: `status-quo-ecf-string-ids`, `research-iii-manifest-schema`, `wycheproof-style-json-schema`,
`malo-directory-class-layout`, `spec-clause-derived-ids`, `annotated-vector-text-format`. For each: (i) run
`audit_traceability.py --schema <candidate>` that uses only the information present in that rendering to compute the
rule-to-case coverage; compare with EXP-CONF-003's reference audit; (ii) count case fields lost in the rendering
(`info_loss.py`); (iii) validator LoC for the schema (JSON Schema or parser, `tokei`), descriptive.

### B. Taxonomy rating (DEC-ECO-013)

- Sample: 300 cases drawn with seed `2809170005`: 200 native cases stratified by `ebcc-v1` family (proportional,
  minimum 5 per family) and 100 legacy adapter cases from EXP-CONF-011 stratified by format.
- Raters R1 and R2 (CC§11: neither authored or reviewed those expectations) rate each case under
  `five-classes-with-profile-binding` (class, and for `PROFILE_DEPENDENT` the profile and per-profile outcome), from
  the case's bytes description, property and cited statements only (not production outcomes).
- Other taxonomies are derived by a crosswalk committed before rating (`taxonomy-crosswalk-v1.json`): five → malo four
  (`REQUIRED`→accept, `FORBIDDEN`→reject or malicious by the case's `security` flag, `ACCEPTABLE`→iffy,
  `PROFILE_DEPENDENT`→iffy, `UNRESOLVED`→iffy), five → R3 three, five → binary; where the crosswalk is not a function of
  the stored fields, the rater records the choice in the same pass.
- Outputs: per taxonomy Cohen's kappa and raw agreement between R1 and R2; count of native cases rated `UNRESOLVED`;
  count of cases whose `FORBIDDEN` class merges into an accepted class (the SPEC §19.3 registry failure mode);
  count of `PROFILE_DEPENDENT` cases whose per-profile outcomes disagree between raters.

### C. Release form and regeneration determinism (DEC-ECO-042)

- `research/conformance/tools/regen_check.py --gen <ebcc-gen> --seed 2809170401 --out <dir> --compare research/conformance/corpus/ebcc-v1/manifest.json`
  regenerates the full corpus and compares every case SHA-256 (hash-locked large cases materialized and hashed).
- Hosts: WSL x86-64 (two runs), Windows host x86-64 (two runs, `ebcc-gen.exe`, same toolchain `1.98.1`), linux/arm64
  under `qemu-aarch64-static` in WSL (package `qemu-user-static`, cross build `--target aarch64-unknown-linux-gnu` with
  `gcc-aarch64-linux-gnu`; label `EMULATED`; two runs). Docker linux/amd64 and native ARM64 are in EXP-CONF-014.
- Consumption test (H4): `ebir conformance --corpus <exported dir> --manifest manifest.json` on a machine path with no
  Rust toolchain on `PATH` (a clean user account environment in WSL: `env -i`), counting cases that require generator
  execution. Size and churn: bytes per family; for `ebcc-v1` → `ebcc-v1.1`, added, removed and re-expected case counts.

### D. Verifier interface prototypes (DEC-ECO-014)

- `ebcc-verify` implements the candidate interfaces: `data-only-no-runner` (baseline: consumers read the manifest),
  `subprocess-adapter-protocol` (JSON lines on stdin/stdout: request `{case_id, path, policy, secrets}` → response
  `{outcome_class, reason_code, unverified, scope, digests, profile}`), `in-process-rust-trait-harness` (production
  only), `ci-binary-with-junit-output` (JUnit XML plus JSON), `report-json-schema-with-scope` (report schema
  `ebcc.report.v1` with per-case verdict and verification scope).
- Adapters: production (`ebound` CLI), `ebir`, and two incumbent legacy readers on a 100-case legacy slice from
  EXP-CONF-011: `bsdtar 3.7.2` and Python 3.12.3 `zipfile`/`tarfile` (adapter-corpus tier, where `UNRESOLVED` and
  `PROFILE_DEPENDENT` occur).
- Measures: adapter LoC per implementation and interface (descriptive); for each interface, binary capability checks
  (can express scope; can express profile-bound verdicts; report validates against `ebcc.report.v1`); per-case
  overhead (informational only, `NOT_DECISION_GRADE`).

### E. Observed-outcome matrix maintenance cost (DEC-ECO-046)

Wall time and diff size of regenerating the native and legacy observed-outcome matrices from arms C-D and EXP-CONF-011
outputs (informational; supports the governance candidates, never selects).

### F. Golden fixtures for machine-readable output (DEC-ECO-010, partial)

`golden_diff.py` records `ebound inspect --json` and `ebound verify --json` outputs for every valid `ebcc-v1` case under
`ebound-baseline` and `ebound-head`, canonicalizes with RFC 8785 (pinned `json-canonicalization` reference, EXP-CONF-012
V8), and reports schema field additions, removals, type changes and value-semantics changes per report identifier
(`entrybound/inspection-v1`, `entrybound/archive-diff-v1`). Human-facing parts of DEC-ECO-010 (consumer survey) are not
addressed (listed uncovered in `conformance-uncovered.jsonl`).

## 4. Candidates

The ledger candidates of DEC-ECO-012, DEC-ECO-013, DEC-ECO-014 and DEC-ECO-042 as enumerated in arms A-D (CC§1: none
added). Not executed, with reasons: DEC-ECO-042 `separate-corpus-repository` and `corpus-embedded-in-spec-release`
are governance and packaging choices without a measurable difference here beyond arm C's churn counts (recorded as
analytical); DEC-ECO-014 `ci-binary-with-junit-output` in container form needs Docker (EXP-CONF-014).

## 5. Inputs

`ebcc-v1` manifest and bytes; EXP-CONF-003 statements and reference audit; EXP-CONF-011 legacy case manifest (tuning
cases only for arm B and D); no corpus split items.

## 6. Environment

WSL2 Ubuntu 24.04 x86-64; Windows host (non-admin) for arm C; QEMU user-mode linux/arm64 in WSL (`EMULATED`). No timing.
Network once for `qemu-user-static`, `gcc-aarch64-linux-gnu` (apt) and the Rust `aarch64-unknown-linux-gnu` target.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CONF-005/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-005/spec.yaml           # arm C (WSL x86-64, arm64 emulated); raw id EXP-CONF-005
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-005/spec-adapters.yaml  # arm D; raw id EXP-CONF-005-ADAPT
# Windows host (PowerShell 7), ebr runner from the repository with C:/Python313/python.exe:
#   python -m ebr run --spec research/experiments/EXP-CONF-005/spec-windows.yaml                         # arm C (Windows); raw id EXP-CONF-005-WIN
python3 research/conformance/schema/run_arm_a.py --manifest research/conformance/corpus/ebcc-v1/manifest.json
python3 research/conformance/taxonomy/draw_sample.py --seed 2809170005 --n-native 200 --n-legacy 100
python3 research/conformance/taxonomy/score_ratings.py --r1 ratings-r1.jsonl --r2 ratings-r2.jsonl --crosswalk taxonomy-crosswalk-v1.json
python3 research/conformance/tools/golden_diff.py --baseline /root/eb-research/target/conf-baseline/release/ebound --head /root/eb-research/target/conf-head/release/ebound
```

## 8. Seeds, warmup, replication

Rating sample seed `2809170005`; generation seed `2809170401`. Regeneration: two runs per host (§4.13 repeat-hash).
Ratings: two raters. No warmup.

## 9. Metrics (CC§9)

| Metric | Role | Unit | Arm |
|---|---|---|---|
| `normative_rule_coverage_fraction` computed per schema rendering | binary (HC-12) | fraction | A |
| Regenerated cases differing in SHA-256 per host (H3) | binary by the determinism rule (§4.13) | count | C |
| Cases requiring generator execution to consume (H4) | descriptive, non-canonical | count | C |
| Kappa and agreement per taxonomy; native `UNRESOLVED` ratings; `FORBIDDEN`-to-accepted merges; profile disagreements | descriptive, heuristic (`SIMULATED` raters) | count, kappa | B |
| Interface capability checks; adapter LoC; report schema validity | descriptive, non-canonical | binary, LoC | D |
| Field changes between baseline and head report schemas | descriptive | count | F |

## 10. Normalization and statistical analysis

Deterministic counts (AGG-P). Kappa is reported with its bootstrap 95% interval (2,000 resamples, seed
`2809170005`), descriptive only. No verdict is CI-based. Human-facing claims (which schema or taxonomy people prefer) are
not made (§4.16); the arms measure mechanical properties. The analysis plan needs adoption by a non-exposed session
(CC§1).

## 11. Practical-significance thresholds

§3.3 for the binary coverage and determinism checks; no band for descriptive and heuristic quantities.

## 12. Sensitivity analysis

- Arm B: crosswalk alternatives for `ACCEPTABLE` (→ accept instead of iffy) and `PROFILE_DEPENDENT` (→ reject); native
  and legacy strata reported separately.
- Arm C: regeneration with `--jobs 1` versus all cores (thread-count sensitivity of the generator).
- Arm D: adapters written by a second session for one implementation (adapter LoC variability).

## 13. Split discipline and held-out placeholder

No corpus split is used; legacy cases come from the tuning instantiation of EXP-CONF-011. No held-out placeholder.

## 14. Expected negative results worth recording

- Taxonomies that merge `FORBIDDEN` into accepted classes (the SPEC §19.3 failure mode).
- Low rater agreement on `ACCEPTABLE` versus `PROFILE_DEPENDENT` for legacy cases.
- Generator nondeterminism across hosts (path separators, map iteration order, thread count).
- Hash-locked large cases that inert-data consumers cannot run without the generator.

## 15. Threats to validity

- Raters are agents sharing a base model (CC§11); agreement overstates human agreement; labelled `SIMULATED`.
- Converter quality determines arm A; converters are reviewed against each candidate's published format description.
- QEMU user-mode emulates the CPU, not a native ARM64 host; `EMULATED` evidence covers determinism only.
- Incumbent adapters in arm D exercise the interface, not the incumbents' conformance.

## 16. Cost estimates

| Item | Estimate |
|---|---|
| Compute | 12 CPU-hours (regeneration: WSL 2 × 1 h, Windows 2 × 1 h, arm64 emulated 2 × 3 h; arms A, D, F ≈ 2 h) |
| Disk | 15 GB (regenerated corpora per host, transient large cases) |
| Agent effort | **judgment, medium**: two rating sessions (300 cases), converter and adapter review; the rest scripted |
| Platform | Linux x86-64 (WSL), Windows x86-64 host, linux/arm64 emulated (QEMU user-mode); no timing |

## 17. Outcome obligations

`outcome.json` (§10.1); the selected schema and taxonomy remain decisions of the ledger; this experiment's outputs are
their evidence and counterevidence.
