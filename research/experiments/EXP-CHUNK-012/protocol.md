# EXP-CHUNK-012: Chunker conformance: vectors, clean-room reimplementation, streaming and cross-host identity, chunker_id grammar, PCR binding

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CHUNK-012 |
| Status | NEEDS_TOOLING. Needed: `ebr-chunk stream-identity`, `ebr-chunk vectors`, `ebr-chunk chunker-id-parse`, an `ebr-cdcpack --chunker-id-override` research writer, a cargo-fuzz target, the Windows build of `ebr-chunk`, `pcr_census.py`, and **an independent clean-room session**. This session has read `crates/entrybound/src/chunker.rs` (parameters and selection code) and therefore must not write the clean-room reader. |
| Arms | **S** streaming identity (`spec.yaml`). **X** cross-host boundary identity, Windows host vs WSL (`spec-cross-host-windows.yaml`, generated). **V** conformance vectors (committed files). **C** clean-room reimplementation differential. **G** chunker_id grammar: reader behaviour and fuzzing. **P** PCR divergence census and PCR-binding counterexample search. Cross-ISA identity is EXP-CHUNK-013 (BLOCKED). |
| decision_ids | DEC-CMP-005 (grammar, vectors, accelerated-path differential), DEC-CMP-001 (independent implementability and scalar reference of successor candidates), DEC-MOD-039 (PCR divergence frequency across profiles, rekey, categorisers; binding alternatives), DEC-ACC-029 (streaming CDC boundary identity against the in-memory reference), DEC-CRY-023 (keyed vectors reproducible from docs; with the crypto domain, DEC-CRY-051) |
| Proposed decision types | DEC-CMP-005: FORMAL + conformance EMPIRICAL. DEC-MOD-039: FORMAL (binding soundness) + EMPIRICAL (divergence frequency). DEC-CMP-001: its OD-21 part. |
| Proposed od_ids | OD-04, OD-13 (determinism across splits and hosts), OD-21, OD-23 |
| Proposed hc_ids | HC-03, HC-08 (host independence of PCR), HC-10, HC-12, HC-13, HC-16 |
| External review | HC-12 final assurance needs an organizationally external implementer or reviewer (objective HC-12 status). The internal clean-room session is necessary evidence, not sufficient. |
| Gates / author / L7 | As EXP-CHUNK-001 section 1 |

## 2. Question

Are the chunk boundaries of the status-quo and candidate chunkers a pure function of plaintext and declared parameters? That is, are they independent of:

- how the input is streamed (buffer splits);
- the host (Windows vs WSL);
- the implementation (a clean-room reader built from the written specification)?

And, for chunker_id grammar validation and PCR-binding alternatives: what do they cost, and what do they break?

## 3. Hypotheses

- **H-S (binary, HC-13).** An incremental chunker fed 64 seeded random buffer splittings (1 B to 64 MiB pieces) produces boundaries equal to whole-buffer chunking on every item. The same holds in a 20-repetition stress cell. **Violation:** any mismatch. The artifact is the item, split sequence and both boundary lists. PHTE's rolling window continuity across cuts (`docs/crypto-suite-v1.md`) is the highest-risk case.
- **H-X (binary, HC-08).** `boundary_set_sha256` from the Windows-host build equals the WSL build for every tuning item ≤ 256 MiB, candidate and preset. **Violation:** any difference, triaged under objective §2.0 rule 8.
- **H-C (binary, HC-12).** A clean-room Python implementation of `gear-norm-v1`, `gear-norm-secret-table-v1` and `phte-aes128-norm-v1`, written from `docs/chunking-v1.md` and `docs/crypto-suite-v1.md` alone, reproduces 100% of the committed vectors. It also reproduces production boundaries on all tuning regular files ≤ 64 MiB.
  - `cleanroom_pass_fraction` = 1 is required.
  - Every failure traced to missing or ambiguous text is a specification violation until the text is fixed.
  - Each successor construction in DEC-CMP-001's validation survivor set S gets the same test from its written specification.
- **H-G.** Production `ebound verify` and `inspect` accept archives whose chunker_id is any non-empty string (status quo, including malformed grammar), with outcome `OK`. Under the `parsed-grammar-validated` model the same archives would be `NONCONFORMING`, which would change the decided outcome class of existing valid archives produced by replan (EXP-CHUNK-011).
  - **Falsified if** production rejects malformed ids.
  - The fuzz target on the candidate parser reaches the coverage target with 0 unresolved findings.
- **H-P1 (divergence frequency).** For identical content, PCR differs across the four profiles on ≥ 50% of tuning items (frozen presets differ). It differs across 8 rekeys of encrypted archives on 100% of items (keyed boundaries). **Falsified if** below those rates.
- **H-P2 (binding soundness, FORMAL).** Under `pcr-excludes-chunker-id`, two archives with identical chunk roots and counts but different declared chunker parameters produce equal PCR. The counterexample search finds such a pair on the committed vectors, making the parameter provenance unbound. Whether that breaks a verified-range proof is a FORMAL question for the independent reviewer. The search artifact is committed either way.
- **H0.** All binary checks pass; divergence rates are low.

