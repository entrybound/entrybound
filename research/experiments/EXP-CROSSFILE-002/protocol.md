# EXP-CROSSFILE-002: Base physical order and clustering mode per profile (ratio, remote range counts, STREAM window)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: `ebr-crossfile order` with `--remote-tasks` (`research/harness/ebr-crossfile-DESIGN.md`) is not built |
| Domain / program sections | crossfile / 8.3 |
| decision_ids | DEC-CMP-006 (primary); informs DEC-ACC-003 (hot-set ordering, program §9/§12, not in this domain) |
| Evidence type | deterministic byte and request counts (§4.6, §4.13); modelled remote latency (§4.15); planning CPU in EXP-CROSSFILE-007 |
| Method | `research/decision-method.md`, pre-registered |
| Record | Design only; `record.md` (§12.2) is instantiated once gate G-A holds. No run before that commit. |
| Design session and disclosure | Phase C-design crossfile designer; L7 disclosure identical to EXP-CROSSFILE-001 (read `coverage.md` and `PROGRESS.md` held-out identifiers; no held-out content, listing or statistic opened). |

## 1. Question

Should profiles use the SPEC clustering modes (fast: no clustering, input order; balanced: cheap
ordering within directories and categories; dense/extreme: full similarity) or keep the frozen
digest-ordered base physical order with archive-wide bottom-k cohorts for balanced, dense and
extreme, given the effect of physical order on complete-cost bytes, on the number and size of
HTTP range requests for realistic remote tasks, and on the STREAM dependency window?

## 2. Hypotheses

- **H1.** Path-order base with cohorts placed at first use (`path-order-base`) is non-inferior on
  `artifact_bytes` (within the T-01 band at corpus level) and reduces `http_requests` for the
  directory-subtree extraction task (task T1) by at least 1 band unit of T-12 on F01, F03, F04
  and F18, because an entry's Chunks and its directory neighbours become contiguous and coalesce
  into fewer ranges. Predicted: 2 to 6 band units fewer requests on T1, 0 to 1 band unit on T2
  (random entries), no difference on T3 (sequential whole-archive read).
- **H0.** Physical order changes neither bytes nor request counts beyond the bands (Chunk frames
  are small relative to the coalescing gap), and R10 keeps the status quo.
- **Falsification of H1.** `path-order-base` is `DISTINGUISHABLE_WORSE` on `artifact_bytes`, or
  its T1 `http_requests` interval does not lie beyond the band on the favourable side on
  validation.

## 3. Decisions informed; proposed OD and HC sets

| Decision | Proposed type | Proposed od_ids | Proposed hc_ids |
|---|---|---|---|
| DEC-CMP-006 | EMPIRICAL | OD-05, OD-06, OD-12 (OD-09 secondary: STREAM order is owned by the access decision on STREAM frame order) | HC-09 (group contiguity and backward-only frame order), HC-13, HC-16, HC-02 |

Binding sets come from ledger schema v2 (gate G-A).

## 4. Candidates

All orderings keep canonical Entry order and ContentObject Chunk-reference order unchanged and
keep every accepted cohort contiguous in the order its lookback plan requires (the frame-order
dependency authority); only the placement of cohorts and of non-cohort Chunks differs. Each
non-status-quo ordering is a new planner identifier (HC-16).

| candidate_id | fast | balanced | dense / extreme | Expected tier |
|---|---|---|---|---|
| `sq-digest-cohorts` (status quo) | digest order, no cohorts | archive-wide cohorts by cohort ID, then digest order | same as balanced with profile policies | reference |
| `spec-clustering-modes` (SPEC) | input (canonical Entry) order, no cohorts | cheap ordering: (extension category, directory, path) order; cohorts restricted to Chunks within one directory or category | full similarity cohorts, placed at first use in path order | T1 |
| `path-order-base` | canonical Entry order | canonical Entry order; cohorts placed at the physical position of their first member's first use | same | T1 |
| `type-then-path` | digest order (unchanged) | order by (extension category, path) with archive-wide cohorts placed at first use | status quo | T1 |
| `cohort-only-dense-extreme` | status quo | no cohorts for balanced (digest order, independent Chunks) | status quo | T1 |
| `size-then-path` (designer-proposed alternative) | status quo | order non-cohort Chunks by (logical length bucket powers of two, path); cohorts at first use | same | T1 |
| `critic-TBD` | R0 item 6 slot | | | |

