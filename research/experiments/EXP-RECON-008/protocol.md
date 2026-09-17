# EXP-RECON-008: Decoder independence of reconstruction formats — cross-version, cross-implementation and cross-platform decoding of extracted reconstruction vectors; candidate permanent vectors

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-RECON-008 |
| Domain | reconstruction (8.8); common provisions: `research/experiments/_index/reconstruction-common.md` |
| Status | **NEEDS_TOOLING**. Required: `ebr-recon vectors extract|decode|mutate`; per-release research workspaces `research/harness/recon-xver/{preflate-rs,jxl-oxide,lzma-rust2}-<ver>`; libjxl pinned build (shared with EXP-RECON-003); Ubuntu `libjxl-tools` 0.7.0 (installed; `djxl v0.7.0` observed on 2026-09-17); `qemu-user-static` and the `aarch64-unknown-linux-gnu` cross toolchain in WSL (approved downloads); a Windows research reader build. |
| Stage | Correctness and independence evidence on tuning vectors. There are no comparisons with bands and no validation looks. Validation vectors are decoded at EXP-RECON-004 look 1 for R1 screens only. |
| decision_ids | DEC-INT-030, DEC-CMP-038, DEC-CON-007, DEC-CMP-023, DEC-CMP-026 |
| Proposed types/ODs/HCs | common §2: FORMAL + EMPIRICAL. OD-04 (`reader_disagreements`), OD-18 (`cross_host_identity_agreement` for decode outputs). HC-11 (MVT-11(a) variation matrix restricted to reconstruction decode; MVT-11(b) alternate decoder), HC-12(c) (permanent vectors for gated steps), HC-13 (forward determinism within host class), HC-16 (frozen IDs decode unchanged). |
| Gates | G-A. HC findings are defects regardless. |
| Author and exposure | common §3 |
| Feasibility smoke | none. A tool presence check only: `djxl` reports v0.7.0 in WSL. |

## 2. Question

Can reconstruction data written under the frozen identifiers be decoded to the exact original bytes by the following, with identical results?
- **(a)** later releases of the same crates (preflate-rs, jxl-oxide);
- **(b)** an independent implementation (libjxl for JPEG XL; any available decoder of preflate-rs corrections);
- **(c)** the production reader under platform, CPU-feature, architecture (emulated), thread, locale and time-zone variation.

And:
- Do independently encoded JPEG XL representations (libjxl) reconstruct identically in the production reader stack?
- Is forward encoding deterministic across hosts?
- Which minimal vector set covers the reconstruction paths as candidate permanent vectors?

## 3. Hypotheses

All hypotheses are `[INFERRED]`.

- **H1 (JPEG XL reconstruction is standard-defined).** Every later jxl-oxide release and libjxl `djxl` (pinned and 0.7.0) reconstruct byte-identical JPEGs from 100 % of jixel-0.2.19 representations.
  - **Falsified by** any disagreement the triage rule (§11 step 2) does not attribute to a bug in the other decoder.
- **H2 (reverse direction).** jxl-oxide 0.12.6, as used by the production reader, reconstructs byte-identical JPEGs from ≥ 99 % of libjxl-encoded (`cjxl --lossless_jpeg=1`) representations of the files libjxl accepts.
  - **Falsified if** < 99 %.
- **H3 (preflate corrections are version-bound).** At least one preflate-rs release after 0.7.6 fails to recreate exact bytes from at least one 0.7.6 vector.
  - **Falsified if** every later release agrees on 100 % of vectors.
- **H4 (HC-11).** Production reconstruction decode output digests and outcome classes are identical across all variation cells (§5): 0 `reader_disagreements`.
  - **Falsified by** any difference: an R1 violation of the status quo, with artifacts committed.
