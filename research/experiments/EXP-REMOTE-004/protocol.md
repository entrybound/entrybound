# EXP-REMOTE-004: Remote open path — metadata discovery, revalidation, and Index/Manifest structure fetch cost

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T1, T2, T4 `TailWindowSource`/`RevalidateOnceSource`/persistent cache, T5, T6, plus `metasim.py` structure encoders and the scale-series generator of section 5) |
| Domain / program section | remote / §12 (with §11 container for DEC-CON-015/018/028 and DEC-ACC-018/019) |
| decision_ids | DEC-ACC-018, DEC-ACC-019, DEC-CON-015, DEC-CON-018, DEC-CON-028 |
| Decision type (provisional) | DEC-ACC-018 EMPIRICAL (+ FORMAL wire-cost analysis for locator tables); DEC-ACC-019 EMPIRICAL + FORMAL (Index doctrine); DEC-CON-015/018/028 EMPIRICAL (remote leg only here) |
| OD / HC (provisional until G-A) | OD-12 (M12.1-M12.3), OD-10 (M10.3 open cost), OD-08 guard (client metadata memory), OD-05 guard (metadata bytes, supplied by container domain); HC-01 (Index is a cache: I1, A4), HC-18 (revision safety of revalidation candidates), HC-16 (format revisions allocate new identifiers) |
| Author / L7 / gates | As EXP-REMOTE-001; needs EXP-REMOTE-002 H4 pass (simulated candidates) and EXP-REMOTE-001 pass (emulated numbers). Coordinates with the container domain: its structure byte counts, parser memory and fuzz robustness are the complementary evidence; this experiment consumes the same generated scale trees where the container domain commits them, else its own (section 5). |

## 1. Question

How many requests, bytes and how much modelled latency does a remote reader spend to open an INDEXED archive and resolve its first and subsequent entries, as a function of section count, entry count (10^3 to 10^7), Chunk count and network profile, under each metadata-discovery and revalidation strategy and each candidate Index and Manifest representation; and does the Index's presence and validation doctrine change open and first-read cost?

## 2. Hypotheses

- **H1 (DEC-ACC-018, discovery).** The status-quo header walk costs one request per section header plus Descriptor, Manifest and Index ranges plus four HEAD revalidations per open-and-read (`random.rs:296,601`, `random_access.rs:204-259`); `speculative-head-tail-window` (256 KiB) and `revalidate-once-per-session` each reduce `remote_first_entry_latency_s` by at least 1 band unit at NP-CONT and NP-INTER for archives with ≤ 10^5 entries, and are `NON_INFERIOR` on `http_bytes` at NP-LAN.
- **H2 (DEC-ACC-018/DEC-CON-018, scale).** Above about 10^5 entries, whole-Manifest fetch dominates open latency at NP-INTER and paged or segmented manifests become `DISTINGUISHABLE_BETTER` for WL-FIRST, while WL-OPEN-with-list (full enumeration) favours whole-section compression.
- **H3 (DEC-ACC-019).** An absent or invalid Index makes WL-FIRST cost scale with total Chunk count (one frame-header request per frame, `random.rs:692-752`), so `status-quo-lazy-validation` is `DISTINGUISHABLE_BETTER` than `remove-index-v1` and `always-cross-check` on every archive with more than a few hundred Chunks at every profile.
- **H4 (DEC-CON-015, remote leg).** For cold single-entry lookups at ≥ 10^5 Chunks, structures that allow partial fetch (paged two-level, fanout + sorted digest table, per-ContentObject locator runs) beat the whole-section 100-byte-record status quo on `remote_first_entry_latency_s` at NP-INTER, while at ≤ 10^4 Chunks all are `EQUIVALENT`.
- **H0 / falsification.** Each Hi is falsified by the absence of the stated verdict in its stated scope (for H3 by any archive above 1,000 Chunks where the Index-less path is not `DISTINGUISHABLE_WORSE`).

## 3. Decisions informed

