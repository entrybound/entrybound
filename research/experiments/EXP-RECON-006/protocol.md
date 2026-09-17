# EXP-RECON-006: Resource bounds of reconstruction — exact allocator peaks versus size and megapixels, declared-requirement tightness, frozen limits, default decoder ceilings, planner/validation mirroring, hostile CPU scaling

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-RECON-006 |
| Domain | reconstruction (8.8); common provisions: `research/experiments/_index/reconstruction-common.md` |
| Status | **NEEDS_TOOLING**: counting allocator with allocation-site attribution (§14 item 17); `ebr-recon probe-alloc`, `decode-probe`, `pack-outcome`, `cpu-scale`; case generator `gen_resource_cases.py`; cgroup-v2 wrapper `cgroup_run.sh`; baseline R0_max measurement |
| Stage | Tuning (exploratory: limit and ceiling sweeps; HC-06 screens), then validation look 1 for DEC-CON-005 and DEC-CMP-016 ceiling and limit selection (registered with the EXP-RECON-004 look) |
| decision_ids | DEC-CMP-016, DEC-CON-004, DEC-CON-005, DEC-CMP-037, DEC-CMP-026, DEC-CMP-018, DEC-CMP-012 |
| Proposed types/ODs/HCs | common §2. OD-08 (`alloc_peak_bytes` primary), OD-17 (`declared_tightness_ratio`, `benign_false_refusal_fraction`), HC-06 (MVT-06(a) exact allocator oracle, MVT-06(b) declaration consistency, MVT-06(d) CPU scaling), HC-15 (refusal classes) |
| Gates | G-A; G-B for T-06/T-23/T-20 verdicts. **HC-06 is `HC_UNVERIFIED` for every candidate until the counting allocator and the pre-validation bound H exist** (objective HC-06 status). This experiment builds the reconstruction part of that instrument. |
| Author and exposure | common §3 |
| Feasibility smoke | none |

## 2. Question

For DEFLATE (`deflate-reconstruct/v1`) and JPEG (`jpeg-jxl-reconstruct/v1`) reconstruction, and for their candidate variants, what are the exact allocator peaks of the forward (writer) and inverse (reader) steps as functions of input size, megapixels, coding mode and producer? The measurement includes output and intermediate buffers, attributed by allocation site. Beyond that:
- Are archive-level `DecodeRequirements` declarations upper bounds on measured decode memory?
- How tight are they?
- Which frozen limits exclude which real content, and at what memory saving?
- Which default decoder ceiling admits the most benign archives without admitting a decode whose measured peak exceeds the ceiling?
- Does the planner fall back or fail when a validation bound would reject its selection?
- Does CPU work stay linear on hostile near-valid inputs?

## 3. Hypotheses

All hypotheses are `[INFERRED]` from `crates/entrybound/src/{reconstruction,jpeg_reconstruction}.rs` and the frozen docs.

- **H1 (DEC-CON-004, JPEG).** At least one accepted JPEG-region archive has a measured whole-decode allocator peak above its declared `working_set_bytes`: `declared_tightness_ratio` < 1, an HC-06 declaration defect of the status quo.
  - Rationale: the jxl-oxide `AllocTracker` limit (256 MiB) covers dependency allocations only. The bounded output writer (≤ 64 MiB), the representation bytes (≤ 64 MiB) and reader buffers are outside it.
  - **Falsified if** every accepted case has ratio ≥ 1.
