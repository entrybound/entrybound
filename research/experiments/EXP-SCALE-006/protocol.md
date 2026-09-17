# EXP-SCALE-006: Allocator-oracle audit of bounded-memory claims and adversarial resource declarations (HC-06, M08.3)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** |
| Domain | scale (program §13; HC-06 instruments shared with conformance §29-§30) |
| Platforms | Linux x86-64 (WSL2, root, cgroup-v1); Windows host secondary arm for peak commit corroboration only |
| Requires timing | false (refusal CPU is measured with in-process counters as a deterministic-precision descriptive value) |
| Compute estimate | about 60 machine-hours after tooling |
| Disk estimate | about 260 GB peak (64 GiB probes of STREAM and INDEXED readers) |
| Agent effort | judgment (allocator feature, forge generators, pre-validation bound H, claim table); scripted execution |
| Tooling to build | (1) the shared counting allocator with allocation-site attribution `research/harness/crates/ebr-alloc` designed in EXP-CONF-007 (one crate, built once; `decision-method.md` §14 item 17), linked into `ebr-access` behind a harness feature `ebr-alloc`; if its reviewed `unsafe` lint exception is not granted, the fallback is a safe `#[global_allocator]` static over the pinned `stats_alloc` crate for peak counts plus pinned `dhat` runs for attribution at small sizes; (2) research-internals trace hooks (default-off, additive, identity re-proved with `internals-identity-check`) emitting `budget_checked(field)` events so the allocator log can order untrusted-sized allocations against budget checks (objective MVT-06(a)); (3) `ebr-access --mode alloc-probe` (operations below, `--probe-total-bytes`, `--probe-entries`, `--limits bootstrap|raised`, `--isolate true`); (4) `ebr-forge` binary in `ebr-pack` (format-building code, F-18) for the adversarial STREAM and INDEXED set; (5) `research/experiments/EXP-SCALE-006/cells.json`, `claims.md` (claim-by-claim table with doc locators) and `pre-validation-bound.md` (H per candidate, FORMAL derivation signed off by a second session) |

## Pre-registration

- **decision_ids:** DEC-ACC-005, DEC-ACC-028, DEC-ACC-036, DEC-ACC-052, DEC-ACC-057, DEC-ACC-060, DEC-ACC-062, DEC-CMP-017.
- **Decision type:** EMPIRICAL (allocator peaks, outcomes) with a FORMAL component (the bound H and claim table), each routed per §1.2.
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-08 (M08.3 binary, M08.1 `alloc_peak_bytes`), OD-17 (M17.1, M17.2, M17.4, M17.5); HC-06 (MVT-06(a), (b), (d)), HC-15.
- **Method, gate, author, L7:** as EXP-SCALE-001; inputs are generated and forged; analysis provisional pending non-exposed sign-off.
- **Gate §14 item 17** (counting allocator and pre-validation bound H) must exist before the first HC-06 record of any candidate; this experiment builds it.

## Question

Which bounded-memory claims made for status-quo readers, staging and random access hold under the exact allocator oracle at 1, 4, 16 and 64 GiB and at up to 10 million entries under raised policy; does any allocation sized from an untrusted field precede its budget check; how do status-quo readers behave (outcome class, reason code, allocator peak, bytes read, CPU) on adversarial archives with maximal windows, maximal items, huge manifest streams, lying declarations and undeclared budgets exceeding caller policy; and what does dedup indexing cost in memory at the 4,000,000-chunk policy limit?

## Hypotheses

