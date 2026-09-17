# EXP-CROSSFILE-013: Legacy export invariance to the source archive's cross-file physical structure

| Field | Value |
|---|---|
| Status | **READY** (collection): production CLI export exists; the collector `export_case.py` exists and passed a non-decision-grade smoke (below). |
| Domain / program sections | crossfile / 8.5 (legacy export, program §27) |
| decision_ids | DEC-LEG-016 |
| Evidence type | binary determinism test (objective HC-13 MVT-13(d)) plus exact outcome census; formal argument for consumer read cost |
| Method | `research/decision-method.md`, pre-registered |
| Record | Design only; `record.md` (§12.2) instantiated once gate G-A holds. |
| Design session and disclosure | Phase C-design crossfile designer; L7 disclosure as in EXP-CROSSFILE-001. |

## 1. Question

Do physical-only differences of the native source (profile, per-entry codecs, trained Dictionaries,
ChunkGroups, layout) ever change a legacy export's bytes, outcome class (LOSSLESS, LOSSY, REFUSED)
or issue list, and is there any export target whose consumer read cost depends on source ChunkGroup
structure, so that physical differences are correctly compatibility notes rather than LOSSY or
REFUSED outcomes (confirming the repository resolution of SPEC §15.2)?

## 2. Hypotheses

- **H1 (status quo).** For every (item, target), `export_sha256`, `outcome` and `issues_sha256` are
  identical across every source configuration.
