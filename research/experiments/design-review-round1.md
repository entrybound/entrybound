# Experiment pre-registration design review, round 1

| Item | Value |
|---|---|
| Date | 2026-09-17 |
| Role | Phase C-design adversarial reviewer of the experiment pre-registration (program §6-§35). This session designed no experiment, candidate or analysis. |
| Reviewed | `research/experiments/index.csv` (237 experiments, SHA-256 `9dc7394d…`), `research/experiments/decision-coverage.csv` (596 decisions, SHA-256 `c65e8f68…`), every `protocol.md` and ebr spec document under `research/experiments/EXP-*` (308 spec documents), the domain conventions (`_index/*-conventions.md`, `_index/platform-common-provisions.md`, `_index/reconstruction-common.md`, `_legacy/conventions.md`, `research/planner/README.md`) and the 15 `*-uncovered.jsonl` fragments, at HEAD `6b76efb`. |
| Against | `research/decision-method.md` (SHA-256 `7b00d9ef…`, pre-registration `14b977c`), `research/methods/thresholds.json` (`9cd4b5be…`), `research/archetypal-objective.md`, `research/harness/harness-review-round1.md` (binding use constraints), `research/baselines/README.md`, `research/corpus/coverage.md`, `research/PROGRESS.md`. |
| Method | (1) A mechanical metadata scan of **all 237** experiments and 308 spec documents (`research/experiments/_review/design_review_round1_scan.py`, output `research/experiments/_review/design-review-round1-metadata.csv`). (2) Scripted cross-checks of coverage against the ledger, per-decision validation-look pressure, timing-confound controls, split selectors, platform prerequisites and repository paths. (3) Full reading of the protocols most exposed to the review questions: EXP-CHUNK-004, 008; EXP-EVAL-002, 004, 005, 012; EXP-REMOTE-001; EXP-RECON-004; EXP-PLANNER-002, 006 (opening), 012; EXP-CRYPTO-003, 005, 006, 021 (key sections); EXP-ECO-019; EXP-LEGACY-021; plus targeted sections of about 60 further protocols. (4) Read-only environment probes inside WSL (installed packages, `/dev/kvm`, kernel config, free space). |
| Timing | No experiment was run. The only computation is the synthetic MDE feasibility table of DR1-06, which uses group counts and assumed variances, no experiment data (not decision-relevant, §0.1). |
| Verdict | **Not ready to serve as the pre-registration record.** 2 BLOCKER, 10 HIGH, 15 MEDIUM, 8 LOW findings (35). The domain designs are, on the whole, careful and method-literate: most per-experiment protocols handle bands, verdict classes, MDE gates, modelled-versus-measured evidence, `VIRTUALIZED`/`EMULATED` labels, slow start and harness use constraints correctly. The blocking problems are program-level: the identity exposure of every design session, and metric-registry gaps that become irreversible at the first decision-relevant result. |

Severity: **BLOCKER** = the pre-registration cannot be accepted as the record, or a class of decisions becomes permanently undecidable, unless fixed before G-A/G-B. **HIGH** = an experiment or a class of experiments cannot produce the decision-grade evidence it is designed for, or its conclusions would be biased or leak across splits, as designed. **MEDIUM** = a confound, unfairness, gap or metadata error with contained effect, or an evidence route that is weaker than it needs to be. **LOW** = documentation, bookkeeping or narrow-scope issue.

## Disclosure (L7)

This review session read `research/corpus/coverage.md` and `research/PROGRESS.md` as its instructions required. `coverage.md`'s independence-group table names held-out item identifiers and upstream sources in every family, and a read-only listing of `/root/eb-research` showed the held-out root's name. No held-out content, listing of a held-out directory, statistic or path below the held-out roots was opened. No held-out identifier is repeated in this file, in the scan script or in its CSV. A one-off check by this session found **zero** held-out item identifiers in any file under `research/experiments/` (the committed scan script deliberately omits that check so that re-running it does not expose a session). This session is therefore L7-exposed for all families and must not be the unexposed re-signer that DR1-01 requires.

## Summary

