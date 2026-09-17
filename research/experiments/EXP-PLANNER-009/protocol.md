# EXP-PLANNER-009: Encoder output identity across versions, CPU dispatch paths and emulated ARM64

| Field | Value |
|---|---|
| Domain | planner (program §9, §10) |
| Index status | NEEDS_TOOLING (TP-10 `research/planner/tools/encoder-drift/`, TP-11 installs of `qemu-user-static`, `gcc-aarch64-linux-gnu` and the `aarch64-unknown-linux-gnu` target, TP-06 for host packs, TP-18 cgroup wrapper, EXP-PLANNER-008 bundles, the §14 item 7 cost scripts) |
| Document state | Phase C design; `record.md` after gate G-A |
| Shared conventions | `research/planner/README.md` |
| Runner specs | `spec.yaml` (encoder-version drift matrix, WSL native); `spec-hosts.yaml` (host-class identity: native, emulated x86-64 CPU models, emulated aarch64, memory ceilings, locale and time zone); `spec-windows.yaml` (Windows native identity of plan tables and Chunk payloads) |

## 1. Pre-registration header

- **decision_ids:**
  - DEC-CMP-031 (primary): what a frozen planner ID promises, and encoder pinning and recording.
  - DEC-CMP-019: the reproducibility matrix under the declared resource model across host classes, memory ceilings, locale and time zone.
  - DEC-CMP-040: whether replanning with a pinned historical planner can reproduce prior output across encoder releases.
- **Decision type (proposed):** EMPIRICAL for drift and identity rates and costs. FORMAL for what each promise means and whether measurements contradict it; the counterexample-search protocol is this experiment. Reviewer sign-off by a session not involved is required (§1.2).
- **Proposed `od_ids`:** OD-04 (deterministic interpretation), OD-20 (deterministic migration, via DEC-CMP-040), OD-22 (dependency longevity cost inputs), OD-05 (bytes of recorded encoder versions). **Proposed `hc_ids`:** HC-11 (decoded output invariant under CPU-feature and architecture variation, including library-version-defined reconstruction decode steps), HC-13 (deterministic modes: identical PCI within a host class; identical LAI and PCR across classes per MVT-13(a)), HC-16 (frozen-ID goldens).
- **Author / L7:** README §10.

## 2. Question and hypotheses

- **Question.** Do the production encoders produce byte-identical output at frozen parameters:
  - across their released versions;
  - across CPU dispatch paths (x86-64 feature levels);
  - across architectures (emulated aarch64);
  - across compilers and hosts (MSVC Windows against GCC WSL);
  - under different memory ceilings, locales and time zones?
  Given the answers, which frozen-planner-ID promise can production actually keep, at what dependency and specification cost?
- **H1.**
  1. At least one libzstd minor release in the matrix changes Chunk payload bytes at some frozen level, and some of those changes alter plan decisions (a candidate whose size ranking flips).
  2. Pinned builds are byte-identical across `qemu-x86_64 -cpu Nehalem/Haswell/max` and native, and across emulated aarch64, for Chunk payloads and PCI.
  3. MSVC Windows and GCC WSL builds give identical Chunk payload and plan-table bytes (PCI differs only through the F-0001/F-0002 metadata classes).
  4. Memory ceilings, locale and time zone never change PCI.
  5. The library-version-defined decode steps (preflate-rs inverse, jixel/jxl-oxide reconstruction) produce identical decoded bytes across the versions in the matrix. Any difference makes the version pin load-bearing (HC-11 gated-feature rule).
