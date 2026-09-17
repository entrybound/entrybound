# EXP-LEGACY-033: export wrapper codec parameters — size, encode cost, consumer decode cost and decoder memory per level, block/seekable variants

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (harness binary `ebr-legacy-wrap` applying the production wrapper crates at alternative levels to identical `tar/pax-v1` bytes and ZIP entries; consumer-decoder driver; calibration and runner prerequisites of C§2) |
| Domain / program sections | legacy / §27 (with §6 incumbents and §8.6 codec parameters; coordinate with the codecs domain, which owns native codec decisions) |
| decision_ids | DEC-LEG-027, DEC-CMP-024 |
| Decision types (proposed) | EMPIRICAL |
| OD / HC (proposed) | OD-05 (M05.1), OD-06 (M06.1-M06.2), OD-07 (M07.1), OD-08 (M08.1, consumer decoder memory); HC-16 (frozen profiles untouched: any change is a new profile id), HC-13 |
| Shared rules / L7 / gates | C§ header, C§2 (calibration for `time_wall`, `time_cpu`, `memory_peak`; runner additions) |
| Timing | `timing: true` (WSL, `VIRTUALIZED`) |

## 1. Question

For the legacy export wrappers (gzip, zstd, xz, bzip2 over `tar/pax-v1`; per-entry DEFLATE in ZIP), how do the frozen
parameters (DEFLATE 6 when smaller, gzip 6, zstd 9 with checksum, xz preset 6 single block CRC64, bzip2 900k) compare
with fast, maximum-ratio, incumbent-default, block/seekable and BGZF-style alternatives on output bytes, encode cost,
consumer decode time and consumer decoder memory, and do any alternatives justify a new profile version?

## 2. Hypotheses

- **H1.** Against `status-quo-frozen-levels`, `max-ratio-levels` reduce `artifact_bytes` by more than the T2
  minimum-gain band (`decision-method.md` §7.3 "New profile", k = 3) for `tar.xz` and `tar.zst` in at least 3 families,
  while increasing consumer `decode_wall_s` beyond the T-04 band only for xz 9e `[INFERRED]`.
- **H2.** `fast-levels` are non-inferior on consumer `decode_wall_s` and inferior on `artifact_bytes` beyond the T-01 band
  in every family except F11/F12/F13 (already-compressed content).
- **H3.** Multi-block xz (16 MiB blocks) and seekable zstd (16 MiB frames) cost ≤ 2% `artifact_bytes` and enable parallel
  or random decode in at least one common consumer tool each (`xz -T0`, `pixz`; zstd seekable CLI), while BGZF gzip is
  decoded in parallel by `bgzip -@` only.
- **H0.** No alternative is distinguishable from the frozen parameters on any primary metric.

## 3. Decisions informed

| Decision | Supplied here | Remaining elsewhere |
|---|---|---|
| DEC-LEG-027 | Size, encode time/CPU, consumer decode time and decoder memory per level and wrapper on the corpus | Reader acceptance of any new profile EXP-LEGACY-030 |
| DEC-CMP-024 | Size and speed of alternative levels; parallel and seekable decoder support in common tools; BGZF | Golden vectors and encoder-upgrade behaviour EXP-LEGACY-031 |

## 4. Candidates

| Decision | Candidates | Configurations measured |
|---|---|---|
| DEC-LEG-027 | status-quo-frozen-levels; max-ratio-levels; fast-levels; incumbent-defaults; multiple-profile-variants | gzip {1, 6, 9}; zstd {3, 9+checksum, 19, 19 `--long=27`}; xz {1, 6 single-block CRC64, 9, 9e}; bzip2 {1, 9}; ZIP DEFLATE per entry {1, 6 when smaller, 9}; incumbent defaults = gzip 6, zstd 3, xz 6, bzip2 9, `zip -6`; `multiple-profile-variants` evaluated as an R9 partition by family |
| DEC-CMP-024 | status-quo-frozen-wrappers; retune-levels; add-seekable-profiles; bgzf-style-gzip; new-profile-on-encoder-change (EXP-LEGACY-031) | xz multi-block 16 MiB; zstd seekable format 16 MiB frames; BGZF 64 KiB blocks level 6 |

Encoders are the production crates (`flate2` with the release backend, `zstd`, `lzma-rust2`, `bzip2`) invoked by
`ebr-legacy-wrap` on the exact `tar/pax-v1` bytes exported by `ebound-head`; the status quo is additionally measured
end-to-end through the CLI (`ebound convert --to tar.xz` etc.) to confirm byte identity with the harness path for the
frozen parameters (harness use constraint: production-parity before timing claims). Consumer decoders: GNU gzip 1.12,
pigz, zstd 1.5.5, xz 5.4.5 (and `-T0`), pixz, bzip2 1.0.8, lbzip2, Python 3.12 modules, bgzip.

## 5. Corpus selectors