- **H5 (forward determinism).** jixel-0.2.19 and preflate-rs-0.7.6 forward representations are byte-identical within each host class across repetitions (HC-13), and across WSL x86-64, Windows x86-64 and emulated arm64.
  - **Falsified within a host class:** R1 violation.
  - **Falsified across host classes:** reported. HC-13 MVT-13(a) does not require cross-class PCI identity, but a cross-architecture difference in planner output under one frozen planner ID is recorded against DEC-CMP-038 (the "planner determinism" scope).
- **H6 (lzma-rust2, DEC-CON-007 reconstruction-adjacent arm).** Declared working-set numbers for the three production LZMA2 presets are identical across lzma-rust2 releases ≥ 0.20.0.
  - **Falsified by** any difference.

## 4. Decisions informed and how

| Decision | Candidates (ledger) | Contribution |
|---|---|---|
| DEC-INT-030 | status-quo-own-decoder, registered-test-vectors, ci-differential-independent, pinned-reference-decoder, extended-tier-only (`trust-encoder-no-verify` excluded) | "Differential decoding with independent preflate and JPEG XL implementations" (H1–H3); "vectors across builds and platforms (x86_64, emulated ARM64)" (H4/H5); the candidate permanent vector set with coverage (§11 step 5); feasibility of `ci-differential-independent` (an independent implementation exists for JPEG XL, but H3 and the EXP-RECON-011 search show whether one exists for preflate corrections) |
| DEC-CMP-038 | status-quo-crate-bound, write-independent-spec, vendor-and-freeze, extended-set-crate-reference, standard-format-only, exclude-from-v1 | "Decode compatibility of 0.7.6 correction data with later preflate-rs releases" (H3); "cross-implementation reconstruction of JPEG XL payloads (jxl-oxide vs libjxl)" (H1/H2). `standard-format-only` is supportable only if H1 and H2 hold. |
| DEC-CON-007 | dependency-defined-status-quo, vendor-and-pin-forever, independent-spec-plus-second-implementation, freeze-derived-constants, extended-set-with-refusal-profile | Cross-version differential decoding (H1, H3) and lzma-rust2 estimates (H6) |
| DEC-CMP-023 | promote or withdraw generations | Longevity evidence per generation: whether v5 and v6 decode survives dependency upgrades |
| DEC-CMP-026 | backend choice | Whether `jpeg-libjxl` could decode archives written by jixel and vice versa (a backend switch without a new format) |

## 5. Candidates (decoder implementations and variation cells)

**Native Linux cells (`spec.yaml`):**

| Cell | Definition |
|---|---|
| `deflate-preflate-rs-0.7.6-ref` | `recreate_whole_deflate_stream` at 0.7.6 plus production wrapper check (reference) |
| `deflate-preflate-rs-release-set` | the same call compiled against every preflate-rs release after 0.7.6 up to the EXP-RECON-003 latest pin, one workspace per release; per-release agreement reported |
| `jpeg-jxl-oxide-0.12.6-ref` | production `jpeg_reconstruction::inverse` (reference) |
| `jpeg-jxl-oxide-release-set` | jxl-oxide `reconstruct_jpeg` at every release after 0.12.6 up to the latest pin |
| `jpeg-libjxl-djxl-pinned` | `djxl --num_threads=1 <rep>.jxl out.jpg` (pinned libjxl build) |
| `jpeg-libjxl-djxl-ubuntu-0.7.0` | `/usr/bin/djxl` from Ubuntu `libjxl-tools` 0.7.0 |
| `jpeg-libjxl-encoded-to-jxl-oxide` | reverse direction: the vector's original JPEG → `cjxl --lossless_jpeg=1 -e 7` (pinned) → production `jpeg_reconstruction::inverse` |
| `prod-reader-affinity-1`, `-2`, `-8`, `-32` | research reader decoding the vector's single-object archive (built by production v6 dense) under `taskset` vCPU sets {2}, {2,3}, {2..9}, {0..31}, and with the reader's worker-count parameter set to 1/2/8/32 **if** the decode path exposes one (recorded; if none exists, affinity alone varies) |
| `prod-reader-locale-tr`, `prod-reader-locale-ja`, `prod-reader-tz-chatham` | same, with `LC_ALL=tr_TR.UTF-8`, `ja_JP.UTF-8`, `TZ=Pacific/Chatham`; locale canary (Turkish dotted-i mapping differs from `C`), else the cell is `HC_UNVERIFIED` |
| `forward-jixel-determinism-wsl`, `forward-preflate-determinism-wsl` | re-run status-quo forward on the vector's original bytes; representation SHA-256 |
| `lzma-rust2-estimate-release-set` | production LZMA2 preset declared working set per lzma-rust2 release ≥ 0.20.0 |

