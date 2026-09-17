# EXP-CODEC-003: Codec decoder memory: allocator-exact peaks per configuration, declared-requirement tightness and large-window refusals

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CODEC-003 |
| Domain | codecs (program sections 8.6, 8.7) |
| Status | NEEDS_TOOLING. Needs three things: the counting allocator with allocation-site attribution (`research/harness/crates/ebr-alloc`, `decision-method.md` §14 item 17, designed in EXP-CONF-007 and shared with it), the `ebr-codec decode-memory` subcommand, and the window-refusal probe script. |
| Stage | Deterministic memory census: tuning, then validation (W and S as each decision needs). The R2 bound test (a measured peak within the declared or declarable bound) runs on every available tuning and validation item. Held-out: Phase D placeholder. |
| decision_ids | DEC-CMP-045 (decoder memory per window log and LDM; tool refusals for large windows), DEC-CMP-008 and DEC-CMP-009 (the SPEC §5.8 "bounded declarable decode memory" criterion as an R2 test per codec configuration; the `alloc_peak_bytes` guard for the EXP-CODEC-001 portfolios), DEC-CMP-043 (transform inverse buffers), DEC-CON-004 (codec-level attribution of decode memory to state, output and transform buffers, which the declaration candidates would or would not cover) |
| Scope boundary | These related questions are designed by other domains:<br>- archive-level HC-06 ceilings and hostile resource cases (conformance F-RES), default decoder ceilings (DEC-CON-005): EXP-CONF-007;<br>- codec slack and "window above declaration" conformance cases: EXP-CONF-004 (F-CODEC);<br>- dictionary and lookback memory (DEC-CMP-014, DEC-CON-004 lookback share): EXP-CROSSFILE-003/004;<br>- reconstruction working sets: EXP-RECON-001/006;<br>- legacy decoder bombs and import memory (DEC-LEG-052/085): EXP-LEGACY-001 (C14 bomb fixtures), EXP-LEGACY-040 (import memory at scale), EXP-LEGACY-021 (decoder dependency audit);<br>- `lzma-rust2` memory estimates across versions (DEC-CON-007): EXP-ECO-002. |
| Proposed decision type | EMPIRICAL (measured memory); FORMAL (declared-bound formulas, EXP-CODEC-005) |
| Proposed od_ids | OD-08 (`alloc_peak_bytes`), OD-17 (`declared_tightness_ratio`) |
| Proposed hc_ids | HC-06 (MVT-06(b) declaration consistency per codec configuration; the accepted-case bound of MVT-06(a)), HC-11 (decoded output independent of available memory above the declared requirement) |
| Method, gates, author, L7 | As EXP-CODEC-001 section 1 and Appendix C (CC-4, CC-13). HC-06 results for a candidate stay `HC_UNVERIFIED` until the allocator exists (objective HC-06 status). |

## 2. Question

For every codec configuration in the EXP-CODEC-001/004 universes (level, preset, window, LDM, transform), the experiment asks:

- How much memory does decoding a production chunk allocate, counted exactly per allocation and attributed to codec state, output buffer and transform buffers?
- How tight is each candidate definition of the declared decode requirement relative to that?
- Is every codec's decode memory bounded by a formula computable from declared parameters?
- Which common third-party Zstandard decoders refuse large windows by default?

## 3. Hypotheses

- **H1a (DEC-CON-004).** For Zstandard at window log 20, the counting-allocator decode peak minus R0_max exceeds the declared 4 MiB working set once the output buffer is included, and stays within the declaration when output is excluded. The status-quo `working_set_bytes` therefore describes codec state only (candidate `status-quo-codec-state-maxima`).
- **H1b (LZMA2).** The decode peak minus R0_max is at most `lzma2_get_memory_usage(dict)` plus the output buffer for all three registered pairs. `declared_tightness_ratio` is inside the T-23 band of 1 under state-only accounting.
- **H1c (DEC-CMP-045).** Zstandard decode memory grows with the frame window, not with chunk size. At log 27, state-only decode allocates about 2^27 bytes plus tables even for a 128 KiB chunk, unless the decoder sizes its window to the frame content size (single-segment frames). Production frames are single-segment, so the prediction is that allocation follows content size and the window series is `EQUIVALENT` for chunks of at most 2 MiB.
- **H1d (DEC-CMP-009).** PPMd decode memory equals its model size (16/64/256 MiB at 7-Zip levels 5/7/9, reduced for small inputs), which is declarable. brotli large-window (lgwin 30) is bounded by 2^30 plus tables, and is declarable. bsc decode memory is bounded by a block-size multiple. Every extended candidate passes the "bounded declarable" test, and the failures lie in the specification criterion (EXP-CODEC-005).
- **H1e (tool refusals).** zstd CLI 1.5.5 without `--long`, libzstd's default `windowLogMax` of 27, and python-zstandard's default limit refuse frames with windows above log 27. ruzstd accepts up to its compiled maximum.
- **H1f (DEC-CMP-043).** Structural inverse transforms allocate one intermediate buffer equal to the chunk length (`transform::research::intermediate_len`) and nothing else.
- **H0.** Every measured decode peak lies within the status-quo declaration under total accounting.
- **Falsification.** Each hypothesis is decided by the exact integer oracles below, without bands.

