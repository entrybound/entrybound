# EXP-REMOTE-011: Dependency-closure rules, lookback and dictionary placement — remote random-read cost

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T1 `layout`/`trace`, T5 `fetchsim` closure-rule and dictionary-fetch strategies, `closuresim.py` group-structure transformer) |
| Domain / program section | remote / §12 (§8.4 dictionaries, §8.5 lookback) |
| decision_ids | DEC-ACC-025, DEC-CMP-027, DEC-CMP-020 |
| Decision type (provisional) | EMPIRICAL (remote/access legs); FORMAL for declaration semantics (DEC-ACC-025) and planner-ID impact (HC-16) |
| OD / HC (provisional until G-A) | OD-10 (T-09 access granularity; M10.2), OD-12 (M12.1-M12.3), OD-05 guard (supplied by the crossfile domain); HC-09 (closures bounded, backward-only, within declaration; MVT-09(a),(b)), HC-06 (declared bounds), HC-16 (frozen planner IDs: rule changes need new IDs) |
| Author / L7 / gates | As EXP-REMOTE-001. Consumes EXP-REMOTE-002 status-quo closure measurements (H3 there) and, when committed, archives produced by the crossfile domain's lookback and dictionary sweeps (§8.4-§8.5). This experiment supplies only the access and remote legs; the ratio legs and the joint Pareto analysis belong to the crossfile domain's decision records. |

## 1. Question

For ChunkGroup dependency rules (declared lookback vs transitive closure, group-length caps, periodic independent members, non-transitive prefixes), lookback values and topologies (0/1/2/4/8/16, leader-only delta, restarts, bounded solid blocks), and dictionary placement and fetch rules, what worst-case and typical decoded bytes, fetched bytes, requests and modelled latency does a random or remote read of one entry incur, and which declaration rule never understates it?

## 2. Hypotheses

- **H1 (DEC-ACC-025).** Under the status quo, `access_granularity_bytes` for the last member of a length-L group grows with L (transitive chained prefixes), not with the registered lookback k; `non-transitive-prefix` and `cap-group-length-16` bound it by (k + 1) and 16 Chunks respectively and are `DISTINGUISHABLE_BETTER` on T-09 and on `remote_random_entry_latency_s` at NP-CONT/NP-INTER for extreme archives with groups longer than 16; `declare-transitive-closure` changes no bytes and makes `declared_cost_accuracy` = 1.
- **H2 (DEC-CMP-027, access leg).** Remote first-entry and random-entry latency increase monotonically with lookback for chained topologies; `leader-only-delta` keeps worst-case closure at 2 Chunks for every lookback; `bounded-solid-blocks` has worst-case access equal to the block size.
- **H3 (DEC-CMP-020, access leg).** The status-quo reader fetches the whole Dictionaries section on the first entry read (`random.rs:635-644`), so archives with many dictionaries pay dictionary bytes unrelated to the entry; a referenced-dictionary-only fetch removes that cost, and the expected remote fetch bytes per referencing read is the quantity a `decoder-and-access-cost-charged` acceptance rule must charge.
- **H0 / falsification.** H1 falsified if status-quo worst-member granularity does not grow with L beyond k + 1, or if the bounded rules are not better at groups > 16; H2 falsified by a non-monotone latency-vs-lookback curve for chained topologies; H3 falsified if first-read dictionary bytes do not depend on dictionary count.

## 3. Decisions informed

