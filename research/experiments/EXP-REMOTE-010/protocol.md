# EXP-REMOTE-010: Encrypted remote access — segment discovery, segment packing, access-pattern mitigations, and verification-scope fetch cost

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T1 encrypted and signed trace support, T5 `fetchsim` encrypted strategies and segment-layout simulator `seglayout.py`, T6 `balanced-enc-*` and `*-signed` configs, trace-observer attack script `trace_attack.py`) |
| Domain / program section | remote / §12 (§22.3-§22.5 crypto, §24 integrity) |
| decision_ids | DEC-ACC-010, DEC-CRY-072, DEC-CRY-081, DEC-INT-010, DEC-INT-044, DEC-CRY-073 |
| Decision type (provisional) | EMPIRICAL for costs and trace-attack accuracy; EXTERNAL for the security of any new crypto wire element (locator tables, segment digests, packing changes) and for padding/mitigation effectiveness claims (`decision-method.md` §8.3, T-17 note); FORMAL for partial-read guarantees (DEC-INT-010) |
| OD / HC (provisional until G-A) | OD-12 (M12.1-M12.3), OD-16 (M16.1, M16.2, M16.4), OD-11 (M11.3), OD-15 guard (encrypted random latency, M10.5 via OD-10); HC-14 (crypto correctness: keyed boundaries, recorded padding mode, public lengths conform; frozen crypto-v1 edits excluded at L0), HC-15 (encrypted partial reads never claim full segment-sequence verification), HC-16 (new crypto versions are new identifiers), HC-06 (segment walk bounded) |
| Author / L7 / gates | As EXP-REMOTE-001. OD-16 metrics are **not decision-grade** until the committed attack suite and observer model exist (`decision-method.md` §14 item 16); the trace-observer attack defined here is proposed as a member of that suite, not a substitute for it. Crypto-v1 wire is frozen (`docs/crypto-wire-v1.md`): candidates that change it are evaluated only as new-version designs (cost modelled, security routed to external review). |

## 1. Question

For encrypted INDEXED archives read over HTTP, how does remote open cost scale with segment count and padding schedule under each segment-discovery strategy; how do alternative segment packings change random-access bytes, requests and observable range-length signatures; how much do access-pattern mitigations (batched dependency prefetch, dummy ranges, https-only) cost and how much do they reduce a trace observer's ability to tell which entry was read; and what do extra verification scopes (touched segment ENDs, remote whole-archive verification, FIDELITY/AUX in random reads, embedded signatures on range open) add in bytes, requests and latency?

## 2. Hypotheses

- **H1 (DEC-ACC-010).** Status-quo encrypted open issues one 64 B range per SegmentHeader (`crypto/random.rs:1202-1245`), so `http_requests` for WL-OPEN grows linearly in segment count (slope 1.0 on log-log within the 0.99 interval) and remote open at NP-INTER exceeds 10 s once segments exceed about 50 (a model consequence to be computed, not assumed); a locator table or coalesced header prefetch makes open requests sublinear, with the locator table `DISTINGUISHABLE_BETTER` on `remote_first_entry_latency_s` for archives with ≥ 256 segments at every profile except NP-LAN.
- **H2 (DEC-CRY-081).** Batching objects into 1 MiB segments cuts WL-OPEN requests and `range_length_signatures` relative to one object per segment, while WL-RAND1 `http_bytes` rise by less than the T-11 band for small entries and more for large ones.
- **H3 (DEC-CRY-072).** Against the trace observer (section 4.3), the status quo lets the observer identify the read entry among candidates with `presence_advantage` near 1 for entries whose segment ranges are unique; `batched-dependency-prefetch` with batch 16 and `dummy-range-requests` with k = 3 each reduce the advantage but cost at least 1 band unit of `remote_task_latency_s` at NP-CONT.
- **H4 (DEC-INT-010, DEC-INT-044, DEC-CRY-073 costs).** Verifying touched segment ENDs and fetching FIDELITY/AUX in random reads each add at most one request per read after coalescing and less than the T-11 absolute floor in bytes for tuning archives (`NON_INFERIOR` on `http_bytes`); remote whole-archive verification costs the archive's full bytes (formally) and its latency is dominated by transfer at NP-CONT/NP-INTER; signature verification on range open adds one coalescable range.
- **H0 / falsification.** H1 falsified if status-quo open requests are not linear in segments or the locator table is not better at ≥ 256 segments; H2 if batching does not reduce open requests or signatures; H3 if the status-quo observer advantage is not high for unique-range entries or a mitigation reduces it at no measurable cost; H4 if any added scope exceeds its stated bound.

