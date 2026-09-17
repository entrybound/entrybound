# EXP-EVAL-005: Whole-system timing, memory and local-access baseline, REF-PROD against capability-matched incumbents (quiet windows, production-built `ebound`)

| Field | Value |
|---|---|
| Domain | evaluation (program sections 6, 7, 34, 35) |
| Status | NEEDS_TOOLING (gates: G-A, G-B, EXP-EVAL-003 pass including arm P, EXP-EVAL-004 calibration pass per environment and family; tooling: operation wrappers, in-process incumbent access driver, Windows incumbent staging, spec generator) |
| Kind | Decision-relevant (EMPIRICAL timing, memory and local-access parts of the claim decisions of program §35 and the capability-matrix marks). Remote access over emulated networks belongs to the remote domain; this experiment covers local operations only. |
| Method | `research/decision-method.md` §3, §4 (all), §6 at `14b977c`; harness review use constraints 1-3 |

## Author, independence and exposure

Designed by the Phase C-design evaluation-domain session. Candidates are fixed by objective §1.3 and §3.0 and by `research/baselines/configs.json`; no candidate is designed here. The L7 exposure is recorded in the EXP-EVAL-001 protocol. The tuning analysis, the declaration of superiority metrics, the MDE computation and the validation analysis are executed by an unexposed session.

## Question

On the Windows host and WSL2, with resource-matched affinity and thread counts, how do REF-PROD `ebound` operations compare with the capability-matched incumbent configurations, and are the SPEC §22-§23 speed and access marks supported, qualified or contradicted?

- Operations: pack, full unpack, verify, list, random entry read, stream unpack.
- Metrics: wall time, CPU time, throughput, peak memory and per-operation latency.

## Hypotheses and falsification

The directions and sizes are predictions, `INFERRED` from SPEC §23.4. They are pre-declared so that superiority metrics are fixed before the validation look, and the tuning analysis session re-declares them in `record.md` before that look.

- **H1 (random access, CC-01 and CC-02).** `random_entry_latency_s` of `balanced` INDEXED (in-process batch) is `NON_INFERIOR` to `7z-mx9-nosolid` read through libarchive in-process, and `DISTINGUISHABLE_BETTER` than solid 7z and every tar pipeline, which must decode a prefix. Falsified by a not-non-inferior corpus verdict against 7z non-solid. The random-access mark then becomes qualified.
- **H2 (decode speed).** Full-unpack `decode_wall_s` of `fast` and `balanced` is `NON_INFERIOR` to `tarzstd-3-t1` extraction at T-04, in `st-v`/`st-p` and in `mt-v-8`/`mt-p-7`. Write-time verification cost appears on the pack side (OD-06). Falsified by a not-non-inferior verdict in either environment. The decompression-speed cell then becomes partial (DEC-ECO-070).
- **H3 (encode speed, profile-qualified).** `encode_wall_s` of `fast` is `NON_INFERIOR` to `tarzstd-3-t1` at the `fast` band. `extreme` is expected to be `DISTINGUISHABLE_WORSE` than `7z-mx9-solid`, a declared cost reported under SPEC §23.4 "write-time verification". Falsified by `fast` failing non-inferiority.
- **H4 (memory).** Decode `peak_rss_bytes` (Linux) and `peak_commit_bytes` (Windows) of `balanced` are within the T-06 band of `tarzstd-3-t1` extraction or better. Falsified by `DISTINGUISHABLE_WORSE`.
- **H5 (streaming, binary plus graded).** STREAM pack to a pipe and unpack from a pipe succeed with `scratch_write_bytes` equal to the extraction output only: no staging spill beyond the declared bound, by write accounting with tolerance 0 (§4.14). `stream_unpack_wall_s` is `NON_INFERIOR` to `zstd -d | tar -x` at T-04. Falsified by any extra scratch write (binary: an HC-06/F-09 finding) or by not-non-inferior timing.
- **H6 (rank stability, DEC-ECO-031).** The rank order of incumbent configurations by `decode_wall_s` and `encode_wall_s` is stable across three views: defaults versus defaults, tuned configurations and resource-matched configurations (Kendall tau-b ≥ 0.8). Lower agreement is reported as ranking fragility, and every claim then carries all three views.

## Decisions informed

`DEC-CMP-011`, `DEC-ECO-070`, `DEC-ACC-002` (local streaming, random-access, partial-retrieval and parallel-decode marks), `DEC-ECO-031`, `DEC-ECO-069`, `DEC-ECO-043`, `DEC-CRY-042` (encrypted random-entry latency against 7z AES non-solid; CC-04 OD-10 part).

## Candidates

