# EXP-CONF-013 — Organizationally independent implementation and external review of the specification, conformance corpus and fuzzing adequacy (BLOCKED)

| Field | Value |
|---|---|
| Status | **`BLOCKED`**: requires at least one external human implementer with no prior Entrybound source access and at least one external reviewer (specification/conformance and security); this program has no human participants and no external reviewers (`research/PROGRESS.md` known limitations; `decision-method.md` §8.3). Also requires owner decisions to publish a specification snapshot and corpus under a licence (DEC-ECO-067) and to contact external parties. |
| Domain / program sections | conformance / §28 (primary), §29, §30 |
| Decision IDs informed | DEC-ECO-032, DEC-ECO-038, DEC-ECO-014, DEC-ECO-042, DEC-ECO-028, DEC-CRY-016, DEC-CRY-017 |
| Requirements | REQ-ECO-0072 (independent implementation before v1.0 stable), REQ-ECO-0030, SPEC §20.6 |
| HC oracles | HC-12 MVT-12(a) with organizational independence (objective HC-12 status: `EXPERT_REVIEW_REQUIRED`); OD-21 M21.3 bias check |
| Decision type | EXTERNAL parts per §1.2 and §8.3 (assignment by the independent session at G-A) |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1 |
| Depends on | EXP-CONF-001 to -012 outputs as the dossier; owner approvals |

## 1. Question

Does an implementer outside the program, with no Entrybound source access and no shared base model, build a baseline
reader from the published specification snapshot, vectors and exported corpus that passes the corpus with matching
outcome classes and reason codes; how many specification defects does that implementer report per 1,000 normative
words compared with the in-program clean-room reader; and do external reviewers accept the traceability, taxonomy,
fuzzing adequacy criterion and fuzzing results as sufficient for v1?

## 2. Hypotheses and falsification

- **H1 (HC-12, organizational independence).** The external reader passes `ebcc-v1` (current version) at
  `cleanroom_pass_fraction` = 1 after triage attributes every remaining disagreement to the external reader.
  *Falsified* by one disagreement attributed to production or to specification text.
- **H2 (in-program bias of M21.3).** The external `spec_defects_per_1000_words` is not lower than the in-program value
  of EXP-CONF-001. This is descriptive: a higher external value is evidence that the in-program count is biased low
  (objective OD-21 note).
- **H3 (review acceptance).** External reviewers raise no blocking finding on (a) traceability and recall (EXP-CONF-003),
  (b) the expectation taxonomy and corpus release form (EXP-CONF-005), (c) the fuzzing adequacy criterion and campaign
  results (EXP-CONF-008/009/010), (d) the crypto conformance evidence handed to the crypto review dossier
  (EXP-CONF-009 T08-T12, EXP-CONF-012 V4-V6). *Falsified* by one blocking finding.

## 3. Design (to execute when unblocked)

1. **Package**: specification snapshot (SPEC plus normative documents at a tagged commit), permanent vectors, exported
   `ebcc-v1` bytes and manifest, `ebcc.report.v1` schema and the subprocess adapter protocol of EXP-CONF-005 arm D, the
   `ebr.ambiguity.v1` schema; no source code.
2. **Implementer brief**: scope L-base (EXP-CONF-001 §7) in any language except Rust, with no shared Entrybound crates;
   log ambiguities at encounter; phase A1 without corpus expectations, then corpus access; report time spent.
3. **Program-side measurement**: the program runs EXP-CONF-002's corpus-pass, mutation and fragmentation modes with the
   external reader binary as `other` (spec derived from `research/experiments/EXP-CONF-002/spec.yaml` by replacing the
   `EBIR-*` candidates with `EXT-*`), triage per CC§6 by a program session, with the implementer invited to contest
   attributions.
4. **Review**: reviewers receive the dossier (question, candidates, internal evidence, specific review questions per
   §8.3) and return findings classified blocking or non-blocking; dispositions are recorded in the ledger.

## 4. Candidates

DEC-ECO-032 `commissioned-external-full-reader` (full reader including verification; encryption tier only if the
implementer and crypto reviewers agree), compared with the in-program `agent-clean-room-reader-with-ambiguity-log`
results. Other decisions' candidates as in their ledger rows (CC§1: none added).

## 5. Inputs, environment, platform

Published package only; the implementer's own environment; program-side runs in WSL as EXP-CONF-002. No corpus split
items; no held-out data.

## 5a. Seeds and replication

Mutation and truncation seeds as EXP-CONF-002 (`2809170001`); triage reliability sample `2809170003` (CC§12); two
repetitions of every program-side mode with repeat-hash (§4.13). The external implementation is a single construction
(no replication is possible with one implementer; recorded as a limitation).

## 6. Metrics (CC§9)

`cleanroom_pass_fraction` (binary), `reader_disagreements` (binary), `independent_reader_loc` (cost input),
`spec_defects_per_1000_words` (descriptive), external review findings by class (descriptive).

## 7. Analysis, thresholds, sensitivity

As EXP-CONF-002 §10-§13 for the differential; H2 descriptive comparison without statistics (one external implementer);
review findings dispositioned per §8.3. Thresholds: §3.3 for binary metrics.

## 8. Expected negative results worth recording

External defect counts materially above the in-program count; attributions contested by the implementer; reviewers
requiring RFC 2119 rewording before HC-12 can pass (objective MVT-12(b) consequence).

## 9. Threats to validity

A single external implementer is one reading; implementer skill and time limits; commissioned work may be shaped by
the brief; publication before v1 exposes the unfrozen specification to external readers (owner decision).

## 10. Cost estimates

| Item | Estimate |
|---|---|
| Compute (program side) | 160 CPU-hours (EXP-CONF-002 modes against the external reader) |
| Disk | 25 GB |
| Agent effort | **judgment, medium** (packaging, triage, dossier); the external implementation and reviews are human effort outside the program |
| Platform | Linux x86-64 (WSL) for program-side runs; external implementer's platform unconstrained; **external humans required** |

## 11. Unblocking condition

Owner approval to publish the package and to contact implementers and reviewers, plus a signed-up external implementer
and reviewers; then `record.md` is instantiated and gates CC§3 apply.
