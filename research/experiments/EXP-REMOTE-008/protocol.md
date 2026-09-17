# EXP-REMOTE-008: Random-access policy caps, Index rebuild fallback, and hostile-origin amplification

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T1, T4 `ReadaheadSource`, T5, T6 plus `mutate_archive.py` hostile-archive and hostile-Index generator, `policies.json` candidate policy sets) |
| Domain / program section | remote / §12 (§11 Index doctrine; §17 resource-bounded decoding) |
| decision_ids | DEC-ACC-039, DEC-ACC-044, DEC-ACC-019 |
| Decision type (provisional) | EMPIRICAL (caps and rebuild costs); HUMAN-FACING for the preset/flag usability part of DEC-ACC-039 (human claims routed to external review, §4.16) |
| OD / HC (provisional until G-A) | OD-17 (M17.1-M17.4), OD-12 (M12.1-M12.3), OD-25 cost input (flags per task); HC-06 (declared bounds, refusal before proportional work; MVT-06(a)), HC-15 (no wrong bytes reported verified; distinguishable refusal classes), HC-01 (Index is a cache), HC-18 (hostile origin) |
| Author / L7 / gates | As EXP-REMOTE-001. Refusal memory is decision-grade only with the counting allocator and pre-validation bound H (`decision-method.md` §14 item 17; objective MVT-06(a)); until then HC-06 memory results are HC_UNVERIFIED and memory is reported as scoped RSS. |

## 1. Question

Which default `RandomAccessPolicy` values (and which form: fixed, scaled to declarations, split local/remote, named presets, CLI flags, or planner-guaranteed closures) admit every legitimate tuning-corpus workload and large-file read while refusing every adversarial archive before proportional work; what does Index rebuild cost over a remote source when the Index is absent, invalid, incomplete or refused, and should remote sessions refuse or rebuild; and how much work can a hostile origin force through the Index doctrine?

## 2. Hypotheses

- **H1 (DEC-ACC-039).** The status-quo defaults refuse at least one legitimate case in the pre-registered benign set: remote reads of a single ≥ 1 GiB regular file exceed `max_total_bytes_fetched` = 512 MiB (`random_access.rs:405-424`), and the 16 GiB entry exceeds `max_decoded_logical_bytes` = 8 GiB; `scaled-to-declarations` reaches `benign_false_refusal_fraction` = 0 on the benign set with `adversarial_true_refusal_fraction` = 1.
- **H2 (DEC-ACC-044).** Index-less remote opening costs one request per Chunk frame (`random.rs:692-752`); `coalesced-header-prefetch` with a 1 MiB readahead cuts requests by at least the T-12 band on archives with median stored Chunk ≤ 64 KiB, and does not help (or loses on bytes) for large Chunks; `refuse-remote-rebuild-by-default` bounds hostile amplification to the valid-Index cost at the price of refusing legitimate Index-less archives.
- **H3 (DEC-ACC-019 hostile Index).** For every hostile-Index variant the production reader never returns bytes that fail the object digest and never reports them verified (binary), but the worst variant (a structurally valid Index pointing at wrong frames) forces a full rebuild, an amplification of `http_requests` proportional to total Chunks rather than requested Chunks.
- **H0 / falsification.** H1 falsified if status quo refuses no benign case or if `scaled-to-declarations` misses any adversarial case; H2 falsified if coalesced prefetch fails the T-12 band on small-Chunk archives; H3 falsified by any wrong-bytes-reported-verified event (which would also be an HC-15 violation artifact) or by bounded amplification of the stale-Index variant.

## 3. Decisions informed

