# EXP-SCALE-007: Bounded-memory creation prototypes (pack pipeline, INDEXED writer, incremental STREAM producer)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** |
| Domain | scale (program §13; interfaces with chunking §8.1, planner §9, container §11, model §7) |
| Platforms | Linux x86-64 (WSL2, root, cgroup-v1); Windows host for the timing phase's second environment |
| Requires timing | true (phase T only; phases M and S are deterministic or memory measurements) |
| Compute estimate | about 80 machine-hours (phase M about 35 h, phase S about 20 h, phase T about 25 h in quiet windows) |
| Disk estimate | about 300 GB peak (64 GiB inputs with spill and outputs; 1e8-record synthetic sort files about 30 GB) |
| Agent effort | judgment (prototype construction and equivalence arguments); scripted execution |
| Tooling to build | new research-only harness crate `research/harness/crates/ebr-scale-writer` (binary `ebr-scale-writer`), using `entrybound` public API plus default-off `research-internals` wrappers for chunking, planning and frame encoding; designs listed under Candidates; shared counting allocator `ebr-alloc` (EXP-CONF-007); `research/tools/scale/mutate_during_pack.py` (seeded source mutator); runner additions of `decision-method.md` §14 item 3 and calibration §4.7 for phase T |

## Pre-registration

- **decision_ids:** DEC-ACC-029, DEC-ACC-020, DEC-ACC-058, DEC-ACC-061, DEC-CON-027, DEC-MOD-004, DEC-ACC-056, DEC-CMP-017, DEC-CMP-041.
- **Decision type:** EMPIRICAL (memory, size, identity, time); candidates adding wire records or feature bits (trailer amendment, superset declarations) also carry FORMAL specification parts outside this experiment.
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-08, OD-09 (M09.1, M09.4), OD-05 (`artifact_bytes` loss of windowed planning), OD-06 (phase T); HC-06, HC-07 (MVT-07(d) unstable sources), HC-13 (deterministic output of every prototype), HC-02, HC-09.
- **Method, gate, author, L7:** as EXP-SCALE-001. Candidate construction here is design work; per §5.4 item 3 the candidate set needs the R0 item 6 critic candidate and sign-off by a session without L7 exposure before G-A.

## Question

Which creation architectures let `pack`, INDEXED writing and STREAM production handle trees larger than RAM, in bounded memory, with deterministic output, and at what cost in scratch, compressed size, identity equality with the in-memory reference, and throughput; how much memory and time does canonical ordering need at 10^7 to 10^8 entries under each path comparator; how should an incremental producer handle sources that change after frames were emitted; and which pre-payload declarations can a single-pass producer make?

## Hypotheses