- **H0.** No drift anywhere, so `status-quo-cargo-pins` keeps a full byte-identity promise with no extra cost.
- **Falsification.** H1.1 fails if no release changes bytes. H1.2 fails on any payload or PCI difference between pinned emulated and native builds, which is an HC-13 or HC-11 violation artifact committed without rerun (§4.13). H1.3 fails on any payload or plan-table difference between hosts. H1.4 fails on any PCI difference under ceilings, locale or time zone. H1.5 fails on any decoded-byte difference, committed as an HC-11 artifact.

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CMP-031 | Output drift of zstd, lz4_flex, lzma-rust2, preflate-rs and jixel across releases at fixed parameters; plan-decision drift; cross-dispatch and cross-architecture (emulated) byte identity; golden vectors per planner ID across upgrades (HC-16 MVT-16(c)); cost of vendored encoders (C3: native lines, `unsafe` counts, maintainers, advisories; C7 churn per year) | Native ARM64 and macOS identity: EXP-PLANNER-011 (BLOCKED); overall byte-determinism promise scope: model domain |
| DEC-CMP-019 | Planner output identity across host classes, memory ceilings, locale and time zone under the declared model; recorded resource-model bytes (with EXP-PLANNER-006) | Thread-count variation of deterministic parallel prototypes: scale (§14); `verify --reproducible`: EXP-PLANNER-010 and integrity (§24) |
| DEC-CMP-040 | Whether a pinned historical planner ID still reproduces output after encoder upgrades (the drift matrix applied to planner generations v4-v6) | Replan policies: EXP-PLANNER-010 |
| DEC-CMP-029 (secondary) | Determinism of persisted reconstruction audit records across host classes (`audit_records_sha256` in `spec-hosts.yaml` and `spec-windows.yaml`) | Decision-log sizes and explain fidelity: EXP-PLANNER-010 |

## 4. Candidates (DEC-CMP-031 promise policies)

| candidate_id | Promise | Measured consequence here | Expected tier |
|---|---|---|---|
| status-quo-cargo-pins (reference) | Exact Cargo pins; IDs do not record encoder versions; same-machine determinism | Violated if any in-scope host class or allowed upgrade changes bytes without a new ID | status quo |
| record-encoder-versions | Encoder name and version recorded per plan or in the planner ID | Recording bytes per archive (`artifact_bytes` delta), C1 words | T2 (new field) |
| decision-only-freeze | Plan decisions and boundaries frozen; compressed bytes may change | Contradicted if drift flips decisions (H1.1) | T1 |
| vendored-encoders | Frozen encoder sources vendored per planner ID | C3/C4 costs of vendored C and Rust sources, native `unsafe`, build complexity | T3 (dependency with native code) |
| new-id-on-any-encoder-change | Any encoder change mints a new planner ID | C7 churn: number of output-changing releases per 24 months in the matrix | T1 per change (recurring) |
| toolchain-scoped-reproducibility | Byte reproducibility only for a declared writer build | Holds by construction within a build; cross-host classes reported as outside the promise | T1 plus C1 words |
| critic-proposed-n | R0 item 6 slot | – | – |

- **Reference candidate:** status-quo-cargo-pins.
- **Excluded:** none. The SPEC §5.7 statement "planner ID pins its encoder versions" is a constraint every candidate must honour or explicitly re-scope. `decision-only-freeze` re-scopes it and is flagged as a SPEC-revision candidate.

## 5. Corpus and matrices

- **Encoder inputs:** EXP-PLANNER-008 decode bundles (tuning; up to 64 Chunks per (family, category); plaintext only).
- **Version matrix (TP-10 `make_projects.py`):** for each crate, the pinned version plus every release in the 24 months before the record date and the latest at record time, capped at 4 versions per crate. The exact list is committed in the record before execution:
  - zstd via `zstd-sys` bundling libzstd 1.5.5, 1.5.6, 1.5.7 (pinned) and the latest;
  - `lz4_flex` (pinned 0.14.0);
  - `lzma-rust2` (pinned 0.20.0);
  - `preflate-rs` (pinned 0.7.6), encode and inverse;
  - `jixel` (pinned 0.2.19) and `jxl-oxide` (pinned 0.12.6), encode and decode.
