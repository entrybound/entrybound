# EXP-RECON-010: Reconstruction wire conformance — structure-aware mutation case table for regions, ReconstructionData and feature bits (both layouts), `plan_ref` sentinel rules, write-time fault injection (HC-02 MVT-02(b)) and mutation adequacy of verification code

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-RECON-010 |
| Domain | reconstruction (8.8); common provisions: `research/experiments/_index/reconstruction-common.md` |
| Status | **NEEDS_TOOLING**. Required: `ebr-recon mutate` (research ECF record re-encoder); default-off production research feature `research-fault-injection` (Internals agent; byte-identical when disabled, MVT-16(c)); research reader variants `--sentinel-rule {distinct-sentinel, explicit-region-flag, forbid-unplanned-zero}`; `cargo-mutants` at a pinned version (install approved); case table `case-table.json` (§6, fixed here in prose; machine form written by the tooling session and checked against this protocol by a reviewer before any run). |
| Stage | Conformance and HC screens (binary and coverage fractions). No validation looks: the case table is not a corpus sample. Wire-compatibility re-decode uses EXP-RECON-001/004 tuning archives, and their validation archives inside look 1. |
| decision_ids | DEC-MOD-040, DEC-INT-030, DEC-CON-010, DEC-CON-004, DEC-CMP-016 |
| Proposed types/ODs/HCs | common §2: DEC-MOD-040 FORMAL + EMPIRICAL. OD-04 (`reader_disagreements`, `reason_code_specificity_fraction`), OD-17 (refusal cost). HC-02 (MVT-02(b) fault injection; MVT-02(e) mutation adequacy), HC-03 (MVT-03(b) differential across layouts and rule readers), HC-04 (unknown transform, format and bits fail closed), HC-06 (declared-size lies refused before allocation), HC-09 (understated access declarations), HC-15 (fault-class matrix per MVT-15(b) precedence), HC-16 (sentinel rule candidates must not reinterpret frozen v6 archives) |
| Gates | G-A. Violations are defects regardless of gates. |
| Author and exposure | common §3 |
| Feasibility smoke | none |

## 2. Question

For archives using DEFLATE reconstruction (bits 0x4) and JPEG whole-object regions (bit 0x8), in INDEXED and STREAM layouts:
- Does the production reader give exactly one conforming outcome, with a stable, specific reason code and no plaintext released, for every structural mutation of region records, chunk frames (`plan_ref`, region-owned flag), ReconstructionData and feature bits?
- Is the shared `plan_ref = 0` sentinel ambiguous? Would the alternative sentinel rules reject or reinterpret any existing frozen v6 archive?
- Does the writer refuse, emitting no archive, whenever a reconstruction is one bit wrong?
- Do the tests kill mutants of the verification code?

## 3. Hypotheses

All hypotheses are `[INFERRED]`.

- **H1 (HC-03/HC-15).** Every case in the table yields its expected outcome class, with zero output bytes, identically on the INDEXED and STREAM read paths.
  - **Falsified by** any off-diagonal cell (MVT-15(b) precedence) or any layout disagreement. Each is an R1 artifact for the status quo, or a specification ambiguity if the triage rule finds no mandating sentence.
- **H2 (DEC-MOD-040 ambiguity).** At least one of the `plan_ref` / region-flag cases (M1–M4) is accepted as `OK` or disagrees between layouts under the status quo.
  - **Falsified if** all four are refused identically with specific reason codes. That outcome is evidence that the shared sentinel is not ambiguous in practice, since the flag and region coverage disambiguate.
- **H3 (HC-16 compatibility of sentinel candidates).** `explicit-region-flag` rejects or reinterprets 0 frozen v6 archives from EXP-RECON-001/004 tuning (the flag already exists). `distinct-sentinel` and `forbid-unplanned-zero` reject every archive that contains a region (status-quo writers emit `plan_ref = 0` for members).
  - **Falsified by** other counts. A candidate that rejects any frozen archive is admissible only as a new identifier (HC-16, L0).
- **H4 (MVT-02(b)).** Under every fault point (F1–F4), the writer refuses with a stable reason code and writes no archive, and the reader returns `CORRUPT` with no destination object.
  - **Falsified by** any archive written, or any output released. That is an HC-02/HC-15 violation.
- **H5 (MVT-02(e)).** `cargo-mutants` over the reconstruction verification code leaves 0 surviving mutants not shown equivalent by a non-involved reviewer.
  - **Falsified by** any unexplained survivor.

## 4. Decisions informed and how

