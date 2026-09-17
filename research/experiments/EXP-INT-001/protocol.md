# EXP-INT-001: Region-stratified corruption campaign: detection, fault class, localization and blast radius

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T1, T2, T3, T4, T5, T11 of `research/experiments/_index/integrity-conventions.md` C2) |
| Domain | integrity (program §24; objective OD-14) |
| Decisions informed | DEC-INT-002, DEC-INT-006, DEC-INT-007, DEC-INT-018, DEC-INT-019, DEC-INT-020, DEC-CMP-027, DEC-MOD-014 |
| Gates | C0 of the conventions: G-A before this becomes a §12.2 record; G-B before any decision-relevant result |
| Method | `research/decision-method.md` @ `14b977c79f157c58278a70c0f281f195430ca4f0` |
| Author and exposure | conventions C0 (L7 disclosure applies) |
| Spec | `spec.yaml` (tuning run of every configuration); the validation look spec is written after tuning names W and S (§12.2) |

## 1. Question

For one injected damage event of each pre-registered class (conventions C6) in each
region stratum (C4), how completely does each Entrybound archive configuration and each
candidate reporting design detect the damage, classify it, and localize it to the
affected entries, and how many logical bytes and entries become unverifiable per event,
compared with capability-matched incumbent formats?

## 2. Hypotheses and falsification

| ID | Decision | Directional hypothesis (effect in band units of the cited threshold) | H0 / equivalence | Falsified by |
|---|---|---|---|---|
| H1 | DEC-INT-002 | The measured unverifiable-entry set under M-SQ-READ equals the transitive dependency closure including dictionary and reconstruction users: definitions `transitive-closure` and `dictionary-inclusive` reach localization recall 1 and precision 1 on R-PAY events; `k-following` has recall below 1 on archives with lookback >= 2 (chained prefixes are transitive, consistent with REQ-ACC-0001 being CONTRADICTED); `whole-group` has precision below 1. | All definitions predict the measured set equally (precision difference within the T-20 floor). | Any event whose measured affected set is larger than the transitive closure (an HC-09 violation of the status quo, recorded as a defect), or `k-following` recall = 1 on every lookback >= 2 archive. |
| H2 | DEC-CMP-027 | On R-PAY, `blast_logical_bytes_p95` rises with lookback: dense and extreme profiles (lookback 1-8) exceed fast and balanced (lookback 0) by at least 1 band unit of T-15 on families with grouped cohorts (F01, F17, F18). | `EQUIVALENT` at corpus level. | Corpus-level `EQUIVALENT` or `DISTINGUISHABLE` in the opposite direction on tuning. |
| H3 | DEC-INT-007 | Fail-fast M-SQ-VERIFY reports every entry affected on every detected event, so its localization precision equals the affected share of entries; M-MAP reaches precision >= 0.99 on R-PAY and remains at recall 1 on all strata. Predicted difference exceeds the T-20 floor by more than 10 band units. | No distinguishable precision difference. | M-MAP precision not `DISTINGUISHABLE_BETTER` at corpus level, or any M-MAP recall < 1 (which also eliminates M-MAP at R1). |
| H4 | DEC-INT-006 | Every Entrybound configuration detects 100% of events in every class and stratum (I21) and its classes lie on the MVT-15(b) diagonal except adjudication cells A1-A3; incumbents without per-entry integrity data (tar headers inside compressed streams) show undetected or unlocalized damage, and 7z solid shows blast radius of a whole solid block. | Incumbent and Entrybound localization equivalent. | Any Entrybound undetected injection (then the H4 claim is false and the configuration is eliminated at R1 for the decisions it screens). |
| H5 | DEC-INT-018 | With the status quo, damage to the INDEX section header is fatal (`CORRUPT`) while INDEX payload damage is rebuilt; the `ignore-damaged-index-section` prototype turns header damage into `OK` with `EB_ECF_INDEX_INVALID_REBUILT`-class reporting and identical EAM, with zero events where ignoring changes the interpretation. | No outcome difference. | Any event where an ignore candidate yields a different EAM, entry set or identity than the pristine archive (R1 elimination of that candidate). |
| H6 | DEC-INT-019 | Under the status quo, every D-P1 and D-S7 event in section headers and footer is detected (canonical checks and the preamble binding), but reason-code specificity on those events is lower than on R-PAY; the footer-self-checksum and section-directory-digest models raise specificity without changing detection. | No specificity difference. | An undetected header or footer event under the status quo (defect), or no specificity difference. |
| H7 | DEC-INT-020, DEC-MOD-014 | Under the status quo, section, offset and chunk id are available in the diagnostic detail for fewer than half of detected R-PAY events at verify level (section-level code), and affected entries are never listed. | Detail availability equal across candidates. | Status quo already carries every field on >= 95% of detected events. |