- **Configurations:** every v4-v6 frozen configuration (the EXP-PLANNER-008 list), plus v1-v3 Zstandard levels.
- **Host-class matrix (pinned build):** native WSL x86-64; `qemu-x86_64 -cpu Nehalem`, `-cpu Haswell`, `-cpu max`; `qemu-aarch64` (cross-compiled with `aarch64-linux-gnu-gcc`; labelled `EMULATED`, correctness only); cgroup `memory.max` 2 GiB and 8 GiB; `LANG=C.UTF-8 TZ=UTC` against `LANG=tr_TR.UTF-8 TZ=Pacific/Kiritimati`.
- **Host packs:** G1-PSUB-64 sub-items at most 16 MiB (small parent tier) for emulated classes; all G1-PSUB-64 sub-items for native, cgroup and locale classes. Profiles fast, balanced and dense (extreme on native, cgroup and locale classes only, since emulation cost is prohibitive; recorded as a limitation).
- **Windows:** the same G1-PSUB-64 small sub-items, native MSVC build, fast and balanced.

## 6. Environment and platform

- WSL root for cgroups, the qemu user-mode binfmt-free invocation (`qemu-aarch64 -L /usr/aarch64-linux-gnu`) and apt installs (downloads approved; PROGRESS standing decision).
- Windows non-admin native build.
- No Docker needed: user-mode qemu runs directly.
- Platforms: Linux x86-64 (native plus emulated models), emulated ARM64, Windows x86-64. Native ARM64 and macOS are BLOCKED (EXP-PLANNER-011).
- Deterministic metrics only; no timing.

## 7. Commands

```sh
apt-get install -y qemu-user-static gcc-aarch64-linux-gnu   # TP-11, inside WSL (root)
/root/.cargo/bin/rustup target add aarch64-unknown-linux-gnu --toolchain 1.98.1
python3 research/planner/tools/encoder-drift/make_projects.py --versions research/experiments/EXP-PLANNER-009/versions.json \
  --out /root/eb-research/build/encoder-drift
bash research/planner/tools/encoder-drift/build_all.sh /root/eb-research/build/encoder-drift
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-PLANNER-009/spec.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-PLANNER-009/spec-hosts.yaml
# Windows host
python -m ebr run --spec research/experiments/EXP-PLANNER-009/spec-windows.yaml
python3 research/planner/tools/encoder-drift/analyze.py --normalized research/normalized/EXP-PLANNER-009 \
  --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces --out research/normalized/EXP-PLANNER-009/decision/
# C1-C3 costs of vendored-encoders: the §14 item 7 cost scripts (shared gate; path fixed by their builder) over /root/eb-research/build/encoder-drift/vendored
```

`versions.json` is written by the record author from crates.io release listings at record time and committed before execution.

## 8. Seeds, warmup, replication

Seeds: order 260917091, bootstrap 260917092, command 260917093. Deterministic: 0 warmups, 2 repetitions per cell. The pinned native build gets the §4.13 repeat-hash check. Determinism-claim cells (host classes) use 3 repetitions per variation cell, plus one 20-repetition stress cell at the host's maximum thread count with seeded background CPU load on native WSL (objective MVT-13(a); `protocol.determinism_claim`). Any difference within a claimed-deterministic cell is committed as the violation artifact without rerun.

## 9. Metrics