| Decision | Candidates (ledger) | Contribution |
|---|---|---|
| DEC-MOD-040 | status-quo-shared-zero, distinct-sentinel, explicit-region-flag, forbid-unplanned-zero-in-archives | "Conformance tests of non-region chunks with plan_ref 0 in both layouts" (M1–M4 × layouts × rule readers); "wire compatibility impact on frozen v6 archives" (H3). The R1(b) formal counterexample search is the exhaustive enumeration of M1–M4 field combinations (§6), committed as `adversarial-search/`. |
| DEC-INT-030 | test vectors versus own-decoder | Fault injection proves the write-time and reader verification guarantee is effective (H4). Mutation adequacy of that code (H5). |
| DEC-CON-010 | bit activation | Reader behaviour when bits and sections disagree (M16–M17, D9); the refusal layer for minimal readers |
| DEC-CON-004 | flag-registry-reject-unknown, flags-removed | All-ones `decode_flags` on reconstruction archives (D11): the rejection layer (parse versus enforcement) and outcome class |
| DEC-CMP-016 | limits | Declared-size lies at and beyond the frozen limits refused before allocation (M12–M13, D10), with `refusal_alloc_peak_bytes` |

## 5. Candidates (reader and writer cells)

| Cell | Definition |
|---|---|
| `prod-reader-indexed` | production reader, INDEXED open + verify + unpack of the case archive |
| `prod-reader-stream` | production reader, STREAM path (`open_stream` + unpack) of the case's STREAM twin |
| `rule-distinct-sentinel` | research reader: region members must carry a reserved `plan_ref` sentinel value distinct from `UNPLANNED_PLAN_ID`; otherwise `NONCONFORMING` |
| `rule-explicit-region-flag` | research reader: the region-owned frame flag is the sole authority for membership; `plan_ref` of flagged frames is ignored and must be 0; unflagged frames with `plan_ref = 0` are `NONCONFORMING` |
| `rule-forbid-unplanned-zero` | research reader: `plan_ref = 0` is forbidden in every frame; region members use the region's plan id |
| `fault-deflate-forward`, `fault-jpeg-forward`, `fault-writer-repeat`, `fault-reader-inverse` | production build with `research-fault-injection` enabled at one point: F1 DEFLATE forward recreated output bit flip; F2 JPEG forward recreated output bit flip; F3 ECF writer repeat check input bit flip; F4 reader inverse output bit flip |
| `wirecompat-<rule>` | the three rule readers re-decoding every frozen v6 archive from EXP-RECON-001 (plain cells) and EXP-RECON-004 (`gen-v6` cells) |

## 6. Case table (fixed now; generated deterministically, seed 20261001)

- **Base archives.** Built by production v6 from generated trees (`gen_base_trees.py`, seed 20261001):
  - `base-deflate`: gzip, zlib and raw streams from zlib level 1 over repetitive text. A seed search ensures selection; the searched seeds are recorded.
  - `base-jpeg-1chunk`: a baseline JPEG < balanced minimum chunk, selected in balanced.
  - `base-jpeg-multichunk`: a baseline JPEG of about 3 MiB selected as a multi-chunk region in dense.
  - `base-mixed`: all of the above plus ordinary files and a duplicate.

  Each is built in INDEXED and STREAM, with a digest-consistent mutation mode (the research re-encoder recomputes every digest so the semantic check is reached) and a raw mode (no digest fix-up).

**Region and frame cases.** `NONC` = `NONCONFORMING`; `UNSUP` = `UNSUPPORTED`; `POLICY` = `POLICY_REFUSED`.

| Id | Mutation | Expected class (status quo) |
|---|---|---|
| M1 | unflagged frame, `plan_ref = 0`, not covered by a region | NONC |
| M2 | flagged frame not covered by any region | NONC |
| M3 | covered frame, flag clear, `plan_ref = 0` | NONC |
| M4 | covered frame, flag set, `plan_ref ≠ 0` | NONC |
| M5 | covered frame carrying an independent payload | NONC (or CORRUPT per read order) |
| M6 | overlapping regions | NONC |
| M7 | region member Chunk referenced by a second ContentObject | *specification silent*: outcome recorded; triage decides HC-03 attribution |
| M8 | `start_chunk_index + chunk_count` beyond the object's Chunk count | NONC |
| M9 | region `logical_bytes` ≠ sum of member lengths | NONC or CORRUPT (precedence) |
| M10 | `access.worst_reconstructed_bytes` < region logical bytes (understated) | NONC (HC-09) |
| M11 | `transformed_bytes` ≠ representation length | CORRUPT |
| M12 | declared expansion > 64:1 | POLICY or NONC before decode |
| M13 | representation length field > 64 MiB (bytes absent) | POLICY before allocation (`refusal_alloc_peak_bytes` ≤ R0_max + H) |
| M14 | unknown transform id in TransformStep v3 | UNSUP |
| M15 | JJ01 parameters naming other versions | NONC (invalid parameters) |
| M16 | RECONSTRUCTION_REGIONS section present, bit 0x8 clear | per precedence: NONC |
| M17 | bit 0x8 set, bit 0x4 clear | NONC |
| M18 | representation bytes altered, digests recomputed, JPEG XL parses but reconstructs different bytes | CORRUPT after member digest check; zero plaintext released |

