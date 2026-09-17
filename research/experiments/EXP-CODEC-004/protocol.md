# EXP-CODEC-004: Structural transforms: gain per data class, false-positive cost and applicability-gate accuracy

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CODEC-004 |
| Domain | codecs (program section 8.7; program Task 8.7, REQ-CMP-0132) |
| Status | NEEDS_TOOLING. Needs these harness transforms: float-field split, delta with distance, the BCJ filters beyond the existing x86/ARM64/RISC-V (ARM32, ARM-Thumb, PowerPC, SPARC, IA-64), and multi-step pipelines. Also needs the applicability gates, `ebr-codec transform-matrix`, which reuses the EXP-CODEC-001 chunk-matrix machinery, and the `ebr-planner portfolio --transform-set` extension. |
| Stage | Stage A screen (tuning sample, exploratory) → Stage B exact (tuning selection, validation look). Timing of trial cost and inverse decode runs in EXP-CODEC-002. Held-out: Phase D placeholder. |
| decision_ids | DEC-CMP-043 (which structural transforms ship in v1, with what parameters) |
| Scope boundary | The planner's gating decision (DEC-CMP-012: frozen probe, retuned probe, entropy gate, always-try, per-category scope) is EXP-PLANNER-004 arm B, plus EXP-RECON-002/003 for reconstruction gating. Transform entries of the frozen profile tables (DEC-CMP-034) are EXP-PLANNER-004 arm A. The generation-level share of v4 savings (DEC-CMP-023) is EXP-RECON-004. This experiment evaluates **new** transform families together with the applicability rule each needs (the ledger's "per detected architecture" for BCJ), and measures transform-level false-positive cost as program Task 8.7 requires. Its per-transform gate-accuracy tables are offered to EXP-PLANNER-004 as inputs. |
| Proposed decision type | EMPIRICAL (size, cost, applicability accuracy); FORMAL for exact invertibility (proof plus exhaustive tail-length test, as for the existing transforms) |
| Proposed od_ids | OD-05 (`artifact_bytes`), OD-06 (`encode_wall_s`), OD-07 (`decode_wall_s` guard), OD-23 (cost input) |
| Proposed hc_ids | HC-02 (inverse exactness), HC-06 (MVT-06(d) CPU per byte of applicability gates on adversarial inputs), HC-08 (applicability never by filename or extension, SPEC §5.2), HC-12 (every transform needs a normative definition), HC-13 (deterministic applicability decisions), HC-16 (`delta8/v1`, `byte-shuffle/v1` frozen) |
| Method, gates, author, L7 | EXP-CODEC-001 section 1 and Appendix C. |

## 2. Question

For each data class (corpus family, and the detected architecture or numeric type within it), how much does each structural transform family, and each pre-registered combination, reduce `artifact_bytes` at production chunk scope ahead of each profile's codecs?

- **Trial cost.** What does a transform cost when it is tried on chunks where it does not win, in trial encodes and time?
- **Wrong application.** What does it cost when applied without trial where it loses, in bytes?
- **Applicability accuracy.** How accurately does each transform's applicability rule predict where the transform pays off?

## 3. Hypotheses