- **H1a:** `streaming-cdc-spill` with global planning over digests and statistics produces PCI byte-identical to the status quo for fast and balanced (no cohorts or dictionaries) with M08.3-bounded allocator growth on the bytes axis and O(unique chunks) growth on the entries axis.
- **H1b:** `windowed-planner` (dense, extreme) loses at most 1 band unit of `artifact_bytes` (T-01) versus global planning at window 1 GiB on at least 2 of F01, F03, F05, F06, F18 tuning families, and more on F17 (duplicate trees whose duplicates are far apart).
- **H1c:** `indexed-footer-last` produces PCI identical to the status quo for every profile with bounded memory; `spill-then-copy` is identical with one archive of temporary disk.
- **H1d:** an `incremental-stream-producer` with `external-sort-canonical-order` needs O(1) allocator memory in entries and external-sort scratch of about 100 B per entry; `length-first` (status quo) and `bytewise` comparators cost the same within 1 band unit of T-03 at 10^8 entries.
- **H1e (source mutation):** `plan-before-emit` never emits an entry mixing versions; `orphan-frames` produces archives whose canonical re-encode differs (HC-03 violation for the orphan design); `mark-unstable` and `abort` satisfy HC-07 MVT-07(d).
- **H0 (each):** the prototype's output differs from the reference where identity is claimed, or its growth exceeds the M08.3 tolerance, or its size loss exceeds 1 band unit.
- **Falsification:** as H0, per candidate; identity and boundedness are binary (one counterexample); size loss uses the T-01 band at corpus level on tuning (exploratory) then validation (confirmatory, one registered look).

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-ACC-029 | Peak memory vs input size per profile under caps for prototypes; ratio/dedup loss of windowed vs global planning on corpus families; byte-identical output of streaming designs vs the in-memory reference; two-pass source-stability behaviour under concurrent modification | cross-platform identity of prototypes: EXP-SCALE-014 |
| DEC-ACC-020 | Peak RSS vs archive size for writer prototypes; byte identity with the current writer; interaction with preamble declarations; encrypted segment emission is EXP-SCALE-010 | — |
| DEC-ACC-058 | Writer peak and throughput vs size per producer prototype; canonical-order sorting memory/time at 10^7-10^8 entries; pipe-to-pipe tar parity benchmark vs `tar | zstd -T`; correctness under source mutation | — |
| DEC-ACC-061 | Mutation-during-pack tests per strategy; canonicality of orphan frames; staging cost of buffer-whole-file | identity analysis of orphan frames (formal part): container domain review |
| DEC-CON-027 | Prototype incremental producer with superset declarations; mutating-tree packs to pipes; which declarations are knowable (enumerated by construction and recorded) | security review of superset and sentinel declarations: external |
| DEC-MOD-004 | STREAM producer memory and scratch to emit canonical order for each comparator candidate at large entry counts | diff/lookup locality and EBCS equivalence: model domain |
| DEC-ACC-056 | Window and retention distributions per frame order (digest, path, dedup-aware, group-locality) and time to first object | — |
| DEC-CMP-017 | External on-disk dedup index memory and CPU at 4M+ chunks; windowed dedup loss | per-scope net bytes: crossfile domain |
| DEC-CMP-041 | Windowed similarity/cohort formation beyond RAM: recall loss vs global, CPU and memory | similarity method choice: crossfile domain |

## Candidates

All prototypes are research-only (F-24), never compiled into production crates.

| candidate | Definition | Identity claim tested |
|---|---|---|
| `status-quo-in-memory` | production `ebound pack` (reference) | — |
| `streaming-cdc-spill` | incremental gear-norm-v1 CDC over file streams; chunk plaintext spilled to a content-addressed spill directory; planning over digests and statistics; encoding reads spill | PCI = reference for fast/balanced; LAI/PCR/AUX = reference for all profiles |
| `windowed-planner-<w>` for w in {64 MiB, 256 MiB, 1 GiB} | cohort, dictionary and similarity planning restricted to deterministic windows of w plaintext bytes in canonical order (new planner id required if adopted) | LAI/PCR/AUX = reference; PCI may differ |
| `external-dedup-index` | exact chunk dedup via sharded on-disk digest table with external sort | PCI = reference |
| `two-pass-plan-then-encode` | pass 1 digests/statistics, pass 2 re-reads sources and encodes; refuses if a source digest changed between passes | PCI = reference on stable sources |
| `profile-scoped-bound` | streaming CDC for fast/balanced, status quo for dense/extreme | as `streaming-cdc-spill` for fast/balanced |
| `declared-ceiling-refusal` | status quo plus a pre-capture typed refusal when the input's logical size exceeds a caller ceiling (stat-only preflight) | PCI = reference below the ceiling |
| `indexed-footer-last` | write CHUNK_DATA progressively, then Manifest, Index, footer; preamble declarations computed in a statistics pre-pass | PCI = reference |
| `spill-then-copy` | encode to a temporary file then rename | PCI = reference |
| `stream-then-repack` | produce STREAM, then bounded repack to INDEXED (EXP-SCALE-008 repack prototype) | LAI/PCR/AUX = reference |
| `incremental-stream-producer` | entry iterator API; bounded dedup window; pending state spilled | LAI = reference; PCI may differ (window) |
| `undeclared-budget-pipe` | unseekable input mode emitting `BudgetDeclared = false` with per-object emission | LAI = reference |
| `external-sort-canonical-order-<cmp>` for cmp in {length-first (status quo), bytewise, flat-index, encoded-byte, depth-first, hash} | external merge sort of synthetic path records, 64 MiB run size | order equals an in-memory sort of the same records (binary) |
| `tar-transcoder` | `tar` stream on stdin to STREAM on stdout without materializing files | LAI = reference of the extracted tree |
| `order-<o>` for o in {digest (status quo), path, dedup-aware, group-locality} | STREAM frame order variants | LAI/PCR/AUX = reference |
| `mutation-<s>` for s in {plan-before-emit, buffer-whole-file, orphan-frames, mark-unstable, abort} | incremental producer source-change strategies under `mutate_during_pack.py` | HC-07 MVT-07(d) and HC-03 canonical re-encode |
| `declare-<d>` for d in {conservative-superset, per-item-undeclared-sentinels, spool-then-emit, refuse-unknowable, trailer-amendment} | pre-payload declaration strategies of the incremental producer | production reader outcome on the produced bytes (OK or clean refusal); declared vs actual tightness |
| `inc-tar-zstd` | `tar -cf - | zstd -T1 -3` and `-T0` (REF-INC, phase T) | — |