- **H1 (claims, `[INFERRED]` from code and docs):** STREAM `verify` and `unpack` satisfy M08.3 growth ≤ max(4 MiB, 0.10 × R0) on the **bytes** axis at fixed entry count, but grow by O(entries + chunks) metadata on the entries axis (DEC-ACC-060 status quo), and retain full Chunks for POSIX-metadata archives (growth proportional to plaintext); INDEXED `verify`, `list` and `unpack` fail M08.3 on the bytes axis (whole-file read); INDEXED `read` of one entry satisfies M08.3 on the bytes axis.
- **H1 (adversarial):** every lying or over-policy case is refused (`POLICY_REFUSED`, `CORRUPT` or `NONCONFORMING` per MVT-06(a)) with allocator peak ≤ R0_max + H; STREAM with `BudgetDeclared = false` exceeding caller policy is refused with a policy code distinguishable from corruption (DEC-ACC-036), and metadata and decode-consistency violations are detected only at EOF (DEC-ACC-057 status quo), so peak memory before refusal grows with the violating stream's length.
- **H0:** claims hold as written in every row of the claim table.
- **Falsification:** one run exceeding the growth tolerance falsifies that (operation, axis) claim; one untrusted-sized allocation before its check, one exceedance of R0_max + H, or one unrefused lie is an HC-06 violation artifact.

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-ACC-005 | Peak (allocator) under Verify/Stage/Retain for up to 1M entries and 4M Chunks (raised policy) and hostile maximal windows, Chunks, regions and manifests; POSIX-metadata STREAM verify retention; claim-by-claim table | fixes and prototypes: EXP-SCALE-008 |
| DEC-ACC-028 | Binary bounded-memory classification of reader operations (the RSS inventory is EXP-SCALE-001) | — |
| DEC-ACC-036 | Enumeration of refusal paths and codes when caller limits are exceeded with `BudgetDeclared = false`; per-Chunk policy vs corruption distinction | conformance cases: conformance domain (Task 29) |
| DEC-ACC-052 | Behaviour of undeclared-budget hostile archives under the status-quo SequentialLimits; allocator peak and spill at the defaults | candidate default values: EXP-SCALE-003 |
| DEC-ACC-057 | Adversarial STREAM archives (huge manifest streams, plans exceeding declared working set, maximal items and regions) with allocator tracking | fuzz campaign: conformance domain (Task 30) |
| DEC-ACC-060 | Reader allocator peak for 1M-10M entries and 4M-40M Chunks under raised policy | metadata spill and incremental validation prototypes: EXP-SCALE-008 |
| DEC-ACC-062 | Binary bounded-memory status of INDEXED verify/list/unpack on large archives | prototypes: EXP-SCALE-008 |
| DEC-CMP-017 | Dedup index memory at the 4,000,000-chunk limit (raised entry policy, dense profile) | net bytes per scope: crossfile domain; external index: EXP-SCALE-007 |

## Candidates

Status-quo operations measured in-process through the public library API (same code paths as the CLI; harness review use constraint 3 requires `--probe-isolate true`):

| candidate | Library path | Probe axes |
|---|---|---|
| `stream-verify` | `ecf::verify_stream_with_limits` over a STREAM file | bytes 1/4/16/64 GiB at 64 entries; entries 1e3, 1e5, 999k (bootstrap), 4M and 10M (raised policy) |
| `stream-unpack` | `archive::unpack_stream` (staging included) | bytes; entries up to 999k |
| `stream-list` | `ecf::open_stream` + `archive::list` | entries up to 10M (raised) |
| `stream-verify-posix` | as `stream-verify` on archives with POSIX metadata captured (Retain) | bytes 1/4/16 GiB |
| `indexed-verify` | `ecf::open` + verify | bytes; entries |
| `indexed-list` | `ecf::open_indexed_random` + list | entries up to 999k; bytes |
| `indexed-read-one` | `ecf::open_indexed_random` + `read_entry` of one seeded entry | bytes 1/4/16/64 GiB; entries |
| `indexed-unpack` | `archive::unpack` | bytes |
| `inspect-indexed` | inspection report build | entries |
| `pack-dense-dedup-index` | `archive::plan_directory` (dense) on a 4,000,000-chunk input under raised policy (allocator attribution of the dedup map) | chunks 1e5, 1e6, 4e6 |
| `forge-*` (adversarial set, run through `stream-verify`, `stream-unpack`, `indexed-verify` and `indexed-read-one` with bootstrap limits) | `ebr-forge --case <case>` produces the archive; the reader is unmodified production | cases below |

Adversarial cases (`ebr-forge --case`), pre-registered enumeration from objective MVT-06(a) plus the STREAM specifics of DEC-ACC-057: `declared-entries-over-policy`, `declared-bytes-over-policy`, `declared-below-reality` (for entries, chunks, metadata bytes, logical bytes, largest entry), `expansion-bomb`, `path-depth-bomb`, `metadata-bytes-bomb`, `long-lookback-chain`, `transitive-lookback`, `kdf-cost-over-ceiling` (instrumented Argon2id counter must stay 0), `overflowing-arithmetic-fields`, `stream-undeclared-exceeds-entries`, `stream-undeclared-exceeds-bytes`, `stream-undeclared-exceeds-metadata`, `stream-max-window-max-chunk` (declared window 4,000,000 with 8 MiB chunks), `stream-huge-manifest` (10M manifest records), `stream-plan-exceeds-working-set`, `stream-max-item` (4 GiB item), `stream-max-regions`, `element-count-mismatch-up`, `element-count-mismatch-down`. Each case also has a benign twin just inside policy (true-refusal and false-refusal accounting).