- **Reference candidate:** `sq-digest-cohorts` per profile.
- **Excluded before execution:** any ordering that places a lookback-group member after a later
  member or splits a group: R1 (HC-09, `eam/validate.rs` contiguity and backward-only
  dependencies; objective MVT-09(a)); any in-place change of v3-v6 frozen physical order: L0,
  HC-16.
- Cohort formation uses `sq-bottomk-v1`; if EXP-CROSSFILE-001 selects a different similarity
  method before this experiment's validation look, the look is re-registered with that method
  as a documented change (§5.2 rules apply).

## 5. Corpus selectors

- Manifest `research/corpus/manifest.json` (`ed28b1b4…`), tuning split only.
- **Tuning (`spec.yaml`):** the same 50-item cross-file target set as EXP-CROSSFILE-001 (rule
  stated there), all four profiles for the orderings that differ per profile.
- **Remote task set (pre-registered, evaluation condition "user task", never averaged):**
  - T1 directory subtree: 10 directories per item with >= 5 regular files, chosen by a seeded
    draw (SHA-256(`EXP-CROSSFILE-002 T1\0` || item_id)) over directories sorted by path bytes;
    the task reads every regular file under the directory. Items with fewer than 5 qualifying
    directories use all qualifying directories; items with none skip T1.
  - T2 random entries: 32 entries by the seeded draw of EXP-CROSSFILE-005's `entry` stratum.
  - T3 sequential: every regular file in canonical Entry order.
- Network profiles: NP-LAN, NP-METRO, NP-CONT, NP-INTER (§4.15), modelled latency only; the
  emulated corroboration is out of scope here (request and byte counts are exact and are the
  primary quantities).
- **Validation look:** validation split, all families small and medium, plus the four large
  items named in EXP-CROSSFILE-001 §5; W and S only, after the MDE gate.

## 6. Environment and platform requirements

`wsl-ubuntu`, x86-64 Linux, ext4 under `/root/eb-research`; loopback HTTP only (a child
`ebr-origin` bound to 127.0.0.1 per sample, no proxy, so counts are exact and not
emulation-dependent); no privilege; `timing: false`. Prerequisites as EXP-CROSSFILE-001 §6,
plus decision-method §14 item 18 (`EXP-NETEM-CAL` connection model per profile) before modelled
latency is reported as decision-grade.

## 7. Commands

`ebr validate|plan|run|verify-raw|normalize|report --spec research/experiments/EXP-CROSSFILE-002/spec.yaml`
exactly as EXP-CROSSFILE-001 §7. Per-sample command:

```text
/root/eb-research/target/harness/release/ebr-crossfile order
  --item-path {item_path} --item-id {item_id} --experiment-id EXP-CROSSFILE-002 --env-name wsl-ubuntu
  --profile {profile} --order {order} --archive-out {scratch}/out.eb
  --stream-layout true --remote-tasks true --origin-bin /root/eb-research/target/harness/release/ebr-origin
  --verify-roundtrip true
```

`--remote-tasks true` opens the archive through `entrybound::random_access::HttpRangeSource`
against the child origin with the default `RandomAccessPolicy` (the coalescing gap is recorded
in every row), runs T1-T3 in fresh sessions, and computes the §4.15 latency model per profile
from the exact request/byte log, once with the RFC 6928 slow-start model and once without
(harness review R1-14 sensitivity).

## 8. Seeds, warmups, repetitions

Seeds `order` 20260917011, `bootstrap` 20260917012, `command` 20260917013; task draws as in §5.
Warmups 0. Repetitions 2 plus the repeat-hash check on `archive_sha256` and on the request-log
SHA-256 (`request_log_sha256`); a request-log mismatch under identical archive bytes is a
determinism defect of the reader (reported to the remote domain), not an ordering effect.