The REF-PROD binary is the EXP-EVAL-002 build (`9e44608`, `--locked`, default features); on Windows it is `ebound.exe` built the same way with `D:/eb-research/target/refprod-win`. Incumbent tool versions are the WSL apt versions (`tool-versions.json`). The Windows tools are the 7-Zip Windows release matching 23.01 where available (otherwise the pinned newest, recorded), the built-in bsdtar 3.8.8, and the zstd v1.5.5 Windows release, each staged with its SHA-256 in `research/baselines/tool-versions-windows.json` (to be written).

| Operation (spec group) | REF-PROD candidates | Incumbent candidates (resource-matched thread count = logical CPUs in the affinity set) |
|---|---|---|
| O1 pack | `refprod-{fast,balanced,dense,extreme}-indexed`, `refprod-balanced-stream` | `tarzstd-3`, `tarzstd-19`, `targz-gzip-6` (default), `zip-6` (default), `7z-mx5-solid` (default), `7z-mx9-solid`, `7z-mx9-nosolid`, `tarxz-6`, `squashfs-zstd`, `dwarfs-l7` |
| O2 full unpack | the O1 REF-PROD archives via `ebound unpack` | extraction of each O1 incumbent artifact with its own tool |
| O3 verify | `ebound verify` | `7z t`, `zstd -t` (plus the tar walk), `xz -t`, `unzip -t`; squashfs and dwarfs have no content verify, so they are marked `NOT_APPLICABLE` and reported |
| O4 list | `ebound list` | `7z l`, `unzip -l`, `tar -tf` over each compressed stream, `unsquashfs -l`, `dwarfsck -l`. **Process-level list timing is informational only** (T-08) |
| O5 random entry read (in-process batch, 1,000 seeded entries per round) | `ebr-access` INDEXED `balanced` and `dense`, plus the `refprod-balanced-indexed-enc-bucketed` archive for `encrypted_random_entry_latency_s` | libarchive 3.7.2 in-process (`libarchive-c`) over `7z-mx9-nosolid`, `7z-mx9-solid`, `zip-6`, `tarxz-pixz`, `tarzstd-3`; CPython `zipfile` over `zip-6`; `7z-mx9-nosolid` AES-256 with `-mhe=on` via libarchive for the encrypted row; `squashfs-zstd` and `dwarfs-l7` through their FUSE mounts (WSL only; access path includes FUSE, labelled) |
| O6 stream unpack | `ebound unpack - dest` reading STREAM from a pipe | `zstd -dc artifact | tar -x`; `xz -dc | tar -x` |

- **Reference candidate** per operation: `refprod-balanced-indexed`, or its operation counterpart.
- **Views for H6 and DEC-ECO-031.**
  - Defaults versus defaults: the tool with no level flag.
  - Tuned: the per-family best configuration selected on tuning by EXP-EVAL-007's incumbent tuning-parity arm.
  - Resource-matched: the affinity and thread-count configurations above.
- **Excluded.**
  - `zpaq-m5`, `tarbrotli-q11`, `tarxz-9e`: timing unaffordable at the §4.6 round counts; they are size references only (EXP-EVAL-001).
  - `borg-repo`, `restic-repo`: backup repositories, not archive operations. Their capability rows live in EXP-EVAL-006.
  - `wim-*`: Windows-specific. They are included only in the Windows O1/O2 groups, and only if wimlib staging succeeds.
  - Hardware counters: `PLATFORM_BLOCKED` (EXP-EVAL-004).
- **Precondition for O5 on the harness.** EXP-EVAL-003 arm P must show timing parity between `ebr-access` built with `research-internals` and the same crate built against `entrybound` with the feature off (`--features` override in a research-only build profile). Without it, O5 Entrybound rows use the feature-off build only (harness review use constraint 2).

## Corpus selectors

- **Rule, deterministic and content-blind.** Within each split, take one item per (family, independence group, scale tier): the lexicographically smallest `item_id`. For O1, O2, O3 and O6 on the large tier, `dense` and `extreme` are excluded on items above 2 GiB (machine-time cap). That exclusion is a stated limitation and is not imputed.
- **Tuning look.** The tuning split under that rule.
- **Validation.** The validation split under that rule, one look, registered before execution.
- **Minimum support** (§4.9) is checked by the spec generator. It refuses to emit a validation spec with fewer than 10 groups across 5 families per operation.
- **Windows.** NTFS copies staged with content verification (the EXP-EVAL-004 staging script).
- **Family weights.** `workload-mix.json` (G-B); equal weights as a sensitivity arm.

## Environment

`windows-host` (Defender on, recorded; AC power; PDH covariates; `st-p`, `mt-p-7`, `mt-psmt-14` for T-14 context) and `wsl-ubuntu` (`st-v`, `mt-v-8`; `VIRTUALIZED`). Every comparison is paired within one environment, one affinity configuration and one ebr run (§4.1). External comparisons on Windows add:

- (a) in-process harness timers that exclude file close, where they exist;
- (b) the owner-excluded scratch directory arm, if the owner has configured one (§4.2).

Cache state is `hot`. The run happens only in exclusive windows after calibration passes.

## Commands

```sh
# prerequisites: EXP-EVAL-003 PASS (incl. arm P), EXP-EVAL-004 PASS for (env, family, affinity); lock_parity.py and harness-cli-parity.sh PASS
# artifact prep (not timed): research/experiments/EXP-EVAL-005/prep_artifacts.sh -> /root/eb-research/cache/eval005/<item_id>/<candidate>.{eb,bin,dir}
# spec generation (TO BE WRITTEN): research/experiments/EXP-EVAL-005/make_specs.py --split {tuning,validation} --env {wsl-ubuntu,windows-host} --op {O1..O6} --affinity {st-v,mt-v-8,st-p,mt-p-7}
#   -> research/experiments/EXP-EVAL-005/specs/EXP-EVAL-005-<op>-<env>-<affinity>-<split>.yaml
# operation wrappers (TO BE WRITTEN): research/experiments/EXP-EVAL-005/ops/{extract.sh,verify.sh,list.sh,stream_unpack.sh}
# in-process incumbent access driver (TO BE WRITTEN): research/experiments/EXP-EVAL-005/ops/incumbent_random_access.py
#   --format {7z,zip,tar.xz,tar.zst} --archive <path> --entries <seeded list> --batch 1000  -> one JSON line {random_entry_latency_s, entries_verified_by_content_hash}
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-EVAL-005/spec.yaml'
#   python -m ebr run --timing --gzip --cpus 2 --spec <spec>; verify-raw; normalize
# Windows: pwsh -File research/experiments/EXP-EVAL-004/run_windows_arm.ps1 -Spec <windows spec>
# decision analysis (research/tools/decide; TO BE WRITTEN): tuning exploratory -> record.md superiority declarations -> MDE gate -> validation look
```

`spec.yaml` in this directory is the O2 pilot: WSL, `st-v`, tuning, small tier.

## Seeds

- `seeds: {order: 20260926, bootstrap: 20260926, command: 20260926}`.
- Random-entry selection: 20260927.
- Selection-stability bootstrap: 20260928.

## Warmup

§4.5: 2 rounds on the small and medium tiers and 1 on the large tier, recorded. The warmup-adequacy check (Kendall tau ±0.3) is inherited from EXP-EVAL-004 per environment and family.

## Measurement method and metrics

| Canonical metric | OD | Role | Family | Threshold ID | Source |
|---|---|---|---|---|---|
| `encode_wall_s` | OD-06 | primary (O1) | time_wall | T-03 (per profile band) | runner resource, process; small tier in-process harness timer is primary (T-03 small-tier rule), process informational |
| `encode_cpu_s` | OD-06 | secondary | time_cpu | T-05 | runner |
| `encode_throughput_bps` | OD-06 | secondary (derived) | throughput | T-05b | derived |
| `decode_wall_s` | OD-07 | primary (O2) | time_wall | T-04 | runner / small-tier in-process |
| `decode_cpu_s` | OD-07 | secondary | time_cpu | T-05 | runner |
| `verify_wall_s` | OD-07 | secondary (O3; OD-07 already has `decode_wall_s` primary) | time_wall | T-04 | runner |
| `list_latency_s` | OD-07 | informational (process-level) | time_wall | T-08 | runner |
| `random_entry_latency_s` | OD-10 | primary (O5) | time_wall | T-08 | in-process batch |
| `encrypted_random_entry_latency_s` | OD-10 | secondary (CC-04 row) | time_wall | T-08 | in-process batch |
| `stream_unpack_wall_s` | OD-09 | primary (O6) | time_wall | T-04 | runner |
| `streaming_capability` | OD-09 | binary | correctness | §3.3 | wrapper |
| `scratch_write_bytes` | OD-08 | binary where zero-extra-scratch is claimed | correctness | §4.14 | write accounting |
| `peak_rss_bytes` / `peak_commit_bytes` | OD-08 | primary (O2 decode memory; one per OS) | memory_peak | T-06 | GNU time (Linux, R1-16 rule) / job commit (Windows) |
| `parallel_speedup` | OD-13 | secondary (context for the parallel-decode mark) | other | T-14 | derived within core class |

- **Correctness gating** (objective §3.0): every timed sample's output tree is hashed after the round and compared with the item's `content_tree_sha256` (O2, O6), or the entry bytes are checked against the manifest file hashes (O5). A failing sample is excluded and reported as an HC-02 finding.
- **Primary-metric discipline** (R3). O1-O6 are separate comparisons with separate ODs. Where one decision uses two operations for the same OD (OD-07 decode and verify), `decode_wall_s` is primary and `verify_wall_s` secondary.