- **H1a (BCJ).** Architecture-matched BCJ (x86 on x86-64 ELF/PE, ARM64 on AArch64 ELF/Mach-O, RISC-V on riscv64 ELF) ahead of LZMA2 6/4M or zstd-15 is `DISTINGUISHABLE_BETTER` on `artifact_bytes` in F09. The predicted ratio is 0.90-0.96, which clears the §7.3 "New reversible transform" band in F09. The design smoke found an x86 BCJ gain before zstd-3 on `/usr/bin/ls` (NOT_DECISION_GRADE).
- **H1b (float split and byte-plane).** On F07 float tensors and arrays (bf16/fp16/fp32 safetensors, float32 SDRBench), float-field split or byte-plane split at the element width beats the status-quo `byte-shuffle-4/8` by at least 1 band unit, and clears the transform band against no transform. Delta-of-delta clears it only on smooth series. The smoke found `delta-of-delta-4` better than `byte-shuffle-4` on a float32 sine ramp (NOT_DECISION_GRADE).
- **H1c (delta distance).** Delta at distance d > 1 (xz Delta filter) beats `delta8` (d = 1) by at least 1 band unit only on fixed-record numeric data (F07/F08 records of width d). Elsewhere it is `EQUIVALENT` or worse.
- **H1d (multi-step).** Delta-then-shuffle combinations add less than 1 band unit over the best single step, except on F07 integer arrays.
- **H1e (applicability).** The signature applicability rule for BCJ and float split reaches the always-try `artifact_bytes` within 1 band unit on target families, with at least 50% fewer trial encodes than always-try. The frozen sampling probe misses at least 5% of the plaintext bytes where a new transform wins. The reason: its "adjacent repeat" samples bytes about len/4096 apart (REQ-CMP-0035).
- **H1f (non-target cost).** Under always-try, non-target families are `NON_INFERIOR` on `artifact_bytes` (by construction of the per-chunk minimum, up to plan-record bytes), but their `encode_wall_s` fails non-inferiority at the §7.3 k = 2 band at dense and extreme. This is the false-positive cost that an applicability rule must remove.
- **H0.** Every new transform arm is `EQUIVALENT` to status quo in every family.
- **Falsification.** At the validation look, the verdict class differs from the one stated for the named family and scope.
- **Exactness control (binary, HC-02).** Before Stage A, every transform must round-trip on every chunk and pass an exhaustive tail-length test: every length from 0 to 4 × element width + 1, over pseudo-random bytes. A transform that fails either is excluded (R1).
- **Identity controls (binary).** `delta-distance(1)` must equal `delta8/v1` byte for byte, and `byte-plane-split(2,4,8)` must equal `byte-shuffle/v1`. The harness BCJ output must equal `xz --format=raw --filters=<arch>,lzma2` filter output on the F09 tuning sample. Any mismatch is a harness defect.
- **Negative control.** On CSPRNG items (`f20-tuning-csprng`), no transform is ever selected, and the applicability rules classify at least 99% of bytes as not applicable.

## 4. Candidates

### 4.1 DEC-CMP-043 (ledger candidates) and transform configurations

| candidate_id | Transform configurations (`configs/transform-universe.json`) | Applicability rule (`configs/applicability.json`) |
|---|---|---|
| status-quo-delta8-shuffle (reference) | `delta8/v1`; `byte-shuffle/v1` w ∈ {2, 4, 8}; one step per family | frozen production probe (as in v4) |
| add-bcj-family | BCJ x86, ARM32, ARM-Thumb, ARM64, PowerPC, SPARC, IA-64, RISC-V (lzma-rust2 filters, pinned =0.20.0; harness ports where missing) | "per detected architecture": the ContentObject header signature (ELF `e_machine`, PE `Machine`, Mach-O `cputype` including fat slices) selects the matching BCJ |
| add-delta-distance | xz-Delta semantics, distance d ∈ {1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 64, 128, 256} | frozen production probe; declared integer dtype width from tensor/array headers where present |
| add-float-split | sign/exponent/mantissa stream separation (ZipNN class) for FP16, BF16, FP32 and FP64, little-endian; tail bytes copied | dtype from safetensors/GGUF/npy/FITS/HDF5 headers |
| add-stream-split | byte-plane (column) split at widths w ∈ {3, 5, 6, 7, 12, 16, 24, 32} (widths 2/4/8 are `byte-shuffle`); delta-of-delta w ∈ {1, 2, 4, 8} | frozen production probe; dtype width from headers where present |
| multi-step-combinations | pre-registered list: delta-d then byte-plane-d for d ∈ {2, 4, 8}; byte-plane-w then per-lane delta8 for w ∈ {4, 8}; float-split then delta8 on the exponent stream (FP16/BF16/FP32) | as the first step's rule |
| drop-structural | no transform | — |

Each candidate is also evaluated under **always-try** (an oracle arm for size) and under **no rule** (all chunks transformed, the worst-case arm). These sensitivity arms bound the value of the applicability rule; they are not DEC-CMP-012 candidates.

R0 gaps are flagged for the critic, not added here:

- The strongest incumbent technique: the 7-Zip/xz filter chains BCJ + LZMA2 and Delta + LZMA2. These are measured as `add-bcj-family` and `add-delta-distance` over LZMA2. BCJ2, a four-stream filter, is flagged and not measured.
- "Defer", meaning keep only the frozen transforms.
- A critic candidate.

