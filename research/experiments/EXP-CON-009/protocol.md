# EXP-CON-009: Native ARM64 low-memory devices — startup latency, parser memory and default decoder ceilings

| Field | Value |
|---|---|
| Status | **BLOCKED** — `PLATFORM_BLOCKED`: no native ARM64 hardware on this program's hosts (standing decision: local Windows/WSL/Docker only; QEMU arm64 is `EMULATED` and valid only for correctness and determinism, never for memory or timing) |
| Exact missing access | One native ARM64 Linux host with 2 GiB RAM and one with 4 GiB RAM (for example Raspberry Pi 4/5-class boards or equivalent arm64 SBCs), 32 GB local storage each, SSH access, Ubuntu 24.04 arm64, Rust 1.98.1 aarch64 toolchain; optionally an arm64 cloud instance of the same memory classes (external service, needs owner approval) |
| Domain / program section | container / §11 (with §13, §17) |
| decision_ids | DEC-CON-005, DEC-CON-018, DEC-CON-015 |
| Decision type (proposal) | EMPIRICAL |
| OD / HC (proposal) | OD-07, OD-08, OD-10, OD-17; HC-06, HC-11 |
| Specs | `spec.yaml` (validates; paths are placeholders for the future host); `validation-items.txt` |
| Method, author, L7, gates | `../_index/container-conventions.md` §1-§2 |

## 1. Question

On native ARM64 devices with 2 GiB and 4 GiB RAM, do the candidate default decoder ceilings (status-quo 8 MiB window / 384 MiB working set, 128 MiB, 64 MiB with opt-in) prevent memory exhaustion (OOM kill) while opening, listing and extracting default-profile archives, and do production open and list latency and parser memory scale with entry count as on x86-64 (so that container-structure conclusions drawn on x86-64 hold for constrained devices)?

## 2. Hypotheses

- **H1.** Under `c8m_384m_sq`, no default-profile tuning archive is OOM-killed on the 4 GiB device, and at least one v6 JPEG-region archive (256 MiB reconstruction working set) or `extreme` LZMA2 archive is OOM-killed or refused on the 2 GiB device during `unpack`.
- **H2.** Under `c64m_optin`, no archive is OOM-killed on either device; refusals are named and occur before allocation.
- **H3.** The ratio of ARM64 to x86-64 WSL `open_latency_s` is stable across entry-count strata (the log-log slopes of EXP-CON-005 and this experiment differ by less than 0.1).
- **Falsification.** H1 by an OOM kill on the 4 GiB device or none on the 2 GiB device for the named archive classes; H2 by any OOM kill; H3 by slope intervals that exclude each other.

## 3. Decisions informed

| Decision | Supplied here (once unblocked) | Elsewhere now |
|---|---|---|
| DEC-CON-005 | Behaviour on low-memory devices under each default ceiling (the ledger's "platform-limited" evidence item) | Declared requirements and ceiling refusal shares: EXP-CONF-007, EXP-RECON-001; per-codec memory: EXP-CODEC-003 |
| DEC-CON-018, DEC-CON-015 | Whether startup latency and parser memory scaling hold on constrained ARM64 | x86-64 anchors: EXP-CON-005; prototypes: EXP-CON-001, EXP-CON-002 |

Until unblocked, every decision scope that includes ARM64 devices stays `INSUFFICIENT_EVIDENCE` for these parts (§8.4 platform block), or the decision scope excludes ARM64 explicitly.

## 4. Candidates

Default decoder ceilings: `c8m_384m_sq` (status quo `bootstrap_decode_policy`), `c128m` (zstd-CLI-like 128 MiB), `c64m_optin` (64 MiB with explicit opt-in for reconstruction-heavy archives). Operations: `open-list`, `unpack`. Production `ebound` built natively for aarch64 at `9e44608`.

## 5. Corpus selectors

Tuning small and medium items (`spec.yaml`), packed on x86-64 by `ebound-prod` (archives are platform-independent bytes; PCR equality across hosts is checked) and copied to the device; validation small and medium items for the look (`validation-items.txt`); the EXP-CON-005 F04 scale trees up to 1e6 entries for H3. Held-out: Phase D placeholder as EXP-CON-001 section 15.

## 6. Environment and platform requirements

Native Linux ARM64 (required, missing); 2 GiB and 4 GiB RAM classes; no swap (or swap disabled for the run, recorded) so that OOM behaviour is observable; kernel OOM killer events captured from `dmesg` (requires root on the device); quiet-machine guard with `max_loadavg_1m` 2.0; process memory via GNU time `%M` and cgroup `memory.peak`.

## 7. Commands (to run once unblocked)

```sh
# on the device
bash research/tools/container/arm64_probe.sh --self-test
PYTHONPATH=research/tools python3 -m ebr run --spec research/experiments/EXP-CON-009/spec.yaml --timing
```

Tooling to build when unblocked: `research/tools/container/arm64_probe.sh` (applies the named ceiling through `ebound` caller policy flags or the library policy, runs the operation under GNU time, records `desc_outcome` = OK / POLICY_REFUSED code / OOM_KILLED with the `dmesg` evidence), a native aarch64 build of `ebound-prod`, and an environment capture for the device registered in `research/environment/index.json`.

## 8. Seeds, warmups, repetitions

Seeds 19001-19003. Timing per §4.6 with the device registered as a new `env_id`; calibration `EXP-CAL-AA-<device>` and `EXP-CAL-KE-<device>` must pass before any timing verdict (§4.7). OOM outcomes are deterministic facts per run; each (archive, ceiling, operation) runs 3 times and any OOM is recorded.

## 9. Metrics

`desc_outcome` and `desc_oom_killed` (binary outcome rows; an OOM kill under a ceiling that declared the archive acceptable is an HC-06 violation of the declared-bound claim), `peak_rss_bytes` (T-06), `open_latency_s` and `list_latency_s` (T-08, process-level on the device, in-process arm if the harness builds natively).

## 10. Analysis

Outcome tables per ceiling, device class and operation (AGG-W); slope comparison against EXP-CON-005 (descriptive intervals); refusal-before-allocation check (named refusal with peak below R0 + H, reusing EXP-CONF-007's H definition).

## 11. Practical-significance thresholds

T-06 and T-08 of `research/methods/thresholds.json`.

## 12. Sensitivity analysis

Swap-enabled arm; profile arm (`fast` vs `extreme`); 4 GiB vs 2 GiB device class.

## 13. Expected negative results

Default ceilings that let the kernel OOM-kill the process on 2 GiB devices; ARM64 slopes steeper than x86-64 (memory bandwidth).

## 14. Threats to validity

Board-to-board variation (thermal throttling on passively cooled SBCs; record temperatures where readable); storage speed differences (microSD vs NVMe) confound open latency (record and stratify).

## 15. Phase D held-out placeholder

As EXP-CON-001 section 15, on the device.

## 16. Cost and effort (when unblocked)

About 40 device hours per device class; 20 GB storage per device; agent effort scripted plus device setup by the owner.
