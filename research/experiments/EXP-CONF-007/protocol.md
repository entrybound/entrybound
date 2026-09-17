# EXP-CONF-007 — Resource ceilings of conformance cases: counting-allocator bounds, refusal cost and caller-policy presets

| Field | Value |
|---|---|
| Status | `NEEDS_TOOLING` (CC§2): counting global allocator with allocation-site attribution and untrusted-size tags (§14 item 17), KDF-invocation counter hook, byte-counting source wrapper, pre-validation bound H per family, `ebcc-resource` driver, policy preset files; calibration `EXP-CAL-*-wsl-ubuntu` (§4.7) for the timing arm |
| Domain / program sections | conformance / §29 (resource ceilings per case), §30 (hostile-input resource behaviour) |
| Decision IDs informed | DEC-CON-001, DEC-CON-004, DEC-CON-005, DEC-CON-006, DEC-CON-009, DEC-CRY-013, DEC-CRY-015, DEC-CRY-029, DEC-CRY-067, DEC-ACC-036, DEC-ACC-057, DEC-MOD-036, DEC-PLT-002, DEC-PLT-045, DEC-PLT-060, DEC-CMP-016 |
| HC oracles | HC-06 MVT-06(a) (allocator oracle, R0_max, H, KDF counter), MVT-06(b) (declaration consistency); proposal for the assigner: `hc_ids` HC-06, HC-05; `od_ids` OD-08, OD-17 |
| Decision type | not assigned (gate G-A) |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1; the benign-refusal analysis uses corpus families and needs adoption by a non-exposed session |
| Depends on | EXP-CONF-004 (`ebcc-v1` families F-RES, F-BOUND, F-CODEC, F-CRYPTO); calibration experiments of the method domain for timing |

## 1. Question

For each adversarial (F-RES) and frozen-bound (F-BOUND) conformance case, does production refuse or accept within the
exact ceiling the format implies (allocator peak ≤ R0_max + H before any budget check; ≤ R0_max + B_memory after
acceptance; zero KDF invocations above the ceiling); what do refusals cost in allocator peak, CPU and bytes read under
each candidate default caller policy; how tight are declared requirements on valid archives; and how often does each
candidate policy refuse benign archives?

## 2. Hypotheses and falsification

- **H1 (MVT-06(a) pre-check bound).** For every refused case under every policy, the allocator peak until the refusal is
  ≤ R0_max + H, and no allocation sized from an untrusted field precedes that field's budget check. *Falsified* by one
  exceedance or one attributed pre-check allocation.
- **H2 (MVT-06(a) accepted bound).** For every accepted case, allocator peak ≤ R0_max + B_memory (declared decode
  requirement) and scratch bytes written ≤ B_scratch. *Falsified* by one exceedance.
- **H3 (KDF ceiling).** For every case whose KDF parameters exceed the policy ceiling, the Argon2id invocation counter is
  0. *Falsified* by one invocation.
- **H4 (true refusal, M17.4).** Under each candidate policy, every F-RES case is refused
  (`adversarial_true_refusal_fraction` = 1). *Falsified* per policy by one accepted case; that policy is eliminated at
  R1 for DEC-CON-005/-006.
- **H5 (MVT-06(b) declaration consistency).** On every valid archive, reader-derived aggregates equal the declared
  requirements and actuals stay within declared upper bounds (`declared_tightness_ratio` ≥ 1 everywhere).
  *Falsified* by one ratio < 1.

## 3. Candidates

Only ledger candidates that are expressible as caller-policy values without a wire change are executed (CC§1: none
added). Each axis varies one factor with the other axes at status quo (one factor at a time), plus the all-status-quo
reference `POL-SQ`.