`extension-based` applicability is excluded at L0 (SPEC §5.2; docs/planner-v1.md) and is not measured.

## 5. Corpus

- **Stage A.** Tuning split, an object-preserving 64 MiB sample per item (seed 20260917), chunking at dense and extreme.
- **Stage B.** Complete tuning items (selection), then complete validation items (one registered look).
- **Target families by hypothesis.** F09 (BCJ); F07 and F08 (float, delta, byte-plane); F10 and F16 (embedded executables and firmware). Every other family is non-target and is still measured, to charge false-positive cost (§4.9 applicability rule).
- **Data-class strata within families** come from byte signatures only and are recorded per chunk.
- **Adversarial inputs.** Generated under seed 20260917 and lock-filed under `/root/eb-research/generated/EXP-CODEC-004/adversarial-v1/`:
  - spoofed ELF, PE and Mach-O headers followed by CSPRNG bytes;
  - a safetensors header declaring 10^12 FP16 elements over a short payload;
  - npy headers with a mismatched dtype;
  - "probe-passing noise": uniform bytes over a 192-symbol alphabet;
  - the F20 `gear-extremes` and `csprng` tuning items.

  These drive the MVT-06(d) CPU-per-byte series of the applicability rules, from 2^10 to 2^20 objects or chunks.

## 6. Environment

WSL2 x86-64, deterministic size measurement. Timing arms run in EXP-CODEC-002 (Windows and WSL).

## 7. Tooling and commands

### 7.1 To build

1. Additions to `ebr-codec/src/transform.rs`: `float_field_split(kind)`, `delta_distance(d)`, BCJ ARM32/ARM-Thumb/PowerPC/SPARC/IA-64 (via lzma-rust2 filters where exposed), and a multi-step pipeline runner. Each comes with exhaustive tail-length round-trip tests and the identity controls.
2. `ebr-codec transform-matrix --universe <transform-universe.json> --codecs stage_a|profile-tables --production-chunking object --detail-out ... --summary-stdout`. For each chunk it runs every (transform, codec) cell and the untransformed reference cells, recording stored bytes, plan-record model bytes, round trip and the applicability-rule inputs.
3. `ebr-codec/src/applicability.rs`: signature and dtype detection plus the frozen-probe adapter. These are pure functions of chunk and object bytes, with every parameter recorded.
4. `ebr-planner portfolio --transform-set <set> --applicability <rule|always-try|none>`, an extension of the EXP-CODEC-001 tool. It applies the rule to decide which transform cells are tried, then the production v4 qualification rule, and reports `artifact_bytes`, trial encodes, and applicability confusion counts against the always-try oracle.
5. `ebr-codec applicability-cost` (adversarial CPU-per-byte series).
6. `research/experiments/EXP-CODEC-004/gen_adversarial.py` with its lock file; `screen.py` (the Stage A rule of section 12.1).

### 7.2 Commands

```sh
PY=/root/eb-research/venv/bin/python
for S in spec-screen.yaml spec.yaml spec-adversarial.yaml; do
  $PY -m ebr validate --spec research/experiments/EXP-CODEC-004/$S
done
# Stage A -> screen.py -> commit configs/transform-universe-stageB.json -> Stage B tuning -> validation look
```

## 8. Seeds

Order, bootstrap and sample seeds are 20260917. The adversarial generator seed is 20260917.

## 9. Metrics

| Metric | OD | Role | Family | Threshold |
|---|---|---|---|---|
| `artifact_bytes` | OD-05 | primary (target and non-target families) | size | T-01 |
| `encode_wall_s` | OD-06 | §7.3 non-target guard (joined from EXP-CODEC-002) | time_wall | T-03 (profile) |
| `decode_wall_s` | OD-07 | guard (joined from EXP-CODEC-002) | time_wall | T-04 |
| `roundtrip_mismatches` | OD-02 | binary | correctness | §3.3 |
| `hostile_cpu_scaling_exponent` | OD-17 | descriptive (applicability-rule CPU scaling on the adversarial series) | other | — |
| trial encodes per chunk; trials not selected; selection-loss bytes (applied without trial, and it lost); missed-saving bytes (not applied where the always-try oracle wins); applicability TP/FP/FN/TN over plaintext bytes per family; transform win-byte fraction per data class | — | descriptive, non-canonical, report-only | other | — |