## 3. Decisions and candidates

Candidates are the ledger `candidate_set`; conventions C0 governs additions and
proposed exclusions.

**Measured archive configurations (the injection subjects).**

| candidate (spec name) | Definition | Role |
|---|---|---|
| `eb-indexed-balanced` | B-PROD `ebound pack --layout indexed --profile balanced` | reference (REF-PROD), status quo default |
| `eb-indexed-fast`, `eb-indexed-dense`, `eb-indexed-extreme` | same, other profiles (status quo lookbacks: fast and balanced 0; dense {1,2,4}; extreme {1,2,4,8}) | DEC-CMP-027 lookback arm, DEC-INT-002 dependency structures |
| `eb-stream-balanced`, `eb-stream-extreme` | `--layout stream --stream-window auto` | STREAM localization |
| `inc-zip-deflate-6` | Info-ZIP `zip -6 -X -r` | REF-INC |
| `inc-tar-gz-6` | GNU tar `--format=pax` piped to `gzip -6 -n` | REF-INC |
| `inc-tar-zst-3` | GNU tar piped to `zstd -3` (checksum setting recorded from the pinned tool's help output, not assumed) | REF-INC |
| `inc-tar-xz-6` | GNU tar piped to `xz -6` (check type recorded from the produced stream header, not assumed) | REF-INC |
| `inc-7z-lzma2-solid` | `7z a -mx=5 -ms=on` | REF-INC |
| `inc-7z-lzma2-nonsolid` | `7z a -mx=5 -ms=off` | REF-INC |

Incumbent command lines are copied from `research/baselines/configs.json` at the
pre-registration commit; a config absent there is added to `configs.json` first.

**Reporting and structure candidates, evaluated on the same events by probe mode or model.**

| Decision | Candidates (ledger ids) | How each is evaluated |
|---|---|---|
| DEC-INT-002 | `status-quo-direct-prerequisite`, `k-following`, `transitive-closure`, `whole-group`, `byte-bounded`, `dictionary-inclusive` | Each definition yields a predicted affected set from the T1 map; compared with the measured M-SQ-READ set (recall must be 1; precision and `declared_tightness_ratio` of predicted to measured blast bytes graded). |
| DEC-INT-007 | `status-quo-fail-fast` (M-SQ-VERIFY), `verify-continue-collect` (M-CONT), `verify-damage-map` (M-MAP), `separate-damage-report-mode` (same computation as M-MAP; differs only in CLI surface, so its measured sets are M-MAP's and its difference is cost C6), `index-guided-localisation` (M-IDXGUIDE) | Probe modes C7. Continue-on-error resource amplification is EXP-INT-004. |
| DEC-INT-018 | `status-quo-header-fatal`, `ignore-damaged-index-section`, `ignore-if-footer-consistent`, `nonconforming-refuse-explicit`, `trust-index-locators` | Reader options of T11(d) on R-IDX and D-S7 Index events; `nonconforming-refuse-explicit` is a class and code mapping evaluated on the status quo trace. `trust-index-locators` is measured, not excluded: its R1 outcome must be shown by artifact. |
| DEC-INT-019 | `status-quo-payload-digests`, `footer-self-checksum`, `section-directory-digest`, `whole-body-digest`, `identity-roots-only` | Status quo measured. The digest candidates are wire changes: detection over their covered bytes is exact by construction, so they are evaluated by a model verifier inside T3 that adds the candidate digest check at its specified read position; their extra bytes are exact (`artifact_bytes` delta). `identity-roots-only` is evaluated by disabling section-digest checks in T11(d) and relying on LAI/PCR/AUX recomputation. |
| DEC-INT-020, DEC-MOD-014 | DEC-INT-020: `status-quo-code-and-message`, `structured-detail-normative`, `detail-plus-affected-entries`, `code-normative-detail-informative`, `redacted-detail-for-encrypted` (encrypted arm in EXP-INT-016); DEC-MOD-014: `free-form-detail-status-quo`, `structured-detail-enum`, `qualifier-flags-on-ok`, `separate-warning-channel`, `verification-state-on-reader-results` | Per detected event, T3 records which detail fields (section, byte offset, chunk id, expected digest, actual digest, affected entries) are available at the point of detection in the status quo diagnostic, and which the reader's state at that point could supply (research log T11(a)). Human value of details is not claimed (§4.16). |
| DEC-CMP-027 | `status-quo-profile-lookbacks` (via the profile configurations above), `lookback-zero-everywhere` (fast and balanced profiles are its instances for corruption purposes); other lookback candidates (`larger-prefix-window`, `zstd-ldm-long-window`, `leader-only-delta`, `group-restart-interval`, `bounded-solid-blocks`, `per-category-lookback`, `lookback-16-plus`, `unbounded-solid`) are measured here only if the crossfile domain's packing tooling can emit them; otherwise their corruption propagation is left to the crossfile experiments and recorded as a gap. `unbounded-solid` is excluded by the SPEC §2.3 L111 freeze cited in the ledger (R1; sign-off pending). |
| DEC-INT-006 | `status-quo-single-mutation-corpus`, `every-offset-region-generator`, `prevalence-weighted-models`, `sector-burst-models`, `adversarial-split`, `versioned-conformance-integration` | This experiment *is* the generator candidate family (C6 classes); the decision's comparison is which class set changes conclusions of DEC-INT-002/007/018/019, evaluated by the class-weight sensitivity arms of §10. The prevalence weights come from the primary-source survey file `research/experiments/EXP-INT-001/damage-prevalence.md`, written and committed **before** any run (§9.3 citation rules; if the sources do not support numeric weights, that arm is reported as not computable). Integration into the conformance corpus is the conformance domain's decision input. |

## 4. Corpus

- Tuning run: INT-CORE-T (conventions C3) under every configuration; INT-LARGE-T under
  `eb-indexed-balanced`, `eb-indexed-extreme`, `eb-stream-balanced`, `inc-zip-deflate-6`
  and `inc-7z-lzma2-solid` only (scale arm).
- Validation look (after W and S): INT-CORE-V, plus INT-LARGE-V for the same five
  configurations.
- Minimum support: INT-CORE sets meet §4.9 (C3).
- Families applicable: all 14 of each INT-CORE set; strata applicability per C4.
- No generated items beyond the corpus; all injections are generated by T2 from C8 seeds.

## 5. Environment and platform requirements

- `wsl-ubuntu` (x86_64, ext4 under `/root/eb-research/scratch`), `timing: false`.
  Deterministic metrics only; no quiet window needed; parallel workers allowed (one
  scratch directory per worker).
- Cross-host arm (informational, HC-11 corroboration): a seeded 5% subsample of D-P1 and
  all D-S7 events re-probed with B-PROD on `windows-host` NTFS; any outcome difference is
  an HC-11 finding routed to the platform domain.
- Requires: B-PROD, B-RES with T11, TOOL-INC; Linux root not required. No network.
- Not required: macOS, ARM64, Docker.

## 6. Procedure and commands

1. Preconditions: `python research/harness/lock_parity.py` and
   `bash research/harness/harness-cli-parity.sh` print `RESULT PASS`;
   `python research/orchestration/disk_guard.py` passes; `damage-prevalence.md` is
   committed.
2. Equivalence precondition for T3's overlay optimization (C2 T3) on every small-tier
   item of INT-CORE-T; result recorded in `research/raw/EXP-INT-001/overlay-equivalence.json`.
3. Tuning run:
   ```sh
   wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools &&
     P=/root/eb-research/venv/bin/python; S=research/experiments/EXP-INT-001/spec.yaml;
     $P -m ebr validate --spec $S && $P -m ebr run --spec $S && $P -m ebr verify-raw --spec $S --refingerprint &&
     $P research/tools/integrity/verify_detail.py --experiment EXP-INT-001 && $P -m ebr normalize --spec $S && $P -m ebr report --spec $S'
   ```
   `research/tools/integrity/verify_detail.py` (to be written, T3 companion) re-hashes
   every detail file against the raw rows and the detail manifest (C9).
4. Decision analysis on tuning (§14 item 2 tooling), naming W and S per decision.
5. Validation look: `spec-validation.yaml` generated from `spec.yaml` by replacing the
   corpus selector with INT-CORE-V and the candidate list with W and S; registered in
   `research/decisions/validation-looks.jsonl` before the run.

The per-sample command (see `spec.yaml`) is
`ebr-int-campaign`, a thin driver in the `ebr-integrity` crate that packs the item with
the candidate's configuration (Entrybound through B-PROD; incumbents through TOOL-INC),
maps it (T1 or T4), injects every C6 event of classes D-P1 and D-S1 to D-S8 (T2), probes
each event in every applicable mode (T3), writes detail rows (C9) and prints one summary
row.

## 7. Seeds, warmups, replication

- `seeds.order` 2026091701, `seeds.bootstrap` 2026091702, `seeds.command` 2026091703;
  event seeds per C8.
- Warmups 0 (deterministic metrics). Repetitions 2 per (configuration, item) in different
  rounds and processes; repeat-hash check on `artifact_sha256` and `detail_sha256` (§4.13).
  Pack outputs of Entrybound configurations are deterministic (HC-13 claim for
  unencrypted packs); a mismatch is an HC-13 violation artifact handled per §4.13. Reader
  outcome mismatches for identical bytes are HC-03 violation artifacts.
- No adaptive rule (deterministic). Event counts per C6 are fixed.

## 8. Metrics

| Metric (canonical) | ID | OD | Role here | Unit, direction | Source |
|---|---|---|---|---|---|
| `blast_logical_bytes_p95` | T-15 | OD-14 | **primary** for DEC-CMP-027 (OD-14) and for DEC-INT-007 comparisons of W against the reference on blast radius | B, min | `stdout_json` per mode; strata and worst-stratum values alongside |
| `blast_logical_bytes_max` | T-15 | OD-14 | secondary | B, min | same |
| `blast_entries_p95` | T-15 | OD-14 | secondary (second OD-14 primary would need reviewed justification) | count, min | same |
| `localization_precision` | T-20 | OD-14 | **primary** for DEC-INT-002 and DEC-INT-007 (OD-14) | fraction, max | same |
| `localization_recall` | HC-15 | OD-14 floor | binary (R1) | fraction = 1 | same |
| `reason_code_specificity_fraction` | T-20 | OD-04 | **primary** for DEC-INT-019 and DEC-INT-020 (OD-04) | fraction, max | fraction of detected events whose code identifies the violated structure (pre-registered code-to-structure table in T3 source, committed before the run) |
| `declared_tightness_ratio` | T-23 | OD-17 | secondary for DEC-INT-002 (predicted blast bytes of a definition / measured) | ratio, min toward 1; values below 1 are violations | analysis over detail rows |
| `artifact_bytes` | T-01 | OD-05 | secondary context (profile cost; DEC-INT-019 digest overhead is exact bytes) | B, min | `path_size` of the archive |
| `index_bytes`, `metadata_bytes` | T-02 | OD-05 | diagnostic | B | T1 |
| `undetected_injections` | C11 | HC-15 / I21 | binary (R1) | count = 0 | T3 |
| `mvt15b_offdiagonal_events` | C11 | HC-15 | binary (R1); adjudication cells A1-A3 reported separately | count = 0 | T3 |
| detail field availability (section, offset, chunk id, expected, actual, affected entries) | none (descriptive census) | OD-04 | descriptive | fraction per field | T3 |

Strata are reported for every T-15 metric; the worst-stratum guard applies at R4 and R8.

## 9. Normalization

- Per item and configuration: T-15 metrics per stratum as C5; headline over strata with
  equal weights 0.2 over applicable strata (C4).
- Paired log ratios candidate / `eb-indexed-balanced` per item with the T-15 absolute
  floor applied at item level (§3.1); incumbents are compared against the same
  reference as REF-INC comparisons (reported beside REF-PROD comparisons, §6.5 reference
  arm).
- T-20 fractions: arithmetic item-level differences (absolute-only metrics, §3.1).
- Aggregation L2-L4 and intervals per conventions C10.

## 10. Statistical analysis and sensitivity

- Analysis per C10. Confirmatory comparisons on validation only: W against each member of
  S, per primary metric, at corpus level. Families and tiers guarded; the worst-stratum
  guard applies to T-15 metrics.
- Binary metrics: any `undetected_injections`, `localization_recall` < 1 or off-diagonal
  event (outside A1-A3) for an Entrybound configuration is an R1 elimination of the
  corresponding reporting candidate and, for the status quo, a production defect
  (objective §2.0 rule 5), committed with the injected bytes as the violation artifact.
- Thresholds: T-15, T-20, T-23, T-01, T-02 by reference only (decision-method §3.2,
  Appendix A.2, `thresholds.json` `metric_thresholds`).
- Sensitivity: every §6 test (C10) plus:
  - **Damage-class weight arms (DEC-INT-006):** (a) D-P1 only (the T-15 definition);
    (b) uniform over D-P1 and D-S1 to D-S8; (c) prevalence-weighted from
    `damage-prevalence.md`; (d) sector and burst classes only (D-S2 to D-S6). A selection
    for DEC-INT-002, 007, 018 or 019 that changes across arms is reported as
    damage-model dependent and soft-capped at MEDIUM (§6.4 treatment by analogy is not
    assumed; the flag is recorded as a known limitation).
  - **Stratum weighting arm:** byte-proportional combination instead of equal 0.2.
  - **Tier arm:** INT-LARGE results reported separately and never pooled with INT-CORE.

## 11. Expected negative results worth recording

- STREAM configurations under the status quo: any R-PAY event makes every entry
  unverifiable (whole-body digest), so STREAM blast radius exceeds ZIP's per-entry CRC
  localization. This is a negative result against Entrybound for OD-14 in STREAM.
- INDEXED fail-fast verify localizes only to a section (smoke observation, non-decision-grade),
  so per-entry localization depends entirely on a new mode.
- Adjudication cell A1 (budget field corruption reported as `POLICY_REFUSED`) may resolve
  as an HC-15 defect of the status quo.
- Lookback > 0 profiles may show blast radius factors that make dense/extreme
  distinguishably worse on OD-14 even where they win on OD-05.
- tar.zst and tar.xz may localize payload damage no better than Entrybound fail-fast
  (whole stream), which bounds any "incumbents localize better" claim.

## 12. Threats to validity

- Oracle correctness depends on T1's map and closure computation: mitigated by the exact
  size reconciliation with `ebr-inspect-bytes`, by recomputing closures from two sources
  (recorded plans and the research frame walk), and later by the independent reader
  (conformance domain).
- Research prototypes (M-CONT, M-MAP, M-IDXGUIDE, digest models) are not production
  implementations; their results bound what a design can achieve, not what a future
  implementation will do. Production integration requires re-measurement.
- Uniform byte sampling under-represents small strata; strata and D-S7 exhaustive field
  edits address this. Real-world damage is correlated with sectors, pages and transfer
  chunk boundaries; D-S2 to D-S5 cover aligned classes, but true prevalence is only as
  good as `damage-prevalence.md`.
- Incumbent strata are not isomorphic to Entrybound strata (for example tar headers inside
  a compressed stream cannot be injected separately); comparisons are made at headline and
  per-class level, and non-applicable strata are listed.
- The overlay optimization could hide reader behaviour that depends on untouched bytes;
  the equivalence precondition bounds this on small items only.
- Deterministic metrics measured in WSL only; cross-host outcome identity is checked on a
  subsample (HC-11 is owned by the platform domain).

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12. The held-out spec copies the validation spec with `splits: [heldout]`.

## 14. Cost estimates

- Compute: INT-CORE-T 21 items x 12 configurations x 2 repetitions x about 2,750 events,
  about 0.1-0.3 CPU-s per event with the overlay optimization on items up to 400 MB:
  about 90 CPU-hours; INT-LARGE-T (2 items, 5 configurations, 2 repetitions) about 60
  CPU-hours; validation look about 110 CPU-hours for W and S. Wall time about 10-14 hours
  at 20 parallel workers per phase.
- Disk: scratch peak about 40 GB (workers x largest packed item x 3 copies); detail store
  about 6 GB compressed.
- Agent effort: tooling build (scripted by a mechanical-stage agent, reviewed for oracle
  correctness: judgment), execution none (scripted), analysis and adjudication of A1-A3
  judgment by a session not involved in design.

## 15. Deviations (append-only; UTC time, author, reason, results visible yes/no)

None.
