# EXP-CHUNK-013: Cross-architecture boundary identity and ARM64 chunking throughput (BLOCKED)

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CHUNK-013 |
| Status | **BLOCKED.** Three independent blockers, each named precisely below. |
| Blocker 1 (platform) | **Native ARM64 hardware is PLATFORM_BLOCKED:** no local ARM64 machine, and hosted runners were declined (PROGRESS standing decision 2026-09-12). This blocks arm N (native throughput and native boundary identity). |
| Blocker 2 (tooling/platform) | **QEMU linux/arm64 emulation is unavailable:** Docker Desktop fails to start on this host (PROGRESS storage incident: stale AF_UNIX socket reparse points, Win32 error 1920). This blocks arm Q (emulated boundary identity, correctness only). **Unblock route without Docker:** install `qemu-user-static` in WSL (apt; downloads approved), cross-compile the harness for `aarch64-unknown-linux-gnu` with the pinned toolchain (`rustup target add` plus the `gcc-aarch64-linux-gnu` linker), and run binaries under `qemu-aarch64-static` explicitly. Then re-label arm Q READY by amendment. |
| Blocker 3 (policy) | **An accelerated SIMD chunker (VectorCDC-style) cannot be built** in the harness: workspace lints are `unsafe_code = "forbid"` (mirroring the production freeze F-23), `std::arch` intrinsics require `unsafe`, and portable SIMD (`std::simd`) is nightly-only. This blocks the `vectorcdc-vram-neon` and x86 AVX2 accelerated-path cells of arms N and Q. **Unblock route:** an owner decision to allow a research-only crate outside the harness workspace with `unsafe` permitted, or a nightly `portable_simd` research build labelled as such. The production consequence is recorded either way: a SIMD chunker in production needs a dependency containing `unsafe` (tier T3, §7.2) or a nightly feature, which the product excludes. |
| Arms | **N:** native ARM64 throughput and boundary identity (`spec.yaml`). **Q:** QEMU-emulated ARM64 boundary identity, correctness only, never timing (§4.1) (`spec-qemu.yaml`, generated when unblocked). **A:** x86-64 accelerated path vs scalar identity (`spec-x86-simd.yaml`, blocked by blocker 3). |
| decision_ids | DEC-CMP-001 ("Scalar vs accelerated boundary identity across ISAs where applicable"; ARM64 throughput), DEC-CMP-005 ("Cross-ISA differential results for any accelerated chunker"), DEC-CRY-023 ("Chunking throughput … with/without AES-NI and emulated ARM64") |
| Consequence for decisions | A DEC-CMP-001 or DEC-CRY-023 selection whose scope includes ARM64 performance cannot be `DECIDED` while blocker 1 holds (`decision-method.md` §8.4: a `PLATFORM_BLOCKED` platform inside the scope). The decision must exclude ARM64 performance from `decision_scope` explicitly, or stay `INSUFFICIENT_EVIDENCE`. Boundary identity (HC-08, HC-11) is a property, not a platform: its HC_UNVERIFIED status on ARM64 can be tolerated only by scoping out ARM64 (objective §2.0 rule 4). The remaining-unknowns report cites this experiment. |
| Gates / author / L7 | As EXP-CHUNK-001 section 1 |

## 2. Question

On ARM64, do CDC constructions and keyed boundary modes produce boundaries identical to x86-64, on both scalar and accelerated paths? What chunk-stage throughput do they reach there, including PHTE with and without ARMv8 cryptography extensions, and a NEON accelerated VectorCDC-style path?

## 3. Hypotheses

- **H1 (binary; HC-08/HC-11 for creation determinism, HC-13).** `boundary_set_sha256` on ARM64 (native in arm N, emulated in arm Q) equals the x86-64 value from EXP-CHUNK-008/012 for every candidate and item, including accelerated paths against their scalar references. **Violation:** any difference. The artifact is the item, candidate and both boundary lists.
- **H2 (throughput, arm N only).** The EXP-CHUNK-008 arm T1 ordering of constructions holds on ARM64 (Kendall τ ≥ 0.8 between architectures, descriptive). PHTE with hardware AES is within 1 band unit of the secret Gear table in chunk-stage throughput; with software AES it is worse by more than 1 band unit. **Falsified** where the corpus verdicts differ.
- **H3 (accelerated path).** `vectorcdc-vram-neon` is `DISTINGUISHABLE_BETTER` than `vectorcdc-vram-scalar` in chunk-stage throughput and has identical boundaries. **Falsified** if boundaries differ (a binary violation) or throughput is not better.
- **H0.** No identity differences; throughput differences within band.

## 4. Decisions informed

| Decision | Contribution when unblocked |
|---|---|
| DEC-CMP-001 | Cross-ISA scalar/accelerated identity; ARM64 OD-06 evidence; the independent-implementability cost of SIMD paths |
| DEC-CMP-005 | `accelerated-path-differential` candidate: the differential test itself |
| DEC-CRY-023 | PHTE throughput without x86 AES-NI on ARM64 (hardware vs software AES) |

## 5. Candidates

| Candidate | Definition | Blocked by |
|---|---|---|
| `gear-norm-v1` | Status quo, scalar (**reference**) | 1 (N), 2 (Q) |
| `gear-spread-nc2`, `fastcdc-2020-nc2` | Scalar successors (EXP-CHUNK-002) | 1, 2 |
| `vectorcdc-vram-scalar` | Scalar VRAM rule (EXP-CHUNK-002) | 1, 2 |
| `vectorcdc-vram-neon` | NEON-accelerated VRAM rule (DEC-CMP-001 `vectorcdc-simd`) | 1, 2, 3 |
| `secret-gear-table` | Encrypted status quo | 1, 2 |
| `phte-aes128-armv8-crypto`, `phte-aes128-softaes` | PHTE with the `aes` crate's hardware backend vs forced software backend | 1 (and 2 for identity) |