**ReconstructionData cases.**

| Id | Mutation | Expected class |
|---|---|---|
| D1 | `reconstruction_id` ≠ SHA-256(bytes) | CORRUPT (digest mismatch code) |
| D2 | unknown `format` string | UNSUP |
| D3 | wrapper byte = 3 | CORRUPT or NONC |
| D4 | `prefix_length` beyond the record | CORRUPT |
| D5 | truncated corrections, digest recomputed | CORRUPT |
| D6 | `intermediate_len` ≠ actual | CORRUPT (length mismatch code) |
| D7 | max-chain parameter 1024 | NONC (invalid parameters) |
| D8 | corrections altered, digests recomputed | CORRUPT at the chunk digest; no plaintext released |
| D9 | type 14/15 records present, bit 0x4 clear | NONC |
| D10 | ReconstructionData length field > 16 MiB | POLICY before allocation |
| D11 | `decode_flags` = all ones | recorded: parse-layer versus enforcement-layer rejection (DEC-CON-004) |
| D12 | fallback audit record referencing a nonexistent Chunk digest | OK. The EAM and every entry digest are unchanged, and PCI covers the record (HC-01: a non-authoritative record never changes semantics). |

- **Exhaustive sentinel enumeration (R1(b) search for DEC-MOD-040).** For every frame of `base-mixed` (both layouts), every combination of {flag clear or set} × {`plan_ref` ∈ {0, valid plan id, invalid id}} × {covered by region or not}. Outcome classes are recorded for the production reader and the three rule readers.
- **Truncation.** Truncation at every section boundary of region and ReconstructionData sections (MVT-15(b)): `TRUNCATED`.
- **Fault points F1–F4.** Applied to all four base trees in dense and balanced, both layouts where applicable.
- **Expected reason codes.** Where the production registry (`crates/entrybound/src/diagnostics.rs` `ReasonCode`) has a reconstruction-specific code, it is named in `case-table.json`. Otherwise the expectation is "a reconstruction- or region-specific code". A generic code counts against `reason_code_specificity_fraction`.

## 7. Environment and platform requirements

`wsl-ubuntu`, x86-64, no timing. Windows is not required: decoders are platform-independent by HC-11, which EXP-RECON-008 tests. Storage: case archives < 2 GB; `cargo-mutants` target directories about 20 GB (on `/root/eb-research/target`).

## 8. Commands and tooling to build

```sh
C:/Python313/python.exe research/orchestration/disk_guard.py
# generation (to be written)
wsl.exe -d Ubuntu -- /root/eb-research/venv/bin/python /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-RECON-010/gen_base_trees.py --seed 20261001 --out /root/eb-research/generated/EXP-RECON-010/base
wsl.exe -d Ubuntu -- /root/eb-research/tools/EXP-RECON-010/bin/ebr-recon mutate --base-dir /root/eb-research/generated/EXP-RECON-010/base \
   --case-table /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-RECON-010/case-table.json --seed 20261001 \
   --out /root/eb-research/generated/EXP-RECON-010/cases --manifest-out /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-RECON-010/cases-tuning.jsonl
# runner: spec.yaml (reader, rule and fault cells over cases-tuning.jsonl + archives-tuning.jsonl)
# mutation testing (MVT-02(e)): cargo-mutants pinned; files: crates/entrybound/src/reconstruction.rs, crates/entrybound/src/jpeg_reconstruction.rs,
#   and the writer verification call sites of verified_forward in crates/entrybound/src/ecf (research copy of the workspace, never the repo tree):
wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-RECON-010/run_mutants.sh --cargo-mutants-version pinned \
   --copy-to /root/eb-research/src/EXP-RECON-010-mutants --out /root/eb-research/results/EXP-RECON-010/mutants
# analysis (to be written): research/tools/recon/conformance_matrix.py
```

The mutation run operates on a copy of the workspace, so no production file in the repository is modified.

## 9. Seeds, warmups, replication

- Seed 20261001 for generation, mutation and order.
- Warmups: 0.
- Repetitions: 2 per (cell, case). Outcomes are deterministic; any difference is an HC-03 finding.
- `cargo-mutants`: one full run. Surviving mutants are re-run once to exclude flakiness.

## 10. Metrics