| ID | Sev. | Area | Experiments / decisions | Finding (one line) |
|---|---|---|---|---|
| DR1-01 | BLOCKER | Split discipline (L7) | all 237; every domain designer and this reviewer | Every design session was directed to read `coverage.md`, so §5.4 item 3 bars all of them from designing candidates and analyses for every family; no unexposed re-signing is planned |
| DR1-02 | BLOCKER | Metric registry | EXP-INT-*, EXP-CONF-008 and CONF-*, EXP-CHUNK-003 | Decision-bearing quantities without Appendix A.2 names become secondary forever at the first result (§0.2 item 4); designers asked for disposition and none exists |
| DR1-03 | HIGH | Validation looks | 103 decisions (e.g. DEC-CON-004, DEC-CMP-026, DEC-CMP-001) | No cross-domain look allocation; many experiments each plan a look for the same decision; planner look waits on other domains (deadlock risk) |
| DR1-04 | HIGH | Calibration contract | EXP-EVAL-004 and every timing experiment | `env_id` hashes the repository commit and Docker plugin versions, so §4.7 "repeat whenever env_id changes" is ill-defined |
| DR1-05 | HIGH | Cost | program (7,621 timing h; 23,137 h; 9,593 GB summed disk) | Exclusive-window timing and storage budgets are not reconciled with the host; four experiments exceed D: headroom; VHDX can outgrow D: |
| DR1-06 | HIGH | Power | size-banded decisions (EXP-CHUNK-004, PLANNER-006, CODEC-001, CROSSFILE-001/003/004, RECON-004) | Feasibility of the MDE gate is not checked before large campaigns; T-01 passes only for within-family SD ≲ 0.023 log units |
| DR1-07 | HIGH | Pre-registration completeness | EXP-CRYPTO-001 … 022 | All 22 crypto protocols have empty generated `AUTO:meta/candidates/common` blocks; crypto specs contradict their protocols |
| DR1-08 | HIGH | External review | DEC-CRY-025, 028, 030, 034, 045, 050, 060, 064, 069, 072, 077, 089, 105; DEC-CRY-039/049/053/084 | Construction and leakage-acceptance decisions are outside the EXP-CRYPTO-021 dossier scope and routed only through internal experiments |
| DR1-09 | HIGH | Split discipline (validation) | 38 specs mixing splits; EXP-LEGACY-032/033/040/041/050, EXP-PLAT-015; legacy generated cases | Validation rows of all candidates are produced alongside tuning; generated legacy validation shares structure (C0 byte-identical) and group ids with tuning |
| DR1-10 | HIGH | Timing guards | EXP-PLANNER-003, 004, 005, 006 (with 007) | Encode/decode guards use work-unit proxies on tuning and are first timed at look 1, so the §4.12 MDE gate is uncomputable, and Windows has no tuning timing at all |
| DR1-11 | HIGH | Incumbent fairness | EXP-EVAL-005 (DEC-CMP-011, DEC-ECO-070, DEC-ACC-002, DEC-CRY-042) | Random access to 7z/ZIP/tar incumbents goes through a streaming libarchive Python binding and FUSE, a weak reader that biases published marks toward Entrybound |
| DR1-12 | HIGH | Candidate completeness | program; slots left open in EXP-CHUNK-001/002, CROSSFILE-001, PLANNER-003/005, integrity C0, legacy C11 | R0 item 6 (critic-proposed candidate) is open everywhere and assigned to "the design review", which cannot supply it |
| DR1-13 | MEDIUM | Status accuracy | 39 of 43 READY experiments; EXP-ECO-004, PLAT-024, LEGACY-020 | READY rows name unwritten drivers; qemu-user, aarch64 cross-gcc and podman are not installed; decision-linked specs lack a gate-state guard |
| DR1-14 | MEDIUM | Blocked routes | EXP-INT-018, SCALE-016, CON-009, CON-010, REMOTE-001 (A5) | Nested KVM is available in WSL, so kernel netem, dm-flakey/dm-log-writes and throttled block devices are reachable in a guest; several BLOCKED arms are partly unblockable |
| DR1-15 | MEDIUM | Timing confounds | EXP-EVAL-004, SCALE-013, CROSSFILE-008, all in-process timers | Calibration covers process timers only for known effects; T-14 pairs come from separate runs; WSL vCPU core-class placement is undetected |
| DR1-16 | MEDIUM | Timing confounds | EXP-CODEC-004, CON-003, CON-005, ECO-008, EVAL-005, PLANNER-003, PLANNER-007, REMOTE-001, REMOTE-006, SCALE-016 | Timed commands load wrapper scripts or helpers from `/mnt/d` (drvfs/9P) |
| DR1-17 | MEDIUM | Build parity, duplication | EXP-CHUNK-008, EXP-CRYPTO-003 (DEC-CRY-023, DEC-CMP-001) | Parity arm does not cover the `ebr-cdcpack` wrapper; thread-CPU `encode_cpu_s` departs from T-05; keyed-chunking throughput is measured twice with different ids and metric semantics |
| DR1-18 | MEDIUM | Controls | EXP-SCALE-005/011/012/014, CROSSFILE-009/010, PLANNER-009, LEGACY-031, ECO-004, CHUNK-012; EXP-CRYPTO-003 | Determinism screens have no planted-nondeterminism positive control; HC-14 MVT-14(e) for status-quo keyed modes is tuning-only |
| DR1-19 | MEDIUM | Ablation | v6 stages across CHUNK-007, CROSSFILE-001…004, CODEC-004, RECON-004, PLANNER-004/006 | Per-stage off-arms exist, but dedup-off is modelled only and no end-to-end leave-one-stage-out matrix attributes overlapping savings |
| DR1-20 | MEDIUM | Held-out placeholder | EXP-EVAL-012, EXP-PLANNER-012 | Two held-out spec generators and run plans for the same decisions; EXP-EVAL-012's 500 h omits planner's 700 h |
| DR1-21 | MEDIUM | Leakage (lineage) | EXP-ECO-011, ECO-013, CON-006 | External-population samples lack the §5.4 item 2 same-upstream check that EXP-LEGACY-010 has |
| DR1-22 | MEDIUM | Decision rule | EXP-LEGACY-021 (DEC-LEG-005, 035, 037, 039, 042, 047, 057, 073, 024) | Evidence-rule classification selects adapter scope outside R0-R10 and cost tiers |
| DR1-23 | MEDIUM | Evidence routes | DEC-ECO-026, 078, 054, 027; DEC-ACC-027; DEC-INT-009 | Uncovered or misrouted decisions (legal FTO, rule simulation, FORMAL routes, unmeasured empirical parts) |
| DR1-24 | MEDIUM | Human/owner routes | DEC-LEG-088, 099, 100; DEC-ACC-004, 047; DEC-MOD-028; DEC-ECO-025; DEC-ECO-018, 033, 034, 039, 048, 052, 053, 056, 059 | HUMAN_PARTICIPANTS and registration decisions lack a human/external route; strategy decisions rest only on a BLOCKED field study |
| DR1-25 | MEDIUM | Representativeness | EXP-PLANNER-002…006 (PSUB-256) | Large-tier size behaviour beyond 256 MiB is unobserved but labelled by parent tier |
| DR1-26 | MEDIUM | Tier coverage | EXP-INT-001 (DEC-INT-002, DEC-CMP-027) | Blast-radius sets have no usable large tier (2 items), so the R4 tier guard cannot be evaluated where chunk size matters |
| DR1-27 | MEDIUM | Power / support | EXP-CRYPTO-005 (DEC-CRY-028) | Four victim contexts per split cannot support the corpus-level "restriction removes the advantage" verdict |
| DR1-28 | LOW | Guard | EXP-REMOTE-001 | `max_loadavg_1m` 4.0 with one pinned vCPU is an undeclared deviation from §4.4 |
| DR1-29 | LOW | Missing specs | 24 experiments (heuristic list in the CSV) | Protocols cite spec files that are neither committed nor marked as generated |
| DR1-30 | LOW | Index metadata | EXP-CODEC-004, CHUNK-001, CHUNK-011, INT-009, CRYPTO-001; DEC-ECO-026 | Timing flag, READY rows with blocked reasons, instrument-only row, blocker class mismatch |
| DR1-31 | LOW | Status quo identity | EXP-ECO-016, 018, 019 | Specs use dev-HEAD `ebound` as the analysis baseline instead of REF-PROD `9e44608` |
| DR1-32 | LOW | Fairness | EXP-EVAL-002 | Size ratios over the intersection of completed items hide Entrybound refusals from the CC rows |
| DR1-33 | LOW | Modelled incumbents | EXP-REMOTE-012 | Structure-derived request plans for squashfs/EROFS/DwarFS are modelled, not measured readers |
| DR1-34 | LOW | Emulation fidelity | EXP-SCALE-014 | `-cpu Icelake-Server` under QEMU TCG cannot expose AVX-512; the x86-64-v4 cell must fail, not degrade |
| DR1-35 | LOW | Method note | EXP-SCALE-006 | M08.3 at n = 3 uses the sample maximum; acceptable as a binary MVT criterion, record for round 2 |

## BLOCKER findings

### DR1-01 (BLOCKER): every design session is L7-exposed for every family; the designs need unexposed re-signing

- **Evidence.**
  - Instructions: the shared Phase C-design rules tell each domain session to read `research/corpus/coverage.md` and `research/PROGRESS.md`. The group table in `coverage.md` lists held-out item identifiers and upstream sources for all 20 families. The PROGRESS change log names held-out upstreams (`decision-method.md` §0.1).
  - Disclosures: the domains record the exposure as follows.
    - Via `coverage.md`, hence every family: codecs CC-13, container conventions §1, integrity C0, reconstruction-common §3, conformance CC1, ecosystem §1, legacy conventions header, platform PCP-1, EXP-CHUNK-001, EXP-CROSSFILE-001, EXP-REMOTE-001, EXP-SCALE-001.
    - Via PROGRESS: planner README §10 (9 families; the README itself says this affects "all planner decisions").
    - Via PROGRESS and the corpus critique: EXP-EVAL-001 (13 families).
    - The crypto domain records nothing, because its generated disclosure block is empty (DR1-07); it read the same inputs.
  - Rule: under §5.4 item 3 and event L7 (§5.7), "a session with a recorded identity exposure may not design candidates or analyses for decisions whose applicable families include the exposed items". Every candidate operationalization, pruning and fitting rule and analysis plan in the 237 protocols was therefore written by a barred session.
  - Requests: codecs CC-13(c), container §1, integrity C0, reconstruction-common §3 and planner README §10 explicitly ask this review, or an unexposed session, to decide the consequence.
  - The index-building session and this review session are exposed in the same way.
- **Why BLOCKER.** §8.2 item 12 and §5.7 make the evidence of any decision whose candidates or analyses came from a barred session non-conforming. Fixing this after results exist would require re-running confirmatory looks.
- **Required fix.**
  1. Append every Phase C-design session (15 domain designers, the index builder, this reviewer, and the revision session if it reads the same inputs) to `research/decisions/l7-exposures.jsonl` with the documents read (§5.8 h).
  2. Change the shared design and analysis rules so that no design, revision or analysis session reads `coverage.md`, the PROGRESS change log or the critique files. Provide a redacted view generated by a corpus-owner session: tuning and validation rows plus held-out **counts per family and group** only (the fields §5.4 allows), for example `research/corpus/coverage-design-view.md`.
  3. Before G-A, a session that has read only the redacted view re-signs each domain's candidate operationalizations, exclusions, fitting and pruning rules and analysis plans (a signed `resign-<domain>.md` listing what was checked and any change). Decision analyses are executed by unexposed sessions only, as several domains already propose.
  4. If the owner prefers not to re-sign, record an owner-accepted deviation under §0.2 stating that the `identity_aware_heldout` cap applies to every decision, and that the sealed tranche (EXP-EVAL-011) is the only route to lift it. This is a weaker alternative. It does not repair §5.4 item 3 for analyses, which must still be done by unexposed sessions.