| Decision | Supplied here | Remaining |
|---|---|---|
| DEC-ACC-039 | Largest legitimate per-profile ranges, closures, section counts, metadata bytes and segment counts (census over EXP-REMOTE-002 tuning traces); adversarial archives probing each cap with refusal cost; 1-16 GiB large-file read behaviour under each candidate policy | Developer usability study of presets vs raw limits: HUMAN_PARTICIPANTS (blocked; simulated sessions under §4.16 can supply only flag counts and error-message censuses, ecosystem §33) |
| DEC-ACC-044 | Rebuild requests, bytes and time over HTTP for 10^3-10^6 Chunks measured and up to 4·10^6 by the validated per-frame formula (plain and encrypted); hostile-server amplification (stripped/corrupted Index); regression cases for incomplete Index and partial encrypted Index with outcomes | none beyond validation/held-out |
| DEC-ACC-019 | Hostile-Index test suite outcomes and costs per validation doctrine | Latency with/without Index: EXP-REMOTE-004; wire-cost of digest binding: container domain |

## 4. Candidates

### 4.1 Policy form and values (DEC-ACC-039); exact values in `policies.json`, committed before any run

| candidate_id | Definition |
|---|---|
| `status-quo-defaults` (reference) | 64 MiB range / 512 MiB total / 100,000 requests / 128 MiB metadata / 64 sections / 4,000,000 frames / 65,536 dependency Chunks / 8 GiB decoded / 128 MiB cache / 4,000,000 segment headers / 100,000 trace entries / 4,096 B coalesce gap (`random_access.rs:405-424`, `docs/random-access-v1.md`) |
| `scaled-to-declarations` | per session: total fetch cap = min(64 GiB, declared archive length); per read: decoded cap = declared largest entry; dependency cap = declared worst closure; metadata cap = declared metadata bytes × 1.25; absolute ceilings as the status quo's where larger; caller caps can only narrow (HC-05) |
| `local-vs-remote-profiles` | local sources: status quo with total fetch = 64 GiB and decoded = 64 GiB; remote sources: status quo |
| `named-presets` | `untrusted-remote` (status quo ÷ 4 on every byte and count cap), `ci` (status quo), `desktop` (local-vs-remote local values) |
| `cli-flags` | status quo plus `--max-fetch`, `--max-decoded`, `--max-requests` flags (C6 cost: 3 flags) |
| `planner-guaranteed-closures` | status quo values; the planner guarantees and records that its outputs fit them (evaluated as the census fraction of planner outputs that would violate the defaults; a planner-domain obligation if adopted) |
| critic slot | R0 item 6 |

### 4.2 Index fallback (DEC-ACC-044)

`status-quo-full-uncoalesced-scan` (reference; production), `refuse-remote-rebuild-by-default` (remote session refuses with a stable code unless opted in; source-type check in the `ebr-remote` wrapper), `coalesced-header-prefetch-<W>` W ∈ {256 KiB, 1 MiB, 4 MiB} (`ReadaheadSource` over CHUNK_DATA during scan), `separate-rebuild-budget` (rebuild counts against distinct caps of 1,000,000 requests and 4 GiB bytes), `merged-partial-rebuild` (for an incomplete but digest-valid Index, scan only for missing locators — `fetchsim`, since the production reader treats incompleteness as failure), `mandatory-index-for-remote` (refuse any remote session whose Index is not present-valid).

### 4.3 Index doctrine under hostile Index (DEC-ACC-019)

`status-quo-lazy-validation` (production), `always-cross-check` (`fetchsim`), `digest-bound-index` (`fetchsim`: a hostile Index fails the bound digest at open, then the declared fallback of 4.2 applies), `per-record-validation` (= status quo pattern), `remove-index-v1` (Index ignored), `authoritative-index` (proposed L0 exclusion per HC-01; its outcome under hostile Index is computed to produce the written counterexample the exclusion needs).

### 4.4 Test archives

