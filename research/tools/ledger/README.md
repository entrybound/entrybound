# Ledger tools

Phase A tooling that turns extraction records into the research program's requirement and decision
ledgers. All tools use the Python 3 standard library only.

| Tool | Purpose |
|---|---|
| `split_by_cluster.py` | Split `research/audit/extract/*.jsonl` into per-cluster inputs (`research/audit/by-cluster/`). |
| `make_merge_shards.py` | Build compact topic shards for the merge agents (derived, not committed). |
| `assemble.py` | Assemble the ledgers from merged rows and addenda (this document). |
| `near-duplicates.json` | Reviewed adjudications of near-duplicate candidates, read by `assemble.py`. |
| `requirement-ledger.schema.json`, `decision-ledger.schema.json` | JSON Schemas written and enforced by `assemble.py`. |

## assemble.py

```
C:/Python313/python.exe research/tools/ledger/assemble.py [--round N] [--date YYYY-MM-DD]
    [--write-coverage-addenda] [--skip-inventory] [--external-root DIR] [--sources-root DIR]
```

The tool is deterministic and rerunnable: with unchanged inputs, a rerun produces byte-identical outputs.
It prints a JSON summary (row counts, uncovered extraction keys, validation errors, counts by state) and
exits 0 when validation passes, 1 when validation errors remain. Outputs are written either way so the
errors can be inspected. `--date` sets the date of new ledger history entries (default: today).
`--external-root` (default `D:/Projects/entrybound`) holds `design/` and `research/`, the unpublished
inputs. `--sources-root` (default `D:/eb-research/sources`) holds the extracted `appendix/` and
`r3-instrumentation/` members. `--skip-inventory` leaves `source-inventory.md` untouched, for hosts without
those inputs.

### Inputs

- `research/audit/extract/*.jsonl`: extraction records, one per `local_key`.
- `research/audit/merged/<job>-requirements.jsonl` and `<job>-decisions.jsonl`: one pair per merge job.
  Several jobs can share a cluster, for example `crypto-keys-recipients`, `crypto-leakage-suite` and
  `crypto-signatures-trust`. Row formats follow the merge contract in
  `research/orchestration/workflows/phase-a3-ledgers-method.js`.
- `research/audit/addenda/*.jsonl`: the same row formats, applied in file-name order and then line order.
- `research/tools/ledger/near-duplicates.json`: near-duplicate adjudications.
- State carried between runs: `research/audit/id-map.json` and the existing `research/decision-ledger.jsonl`.

### Outputs

| Output | Content |
|---|---|
| `research/requirement-ledger.csv` | RFC 4180, UTF-8, CRLF, header row, sorted by `req_id`. `additional_sources` is a JSON array. `decision_ids` and `program_sections` are `; `-separated. |
| `research/decision-ledger.jsonl` | One JSON object per line, sorted by `decision_id`, with the field order of the program contract. |
| `research/supersession-ledger.md` | Per cluster: req_id, original authority@location, superseding authority@location, nature of change, governing authority. |
| `research/source-inventory.md` | Every source file with path, SHA-256, bytes, lines, role, authority rank, extraction slice keys and citing-row count. |
| `research/audit/id-map.json` | Stable IDs, aliases, retired IDs and per-code high-water marks. |
| `research/audit/assembly-log.md` | Input hashes, enumeration fixes, collisions, merges (with merged-away text), addenda applied, reference resolutions, validation errors and warnings. |
| `research/audit/coverage-report.md` | Mechanical extraction-key coverage per slice, uncovered keys, and keys shared by several rows. |
| `research/tools/ledger/*.schema.json` | JSON Schemas generated from the enumerations in `assemble.py`. |

The repository's `.gitattributes` (`* text=auto eol=lf`) normalizes the CSV to LF inside git. CSV readers
must therefore accept both CRLF and LF record separators. The tool always writes CRLF.

### Pipeline

1. **Load and fix enumerations.** Enumerated values (`kind`, `implementation_state`, `evidence_state`,
   `release_relevance`, `initial_status`/`status`, `blocker_class`) and `program_sections` are normalized.
   Case, spaces and hyphens are canonicalized, and a small synonym table maps values such as
   `partially implemented` to `PARTIAL`. Every change is logged. Values with no minimal fix are validation
   errors. Inputs are never rewritten.
2. **Key collisions within a cluster.** Rows of one record type that share `<cluster>/<key>` are compared
   by TF-IDF cosine of their text. At or above `review_threshold`, they merge into the row from the first
   job in sorted order. Below it, or when listed in `distinct_collisions`, the later row is renamed to
   `<key>--<job suffix>`, and references from the same merge job follow the rename.
