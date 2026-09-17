# EXP-LEGACY-031: byte-identical export scope — repetition, threads, environment, Windows/WSL, emulated arm64, source layout/profile/encryption, compression-backend and encoder-upgrade arms; golden vectors and receipt-based reproduction

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (arms A, B, F, G run with existing builds; arm C needs `qemu-user-static`, `gcc-aarch64-linux-gnu` and the `aarch64-unknown-linux-gnu` Rust target; arms D/E need scripted research clones with alternative `flate2` backends and upgraded codec crates; golden-vector generator; receipt reproduction script) |
| Domain / program sections | legacy / §27 (with §29 vectors, §31 dependency upgrades) |
| decision_ids | DEC-LEG-008, DEC-CMP-024, DEC-LEG-016, DEC-LEG-014, DEC-LEG-018 |
| Decision types (proposed) | EMPIRICAL (determinism observations) with FORMAL parts (golden vectors as exact data) |
| OD / HC (proposed) | OD-20 (M20.2), OD-04, OD-22 (C7 churn); HC-13 (MVT-13(a), (d)), HC-16, HC-11 |
| Shared rules / L7 / gates | C§ header, C§2 |
| Timing | `timing: false` |

## 1. Question

Under which variations do legacy export bytes stay identical for the same source EAM and target profile — repeated
runs, thread counts, locale/time zone/umask, Windows versus Linux, emulated arm64, different physical source structure
(layout, profile, encryption, repack), different compression backends and upgraded encoder crates — and can a target be
reproduced from a receipt?

## 2. Hypotheses

- **H1 (within build).** Export bytes are identical across ≥ 3 repetitions per cell, threads {1, 8, 32}, locales
  {C, tr_TR.UTF-8, ja_JP.UTF-8}, time zones {UTC, Pacific/Chatham} and umask {022, 077}, including the 20-repetition
  stress cell (`migration_determinism_fraction` = 1). *Falsified by* any two differing outputs (the violation artifact,
  §4.13).
- **H2 (source structure, MVT-13(d), DEC-LEG-016).** For one EAM sourced from INDEXED and STREAM layouts, all four
  profiles, encrypted and plain sources and `repack --index absent`, export bytes are identical per target profile.
  *Falsified by* one difference.
- **H3 (cross host).** Windows and WSL builds of the same commit produce identical export bytes for sources whose EAM is
  identical (EAMs packed once on one host and copied). *Falsified by* one difference.
- **H4 (emulated arm64).** An `aarch64-unknown-linux-gnu` build under QEMU produces identical bytes to x86-64.
- **H5 (backend/upgrade sensitivity, DEC-LEG-008/CMP-024).** At least one compressed profile (`zip/portable-v1` DEFLATE,
  `tar.gz`, `tar.zst`, `tar.xz`, `tar.bz2`) changes bytes under an alternative `flate2` backend or an upgraded codec
  crate; `tar/pax-v1` never changes. *Falsified by* no change anywhere (weakens vendoring and re-versioning candidates).
- **H6 (receipt reproduction, DEC-LEG-018).** Given only a v1/v2 receipt and the source archive, the target cannot be
  reproduced across a build change for compressed profiles (the receipt lacks encoder identity). *Falsified by*
  reproduction success on every profile.
- **H0.** Bytes are invariant under every variation.

## 3. Decisions informed

| Decision | Supplied here |
|---|---|
| DEC-LEG-008 | The observed scope of byte identity (build, profile, host, architecture, backend, crate upgrades) against candidates `status-quo-implicit`, `golden-vectors-ci`, `vendored-encoders`, `receipt-records-encoder-versions`, `reversion-on-change`, `stored-only-profiles` |
| DEC-CMP-024 | Golden vectors per wrapper profile and what happens under encoder upgrade (`status-quo-frozen-wrappers` vs `new-profile-on-encoder-change`); level/seekable parts in EXP-LEGACY-033 |
| DEC-LEG-016 | Whether source physical structure (chunk groups, codecs, layout, encryption) can reach target bytes |
| DEC-LEG-014 | Dimension-parity table: each Reproducible Builds / stabilize-style dimension (mtime, ordering, uid/gid/names, permissions and umask, locale, time zone, gzip header mtime/OS/name, pax header naming, padding) perturbed at the source and its effect on target bytes |
| DEC-LEG-018 | Receipt-field reproduction success across builds; which missing fields block it |

## 4. Candidates

As listed in §3 (ledger ids). `stored-only-profiles` is realised by `tar/pax-v1` (uncompressed) and a research
`proto-zip-stored-only` writer; `vendored-encoders` and `receipt-records-encoder-versions` are evaluated as rules over
arm D/E observations (what they would have prevented or recorded). No exclusions.

## 5. Corpus selectors

- Golden sources (`research/tools/legacy/cases/golden_sources.py`; F19/F04; seeds `S_T = 2509173101`,
  `S_V = 2509173102`): 12 small seeded trees covering each profile's code paths (STORE-vs-DEFLATE choice, pax-needed
  names, ZIP64 thresholds by a 70,000-entry tree, empty directories, executables, timestamps).
- Real sources (subset of EXP-LEGACY-030 §5): tuning `f01-tuning-zstd-v1-5-7-git`, `f04-tuning-sourcelike`,
  `f05-tuning-logrotate-from-loghub-hdfs`, `f12-tuning-kodak-jpeg-edge-cases-small`, `f19-tuning-real-home-snapshot`;
  validation `f01-validation-redis-7-2-16-git`, `f04-validation-objstore`, `f05-validation-logrotate-from-loghub-ssh`,
  `f12-validation-fsa-jpeg-matrix-medium`, `f19-validation-real-home-snapshot`.