| Metric (canonical) | Role | Threshold |
|---|---|---|
| `adversarial_true_refusal_fraction` | binary (= 1 over all hostile cases: expected refusal class, zero output bytes) | §3.3 |
| `reader_disagreements` | binary (INDEXED versus STREAM for the production reader; production versus rule readers is reported, not binary) | §3.3 |
| `reason_code_specificity_fraction` | graded (T-20; n_cases = case count) | T-20 |
| `refusal_alloc_peak_bytes`, `refusal_bytes_read`, `refusal_cpu_s` | secondary (refusal cost, M13, D10) | T-06, T-02, T-05 |
| fault-injection pass (no archive written, no plaintext released, stable code) | binary (MVT-02(b)) | §3.3 |
| surviving unexplained mutants | binary (MVT-02(e)) | §3.3 |
| frozen archives rejected or reinterpreted per rule | descriptive count feeding the HC-16 screen | — |

## 11. Analysis

1. **Confusion matrix** (expected class × observed class) per cell and layout. Off-diagonal cells are listed with reproducers under `research/raw/EXP-RECON-010/violations/`.
2. **Triage** (objective §2.0 rule 8). For M7, precedence ambiguities and any layout disagreement, a non-involved session looks for a mandating sentence in `docs/format-v0.md`, `docs/jpeg-reconstruction-v1.md` or `docs/reconstructive-transform-v1.md`. If none exists, the case is an HC-03 violation of the *specification*, recorded against DEC-MOD-040 or the relevant decision.
3. **DEC-MOD-040.**
   - Exhaustive enumeration results for each rule. Any status-quo accepted-but-ambiguous combination is a written R1(b) counterexample for `status-quo-shared-zero`, which needs reviewer sign-off.
   - Wire-compatibility counts (H3): a rule that rejects or reinterprets any frozen archive is L0-excluded as an in-place change and survives only as a new identifier, with its computed tier from EXP-RECON-011.
   - Remaining candidates are compared on C1 cost (EXP-RECON-011) and case-table outcomes. R10 applies.
4. **Fault injection (H4)** and **mutation adequacy (H5).** Binary screens for the status-quo verification guarantee used by DEC-INT-030 and every reconstruction decision.
5. **Case-table export.** The case table plus observed production outcomes are packaged as `research/experiments/EXP-RECON-010/conformance-proposal/` for the conformance domain (sections 28–29). The independent reader's agreement is measured there.

## 12. Practical-significance thresholds

T-20 for `reason_code_specificity_fraction`; T-06, T-02 and T-05 for refusal costs; §3.3 for binaries.

## 13. Sensitivity analysis

- Digest-consistent versus raw mutation modes are reported separately. Raw mode tests digest-first precedence; consistent mode tests semantic checks.
- Specificity is reported with and without M7 (specification-silent) cases.

## 14. Expected negative results worth recording

- Understated region access declarations (M10) are accepted, making HC-09 unenforced at read time.
- The shared-zero sentinel is accepted in an ambiguous combination (H2): the specification must change under a new identifier.
- Generic reason codes for reconstruction failures lower `reason_code_specificity_fraction`.
- Surviving mutants in write-time verification (H5): the tests do not pin the guarantee.
- M7 has no normative answer, a specification gap.

## 15. Threats to validity

- **The research re-encoder could itself construct invalid framing** unintentionally. Mitigation: each case's unmutated twin must open `OK` (a control per case).
- **The case table reflects the designer's reading of the docs.** Expected classes are checked by a reviewer against the MVT-15(b) precedence table before any run. Disagreements are resolved before results.
- **The fault-injection feature modifies production code under a default-off feature.** MVT-16(c) identity must pass.
- **`cargo-mutants` coverage** depends on the chosen test command. The command is fixed as: the production test suite plus the EXP-RECON-010 research test.
- All other threats: common §13.

## 16. Held-out placeholder

Not applicable to generated cases. After Commit C, wire-compatibility re-decode runs on held-out frozen archives (from the EXP-RECON-001/004 held-out runs) for any sentinel rule adopted as a new identifier.

## 17. Cost and effort estimates

- **Compute.** About 70 CPU-hours: case runs about 5 CPU-hours; wire-compatibility re-decode of about 700 tuning archives × 3 rules × 2 repetitions about 15; `cargo-mutants` about 50.
- **Disk.** About 25 GB (mutants build trees, cases).
- **Tooling.** `ebr-recon mutate`, fault-injection feature (Internals agent), rule readers, generators, `run_mutants.sh`: about two sessions.
- **Agent effort.** Execution: none. Case-table review and triage: judgment (independent). Analysis: judgment.

## 18. Looks, deviations and outcome (append-only)

- Looks: none.
- Deviations: none.
- Outcome: `outcome.json`.