| Metric | OD/HC | Role | Family | Source |
|---|---|---|---|---|
| per-(Chunk, configuration) output SHA-256 equality with the pinned native build | HC-13, HC-16 | binary within a claimed promise; descriptive drift fraction across versions (`diag_encoder_drift_fraction`) | correctness | TP-10 |
| decoded-byte equality of reconstruction inverses across versions and hosts | HC-11 | binary | correctness | TP-10 |
| plan-decision drift fraction (selections recomputed with drifted sizes via TP-03) | – | descriptive; contradicts `decision-only-freeze` when > 0 | other | `analyze.py` |
| PCI equality (within host class), PCR, LAI and plan-table equality (across classes) | HC-13 | binary per MVT-13(a) | correctness | TP-06 host packs |
| `artifact_bytes` delta of recorded encoder versions | OD-05 | primary for record-encoder-versions against status quo | size (T-01) | modelled encoding counter (C1) plus TP-06 `--set record_encoder_versions=true` |
| `decode_path_dependency_count`, `decode_path_native_unsafe_count`, `rustsec_advisory_count`, `maintainer_count`, `msrv_gap`, `release_age_years` | OD-22 | cost_input / descriptive | other | §14 item 7 scripts |
| output-changing releases per 24 months | OD-22 (C7) | cost input | other | `analyze.py` |

## 10. Normalization and statistical analysis

- **R1 (formal screen).** A candidate whose promise is contradicted by a committed counterexample is eliminated at R1: for example status-quo-cargo-pins promising cross-dispatch identity when a dispatch difference exists, or decision-only-freeze when decisions drift under an allowed upgrade. The counterexample and a reviewer not involved are recorded (R1(b)).
- **R6/R10 among survivors.** Computed tiers from the cost scripts, then R10 keys. `artifact_bytes` of recorded versions is compared with the status quo under T-01 (deterministic, exact item values, §4.9 aggregation) and must clear the T2 minimum gain only if it claims a gain. It claims none: it is a cost, so it enters only through R4 non-inferiority and R10.
- **Descriptive tables:** drift matrix (crate × version × configuration × family: fraction of Chunks changed); decision-drift matrix; host-class identity matrix; churn per year.

## 11. Sensitivity analysis

- Version window at 12 against 24 months.
- Configuration subsets: production-selected configurations only (weighted by EXP-PLANNER-001 usage shares) against all frozen configurations.
- The §6.5 family arms on drift fractions (descriptive).
- No weights are involved.

## 12. Expected negative results worth recording

- libzstd upgrades change high-level outputs, so status-quo pins cannot promise byte stability across dependency updates.
- Emulated aarch64 identical to native, with the claim capped at `EMULATED` and native unverified.
- `decision-only-freeze` contradicted on a small fraction of Chunks.
- Reconstruction decode steps stable across versions, which weakens the case for version-pinning them, or unstable, which confirms HC-11's gating.

## 13. Threats to validity

- Emulation reproduces instruction semantics, not microcode or SIMD timing. Identity under qemu is necessary but not sufficient evidence for native aarch64.
- qemu CPU models may not disable every dispatch path (runtime detection via cpuid is honoured, but some features may be emulated).
- The version matrix is limited to 4 versions per crate.
- MSVC and GCC comparisons cover Chunk payloads, not full PCI (F-0001).
- Bundles, not whole archives, carry the version drift results; archive-level PCI drift is checked on G1-PSUB-64 only.
- Designer L7 exposure.

## 14. Cost and effort

- **Compute:** builds about 4 CPU-hours; drift matrix about 12 builds × about 1 GiB of bundles × 24 configurations × 2 repetitions, about 35 CPU-hours; host-class packs (native, cgroup, locale, stress about 10 CPU-hours; emulated about 15 CPU-hours); Windows about 5 hours.
- **Disk:** builds and toolchains about 10 GB; hash outputs about 1 GB.
- **Agent effort:** scripted; judgment for the promise analysis and the R1(b) reviewer coordination.

## 15. Gates and status

- **Tooling:** TP-10, TP-11, TP-06, TP-18, EXP-PLANNER-008 bundles, §14 item 7 cost scripts.
- **Gates:** G-A, G-B, an independent reviewer for R1(b) eliminations.
- **Blocked:** native ARM64 and macOS in EXP-PLANNER-011.
- **Held-out:** not applicable to the FORMAL parts; the EMPIRICAL drift rates are re-checked on held-out bundles in EXP-PLANNER-012.
