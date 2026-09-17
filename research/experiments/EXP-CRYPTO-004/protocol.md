# EXP-CRYPTO-004: Native ARM64 and constrained-platform crypto cost (keyed boundaries, primitives, password-KDF unlock)

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **BLOCKED** — Native ARM64/Apple-Silicon-macOS/mobile/32-bit hardware unavailable (PLATFORM_BLOCKED); hosted runners declined; QEMU never used for timing |
| Kind | decision |
| Domain | crypto (program §22.1, §22.2) |
| Decisions informed | DEC-CRY-023, DEC-CRY-002, DEC-CRY-013, DEC-MOD-016 |
| Platforms | ARM64/ARMv7/macOS (BLOCKED) |
| Requires timing | True |
| Estimates | 300 machine-hours; 250 GB disk |
| Tooling to build | cross-compiled ebr-crypto/ebound; per-device environment captures + calibration |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns no validation look |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
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
#### DEC-CRY-023 — Which chunk-boundary construction is the default for encrypted archives (and per profile or threat setting), given secret Gear tables are known-breakable and PHTE/AES costs more?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CHUNK-006, EXP-CHUNK-008, EXP-CHUNK-012, EXP-CHUNK-013, EXP-CRYPTO-002, EXP-CRYPTO-003, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-secret-gear-default-phte-opt-in` | Status quo: SECRET_GEAR_TABLE default (defense in depth), PHTE_AES128 via --chunk-boundary keyed-prf | status quo | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `phte-default-everywhere` | PHTE_AES128 default for all encrypted archives; secret Gear retained only for reading/compatibility | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `phte-default-for-selected-profiles` | PHTE default for remote/storage-provider oriented or Dense/Extreme profiles; secret Gear elsewhere | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `widened-keyed-rolling-hash` | Hardened secret table (wider state, e.g. 256-bit buzhash as recommended to Borg) with mandatory compression; new boundary mode id | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `fixed-size-chunks-under-encryption` | Fixed-size chunking for encrypted archives (no content-defined boundaries), losing shift-resistant dedup | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `keyed-cdc-with-coarse-rechunking` | Keyed CDC followed by packing chunks into fixed-size padded containers so boundaries are not observable | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `public-unkeyed-cdc` | Public unkeyed gear-norm-v1 boundaries under encryption | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |

**Ledger exclusions:** {"candidate_id": "public-unkeyed-cdc", "reason": "Encrypted boundary modes are keyed by frozen registry; unkeyed boundaries directly expose plaintext-derived structure", "invariant_violated": "I24; docs/crypto-wire-v1.md L56-57"}.