- Held-out: placeholder (C§5).

## 6. Environment

`wsl-ubuntu` (arms A, C-F, G), `windows-host` native build `D:/eb-research/target/legacy-head-win/release/ebound.exe`
invoked through interop on NTFS copies of the same `.eb` sources (arm B), QEMU user-mode `aarch64` (arm C, `EMULATED`).
No timing.

## 7. Commands and tooling

Tooling: `cases/golden_sources.py`; `research/tools/legacy/export_determinism.py` (arms and perturbations; runs
`ebound convert <src.eb> <out> --to <t> --allow-lossy --receipt <r>`; records SHA-256); `build_variants.sh`
(`--variant flate2-zlib-rs|flate2-zlib-ng|crates-latest-compatible|crates-latest-major`, each in a throwaway clone under
`/root/eb-research/src/legacy-variant-<v>` with `Cargo.toml` edits only in that clone, never committed);
`build_aarch64.sh` (`rustup target add aarch64-unknown-linux-gnu`, linker `aarch64-linux-gnu-gcc`, run via
`qemu-aarch64-static`); `receipt_reproduce.py`; `dimension_parity.py`; golden set writer
`research/experiments/EXP-LEGACY-031/golden/expected.jsonl` (proposal for `golden-vectors-ci`, committed only after the
tuning run).

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
bash research/tools/legacy/build_variants.sh --commit <record commit> --variants baseline,head,flate2-zlib-rs,flate2-zlib-ng,crates-latest-compatible,crates-latest-major
bash research/tools/legacy/build_aarch64.sh --commit <record commit>
pwsh -NoProfile -File research/tools/legacy/build_windows.ps1 -Commit <record commit> -TargetDir D:/eb-research/target/legacy-head-win
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-031 --generator golden_sources --split tuning --seed 2509173101 --append-corpus-items research/experiments/EXP-LEGACY-031/real-items-tuning.txt
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-031 --generator golden_sources --split validation --seed 2509173102 --append-corpus-items research/experiments/EXP-LEGACY-031/real-items-validation.txt
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-031/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-031/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-031/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/export_determinism.py --analyze --experiment EXP-LEGACY-031 --out research/normalized/EXP-LEGACY-031/
```

`apt-get install qemu-user-static gcc-aarch64-linux-gnu` inside WSL is a package installation in the research VM (not a
host system setting) and is part of tooling.

## 8. Seeds

`order 2509173111`, `bootstrap 2509173112`, `command 2509173113`; source seeds §5; stress-cell background load seed
`2509173114`.

## 9. Replication

3 repetitions per variation cell; one stress cell per profile with 20 repetitions at 32 threads under seeded background
CPU load (objective MVT-13(a); Appendix D.5 detection bound: per-run mismatch probability ≥ 0.139 detected with 95%).
`spec.yaml` runs 3 repetitions per cell; the stress cell is the separate `spec-stress.yaml` (same driver, arm
`stress:threads=32;background-load-seed=2509173114`, 20 repetitions), run after `spec.yaml` under its own run id.

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `migration_determinism_fraction` (per arm, per profile) | binary | OD-20 | §3.3 |
| `receipt_completeness_fraction` (receipt carries every field needed to reproduce, per profile) | binary for claimed reproducibility | OD-20 | §3.3 |
| differing-byte offsets and first differing structure; dimension-parity table; receipt reproduction success per profile/build pair | non-canonical descriptive | — | none |

## 11-12. Normalization and analysis

Outputs compared by SHA-256; for differences, `diffoscope` (pinned) localises the first differing structure. Any within-
build difference is an R1 violation artifact without rerun (§4.13). Cross-build differences are not violations of the
status quo (it claims same-build determinism only) but are the evidence that separates the DEC-LEG-008 candidates.
Golden vectors are fixed from tuning outputs and checked on validation sources once.

## 13. Practical significance

Binary (§3.3); descriptive statistics carry no band.

## 14. Sensitivity analysis

Source-set arm (golden trees versus real items); stress load arm (with versus without background load); QEMU CPU model
arm (two `-cpu` models, objective MVT-11(a)).

## 15. Expected negative results worth recording

Nondeterminism under threads or locale (a production defect); Windows/WSL byte difference for identical EAMs; backend
changes altering DEFLATE bytes (vendoring or re-versioning needed); receipts insufficient for reproduction.

## 16. Threats to validity

Emulated arm64 is not native ARM64 (native **PLATFORM_BLOCKED**, EXP-LEGACY-080); crate-upgrade arm depends on API
compatibility (non-building upgrades recorded as not measurable); locale canaries must show variation takes effect
(Turkish dotted-i check, objective MVT-11(a)), else the cell is HC_UNVERIFIED.

## 17. Platform requirements

Linux x86-64 WSL2; Windows x86-64 native build (non-admin); QEMU user-mode arm64 (EMULATED); native ARM64 and macOS
**PLATFORM_BLOCKED**; storage ~40 GB (variant targets); network for crate downloads.

## 18. Estimates

Compute ≈ 50 CPU-hours (QEMU ×10 slowdown dominates) plus ≈ 6 build-hours; disk ≈ 40 GB. Agent effort: scripted;
judgment only for difference localisation and candidate evaluation.

## 19. Expected cost tiers `[INFERRED]`

`golden-vectors-ci`: T1 (permanent vectors, C7); `vendored-encoders`: T3 if vendored code contains `unsafe`/native, else
T2 plus maintenance; `receipt-records-encoder-versions`: T2 (new receipt fields, receipt v3); `reversion-on-change`: T2 per
new profile id over time; `stored-only-profiles`: T2 (new profile).