3. **Near-duplicates across merge jobs of one cluster.** Candidates are pairs from different jobs whose
   TF-IDF cosine (question text plus key tokens) reaches `review_threshold` (0.30). Merges listed in
   `near-duplicates.json` fold the listed rows into the named survivor. Pairs listed as `distinct` stay
   separate. Unadjudicated pairs at or above `auto_merge_threshold` (0.70) merge automatically. The
   survivor is the row with an existing ID, then the row with more source references, then job and key
   order. Other unadjudicated pairs stay separate and are reported as warnings for review. In round 0,
   all 187 candidates were reviewed: 29 requirement and 9 decision rows merged, and 148 pairs stayed
   distinct.
4. **Addenda.** A row whose `cluster` plus `canonical_key` or `decision_key` names a live row updates it.
   So does a row that addresses it through `req_id`/`decision_id`, or through a key that is an alias of
   this run or of `id-map.json`, in which case the update is redirected to the survivor.
   `additional_sources` are unioned by `local_key`. Every other non-empty field replaces the old value,
   and lists are replaced whole. `correction_reason` is logged. `status` is accepted as a synonym of
   `initial_status`, and `blocker_class` (or `_blocker_override`) overrides the derived class. A row with an
   unknown key is a new row and must carry the required fields. A row with `correction_reason` is logged
   as a correction. When a replaced list loses items (reference lists are compared after resolution,
   candidates by `candidate_id`), the dropped items are listed under the addendum in the assembly log.
5. **Near-duplicates involving addenda rows.** After addenda are applied, step 3 runs again over the
   post-addenda table, restricted to pairs that include a row added by an addendum (a different job
   from every merge job and from other addenda files). The same adjudication file and thresholds apply.
   Adjudicated merges that name a row only an addendum adds are deferred to this pass. In round 1, the 8
   candidates (all requirement pairs, cosine 0.309-0.409) were reviewed and kept distinct.
6. **Cross-cluster dedupe.** Rows in different clusters that claim the same primary `local_key` merge. The
   survivor is the row in the extraction record's `primary_cluster`. Keys shared as supporting references
   are not merged and are listed in the coverage report.
7. **Reference resolution.** `decision_keys` and `requirement_keys` may be bare keys (same cluster first,
   then a unique match in another cluster), `cluster/key`, or stable IDs. Aliases are followed. Links are
   symmetric: a link declared on either side appears in both `decision_ids` and `requirement_ids`.
   Unresolved or ambiguous references are validation errors.
8. **Integrity and coverage.** Every decision must reference at least one requirement. Every requirement
   with `decision_needed` must reference at least one decision. Every program section in 8.1-8.8, 9-21,
   22.1-22.5 and 23-35 must appear in at least one decision. Every `additional_sources.local_key` must
   exist. Every extraction key must be referenced by a requirement row, either as its primary key or in
   `additional_sources`.
9. **Stable IDs, outputs, schema validation.** The CSV is read back and every row is validated against
   the requirement schema. Every decision row is validated against the decision schema.

**Primary key** of a requirement row: the extraction key in the last bracket of `source_location`, else
`source_local_key`, else the `additional_sources` entry whose location equals `source_location` (the
access and integrity merges record their primary key this way). PROGRAM rows have none.
`source_local_key` is a merge-input field, not a ledger column. When it is the only place the key
appears, the CSV `source_location` gets ` [<key>]` appended, and the change is logged under "Primary keys
made visible". After the CSV is written, the assembler reads it back and fails validation if any covered
extraction key is not visible in the CSV (a `source_location` bracket or an `additional_sources` entry).
Round 1 made 70 integrity-cluster primary keys visible this way.

**Field merge rules.**
- Requirements:
  - `additional_sources`: union, including the merged row's primary source.
  - `evidence_state`: the stricter state, in the order CONTRADICTED, EXTERNAL_REVIEW_REQUIRED, UNTESTED,
    INSUFFICIENT, SUPPORTED_WITH_LIMITATIONS, EMPIRICALLY_SUPPORTED, PROVEN.
  - `implementation_state`: the less complete of MISSING, PARTIAL and IMPLEMENTED. Other conflicts keep
    the survivor's value and are logged.
  - `release_relevance`: the more severe value.
  - `program_sections`, `decision_keys`: union.
  - Text fields (`supersession_note`, `implementation_evidence`, `research_needed`, `decision_needed`):
    distinct parts joined with ` | `. `superseding_authority` parts are joined with `; `.
  - `kind` and `requirement_or_question`: the survivor's value. The merged-away text is kept verbatim in
    the assembly log.
- Decisions:
  - Candidates are unioned by `candidate_id`. Constraints, evidence, sections and requirement links are
    unioned.
  - `initial_status`: the stricter status, in the order EXTERNAL_REVIEW_REQUIRED, INSUFFICIENT_EVIDENCE,
    DECIDED.

### Fixing integrity and coverage gaps

- **Uncovered extraction keys.** Run with `--write-coverage-addenda` to draft minimal rows from the
  extraction records into `research/audit/addenda/assembler-coverage-round<N>.jsonl`. Review the drafts,
  fold them into canonical rows where appropriate, and rerun until `uncovered_extraction_keys` is 0.
