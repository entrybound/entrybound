# EXP-LEGACY-070: legacy CLI surface, receipts, refusal messages and capability-matrix drift (mechanical census)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (candidate CLI grammar files; `flags_census.py`; `alias_drift.py`; `receipt_census.py`; `refusal_paths.py` static extractor; `message_census.py` with a pre-registered rubric; research-only all-findings dry-run and partial-import prototypes behind `research-internals`) |
| Domain / program sections | legacy / §27 and §25 surfaces (with §33 usability — mechanical parts only — and §29 conformance coverage) |
| decision_ids | DEC-LEG-013, DEC-LEG-017, DEC-LEG-026, DEC-LEG-100, DEC-LEG-006, DEC-LEG-099, DEC-LEG-010, DEC-CRY-061 |
| Decision types (proposed) | HUMAN-FACING decisions evaluated only through mechanically checkable properties (§4.16: flag counts, unsafe defaults, message censuses); every claim about human comprehension, task success or preference is routed to EXTERNAL_REVIEW_REQUIRED |
| OD / HC (proposed) | OD-25 (M25.2, M25.3, M25.4), OD-04, OD-21 (conformance coverage inputs); HC-05, HC-15, HC-07 |
| Runner | Not runner-driven (no `spec.yaml`): deterministic scripts over CLI help text, grammar files, repository code and committed observation artifacts of other experiments |
| Shared rules / L7 / gates | C§ header, C§2 |
| Timing | none |

## 1. Question

For the legacy import, export, publish, preservation-recovery and audit tasks, how many flags and concepts does each
candidate CLI surface require, what breaks under each alias policy when v2 profiles appear, where do receipts go in every
export mode, do refusal messages name the failed requirement and a remedy, does documentation match the refusal paths
in code, and how complete are all-findings audit and partial-import prototypes?

## 2. Hypotheses

- **H1 (flags, DEC-LEG-013/CRY-061).** Splitting import and export verbs does not increase `flags_per_task` for any of the
  ten tasks; requiring an explicit flag for plaintext export of encrypted sources adds exactly one flag to one task.
- **H2 (alias drift).** Under `aliases-follow-latest`, at least one automation snippet silently changes output bytes when a
  v2 profile is introduced; under `aliases-pinned-to-v1-forever`, none does.
- **H3 (messages).** `error_actionability_fraction` over distinct legacy refusal messages is below 0.8 `[INFERRED]`.
  *Falsified by* ≥ 0.8.
- **H4 (drift, DEC-LEG-026).** At least one reason code emitted by a legacy adapter is not mentioned in the corresponding
  `docs/*-v1.md`, or a documented refusal has no code path.
- **H5 (audit completeness, DEC-LEG-100).** The status-quo dry-run reports one finding per refused archive while the
  all-findings prototype recovers at least 90% of census-labelled constructs on EXP-LEGACY-001 cases.
- **H0.** Surfaces are equivalent in counts, messages are actionable, docs match code, and first refusal equals all
  findings.

## 3. Decisions informed

| Decision | Supplied here (mechanical only) |
|---|---|
| DEC-LEG-013 | `flags_per_task` per candidate grammar; alias-drift breakage counts |
| DEC-LEG-017 | Receipt presence/location per export mode (file, stdout, dry-run, refused, publish, sidecar), from CLI behaviour and EXP-LEGACY-032 crash observations |
| DEC-LEG-026 | Code-versus-docs refusal drift; reason-code fixture coverage from EXP-LEGACY-001..004; a generated capability matrix prototype |
| DEC-LEG-100 | All-findings prototype completeness and report size versus first-refusal dry-run |
| DEC-LEG-006 | Annotation false-positive rate (computed in EXP-LEGACY-010) and annotation output size |
| DEC-LEG-099 | Share of artifacts importable under an opt-in partial prototype and the typed records it would emit |
| DEC-LEG-010 | Publish surface flag counts and report fields |
| DEC-CRY-061 | Flag census for plaintext export of encrypted sources; whether the status quo default is an unsafe default under M25.4's definition (pre-registered: an export from an authenticated encrypted source to an unencrypted target that proceeds without any explicit caller flag counts as one unsafe default) |

## 4. Candidates

| Decision | Candidates (ledger ids) as grammar files `research/experiments/EXP-LEGACY-070/grammars/<id>.yaml` or prototypes |
|---|---|
| DEC-LEG-013 | status-quo-bidirectional; split-import-export-verbs; aliases-pinned-to-v1-forever; aliases-follow-latest; remove-aliases; spec-flag-names |
| DEC-LEG-017 | status-quo-optional-file; always-sidecar-file; receipt-to-stderr-or-fd; refusal-receipts; embed-in-native-sidecar |
| DEC-LEG-026 | status-quo-per-format-docs; handwritten-matrix; generated-matrix; commons-compress-style |
| DEC-LEG-100 | status-quo-dry-run-first-refusal; all-findings-report; dedicated-inspect-legacy; drop-verify-canonical-legacy; define-canonical-legacy; legacy-legacy-diff |
| DEC-LEG-006 | status-quo-none; matrix-annotations; inspect-explain-mode; lint-command; out-of-scope |
| DEC-LEG-099 | status-quo-whole-archive; opt-in-partial-lossy; partial-in-compat-only; placeholder-entries |
| DEC-LEG-010 | status-quo-freeze-publish-v1; freeze-core-defer-reports; defer-publish-post-v1; ci-integration-first |
| DEC-CRY-061 | status-quo-warning-and-receipt; explicit-flag-required; interactive-confirmation; declared-loss-outcome; silent-export (proposed L0 exclusion, SPEC §15.2 L1697) |

## 5. Inputs