### DR1-02 (BLOCKER): decision-bearing quantities have no canonical metric, and §0.2 item 4 makes that permanent at the first result

- **Evidence.**
  - Integrity conventions C11 list quantities the integrity decisions need that are absent from Appendix A.2: `damage_detection_fraction`, `parity_repair_success_fraction`, `salvage_recovered_fraction`, `carrier_survival_fraction`, `binding_survival_fraction`, `rename_detection_fraction` (all T-20-like), and HC oracle counts `undetected_injections`, `mvt15a_overclaim_cells`, `mvt15b_offdiagonal_events`, `mvt18a_violations`, `confinement_escapes`. C11 asks the design review to add them "by revision now (no decision-relevant result exists)".
  - EXP-CONF-008 asks for `injected_defect_detection_fraction` (T-20, a = 0.5/n_defects); without it DEC-ECO-028 can be decided only through R1/R2 and R10. Conformance CC9 lists HC MVT oracles without names (MVT-01(b)/(d), MVT-03(a)/(c), MVT-04, MVT-15(b), MVT-16(a)/(d), MVT-17(b)).
  - EXP-CHUNK-003 records that update locality (boundary preservation, new unique bytes, resynchronization distance after edits) "has no banded canonical metric", so locality can act only through multi-version `artifact_bytes`.
  - 48 experiments have no canonical metric in any spec (CSV flag `NO_METRIC_CANONICAL`). Most are legitimately binary or census experiments, but the list should be reviewed against their decisions' OD sets.
- **Why BLOCKER.** R3 declares an OD with no primary metric unmeasured, and an unmeasured OD blocks `DECIDED`. A band added after the first decision-relevant result can never make the metric primary (§0.2 item 4, "late bands"). The fix is only cheap now.
- **Required fix.** Before G-B, a method revision (round-2 method review, §0.2 item 1):
  1. adds the integrity C11 and CONF-008 fractions as T-20 metrics, the HC oracle counts as `binary` metrics with their MVT/HC ids, and a locality metric (for example `edit_new_unique_bytes` with a T-01-consistent band, or a T-20 `boundary_preservation_fraction`), each with objective M-number bindings, re-running the Appendix A.3 consistency check;
  2. or records, per affected decision, that the OD stays unmeasured and the decision cannot become `DECIDED` on that OD.
  - Domains then rename spec metrics to the canonical names or their `__stratum` forms.

## HIGH findings

### DR1-03 (HIGH): no cross-domain validation-look allocation

- **Evidence.** A scripted count (regex `validation[- ]look|spec-validation|validation-looks\.jsonl|confirmatory look` over non-BLOCKED protocols; heuristic) finds 130 protocols planning validation looks and **103 decisions informed by more than two such experiments** (for example DEC-CON-004 by 8, DEC-CMP-026 by 8, DEC-CMP-027 by 7, DEC-CMP-001 and DEC-CMP-017 by 6). §5.2 allows two looks per decision and counts looks of related decisions (shared candidate, primary metric or cluster) against each other. Only the planner domain has a joint look plan (planner README §5). Its step 5 waits until "those domains' records for the same decisions have frozen their W and S", while other domains contain symmetric wait clauses (for example EXP-CHUNK-006, "coordinated with the crypto domain's decision record"), so looks can deadlock. The integrity conventions C13 and conformance CC10 list overlaps and ask the review to assign one confirmatory source per comparison.
- **Required fix.**
  1. Commit `research/decisions/look-plan.csv` (decision id, look number, owning experiment, participating specs, W ∪ S source records, MDE record path) generated from `decision-coverage.csv` and reviewed per cluster, with at most two looks per decision and related decisions grouped.
  2. Make the look registry writer refuse a look not present in the plan.
  3. Adopt integrity C13's suggested ownership splits as the default assignments, and add: DEC-CRY-023 throughput owned by EXP-CHUNK-008 T1 (DR1-17); DEC-CMP-001 size owned by EXP-CHUNK-004 V1, with CRYPTO-003, SCALE-012 and CHUNK-001/002 contributing tuning evidence only.
  4. Replace the planner "wait for other domains" rule with the plan's explicit ordering.

### DR1-04 (HIGH): calibration validity is keyed to an identifier that changes with every commit

- **Evidence.** §4.7: calibration "is repeated whenever `env_id` changes". `research/tools/ebr/envinfo.py` states "env_id covers the repository commit". The Windows fingerprint's stable section also hashes Docker Desktop client, engine and CLI plugin versions, BIOS strings and more. Taken literally, every research checkpoint invalidates all timing calibration (about 80 h per EXP-EVAL-004's own estimate, see DR1-05). Taken loosely, calibration validity is undefined, and a decision-grade gate that depends on it is unauditable (§8.6).
- **Required fix.** A method revision defining **calibration identity** as `platform_id` plus an explicit trigger list: OS build and kernel (Windows build, WSL kernel), microcode and BIOS, power scheme and overlay, Defender platform and engine versions, VBS state, WSL memory and vCPU configuration, research toolchain, and the runner commit for timing hooks. Add a cheap per-campaign drift check (A/A spot-check on 3 items plus the drift canary baseline). Implement the key in `envinfo.py` (`calibration_id`) and have `ebr run --timing` refuse when the calibration record for that key is absent.

### DR1-05 (HIGH): cost-blind program budget (exclusive timing windows and storage)

- **Evidence** (from `index.csv`).
  - Total estimated compute: 23,136.5 h, of which 18,623.5 h is NEEDS_TOOLING and 7,621 h is in 65 `requires_timing` experiments that need exclusive windows (no agents, builds, containers or provisioning; §4.2).
  - At 10 h of exclusive window per night, the timing load alone is about two years. Every OS or WSL update then triggers recalibration (DR1-04), and EXP-EVAL-004's 80 h looks low for its own matrix: 24 items, AA/AAP/4 KE arms/BG/SPD, two environments, 4 WSL and 10 decision-grade Windows affinity configurations, pack of 8 GiB items with cooldown.
  - Summed disk estimates are 9,593 GB. Four experiments individually exceed the storage actually available: EXP-SCALE-001 260 GB, SCALE-006 260 GB, SCALE-007 300 GB, SCALE-008 300 GB. At review time D: had 422 GB free, `disk_guard.py` keeps 75 GB and `disk_alarm.py` alarms below 100 GB.
  - Inside WSL, `df` reports 573 GB available on `/`, which exceeds D:'s physical free space. The VHDX can grow past the host disk, the failure mode of the 2026-09-16 storage incident.
- **Required fix.**
  1. A program execution schedule (`research/orchestration/c-exec-plan.csv`) with priority classes (gates and calibration; deterministic screens that eliminate candidates, §13; confirmatory timing only on W ∪ S), a window budget per month, and a stop rule when the schedule exceeds the program horizon.
  2. Per-experiment storage plans with peak versus retained bytes, a scratch cap enforced by the driver, deletion and `fstrim` of the VHDX between shards (sparse mode is on), and a pre-run check that compares the WSL-visible free space with D:'s physical free space.
  3. Re-estimate EXP-EVAL-004 and EXP-EVAL-012 (DR1-20) bottom-up.

### DR1-06 (HIGH): power feasibility is not gated before the large size campaigns

