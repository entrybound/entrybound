# EXP-RECON-007: Containment of reconstruction dependencies — coverage-targeted fuzzing of jixel, jxl-oxide and preflate-rs entry points; panic=abort behaviour; subprocess-isolation and encoder-cap prototypes on the finding corpus

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-RECON-007 |
| Domain | reconstruction (8.8); common provisions: `research/experiments/_index/reconstruction-common.md` |
| Status | **NEEDS_TOOLING**. Required: fuzz crate `research/harness/fuzz-recon` (6 targets); campaign script; `ebound` research build profile `release-panic-abort`; research builds `v6-subprocess-isolated` and `encoder-cap-{128,256,512}`; replay subcommand `ebr-recon contain-replay`. |
| Stage | Tuning (campaigns use tuning-derived seeds only), then replay of the finding corpus under each containment candidate. There are no validation looks: the metrics are binary and count-based, but campaign results are re-checked by the independent re-analysis (CB-7). |
| decision_ids | DEC-CMP-037, DEC-CON-007, DEC-CMP-038, DEC-CMP-016 |
| Proposed types/ODs/HCs | common §2. OD-24 (`unresolved_fuzz_findings_at_coverage_target`, binary), HC-03 and HC-06 (findings are defects), HC-15 (typed refusal versus abort), C4 attack-surface inputs |
| Gates | G-A. A finding is a defect regardless of gates. Selection among containment candidates needs EXP-RECON-005 overhead (G-B) and EXP-RECON-011 cost. |
| Author and exposure | common §3 |
| Feasibility smoke | none |

## 2. Question

The reconstruction dependencies are young and parse attacker-controlled bytes on both the pack side (hostile inputs) and the read side (hostile archives). This experiment asks:
- Under coverage-targeted fuzzing, do they panic, abort, hang, or exceed declared memory?
- Does the status-quo containment (`catch_unwind` plus the jxl-oxide allocation tracker) turn every such event into a typed refusal and fallback? What changes under `panic=abort` builds?
- Do subprocess isolation or an encoder allocation cap contain every finding?

## 3. Hypotheses

All hypotheses are `[INFERRED]`. The absence of findings is evidence under M24.2's coverage rule, not proof.

- **H1 (maturity).** At least one target produces at least one finding (crash, abort, timeout, or memory above the declared bound) before its coverage target or ceiling.
  - **Falsified if** all targets reach the coverage target with zero findings. That outcome is evidence for `fuzz-then-accept` within the enumerated targets.
- **H2 (catch_unwind sufficiency).** At least one finding cannot be contained by `catch_unwind`: stack overflow, allocation-failure abort, double panic, an infinite loop hitting the timeout, or memory beyond the declared bound before the tracker trips.
  - **Falsified if** every finding replayed through the production entry points yields a typed refusal and a fallback.
- **H3 (panic=abort).** Under `release-panic-abort`, at least one input that is a typed refusal in the unwind build terminates the process. **Falsified if** none does. H3 is not evaluable if there are no panic findings.
- **H4 (subprocess isolation).** `v6-subprocess-isolated` yields a typed refusal and fallback, with the parent process surviving, for 100 % of the finding corpus (`adversarial_true_refusal_fraction` = 1). **Falsified if** any input aborts the parent or exceeds the parent's memory bound.
- **H5 (encoder cap).** The smallest `encoder-cap` whose tuning pack reach loss (EXP-RECON-006 §11 step 7 and EXP-RECON-002 distributions) is below 1 % of JPEG file bytes prevents all memory-class findings on the forward target. **Falsified if** memory findings remain at that cap.

## 4. Decisions informed and how