## 4. Candidates

| Decision | Candidates (ledger) | Measured here as |
|---|---|---|
| DEC-CON-004 | status-quo-codec-state-maxima (reference); total-peak-memory-definition; separate-state-and-output-fields; per-group-declarations; flag-registry-reject-unknown; flags-removed | For each (codec, level/preset, window, transform) cell: measured state, output and transform peaks, attributed per allocation site. The tightness each of the first three candidates would achieve is computed from the same measurements. `per-group-declarations`, the flag candidates and lookback shares are the other domains' (scope boundary). |
| DEC-CMP-045 | as EXP-CODEC-001 section 5.2 | decode `alloc_peak_bytes` per window log 17-27 with and without LDM; tool refusal matrix |
| DEC-CMP-008/009 | as EXP-CODEC-001 section 5.2 | R2 bounded-declarable-memory test per configuration: a written bound formula exists (EXP-CODEC-005), **and** measured peaks never exceed it on any tuning or validation item, including F20 high-entropy items |
| DEC-CMP-043 | as EXP-CODEC-004 section 4 | transform inverse buffers |

## 5. Corpus and generated inputs

- **Benign payloads.** Every tuning and validation item. Per item, up to 8 unique production chunks (object-preserving, seed 20260917) per (configuration, profile chunking). Every Stage B survivor configuration of EXP-CODEC-001/004 is added mechanically (the `make_candidates.py` rule of EXP-CODEC-002).
- **Window-limit set.** Zstandard frames at window logs 20-31 over a 256 MiB generated `text-v1` stream followed by a CSPRNG tail, produced by the zstd CLI 1.5.5 with `--long=N` and `--no-content-size` (multi-segment frames), and also with content size present (single-segment where applicable). The generator `window_frames.sh` (seed 20260917) writes under `/root/eb-research/generated/EXP-CODEC-003/window-v1/` and records SHA-256 values in `window-v1.lock.json`, committed before any result.

## 6. Environment

- WSL2 Ubuntu 24.04 (x86-64). The counting allocator runs in-process. External decoders are measured with GNU time (`resource.max_rss_bytes`), using only rows whose `measurement.method` = `gnu-time-v+wait4` (harness review R1-16).
- The allocator build lives in `/root/eb-research/target/harness-alloc`, separate from the timing build, so the allocator never affects EXP-CODEC-002.
- Windows allocator arm: a 10% seeded sample, informational only, checking that allocation sites and requested sizes match (§4.14: memory is never compared across operating systems).

## 7. Tooling and commands

### 7.1 To build

1. `ebr-alloc`, shared with EXP-CONF-007 (one crate, built once): a counting `GlobalAlloc` wrapper with allocation-site attribution. Its `unsafe impl` needs a documented, reviewed lint exception confined to the research harness workspace, never production crates (F-23).
2. `ebr-codec decode-memory --candidate-exact <label> --chunk-profile <p> --payload-sample 8 --r0-runs 20 --summary-stdout`. It emits:
   - `r0_max_bytes`: the maximum allocator peak over 20 decodes of a minimal valid payload of the same configuration;
   - `alloc_peak_bytes`: the maximum over sampled payloads of peak minus `r0_max_bytes`;
   - per-site attribution: `state`, `output`, `transform_intermediate`, `other`;
   - `declared_window_bytes` and `declared_working_set_bytes`, from `codec::research::aggregate_decode_requirements`/`zstd_decode_requirements` for production configurations, or from the EXP-CODEC-005 bound formula for external ones;
   - `declared_tightness_ratio` per accounting definition (state-only, state+output, total);
   - `bound_exceedances`: the count of payloads whose peak exceeds `r0_max_bytes` plus the bound under each definition.
3. `research/experiments/EXP-CODEC-003/window_refusals.sh`. It decodes the window-limit set with:
   - zstd CLI 1.5.5 (default, `--long=31`, `--memory=2048MB`);
   - python-zstandard (pinned wheel, default `max_window_size`);
   - libzstd with default parameters via the `zstd` crate;
   - ruzstd (pin from EXP-ECO-003);
   - production `decode_payload` (`window_log_max` 20).

   For each decoder it records accept or refuse, the error class, and GNU time RSS, as one JSON line per (decoder, frame).