- **Excluded:** `status-quo-undocumented` variants beyond the reference; any candidate changing decode semantics of frozen identifiers (L0, HC-16); append-in-place (SPEC §22.5 disposition 5, refused).
- **Critic candidate (R0 item 6):** to be added by the independent critic before G-A.

## Corpus selectors

- **Phase M (memory, identity):** generated EXP-SCALE-001 shapes `b1g`, `b4g`, `b16g`, `b64g`, `n1e5`, `n999k`, `d16g`; raised-policy `n4m`; synthetic path-record files of 10^7 and 10^8 records (seeds 1711, 1712; no filesystem entries).
- **Phase S (size loss, tuning):** tuning items of F01, F03, F05, F06, F17, F18 (manifest `research/corpus/manifest.json`, split `tuning`), all scales; **validation confirmation** of the same families in one registered look after W and S are named.
- **Phase T (throughput):** `b4g`, `b16g` and tuning medium items of F01 and F05.
- **Held-out placeholder (Phase D):** if any prototype becomes a provisional selection, its phase S and T comparisons are re-run once on held-out F01/F03/F05/F06/F17/F18 items after unlock (§5.5), and phase M on sealed generator seeds.

## Environment

As EXP-SCALE-001 for phase M (caps 24 GiB and 2 GiB, loop-mounted spill, allocator oracle). Phase S deterministic on WSL. Phase T: `timing: true`, WSL `st-v` and Windows `st-p` affinity (§4.2, §4.3), guard `max_loadavg_1m` = 2.0, `max_wait_s` 300, calibration experiments `EXP-CAL-AA-wsl-ubuntu`, `EXP-CAL-KE-wsl-ubuntu`, `EXP-CAL-AA-windows-host`, `EXP-CAL-AAP-windows-host` passing for families time_wall and time_cpu (gate §14 items 3-4).

## Commands

```sh
wsl.exe -d Ubuntu -- bash research/harness/build.sh build --release -p ebr-scale-writer --features ebr-alloc
python research/harness/lock_parity.py && bash research/harness/internals-identity-check.sh
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-SCALE-007/spec.yaml        # phase M
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-SCALE-007/spec-size.yaml   # phase S (to be generated once W/S rules below are fixed; tuning)
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --timing --spec research/experiments/EXP-SCALE-007/spec-timing.yaml  # phase T, quiet window
```

Binary contract: `ebr-scale-writer --design <candidate> --cell <id> --cells research/experiments/EXP-SCALE-007/cells.json --profile <p> --spill-dir <dir> --out <file|-> --seed <n> --experiment-id EXP-SCALE-007 --env-name wsl-ubuntu` prints one `ebr.harness.raw.v1` row; every output is verified with production `ebound verify` and compared with the reference identities by the binary itself (`identity_*` fields).

## Seeds

`seeds.order` 1307001, `seeds.bootstrap` 1307002, `seeds.command` 1307003; mutation schedules derive from the per-sample seed.

## Warmup

Phase M and S: 0. Phase T: §4.5 (2 warmups small/medium, 1 large).

## Measurement method and metrics

