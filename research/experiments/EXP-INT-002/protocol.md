# EXP-INT-002: Truncation, appended-data and boundary-cut campaign: outcome precedence and intact-prefix reporting

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T1 STREAM and crypto-v1 walkers, T2, T3, T6, T11(a) research check trace) |
| Domain | integrity (program §24; objective OD-14, OD-04; HC-15) |
| Decisions informed | DEC-INT-040, DEC-INT-041, DEC-INT-006 (truncation classes) |
| Gates, method, author | conventions C0 (G-A, G-B; method `14b977c`; L7 disclosure) |
| Spec | `spec.yaml` (tuning); validation spec generated after W and S |

## 1. Question

At every structural boundary, inside every header, at seeded offsets, and with appended or
concatenated data, which outcome class and reason code does each Entrybound reader
(library INDEXED, STREAM from a pipe, crypto-v1 INDEXED, and the CLI exit status) report,
how consistent are the layouts, and how much intact-prefix information (entries known
intact, bytes present of total) can each candidate reporting mechanism recover after tail
truncation?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | H0 | Falsified by |
|---|---|---|---|---|
| H1 | DEC-INT-040 | The status quo per-reader rules put at least one cut class off the MVT-15(b) diagonal (record-level truncation reported `CORRUPT`; preamble truncation reusing `EB_ECF_TRUNCATED_FOOTER`; crypto readers diverging on footer codes, as the ledger records), and `reader_disagreements` between INDEXED, STREAM and crypto-v1 for equivalent cut positions is non-zero. `outer-container-authority` and `explicit-precedence-table` reach zero off-diagonal events and zero disagreements. | Status quo already on the diagonal with zero disagreements. | Zero off-diagonal events and zero disagreements under the status quo on tuning. |
| H2 | DEC-INT-041 | Under the status quo, verify after tail truncation reports no intact entries (`truncation_salvage_fraction` = 0 for verify); `stream-forward-scan` reaches a fraction near the share of entries whose records precede the cut on STREAM and near 0 on INDEXED (manifest follows data); `periodic-sync-records` at a 1 MiB interval reaches at least 0.9 on both layouts at an `artifact_bytes` cost inside the T-01 band. | No candidate distinguishable from the status quo on `truncation_salvage_fraction`. | Sync records not `DISTINGUISHABLE_BETTER` or their byte cost outside the T-01 band on corpus level. |
| H3 | DEC-INT-006 | Truncation events are detected in 100% of cuts on every layout (never `OK`). | none (binary) | Any cut reported `OK` without qualification (R1; defect). |

## 3. Decisions and candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-INT-040 | `status-quo-per-reader-rules` (measured), `outer-container-authority`, `explicit-precedence-table`, `truncated-first`, `distinct-preamble-truncation-code`, `appended-bytes-nonconforming`, `merge-truncated-into-corrupt` | The status quo is measured. The alternatives are classification rules; they are evaluated by replaying each rule over the recorded **check trace** of every cut (T11(a): the ordered list of checks reached, their byte ranges and results) to produce each rule's (class, code) vector. Proposed L0 exclusion of `merge-truncated-into-corrupt`: it contradicts HC-15's requirement that truncation and corruption stay distinguishable at every layer (objective HC-15 statement); counterexample = any D-T1 footer cut, which the rule must label `CORRUPT`. Kept as a negative control (its off-diagonal count is computed at no cost). |
| DEC-INT-041 | `status-quo-truncated-footer-only` (measured), `preamble-declared-length`, `periodic-sync-records`, `stream-forward-scan`, `salvage-only-reporting`, `external-receipt-length`, `duplicate-length-preamble-and-footer` | `stream-forward-scan` is a T6 prototype reader. `salvage-only-reporting` is the T6 intact-entry salvage run on the same cuts (shared with EXP-INT-012). The wire candidates (`preamble-declared-length`, `periodic-sync-records` at intervals of 1 MiB, 16 MiB and every 1,000 entries, `duplicate-length-preamble-and-footer`) and `external-receipt-length` are evaluated by an exact information model over the T1 map: for each cut, the entries whose records and closure lie entirely in the surviving prefix and are covered by the last surviving length or sync declaration; their byte cost is the exact encoded size of the declared records under `docs/format-v0.md` canonical record rules, labelled MODEL. `preamble-declared-length` is marked infeasible for STREAM writers (no back-patching, stream-layout-v1 writes without Seek); it is evaluated for INDEXED only and the STREAM gap is recorded. |
| DEC-INT-006 | truncation part of the damage-class decision | D-T1 to D-T4 results feed EXP-INT-001's class-weight arms. |

## 4. Corpus

- Tuning: INT-CORE-T (conventions C3) under `eb-indexed-balanced`, `eb-indexed-dense`
  (dictionaries and groups present), `eb-stream-balanced`, and `eb-crypto-indexed-balanced`
  (B-PROD `--recipient` with a generated X-Wing test recipient; recipients avoid a password
  KDF on every open). STREAM cuts are fed through a pipe (`cat prefix | ebound verify -`).
- Validation look: INT-CORE-V for W and S.
- Encrypted STREAM is unsupported by production and is not a configuration.

## 5. Environment and platform requirements

`wsl-ubuntu` x86_64, `timing: false`, parallel workers. Cross-host arm: all D-T1 cuts of
three INT-SMALL-T items re-run with B-PROD on `windows-host` (informational HC-11 check of
outcome identity). No network, no root, no Docker.

