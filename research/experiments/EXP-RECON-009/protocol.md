# EXP-RECON-009: JPEG region representation conflicts with exact dedup, similarity dictionaries and ChunkGroups — suppression frequency, archive-size and access effects of region-first and joint-cost selection, and audit-reason granularity

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-RECON-009 |
| Domain | reconstruction (8.8); common provisions: `research/experiments/_index/reconstruction-common.md` |
| Status | **NEEDS_TOOLING**. Required: research planner overrides `conflict-region-first` and `conflict-joint-cost` and writer override `conflict-split-audit-reasons` in `ebr-recon variants` (EXP-RECON-004 tooling); generator `gen_photo_library.py` (S-PHOTO-LIBRARY); `ebr-recon audit-oracle`. |
| Stage | Tuning (exploratory), then validation look 1 for DEC-CMP-039 (registered separately in `validation-looks.jsonl`, the same day as the EXP-RECON-004 look) |
| decision_ids | DEC-CMP-039, DEC-CMP-026, DEC-CON-010 |
| Proposed types/ODs/HCs | common §2: EMPIRICAL. OD-05 (`artifact_bytes`), OD-10 (`access_granularity_bytes`), explain accuracy (descriptive). HC-01 (one physical representation per Chunk; the `duplicate-representation-for-shared-chunk` exclusion), HC-02, HC-09 (declared access cost covers cross-object region dependence), HC-16 (frozen v6 audit reasons unchanged; the split reasons are a new audit version) |
| Gates | G-A; G-B; MDE gate |
| Author and exposure | common §3 |
| Feasibility smoke | none |

## 2. Question

- How often does the status-quo rule suppress a JPEG whole-object region because a member Chunk is shared with another ContentObject, or already belongs to a similarity cohort Dictionary or ChunkGroup? This is measured on real items and on photo libraries with duplicates and edits.
- What archive-size and access-cost effects do region-first and joint-complete-cost selection have?
- Does separating dedup-sharing from cohort-conflict audit reasons improve explanation accuracy, and at what byte cost?

## 3. Hypotheses

All hypotheses are `[INFERRED]`.

- **H1 (rarity on real items).** On real tuning items (F12, F17, F18, F19), region suppression by dedup or cohort conflict affects < 1 % of JPEG file bytes, in both dense and extreme.
  - **Falsified if** ≥ 1 %.
- **H2 (edited libraries).** On S-PHOTO-LIBRARY tuning libraries in dense:
  - the status quo suppresses regions for ≥ 50 % of metadata-edited copies;
  - `conflict-joint-cost` is smaller than `conflict-status-quo` by more than 1 T-01 band unit on `artifact_bytes` at library level.
  - **Falsified if** either condition fails.
- **H3 (access cost of region-first).** `conflict-region-first` increases `access_granularity_bytes` for entries that share Chunks with another object's region beyond the T-09 band, compared with the status quo.
  - **Falsified if** within band.
  - **Not evaluable** if the production reader rejects such archives (§5).
- **H4 (audit cost).** `conflict-split-audit-reasons` changes `artifact_bytes` by less than the T-02 floor (64 B) per archive, and raises audit-reason accuracy (§11 step 5) from < 1 to 1 on libraries with mixed causes.
  - **Falsified if** the byte change exceeds the floor, or accuracy does not reach 1.

## 4. Decisions informed and how

| Decision | Contribution | Not settled here |
|---|---|---|
| DEC-CMP-039 | Suppression frequency by cause on F12/F17/F18/F19 and photo libraries (ledger evidence 1); archive-size delta for region-first, cohort-first and joint selection (evidence 2); audit schema cost and explain accuracy (evidence 3) | Timing guards (EXP-RECON-005 pattern; added by amendment if W ≠ status quo) |
| DEC-CMP-026 | Region reach loss attributable to conflict rules (part of the JPEG net-savings picture) | — |
| DEC-CON-010 | Audit-record bytes under split reasons (activation and audit cost) | — |

## 5. Candidates

| Cell (× profile ∈ {balanced, dense, extreme}) | Definition | Expr. / tier est. |
|---|---|---|
| `conflict-status-quo` | production v6 (`gen-v6`) | REAL / existing |
| `conflict-reference-no-regions` | `v6-minus-jpeg` (reference for region gains) | REAL / T1 |
| `conflict-region-first` | the JPEG region stage runs before the cohort stage. Region members are excluded from cohort clustering. A member Chunk shared with another ContentObject may be region-owned; the other object's entry then depends on the region, with its access declaration extended. | REAL if the production reader accepts such archives; else MODELLED (common §8.3), flagged "requires reader or validation-rule change", est. T2–T4 |
| `conflict-joint-cost` | for each eligible object with a conflict, compare region complete-cost savings against the displaced cohort (dictionary or lookback) and dedup savings, recomputed by the production cost functions; choose the lower total. Ties go to the status quo. | REAL (planner override) / T1 |
| `conflict-split-audit-reasons` | status-quo selections. Audit records use a new audit version separating `DedupSharedChunk`, `CohortDictionaryMember` and `ChunkGroupMember` from the frozen single conflict reason. | REAL (writer override) / T2 (new reason codes) |

