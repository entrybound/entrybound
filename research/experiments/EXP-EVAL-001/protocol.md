# EXP-EVAL-001: Incumbent baseline completion (size, determinism, round-trip; timing disabled)

| Field | Value |
|---|---|
| Domain | evaluation (program sections 6, 7, 34, 35) |
| Status | READY (specs generated and validated; detached runner written; analysis script to be written before analysis, not before execution) |
| Kind | Descriptive incumbent-only baseline pass. Under `decision-method.md` §0.1 incumbent-only baseline runs are **not decision experiments**, so the ebr specs carry `decision_ids: []` and no gate of §0.4 blocks execution. Decisions consume these outputs only through the decision-relevant experiments named below. |
| Continues | `EXP-BASE-SIZE` (`research/baselines/README.md`): small tier 2,240/2,240 samples complete (run `size-small-20260917T033541Z`); medium sample (20 items) running as `size-medium-20260917T033541Z` |
| Method | `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` |
| Design pre-registration | this file plus `spec.yaml`, `shards/*.yaml`, `make_specs.py`, `run_completion.sh`, committed before any EXP-EVAL-001 run |
| `record.md` (§12.2) | written at C-exec only if a decision experiment re-uses these rows as a candidate arm; not required for a descriptive pass |

## Author, independence and exposure

- Designed by the Phase C-design evaluation-domain session (workflow `wf_47347bc4-611`). This session designs no candidates.
- **L7 identity exposure (recorded for the §5.8(h) audit).** This session read `research/PROGRESS.md`, whose change log names upstream sources of held-out items (the §0.1(iii) exposure), and a `grep` of `research/corpus/critique-round2.md` displayed held-out item identifiers. Together these cover families F01, F04, F07, F08, F11, F12, F13, F14, F15, F16, F17, F19 and F20. No held-out content, listing, statistic or path under the held-out data roots was opened. Identifiers are not repeated in any file this session wrote. Consequence (§5.4 item 3): analyses of decision-relevant comparisons designed here are executed by sessions without that exposure.

## Question

For every pinned incumbent configuration in `research/baselines/configs.json`, what are the artifact size, the output determinism across two repetitions and the content round-trip result on **every** tuning and validation item of corpus `ebrc-2026.09-v1` (manifest_sha256 `ed28b1b4...`) that `EXP-BASE-SIZE` has not covered?

## Hypotheses and falsification

Descriptive pass; the expectations are pre-registered so that surprises are recorded as findings.

- **E1 (round-trip).** Every configuration except `zpaq-m5` reports `roundtrip_ok = 1` on every item where its extractor ran. `zpaq-m5` fails exactly on items that contain symlinks (documented limitation). *Falsified by* any content mismatch for another configuration, or a `zpaq-m5` failure on a symlink-free item. Each counts as a finding and is written to `research/findings/`.
- **E2 (determinism).** Single-threaded tar pipelines (`targz-*-t1`, `targz-gzip-*`, `tarzstd-*-t1`, `tarxz-6`, `tarxz-9e`, `tarlz4-1`, `tarbrotli-q11`), `zip-*`, `7z-*`, `squashfs-*`, `wim-*`, `dwarfs-*` and `zpaq-m5` produce one `artifact_tree_sha256` per (config, item). `borg-repo` and `restic-repo` do not, because repository ids and keys are random. Multi-threaded `pigz`/`zstd -T0`/`pixz` are expected deterministic, but that expectation is `INFERRED`. *Falsified by* a non-singleton digest set for a configuration expected to be deterministic, or a singleton set for borg/restic.
- **E3 (tier stability).** Per-family median `artifact_bytes/input_bytes` of each configuration on medium and large items lies within a factor of 1.5 of its small-tier value, except F17/F18, where long-window configurations are expected to gain on larger items. *Falsified by* any other family outside that factor. That result is reported as tier dependence and feeds EXP-EVAL-007's tier-stratified arm.

