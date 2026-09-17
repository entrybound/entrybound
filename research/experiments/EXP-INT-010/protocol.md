# EXP-INT-010: External parity: overhead versus recovery probability, verified repair and in-format comparison

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (`ebr-int-parity` wrapper binary for RaptorQ and Reed-Solomon research formats, chunk-aligned in-format parity prototype, par2cmdline-turbo and par3cmdline builds, damage driver reusing T2, repair-verification driver, analytic recovery model) |
| Domain | integrity (program §24, cross-reference §30, §31, §32; objective OD-05, OD-14, HC-15, HC-02) |
| Decisions informed | DEC-INT-012, DEC-INT-027, DEC-INT-015, DEC-INT-031 |
| Gates, method, author | conventions C0 |
| Spec | `spec.yaml` (one sample = the full damage-trial grid for one archive under one format) |

## 1. Question

For each candidate external parity format and redundancy level, what share of damage trials
under each pre-registered damage model is fully repaired and re-verified, what does the parity
cost in bytes, creation and repair resources, does any repair path report success for bytes
that fail Entrybound verification, and does chunk-aligned in-format parity at equal overhead
recover more or less than external parity when damage can hit both the archive and its parity?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-INT-012 | PAR2 (Reed-Solomon over GF(2^16), block-erasure code) repairs every trial whose damaged block count does not exceed its recovery block count (MDS behaviour, exact analytic model agreement 100% on the measured subsample); RaptorQ needs slightly more than the lost source symbols (near-MDS); at equal overhead both are within the T-20 floor of each other on scattered and sector damage; PAR2 block-count limits (32,768 blocks) force large blocks on multi-GiB archives, lowering recovery under scattered single-byte damage relative to RaptorQ. | Measured repair outcomes disagree with the analytic model on any subsample trial (then only measured outcomes are used and the discrepancy is investigated), or no format difference exceeds the T-20 floor on any damage model. |
| H2 | DEC-INT-027 | The overhead needed for `parity_repair_success_fraction` >= 0.99 differs by more than a factor of 4 between damage models (scattered versus burst versus lost parity volumes), so a single fixed percentage default is either wasteful or unsafe for some model; a damage-target default (N sector erasures) is expressible as a closed-form overhead. | One overhead level reaches >= 0.99 on every model within a factor of 1.5 of the minimum for each. |
| H3 | DEC-INT-015 | Under a "same medium" correlated damage model (damage hits archive and parity regions alike), in-format parity recovers less than external parity stored as separate files at equal overhead, because damage to the footer or section directory also removes access to in-format parity; under "separate medium" they are within the T-20 floor. | In-format parity not distinguishably worse under the correlated model. |
| H4 | DEC-INT-031 | A malicious or corrupted recovery set crafted so that par2 reports a successful repair yields bytes that `ebound verify` rejects in 100% of crafted cases; `trust-parity-output` therefore reports invalid archives as valid in 100% of them (HC-15 violation), and `out-file-only` with mandatory re-verification reports 0. | Any crafted case where the repaired archive verifies (then the crafted set did not corrupt payload and the case is re-generated). |