- **H2 (DEC-CON-004, DEFLATE).** Some accepted DEFLATE-reconstructive archive at the 64 MiB intermediate edge has a measured inverse peak above the declared additional 80 MiB plus codec state. **Falsified if** none does.
- **H3 (DEC-CMP-037).** For JPEG, the forward (jixel encode + verify) allocator peak is ≥ 2× the inverse peak at the same megapixels, over the SOF0 series ≥ 12 MP. **Falsified if** < 2× at every size. Either way the result sizes `encoder-allocation-cap`.
- **H4 (DEC-CMP-016 `planner-mirrors-validation`).** At least one generated benign case (within recognised JPEG/DEFLATE structure) makes status-quo `ebound pack` fail with an error instead of falling back to an ordinary representation. **Falsified if** 0 such cases.
- **H5 (MVT-06(d), DEC-CMP-012 adversarial gate cost).** In-process CPU time of failed raw-DEFLATE attempts on near-valid DEFLATE chunks (2^10–2^24 bytes) scales with exponent interval upper bound ≤ 1.15.
  - **Falsified if** the lower bound of the 0.99 interval exceeds 1 + δ (δ = 0.15, objective MVT-06(d)), which is an HC-06 violation.
  - The same test runs for jxl-oxide inverse on crafted JPEG XL representations scaled by frame and group count.
- **H6 (DEC-CON-005).** Under a 128 MiB working-set ceiling (`zstd-cli-128mib`), ≥ 10 % of balanced tuning archives are refused while their measured decode peak is below 128 MiB (declaration-driven false refusal). **Falsified if** < 10 %.
- **Controls (binary).**
  - **Allocator oracle validation.** A test binary allocating known patterns (1 MiB, 17 MiB, fragmented 4 KiB × 10,000) must report exact peaks.
  - **R0_max.** 20 runs opening a minimal valid archive (MVT-06(a)); every run reports the same peak, or the maximum is used.

## 4. Decisions informed and how

| Decision | Candidates (ledger) | Contribution |
|---|---|---|
| DEC-CMP-016 | status-quo-limits, measured-limits, planner-mirrors-validation, lower-default-ceiling, profile-scaled-ceilings | Peak memory versus input size and megapixels (ledger evidence 1); limit-reach trade-off using EXP-RECON-002 distributions (evidence 2); low-memory decode emulation under cgroup memory limits (evidence 3, emulated; physical devices in EXP-RECON-012); pack behaviour on crafted high-expansion JPEGs (evidence 4) |
| DEC-CON-004 | status-quo-codec-state-maxima, total-peak-memory-definition, separate-state-and-output-fields, per-group-declarations, flag-registry-reject-unknown, flags-removed | Allocation-site composition of reconstruction decode peaks (codec state, output, intermediate, representation, reader buffers). Evaluates which definition makes declarations upper bounds (tightness ≥ 1) at the least looseness (T-23). Codec and lookback parts: container and crossfile domains (common §12). |
| DEC-CON-005 | status-quo-8mib-384mib, zstd-cli-128mib, low-memory-default-with-opt-in, per-feature-ceilings, host-derived (`trust-declared-without-ceiling` excluded by ledger) | `benign_false_refusal_fraction` per ceiling on real archives; the binary "admitted but exceeds ceiling" count; cgroup emulation outcomes. `host-derived` is evaluated as a formula `min(available_memory/4, 1 GiB)` applied to a declared set of host classes {1, 2, 4, 8, 64 GiB} (evaluation only; no host probing). |
| DEC-CMP-037 | encoder-allocation-cap, work-unit-timeouts | Forward peaks versus megapixels (cap sizing); hostile CPU scaling (timeout necessity) |
| DEC-CMP-026, DEC-CMP-018 | status quo versus limit variants | Memory cost of reach (limits) as OD-08 inputs |
| DEC-CMP-012 | gates | Adversarial failed-attempt CPU (H5) |

## 5. Candidates

- **Reconstruction steps measured.**
  - `deflate-reconstruct/v1` forward and inverse at max-chain {512, 2048, 4096};
  - `jpeg-jxl-reconstruct/v1` forward and inverse (status quo);
  - EXP-RECON-003 survivors' forward and inverse (libjxl, lepton-jpeg-rust, brunsli, latest jixel/jxl-oxide, latest preflate-rs, preflate-cpp). External C++ backends are measured by process `peak_rss_bytes` only (no in-process allocator); this is a recorded limitation.