## Decisions informed (through downstream experiments)

`DEC-ECO-031` (pinned-version incumbent baselines on shared corpora; best named configuration per row), `DEC-ECO-077` (pilot rankings for weighting sensitivity, EXP-EVAL-007), `DEC-CMP-011` and `DEC-ECO-070` (incumbent side of density claims, EXP-EVAL-002/012), `DEC-ECO-069` (specialist baselines for CC outcomes). The ebr specs carry `decision_ids: []` (see Kind).

## Candidates (incumbent configurations)

All 28 configurations of `configs.json` / `run_baselines.CONFIGS`, pinned in `tool-versions.json`: `zip-6`, `zip-9`, `7z-zip-9`, `targz-gzip-6`, `targz-gzip-9`, `targz-pigz-9-t1`, `targz-pigz-9-tN`, `tarzstd-3-t1`, `tarzstd-19-t1`, `tarzstd-19-tN`, `tarzstd-22-ultra-long27-t1`, `tarxz-6`, `tarxz-9e`, `tarxz-pixz`, `tarlz4-1`, `tarbrotli-q11`, `7z-mx5-solid`, `7z-mx9-solid`, `7z-mx9-nosolid`, `squashfs-zstd`, `squashfs-xz`, `wim-lzx`, `wim-lzms-solid`, `borg-repo`, `restic-repo`, `dwarfs-l7`, `dwarfs-l9`, `zpaq-m5`.

- **Status quo.** The `EXP-BASE-SIZE` spec definitions are reused verbatim. Only the experiment id, the round-trip detail-log path and raw compression differ.
- **Pre-registered exclusions (machine-time caps, not correctness judgements).** The excluded cells are reported as `NOT_RUN` with these reasons and are never imputed.
  - `zpaq-m5`, `tarbrotli-q11` and `tarxz-9e` are excluded on medium items above 128 MiB (`medhi`). All three are single-threaded and ran at no better than a few MB/s in the medium sample. Ratio-specialist coverage on those items is kept by `tarxz-6`, `7z-mx9-solid` and `tarzstd-22-ultra-long27-t1`.
  - On large items only 13 configurations run: `zip-9`, `targz-gzip-6`, `tarzstd-3-t1`, `tarzstd-19-tN`, `tarzstd-22-ultra-long27-t1`, `tarxz-pixz`, `tarlz4-1`, `7z-mx9-solid`, `7z-mx9-nosolid`, `squashfs-zstd`, `dwarfs-l7`, `borg-repo`, `restic-repo`. These are one ubiquity default, one speed, one long-window dedup, one indexed xz, both 7z specialists of objective §1.3, one mountable image, one dedup filesystem and both dedup backup repositories. The other 15 are excluded because they are dominated in role by a kept configuration of the same family (zip-6, 7z-zip-9, gzip-9, both pigz, zstd-19-t1), or because of the same machine-time cap (xz-6, xz-9e, brotli-q11, zpaq-m5), or because they are redundant for the large tier (7z-mx5-solid, squashfs-xz, wim-lzx, wim-lzms-solid, dwarfs-l9).
- **Not in this experiment.** `zstd --patch-from` needs (base, target) pairs and runs in EXP-EVAL-002. Encryption and signature configurations run in EXP-EVAL-006. Timing runs in EXP-EVAL-005.

## Corpus selectors

Generated by `make_specs.py` from `research/corpus/manifest.json` and the two `EXP-BASE-SIZE` coverage files. Splits: `tuning`, `validation` only (`run_baselines.select_items` filters on `split`). Item lists live in the specs and in `*.coverage.json`.

