# EXP-INT-004: Verification levels, continue-on-error damage maps and container-digest pre-checks: detection coverage, cost and hostile amplification

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T5 `ebr-int-verify-levels`, T11(d) reader options, damage corpus from EXP-INT-001/002 tooling) |
| Domain | integrity (program §24; objective OD-07, OD-08, OD-17, HC-06, HC-15) |
| Decisions informed | DEC-INT-043, DEC-INT-007, DEC-INT-028 |
| Gates, method, author | conventions C0; the timing arm additionally needs G-B timing gates (§14 items 3 and 4: runner additions, `EXP-CAL-*` calibration for `wsl-ubuntu` and `windows-host` in families time_wall, memory_peak) |
| Specs | `spec-coverage.yaml` (deterministic coverage and hostile arm, `timing: false`); `spec.yaml` (timing arm, WSL); the Windows timing arm spec is generated from `spec.yaml` when §14 item 3 runner additions exist |

## 1. Question

For each candidate verification level or mode, what share of a pre-registered damage case
set does it detect, what does it cost in wall time, memory and bytes read on realistic and
large archives, and does collecting every failure (instead of stopping at the first) stay
within declared bounds on hostile multi-fault archives?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-INT-043 | Section-digest-without-decode (SPEC default candidate) detects every D-P1 event in every stratum of plaintext INDEXED archives (all stored bytes are under section digests), at `verify_wall_s` lower than full decode by more than 1 band unit of T-04 on medium and large items; it does not detect encoder-produced wrong-but-digest-consistent payloads (EXP-INT-014 fault set), which only full decode detects. | Section-digest level misses any D-P1 event, or is not `DISTINGUISHABLE_BETTER` on `verify_wall_s` at corpus level. |
| H2 | DEC-INT-043 | The stored-bytes-digest level (wire-change stand-in) detects payload damage per chunk without decode at a cost between section-digest and full decode. | Its cost is not distinguishable from full decode. |
| H3 | DEC-INT-007 | Continue-and-collect with a bounded diagnostic list stays within the declared memory bound on hostile archives with 2^10 to 2^20 corrupted chunks (`bounded_memory_growth_bytes` passes), and its CPU scales linearly (`hostile_cpu_scaling_exponent` interval lower bound at most 1.15, the MVT-06(d) criterion). | Any bound exceedance, or superlinear scaling. |
| H4 | DEC-INT-028 | Hashing exact container bytes against an expected digest before parsing costs less than full decode verify by more than 1 band unit of T-04 on every tier, and refuses a mismatching hostile archive with zero parser allocation (`refusal_alloc_peak_bytes` at baseline), whereas parse-first verification spends parser memory and CPU before refusal. | Digest-first not cheaper, or parse-first refusal cost inside the T-06 band of digest-first. |

## 3. Decisions and candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-INT-043 | `status-quo-single-full-verify` (B-PROD `ebound verify`), `spec-default-plus-deep` (T5 levels `section-digest` as default and `full-decode` as `--deep`), `three-levels` (T5 `structure`, `section-digest`, `full-decode`), `full-plus-keyless-public` (plaintext part identical to status quo; the keyless public level is measured in EXP-INT-016 and cross-referenced), `unpack-verify-only-alias` (B-PROD `ebound unpack` to a discarding sink versus `verify`: cost of the unpack path without writes, T5 option), `stored-byte-digest-level` (T5 with the research per-frame stored-bytes digest map; the 32 B per frame wire cost is exact and labelled MODEL), `manifest-only-verify` (T5: verify an extracted directory against the archive manifest by hashing files, no decode) |
| DEC-INT-007 | `status-quo-fail-fast`, `verify-continue-collect`, `verify-damage-map`, `separate-damage-report-mode`, `index-guided-localisation` | Accuracy is EXP-INT-001; here: cost on benign archives (timing arm) and resource amplification on hostile multi-fault archives (coverage arm). |
| DEC-INT-028 | `status-quo-compute-only`, `expected-pci-option` (compare after full verify: cost = full verify + comparison), `digest-before-parse` (T5 `container-digest` level), `signed-pci-binding` (crypto wire change; cost of signature verification modelled from EXP-INT-009 signature timings and marked MODEL), `external-receipt-pci` (same measured cost as `digest-before-parse`), `drop-pci-validation-wording` (documentation only; cost C1) | Parser-differential analysis: for every F20 hostile item and every EXP-INT-001 D-S7 structural event, compare the refusal cost of parse-first against digest-first given the pristine archive's digest. Staleness of PCI claims under rewrite is EXP-INT-009. |