- **Limit sets for DEC-CMP-016** (writer-side; fixed now):
  - `L-status-quo`: DEFLATE 16/16/64 MiB, 64×; JPEG 64 MiB, 100 MP, 4096 Chunks, 64×.
  - `L-measured`: the smallest caps whose measured inverse peaks keep `declared_tightness_ratio` ≥ 1 with the declared working set unchanged. Derived on tuning by the rule in §11 step 5.
  - `L-halved`: DEFLATE 8/8/32 MiB; JPEG 32 MiB, 50 MP, 2048 Chunks.
  - `L-doubled`: DEFLATE 32/32/128 MiB; JPEG 128 MiB, 200 MP, 8192 Chunks.
- **Ceiling policies for DEC-CON-005 / DEC-CMP-016** (reader-side, `{window, working set}`; fixed now):
  - `C-384`: 8 MiB / 384 MiB (status quo);
  - `C-128`: 8 MiB / 128 MiB;
  - `C-64-optin`: 8 MiB / 64 MiB, with a reconstruction opt-in raising the reconstruction allowance to 384 MiB;
  - `C-per-feature`: codec state ≤ 128 MiB and reconstruction ≤ 256 MiB, checked separately;
  - `C-host-{1,2,4,8,64}GiB`: host-derived formula (§4).

  `profile-scaled-ceilings` is evaluated **writer-side** as limit sets per profile (fast: none; balanced: `L-halved`; dense: `L-status-quo`; extreme: `L-doubled`). A reader-side variant keyed by creation profile is **excluded** because archives record requirements, not profile names (SPEC §6.0a; decoders must not depend on creation profile, CONTRIBUTING.md L67-70, R1). The exclusion needs reviewer sign-off (objective §2.0 rule 3).

## 6. Corpus and generated cases

- **Real archives.** Every tuning `{balanced,dense,extreme}-gen-v6` and `-v6-minus-jpeg` archive from EXP-RECON-004 (pre-built, hash-checked), plus EXP-RECON-001 plain INDEXED archives for `fast`. Decode peaks are measured on all of them.
- **S-RESOURCE generated cases** (tuning split; `gen_resource_cases.py`, seed 20260924; output `research/experiments/EXP-RECON-006/cases-tuning.jsonl` with SHA-256 per case; provisioned under `/root/eb-research/generated/EXP-RECON-006/`):
  - **JPEG scaling series.** Content {camera-like tiles from tuning `f12-tuning-nasa-ivl-photos-*`, flat colour, uniform noise} × megapixels {0.1, 1, 4, 12, 24, 48, 99.9, 100.1} × modes {SOF0 4:2:0 q85, SOF0 4:4:4 q95, SOF2 progressive q85, SOF0 restart-interval 1 MCU} × producers {libjpeg-turbo 2.1.5 `cjpeg`, jpegli `cjpegli` from the pinned libjxl build}.
  - **JPEG header-lie cases.** SOF declares 20,000 × 20,000 with truncated scan data.
  - **DEFLATE scaling series.** Plaintext {text concatenated from tuning F05 items, zeros, CSPRNG} × intermediate sizes {64 KiB, 1 MiB, 4 MiB, 15.9 MiB, 16.1 MiB, 63.9 MiB, 64.1 MiB} × producers {zlib 6, zlib 9, libdeflate 12, zopfli, pigz `-p8`} × wrappers {gzip, zlib, raw}. Compressed-input sizes straddling 16 MiB are included via CSPRNG content.
  - **Mirroring cases (H4):**
    - JPEGs whose JPEG XL representation exceeds the reconstructed-to-stored 64:1 expansion bound (flat content, large dimensions);
    - JPEGs requiring more than 4096 Chunks under dense chunking (≤ 64 MiB files at the dense minimum chunk size);
    - gzip whose intermediate exceeds 64× (zeros).
  - **Hostile CPU series (H5):**
    - near-valid DEFLATE: a valid dynamic-block prefix of length 2^k bytes, k ∈ {10…24}, whose final block has an invalid code;
    - crafted JPEG XL representations from libjxl with frame and group counts scaled 2^k, wrapped in region records with valid digests (attacker-built archives, research writer);
    - 3 runs per point (MVT-06(d)).
