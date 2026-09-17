# EXP-PLANNER-010: Planner provenance (replan, historical generations, encrypted planner IDs, explain and decision logs)

| Field | Value |
|---|---|
| Domain | planner (program §9, §10) |
| Index status | NEEDS_TOOLING (TP-12 `pack-generation`/`replan`, TP-13 `explain_fidelity.py`, TP-14 census scripts, TP-03 `decision-log-size`, TP-15 G1-PSUB-64; decision-grade explain and replan timing needs EXP-PLANNER-007 windows) |
| Document state | Phase C design; `record.md` after gate G-A |
| Shared conventions | `research/planner/README.md` |
| Runner specs | `spec.yaml` (replan matrix); `spec-encrypted.yaml` (encrypted planner-ID mapping); `spec-explain.yaml` (explain fidelity and informational cost) |

## 1. Pre-registration header

- **decision_ids:**
  - DEC-CMP-040: repack planner-version policy.
  - DEC-CMP-025: historical creation APIs after v1.
  - DEC-CMP-022: encrypted planner-ID recording.
  - DEC-CMP-029: persisting planner decisions.
  - DEC-ACC-012: explain from recorded statistics.
  - DEC-INT-046: replan-compare cost and feasibility for `verify --reproducible`.
- **Decision type (proposed):**
  - DEC-CMP-025 and DEC-CMP-022: FORMAL, a census plus an identifier-reuse analysis with a counterexample search.
  - DEC-CMP-040, DEC-CMP-029, DEC-ACC-012, DEC-INT-046: EMPIRICAL for measured parts.
  - Leakage of persisted logs and recorded IDs under encryption: EXTERNAL (crypto review).
  - Explain usefulness to people: HUMAN-FACING (ecosystem §33, simulated, heuristic).
- **Proposed `od_ids`:** OD-20 (deterministic migration), OD-05 (log bytes), OD-23 (C1 words), OD-25 (explain usefulness, routed), OD-15/OD-16 (routed). **Proposed `hc_ids`:** HC-10 (identity change pattern of repack per the SPEC §8.1 table), HC-13, HC-15 (explain never presents unverified or non-recorded history as recorded), HC-16 (no identifier reuse; frozen generations reproducible).
- **Author / L7:** README §10.

## 2. Question and hypotheses

- **Question.** How can archives record, reproduce and explain what the planner did, at what size and maintenance cost?
  - What does replanning produce under each planner-version policy?
  - Which historical creation paths are actually needed?
  - Do encrypted archives lose or collide planner-generation identity?
  - How accurate and costly are recorded-only explanations, and how large would persisted decision logs be?
- **H1.**
  1. `status-quo-current-planner` repack changes PCI for every archive from generations v2-v5, and PCR wherever chunking policy changed. LAI is preserved in every cell (SPEC §17.5).
  2. `pinnable-planner-id` reproduces the source PCI exactly for every generation on the same build.
  3. Historical creation APIs are used outside tests and conformance generation in at most 2 production call sites.
  4. The status-quo `<profile>-enc-v1` mapping records identical planner IDs for at least two distinct frozen generations of the same profile. That is an identifier-reuse counterexample (HC-16).
  5. Recorded-facts-only explanation matches trace ground truth for 100% of plan, codec, dictionary and group facts, and reports rankings as `NOT_RECORDED`.
  6. Explain's baseline for `-v6` IDs mismatches the trace baseline on at least one item.
  7. Persisting full candidate rankings costs more than 1 T-01 band unit of `artifact_bytes` at corpus level; a top-k summary costs less than 1 band unit.
- **H0.** Every policy reproduces, every mapping is unique, explanations agree, and logs are negligible.
- **Falsification.** Each H1.k by its complement: H1.1 by any preserved PCI from v2-v5 or any LAI change (the latter an HC-10 violation artifact); H1.2 by any PCI difference; H1.3 by more than 2 production sites; H1.4 by no collision; H1.5 by any disagreement or a ranking presented as recorded (HC-15 violation artifact); H1.6 by zero mismatches; H1.7 by full rankings within 1 band unit or top-k above it.

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CMP-040 | Cross-version reproducibility of replan output per policy; `artifact_bytes` delta current against source planner; retained planner LoC (maintenance, C2/C7) | Encoder drift across releases: EXP-PLANNER-009; user expectations: ecosystem §33 |
| DEC-CMP-025 | Inventory of internal, harness, test and conformance uses of historical creation APIs; conformance fixture needs (HC-16 MVT-16(c)); fixture bytes if stored | Library API surface review: ecosystem §34; conformance corpus: §29 |
| DEC-CMP-022 | Recorded planner IDs of encrypted archives per source generation and profile; collision and pass-through counts; inspect and explain field census for encrypted archives | Leakage review of recording planner version in encrypted descriptors: crypto (§22.2), EXTERNAL |
| DEC-CMP-029 | Size of decision logs per candidate encoding (full rankings, top-k summary, sidecar) on the traces; audit-record byte share and determinism (from EXP-PLANNER-009 host identity) | Leakage of per-Chunk candidate sizes under encryption: crypto; first-use usefulness: ecosystem §33 |
| DEC-ACC-012 | Explain time and memory against pack (informational, and decision-grade inside EXP-PLANNER-007 windows); accuracy of recorded-only explanations against trace ground truth; v6 baseline recomputation comparison | Remote lazy explain bytes and requests: remote (§12) |
| DEC-INT-046 | Replan-compare feasibility (fraction of archives whose same-build replan reproduces PCR and PCI) and replan work units against pack | Adaptive-flag semantics, release workflow: integrity (§24), ecosystem (§32) |

