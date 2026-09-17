# EXP-SCALE-003: Declared-requirement, stream-window, index/manifest and legacy-limit census (deterministic)

| Field | Value |
|---|---|
| Status | **READY** (design draft until gate G-A; the gate and exposure disclosure of EXP-SCALE-001 apply verbatim) |
| Domain | scale (program §13) |
| Platforms | Linux x86-64 (WSL2 Ubuntu 24.04, ext4 corpus under `/root/eb-research/corpus`) |
| Requires timing | false |
| Compute estimate | about 30 machine-hours |
| Disk estimate | about 25 GB scratch peak (largest tuning/validation item is 4.5 GiB; one archive pair at a time) |
| Agent effort | scripted; judgment for the candidate-default table citations (legacy/ecosystem input) |
| Tooling to build | `research/tools/scale/census_item.py` (driver, contract below); `research/tools/scale/legacy_limit_stats.py` (per-archive limit statistics using GNU tar/Python `tarfile`/`zipfile`, `7z l -slt`, `zstd -lv`, `xz --robot -lvv`); `research/tools/scale/predict_refusals.py` (offline refusal predictor). Existing: production `ebound`, harness `ebr-inspect-bytes` (exact section byte accounting, harness review R1-08) |

## Pre-registration

- **decision_ids:** DEC-ACC-054, DEC-ACC-052, DEC-ACC-056, DEC-LEG-022, DEC-LEG-059, DEC-CON-015, DEC-CON-018, DEC-CON-026, DEC-MOD-026.
- **Decision type:** EMPIRICAL (distributions and refusal rates are measured). Assignment pending (§14 item 12).
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-17 (M17.1 declared tightness, M17.3 benign false refusal), OD-09 (M09.4 window dedup loss), OD-05 diagnostics (T-02 components), OD-19 (M19.1); HC-06, HC-09.
- **Method and gate:** as EXP-SCALE-001.
- **Author and L7 exposure:** as EXP-SCALE-001. This experiment reads corpus families F01-F20 on tuning and validation only. Because the designing session saw held-out item identifiers of several of these families (coverage.md), §5.4 item 3 bars this session from designing *candidates or analyses* for decisions whose families include exposed items. The candidate-default table and its analysis (below) are therefore declared **provisional pending re-derivation or sign-off by a non-exposed session** before gate G-A; the census itself (a measurement of status-quo declarations with no candidate choice) is not a candidate design.

## Question

Across real tuning and validation items, what stream dedup windows, retention bounds, declared requirements, entry and chunk counts, index and manifest bytes, and legacy-archive limit statistics does the status quo produce, how often does the default `--stream-window 0` STREAM pack fail, and what refusal rate on legitimate legacy archives would each candidate set of import-limit and sequential-limit defaults produce?

## Hypotheses

- **H1a (DEC-ACC-054):** STREAM packing with the default window 0 fails (or is refused) on at least one item of every family whose items contain exact duplicate chunks (F03, F04, F10, F17, F18, F19), and succeeds on families without duplicates.
- **H1b (DEC-CON-026):** the `auto` window reported by the writer is minimal: packing with `auto − 1` fails for every item with `auto > 0`.
- **H1c (DEC-LEG-022, DEC-LEG-059):** no legitimate tuning or validation legacy archive is refused under the status-quo import limits except by the tar observation cap and the 7z dictionary/folder caps; the offline predictor agrees with the production outcome on every archive (predictor validity).
- **H0:** no family differs; the predictor disagrees with production on at least one archive (then the offline refusal-rate analysis is invalid and the decision needs prototype runs, EXP-SCALE-009).
- **Falsification:** H1a fails if window-0 STREAM pack succeeds on an item with duplicate chunks or fails on one without; H1b fails on any item where `auto − 1` succeeds; H1c fails on any predictor disagreement.

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-ACC-054 | Corpus distribution of required windows per profile and resulting reader retention bytes; failure rate of default-0 STREAM pack on real trees | first-use CLI study of refusal messages: human participants (not measurable) |
| DEC-ACC-052 | Distributions of entry count, total logical bytes, largest entry, chunk count, metadata bytes and required staging (logical bytes) over real items, against the status-quo SequentialLimits and bootstrap policy | adversarial behaviour under each default: EXP-SCALE-006; boundary behaviour: EXP-SCALE-004 |
| DEC-ACC-056 | Window and retention distribution under the status-quo digest order | alternative frame orders: EXP-SCALE-007 |
| DEC-LEG-022 | Corpus distributions of ratio, member size, dictionary, zstd window, folder size and entry count for legacy archives in F11, F14, F20 and any recognized archive; bomb exclusion; pre-registered refusal-rate sensitivity to each default | peak RSS per limit at candidate values: EXP-SCALE-002 (status quo) and EXP-SCALE-009 |
| DEC-LEG-059 | Observations per tar header and provenance bytes on real tars | 500k/1M synthetic tars: EXP-SCALE-002 |
| DEC-CON-015, DEC-CON-018 | Index bytes per chunk and manifest (metadata) bytes per entry on real trees, status quo, per profile and layout | alternative structures, lookup CPU, remote bytes: container and remote domains |
| DEC-CON-026 | Minimality of declared windows (H1b), i.e. how often "reject non-minimal window" would reject writer output | reader cost of computing the minimum: EXP-SCALE-008 |
| DEC-MOD-026 | Status-quo artifact bytes and descriptive pack CPU on real sparse items (F15, F16 raw images) | hole-plan candidates: model/container domain |

