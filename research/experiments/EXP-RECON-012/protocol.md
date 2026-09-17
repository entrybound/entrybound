# EXP-RECON-012 (BLOCKED): Reconstruction on native ARM64, macOS and physical low-memory devices — decode determinism, forward determinism, decode performance and default-ceiling outcomes

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-RECON-012 |
| Domain | reconstruction (8.8); common provisions: `research/experiments/_index/reconstruction-common.md` |
| Status | **BLOCKED.** The platforms are unavailable to this program. Native ARM64 Linux hardware, macOS on Apple Silicon (APFS) and physical low-memory ARM devices (≤ 2 GiB RAM) are all `PLATFORM_BLOCKED`. The standing decision in `research/PROGRESS.md` (2026-09-12) is no GitHub Actions or hosted runners, local Windows/WSL/Docker only. Docker is currently unavailable, and QEMU emulation, when used, serves correctness only (EXP-RECON-008). |
| Stage | Designed now so remaining-unknowns can cite it. When a platform becomes available, reopen rule §11 item 3 applies to every decision whose scope includes it. |
| decision_ids | DEC-INT-030, DEC-CMP-016, DEC-CON-005, DEC-CMP-026, DEC-CMP-018, DEC-CON-007 |
| Proposed types/ODs/HCs | common §2. HC-11 (MVT-11(a) architecture and OS variation, native), HC-13 (forward determinism across host classes), HC-06 (declared bounds on constrained devices), OD-07/OD-06 (native ARM64 timing), OD-17 (`benign_false_refusal_fraction` on devices) |
| Gates | G-A; G-B plus calibration `EXP-CAL-*` for each new `env_id` (§4.7) before any timing |
| Author and exposure | common §3 |

## 2. Question

- **HC-11.** On native ARM64 Linux and macOS arm64, do reconstruction vectors and archives decode to exactly the same bytes and outcomes as on x86-64?
- **HC-13 scope.** Does forward reconstruction produce identical representations across architectures?
- **Scope of timing verdicts.** On native ARM64, do the relative encode and decode costs of reconstruction candidates keep the direction and band verdicts obtained on WSL x86-64? For example, jxl-oxide NEON paths versus AVX2.
- **Devices.** On physical devices with 1–2 GiB RAM, which default ceiling policies refuse, succeed or run out of memory on reconstruction archives, and do the outcomes agree with the EXP-RECON-006 cgroup emulation?

## 3. Hypotheses

All hypotheses are `[INFERRED]`.

- **H1 (HC-11).** 0 `reader_disagreements` across native x86-64, native ARM64 Linux and macOS arm64 over all EXP-RECON-008 vectors and all frozen tuning archives. **Falsified by** any disagreement: an R1 violation for the status quo, with artifacts committed.
- **H2 (forward).** jixel 0.2.19 and preflate-rs 0.7.6 representations are byte-identical on native ARM64 and x86-64. **Falsified by** any difference. The difference is recorded as cross-host-class planner-output variation (HC-13 scope, DEC-CMP-038/CON-007).
- **H3 (timing direction).** Every EXP-RECON-005 WSL verdict on `encode_cpu_s` and `decode_wall_s` for W versus S has the same direction on native ARM64 Linux (same-direction support, §4.1). **Falsified by** any `DISTINGUISHABLE` verdict in the opposite direction.
- **H4 (devices).** Every EXP-RECON-006 cgroup outcome class (success, `POLICY_REFUSED`, OOM) at 1 and 2 GiB-class limits matches the physical-device outcome for the same archive and policy. **Falsified by** any mismatch, which invalidates the emulation for DEC-CON-005 and DEC-CMP-016.

## 4. Decisions informed and how

| Decision | Contribution when unblocked | Consequence while blocked |
|---|---|---|
| DEC-INT-030 | "Vectors across builds and platforms" including native ARM64 and macOS | A decision scope that includes ARM64 or macOS cannot be `DECIDED` (§8.4). Scopes must exclude those platforms explicitly and record `HC_UNVERIFIED` for them (objective §4.6). |
| DEC-CMP-016, DEC-CON-005 | "Low-memory device decode tests" (ledger evidence); agreement with emulation | Ceiling decisions are scoped to hosts with ≥ 8 GiB-class memory and emulated limits, with `EMULATED` qualifiers and a limitation recorded |
| DEC-CMP-026, DEC-CMP-018 | Native ARM64 performance guards | Performance claims are scoped to x86-64 (WSL `VIRTUALIZED`, Windows host) |
| DEC-CON-007 | Cross-architecture decodability of dependency-defined formats | Evidence limited to emulated arm64 (correctness only) |

## 5. Candidates and cells