## 4. Candidates

| Decision | candidate_id (reference first) | Operationalization |
|---|---|---|
| DEC-CMP-040 | status-quo-current-planner; pinnable-planner-id; default-to-source-planner; current-only-with-report | `ebound repack --profile P` (status quo), then TP-12 `replan --policy pinned\|source\|current-report` |
| DEC-CMP-025 | status-quo-public-historical-creation; research-feature-gated; current-only-creation; deprecated-with-warnings | Census rows and fixture-bytes estimates per candidate (TP-14) |
| DEC-CMP-022 | status-quo-profile-enc-v1; versioned-enc-id; separate-crypto-and-planner-fields; refuse-unmapped | The status quo is measured; the others are specified mappings evaluated for collisions over the measured generation × profile set (FORMAL) |
| DEC-CMP-029 | status-quo-audits-only; persist-full-rankings; persist-top-k-summary; recompute-with-planner-id; sidecar-decision-log; drop-audits | TP-03 `decision-log-size` with pre-declared encodings: rankings = per Chunk (shape id varint, complete cost varint) for every evaluated candidate; top-k = per plan (winner shape, runner-up shape, margin varint); sidecar = the same bytes outside the archive; drop-audits = status quo minus audit record bytes |
| DEC-ACC-012 | status-quo-retain-and-reencode; recorded-facts-only; persist-planner-decisions; remote-lazy-explain; fix-v6-baselines | Explain runs per candidate where production supports it; `persist-planner-decisions` is costed through DEC-CMP-029; `remote-lazy-explain` → remote domain |
| DEC-INT-046 | no-reproducible-verify; replan-compare-pcr-pci; record-adaptive-flag; repack-check-reproducible; external-rebuild-tooling | Measured: same-build replan reproduction rate and work units (feasibility inputs only) |

- **Excluded:** none. R0 item 6 critic slots per decision.
- **Expected tiers:** pinnable, source or report flags T2 (user-visible flag); versioned or separate IDs T2 (field or enum), or T4 if the identity participation changes; persisted logs T3 (new section type) or T2 (sidecar schema); feature gating T1; drop-audits T1 with a C7 loss.

## 5. Corpus

- **Replan, encrypted and explain arms:** G1-PSUB-64 tuning sub-items (all 20 families), generations v2-v6 × 4 profiles for source archives.
- **Explain cost arm:** additionally the TS-1 large items, full size, `balanced`. Time and memory are informational outside EXP-PLANNER-007 windows.
- **Decision logs:** EXP-PLANNER-002 PSUB-256 traces (tuning).
- **Census:** repository at the pre-registration commit (`crates/`, `research/harness/`, `tools/`, `tests/`, `docs/`). No corpus.

## 6. Environment and platform

`wsl-ubuntu` x86-64; deterministic metrics. Encrypted arms use generated test recipients (`ebr-pack --encrypt true` convention; passwords in sidecar files never printed). No network. External crypto review for leakage parts. Timing only as informational here.

## 7. Commands

```sh
python3 research/planner/tools/historical_api_census.py --repo . --out research/normalized/EXP-PLANNER-010/census/historical-api.csv
python3 research/planner/tools/planner_surface_loc.py --repo . --out research/normalized/EXP-PLANNER-010/census/planner-loc.csv
for s in spec.yaml spec-encrypted.yaml spec-explain.yaml; do
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-PLANNER-010/$s
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-PLANNER-010/$s --refingerprint
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-PLANNER-010/$s
done
python3 research/planner/tools/replay.py decision-log-size --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces --split tuning \
  --normalized research/normalized/EXP-PLANNER-001 --out research/normalized/EXP-PLANNER-010/decision-logs/
python3 research/planner/tools/explain_fidelity.py --raw research/raw/EXP-PLANNER-010 --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces \
  --out research/normalized/EXP-PLANNER-010/explain/
```

## 8. Seeds, warmup, replication

Seeds: order 260917101, bootstrap 260917102, command 260917103. Deterministic: 0 warmups, 2 repetitions, repeat-hash. Encrypted cells: 2 repetitions. Ciphertext differs by design, so identity checks use the unlocked planner ID, LAI and plan tables, never PCI (objective HC-13 encrypted clause). The census runs once; it is a deterministic script whose output hash is recorded.

