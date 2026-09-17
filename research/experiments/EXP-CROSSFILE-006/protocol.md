# EXP-CROSSFILE-006: Production-archive access cost, declaration honesty and fault injection (measured validation of the dependency models)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: `ebr-crossfile access` and `ebr-crossfile inject` plus the counting allocator (`research/harness/ebr-crossfile-DESIGN.md`) are not built |
| Domain / program sections | crossfile / 8.5 (8.4 for dictionaries) |
| decision_ids | DEC-ACC-001 (primary: predicted versus measured cost); DEC-ACC-025 (measured transitive closure and decoder memory for long groups); DEC-INT-002 (measured blast radius and localization); DEC-CON-004 (declared working set versus measured decode memory); DEC-CMP-027 (measured OD-08 and OD-14 strata, and validity control 4 for EXP-CROSSFILE-004); DEC-CMP-020 (measured dictionary fetch and memory) |
| Evidence type | exact measured byte/request counts per read (local and HTTP), exact in-process allocator peaks, seeded fault injection with a read-every-entry oracle; all deterministic (§4.6, §4.13, §4.14) |
| Method | `research/decision-method.md`, pre-registered |
| Record | Design only; `record.md` (§12.2) instantiated once gate G-A holds; HC-06 records additionally need §14 item 17 (allocation-site attribution and the pre-validation bound H) before they count as MVT-06(a) evidence. |
| Design session and disclosure | Phase C-design crossfile designer; L7 disclosure as in EXP-CROSSFILE-001. |

## 1. Question

On archives the production reader actually decodes (production profiles, and production-encodable
forced lookback/dictionary configurations from EXP-CROSSFILE-003/-004), what are the measured
per-read decoded bytes, dependency Chunks, fetched bytes and range requests (memory, local file,
HTTP), the measured decode memory, and the measured unreadable entries and bytes after seeded
corruption in each region stratum; and do the declared requirements and each candidate cost model
and blast-radius definition predict them soundly (never under) and tightly?

## 2. Hypotheses

- **H1a (DEC-ACC-001).** Only `decode-work-model` and `dual-local-remote-model` have
  `declared_cost_accuracy` <= 1 on every measured read; `status-quo-chunks-and-lookback`
  under-predicts decoded bytes for lookback groups longer than `max_lookback + 1`, and
  `fetch-model` alone under-predicts local decode work for Dictionary users.
- **H1b (DEC-ACC-025).** For the 64- and 128-member stress groups, the measured
  `dependency_chunk_count` of the last member equals its transitive closure (not `max_lookback`),
  and its decode `alloc_peak_bytes` exceeds the declared working set when the closure plaintext
  exceeds `max_preceding_bytes` (HC-06 violation of the status-quo declaration).
- **H1c (DEC-INT-002).** Injection results equal EXP-CROSSFILE-004's exact analytic unreadable
  sets on every payload and dictionary injection (`analytic_model_agreement` = 1), so the
  analytic blast radius can stand for research-only topologies.
- **H1d (DEC-CON-004).** The production declared `working_set_bytes` is an upper bound on every
  measured decode peak but loose by more than the T-23 band on dictionary-heavy archives.
