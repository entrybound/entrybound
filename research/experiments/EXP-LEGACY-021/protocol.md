# EXP-LEGACY-021: adapter priority scoring — evidence rules, decoder dependency audit and classification stability

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (`adapter_score.py`, `dep_audit.py`, `osv_query.py`; `cargo install` of cargo-geiger, cargo-audit, cargo-deny, tokei at pinned versions; pinned RustSec advisory-db clone; depends on EXP-LEGACY-010 and EXP-LEGACY-020 outputs) |
| Domain / program sections | legacy / §26 (with §31 dependency questions, §34 v1 scope) |
| decision_ids | DEC-LEG-024, DEC-LEG-005, DEC-LEG-029, DEC-LEG-035, DEC-LEG-037, DEC-LEG-039, DEC-LEG-042, DEC-LEG-047, DEC-LEG-057, DEC-LEG-073, DEC-LEG-052, DEC-LEG-075, DEC-LEG-082, DEC-LEG-085 |
| Decision types (proposed) | EMPIRICAL inputs (prevalence, fidelity, dependency census) combined by a pre-registered rule; EXTERNAL parts (licence interpretation for RAR5/UnRAR and weak-copyleft decoders, §7.5; security review of legacy decrypt paths) |
| OD / HC (proposed) | OD-19, OD-22 (M22.1-M22.5 cost inputs), OD-24 (M24.1 cost input), OD-26; HC-12 (public specification for any decode step), licence screen §7.5 |
| Runner | Not runner-driven (no `spec.yaml`): deterministic analysis scripts over committed inputs; outputs carry provenance headers per §12.3 |
| Shared rules / L7 / gates | C§ header, C§2 |
| Timing | none |

## 1. Question

Given measured prevalence (EXP-LEGACY-010), fidelity and hazard evidence (EXP-LEGACY-020), decoder dependency costs and
parser security history, which candidate evidence rule classifies each candidate legacy import adapter and export
profile as CORE_V1, POST_V1_CORE, ECOSYSTEM or NOT_JUSTIFIED, and how stable is each format's class under perturbation
of the rule's thresholds and weights?

## 2. Hypotheses

- **H1 (rule sensitivity).** At least two of the pre-registered rules disagree on the class of at least three formats.
  *Falsified by* agreement of all rules on all but two formats.
- **H2 (7z core status, DEC-LEG-024).** Under `prevalence-threshold-rule`, strict 7z import stays CORE_V1 only if S-7Z
  prevalence's lower 95% bound in at least one general stratum is ≥ 1%; predicted: it does not (POST_V1_CORE)
  `[INFERRED]`.
- **H3 (stability).** The class of cpio, ar and OCI layers is preserved in ≥ 90% of threshold and weight perturbations;
  RAR5, LHA and XAR are NOT_JUSTIFIED in ≥ 90% `[INFERRED]`. *Falsified* per format by a lower preservation fraction.
- **H4 (decoders, DEC-LEG-052/075/085).** For Deflate64, PPMd, BCJ2 and RAR5 no pure-Rust decoder crate meets all four
  §7.2 maintenance conditions with zero `unsafe`; for zstd and bzip2 a pure-Rust option exists. *Falsified* per codec by
  the opposite finding.
- **H0.** Every rule yields the same classification and classes are insensitive to thresholds.

## 3. Decisions informed

| Decision | Supplied here |
|---|---|
| DEC-LEG-024 | The comparison of evidence rules and the stability of each class |
| DEC-LEG-005, 035, 037, 039, 042, 047, 057, 073 | Class per format under each rule, with the evidence table behind it |
| DEC-LEG-029 | Priority of candidate export profiles under the same rules (export evidence from EXP-LEGACY-020/030) |
| DEC-LEG-052, DEC-LEG-075, DEC-LEG-085 | Decoder dependency audit per codec/filter/method (cost inputs C3/C4) |
| DEC-LEG-082 | Gatekeeper census: which pipelines depend on which ZIP runtime (pip/uv, Maven/Gradle, Android PackageManager, NuGet, Go toolchain, Windows shell), cited from primary sources at pinned versions |

## 4. Candidates