- **Benign set B** (pre-registered cases, n recorded in `benign-cases.json` before the run): every EXP-REMOTE-002 tuning (archive, workload) cell that the production client completed plus generated single-entry archives of 1 GiB, 4 GiB and 16 GiB (`random-v1` content, two seeds each, `balanced-plain` and `fast-plain`), each read remotely and locally with WL-LARGE and WL-RANGE-1M.
- **Adversarial set A** (`mutate_archive.py`, seeded, applied to 5 small tuning archives and to synthetic minimal archives, digests recomputed where a case needs to pass earlier checks): section count 65; Manifest declared at 129 MiB; 4,000,001 frame headers (synthetic, zero-length payload frames where the format permits, else formula); dependency closure of 65,537 Chunks; decoded size 8 GiB + 1 B; SegmentHeader walk 4,000,001; trace entries 100,001; single range 64 MiB + 1 B; lying declarations below reality for each cap.
- **Hostile-Index set H**: Index absent; Index section digest mismatch; out-of-bounds extent; duplicate digest; locator pointing at a different valid frame; locators permuted with a recomputed section digest; stale Index copied from a sibling archive; incomplete Index (missing 1%, 50% of locators); encrypted Index with a missing PAYLOAD mapping.

## 5. Corpus selectors

Tuning split only for tuning analysis: EXP-REMOTE-002 archives (census, benign set), scale series of EXP-REMOTE-004 (`balanced-plain-noindex` at 10^3-10^6 entries) for rebuild costs, generated sets B (large entries), A and H under `/root/eb-research/generated/EXP-REMOTE-008/` with a committed `generated-manifest.jsonl` (generator, seed, SHA-256). Validation: validation archives and a fresh seeded A/H set for the confirmatory look. Held-out: section 15.

## 6. Environment and platform requirements

`wsl-ubuntu`; http-loopback and memory/local-file sources; deterministic counts and bytes; refusal memory by scoped RSS (counting allocator when available); modelled latency; no timing except the optional informational `wall_s`. About 45 GB of ext4 for the 1/4/16 GiB generated entries and their archives (two seeds); Linux x86-64; no privileges.

**Platform matrix.** Linux x86-64 required (WSL2); Windows: not used; macOS: not applicable; ARM64: not required; network: loopback; privileged: no; large storage: ~50 GB; external review: no.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound; PY=/root/eb-research/venv/bin/python; export PYTHONPATH=research/tools
python research/orchestration/disk_guard.py && python research/harness/lock_parity.py
$PY research/experiments/EXP-REMOTE-008/census.py --traces /root/eb-research/raw-detail/EXP-REMOTE-002 --out research/normalized/EXP-REMOTE-008/census/   # largest legitimate values
$PY research/tools/remote/mutate_archive.py generate --sets B,A,H --seed 20261010 --out /root/eb-research/generated/EXP-REMOTE-008 \
    --manifest research/experiments/EXP-REMOTE-008/generated-manifest.jsonl