- **Referential integrity.** Gaps need judgment. Write minimal addenda to
  `research/audit/addenda/assembler-round<N>.jsonl`: a `decision_keys` link to the owning decision, or a
  new decision when none covers the question.
- **Round 0 fixes.** `assembler-round0.jsonl` holds two kinds of fix. It links 19 requirements to decisions
  that their `decision_needed` text named or that own their question, and it adds the decision
  `crypto/stream-layout-signing-support`.
- **Round 1.** The critic addenda `round1-spec-research.jsonl`, `round1-appendix-docs.jsonl` and
  `round1-program-code.jsonl` added 17 requirements and 12 decisions and corrected 46 rows. The
  round 1 assembly found 0 uncovered extraction keys and 0 integrity errors, so it wrote neither
  `assembler-coverage-round1.jsonl` nor `assembler-round1.jsonl`. Three new crypto rows cite
  `docs/crypto-suite-v1.md` L721-852, which has no extraction record, so they have no primary
  `local_key`. They are reported as warnings.

### Stable IDs (`research/audit/id-map.json`)

- **Format.** `requirements` maps `<cluster>/<canonical_key>` to `REQ-<CODE>-<NNNN>`. `decisions` maps
  `<cluster>/<decision_key>` to `DEC-<CODE>-<NNN>`.
- **Cluster codes.** model=MOD, container=CON, compression=CMP, access=ACC, platform=PLT, crypto=CRY,
  legacy=LEG, integrity=INT, ecosystem=ECO.
- **Numbering.** A new key gets the next number of its code, in sorted key order. `next` holds the
  high-water marks. Registry entries are never deleted, renumbered or reused.
- **Aliases.** `requirement_aliases` and `decision_aliases` map merged-away keys to their survivor and the
  survivor's ID.
- **Retired IDs.** When a key that already had an ID is merged away, `retired_ids` maps its ID to the
  survivor's ID. If the merge is later undone, the key gets its own ID back.

### Decision ledger preservation

Rows are regenerated from the inputs, but existing rows with the same `decision_id` keep their evidence
fields: experiment and result references, counterevidence, sensitivity analysis, costs,
`selected_decision`, scope, confidence, limitations, `reopen_trigger`, `remaining_unknowns_ref` and
`history`.

- **Later-phase rows.** If any `history` entry was written by a later phase (the change text does not
  start with `created by Phase A audit` or `assembler:`), `status` and `blocker_class` are preserved.
- **Audit-owned rows.** Otherwise the status is re-derived from the inputs. A change appends one
  `assembler:` history entry. The entry names its cause: the addendum that set the status or blocker
  class, or the addenda that changed `implementation_state` on a linked requirement, or otherwise
  "reassembled inputs".
- **Retired IDs.** A ledger row whose ID was retired by a merge has its evidence absorbed into the
  survivor, with a history entry.
- **Unknown IDs.** A ledger row with an unknown ID is kept unchanged and reported as an error.

New rows start with `history: [{"date": <--date>, "change": "created by Phase A audit"}]`, and their
evidence fields are empty.

`blocker_class` is derived as follows, unless an addendum overrides it:

| Condition | blocker_class |
|---|---|
| Status is DECIDED | NONE |
| Status is EXTERNAL_REVIEW_REQUIRED | EXTERNAL_REVIEW |
| The question or required evidence needs people (usability studies or tests, interviews, demand, consumer or integrator surveys, adopter tests) | HUMAN_PARTICIPANTS |
| A linked requirement is BLOCKED_PLATFORM | PLATFORM |
| Otherwise | EVIDENCE |

The derivation never assigns RESOURCES; set it through an addendum.

### Source inventory and supersession ledger

- **Unpublished inputs.** The SPEC, Research I-III, the research appendix and the Research III
  instrumentation are external unpublished inputs. They are listed by SHA-256 and never copied into the
  repository. The recorded hashes are checked against the values in `research/PROGRESS.md`.
- **Repository files.** Files are read from the baseline commit `9e44608` with read-only `git ls-tree` and
  `git cat-file`.
- **Authority ranks.**
  1. Repository docs and code at `9e44608`. They supersede the SPEC where they freeze or refine it.
  2. The SPEC.
  3. Research I-III, the appendix and the instrumentation.
- **Unresolved references.** Source references that resolve to no file, such as directory references,
  are listed at the end of the inventory.
- **Supersession ledger.** It lists rows with a non-empty `superseding_authority`. The governing
  authority is the first superseding authority that is not a PROGRAM obligation, or the original authority
  when there is none. The nature of change is a keyword classification of the supersession note (for
  example contradiction/divergence, supersedes, freezes/refines, extends, narrows/defers, implementation),
  followed by the note itself.