**Emulated cells (`spec-emulated.yaml`, `platform.emulated: true`, correctness only, §4.1):**
- `prod-reader-qemu-x86_64-noavx2`: `qemu-x86_64-static -cpu Nehalem` running the x86-64 research reader; the CPU model in effect is recorded from `/proc/cpuinfo` inside.
- `prod-reader-qemu-aarch64-cortex-a72` and `prod-reader-qemu-aarch64-max`: `qemu-aarch64-static -cpu <model>` running the `aarch64-unknown-linux-gnu` research reader.
- `forward-jixel-qemu-aarch64`: forward determinism on emulated arm64.

**Windows cells (`spec-windows.yaml`):** `prod-reader-windows-x86_64` and `forward-jixel-windows-x86_64` (native Windows build of the research reader).

**Native ARM64 and macOS:** `PLATFORM_BLOCKED` (EXP-RECON-012).

## 6. Corpus (vectors)

- **Extraction** (tuning; `ebr-recon vectors extract`). From every EXP-RECON-004 tuning archive of cells `{balanced,dense,extreme}-gen-v5` and `-gen-v6`:
  - each ReconstructionData object: ERD1 bytes, max-chain parameter, decoded intermediate, expected original Chunk SHA-256 and length;
  - each JPEG region: JPEG XL representation, expected member Chunk digests and lengths, expected original object SHA-256;
  - one single-object archive per vector (production v6 dense), for reader cells.

  Vectors are de-duplicated by representation SHA-256.
- **Generated vectors.** S-DEFLATE-PRODUCERS and S-JPEG-PRODUCERS items (tuning) passed through status-quo forward, so producer diversity reaches the vector set even where corpus items lack it.
- **Adversarial mutations** (`adversarial-search/`, seed 20260929). 10,000 seeded mutations per class (ERD1 corrections; JPEG XL representation bytes) with digests recomputed. All implementations decode them. Disagreement on accept/reject or output bytes is recorded (§11 step 3).
- **Manifest.** `vectors-tuning.jsonl`: one row per source corpus item, whose directory holds that item's vectors (path under `/root/eb-research/results/EXP-RECON-008/vectors/`), with SHA-256 per vector file.
- **Validation.** Vectors extracted at EXP-RECON-004 look 1 are decoded by the reference and all independent implementations for R1 screens only.
- **Held-out.** Phase D placeholder: HC-11 screens of frozen candidates on held-out vectors (common §5).

## 7. Environment and platform requirements

- `wsl-ubuntu` x86-64 (native cells); `windows-host` x86-64 (Windows cells; runner on Windows, no timing); QEMU user-mode in WSL (emulated cells; correctness only).
- Locales `tr_TR.UTF-8` and `ja_JP.UTF-8` generated in WSL (`locale-gen`; package install approved). Windows locale and time-zone variation is `HC_UNVERIFIED` (a settings change is not allowed; objective MVT-11(a)).
- No timing. Storage: vectors about 15 GB outside git; committed candidate permanent vector subset ≤ 20 MB.
- `PLATFORM_BLOCKED`: native ARM64, macOS (EXP-RECON-012). Docker is unavailable and not needed: QEMU user-mode runs directly in WSL.

## 8. Commands and tooling to build

