# EXP-RECON-003: Stream-level reconstruction backend matrix — exact round-trip success, side data, representation and net stream bytes per producer, coding mode and backend

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-RECON-003 |
| Domain | reconstruction (8.8); common provisions: `research/experiments/_index/reconstruction-common.md` |
| Status | **NEEDS_TOOLING**: `ebr-recon attempt`; pinned builds of external backends; licence census (§8) |
| Stage | Tuning screen: exploratory R1/R2 screens and backend refinement; forms the backend survivor set carried into EXP-RECON-004 modelling. The validation stream runs cover only candidates in S of EXP-RECON-004 and happen inside its registered look. |
| decision_ids | DEC-CMP-018, DEC-CMP-026, DEC-CMP-012, DEC-CMP-038, DEC-CON-007, DEC-CMP-016; informs DEC-CMP-034 (JPEG post-codec lists) |
| Proposed types/ODs/HCs | common §2; HC-02 (status-quo backends: mismatch after verified success = violation), HC-11/HC-12(c) facts (library-defined steps), HC-06 facts (bounds exclusions); R2 licence screen (§7.5) |
| Gates | G-A for the record. Stream-level results are screening inputs; decision-grade size comparisons are artifact-level (EXP-RECON-004). |
| Author and exposure | common §3 |
| Feasibility smoke | none (tool not built) |

## 2. Question

For every DEFLATE and JPEG stream located by the EXP-RECON-002 census, run each backend and configuration. The questions are:
- Does it reproduce the exact original bytes? Which failure class occurs otherwise: not recognised, unsupported, verification failed, backend mismatch after claimed success, crash, timeout, or resource-excluded?
- How many representation and side-data bytes does it need?
- After the profile's production post-codec candidates, what net stream saving does it achieve against the ordinary v4 representation of the same bytes, per producer class, coding mode and container class?

## 3. Hypotheses

All hypotheses are `[INFERRED]` predictions. The literature-derived expectations in the ledger's `required_evidence` are not treated as evidence.

- **H1 (DEC-CMP-018, producer dependence).**
  - preflate-rs 0.7.6 exact-success byte rate on streams with `producer_class = zlib-*` is ≥ 95 %.
  - The median side-data-to-plaintext ratio of non-zlib producer classes (libdeflate levels ≥ 10, zopfli, 7-Zip) is ≥ 5× that of zlib classes.
  - **Falsified if** the zlib success rate is < 95 %, or the non-zlib median ratio is < 2× the zlib ratio.
- **H2 (DEC-CMP-026, backend ordering on SOF0).** On SOF0 files ending at EOI, at least one non-status-quo backend (libjxl, lepton-jpeg-rust, brunsli) has group-mean total stream bytes (best post-codec payload + side data) smaller than `jixel-0.2.19-status-quo` by more than 1 T-01 band unit, in both F12 tuning groups.
  - **Falsified if** no backend is beyond the band in both groups.
  - **H2-EQ:** all group means lie within the band.
- **H3 (progressive).** `jixel-0.2.19-ungated` fails (any non-exact class) on ≥ 50 % of SOF2 files, and at least one other backend is exact on ≥ 95 % of SOF2 files. **Falsified if** jixel fails on < 10 %, or no backend reaches 95 %.
- **H4 (known-hard, MVT-02(d)).** Both JPEG XL backends (jixel, libjxl) are non-exact or cost-losing on ≥ 50 % of jpegli-produced files (S-JPEG-PRODUCERS). **Falsified if** < 10 %.
- **H5 (DEC-CMP-012, always-try raw attempts).** On tuning items outside the DEFLATE target families (common §7 rule), `oracle-chunk-attempts` yields zero exact raw reconstructions that also pass the status-quo gain rule. **Falsified by** any such chunk; each one is listed.
- **H6 (DEC-CMP-016, limits).** Lifting the 16 MiB input and 64 MiB intermediate bounds (`preflate-rs-0.7.6-mc4096-unbounded`) increases exact DEFLATE stream bytes by ≥ 10 % in at least one of F11, F14, F16. **Falsified if** < 10 % in all three.
- **Controls (binary; the run is invalid on failure).**
  - **Positive:** every backend is exact on its built-in reference vectors from the backend's own test suite (§8.2), run once per build.
  - **Negative:** every backend reports not-recognised or unsupported (never exact) on the 4 KiB CSPRNG blob of the EXP-RECON-002 self-test tree.

## 4. Decisions informed and how