- **Excluded:** as EXP-CHUNK-002 section 5 (`quickcdc-feature-acceptance` R1; `fixed-size` never selectable).
- Successor candidates are replaced by DEC-CMP-001's S by amendment.

## 6. Corpus

- **Tuning:** the EXP-CHUNK-008 40-item timing subset (ids in `spec.yaml`).
- **Arm Q:** tuning items ≤ 256 MiB (EXP-CHUNK-012 arm S selection), with a 64 MiB prefix per file because emulation is slow.
- **Validation:** the same selection rule on validation as an HC screen when unblocked.
- **Held-out:** Phase D placeholder (§5.5).

## 7. Environment and platform requirements

| Arm | Requirement | Available here |
|---|---|---|
| N | Native linux/aarch64 (≥ 4 cores, ARMv8.2+ with crypto extensions), Linux, pinned toolchain 1.98.1, quiet-machine guard, calibration (§4.7) in that environment | **No** (PLATFORM_BLOCKED) |
| Q | QEMU user-mode or system arm64 via Docker, or `qemu-user-static` in WSL | **No** at design time (Docker unavailable; qemu-user not installed) |
| A | Research build permitting `unsafe` or nightly portable SIMD | **No** (policy) |

Emulated results carry the `EMULATED` qualifier and never support timing (§4.1).

## 8. Commands (when unblocked)

```sh
# Arm Q via WSL without Docker (unblock route for blocker 2)
apt-get install -y qemu-user-static gcc-aarch64-linux-gnu        # approved download policy; record versions
/root/.cargo/bin/rustup target add aarch64-unknown-linux-gnu --toolchain 1.98.1
CARGO_TARGET_DIR=/root/eb-research/target/exp-chunk-aarch64 \
  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
  bash research/harness/build.sh build --release -p ebr-chunk --target aarch64-unknown-linux-gnu
$PY research/experiments/EXP-CHUNK-013/make_specs.py --arm qemu --out research/experiments/EXP-CHUNK-013/spec-qemu.yaml
#   command: qemu-aarch64-static <aarch64 ebr-chunk> tree-dedup ... (boundary_set_sha256 only)
# Arm N on native hardware (when available): same spec.yaml via ebr run --timing on that host, after calibration
$PY research/experiments/EXP-CHUNK-013/compare_isa.py --x86 research/normalized/EXP-CHUNK-012 --arm64 research/normalized/EXP-CHUNK-013 \
    --out research/normalized/EXP-CHUNK-013/decision/isa-identity.csv
```

## 9. Seeds

- **Spec seeds:** `{order: 2026091831, bootstrap: 2026091832, command: 2026091833}`.
- **Key seed base:** 20260917, identical to the x86 runs so keyed boundaries are comparable.
- **Warmups:** 2 (arm N).

## 10. Metrics

| Metric | Canonical | Role |
|---|---|---|
| `boundary_set_sha256` equality with x86-64 | `cross_host_identity_agreement` | binary |
| `chunk_stage_throughput_bps`, `chunk_stage_cpu_seconds_per_pass` (arm N) | no (component timings, as EXP-CHUNK-008) | secondary |
| `encode_cpu_s` for W/S packs on arm64 (arm N extension) | yes | primary for an ARM64-scoped OD-06 claim (T-05) |
| `cpu_features` | no | recorded covariate |

## 11. Replication

- **Arm N:** §4.6 timing rounds and adaptive rule (as EXP-CHUNK-008).
- **Arm Q:** 2 repetitions and repeat-hash.

## 12. Analysis

- **Identity:** a pass/fail table (HC); violation artifacts committed.
- **Arm N:** as EXP-CHUNK-008 section 12, within the ARM64 environment only. Numbers from different environments are never paired (§4.1).
- **Cross-architecture ranking agreement:** Kendall τ, descriptive.

## 13. Thresholds

T-05 and T-05b as transcribed in `research/methods/thresholds.json` for arm N's canonical metrics; binary for identity. Not restated.

## 14. Sensitivity

Arm N only: core type on big.LITTLE ARM hardware reported separately; hardware vs software AES.

## 15. Expected negative results worth recording

- ARM64 evidence remains unavailable for v1, so every performance-scoped chunker decision excludes ARM64 explicitly.
- A NEON path, if built, differs in boundaries on tail handling (an HC violation) or offers no end-to-end gain.

## 16. Threats to validity

- QEMU emulation validates only instruction semantics as QEMU implements them. It is not hardware; identity under QEMU is strong but not conclusive evidence for native execution (TCG bugs).
- Cross-compilation toolchain differences from a native build (the linker only; codegen is the same rustc target).
- On the day hardware becomes available, calibration must precede arm N.

## 17. Estimates (when unblocked)

| Arm | Estimate |
|---|---|
| Q | about 10-20× native cost for about 2 TB of emulated chunking: about **40-80 CPU-hours** |
| N | about **30-60 wall-hours** on native hardware |

- **Disk:** < 5 GB.
- **Agent effort:** scripted after the unblock; owner decision needed for blocker 3.

## 18. Tooling to build

1. The aarch64 cross-build recipe and `qemu-aarch64-static` wrapper (`make_specs.py --arm qemu`).
2. `ebr-chunk throughput --accel scalar|simd-neon|hw-aes|soft-aes`. The accelerated modes exist only in a build that permits them (blocker 3).
3. `compare_isa.py`.

## 19. Deviations

(append-only)
