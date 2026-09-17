# Crypto domain (program section 22): shared experiment conventions

These conventions apply to EXP-CRYPTO-001 … EXP-CRYPTO-023. They were written by the Phase C design revision
(round 1) because the crypto protocols' generated `AUTO:meta`, `AUTO:candidates` and `AUTO:common` blocks were
empty (design review finding DR1-07). The generated blocks are now produced by
`research/tools/experiments/fill_crypto_auto_blocks.py` from `research/experiments/_index/crypto.jsonl`, the
decision ledger and these conventions; the prose sections of each protocol are the designer's text, amended only
where this revision says so. Program-level amendments PA-01 … PA-27
(`research/experiments/_program/design-revision-round1.md`) bind every crypto protocol.

## CR0. Status, gates and decision-grade conditions

- A protocol is a Phase C design artifact. It becomes a `decision-method.md` §12.2 record only when gate G-A holds
  (constraint crosswalk, ledger schema v2 with independently assigned `decision_type`, `od_ids` and `hc_ids`,
  validation-look registry, L7 record, unexposed re-signing under PA-01 and the per-cluster critic pass under PA-12).
- Every run before G-B (decision tooling, frozen held-out lock, workload mix, metric-registry disposition under
  PA-02, runner additions) is `NOT_DECISION_GRADE`. Timing runs additionally need a PASS calibration record for the
  run's calibration identity (PA-04).
- Specs with non-empty `decision_ids` are launched only through `research/tools/experiments/run_guarded.py`
  (PA-13); anything else runs with `--exploratory`, which strips `decision_ids`.

## CR1. Author, independence and held-out identity exposure (L7)

- **Author of the designs:** the Phase C-design crypto designer session (workflow `wf_47347bc4-611`). Its generated
  disclosure block was empty, so this revision records the exposure on its behalf: the shared Phase C-design
  instructions directed every domain designer to read `research/corpus/coverage.md` (which names held-out item
  identifiers and upstream sources in every family) and `research/PROGRESS.md` (whose change log names held-out
  upstreams). The crypto designer is therefore treated as **L7-exposed for all families** (design review DR1-01).
  No held-out content, listing, statistic or path was reported opened, and no crypto protocol names a held-out
  item (checked by the design review).
- **Consequence (§5.4 item 3):** the designer's candidate operationalizations, exclusions and analysis plans must be
  re-signed by an unexposed session before G-A (PA-01), and every decision analysis is executed by an unexposed
  session. The record is `research/experiments/_program/l7-exposures-design-phase.jsonl`.
- **Revision session:** the round-1 design revision did not read `coverage.md`, the PROGRESS change log or the
  corpus critique files; it read the tuning and validation rows of `research/corpus/manifest.json` through scripts
  that drop held-out rows in memory. It edited designs, so it cannot re-sign them.

## CR2. Candidates and R0 completeness

- Candidate ids come from the ledger `candidate_set`. The generated `AUTO:candidates` block lists, per decision, every
  ledger candidate, whether this protocol names it as an arm, which other experiments name it, the ledger exclusion
  reasons, and the R0 checklist (status quo, SPEC design, ledger alternatives, incumbent, defer, critic, re-specify).
- A candidate that no experiment names is an **R0 gap**, listed in the block and in
  `research/experiments/_program/critic-pass-plan.csv` for the crypto cluster.
- **R0 item 6 (critic-proposed candidate)** is open for every crypto decision. It is supplied only by the unexposed
  per-cluster critic pass of PA-12; neither the designer nor this revision can supply it.
- **Exclusions** named in protocols ("excluded at R1") are proposals. Each needs sign-off by a session not involved
  in the candidate set (objective §2.0 rule 3). Cheap excluded candidates may still be run as negative controls.

## CR3. Security claims are provisional; attack results are lower bounds (DR1-08)

- Every attack, leakage or tamper result in EXP-CRYPTO-001 … 019, EXP-CHUNK-006 and EXP-REMOTE-010 is a **lower
  bound** on what an adversary can achieve: a stronger attack may exist. A result of "no advantage", "no
  undetected tamper" or "no recovery" never supports "resists", "is sufficient", "is effective" or "leakage is
  acceptable" on its own.
- A selection whose rationale is a security claim about a construction, the sufficiency of a restriction or padding
  schedule, or the acceptance of residual leakage is **provisional** until independent external review is received
  and dispositioned (§8.3). Its internal evidence goes to the dossier (EXP-CRYPTO-021); internal evidence may
  eliminate candidates (R1/R2 on measured violations) but cannot select on security grounds.
