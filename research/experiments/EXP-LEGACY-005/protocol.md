# EXP-LEGACY-005: differential fuzzing of compatibility rule models against their pinned runtimes (ZIP profiles; tar and 7z model prototypes)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (`fuzz/zipmut.py`, `fuzz/tarmut.py`, `fuzz/sevenzmut.py`, `fuzz/campaign.py` with behavioural-signature feedback, `fuzz/minimize.py`; long-lived runtime servers for JVM and Python anchors; `ebr-legacy-import` in-process compat projection; tar/7z model prototypes from EXP-LEGACY-002/003) |
| Domain / program sections | legacy / §25 (feeds §29 fuzz-derived regressions and §30) |
| decision_ids | DEC-LEG-001, DEC-LEG-063, DEC-LEG-078, DEC-LEG-081, DEC-LEG-082, DEC-LEG-087, DEC-ECO-047 |
| Decision types (proposed) | EMPIRICAL |
| OD / HC (proposed) | OD-19 (M19.2), OD-24 (M24.2 crash findings), OD-04; HC-03/HC-06 (crashes, hangs), HC-07 |
| Depends on | EXP-LEGACY-001 (tuning matrix and seed cases), EXP-LEGACY-002/003 (tuning matrices for model prototypes) |
| Shared rules / L7 / gates | C§ header, C§2 |
| Timing | `timing: false` (budgets are CPU-seconds per shard; no timing metric is analysed) |

## 1. Question

Do the frozen ZIP compatibility profiles generalise beyond the cases they were derived from — how often does a
profile's projection disagree with its anchor runtime on structure-aware mutated inputs, how often does an input's
conflict pattern fall outside the observed matrix — and what agreement and build effort do first tar and 7z rule-model
prototypes achieve?

## 2. Hypotheses

- **H1 (ZIP generalisation, DEC-LEG-001/081).** On the validation fuzz corpus, each frozen profile's
  `import_agreement_fraction` with its anchor is below 1 (at least one disagreement not attributable to the runtime
  under the triage rule). *Falsified by* agreement 1 for a profile with at least 10,000 retained distinct-signature
  inputs.
- **H2 (unmatched patterns).** At least 5% of retained validation inputs carry a conflict-pattern signature absent from
  the EXP-LEGACY-001 tuning matrix `[INFERRED]`, so `matrix-lookup-refuse-unmatched` would refuse them. *Falsified by*
  a lower share.
- **H3 (content selection, DEC-LEG-078).** The campaign finds at least one content-selection disagreement class beyond
  C11 (for example truncation or concatenation behaviour on new size/descriptor combinations).
- **H4 (tar/7z models, DEC-LEG-001/063).** A tar rule model of GNU tar 1.35 built from the EXP-LEGACY-002 tuning matrix
  in at most 40 engineer-agent hours agrees with GNU tar on ≥ 0.99 of validation fuzz inputs; the libarchive model
  and the 7-Zip 23.01 7z model agree on less. *Falsified by* the GNU tar model below 0.99.
- **H5 (safety, HC-03/HC-06).** No Entrybound import (strict or compat) crashes, panics, hangs beyond 60 s or exceeds
  declared memory on any input. *Falsified by* one such finding (a defect, not a trade-off).
- **H0.** Profiles and models agree with their runtimes on every generated input.

## 3. Decisions informed

| Decision | Supplied here | Remaining elsewhere |
|---|---|---|
| DEC-LEG-001 | Agreement of profiles and model prototypes on fuzzed inputs; unmatched-pattern frequency; model build effort | Tail frequency of unmatched patterns in real corpora EXP-LEGACY-010 |
| DEC-LEG-063 | Model feasibility and agreement for GNU tar and libarchive tar profiles; per-runtime disagreement signatures | Refusal rates showing need EXP-LEGACY-010 |
| DEC-LEG-078 | New content-selection disagreement classes, minimized | Security review of candidates |
| DEC-LEG-081 | Differential fuzzing disagreements per CPU-hour per profile; contradiction handling evidence | Matrix replay EXP-LEGACY-001 |
| DEC-LEG-082 | Replay of the retained fuzz corpus through candidate runtimes (Go, .NET, 7-Zip, Info-ZIP, zip2, Commons): distinguishing signatures per runtime | Pipeline gatekeeping evidence EXP-LEGACY-021 |
| DEC-LEG-087 | Minimized fuzz-derived regression tier for the unified matrix | — |
| DEC-ECO-047 | Planted-divergence positive controls for the campaign detector | — |

