# EXP-CROSSFILE-009: Writer determinism of dictionary- and group-bearing archives on the available x86-64 hosts (WSL2 ext4 and Windows NTFS)

| Field | Value |
|---|---|
| Status | **READY** (collection): production CLI builds on both hosts, the collector `crossfile_fingerprint.py` and the generator `make_dictionary_tree.py` exist and passed non-decision-grade smokes. The architecture, CPU-feature and libzstd-release arms are EXP-CROSSFILE-010 (NEEDS_TOOLING); native ARM64 and macOS are EXP-CROSSFILE-011 (BLOCKED). |
| Domain / program sections | crossfile / 8.4 |
| decision_ids | DEC-CMP-021 |
| Evidence type | determinism claim test (objective HC-13 MVT-13(a) repeat-hash with variation cells and a stress cell; decision-method §4.13) |
| Method | `research/decision-method.md`, pre-registered |
| Record | Design only; `record.md` (§12.2) instantiated once gate G-A holds. |
| Design session and disclosure | Phase C-design crossfile designer; L7 disclosure as in EXP-CROSSFILE-001. |

## 1. Question

Are trained Dictionaries, TransformPlans, ChunkGroups and Chunk-to-plan assignments of archives
packed by the production writer byte-identical across repeated runs, CPU-affinity sets, locale
and time-zone settings, and across the two available hosts (WSL2 Linux ext4 and Windows 11 NTFS),
so that the status-quo construction-string pin (`zstd` 0.13.3 / libzstd 1.5.7) is empirically
supported on these hosts?

## 2. Hypotheses

- **H1.** Within each host, every repetition and variation cell yields one PCI per (input,
  profile); across hosts, whenever the Chunk digest multiset is identical (PCR equal), the
  cross-file physical fingerprint (plans, Dictionary ids, assignments) is identical.
- **H0 / violation.** Some cell yields two different outputs for identical input, configuration
  and build: an HC-13 violation artifact (objective §2.0 rule 2), eliminating
  `status-quo-construction-string-pin` at R1 unless an infrastructure fault is proven.
- **Detection bound.** The 20-repetition stress cell detects a per-run mismatch probability of
  0.139 or more with 95% probability (Appendix D.5); rarer races are regression evidence only.

## 3. Decision informed; proposed sets

| Decision | Proposed type | Proposed od_ids | Proposed hc_ids |
|---|---|---|---|
| DEC-CMP-021 | FORMAL + EMPIRICAL (the dependency and trainer-algorithm parts are formal/cost; the byte identity is empirical) | none graded (binary HC test); OD-22 cost inputs | HC-11, HC-13, HC-16 |

## 4. Candidates

The ledger candidates are policies; this experiment supplies the empirical input they share:

| candidate_id | What this experiment contributes |
|---|---|
| `status-quo-construction-string-pin` | Direct test on two hosts (pass/fail) |
| `verified-pin-with-planner-bump` | The cross-platform byte-identity test suite it requires (this spec is that suite for x86-64 Windows and Linux) |
| `vendored-deterministic-trainer` | Baseline for comparison: if the pin passes everywhere measurable, the vendoring benefit is limited to upgrade drift (EXP-CROSSFILE-010) |
| `determinism-scope-excludes-dictionaries` | Screened at R1 against HC-13 (unencrypted `--deterministic`/default-deterministic packs must be byte-identical); admissible only as the variant below |
| `deterministic-mode-disables-trained-dictionaries` (designer-proposed variant) | Measured cost: `artifact_bytes` difference of packs with dictionaries disabled comes from EXP-CROSSFILE-003 `no-dictionaries` |
| `decouple-construction-from-planner-id` | Formal/cost only (C1, C7); no measurement here |

Excluded before execution: none (no candidate changes bytes in this experiment).

## 5. Inputs

- Corpus items (tuning split; manifest `ed28b1b4…`): `f01-tuning-ripgrep-14-1-1-git`,
  `f01-tuning-zstd-v1-5-7-git`, `f03-tuning-fzf-go-mod-vendor`, `f04-tuning-sourcelike`,
  `f05-tuning-gutenberg-books`, `f05-tuning-logrotate-from-loghub-hdfs`,
  `f06-tuning-census-popest-2023`, `f06-tuning-gen-structured-a-small`,
  `f08-tuning-chinook-sqlite`, `f18-tuning-lz4-releases-1-9-to-1-10`, `f17-tuning-duptree-mixed`,
  `f18-tuning-zstd-releases-1-5-x` (rule: every small cross-file target item except
  `f19-tuning-zoo`, which `ebound pack` refuses, and the F07/F08 numeric/database items without
  cross-file redundancy, plus the two medium items with the most files in F17 and F18).