## 3. Decisions informed

| Decision | Supplied here | Remaining |
|---|---|---|
| DEC-ACC-010 | Open request count and latency vs unique Chunk count and padding schedule (NP profiles through the calibrated emulator and model; S3 real path in EXP-REMOTE-013); MAXIMUM-padded archive range-access tests (outcomes and costs); `min(declared, caller)` bound behaviour tests | Leakage and security review of locator tables: crypto domain + EXTERNAL; whether a new crypto wire version is required: FORMAL (crypto-v1 frozen) |
| DEC-CRY-072 | Trace-observer identification advantage per mitigation (candidate attack for the §14 item 16 suite); bytes and latency overhead per mitigation | Usage evidence of plain-http sources: literature (no experiment); effectiveness claims: EXTERNAL (T-17 note); ORAM/PIR: non-goal (section 4) |
| DEC-CRY-081 | Random-access bytes fetched, requests, latency and `range_length_signatures` per segment packing layout | Inference accuracy of entry and unique-Chunk counts per layout, blast radius, memory and mutation payload reuse: crypto and integrity domains |
| DEC-INT-010 | Bytes, requests and time for remote whole-archive verification (and for verifying touched segment ENDs) under emulation | Adversarial corpus tampering unread segments with read outcomes; formal model of partial-read guarantees; external-review dossier: integrity and crypto domains |
| DEC-INT-044 | Bytes and range requests added by verifying FIDELITY/AUX (and used dictionaries/groups) in random reads over HTTP | Code audit mapping flags to checks; mutation tests of unread FIDELITY/dictionaries; API review: integrity domain |
| DEC-CRY-073 | Request and byte cost of remote signature verification on range open (embedded and detached) | Substitution demo: EXP-REMOTE-009; signature/trust semantics: crypto domain |

## 4. Candidates

### 4.1 Segment discovery (DEC-ACC-010)

| candidate_id | Definition | Realization |
|---|---|---|
| `status-quo-header-walk` (reference) | one 64 B range per SegmentHeader; whole-segment fetch; declared budget bounds Chunks | production encrypted random reader |
| `coalesced-header-prefetch` | batch SegmentHeader reads using a readahead bounded by declared extents (no wire change) | `ReadaheadSource` + `fetchsim` |
| `segment-locator-table-v2` | authenticated table of (offset, extent, class) per segment in a new crypto version, fetched with the tail | `fetchsim` with table size from `seglayout.py` (declared 32 B per segment); security EXTERNAL |
| `chunked-segment-fetch` | fetch large segments in bounded sub-ranges per AEAD message instead of whole-segment | `fetchsim` over message boundaries recorded in the layout map |
| `min-declared-caller-bounds` | use min(declared, caller) for range Chunk bounds | behaviour test on MAXIMUM-padded and lying-declaration archives (outcome, bytes before refusal) |

### 4.2 Segment packing (DEC-CRY-081), plaintext objects held fixed

`status-quo-one-object-per-segment` (reference; measured), `batch-objects-to-target-segment-size-<t>` t ∈ {64 KiB, 1 MiB}, `fixed-size-padded-segments-4m`, `group-by-chunkgroup`, `dummy-segments-pow2` (segment count quantized to the next power of two). Non-status-quo packings are simulated by `seglayout.py` from each archive's plaintext object sizes, the recorded padding mode's length function and crypto-v1 framing sizes (SegmentHeader 64 B, per-message overhead from `docs/crypto-wire-v1.md`), producing layout maps consumed by `fetchsim`; they are new crypto-version designs (HC-16) whose security is EXTERNAL.

### 4.3 Access-pattern mitigations (DEC-CRY-072) and the trace observer

Candidates: `status-quo-documented-non-goal` (reference), `batched-dependency-prefetch-<B>` B ∈ {4, 16} (each read also fetches a seeded batch of B − 1 other segments or dependency closures in one coalesced plan), `dummy-range-requests-<k>` k ∈ {1, 3} (k uniformly chosen dummy segment ranges per read, interleaved), `https-only-default` (network observers other than the server see only TLS record sizes and timing: modelled as the same length observations aggregated per coalesced response). `full-oblivious-access` (ORAM/PIR) is **not measured**: it is a declared non-goal (`docs/crypto-threat-model-v1.md` L198) and therefore a `NON_GOAL`/`DECISION_INPUT` class under the R1 crosswalk, not a screened candidate; recorded with that reason.