## Candidates

Census variants of the status quo (each is an ebr candidate):

| candidate | Action on one corpus item (all via production `ebound` at the recorded commit) | Applicability |
|---|---|---|
| `fast-indexed` | `pack --profile fast`; `ebr-inspect-bytes --layout indexed`; `inspect --json --chunks` | all |
| `balanced-indexed` | as above, balanced | all |
| `dense-indexed` | as above, dense | scale small or medium |
| `extreme-indexed` | as above, extreme | scale small |
| `balanced-stream-auto` | `pack --layout stream --stream-window auto --profile balanced`; record the reported window W; if W > 0 also pack with `--stream-window W−1` (minimality check); `ebr-inspect-bytes --layout stream` | all |
| `balanced-stream-window0` | `pack --layout stream --profile balanced` (CLI default window) | all |
| `dense-stream-auto` | as `balanced-stream-auto`, dense | small or medium |
| `legacy-stats` | `legacy_limit_stats.py` over every archive file in the item recognized by magic (tar, gzip, zstd, xz, bzip2, ZIP, 7z) | items containing at least one recognized archive |
| `legacy-import-default` | `ebound convert <archive> <scratch>/imp.eb --from <detected>` for each recognized archive, default policy | as `legacy-stats` |

- No other candidate: this experiment characterizes the status quo; candidate *defaults* are evaluated offline (below), not as runs.
- **Candidate default sets for the offline analysis** (`research/experiments/EXP-SCALE-003/candidate-defaults.json`, committed before the first validation look; provisional per the L7 note): `status-quo` (constants read from `crates/entrybound/src/legacy/{tar,sevenz,stream,zip,preserve}.rs` and `crates/entrybound/src/ecf/stream.rs` at the binary commit by `predict_refusals.py --dump-status-quo`); `corpus-percentile` (each limit = tuning-split p99.9 × 4, rounded up to a power of two; **computed from tuning only**); `memory-envelope-2g` (limits whose status-quo peak RSS in EXP-SCALE-002 stays below 2 GiB); `incumbent-aligned` (libarchive, 7-Zip, Go archive/tar values, each with a primary-source citation supplied by the legacy domain); `ultra-admitting` (7z dictionary 1,536 MiB and folder 16 GiB, others status quo). Candidates `tiered-presets`, `absolute-decoded-budgets`, `per-format-ratio` and `caller-declared-budgets` are evaluated as the same predictor over their parameter values once the legacy domain supplies them; a candidate without values is excluded with that reason.

## Corpus selectors