## 4. Candidates

| Decision | Candidates | Treatment |
|---|---|---|
| DEC-LEG-001 | status-quo-hand-rule-tables (frozen profiles as implemented); matrix-lookup-refuse-unmatched (rule applied to observations); rule-model-fuzz-validated (research model refined on the tuning campaign only, scored on validation); narrow-runtime-set (agreement and effort with a subset of profiles); runtime-in-the-loop; source-derived-models | `runtime-in-the-loop` is a proposed L0/R1 exclusion (freeze: production never executes a legacy runtime, `docs/zip-compatibility-profiles-v1.md` Status; `docs/7z-import-v1.md` L116-117); it is still computed as the agreement upper bound (1 by construction) for reference. `source-derived-models` is a proposed exclusion (SPEC §14.3 L1636: models derive from the matrix, not source reading); not built. |
| DEC-LEG-063 | gnu-tar-profile; libarchive-profile; python-tarfile-profile; consensus-profile; status-quo-strict-only; defer-post-v1 | GNU tar and libarchive models built; Python and consensus models built only if the first two meet H4 within budget |
| DEC-LEG-078, DEC-LEG-081, DEC-LEG-082, DEC-LEG-087 | as EXP-LEGACY-001 §4 | fuzz-derived evidence tier |

## 5. Corpus selectors

- **Seed pools.** Tuning pool: EXP-LEGACY-001 C0-C14 tuning cases, EXP-LEGACY-002 T0-T13 tuning cases,
  EXP-LEGACY-003 S1-S9 tuning cases, and ≤ 200 small archives sampled (seed `2509170501`) from
  `f11-pypi-binary-wheels-cp312`, `f14-gen-nested-archives-py`, `f20-tuning-real-libarchive-testsuite`. Validation pool:
  the validation instantiations of the same generators plus ≤ 200 archives from `f11-maven-central-jvm-jars`,
  `f14-gen-office-documents`, `f20-validation-real-commons-compress-testdata` (seed `2509170502`).
- **Retained corpus.** Each campaign retains inputs with a novel behavioural signature (Entrybound conflict classes ×
  runtime outcome class × profile agreement bit); the retained validation corpus is frozen as the pre-registered case
  list for T-20 comparisons between candidates.
- **Planted controls.** 50 inputs per format with a known divergence (from EXP-LEGACY-001/002/003 tuning divergences)
  are injected blind into the stream; detector recall must be 1 (C§9).
- **Held-out:** placeholder (C§5): a post-freeze campaign from a sealed RNG seed.

## 6. Environment

`wsl-ubuntu`; Linux builds of the anchors (`python-zipfile-3.13.5-linux`, `java-zip-21.0.12-linux`,
`libarchive-3.8.8-linux`) run as long-lived batch servers inside bubblewrap; a 1% random sample of retained inputs is
replayed on the Windows anchors through interop to check that Linux anchors stand in for them (H1 of EXP-LEGACY-001).
32 vCPUs, one campaign shard per vCPU.

## 7. Commands and tooling