Observer model for this experiment (proposed for the §14 item 16 suite): the observer is the origin, knows the public archive bytes (segment offsets and extents, padding mode) and a candidate list of K = 32 plaintext files, one of which is read by a WL-RAND1 session; it sees the ordered (offset, length) sequence of the session. Attack: map each observed payload range to segments, rank candidate files by the likelihood that their encrypted object sizes (derived from plaintext size through the padding length function) match, and output the top candidate; `presence_advantage` = TPR − FPR at FPR 0.01 over 1,000 seeded (session, candidate list) trials per archive.

### 4.4 Verification scope costs (DEC-INT-010, DEC-INT-044, DEC-CRY-073)

`status-quo-per-object-plus-archivefinal` (reference), `verify-touched-segment-ends`, `per-segment-digests-in-archivefinal` (simulated added bytes in ArchiveFinal), `remote-streaming-whole-verify` (all ranges fetched with bounded memory, PCR/PCI computed: bytes = archive length; model + measured local verify throughput), `full-download-only`, `merkle-over-segments` (log proofs; `proofsim.py` from EXP-REMOTE-007); `verify-more-in-random-reader` (fetch FIDELITY/AUX sections and used dictionaries/groups), the zero-fetch-cost semantic candidates of DEC-INT-044 (`tristate-per-check`, `narrow-flag-definitions`, `typed-opened-archive-scope`, `scope-enum-only`) recorded as cost-equal to the status quo; `verify-signatures-on-range-open` (embedded CONTROL signature data and detached signature file) vs `status-quo-no-signatures-remote`.

## 5. Corpus selectors

- Tuning small + medium archives from EXP-REMOTE-002 for `balanced-enc-bucketed`, `balanced-enc-max`, `balanced-enc-none` (3 key replicates each), and signed variants `balanced-plain-signed` and `balanced-enc-bucketed-signed` (T6: `ebound key generate-signing` with a research-only key under `/root/eb-research/keys`, `ebound sign --embed` and `--detached`), all tuning small + medium.
- Segment-count series: EXP-REMOTE-004 scale trees at 10^3, 10^4, 10^5 regular files packed `balanced-enc-bucketed` (10^6 by the validated per-header formula).
- MAXIMUM-padded range tests: every `balanced-enc-max` archive with WL-RANGE-1M and WL-LARGE.
- Validation look: validation small+medium encrypted archives; held-out: section 15.

## 6. Environment and platform requirements

`wsl-ubuntu`; deterministic arms (counts, bytes, modelled latency, attack accuracy) with no timing; local verify throughput for `remote-streaming-whole-verify` from EXP-REMOTE-005's local timing arm (encrypted archives added there); emulated corroboration of W vs reference at NP-CONT and NP-INTER on 12 encrypted archives. Linux x86-64; research passphrase via environment variable only inside WSL (never in files committed to git); no privileges.

**Platform matrix.** Linux x86-64 required (WSL2); Windows: not used; macOS: not applicable; ARM64: not required; network: loopback emulation; privileged: no; large storage: ~40 GB; external review: yes for new crypto wire elements and mitigation effectiveness.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound; PY=/root/eb-research/venv/bin/python; export PYTHONPATH=research/tools
python research/orchestration/disk_guard.py && python research/harness/lock_parity.py
$PY research/tools/remote/prepare_archives.py ensure --items research/experiments/EXP-REMOTE-002/items-tuning-small-medium.txt \
    --configs balanced-enc-max,balanced-enc-none,balanced-plain-signed,balanced-enc-bucketed-signed --key-replicates 3
$PY -m ebr run --spec research/experiments/EXP-REMOTE-010/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-REMOTE-010/spec.yaml && $PY -m ebr normalize --spec research/experiments/EXP-REMOTE-010/spec.yaml
$PY research/tools/remote/seglayout.py simulate --layouts /root/eb-research/cache/remote-archives --packings research/experiments/EXP-REMOTE-010/packings.json \
    --out /root/eb-research/raw-detail/EXP-REMOTE-010/packings
$PY research/tools/remote/fetchsim.py sweep --strategies research/experiments/EXP-REMOTE-010/strategies.json --out research/raw/EXP-REMOTE-010/sim/
$PY research/experiments/EXP-REMOTE-010/trace_attack.py --traces research/raw/EXP-REMOTE-010 --trials 1000 --candidates 32 \
    --out research/normalized/EXP-REMOTE-010/decision/trace-attack.json