| Axis (decision) | Executed candidates | Not executed and why |
|---|---|---|
| Resource budget (DEC-CON-006) | `bootstrap-generous-status-quo` (= `POL-SQ`), `go-extract-style-conservative`, `corpus-percentile-derived`, `context-presets` (instantiated as three presets named after the SPEC §6 use contexts: untrusted download, CI build, backup restore; each run separately), `host-resource-derived` (simulated MemAvailable 16 GiB; 4 and 64 GiB in sensitivity), `declared-with-hard-ceiling`, `archive-declaration-raises-limits` | `no-defaults-caller-supplied` is not a policy value; its refusal behaviour equals whatever the caller supplies (analytical). `archive-declaration-raises-limits` contradicts HC-05 on its face; it is **run** because it is cheap, and its R1 exclusion needs a counterexample reviewed by an uninvolved session (objective §2.0 rule 3) |
| Decoder ceilings (DEC-CON-005, DEC-CMP-016) | `status-quo-8mib-384mib`, `zstd-cli-128mib`, `low-memory-default-with-opt-in`, `per-feature-ceilings`, `host-derived`, `trust-declared-without-ceiling` (run; proposed R1 exclusion as above) | none |
| Crypto open policy (DEC-CRY-013, DEC-CRY-015) | `status-quo-default-equals-ceiling` / `status-quo-limits`, `default-equals-creation-default`, `hardware-probed-default` (probe fixed at the WSL VM's values, recorded), `lower-recipient-cap`, `measured-defaults-same-caps`, `tiered-policy-profiles` | `confirm-above-threshold` is an interaction design (HUMAN-FACING); `smaller-record-caps` and `larger-control-cap` change frozen crypto-v1 wire limits (F-12) and are not caller policies: excluded at L0 pending the uninvolved-session sign-off, and routed to the crypto domain |

Numeric values for every non-status-quo preset are fixed **before the record is committed** by
`research/conformance/resource/derive_presets.py`: `go-extract-style-conservative` from the go-extract default limits at
a pinned release tag (primary source, cited by URL, tag and file line); `zstd-cli-128mib` from the zstd 1.5.5 manual's
default decompression memory limit; `corpus-percentile-derived` as 2 × the 99.9th percentile of the **tuning** split
statistics (`research/corpus/statistics.json`, tuning rows only); `per-feature-ceilings` from the decode requirements
declared in `docs/codec-transform-v1.md`, `docs/reconstructive-transform-v1.md` and `docs/jpeg-reconstruction-v1.md`.
The output `presets-v1.json` is committed with its inputs' SHA-256.

## 4. Inputs

- `ebcc-v1` families F-RES and F-BOUND (all cases), F-CODEC (window and dictionary cases), F-CRYPTO (KDF-parameter and
  envelope cases), F-DESC (all-ones budget and flag vectors).
- **Benign set** (H4 false-refusal and H5): `ebound-head` deterministic packs (balanced and extreme profiles × indexed
  and stream layouts, unencrypted) of every small- and medium-tier **tuning** item; one registered validation look on
  small- and medium-tier validation items (§12).

## 5. Instruments

- **Counting allocator** (`research/harness/crates/ebr-alloc`): a `GlobalAlloc` wrapper recording live bytes, peak and,
  for allocations ≥ 64 KiB, a symbolized call-site key; plus a scope API used by `ebound-research` hooks
  (`research::alloc_tag::untrusted(field)`, default-off, F-24) that marks allocations whose size derives from an
  untrusted field until the corresponding budget check calls `research::alloc_tag::checked(field)`. The MVT-16(c)
  identity proof for `ebound-research` passes before any record (CC§4).
- **R0_max**: maximum allocator peak over 20 runs of opening the minimal valid archive of each layout (objective
  MVT-06(a); Appendix C.1 allocator baseline).
- **H per family**: the largest structure the documents permit a reader to buffer before the budget comparison
  (preamble, Descriptor and its enclosing record, per layout), computed in bytes from the wire documents by
  `compute_h.py` and committed with citations **before** the first record. If H is not computed for a family, HC-06 is
  `HC_UNVERIFIED` for it.
- **KDF counter**: `research::kdf_counter` hook incremented at every Argon2id invocation.
- **Bytes read**: a counting wrapper over the public Source/Read abstraction (Memory and LocalFile).
- **CPU**: thread CPU time (`CLOCK_THREAD_CPUTIME_ID`) around each open-to-outcome operation, in process.
- **Build**: `ebcc-resource` links `ebound-research` (CC§4) and is built into
  `CARGO_TARGET_DIR=/root/eb-research/target/harness-research` so that research-feature builds never share a target
  directory with the public-API harness builds.

## 6. Environment and timing protocol

- WSL2 Ubuntu 24.04, 32 vCPUs, ext4 under `/root/eb-research`; affinity `st-v` = {2} (§4.3); `VIRTUALIZED` qualifier;
  cache state `hot` (§4.5).
- **Timing arm** (`spec.yaml`, `timing: true`): `refusal_cpu_s` and allocator peaks. Guard: `max_loadavg_1m` 2.0
  (1.0 + 1 thread), `max_other_cpu_percent` 3.0, `max_wait_s` 300 (§4.4); cooldown, drift canary and host-side sampler
  per §4.2-§4.3 (runner additions §14 item 3). Exclusive window: no fuzzing, builds or other experiments (CC§3).
- **Deterministic arm** (`spec-deterministic.yaml`, `timing: false`): refusal outcomes, bytes read, KDF counter, H1-H3
  allocation attribution checks, H4 refusal fractions, H5 tightness.
- Calibration prerequisite: `EXP-CAL-AA-wsl-ubuntu`, `EXP-CAL-KE-wsl-ubuntu` and `EXP-CAL-BG-wsl-ubuntu` pass for
  families `time_cpu` and `memory_peak` before any timing-arm verdict is decision-grade (§4.7).
- **Memory-limited host arm** (`spec-lowmem.yaml`, `timing: false`; DEC-CON-005 "behaviour on low-memory devices",
  DEC-CRY-013 "Argon2id at ceiling on low-end hardware"): the candidates `POL-SQ`, `DEC-low-memory-opt-in`,
  `DEC-zstd-cli-128mib` and `CRY-creation-default` each run under an address-space limit
  (`prlimit --as=<limit>`) of {512 MiB, 1 GiB, 2 GiB} applied to the release (non-sanitizer) driver process, over the
  benign packs and the F-CRYPTO KDF cases; outcomes are completed, refused (class and code), or aborted on allocation
  failure. Every run first checks enforcement by allocating 2 × the limit in a probe process under the same limit (the
  probe must fail); a run whose probe succeeds is invalid. A feasibility smoke at design time (non-decision-grade, no
  numbers recorded) showed that `systemd-run --scope -p MemoryMax`/`MemoryLimit` scopes are **not enforced** on this
  WSL2 cgroup configuration while `prlimit --as` is, which is why the address-space limit is used. An address-space limit
  emulates memory scarcity only and counts virtual reservations (labelled `EMULATED`); resident-memory limits (Docker
  memory limits) and real low-end devices are BLOCKED in EXP-CONF-014.

## 7. Commands

```sh
python3 research/conformance/resource/derive_presets.py --out research/conformance/resource/presets-v1.json
python3 research/conformance/resource/compute_h.py --docs <snapshot>/docs --out research/conformance/resource/h-v1.json
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CONF-007/spec-deterministic.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-007/spec-deterministic.yaml   # raw id EXP-CONF-007-DET
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-007/spec-lowmem.yaml          # raw id EXP-CONF-007-LM
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CONF-007/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --timing --spec research/experiments/EXP-CONF-007/spec.yaml          # quiet window only
```

One sample = one (policy candidate, case family bundle or benign item) in one process: `ebcc-resource` opens every case
of the bundle in a fixed seeded order, each case measured with fresh allocator counters, and writes per-case values to
the raw-artifact directory; stdout carries family summaries. Per-case operations shorter than 1 ms are measured as
batches of ≥ 1,000 repetitions inside the process (§3.2 T-08 rule applied to `refusal_cpu_s`).

## 8. Seeds, warmup, replication and adaptive rule

- Deterministic arm: 2 repetitions + repeat-hash (§4.6, §4.13).
- Timing arm (small tier, §4.6): minimum 10 rounds, step 10, cap 30, and at least n_min for the confidence in use;
  warmups 2 (§4.5); adaptive rule of §4.6 on the per-case exact order-statistic interval half-width of the median
  paired log ratio against `POL-SQ` (target 0.5 × `log1p(r)`), direction-blind; cases still above target at the cap are
  flagged `imprecise`.
- Order: randomized blocks (§4.9 pairing within rounds); seeds CC§12.

## 9. Metrics (CC§9)

| Metric | Role | Unit | Measurement |
|---|---|---|---|
| `refusal_alloc_peak_bytes` | banded (T-06) | B | allocator peak from open to refusal, per case (exact allocator) |
| `refusal_cpu_s` | banded (T-05, in-process) | s | thread CPU time open to refusal, per case |
| `refusal_bytes_read` | banded (T-02) | B | bytes consumed from the source before refusal |
| `alloc_peak_bytes` | banded (T-06) | B | allocator peak on accepted F-BOUND cases |
| `adversarial_true_refusal_fraction` | binary | fraction | F-RES cases refused under the policy (H4) |
| `benign_false_refusal_fraction` | banded (T-20) | fraction | benign-set archives refused under the policy |
| `declared_tightness_ratio` | banded (T-23) | ratio | declared requirement ÷ measured actual (memory, decoded bytes, entries, chunks) on benign and valid F-BOUND archives (H5) |
| MVT-06(a) violations (pre-check allocations, exceedances, KDF invocations above ceiling) | binary (HC-06 oracle) | count | H1-H3 |
| `peak_rss_bytes` | banded (T-06), secondary corroboration only | B | GNU time `ru_maxrss` of the sample process (never the oracle; R1-16 constraint) |

## 10. Normalization and statistical analysis

- **Screens first** (R1/R2): H1-H3 violations eliminate the (build, bound) pair; H4 failures eliminate the policy
  candidate for DEC-CON-005/-006; H5 failures are HC-06 declaration defects.
- **Refusal cost** (`refusal_*`): item level = conformance case, exact order-statistic interval (§4.10) for timing and
  allocator metrics; deterministic values exact. Conformance case families are **not corpus families**, and §4.9
  minimum support (≥ 10 groups across ≥ 5 families) cannot hold, so refusal-cost comparisons between policies are
  **diagnostic evidence per case family** and are not used as primary metrics for a corpus-level verdict.
- **Benign false refusal** (primary for DEC-CON-005/-006 once H4 passes): per tuning item exact 0/1 per archive; L2-L4
  aggregation over independence groups and families with base weights from `research/methods/workload-mix.json`
  (§4.9); Welch-Satterthwaite corpus interval (§4.10); non-inferiority and superiority per §4.11-§4.12 against `POL-SQ`,
  confirmatory only on the registered validation look.
- **Tightness**: AGG-G per family (T-23), reported per resource dimension.
- Verdicts come only from the decision tooling (§14 item 2). The analysis plan needs adoption by a non-exposed session
  (CC§1).

## 11. Practical-significance thresholds

T-06 (`metric_thresholds.refusal_alloc_peak_bytes`, `…alloc_peak_bytes`, `…peak_rss_bytes`), T-05
(`…refusal_cpu_s`, in-process floor), T-02 (`…refusal_bytes_read`), T-20 (`…benign_false_refusal_fraction`),
T-23 (`…declared_tightness_ratio`); §3.3 for binary checks. Values are those in `research/methods/thresholds.json`.

## 12. Split discipline and held-out placeholder

Tuning packs for all exploratory comparisons; one registered validation look for the benign-set confirmatory comparison
(small and medium validation items, same packs), registered in `research/decisions/validation-looks.jsonl` with its MDE
computed from the tuning group variance (§4.12). Held-out placeholder (Phase D, §5.5): benign false refusal of the frozen
policy candidates on held-out packs, once, with a spec written at Commit A.

## 13. Sensitivity analysis

§6 arms applicable to the benign-set comparison: weight grid and Dirichlet over the decision's OD set, equal-weight arm,
leave-one-family-out, leave-three-families-out, family Dirichlet, WM-* mixes, tier-stratified, byte-weighted, threshold
perturbation with the resolvability condition, selection-stability bootstrap (seeds CC§12). Additional: host-derived
presets at simulated 4 and 64 GiB; H computed with the conservative (largest permitted) versus exact buffering rule;
allocation-attribution threshold 64 KiB versus 4 KiB.

## 14. Expected negative results worth recording

- Allocations sized from untrusted counts before budget checks on some parse paths (the ledger's CON-001 status quo
  questions this).
- `trust-declared-without-ceiling` and `archive-declaration-raises-limits` accepting F-RES liars (R1 evidence).
- Conservative presets refusing benign medium-tier archives (for example JPEG-reconstruction or dictionary archives).
- STREAM `BudgetDeclared=false` cases whose refusal happens only at end of stream (DEC-ACC-057 status quo).
- Declared working sets far above measured actuals (tightness ≫ 1).

## 15. Threats to validity

- **Instrumentation** changes allocation and inlining; allocator values come from `ebound-research` and are exact for
  that build; the identity proof covers bytes and outcomes, not allocation patterns (recorded limitation).
- **H depends on a reading of the documents**; `compute_h.py` cites every term, and a disputed H uses the larger value.
- **Virtualized timing** (`VIRTUALIZED`, soft cap MEDIUM for Linux-native claims).
- **Preset values** from external sources may be version-specific; each is pinned by tag and date.
- **Benign set** covers what the packer emits for two profiles; other profiles are a recorded scope limit.

## 16. Cost estimates

| Item | Estimate |
|---|---|
| Compute | 62 CPU-hours: deterministic arm ≈ 15 h (19 policies × families and benign packs × 2 repetitions, parallel); memory-limited arm ≈ 7 h (12 candidates × benign packs × 2 repetitions); timing arm ≈ 40 h wall on one pinned vCPU in a quiet window (19 policies × 5 bundles × 10-30 rounds, 2 s minimum cooldown) |
| Disk | 50 GB (large F-BOUND cases materialized; benign packs of medium tuning items) |
| Agent effort | **scripted**; **judgment, low** (H derivation review; R1 exclusion counterexample review by an uninvolved session) |
| Platform | Linux x86-64 (WSL), pinned vCPU; exclusive quiet window for the timing arm; no network after preset derivation |

## 17. Outcome obligations

`outcome.json` (§10.1); per-case measured ceilings are written back to the `resource_ceiling` field of `ebcc-v1.1`
cases; every violation is an HC-06 defect record.