| Decision | Candidates (ledger) | Contribution |
|---|---|---|
| DEC-CMP-037 | status-quo-catch-unwind, subprocess-isolation, encoder-allocation-cap, work-unit-timeouts, fuzz-then-accept, exclude-reconstruction | "Fuzzing campaigns of jixel, jxl-oxide and preflate-rs"; "panic=abort build behaviour"; containment outcome of each prototype on the finding corpus. Overhead comes from EXP-RECON-005 (H5 there); the encoder-cap parameter from EXP-RECON-006. For `work-unit-timeouts`, the hang-finding count serves as the necessity signal, while dependency API support is judged in EXP-RECON-011 §D. |
| DEC-CON-007, DEC-CMP-038 | dependency-defined, vendor-and-pin-forever, … | Maturity evidence: finding counts, and whether fixes exist in later releases. Each finding is replayed on the `*-latest` pins of EXP-RECON-003; a finding fixed only in a later release shows the maintenance pressure on a version-pinned format. |
| DEC-CMP-016 | limits | Memory-class findings within the frozen limits (limits insufficient) versus outside them (limits effective) |

## 5. Candidates

**Containment candidates** (replay arm; spec cells):

| Cell | Definition |
|---|---|
| `status-quo-catch-unwind` | production release build (`panic = unwind`), in-process, `catch_unwind` + jxl-oxide `AllocTracker` 256 MiB |
| `panic-abort-build` | same source, `-C panic=abort` research profile `release-panic-abort` (no code change) |
| `subprocess-isolation` | research build: forward and inverse reconstruction run in a child `ebr-recon recon-worker` with `RLIMIT_AS` = declared working set + 64 MiB and wall timeout 120 s. The parent maps signal, exit or timeout to the typed failure used today (`Unsupported`/`VerificationFailed` for pack, which falls back; `CORRUPT` with a reconstruction reason code for read). |
| `encoder-cap-128`, `encoder-cap-256`, `encoder-cap-512` | status quo plus a refusal of forward reconstruction when the megapixel-derived forward-peak estimate (EXP-RECON-006 regression) exceeds 128, 256 or 512 MiB (`ResourceExcluded`, fallback) |
| `latest-deps` | status-quo code with jixel, jxl-oxide and preflate-rs at the EXP-RECON-003 `*-latest` pins (research workspace) |

- `fuzz-then-accept` is a decision rule applied to H1's outcome. `exclude-reconstruction` is measured by size in EXP-RECON-004. `work-unit-timeouts` has no implementable prototype without dependency changes: recorded, and evaluated analytically (EXP-RECON-011 §D).

**Fuzz targets** (campaign arm; `research/harness/fuzz-recon/fuzz_targets/`):

| Target | Entry | Flavor |
|---|---|---|
| `jpeg_forward_prod` | `entrybound::research::jpeg_reconstruction::verified_forward` | production (catch_unwind inside) |
| `jpeg_forward_raw` | `jixel::encode_jpeg_lossless_with_config` (threads 1, reconstruction true) + `jxl_oxide` reconstruct | raw dependency (panics visible) |
| `jxl_inverse_prod` | `entrybound::research::jpeg_reconstruction::inverse` | production |
| `deflate_forward_prod` | `entrybound::research::reconstruction::try_forward(data, 4096)` | production |
| `deflate_inverse_raw` | structured (arbitrary plaintext ≤ 1 MiB, arbitrary ERD1 side data) → `preflate_rs::recreate_whole_deflate_stream` + wrapper check | raw dependency |
| `region_archive_decode` | structure-aware: a generated minimal v6 archive whose region record fields and representation bytes are mutated, with digests recomputed → production `ecf::open` + entry read + unpack | production reader |

## 6. Corpus

- **Seeds** (tuning only):
  - JPEG files ≤ 1 MiB and DEFLATE streams ≤ 1 MiB from the EXP-RECON-002 tuning census;
  - JPEG XL representations and ERD1 side data extracted from EXP-RECON-004 tuning archives;
  - EXP-RECON-003 `adversarial-search` cases;
  - EXP-RECON-006 hostile series (downscaled to `-max_len`);
  - embedded streams of F20 tuning items.

  Seed-set SHA-256 manifest: `research/experiments/EXP-RECON-007/seeds-manifest.json`.