$PY -m decide analyze --experiment EXP-REMOTE-010
```

## 8. Seeds, warmups, repetitions

Seeds `order` 20261014, `bootstrap` 20261015, `command` 20261016 (attack trials, dummy and batch selection). Encrypted archives: 3 key replicates (fresh packs), counts treated as stochastic across keys (§4.13 reclassification), 2 trace repetitions per replicate with repeat-hash; emulated corroboration: warmups 2, rounds min 10, step 10, cap 20, precision rule.

## 9. Metrics

| Metric | OD | Role | Threshold |
|---|---|---|---|
| `remote_first_entry_latency_s` (encrypted open + first entry) — modelled | OD-12 | **primary** (DEC-ACC-010, -081, INT-010, INT-044, CRY-073 costs) | T-10 |
| `presence_advantage` of the trace observer | OD-16 | **primary for DEC-CRY-072 and DEC-CRY-081**, not decision-grade until §14 item 16 | T-17 |
| `range_length_signatures` per session | OD-16 | secondary | T-12 |
| `http_requests`, `http_bytes` per workload | OD-12 | secondary | T-12, T-11 |
| `remote_task_latency_s` for WL-RANDK16 and WL-FULL (whole verify) | OD-12 | secondary | T-10 |
| `encrypted_random_entry_latency_s` (local-file, in-process batches) | OD-10 | guard | T-08 |
| `verified_fraction` (partial reads in the strongest state the archive supports, per scope candidate) | OD-11 | secondary (INT-010/044) | T-20 |
| `padding_overhead_pp` per packing | OD-16 | secondary (packing changes padding) | T-16 |
| `origin_protocol_safety` for protocol-changing candidates | – | binary via EXP-REMOTE-009 screen | §3.3 |
| `slope_open_requests_vs_segments` | – | descriptive (H1) | – |

## 10. Normalization and statistical analysis

- Item = archive (median over key replicates); groups and families from corpus items; segment-count series as family `SCALE-SEG`; strata per padding mode (an evaluation condition, AGG-S), profile and workload; never averaged across padding modes.
- Tuning W/S; validation W vs S with §4.12 confidences and MDE gate (DEC-CRY-072 and DEC-CRY-081 OD-16 comparisons are reported but cannot become decision-grade before §14 item 16).
- R6 dominates for new crypto versions (T4: cryptographic construction or wire change of a frozen suite) against the status quo's T0/T1 client-side mitigations.
- Attack accuracy is computed per archive then aggregated with the corpus weights; per-padding-mode worst case reported (AGG-W).

## 11. Practical significance

T-10, T-17, T-12, T-11, T-08, T-20, T-16 by reference; minimum gains per computed tier (T4 k = 10 for crypto constructions).

## 12. Sensitivity analysis

Slow-start and connection-model arms; candidate list size K ∈ {8, 128} and a prior over read files (uniform vs size-weighted) for the attack; dummy/batch seeds; padding mode as condition; segment-count formula check at 10^6; §6.3-§6.7 arms.

## 13. Expected negative results worth recording

- Encrypted open is linear in segment count in the status quo (from code), making remote open of many-segment archives impractical at high RTT.
- Mitigations that meaningfully reduce identification cost multiples of the base bytes; `https-only-default` gives no protection against the server as observer.
- Batching segments reduces signatures but raises bytes per small read and may conflict with blast-radius goals (integrity domain).
- Verification-scope additions are cheap remotely; whole-archive remote verification is as expensive as a download.

## 14. Threats to validity

- Simulated packings and locator tables assume declared framing; real crypto-version implementations may differ.
- The trace observer is one attack; the committed suite may include stronger adaptive attacks (T-17 requires them).
- Key replicates (3) give limited coverage of keyed-boundary variability.
- `https-only` observation is modelled, not captured on the wire.

## 15. Held-out (Phase D placeholder)

Frozen W and S rerun once on held-out encrypted archives packed after unlock with fresh keys (Commit C, `decision-method.md` §5.5), deterministic arms plus the attack with a post-freeze seed, report-only per §5.6.

## 16. Estimates

- Compute: about 10 machine-hours (encrypted packing with 3 replicates and signed variants about 4 h; segment-count series about 2 h; attack trials and simulation about 3 h; emulated corroboration about 1 h).
- Disk: about 40 GB peak (encrypted replicates of small+medium tuning items and signed variants).
- Agent effort: scripted; judgment for the observer-model proposal to the crypto domain, non-goal classification record, tier requests and analysis.