- **Evidence.** The MDE gate (§4.12) must pass at ≤ 1 band unit before any look, using tuning group-level variance and the validation group structure. A synthetic check with the §4.10/§4.12 formulas uses validation group counts per family ({F01: 3, F02: 4, F03: 3, F04: 3, F05: 6, F06: 4, F07: 6, F08: 5, F09: 4, F10: 2, F11: 3, F12: 2, F13: 4, F14: 3, F15: 3, F16: 3, F17: 3, F18: 3, F19: 5, F20: 7}), equal family weights, one-sided α 0.005 and power 0.80. It gives the following MDE (command: `C:/Python313/python.exe research/experiments/_review/design_review_round1_scan.py --mde`):

  | within-family between-group SD of paired log ratio | T-01 `artifact_bytes` (r = 0.01) | T-04 decode (r = 0.05) | T-06 memory (r = 0.10) |
  |---|---|---|---|
  | 0.02 | 0.88 band units | 0.18 | 0.09 |
  | 0.03 | 1.31 | 0.27 | 0.14 |
  | 0.05 | 2.19 | 0.45 | 0.23 |
  | 0.10 | 4.38 | 0.89 | 0.46 |

  So every T-01 superiority comparison needs within-family heterogeneity of the candidate effect below about 2.3% (log). A cited workload mix with unequal weights makes this stricter. Chunk-size, dictionary, lookback and codec-portfolio effects plausibly vary more than that between groups of F07, F17 and F18. EXP-EVAL-007 Q3 designs this feasibility analysis, but only on incumbent pairs and not as a precondition. EXP-CHUNK-004 (325 h), EXP-PLANNER-006 (590 h), EXP-CODEC-001 (180 h) and EXP-CROSSFILE-001/003/004 (860 h together) would spend their budgets before learning that no look can be taken.