DEC-LEG-024 (evidence rules): `status-quo-eleven-tokens` (the classification implied by the current `--from` tokens
and docs, extracted with `path:line`), `spec-tiers` (SPEC §14.4 tiers, cited by section and SHA-256), `r1-minimal`
(Research I minimal set, cited), `prevalence-threshold-rule` (pre-registered below), `libarchive-parity` (every format
libarchive 3.8.8 reads is at least POST_V1_CORE), `plugin-interface-only` (everything beyond ZIP/tar/streams is
ECOSYSTEM); R0 critic slot: `weighted-evidence-score` proposed for the critic's confirmation.

**`prevalence-threshold-rule` (pre-registered).** With E1 = lower 95% Clopper-Pearson bound of the format's share in its
best general stratum, E2 = number of cited gatekeeping pipelines, E3 = minimum over core EAM classes (path, kind,
content, executable, mtime) of `capture_fidelity_fraction`, E7 = public normative specification available:
CORE_V1 if E1 ≥ 1% and E3 ≥ 0.8 and E7; POST_V1_CORE if (E1 ≥ 0.1% or E2 ≥ 2) and E3 ≥ 0.5 and E7; ECOSYSTEM if
E2 = 1 or (E1 ≥ 0.1% and not E7); NOT_JUSTIFIED otherwise. A format with an unresolved licence question is capped at
ECOSYSTEM pending external review.

**`weighted-evidence-score` (critic slot).** Score = Σ w_d · z_d over normalised dimensions E1-E8 (E4 hazard classes,
E5 parser CVE count, E6 decoder dependency cost, E8 licence flag negative), classes by fixed score cut points; base
weights equal.

## 5. Inputs (no corpus access of its own)

- EXP-LEGACY-010 normalized tuning outputs for rule development and validation outputs for confirmation (look
  registry), format shares per stratum with intervals.
- EXP-LEGACY-020 normalized outputs: `capture_fidelity_fraction`, hazard table, reader agreement.
- Dependency audit (`dep_audit.py`) of candidate decoder crates, each in a throwaway workspace under
  `/root/eb-research/scratch/dep-audit/<crate>@<version>` (never the production workspace): lzma-rust2 0.20.0,
  liblzma/xz2, lzma-rs; bzip2 (bzip2-sys and libbz2-rs-sys backends); deflate64; zstd (zstd-sys) and ruzstd; ppmd-rust;
  sevenz-rust2; brotli-decompressor; lz4_flex; weezl; unrar; backhand; cpio; ar; cab; delharc; iso9660 readers;
  nix-nar; tar (OCI). Candidate crate versions are the latest releases at retrieval, pinned in
  `research/experiments/EXP-LEGACY-021/crates.lock.json`.
- OSV/NVD parser-vulnerability history 2016-2026 per reference implementation (libarchive, 7-Zip, unrar, wimlib,
  squashfs-tools, GNU cpio, binutils ar, lhasa, xar, libmspack/cabextract), queried through the OSV API with retrieval
  date; `[EXTERNALLY_SOURCED]` with record ids.
- Gatekeeper census `research/experiments/EXP-LEGACY-021/gatekeepers.md`: each claim cites official documentation or
  source at a pinned version with locator (§9.3).
- Held-out: the classification is re-evaluated once on the post-freeze EXP-LEGACY-010 sealed sample (placeholder, C§5).

## 6. Environment

`wsl-ubuntu`; network for crates.io metadata, OSV API and advisory-db clone; Rust 1.98.1 pinned toolchain.

## 7. Commands and tooling

