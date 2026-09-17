# EXP-REMOTE-002: Status-quo remote access trace census, declared-cost accuracy and fetch-simulator validation

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T1 `ebr-remote`, T2, T3 header bytes, T5 `fetchsim`, T6 archive preparation). A reduced status-quo arm is runnable today with the production CLI (`ebound read <url> <path> --access-report` prints the full access trace), see section 7. |
| Domain / program section | remote / §12 (also §8.5 and §10 for DEC-ACC-001, §23 for DEC-ACC-038) |
| decision_ids | DEC-ACC-001, DEC-ACC-025, DEC-ACC-038 |
| Decision type (provisional; independent assignment per §1.2) | DEC-ACC-001 EMPIRICAL + FORMAL (formula); DEC-ACC-025 EMPIRICAL + FORMAL; DEC-ACC-038 FORMAL (API scope) with EMPIRICAL refusal census |
| OD / HC (provisional until G-A) | OD-10 (M10.2, M10.3, M10.4), OD-12 (M12.1-M12.3), OD-17 (M17.1); HC-09 (MVT-09(a),(b)), HC-15 (verification state never overstated), HC-06 (declared bounds) |
| Role for the domain | Foundation: produces the exact status-quo request logs, the archive layout maps, the per-condition modelled latencies and the validated simulator that EXP-REMOTE-003..012 reuse. It also defines the domain tooling (section 7) and the shared workloads (`workloads.json`). |
| Author / independence / L7 | As EXP-REMOTE-001 (same session; `coverage.md` identity exposure disclosed there). |
| Gates | G-A unmet at design time; §14 item 18 (EXP-REMOTE-001) must pass before any emulated number is reported; deterministic trace and model metrics do not depend on it. |

## 1. Question

For the production random-access client (baseline `9e44608` behaviour, `crates/entrybound/src/ecf/random.rs`, `crypto/random.rs`, `random_access.rs`), exactly how many HTTP requests and bytes, and how much modelled latency, does each workload of `workloads.json` cost across profiles, encryption and Index states on the tuning corpus; how accurately do the candidate access-cost declaration models predict measured decode and fetch work; how far do measured dependency closures exceed the declared ChunkGroup bounds; and which entry kinds does remote random access serve or refuse?

## 2. Hypotheses

