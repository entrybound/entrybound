# EXP-CROSSFILE-010: Dictionary trainer and archive determinism under emulated ARM64, restricted x86-64 CPU features, and libzstd 1.5.x release drift

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: (1) `ebr-crossfile determinism-cell` (fixed training-sample vectors in, Dictionary SHA-256 set out, plus packs of the input at three profiles); (2) per-release research builds pinning `zstd-sys` to each libzstd 1.5.x in `research/harness/zstd-drift/` (separate Cargo workspaces, never the production lock); (3) WSL user-mode emulation and cross toolchain: `qemu-user-static`, `gcc-aarch64-linux-gnu`, Rust target `aarch64-unknown-linux-gnu` for toolchain 1.98.1 (apt/rustup downloads, approved by the program's download policy). Docker is not required. |
| Domain / program sections | crossfile / 8.4 |
| decision_ids | DEC-CMP-021 |
| Evidence type | determinism claim tests (HC-13 MVT-13(a), HC-11 MVT-11 CPU-feature and architecture variation); `EMULATED` qualifier for the ARM64 and restricted-CPU arms (correctness and determinism only, never timing, §4.1) |
| Method | `research/decision-method.md`, pre-registered |
| Record | Design only; `record.md` (§12.2) instantiated once gate G-A holds. |
| Design session and disclosure | Phase C-design crossfile designer; L7 disclosure as in EXP-CROSSFILE-001. |

## 1. Question

Do trained Dictionaries (and the archives that embed them) stay byte-identical when the same
writer runs on emulated linux/arm64, on x86-64 with restricted CPU feature levels, and when the
bundled libzstd moves across 1.5.x releases; and if the trainer drifts across releases, which
DEC-CMP-021 candidate (pin with planner bump, vendored trainer, decoupled construction string)
does the measured drift frequency favour?

## 2. Hypotheses

- **H1a.** Training output is identical across x86-64 (native, `qemu-x86_64 -cpu qemu64`,
  `-cpu Nehalem`, `-cpu Haswell`) and emulated aarch64 for the same libzstd release (training is
  portable C without runtime dispatch in the dictionary builder).
- **H1b.** Training output differs between at least two libzstd 1.5.x releases for at least one
  sample vector (the dictionary builder has changed across 1.5.x), so the construction-string pin
  must bump planner IDs on upgrade.
- **H0.** No difference in any arm.
- **Falsification.** H1a: any Dictionary or archive difference between CPU/architecture cells at a
  fixed release (an HC-11/HC-13 violation artifact). H1b: identical Dictionaries for every vector
  across all tested releases.

## 3. Decision informed; proposed sets

As EXP-CROSSFILE-009 §3.

## 4. Cells (candidates are the DEC-CMP-021 policies; see EXP-CROSSFILE-009 §4)

| cell_id | Writer | Platform |
|---|---|---|
| `x86-native-1.5.7` | production planner (research build, `zstd` 0.13.3 / libzstd 1.5.7) | WSL x86-64 native |
| `x86-qemu64-1.5.7`, `x86-nehalem-1.5.7`, `x86-haswell-1.5.7` | same static binary under `qemu-x86_64 -cpu <model>` | WSL, `EMULATED` |
| `arm64-qemu-1.5.7` | same source cross-built for aarch64 | WSL `qemu-aarch64`, `EMULATED` |
| `x86-native-1.5.{0,2,4,5,6}` | `ebr-crossfile determinism-cell` built against the `zstd-sys` release bundling that libzstd (exact crate versions resolved and recorded from crates.io metadata at build time) | WSL x86-64 native |

Research builds use the production planner and codec code unchanged except for the `zstd-sys`
version in a research-only lock file; `lock_parity.py` is expected to report the intended
difference only.

## 5. Inputs

- **Training vectors:** for every cohort that attempted training in EXP-CROSSFILE-003's tuning run
  (`sq-abs128`, all profiles), the exact ordered sample set (samples in Chunk-digest order,
  truncated to the profile cap) serialised as a vector file with its SHA-256; plus the same
  procedure over the 13 inputs of EXP-CROSSFILE-009. Vectors are stored under
  `/root/eb-research/cache/crossfile/EXP-CROSSFILE-010/vectors/` and listed by SHA-256 in the
  record addendum before the run.
- **Archives:** the 13 inputs of EXP-CROSSFILE-009 at balanced, dense and extreme.
- Tuning split only; validation counterparts before the freeze; held-out placeholder as below.

## 6. Environment and platform requirements

WSL2 x86-64 Linux with user-mode QEMU (no Docker, no privilege beyond root inside WSL for apt);
ARM64 and restricted-CPU arms are `EMULATED`: determinism and correctness only. Native ARM64 and
macOS are EXP-CROSSFILE-011 (BLOCKED). `timing: false`.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
PY=/root/eb-research/venv/bin/python
$PY -m ebr validate --spec research/experiments/EXP-CROSSFILE-010/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-CROSSFILE-010/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-CROSSFILE-010/spec.yaml
$PY -m ebr normalize --spec research/experiments/EXP-CROSSFILE-010/spec.yaml
```

Per-sample command (candidate-level in `spec.yaml`): each cell runs one `determinism-cell`
invocation that trains every vector and packs the input at balanced, dense and extreme with the
`sq-abs128` rule, printing one JSON line with set digests. `{prefix}` is empty (native),
`qemu-x86_64 -cpu <model> ` or `qemu-aarch64 -L /usr/aarch64-linux-gnu `; `{bin}` is the cell's
research build under `/root/eb-research/target/zstd-drift/<build>/`.

```text
{prefix}{bin} determinism-cell --vectors /root/eb-research/cache/crossfile/EXP-CROSSFILE-010/vectors
  --item-path {item_path} --item-id {item_id} --profiles balanced,dense,extreme
  --archive-dir {scratch} --experiment-id EXP-CROSSFILE-010 --env-name wsl-ubuntu
```

Inputs come from the EXP-CROSSFILE-009 input root (`/root/eb-research/scratch/xf009-inputs`).

## 8. Seeds, warmups, repetitions

Seeds `order` 20260917101, `bootstrap` 20260917102. Warmups 0. Repetitions 3 per cell (MVT-13(a));
no stress cell here (emulation serialises execution; the stress cell is EXP-CROSSFILE-009's).

## 9. Metrics

| Metric | Role | ebr family |
|---|---|---|
| `dictionary_vector_set_sha256` (SHA-256 over the ordered list of per-vector Dictionary SHA-256) | binary cross-cell equality | correctness |
| `dictionary_mismatch_count` (per release pair, generated at analysis from the per-vector list the tool writes beside the row) | drift frequency (descriptive) | other |
| `archive_set_sha256`, `crossfile_physical_set_sha256` (over the three profile packs) | binary cross-cell equality (same release) | correctness |
| `vectors_trained` | coverage (descriptive) | other |
| `artifact_bytes_total` | descriptive (size effect of drift) | size |

## 10. Normalization and statistical analysis

Binary rules as EXP-CROSSFILE-009 §10 across cells at a fixed release (HC-11/HC-13 violations are
R1 for the status quo on that platform class). Across releases, drift is not a violation (the
construction string names the release); the analysis reports the fraction of vectors whose
Dictionary changes per release step and the resulting `artifact_bytes` change, which is the
empirical input to the C7 churn cost of each DEC-CMP-021 candidate.

## 11. Practical-significance thresholds

None for the binary tests; `artifact_bytes` drift effects are reported against T-01 for context.

## 12. Sensitivity analysis

Not applicable (binary). Robustness: two QEMU versions if available from apt (recorded), because
an emulator defect could masquerade as nondeterminism; a single-cell mismatch is re-checked under
the second emulator before being recorded as a violation only if the §2.0 rule 2 infrastructure
exception can be argued (otherwise it stands).

## 13. Validation look and held-out placeholder

Validation counterparts before the freeze; `EXP-CROSSFILE-010-HO` at Commit A (§5.5), once after
unlock.

## 14. Expected negative results worth recording

- The dictionary builder changed between 1.5.x releases for a sizable share of vectors, so any
  libzstd upgrade silently changes trained Dictionaries unless construction strings and planner IDs
  bump (supports `verified-pin-with-planner-bump` or vendoring).
- Emulation cannot detect native-ARM64 compiler differences (NEON paths are not used by the
  trainer, but the claim remains unverified natively: EXP-CROSSFILE-011).

## 15. Threats to validity

QEMU user-mode is not native silicon; `-cpu` models restrict CPUID but not compiler-selected
instruction paths chosen at build time; `zstd-sys` releases also change build flags; sample vectors
come from tuning cohorts only.

## 16. Cost

About 25 CPU-hours native plus about 60 CPU-hours emulated (roughly 10x slower); disk: vectors
about 1 GB, research builds about 6 GB of targets under `/root/eb-research/target/zstd-drift`.
Agent effort: scripted tooling (build matrix), none for execution, judgment for triage.