## 10. Replication

Deterministic: 2 repetitions plus a repeat-hash on `matrix_digest_sha256` and on the applicability-decision digests (CC-7). Applicability decisions must be deterministic (HC-13); a mismatch is a violation artifact.

## 11. Normalization and aggregation

CC-9, stratified by profile and tier. Data-class strata are reported but are never averaged into selection. Selection scopes follow R9 expressibility: per-family selection is expressible only where the product categorises content (SPEC §6.1: fast performs no deep categorisation).

## 12. Statistical analysis

### 12.1 Stage A screening rule (mechanical)

A transform configuration advances if any of the following holds on the tuning sample:

- (a) it is status quo;
- (b) it is selected on at least 0.5% of sampled plaintext bytes in some family at some profile under always-try;
- (c) its best-profile family group-mean `artifact_bytes` log ratio against no transform is at least 1 T-01 band unit;
- (d) it is the best configuration of a ledger candidate (BCJ, delta distance, float split, stream split, multi-step).

The cap is 20 configurations. Configurations admitted under (a) or (d) are never dropped.

### 12.2 Stage B

- **Screens before comparison.** R1 (exactness and identity controls), then R2: a normative definition exists and every parameter is data (EXP-CODEC-005).
- **R4 comparison.** On `artifact_bytes` in target families, and on the §7.3 "New reversible transform" non-target conditions: `artifact_bytes` and `encode_wall_s` `NON_INFERIOR` at the k = 2 band.
- **R6 with the computed tier.** A new transform in the baseline set is T3; in a registered extended set it is T2. Set membership follows DEC-CMP-008/009 and EXP-CODEC-005.
- **Applicability-rule gate.** A new transform whose applicability rule fails the MVT-06(d) CPU-per-byte linearity test is eliminated at R1 together with that rule.
- **Look budget, MDE and confirmatory set.** CC-10.

## 13. Practical-significance thresholds

T-01, T-03 (per profile), T-04; the §7.3 "New reversible transform" row; MVT-06(d) δ from Appendix C.1.

## 14. Sensitivity

CC-11, plus:

- always-try against the applicability rule against no rule;
- oracle minimum against the production qualification rule;
- reference codec set: production tables against the best Stage B codec portfolio from EXP-CODEC-001;
- leave-F09-out and leave-F07-out arms.

## 15. Held-out (Phase D placeholder)

CC-12. Applicability-rule parameters are frozen at Commit A.

## 16. Expected negative results

- Most transforms are never selected outside their target family, while trial cost is paid everywhere without an applicability rule.
- Delta distances above 8 rarely win.
- Multi-step pipelines add complexity for sub-band gains.
- The frozen probe misses compressible numeric data. Its false negatives go to EXP-PLANNER-004 as counterevidence against the frozen v4 probe.

## 17. Threats to validity

- The plan-record model for new transforms (CC-6).
- lzma-rust2 BCJ ports may differ from xz's reference filters. The identity control addresses this.
- Signature detection on real files depends on well-formed headers. Malformed or stripped binaries fall back to the probe, which is recorded.
- F07 typed-data coverage per group is uneven (safetensors-heavy). Single-group data classes cannot support scoped winners (§4.9).

## 18. Platform and cost

| Item | Value |
|---|---|
| Platforms | Linux x86-64 WSL2 (size, deterministic). No network, no privileges. |
| Compute | Stage A ≈ 35 CPU-h (three reference codecs per chunk); Stage B ≈ 250 CPU-h (≤ 20 transform configurations × profile codec tables × complete items at balanced, dense and extreme); adversarial series ≈ 3 CPU-h. Total ≈ 290 CPU-h (≈ 18-20 h wall at 16 workers). |
| Disk | Detail rows ≈ 4 GB gzip; adversarial set ≈ 2 GB; scratch ≈ 10 GB. |
| Agent effort | Scripted. Judgment: review of the exhaustive tests for new transforms; analysis by a session without the L7 exposure. |

## 19. Deviations (append-only)

None.