## 3. Decisions and candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-INT-012 | `par2` (par2cmdline 0.8.1, apt; par2cmdline-turbo at a pinned release as a performance variant of the same format), `par3-draft` (par3cmdline at a pinned commit; if it fails to build or crashes, the failure is recorded as its maturity evidence), `raptorq-rfc6330` (`raptorq` Rust crate, Apache-2.0, in `ebr-int-parity` with a research container recording object transmission information and symbol ids), `custom-rs-erasure-format` (`reed-solomon-simd` or `reed-solomon-erasure` crate over fixed blocks and, as a variant, over Chunk-frame-aligned blocks from the T1 map), `ldpc-fountain` (measured only if a permissively licensed maintained implementation is identified by the dependency survey before the run; otherwise excluded for lack of an implementation, with the survey as evidence), `no-normative-format` (evaluated as "whatever tool the user brings" = the best measured tool per damage model, plus a digest binding; its cost is C1 only), `rar-style-recovery-volumes-external` (not measured: RAR tools are proprietary and redistribution-restricted; literature and licence evidence only; exclusion reason recorded) | Recovery, overhead, resources (sections 6 and 8). Maturity, maintenance, licence and patent status (RaptorQ patents per RFC 6330 IPR disclosures) are a primary-source survey arm in `survey/parity-formats.md`, written before results; CVE history for parity parsers is part of the same survey; fuzzing parity parsers is the conformance domain's (§30), cited when available. |
| DEC-INT-027 | `no-default`, `fixed-percentage`, `damage-target`, `per-medium-profiles`, `rateless-top-up` | Curves of repair success versus overhead per damage model and size; `damage-target` evaluated as the overhead that survives N = 1, 8, 64 sector erasures of 4,096 B; `per-medium-profiles` from the prevalence survey of EXP-INT-001 (`damage-prevalence.md`) when numeric; `rateless-top-up` measured with RaptorQ: an initial 1% repair set extended to 2%, 5%, 10% by generating additional symbols without regenerating earlier ones (overhead equivalence to a one-shot set of the same total). User workflow evaluation (program §32) is the ecosystem domain's. |
| DEC-INT-015 | `status-quo-permanent-refusal`, `deferred-extension`, `optional-recovery-section-extended-tier`, `assessment-only-no-parity`, `third-party-parity-unbound` | Recovery effectiveness of the in-format prototype (chunk-aligned Reed-Solomon parity stored as a research trailing section, never a wire proposal) against external parity at equal overhead, under separate-medium and same-medium damage (H3). Recovery-parser attack surface: `untrusted_input_loc` and `attacker_controlled_parameters` counted over each prototype's parser (C4 cost inputs), plus the survey of recovery-parser CVEs (RAR, PAR, similar tools) with primary-source identifiers. User need for repair versus assessment is human-facing (gap; ecosystem domain). |
| DEC-INT-031 | `out-file-only`, `in-place-atomic`, `in-place-non-atomic`, `repair-then-reencode`, `dry-run-assessment`, `trust-parity-output` | Repair success rates (this experiment), wrong-reconstruction injection (H4), dry-run accuracy (fraction of trials where the tool's verify-only prediction of repairability equals the actual repair outcome). Crash behaviour of in-place variants is EXP-INT-005. Proposed L0 exclusion of `trust-parity-output` (HC-15: "repaired data is re-verified before it is trusted"); negative control. |

## 4. Corpus and damage models

- Archives: B-PROD `ebound pack --profile balanced` outputs of a size ladder from tuning items:
  `f06-tuning-census-popest-2023` (about 1.4 MB input), `f07-tuning-silesia-xray` (about 8 MB),
  `f05-tuning-enwik8` (100 MB), `f07-tuning-pythia-160m-safetensors-main` (about 375 MB),
  `f05-tuning-loghub-hdfs-v1` (about 1.8 GB); plus a crypto-v1 archive of
  `f07-tuning-silesia-xray` (parity over ciphertext; also informs DEC-INT-033's
  `parity-required-for-encrypted`). Validation look: `f05-validation-rfc-texts`,
  `f08-validation-northwind-sqlite`, `f07-validation-cmip6-ncar-cesm2-tas-zarr`,
  `f07-validation-smollm2-360m-safetensors`, `f06-validation-noaa-ghcn-daily-csv-2023`.
- Redundancy levels: 0.5%, 1%, 2%, 5%, 10%, 20%, 30% of archive bytes; block or symbol size
  per format: the tool default plus 4 KiB, 64 KiB and 1 MiB (PAR2 block count capped by the
  format; the realized block count is recorded).
- Damage models (T2 on the archive file and, where stated, on parity files): scattered
  single-byte errors (k = 1, 16, 256, 4,096); 512 B and 4,096 B sector erasures (N = 1, 8, 64,
  512, aligned); bursts of 64 KiB, 1 MiB, 16 MiB; tail truncation (1%, 10%); loss of whole
  parity volumes (1 of n, half); damage applied to parity files with the same model; correlated
  same-medium damage over the concatenated archive-plus-parity byte space.
- Trials: 200 seeded trials per (format, level, block size, damage model, archive) cell for
  the analytic-model path; a stratified measured subsample of 20 trials per cell repaired with
  the real tool. The analytic model (exact for erasure codes given the damaged block set) is
  used for the full 200 only after the subsample agrees on 100% of trials; otherwise all 200
  are measured.
- These are coverage fractions over pre-registered trial sets; the size ladder is not a family
  sample, so banded corpus-level verdicts are not drawn (C3); the decision relies on per-cell
  exact fractions with Wilson intervals reported descriptively.

## 5. Environment and platform requirements

`wsl-ubuntu`, `timing: false` for recovery; resource figures (parity creation and repair wall
time and RSS) recorded as secondary and re-measured in a quiet window only if a selection would
depend on them. Every parity tool is pinned to one thread for recovery trials (feasibility smoke:
par2cmdline's default thread pool consumed many CPU-minutes on a tiny file) and run with its
default threads in the resource arm. Downloads (par2cmdline-turbo, par3cmdline sources) under the
approved download policy with SHA-256 recorded. Windows arm not needed (format behaviour is
platform-independent); Windows repair tools are not candidates.

## 6. Procedure and commands

1. Survey files `survey/parity-formats.md` and the licence and patent notes committed before any
   run (primary sources only, §9.3).
2. Build tools and record versions (extend `research/baselines/probe_tool_versions.py`).
3. `ebr run` of `spec.yaml`; each sample: create parity at every level and block size, run the
   damage trials, apply the analytic model, repair the measured subsample, re-verify every
   repaired archive with B-PROD `ebound verify`, run dry-run assessment, run the H4 crafted
   malicious recovery sets (20 per format), write detail rows.
4. `verify-raw`, `verify_detail.py`, `normalize`, `report`; curves generated by
   `research/tools/integrity/parity_curves.py --experiment EXP-INT-010` from detail files.

## 7. Seeds, warmups, replication

Seeds 2026091801/02/03; trial seeds per C8 with `class_id` = damage model id. Warmups 0.
2 repetitions: RaptorQ and Reed-Solomon parity outputs are deterministic functions of input and
parameters; par2cmdline output bytes may embed creator strings but not randomness (checked by the
repeat-hash of parity bytes; a mismatch reclassifies parity bytes as nondeterministic, which does
not affect repair outcomes, whose vector is repeat-hashed separately).

## 8. Metrics

| Metric | ID | OD | Role |
|---|---|---|---|
| `parity_repair_success_fraction` | C11 proposed | OD-14 (as assigned) | secondary until named; **primary candidate** for DEC-INT-012, DEC-INT-027, DEC-INT-015 if the review adds it |
| `artifact_bytes` (archive plus parity) | T-01 | OD-05 | **primary** for OD-05 |
| `side_data_bytes` (parity only) | T-02 | OD-05 | diagnostic |
| `roundtrip_mismatches` | HC-02 | OD-02 floor | binary: repaired archive reported repaired whose bytes differ from the original |
| `mvt15a_overclaim_cells` | C11 | HC-15 | binary: repair reported valid without re-verification passing (H4) |
| `encode_wall_s`, `peak_rss_bytes` (parity creation), `decode_wall_s` (repair) | T-03, T-06, T-04 | OD-06, OD-08, OD-07 | secondary (not decision-grade timing) |
| dry-run prediction accuracy | none | - | descriptive |
| `untrusted_input_loc`, `attacker_controlled_parameters`, `decode_path_dependency_count`, `rustsec_advisory_count`, `maintainer_count`, `license_compatibility` | C4, C3, R2 | OD-24, OD-22 | cost inputs and the R2 licence screen for any parity code that would ship in production crates |

## 9. Normalization

Success fractions exact per cell; overhead as parity bytes over archive bytes; comparisons of
formats at matched realized overhead (interpolated curves are descriptive only; decisions use
measured grid points).

## 10. Statistical analysis and sensitivity

Per-cell exact fractions over 200 trials; Wilson 0.99 intervals reported descriptively; format
comparisons use the T-20 absolute floor (0.5 / n_cases) on matched cells, counted per damage
model (AGG-S: damage model is an evaluation condition, headline = worst model). Sensitivity:
block-size arm, size-ladder arm (never pooled), damage-model weighting arm using the EXP-INT-001
prevalence weights when numeric, parity-damage arm (parity files intact versus damaged).

## 11. Expected negative results worth recording

- No single default percentage is adequate across damage models (H2).
- PAR2 block-count limits reduce effectiveness against scattered errors on large archives.
- PAR3 tooling is not mature enough to measure fully.
- In-format parity loses to external parity under correlated damage, supporting the frozen
  refusal; or the opposite, which would be counterevidence to the SPEC §22.5 refusal and must be
  reported prominently.
- par2 bindings authenticate nothing against substitution (EXP-INT-009 analysis).

## 12. Threats to validity

- Damage models are synthetic; real media failure distributions come only from the survey.
- The analytic model is admitted per cell only after 100% agreement on the measured subsample;
  rare disagreements outside the subsample are possible (regression evidence).
- RaptorQ and Reed-Solomon research containers are this program's wrappers; production formats
  would add framing overhead not captured (recorded as a lower bound on overhead).
- Encrypted archives behave like random data for parity; results do not depend on content.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12. Recovery behaviour is content-independent; held-out runs repeat the size-ladder
cells on held-out-sized archives only.

## 14. Cost estimates

- Compute: parity creation for 5 archives x 7 levels x 4 block sizes x 5 formats; 200 analytic
  trials and 20 measured repairs per cell over about 25 damage models. Measured repairs dominate:
  about 150 CPU-hours (large archive repairs of tens of seconds each); analytic trials about 5
  CPU-hours. Wall about 10 hours at 16 workers (one thread per tool).
- Disk: parity at 30% of 1.8 GB plus working copies per worker: about 60 GB peak.
- Agent effort: survey writing (judgment, primary sources), tooling build scripted with review,
  execution none, analysis judgment.

## 15. Deviations (append-only)

None.