## 9. Metrics

| Metric | OD/HC | Role | Family | Source |
|---|---|---|---|---|
| `migration_identity_conformance_fraction` (LAI, PCR and AUX change pattern of repack per SPEC §8.1 and F-08) | HC-10 | binary | correctness | TP-12 |
| `migration_determinism_fraction` (replan output identical across runs) | HC-13 | binary | correctness | TP-12 |
| PCI and PCR reproduction rate against the source archive per policy | OD-20 | descriptive (a policy's promise makes it binary for that policy) | other | TP-12 |
| `artifact_bytes` (current against source planner; decision-log candidates) | OD-05 | primary for DEC-CMP-029 costs and DEC-CMP-040 size delta | size | TP-12, TP-03 |
| recorded planner ID per (generation, profile, encrypted); collision count; unmapped pass-through count | HC-16 | binary (collision = identifier reuse) | correctness | spec-encrypted |
| explain agreement with traces per fact class; `NOT_RECORDED` honesty violations; v6 baseline mismatches | HC-15 | binary (honesty); descriptive (agreement) | correctness/other | TP-13 |
| explain `wall_s`, `max_rss_bytes` against pack | OD-06/OD-08 | informational (decision grade only in EXP-PLANNER-007 windows) | time_wall, memory_peak | runner resource |
| call-site counts, retained planner writer LoC, fixture bytes | C2/C7 | cost inputs | other | TP-14 |
| work units of replan against pack | – | descriptive (DEC-INT-046 feasibility) | other | TP-12 |

## 10. Normalization and statistical analysis

- **Binary and FORMAL parts:**
  - Any HC-10, HC-13, HC-15 or HC-16 violation is an R1 counterexample, committed with a reviewer not involved.
  - DEC-CMP-022 candidates are screened by constructing their mapping over the measured (generation, profile) set; a mapping with a collision is eliminated at R1(b).
  - DEC-CMP-025 is FORMAL: the census, the fixture needs and the C2/C7 costs feed R6 and R10; there is no empirical superiority.
- **DEC-CMP-029:** per candidate, `artifact_bytes` ratio against the status quo with §4.9 aggregation and T-01 verdict classes. A log format costing more than the band is not free, and its benefit claims (explain usefulness) are HUMAN-FACING, routed and never a selection basis here. Selection among logs is therefore by R4 non-inferiority on size plus R10, pending external leakage review (`EXTERNAL_REVIEW_REQUIRED` if a logged-sizes candidate survives under encryption).
- **DEC-CMP-040:** policies are compared on reproduction promise (R1 against the promise), size delta (descriptive; a migration cost M20.3) and C2/C7 maintenance cost (R6, R10).
- **DEC-ACC-012:** agreement fractions are per fact class over a pre-registered case list (every Chunk plan fact, dictionary fact and group fact of every archive). Honesty violations are R1. Cost is informational here.
- **DEC-INT-046:** feasibility numbers are passed to the integrity domain.

## 11. Sensitivity analysis

- DEC-CMP-029: encoding variants (fixed-width against varint) and the family arms of §6.5 on log size.
- DEC-CMP-040: size delta per tier and family.
- No weights are involved.

## 12. Expected negative results worth recording

- Every repack of an older archive silently changes PCI under the status quo (DEC-CMP-040 negative for current-only without a report).
- `-enc-v1` IDs collide across generations (a DEC-CMP-022 defect).
- Explain's v6 baseline mismatches.
- Full rankings are too large to persist.
- Historical creation APIs are used only by tests and conformance generation, which supports feature gating.

## 13. Threats to validity

- The research harness (`PlannerVersion::plan`) stands in for historical creation.
- Encrypted identity checks rely on unlocking with generated test credentials.
- Explain cost measured outside timing windows is informational only.
- Census completeness depends on static patterns (dynamic dispatch through generic planner-version enums is resolved by listing `PlannerVersion` match arms).
- Designer L7 exposure.

## 14. Cost and effort

- **Compute:** source packs over 5 generations × 4 profiles on G1-PSUB-64, about 90 CPU-hours (extreme dominates); replans under 4 policies about 60 CPU-hours; encrypted arm about 20 CPU-hours; explain arm about 10 CPU-hours; decision-log replay about 5 CPU-hours; census minutes.
- **Disk:** source archives transient about 20 GB; raw about 2 GB.
- **Agent effort:** scripted execution; judgment for FORMAL mapping analysis, census interpretation, reviewer coordination.

## 15. Gates and status

- **Tooling:** TP-03, TP-12, TP-13, TP-14, TP-15.
- **Gates:** G-A, G-B; an independent reviewer for FORMAL exclusions; external crypto review for leakage parts; EXP-PLANNER-007 windows for decision-grade explain and replan cost.
- **Held-out:** EXP-PLANNER-012 re-runs the replan identity checks on held-out sub-items (HC tests on every split, §8.2 item 13).
