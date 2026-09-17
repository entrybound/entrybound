# EXP-RECON-001: Status-quo reconstruction baseline — selections, fallbacks, declarations, exact round trip and determinism under production v6 across profile × layout × encryption

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-RECON-001 |
| Domain | reconstruction (program section 8.8); common provisions in `research/experiments/_index/reconstruction-common.md` (cited as common §N) |
| Status | **READY**. It runs with the production `ebound` CLI and the existing harness binaries `ebr-pack`, `ebr-unpack` and `ebr-inspect-bytes`, rebuilt at the pre-registration commit by `prepare.sh`, and the checked-in wrapper `recon_sample.py`. |
| Stage | Status-quo characterisation plus binary HC screens on **tuning**. No candidate comparison, no W/S, no confirmatory verdict. |
| decision_ids | DEC-CMP-018, DEC-CMP-026, DEC-CMP-023, DEC-CMP-016, DEC-CMP-012, DEC-CON-004, DEC-CON-005, DEC-CON-010 |
| Proposed decision types, od_ids, hc_ids | common §2. HCs exercised here: HC-02 (MVT-02(a) round-trip matrix; MVT-02(e) path-coverage counts), HC-13 (MVT-13(a) repeat-hash, 2-repetition form only), HC-15 (verification status), HC-11 R2 facts (default-path emission). |
| Method | `research/decision-method.md` at the commit that adds this file; thresholds from `research/methods/thresholds.json`. |
| Gates | G-A unmet at design: runs are tuning-exploratory. HC screens are binary and need no G-B statistics, but a violation found here is recorded as an R1 artifact only once G-A holds (common §2). |
| Author and exposure | common §3 |
| Feasibility smoke (NOT_DECISION_GRADE) | `recon_sample.py` ran on a generated 5-file tree (not corpus) for `balanced-indexed-plain`, `dense-stream-plain` and `dense-indexed-enc`. All exited 0, emitted one JSON line each, and gave 0 mismatches over 5 files. The run took under 10 s. Binaries were stale pre-review builds, used for wiring only. |

## 2. Question

For every tuning item, what does the production planner (`plan_archive_v6`, the only generation `ebound pack` emits) do in each cell of {fast, balanced, dense, extreme} × {INDEXED, STREAM} × {plain}, plus {balanced, dense, extreme} × INDEXED × {password-encrypted}?

- Which objects and chunks does it select for DEFLATE or JPEG reconstruction, and which does it reject, with what audit reason?
- What does the archive declare: required feature bits, `DecodeRequirements` window, working set and flags, and region access cost?
- What does production itself report as the recorded, replayed or derived savings?
- Does every cell reproduce every file exactly (HC-02), deterministically across two processes (HC-13)?

## 3. Hypotheses

Every hypothesis below is `[INFERRED]`, from `docs/reconstructive-transform-v1.md`, `docs/jpeg-reconstruction-v1.md`, `crates/entrybound/src/{reconstruction,jpeg_reconstruction}.rs`, chunk policies in `docs/chunking-v1.md`, and the wiring smoke (common §3).