- Generated: `xf009-json-dictionary-tree` from `make_dictionary_tree.py --seed 20260917091
  --count 4000` (ASCII names, no links; identical bytes on both hosts, content digest printed by
  the generator and recorded).
- **Input roots.** Both hosts pack from a prepared input root so that the selector is identical:
  - WSL: `/root/eb-research/scratch/xf009-inputs/tuning/<item_id>` = `cp -a` of
    `/root/eb-research/corpus/tuning/<item_id>` plus the generated tree.
  - Windows: `D:/eb-research/xf009-win/tuning/<item_id>` = `robocopy /E /COPY:D /DCOPY:D /R:0 /W:0`
    from `\\wsl.localhost\Ubuntu\root\eb-research\scratch\xf009-inputs\tuning\<item_id>` (data only;
    symlinks become files or are skipped) plus the generated tree regenerated natively.
  Cross-host comparisons are made only where the Chunk digest multiset matches (equal PCR);
  capture-divergent items are excluded and reported (F-0001/F-0002 explain AUX/LAI/PCI
  differences, which are out of scope here).
- Observed in the design smoke (non-decision-grade): small similar-file trees did not trigger
  trained Dictionaries under balanced; dense produced ChunkGroups on one variant. The stress-cell
  rule below therefore selects the input by measured cross-file structure, and a pass without any
  Dictionary-bearing archive is recorded as "trained-dictionary path not exercised", not as a
  determinism pass for Dictionaries.

## 6. Environment and platform requirements

- WSL2 Ubuntu 24.04 (`wsl-ubuntu`), x86-64; Windows 11 host (`windows-host`), x86-64, non-admin.
- Production CLI built from the production workspace at the pre-registration commit on each host
  with the pinned toolchain:
  - WSL: `git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/xf-prod && cd
    /root/eb-research/src/xf-prod && git checkout <pre-registration sha> &&
    CARGO_TARGET_DIR=/root/eb-research/target/prod-cli /root/.cargo/bin/cargo +1.98.1 build
    --release -p entrybound-cli` giving `/root/eb-research/target/prod-cli/release/ebound`.
  - Windows: same commit, `%USERPROFILE%\.cargo\bin\cargo.exe +1.98.1 build --release -p
    entrybound-cli` with `CARGO_TARGET_DIR=D:/eb-research/target/prod-cli-win`.
  - `Cargo.lock` SHA-256 and `rustc -vV` recorded per host.
- `timing: false`; no network; no privilege. Background load for the stress cell: seeded busy-loop
  processes on all vCPUs except the pinned one (`stress_load.py` equivalent inline in the command).

## 7. Commands

```sh
# WSL
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
PY=/root/eb-research/venv/bin/python
$PY -m ebr validate --spec research/experiments/EXP-CROSSFILE-009/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-CROSSFILE-009/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-CROSSFILE-009/spec.yaml
$PY -m ebr normalize --spec research/experiments/EXP-CROSSFILE-009/spec.yaml
```

```powershell
# Windows (PowerShell 7), after the WSL run and the input copy
$env:PYTHONPATH = "D:/Projects/entrybound/entrybound/research/tools"
C:/Python313/python.exe -m ebr validate --spec research/experiments/EXP-CROSSFILE-009/spec-windows.yaml
C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-CROSSFILE-009/spec-windows.yaml
C:/Python313/python.exe -m ebr verify-raw --spec research/experiments/EXP-CROSSFILE-009/spec-windows.yaml
C:/Python313/python.exe -m ebr normalize --spec research/experiments/EXP-CROSSFILE-009/spec-windows.yaml
```

The stress cell is `spec-stress.yaml`, generated after the first WSL pass by the rule "the input
and profile with the largest `dictionary_chunk_count`, ties broken by `grouped_chunk_count`, then
by item id" and committed before it runs (20 repetitions, extreme background load). The cross-host
comparison is `compare_hosts.py` (analysis script, to be written before analysis; it only joins
the two normalized outputs by (item, profile) and applies the rule of §10).

## 8. Seeds, warmups, repetitions