- Manifest `research/corpus/manifest.json` (`ed28b1b4…`), splits `tuning` and `validation`, all families. 203 items; no item excluded (the non-UTF-8 `f19-tuning-zoo` refusal is itself a recorded outcome).
- Split use (§5.1): tuning items fix `corpus-percentile` values and any threshold search; validation items are used only to confirm refusal rates of the finalized candidate table (one validation look, registered in `research/decisions/validation-looks.jsonl` when G-A exists).
- **Minimum support (§4.9):** distributions are reported per family and per independence group; corpus-level refusal rates are reported with the group structure (tuning families have 2-6 groups; validation 1-4) and are descriptive until a decision uses them as a primary metric.
- **Held-out placeholder (Phase D):** the finalized default table is re-evaluated once on held-out after the design freeze with `census_item.py` variants `legacy-stats` and `legacy-import-default` and `balanced-stream-auto`, per `decision-method.md` §5.5 (Commit A lists this spec and its analysis command; run with `EB_HELDOUT_UNLOCK`). Not runnable before unlock; no held-out path appears here.

## Environment

`env_id` `wsl-ubuntu`, `VIRTUALIZED` (irrelevant: all primary metrics are deterministic). Items read in place from `/root/eb-research/corpus/<split>/<item>` (ext4, never `/mnt/*`). Outputs under the ebr run scratch. `TMPDIR` on the run scratch. `LANG=C.UTF-8`, `TZ=UTC`. No memory cap (a safety `ulimit -v` is not used because it breaks allocators; `memcap_run.sh --cap 25769803776` wraps each pack as a VM safety cap only).

## Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound
export PYTHONPATH=research/tools PYTHONDONTWRITEBYTECODE=1; PY=/root/eb-research/venv/bin/python
$PY -m ebr validate --spec research/experiments/EXP-SCALE-003/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-SCALE-003/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-SCALE-003/spec.yaml --refingerprint
$PY -m ebr normalize --spec research/experiments/EXP-SCALE-003/spec.yaml
$PY research/tools/scale/predict_refusals.py --raw research/raw/EXP-SCALE-003 \
    --defaults research/experiments/EXP-SCALE-003/candidate-defaults.json \
    --out research/normalized/EXP-SCALE-003/decision/refusal-rates.csv