CLI help text of `ebound-head` (all subcommands); `docs/{zip-import-v1, tar-import-v1, 7z-import-v1,
compressed-stream-import-v1, legacy-export-v1, zip-export-v1, tar-export-v1, compressed-tar-export-v1,
legacy-preservation-v1, migration-workflows-v1, zip-compatibility-profiles-v1}.md`; `crates/entrybound/src/legacy/*.rs`
and `crates/entrybound-cli/src/lib.rs` at the record commit; committed observation artifacts of EXP-LEGACY-001..004,
006, 010 (tuning and validation), 032 (crash arm) and 041; an automation-snippet set generated from the CLI grammar
(every documented invocation form × target × profile flag) by `alias_drift.py --seed 2509177001`. No held-out input
(the held-out task set for §4.16 mechanical parts is a placeholder: task variants written before design iteration by a
non-exposed session and sealed per C§5).

## 6. Environment

`wsl-ubuntu`; no network; no timing.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python research/tools/legacy/flags_census.py --binary /root/eb-research/target/legacy-head/release/ebound --grammars research/experiments/EXP-LEGACY-070/grammars --tasks research/experiments/EXP-LEGACY-070/tasks.yaml --out research/normalized/EXP-LEGACY-070/flags.csv
/root/eb-research/venv/bin/python research/tools/legacy/alias_drift.py --grammars research/experiments/EXP-LEGACY-070/grammars --seed 2509177001 --out research/normalized/EXP-LEGACY-070/alias-drift.csv
/root/eb-research/venv/bin/python research/tools/legacy/receipt_census.py --binary /root/eb-research/target/legacy-head/release/ebound --crash-observations research/raw/EXP-LEGACY-032/crash --out research/normalized/EXP-LEGACY-070/receipts.csv
/root/eb-research/venv/bin/python research/tools/legacy/refusal_paths.py --code crates/entrybound/src/legacy --docs docs --fixtures EXP-LEGACY-001,EXP-LEGACY-002,EXP-LEGACY-003,EXP-LEGACY-004 --out research/normalized/EXP-LEGACY-070/drift/
/root/eb-research/venv/bin/python research/tools/legacy/message_census.py --observations EXP-LEGACY-001,EXP-LEGACY-002,EXP-LEGACY-003,EXP-LEGACY-004,EXP-LEGACY-006,EXP-LEGACY-010,EXP-LEGACY-041 --rubric research/experiments/EXP-LEGACY-070/message-rubric.yaml --out research/normalized/EXP-LEGACY-070/messages.csv
/root/eb-research/venv/bin/python research/tools/legacy/audit_prototype.py --build /root/eb-research/target/legacy-research/release/ebound --cases EXP-LEGACY-001 --labels census --out research/normalized/EXP-LEGACY-070/audit/
/root/eb-research/venv/bin/python research/tools/legacy/partial_import.py --build /root/eb-research/target/legacy-research/release/ebound --from EXP-LEGACY-010 --split tuning --out research/normalized/EXP-LEGACY-070/partial/
```

**Pre-registered message rubric (`message-rubric.yaml`).** A refusal message is actionable iff (a) it contains a stable
reason code and the offending entry path, byte offset or limit name, and (b) it names at least one remedy token from the
registered list (a flag such as `--compat=`, `--allow-lossy`, `--from`, `--entry-name`, `--target-profile`; a profile id;
or an explicit "not supported in v1" statement naming the unsupported feature). The rubric is applied by pattern match
only.

## 8. Seeds

Snippet generation `2509177001`; no other randomness.

## 9. Replication

Every script runs twice; outputs must be byte-identical (§12.3 item 4).

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `flags_per_task` | cost_input (C6) | OD-25 | §7 |
| `error_actionability_fraction` | T-20 | OD-25 | T-20 |
| `unsafe_defaults` | binary | OD-25 | §3.3 |
| `declared_gap_count` (docs-code drift items) | descriptive | OD-04 | none |
| receipt presence table; alias-drift breakage counts; reason-code fixture coverage; audit recall and report bytes; partial-import share | non-canonical descriptive | — | none |

## 11-12. Normalization and analysis

Exact counts per task and candidate grammar (AGG-P per design, mean and maximum over tasks per objective OD-25);
message census per adapter and overall (AGG-S worst adapter); binary `unsafe_defaults` per candidate. No simulated
sessions are run here (the §4.16 heuristic sessions belong to the evaluation domain, labelled SIMULATED, never primary
support).

## 13. Practical significance

T-20 for `error_actionability_fraction`; binary §3.3; cost inputs feed §7 tiers.

## 14. Sensitivity analysis

Rubric arm (requirement-only and remedy-only variants reported beside the conjunction); task-set arm (drop each task);
snippet-set arm (documented forms only versus full grammar expansion).

## 15. Expected negative results worth recording

Messages already actionable; no docs-code drift; the all-findings prototype no more complete than first refusal on real
archives (single-construct refusals dominate).

## 16. Threats to validity

Grammar files for non-implemented candidates are paper designs (counts can be optimistic); pattern-matched actionability
is a proxy that never supports claims about human understanding (routed to EXTERNAL review).

## 17. Platform requirements

Linux x86-64; no network; no privileged operations; human participants: none (claims needing them are EXTERNAL).

## 18. Estimates

Compute ≈ 2 CPU-hours; disk ≈ 1 GB. Agent effort: grammar files and rubric are judgment (fixed before any census run);
scripts scripted.

## 19. Expected cost tiers `[INFERRED]`

Verb split or flag renames: T2 (user-visible surface); explicit plaintext-export flag: T2; refusal receipts: T2 (receipt
version); generated capability matrix: T0-T1 (documentation tooling); all-findings audit mode: T2; partial import: T2-T3
(new typed LOSSY records).