## 4. Decisions informed

| Decision | Ledger required evidence covered |
|---|---|
| DEC-CMP-005 | "Independent reimplementation reproduces boundaries from docs/chunking-v1.md and docs/crypto-suite-v1.md alone" (C); "Fuzzing of chunker_id parsing and verify reporting" (G); "Cross-ISA differential results for any accelerated chunker" (x86-64 Windows vs WSL here; ARM64 in EXP-CHUNK-013); conformance vectors (V). The producer audit is EXP-CHUNK-011. |
| DEC-CMP-001 | Independent implementability (OD-21) of W and S; scalar-reference identity; streaming identity of candidates |
| DEC-MOD-039 | "Frequency of PCR divergence for identical content across profiles, categorisers and rekey" (P1); "Verified-range soundness when chunker id is absent" (P2, FORMAL, reviewer sign-off) |
| DEC-ACC-029 | "Byte-identical output of streaming designs vs in-memory reference": the chunker component (S). Whole-pack streaming designs belong to the scale domain. |
| DEC-CRY-023 | Keyed-mode vectors and clean-room reproduction (V, C); with the crypto domain's DEC-CRY-051 |

## 5. Candidates

- **Arms S, X, V and C (implementations under test):**
  - `gear-norm-v1` at the four frozen presets (status quo);
  - `gear-norm-secret-table-v1` and `phte-aes128-norm-v1` (encrypted status quo);
  - `gear-spread-nc2` and `fastcdc-2016-nc2` as the spec's provisional successor slots. They are replaced or augmented by amendment with DEC-CMP-001's tuning S once EXP-CHUNK-004 arm A has run.
- **Arm G, DEC-CMP-005 candidates:**

| Candidate | Modelled behaviour |
|---|---|
| `status-quo-free-form-nonempty` (**reference**) | Production |
| `parsed-grammar-validated` | Malformed id → `NONCONFORMING` with a stable code |
| `registry-unknown-opaque` | Known ids parsed and validated; unknown ids opaque |
| `conformance-vectors-scalar-reference` | Arm V |
| `accelerated-path-differential` | Arm X and EXP-CHUNK-013 |

- **Arm P, DEC-MOD-039 candidates:**
  - `single-chunker-id-status-quo` (reference);
  - `per-object-chunker-id`;
  - `chunker-set-in-descriptor`;
  - `uniform-chunking-across-profiles` (cost in EXP-CHUNK-005 `gran-global-single`);
  - `pcr-excludes-chunker-id` (arm P2 counterexample search).
- R0 item 6: critic candidates open.

## 6. Corpus

- **Arm S:** tuning items ≤ 256 MiB, 84 items, 20 families, 5.6 GiB (ids in `spec.yaml`). All regular files of each item are checked.
- **Arm X:** the same items, read from Windows through `\\wsl.localhost\Ubuntu\root\eb-research\corpus\tuning\…`. The harness held-out guard refuses UNC paths only into held-out roots (R1-01). Tuning content bytes are unchanged by the access path.
- **Arm C:** committed vectors plus tuning regular files ≤ 64 MiB.
- **Arm G:** ids observed in EXP-CHUNK-011, 100,000 seeded mutations (seed 20260917; mutation operators: character substitution, truncation, numeric overflow, reordering, separator duplication, Unicode confusables, empty components), and 200 research archives with overridden chunker_id built from 10 small tuning items.
- **Arm P1:** tuning items ≤ 256 MiB. The PCR under 4 profiles comes from EXP-CHUNK-004 arm SQ; the encrypted PCR under 8 fresh encryptions of each profile comes from the harness; the categoriser PCR comes from EXP-CHUNK-005 `gran-*` archives.
- **Validation:** arms S, X and C are re-run on validation items ≤ 256 MiB as HC screens (look registry, non-confirmatory).
- **Held-out:** HC re-runs after unlock (Phase D placeholder, §5.5).

## 7. Environment and platform

| Arm | Platform |
|---|---|
| S, V, G, P | WSL x86-64 |
| X | Windows 11 host (non-admin, NTFS, `D:/eb-research/target/exp-chunk-win`) plus WSL |
| C | Any; the clean-room session runs in a separate checkout that excludes `crates/` and `tools/` (objective MVT-12(a)), Python 3.12 in WSL with no third-party packages except a pinned AES implementation (`cryptography` from the research venv lock, version recorded), with a provenance log of every consulted file |
| G fuzzing | WSL nightly-2026-08-01 with cargo-fuzz 0.13.2 (PROGRESS environment) |