- **H0 / violation.** Some (item, target) differs across source configurations: an HC-13 MVT-13(d)
  violation ("legacy artifact whose bytes differ when the same EAM is sourced from different
  layouts, profiles or encryption modes").
- **Formal part.** Exported artifacts carry no Entrybound ChunkGroups or Dictionaries (the export
  consumes EAM, `docs/legacy-export-v1.md`; F-22), so the consumer's read cost is a function of the
  target profile only; this is recorded as `FORMALLY_DERIVED` with the doc citation, and H1's
  byte identity is its empirical check.

## 3. Decision informed; proposed sets

| Decision | Proposed type | Proposed od_ids | Proposed hc_ids |
|---|---|---|---|
| DEC-LEG-016 | FORMAL + EMPIRICAL | OD-19 (export outcomes, descriptive here), OD-20 | HC-07, HC-10, HC-13 |

## 4. Candidates

| candidate_id | Meaning | How this experiment bears on it |
|---|---|---|
| `status-quo-physical-not-semantic` | Physical choices never affect outcome or bytes | H1 is its direct test |
| `spec-lossy-and-refuse` | SPEC §15.2 text: per-entry codec loss LOSSY; chunk-group dependent exports REFUSED | Measured consequence: the fraction of source archives that would be REFUSED/LOSSY solely for physical structure (`source_group_count > 0` or multiple codecs) with no semantic difference in the export |
| `compatibility-note` | Record physical differences as receipt compatibility notes | Cost only (C1: receipt fields; ExportReceipt v1 is frozen, F-22, so a note needs a new receipt version, HC-16) |

Source configurations (the experiment's varied factor): `fast`, `balanced`, `dense`, `extreme`
(INDEXED), and `balanced-stream-auto` (STREAM layout, `--stream-window auto`). Encrypted sources
are out of scope here (export from encrypted sources is the crypto/legacy domains').

## 5. Corpus selectors

- Manifest `ed28b1b4…`, tuning only: the 50-item cross-file target set of EXP-CROSSFILE-001 §5
  (items the source pack refuses, such as `f19-tuning-zoo`, record `SOURCE_REFUSED` identically for
  every configuration), targets `tar`, `zip`, `tar.zst`.
- Validation counterparts (validation small and medium items of the same families) run before the
  freeze; HC tests also run on held-out after unlock (placeholder below).

## 6. Environment and platform requirements

`wsl-ubuntu`, x86-64 Linux, ext4; production CLI built from the production workspace at the
pre-registration commit (`/root/eb-research/target/prod-cli/release/ebound`, build command in
EXP-CROSSFILE-009 §6); `timing: false`; no network; no privilege.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
PY=/root/eb-research/venv/bin/python
$PY -m ebr validate --spec research/experiments/EXP-CROSSFILE-013/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-CROSSFILE-013/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-CROSSFILE-013/spec.yaml
$PY -m ebr normalize --spec research/experiments/EXP-CROSSFILE-013/spec.yaml
```

Per-sample command (in `spec.yaml`): `python3 research/experiments/EXP-CROSSFILE-013/export_case.py
--ebound <prod-cli> --item-path {item_path} --scratch {scratch}/case --profile {profile} --layout
{layout} --target {target}`, which packs, inspects source structure, exports with `--allow-lossy`
and a receipt, and prints one JSON line.

**Feasibility smoke (non-decision-grade, design only).** On a generated 300-file tree in WSL the
collector packed with `fast` and `balanced`, exported to `tar`, and produced identical
`export_sha256`, `outcome` and `issues_sha256` for the two sources, with differing receipt hashes
(receipts bind the source archive). It checks that the collector runs; nothing from it is a result.

## 8. Seeds, warmups, repetitions

Seeds `order` 20260917131, `bootstrap` 20260917132. Warmups 0. Repetitions 2 plus repeat-hash on
`export_sha256` within a configuration (a within-configuration mismatch is also an HC-13 violation).

## 9. Metrics

| Metric | Role | ebr family | Source |
|---|---|---|---|
| `export_sha256` | binary cross-configuration identity (MVT-13(d)) | correctness | `stdout_json` |
| `outcome`, `issues_sha256` | binary cross-configuration identity of the outcome class and issue census | correctness | `stdout_json` |
| `export_outcome_distribution` | descriptive (M19.4) per target | other | derived at analysis |
| `receipt_sha256` | descriptive (expected to differ: receipts bind the source identity) | correctness | `stdout_json` |
| `source_group_count`, `source_grouped_chunks`, `source_dictionary_chunks` | coverage of the physical factor (descriptive) | other | `stdout_json` |
| `export_bytes`, `source_bytes` | descriptive | size | `stdout_json` |

## 10. Normalization and statistical analysis

Binary rules: group rows by (item, target); every valid source configuration must share one
`export_sha256`, one `outcome` and one `issues_sha256`. A mismatch commits both outputs under
`research/raw/EXP-CROSSFILE-013/violations/` and is an R1 result against
`status-quo-physical-not-semantic`. Coverage is reported: the share of (item, target) cases whose
sources actually differed in ChunkGroups or Dictionaries; a pass on cases without such differences
does not test the question and is reported separately. For `spec-lossy-and-refuse`, the generated
table counts cases that candidate would classify LOSSY/REFUSED although the exported bytes are
identical (its false-refusal cost).

## 11. Practical-significance thresholds

None (binary); `benign_false_refusal_fraction` (T-20) is reported for `spec-lossy-and-refuse` as a
secondary quantity.

## 12. Sensitivity analysis

Not applicable (binary); coverage arm: repeat on the EXP-CROSSFILE-004 forced `chain-k8` and
EXP-CROSSFILE-003 forced dictionary archives once that tooling exists (guarantees physical
structure is present).

## 13. Validation look and held-out placeholder

Validation counterparts before the freeze; `EXP-CROSSFILE-013-HO` at Commit A (§5.5), once after
unlock.

## 14. Expected negative results worth recording

- Default profiles rarely create ChunkGroups on real items, so the invariance is tested mostly
  against codec and order differences (coverage limitation recorded).
- An export difference traced to source layout (STREAM versus INDEXED) rather than cross-file
  structure would be a legacy-domain defect.

## 15. Threats to validity

Receipts differ by design and are excluded from the identity test; issue lines are hashed as a
sorted multiset (order differences are not violations); export of symlink-bearing trees is refused
by the current build (`research/baselines/README.md`) identically across sources.

## 16. Cost

About 70 CPU-hours (extreme source packs on medium items dominate), wall about 5 h sharded; disk:
scratch about 3 GB, raw about 100 MB. Agent effort: none for execution; judgment only for violation
triage.
