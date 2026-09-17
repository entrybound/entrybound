# EXP-PLANNER-011: Cross-architecture and constrained-device replication

| Field | Value |
|---|---|
| Domain | planner (program §9, §10) |
| Index status | **BLOCKED**: PLATFORM_BLOCKED. There is no native ARM64 host (Linux or Apple-silicon macOS), no AMD or other-vendor x86-64 host, and no low-memory ARM device (1-2 GiB RAM). Hosted runners were declined by the program owner (`research/PROGRESS.md` standing decision 2026-09-12). Docker is unavailable at design time; emulation cannot provide native timing or microarchitecture evidence anyway (decision-method §4.1: QEMU is correctness only) |
| Document state | Phase C design. Complete, so that `remaining-unknowns` can cite it and it can run unchanged once a platform appears |
| Shared conventions | `research/planner/README.md` |
| Runner spec | `spec.yaml` (native aarch64 Linux or darwin; validates now, cannot run on this host) |

## 1. Pre-registration header

- **decision_ids:**
  - DEC-CMP-015: decode-cost model ranks and floors on other microarchitectures.
  - DEC-CMP-036: profile Pareto placement and default on a second architecture.
  - DEC-CMP-031: native (not emulated) aarch64 encoder dispatch identity.
  - DEC-CMP-016: native low-memory device decode.
  - DEC-CMP-019: macOS/APFS and native aarch64 host classes in the reproducibility matrix.
- **Decision type (proposed):** EMPIRICAL (replication arms of EXP-PLANNER-007, -008 and -009), and FORMAL counterexample search for the identity arms.
- **Proposed `od_ids` / `hc_ids`:** as in the replicated experiments (OD-05, OD-06, OD-07, OD-08, OD-17; HC-06, HC-11, HC-13).
- **Scope consequence while blocked (§4.1, §8.4):** every listed decision either declares a scope that explicitly excludes ARM64, other x86 vendors, macOS and low-memory devices, or stays `INSUFFICIENT_EVIDENCE`. A claim about "any machine" (SPEC §5.7 determinism guarantee; `min_decode_rate` "on a declared reference machine") cannot be `DECIDED` beyond x86-64 hybrid hosts.
- **Author / L7:** README §10.

## 2. Question and hypotheses

- **Question.** Do the planner-domain conclusions reached on the Intel i9-14900HX host hold on native ARM64 (Linux and macOS), on a second x86-64 vendor, and on a memory-constrained device? The conclusions in question are the decode-cost model ranks and floors, profile placement and the default, encoder output identity, and low-memory decode behaviour.
- **H1.**
  1. The reference-machine decode table's Kendall τ-b against native ARM64 and AMD measurements is at least 0.8.
  2. Profile Pareto order and the default's utility winner agree in direction with the x86 result (same-direction support, §4.1).
  3. Pinned native aarch64 builds produce the Chunk payload, plan-table and PCR bytes that EXP-PLANNER-009 recorded for emulated aarch64 and x86-64.
  4. On a 1-2 GiB device every archive either decodes within its declaration or refuses with a typed diagnostic. There are no OOM kills when the declaration fits.
- **H0.** Architecture-dependent divergence in any arm.
- **Falsification.** τ-b below 0.8; an opposite-direction verdict; any byte difference in (3), which is committed as an HC-11 or HC-13 violation artifact; any OOM kill at or above a fitting declaration, which is an HC-06 violation artifact.

## 3. Candidates

The candidates are exactly those frozen at planner look 1 for the replicated decisions: DEC-CMP-015 W ∪ S (EXP-PLANNER-008), DEC-CMP-036 menus and defaults (EXP-PLANNER-007), the DEC-CMP-031 promise candidates (EXP-PLANNER-009), and the DEC-CMP-016 ceilings (EXP-PLANNER-006). No new candidates are introduced here. A candidate added on a new platform would be a new selection (§5.6 rules on fresh data apply).

## 4. Corpus

The TS-1-V validation subset and the EXP-PLANNER-008 validation decode bundles, the same item identities as the x86 confirmatory runs. This is replication on validation, not a new look: the look registry records it as a platform replication of planner look 1, and nothing may be re-selected from it.

Low-memory arm: G1-PSUB-64 validation sub-items at most 16 MiB.

## 5. Environment and platform requirements (missing)

| Arm | Required platform | Why unavailable |
|---|---|---|
| A: decode-cost model | native ARM64 Linux (for example an Ampere or Graviton server, or a Raspberry Pi 5), Apple-silicon macOS, AMD Zen x86-64 | no such hosts; hosted runners declined |
| B: profile timing placement | the same, with quiet-machine controls equivalent to §4.2-§4.4 (platform-specific affinity sets must be pre-registered per host before use) | as above |
| C: native encoder identity | native aarch64 Linux and macOS (MSVC-free builds) | as above |
| D: constrained-device decode | a 1-2 GiB RAM ARM64 device with a real memory ceiling (not a cgroup inside a 31 GiB VM) | no device |
| E: APFS host class for DEC-CMP-019 | macOS with APFS | PLATFORM_BLOCKED |

Each new host requires an environment capture (`research/environment/`), calibration (`EXP-CAL-*-<env>`), and the harness use constraints 1-7.

## 6. Commands (to run when a platform exists)

```sh
# per new host, after environment capture and calibration
bash research/harness/build.sh build --release            # native toolchain 1.98.1
PYTHONPATH=research/tools python -m ebr run --timing --spec research/experiments/EXP-PLANNER-011/spec.yaml
bash research/planner/tools/encoder-drift/host_pack.sh --host-class native-$(uname -m) --profile balanced ...   # arm C, as EXP-PLANNER-009 spec-hosts.yaml
bash research/planner/tools/cgroup_decode.sh ...          # arm D on the device itself, without cgroups (the device ceiling is real)
```

## 7. Seeds, replication, metrics, statistics

- Identical to the replicated experiments: seeds 260917111/112/113, rounds and warmups per §4.5-§4.6, metrics `decode_throughput_bps`, `encode_wall_s`, `decode_wall_s`, `alloc_peak_bytes`, `declared_tightness_ratio`, and identity binaries.
- Analysis: the environment arm of §6.5 (same-direction support, §4.1) and R5-style replication classification per comparison: replicated, attenuated or failed, reported, never re-selecting.
- Practical significance: the thresholds.json keys of the replicated metrics.

## 8. Sensitivity, negative results, threats

- **Sensitivity:** environment arm only.
- **Expected negative results:** P-core-derived decode tables mis-rank LZMA2 and reconstruction on ARM64; the default utility winner changes on slower cores.
- **Threats:** host heterogeneity (thermal design, OS scheduler); macOS lacks the Windows-style PDH covariates (a platform-specific substitute must be pre-registered); a device-class sample of one.

## 9. Cost and effort

When runnable: about 30 hours of timing wall per host for arms A and B; about 10 hours for arm C; about 5 hours for arm D; about 50 GB of disk per host. Agent effort: scripted plus judgment for the replication write-up.

## 10. Gates and status

BLOCKED (platform). All gates of the replicated experiments also apply. Held-out: platform replication of EXP-PLANNER-012 is out of scope unless a platform exists before Phase D.