- **Required fix.**
  1. Run EXP-EVAL-007 Q3 first, extended with tuning-only candidate-effect proxies that are already cheap (EXP-CHUNK-001's screen on tuning; EXP-BASE-SIZE incumbent pairs at matched levels).
  2. Make "projected MDE ≤ 1 band unit at the declared scope" an entry gate for each size campaign's full tuning run.
  3. Where it fails, redesign the scope in advance (profile or family partitions under R9 expressibility, more validation groups, or non-inferiority framing where the decision permits), rather than discovering it at the look.

### DR1-07 (HIGH): crypto-domain pre-registration is incomplete, and its specs contradict its protocols

- **Evidence.**
  - All 22 `EXP-CRYPTO-*` protocols contain empty `<!-- BEGIN AUTO:meta -->`, `AUTO:candidates` and `AUTO:common` blocks (CSV flag `AUTO_BLOCKS_EMPTY`). No generator exists in the repository. The protocols therefore lack status, decision ids, the per-decision "Decisions informed" table (candidates per decision, status quo, exclusions with reasons), author/L7 disclosure, gates and shared provisions. Section 3 "Decisions informed" is literally empty, and decision ids exist only in `crypto.jsonl`.
  - Specs versus protocols:
    - EXP-CRYPTO-003's `spec.yaml` has 3 candidates (`gear-norm-v1`, `secret-gear`, `phte-aes128`). Its protocol defines U0, K1, K2, K2s, K3, KV-*, Z1 and end-to-end `ebound pack` arms, plus `spec-windows.yaml` and `spec-mvt14e.yaml`, which do not exist. The spec has no `cpu_affinity`, while the protocol says `st-v`/`mt-v-4`, and it uses `max_loadavg_1m: 3.0`.
    - The timing specs of EXP-CRYPTO-012 and 017 (and BLOCKED 004) likewise lack affinity.
    - EXP-CRYPTO-003 H1 applies `encode_throughput_bps` (T-05b, whole-encode logical bytes per second) to a chunking-stage component, and at the same time calls it "diagnostic".
- **Required fix.** Fill the generated blocks, or replace them with written sections, for all 22 protocols (per-decision candidate tables with status quo, SPEC design, ledger alternatives, incumbent, defer, critic slot and exclusions with rule ids). Align every crypto spec with its protocol arms, affinity and guard (`max_loadavg_1m` = 1.0 + n). Rename component throughput to a `desc_`/`__stage` form so it cannot be mistaken for T-05b. Re-run `ebr validate`.

### DR1-08 (HIGH): crypto construction and leakage-acceptance conclusions can bypass external review

- **Evidence.** EXP-CRYPTO-021's dossier scope is "status `EXTERNAL_REVIEW_REQUIRED`" plus "rows whose `external_review_requirement` is non-empty". A ledger query (crypto cluster, status `INSUFFICIENT_EVIDENCE`, empty `external_review_requirement`, question about leakage, oracles, commitment, key sharing, downgrade, binding or adversaries) returns decisions whose selection is a security claim about a construction or an acceptance of leakage:
  - DEC-CRY-025 (dedup under encryption);
  - DEC-CRY-028 (cross-file context under encryption; EXP-CRYPTO-005 itself says "the claim that a restriction is sufficient is an external-review item");
  - DEC-CRY-030 and DEC-CRY-064 (public bytes and recipient fields);
  - DEC-CRY-034 (suite introduction and downgrade protection);
  - DEC-CRY-045 (recipient-set integrity);
  - DEC-CRY-050 and DEC-CRY-069 (AFK rotation and key-sharing sibling archives);
  - DEC-CRY-060 (what a PCR binding attests);
  - DEC-CRY-072 (remote access-pattern leakage accepted);
  - DEC-CRY-077 (indistinguishable secret-dependent failures);
  - DEC-CRY-089 (suite id in signatures);
  - DEC-CRY-105 (unlock continuation after commitment/MAC failure). The integrity uncovered fragment itself calls this "EXTERNAL: crypto domain dossier with formal-analysis coverage".

  Their evidence routes are internal only (EXP-CRYPTO-005, 007, 008, 010, 015, EXP-REMOTE-010, EXP-CHUNK-006). §8.3 routes "cryptographic constructions … including the effectiveness of padding against leakage attacks" and security proofs to external review, and internal agents are not independent. Conversely, DEC-CRY-039, 049, 053 and 084 are mapped only to the BLOCKED dossier. Their internal evidence (EXP-CRYPTO-002/006/007 for 039, EXP-CRYPTO-014/015 for 084) is not linked in `decision-coverage.csv`. No internal FORMAL preparation (for example a symbolic model of the key hierarchy, commitment and unlock order for DEC-CRY-049) is designed, although §8.3 requires the dossier to contain "the internal evidence under this method".
- **Required fix.**
  1. The independent `decision_type` assignment (§1.2, G-A) marks the security-claim parts of the listed decisions EXTERNAL, and their `external_review_requirement` is filled, so that EXP-CRYPTO-021's scope rule includes them.
  2. Each internal protocol states that attack results are lower bounds, and that "sufficient", "accepted" or "resists" selections are provisional until review.
  3. Link the internal evidence experiments for DEC-CRY-039/049/053/084 in `decision-coverage.csv`, and design an internal FORMAL preparation experiment (committed model files, properties, tool versions, counterexample budget) for the key-hierarchy, commitment, unlock-order and binding questions as dossier input.

### DR1-09 (HIGH): combined-split specs and shared legacy case structure weaken validation independence

- **Evidence.**
  - 38 experiments have at least one spec selecting `splits: [tuning, validation]` (CSV flag `SPEC_MIXES_SPLITS`): 18 legacy experiments (every legacy experiment with a spec), 15 platform experiments (EXP-PLAT-003, 004, 005, 007, 008, 009, 010, 011, 012, 014, 015, 017, 018, 024, 025), EXP-SCALE-003/005/014/015 and EXP-EVAL-001. For HC screens (determinism, fidelity, refusal) running validation is required (§8.2 item 13) and acceptable. But the same pattern carries graded, candidate-selecting comparisons: EXP-LEGACY-033 (export wrapper codec parameters, size and encode cost), LEGACY-032/040 (memory and time strategies), LEGACY-041 (limit defaults), LEGACY-050 (preservation overhead) and EXP-PLAT-015 (timing, 11 candidates, 4 validation items).
    - Legacy sequencing is procedural: validation cases are materialized into `cases-manifest.jsonl` later. The same spec then runs **every** candidate on validation, not W ∪ S.
    - §5.1 says choosing among variants by validation results is tuning, and §5.2 requires the look to be registered with W, S and the MDE gate before execution.
  - Legacy conventions C5: "Structural primitive classes are exhaustive and **identical across splits**; what varies by split is the seeded instantiation". EXP-LEGACY-001's C0 set is "byte-identical … identical in both splits". Generated groups are named `legacy-gen-<generator>-<case_family>`, with no split component, so the same independence group appears in both splits. Corpus methodology forbids that, and it defeats the §5.3 content-overlap audit (≤ 0.01 of logical bytes per family and split pair).
- **Required fix.**
  1. Split every graded legacy, platform and scale spec into a tuning spec and a look spec derived from the tuning record with the candidates restricted to W ∪ S (the container CT-4 pattern). Add a spec lint rule to §14 item 3: a spec whose candidates are compared on banded metrics may not select validation unless it references a registered look.
  2. Exclude C0 (and any byte-identical case) from validation analyses, reporting them as regression checks.
  3. Include the split in generated group ids.
  4. Rest confirmatory legacy verdicts on real-archive tiers plus structural variants authored after tuning by a session not involved in the generators, or label the generated-case validation as seed-robustness evidence only.

### DR1-10 (HIGH): planner timing guards have no tuning variance, so the MDE gate cannot be computed

- **Evidence.**
  - EXP-PLANNER-003: "Guards on tuning use deterministic proxies (work units for encode; declared decode requirements)"; the timing arm is "look 1 only: TS-1-V".
  - EXP-PLANNER-004: "`encode_wall_s` guard; deterministic work units on tuning, timed at look 1".
  - EXP-PLANNER-006: "guard (work units on tuning; timed in EXP-PLANNER-007 look 1)".
  - EXP-PLANNER-007 times TS-1 on tuning only for its own profile configurations, on WSL only. Windows timing starts at look 1.
  - §4.12 computes each confirmatory comparison's MDE from "the tuning-split group-level variance of the same comparison", and non-inferiority guards are part of R4 condition 2.
  - The W ∪ S of 003/004/006 therefore enter look 1 without an MDE for their encode and decode guards, and in the Windows environment without any tuning data. The work-unit proxy is not validated against time: no correlation or calibration arm is registered.
- **Required fix.**
  1. Time W ∪ S of each planner decision on TS-1 (tuning) in both environments before the look, reusing EXP-PLANNER-007 windows.
  2. Add a registered proxy-validation arm (Kendall τ and a fitted ratio between work units and in-process encode time per profile and family) before work units may screen at R2.
  3. Otherwise record that the guard comparisons fail the MDE gate, and that the decisions stay `INSUFFICIENT_EVIDENCE`.

### DR1-11 (HIGH): EXP-EVAL-005 random-access incumbents use a weak reader

- **Evidence.** O5 measures incumbent `random_entry_latency_s` through libarchive 3.7.2 via the `libarchive-c` Python binding over `7z-mx9-nosolid`, `7z-mx9-solid`, `zip-6`, `tarxz-pixz` and `tarzstd-3`, with CPython `zipfile` for ZIP, and squashfs/DwarFS through FUSE. libarchive exposes a forward-only entry iterator, so reading entry N of a non-solid 7z means iterating headers and skipping data from the start. A Python loop adds per-entry interpreter overhead at sub-millisecond scale, and FUSE adds kernel crossings. The objective's CC-01 specialist is "7z non-solid (LZMA2)" for random access, which the format supports by direct seek. H1 ("NON_INFERIOR to `7z-mx9-nosolid` … DISTINGUISHABLE_BETTER than solid 7z") and the resulting SPEC random-access marks would be biased toward Entrybound. The threat is acknowledged ("implementation, not format"), but no best-available reader is included. Secondary issues in the same protocol:
  - O3 compares `ebound verify` (SHA-256 digests) with `zstd -t` (xxhash) and CRC checks, which is a capability mismatch;
  - pipeline incumbents' memory is the per-process maximum (§4.14), not the pipeline sum; this is correct by the method, but it must be labelled when compared with a single `ebound` process.
- **Required fix.** Add in-process native readers that seek: 7-Zip's own decoder through a small C++ harness over `7z.so` (or `7zz` process-level `e` of one entry, reported beside it), `libzip` for ZIP, the pixz index for tar.xz, and libsquashfs or `sqfs` tooling for squashfs. Take the **best** incumbent reader per item as REF-INC, which is conservative for Entrybound, as EXP-EVAL-002 does for size. Add capability-equalized arms (for example tar+zstd followed by a SHA-256 manifest check) beside the native arms for decode and verify claims. Label multi-process memory.

### DR1-12 (HIGH): R0 item 6 critic candidates are open everywhere

- **Evidence.** R0 item 6 requires "at least one candidate proposed by a critic not involved in designing the others" for every decision, checked by a critic pass recorded in the ledger `history`. The protocols leave the slot open and assign it to this review: EXP-CHUNK-001 ("the design review adds it"), EXP-CHUNK-002, EXP-CROSSFILE-001 (`critic-TBD`), EXP-PLANNER-003 (`critic-proposed-n`), EXP-PLANNER-005 (`CD-n`), integrity conventions C0, legacy conventions C11. A single program-level review is not a per-decision critic pass, and this session is L7-exposed (DR1-01).
- **Required fix.** Before G-A, run per-cluster critic sessions (unexposed, independent of the designers). Each reads the ledger candidate set and the protocol candidate tables, proposes at least one candidate per decision or records "no further candidate" with reasons, and writes to the ledger `history`. The protocols add the critic candidates by amendment before execution. Illustrative, unchecked prompts for those sessions:
  - a per-extension static codec table as a planner baseline (DEC-CMP-032/034);
  - size-bucket plus extension grouping without sketches as a similarity baseline (DEC-CMP-006/041);
  - one full-entry range request per entry as a coalescing baseline (DEC-ACC-046);
  - a fixed-size chunk list aligned to filesystem block size for sparse items (DEC-MOD-026).

## MEDIUM findings

### DR1-13 (MEDIUM): READY overstates runnability; decision-linked specs need a gate-state guard

- **Evidence.**
  - Tooling: 39 of 43 READY experiments name repository paths in `tooling_to_build` that do not exist (CSV flag `READY_UNBUILT`), for example 13 scripts for EXP-ECO-012 and 7 for EXP-PLAT-002. Domain vocabularies define READY differently: ecosystem and conformance count "fully specified, mechanical to write" drivers as READY, while others do not.
  - Environment: `qemu-user` (`qemu-aarch64`, user-mode `qemu-x86_64`), `gcc-aarch64-linux-gnu` and `podman` are not installed in WSL (only `qemu-system-x86` 8.2.2 is). EXP-ECO-004 (READY) needs qemu-user. EXP-PLAT-024 and EXP-LEGACY-020 need podman. 16 protocols use qemu-user.
  - Gates: the specs carry non-empty `decision_ids` while G-A is unmet (§0.4: "before any experiment record with non-empty `decision_ids` is committed"). They are labelled drafts, but nothing prevents an orchestrator from launching them.
- **Required fix.** Add `runnable_now` (instruments and packages present) and `gate_state` (G-A/G-B/calibration satisfied) columns to the index, generated by script. Harmonize the status vocabulary. Have `ebr run` refuse specs with non-empty `decision_ids` until the §14 item 3 pre-registration-ancestry check passes, or run them under an explicit `--exploratory` label that decision tooling rejects.

### DR1-14 (MEDIUM): some BLOCKED arms are reachable through nested KVM

- **Evidence.** Inside WSL, `/dev/kvm` exists, `CONFIG_KVM_INTEL=y`, `kvm_intel.nested=Y`, and `qemu-system-x86_64` 8.2.2 is installed. The WSL kernel lacks `CONFIG_NET_SCH_NETEM`, `CONFIG_DM_FLAKEY` and `CONFIG_DM_LOG_WRITES`, but a KVM guest running a stock distribution kernel has all three. No host admin or host configuration change is needed. Not smoke-tested here. The following BLOCKED reasons or arms therefore overstate the block:
  - EXP-INT-018: the Linux filesystem power-loss arm via dm-log-writes/dm-flakey in a guest (NTFS/ReFS stay blocked);
  - EXP-SCALE-016: slow storage via QEMU drive throttling or guest `dm-delay`, and NFS/SMB between guests or guest and WSL;
  - EXP-REMOTE-001: a kernel-TCP arm with real `sch_netem` on a veth pair inside one guest. This is stronger than the TUN relay A3 and directly addresses harness review R1-14;
  - EXP-CON-009: a low-memory x86-64 guest arm (not ARM64);
  - EXP-CON-010: the browser arm can use Edge on the Windows host through non-admin automation (the macOS arm stays blocked).
- **Required fix.** Add guest-based arms with a feasibility smoke and nested-virtualization calibration (timing from a guest inside WSL is not comparable with WSL timing, so label it and calibrate as its own environment). Narrow the BLOCKED reasons to the parts that remain blocked.

### DR1-15 (MEDIUM): calibration does not cover the timing instruments decisions use

- **Evidence.**
  - Known effects: EXP-EVAL-004's KE arms append a busy loop to REF-PROD `ebound` processes. The in-process workload is only `ebr-access` random-entry batches, with no in-process KE. Decisions time in-process instruments with different noise structures: `ebr-chunk throughput` (≥ 1 s batches), `ebr-codec bench`, `ebr-crypto chunk-cost`, `ebr-openers --mode bench`, `ebr-outboard --timing-batch`, `ebr-cdcpack` phase timers and `ebr-recon`. §4.12 uses "the A/A item half-widths at the cap" for item-level MDEs, which should come from the instrument actually used.
  - Speedup pairs: T-14 compares T(1) and T(n) across affinity configurations. ebr's `platform.cpu_affinity` is spec-level, so `st-p` and `mt-p-n` samples come from separate runs, not from "the same round block" (EXP-SCALE-013, EXP-CROSSFILE-008 `spec-speedup.yaml`, EXP-EVAL-004 SPD). Slow drift between runs is not controlled by randomized blocks.
  - WSL placement: WSL `st-v` pins a vCPU, whose host placement can migrate between P-cores and E-cores. The frequency covariate is Windows-only, and no per-sample placement check exists.
- **Required fix.**
  1. Calibrate A/A and KE per instrument class (process, in-process batch, per-operation batch), with an in-process KE injector (a calibrated loop inside the timed region behind a research flag).
  2. Add per-candidate affinity to the runner so that T-14 arms interleave, or alternate short runs ABAB and treat the run pair as the block.
  3. Add a WSL in-sample micro-canary (a fixed loop before and after each sample) whose ratio classifies P-class or E-class placement, and invalidate mixed samples under the §4.4 balance rules.

### DR1-16 (MEDIUM): timed commands read from `/mnt/d`

- **Evidence.** The timing specs of EXP-CODEC-004 (`spec-adversarial`), CON-003, CON-005, ECO-008, EVAL-005 (pilot), PLANNER-003 (`spec-timing`), PLANNER-007, REMOTE-001, REMOTE-006 and SCALE-016 invoke wrapper scripts or helpers under `/mnt/d/Projects/…` inside the timed command (CSV flag `TIMED_CMD_ON_MNT`). §4.1 keeps WSL timing off drvfs/9P for inputs and scratch. 9P reads cross into the Windows host (with Defender scanning of D:), so latency jitter enters process-level `wall_s`, and EXP-CON-003 even runs a Python second implementation inside the timed command.
- **Required fix.** Stage every script and helper to ext4 (`/root/eb-research/tools/<EXP>/`, SHA-256 recorded in meta) before a timing window, and move cross-check implementations outside timed samples. Add this to the spec lint.

### DR1-17 (MEDIUM): harness/production parity gaps and duplicate keyed-chunking measurement

- **Evidence.**
  - EXP-CHUNK-008 arm P compares `ebound pack` with harness `ebr-pack`. The T2 alternatives run through `ebr-cdcpack`, which calls a new research-internals wrapper that injects ranges through `&mut dyn FnMut` (EXP-CHUNK-004 §18). Codegen of that path is not covered by P or by the `gear-norm-v1-copy` control, which covers only chunk-stage T1.
  - T2's primary `encode_cpu_s` is "in-process thread CPU", while T-05 defines it as user+system of the whole process tree. Any worker thread in planning or encoding is missed.
  - EXP-CRYPTO-003 measures keyed-chunking throughput (DEC-CRY-023, DEC-CMP-001) through `ebr-crypto chunk-cost` with no codegen control. EXP-CHUNK-008 T1 measures the same quantities (secret table, PHTE with AES-NI and soft-AES) under different candidate ids (`phte-aes128-norm-v1` versus `phte-aes128-aesni`).
- **Required fix.**
  1. Add a parity cell `ebr-cdcpack` (`gear-norm-v1`, frozen presets) against `ebound pack` for timing.
  2. Measure `encode_cpu_s` as process-tree CPU, or define a new component metric.
  3. Make EXP-CHUNK-008 T1 the single source of chunk-stage throughput, with EXP-CRYPTO-003 keeping MVT-14(e), end-to-end encrypted pack cost and dedup. Unify candidate ids through an alias table in the index.

### DR1-18 (MEDIUM): determinism and HC-14(e) screens lack positive controls or split coverage

- **Evidence.**
  - Positive controls: the determinism experiments (EXP-SCALE-005, 011, 012, 014, CROSSFILE-009, 010, PLANNER-009, LEGACY-031, ECO-004, CHUNK-012) register variation cells (thread counts, affinities, memory caps, locale, creation order, emulated ISAs, a 20-repetition stress cell) but no planted nondeterministic candidate to show that the cells actually perturb what they claim. For example, a worker pool may size itself from `available_parallelism()` read before `taskset` applies, or a copy may preserve directory order. The Appendix D.5 detection bounds assume that the perturbation is effective.
  - HC-14(e) coverage: EXP-CHUNK-006 runs MVT-14(e) for the status-quo keyed modes on a 10-item control subset only, "because the crypto domain owns the full HC-14 verdict", and asks the review to extend it if the crypto pre-registration does not cover tuning **and** validation. EXP-CRYPTO-003 covers "every tuning item of at least 1 MiB" only.
- **Required fix.**
  1. Add a research-only mutant per variation class (thread-completion-order output, `HashMap`-randomized iteration, `read_dir` order dependence, CPU-feature-dependent path), expected to fail its cell. A cell that does not detect its mutant is not evidence.
  2. Extend EXP-CRYPTO-003 `spec-mvt14e` to all validation items of at least 1 MiB and to the conformance corpus, so that EXP-CHUNK-006 can keep its control subset.

### DR1-19 (MEDIUM): no end-to-end leave-one-stage-out ablation of the v6 physical optimizations

- **Evidence.** Stage-level off-arms exist:
  - EXP-RECON-004: generations v3-v6 and v6 minus DEFLATE/JPEG, materialized;
  - EXP-CROSSFILE-003: `no-dictionaries`;
  - EXP-CROSSFILE-004: lookback sweep;
  - EXP-CODEC-004 and EXP-PLANNER-006: `transforms-none`, `dict-none`, `lookback-0`, `xfile-none`, `cluster-none`;
  - EXP-CODEC-006: write-time verification scope.

  Gaps remain:
  - The always-on exact dedup ablation is only modelled (EXP-CHUNK-007 `scope-none`, excluded from selection).
  - Each domain ablates its own stage against a status quo where the others are on, so savings that overlap (dictionaries versus lookback groups versus cohorts versus dedup) can each be credited to their own decision.
  - No single matrix on identical items reports bytes plus decode time, access amplification and memory per removed stage.
- **Required fix.** Add one materialized leave-one-stage-out experiment on REF-PROD (dedup-off via a research writer, cohorts, dictionaries, lookback groups, transforms, DEFLATE and JPEG reconstruction, write-time verification), using the TS-1 and PSUB-256 selections for cost. Produce a marginal-contribution table that the stage decisions cite as counterevidence when their credited gains do not add up.

### DR1-20 (MEDIUM): duplicate held-out placeholders and an underestimated held-out run

- **Evidence.**
  - Two generators: evaluation conventions §3 say that no domain writes held-out specs, that `make_heldout_specs.py` (EXP-EVAL-012) derives all of them at Commit A, and that "R5 for every frozen decision runs once, inside EXP-EVAL-012". EXP-PLANNER-012 instead writes its own held-out specs "by copying each planner experiment's look-1 specs", with its own analysis. Several other domains (for example EXP-CHUNK-004 `spec-heldout.yaml`, codecs CC-12) also say they author held-out specs at Commit A. Two generators for the same decisions risk two held-out executions, which §5.5 forbids ("runs exactly once").
  - Budget: EXP-EVAL-012 lists 33 decisions and 500 h, while EXP-PLANNER-012 alone estimates 700 h, and the validation-look totals of the timing campaigns sum to far more.
- **Required fix.** One generator and one run manifest (EXP-EVAL-012). Domain placeholders become sections that declare their selection rule only. Re-estimate EXP-EVAL-012 from the look plan of DR1-03.

### DR1-21 (MEDIUM): external-population experiments lack the same-upstream lineage check

- **Evidence.** EXP-ECO-011 (registry tail scan over PyPI, npm, crates, Go, RubyGems, Maven, Debian and others; feeds legacy strict-refusal defaults), EXP-ECO-013 (CLI usage census from package sources) and EXP-CON-006 (default resource budgets from ecosystem populations) tune defaults on public populations that include the registries several held-out lineages come from. Only EXP-LEGACY-010 declares the §5.4 item 2 check ("a held-out-aware session records pass/fail per stratum").
- **Required fix.** Declare each population's upstream sources in the record, and add the held-out-aware pass/fail check before the first look (EXP-EVAL-010 `upstream_lineage_check.py`), excluding failing strata from held-out-evidence claims.

### DR1-22 (MEDIUM): EXP-LEGACY-021 classifies adapter scope outside R0-R10

- **Evidence.** EXP-LEGACY-021 applies evidence rules (`prevalence-threshold-rule`, `libarchive-parity`, `spec-tiers`, …) to classify each adapter and export profile as CORE_V1, POST_V1_CORE, ECOSYSTEM or NOT_JUSTIFIED. It feeds DEC-LEG-005, 035, 037, 039, 042, 047, 057 and 073. The protocol never refers to R4-R10, minimum gains or computed tiers. §2.0 makes R0-R10 the only decision procedure; a prevalence threshold is not a selection rule under the method.
- **Required fix.** Restrict the rules to DEC-LEG-024, as candidates for *how to present* v1 scope evidence. Decide each adapter decision through R1-R10, with OD-19/OD-22/OD-24 metrics and cost inputs and adapter candidates (add, defer, plugin-only, refuse) at computed tiers, using the classification only as descriptive context.

### DR1-23 (MEDIUM): uncovered and misrouted decisions

| Decision | Problem | Required fix |
|---|---|---|
| DEC-ECO-026 (chunker freedom to operate) | Legal question; no dossier experiment; ledger `blocker_class` is EVIDENCE | Reclassify as EXTERNAL; add a legal-review dossier experiment (candidate constructions, primary-source publication dates and locators, frozen identifiers at stake) |
| DEC-ECO-078 (selection rule) | Evidence is the §14 item 9 rule simulation, which has no pre-registered experiment | Register an `EXP-METHOD-00x` protocol (scenarios, seeds, acceptance) so that its outputs are pre-registered and citable |
| DEC-ECO-054, DEC-ACC-027 | "Analytical/FORMAL" routes without the counterexample-search protocol §1.2 requires | Add FORMAL protocols (generator, budget, reviewer), or route to owner or EXTERNAL |
| DEC-ECO-027 (sustainability) | Owner resourcing decision; not decidable by this method | Mark it owner-decided outside the method in the ledger, with no `DECIDED` under §8 |
| DEC-INT-009 (encrypted sidecar/incremental post-v1) | Only the BLOCKED dossier; the empirical part "leakage from external plaintext digests" is unmeasured | Add an arm to EXP-CRYPTO-007 or EXP-INT-007 |

### DR1-24 (MEDIUM): human-facing, registration and strategy decisions lack a matching route

- **Evidence.**
  - HUMAN_PARTICIPANTS rows DEC-LEG-088, DEC-LEG-099, DEC-LEG-100, DEC-ACC-004, DEC-ACC-047 and DEC-MOD-028 have only mechanical or BLOCKED-infrastructure experiments, with no EXP-ECO-021 (human) route for their human claims. §1.2 requires those claims to be routed to external or human review, or removed from the decision.
  - DEC-ECO-025 (IANA, libmagic, PRONOM registrations) is an §8.3 external-authority matter but routes only through the EXP-ECO-012 census.
  - DEC-ECO-018, 033, 034, 039, 048, 052, 053, 056 and 059 (adoption wedge, target users, bindings, headline, governance) route only through BLOCKED EXP-ECO-022 with `blocker_class` EVIDENCE or HUMAN_PARTICIPANTS. They are product-strategy choices without an internal empirical route.
- **Required fix.**
  - Add EXP-ECO-021 to the human rows, or strip the human claims.
  - Add EXTERNAL routing for DEC-ECO-025.
  - For the strategy rows, record them as owner decisions informed by the internal censuses, or keep them `INSUFFICIENT_EVIDENCE` with an explicit "no internal route" note that remaining-unknowns can cite.

### DR1-25 (MEDIUM): PSUB-256 sub-items stand in for large-tier planner behaviour

- **Evidence.** Planner README §3: planner size tuning (EXP-PLANNER-002…006) uses PSUB-256 sub-items that "inherit the parent's … scale tier". Candidates whose effect depends on reach beyond 256 MiB never see it: Zstandard window logs 25-27 and long-distance matching, lookback, windowed planning at 64 MiB, cohort and dictionary formation across large trees, and dedup across F17/F18 versions. The R4 tier guard for "large" then evaluates sub-items.
- **Required fix.** Before look 1, materialize W ∪ S on the full large-tier tuning items (at least one per family where available), or restrict the claims to ≤ 256 MiB scopes with the tier guard reported as not evaluable.

### DR1-26 (MEDIUM): blast-radius evidence lacks a usable large tier

- **Evidence.** EXP-INT-001 draws banded verdicts from INT-CORE-T/V (21 items each: 13 small and 8 medium on tuning, 0.43 GiB). INT-LARGE has 2 items per split with 5 configurations. T-15 bands target chunk-size-dependent blast radius from 64 KiB to MiB, where large archives matter, and R4 condition 3 requires the guard in every tier.
- **Required fix.** Add large-tier items (≥ 2 groups per split) to the banded arm for chunk-size-relevant configurations, or scope DEC-INT-002 and the DEC-CMP-027 blast-radius parts to small and medium with the large tier recorded as a limitation.

### DR1-27 (MEDIUM): EXP-CRYPTO-005 has four victim contexts per split

- **Evidence.** EXP-CRYPTO-005 builds victim contexts from 4 tuning items (F04, F01, F19, F06) and 4 validation items. H3 claims that a restriction makes `presence_advantage` `EQUIVALENT` with 0, and that its size cost is `DISTINGUISHABLE_WORSE` on named families. §4.9 requires ≥ 10 groups across ≥ 5 families for a corpus-level verdict, and an `EQUIVALENT` security verdict from 4 groups is not supportable.
- **Required fix.** At least 10 victim contexts from distinct groups across ≥ 5 families per split, or state H3's advantage claim per context as descriptive and leave sufficiency to external review (DR1-08).

## LOW findings

- **DR1-28 (EXP-REMOTE-001).** The spec sets `max_loadavg_1m: 4.0` with `cpu_affinity: [2]`. The protocol justifies 1 + client + 2 server threads, but no §0.2/§4.4 deviation is recorded. Record it, or pin the server processes and count their threads in n.
- **DR1-29 (missing specs).** 24 protocols cite spec files that are neither committed nor worded as generated (CSV flag `SPEC_REF_MISSING`; heuristic): for example EXP-CRYPTO-003 `spec-windows.yaml` and `spec-mvt14e.yaml`, EXP-CHUNK-006 `spec-encrypted.yaml`, EXP-SCALE-007…010 `spec-timing.yaml`, EXP-REMOTE-002 `spec-dense-extreme.yaml` and `spec-encrypted.yaml`. Commit them, or name the generator and selection rule.
- **DR1-30 (index metadata).**
  - EXP-CODEC-004 `requires_timing` is False, but `spec-adversarial.yaml` has `timing: true`.
  - EXP-CHUNK-001, CHUNK-011 and INT-009 are READY with text in `blocked_reason`.
  - EXP-CRYPTO-001 has `decision_count` 0 (an instrument experiment; add a `kind` column).
  - DEC-ECO-026 `blocker_class` EVIDENCE contradicts its legal route.
- **DR1-31 (EXP-ECO-016, 018, 019).** Specs use `ebound-dev-head` as `analysis.baseline` and EXP-ECO-019 calls dev-HEAD the status quo, while R0 item 1 and ecosystem conventions §4 make REF-PROD `9e44608` the status quo. `crates/` has no commits since `9e44608` today (`git rev-list --count 9e44608..HEAD -- crates` = 0), so the builds coincide now. Pin REF-PROD as the baseline and report dev-HEAD beside it.
- **DR1-32 (EXP-EVAL-002).** Ratios use "the intersection of items both sides completed", with the refused share reported beside them. For CC outcome rows a refused item is a capability failure. Count refusals in the CC rows (`NOT_SERVED` for that item's scope), not only as a side note.
- **DR1-33 (EXP-REMOTE-012).** squashfs, EROFS and DwarFS request plans are "structure-derived". Label them `MODELLED` in every table, and do not let them support "better than" claims without a measured reader.
- **DR1-34 (EXP-SCALE-014).** QEMU TCG does not implement AVX-512, so `-cpu Icelake-Server` degrades to a lower feature set. The `cpu_model_verified` check must mark that cell failed, not relabel it, and the x86-64-v4 claim stays unverified.
- **DR1-35 (EXP-SCALE-006).** M08.3 uses 3 runs, and §4.10's exact 0.99 interval needs n ≥ 8. Using the sample maximum is acceptable because M08.3 is a binary MVT criterion with an exact pre-registered bound (objective §2.0 rule 1). Record this reading for the round-2 method review.

## Dispositions of requests addressed to this review by designers

| Request (source) | Disposition |
|---|---|
| Decide the L7 consequence (codecs CC-13(c), container §1, integrity C0, reconstruction-common §3) | DR1-01: re-signing by unexposed sessions before G-A; analyses by unexposed sessions; redacted corpus view. This reviewer is exposed and cannot re-sign. |
| Supply R0 item 6 critic candidates (CHUNK-001/002, CROSSFILE-001, PLANNER-003, integrity C0) | DR1-12: not suppliable by a program-level exposed review; per-cluster unexposed critic passes required. |
| Add canonical metrics (integrity C11, CONF-008, conformance CC9, CHUNK-003) | DR1-02: method revision before G-B, with the names proposed there. |
| Merge overlapping experiments (integrity C13, conformance CC10) | DR1-03 and DR1-17: adopt C13's ownership splits in the look plan; DEC-CRY-023 throughput owned by EXP-CHUNK-008 T1. |
| Extend MVT-14(e) if crypto coverage is tuning-only (EXP-CHUNK-006) | DR1-18: the condition holds; extend EXP-CRYPTO-003 to validation and conformance. |
| M08.3 at n = 3 (EXP-SCALE-006) | DR1-35: acceptable as a binary criterion; record for round 2. |
| Workflow catalogue completeness against program Task 32 (EXP-ECO-006) | Not checkable here either: the program text is not committed. The owner or a session with the program text reconciles it before G-A. |
| Menu-coverage extension (EXP-PLANNER-007 (d)) | Descriptive only until the round-2 method review accepts it, as the protocol already states; no objection. |

## What was checked and found adequate (no finding)

- **Held-out.** No held-out item identifier occurs in any file under `research/experiments/`. Every corpus selector names tuning, validation, generated, conformance or synthetic sets. Every held-out design is a placeholder that references §5.5.
- **Coverage.** Every ledger decision (596) appears in `decision-coverage.csv`. 591 have at least one experiment, and the 5 uncovered ones carry reasons (routes judged in DR1-23). 15 decisions have only BLOCKED experiments (DEC-CRY-039, 049, 053, 084; DEC-ECO-018, 022, 033, 034, 039, 048, 052, 053, 056, 059; DEC-INT-009), addressed in DR1-08, DR1-23 and DR1-24.
- **Remote.** Remote designs model latency from exact logs with slow-start arms (`connection-model.json`), calibrate the emulator before any remote result (EXP-REMOTE-001), and block `DECIDED` on `slow_start_dependent` outcomes, satisfying harness use constraint 7.
- **Modelled evidence.** Modelled candidates cannot select without materialization (planner README §7, EXP-CHUNK-007 §12 item 5, EXP-CHUNK-014, EXP-RECON-004 model gate).
- **Harness use constraints.** Byte identity against production is gated (EXP-CHUNK-004 SQ gate, EXP-RECON-004 identity and generation controls, `lock_parity.py` and `harness-cli-parity.sh` preconditions). R1-16 is respected (GNU-time rows only) and codec rows carry window and decoder memory (EXP-CODEC-002).
- **Platform labels.** `EMULATED`, `VIRTUALIZED` and `PLATFORM_BLOCKED` are applied, and QEMU is never used for timing.
- **Timing controls.** Where timing controls are designed in detail (EXP-CHUNK-008, CODEC-002, EVAL-004, EVAL-005, SCALE-013, PLANNER-007, RECON-005), they follow §4.2-§4.6, apart from the issues above.
- **Blast radius.** Blast-radius oracles are independent of the reader under test, and use region strata with equal weights and worst-stratum guards (integrity C4-C5).

## Reproduction

```sh
# per-experiment metadata flags (all 237 experiments) -> research/experiments/_review/design-review-round1-metadata.csv
C:/Python313/python.exe research/experiments/_review/design_review_round1_scan.py
# synthetic MDE feasibility table of DR1-06 (group counts only; assumed SDs)
C:/Python313/python.exe research/experiments/_review/design_review_round1_scan.py --mde
# coverage counts
C:/Python313/python.exe -c "import csv,collections;r=list(csv.DictReader(open('research/experiments/decision-coverage.csv',encoding='utf-8')));print(len(r),collections.Counter(x['covered'] for x in r))"
# budget sums
C:/Python313/python.exe -c "import csv;r=list(csv.DictReader(open('research/experiments/index.csv',encoding='utf-8')));f=lambda k,c=lambda x:True:sum(float(x[k]) for x in r if c(x));print(f('compute_hours_estimate'),f('compute_hours_estimate',lambda x:x['requires_timing']=='True'),f('disk_gb_estimate'))"
```

The look-pressure count of DR1-03 is a review-session heuristic (regex stated in DR1-03). The ledger queries of DR1-08 and DR1-24 filter `research/decision-ledger.jsonl` by cluster, status, `blocker_class` and `external_review_requirement`. Environment facts in DR1-13 and DR1-14 come from read-only `dpkg -l`, `ls /dev/kvm`, `/sys/module/kvm_intel/parameters/nested` and `/proc/config.gz` inside WSL, and from host drive free space at review time.