```sh
cd /mnt/d/Projects/entrybound/entrybound
/root/.cargo/bin/cargo +1.98.1 install --locked cargo-geiger@<pinned> cargo-audit@<pinned> cargo-deny@<pinned> tokei@<pinned>   # versions recorded in crates.lock.json
git clone https://github.com/rustsec/advisory-db /root/eb-research/tools/advisory-db && git -C /root/eb-research/tools/advisory-db checkout <pinned commit>
/root/eb-research/venv/bin/python research/tools/legacy/dep_audit.py --crates research/experiments/EXP-LEGACY-021/crates.lock.json --advisory-db /root/eb-research/tools/advisory-db --out research/normalized/EXP-LEGACY-021/dep-audit.csv
/root/eb-research/venv/bin/python research/tools/legacy/osv_query.py --packages research/experiments/EXP-LEGACY-021/osv-packages.json --since 2016-01-01 --out research/normalized/EXP-LEGACY-021/osv.csv
/root/eb-research/venv/bin/python research/tools/legacy/adapter_score.py --prevalence research/normalized/EXP-LEGACY-010/tuning --fidelity research/normalized/EXP-LEGACY-020 --deps research/normalized/EXP-LEGACY-021/dep-audit.csv --osv research/normalized/EXP-LEGACY-021/osv.csv --gatekeepers research/experiments/EXP-LEGACY-021/gatekeepers.md --rules all --perturb --seed 2509172101 --out research/normalized/EXP-LEGACY-021/tuning/
/root/eb-research/venv/bin/python research/tools/legacy/adapter_score.py --prevalence research/normalized/EXP-LEGACY-010/validation --fidelity research/normalized/EXP-LEGACY-020 --deps research/normalized/EXP-LEGACY-021/dep-audit.csv --osv research/normalized/EXP-LEGACY-021/osv.csv --gatekeepers research/experiments/EXP-LEGACY-021/gatekeepers.md --rules all --perturb --seed 2509172102 --out research/normalized/EXP-LEGACY-021/validation/
```

`cargo install` and the advisory-db clone are downloads of public tools (approved); they do not modify the production
workspace.

## 8. Seeds

Perturbation sampling `2509172101` (tuning inputs), `2509172102` (validation inputs).

## 9. Replication

Deterministic scripts: run twice; outputs must be byte-identical (§12.3 item 4). Dependency audit rerun at a second
retrieval date ≥ 7 days later to record metadata drift (maintainers, advisories).

## 10. Metrics

| Metric | Canonical / role | Use |
|---|---|---|
| `decode_path_dependency_count`, `decode_path_native_unsafe_count`, `rustsec_advisory_count`, `maintainer_count`, `msrv_gap` | cost_input (C3) | E6, maintenance penalty §7.2 |
| `release_age_years` | descriptive | E6 context |
| `license_compatibility` | binary (R2 screen §7.5) | E8 |
| `baseline_codec_public_spec` | binary (HC-12) | E7 |
| `untrusted_input_loc` | cost_input (C4) | E6 |
| class per (format, rule); fraction preserving class under perturbation; OSV counts; gatekeeper counts | non-canonical descriptive | rule comparison |

## 11-12. Normalization and analysis

Each rule is applied deterministically to the tuning evidence table; classes and their stability are recorded; the
validation evidence table is then applied once. Stability: threshold perturbation ×0.5 and ×2 on every numeric
threshold of `prevalence-threshold-rule` (single and global, §6.4 pattern) and, for `weighted-evidence-score`, the §6.3
weight grid (factors 0.5-1.5) and Dirichlet (10,000 samples, concentration 4.0, seeded); per-format preservation
fraction against the §6.6 bars (≥ 0.90 pass, [0.80, 0.90) MEDIUM, < 0.80 fail). Leave-one-stratum-out on E1.

## 13. Practical significance

No banded primary metric is compared here; cost inputs feed §7 tiers; binary screens per §3.3/§7.5.

## 14. Sensitivity analysis

As §11-12, plus an evidence-source arm (E5 from OSV only versus OSV plus NVD) and a retrieval-date arm.

## 15. Expected negative results worth recording

A format the SPEC places in Tier 2 classified NOT_JUSTIFIED by every data-driven rule (or the reverse); 7z strict
import not justified as core by prevalence; no pure-Rust decoder meeting maintenance conditions for formats the status
quo already decodes.

## 16. Threats to validity

Prevalence frames are public-ecosystem biased (EXP-LEGACY-010 §16); gatekeeper citations are selected by the analyst
(a second, non-exposed session re-derives the census); rule thresholds are method choices `[INFERRED]` fixed now,
before any prevalence result exists.

## 17. Platform requirements

Linux x86-64; network; no privileged operations; licence interpretation beyond SPDX and RAR5 legal questions are
**EXTERNAL_REVIEW_REQUIRED** (§8.3), not resolved here.

## 18. Estimates

Compute ≈ 3 CPU-hours (crate builds for geiger); disk ≈ 10 GB. Agent effort: gatekeeper census judgment; the rest
scripted.

## 19. Expected cost tiers `[INFERRED]`

Classification changes themselves are T0-T1 (documentation/policy); adapters they admit carry EXP-LEGACY-020 §19 tiers.
