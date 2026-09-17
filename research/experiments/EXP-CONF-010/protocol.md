# EXP-CONF-010 — Differential and property fuzzing across production reader paths, codec backends and the independent reader

| Field | Value |
|---|---|
| Status | `NEEDS_TOOLING` (CC§2): differential fuzz targets DF1-DF9, structure-aware EAM generator, `ebir` persistent batch mode, `ruzstd` and `xz2`/`lz4` comparison shims, key/nonce logging hook in `ebound-research` (F-24, MVT-16(c)) |
| Domain / program sections | conformance / §30 (primary), §28 |
| Decision IDs informed | DEC-ECO-029, DEC-ECO-032, DEC-CON-014, DEC-CON-018, DEC-CON-025, DEC-CRY-054, DEC-CRY-065, DEC-CMP-007, DEC-ECO-041, DEC-INT-019, DEC-INT-040, DEC-MOD-006 |
| Requirements | REQ-ECO-0037, REQ-ECO-0134 |
| HC oracles | HC-03 MVT-03(a), MVT-03(b); HC-01 MVT-01(b) (reinterpretation); F-07 layout equivalence; HC-14 MVT-14(c)-style uniqueness under hostile mutation inputs; proposal for the assigner: `hc_ids` HC-01, HC-03, HC-14, HC-15; `od_ids` OD-01, OD-04, OD-24 |
| Decision type | not assigned (gate G-A) |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1 |
| Depends on | EXP-CONF-001 (`ebir` v1/v2), EXP-CONF-004 (seeds), EXP-CONF-008 (build pipeline) |

## 1. Question

Under coverage-guided generation and mutation, do readers that claim identical semantics — INDEXED versus STREAM, full
versus range encrypted readers, in-memory versus file versus HTTP-range sources, random versus full readers, libzstd
versus a pure-Rust Zstandard decoder, and production versus the clean-room reader — ever disagree; does any accepted
input fail to re-encode canonically; can a mutation be accepted with a different interpretation and no integrity
failure; and do mutation operations on hostile encrypted inputs ever repeat a (key, nonce) pair?

## 2. Hypotheses and falsification

Each oracle below is a binary hypothesis; one committed counterexample falsifies it.

| Id | Pair or property | Oracle (violation) |
|---|---|---|
| DF1 | INDEXED versus STREAM of the same EAM, from a structure-aware generator of EAM trees packed in deterministic mode | any difference in entries, ContentObjects, Chunks, declared bounds, LAI, PCR or AUX (F-07) |
| DF2 | encrypted full reader versus encrypted range reader on the same (mutated) bytes with the public test secret | for the same requested scope, one accepts and the other refuses, or both accept with different EAM or entry bytes (DEC-CRY-065, DEC-CON-025) |
| DF3 | Memory versus LocalFile versus HttpRange (in-process `ebr-origin`) sources on the same (mutated) INDEXED bytes | any comparison-key difference (CC§5) |
| DF4 | random-access reader versus full reader | an entry both return with different bytes, or a random-access result claiming `whole_archive_verified` |
| DF5 | Zstandard frames decoded by production (libzstd via `zstd-sys`) versus `ruzstd` (pinned) under each `zstandard/v1` parameter record (plain, dictionary, raw-content prefix) and the declared window | accept/reject difference or output difference not attributable to a documented limit of `ruzstd` (DEC-CMP-007, DEC-ECO-041); LZMA2 (`lzma-rust2` versus `xz2`/liblzma raw LZMA2) and LZ4 block (`lz4_flex` versus `lz4` C binding) in the same way |
| DF6 | canonical re-encode property on every input any production path accepts as `OK` | re-encoding with the same identifiers is not byte-identical (MVT-03(a); DEC-MOD-006, DEC-CON-014) |
| DF7 | undetected reinterpretation: mutations of a valid unencrypted INDEXED archive | a mutant accepted as `OK` whose EAM or identities differ from the original's (DEC-INT-019); reported with the mutated region |
| DF8 | production (`ebcc-readtuple`, in process) versus `ebir` (persistent `ebir batch` subprocess) on libFuzzer-mutated archives | comparison-key disagreement not attributed to `ebir` by triage (CC§6) |
| DF9 | hostile encrypted inputs (fuzzed, then made valid under the test password by the harness) followed by recipient add, recipient remove, password rotation and signature embedding through the library | a repeated (derived key identifier, nonce) pair in the `ebound-research` log (DEC-CRY-054) |

## 3. Candidates

The paired implementations above are the objects compared; design candidates of the informed decisions are the
ledger's (CC§1: none added). Relevant mappings: DEC-ECO-029 `internal-differential-fuzzing`, `property-based-tests`,
`structure-aware-arbitrary-generators`; DEC-CON-025 `differential-conformance-gate`; DEC-CRY-065 `differential-fuzzing-gate`;
DEC-CMP-007 `pure-rust-default-c-optional` and `pure-rust-decode-c-encode` (evidence: DF5); DEC-ECO-041
`pure-rust-zstd-decoder-for-readers` (DF5). Not executed here: `incumbent-differential-fuzzing` (EXP-CONF-011).

## 4. Inputs