## Replication and adaptive rule

§4.6 timing rounds (small 10/10/30, medium 10/10/20, large 10/5/20), minimum at least n_min for the comparison's confidence (Appendix D.4: 8 at 0.99, 9 at 0.995), with the precision-based adaptive rule. O5 uses at least 40 sampled operations per item per round when p95 is reported (§4.6), and batches of 1,000.

## Normalization

§4.9: L1 item median of paired log ratios (`pairing: item_repetition`), L2 groups, L3 families, L4 workload mix. Tier, environment, affinity and operation strata are never averaged. §4.2 and §4.4 invalid-sample balance rules apply.

## Statistical analysis

- **Tuning (exploratory).** Describe and form the superiority declarations per pair.
- **MDE gate** (§4.12) from tuning variance and the validation group structure: every MDE at most 1 band unit, and the item-level MDE from the EXP-EVAL-004 A/A half-widths at most 1 band unit. Otherwise the look is not taken.
- **Validation.**
  - Superiority at 1 - 0.01/k_sup; non-inferiority at two-sided 0.99.
  - Family harm guard at 0.90 plus the 1-band point bound.
  - Both-environment claims need same-direction decision-grade support in both (§4.1).
  - Linux-only claims carry the `VIRTUALIZED` soft cap.
- **External-comparison rule on Windows** (§4.2): the claim needs the same direction in `defender_on` process timing and in the harness in-process timing, and in the exclusion-directory arm where it exists.
- **Claim table output.** Per SPEC mark (density is EXP-EVAL-002's; speed, decompression, random access, streaming and parallel are this experiment's), one of `SUPPORTED`, `QUALIFIED:<scope>`, `CONTRADICTED` or `INCONCLUSIVE`, with the verdict rows cited per §12.3.

## Practical-significance thresholds

As registered in `thresholds.json` `metric_thresholds`, without restatement: T-03 per profile, T-04, T-05, T-05b, T-06, T-08 (in-process floor only), T-14, and §3.3 binary metrics.

## Sensitivity analysis (§6.5)

- Equal-weight arm.
- Leave-one-family-out and leave-three-families-out.
- Family Dirichlet.
- WM-* mixes.
- Tier-stratified.
- Byte-weighted (total time).
- Environment arm (Windows versus WSL).
- Reference arm (REF-PROD versus REF-INC).
- Threshold perturbation x0.5 (only where the EXP-EVAL-004 resolvability condition holds) and x2.
- Selection-stability bootstrap (2,000 group resamples).
- H6 views: defaults, tuned and resource-matched.

## Expected negative results worth recording

- `dense` and `extreme` pack much slower than `7z-mx9-solid` (write-time verification plus planning).
- `ebound list` and `verify` process start-up dominating on the small tier.
- `dwarfs-l7` FUSE random reads beating INDEXED `balanced` on F17 and F18.
- `tarzstd-3` multi-threaded decode beating `balanced` where Entrybound decode parallelism is limited.
- Windows Defender scan-on-close penalising `.eb` outputs differently from `.7z`.

All go to the negative-results register, and EXP-EVAL-012 reports them under "workloads where Entrybound is not best".

## Threats to validity

- **Implementation, not format.** libarchive's 7z reader is not 7-Zip's own decoder. In-process incumbent latencies measure the chosen library, which is named in every row. The CLI process-level extraction of O2 uses the native 7-Zip tool.
- **FUSE access paths** add kernel crossings. They are labelled and never compared with in-process numbers without that label.
- **REF-PROD is the research baseline, not the frozen design.** A frozen-design rerun of O1, O2 and O5 on validation happens at Commit A, and EXP-EVAL-012 uses it.
- **WSL2 I/O** runs through a VHDX on NTFS. Absolute throughput differs from native Linux, hence the `VIRTUALIZED` cap.
- **Windows incumbent builds** may differ from the Linux ones (compiler, SIMD dispatch). Cross-OS numbers are never paired (§4.1).

## Platform requirements

Windows 11 host (non-admin, exclusive quiet window, AC power); WSL2 root; x86-64; FUSE in WSL for the squashfs and dwarfs access rows (if `/dev/fuse` is unavailable, those rows are `BLOCKED` and reported); no network (tool staging downloads happen before the window); large storage. macOS and native ARM64 extension: EXP-EVAL-013 (BLOCKED).

## Estimates

- Machine time: about 260 h in exclusive windows (WSL about 140 h; Windows about 120 h). The dominant cost is O1 and O2 at 10-30 rounds with cooldown.
- Disk: about 120 GiB (prebuilt artifacts for every candidate and item, plus NTFS copies).
- Agent effort: none during runs; scripted analysis; judgment for the superiority declarations and the claim-table interpretation (unexposed analysis session).