- **H1a (HC-02, binary).** `roundtrip_mismatches = 0` in every evaluated sample. **Falsified by** any mismatch. The two differing artifacts (archive and unpacked file) are committed under `research/raw/EXP-RECON-001/violations/` as the R1 violation artifact (objective §2.0 rule 2), with no confirming rerun.
- **H1b (HC-13, binary).** Within each unencrypted cell, both repetitions share one `determinism_key` (archive SHA-256). Within encrypted cells, `lai` is equal and the ciphertext differs. **Falsified by** any unencrypted hash mismatch (R1 artifact) or any equal ciphertext.
- **H1c (path coverage, MVT-02(e)).** At least one (profile, layout) cell exercises the DEFLATE-reconstruction path fewer than 5 times over the tuning split. The path count is the number of ReconstructionData objects. Whole-Chunk eligibility requires a complete stream inside one Chunk, and balanced Chunks are at most 2 MiB. The prediction is that at least one cell is `HC_UNVERIFIED`. **Falsified if** every non-fast cell reaches ≥ 5 DEFLATE and ≥ 5 JPEG-region exercises.
- **H1d (encryption).** No reconstruction path is exercised in encrypted cells (`recon_transform_chunk_count = 0` and `whole_object_region_count = 0` for every encrypted sample). The smoke recorded the planner id `dense-enc-v1`, a distinct encrypted planner generation. **Falsified by** any encrypted archive with a reconstructive object. Either outcome is recorded. If H1d holds, the HC-02 claim for reconstruction under encryption is vacuous and must be stated as such, and reconstruction is a plaintext-only size lever.
- **H1e (DEC-CON-010).** Among non-fast unencrypted archives with **zero** reconstructive objects, ≥ 90 % still require bits 0x4 and 0x8 (status quo "always set when audits exist"). **Falsified if** < 90 %.
- **H1f (DEC-CON-005 / DEC-CMP-016).** Every archive with ≥ 1 JPEG region declares `declared_working_set_bytes` ≥ the JPEG working-set constant (256 MiB) and would therefore be refused by any default ceiling below it. Archives without regions declare less. This is a deterministic consequence to confirm, not a statistical claim. **Falsified by** any region archive below 256 MiB, which would be an HC-06 declaration defect, or any region-free archive at or above it.
- **H0.** No quantity here is compared against a band. Every size quantity is descriptive status-quo data (common §8.2) feeding R0 and R3 of the comparison experiments EXP-RECON-004, 005 and 006.

## 4. Decisions informed and how

| Decision | Contribution | Not settled here |
|---|---|---|
| DEC-CMP-018 | Status-quo DEFLATE selections and fallback reasons per profile and layout; production's replayed net savings (descriptive); HC-02 and HC-13 screen of `deflate-v5-status-quo`; path coverage | Net savings versus `deflate-drop` (EXP-RECON-004); producers (002, 003); CPU (005) |
| DEC-CMP-026 | JPEG region selections, fallback reasons (not recognized, unsupported/progressive, verification failed, cost, dedup conflict, resource), derived net savings, declared access cost; HC screens of `jpeg-v6-status-quo` | Backends and subsets (003, 004); latency (005) |
| DEC-CMP-023 | Whether default archives created today already depend on v5/v6 features: the share of archives requiring 0x4/0x8, and the share with emitted reconstructive bytes | Savings share per generation (004) |
| DEC-CMP-016, DEC-CON-005 | Declared window and working set per archive. `benign_false_refusal_fraction` of each ceiling candidate is computed offline from declarations (§11). Pack refusals on benign items (planner not mirroring validation bounds) are observed. | Measured peaks and tightness (006) |
| DEC-CON-004 | Declared values as written by the status quo | Measured composition of reconstruction decode memory (006) |
| DEC-CON-010 | Bit activation versus emitted bytes (H1e); audit record counts per archive; bit determinism across repetitions | Byte cost of audits and minimal activation (004) |
| DEC-CMP-012 | Status-quo attempt outcomes per chunk (`unrecognized-or-verification-failed` versus `complete-cost-rejected`) on non-target families, as the attempt-volume baseline for gating | Gate variants (004, 005, 006) |

## 5. Candidates

This experiment characterises the single status-quo design (common §6: `deflate-v5-status-quo`, `jpeg-v6-status-quo`, `gen-v6`, `bits-status-quo`) in 11 cells. The spec's `candidates` are cells, not alternatives:

| Cell | Tool | Notes |
|---|---|---|
| `{fast,balanced,dense,extreme}-indexed-plain` | `ebound pack … --profile P --layout indexed` | `fast` is the no-reconstruction control (v6 fast has none) |
| `{fast,balanced,dense,extreme}-stream-plain` | `ebound pack … --layout stream` (window `Ceiling(0)`, the CLI default) | Layout-equivalence facts: LAI/AUX/PCR equal to INDEXED per item and profile |
| `{balanced,dense,extreme}-indexed-enc` | `ebr-pack --encrypt true` (harness; bytes proven identical to CLI, R1-06) | The CLI's `--password` is interactive only, and `ebound` offers no recipient-key generation, so the harness path is used. Encrypted STREAM does not exist in production. |