4. `window_frames.sh` and its lock file.

### 7.2 Commands

```sh
PY=/root/eb-research/venv/bin/python; S=research/experiments/EXP-CODEC-003/spec.yaml
$PY -m ebr validate --spec $S && $PY -m ebr run --spec $S && $PY -m ebr verify-raw --spec $S && $PY -m ebr normalize --spec $S
bash research/experiments/EXP-CODEC-003/window_frames.sh
bash research/experiments/EXP-CODEC-003/window_refusals.sh > research/raw/EXP-CODEC-003-windows/refusals.jsonl
```

## 8. Seeds and warmups

- Payload sample seed and window generator seed: 20260917.
- No warmups; the allocator count is exact.
- R0_max is the maximum over 20 runs (objective MVT-06(a); Appendix C.1).

## 9. Metrics

| Metric | OD | Role | Family | Threshold |
|---|---|---|---|---|
| `alloc_peak_bytes` (decode) | OD-08 | primary for DEC-CON-004 attribution comparisons; guard for the DEC-CMP-008/009/045 portfolios and DEC-CMP-043 | memory_peak | T-06 (allocator floor) |
| `declared_tightness_ratio` | OD-17 | primary for DEC-CON-004 (codec part) | other | T-23 |
| bound exceedance under a candidate's claimed-upper-bound definition | — | binary (HC-06 MVT-06(b); R2 for DEC-CMP-008/009) | correctness | §3.3 |
| `peak_rss_bytes` | OD-08 | secondary (external decoders; GNU time only) | memory_peak | T-06 |
| window refusal outcome per tool | — | descriptive | — | — |

## 10. Replication

Single-threaded decode allocator peaks are deterministic: 2 repetitions plus an equality check (§4.13). A mismatch reclassifies the cell as stochastic, measured over 10 runs with the median reported and the maximum used for bound tests. For bound tests, one reproducible exceedance is enough (objective §2.0 rule 2).

## 11. Normalization and aggregation

- `alloc_peak_bytes` and `declared_tightness_ratio` follow CC-9: AGG-G for typical values, AGG-W for the worst case per configuration.
- Binary metrics: AGG-W (any failure).

## 12. Statistical analysis

- Bound tests are exact integer oracles with no statistics. Any exceedance of a claimed bound eliminates the (candidate, bound) pair at R2. If the configuration is production's, it is a production HC-06 defect.
- DEC-CON-004 (codec part): R4 on `declared_tightness_ratio` across the declaration definitions. Values below 1 are HC-06 violations, not trade-offs. This experiment supplies the codec-level evidence; the archive-level decision analysis joins EXP-CONF-007, EXP-CROSSFILE-003/004 and EXP-RECON-001.
- Guards for the portfolio decisions follow CC-10.

## 13. Practical-significance thresholds

T-06, T-23; §3.3 binary metrics.

## 14. Sensitivity

CC-11 for non-binary metrics, plus two arms:

- accounting definition: state-only, state+output, total;
- payload sample size: 8 against 32 chunks on 20 seeded items.

## 15. Held-out (Phase D placeholder)

CC-12. Bound tests are re-run on held-out items after unlock (R5 (a)).

## 16. Expected negative results

- The declared 4 MiB Zstandard working set excludes output buffers. "Declared requirement" is therefore not total memory, which is evidence for a DEC-CON-004 re-specification.
- Common third-party zstd decoders refuse windows above log 27, so per-profile long windows would make archives unreadable by default-configured readers.
- Context-mixing decoders need gigabytes (small-tier measurement only).

## 17. Threats to validity

- The allocator counts requested bytes, not RSS, so fragmentation and allocator metadata are invisible. RSS is reported as secondary.
- Scoped in-process measurement can understate peaks through allocator reuse (harness review L-14). Mitigation: one configuration per process.
- The research build with the counting allocator is instrumented (F-24). Its HC results count only while MVT-16(c) passes for that build (objective §2.0 rule 9).
- Window-refusal behaviour is version-specific. Tool versions are recorded, and any claim is scoped to them.

## 18. Platform and cost

| Item | Value |
|---|---|
| Platforms | Linux x86-64 WSL2 (primary); Windows host (attribution cross-check, informational). No network, no privileges. |
| Compute | Benign census ≈ 8 CPU-h; window refusals ≈ 1 CPU-h. Total ≈ 9 CPU-h. |
| Disk | Window-limit set ≈ 3 GB; raw < 1 GB. |
| Agent effort | Scripted. Judgment only for the analysis, by a session without the L7 exposure. |

## 19. Deviations (append-only)

None.