- **Excluded before execution:** `duplicate-representation-for-shared-chunk`. Ledger exclusion: I1; unique Chunk frames (`ecf/container.rs:1856-1858`); `eam/validate.rs:419-481`. As an L0 exclusion it needs a written counterexample and reviewer sign-off (objective §2.0 rule 3). The counterexample is the archive state "Chunk digest d has both an independent frame and region membership", which the cited validation rejects. Sign-off is pending.
- **Reader acceptance probe** (binary, decides REAL versus MODELLED for `conflict-region-first`, fixed now). A generated two-object library (object B shares 2 Chunks with object A's region) is encoded by the research writer and opened by production `ecf::open`. `OK` with exact unpack means REAL; any refusal means MODELLED.

## 6. Corpus

- **Real tuning items.** Every tuning item in F12, F17, F18 and F19 that the EXP-RECON-002 census reports with JPEG file bytes > 0.
- **S-PHOTO-LIBRARY (tuning).** `gen_photo_library.py`, seed 20260930. Sources: SOF0 JPEGs ≥ 2 MiB from `f12-tuning-nasa-ivl-photos-medium` and `-large`, plus all SOF0 files of `f12-tuning-kodak-jpeg-matrix-medium`. Two libraries (`lib-small`: 50 sources; `lib-medium`: 300 sources), each containing per source a seeded mix of:
  - (a) 1–3 exact copies at other paths;
  - (b) metadata-edited copies (`exiftool` 12.x from Ubuntu: DateTimeOriginal, Orientation, Rating, XMP subject), which change APP1 length;
  - (c) appended-trailer copies (4–64 KiB, seeded bytes);
  - (d) lossless rotations (`jpegtran -rotate 90 -copy all`, libjpeg-turbo 2.1.5);
  - (e) re-encodes (decode → `cjpeg -quality 90`);
  - (f) burst near-duplicates (1–5 % pixel perturbation → `cjpeg` at the source's estimated quality);
  - (g) a ZIP wrapping 10 originals (stored).

  Mix proportions are fixed in the generator: 30 % a, 30 % b, 10 % c, 10 % d, 10 % e, 10 % f. SHA-256 per file goes in `items-tuning.jsonl`. The libraries inherit the independence group of their sources (corpus methodology §3).
- **Validation (look 1).** Real validation items as above. S-PHOTO-LIBRARY (validation) is generated with seed 20260931 from `f12-validation-commons-cc0-photos-*` and `f12-validation-fsa-jpeg-matrix-medium`.
- **Minimum support.**
  - F12 has 2 groups per split, and the generated libraries add strata, not groups. A corpus-level verdict across F12/F17/F18/F19 meets ≥ 10 groups only if those families' JPEG-bearing items span enough groups (checked from the census before the look).
  - A family-scoped F12 claim is 2-group. The MDE gate is expected to fail without S-JPEG-REAL, in which case the decision stays `INSUFFICIENT_EVIDENCE` with the gap recorded (common §7).
- **Held-out.** Phase D placeholder (common §5).

## 7. Environment and platform requirements

`wsl-ubuntu`, x86-64, no timing. Tools: `exiftool` and `jpegtran`/`cjpeg` from Ubuntu (installed; versions recorded by the generator); Pillow 12.3.0 in the research venv. Storage: generated libraries about 6 GB per split; results about 1 GB.

## 8. Commands and tooling to build

```sh
C:/Python313/python.exe research/orchestration/disk_guard.py
wsl.exe -d Ubuntu -- /root/eb-research/venv/bin/python /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-RECON-009/gen_photo_library.py \
    --split tuning --seed 20260930 --out /root/eb-research/generated/EXP-RECON-009/tuning \
    --manifest-out /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-RECON-009/items-tuning.jsonl
# reader acceptance probe (binary; decides REAL vs MODELLED for region-first):
#   /root/eb-research/tools/EXP-RECON-009/bin/ebr-recon variants --probe region-first-acceptance --out research/experiments/EXP-RECON-009/region-first-acceptance.json
# runner: ebr validate/run/verify-raw/normalize on research/experiments/EXP-RECON-009/spec.yaml
# oracle (to be written): ebr-recon audit-oracle --archive <a.eb> --trace-cache <dir>  (true cause per suppressed object from traces)
```

The runner cell command is `ebr-recon variants --variant conflict-*` (as EXP-RECON-004 §8.1), plus `--audit-oracle true` and the round-trip and access sample.

## 9. Seeds, warmups, replication

- Seeds: generator 20260930 (tuning) and 20260931 (validation); order, bootstrap and command 20260930; access sample seed 20260930.
- Warmups: 0.
- Repetitions: 2 (deterministic) with repeat-hash (§4.13).

## 10. Metrics

| Metric (canonical) | Role | OD | Threshold |
|---|---|---|---|
| `artifact_bytes` | primary | OD-05 | T-01 |
| `access_granularity_bytes` | primary guard | OD-10 | T-09 |
| `access_amplification` (strata) | secondary | OD-10 | T-22 |
| `side_data_bytes` (region and audit records) | diagnostic | OD-05 | T-02 |
| `roundtrip_mismatches`, `declared_cost_accuracy` | binary | — | §3.3 |
| suppression counts by cause, suppressed JPEG bytes | descriptive | — | — |
| `audit_reason_accuracy` (fraction of suppressed objects whose recorded reason equals the oracle cause at the finest granularity the audit version can express) | descriptive (non-canonical; common §8.2) | — | — |

## 11. Analysis

1. **Screens.** Round trip, repeat-hash and `declared_cost_accuracy` for every cell. For `conflict-region-first`, the declaration must cover cross-object region dependence (HC-09). A violation is R1.
2. **Suppression frequency (H1, H2).** Per item, library and profile: suppressed JPEG objects and bytes by oracle cause (dedup-shared, cohort dictionary, ChunkGroup), aggregated per §4.9 (descriptive).
3. **Size and access (tuning).** For each cell against `conflict-status-quo`: item log ratios of `artifact_bytes`, and `access_granularity_bytes` per affected entry class. §4.9 aggregation over real items and libraries, with library strata reported separately (never averaged into real-item families). Form W and S per profile (R3/R4 exploratory).
4. **Validation look 1.** MDE gate (§4.12) first. Then W versus S with the confidences of common §9; family and stratum guards; R6 with computed tiers (split reasons T2; region-first T2–T4 if a reader change is needed); R8–R10.
5. **Explain accuracy (H4).** `audit_reason_accuracy` per audit version on libraries containing all causes. Byte cost is the `side_data_bytes` difference.

## 12. Practical-significance thresholds

T-01, T-09, T-22 and T-02 by id; §3.3 binaries.

## 13. Sensitivity analysis

- common §10.
- Library mix proportions ±50 % per edit class, regenerated with seed + 1…+4 (the generator's `--mix-perturb k`). Reported to test whether H2 depends on the mix.
- Results with and without re-encode and burst classes (which drive cohort conflicts rather than dedup conflicts).

## 14. Expected negative results worth recording

- Conflicts are rare on real items (H1). Region-first and joint selection then have no measurable benefit outside synthetic libraries, and R10 keeps the status quo.
- Region-first worsens access for deduplicated entries beyond band (H3): the conservative status-quo rule is justified.
- Split audit reasons add wire items (T2) for accuracy that no primary metric values: a cost without a measurable gain.

## 15. Threats to validity

- **Synthetic photo libraries** encode assumptions about edit behaviour (mix proportions). Mitigated by mix perturbation and by reporting real-item results separately.
- **Region-first may need reader changes.** Then MODELLED (common §8.3), with the provisional-selection limitation.
- **Two F12 groups per split:** see §6 and the MDE gate.
- All other threats: common §13.

## 16. Held-out placeholder

Every frozen `conflict-*` cell runs once on held-out F12/F17/F18/F19 items after Commit C. Held-out libraries are generated after the freeze from held-out sources by a held-out-aware corpus session, seed committed only as its SHA-256 (sealed-tranche route, §5.4).

## 17. Cost and effort estimates

- **Compute.** About 40 CPU-hours: 15 cells × about 20 real items plus 2 libraries per split × 2 repetitions, dense and extreme dominating.
- **Disk.** About 12 GB of generated libraries (both splits); scratch ≤ 10 GiB.
- **Tooling.** Planner and writer overrides (shared with EXP-RECON-004), generator and oracle: about one harness session.
- **Agent effort.** Execution: none. Analysis: judgment.

## 18. Looks, deviations and outcome (append-only)

- Look 1 (DEC-CMP-039): to be registered. W, S, k_sup and MDE are appended here before the look.
- Deviations: none.
- Outcome: `outcome.json`.