Exclusions are not applicable here (no alternatives). The alternatives of every decision are measured in the experiments of common §1.

## 6. Corpus

- **Split.** Tuning only: every tuning item of `research/corpus/manifest.json` (112 items, 20 families, 89 independence groups, 36.1 GiB). Items are selected by `splits: [tuning]`.
  - Count command: `python -c "import json;m=json.load(open('research/corpus/manifest.json'));it=[i for i in m['items'] if i['split']=='tuning'];print(len(it),sum(i['bytes'] for i in it)/2**30,len({(i['family'],i['independence_group']) for i in it}))"`.
  - No item is excluded. Known product refusals, for example non-UTF-8 names in `f19-tuning-zoo` (baselines README), are recorded as `pack_refused` with their reason code, not as failures.
- **Validation.** This spec is re-run with `splits: [validation]` only as part of the first registered validation look of DEC-CMP-018/026/023 (common §5), in the same look as EXP-RECON-004.
- **Held-out.** Phase D placeholder per common §5 (status-quo HC screens on held-out after Commit C).
- **Generated items.** None.

## 7. Environment and platform requirements

- `wsl-ubuntu` (WSL2 Ubuntu 24.04, ext4 under `/root/eb-research`), x86-64, **no timing**, cache state irrelevant. The Windows host is not needed: F-0001/F-0002 cross-host differences belong to the platform domain.
- **Privileges.** Root inside WSL (existing). No network. Storage: scratch peak about 2 × the largest tuning item (3.5 GiB) + archive, so ≤ 12 GiB. Results: about 3 GB outside git.
- **Preconditions.** common §4 items 1–3 and 7, enforced by `prepare.sh`.

## 8. Commands

```sh
# 0. orchestrator (Windows host): disk guard
C:/Python313/python.exe research/orchestration/disk_guard.py
# 1. build + parity (WSL)
wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-RECON-001/prepare.sh
# 2. run, verify, normalize, report (WSL)
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools &&
  PY=/root/eb-research/venv/bin/python &&
  $PY -m ebr validate --spec research/experiments/EXP-RECON-001/spec.yaml &&
  $PY -m ebr run --spec research/experiments/EXP-RECON-001/spec.yaml --gzip &&
  $PY -m ebr verify-raw --spec research/experiments/EXP-RECON-001/spec.yaml --refingerprint &&
  $PY -m ebr normalize --spec research/experiments/EXP-RECON-001/spec.yaml &&
  $PY -m ebr report --spec research/experiments/EXP-RECON-001/spec.yaml'
# 3. offline analysis (script to be written with the decision tooling, §14 item 2):
#    research/tools/recon/analyze_status_quo.py --normalized research/normalized/EXP-RECON-001 \
#        --detail /root/eb-research/results/EXP-RECON-001/detail --out research/normalized/EXP-RECON-001/decision/
```

- **Per-sample wrapper** (`recon_sample.py`, checked in):
  1. `ebound pack` (or `ebr-pack --encrypt true`)
  2. `ebr-inspect-bytes`
  3. `ebound inspect --json --reconstruction` and `ebound explain` (plain cells)
  4. `ebound unpack --symlinks all` (or `ebr-unpack --encrypted true`)
  5. Per-file SHA-256 and symlink-target comparison against the source item. The last stdout line is JSON.
- **Detail files.** Audits by (transform, reason), region list, payload bytes by transform, mismatch examples and tool logs go to `/root/eb-research/results/EXP-RECON-001/detail/<item>/`. Their SHA-256 is carried in each raw row (`detail_sha256`). At checkpoint the detail JSON files, excluding logs, are archived as `research/raw/EXP-RECON-001/detail.tar.zst` if ≤ 50 MB; otherwise they stay outside git with the SHA-256 manifest committed.