## 9. Metrics

| Metric | OD | Role | ebr family | Threshold | Source |
|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | **primary** | size | T-01 | `payload.artifact_bytes` |
| `http_requests` per task T1, T2, T3 | OD-12 | **primary** (per task condition) | other | T-12 | `payload.http_requests_t1` etc. |
| `http_bytes` per task | OD-12 | secondary | size | T-11 | `payload.http_bytes_t1` etc. |
| `remote_task_latency_s` per task and profile, modelled | OD-12 | secondary (modelled; emulated corroboration not run) | time_wall | T-10 | `payload.remote_task_latency_s_*` |
| `artifact_bytes` of STREAM with window `auto` | OD-09 | secondary (M09.2 comparison within an ordering) | size | T-01 | `payload.stream_artifact_bytes` |
| `stream_window_required_bytes` | OD-09 | descriptive | size | none | `payload.stream_window_required_bytes` |
| `encode_cpu_s` | OD-06 | primary, measured in EXP-CROSSFILE-007 | time_cpu | T-05 | EXP-CROSSFILE-007 |
| `roundtrip_ok`, `drift_ok`, `archive_sha256`, `request_log_sha256` | HC-02, HC-13 | binary / repeat-hash | correctness | §3.3, §4.13 | `payload.*` |

## 10. Normalization and statistical analysis

As EXP-CROSSFILE-001 §10, with `http_requests` analysed per task (AGG-S: the task is an
evaluation condition; R4 and R8 apply per task and no task average exists). Absolute floor of
T-12 applies at item level (integer counts). Profile-scope question: if DEC-CMP-006 is classified
profile-scope, Appendix B.1 applies (superiority on `artifact_bytes` only; the request counts
become R9 partition evidence and secondary). The decision can then be decided per profile
(R9 partition 1), which is expressible because profiles already own their ordering policy.

## 11. Practical-significance thresholds

T-01, T-10, T-11, T-12, T-05 by reference (`metric_thresholds.<name>`); the T-10 floor uses the
profile RTT rule.

## 12. Sensitivity analysis

decision-method §6 in full; plus (a) slow-start model on versus off (R1-14); (b) coalescing gap
x0.5 and x2 (the default is a caller policy, so an ordering that wins only at one gap is
recorded as policy-dependent); (c) T1 directory draw with a second pre-registered seed
(20260917014).

## 13. Validation look and held-out placeholder

Validation look as EXP-CROSSFILE-001 §13. Held-out placeholder `EXP-CROSSFILE-002-HO` at Commit
A (decision-method §5.5), all frozen candidates, once, report-only on failure,
`identity_aware_heldout` cap unless sealed tranches are used.

## 14. Expected negative results worth recording

- Ordering has no practical byte effect (cohorts already contiguous; Zstandard frames independent).
- `path-order-base` helps T1 only on many-small-file families and is neutral elsewhere.
- Cheap balanced ordering loses dictionary opportunities that archive-wide cohorts found (F18
  version series spread across directories).
- STREAM window needs are dominated by exact-dedup back references, not ordering.

## 15. Threats to validity

- Loopback HTTP has no latency; counts are exact but latency is modelled (§4.15) and the model
  depends on the declared connection model (gate §14 item 18).
- The default `RandomAccessPolicy` coalescing gap is itself an open remote-domain decision;
  sensitivity (b) exposes dependence.
- Placement "at first use" is ambiguous for Chunks shared by several objects; the tool uses the
  first use in canonical Entry order and records the rule.
- Same corpus and L7 threats as EXP-CROSSFILE-001.

## 16. Cost

About 60 CPU-hours (cohort plans are memoised within this experiment's own per-repetition memo
directory `/root/eb-research/cache/crossfile/EXP-CROSSFILE-002/rep{rep}`, never shared with
another experiment), wall about 4 h sharded; disk: scratch about 3 GB, memo about 3 GB, raw about
1 GB. Agent
effort: none for execution; scripted tooling; judgment for analysis.