- The decisions whose security-claim parts this revision routes to external review in addition to the ledger's
  `EXTERNAL_REVIEW_REQUIRED` set (review DR1-08): DEC-CRY-025, 028, 030, 034, 045, 050, 060, 064, 069, 072, 077,
  089, 105. EXP-CRYPTO-021's scope names them explicitly; the independent `decision_type` assignment (§1.2) is asked
  to mark those parts EXTERNAL and fill `external_review_requirement`.
- Internal FORMAL preparation of the key hierarchy, commitment, unlock order, binding and mutation questions is
  EXP-CRYPTO-023 (symbolic models, dossier input only).

## CR4. Builds

- End-to-end timing and memory of encrypted packing and opening use `ebound` built from the production workspace at
  the pre-registration commit (harness use constraint 2), target dir `/root/eb-research/target/production/release/`
  on WSL and `D:/eb-research/target/production/release/` on Windows.
- Harness binaries (`ebr-crypto`, research chunkers, research-internals accessors) build from `research/harness`
  after `lock_parity.py` and `harness-cli-parity.sh` print `RESULT PASS` (use constraint 1).
- Research patches of production crates (for example Z1 in EXP-CRYPTO-003) are applied only in a separate worktree
  export, built to their own target directory, and are labelled research builds in every table.

## CR5. Timing and memory

- Affinity configurations (§4.2/§4.3): WSL `st-v` = `[2]`, `mt-v-4` = `[2, 3, 4, 5]`; Windows `st-p` = `[2]`,
  `mt-p-4` = `[2, 4, 6, 8]`. Candidate thread counts equal the configuration size.
- Guards: `max_loadavg_1m` = 1.0 + |cpu_affinity|, `max_other_cpu_percent` ≤ 3.0, `max_wait_s` ≥ 300 (§4.4).
  `research/tools/experiments/spec_lint.py` enforces this.
- Timed commands never read from `/mnt/*` or repository-relative paths; helpers and inputs are staged with
  `stage_tools.py` (PA-16).
- Calibration identity and the instrument-class calibration of PA-04/PA-15 apply; in-process component timers
  (for example `ebr-crypto bench`) need the in-process instrument class calibrated.

## CR6. Metric names

- Canonical names and roles come from `decision-method.md` Appendix A.2; bands are referenced by threshold id, never
  restated.
- **Component throughput** (a primitive, a chunking stage, an AEAD pass) is never reported under the bare T-05b
  names, which mean whole-encode logical bytes per second. Crypto specs use the stratum forms
  `encode_throughput_bps__component_<name>`, `verify_throughput_bps__component_<name>` and
  `decode_throughput_bps__component_<name>`. They are `diagnostic` and may be primary only for a decision about that
  component (for example DEC-MOD-016 digest selection).
- Chunk-stage throughput of keyed boundary constructions has a single source, EXP-CHUNK-008 T1 (PA-17); crypto specs
  do not measure it.
- HC oracle counts without an Appendix A.2 name are reported under their MVT identifiers as binary counts; the names
  proposed in `research/experiments/_program/metric-registry-proposal.json` are used once the round-2 method review
  adopts them (PA-02).

## CR7. Split discipline and look specs

- Committed crypto specs select `tuning` only, or are binary HC screens (`status: hc-screen`) that run on every split
  as §8.2 item 13 requires.
- A graded validation look is run from a look spec derived at look time from the tuning record, with candidates
  restricted to W ∪ S and `status: look-1` or `look-2`, and only by the look owner or a participant named in
  `research/experiments/_program/look-plan.csv` (PA-03, PA-09).

## CR8. Held-out placeholder

Held-out specs are generated only by EXP-EVAL-012 at Commit A (PA-20). Each crypto protocol's held-out section
declares its selection rule only; no held-out item, path or statistic appears in any crypto file.

## CR9. Cross-reference corrections applied by this revision

The crypto protocols were renumbered during design. This revision corrected stale references: the external-review
dossier is EXP-CRYPTO-021 (earlier text said EXP-CRYPTO-022, 024 or 025); password-KDF cost is EXP-CRYPTO-017
(earlier text in EXP-CRYPTO-004 and 007 said EXP-CRYPTO-021); primitive throughput is EXP-CRYPTO-012 (earlier text in
EXP-CRYPTO-004 said EXP-CRYPTO-013); simulated first-use is EXP-CRYPTO-022 (unchanged).

## CR10. Seeds

Unless a spec states otherwise: ebr `order`, `bootstrap` and `command` seeds `20260917`; key seeds derive from
`seeds.command` as `sha256(seed || cell || i)`; these are research test keys, never production randomness.
