# EXP-EVAL-012: Final held-out whole-system evaluation (Phase D; placeholder design under `decision-method.md` §5)

| Field | Value |
|---|---|
| Domain | evaluation (program sections 6, 7, 34, 35) |
| Status | **BLOCKED**. Phase D prerequisites are unmet: design freeze Commit A and Commit B (`research/decisions/design-freeze.json` with `design_freeze_sha`), unlock Commit C (`research/corpus/heldout-unlock.json`), gates G-A, G-B and G-C, held-out lock `frozen`, EXP-EVAL-010 audits passing, and the EXP-EVAL-011 sealed tranche. Held-out access of any kind is forbidden before Commit C (§5.4). |
| Kind | The single program-wide held-out run (§5.5): R1, R2, R4, R5 and R7 re-run for every frozen decision, with no re-selection. It also produces the whole-system final evaluation of program §35: capability-matched incumbent comparisons, CC outcomes, and the section on workloads where Entrybound is not best. |
| This file | A placeholder design. The executable held-out specs and the pre-unlock record are generated at Commit A from the frozen C-exec validation specs and committed inside Commit A (§5.5 item 1). No held-out item is named here; corpus selectors name splits only. |
| Method | `research/decision-method.md` §2 R5-R10, §4.12, §5 (all), §6.6, §8, §10 at `14b977c` |

## Author, independence and exposure

Designed by the Phase C-design evaluation-domain session, which has a recorded L7 exposure to identifiers of the identity-aware held-out set (EXP-EVAL-001 protocol).

**Pre-registered consequences (§5.4 item 3):**
- This session may not generate the Commit A held-out specs, the pre-unlock MDE record, or any held-out analysis.
- Those tasks go to sessions with no recorded exposure for the affected families. The exposure record is `research/decisions/l7-exposures.jsonl`, verified by the EXP-EVAL-010 transcript audit.
- If no such session exists for a family, that family's identity-aware held-out analysis carries the exposure in its record. The sealed tranche is analysed only by an unexposed session.

## Question

1. **R5 per frozen decision.** For every decision frozen at Commit A, does its provisional selection pass R5 on held-out? The R5 conditions are:
   - (a) no R1 or R2 failure;
   - (b) W not eliminated;
   - (c) superiority replicated, with no failed verdict and at least one replicated verdict per pair;
   - (d) non-inferiority and equivalence verdicts hold at one-sided 0.95;
   - (e) the family, tier and condition guard holds;
   - (f) the R7 bars hold on held-out.
2. **Whole-system (program §35).** On held-out, how does the frozen design (`REF-FROZEN`: `ebound` built at Commit A, default configuration and decided configurations) compare with the capability-matched incumbents on size, speed, memory, access, fidelity and security properties? What are the CC-01 to CC-10 outcomes for the default configuration and for the decided set? On which workloads is Entrybound not best, and by how much?

## Hypotheses and falsification

Pre-registered by reference, never restated: each decision's H1 is its own validation-look superiority, non-inferiority or equivalence declaration, frozen in its `record.md` at Commit A. R5 conditions (a)-(f) are the falsification rules.

For the whole-system part:
- **H-SYS.** The EXP-EVAL-002, EXP-EVAL-005 and EXP-EVAL-006 claim verdicts obtained on validation with the frozen build replicate on held-out under R5(c)-(d).
- A claim is **withdrawn** if any supporting verdict is *failed* on held-out.
- A claim is **qualified** if its support is only *attenuated*.

## Decisions informed

Every decision frozen at Commit A (R5). The program-§35 decisions are listed for this experiment in `research/experiments/_index/evaluation.jsonl`: the claim decisions (`DEC-CMP-011`, `DEC-ECO-069`, `DEC-ECO-070`, `DEC-CRY-042`, `DEC-CRY-043`, `DEC-CRY-044`, `DEC-PLT-024`, `DEC-PLT-039`, `DEC-ACC-002`, `DEC-ACC-041`) and the method decisions whose evidence includes the held-out run (`DEC-ECO-031`, `DEC-ECO-076`, `DEC-ECO-077`).

## Candidates

