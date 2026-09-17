# EXP-CROSSFILE-005: Incumbent block-bounded solid frontier (artifact bytes versus exact access granularity and amplification)

| Field | Value |
|---|---|
| Status | **READY** (collection): incumbents are installed and pinned (`research/baselines/tool-versions.json`), the collector `solid_access.py` exists and passed a non-decision-grade smoke (below). Decision use additionally needs EXP-CROSSFILE-004 for the Entrybound side of the comparison. |
| Domain / program sections | crossfile / 8.5 |
| decision_ids | DEC-CMP-027 (incumbent reference and SPEC §23.4 declared-cost check) |
| Evidence type | deterministic bytes and exact block-layout analytics (§4.6, §4.13); REF-INC measurements |
| Method | `research/decision-method.md`, pre-registered |
| Record | Design only; `record.md` (§12.2) instantiated once gate G-A holds. |
| Design session and disclosure | Phase C-design crossfile designer; L7 disclosure as in EXP-CROSSFILE-001. |

## 1. Question

For block-bounded solid incumbents (7-Zip LZMA2 solid blocks, tar.xz blocks, `zstd --long`,
squashfs, DwarFS), what artifact size do they reach at each exact access granularity and access
amplification, on the same items and byte ranges as Entrybound's lookback sweep, so that
DEC-CMP-027's "ratio gap versus 7z/xz solid, zstd --long and DwarFS at equivalent access cost" and
the SPEC §23.4 "a few percent versus solid" claim can be tested?

## 2. Hypotheses

- **H1.** On the cross-file target families, the incumbent frontier (minimum `artifact_bytes` at
  each `access_granularity_bytes` level) at 1 MiB and 4 MiB granularity is within 1.05x of the
  unbounded-solid best (7z `-ms=on` or `xz -9e` single block) on F01, F03 and F05, but not on
  F18 (version series need long windows).
- **H0.** Block bounding at 1-4 MiB costs more than 5% against unbounded solid on most target
  families, so "a few percent versus solid" is not reachable by any block-bounded design,
  incumbent or Entrybound, at that granularity.
- **Falsification.** H1 is falsified if the per-family ratio of the 1 MiB-granularity frontier to
  the unbounded best exceeds 1.05 on F01, F03 or F05 at the family level on validation.

## 3. Decisions informed; proposed OD and HC sets

DEC-CMP-027 (see EXP-CROSSFILE-004 §3). The incumbents are references (REF-INC), not candidates
for the decision; they enter R0 item 4 and the CC-01 capability-combination report (objective
§1.3, reporting only).

## 4. Configurations (REF-INC)

Pinned tool versions: `research/baselines/tool-versions.json` (7-Zip 23.01, xz 5.4.5, zstd 1.5.5,
squashfs-tools 4.6.1, DwarFS 0.15.7 static binary SHA-256
`baa03026e7d2c195fdb78bf261cd7d20f620b20685f8a3e26162ba9d652b0d78`). All single-threaded and with
fixed timestamps for the repeat-hash check.

| config_id | Command core | Granularity source |
|---|---|---|
| `7z-mx9-ms{off,1m,4m,16m,64m,on}` | `7z a -t7z -mx=9 -mmt=1 -ms=<solid> -snl` | exact (`7z l -slt` folders) |
| `7z-mx9-ms16m-qs` | as above with `-mqs=on` (sort by type within solid blocks: the incumbent analogue of similarity ordering) | exact |
| `tarxz-9e-b{1m,4m,16m,64m}` | sorted deterministic tar piped to `xz -9e -T1 --block-size=<B>` | exact (`xz --robot --list -vv` blocks, tar data offsets) |
| `tarxz-9e-single` | `xz -9e -T1` (one block; unbounded solid reference) | exact |
| `tarzstd-19-long27` | `zstd -19 --long=27 -T1` (one frame) | exact |
| `squashfs-xz-b1m`, `squashfs-zstd19-b1m` | `mksquashfs -b 1M -processors 1 -mkfs-time 0` | declared bound (block size) |
| `dwarfs-l7-S{20,22,24,26}` | `mkdwarfs -l7 -S <bits> -N 1 --no-history-timestamps --no-create-timestamp` (default nilsimsa order) | declared bound (block size) |
| `dwarfs-l7-S24-order-path` | as above with `--order=path` (ordering control) | declared bound |

- Reference configuration for ratios: `7z-mx9-ms-on` per item (unbounded solid), and
  `tarxz-9e-single` as a second reference (the better of the two per item is the objective CC-01
  "ratio" specialist baseline).
- **Excluded:** multithreaded variants (block boundaries depend on thread count, so the
  granularity would not be a function of the configuration); zpaq (drops symlinks silently,
  `research/baselines/README.md`); formats without a random-access path (tar.gz) beyond the
  existing EXP-BASE-SIZE pass.

## 5. Corpus selectors

- Manifest `ed28b1b4…`, tuning only. `spec.yaml`: the 50-item cross-file target set of
  EXP-CROSSFILE-001 §5 (identical items, so rows join with EXP-CROSSFILE-004 per item).
- Access sample: `solid_access.py sample_plan` (seed = SHA-256 of `EXP-CROSSFILE access-sample
  v1\0` || item_id), shared with EXP-CROSSFILE-004/-006.
- Validation: the validation look of EXP-CROSSFILE-004 re-runs these configurations on the same
  validation items (a separate `spec-validation.yaml`, generated with the same configurations,
  committed before that look).

## 6. Environment and platform requirements