## 6. Procedure and commands

1. Preconditions as EXP-INT-001 step 1.
2. Tuning run through ebr (`spec.yaml`): `validate`, `run`, `verify-raw --refingerprint`,
   `research/tools/integrity/verify_detail.py --experiment EXP-INT-002`, `normalize`, `report`.
3. Rule replay and information model: `research/tools/integrity/replay_rules.py
   --experiment EXP-INT-002 --rules research/experiments/EXP-INT-002/rules/*.json` (to be
   written; one JSON file per DEC-INT-040 candidate, committed before the run) and
   `research/tools/integrity/truncation_info_model.py --experiment EXP-INT-002`.
4. Decision analysis, W and S, validation look (C10).

Per sample, `ebr-int-campaign` packs the item, maps it, generates every D-T1, D-T2, D-T3
and D-T4 event, probes each with M-SQ-VERIFY (library, all events; CLI exit status on all
D-T1 and a 10% seeded subsample of D-T2), M-SQ-READ, and the `stream-forward-scan` and
salvage prototypes, and records the check trace.

## 7. Seeds, warmups, replication

Seeds 2026091711 (order), 2026091712 (bootstrap), 2026091713 (command); C8 event seeds.
Warmups 0; 2 repetitions with repeat-hash of `detail_sha256` (C10). Crypto-v1 archives are
intentionally nondeterministic (HC-13(b)); for them the archive is packed once per
repetition and the repeat-hash check applies to the **per-cut outcome vector** over
structurally equivalent cut positions, not to archive bytes.

## 8. Metrics

| Metric | ID | OD | Role | Source |
|---|---|---|---|---|
| `truncation_salvage_fraction` | T-20 | OD-14 | **primary** for DEC-INT-041 | per cut: oracle-intact entries identified as intact / oracle-intact entries; item value = mean over cuts |
| `artifact_bytes` | T-01 | OD-05 | **primary** for DEC-INT-041 (cost of wire candidates; MODEL delta for sync records) | exact |
| `reason_code_specificity_fraction` | T-20 | OD-04 | **primary** for DEC-INT-040 | fraction of cuts whose code identifies the cut structure |
| `reader_disagreements` | HC-03 | OD-04 floor | binary for DEC-INT-040 (INDEXED vs STREAM vs crypto-v1 at equivalent cut classes) | count = 0 |
| `mvt15b_offdiagonal_events` | C11 | HC-15 | binary; adjudication cell A2 (appended data) reported separately | count = 0 |
| `undetected_injections` | C11 | HC-15 / I21 | binary | count = 0 |
| `normative_words_added`, `wire_items_added` | C1 | OD-23 | cost inputs per candidate (checked-in counter, §7.1) | count |

## 9. Normalization

T-20 fractions: item-level arithmetic differences against the status quo candidate.
`artifact_bytes`: paired log ratio against the status quo archive (T-01 with tier floors).
Aggregation L2-L4 per C10; strata: layout is an evaluation condition (never averaged
across layouts, objective AGG-S).

## 10. Statistical analysis and sensitivity

Per C10. Layouts (INDEXED, STREAM, crypto-v1) are evaluation conditions: R4 and R8 guards
apply per layout; a candidate infeasible for a layout (for example `preamble-declared-length`
in STREAM) is inapplicable there and cannot be a universal winner (R9 partition by layout
policy, subject to expressibility). Sensitivity: §6 tests plus (a) sync-record interval arm
(1 MiB, 16 MiB, 1,000 entries); (b) cut-position weighting arm: uniform over bytes (D-T2
only) versus boundary-heavy (D-T1 only); (c) tier arm with INT-CORE only (no large items in
this experiment, recorded as a limitation).

## 11. Expected negative results worth recording

- Status quo truncation codes collapse several structures into one code (specificity below 1).
- INDEXED cannot report intact entries after tail truncation without a wire change, because
  manifests follow chunk data; STREAM does better only with a forward scan.
- Sync records may cost more than the T-01 band on small-tier archives (many tiny items).
- Adjudication A2 may show concatenated archives read as `OK` (first archive) by some reader.

## 12. Threats to validity

- The wire candidates are evaluated by an information model, not an implementation; the
  model is exact for "which entries are provably intact given declared records" but does not
  capture implementation defects. Labelled MODEL; evidence state at most
  `SUPPORTED_WITH_LIMITATIONS` until implemented.
- Rule replay depends on the completeness of the T11(a) check trace; a check the hook does not
  log would bias alternatives. Mitigation: the trace is validated by reproducing the status quo
  classes exactly from the trace before any alternative is replayed (acceptance: 100% agreement).
- Pipe reads for STREAM depend on buffering; cut positions are byte-exact, but reported
  offsets may be buffered.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12.

## 14. Cost estimates

- Compute: about 2,000-12,000 cuts per archive, about 0.02-0.5 CPU-s each on INT-CORE items:
  about 50 CPU-hours for tuning (4 configurations x 21 items x 2 repetitions), similar for the
  validation look on W and S; wall about 4 hours at 20 workers.
- Disk: scratch about 20 GB; detail about 2 GB.
- Agent effort: tooling build scripted with review of the check-trace hook (judgment);
  execution none; analysis and A2 adjudication judgment.

## 15. Deviations (append-only)

None.