No network. External review is required for final HC-12 assurance (section 1).

## 8. Commands

```sh
# Arm S
$PY -m ebr run --spec research/experiments/EXP-CHUNK-012/spec.yaml --gzip   # + verify-raw, normalize
# Arm V (commit outputs)
/root/eb-research/target/exp-chunk/release/ebr-chunk vectors --out research/experiments/EXP-CHUNK-012/vectors \
    --algorithms gear-norm-v1,gear-norm-secret-table-v1,phte-aes128-norm-v1,gear-spread-nc2,fastcdc-2016-nc2 \
    --presets frozen --synthetic-keys 4 --key-seed-base 20260917 \
    --cases empty,one-byte,min-1,min,min+1,target-1,target,target+1,max-1,max,max+1,all-zero,periodic-2,periodic-64,periodic-4096,random-8m,gear-extremes
# Arm X (Windows host, PowerShell)
#   research/harness/build.ps1 build --release -p ebr-chunk ; then $PY make_specs.py --arm cross-host-windows ; ebr run on Windows
# Arm C: independent session; entry script to be written by that session:
#   research/independent-reader/chunker-cleanroom/cleanroom_chunker.py  (+ PROVENANCE.md)
$PY research/experiments/EXP-CHUNK-012/cleanroom_differential.py --vectors research/experiments/EXP-CHUNK-012/vectors \
    --cleanroom research/independent-reader/chunker-cleanroom/cleanroom_chunker.py --items <tuning <= 64 MiB> \
    --out research/normalized/EXP-CHUNK-012/decision/cleanroom.csv
# Arm G
/root/eb-research/target/exp-chunk/release/ebr-chunk chunker-id-parse --mutations 100000 --seed 20260917 \
    --observed research/normalized/EXP-CHUNK-011 --out /root/eb-research/results/EXP-CHUNK-012/grammar.jsonl.gz
$PY research/experiments/EXP-CHUNK-012/override_archives.py --count 200 --ebr-cdcpack /root/eb-research/target/exp-chunk/release/ebr-cdcpack \
    --ebound /root/eb-research/target/exp-chunk-prod/release/ebound --out research/normalized/EXP-CHUNK-012/decision/reader-behaviour.csv
cd research/harness && cargo +nightly-2026-08-01 fuzz run fuzz_chunker_id_parse -- -max_total_time=14400
# Arm P
$PY research/experiments/EXP-CHUNK-012/pcr_census.py --sq research/normalized/EXP-CHUNK-004 \
    --gran research/normalized/EXP-CHUNK-005 --encrypted-reps 8 --out research/normalized/EXP-CHUNK-012/decision/pcr-census.csv
$PY research/experiments/EXP-CHUNK-012/pcr_binding_counterexample.py --vectors research/experiments/EXP-CHUNK-012/vectors \
    --out research/experiments/EXP-CHUNK-012/adversarial-search/pcr-binding/
```

## 9. Seeds

- **Spec seeds:** `{order: 2026091821, bootstrap: 2026091822, command: 2026091823}`.
- **Split seed, mutation seed, key seed base:** 20260917 each.
- **Stress:** 20 repetitions with split seeds 20260917+k.
- Warmups 0.

## 10. Metrics

| Metric | Canonical | Role |
|---|---|---|
| `stream_split_mismatches`, `stress_mismatches` (S) | HC-13 binary (`migration_determinism_fraction` is not applicable; reported as the objective MVT-13(a)-style repeat-hash outcome) | binary |
| Windows/WSL `boundary_set_sha256` agreement (X) | `cross_host_identity_agreement` | binary |
| `cleanroom_pass_fraction` (C) | yes | binary (= 1) |
| `spec_defects_per_1000_words` (C ambiguity log) | yes | descriptive |
| `reader_disagreements` between modelled candidate readers and production on override archives (G) | yes | binary for each candidate model's claimed behaviour |
| `reason_code_specificity_fraction` for malformed-id refusals under candidate models (G) | yes | banded T-20 (secondary here) |
| `unresolved_fuzz_findings_at_coverage_target` (G) | yes | binary |
| `independent_reader_loc` (C, per construction) | yes | cost_input (C2/C5) |
| PCR distinct-per-LAI counts, profile/rekey/categoriser divergence fractions (P1) | no | secondary (descriptive for the FORMAL decision) |
| Counterexample found / not found with search budget (P2) | no | FORMAL artifact (§1.2) |

## 11. Replication

- **Arm S:** 2 repetitions plus the stress cell.
- **Arm X:** 2 repetitions per host.
- **Arm C:** deterministic, run once plus a rerun by the reviewer.
- **Arm G:** mutation corpus deterministic; fuzzing is time-boxed at 4 CPU-hours and regression evidence (no detection bound).
- **Arm P:** deterministic from source archives. Encrypted PCR uses 8 fresh encryptions per item and profile.