## 4. Corpus

- **Coverage arm (`spec-coverage.yaml`):** the damage case set is the D-P1, D-S2, D-S4, D-S6,
  D-S7 and D-T1 events generated for INT-CORE-T under `eb-indexed-balanced` and
  `eb-stream-balanced` (same C8 seeds as EXP-INT-001, so events are identical), plus the
  EXP-INT-014 encoder-fault archives when that tooling exists (otherwise that class is a
  recorded gap). Hostile multi-fault archives are generated from INT-SMALL-T items by T2 with
  2^k corrupted chunks, k = 0..20 capped at the item's chunk count, and from INT-HOSTILE-T.
- **Timing arm (`spec.yaml`):** INT-CORE-T plus INT-LARGE-T, packed once per configuration
  and stored outside scratch so that every level reads the same bytes.
- Validation look: INT-CORE-V plus INT-LARGE-V and INT-HOSTILE-V for W and S.

## 5. Environment and platform requirements

- Coverage arm: `wsl-ubuntu`, `timing: false`, parallel.
- Timing arm: `wsl-ubuntu` with affinity `st-v` (vCPU 2), `VIRTUALIZED` qualifier, exclusive
  quiet window, guard `max_loadavg_1m` 2.0 (1.0 + 1 thread), `max_other_cpu_percent` 3.0,
  `max_wait_s` 300; `windows-host` arm with `st-p` once runner additions exist. A universal
  claim needs same-direction support in both environments (§4.1); until the Windows arm is
  decision-grade, a Linux-only scope is recorded.
- Cache state `hot` (inputs read in warmups); `vm-cold` sensitivity arm on WSL for large items
  (no cold-cache claim, §4.5).
- Requires B-PROD (timing uses the production build, harness constraint 2) and the T5
  prototypes built from B-RES; a **timing parity check** between B-PROD `ebound verify` and T5
  `full-decode` on the same inputs must show `EQUIVALENT` on `verify_wall_s` before any T5
  level is compared with a B-PROD level; otherwise only T5-versus-T5 comparisons are made.

## 6. Procedure and commands

1. Preconditions: harness constraints (C1), calibration experiments passed, disk guard.
2. Coverage arm: `ebr validate/run/verify-raw/normalize/report` on `spec-coverage.yaml`,
   then `research/tools/integrity/verify_detail.py --experiment EXP-INT-004`.
3. Timing arm: in a scheduled quiet window,
   `python -m ebr run --spec research/experiments/EXP-INT-004/spec.yaml --timing`, repeated
   with `--run-id` steps under the adaptive rule (§7), then `verify-raw`, `normalize`, `report`.
4. Decision analysis (C10).

## 7. Seeds, warmups, replication

- Seeds: coverage 2026091731/32/33; timing 2026091734/35/36.
- Coverage arm: warmups 0, 2 repetitions, repeat-hash (C10).
- Timing arm: warmups 2 (small, medium) and 1 (large); minimum rounds 10 per tier with
  n_min(c) for the comparison's confidence (§4.6); adaptive rule: after the minimum, add one
  step (10 small and medium, 5 large) for any item whose exact order-statistic half-width of
  the median paired log ratio exceeds 0.5 x log1p(r) for T-04; caps 30, 20, 20. The runner is
  invoked per step with a new `--run-id`; the precision computation is done by the decision
  tooling and never inspects effect direction.

## 8. Metrics