Seed corpora of EXP-CONF-008 §6 for the corresponding boundaries; for DF5, frames extracted from `ebound-head` packs of
the 14 small tuning items (EXP-CONF-002 §5.1) plus `ebcc-v1` F-CODEC cases; for DF9, the encrypted valid cases of
`ebcc-v1` F-CRYPTO. Tuning and generated inputs only (CC§7).

## 5. Environment, budgets and commands

WSL2 Ubuntu 24.04 x86-64; `nightly-2026-08-01`, cargo-fuzz 0.13.2, sanitizer address; fork mode with 4 workers.
Budgets (pre-registered, one libFuzzer seed `2809171001`): DF1-DF7 and DF9 24 CPU-hours each; DF8 72 CPU-hours (the
reference-versus-independent campaign REQ-ECO-0134 names). `ebir` runs as a persistent subprocess with length-prefixed
batch input; its CPU time is included in the budget. Never concurrent with timing windows (CC§3).

```sh
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CONF-010/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-010/spec.yaml
python3 research/harness/fuzz/tools/diff_triage_prep.py --exp EXP-CONF-010   # clusters for CC6 triage
python3 research/harness/fuzz/tools/promote.py --exp EXP-CONF-010 --ledger research/conformance/regressions.jsonl --family F-DIFF
```

Every disagreement input is minimized (`cargo fuzz tmin` with the differential oracle as the crash condition, ≤ 10 wall
minutes), reproduced 3 times, clustered (CC§6), triaged, and promoted to the regression ledger and `ebcc-v1.1` family
`F-DIFF` with a triaged expectation.

## 6. Replication

One persistent campaign per oracle (seed variability is measured in EXP-CONF-008). Each counterexample is reproduced 3
times in non-fork processes on the same builds; a nonreproducible disagreement is recorded, kept and flagged as a
possible nondeterminism finding (objective §2.0 rule 2 applies when two runs of one reader differ on one input).

## 7. Metrics (CC§9)

| Metric | Role | Unit | Oracles |
|---|---|---|---|
| `reader_disagreements` | binary (HC-03) | count | DF2, DF3, DF4, DF8 (distinct clusters not attributed to the other reader) |
| `equivalence_assertion_pass_fraction` | binary | fraction | DF1 (passing generated EAMs ÷ generated EAMs) |
| MVT-03(a) canonical re-encode violations; MVT-01(b)-style reinterpretations; repeated (key id, nonce) pairs | binary (HC oracles) | count | DF6; DF7; DF9 |
| Codec backend disagreements by attribution (production library, comparison library, documented limit) | descriptive, non-canonical | count | DF5 |
| `unresolved_fuzz_findings_at_coverage_target` contributions (crashes found incidentally) | binary | count | all; handed to EXP-CONF-009's ledger |

## 8. Analysis

Counts per oracle, cluster and attribution (AGG-W). DF5 disagreements attributed to the comparison library are reported
as that library's defects and are not evidence against production; disagreements attributed to production or to an
undocumented limit are evidence for DEC-CMP-007 and DEC-ECO-041. DF7 results separate mutations inside digest coverage
(expected `CORRUPT`) from mutations of uncovered structure (evidence for DEC-INT-019 candidates). The analysis plan needs
adoption by a non-exposed session (CC§1).

## 9. Practical-significance thresholds

§3.3 for every binary oracle; no band for descriptive counts.

## 10. Sensitivity analysis

- DF8 against `ebir` v1 and v2 (errata effect).
- DF2 and DF3 with and without HTTP transport faults injected by `ebr-origin` (short 206, ETag change), reported
  separately so that remote-fault handling differences are not mixed with parsing differences.
- DF5 with the declared window enforced by the harness versus by each library.

## 11. Split discipline and held-out placeholder

Tuning and generated seeds only; no validation or held-out input. The regression replay gate (EXP-CONF-009 §4 step 7)
covers `F-DIFF` inputs at Commit A.

## 12. Expected negative results worth recording

- Full versus range encrypted reader disagreements on object order or per-stanza limits (DEC-CRY-065 status quo).
- `ruzstd` rejecting or mis-decoding raw-content-prefix frames that libzstd accepts (or the reverse).
- Accepted non-minimal TLV encodings failing canonical re-encode.
- Structure outside digest coverage (for example advisory footer fields) changing interpretation without `CORRUPT`.

## 13. Threats to validity

- Differential oracles detect disagreement, not correctness: both readers may share a wrong reading (CC§11).
- The DF9 hook is instrumentation (F-24); its identity proof covers archive bytes and outcomes, not timing.
- `ebir` subprocess overhead reduces DF8 executions per CPU-hour relative to in-process targets.
- One seed per oracle; absence of disagreements is regression evidence only.

## 14. Cost estimates

| Item | Estimate |
|---|---|
| Compute | 330 CPU-hours: 8 oracles × 24 h + DF8 72 h = 264 h; minimization, reproduction and builds ≈ 66 h. ≈ 11 wall hours on 32 vCPUs |
| Disk | 30 GB |
| Agent effort | **scripted** campaigns; **judgment, medium**: triage of disagreement clusters |
| Platform | Linux x86-64 (WSL); CPU-saturating; no timing; no network after pinned crate downloads |

## 15. Outcome obligations

`outcome.json` (§10.1); `F-DIFF` cases and regression entries; each production-attributed disagreement is a defect work
item and `counterevidence` for the decisions in the header.