## 12. Analysis

- **Binary arms:** a pass/fail table per candidate and item. Any violation is committed with its artifact (objective §2.0 rule 2).
- **HC failures** of the status quo are production defects (objective §2.0 rule 5; ledger work items). HC failures of candidates are R1 eliminations.
- **Arm C:** `cleanroom_pass_fraction` per construction. Every ambiguity-log entry is classified (missing text, ambiguous text, reader bug) by a session involved in neither reader (rule 8). The reader's LoC is recorded as C2/C5 cost input.
- **Arm G:** a reader-behaviour matrix (candidate model × mutation class → outcome class and code). Any valid archive that a candidate model would newly reject is listed; this is the compatibility cost of the grammar candidate (HC-16: decode support for published identifiers persists).
- **Arm P1:** per item, distinct PCR count by source of divergence (profile, rekey, categoriser, producer from EXP-CHUNK-011), by family (descriptive).
- **Arm P2:** the counterexample-search protocol (generator, budget, result) is committed under `adversarial-search/`, then checked and signed off by a reviewer who did not produce it (§1.2 FORMAL).

## 13. Thresholds

- Binary tests have no band (§3.3).
- T-20 for `reason_code_specificity_fraction`, as transcribed in `research/methods/thresholds.json` (secondary here).

## 14. Sensitivity

- **Split distribution:** log-uniform vs uniform split sizes.
- **Validation-split replication** of arms S, X and C.
- **Arm C second reader** (optional; requires an external implementer, not available internally).

## 15. Expected negative results worth recording

- Clean-room ambiguities in the normalization mask description ("low b+1 bits"), in tail handling or in PHTE rejection sampling. These are specification defects, not reader bugs.
- Production accepts malformed chunker_id silently.
- PCR diverges across profiles on most items, making "PCR identity" profile-dependent for users.
- The binding counterexample exists for `pcr-excludes-chunker-id`.

## 16. Threats to validity

- The internal clean-room session is not organizationally independent, so assurance stays EXTERNAL_REVIEW_REQUIRED.
- Windows access through `\\wsl.localhost` could in principle alter bytes. Mitigation: arm X also compares the file SHA-256 read on both hosts.
- The mutation corpus is not exhaustive (regression evidence).
- The PCR census depends on EXP-CHUNK-004/005 archives existing.
- Successor candidates' written specifications are drafted by harness implementers, who may leak implementation detail into the text. The clean-room reader flags any text that reads as code transcription.

## 17. Estimates

| Arm | Compute |
|---|---|
| S | 8 candidates × 84 items (5.6 GiB) × (1 + 64 split passes + 20 stress passes) × 2 reps ≈ 8 × 5.6 × 85 × 2 ≈ 7.6 TB of chunking: about **8-40 CPU-hours** (PHTE dominates). **Reduction pre-registered:** splits apply to the first 64 MiB of each file, so the total is closer to 2 TB, about **3-12 CPU-hours**. |
| X | about 2 × arm S whole-buffer passes on Windows: about 3 CPU-hours |
| C | agent judgment, about 1 session; differential about 2 CPU-hours |
| G | about 1 CPU-hour plus 4 CPU-hours fuzzing |
| P | minutes, plus 8 encryptions × 4 profiles × items ≤ 256 MiB: about **30 CPU-hours** |

- **Disk:** vectors < 20 MB committed; results about 2 GB.
- **Agent effort:** judgment for arm C (independent session), the arm P2 counterexample search and review, and the arm G matrix interpretation. Scripted otherwise.

## 18. Tooling to build

1. **`ebr-chunk stream-identity`:** an incremental chunker wrapper per algorithm, each module exposing a streaming API (state carried across buffers; PHTE window continuity), with split sequences, a stress cell and a violation artifact directory. **Pre-registered reduction:** splitting is applied to the first 64 MiB of each file.
2. **`ebr-chunk vectors`:** vector cases listed in section 8; the Gear-table SHA-256; keyed vectors with synthetic keys; JSON plus binary inputs as generator specs (no large blobs).
3. **`ebr-chunk chunker-id-parse`:** candidate grammar parser, mutation corpus generator, and cargo-fuzz target `fuzz_chunker_id_parse` under `research/harness/fuzz/`.
4. **`ebr-cdcpack --chunker-id-override <string>`** (research writer only) and `override_archives.py`.
5. **Windows build and `make_specs.py --arm cross-host-windows`.**
6. **`cleanroom_differential.py`, `pcr_census.py`, `pcr_binding_counterexample.py`.**
7. **Clean-room reader** `research/independent-reader/chunker-cleanroom/` (independent session; coordinate with the conformance domain's independent reader, program §28).

## 19. Deviations

(append-only)