| Decision | Supplied here | Remaining |
|---|---|---|
| DEC-ACC-018 (V1_BLOCKING) | Cold/warm request counts and latency for open and first entry vs section count, entry count 10^3-10^7 and RTT; Manifest and Index bytes per entry/Chunk and whole-vs-paged fetch latency; cost of per-operation HEAD; suffix-range feasibility against ebr-origin | Server support for suffix ranges and If-Match on GET across real servers: EXP-REMOTE-009 (local servers), EXP-REMOTE-013 (cloud, BLOCKED); wire-cost/compatibility of locator tables or paged manifests: container domain + independent format review (FORMAL, T4 tier assessment) |
| DEC-ACC-019 | Open/first-read latency and requests with and without Index, locally and over HTTP; `always-cross-check` cost | Hostile-Index suite: EXP-REMOTE-008; wire-cost of digest-bound Index: container domain |
| DEC-CON-015 | Requests and bytes to resolve an entry to frame ranges remotely, warm and cold, at 10^3-4·10^6 Chunks per Index structure; cacheability of Index pages (fraction of page bytes reused across WL-RANDK16 lookups) | Index bytes per Chunk, lookup CPU, rebuild cost, independent-reader effort and fuzz robustness: container and conformance domains |
| DEC-CON-018 | Open latency, list and single-entry lookup cost remotely (bytes, requests, RTT) per manifest representation at 10^3-10^7 entries; client peak metadata memory for the status quo | Manifest bytes per entry on real trees, compression ratio, parser robustness, inspectability: container domain |
| DEC-CON-028 | Remote bytes and requests to open and list per trigger candidate; the manifest-share-vs-entry-count curve over the corpus and scale series | Reader-compatibility cost of an incompat bit and transparency: container domain (FORMAL/qualitative) |

## 4. Candidates

### 4.1 Discovery and revalidation (DEC-ACC-018)

| candidate_id | Definition | Realization |
|---|---|---|
| `status-quo-header-walk` (reference) | HEAD at source open and session start; footer, preamble, one 64 B range per section header, whole Descriptor, Manifest and Index; HEAD `check_stable` after open and after every read | production client |
| `speculative-tail-window-<w>`, w ∈ {64 KiB, 256 KiB, 1 MiB} | first range = last w bytes (covers footer and, section order permitting, Manifest/Fidelity/Index headers and payloads, `container.rs:175-275` order); later reads served from the containing cache entry | `TailWindowSource` (T4) + `fetchsim` |
| `speculative-head-tail-window-<w>` (designer-proposed) | as above plus the first w bytes (preamble, Descriptor, TransformPlans and small extended sections) | `TailWindowSource` + `fetchsim` |
| `coalesced-header-prefetch` | when a section header declares a payload ≤ 64 KiB, fetch the next header together with that payload (readahead bounded by declared lengths) | `ReadaheadSource` bounded variant + `fetchsim` |
| `footer-section-locator-table` (format revision) | footer carries (kind, offset, length, digest) per section; open = one tail range of footer + table, then direct coalesced section fetches | `fetchsim` over the real layout with the table's size computed from the section count; FORMAL wire-cost to container domain |
| `paged-manifest-and-index` | Manifest and Index split in pages of P ∈ {64 KiB, 1 MiB} with a page directory keyed by first LogicalPath / first digest; open fetches directories only; lookups fetch one page each | `metasim.py` encodes directories and pages from the real Manifest/Index bytes (page boundaries on record boundaries) + `fetchsim` |
| `metadata-first-layout` | metadata sections placed immediately after the preamble; open = one range covering preamble through Index when their total is ≤ 1 MiB, else header walk over the prefix | `fetchsim` over a permuted layout (sizes unchanged) |
| `revalidate-once-per-session` | revision checked once at session start; every range carries `If-Match` (as today) so a mid-session change yields 412; no per-operation HEAD | `RevalidateOnceSource` + `fetchsim`; HC-18 churn test in EXP-REMOTE-009 |
| `suffix-range-open` (designer-proposed; related to DEC-ACC-015 `get-range-probe`) | no initial HEAD: first request `Range: bytes=-w` returns length (Content-Range total) and strong ETag with the tail | `TailWindowSource` variant against `ebr-origin` + `fetchsim` |
| `persistent-metadata-cache` | verified Descriptor/Manifest/Index keyed by (URL, strong ETag, length) stored across sessions; warm open = one conditional request | cache wrapper + `fetchsim` on WL-REVISIT |
| critic slot | R0 item 6 | before first look |