- **Per decision.** **Every candidate that reached the freeze** (R5: "not only W"), exactly as frozen: commit, build flags, `Cargo.lock` SHA-256 and toolchain recorded in Commit A.
- **Whole-system.**
  - `REF-FROZEN-default`: `ebound pack` with no flags, plus the default extraction policy.
  - `REF-FROZEN-<decided configuration>` for each decided profile, layout and encryption mode.
  - `REF-PROD` (research baseline `9e44608`) as the historical reference.
  - `REF-INC`: the specialists of objective §1.3 and the EXP-EVAL-002/005/006 incumbent sets, including the tuned-view incumbent configurations frozen from EXP-EVAL-007 Q2.
- **Excluded.** Any candidate modified after Commit A: its runs are `post-unlock-fix`, report-only, and it cannot become DECIDED on this evidence (§5.6).

## Corpus selectors (splits only; no item named)

- `heldout` (the identity-aware held-out set frozen by `heldout-lock.json`), and `heldout-sealed` (EXP-EVAL-011 tranche S1, opaque ids).
- Item lists are **not** chosen here. Each held-out spec uses the same selection rule as its validation spec, applied to the held-out split by the spec generator at Commit A. For example, EXP-EVAL-005 takes one item per (family, independence group, tier), and EXP-EVAL-002 takes all items.
- Family weights: `workload-mix.json` at its Commit A SHA-256.

## Environment

The same environment, affinity and cache configurations as each decision's validation look (§4.1: numbers from different environments are never paired). Timing runs in exclusive quiet windows, after re-running EXP-EVAL-004 A/A drift checks if the environment fingerprint changed since calibration (§4.7: "repeated whenever `env_id` changes").

## Commands (generated at Commit A; TO BE WRITTEN)

```sh
# Commit A (by an unexposed session): generate held-out specs + pre-unlock record
C:/Python313/python.exe research/decisions/tools/make_heldout_specs.py --frozen-ledger research/decision-ledger.jsonl --validation-specs research/experiments --splits heldout,heldout-sealed --out research/experiments/EXP-EVAL-012/specs/
C:/Python313/python.exe -m decide heldout-mde --specs research/experiments/EXP-EVAL-012/specs/ --heldout-group-counts research/corpus/heldout-lock.json#fingerprint-fields --sealed-group-counts research/corpus/sealed/manifest.public.json --out research/experiments/EXP-EVAL-012/pre-unlock-record.json
#   -> per supporting comparison: MDE (band units) on identity-aware, sealed, combined; decisions with any MDE > 2 routed to INSUFFICIENT_EVIDENCE (still run, report-only)
# Commit B: research/decisions/design-freeze.json {design_freeze_sha: <A>, decision_ledger_sha256, heldout_set_sha256, decisions, heldout_specs, hashes..., approved_by, date}
# Commit C: research/corpus/heldout-unlock.json {design_freeze_commit: <A>, approver, reason}
# provision + verify held-out (corpus session): research/corpus/tools/provision.py --split heldout --verify-lock ; sealed: owner decrypt + verify
# run once per spec: EB_HELDOUT_UNLOCK=<A> python -m ebr run --spec <spec> [--timing ...]; verify-raw; normalize
# analysis (unexposed session): python -m decide heldout --experiment <EXP> --pre-unlock-record ... ; python -m decide system-eval --out research/reports/final-system-evaluation/
```

## Seeds

The seeds of each frozen validation spec, plus the constant 1,000,000 added to `order` and `bootstrap`. That gives a new, recorded run order without per-run choice. The sealed tranche uses the same rule.

## Warmup

Per each decision's validation protocol (§4.5).

## Measurement method and metrics

Exactly the primary, secondary and binary metrics frozen in each decision's `record.md`: canonical names from Appendix A.2, with no additions. A metric added after unlock is secondary and cannot support anything (§0.2, §4.12). The whole-system report uses these:
- EXP-EVAL-002 metrics: `artifact_bytes`, `fixed_overhead_bytes`, `roundtrip_mismatches`, `single_file_self_contained`;
- EXP-EVAL-005 metrics: `encode_wall_s`, `decode_wall_s`, `random_entry_latency_s`, `stream_unpack_wall_s`, `peak_rss_bytes`, `peak_commit_bytes`;
- EXP-EVAL-006 metrics: `undetected_tampers`, `restore_fidelity_fraction`, `hostile_name_handling_fraction`, `streaming_capability`;
- remote-domain metrics for CC-07: `http_requests`, `verified_range_fetch_bytes`, `remote_random_entry_latency_s`.

## Replication and adaptive rule

Each held-out experiment runs **exactly once** (§5.5 item 4). Repetitions and adaptive rounds follow each validation protocol (§4.6). Re-runs happen only for infrastructure failures (invalid samples, crashes, guard aborts), every attempt is kept and reported, and a re-run chosen by outcome is leakage (§5.6, §5.7).