- **Validation.** Real validation archives from EXP-RECON-004 look 1. Generated cases are not split-specific tuning content; they are re-generated with seed 20260925 for validation ceiling analyses.
- **Held-out.** Phase D placeholder: decode peaks and ceilings on held-out archives of frozen candidates (common §5).

## 7. Environment and platform requirements

- `wsl-ubuntu`, x86-64. The allocator is in-process (exact) and needs no timing controls, so no quiet window is required for allocator metrics.
- CPU-scaling fits use in-process counters. They run in a quiet window with `st-v` pinning to reduce noise, but no band comparison is made on them.
- **cgroup v2 emulation.** Root in WSL; `/sys/fs/cgroup` delegated. `cgroup_run.sh` creates `/sys/fs/cgroup/eb-recon-006/<case>` with `memory.max` ∈ {64, 128, 256, 384, 512} MiB and `memory.swap.max` = 0, runs the decode and records the OOM-kill (`memory.events`). Labelled `EMULATED` (a memory limit, not a device).
- **Windows arm** (optional, NEEDS_TOOLING). Archive-level decode `peak_commit_bytes` via the runner's job-object sampling (`winjob.py`) on the Windows host. Memory is never compared across operating systems (§4.14).
- **PLATFORM_BLOCKED.** Physical low-memory ARM devices: EXP-RECON-012.

## 8. Commands and tooling to build

- **Counting allocator** (§14 item 17). `research/harness/crates/ebr-common/src/alloc_count.rs`: a global allocator wrapper recording current, peak and per-site peaks. Sites are attributed by a thread-local "scope stack" pushed by research wrappers around dependency calls (jixel, jxl-oxide, preflate-rs, Entrybound output buffers). Implementing `GlobalAlloc` needs an `unsafe impl`, which the harness workspace forbids (`unsafe_code = "forbid"`). The allocator therefore comes from either:
  - (a) a pinned third-party counting-allocator crate whose `unsafe` stays inside the dependency, as `ppmd-rust`'s does (harness README), with peak and scope snapshots read through its safe API; or
  - (b) a separate research crate `ebr-alloc` with a documented, reviewed `allow`.

  The choice is recorded by the tooling session before any run. If neither passes harness review, the fallback is Linux `VmHWM` scoped peaks (T-06 RSS floor) with no site attribution, and H1/H2 attribution becomes `NOT_DECISION_GRADE`.
- **Subcommands:**

```text
ebr-recon probe-alloc  --case <dir> --step forward|inverse --backend <id> [--max-chain N] --experiment-id EXP-RECON-006 --env-name wsl-ubuntu
ebr-recon decode-probe --archive <path> --policy-window <B> --policy-working-set <B> [--reconstruction-opt-in true]
ebr-recon pack-outcome --item-path <dir> --profile dense|extreme|balanced   # production ebound pack outcome + fallback audit reasons
ebr-recon cpu-scale    --case <dir> --step raw-attempt|jxl-inverse --runs 3
bash research/experiments/EXP-RECON-006/cgroup_run.sh <memory.max MiB> -- ebr-recon decode-probe --archive <path> --policy-working-set <B>
python3 research/experiments/EXP-RECON-006/gen_resource_cases.py --seed 20260924 --split tuning --out /root/eb-research/generated/EXP-RECON-006
```

- **Runner.** `spec.yaml` (cells in §5) over `cases-tuning.jsonl` plus `archives-tuning.jsonl` (the EXP-RECON-004 archive manifest). Then `verify-raw`, `normalize`, and analysis with `research/tools/recon/resource_tables.py` (to be written). The same PowerShell caution as EXP-RECON-005 §8 applies.

