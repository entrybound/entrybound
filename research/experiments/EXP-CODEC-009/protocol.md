# EXP-CODEC-009: Native ARM64 codec and transform throughput (BLOCKED)

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CODEC-009 |
| Domain | codecs (8.6, 8.7) |
| Status | **BLOCKED: PLATFORM_BLOCKED.** No native ARM64 hardware (Linux aarch64 or macOS on Apple Silicon) is available. Hosted runners were declined by the program owner (PROGRESS standing decision 2026-09-12). QEMU or Docker emulation is not admissible for timing (`decision-method.md` §4.1), and Docker is currently unavailable in any case. |
| Unblocking condition | Access to one native ARM64 host with the program owner's approval: Linux aarch64 (for example an Ampere or Graviton class server, or a Raspberry Pi 5 as a low-end arm), and optionally macOS on Apple Silicon. The host must be fingerprinted under `research/environment/`, with non-admin controls equivalent to §4.2/§4.3 and a calibration pass (§4.7) for `time_wall`, `time_cpu` and `throughput`. |
| Stage | When unblocked: tuning (all candidates), then a validation look (W and S), then the Phase D held-out placeholder. The same candidate set and batches as EXP-CODEC-002 are used, so the environment arm (§6.5) is available. |
| decision_ids | DEC-CMP-008 and DEC-CMP-009 (throughput branch of the §7.3 rows on ARM64: the LZ4 speed tier and extended-codec decode cost), DEC-CMP-045 (window decode cost), DEC-CMP-043 (ARM64 BCJ encode and decode cost; transform inverse cost on NEON-less and NEON builds), DEC-CMP-007 (backend decode throughput on ARM64, where the C libraries' hand-tuned x86 paths do not apply) |
| Proposed od_ids / hc_ids | OD-06, OD-07 / HC-02, HC-11 |
| Method, gates, author, L7 | EXP-CODEC-001 section 1 and Appendix C. |

## 2. Question

Do the throughput rankings that EXP-CODEC-002 measured on x86-64 hold on native ARM64? In particular:

- LZ4 against Zstandard decode;
- LZMA2 and PPMd decode cost;
- libzstd, liblz4 and liblzma against the pure-Rust decoders;
- structural transform inverse cost.

## 3. Hypotheses

- **H1a.** The LZ4-over-zstd-1 decode ratio on ARM64 is within 1 T-05b band unit of the x86-64 ratio (same direction, similar magnitude).
- **H1b.** C-reference decoders' advantage over pure-Rust decoders is smaller on ARM64 than on x86-64, where the C libraries carry x86-specific SIMD paths.
- **H0.** Every comparison has the same verdict class as on x86-64.
- **Falsification.** A comparison whose verdict class differs between ARM64 and x86-64 at the validation look. Such a difference makes the decision's scope platform-specific (§4.1, same-direction rule).

## 4. Candidates

The EXP-CODEC-002 candidate set, including its mechanically appended survivors, is used without changes. The only additions are architecture-specific build variants:

- libzstd with and without `ZSTD_NO_ASM` (x86 assembly is inapplicable, so this is a control);
- `lz4_flex` with its `safe-decode` default.

## 5. Corpus

As EXP-CODEC-002: every tuning and validation item and the same 8 MiB batches (identical batch files, copied with SHA-256 verification).

## 6. Environment

| Environment | Affinity | Status |
|---|---|---|
| native Linux aarch64 | single-thread pinned to one big core (`taskset`) | BLOCKED (no hardware) |
| native macOS arm64 | unpinned (macOS has no user affinity), with QoS user-interactive and a documented limitation | BLOCKED (no hardware); `os-scheduled` informational only |
| QEMU user-mode aarch64 | — | not admissible for timing. Usable only for HC-11 output identity, which the planner and ecosystem domains design (EXP-PLANNER-009, EXP-ECO-002). |

## 7. Tooling and commands

- The EXP-CODEC-002 tooling, cross-compiled for `aarch64-unknown-linux-gnu` (and `aarch64-apple-darwin`) with the pinned toolchain 1.98.1.
- `spec.yaml`: the same metrics as EXP-CODEC-002, with `platform.arch: [aarch64]`.
- The runner additions of §14 item 3 as available on the target OS.

```sh
PY=<venv on the ARM64 host>/bin/python; S=research/experiments/EXP-CODEC-009/spec.yaml
$PY -m ebr validate --spec $S && $PY -m ebr run --spec $S && $PY -m ebr verify-raw --spec $S && $PY -m ebr normalize --spec $S
```

## 8-11. Seeds, metrics, replication, normalization

As EXP-CODEC-002 sections 8-11, per environment. Metrics are never paired across environments; the §6.5 environment arm compares verdict directions only.

## 12. Statistical analysis

CC-10 within the ARM64 environment, plus the §4.1 same-direction rule for any claim whose scope includes ARM64. Until this experiment runs, every decision informed here must either exclude ARM64 from its `decision_scope` explicitly, or stay `INSUFFICIENT_EVIDENCE` with blocker class PLATFORM (§8.4).

## 13. Practical-significance thresholds

T-03, T-04, T-05, T-05b; §7.3 rows as in EXP-CODEC-002.

## 14. Sensitivity

CC-11, plus a low-end ARM64 arm (a Raspberry Pi 5 class device) if available.

## 15. Held-out (Phase D placeholder)

CC-12.

## 16. Expected negative results

- Pure-Rust decoders are competitive on ARM64, which weakens the C-reference speed argument.
- LZ4's relative advantage may shrink on low-end cores with small caches.

## 17. Threats to validity

- A single ARM64 host is not representative of the ARM64 range (server cores against phones against SBCs).
- macOS has no affinity control, so its timing is informational.
- Thermal throttling on small devices needs the throttling-onset probe (§4.2 analogue).

## 18. Platform and cost

| Item | Value |
|---|---|
| Platforms | Native ARM64 Linux (required), macOS arm64 (optional). **Currently unavailable (BLOCKED).** |
| Compute (when unblocked) | ≈ 65 h exclusive per ARM64 host (as the WSL arm of EXP-CODEC-002) |
| Disk | ≈ 7 GB (batches plus raw) |
| Agent effort | Scripted once the host exists; judgment for the environment fingerprint and the analysis |

## 19. Deviations (append-only)

None.