| Cell | Platform | Definition |
|---|---|---|
| `native-arm64-linux-decode-vectors` | ARM64 Linux (≥ 8 GiB) | EXP-RECON-008 `vectors decode --impl production-reader-archive` plus the reference implementations |
| `native-arm64-linux-forward` | ARM64 Linux | EXP-RECON-008 `vectors forward` for jixel and preflate-rs |
| `macos-arm64-decode-vectors` | macOS arm64 (APFS) | as above, native macOS build |
| `native-arm64-linux-archives-unpack` | ARM64 Linux | `ebound unpack` of every frozen tuning archive of EXP-RECON-004 `gen-v6`, `v6-minus-jpeg` and `v6-minus-deflate`, with the round-trip check |
| `lowmem-2gib-decode-policy-<C>` / `lowmem-1gib-decode-policy-<C>` | ARM64 devices with 2 GiB and 1 GiB RAM, no swap | `ebr-recon decode-probe` under each EXP-RECON-006 ceiling policy C, on region and DEFLATE archives |
| timing arm | ARM64 Linux | A copy of the EXP-RECON-005 target spec with `platform.arch: [aarch64]`, an affinity configuration defined after an SMT/core-type capture of that host, and calibration experiments for the new `env_id`. Written when the platform exists, before any timing result, as an amendment. |

## 6. Corpus

- **Vectors and archives.** EXP-RECON-008 vectors (tuning) and EXP-RECON-004 frozen tuning archives, copied by hash. Validation vectors and archives are added inside look 1 when available.
- **Held-out.** Phase D placeholder: platform HC screens of frozen candidates after Commit C, if the platform exists by then (common §5).

## 7. Platform requirements (exact missing access)

- **Native ARM64 Linux host.** aarch64, ≥ 8 GiB RAM, local SSD, root, Ubuntu 24.04 (to match the WSL userland), pinned Rust 1.98.1 toolchain.
- **macOS on Apple Silicon.** macOS ≥ 14, APFS, Xcode command-line tools, pinned Rust 1.98.1.
- **Physical low-memory ARM64 devices.** A 2 GiB and a 1 GiB board, e.g. single-board computers, Linux, no swap.
- **None of these may be hosted runners** (standing decision). They must be owner-provided local machines.

## 8. Commands (when unblocked)

```sh
# per platform: environment capture (research/tools/env) -> new env_id; build research reader + ebound natively at the pre-registration commit
# correctness (no timing): ebr run --spec research/experiments/EXP-RECON-012/spec.yaml  (os/arch per platform)
# low-memory: ebr-recon decode-probe --item <archive> --policy-window <B> --policy-working-set <B> --reconstruction-opt-in <flag>
# timing: calibration EXP-CAL-{AA,AAP,KE,BG}-<env_id> PASS, then spec-timing-arm64.yaml (amendment) with ebr run --timing
```

## 9. Seeds, warmups, replication

- Seeds: 20261002.
- Correctness cells: 3 repetitions per cell (MVT-13(a)), plus a 20-repetition stress cell on `native-arm64-linux-decode-vectors` for the 50 largest vectors.
- Device cells: 2 repetitions. An OOM in either counts.
- Timing: per EXP-RECON-005 §9 on the new environment.

## 10. Metrics

`reader_disagreements` (binary), `cross_host_identity_agreement` (binary for decode outputs), `roundtrip_mismatches` (binary), `alloc_peak_bytes` and `peak_rss_bytes` (T-06), `benign_false_refusal_fraction` (T-20), outcome-class agreement with emulation (descriptive), and timing metrics per EXP-RECON-005 §10.

## 11. Analysis

As EXP-RECON-008 §11 (screens, triage) and EXP-RECON-006 §11 steps 2 and 4 (bounds and ceilings) on the new platforms. Timing analysis follows EXP-RECON-005 §11 with the environment arm (§6.5) and same-direction support (§4.1).

## 12. Practical-significance thresholds

§3.3 binaries; T-06, T-20 and timing thresholds by id (EXP-RECON-005).

## 13. Sensitivity analysis

Environment arm (§6.5) for timing; device classes reported separately and never pooled.

## 14. Expected negative results worth recording

- Architecture-dependent jixel output (H2 falsified): frozen v6 physical outputs differ by host architecture.
- Emulated memory limits disagree with devices (H4 falsified): DEC-CON-005 evidence would need physical confirmation.

## 15. Threats to validity

Device variability (kernel, allocator, page cache) and small device counts (one per class). Native-hardware results would be the first `EMPIRICALLY_MEASURED` non-x86 evidence in the program, so they cannot be cross-checked internally.

## 16. Held-out placeholder

Platform HC screens on held-out frozen archives after Commit C, only if the platforms exist by then. Otherwise every affected decision scope excludes the platform (objective §4.6).

## 17. Cost and effort estimates (when unblocked)

- **Compute.** About 40 machine-hours of correctness and device runs, plus about 150 machine-hours for an ARM64 timing campaign (calibration + target items).
- **Disk.** About 30 GB per platform (vectors, archives).
- **Agent effort.** Environment capture and builds: scripted. Analysis: judgment.

## 18. Deviations and outcome (append-only)

- Deviations: none.
- Outcome: `outcome.json` with classification `abandoned (platform unavailable)` if the program closes while the experiment is still blocked. Raw data: none.