| Spec (experiment_id) | Items | Item bytes | Configs | Samples (x2 reps) |
|---|---|---|---|---|
| `EXP-EVAL-001` (`spec.yaml`), small items not in `EXP-BASE-SIZE-small` | 19 | 106,881,127 | 28 | 1,064 |
| `EXP-EVAL-001-medlo-s0..s3`, remaining medium items of at most 128 MiB | 45 | 3,095,796,945 | 28 | 2,520 |
| `EXP-EVAL-001-medhi-s0..s3`, remaining medium items above 128 MiB | 51 | 15,420,165,711 | 25 | 2,550 |
| `EXP-EVAL-001-large-s0..s3`, all large items | 28 | 44,345,264,473 | 13 | 728 |

The byte totals are copied from the generator's stdout (the command in "Commands"). Shards are balanced greedily by bytes.

## Environment

`wsl-ubuntu` (WSL2 Ubuntu 24.04, root, ext4 under `/root/eb-research`). Items are read from `/root/eb-research/corpus`; scratch is `/root/eb-research/scratch/ebr`. Raw rows are written under `research/raw/<experiment_id>/`. Tools are the apt packages and the DwarFS static binary pinned in `research/baselines/tool-versions.json`. Before launch, `research/orchestration/disk_guard.py` must pass (standing decision: never consume C:).

## Commands

```sh
# 1. (done at design time; deterministic, re-runnable) generate + validate specs
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && PYTHONPATH=research/tools:research/baselines /root/eb-research/venv/bin/python research/experiments/EXP-EVAL-001/make_specs.py --shards 4'
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-EVAL-001/spec.yaml'
# 2. execution (detached; waits for EXP-BASE-SIZE medium sample's size-pass.done marker)
C:/Python313/python.exe research/orchestration/disk_guard.py
wsl.exe -d Ubuntu -- bash -lc 'setsid nohup bash /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-EVAL-001/run_completion.sh >/dev/null 2>&1 &'
# per spec, run_completion.sh executes: ebr validate; ebr run --gzip; ebr verify-raw --refingerprint --allow-incomplete; ebr normalize --include-incomplete
# 3. analysis (script TO BE WRITTEN before analysis): research/baselines/merge_size_pass.py
#    inputs: research/normalized/EXP-BASE-SIZE/, research/normalized/EXP-EVAL-001*/
#    outputs: research/normalized/EXP-EVAL-001/merged/{samples.csv,item_config.csv,determinism.csv,roundtrip_failures.csv,best_incumbent_per_item.csv}
#    then: research/baselines/build_capability_matrix.py (deterministic_output rows regenerated from merged determinism.csv)
```

Within a group, shards run in parallel, which is valid only because `timing: false`. Groups run one after another. Before each shard starts, the runner requires at least 100-120 GiB free on the WSL root and at least 8-12 GiB MemAvailable.

## Seeds

`seeds.order = seeds.bootstrap = 20260917` in every spec (randomized-block order). Generation is deterministic: the manifest and coverage files fix the items, and greedy byte balancing fixes the shard assignment.

## Warmup

`warmups: 0`. No warmup is needed for deterministic byte metrics (§4.5 applies to timing and memory).

## Measurement method and metrics

| ebr metric (spec) | Canonical metric (Appendix A.2) | Role here | Source |
|---|---|---|---|
| `output_bytes` | `artifact_bytes` (T-01); `total_stored_bytes` for repository configs | descriptive reference data | `path_size` of `{scratch}/artifact.bin` or repository directory |
| `artifact_tree_sha256`, `artifact_sha256`, `artifact_logical_tree_sha256` | repeat-hash check (§4.13) | determinism verdict per (config, item) | `stdout_json` from `probes/finalize_artifact.py` |
| `roundtrip_ok` | `roundtrip_mismatches` (HC-02 oracle, content equality) | binary finding | `check` via `probes/roundtrip_check.py` |
| `ratio` | none (descriptive) | descriptive | derived `output_bytes / input_bytes` |
| `input_bytes` | none | normalizer | `input` |

Logical metadata equality (`logical_tree_sha256`) is recorded per sample in `research/raw/<id>/roundtrip-detail.jsonl`. It is informational, because incumbents are not expected to carry every metadata field. Graded fidelity belongs to EXP-EVAL-006.