- **H1 (reconciliation, FORMAL check).** For every sample, `http_requests` from the origin log equals the session's `range_request_count` plus the number of revision checks (`HttpRangeSource::open` HEAD, `RangeSession::new` revision, and one `check_stable` HEAD per `open` and per `read_entry`; `random_access.rs:204-259`, `random.rs:296,601`), and response body bytes equal the session's `bytes_fetched`. Falsified by one mismatch; a mismatch invalidates every count metric of the domain until explained.
- **H2 (declared cost, DEC-ACC-001).** The status-quo declaration (`max_lookback`, `max_preceding_bytes`, dictionaries counted independent) understates measured decode work (measured/declared > 1) on at least one dense or extreme archive with ChunkGroups of 8 or more members, while `decode-work-model` and `fetch-model` never understate (their `declared_cost_accuracy` = 1 on every item) and have a median `declared_tightness_ratio` within 1 + r of 1 (T-23 r). Falsified if status quo never understates on any tuning item (then REQ-ACC-0001's CONTRADICTED state is not reproduced), or if either transitive model understates anywhere.
- **H3 (closure, DEC-ACC-025).** For the last member of a ChunkGroup of length L with registered lookback k, the measured `dependency_chunk_count` equals L − 1 (chained prefixes decode transitively) rather than k. Falsified by any group where it equals k.
- **H4 (simulator validation).** `fetchsim` reproduces the production request sequence (offset, length, purpose, cache hit, HEAD positions) exactly for the status-quo strategy and every production `RandomAccessPolicy` value used in EXP-REMOTE-003/008, on 100% of (item, candidate, workload) cells. Falsified by one mismatch.
- **H0.** Otherwise.

## 3. Decisions informed

| Decision | What this experiment supplies | Remaining evidence and where |
|---|---|---|
| DEC-ACC-001 | Predicted-vs-measured cost for the five candidate models on local and HTTP traces (primary `declared_tightness_ratio`, binary `declared_cost_accuracy`) | "Pack-time and inspect reporting usability check": HUMAN-FACING, routed to ecosystem §33 simulated sessions and external review; calibrated-constant confirmation on validation (second look) |
| DEC-ACC-025 | Status-quo measured `dependency_chunk_count`, decoded bytes and remote cost for last members of dense/extreme groups; the declared-vs-transitive gap | Alternative closure rules (caps, resets, non-transitive prefixes) and their remote cost: EXP-REMOTE-011; ratio impact: crossfile domain (§8.5); planner-ID impact: FORMAL (frozen planner IDs, HC-16) |
| DEC-ACC-038 | Refusal/serve census per entry kind and archive role over memory, local and HTTP sources (outcome class, reason code, bytes and requests spent before refusal) | Serving hardlink members or reparse metadata requires an API change: its fetch cost is formally that of the shared ContentObject (FORMALLY_DERIVED from Entry/ContentObject indirection); base-chain cost for non-Complete roles: integrity domain §23 |

## 4. Candidates

### 4.1 Archive configurations measured (census axes, not competing candidates)

| config id | Pack command (production `ebound` built from the production workspace per harness review use constraint 2) | Items |
|---|---|---|
| `fast-plain` | `ebound pack <tree> <out.eb> --profile fast` | all tuning |
| `balanced-plain` | `--profile balanced` (REF-PROD) | all tuning |
| `dense-plain` | `--profile dense` | tuning small + medium |
| `extreme-plain` | `--profile extreme` | tuning small + medium |
| `balanced-plain-noindex` | balanced planned archive encoded with `WriteOptions { include_index: false }` via `ebr-pack --include-index false` (T6) | all tuning |
| `balanced-enc-bucketed` | `--profile balanced --password --pad bucketed` (test passphrase from the WSL environment variable `EB_RESEARCH_TEST_PASSWORD`, research-only, never committed) | tuning small + medium |
| `balanced-enc-max` | `--pad max` | tuning small + medium |
| `balanced-enc-none` | `--pad none` | tuning small + medium |

Configurations added by later remote experiments through the same T6 recipe: `balanced-stream` (EXP-REMOTE-005), `balanced-plain-signed` and `balanced-enc-bucketed-signed` (EXP-REMOTE-010).

Plain archives are deterministic; each is packed once and its SHA-256 recorded (determinism itself is tested by the scale/platform domains). Encrypted archives use fresh randomness (keyed boundaries and nonces differ per pack), so each repetition packs anew and encrypted count metrics are treated as stochastic (§4.13 reclassification).

### 4.2 DEC-ACC-001 cost-declaration models (competing candidates)

| candidate_id | Predicted cost for reading entry e | Computed from |
|---|---|---|
| `status-quo-chunks-and-lookback` | decoded chunks = chunks(e) + max_lookback; decoded bytes = logical(e) + max_preceding_bytes; dictionaries independent; fetched ranges heuristic = chunks(e) | Descriptor/preamble declarations and ChunkGroup records |
| `decode-work-model` | worst decoded bytes and chunks over the transitive closure of groups, reconstruction regions and dictionaries | archive declarations evaluated transitively (formula committed in `models.py`) |
| `fetch-model` | worst fetched bytes and ranges including metadata sections, dictionaries, section headers and frame headers | layout map + declarations |
| `dual-local-remote-model` | the pair (decode-work-model, fetch-model) reported separately | as above |
| `measured-calibration` | linear model of fetched bytes and requests on declared quantities, constants fitted by least squares on tuning traces, published with the tuning 99th-percentile error bound | tuning traces; frozen before any validation look |

Reference candidate: `status-quo-chunks-and-lookback`. No candidate excluded. Designer note (not the R0 item 6 critic candidate): the completeness critic should consider a `closure-exact-per-entry` model that stores per-entry exact costs (a wire change, T2 or higher).

## 5. Corpus selectors

- `research/corpus/manifest.json`, split `tuning` only for this record (all 112 tuning items for fast/balanced/noindex configs; the 95 small+medium tuning items for dense, extreme and encrypted configs). Exact item lists are generated by `research/experiments/EXP-REMOTE-002/make_item_lists.py` from manifest fields `split`, `scale`, `item_id` only, and committed as `items-tuning-all.txt`, `items-tuning-small-medium.txt` and the matching ebr manifests `manifest-tuning-small-medium.jsonl` (rows copied from `research/corpus/manifest.json`) before the run. Later experiments' seeded subsets (`subset-manifest.jsonl`) are produced by the shared `research/tools/remote/make_subset.py` from the same manifest fields plus `file_count` and `bytes`, never from corpus statistics.
- Items that production `ebound pack` refuses (for example non-UTF-8 names or ACL-mask disagreement, `research/baselines/README.md` findings) are recorded as `pack_refused` with outcome and code and excluded from count metrics; they still count toward the DEC-ACC-038 refusal census.
- Minimum support check (§4.9): tuning has ≥ 2 groups in every family (manifest group counts), so corpus-level verdicts on tuning are supported; the applicable-family list per workload is computed from `workloads.json` applicability.
- Validation: a second spec (`spec-validation.yaml`) is generated only at a registered validation look (§5.2) for DEC-ACC-001 (W and S from tuning) using the same generator over split `validation`. Held-out: section 15.

## 6. Environment and platform requirements

- `wsl-ubuntu`; all deterministic metrics (counts, bytes, modelled latency) are platform-independent given identical archive bytes; timing is not collected (`timing: false`), except the informational `wall_s`.
- A Windows-host arm packs the same trees on NTFS for 10 seeded tuning items and compares traces (F-0001/F-0002 predict different AUX/feature bytes, hence possibly different section sizes): reported as a platform stratum, not pooled.
- Linux x86-64; no privileged operations; loopback network only; disk for cached archives (section 16).
- Production binary: `ebound` built from the production workspace at the pinned research commit with default features, `CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release --locked -p entrybound-cli`; `research/harness/harness-cli-parity.sh` must print `RESULT PASS` before the run (harness review use constraint 1).

**Platform matrix.** Linux x86-64 required (WSL2); Windows x86-64 for the 10-item packing platform arm; macOS: PLATFORM_BLOCKED (not in scope for this census; F-0001/F-0002 show host-dependent bytes, so macOS-packed layouts remain unmeasured); ARM64: not required (counts are architecture-independent given identical archive bytes; emulated arm64 determinism belongs to the platform domain); network: loopback; privileged: no; large storage: yes (~95 GB on D:); external review: no.

## 7. Tooling and commands (domain-level definitions reused by EXP-REMOTE-003..012)

| ID | Tool | Contract |
|---|---|---|
| T1 | `ebr-remote` (new crate `research/harness/crates/ebr-remote`, no `unsafe`) | `layout --archive A` emits the exact layout map (sections with offsets and lengths, Chunk frames with offset, header length, stored length, chunk id, plan, group, region flag; ContentObject → chunk lists; ChunkGroups with max_lookback and max_preceding_bytes; dictionaries; encrypted segments with offsets, extents and classes; declared budgets). `trace --archive A --workloads workloads.json --source memory|http-loopback|http-emulated|local-file --policy <json>` runs the workloads through the production API and emits per-operation traces (`AccessTraceEntry` list plus source-level `read_exact_at`/revision calls), outcome classes and codes, bytes returned with their SHA-256 checked against the tree, and per-condition modelled latency via T2. `emulate` replays or runs workloads through an emulator. `sim` runs `fetchsim` strategies. Output: one `ebr.harness.raw.v1` JSON line on stdout plus a gzip detail file whose SHA-256 is in the line. Every input path passes `ebr_common::heldout::assert_inputs_not_heldout`. |
| T2 | Latency model twins | `connection-model.json` algorithm in Rust (`ebr_netem::model`) and Python (`research/tools/remote/latency_model.py`), cross-checked (EXP-REMOTE-001) |
| T3 | Origin/proxy extensions | header-byte logging, shared-link shaping, fault injection modes (EXP-REMOTE-009), `--profiles` calibration |
| T4 | Source-layer prototypes | `RandomReadSource` wrappers in `ebr-remote` implementing candidate client behaviours without touching production: `ReadaheadSource`, `TailWindowSource`, `RevalidateOnceSource`, `LayoutPrefetchSource` (cross-purpose and header+payload merges, multipart ranges, planned parallel fetch), `CachePolicySource` (FIFO/LRU/2Q/ARC/adjacent-merge/size-scaled/persistent), `RetryingSource`. HTTP-level counts for these come from the origin log, never from session counters. |
| T5 | `fetchsim` (`research/tools/remote/fetchsim.py`, stdlib only) | Given a layout map, a workload and a strategy, emits the request sequence the strategy issues, including cache behaviour; status-quo strategy mirrors `random.rs`/`random_access.rs` logic line by line with citations. Validated by H4 before any simulated-only candidate result is used. |
| T6 | `research/tools/remote/prepare_archives.py` | Packs items with production `ebound` (and `ebr-pack` for `--include-index false`, Index corruption and section edits), caches under `/root/eb-research/cache/remote-archives/<item_id>/<config>/`, records SHA-256, `ebound inspect --json`, and the T1 layout map; refuses held-out paths. |
| T7 | `ebr-tunem` | kernel-TCP loopback emulator (EXP-REMOTE-001) |
| T8 | Incumbent remote readers | EXP-REMOTE-012 |
| T9 | `proofsim.py` | proof-structure calculator (EXP-REMOTE-007) |
| T10 | revision mutator harness | EXP-REMOTE-014 |
| T11 | Decision-analysis tooling | program-wide `decision-method.md` §14 item 2; not remote-specific; a gate for every decision-grade verdict |

Reduced arm runnable today (status quo, plain archives, informational until T1-T3 exist): `ebound pack`, `ebr-origin --log`, `ebound read http://127.0.0.1:<port>/x.eb <path> --access-report` parsed by a script; the Phase C-design feasibility smoke (non-decision-grade, tiny generated tree, 2026-09-17) confirmed the trace prints purposes and cache hits and that the origin log separates HEAD from ranged GET requests.

Run (WSL):

```sh
cd /mnt/d/Projects/entrybound/entrybound
python research/orchestration/disk_guard.py && python research/harness/lock_parity.py
PY=/root/eb-research/venv/bin/python; export PYTHONPATH=research/tools
$PY research/experiments/EXP-REMOTE-002/make_item_lists.py --manifest research/corpus/manifest.json --split tuning
$PY research/tools/remote/prepare_archives.py ensure --items research/experiments/EXP-REMOTE-002/items-tuning-all.txt \
    --configs fast-plain,balanced-plain,balanced-plain-noindex
$PY research/tools/remote/prepare_archives.py ensure --items research/experiments/EXP-REMOTE-002/items-tuning-small-medium.txt \
    --configs dense-plain,extreme-plain
for S in spec.yaml spec-dense-extreme.yaml spec-encrypted.yaml; do
  $PY -m ebr validate --spec research/experiments/EXP-REMOTE-002/$S
  $PY -m ebr run --spec research/experiments/EXP-REMOTE-002/$S
  $PY -m ebr verify-raw --spec research/experiments/EXP-REMOTE-002/$S
  $PY -m ebr normalize --spec research/experiments/EXP-REMOTE-002/$S
done
$PY research/tools/remote/fetchsim.py validate --traces /root/eb-research/raw-detail/EXP-REMOTE-002 \
    --out research/normalized/EXP-REMOTE-002/decision/fetchsim-validation.json
$PY research/experiments/EXP-REMOTE-002/analyze_cost_models.py --normalized research/normalized/EXP-REMOTE-002 \
    --out research/normalized/EXP-REMOTE-002/decision/
```

`spec.yaml` (fast, balanced, noindex over all tuning items), `spec-dense-extreme.yaml` and `spec-encrypted.yaml` (small+medium lists) share the command template; only `spec.yaml` is committed at design time, the other two are generated from it by `make_item_lists.py --emit-specs` (candidate and item substitutions only) so they cannot diverge.

## 8. Seeds, warmups, repetitions

- Seeds: `order` 20260920, `bootstrap` 20260921, `command` 20260922 (workload seed rule in `workloads.json`).
- Warmups: 0 (deterministic metrics; no timing).
- Repetitions: 2 for plain configs (different rounds and processes, §4.6) with the repeat-hash check on `trace_sha256` (§4.13): the two traces must be identical; a mismatch for a plain archive is a nondeterminism artifact of the client (HC-03/HC-11 matter, committed per objective §2.0 rule 2). Encrypted configs: 3 repetitions, each with a fresh pack.
- Adaptive rule: not applicable (exact values).

## 9. Metrics

| Metric (canonical; spec suffix `__<workload>`/`__<profile>`) | OD | Role here | Threshold ID | Source |
|---|---|---|---|---|
| `declared_tightness_ratio` (per model, worst and median over entries) | OD-17 | **primary** for DEC-ACC-001 | T-23 | T1 layout + trace |
| `declared_cost_accuracy` (per model) | OD-10 | binary (HC-09 MVT-09(b)) | §3.3 | trace |
| `http_requests` | OD-12 | primary for census reporting of M12.1 | T-12 | origin log |
| `http_bytes` | OD-12 | secondary | T-11 | origin log |
| `open_bytes_read` | OD-10 | diagnostic | T-11 | session delta for WL-OPEN |
| `remote_first_entry_latency_s`, `remote_random_entry_latency_s`, `remote_task_latency_s` (modelled, CM-H1-TLS-KA, SS-RFC6928 primary arm; other arms in detail) | OD-12 | reported per profile | T-10 | T2 |
| `access_amplification` per T-22 stratum (WL-RANGE) | OD-10 | reported | T-22 | trace (decoded bytes / requested bytes) |
| `verification_digest_ops` | OD-11 | diagnostic | T-12 | trace (chunk digests + roots) |
| `dependency_chunk_count_max`, decoded bytes of last group members | OD-10 | diagnostic for DEC-ACC-025 | T-12 / T-09 | trace |
| `reconcile_ok`, `roundtrip_ok`, `sim_trace_mismatches` | – | correctness gates (H1, H4, objective §3.0 correctness gating) | §3.3 | T1/T5 |
| `trace_sha256` | – | repeat-hash | §4.13 | T1 |
| refusal census: outcome class and reason code per entry kind × source, `refusal_bytes_read`, `http_requests` before refusal | OD-17 | reported (DEC-ACC-038) | T-02, T-12 | trace |

At most one primary per OD holds per decision: DEC-ACC-001 uses only `declared_tightness_ratio` (OD-17) plus the binary; DEC-ACC-025 and DEC-ACC-038 have no primary here (supporting evidence; their selections run in EXP-REMOTE-011 or are FORMAL).

## 10. Normalization and statistical analysis

- Deterministic metrics: item value = exact value (plain) or median over key replicates (encrypted). Aggregation L2-L4 with the cited workload mix (`research/methods/workload-mix.json`, gate G-B) and equal family weights as a sensitivity arm; strata per scale tier, per workload (user task) and per profile; never averaged across conditions.
- DEC-ACC-001: for each model, per item the worst ratio over entries (AGG-W, the HC-09 floor) and the median ratio (graded, T-23); confirmatory comparisons on validation only: W vs each survivor at corpus level, `NON_INFERIOR`/superiority per §4.11-§4.12 via decision tooling T11; R4 cost condition uses computed tiers: status quo and `measured-calibration` are T1 (reporting/planner-level) unless they need new reason codes or declaration fields (then T2); `decode-work-model`/`fetch-model` need declaration semantics changes of access-cost fields: tier assessed by an independent session before the first validation look (§7.2).
- DEC-ACC-025 (status quo characterization): per group-length bin {1, 2-8, 9-64, 65-128, >128}, distribution of measured/declared closure; any measured > declared on a Complete archive is recorded as an HC-09 violation artifact of the status quo (objective §2.0 rule 5: a defect, not a relaxation).
- H1/H4: exact equality; failures reported by item and workload.

## 11. Practical significance

T-23 for `declared_tightness_ratio`; T-12, T-11, T-10, T-22 for reported metrics; binaries per §3.3. Referenced only (`thresholds.json#metric_thresholds`).

## 12. Sensitivity analysis

- Connection models CM-H1-PLAIN-KA and CM-H1-TLS-FRESH and slow-start arms SS-NONE, SS-IDLE-RESET, SS-IW4 for every modelled latency (detail files).
- Source arm: memory vs http-loopback vs local-file traces must give identical session ranges (source independence of the fetch plan); any difference is reported as a finding.
- Platform arm (Windows-packed archives, 10 items).
- For DEC-ACC-001: leave-one-family-out, family Dirichlet and tier-stratified arms of §6.5 on validation; `measured-calibration` refitted with leave-one-group-out to estimate its out-of-sample error (reported as its published bound candidate).

## 13. Expected negative results worth recording

- HEAD revalidation requests are absent from the library's own `range_request_count`, so the report understates HTTP requests (expected from code; H1 quantifies).
- Chunk frame headers are fetched one request per Chunk before payload prefetch (`random.rs:754-790`), so many-small-file workloads cost roughly twice as many ranges as Chunks (expected; motivates EXP-REMOTE-003 header+payload merging).
- Index-less archives trigger a whole CHUNK_DATA header scan of one request per frame (`random.rs:692-752`).
- Status-quo lookback declarations understate transitive decode (REQ-ACC-0001 CONTRADICTED) if H2/H3 hold.
- Encrypted open costs one request per SegmentHeader (`crypto/random.rs:1202-1245`), linear in segment count.

## 14. Threats to validity

- Archives packed on WSL only (except the platform arm); section sizes depend on captured metadata (F-0001/F-0002).
- Workload file selection is synthetic (uniform/Zipf/subtree) rather than recorded real traces ([INFERRED] representativeness); WL-SUBTREE and WL-ZIPF approximate package installs and dataset sampling.
- `measured-calibration` fitted on tuning may overfit family mixes; validation look required.
- Loopback origin header sizes are ebr-origin's; real servers differ (EXP-REMOTE-009 P5-style probe on nginx/Apache/Caddy).
- The research build of `ebr-remote` enables `research-internals`; counts and bytes are unaffected (identity proof covers bytes; traces are deterministic protocol behaviour), and no timing is taken here.

## 15. Held-out (Phase D placeholder)

After the single program-wide freeze (`decision-method.md` §5.5, Commits A-C), `spec-heldout.yaml` (generated by the same `make_item_lists.py` over split `heldout` with `EB_HELDOUT_UNLOCK` set) reruns the frozen DEC-ACC-001 model set once, report-only per §5.6, with the held-out MDE recorded before unlock (§R5). Nothing in this design reads held-out items.

## 16. Estimates

- Compute: about 14 machine-hours (packing about 9.7 GiB medium + 26 GiB large tuning items under fast/balanced/noindex, small+medium under dense/extreme and three encrypted configs; traces are fast; WL-SEQALL and WL-FRACTION-1.0 decode whole archives).
- Disk: about 90 GB peak for cached archives (regenerable; plain archives deduplicated by SHA-256), under 5 GB of trace detail outside git, under 50 MB of committed per-condition summaries (gzip JSONL; if larger, kept under `/root/eb-research/raw-detail` with a committed SHA-256 manifest).
- Agent effort: scripted; judgment for DEC-ACC-001 model tier assessment and the analysis write-up.