```sh
C:/Python313/python.exe research/orchestration/disk_guard.py
# tooling install (WSL, approved downloads): apt-get install qemu-user-static gcc-aarch64-linux-gnu locales && locale-gen tr_TR.UTF-8 ja_JP.UTF-8
#   rustup target add aarch64-unknown-linux-gnu --toolchain 1.98.1
# per-release workspaces (to be written): research/harness/recon-xver/make_release_workspaces.py --crates preflate-rs,jxl-oxide,lzma-rust2 \
#   --from 0.7.6,0.12.6,0.20.0 --to-manifest research/harness/recon-backends/manifest.json
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && /root/eb-research/tools/EXP-RECON-008/bin/ebr-recon vectors extract \
   --archives-manifest research/normalized/EXP-RECON-004/archives-tuning.jsonl --out /root/eb-research/results/EXP-RECON-008/vectors \
   --manifest-out research/experiments/EXP-RECON-008/vectors-tuning.jsonl'
# runner (WSL native, then emulated): ebr validate/run/verify-raw/normalize on spec.yaml and spec-emulated.yaml
# runner (Windows host, PowerShell): C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-RECON-008/spec-windows.yaml  (PYTHONPATH=research/tools)
# adversarial: ebr-recon vectors mutate --seed 20260929 --count 10000 --classes erd1,jxl --out research/experiments/EXP-RECON-008/adversarial-search/
# analysis (to be written): research/tools/recon/independence_tables.py
```

## 9. Seeds, warmups, replication

- Seeds: order, bootstrap and command 20260929; mutation seed 20260929.
- Warmups: 0.
- Repetitions: 3 per variation cell (objective MVT-13(a) determinism-claim count), plus a 20-repetition stress cell. The stress cell is `prod-reader-affinity-32` on the 50 largest vectors, at maximum affinity with the seeded background CPU load of `research/tools/ebr` guard tests (MVT-13(a) stress).
- Cross-version and cross-implementation cells: 2 repetitions (deterministic).

## 10. Metrics

| Metric (canonical) | Role | Notes |
|---|---|---|
| `reader_disagreements` | binary (HC-03/HC-11) | distinct (vector, cell pair) where the production reader's (outcome class, reason code, output digest) differs across variation cells |
| `cross_host_identity_agreement` | binary per vector (decode outputs across Windows, WSL and emulated) | forward representation identity across hosts is descriptive (H5 across classes) |
| `unspecified_baseline_decode_steps` | binary input to EXP-RECON-011 | a reconstruction step on a baseline path without a public spec or vectors (HC-12(c)) |
| cross-version agreement fraction, cross-implementation agreement fraction, reverse-direction success fraction | descriptive (non-canonical) | inputs to FORMAL/EXTERNAL parts of DEC-CMP-038, DEC-CON-007, DEC-INT-030 |
| vector coverage counts per path class | descriptive | MVT-02(e) and HC-12(c) coverage |

## 11. Analysis

1. **HC-11 and HC-13 screens (binary).** Sum `reader_disagreements` over all variation cells (AGG-W). A cell whose canary shows no locale effect is `HC_UNVERIFIED`. OOM kills are classified separately (MVT-11(a)). Forward representation hash sets must be singletons within each host class.
2. **Triage rule (objective §2.0 rule 8).**
   - A disagreement between the production stack and another decoder counts as a bug in the other decoder only if a cited normative sentence (document, section and line) unambiguously mandates the production behaviour. That citation comes from ISO/IEC 18181-1/-2 for JPEG XL, whose access is an EXTERNAL-review matter (common §12), or from `docs/reconstructive-transform-v1.md` for ERD1 framing.
   - A session involved in neither decoder adjudicates. The triage log is committed.
   - Uncited disagreements count as "defined by implementation" evidence against `standard-format-only` and `independent-spec-plus-second-implementation`.
