# EXP-CRYPTO-004: Native ARM64 and constrained-platform crypto cost (keyed boundaries, primitives, password-KDF unlock)

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

On hardware the program cannot access here, what do Entrybound's crypto choices cost? The hardware in question is native ARM64 (server, SBC, mobile-class), 32-bit ARM, Apple Silicon/macOS, and memory-capped CI containers. The costs are PHTE-AES128 versus secret-Gear chunking throughput with and without ARMv8 cryptographic extensions, AEAD and digest throughput, and Argon2id unlock latency and peak memory at the creation default and at the crypto-v1 wire ceiling. Does any frozen default fail outright (for example out-of-memory at 256 MiB or 1 GiB) on a platform in a decision's scope?

## 2. Hypotheses and falsification

- **H1.** On ARM64 cores with the AES and PMULL extensions, PHTE chunking overhead relative to secret Gear is within 1 band unit (T-05b) of the x86-64 AES-NI overhead that EXP-CRYPTO-003 measured. Without the extensions it exceeds it by at least 2 band units.
- **H2.** Argon2id at the status-quo creation parameters (256 MiB, t=3, p=4) completes on 1 GiB-RAM ARM64 SBCs and in 512 MiB-capped containers. At the wire ceiling (1 GiB, t=10, p=16) it is refused by policy or fails in the 512 MiB-capped container and on 32-bit ARM.
- **H3.** SHA-256 with ARMv8 SHA extensions is `NON_INFERIOR` to BLAKE3 single-thread on native ARM64 server cores (DEC-MOD-016 input).
- **Falsification:** each H is falsified by the opposite verdict at the pre-registered bands on the platform stratum it names.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` on native hardware. **QEMU-emulated ARM64 is never used for timing (§4.1).** Until this experiment runs, every decision scope that includes these platforms stays `INSUFFICIENT_EVIDENCE` (`PLATFORM_BLOCKED`, §8.4), or excludes them explicitly in `decision_scope`.

## 4. Candidates and arms

- Chunking: the arms U0, K1, K2 and K3 of EXP-CRYPTO-003 (ARM64 builds; K2 with ARMv8 AES autodetect and with `aes_force_soft`).
- Primitives: the AEAD, digest and KEM arms of EXP-CRYPTO-013, cross-compiled for `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin` and `armv7-unknown-linux-gnueabihf`.
- KDF: the Argon2id and alternative-KDF candidate parameter sets of EXP-CRYPTO-021 (status quo 256 MiB/t3/p4; RFC 9106 second recommendation 64 MiB/t3/p4; wire ceiling 1 GiB/t10/p16; scrypt N=2^18 r=8 p=1; PBKDF2-HMAC-SHA256 600,000).
- Containers: Docker `--memory` caps 256 MiB, 512 MiB and 1 GiB with `linux/amd64` and `linux/arm64` images (native ARM64 Docker host required for the ARM64 arm).

Reference candidate: the status quo of each decision.

## 5. Corpus selectors

The tuning items of the EXP-CRYPTO-003 timing set (fitting within the device's storage), plus generated `random-v1` buffers of 4 KiB, 1 MiB and 64 MiB for primitive throughput. For the confirmatory look, the validation items of the same families.

## 6. Environment and platform requirements (why BLOCKED)

| Required | Status on this host |
|---|---|
| Native ARM64 Linux server (for example Neoverse class) with AES/PMULL/SHA extensions | **PLATFORM_BLOCKED**: no local ARM64 hardware; hosted runners declined by the program owner (PROGRESS standing decision 2026-09-12) |
| ARM64 SBC with at most 1 GiB RAM and an ARM64 core without crypto extensions | **PLATFORM_BLOCKED** (no hardware) |
| Apple Silicon Mac running macOS | **PLATFORM_BLOCKED** (no local macOS) |
| Mobile-class device (Android ARM64) | **PLATFORM_BLOCKED** (no device) |
| 32-bit ARMv7 device | **PLATFORM_BLOCKED** (no device) |
| Docker with memory-capped containers | **BLOCKED**: Docker Desktop fails to start on this host (PROGRESS storage incident, error 1920). Partly substituted by the WSL cgroup memory caps in EXP-CRYPTO-021, which cover x86-64 only |

Each platform gets its own environment capture (`research/environment/<env_id>.json`) and its own calibration experiments (§4.7) before any decision-grade timing.

## 7. Commands and tooling

- Cross-compiled `ebr-crypto` (subcommands `chunk`, `bench`, `unlock`) and production `ebound` built with `cargo +1.98.1 build --release --target <triple>` on the device or through a cross toolchain. On-device execution uses the ebr runner with `platform.arch: [aarch64]` (or `armv7`) and `emulated: false`.
- `spec.yaml` (experiment id `EXP-CRYPTO-004`) is committed in its design form with `platform.arch: [aarch64]` and is not runnable here. Per-device specs `spec-<device>.yaml` are added when hardware is available, before any run, under the §0.2 amendment rules.

## 8. Seeds, warmup, repetitions and adaptive rule

As EXP-CRYPTO-003 and EXP-CRYPTO-021 (§4.5-§4.6 warmups and rounds, adaptive precision rule), on each device's own calibrated affinity configuration (single-thread pinned to one big core, plus a little-core stratum on heterogeneous SoCs).

## 9. Measurement method and metrics

| Metric (canonical) | ID | Role |
|---|---|---|
| `encode_throughput_bps` (chunking, AEAD encrypt, digest) | T-05b | diagnostic per component |
| `decode_throughput_bps` (AEAD decrypt), `verify_throughput_bps` (digest) | T-05b | diagnostic |
| `encode_wall_s` (end-to-end encrypted `ebound pack`) | T-03 | primary for DEC-CRY-023 platform stratum |
| `open_latency_s` (unlock) | T-08 | primary for KDF decisions |
| `alloc_peak_bytes`, `peak_rss_bytes` | T-06 | primary for KDF memory feasibility |
| unlock success or failure under a memory cap | binary feasibility (R2 for a platform in scope) | screen |

## 10. Normalization and statistical analysis

As EXP-CRYPTO-003 and EXP-CRYPTO-021, per device environment. Devices are strata, never pooled or averaged with x86-64 (§4.1). A claim covering x86-64 and ARM64 needs same-direction decision-grade support on both.

## 11. Practical significance

T-03, T-05b, T-06 and T-08 bands from `research/methods/thresholds.json`. Feasibility failures are binary.

## 12. Sensitivity analysis

§6 arms, plus: big versus little core class, thermal state (a sustained-load probe records the throttling onset t_on as in §4.2), and memory-cap levels.

## 13. Expected negative results worth recording

- Argon2id status-quo parameters fail in 256 MiB containers. This is a real CI constraint against DEC-CRY-002's status quo.
- PHTE overhead without ARM crypto extensions makes keyed-prf impractical on low-end ARM, which narrows any PHTE default to an x86-64/ARMv8-crypto scope that the product cannot express (R9 expressibility).

## 14. Threats to validity

- Device heterogeneity; vendor kernels; DVFS governors that cannot be controlled without root on mobile.
- Single-device-per-class sampling (report as case studies, soft-capped).
- macOS timing needs its own controls (not specified by §4.2). They are added by amendment before use.

## 15. Held-out placeholder (Phase D)

As EXP-CRYPTO-003 on each device environment after the freeze, if the platform is within a frozen decision's scope. No held-out item is named here.

## 16. Cost estimate and agent effort

- Compute (when unblocked): about 60 machine-hours per device class across 4-5 device classes. Disk: 50 GB per device.
- Agent effort: **scripted** execution; **judgment** for per-device environment capture and calibration review.

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