- **Finding corpus** (replay arm; rule fixed now): every minimised finding from all campaigns, plus every non-exact, crash or timeout input from EXP-RECON-003 and every memory or CPU violation input from EXP-RECON-006. Manifest `findings-manifest.jsonl`, committed after campaigns and before replay.
- **Validation and held-out.** Not used. The fuzz and replay results are corpus-independent defect evidence. Held-out HC screens of frozen candidates happen in EXP-RECON-001/004.

## 7. Environment and platform requirements

- `wsl-ubuntu`, x86-64, nightly-2026-08-01 with cargo-fuzz 0.13.2 (installed per PROGRESS). Sanitizer: libFuzzer with ASan (default cargo-fuzz). An ASan run on safe Rust still catches stack overflow and aborts.
- CPU: up to 16 parallel jobs per campaign window. Campaigns are not timing experiments and may run outside quiet windows, but not concurrently with timing windows (§4.2 exclusivity).
- Memory: `-rss_limit_mb` = 2048 per worker; `-malloc_limit_mb` = 1024. Over-limit events are findings, then replayed with the counting allocator (EXP-RECON-006) to classify them against declared bounds.
- No network. No held-out access. Upstream disclosure of findings is an owner decision, not performed by agents.

## 8. Commands and tooling to build

```sh
C:/Python313/python.exe research/orchestration/disk_guard.py
# campaigns (to be written): per target, until the coverage target or the 72 CPU-hour ceiling
wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-RECON-007/run_campaigns.sh \
    --targets jpeg_forward_prod,jpeg_forward_raw,jxl_inverse_prod,deflate_forward_prod,deflate_inverse_raw,region_archive_decode \
    --jobs 16 --max-len 1048576 --timeout-s 60 --rss-limit-mb 2048 --ceiling-cpu-hours 72 \
    --plateau-window-cpu-hours 12 --plateau-min-new-features 10 --seed 20260927 \
    --out /root/eb-research/results/EXP-RECON-007/campaigns
# minimisation + dedup by stack hash (cargo fuzz tmin; stack hash = first 5 frames in entrybound/jixel/jxl_oxide/preflate_rs)
wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-RECON-007/minimise.sh
# replay arm (runner): spec.yaml over findings-manifest.jsonl
# then ebr verify-raw / normalize; triage: research/tools/recon/triage_findings.py (to be written)
```

- **Coverage target (M24.2, fixed now).** A campaign stops at whichever comes first:
  - **plateau:** fewer than 10 new libFuzzer coverage features over the last 12 CPU-hours of the campaign;
  - **ceiling:** 72 CPU-hours.

  The coverage fraction of instrumented edges in each dependency crate is reported (`cargo fuzz coverage` + `llvm-cov`). A target stopped at the ceiling without reaching plateau carries `coverage_target_unmet`, so its zero-finding result cannot support `fuzz-then-accept`.
- **Finding classes.** Crash (panic or abort), timeout (> 60 s), OOM (> RSS limit), ASan report, and a leak report (reported but not a defect unless unbounded).

## 9. Seeds, warmups, replication

- Campaign RNG seed 20260927 + target index. Minimisation is deterministic per input.
- Replay: 2 repetitions per (cell, input). Outcome classes are deterministic except timeouts, where a timeout in either repetition counts.
- No warmups.

## 10. Metrics

| Metric (canonical) | Role | Notes |
|---|---|---|
| `unresolved_fuzz_findings_at_coverage_target` | binary (= 0 required) per target and candidate | "Unresolved" means the finding still reproduces under the candidate's containment as a process abort, hang or bound exceedance. Caught-and-typed findings count as resolved for containment candidates but are still reported as dependency defects (C4/C7). |
| `adversarial_true_refusal_fraction` | binary (= 1 required) per containment cell over the finding corpus | typed refusal + fallback, parent alive |
| `refusal_cpu_s`, `refusal_alloc_peak_bytes` | secondary (cost of containment on hostile input) | T-05, T-06 |
| coverage features, edge-coverage fraction, CPU-hours, findings by class and target, fixed-in-latest flag | descriptive | |

## 11. Analysis