3. **Adversarial differential.** For mutated inputs, report accept/reject and output disagreements per implementation pair. Any input that the production reader accepts as `OK` while an independent standard decoder rejects or decodes differently is listed for triage. It is not an HC violation by itself, because production reader behaviour is what the frozen identifier defines, but it is counterevidence for standard-defined claims.
4. **Per-decision evidence tables.**
   - DEC-CMP-038 and DEC-CON-007: agreement fractions per release, per implementation and per path class (H1–H3, H6).
   - DEC-INT-030: whether each candidate is feasible (vectors reproduce on every platform; an independent decoder exists and agrees).
5. **Candidate permanent vector set (for `registered-test-vectors`; rule fixed now).** Choose a minimal set by greedy covering:
   - DEFLATE: every wrapper (gzip, zlib, raw) × max-chain (512, 2048, 4096) × producer class present × size stratum (< 64 KiB, 64 KiB–1 MiB, 1–16 MiB);
   - JPEG: every present {chroma sampling, restart interval yes/no, APPn marker-set class, size stratum};
   - every adversarial mutation class that production accepts.

   Constraints: each chosen vector decodes identically in every cell and implementation that H1/H3 do not flag, and the set totals ≤ 20 MB. The set is written to `research/experiments/EXP-RECON-008/candidate-vectors/` with SHA-256 and provenance. Its adoption into `docs/*vectors*` is a production change governed by the decision, not by this experiment.

## 12. Practical-significance thresholds

None. All decision-relevant metrics here are binary (§3.3) or descriptive.

## 13. Sensitivity analysis

- Disagreement tables with and without generated producer vectors and adversarial mutations.
- Separate reporting for vectors from jpegli and zopfli producers (known-hard).

## 14. Expected negative results worth recording

- preflate-rs 0.7.6 corrections not decodable by later releases (H3 holds): the v5 format is bound to one crate version forever, and `vendor-and-pin-forever` or `write-independent-spec` become the only longevity routes.
- No independent decoder of the preflate correction format exists: `ci-differential-independent` is infeasible for DEFLATE.
- libjxl and jxl-oxide disagree on some representations (H1 falsified): JPEG reconstruction is not purely standard-defined in practice.
- Emulated arm64 or no-AVX2 decode differences (H4 falsified): an HC-11 defect.

## 15. Threats to validity

- **Emulation.** QEMU user-mode may not reproduce all CPU-feature dispatch. The CPU model in effect is recorded. Native ARM64 remains blocked.
- **Release-set completeness.** Releases are enumerated from the crates.io index snapshot at the tooling date, with the snapshot hash recorded. Yanked releases are included only if still downloadable.
- **Vector representativeness.** Vectors come from what the status-quo planner selected on tuning plus generated producers. Selection bias toward "easy" inputs is mitigated by adversarial mutations.
- **Standards access.** ISO/IEC 18181 text may not be available to program agents, so triage citations may be impossible. Such disagreements remain uncited and count against standard-defined claims until external review (common §12).
- All other threats: common §13.

## 16. Held-out placeholder

After Commit C, vectors are extracted from held-out archives of frozen candidates and decoded by the reference and independent implementations and under the variation matrix (R5 (a) HC screens). Report-only for descriptive agreement fractions.

## 17. Cost and effort estimates

- **Compute.** About 70 CPU-hours: vectors × about 20 native cells × 2–3 repetitions (fast decodes); emulated arm64 decode about 20× slower on the largest vectors (bounded by the stress cell selection); 20,000 mutations × implementations.
- **Disk.** Vectors about 15 GB; per-release workspace builds about 10 GB; results < 1 GB.
- **Tooling.** Vectors subcommands, release workspaces, cross toolchain and QEMU setup, Windows reader build: about two harness sessions.
- **Agent effort.** Execution: none. Triage: judgment (independent). Analysis: judgment.

## 18. Looks, deviations and outcome (append-only)

- Looks: none (validation vectors decoded for R1 screens inside EXP-RECON-004 look 1).
- Deviations: none.
- Outcome: `outcome.json`.
