# EXP-CON-007: Feature-bit tier census (encrypted, conversion, Windows host) and minimal-reader acceptance under tier policies

| Field | Value |
|---|---|
| Status | **READY**: `spec.yaml` runs with `ebound-prod`, harness `ebr-inspect-bytes` and `ebr-pack`, and `jq`; `spec-windows.yaml` runs after the Windows `ebound-prod` build and `stage_ntfs_copies.ps1`; the counterfactual analysis script `feature_tier_analysis.py` is written at analysis time from the rules in section 10 |
| Domain / program section | container / §11 (with §15, §20.1 tier semantics, §34) |
| decision_ids | DEC-CON-010, DEC-MOD-024 |
| Decision type (proposal) | EMPIRICAL (census and acceptance fractions); FORMAL for the per-bit misinterpretation analysis (whether an old reader ignoring a bit could misread meaning) |
| OD / HC (proposal) | OD-21 (M21.6 reader fragmentation), OD-26, OD-23, OD-04; HC-04, HC-07, HC-10, HC-16 |
| Division of labour | WSL profile-by-layout feature-bit census of plain archives: `EXP-PLANNER-001` (joined through the `balanced-indexed-join-key` cell). Reconstruction audit activation ablation and 0x4/0x8 minimal-reader refusal: `EXP-RECON-004`. Authentication coverage of ignorable items and unknown-bit behaviour of readers: `EXP-CONF-004`. Tier semantics under encryption: `EXP-CRYPTO-016`. This experiment adds the encrypted, conversion/preservation and Windows-host cells and the policy counterfactual over all of them |
| Specs | `spec.yaml` (WSL, every tuning item, 4 cells), `spec-windows.yaml` (Windows host, NTFS copies of small and medium tuning items, 9 cells); `census_row.sh`, `census_row.ps1`, `stage_ntfs_copies.ps1`, `ntfs-items.json`; `validation-items.txt` |
| Feasibility smoke | `feasibility-smoke/` (non-decision-grade tooling check of all five `census_row.sh` modes) |
| Method, author, L7, gates | `../_index/container-conventions.md` §1-§2 |

## 1. Question

Which incompatible, read-only-compatible and compatible feature bits do encrypted archives, legacy conversions (strict and preserving) and Windows-packed archives require on real inputs; how many bytes do the records behind auxiliary features (conversion provenance, preservation evidence, platform security metadata, reconstruction audits) cost; is the active feature set deterministic; and, under each candidate feature-bit activation and tier-assignment policy, what fraction of real archives must a minimal reader (baseline feature set only) or an older reader refuse?

## 2. Hypotheses

- **H1 (auxiliary bits gate readability).** Every Windows-packed archive requires `0x10000` (Finding F-0001), every strict conversion requires `0x2000`, and every preserving conversion requires `0x4000`; therefore under the status-quo all-incompat policy a content-only minimal reader refuses 100% of these archives, while under `auxiliary-as-ro_compat-or-compat` it refuses 0% of them for content extraction.
- **H2 (activation without bytes).** Under the status quo, `balanced`-profile archives set `0x4` and `0x8` and carry reconstruction audit records even when no reconstruction data is emitted (non-decision-grade smoke observation in `feasibility-smoke/`), so `minimal-activation` would clear those bits on a majority of `balanced` tuning archives (to be confirmed jointly with EXP-PLANNER-001 rows).
- **H3 (determinism).** The active feature set of plain cells is identical across the two repetitions and equal to EXP-PLANNER-001's for the join-key cell; encrypted cells have identical public feature words across repetitions.
- **H0 / falsification.** H1 falsified by one Windows-packed archive without `0x10000`, or one conversion without `0x2000`; H2 if `0x4`/`0x8` appear only with reconstruction bytes; H3 by any feature-set difference between repetitions (a determinism violation artifact).

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CON-010 | Share of archives per cell requiring each bit; auxiliary record bytes per archive; determinism of active sets; refusal fractions under each activation policy | Planner-profile census: EXP-PLANNER-001; audit activation ablation: EXP-RECON-004; AUX verification impact of skippable auxiliary records: EXP-CONF-004 |
| DEC-MOD-024 | Per-bit analysis of whether ignoring the bit can change meaning (FORMAL table) joined with real-archive prevalence; refusal fractions under each tier-assignment policy | Encrypted feature assignments: EXP-CRYPTO-016; platform-metadata semantics (F-0001 follow-ups): platform domain |

## 4. Candidates

Measured cells (status quo, production at `9e44608`):