## Normalization

Per each validation protocol (§4.9). Three strata are reported: sealed only, identity-aware only, and combined. They are never silently pooled.

## Statistical analysis

- **R5 per decision.** Held-out confidences of §4.12: non-inferiority and replication at one-sided 0.95 (two-sided 0.90 interval); family guard at 0.90; classification as replicated, attenuated or failed per R5(c); R7 bars of §6.6 re-evaluated on held-out (R5 f).
- **Cap rule (§5.4).** Any R5 pass whose held-out evidence includes identity-aware items carries `identity_aware_heldout` (MEDIUM). A pass on sealed-only evidence carries no such cap, provided the sealed stratum meets §4.9 minimum support and every MDE is at most 2 band units on the sealed stratum alone.
- **Program level (§4.12).** Benjamini-Yekutieli q-values at q = 0.05 over every decision-supporting comparison. A support that does not survive is recorded as counterevidence.
- **CB-6** replication-failure trigger per §10.2.
- **Whole-system.**
  - CC outcome table (objective §1.3: `SERVED`, `SERVED_WITH_DECLARED_COST`, `NOT_SERVED`), separately for the default configuration and the decided set, per tranche.
  - Claims table: final status per SPEC §22-§23 mark (`SUPPORTED`, `QUALIFIED:<scope>`, `WITHDRAWN`, `BLOCKED:<platform>`).
  - **Workloads where Entrybound is not best:** every (family, tier, operation, metric) whose held-out point estimate favours a named incumbent beyond the band, with magnitudes produced per §12.3 (§10.1 item 4, program Task 35).

## Practical-significance thresholds

Each decision's frozen primary metric bands from `thresholds.json` `metric_thresholds` at its Commit A SHA-256, unchanged (§0.2 item 2). Held-out MDE limit of 2 band units (R5).

## Sensitivity analysis

§6.5 arms on held-out:
- equal-weight arm;
- leave-one-family-out and leave-three-families-out;
- family Dirichlet;
- WM-* mixes;
- tier-stratified;
- byte-weighted;
- environment arm;
- reference arm (REF-FROZEN versus REF-INC and REF-PROD);
- with and without public-benchmark-tagged items (L5);
- with and without covariate-flagged items (EXP-EVAL-010 overlap flags);
- tranche strata.

## Expected negative results worth recording

- Attenuated or failed replications of marginal validation wins (CB-4 cases).
- `NOT_SERVED` CC outcomes: CC-10 pending external review, and CC-01 in families where solid compression wins by more than the declared cost.
- Held-out families where incumbents dominate (expected F11-F13 already-compressed media for size, F17 dedup filesystems, raw zstd speed).
- Decisions routed to INSUFFICIENT_EVIDENCE by held-out MDE before unlock.

Everything goes to the negative-results register and to `research/reports/final-system-evaluation/` (program §35 deliverable).

## Threats to validity

- **Identity-aware exposure of the original held-out set.** Mitigated by the cap and the sealed stratum.
- **Frozen build versus the later released build.** Any post-freeze change needs fresh confirmatory data: sealed tranche S2 via EXP-EVAL-011, per §5.6.
- **Held-out group counts per family** are mostly 2-4, so family guards are weak. That is a recorded limitation of every scoped claim (§4.9).
- **Environment drift** between validation and held-out runs (Windows updates, Defender signatures). Environment fingerprints are compared, and a changed `env_id` triggers the EXP-EVAL-004 A/A re-check before timing runs.

## Platform requirements

Same as the union of the frozen experiments: Windows host, WSL2, application-level network emulation (remote decisions), large storage (the identity-aware held-out set is about 42 GiB; the sealed tranche is at most 60 GiB), exclusive quiet windows, and owner actions for Commits B and C and the sealed decryption. macOS and native ARM64 scope: EXP-EVAL-013 (BLOCKED).

## Estimates

- Machine time: about 500 h (held-out re-run of every frozen decision's validation-grade experiments, including about 300 h of timing in quiet windows; this estimate covers this domain's whole-system part at about 150 h, plus the program-wide R5 re-runs owned by other domains' frozen specs).
- Disk: about 250 GiB peak.
- Agent effort: none during runs; judgment for the pre-unlock record and held-out analysis (unexposed sessions) and for the final evaluation report.