### 4.2 Index role (DEC-ACC-019)

| candidate_id | Definition | Realization |
|---|---|---|
| `status-quo-lazy-validation` (reference) | locators trusted after bounds/uniqueness checks; each used locator checked against its frame header; rebuild on mismatch | production |
| `always-cross-check` | rebuild the locator map from all frame headers at open and compare | `fetchsim` (full header scan) |
| `digest-bound-index` | Index digest bound into an authenticated root; one check then trust | `fetchsim` (fetch cost as status quo minus per-Chunk header requests where the header is not otherwise needed) |
| `per-record-validation` | validate each used locator against its frame header only | equals status quo fetch pattern (recorded as such) |
| `remove-index-v1` | no Index; header scan | production with `balanced-plain-noindex` archives |
| `authoritative-index` | Index trusted without validation | **Proposed L0 exclusion** (objective HC-01 statement: indexes are caches; A4): needs a written counterexample and sign-off by an independent session before it is recorded; its fetch cost equals `digest-bound-index` and is reported for completeness only |

### 4.3 Index structure (DEC-CON-015, remote leg)

`sorted-record-list-status-quo` (reference), `fanout-plus-sorted-oid-table`, `paged-two-level-index` (pages 64 KiB), `dual-coordinate-seek-table`, `sizes-sister-list`, `delta-compressed-locators` (zstd level 19 over delta-encoded offsets, measured by actual compression of the real Index content), `per-content-object-locator-runs`, `no-index-scan-only`, `authoritative-index` (see 4.2). Each is encoded by `metasim.py` from the real Chunk layout according to a byte-layout description committed with the tool (field widths declared; e.g. fanout 256 × 4 B, digest 32 B, offset 8 B), so sizes are exact for the declared encoding.

### 4.4 Manifest representation (DEC-CON-018, remote leg) and trigger (DEC-CON-028)