1. **Triage.** An independent session, not the designer of the containment prototypes, classifies each unique finding: dependency defect, Entrybound wrapper defect, or harness artifact. It records a citation to the reproducing input and the stack hash. Harness artifacts are removed with a reason; every classification is logged (objective §2.0 rule 8 analogue).
2. **HC outcomes.** Any Entrybound-reachable production abort, hang, or declared-bound exceedance on the pack or read path is an HC-03/HC-06 defect of the status quo (R1 for `status-quo-catch-unwind`), with the reproducer committed under `research/raw/EXP-RECON-007/violations/`.
3. **Containment cells.** `adversarial_true_refusal_fraction` and unresolved-finding counts per cell (H2–H5). A cell with any unresolved finding fails its binary screen and is eliminated at R1 for DEC-CMP-037.
4. **Maturity signals.**
   - Findings per CPU-hour per target (descriptive only; M24.2 forbids a graded rate).
   - Fixed-in-latest flags.
   - Unresolved advisories are cross-checked by EXP-RECON-011 (RUSTSEC snapshot).
5. **Decision assembly (DEC-CMP-037).** Survivors of the binary screen are compared on:
   - EXP-RECON-005 pack and unpack overhead (OD-06/07 non-inferiority);
   - EXP-RECON-011 cost tiers (for example, subprocess isolation adds a worker protocol and process attack surface, C4).

   R10 applies on ties. `fuzz-then-accept` is admissible only if H1 is falsified with every target at plateau (no `coverage_target_unmet`).

## 12. Practical-significance thresholds

Binary metrics per §3.3. T-05/T-06 are used only for secondary refusal-cost reporting. Overhead thresholds live in EXP-RECON-005.

## 13. Sensitivity analysis

- Campaign length: findings by the 24, 48 and 72 CPU-hour marks (discovery curve).
- Replay results with and without harness-classified artifacts (CB-5).
- A second seed (20260928) for the two production read-path targets (`jxl_inverse_prod`, `region_archive_decode`) at 24 CPU-hours each, to check seed dependence of zero-finding claims.

## 14. Expected negative results worth recording

- Findings in young dependencies that `catch_unwind` cannot contain (H2): status-quo in-process execution is not safe enough.
- `panic=abort` turns contained panics into process aborts (H3): the status quo silently depends on the unwind profile, a release-build constraint to document.
- Subprocess isolation still fails on memory-bound findings when the RLIMIT exceeds the reader's policy (H4 variant).
- Findings fixed only in later dependency releases, which a frozen version-bound format cannot adopt without a new identifier (DEC-CMP-038/CON-007).

## 15. Threats to validity

- **Fuzzing is not exhaustive** (M24.2). Zero findings is regression evidence within targets and coverage only. Mitigations: structure-aware seeds, a second seed, the plateau rule.
- **ASan and libFuzzer builds** differ in allocation behaviour from release builds. Every finding is replayed on the release, `panic-abort` and isolation builds before classification.
- **Harness wrappers** can introduce artifacts. Independent triage mitigates this.
- **Designer bias.** The containment prototypes were designed here. CB-7 independent re-analysis is required before any selection.
- All other threats: common §13.

## 16. Held-out placeholder

Not applicable: no corpus split is used for campaigns. Held-out archives of frozen candidates are screened in EXP-RECON-001/004 (common §5).

## 17. Cost and effort estimates

- **Compute.** At most about 450 CPU-hours: 6 targets × 72 CPU-hour ceiling, plus the second-seed arm at 48, plus minimisation and replay (≤ 20). Wall time is about 2 days at 16 jobs.
- **Disk.** About 20 GB (corpora, crash artifacts, coverage builds).
- **Tooling.** Fuzz crate, campaign and minimise scripts, 3 research build variants, and a replay subcommand: about two harness sessions.
- **Agent effort.** Execution: none. Triage: judgment (an independent session). Analysis: judgment.

## 18. Looks, deviations and outcome (append-only)

- Looks: none.
- Deviations: none.
- Outcome: `outcome.json`.