| cell | Definition |
|---|---|
| `balanced-indexed-join-key` (WSL) | `ebound-prod pack --profile balanced --layout indexed`; joins rows to EXP-PLANNER-001 |
| `balanced-indexed-encrypted` (WSL and Windows) | harness `ebr-pack --encrypt true` (password), public feature words from preamble offset 16 (`preamble_feature_words_hex`) and byte accounting |
| `convert-strict-zip-members` (WSL) | `ebound-prod convert <member> --strict` for every ZIP-family member file of the item (at most 20, sorted) |
| `convert-preserve-zip-members` (WSL) | `ebound-prod convert <member> --preserve --compat=zip/python-zipfile@3.13.5` |
| `<profile>-<layout>` × 8 (Windows) | `ebound-prod.exe pack` on NTFS copies, all four profiles × INDEXED/STREAM |

Policy candidates evaluated counterfactually on the census (no additional runs):

- DEC-CON-010: `status-quo-always-set-cumulative`, `minimal-activation`, `independent-composable-bits`, `auxiliary-as-ro_compat-or-compat`, `derived-supported-mask`, `exact-presence-with-documented-implication`.
- DEC-MOD-024: `all-incompat-status-quo`, `auxiliary-as-compat`, `independent-feature-bits`, `writer-tristate-activation`, `profile-level-bits`, `derived-supported-mask`.

No exclusions. Every policy candidate that changes the bit a writer emits for an existing identifier is a new identifier assignment (HC-16: existing archives keep their bits; the counterfactual describes future writers), costed T2 or higher by the assessor.

## 5. Corpus selectors

- `spec.yaml`: every tuning item (112). Convert cells produce `NO_INPUTS` for items without ZIP-family member files (recorded, not failures); ZIP-family members exist in F11, F14 and some F03/F09/F10 items.
- `spec-windows.yaml`: tuning small and medium items (95) staged as NTFS copies by `stage_ntfs_copies.ps1` (staging report committed with the run; symbolic links that the non-admin session cannot create, and names the 9P bridge cannot carry, are recorded fidelity gaps).
- Validation (CT-4): `validation-items.txt` (every validation item; Windows arm validation small and medium items). Held-out: none (section 15).

## 6. Environment and platform requirements

`wsl-ubuntu` (ext4) and `windows-host` (NTFS, non-admin). Deterministic census, no timing. Linux x86-64 and Windows x86-64 required; macOS/APFS capture (the third platform whose metadata sets bits) is `PLATFORM_BLOCKED` and is excluded from the decision scope explicitly. Disk: transient archives only (deleted after each row), NTFS copies about 20 GB on D:.

## 7. Commands

```sh
# WSL (READY)
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CON-007/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CON-007/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-CON-007/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-CON-007/spec.yaml
```

```powershell
# Windows host (after building ebound-prod.exe per conventions §3 and the harness release build)
pwsh -File research/experiments/EXP-CON-007/stage_ntfs_copies.ps1 -Split tuning
C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-CON-007/spec-windows.yaml
```

```sh
# counterfactual analysis (written at analysis time from section 10; its rules are fixed here)
python3 research/tools/container/feature_tier_analysis.py --census research/normalized/EXP-CON-007 --planner-census research/normalized/EXP-PLANNER-001 --bit-table research/experiments/EXP-CON-007/bit-semantics.md --out research/normalized/EXP-CON-007/decision/
```

`bit-semantics.md` (FORMAL table, written before the analysis by a session not involved in the policy candidates and reviewed by a second): for every assigned bit (`0x1`, `0x2`, `0x4`, `0x8`, `0x10`, `0x20`-`0x1000` crypto, `0x2000`, `0x4000`, `0x8000`, `0x10000`), the records it gates, whether a reader ignoring those records could produce different entry bytes, paths, kinds or LAI-level metadata (misinterpretation → must stay incompat), only different AUX-level metadata or reports (candidate ro_compat/compat), and the citation (document and line) supporting the classification.

## 8. Seeds and repetitions

Seeds 17001-17003 (WSL), 17011-17013 (Windows). Two repetitions per cell (deterministic census); plain archives repeat-hashed (`desc_archive_sha256`); encrypted archives differ by design and are compared on public feature words only.

## 9. Measurement method and metrics

| Metric | OD | Role | Family | T-ID |
|---|---|---|---|---|
| `desc_incompat_bits`, `desc_ro_compat_bits`, `desc_compat_bits`, `desc_incompat_values`, `desc_preamble_feature_words_hex` | OD-21, OD-26 | census (descriptive) | other / correctness | none |
| `side_data_bytes__reconstruction_section`, `side_data_bytes__metadata_sections` | OD-05 | diagnostic (auxiliary record cost) | size | T-02 |
| `metadata_bytes`, `artifact_bytes` | OD-05 | context | size | T-02 / T-01 |
| `reader_fragmentation_count` (derived per policy: number of optional or extended features whose presence makes an archive unreadable by a minimal baseline reader) | OD-21 (M21.6) | cost input | other | none |
| `benign_false_refusal_fraction` (derived per policy and reader class: fraction of real archives a minimal or older reader must refuse although it could extract content correctly) | OD-26 | **primary** for the policy comparison | other | T-20 |
| `desc_converted`, `desc_refused`, `desc_refusal_codes`, `desc_reconstruction_audit_count`, `desc_entry_count`, `desc_planner_id`, `desc_pack_outcome` | — | descriptive | other / correctness | none |