$PY -m ebr run --spec research/experiments/EXP-REMOTE-008/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-REMOTE-008/spec.yaml && $PY -m ebr normalize --spec research/experiments/EXP-REMOTE-008/spec.yaml
$PY research/tools/remote/fetchsim.py sweep --strategies research/experiments/EXP-REMOTE-008/strategies.json --out research/raw/EXP-REMOTE-008/sim/
$PY -m decide analyze --experiment EXP-REMOTE-008
```

## 8. Seeds, warmups, repetitions

Seeds `order` 20261008, `bootstrap` 20261009, `command` 20261010 (mutation schedules, large-entry content). Deterministic: 2 repetitions + trace repeat-hash; encrypted cases 3 key replicates. Refusal memory (scoped RSS): 3 runs per case, maximum reported (objective MVT-06(a) uses 20 runs for R0_max; applied when the counting allocator exists).

## 9. Metrics

| Metric | OD | Role | Threshold |
|---|---|---|---|
| `benign_false_refusal_fraction` over set B | OD-17 | **primary** (DEC-ACC-039, DEC-ACC-044) | T-20 (a = 0.5 / n_cases) |
| `adversarial_true_refusal_fraction` over set A | OD-17 | binary (HC-06) | §3.3 |
| `remote_first_entry_latency_s` for Index-absent and hostile-Index cases that succeed — modelled | OD-12 | **primary** (DEC-ACC-044, DEC-ACC-019) | T-10 |
| `refusal_bytes_read`, `http_requests` before refusal | OD-17 / OD-12 | secondary | T-02, T-12 |
| `refusal_alloc_peak_bytes` (scoped RSS until §14 item 17) | OD-17 | secondary; HC_UNVERIFIED for HC-06 until the allocator exists | T-06 |
| `amplification_factor` = hostile-case `http_requests` / valid-Index `http_requests` for the same workload | OD-12 | descriptive | – |
| `wrong_bytes_reported_verified` | – | binary (HC-15) | §3.3 |
| `reason_code_specificity_fraction` over sets A and H | OD-04 | secondary | T-20 |
| `flags_per_task` for `cli-flags` and `named-presets` | OD-25 | cost input (§7.1 C6) | – |

## 10. Normalization and statistical analysis

- Coverage fractions are deterministic counts over fixed case lists (AGG-C: per family then minimum over families, mean secondary); a one-case difference exceeds the band by construction (T-20).
- Census: per profile, maximum and 99th percentile of each capped quantity over tuning traces (descriptive), stratified by scale tier.
- Rebuild costs: per archive exact values; scaling slope on ln(Chunk count) with the per-frame formula checked for exact agreement on 10^3-10^6 before extrapolating to 4·10^6.
- Tuning W/S by R4 on the two primaries; validation look on validation archives plus a fresh A/H set; R8/R9 with the source partition (local vs remote) expressible.
- Tiers (independent assessment): default value changes T1; new flags or presets T2 (user-visible mode, C6); `refuse-remote-rebuild-by-default` adds a reason code (T2); `digest-bound-index` is a wire/identity change (T3-T4).

## 11. Practical significance

T-20, T-10, T-02, T-12, T-06 by reference; binaries per §3.3; minimum gains per computed tier.

## 12. Sensitivity analysis

Case-list sensitivity: benign set without the 16 GiB entries, and with generated 64 GiB sparse entries (zeros content) as a stress arm; profile arms (fast vs extreme closures); slow-start arms for rebuild latency; hostile set with digests not recomputed (checks earlier layers); §6.5 family arms on the census.

## 13. Expected negative results worth recording

- Status-quo defaults refuse legitimate remote reads of ≥ 512 MiB single files (from code) — a v1 usability defect, recorded as a production defect per objective §2.0 rule 5.
- Scaled caps let an honest-but-large archive consume large fetch budgets on untrusted origins (the trade-off DEC-ACC-039 must declare).
- Coalesced prefetch gives no benefit for multi-MiB Chunks.
- A stale but structurally valid Index forces full rebuild: bounded only by frame caps (amplification proportional to total Chunks).

## 14. Threats to validity

- Adversarial set A is a finite enumeration (regression evidence, objective MVT-06(a) detection note).
- Scoped RSS undercounts allocator reuse; HC-06 stays HC_UNVERIFIED until the counting allocator and bound H exist.
- 4,000,001-frame cases may be formula-derived if synthetic frames cannot be encoded compactly.
- Large generated entries are incompressible random data (worst case for fetch caps; not representative of compressible large files).

## 15. Held-out (Phase D placeholder)

Frozen W policies and Index doctrine rerun once on held-out archives with a post-freeze seeded A/H set after Commit C (`decision-method.md` §5.5), report-only per §5.6.

## 16. Estimates

- Compute: about 9 machine-hours (packing and reading 1/4/16 GiB entries twice dominates; Index-less scans of 10^6-Chunk archives over loopback about 2 h).
- Disk: about 50 GB peak (large generated entries and archives; removed after the run, regenerable by seed).
- Agent effort: scripted; judgment for policy value justification in `policies.json` (written before results), tier assessment requests and analysis.