| Decision | Supplied here | Remaining |
|---|---|---|
| DEC-ACC-025 (V1_BLOCKING) | Measured dependency Chunk count, decoded bytes and latency for last members of 8/64/128-Chunk groups (dense/extreme) under each rule; declaration accuracy per rule | Ratio impact of caps and resets: crossfile domain (§8.5); planner-version impact (frozen planner IDs): FORMAL (HC-16) |
| DEC-CMP-027 (V1_BLOCKING) | First-byte latency local (this experiment's local timing arm) and under remote profiles; worst-case and mean bytes and Chunks decoded per random read for lookback 0, 1, 2, 4, 8, 16 and each topology (the access axis of the SPEC §25.3 L2572 curve) | Ratio gain axis, parallel decode throughput loss, corruption propagation, ratio gap vs 7z/xz solid, zstd --long and DwarFS, prefix window vs decoder memory, dictionaries vs lookback: crossfile, scale and integrity domains |
| DEC-CMP-020 | Extra bytes and requests for random and remote reads caused by dictionaries per policy; expected fetch bytes per referencing read (input to the acceptance-rule candidate) | Net savings per dictionary, break-even reuse, trainer comparisons, decoder memory and decode time: crossfile and codecs domains |

## 4. Candidates

### 4.1 Closure rules (DEC-ACC-025)

| candidate_id | Definition | Realization |
|---|---|---|
| `status-quo-declare-max-lookback` (reference) | declare `max_lookback`/`max_preceding_bytes`; decode transitively | production archives (dense, extreme) and traces |
| `declare-transitive-closure` | declare worst transitive closure Chunks/bytes per group; decode unchanged | declarations recomputed by `closuresim.py`; fetch identical to status quo |
| `cap-group-length-<L>`, L ∈ {8, 16, 64} | groups split so the transitive closure ≤ L Chunks | `closuresim.py` regroups the status-quo physical order (frame sizes held fixed, label `size-held-fixed`) or crossfile-domain archives when committed |
| `periodic-independent-members-<k>`, k ∈ {4, 8, 16} | every k-th member coded without prefix | as above |
| `non-transitive-prefix` | prefixes built from raw predecessor plaintext only; closure = lookback | as above |
| `iterative-bounded-decoder` | semantics unchanged; recursion and complexity fixed in the decoder | fetch identical to status quo; local decode CPU and stack depth are scale/conformance matters (reported as unmeasured here) |

### 4.2 Lookback values and topologies (DEC-CMP-027, access leg)

`status-quo-profile-lookbacks` (reference: fast/balanced 0; dense {1, 2, 4}; extreme {1, 2, 4, 8}), `lookback-zero-everywhere`, `larger-prefix-window` (2-8 MiB: affects decoded bytes per dependency, not closure count), `zstd-ldm-long-window` (same closure as its group rule; window affects decoder memory only), `leader-only-delta`, `group-restart-interval` (= `periodic-independent-members`), `bounded-solid-blocks` (block sizes 4, 16, 64 MiB), `per-category-lookback` (closure per category from the crossfile domain's category map), `lookback-16-plus`. `unbounded-solid` is a **proposed L0 exclusion**: SPEC §2.3 L111 rejects unbounded solid (ledger hard constraint of DEC-CMP-027) and it is unboundable under R2 (random-access decoded bytes have no bound computable before payload); its worst-case access equals the whole CHUNK_DATA and is reported only as a counterexample value for the independent reviewer.

### 4.3 Dictionary fetch (DEC-CMP-020, access leg)

Policies are the crossfile domain's candidates (`status-quo-absolute-128`, `relative-plus-absolute-margin`, `decoder-and-access-cost-charged`, `minimum-reuse-count`, size and sample sweeps, `cover-fastcover-trainer`, `raw-content-dictionary`, `supplied-dictionary`, `per-category-or-archive-dictionary`, `no-dictionaries`); this experiment measures each committed policy archive under two reader fetch rules: `status-quo-whole-dictionaries-section` (reference) and `referenced-dictionaries-only` (designer-proposed reader rule, T0-T1: fetch only the dictionary records referenced by the plans of the requested closure; realization `fetchsim`, record boundaries from the layout map). Until crossfile archives exist, the production balanced/dense/extreme archives (whatever dictionaries the status quo trains) are measured.

## 5. Corpus selectors

- Tuning `dense-plain` and `extreme-plain` archives (small + medium) and `balanced-plain` (all tuning) from EXP-REMOTE-002; entries grouped by the length of their longest group (bins 1, 2-8, 9-64, 65-128, > 128).
- Crossfile-domain archives: by SHA-256 from that domain's committed raw outputs (tuning split only), listed in `external-archives.jsonl` when available.
- Worst-member workload `WL-WORST` (defined here, deterministic): per archive, the 16 regular files whose recorded closure (from the layout map) is largest, ties by path; plus WL-FIRST, WL-RAND1 and WL-RANDK16 from `workloads.json`.
- Validation look for W vs S; held-out: section 15.

## 6. Environment and platform requirements

`wsl-ubuntu`; deterministic computation (no timing) over layouts and traces; local first-byte in-process latency for status-quo lookbacks is measured by this experiment's own generated `spec-local-timing.yaml` (`ebr-remote trace --source local-file`, in-process batches of ≥ 1,000 reads, quiet window, `st-v`, VIRTUALIZED). Linux x86-64; no privileges.

**Platform matrix.** Linux x86-64 required (WSL2); Windows: not used; macOS: not applicable; ARM64: not required; network: loopback; privileged: no; large storage: no; external review: no.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound; PY=/root/eb-research/venv/bin/python; export PYTHONPATH=research/tools
python research/orchestration/disk_guard.py
$PY -m ebr run --spec research/experiments/EXP-REMOTE-011/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-REMOTE-011/spec.yaml && $PY -m ebr normalize --spec research/experiments/EXP-REMOTE-011/spec.yaml
$PY research/tools/remote/closuresim.py sweep --rules research/experiments/EXP-REMOTE-011/rules.json --layouts /root/eb-research/cache/remote-archives \
    --external research/experiments/EXP-REMOTE-011/external-archives.jsonl --out /root/eb-research/raw-detail/EXP-REMOTE-011/layouts
$PY research/tools/remote/fetchsim.py sweep --strategies research/experiments/EXP-REMOTE-011/strategies.json --layouts /root/eb-research/raw-detail/EXP-REMOTE-011/layouts \
    --workloads research/experiments/EXP-REMOTE-002/workloads.json,research/experiments/EXP-REMOTE-011/wl-worst.json \
    --connection-model research/experiments/EXP-REMOTE-001/connection-model.json --out research/raw/EXP-REMOTE-011/sim/
$PY -m decide analyze --experiment EXP-REMOTE-011
```

## 8. Seeds, warmups, repetitions

Seeds `order` 20261017, `bootstrap` 20261018, `command` 20261019. Deterministic: 2 repetitions + repeat-hash on traces and simulated layouts.

## 9. Metrics

| Metric | OD | Role | Threshold |
|---|---|---|---|
| `access_granularity_bytes` (worst-case decoded bytes to read one byte, per entry; AGG-W per archive) | OD-10 | **primary** (DEC-ACC-025, DEC-CMP-027 access legs) | T-09 |
| `remote_random_entry_latency_s` (WL-WORST and WL-RAND1, modelled) | OD-12 | **primary** | T-10 |
| `http_bytes` per read, with dictionary bytes separated | OD-12 | secondary; the DEC-CMP-020 access-leg quantity | T-11 |
| `declared_cost_accuracy` per rule | OD-10 | binary (HC-09) | §3.3 |
| `declared_tightness_ratio` per rule | OD-17 | secondary | T-23 |
| `access_amplification` per stratum | OD-10 | secondary | T-22 |
| `http_requests`, dependency Chunk count | OD-12 | secondary / diagnostic | T-12 |
| `first_entry_latency_s` (local, in-process, status-quo lookbacks) | OD-07 | secondary | T-08 |

## 10. Normalization and statistical analysis

- Item = archive; worst-case metrics AGG-W per item, typical metrics as medians over entries; L2-L4 with `workload-mix.json`; strata per profile config (dense, extreme), group-length bin, network profile.
- Tuning W/S per decision's access leg; validation W vs S per §4.11-§4.12. Because the ratio axis is supplied by the crossfile domain, R4 here is evaluated on the access and remote primaries only and the results are handed to the crossfile decision records as their OD-10/OD-12 primaries (one per OD), where the joint R4 with `artifact_bytes` is run; `size-held-fixed` simulated values are marked non-decision-grade for any candidate whose real archive exists.
- Tiers (independent assessment): declaration semantic changes T2 (field meaning) or T1 (report-only); new closure rules and topologies require new planner IDs (T1 with permanent-vector cost) and, where decoders change (non-transitive prefix, bounded solid blocks), T2-T3; reader dictionary fetch rule T0.

## 11. Practical significance

T-09, T-10, T-11, T-22, T-23, T-12, T-08 by reference; binaries §3.3.

## 12. Sensitivity analysis

Slow-start and connection-model arms; coalescing policy arm (status quo vs EXP-REMOTE-003 W) since dependency prefetch is coalesced; `size-held-fixed` vs real crossfile archives where both exist (reports the error of the simulation); §6.5 family arms restricted to families with dense/extreme archives.

## 13. Expected negative results worth recording

- Status-quo declarations understate transitive decode on long groups (HC-09 defect, not a relaxation).
- Lookback gains that the crossfile domain measures in ratio cost multiple band units of remote random latency at NP-INTER.
- Whole-section dictionary fetch adds dictionary bytes to every first read regardless of use.

## 14. Threats to validity

- Simulated closure rules on status-quo frame sizes ignore ratio changes (flagged; replaced by real archives when available).
- Worst-member workload is adversarially chosen; typical reads may not show the effect.
- Dictionary counts in status-quo archives may be low, weakening H3 until crossfile archives exist.

## 15. Held-out (Phase D placeholder)

Frozen W and S access legs rerun once on held-out dense/extreme archives (and frozen crossfile candidates' held-out archives) after Commit C (`decision-method.md` §5.5), report-only per §5.6.

## 16. Estimates

- Compute: about 3 machine-hours (simulation over cached layouts; no new packing unless crossfile archives are imported).
- Disk: under 5 GB.
- Agent effort: scripted; judgment for coordination with the crossfile domain and analysis.