- **Reference:** R0_max per operation = maximum allocator peak over 20 runs on a minimal valid archive (one empty file) of the same layout.
- **Excluded:** writer operations (pack is plaintext-proportional by construction in the status quo; writer bounds are EXP-SCALE-007's prototypes); encrypted readers (EXP-SCALE-010 reuses this allocator feature).

## Corpus selectors

Generated inputs: EXP-SCALE-001 shapes (`b1g`, `b4g`, `b16g`, `b64g`, `n1e3`, `n1e5`, `n999k`) plus raised-policy entry shapes `n4m` (4,000,000 files of 64 B, seed 1621) and `n10m` (10,000,000 files of 16 B, seed 1622; about 20 GB of inodes on ext4, created on a dedicated loop-mounted ext4 image with `-N 12000000`), POSIX-metadata trees (`b4g` with per-file xattr `user.ebr=<seed>` and a POSIX ACL entry, seed 1631), and the forged cases (deterministic from `--seed`). Manifest `research/experiments/EXP-SCALE-006/cells.jsonl`. **Held-out placeholder (Phase D):** re-run of the M08.3 probes on post-freeze generator seeds whose values are committed only as SHA-256 before Commit A (sealing route, `decision-method.md` §5.4), with forged-case seeds likewise sealed.

## Environment

`env_id` `wsl-ubuntu` (`VIRTUALIZED` irrelevant for allocator counts). Harness build with the `ebr-alloc` feature in `CARGO_TARGET_DIR=/root/eb-research/target/scale-alloc`; `lock_parity.py` and `harness-cli-parity.sh` must pass (review use constraint 1) and `internals-identity-check.sh` must pass with the trace hooks. Process-level `memcap_run.sh --cap 25769803776` safety cap; `peak_rss_bytes` (VmHWM of the isolated child) recorded as secondary corroboration. Windows secondary arm: the same probe at 1 and 4 GiB with job peak commit (§4.14), reported, never compared with Linux.

## Commands

```sh
wsl.exe -d Ubuntu -- bash research/harness/build.sh build --release --features ebr-alloc -p ebr-access -p ebr-pack
python research/harness/lock_parity.py && bash research/harness/harness-cli-parity.sh && bash research/harness/internals-identity-check.sh
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-SCALE-006/spec.yaml
# then verify-raw, normalize; analysis:
/root/eb-research/venv/bin/python research/tools/scale/bound_check.py --raw research/raw/EXP-SCALE-006 \
   --claims research/experiments/EXP-SCALE-006/claims.md --h research/experiments/EXP-SCALE-006/pre-validation-bound.md \
   --out research/normalized/EXP-SCALE-006/decision/
```

Probe command per sample (in the spec): `ebr-access --mode alloc-probe --operation <op> --cell {item_id} --cells <cells.json> --isolate true --limits <bootstrap|raised> --seed {seed} --scratch {scratch}` printing one `ebr.harness.raw.v1` row whose `payload` carries the metrics below.

## Seeds

`seeds.order` 1306001, `seeds.bootstrap` 1306002, `seeds.command` 1306003; generator and forge seeds in `cells.json`.

## Warmup

0 for allocator counts (exact); R0_max uses 20 runs by definition.

## Measurement method and metrics

| Payload key | Canonical name | Role | Family |
|---|---|---|---|
| `alloc_peak_bytes` | `alloc_peak_bytes` (T-06) | primary (graded, when compared with prototypes in EXP-SCALE-008) | memory_peak |
| `bounded_memory_growth_bytes` | `bounded_memory_growth_bytes` (HC-06, M08.3) | binary, computed per (operation, axis) by `bound_check.py` | correctness |
| `untrusted_alloc_before_check` | MVT-06(a) violation count | binary (= 0) | correctness |
| `refusal_alloc_peak_bytes`, `refusal_cpu_s` (in-process), `refusal_bytes_read` | T-06, T-05, T-02 | banded metrics recorded for OD-17 (status-quo row) | memory_peak, time_cpu, size |
| `adversarial_true_refusal_fraction` | HC-06 (M17.4) over the case list | binary (= 1) | correctness |
| `benign_false_refusal_fraction` | T-20 (M17.3) over the benign twins | graded | other |
| `declared_tightness_ratio` | T-23 (M17.1) for declared working set and decode window vs measured | graded, descriptive here | other |
| `outcome`, `reason_code` | none | DEC-ACC-036 enumeration | correctness |
| `argon2_invocations` | MVT-06(a) KDF oracle | binary (= 0 for over-ceiling) | correctness |
| `peak_rss_bytes` | `peak_rss_bytes` | secondary corroboration only | memory_peak |
| `site_top10` (dhat attribution, small sizes) | none | diagnostic (string, JSON) | other |
| `hostile_cpu_scaling_exponent` | M17.5 (descriptive) for `stream-huge-manifest` and `element-count-mismatch` scaling series 2^10..2^20 (MVT-06(d)) | descriptive plus MVT-06(d) binary rule | other |

## Replication and adaptive rule

- M08.3 probes: 3 runs per size (objective M08.3). **Interval note:** at n = 3 the exact order-statistic interval of §4.10 does not exist at 0.99 (n_min = 8, Appendix D.4); this protocol pre-registers the sample maximum x_(3) as the "upper bound" M08.3 requires and flags the ambiguity to the round-2 method review (it is the most conservative bound available at n = 3).
- R0_max: 20 runs. Adversarial cases: 3 runs each (deterministic outcomes, repeat-checked; allocator peak maximum over runs).
- MVT-06(d) scaling series: 3 runs per point, slope fitted as the objective specifies (0.99 interval, δ = 0.15).
- Adaptive: the 64 GiB and 10M-entry probes run only for (operation, axis) pairs not already falsified at 16 GiB / 1M entries (same screening logic as EXP-SCALE-001 stage 2, committed spec before the run).

## Normalization

Growth G = max over runs of `alloc_peak_bytes` at the largest size − R0 (R0 = allocator peak on the smallest probe per M08.3 definition; R0_max separately for the refusal oracle).

## Statistical analysis

- M08.3 per (operation, axis): claim holds iff G ≤ max(4 MiB, 0.10 × R0) for every run at every size; otherwise false, with the first failing size.
- Refusal oracle per case: exact integer comparisons against R0_max + H and against declared budgets (no band).
- DEC-ACC-036 table: case × (outcome class, reason code, whether the code names the policy field).
- Graded OD-17 metrics are reported per case (status-quo row) with no verdict; comparisons are EXP-SCALE-008's (T-06, T-05, T-02 with §4.6 rounds, in-process timers).
- MVT-06(d): violation iff the slope interval's lower bound exceeds 1.15.

## Practical-significance thresholds

Referenced: objective M08.3 tolerance; MVT-06(a) exact oracles; T-06, T-05, T-02, T-20, T-23 for graded refusal metrics. No restatement.

## Sensitivity analysis

- Allocator arm: `ebr-alloc` peak vs an independent `dhat` peak at 1 GiB (must agree within 1 MiB, otherwise the oracle is suspect and the run is invalid).
- Isolation arm: `--isolate true` vs `false` at 1 GiB (shows contamination size; only isolated values are used).
- Limits arm: bootstrap vs raised policy on `n999k` (raised policy must not change status-quo allocations below the bootstrap caps).

## Expected negative results worth recording

STREAM readers not bounded on the entries axis; POSIX-metadata STREAM verify retaining plaintext; INDEXED verify failing M08.3; late (EOF) detection of metadata-budget and decode-consistency violations with memory growing before refusal; `CORRUPT EB_RESOURCE_LIMIT` used for caller-policy violations; dedup index memory making 4M-chunk archives unpackable within modest caps.

## Threats to validity

1. The trace hooks change code generation in the research build (review R1-06); allocator counts are exact for the instrumented build, and the black-box RSS corroboration on the production build is reported beside them (objective §2.0 rule 9).
2. Forged archives cover a pre-registered enumeration only (regression evidence beyond it).
3. The pre-validation bound H is a FORMAL derivation that needs an independent reviewer; until signed, HC-06 stays HC_UNVERIFIED.
4. 10M-entry trees stress ext4 metadata and the VHDX; generation failures are infrastructure faults.

## Platform requirements

Linux x86-64 WSL2 root; about 260 GB; shared `ebr-alloc` crate (or the pinned `stats_alloc` fallback) and pinned `dhat` (downloads approved); Windows host for the secondary commit arm.

## Estimated compute and effort

About 60 h (64 GiB probes about 30 h; 10M-entry cells about 10 h; adversarial set about 5 h; dhat attribution about 5 h; builds and parity checks about 2 h). Tooling effort: judgment (allocator feature, trace hooks, forge generator, H derivation).

## Deviations (append-only)

None.