- **H0.** Declarations and the status-quo model are sound and within the T-23 band on every read.
- **Falsification.** H1a/H1b: no measured read exceeds its status-quo declaration. H1c: any
  injection whose measured unreadable set differs from the analytic set (then validity control 4
  fails and EXP-CROSSFILE-004's modelled values stay informational). H1d: a measured peak above the
  declaration (HC-06 violation, a stronger negative result) or tightness within band everywhere.

## 3. Decisions informed; proposed OD and HC sets

As EXP-CROSSFILE-004 §3 for DEC-CMP-027, DEC-ACC-025, DEC-INT-002, DEC-ACC-001, DEC-CON-004; as
EXP-CROSSFILE-003 §3 for DEC-CMP-020.

## 4. Candidates (archive configurations and models under test)

| candidate_id | Archive written by | Purpose |
|---|---|---|
| `prod-{balanced,dense,extreme}` | `ebr-pack --profile <p> --layout indexed` (byte-identical to `ebound pack`, `internals-identity-check.md`) | status quo |
| `forced-chain-k{1,2,4,8}-w1m-{dense,extreme}` | `ebr-crossfile topology --topology chain-k<k>-w1m` (production-encodable) | measured closure and blast radius per lookback |
| `forced-cap-group-len{4,8}` | `ebr-crossfile topology` (production-encodable when L <= k + 1) | capped-group validation |
| `forced-dict-sq-abs128`, `forced-dict-charge-access-k1`, `forced-no-dictionaries` | `ebr-crossfile dict-sweep` (production-encodable rules) | dictionary fetch and memory |
| `iterative-bounded-decoder` | same archives; research decoder variant (`ebr-crossfile access --decoder iterative-bounded`) | DEC-ACC-025 decoder candidate (memory) |

Models and definitions evaluated against measurements: the five DEC-ACC-001 cost models, the three
DEC-ACC-025 declaration models, the six DEC-INT-002 blast definitions, and the three DEC-CON-004
working-set formulas (EXP-CROSSFILE-003 §10) — each predicted per read or per injection by the
tool and compared with the measured oracle.

Reference: `prod-<profile>` status-quo declarations and model. No candidate is excluded before
execution (all are production-decodable).

## 5. Corpus selectors

- Manifest `ed28b1b4…`, tuning only.
- **Access (`spec.yaml`, candidates `access-*`):** the 50-item cross-file target set of
  EXP-CROSSFILE-001 §5, with the shared EXP-CROSSFILE-005 access sample and the EXP-CROSSFILE-002
  T2 draw, plus the three long-group stress items of EXP-CROSSFILE-004 §5.
- **Injection (`spec-inject.yaml`, candidates `inject-*`):** the 20 small items of the target set (whole
  archive re-read per injection is affordable only on small archives) plus the stress items;
  >= 200 single-byte corruptions per region stratum per archive (Appendix C.1), strata: payload,
  Index, metadata and manifest, dictionaries (when present), integrity records and footer;
  secondary classes 512 B, 64 KiB, 1 MiB overwrites and truncation at section boundaries and at
  seeded offsets.
- **Validation look:** validation split small items for injection, small and medium for access;
  after the MDE gate; W and S only.

## 6. Environment and platform requirements

`wsl-ubuntu`, x86-64 Linux, ext4; loopback child `ebr-origin` for HTTP counts (exact, no proxy);
`timing: false`; counting allocator with one decode per process for per-read peaks (harness
review use constraint 3); `--probe-isolate` style child processes for peak isolation. No privilege.
The `verify` localization report comes from the production CLI built from the production
workspace (`ebound verify --json` if available at the pre-registration commit; otherwise the
library `VerificationReport`), recorded per row.

## 7. Commands

`ebr validate|plan|run|verify-raw|normalize|report` with `--spec research/experiments/EXP-CROSSFILE-006/spec.yaml` (access) and then `--spec research/experiments/EXP-CROSSFILE-006/spec-inject.yaml` (injection)
as EXP-CROSSFILE-001 §7. Per-sample commands:

```text
# access candidates
/root/eb-research/target/harness/release/ebr-crossfile access
  --item-path {item_path} --item-id {item_id} --experiment-id EXP-CROSSFILE-006 --env-name wsl-ubuntu
  --build {build} --profile {profile} --build-args {build_args} --synthetic-from-item-id true --archive-out {scratch}/out.eb
  --sources memory,local-file,http --origin-bin /root/eb-research/target/harness/release/ebr-origin
  --reads access-sample-v1,t2 --decoder {decoder} --models all --alloc-isolate true

# inject candidates
/root/eb-research/target/harness/release/ebr-crossfile inject
  --item-path {item_path} --item-id {item_id} --experiment-id EXP-CROSSFILE-006 --env-name wsl-ubuntu
  --build {build} --profile {profile} --build-args {build_args} --synthetic-from-item-id true --archive-out {scratch}/out.eb
  --strata payload,index,metadata,dictionary,integrity --events 200
  --classes byte,512b,64k,1m,truncate --seed {seed} --oracle read-every-entry
  --verify-cli /root/eb-research/target/prod-cli/release/ebound --compare-analytic true
```

`--build ebr-pack` delegates archive creation to `ebr-pack` (status quo); `--build topology` and
`--build dict-sweep` reuse the EXP-CROSSFILE-004/-003 research encoders restricted to
production-encodable configurations (the tool refuses others).

## 8. Seeds, warmups, repetitions

Seeds `order` 20260917061, `bootstrap` 20260917062, `command` 20260917063; injection positions
from SHA-256(`EXP-CROSSFILE-006 inject\0` || item_id || stratum || class || event index).
Warmups 0. Repetitions 2 plus repeat-hash on `archive_sha256`, `read_log_sha256` and
`injection_outcomes_sha256`. Detection: every injection must be detected (`detection_rate` = 1,
§3.3, I21); a single undetected corruption is an R1 violation record for that configuration.

## 9. Metrics

| Metric | OD | Role | ebr family | Threshold | Source |
|---|---|---|---|---|---|
| `declared_tightness_ratio` (per model, per quantity: decoded bytes, dependency Chunks, fetched bytes, requests, working set) | OD-17 | **primary** (DEC-ACC-001; DEC-CON-004 for working set) | other | T-23 | `payload.*tightness*` |
| `declared_cost_accuracy` (per model and declaration) | HC-09 / HC-06 | binary | correctness | §3.3 | `payload.*accuracy*` |
| `alloc_peak_bytes` (decode, per read) | OD-08 | **primary** (DEC-CMP-027, DEC-ACC-025 decoder candidates) | memory_peak | T-06 | `payload.alloc_peak_bytes_per_read` |
| `access_amplification` (measured, production API: whole-entry reads) | OD-10 | secondary corroboration of EXP-CROSSFILE-004 | other | T-22 | `payload.*` |
| `http_bytes`, `http_requests` per read | OD-12 | secondary corroboration of EXP-CROSSFILE-004 models | size / other | T-11 / T-12 | `payload.*` |
| `blast_logical_bytes_p95`, `blast_logical_bytes_max`, `blast_entries_p95` per stratum and headline | OD-14 | **primary** measured strata for DEC-CMP-027 (joined with EXP-CROSSFILE-004 payload analytics) | other | T-15 | `payload.*` |
| `localization_recall` | HC-15 | binary | correctness | §3.3 | `payload.localization_recall` |
| `localization_precision` | OD-14 | **primary** (DEC-INT-002; case set = the pre-registered injections) | other | T-20 | `payload.localization_precision` |
| `truncation_salvage_fraction` | OD-14 | secondary | other | T-20 | `payload.*` |
| `detection_rate` | HC-15 (I21) | binary | correctness | §3.3 | `payload.detection_rate` |
| `analytic_model_agreement` | validity control 4 | binary gate for EXP-CROSSFILE-004 modelled values | correctness | exact | `payload.analytic_model_agreement` |
| `dependency_chunk_count_per_read`, `decoded_logical_bytes_per_read` (p50, p95, max) | - | descriptive | other | none | `payload.*` |
| `benign_false_refusal_fraction` (default `RandomAccessPolicy` and default decode budget) | OD-17 | secondary | other | T-20 | `payload.*` |

## 10. Normalization and statistical analysis

As EXP-CROSSFILE-001 §10, with:
- R1 first: any configuration with a detection failure, any model or declaration with
  `declared_cost_accuracy` > 1 on any read, any blast definition with recall < 1 is eliminated
  for that pair; eliminations are recorded with the violating (item, read or injection) as the
  committed violation artifact (objective §2.0 rule 2).
- Survivors are compared on `declared_tightness_ratio` (T-23; minimize toward 1) at corpus level;
  item value = the maximum over reads for soundness, the median over reads for tightness (both
  reported; the median is the graded value, the maximum is the R1 screen).
- `measured-calibration` constants are fitted on tuning reads only and frozen before the
  validation look; its validation tightness is the confirmatory value.
- Blast radius: region-stratified, equal stratum weights 0.2 headline, worst-stratum guard
  (objective OD-14), per item p95 and max over events; joined with EXP-CROSSFILE-004 per
  configuration.
- The DEC-ACC-001 reporting-usability part ("pack-time and inspect reporting usability check") is
  HUMAN-FACING and is not measured here; only the mechanically checkable part (the formula's
  soundness and tightness) is.

## 11. Practical-significance thresholds

T-06, T-11, T-12, T-15, T-20, T-22, T-23 by reference.

## 12. Sensitivity analysis

decision-method §6 where a selection exists; plus (a) source kind: memory versus local file versus
HTTP (fetch models must hold on all three; conditions, never averaged); (b) coalescing gap x0.5 and
x2; (c) injection classes separately (byte versus burst versus truncation); (d) default versus a
32 KiB `max_cached_bytes` session cache (cache warmth changes per-read deltas).

## 13. Validation look and held-out placeholder

As EXP-CROSSFILE-001 §13 (`EXP-CROSSFILE-006-HO` at Commit A; single freeze).

## 14. Expected negative results worth recording

- The status-quo declaration and access-cost report under-state transitive work (REQ-ACC-0001
  `CONTRADICTED` reproduced as a committed violation artifact).
- The production working set over-declares dictionary-heavy archives so much that a strict caller
  policy would refuse benign archives.
- `verify` localization reports only directly damaged entries and misses dependents (HC-15
  violation) or reports whole groups (low precision).
- The analytic model disagrees with measurement on some stratum (validity control 4 fails),
  blocking EXP-CROSSFILE-004's modelled comparisons.

## 15. Threats to validity

- Per-read counts depend on session cache state; each read runs in a fresh session unless the
  sensitivity arm says otherwise.
- The allocator oracle measures heap only; codec-internal `mmap` would escape it (recorded if
  `/proc/self/status` VmHWM exceeds the allocator peak by more than the T-06 absolute floor).
- The read-every-entry oracle defines "unreadable" as a failed verified read; salvage semantics are
  the integrity domain's (program §24) and are only reported here.
- `ebr-pack` byte identity with `ebound` is proven for a fixed generated tree
  (`internals-identity-check.md`); validity control 1 extends it per item.
- Corpus and L7 threats as EXP-CROSSFILE-001.

## 16. Cost

About 150 CPU-hours (injection re-reads dominate: about 20 archives x 5 strata x 200 events x
5 classes x whole-archive reads, on small items), wall about 8 h sharded; disk: scratch about 2 GB,
raw about 6 GB (per-event rows, gzip). Agent effort: none for execution; scripted tooling;
judgment for analysis.

## 17. Evidence these decisions need that this experiment does not produce

- **DEC-ACC-001:** reporting usability (HUMAN-FACING, evaluation domain).
- **DEC-CON-004:** flag-registry and unknown-bit behaviour and the constrained-reader use-case study
  (container domain, program §11).
- **DEC-INT-002:** salvage semantics and verify/salvage/repair report design (integrity domain,
  program §24); this experiment reports localization only as far as `verify` exposes it.