| Metric | Canonical name | Role | Family |
|---|---|---|---|
| `alloc_peak_bytes`, `bounded_memory_growth_bytes` | T-06; M08.3 | primary (graded vs status quo); binary claim | memory_peak; correctness |
| `peak_rss_bytes` | T-06 | secondary | memory_peak |
| `scratch_write_bytes`, `scratch_peak_bytes` | §4.14; T-07 | primary cost of boundedness | correctness; scratch_peak |
| `artifact_bytes` | T-01 | primary for windowed designs (phase S) | size |
| `identity_pci_equal`, `identity_lai_equal`, `identity_pcr_equal`, `identity_aux_equal` | HC-13 / repeat-hash | binary | correctness |
| `canonical_reencode_equal` | HC-03 MVT-03(a) | binary | correctness |
| `unstable_source_declared_ok` | HC-07 MVT-07(d) | binary | correctness |
| `declared_tightness_ratio` | T-23 | graded (declaration strategies) | other |
| `reader_outcome` | none | binary (OK or clean typed refusal) | correctness |
| `sort_alloc_peak_bytes`, `sort_scratch_bytes`, `sort_cpu_s` | T-06; T-07; T-05 | primary for DEC-MOD-004 comparator costs | memory_peak; scratch_peak; time_cpu |
| `required_window`, `retention_bound_bytes`, `ttfo_s` (time to first object, in-process) | none; none; T-08 in-process | descriptive; primary for DEC-ACC-056 in phase T | other; size; time_wall |
| `encode_wall_s`, `encode_cpu_s`, `encode_throughput_bps` | T-03, T-05, T-05b | primary (phase T) | time_wall, time_cpu, throughput |

## Replication and adaptive rule

Phase M: 3 runs per size (M08.3), repeat-hash across runs. Phase S: 2 repetitions (deterministic). Phase T: §4.6 (10/10/30, 10/10/20, 10/5/20; precision-based adaptive rule with half-width target 0.5 × log1p(r)). No other adaptive step.

## Normalization

Phase S: per item, log ratio of `artifact_bytes` of each windowed candidate to `status-quo-in-memory` (same profile); aggregated L1-L4 per §4.9 with the cited workload mix when it exists (gate G-B), equal weights as the sensitivity arm. Phase T: paired per-round log ratios vs the status quo.

## Statistical analysis

R3/R4 on tuning (exploratory) to name W and S per decision; MDE gate (§4.12) before the single validation look; confirmatory comparisons W vs each s at corpus level on primary metrics (T-01 for DEC-ACC-029 size loss; T-06/T-07 for memory and scratch; T-03 for throughput) with §4.10 intervals, §4.11 verdicts, §4.12 confidences; family harm guard (§4.9); cost tiers computed per §7.2 (a new planner id is T1; `indexed-footer-last` with unchanged bytes is T0; superset declarations with new fields T2 or higher); R6 minimum gains; binary claims decided by counterexample.

## Practical-significance thresholds

Referenced by ID: T-01, T-03, T-05, T-05b, T-06, T-07, T-08, T-23; M08.3; minimum-gain bands per §7.3 and Appendix D.6. None restated.

## Sensitivity analysis

§6 in full for any decision reaching R8/R9 (weight grid and Dirichlet over the decision's ODs, threshold ×0.5/×2, leave-one-family-out, leave-three-out, family Dirichlet, WM-* mixes, tier-stratified, byte-weighted, environment arm WSL vs Windows for phase T, selection-stability bootstrap).

## Expected negative results worth recording

Windowed planning losing more than the T1 minimum gain on F17/F18 (dense and extreme cannot be bounded without a real size cost); external dedup index slower than the in-memory map by several bands; incremental producer unable to declare decode requirements without spooling; `orphan-frames` breaking canonicality; bounded writers slower than `tar | zstd -T0`.

## Threats to validity

1. Prototypes built from research-internals wrappers may not match a production implementation's constant factors; memory boundedness (binary) transfers, throughput is prototype-specific (reported as such).
2. Synthetic path records ignore filesystem traversal cost for 10^8-entry ordering.
3. Size loss depends on the corpus mix (gate G-B workload mix); WM-* arms report dependence.
4. Mutation timing races are seeded but not exhaustive (regression evidence).
5. Phase T timing is `VIRTUALIZED` on WSL (soft cap) and needs same-direction Windows support.

## Platform requirements

Linux x86-64 WSL2 root; Windows host (phase T second environment); about 300 GB; runner additions and calibration for phase T; no network.

## Estimated compute and effort

About 80 h after tooling. Tooling: judgment (the largest prototype effort in the scale domain).

## Deviations (append-only)

None.