```

Driver contract (`census_item.py --item {item_path} --item-id {item_id} --split {split} --family {family} --variant {candidate} --scratch {scratch} --manifest research/corpus/manifest.json`): print one `ebr.scale.census.v1` JSON line with the payload fields below; `applicable: 0` when the variant does not apply; exit non-zero only on infrastructure failure. Every produced archive is checked with `ebound verify` before its numbers are recorded (correctness gating); `ebr-inspect-bytes` bucket totals must reconcile exactly to `artifact_bytes` (§4.13 size reconciliation) or the sample is invalid.

## Seeds

`seeds.order` 1303001, `seeds.bootstrap` 1303002, `seeds.command` 1303003 (unused by the variants; recorded).

## Warmup

0 (deterministic metrics, §4.6).

## Measurement method and metrics

| Payload key | Canonical name | Role | Family | Unit |
|---|---|---|---|---|
| `outcome`, `reason_code` | none | outcome | correctness | enum, string |
| `artifact_bytes` | `artifact_bytes` (T-01) | descriptive covariate here | size | B |
| `metadata_bytes`, `index_bytes`, `dictionary_bytes`, `side_data_bytes` | T-02 components | diagnostic (primary for DEC-CON-015/018 only when those decisions are evaluated) | size | B |
| `metadata_bytes_per_entry` | `metadata_bytes_per_entry` (T-02b) | banded; primary candidate for DEC-CON-018 | size | B/entry |
| `index_bytes_per_chunk` | none (derived: `index_bytes / chunk_count`) | diagnostic | size | B/chunk |
| `entry_count`, `chunk_count`, `chunk_refs`, `max_chunk_plaintext_bytes`, `logical_bytes`, `max_entry_bytes`, `path_depth_max` | `scale_limits` inputs (M17.6, descriptive) | descriptive | other | count, B |
| `declared_window`, `auto_window_minimal` | none; `auto_window_minimal` is binary per item | H1b | other; correctness | chunks; 0/1 |
| `window0_ok` | `streaming_capability` input (M09.1) for default-window STREAM pack | H1a | correctness | 0/1 |
| `retention_bound_bytes` | none (derived: `declared_window × max_chunk_plaintext_bytes`) | descriptive | size | B |
| `declared_decode_window_bytes`, `declared_working_set_bytes`, eight declared budget fields | none; with `logical_bytes` etc. gives `declared_tightness_ratio` (T-23) | T-23 descriptive | other | B, count, ratio |
| `legacy_*` statistics (format, entries, observations, max member, total decoded, max ratio, dictionary, zstd window, folder bytes, pax records) | none | inputs to the predictor | other | count, B, ratio |
| `legacy_import_outcome`, `legacy_import_reason` | none | predictor validation | correctness | enum, string |
| `pack_cpu_s` | none (descriptive, not `encode_cpu_s`) | descriptive | time_cpu | s |

## Replication

2 repetitions (deterministic metrics, §4.6) plus the repeat-hash check (§4.13) on every archive. A mismatch where determinism is claimed (unencrypted pack) is an R1 violation artifact under objective §2.0 rule 2, committed under `research/raw/EXP-SCALE-003/nondeterminism/`, and is also reported to EXP-SCALE-005.

## Normalization

Per item and variant, exact values; per family: distributions (min, p50, p90, p99, max) over items, and per independence group; `benign_false_refusal_fraction` per candidate default set = refused legitimate archives / legitimate archives (F20 adversarial items and generated bombs are excluded from "legitimate" by their manifest `kind`/family, recorded in the case list before the look).

## Statistical analysis

- H1a, H1b, H1c: per-item binary outcomes, reported as counts with the items named; any counterexample falsifies (no statistics).
- Refusal rates per candidate default set: `benign_false_refusal_fraction` (T-20) over the pre-registered list of legitimate archives (n fixed when `legacy-stats` completes on tuning, before any candidate values are computed), per format and overall; the case list is committed with the candidate table.
- Window and retention distributions: descriptive per family and profile; no verdict.
- DEC-CON-015/018 per-entry and per-chunk bytes: descriptive here; when the container domain compares alternatives it uses these rows as the status-quo candidate under T-02b/T-02.
- ebr intervals are informational (§3.6).

## Practical-significance thresholds

Referenced: T-20 (`benign_false_refusal_fraction`, `import_acceptance_fraction`: a = 0.5/n); T-02 and T-02b; T-23 for tightness. None restated.

## Sensitivity analysis

- `corpus-percentile` rule sensitivity: p99 and p99.99 with margins 2 and 8, reported beside p99.9 × 4.
- Leave-one-family-out refusal rate per candidate set (§6.5 arm adapted to a descriptive rate).
- Profile arm: window distributions for balanced vs dense.

## Expected negative results worth recording

Default window 0 makes STREAM creation fail on common real trees (vendor trees, duplicate trees); `auto` windows reaching hundreds or thousands of chunks with multi-MiB retention bounds; status-quo legacy limits refusing legitimate F14 or F11 archives; per-entry metadata bytes that dominate artifact bytes on F04.

## Threats to validity

1. Corpus composition (F14 and F11 are small families) bounds refusal-rate generalization; the held-out re-evaluation and EXP-SCALE-002 synthetic tails complement it.
2. The offline predictor is valid only if it agrees with production on every archive (H1c); a disagreement voids the offline analysis for that format.
3. `ebr-inspect-bytes` STREAM and encrypted accounting is a single lump for some buckets (harness review L-03); STREAM `index_bytes` is zero by construction.
4. Incumbent citations for `incumbent-aligned` are external facts that must be re-verified at pinned versions (§9.1).
5. L7 exposure of the designing session (Pre-registration).

## Platform requirements

Linux x86-64 WSL2; corpus provisioned (`provision.py --check` clean); no root needed except the safety cap wrapper (root in WSL); no network; about 25 GB scratch.

## Estimated compute and effort

About 30 h: roughly 60 GiB of tuning+validation content × (fast, balanced, 2 STREAM variants, minimality re-pack) × 2 repetitions at 50-100 MiB/s, plus dense/extreme on small and medium items (about 25 GiB × 3 variants × 2 at 5-20 MiB/s). Scripted; judgment only for the candidate-default citations and the final interpretation.

## Deviations (append-only)

None.