| Decision | Contribution | Not settled here |
|---|---|---|
| DEC-CMP-018 | Round-trip success and correction size per producer and container class (the ledger's second required evidence); max-chain effect; cost of the 16/64 MiB bounds; reach of `deflate-zlib-replay` and `deflate-preflate-cpp`; per-stream inputs to modelled candidates (`deflate-embedded-streams`, `deflate-multi-member-gzip`, `deflate-whole-object-regions`, `deflate-preflate-rs-latest`) | Artifact-level net savings (004), timing (005), memory (006) |
| DEC-CMP-026 | Eligibility, verification failures and net stream savings per backend and subset (SOF0/SOF1/SOF2/arithmetic, trailers, embedded), per producer (cameras, phones, web, jpegli, scans once supplemented) | Artifact-level savings, access cost (004); latency (005) |
| DEC-CMP-012 | Always-try oracle: per-chunk raw-attempt outcomes and attempt CPU (informational), giving false-negative/false-positive counts for `deflate-signature-gated` and `deflate-entropy-gated-*` against the EXP-RECON-002 truth set | Decision-grade pack time saved (005); archive-level regret (004) |
| DEC-CMP-038, DEC-CON-007 | Behaviour differences between pinned and latest releases on forward encoding (reach, success, sizes); backend maturity signals (crashes, mismatches after claimed success) | Cross-version and cross-implementation decode of existing data (008); spec feasibility (011) |
| DEC-CMP-016 | Stream counts and bytes excluded by each frozen limit, and what lifting them recovers (H6) | Memory cost of lifting (006) |
| DEC-CMP-034 (informs) | Which v6 JPEG post-codec list members ever produce the minimum on JPEG XL, Lepton or brunsli representations | Planner-domain decision |

## 5. Candidates (backend configurations; common §6 ids in brackets)

| Spec candidate | Definition | Maps to |
|---|---|---|
| `preflate-rs-0.7.6-mc512` / `-mc2048` / `-mc4096` | production `entrybound::research::reconstruction::try_forward` with the frozen bounds, wrapper parsing and exact dual check; raw bodies of embedded streams submitted with the raw wrapper | `deflate-v5-status-quo` (per-profile max-chain) |
| `preflate-rs-0.7.6-mc4096-unbounded` | same crate API (`preflate_whole_deflate_stream`/`recreate_whole_deflate_stream`) called directly by research code with input ≤ 1 GiB and intermediate ≤ 4 GiB; exact dual check | limits arm (DEC-CMP-016 `measured-limits`) |
| `preflate-rs-0.7.6-mc256` / `-mc1024` / `-mc8192` / `-mc32768` | direct crate API, frozen bounds | `deflate-preflate-rs-maxchain-sweep` |
| `preflate-rs-latest-mc512` / `-mc2048` / `-mc4096` | newest preflate-rs release on or before 2026-09-17, pinned in `research/harness/recon-backends/manifest.json` before any run; separate Cargo workspace `research/harness/recon-xver/preflate-latest` | `deflate-preflate-rs-latest` |
| `preflate-cpp-pinned` | preflate C++ library from the pinned precomp-cpp commit, driven by a research wrapper (`recon-backends/preflate-cpp/preflate_stream.cpp`: raw body → unpacked plaintext + reconstruction data → recreated body) | `deflate-preflate-cpp` |
| `zlib-replay` | EXP-RECON-002 `producer_class` exact match → side data = the parameter record (fixed 16 B); else not recognised; recreation by re-deflating with that pinned library; exact dual check | `deflate-zlib-replay` |
| `oracle-chunk-attempts` | production `try_forward` on **every production Chunk** of the item under balanced (512), dense (2048) and extreme (4096) chunk ranges and max-chains; records wrapper class, outcome, intermediate and correction lengths, attempt CPU (child process, informational) | always-try oracle (DEC-CMP-012) |
| `jixel-0.2.19-status-quo` | production `entrybound::research::jpeg_reconstruction::verified_forward` (SOF0 gate, EOI end, 64 MiB, 100 MP, single thread) | `jpeg-v6-status-quo` |
| `jixel-0.2.19-ungated` | jixel 0.2.19 + jxl-oxide 0.12.6 called directly on every whole or embedded JPEG (including SOF1/SOF2/arithmetic, with trailers stripped into a suffix); child process per stream, `catch_unwind` plus signal capture | `jpeg-add-progressive`, `jpeg-add-trailing`, `jpeg-nested` with the pinned stack |
| `jixel-latest-ungated` | newest jixel and jxl-oxide releases on or before 2026-09-17 (pinned in manifest), otherwise as above | `jpeg-jixel-latest` |
| `libjxl-pinned-e7` / `-e9` | libjxl at the pinned release tag built from source: `cjxl --lossless_jpeg=1 -e {7,9} --num_threads=1`, `djxl --num_threads=1` to JPEG | `jpeg-libjxl` |
| `lepton-jpeg-rust-pinned` | `lepton_jpeg` crate at the pinned release, single-threaded API; fallback rule of common §6.2 | `jpeg-lepton` |
| `brunsli-pinned` | google/brunsli at the pinned commit (`cbrunsli`/`dbrunsli`) | `jpeg-brunsli` |
| `packjpg-pinned` | packJPG at the pinned commit; reference only if its LICENSE is strong copyleft (§7.5) | `jpeg-packjpg` |

- **Trailer handling.** For JPEG files with post-EOI bytes, every ungated backend makes two attempts: `whole` (input as is) and `split` (JPEG through EOI via the backend; trailer as raw side data). Both outcomes are recorded.
- **Considered and not included.**
  - `grittibanzli` (DEFLATE re-coder): not named by any ledger row, upstream archived, and the preflate technique class is already represented. A critic may add it under R0 item 6.
  - dropbox/lepton C++: represented by `lepton-jpeg-rust` unless the fallback rule triggers.
  - `pixel-reencode` and `substitutive-recompress`: ledger exclusions (I14), recorded, not measured.
- **Licence screen (§7.5, R2).** `recon-backends/license_census.py` records per backend: the LICENSE file SHA-256 at the pin, the SPDX expression declared by the project, and the first 40 lines of the LICENSE file. A reviewer not involved classifies it. Strong copyleft means measured as reference only, never selectable. Weak copyleft or unusual licences go to external review (§8.3).

## 6. Corpus

- **Tuning.** Every stream in the EXP-RECON-002 tuning detail (all 112 items; items without streams give empty rows).
- **Supplements.** S-DEFLATE-PRODUCERS, S-JPEG-PRODUCERS and S-JPEG-REAL (common §7) are added by amendment before their first run. H1 and H4 are evaluable only on them; if they are absent, H1 and H4 are recorded as `not evaluable`.
- **Adversarial generated cases** (`adversarial-search/`):
  - 5,000 seeded JPEG marker-layout mutations of tuning SOF0/SOF2 files: APPn reorder, fill bytes `FF FF` between markers, DHT/DQT after SOF, restart intervals, redundant EOI, extra scans;
  - 5,000 seeded DEFLATE layouts from tuning plaintexts: stored, fixed-Huffman and empty blocks, `Z_SYNC_FLUSH`/`Z_FULL_FLUSH` cadences, pigz-style independent blocks, pathological code-length trees via a research encoder.

  Seeds: 20260920 (JPEG) and 20260921 (DEFLATE). Generator `adversarial-search/generate.py` (to be written). SHA-256 per case is recorded in `adversarial-search/cases.jsonl`.
- **Validation.** Stream runs for backends in EXP-RECON-004's S only, inside its registered look.
- **Held-out.** Phase D placeholder: only candidates frozen at Commit A (common §5).

## 7. Environment and platform requirements

- `wsl-ubuntu`, x86-64, **no timing** (CPU and memory per attempt are informational smoke only).
- C/C++ toolchain for libjxl, brunsli, packJPG and preflate-cpp: gcc/g++ and cmake from apt, pinned versions recorded. Rust 1.98.1 for the crates.
- Network: one-time source downloads, approved (PROGRESS). No held-out access.
- Storage: backend sources and builds about 6 GB; extracted streams and representations transient; detail about 10 GB outside git.

## 8. Commands and tooling to build

### 8.1 `ebr-recon attempt`

```text
ebr-recon attempt --item-path <dir> --experiment-id EXP-RECON-003 --env-name wsl-ubuntu
                  --backend <spec candidate name> --stream-index <EXP-RECON-002 detail for the item>
                  --post-codec-profiles balanced,dense,extreme --per-stream-timeout-s 600
                  --isolate child --detail-out /root/eb-research/results/EXP-RECON-003/detail/<item>-<backend>-rep<rep>.jsonl.zst
```

For each stream of the backend's class:

1. **Extract.** Take the stream bytes (whole file, or the embedded range plus its wrapper facts).
2. **Attempt.** Run the backend forward and inverse in a child process: `ebr-recon attempt-one`, with `RLIMIT_AS` 8 GiB and wall timeout.
3. **Verify.** Compare SHA-256 and bytes of the recreated stream with the original.
4. **Post-code.** Compress the representation with every member of the profile's production post-codec list, using `entrybound::research::codec` production paths:
   - v5 list for the DEFLATE intermediate;
   - v6 list for JPEG representations.
5. **Ordinary cost.** Compute the ordinary v4 cost of the original stream bytes as one Chunk: `entrybound::research::planner::trace_chunk(profile, V4, bytes)`, selected `complete_cost`.
6. **Net saving.** `net_saving = ordinary − (best post-codec payload + side data + record overhead)`, where record overhead uses the exact v5/v6 record lengths (`encoded_transform_plan_v2_len`, `encoded_reconstruction_data_len`, `encoded_reconstruction_region_len`, section header). `qualifying = net_saving` passes the status-quo gain rule (> 256 B and > 2 %).
7. **Record.** Outcome class, sizes, post-codec sizes per list member, attempt CPU seconds and child `VmHWM` (informational).

It prints one `ebr.harness.raw.v1` summary line.

### 8.2 External backends

`research/harness/recon-backends/<backend>/build.sh` (to be written, one per backend):
- fetch source at the pin, verify the tarball SHA-256, build single-threaded CLI or library wrappers;
- run the backend's own test vectors (positive control);
- write `/root/eb-research/tools/recon-backends/<backend>-<pin>/` plus `manifest.json` entries: source URL, tag or commit, source SHA-256, compiler versions, flags, binary SHA-256, LICENSE SHA-256.

`research/harness/recon-backends/license_census.py` writes `research/experiments/EXP-RECON-003/license-census.json` for reviewer classification.

### 8.3 Runner

```sh
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools &&
  PY=/root/eb-research/venv/bin/python && S=research/experiments/EXP-RECON-003/spec.yaml &&
  $PY -m ebr validate --spec $S && $PY -m ebr run --spec $S --gzip && $PY -m ebr verify-raw --spec $S --refingerprint &&
  $PY -m ebr normalize --spec $S'
# adversarial search (scripted): python3 research/experiments/EXP-RECON-003/adversarial-search/generate.py --seed-jpeg 20260920 --seed-deflate 20260921
#   then ebr-recon attempt over adversarial-search/cases.jsonl for every backend
# analysis (to be written): research/tools/recon/stream_tables.py --census research/normalized/EXP-RECON-002 --normalized research/normalized/EXP-RECON-003
```

## 9. Seeds, warmups, replication

- Seeds: order, bootstrap and command 20260920. Adversarial cases as in §6.
- Warmups: 0.
- Repetitions: 2 (deterministic outputs; repeat-hash of representation bytes per stream via `detail_sha256`). A representation-hash mismatch across repetitions is recorded as backend nondeterminism, with both outputs committed for `jixel-0.2.19-status-quo` and `preflate-rs-0.7.6-*`, where determinism is claimed (HC-13 planner determinism: R1 artifact). For external backends it is a C7 longevity finding.

## 10. Metrics

| Metric | Role | Notes |
|---|---|---|
| `roundtrip_mismatches` | binary | For status-quo backends: exact-claimed success whose production inverse differs (HC-02 violation). For other backends: the backend's own decoder differs after the backend reported success. That is recorded as a backend defect (C4/C7), not an Entrybound HC violation, because Entrybound's write-time check would reject it. |
| `side_data_bytes` | diagnostic (T-02) | correction or side bytes per stream, aggregated per item |
| `artifact_bytes` | not measured here | stream-level `net_saving_bytes` and `qualifying_net_saving_bytes` per profile are descriptive inputs to the exact cost model (common §8.3) |
| outcome-class counts and bytes; `streams_crashed`, `streams_timed_out` | descriptive; crash/timeout counts feed DEC-CMP-037 | |
| attempt CPU seconds, child VmHWM | informational (`NOT_DECISION_GRADE`) | decision-grade in 005/006 |
| licence classification | binary `license_compatibility` (R2) | reviewer |

## 11. Normalization and analysis

1. `verify-raw`, then `normalize`. Failed controls invalidate the backend's samples.
2. **Per stream to item.** Bytes-weighted exact rate per (backend, class, producer class, SOF class). Net and qualifying savings per profile summed per item, then expressed as a ratio to the item's located stream bytes.
3. **Item to group to family.** §4.9 means of item log ratios of "stream bytes after reconstruction ÷ ordinary stream bytes" per backend. Items with no exact streams contribute their ordinary cost, a ratio of 1.
4. **Hypothesis evaluation** (tuning, exploratory, no multiplicity control): H1–H6 on the stated quantities. H2 uses the T-01 band as a descriptive yardstick at group level only.
5. **Backend survivor set for EXP-RECON-004 (tuning R1/R2/R4 screen, rule fixed now).** A configuration is carried forward if all of the following hold:
   - (a) zero unresolved `roundtrip_mismatches` after claimed success, and zero crashes or timeouts on non-adversarial tuning streams (adversarial crashes are carried to EXP-RECON-007 triage, not excluded);
   - (b) its build reproduces the same binary SHA-256 twice (C7 build determinism) and passes its positive control;
   - (c) it is not licence-excluded (R2);
   - (d) in no DEFLATE or JPEG target family is its family mean stream-bytes ratio worse than the status-quo backend of the same class by more than 1 T-01 band unit.

   Status-quo backends are always carried. Among max-chain variants, the smallest-bytes one per profile is carried along with the frozen value.
6. **Gate analysis (DEC-CMP-012, exploratory).** From `oracle-chunk-attempts` and the EXP-RECON-002 truth set, per family: counts of (validated stream present × raw attempt exact × qualifying) and the implied false-negative and false-positive counts for `deflate-signature-gated` and `deflate-entropy-gated-{7000,7500,7800}`. The entropy estimate is computed on each chunk by the integer order-0 estimator defined in EXP-RECON-004 §8.1. Attempt CPU is summed per outcome class (informational).
7. **Post-codec analysis (DEC-CMP-034 informs).** Frequency with which each v6 post-codec list member is the minimum per representation type and profile.

## 12. Practical-significance thresholds

The T-01 band (`artifact_bytes`) is used only as the descriptive yardstick in H2 and in survivor rule (d). Decision-grade application is at artifact level in EXP-RECON-004 (T-01, and R6 minimum gains per computed tier).

## 13. Sensitivity analysis

- Survivor sets recomputed with rule (d) at 0.5 and 2 band units: reported, not governing.
- Exact rates with and without supplements, with and without F20 and adversarial cases, and by stream-size tier (< 64 KiB, 64 KiB–1 MiB, 1–16 MiB, > 16 MiB).

## 14. Expected negative results worth recording

- preflate-rs reach collapses for non-zlib producers: large corrections, no qualifying saving (H1).
- `jixel-0.2.19-ungated` fails on progressive JPEGs (H3), confirming the frozen SOF0 gate as a real limitation.
- JPEG XL backends fail on jpegli output (H4).
- Always-try raw attempts never pay off outside target families (H5), which supports gating.
- The newest releases change reach or sizes, so pinned-version formats cannot simply be upgraded. This is evidence for DEC-CMP-038/CON-007 costs.
- packJPG is licence-excluded despite competitive sizes, recorded as a limitation.

## 15. Threats to validity

- **Stream extraction versus whole-object reality.** Stream-level savings ignore dedup, cohort and chunk-alignment interactions. The exact cost model (common §8.3) and real ablations in 004 correct for this.
- **External CLI overhead and isolation.** Per-stream processes and file I/O inflate CPU. CPU is informational only.
- **Backend configuration fairness.** Every backend is run single-threaded at its documented default and one maximum-effort setting (libjxl e7/e9). Other backends use their single documented mode. Non-default tuning of competitors is out of scope and recorded as a limitation (R0 item 6 critic may add).
- **"Latest" pins** can be superseded (reopen rule §11 item 4).
- **Producer attribution** from EXP-RECON-002 is a lower bound (its §15).
- All other threats: common §13.

## 16. Held-out placeholder

The candidates frozen at Commit A (as backends of frozen archive-level candidates) are run once on held-out streams after Commit C, report-only unless used by a held-out R5 comparison in EXP-RECON-004 (common §5).

## 17. Cost and effort estimates

- **Compute.** About 120 CPU-hours: 22 configurations × tuning streams × 2 repetitions, plus 10,000 adversarial cases × 11 backends.
- **Disk.** About 20 GB: backend builds 6 GB, transient extracted streams, 10 GB detail.
- **Tooling.** Judgment-light scripted builds; one harness session for `ebr-recon attempt` and one for the backend build scripts.
- **Agent effort.** Execution: none. Analysis: scripted tables plus a judgment session for survivor-rule review and licence classification (independent reviewer).

## 18. Looks, deviations and outcome (append-only)

- Looks: none under this spec (tuning). Validation stream runs are part of EXP-RECON-004 look 1.
- Deviations: none.
- Outcome: `outcome.json`.