**R0 checklist:** 1 status quo: `status-quo-secret-gear-default-phte-opt-in`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: `widened-keyed-rolling-hash`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-002 — Is Argon2id v19 the v1 password KDF, and are the frozen creation parameters (256 MiB, t=3, p=4, random unlabelled 16-byte salt) the right defaults, or should defaults, creator tunability or salt domain separation change?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-017, EXP-CRYPTO-020, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-256mib-t3-p4` | Status quo: Argon2id m=256 MiB, t=3, p=4 fixed at creation | status quo | no | EXP-CRYPTO-017 |
| `rfc9106-second-64mib` | RFC 9106 second recommendation m=64 MiB, t=3, p=4 | ledger alternative | no | EXP-CRYPTO-017 |
| `rfc9106-first-2gib` | RFC 9106 first recommendation m=2 GiB, t=1, p=4 | ledger alternative | no | EXP-CRYPTO-017 |
| `owasp-19mib` | OWASP m=19 MiB, t=2, p=1 | ledger alternative | no | EXP-CRYPTO-017 |
| `calibrated-per-host` | Calibrate to a target unlock latency on the creating host within wire bounds | ledger alternative | no | EXP-CRYPTO-017 |
| `named-cost-profiles` | Named cost profiles selectable at creation | ledger alternative | no | EXP-CRYPTO-017 |
| `scrypt` | scrypt as age uses (N=2^18, r=8, p=1) | ledger alternative | no | EXP-CRYPTO-017 |
| `balloon-hashing` | Balloon hashing | ledger alternative | no | EXP-CRYPTO-017 |
| `pbkdf2-fips` | PBKDF2-HMAC-SHA256 with 600,000 iterations for FIPS environments | ledger alternative | no | EXP-CRYPTO-017 |
| `labelled-salt-or-ad` | Status quo parameters plus format-label domain separation in salt or Argon2 associated data | status quo | no | EXP-CRYPTO-017 |

**Ledger exclusions:** {"candidate_id": "rfc9106-first-2gib", "reason": "2 GiB exceeds the frozen 1 GiB crypto v1 reader memory bound", "invariant_violated": "docs/crypto-suite-v1.md L510-520 reader wire bounds"}.

**R0 checklist:** 1 status quo: `status-quo-256mib-t3-p4`, `labelled-salt-or-ad`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 10 ledger candidates listed above; 4 strongest incumbent: `rfc9106-second-64mib`, `rfc9106-first-2gib`, `scrypt`, `pbkdf2-fips`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-013 — Are the crypto resource ceilings justified: Argon2id caller defaults equal to the wire ceiling (1 GiB, 10 passes, parallelism 16) versus lower open defaults, and the 1,024-recipient cap?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-007, EXP-CONF-014, EXP-CRYPTO-009, EXP-CRYPTO-017, EXP-SCALE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-default-equals-ceiling` | Status quo: open defaults equal wire ceiling; 1,024 recipients | status quo | no | EXP-CONF-007, EXP-CRYPTO-017 |
| `default-equals-creation-default` | Open default equals 256 MiB/3-pass creation default | ledger alternative | no | EXP-CONF-007, EXP-CRYPTO-017 |
| `hardware-probed-default` | Default derived from available memory | ledger alternative | no | EXP-CONF-007, EXP-CRYPTO-017 |
| `confirm-above-threshold` | Interactive confirmation above a cost threshold | ledger alternative | no | EXP-CONF-007, EXP-CRYPTO-017 |
| `lower-recipient-cap` | Lower recipient cap based on measured unlock cost | ledger alternative | no | EXP-CONF-007, EXP-CRYPTO-017 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-default-equals-ceiling`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-MOD-016 — Which single digest algorithm does Entrybound format version 1 use for chunk, content, structured, section, PCI and identity-descriptor digests?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-012, EXP-CRYPTO-020, EXP-CRYPTO-021, EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `sha-256-status-quo` | SHA-256 (FIPS 180-4) via RustCrypto sha2 =0.11.0, as implemented for every digest and bound as algorithm id 1 | ledger alternative | no | — |
| `sha-512-256` | SHA-512/256 (FIPS 180-4): faster on 64-bit CPUs, same standard family, length-extension resistant | ledger alternative | no | — |
| `blake2b-256` | BLAKE2b-256 (RFC 7693): fast in software on 64-bit CPUs without SHA extensions; the Borg incumbent's chunk-ID hash; no tree mode in the RFC profile | ledger alternative | no | — |
| `blake3` | BLAKE3 (C2SP BLAKE3 v1.0.0): tree-native, SIMD/parallel, not NIST/IETF standardised | ledger alternative | no | — |
| `sha3-256` | SHA3-256 (FIPS 202) | ledger alternative | no | — |
| `kangarootwelve-kt128` | KT128/KT256 (RFC 9861, IRTF informational): tree-capable Keccak variant | ledger alternative | no | — |
| `ascon-hash256` | Ascon-Hash256/Ascon-CXOF128 (NIST SP 800-232): lightweight, customization strings for domain separation | ledger alternative | no | — |
| `sha-256-with-hardware-accel-only-fallback` | SHA-256 retained but mandate SHA-NI/ARMv8 crypto-extension paths and parallel per-chunk hashing | ledger alternative | no | — |
| `dual-digest-transition` | SHA-256 for v1 plus a registered second algorithm id reserved for a future format version (no in-version negotiation) | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 9 ledger candidates listed above; 4 strongest incumbent: `blake2b-256`, `kangarootwelve-kt128`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `sha-256-status-quo`, `sha-512-256`, `blake2b-256`, `blake3`, `sha3-256`, `kangarootwelve-kt128`, `ascon-hash256`, `sha-256-with-hardware-accel-only-fallback`, `dual-digest-transition`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` on native hardware. **QEMU-emulated ARM64 is never used for timing (§4.1).** Until this experiment runs, every decision scope that includes these platforms stays `INSUFFICIENT_EVIDENCE` (`PLATFORM_BLOCKED`, §8.4), or excludes them explicitly in `decision_scope`.

## 4. Candidates and arms

- Chunking: the arms U0, K1, K2 and K3 of EXP-CRYPTO-003 (ARM64 builds; K2 with ARMv8 AES autodetect and with `aes_force_soft`).
- Primitives: the AEAD, digest and KEM arms of EXP-CRYPTO-012, cross-compiled for `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin` and `armv7-unknown-linux-gnueabihf`.
- KDF: the Argon2id and alternative-KDF candidate parameter sets of EXP-CRYPTO-017 (status quo 256 MiB/t3/p4; RFC 9106 second recommendation 64 MiB/t3/p4; wire ceiling 1 GiB/t10/p16; scrypt N=2^18 r=8 p=1; PBKDF2-HMAC-SHA256 600,000).
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
| Docker with memory-capped containers | **BLOCKED**: Docker Desktop fails to start on this host (PROGRESS storage incident, error 1920). Partly substituted by the WSL cgroup memory caps in EXP-CRYPTO-017, which cover x86-64 only |

Each platform gets its own environment capture (`research/environment/<env_id>.json`) and its own calibration experiments (§4.7) before any decision-grade timing.

## 7. Commands and tooling

- Cross-compiled `ebr-crypto` (subcommands `chunk`, `bench`, `unlock`) and production `ebound` built with `cargo +1.98.1 build --release --target <triple>` on the device or through a cross toolchain. On-device execution uses the ebr runner with `platform.arch: [aarch64]` (or `armv7`) and `emulated: false`.
- `spec.yaml` (experiment id `EXP-CRYPTO-004`) is committed in its design form with `platform.arch: [aarch64]` and is not runnable here. Per-device specs `spec-<device>.yaml` are added when hardware is available, before any run, under the §0.2 amendment rules.

## 8. Seeds, warmup, repetitions and adaptive rule

As EXP-CRYPTO-003 and EXP-CRYPTO-017 (§4.5-§4.6 warmups and rounds, adaptive precision rule), on each device's own calibrated affinity configuration (single-thread pinned to one big core, plus a little-core stratum on heterogeneous SoCs).

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

As EXP-CRYPTO-003 and EXP-CRYPTO-017, per device environment. Devices are strata, never pooled or averaged with x86-64 (§4.1). A claim covering x86-64 and ARM64 needs same-direction decision-grade support on both.

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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-004; edit the inputs, not this block._
<!-- END AUTO:common -->