## 9. Seeds, warmups, replication

- Seeds: generation 20260924 (tuning) and 20260925 (validation); order, bootstrap and command 20260924.
- Allocator peaks are deterministic single-threaded measurements: 2 repetitions plus an exact-equality check.
  - A mismatch reclassifies the metric as stochastic (§4.13 non-claim branch), with §4.6 timing-style rounds (10 minimum) for that (step, backend).
  - The allocator is not a determinism claim of the product, so this is not an R1 matter.
- CPU scaling: 3 runs per point (MVT-06(d)), with warmups 1.
- cgroup emulation: 2 repetitions per (archive, limit). An OOM in either repetition counts.

## 10. Metrics

| Metric (canonical) | Role | OD | Threshold |
|---|---|---|---|
| `alloc_peak_bytes` (encode and decode variants) | primary (OD-08) | OD-08 | T-06 (allocator floor) |
| `peak_rss_bytes` | secondary corroboration (external C++ backends: sole measure, labelled) | OD-08 | T-06 |
| `declared_tightness_ratio` | primary (OD-17); values < 1 are HC-06 violations (R1) | OD-17 | T-23 |
| `benign_false_refusal_fraction` | primary for ceilings (n_cases = real archives per profile) | OD-17 | T-20 |
| `adversarial_true_refusal_fraction` | binary (hostile archives refused) | — | §3.3 |
| `refusal_alloc_peak_bytes`, `refusal_cpu_s`, `refusal_bytes_read` | secondary (refusal cost) | OD-17 | T-06, T-05, T-02 |
| `hostile_cpu_scaling_exponent` | descriptive; the HC-06 MVT-06(d) violation rule is binary | OD-17 | objective MVT-06(d) δ |
| admitted-but-exceeds-ceiling count; cgroup `oom_killed` outcomes | binary per policy (must be 0 for an admissible policy) | — | §3.3 |
| allocation-site composition shares | diagnostic | — | — |
| limit reach (bytes excluded per limit set, from EXP-RECON-002) | descriptive | — | — |

## 11. Normalization and analysis

1. **Controls.** Allocator oracle validation must pass; R0_max computed.
2. **HC-06 screens (binary).** For every accepted archive and case:
   - allocator peak ≤ R0_max + declared B_memory (MVT-06(a)); any exceedance is an R1 violation of the status quo (H1/H2), with artifacts committed;
   - refusals must be `POLICY_REFUSED`, or `CORRUPT`/`NONCONFORMING` for lying declarations, with `refusal_alloc_peak_bytes` ≤ R0_max + H;
   - CPU-scaling exponent rule (H5).
3. **Composition.** Site shares of peak per step and size (DEC-CON-004 candidates). For each declaration definition (status quo codec-state maxima; total peak; separate state and output fields), compute the declared value it would carry and `declared_tightness_ratio` against measured peaks. A definition with any ratio < 1 is inadmissible (R1). Among admissible definitions, report T-23 looseness (tuning, exploratory; validation in look 1).
4. **Ceilings (DEC-CON-005).** For each policy:
   - `benign_false_refusal_fraction` on real archives;
   - admitted-but-exceeds count (declared ≤ ceiling but measured peak > ceiling), which must be 0;
   - cgroup outcomes.

   On tuning, form W and S among policies with zero admitted-but-exceeds cases: W minimises refusal fraction subject to the policy's ceiling. On validation (look 1): refusal fractions per profile cell with the T-20 absolute band. Superiority and non-inferiority per §4.11 use exact counts; deterministic, so intervals collapse per archive and corpus-level intervals come from group variance (§4.10).
