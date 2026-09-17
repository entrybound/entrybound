# EXP-CROSSFILE-012: Whole-object reconstruction region versus shared, cohort-dictionary or grouped Chunk conflicts

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: `ebr-crossfile region-conflict` (research orchestration of the v6 region stage relative to the cohort stage, via `entrybound::research::planner::{trace_jpeg_object, select_cohort_plan}`; `research/harness/ebr-crossfile-DESIGN.md`) is not built |
| Domain / program sections | crossfile / 8.3 (with 8.2 dedup, 8.8 reconstruction, 9 planner) |
| decision_ids | DEC-CMP-039 |
| Evidence type | deterministic bytes, exact suppression counts, audit-reason census (§4.6, §4.13); planning CPU in EXP-CROSSFILE-007 |
| Method | `research/decision-method.md`, pre-registered |
| Record | Design only; `record.md` (§12.2) instantiated once gate G-A holds. |
| Design session and disclosure | Phase C-design crossfile designer; L7 disclosure as in EXP-CROSSFILE-001. |

## 1. Question

When a Chunk is eligible for a v6 whole-ContentObject JPEG reconstruction region but is shared by
another ContentObject (exact dedup) or has been assigned to a similarity-cohort Dictionary or
ChunkGroup, which representation should win to give the smallest correct archive with one
representation per Chunk, how often does each conflict occur on photo libraries with duplicates
and edits, and should audit records distinguish dedup conflicts from cross-file planning conflicts?

## 2. Hypotheses

- **H1.** Cohort-driven suppression of regions is frequent on F12 JPEG sets that contain near
  duplicates (bursts, edits) and `joint-complete-cost` reduces `artifact_bytes` against the
  status quo by at least 1 band unit of T-01 on F12, while being non-inferior elsewhere.
- **H0.** Conflicts are rare (JPEG entropy-coded payloads seldom form cohorts or dedup across
  objects except exact duplicates), all rules are `EQUIVALENT`, and R10 keeps the status quo.
- **Falsification.** H1 fails if the suppression frequency is below 1% of eligible regions on every
  F12 tuning item or the confirmatory verdict is not `DISTINGUISHABLE_BETTER`.

## 3. Decision informed; proposed sets

| Decision | Proposed type | Proposed od_ids | Proposed hc_ids |
|---|---|---|---|
| DEC-CMP-039 | EMPIRICAL (+ cost C1 for split audit reasons) | OD-05, OD-06, OD-04 (audit specificity, secondary: no canonical metric fits exactly) | HC-01 (one representation per Chunk), HC-02, HC-11 (region decode is a library-version-defined step, T4), HC-15, HC-16 |

## 4. Candidates

| candidate_id | Definition | Expected tier |
|---|---|---|
| `sq-cohorts-first-conservative` | Status quo v6: cohort planning before regions; any shared, grouped or dictionary Chunk makes the region ineligible; one `RegionDedupConflict` audit reason | reference |
| `region-first` | Select regions before similarity cohorts; region members are excluded from cohorts | T1 (new planner ID) |
| `joint-complete-cost` | Compare region savings against the displaced cohort savings (and dedup constraints) and choose the lower complete cost | T1 |
| `split-audit-reasons` | Status-quo selection, new audit reason codes distinguishing dedup sharing, cohort Dictionary and ChunkGroup conflicts (new planner/audit version) | T2 (new reason codes, §7.2) |
| `region-first-split-audit`, `joint-split-audit` | Combinations | T2 |
| `critic-TBD` | R0 item 6 slot | |
| `duplicate-representation-for-shared-chunk` | **Excluded**: R1, HC-01 / I1 and the unique-Chunk-frame rule (`ecf/container.rs` unique frames, `eam/validate.rs`), per the ledger exclusion; reviewer sign-off pending | |

Reference: `sq-cohorts-first-conservative`. Region eligibility and encoding are production v6
(`jixel`/`jxl-oxide` pinned versions, mandatory exact round trip); only the stage order and
conflict rule vary. Every region candidate is admissible only under objective HC-11's gated
conditions (it is already T4 in production); this experiment does not change those conditions.

## 5. Corpus selectors

- Manifest `ed28b1b4…`, tuning only: every tuning item of F12 (`f12-tuning-kodak-jpeg-edge-cases-small`,
  `f12-tuning-kodak-jpeg-matrix-medium`, `f12-tuning-nasa-ivl-photos-small`,
  `f12-tuning-nasa-ivl-photos-medium`; large `f12-tuning-nasa-ivl-photos-large` for finalists),
  F17 (`f17-tuning-duptree-mixed`, `f17-tuning-duptree-nested`,
  `f17-tuning-real-ubuntu-kernel-headers-2ver`), F18 (small and medium tuning items), and
  `f13-tuning-kodak-image-codecs-small` and `f19-tuning-real-home-snapshot` (photos inside home
  snapshots). Non-JPEG families serve as the non-target guard (conflicts must be zero there and
  bytes unchanged).