## 9. Seeds, warmups, replication

- Seeds: `order = bootstrap = command = 20260918`. Order is `randomized_blocks`: every cell × item once before the second repetition.
- Warmups: 0 (no timing).
- Replication: 2 repetitions (§4.6 deterministic metrics) in different rounds and processes, plus the repeat-hash check (§4.13). No adaptive rule applies.
- Per-sample timeout: 12 h (extreme on large items).

## 10. Metrics

Canonical names are from Appendix A.2 (thresholds referenced, not restated). Everything else is descriptive per common §8.2.

| Metric | Role here | OD | Family | Source |
|---|---|---|---|---|
| `roundtrip_mismatches` | binary (HC-02) | OD-02 | correctness | wrapper comparison |
| `artifact_bytes` | descriptive (no comparison here; T-01 applies in 004) | OD-05 | size | file size |
| `side_data_bytes` | diagnostic; realised as `reconstruction_section_bytes` (ReconstructionData + Regions sections incl. audit/fallback records; INDEXED only) | OD-05 | size | `ebr-inspect-bytes` |
| `metadata_bytes`, `index_bytes`, `dictionary_bytes` | diagnostic | OD-05 | size | `ebr-inspect-bytes` |
| `access_granularity_bytes` | descriptive here (T-09 applies in 004); realised as production's declared `worst_access_bytes` | OD-10 | size | `ebound inspect` |
| `benign_false_refusal_fraction` | computed offline per ceiling candidate (§11); n_cases = number of evaluated archives per cell | OD-17 | other | declarations |
| repeat-hash (`determinism_key`, `lai`) | binary (HC-13) | OD-04 | correctness | archive SHA-256 / LAI |
| `pack_refused`, `pack_crashed`, `unpack_crashed`, reason codes | binary screen (crash = HC-03/HC-06 defect); refusal descriptive | — | correctness | exit status, stderr |
| selection, fallback and bit counts, declared window/working set/flags, explain savings | descriptive | — | other/size | `ebound inspect`, `explain` |

## 11. Normalization and analysis

1. **Verification and normalisation.** `ebr verify-raw` must pass before `normalize`. Samples with a wrapper failure (exit 2) are infrastructure failures: rerun them, keeping every attempt (§5.6 analogue for tuning).
2. **HC-02.** Sum `roundtrip_mismatches` over all evaluated samples (AGG-W). Any value > 0 is a violation. Samples with `roundtrip_evaluated = 0` (unpack refused) are listed per cell and reason. They are `HC_UNVERIFIED` for that item, never passes.
3. **HC-13.** Per (cell, item): the set of `determinism_key` values must be a singleton for plain cells, and `lai` a singleton for encrypted cells. A non-singleton is a violation.
4. **Layout equivalence (MVT-10 / HC-10 facts).** For each (profile, item), `lai` and `pcr` of `*-indexed-plain` and `*-stream-plain` must be equal. Differences are reported as HC-10 findings to the container domain.
5. **Path coverage (MVT-02(e)).** Per (profile, layout, encryption) cell over the split: count items and objects with ReconstructionData (DEFLATE path) and with JPEG regions (JPEG path). Cells with < 5 exercises of a registered path are `HC_UNVERIFIED` for that path. The generated items needed to reach 5 are specified in EXP-RECON-006 §6 (S-RESOURCE) and fed back to the next run of this spec as an amendment made before results for those items exist.
6. **Status-quo tables** (descriptive, per family and tier, groups weighted per §4.9):
   - share of archives with ≥ 1 selection per reconstruction kind;
   - audit-reason distribution;
   - production replayed/derived net savings as a fraction of `artifact_bytes`;
   - share of archives requiring 0x4/0x8 with and without emitted reconstructive bytes (H1e);
   - declared working-set distribution (H1f).