Seeds `order` 20260917092, `bootstrap` 20260917093; generator seed 20260917091. Warmups 0.
Repetitions 3 per variation cell (objective MVT-13(a)); stress cell 20. Variation cells (WSL):
`aff1-C-UTC` (taskset one vCPU, `LC_ALL=C`, `TZ=UTC`), `aff8-de-syd` (8 vCPUs, `LC_ALL=C.UTF-8`,
`TZ=Australia/Sydney`), `affall-C-utc` (all vCPUs); Windows: `os-default` and `aff1` (process
affinity mask to one logical processor via `Start-Process`-free `ProcessorAffinity` in the
wrapper). Thread count is not a production writer parameter (no `--threads` flag in
`ebound pack`, confirmed from the CLI usage text), so thread-count cells of MVT-13(a) are recorded
as not applicable with that citation.

## 9. Metrics

| Metric | Role | ebr family | Source |
|---|---|---|---|
| `archive_sha256` (PCI bytes) | within-host repeat-hash (binary) | correctness | `path_sha256` of `{scratch}/out.eb` |
| `crossfile_physical_sha256` | within- and cross-host determinism of plans, Dictionaries, assignments (binary) | correctness | `stdout_json` |
| `plans_sha256`, `assignment_sha256` | localisation of a mismatch | correctness | `stdout_json` |
| `pcr` | cross-host comparability gate (equal Chunk digests) | correctness | `stdout_json` |
| `dictionary_count`, `dictionary_chunk_count`, `group_count`, `grouped_chunk_count` | exercise coverage (descriptive) | other | `stdout_json` |
| `artifact_bytes` | descriptive | size | `path_size` |

## 10. Normalization and statistical analysis

No statistics: binary HC test. Rules:
1. Within a host, all valid samples of an (input, profile) across cells and repetitions must share
   one `archive_sha256`; otherwise both outputs are committed under
   `research/raw/EXP-CROSSFILE-009/violations/` and the status quo is eliminated at R1 for that host
   class (§4.13), unless the stored artifact re-hash differs from its recorded hash.
2. Across hosts, for (input, profile) pairs with equal `pcr`, `crossfile_physical_sha256` must be
   equal; otherwise a cross-host HC-13/HC-11 violation for dictionary-bearing plans. Pairs with
   unequal `pcr` are capture-divergent and reported, not judged.
3. Coverage: the result is reported separately for archives with Dictionaries, with ChunkGroups,
   and with neither; a pass counts for a class only if at least one archive of that class exists.

## 11. Practical-significance thresholds

None (binary, §3.3 "output-hash equality where determinism is claimed").

## 12. Sensitivity analysis

Not applicable to a binary HC test (recorded as such). Robustness arm: repeat the whole WSL pass
after a reboot or `wsl --shutdown` (a new VM boot) on the stress input.

## 13. Validation look and held-out placeholder

HC tests run on tuning and validation before the freeze and on held-out after it (objective §2.0
rule 3). `spec-validation.yaml` (validation counterparts by the same selection rule) runs before
the freeze; `EXP-CROSSFILE-009-HO` at Commit A (§5.5) runs once after unlock.

## 14. Expected negative results worth recording

- The trained-dictionary path is not exercised by these inputs on the default profiles (no
  Dictionary accepted), so the determinism claim for Dictionaries rests only on EXP-CROSSFILE-010's
  forced training.
- A cross-host difference arises from capture (symlink or metadata handling) rather than from the
  planner, making host-to-host PCR equality rarer than expected for real trees.

## 15. Threats to validity

- Two x86-64 hosts share a CPU; CPU-feature and architecture variation are EXP-CROSSFILE-010/-011.
- Windows copy loses symlinks: handled by the PCR comparability gate.
- libzstd is statically built into both binaries from the same `zstd-sys` source, so a pass does
  not test upgrade drift (EXP-CROSSFILE-010).
- Stress load is synthetic; rare interleavings beyond the Appendix D.5 bound are untested (there
  are no production writer threads, so the race surface is small).

## 16. Cost

About 30 CPU-hours per host (13 inputs x 3 profiles x cells x 3 repetitions; extreme on the two
medium inputs dominates) plus the stress cell (about 5 CPU-hours); wall about 10 h per host, not
exclusive (no timing). Disk: WSL input copy about 0.3 GB, Windows copy about 0.3 GB on D:, raw
about 50 MB. Agent effort: none for execution (scripted); judgment only for violation triage.

## 17. Evidence this decision needs that this experiment does not produce

- **DEC-CMP-021:** the dependency audit of the `zstd-sys` C trainer and decoder (C3, C4, C7; ledger
  Task 31) belongs to the ecosystem domain; the vendored or pure-Rust trainer candidate's
  implementation cost is an assessed C2 line; cross-architecture and upgrade drift are
  EXP-CROSSFILE-010 (emulated) and EXP-CROSSFILE-011 (native, BLOCKED).