| Metric | ID | OD | Role | Source |
|---|---|---|---|---|
| `verify_wall_s` | T-04 | OD-07 | **primary** (DEC-INT-043, DEC-INT-007 benign cost, DEC-INT-028) | process samples (medium, large); T5 in-process timer primary on small tier (§3.2 small-tier rule) |
| `verify_throughput_bps` | T-05b | OD-07 | secondary (reciprocal of the primary) | derived |
| `peak_rss_bytes` | T-06 | OD-08 | **primary** for OD-08 | GNU time via `resource.max_rss_bytes` (harness constraint 3, R1-16) |
| `alloc_peak_bytes` | T-06 | OD-08 | secondary (library-level decode memory; T5 counting allocator when §14 item 17 exists) | in-process |
| `damage_detection_fraction` | C11 proposed | OD-15/OD-14 as assigned | secondary until named; detection must be reported honestly per level | coverage over the case set |
| `mvt15a_overclaim_cells` | C11 | HC-15 | binary: a level that reports more than it checked | coverage arm |
| `bounded_memory_growth_bytes` | HC-06 | OD-08 floor | binary for continue-and-collect | coverage arm, counting allocator |
| `hostile_cpu_scaling_exponent` | descriptive | OD-17 | descriptive (MVT-06(d) fit reported) | coverage arm, in-process CPU counters, 3 runs per point |
| `refusal_cpu_s`, `refusal_alloc_peak_bytes`, `refusal_bytes_read` | T-05, T-06, T-02 | OD-17 | **primary** for DEC-INT-028 OD-17 (one of them, `refusal_alloc_peak_bytes`; the others secondary) | in-process |
| bytes read per level | T-19 `fs_read_ops` | - | secondary only (page-cache dependent) | resource |

## 9. Normalization

Timing and memory: per-round paired log ratios against `status-quo-full` within the same
round (pairing `item_repetition`), item median, floors per T-04 and T-06, aggregation per C10.
Coverage: exact fractions per item, differences against the status quo. Scaling exponent: OLS
slope of ln CPU on ln corrupted-chunk count with a 0.99 interval (MVT-06(d)).

## 10. Statistical analysis and sensitivity

Per C10. Scale tier is a stratum reported separately (the level decision may differ by tier;
R9 partitions by tier are not expressible as product behaviour unless the level is chosen by
the user, so tier differences are reported as limitations). Sensitivity: §6 plus a cache-state
arm (`hot` versus `vm-cold`) and an environment arm (WSL versus Windows) when both exist.

## 11. Expected negative results worth recording

- Section-digest verification misses decoder-level faults that full decode catches, so a fast
  default level weakens what "verified" means for encoder bugs.
- Full decode may dominate cost on highly compressed large items, making the status quo
  distinguishably worse on OD-07.
- Continue-and-collect may exceed bounds when every chunk fails, unless the diagnostic list is
  capped.
- Digest-before-parse requires the caller to hold a digest; without one it adds nothing.

## 12. Threats to validity

- T5 prototypes are research implementations; the timing parity check against B-PROD bounds
  code-generation differences (harness constraint 2, R1-06) but not future production code.
- WSL timing is `VIRTUALIZED` (soft cap MEDIUM for Linux-native claims, §4.1); Windows timing
  is not decision-grade until §14 item 3 exists.
- The stored-bytes digest level uses a sidecar map to emulate a wire field; I/O patterns of an
  in-archive field may differ.
- The damage case set is only as representative as EXP-INT-001's classes.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12; the timing arm's held-out run uses the same quiet-window protocol.

## 14. Cost estimates

- Coverage arm: about 30 CPU-hours (events reuse EXP-INT-001 generation), hostile scaling series
  about 10 CPU-hours.
- Timing arm: 8 level configurations x 23 items x 10-30 rounds, verify times from about 0.1 s to
  about 40 s plus cooldowns: about 8 hours of exclusive wall time per environment for tuning and
  about the same for the validation look.
- Disk: packed archives about 10 GB; hostile series about 5 GB; detail about 1 GB.
- Agent effort: tooling build scripted with review; execution none (scheduled windows);
  analysis judgment.

## 15. Deviations (append-only)

None.