7. **Ceiling refusal shares** (DEC-CON-005/DEC-CMP-016, exploratory). For each ceiling candidate, `benign_false_refusal_fraction` is the fraction of archives per profile cell whose declared `window_bytes` or `working_set_bytes` exceeds the ceiling. Candidates:
   - `status-quo-8mib-384mib`
   - `zstd-cli-128mib` (128 MiB working set, 8 MiB window)
   - `low-memory-default-with-opt-in` at 64 MiB
   - `per-feature-ceilings` (reconstruction working set counted separately from codec state)
   - `profile-scaled-ceilings` (fast < balanced < dense < extreme, values fixed in EXP-RECON-006 §5)

   Comparison between ceilings is tuning-exploratory. The confirmatory analysis is in EXP-RECON-006, which adds measured peaks. The metric is T-20 (a coverage fraction over the pre-registered case list of tuning archives).
8. No R4 or R8 comparisons are made here. The outputs are R0 inputs (status-quo facts), R1/R2 screen evidence, and the target-family rule check (common §7) cross-checked against EXP-RECON-002.

## 12. Practical-significance thresholds

None applied (descriptive and binary). The binary metrics use §3.3 (no band). T-01, T-09 and T-20 apply where these quantities become primary in EXP-RECON-004 and 006.

## 13. Sensitivity analysis

Not applicable to binary screens. Descriptive tables are reported with and without F20, with and without derived-producer items, and by scale tier (common §10 reconstruction arms).

## 14. Expected negative results worth recording

- Zero DEFLATE-reconstruction selections in balanced on most families. This follows from the one-Chunk eligibility and the 2 %/256 B gain rule; in the wiring smoke, 3 of 3 generated DEFLATE streams were cost-rejected. If confirmed, the status-quo DEFLATE path is mostly dead weight in the default profile, which is evidence for `deflate-drop`, `deflate-dense-extreme-only` or `recon-opt-in-not-default`.
- Bits 0x4/0x8 required by archives that contain no reconstructive byte (H1e). Minimal readers are refused for no content reason (DEC-CON-010).
- Encrypted archives never exercise reconstruction (H1d). Encryption silently drops the size lever.
- Pack refusals or crashes on benign items caused by region or reconstruction bounds (DEC-CMP-016 `planner-mirrors-validation`).
- `HC_UNVERIFIED` path-coverage cells (H1c).

## 15. Threats to validity

- Harness-built `ebr-pack`/`ebr-unpack`/`ebr-inspect-bytes` are used for encrypted cells and byte accounting. Mitigated by `harness-cli-parity.sh` PASS (plain cells) and MVT-16(c). Encrypted-cell parity is argued, not proven: no CLI encrypted path is scriptable.
- `explain` DEFLATE savings are `[NOT_RECORDED]` replays and JPEG savings are `[DERIVED]`. They are descriptive only; decision-grade net savings come from EXP-RECON-004 real ablations.
- The round-trip oracle compares regular-file bytes and symlink targets only. Metadata restoration is out of scope (platform domain). Unpack policy refusals make items `HC_UNVERIFIED`, not passes.
- Tuning-only prevalence is not representative (common §13 item 1).
- All other threats: common §13.

## 16. Held-out placeholder

This spec, copied with `splits: [heldout]`, runs once after Commit C (§5.5) as the held-out HC screen of every frozen reconstruction candidate that emits under v6. The held-out run has no MDE: binary screens only.

## 17. Cost and effort estimates

- **Compute.** About 80 CPU-hours: 11 cells × 36.1 GiB × 2 repetitions of pack + unpack + hash; extreme cells dominate. The runner is sequential, so wall time is about equal to CPU time unless the runner gains item sharding.
- **Disk.** Scratch peak ≤ 12 GiB; detail about 3 GB; raw rows (gzip) < 50 MB.
- **Agent effort.** Execution: none (scripted). Analysis: judgment (one analysis session reading generated tables).

## 18. Looks, deviations and outcome (append-only)

- Looks: none planned in this spec (tuning only).
- Deviations: none.
- Outcome: `outcome.json` after analysis (§10.1).