Representations: `flat-uncompressed-status-quo` (reference), `flat-with-compact-field-framing` (varint lengths; re-encoded exactly from the real record stream), `compact-manifest-whole-section-compressed` (zstd levels 3 and 19 of the real section), `segmented-compressed-manifest` (independently compressed segments of 64 KiB and 1 MiB plaintext with a seek table), `paged-manifest-with-locator-table`, `path-indexed-tree` (fanout over the first path byte plus sorted path offsets), `columnar-bitpacked-manifest` (size taken from the container domain's encoder output if committed; otherwise recorded as unmeasured here), `external-manifest-sidecar` (same bytes, one extra object: +1 session setup and HEAD).
Triggers (DEC-CON-028): `always-uncompressed-status-quo`, `profile-dependent` (dense/extreme compressed), `entry-count-threshold-<E>` E ∈ {10^4, 10^5, 10^6}, `manifest-share-threshold-<s>` s ∈ {0.01, 0.05, 0.10}, `always-compact`, `uncompressed-plus-prologue` (prologue bytes = container domain's definition; fetch cost = +prologue bytes), `caller-opt-in-only` (= status quo unless opted in; evaluated as both).

## 5. Corpus selectors

- Tuning corpus archives from EXP-REMOTE-002 (`fast-plain`, `balanced-plain`, `balanced-plain-noindex`, `dense-plain`, `extreme-plain`), all tuning items.
- **Scale series (generated, tuning role):** `research/experiments/EXP-REMOTE-004/gen_scale_trees.py` builds trees with N ∈ {10^3, 10^4, 10^5, 10^6} regular files (file sizes drawn from a seeded log-normal with median 1 KiB and a 64 MiB total cap per 10^6 files; path depth 1-8; directory fan-out 16-256; seeds 4001-4004 and a second independent group with seeds 4101-4104 so each N has two independence groups), materialized under `/root/eb-research/generated/EXP-REMOTE-004/` on ext4, packed `balanced-plain` and `balanced-plain-noindex`, listed in `scale-manifest.jsonl` (item_id, split `tuning`, family `SCALE-META`, path, generator, seed, SHA-256) committed before the run. N = 10^7 is not materialized (about 40 GiB of inodes): its Manifest and Index sizes are computed by `metasim.py synth` from the per-record encoding rules applied to a seeded synthetic EntrySet with the same distributions, labelled `[FORMALLY_DERIVED extrapolation]`, and its fetch plans are simulated only. If the container domain commits million-entry trees first, their manifest is used instead and recorded.
- Validation look: the W vs S comparisons over `validation` corpus archives plus a validation scale series with seeds 4201-4204. Held-out: section 15.

## 6. Environment and platform requirements

`wsl-ubuntu`; deterministic counts, bytes and modelled latency (no timing) for every candidate; emulated corroboration of W vs the reference at NP-CONT and NP-INTER on the E12 subset of EXP-REMOTE-003 plus the 10^5 and 10^6 scale archives (quiet window); local-file `open_latency_s` in-process batches for the status quo and Index-less arms (timing window, OD-10 guard). Linux x86-64; about 12 GB of ext4 for the 10^6 trees (two groups); no privileges.

**Platform matrix.** Linux x86-64 required (WSL2); Windows: not used; macOS: not applicable; ARM64: not required; network: loopback emulation; privileged: no; large storage: ~25 GB; external review: format-revision candidates need the container domain's FORMAL wire-cost review (internal, independent session), not external experts.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound; PY=/root/eb-research/venv/bin/python; export PYTHONPATH=research/tools
python research/orchestration/disk_guard.py && python research/harness/lock_parity.py
$PY research/experiments/EXP-REMOTE-004/gen_scale_trees.py --out /root/eb-research/generated/EXP-REMOTE-004 --manifest research/experiments/EXP-REMOTE-004/scale-manifest.jsonl
$PY research/tools/remote/prepare_archives.py ensure --manifest research/experiments/EXP-REMOTE-004/scale-manifest.jsonl --configs balanced-plain,balanced-plain-noindex
$PY -m ebr run --spec research/experiments/EXP-REMOTE-004/spec.yaml            # production + source-layer prototypes, scale series
$PY -m ebr verify-raw --spec research/experiments/EXP-REMOTE-004/spec.yaml && $PY -m ebr normalize --spec research/experiments/EXP-REMOTE-004/spec.yaml
$PY research/tools/remote/metasim.py encode --layouts /root/eb-research/cache/remote-archives --structures research/experiments/EXP-REMOTE-004/structures.json --out /root/eb-research/raw-detail/EXP-REMOTE-004/structures
$PY research/tools/remote/fetchsim.py sweep --strategies research/experiments/EXP-REMOTE-004/strategies.json --structures /root/eb-research/raw-detail/EXP-REMOTE-004/structures \
    --workloads research/experiments/EXP-REMOTE-002/workloads.json --connection-model research/experiments/EXP-REMOTE-001/connection-model.json --out research/raw/EXP-REMOTE-004/sim/
$PY -m decide analyze --experiment EXP-REMOTE-004
```

A corpus-item variant spec (`spec-corpus.yaml`, generated by substituting the corpus selector) runs the same candidates over tuning corpus archives.

## 8. Seeds, warmups, repetitions

Seeds `order` 20260926, `bootstrap` 20260927, `command` 20260928; scale generator seeds as in section 5. Deterministic arms: 2 repetitions + trace repeat-hash. Emulated and local timing arms: warmups 2 (1 for the 10^6 archives, large-tier rule), rounds min 10 / step 10 / cap 20 (medium) with the precision rule.

## 9. Metrics

| Metric | OD | Role | Threshold |
|---|---|---|---|
| `remote_first_entry_latency_s` per profile (WL-FIRST, cold) — modelled | OD-12 | **primary** (DEC-ACC-018, -019, CON-015, CON-018 remote legs) | T-10 |
| `open_bytes_read` (WL-OPEN) | OD-10 | **primary** for OD-10 in DEC-ACC-018 and DEC-CON-028 (one per OD) | T-11 |
| `http_requests`, `http_bytes` per workload | OD-12 | secondary | T-12, T-11 |
| `remote_task_latency_s` for WL-RANDK16 (warm lookups) and WL-OPEN (open + full list) | OD-12 | secondary (condition views) | T-10 |
| `open_latency_s` (local-file, in-process batches) | OD-10 | guard | T-08 |
| `alloc_peak_bytes` of the client during open (status quo, scale series; counting allocator §14 item 17 when available, else scoped RSS labelled) | OD-08 | guard (secondary) | T-06 |
| `index_bytes`, `metadata_bytes` per candidate structure (from `metasim.py`) | OD-05 | diagnostic (container domain owns the primary) | T-02 |
| `page_reuse_fraction` (bytes of fetched pages reused across lookups) | OD-12 | diagnostic (secondary; cacheability) | – |
| `roundtrip_ok`, `report_mismatches` (prototype vs status quo outputs and verification reports) | – | binary | §3.3 |

## 10. Normalization and statistical analysis

- Item = archive; L2-L4 aggregation with `workload-mix.json` for corpus archives; the scale series is a separate family stratum `SCALE-META` (two groups per N) analysed per N and never pooled with corpus families; conditions (profile × workload) never averaged.
- Tuning: utility winner W and survivors S per decision; the continuous band-unit utility uses the primary metrics above with equal OD weights (§6.3 default).
- Validation: W vs S confirmatory comparisons per §4.11-§4.12 (validation corpus + validation scale series), MDE gate ≤ 1 band unit.
- Scaling view (reported, not a verdict): least-squares slope of ln(metric) on ln(N) per candidate with 0.99 interval, as the DEC-ACC-018 "vs entry count" evidence; slopes are descriptive.
- R9: partition by source (local vs remote) and, for DEC-CON-028 triggers, by entry-count or manifest-share expressible thresholds; R6 minimum gains by tier: format revisions (`footer-section-locator-table`, paged structures, compact manifest bit) are T2/T3/T4 per §7.2 as assessed by an independent session; client-only strategies T0/T1.

## 11. Practical significance

T-10, T-11, T-12, T-08, T-06, T-02 by reference; minimum-gain bands per computed tier (§7.3, e.g. a new record or section type at T3).

## 12. Sensitivity analysis

Slow-start and connection-model arms with the robustness rule (whole-section fetches of multi-MiB manifests are exactly where slow start matters); §6.3-§6.7 arms; page-size sensitivity (P = 16 KiB and 4 MiB); scale-series generator sensitivity (median file size 64 KiB instead of 1 KiB); `persistent-metadata-cache` hit rate 0% and 100% brackets.

## 13. Expected negative results worth recording

- Speculative windows waste bytes on small archives at NP-LAN (possibly `DISTINGUISHABLE_WORSE` on `http_bytes` there).
- Paged structures cost more requests than whole fetch for full enumeration (list) workloads.
- Four revalidation HEADs per read are a measurable share of single-entry latency at NP-INTER (from code), which the library's own counters do not show.
- `always-cross-check` is prohibitively expensive remotely for large Chunk counts.

## 14. Threats to validity

- Simulated format revisions use declared encodings whose real implementations may differ in padding and framing; sizes are exact only for the declared layout.
- Scale trees are synthetic; real very-large trees in the corpus top out near 2.3·10^5 files (tuning `f04-tuning-real-netbsd-pkgsrc`), so 10^6-10^7 conclusions rely on generated data (and 10^7 on computation only).
- Client memory for simulated candidates is formula-based.
- Loopback origin lacks CDN behaviours (suffix ranges and If-Match support vary by server; EXP-REMOTE-009/013).

## 15. Held-out (Phase D placeholder)

Frozen W/S per decision rerun once on held-out corpus archives after Commit C plus a post-freeze generated scale series (seeds drawn after freeze and committed only as SHA-256 before use, per the sealing route of §5.4), report-only per §5.6.

## 16. Estimates

- Compute: about 10 machine-hours (packing 10^6-entry trees and Index-less scans dominate; simulation parallel; emulated corroboration about 3 h).
- Disk: about 25 GB peak (two 10^6-file trees, their archives, simulated structure files).
- Agent effort: scripted; judgment for tier assessment requests and analysis.