5. **Measured limits (DEC-CMP-016).**
   - Tuning rule (fixed now): `L-measured` = for each limit dimension, the largest value v in the generated series such that every accepted case with dimension ≤ v has `declared_tightness_ratio` ≥ 1 under the unchanged declaration.
   - Reach effect: bytes admitted and excluded versus `L-status-quo` from EXP-RECON-002 distributions.
   - The artifact-level size effect of each limit set is computed by the EXP-RECON-004 exact cost model (a stream admitted or excluded changes its representation). Every limit set is a new planner parameter set (T1/T2).
6. **Planner mirroring (H4).** Count and list pack failures on benign cases; each is a defect artifact. The `planner-mirrors-validation` candidate removes them by construction. It is verified by re-running the cases with the research planner override `--mirror-validation-bounds` (EXP-RECON-004 tool).
7. **Encoder cap (DEC-CMP-037).** Forward-peak regression versus megapixels per mode (descriptive). Candidate caps are the megapixel values at which the forward peak reaches {128, 256, 512} MiB, passed to EXP-RECON-007/011 as `encoder-allocation-cap` parameters.

## 12. Practical-significance thresholds

T-06, T-23, T-20, T-05 and T-02 by id; §3.3 binaries; objective MVT-06(d) δ = 0.15 (an HC oracle constant, not a band). Not restated otherwise.

## 13. Sensitivity analysis

- Ceiling selection recomputed with and without F12, with and without F20, per profile and tier.
- Workload-mix arms WM-MEDIA and WM-BACKUP for refusal fractions (§6.5).
- Threshold perturbation ×0.5 and ×2 of T-20 and T-23 (§6.4).
- `L-measured` derived with and without the jpegli producer cases.

## 14. Expected negative results worth recording

- Status-quo declarations understate reconstruction decode memory (H1/H2): an HC-06 defect needing a new declaration definition (DEC-CON-004) under a new identifier.
- A 128 MiB default refuses many JPEG-region archives that would decode within 128 MiB (H6): the declaration, not the content, drives refusals.
- The planner fails packs instead of falling back (H4).
- Superlinear CPU on crafted JPEG XL (H5): `work-unit-timeouts` or a narrower transform are needed.

## 15. Threats to validity

- **Allocator attribution completeness.** Dependency-internal thread pools are not used (single thread frozen). Allocations from `std` I/O buffers are attributed to "reader buffers". Validated by control patterns.
- **External C++ backends** lack allocator instrumentation. RSS is their only memory evidence (floor-limited), so comparisons with Rust backends on OD-08 are secondary.
- **cgroup emulation ≠ device.** Page cache, allocator behaviour and swap differ on real low-memory hardware (EXP-RECON-012).
- **Generated scaling content** may not match real worst cases. The adversarial-search artifact (common §11) extends the series by seeded mutation toward peak maximisation: a hill climb on megapixels, sampling and scan layout, 500 steps per mode, seed 20260926.
- All other threats: common §13.

## 16. Held-out placeholder

Decode peaks, tightness and ceiling refusal fractions of frozen candidates are measured on held-out archives once after Commit C (R5 (a) HC screens; (d) non-inferiority of the selected ceiling policy's refusal fraction).

## 17. Cost and effort estimates

- **Compute.** About 60 CPU-hours: generated series about 2,000 cases × steps × backends × 2 repetitions; decode probes on about 450 real archives × 2; cgroup arm 5 limits × region and DEFLATE archives; CPU scaling 15 points × 3 runs × 2 steps.
- **Disk.** Generated cases about 25 GB (100 MP JPEGs; 64 MiB streams); results about 2 GB.
- **Tooling.** Counting allocator (program-wide gate), 4 subcommands, generator, cgroup wrapper: about two harness sessions.
- **Agent effort.** Execution: none. Analysis: judgment (composition and definition admissibility), with independent sign-off of the `profile-scaled` reader-side exclusion.

## 18. Looks, deviations and outcome (append-only)

- Look 1: registered with EXP-RECON-004 look 1 (DEC-CON-005/DEC-CMP-016 ceiling and limit comparisons).
- Deviations: none.
- Outcome: `outcome.json`.