Tuning (exported to `tar/pax-v1` first): `f01-tuning-curl-8-19-0-git`, `f02-tuning-lz4-intree-make-build`,
`f04-tuning-sourcelike`, `f05-tuning-loghub-openstack`, `f06-tuning-gharchive-2015-01-01-h15-h16`,
`f07-tuning-silesia-sao`, `f09-ubuntu-noble-usr-binaries`, `f11-alpine-324-apks`, `f12-tuning-kodak-jpeg-matrix-medium`,
`f18-tuning-zstd-releases-1-5-x`; large tier `f05-tuning-loghub-hdfs-v1`, `f01-tuning-llvm-project-20-1-8-git`
(10+ groups, 10 families). Validation: `f01-validation-cpython-3-13-15-git`, `f02-validation-cjson-cmake-build`,
`f04-validation-objstore`, `f05-validation-loghub-ssh`, `f06-validation-noaa-storm-events-2016`,
`f07-validation-gen-numeric-b`, `f09-fedora-44-usr-binaries`, `f11-maven-central-jvm-jars`,
`f12-validation-fsa-jpeg-matrix-medium`, `f18-validation-redis-7-2-point-releases`; large tier
`f03-validation-bat-cargo-vendor` (11 groups, 10 families). Family weights: `research/methods/workload-mix.json`
(gate G-B). Held-out: placeholder (C§5).

## 6. Environment

`wsl-ubuntu`, exclusive timing window, `st-v` = {2} for single-threaded encoders and decoders; `mt-v-4` = {2..5} for the
parallel-decode arm (informational speedups within one core class, T-14 only if calibrated); cache `hot`; outputs on
ext4 under `/root/eb-research`.

## 7. Commands and tooling

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
python3 research/harness/lock_parity.py && bash research/harness/harness-cli-parity.sh
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-033 --export-tar-with /root/eb-research/target/legacy-head/release/ebound --append-corpus-items research/experiments/EXP-LEGACY-033/real-items.txt
/root/eb-research/venv/bin/python research/tools/legacy/wrap_parity.py --experiment EXP-LEGACY-033   # CLI vs ebr-legacy-wrap byte identity for frozen parameters; must PASS
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-033/spec.yaml --timing
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-033/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-033/spec.yaml
/root/eb-research/venv/bin/python -m ebr report --spec research/experiments/EXP-LEGACY-033/spec.yaml
```

`ebr-legacy-wrap --input <tar> --codec gzip|zstd|xz|bzip2|zip-deflate --level <l> [--block-bytes N] [--seekable]
[--bgzf] --out <file>` prints encode stage timing and allocator peak; the decode arm is the spec's `check`-style consumer
command per decoder.

## 8. Seeds

`order 2509173311`, `bootstrap 2509173312`, `command 2509173313`.

## 9. Warmup, replication, adaptive rule

§4.5 warmups (2 small/medium, 1 large); §4.6 minimum rounds 10 (step 10/10/5, cap 30/20/20) with the precision-based
adaptive rule; `artifact_bytes` is deterministic (2 repetitions + repeat-hash suffice, §4.13).

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `artifact_bytes` | banded T-01 (primary) | OD-05 | T-01 |
| `encode_wall_s` | banded T-03 (primary; band row `balanced`, the default-profile row, pending reviewer confirmation) | OD-06 | T-03 |
| `decode_wall_s` (consumer CLI decoder, whole process) | banded T-04 (primary) | OD-07 | T-04 |
| `peak_rss_bytes` (consumer decoder, GNU time) | banded T-06 (primary) | OD-08 | T-06 |
| `encode_cpu_s`, `decode_cpu_s`, `decode_throughput_bps` | banded T-05/T-05b (secondary) | OD-06/07 | T-05, T-05b |
| parallel/seekable decode support per tool | non-canonical descriptive | — | none |

## 11. Normalization

Ratios candidate / `status-quo-frozen-levels` per (item, wrapper), paired within rounds (`pairing: item_repetition`).

## 12. Statistical analysis

§4.9 aggregation with cited-mix weights (equal weights as a §6.5 arm); §4.10 intervals; §4.11 verdicts; confirmatory
comparisons W versus each tuning survivor declared after tuning with §4.12 confidences and the MDE gate. R4 dominance,
R6 minimum gain for a new profile version (T2, k = 3, §7.3 "New profile" row: at least 3 families), R9 family partition
(`multiple-profile-variants`) subject to expressibility (export profiles are chosen by the caller, so profile-level
partitions are expressible; per-family automatic selection is not) and cost.

## 13. Practical significance

T-01, T-03, T-04, T-05, T-05b, T-06 per `decision-method.md` §3.2; minimum gains §7.3 and Appendix D.6.

## 14. Sensitivity analysis

Full §6: weight grid and Dirichlet over the four ODs, threshold perturbation (§6.4 with the resolvability condition),
leave-one-family-out and leave-three-families-out, tier-stratified, byte-weighted arm, decoder arm (Python modules
instead of CLI decoders), selection-stability bootstrap (§6.7).

## 15. Expected negative results worth recording

Frozen parameters on the non-dominated frontier with no alternative meeting the T2 minimum gain (status quo stands);
seekable/multi-block variants not supported by default consumer tools; xz 9e decoder memory making it unsuitable for
constrained consumers.

## 16. Threats to validity

WSL timing (`VIRTUALIZED`); consumer decoders are C implementations while encoders are Rust crates (the decision is about
what consumers experience); already-compressed families dominate size-neutral outcomes; large-tier coverage is thin.

## 17. Platform requirements

Linux x86-64 WSL2 exclusive timing window; storage ~50 GB; no privileged operations.

## 18. Estimates

Compute ≈ 80 compute-hours (xz 9e and zstd 19 `--long` on large items dominate); disk ≈ 50 GB. Agent effort: scripted
execution; judgment for analysis.

## 19. Expected cost tiers `[INFERRED]`

New wrapper parameter set as a new profile id: T2 (k = 3) with permanent golden vectors (C7); seekable/multi-block
profiles: T2; retuning the frozen profiles in place: excluded (HC-16).