- Validation look: every validation F12, F17, F18 item plus the two F13 image items; W and S only,
  after the MDE gate. F12 validation has one independence group per scale in places: single-group
  families are labelled and cannot alone support a scoped winner (§4.9).

## 6. Environment and platform requirements

`wsl-ubuntu`, x86-64 Linux, ext4; `timing: false`; no network; no privilege. Prerequisites as
EXP-CROSSFILE-001 §6 (validity control 1 for the status quo against `ebr-pack --profile dense` and
`extreme`).

## 7. Commands

`ebr validate|plan|run|verify-raw|normalize|report --spec research/experiments/EXP-CROSSFILE-012/spec.yaml`
as EXP-CROSSFILE-001 §7. Per-sample command:

```text
/root/eb-research/target/harness/release/ebr-crossfile region-conflict
  --item-path {item_path} --item-id {item_id} --experiment-id EXP-CROSSFILE-012 --env-name wsl-ubuntu
  --profile {profile} --rule {rule} --archive-out {scratch}/out.eb
  --memo-dir /root/eb-research/cache/crossfile/EXP-CROSSFILE-012/rep{rep} --verify-roundtrip true
```

## 8. Seeds, warmups, repetitions

Seeds `order` 20260917121, `bootstrap` 20260917122, `command` 20260917123. Warmups 0. Repetitions
2 plus repeat-hash on `archive_sha256` (region encoders are single-threaded by freeze, so a
mismatch is an R1 determinism violation).

## 9. Metrics

| Metric | OD | Role | ebr family | Threshold | Source |
|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | **primary** | size | T-01 | `payload.artifact_bytes` |
| `side_data_bytes` (ReconstructionRegion and plan records) | OD-05 | diagnostic | size | T-02 | `payload.side_data_bytes` |
| `encode_cpu_s` | OD-06 | primary, measured in EXP-CROSSFILE-007 | time_cpu | T-05 | EXP-CROSSFILE-007 |
| `regions_selected`, `regions_suppressed_dedup`, `regions_suppressed_cohort_dictionary`, `regions_suppressed_cohort_group` | - | descriptive (conflict frequency, required evidence) | other | none | `payload.*` |
| `audit_reason_specificity_fraction` (share of suppressed regions whose audit reason names the actual conflict class) | OD-04 | secondary (not the canonical `reason_code_specificity_fraction`, whose case set is conformance cases) | other | none | `payload.*` |
| `wire_items_added`, `normative_words_added` for split audit reasons | C1 | cost inputs (§7.1) | other | cost | cost scripts (§14 item 7) |
| `roundtrip_ok`, `archive_sha256` | HC-02, HC-13 | binary / repeat-hash | correctness | §3.3, §4.13 | `payload.*` |

## 10. Normalization and statistical analysis

As EXP-CROSSFILE-001 §10; applicable families for superiority are F12, F13 (image items), F17, F18
and F19 (home snapshots); every other family is a non-target guard where `artifact_bytes` must be
non-inferior and conflicts zero. R6 minimum gain applies to `split-audit-reasons` variants (T2,
k = 3). The explain-accuracy part (whether explanations say truthfully why a region was not used)
is checked mechanically: for every suppressed region, the recorded audit reason must be consistent
with the conflict class the tool observed; any inconsistency is an HC-15 honesty violation record.

## 11. Practical-significance thresholds

T-01, T-02, T-05 by reference; §7.3 minimum gains.

## 12. Sensitivity analysis

decision-method §6 in full at the selected scope, plus WM-MEDIA (Appendix B.2) as the expected
decisive mix and a leave-one-item-out arm within F12 (few groups).

## 13. Validation look and held-out placeholder

As EXP-CROSSFILE-001 §13 (`EXP-CROSSFILE-012-HO` at Commit A; single freeze).

## 14. Expected negative results worth recording

- Region/cohort conflicts almost never occur on real photo sets, so the ordering question is moot
  and the status quo stands; the audit-reason split is not worth a T2 cost.
- `region-first` loses bytes on F18 where cohort dictionaries on non-JPEG files are displaced.

## 15. Threats to validity

- F12 has few independence groups per split; single-group labels and the family guard limit claims.
- JPEG reconstruction depends on pinned library versions (T4, HC-11 conditions); results do not
  transfer to other reconstruction targets.
- Corpus and L7 threats as EXP-CROSSFILE-001.

## 16. Cost

About 80 CPU-hours (JPEG XL reconstruction round trips dominate on F12 medium and large), wall about
6 h sharded; disk: scratch about 3 GB, raw about 200 MB. Agent effort: none for execution; scripted
tooling; judgment for analysis.