`wsl-ubuntu`, x86-64 Linux, ext4 under `/root/eb-research`; `timing: false`; tools on PATH plus
`/root/eb-research/tools/dwarfs`; no network, no privilege. `disk_guard.py` must pass.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
export PATH=/root/eb-research/tools/dwarfs:$PATH
PY=/root/eb-research/venv/bin/python
$PY -m ebr validate --spec research/experiments/EXP-CROSSFILE-005/spec.yaml
$PY -m ebr run      --spec research/experiments/EXP-CROSSFILE-005/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-CROSSFILE-005/spec.yaml
$PY -m ebr normalize  --spec research/experiments/EXP-CROSSFILE-005/spec.yaml
$PY -m ebr report     --spec research/experiments/EXP-CROSSFILE-005/spec.yaml
```

Per-configuration commands are in `spec.yaml`. Each writes `{scratch}/artifact.bin`, then runs
`research/experiments/EXP-CROSSFILE-005/solid_access.py`, which prints one JSON line
(`artifact_bytes`, `access_granularity_bytes`, `access_amplification_{1b,4k,1m,entry}`,
`granularity_source`), and the existing `research/baselines/probes/roundtrip_check.py` as the
`roundtrip_ok` check.

**Feasibility smoke (non-decision-grade, recorded for design only).** On 2026-09-17 in WSL, on a
generated 300-file 3.7 MiB tree under `/root/eb-research/scratch/crossfile-design-smoke`, the
collector parsed 7z `-ms=1m` and `-ms=off`, tar.xz with 1 MiB blocks, tar.zst and squashfs
artifacts in well under 2 minutes and produced granularities consistent with the configured
units (bounded by the solid block for `-ms=1m` and by the largest file for `-ms=off`). The smoke
checks that the collector runs; nothing from it is a result.

## 8. Seeds, warmups, repetitions

Seeds `order` 20260917051, `bootstrap` 20260917052. Warmups 0. Repetitions 2 plus repeat-hash on
`artifact_sha256`; a configuration whose output hash differs across repetitions is recorded as
nondeterministic (incumbents claim no determinism here, so the metric is reclassified stochastic
per §4.13 and reported, not eliminated).

## 9. Metrics

| Metric | Role | ebr family | Threshold | Source |
|---|---|---|---|---|
| `artifact_bytes` | reference size | size | T-01 | `stdout_json` `artifact_bytes` |
| `access_granularity_bytes` | reference granularity | size | T-09 | `stdout_json` |
| `access_amplification` (1b, 4k, 1m, entry) | reference amplification | other | T-22 | `stdout_json` |
| `artifact_sha256` | repeat-hash | correctness | §4.13 | `path_sha256` |
| `roundtrip_ok` | correctness gate | correctness | §3.3 | `check` |
| `input_bytes` | descriptive | size | none | `input` |

## 10. Normalization and statistical analysis

- Per item: the incumbent frontier is the lower envelope of (`access_granularity_bytes`,
  `artifact_bytes`) over configurations whose granularity source is `listing`; `declared_bound`
  configurations are plotted but not on the exact frontier.
- Joined with EXP-CROSSFILE-004 per item: for each Entrybound candidate, the incumbent frontier
  value at the candidate's granularity (interpolated only between measured configurations of the
  same tool; no extrapolation), and the ratio to the unbounded best.
- Aggregation L1-L4 per decision-method §4.9 on log ratios; family and tier strata. These are
  external-comparison claims: both references are reported (§6.5 reference arm) and the ratio
  to the unbounded-solid specialist is evaluated against the CC-01 declared-cost tolerance
  (objective §1.3), which is a report, not a selection rule.

## 11. Practical-significance thresholds

T-01, T-09, T-22 by reference; the CC-01 tolerance is objective §1.3's declared-cost tolerance.

## 12. Sensitivity analysis

Family-mix arms of §6.5 (equal weights, WM-DIST, WM-BACKUP) for the frontier ratio; `-mqs=on`
versus default ordering; DwarFS nilsimsa versus path order (the ordering effect for an incumbent
with similarity ordering, cross-checked against EXP-CROSSFILE-002).

## 13. Validation look and held-out placeholder

Validation as in §5. Held-out placeholder `EXP-CROSSFILE-005-HO` at Commit A (§5.5), re-running
the same configurations once post-unlock for the external-comparison claims.

## 14. Expected negative results worth recording

- Entrybound's lookback designs lie above the incumbent frontier at every granularity (incumbents
  win ratio at equal access cost).
- Block bounding at 1 MiB costs far more than "a few percent" on version-series families for
  every design, falsifying the SPEC §23.4 wording as a general claim.
- DwarFS similarity ordering beats all 7z/xz configurations at 16 MiB blocks.

## 15. Threats to validity

- squashfs and DwarFS granularities are bounds, not exact; they are excluded from the exact
  frontier.
- The decoded-byte model assumes a sequential decoder that must decode from the unit start;
  LZMA2 and xz cannot seek inside a block, which matches; Zstandard frames likewise.
- 7z `-snl` stores symlinks; items with symlinks behave identically across 7z configurations.
- Corpus and L7 threats as EXP-CROSSFILE-001.

## 16. Cost

About 70 CPU-hours (LZMA2 `-mx=9` and `xz -9e` dominate; 20 configurations x 50 items x 2
repetitions), wall about 5 h sharded; disk: scratch peak about 2 GB per concurrent shard, raw
about 200 MB. Agent effort: none for execution; judgment for the joined analysis.