## Replication and adaptive rule

Two repetitions per (config, item), in different rounds and processes (§4.6 deterministic metrics), with no adaptive extension. If the repeat-hash check fails for a configuration expected to be deterministic, that is a finding: the two outputs are kept via `--keep-scratch` in a targeted re-run of that one cell, and the configuration's byte metric is treated as stochastic in downstream use (§4.13, "otherwise reclassified").

## Normalization

`ebr normalize` per spec. `merge_size_pass.py` concatenates `EXP-BASE-SIZE` and every `EXP-EVAL-001*` normalized sample table, keyed by (config, item_id, repetition). For a (config, item) present in both, it keeps the earlier run and records the duplicate. It then derives per-item values: exact bytes when the repeat-hash is singleton, otherwise the median of the two repetitions, flagged.

## Statistical analysis

Descriptive only; no verdicts. Per (config, family, scale tier): AGG-G geometric mean of `artifact_bytes/input_bytes` with equal group weights within the family (objective §3.0), the worst family, the round-trip failure count and the determinism verdict. `best_incumbent_per_item.csv` lists, per item, the smallest content-round-tripping configuration overall, per capability class (random-access capable: 7z non-solid, zip, squashfs, dwarfs, pixz; streaming: tar pipelines; dedup: borg, restic, dwarfs, zstd long) and per objective §1.3 specialist role. EXP-EVAL-002 uses it as REF-INC.

## Practical-significance thresholds

None applied: the pass is descriptive. Any downstream comparison uses T-01 (`artifact_bytes`, `r` = 0.01; 64 B floor on medium and large, none on small) exactly as `decision-method.md` §3.2 and `thresholds.json` `metric_thresholds.artifact_bytes` define it.

## Sensitivity analysis

Not applicable to a descriptive pass. The byte-weighted versus group-weighted aggregation contrast is computed and passed to EXP-EVAL-007.

## Expected negative results worth recording

- `zpaq-m5` symlink loss, and every other content round-trip failure, go into `roundtrip_failures.csv` and, when new, into `research/findings/`.
- Non-determinism of multi-threaded incumbents (`zstd -T0`, `pigz -p N`, `pixz`, `7z` multithreaded LZMA2), if observed.
- Configurations that time out on large items. A timeout is a result (`NOT_COMPLETED`), not a missing value.

## Threats to validity

- **Build and tool versions.** Ubuntu apt versions (`tool-versions.json`) may differ from upstream latest. Claims cite the pinned version only.
- **Exclusion caps** remove three slow ratio-oriented configurations from items above 128 MiB. Ratio upper bounds on those items are therefore conservative for incumbents, and the reports say so.
- **Page-cache and extent-map effects** on fingerprints are fixed (`posix_fadvise` DONTNEED, PROGRESS 2026-09-17). `verify-raw --refingerprint` re-checks item identity.
- **Parallel shards** share CPU and disk. Byte outputs of deterministic configurations cannot depend on load. Multi-threaded configurations whose output depends on thread scheduling would surface as E2 falsifications, and that is itself the finding.
- **Capability mismatch.** Sizes are for different capability sets: zpaq drops symlinks, tar pipelines have no index, and repositories are directories. Fair-comparison rules 1-5 of `research/baselines/README.md` apply to every downstream use.

## Platform requirements

Linux x86-64 (WSL2), no privileges beyond WSL root, no network, large storage (about 60 GiB peak scratch with 4 parallel large shards).

## Estimates

- Machine time: about 235 CPU-sequential hours (small 1 h, medlo 28 h, medhi 85 h, large 120 h), scaled from the observed EXP-BASE-SIZE per-sample rates. About 60 h wall with 4 shards per group.
- Disk: about 60 GiB peak scratch; raw rows about 50 MB gzip.
- Agent effort: none for execution (detached script); scripted for merge; judgment limited to reading the generated failure lists.