Tooling (`research/tools/legacy/fuzz/`): `zipmut.py` (typed-field mutations of LFH/CD/EOCD/ZIP64/descriptor, extras
insertion/deletion/duplication, name bytes, method/flag/size/offset/CRC with consistency-preserving and -breaking
variants), `tarmut.py` (header fields, pax records, GNU L/K, sparse maps, block alignment), `sevenzmut.py` (property
ids, folder graphs, sizes, attributes), `campaign.py` (shard runner; behavioural-signature novelty feedback; per-input
60 s and 4 GiB limits; crash capture), `minimize.py` (delta debugging preserving the disagreement signature),
`server_py.py`/`ServerJava.java` (batch anchors); `research/tools/legacy/models/{tar_model.py, sevenz_model.py}`
with an effort log `research/experiments/EXP-LEGACY-005/effort-log.jsonl` (UTC start/stop, agent session, lines).

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-005 --seed-pools-from EXP-LEGACY-001,EXP-LEGACY-002,EXP-LEGACY-003 --append-corpus-items research/experiments/EXP-LEGACY-005/real-items.txt
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-005/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-005/spec.yaml --refingerprint
/root/eb-research/venv/bin/python research/tools/legacy/fuzz/minimize.py --experiment EXP-LEGACY-005 --split tuning --out /root/eb-research/raw-artifacts/EXP-LEGACY-005/minimized/
/root/eb-research/venv/bin/python research/tools/legacy/fuzz/replay.py --experiment EXP-LEGACY-005 --retained validation --runtimes go-archive-latest-linux,dotnet-9-linux,7zip-23.01-linux,infozip-unzip-6.00-linux,rust-zip2-tar-crates,commons-compress-1.28.0 --out research/normalized/EXP-LEGACY-005/replay/
/root/eb-research/venv/bin/python research/tools/legacy/matrix.py --experiment EXP-LEGACY-005 --fuzz --out research/normalized/EXP-LEGACY-005/matrix/
```

Order: the tuning campaign runs, model prototypes are refined on tuning disagreements only, rule tables and models are
frozen in the record, then the validation campaign runs once (look registry).

## 8. Seeds

`order 2509170511`, `bootstrap 2509170512`, `command 2509170513`; shard RNG seed = SHA-256(`command seed` ‖ candidate ‖
item_id ‖ rep) truncated to 64 bits (derived inside `campaign.py`); pool sampling seeds §5.

## 9. Warmup, replication, adaptive rule

Each (candidate, pool) runs 24 shards of 1800 CPU-seconds (12 CPU-hours) per split. No adaptive extension. Retained
inputs are re-executed twice in fresh processes; a non-reproducing disagreement is recorded as runtime
nondeterminism.

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `import_agreement_fraction` (profile or model vs anchor, over the frozen retained validation corpus) | T-20 | OD-19 | T-20 |
| `unresolved_fuzz_findings_at_coverage_target` (Entrybound crashes, panics, >60 s, >declared memory) | binary | OD-24 | §3.3 (any finding is a defect) |
| disagreements per CPU-hour; unmatched-signature share with Clopper-Pearson 95% interval; distinguishing signatures per candidate runtime; model lines and effort hours; planted-control recall | non-canonical descriptive | — | none |

## 11. Normalization

Signatures computed from normalized observation tuples (C§6, C§7); inputs deduplicated by SHA-256.

## 12. Statistical analysis

Candidates are compared on the same frozen retained validation corpus with exact counts (T-20). Because retention is
feedback-driven, agreement fractions describe the retained corpus, not the mutation distribution; unmatched-signature
shares are additionally reported over a uniformly sampled (non-retained) 10,000-input stream per profile with exact
binomial intervals. Crash findings are binary R1/R2 evidence with the input bytes as artifacts.

## 13. Practical significance

T-20 `a = 0.5/n_cases`; binary §3.3; descriptive statistics carry no band (C§8).

## 14. Sensitivity analysis

Leave-one-mutation-operator-out (agreement recomputed without inputs whose derivation used that operator); seed-pool
arm (generated-only versus real-only pools); anchor platform arm (Linux anchors vs the 1% Windows replay); budget arm
(first 6 versus all 12 CPU-hours per split, showing whether discovery saturated).

## 15. Expected negative results worth recording

Frozen profiles disagreeing with their anchors on fuzzed inputs (new profile ids or safety overrides needed); tar or 7z
models infeasible within budget (argues for status-quo-strict-only or defer); no crash found (regression evidence only,
no bound claimed).

## 16. Threats to validity

Behavioural-signature feedback biases retention toward disagreement-rich regions; Linux anchors may not equal Windows
anchors (sampled check); mutation operators encode designer assumptions (operator ablation reported); model effort
measured on agents does not transfer to human engineers (labelled).

## 17. Platform requirements

Linux x86-64 (WSL2, 32 vCPUs), Windows anchors via interop for the replay sample; no privileged operations; storage
~30 GB (retained corpora and minimized cases).

## 18. Estimates

Compute ≈ 190 CPU-hours (ZIP 4 profiles × 2 splits × 12 h; tar 2 models × 2 × 12 h; 7z 1 model × 2 × 12 h; replay ≈ 10 h),
about 8 wall hours on 24 shards in parallel per stage; disk ≈ 30 GB. Agent effort: model prototypes and triage are
judgment (effort logged); campaign execution none.

## 19. Expected cost tiers `[INFERRED]`

A new tar or 7z compat profile: T2 each plus recurring matrix maintenance (C7); refuse-unmatched gating: T2 (new reason
code); runtime-in-the-loop: excluded by freeze.
