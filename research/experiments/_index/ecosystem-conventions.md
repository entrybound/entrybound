# Ecosystem domain (program sections 31, 32, 33): shared experiment conventions

Every `EXP-ECO-*` protocol cites this file for the rules below and states only what is specific to it. Where this file and `research/decision-method.md` differ, the method governs; where a protocol deviates from this file it says so explicitly.

| Field | Value |
|---|---|
| Domain | ecosystem: dependency policy and longevity (§31), adoption workflows (§32), CLI and library usability (§33) |
| Experiments | EXP-ECO-001 to EXP-ECO-022 |
| Method | `research/decision-method.md`, pre-registration commit `14b977c` (or the revision in force when a record is instantiated) |
| Objective | `research/archetypal-objective.md`: OD-21, OD-22, OD-25, OD-26 primarily; HC-05, HC-09, HC-12, HC-16, HC-18 screens |
| Designer | Phase C-design ecosystem-domain session, 2026-09-17. It did not construct any ledger candidate set. |
| State of evidence | No experiment in this domain has run. The only observations are two feasibility checks (tool availability for EXP-ECO-001; crates.io version lists used to pre-register EXP-ECO-002 arms). Neither is a result (§12.3 item 6). |

## 1. L7 identity-exposure disclosure

As the Phase C-design rules require, this session read `research/PROGRESS.md` (whose change log names upstream sources of held-out items in F04, F12, F13, F15, F16, F17 and F20) and the start of the `research/corpus/coverage.md` independence-group table (which names held-out items in F03, F05, F09, F11, F15, F16 and F17). No held-out content, listing, statistic or path under a held-out root was opened; every manifest query of this session filtered to `split in {tuning, validation}` before printing. This is an L7 event (§5.7) for the §5.8(h) record. §5.4 item 3 bars an exposed session from designing *candidates or analyses* for decisions whose applicable families include the exposed items. Consequences adopted here:

- No protocol introduces a new decision candidate; candidates are the ledger's `candidate_set` entries (and, where a protocol needs a runnable or countable form of one, the translation is mechanical and is reviewed by the candidate owner before any run).
- Analyses of results for decisions whose applicable families include exposed families are executed by a session without that exposure; the pre-registered analysis plans here are procedures, not analyses of data.
- Every R5 pass carries `identity_aware_heldout` unless its held-out evidence comes from a sealed tranche (§5.4).

## 2. Gates and pre-registration state

- Each `protocol.md` is the design pre-registration. The §12.2 `record.md` is instantiated from it only after gate G-A holds (constraint crosswalk, ledger schema v2 with `hc_ids`, `od_ids` and `decision_type` assigned by an independent session, validation-look registry `research/decisions/validation-looks.jsonl`), and it is committed with `spec.yaml` before any run.
- `decision_type`, `od_ids` and `hc_ids` named in protocols are proposals for the independent assigner (§1.2, §14 item 12); they bind nothing.
- No run with non-empty `decision_ids` starts before G-A. No verdict is decision-grade before G-B (thresholds test update §14 item 1, decision tooling §14 item 2, runner additions §14 item 3 for timing, calibration §4.7 for timing and memory, frozen held-out lock §5.3, workload mix §14 item 13). Deterministic census runs may execute earlier as informational runs labelled `NOT_DECISION_GRADE` and are re-verified by repeat-hash against the same build after G-B.
- Cost tiers (§7.2) come from the §14 item 7 scripts (EXP-ECO-001 builds the C3 part) and from a C4-C7 assessor not involved in the candidates, before the first validation look. Tiers named in protocols are expectations.
- Validation looks: at most two per decision (§5.2), each appended to the look registry.

## 3. Status vocabulary used in `ecosystem.jsonl`

| Status | Meaning in this domain |
|---|---|
| READY | Every instrument exists or is an installable pinned tool or an experiment-local script fully specified in the protocol (mechanical to write, no design judgment), and every platform the experiment needs is available on this host. Program-wide gates (G-A, G-B, calibration) still apply. |
| NEEDS_TOOLING | A non-trivial instrument with design content must be built first (prototype code in a research worktree, a session harness, API sketches); `tooling_to_build` names it. |
| BLOCKED | The experiment needs a platform, access or participants unavailable to this program; `blocked_reason` names exactly what is missing. The protocol is complete so that `remaining-unknowns` can cite it. |

## 4. Builds and identities

