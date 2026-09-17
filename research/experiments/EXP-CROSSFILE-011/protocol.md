# EXP-CROSSFILE-011: Dictionary- and group-bearing archive determinism on native ARM64 (Linux and macOS) and macOS x86-64

| Field | Value |
|---|---|
| Status | **BLOCKED**: `PLATFORM_BLOCKED` — no native ARM64 hardware and no macOS host are available (program standing decision 2026-09-12: no hosted runners; `research/PROGRESS.md` known blockers). Emulated ARM64 is EXP-CROSSFILE-010 and cannot substitute for native silicon or Apple toolchains. |
| Domain / program sections | crossfile / 8.4 |
| decision_ids | DEC-CMP-021 |
| Evidence type | determinism claim tests (HC-13 MVT-13(a), HC-11 MVT-11), native |
| Method | `research/decision-method.md`, pre-registered |
| Record | Designed now so remaining-unknowns can cite it; `record.md` is instantiated only when a platform becomes available (reopen trigger §11 item 3). |
| Design session and disclosure | Phase C-design crossfile designer; L7 disclosure as in EXP-CROSSFILE-001. |

## 1. Question

Are trained Dictionaries, plans, groups and archive bytes produced by the production writer on
native linux/arm64, macOS/arm64 (Apple silicon) and macOS/x86-64 identical to those produced on the
x86-64 Linux and Windows hosts of EXP-CROSSFILE-009 for the same inputs?

## 2. Hypotheses

- **H1.** Identical cross-file physical fingerprints wherever the Chunk digest multiset is identical
  (the planner and trainer are integer-only portable code).
- **H0 / violation.** Any difference is an HC-11/HC-13 violation for that platform class.

## 3. Decision informed; proposed sets

As EXP-CROSSFILE-009 §3.

## 4. Cells

`linux-arm64-native`, `macos-arm64-native` (APFS), `macos-x86_64-native` (APFS), each with the
EXP-CROSSFILE-009 variation cells (affinity, locale, time zone) and a 20-repetition stress cell.

## 5. Inputs

The EXP-CROSSFILE-009 inputs and generated tree (same generator, seed and count), copied as data
only; tuning split before the freeze, validation counterparts, held-out after unlock.

## 6. Environment and platform requirements (missing)

- Native ARM64 Linux machine (any ARMv8.2+ server or SBC with >= 16 GB RAM) with Rust 1.98.1.
- macOS 15+ on Apple silicon and on Intel, APFS volume, Rust 1.98.1, Xcode command-line tools.
- Local execution only (hosted runners declined by the program owner).

## 7. Commands (to run when unblocked)

Build the production CLI at the pre-registration commit on each host as in EXP-CROSSFILE-009 §6;
run `ebr validate/run/verify-raw/normalize` with `research/experiments/EXP-CROSSFILE-011/spec.yaml`
(platform `linux`/`darwin`, arch `aarch64`/`x86_64`); compare with `compare_hosts.py` of
EXP-CROSSFILE-009.

## 8. Seeds, warmups, repetitions

Seeds `order` 20260917111, `bootstrap` 20260917112; generator seed 20260917091. Warmups 0.
Repetitions 3 per cell; stress 20.

## 9. Metrics

As EXP-CROSSFILE-009 §9.

## 10. Normalization and statistical analysis

As EXP-CROSSFILE-009 §10 (binary rules, PCR comparability gate).

## 11. Practical-significance thresholds

None (binary).

## 12. Sensitivity analysis

Not applicable (binary).

## 13. Validation look and held-out placeholder

As EXP-CROSSFILE-009 §13; `EXP-CROSSFILE-011-HO` only if the platform exists before the freeze.

## 14. Expected negative results worth recording

Until run: DEC-CMP-021 cannot claim cross-architecture writer determinism for native ARM64 or
macOS; any decision scope including them stays `INSUFFICIENT_EVIDENCE` (§4.1, §8.4), and a scope
excluding them records the exclusion in `known_limitations`.

## 15. Threats to validity

APFS capture differences (F-0002-style) may make PCR comparability rare; handled by the gate.

## 16. Cost

About 30 CPU-hours per platform, 1 GB disk each; agent effort: scripted, once hardware exists.