## 10. Normalization and statistical analysis

1. **Join.** Plain WSL rows of EXP-PLANNER-001 (8 profile × layout cells) are joined with this experiment's rows by item and the join-key cell (bits must agree; disagreement is a determinism finding).
2. **Reader classes.** `minimal-baseline` (supports only the baseline features the conformance domain defines for the minimal reader), `v-older` (supports the features of the planner-v4 generation, i.e. without `0x4`, `0x8`, `0x2000`, `0x4000`, `0x8000`, `0x10000`), `content-only` (supports every content-affecting feature but no auxiliary metadata features).
3. **Policy counterfactual.** For each policy and archive, recompute the bits a writer following the policy would emit (rules: `minimal-activation` sets a bit iff its records are non-empty in the byte accounting; `auxiliary-as-ro_compat-or-compat` moves bits classified AUX-only in `bit-semantics.md`; `independent-composable-bits` removes implied dependencies; `derived-supported-mask` and `exact-presence-with-documented-implication` change reader rules only), then decide acceptance per reader class: refuse iff an unknown bit is in the incompat tier (ro_compat unknown bits allow read, per SPEC §20.1). `benign_false_refusal_fraction` = refused archives whose content the reader class could extract correctly by the `bit-semantics.md` classification, over all archives in the cell.
4. **Aggregation.** Deterministic fractions per cell with T-20 floors; L2-L4 aggregation over families for the corpus-level policy verdict (§4.9); evaluation conditions: packing host (WSL, Windows) and cell type (native, encrypted, conversion), never pooled (AGG-S).
5. **Selection.** Policies are compared under R4-R10 with cost tiers from §7.2 (reader-rule-only policies T0-T1; writer bit reassignment for new identifiers T2; tier changes of assigned bits for future writers T2-T4 as assessed). A policy whose counterfactual lets a reader misinterpret content under `bit-semantics.md` is eliminated at R1(b) (HC-04), with the written counterexample.

## 11. Practical-significance thresholds

T-01, T-02, T-20 as transcribed in `research/methods/thresholds.json`. Not restated.

## 12. Sensitivity analysis

(a) Reader-class arm (the three classes above); (b) cap arm for conversion cells (first 5 vs first 20 members); (c) profile arm on Windows (all four profiles, from `spec-windows.yaml`); (d) classification arm: re-run the counterfactual with every disputed `bit-semantics.md` row set to the stricter class; (e) family arms of §6.5 for the corpus-level fraction.

## 13. Adversarial search and expected negative results

- `adversarial-search/`: for each bit proposed as ro_compat or compat, construct (in the conformance-case style, format-building code) an archive where ignoring the gated records changes extracted bytes, paths or LAI-level metadata; any success moves the bit back to incompat and is recorded as the R1(b) counterexample.
- Negative results worth recording: auxiliary bits that cannot be demoted because their records carry content-affecting data; STREAM or encrypted cells that must keep incompat bits; Windows archives whose security descriptors are required for correct restoration semantics under the default policy.

## 14. Threats to validity

NTFS copies are not native Windows-authored trees (copied metadata; reported); convert cells cover ZIP-family inputs only (tar and 7z conversion bits: legacy domain); the minimal-reader definition depends on the conformance domain's baseline set (sensitivity (a)); encrypted archives come from the harness packer (plain parity proven, encrypted bytes randomized); counterfactual policies are not implemented writers (bit recomputation rules are exact given the byte accounting, but a real writer might emit different records). L7 exposure (conventions §1).

## 15. Phase D held-out placeholder

As EXP-CON-001 section 15 (the census cells run once on the frozen held-out items; the counterfactual is recomputed with the frozen rules).

## 16. Cost and effort

Compute: WSL pass about 10 h (4 cells × 112 items × 2 repetitions; plain join-key packs shared with the pack cache), Windows pass about 12 h (9 cells × 95 items × 2) plus staging about 3 h, validation about 18 h: about 45 machine hours. Disk: about 20 GB NTFS copies; transient archives. Agent effort: judgment for `bit-semantics.md` (two sessions) and the analysis record; scripted otherwise.