| Build | Identity | Built how | Used for |
|---|---|---|---|
| REF-PROD `ebound` | research baseline `9e44608` | fresh `git clone` of the repo at that SHA into `/root/eb-research/src/eco-ref-9e44608`, `cargo +1.98.1 build --release --locked -p entrybound-cli`, target dir `/root/eb-research/target/eco-ref` (WSL) or `D:/eb-research/target/eco-ref-win` (Windows) | status-quo reference |
| dev-HEAD `ebound` | execution commit, `dirty=false` | same procedure at the execution SHA, target `.../eco-head` | current behaviour; reported beside REF-PROD |
| Research worktree builds | `9e44608` or dev HEAD plus a committed patch under `research/experiments/<EXP>/patches/` | applied in a clone under `/root/eb-research/src/<exp>-<arm>`; never applied to the production tree (standing decision 2026-09-12) | partition prototypes, dependency-upgrade arms, CLI shims |

Decision-grade timing uses production-workspace `ebound` builds without `research-internals` (harness review R1-06). `lock_parity.py` and `harness-cli-parity.sh` must print `RESULT PASS` before any campaign that uses harness binaries (harness review use constraint 1).

## 5. Human-facing evidence (§1.2, §4.16, T-18)

- Mechanically checkable properties decide: `flags_per_task` (C6 cost input), `unsafe_defaults` (binary, HC-05), `error_actionability_fraction` (T-20), exit-status and message censuses, parser conformance.
- Simulated first-use sessions (EXP-ECO-019, EXP-ECO-020 integrator arm) are labelled `SIMULATED`, role `heuristic` (T-18): per-task success counts and qualitative failure modes only, no confidence-interval verdict, at least 20 sessions per (task, candidate), maximum evidence state `SUPPORTED_WITH_LIMITATIONS`, never sole support.
- Any claim about human comprehension, task success or preference is routed to `EXTERNAL_REVIEW_REQUIRED` or removed from the decision (§4.16). The human studies that could carry such claims are designed as BLOCKED experiments EXP-ECO-021 and EXP-ECO-022.
- Held-out task sets for the mechanical parts (§4.16 "a held-out task set written before design iteration") are authored by a session not involved in CLI candidate design, committed only as the SHA-256 of an owner-held file before any candidate iteration, and opened only after the design freeze (sealing route, §5.4).

## 6. Sampled external populations

EXP-ECO-011 (registry artifacts) and EXP-ECO-013 (public build scripts) sample populations outside the corpus. Rules:

- The population frame (index files, dumps) is downloaded once, hashed and stored under `/root/eb-research/cache/<exp>/frame/`; the frame hash is recorded before sampling.
- Tuning and validation samples are drawn with seeds committed in the protocol, disjoint by construction (sampling without replacement in one seeded permutation: first block tuning, second block validation).
- The held-out sample is drawn after the design freeze from a fresh frame snapshot with an owner-held seed committed now only as its SHA-256 (§5.4 sealing route). Until then no one draws it.
- These samples are not corpus items; decision use requires the corpus-methodology addendum listed as a threat in each protocol (L4-style change control: the sample list is committed before measurement and never edited after results are visible).

## 7. Commands and paths

- Scripts named in protocols are to be written under `research/tools/ecosystem/<experiment-topic>/` (stdlib Python 3.12 in `/root/eb-research/venv` unless stated) and committed before the record; every output carries a provenance header (command, git SHA, tool versions, input hashes).
- `ebr` specs run from the repository root in WSL: `PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr {validate,run,verify-raw,normalize,report} --spec research/experiments/<EXP>/spec.yaml`.
- Large downloads live under `/root/eb-research/cache/<exp>/` (WSL ext4 on D:); `research/orchestration/disk_guard.py` passes before launch; nothing is written to C:.
- Nothing in these experiments sends messages, publishes content, creates accounts, accepts terms, or changes host security or system settings. Steps that would need any of those are excluded or BLOCKED with the reason.

## 8. Held-out (Phase D placeholder)

For corpus-driven parts: held-out items are run once after Commit C under the single program-wide freeze (§5.5), every candidate that reached the freeze is run (R5), results are report-only after unlock (§5.6), and the held-out MDE gate (≤ 2 band units) is computed from validation group variance and fingerprint-level group counts before unlock. For AGG-P census facts: recomputed at Commit A on the frozen lockfile or build; any change is a deviation.
