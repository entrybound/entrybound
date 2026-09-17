# EXP-ECO-020: Library API shape prototypes — integration-task complexity, type-level verification guarantees and semver cost

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-020 |
| Domain | ecosystem (program §33; cross-cuts §34 stable-v1 API surface, §13 concurrency, §24 verification state) |
| Status | NEEDS_TOOLING: compile-checked API sketch crates per candidate, integration examples, `trybuild` compile-fail suites; the simulated integrator arm reuses EXP-ECO-019's harness |
| Runner | script-driven (`research/tools/ecosystem/api/`); compile and census steps are deterministic |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |
| Timing | none (throughput of concurrency models is the scale domain's) |

## 1. Question

For the candidate shapes of the v1 library API (free functions, the SPEC four-object shape, plan-based, sans-io core with sync and async shells, layered low/high level), their verification-state representations, progress and cancellation contracts, legacy read adapters and random-access policy presets: how much public surface does each expose, how much integration code and how many API concepts do pre-registered integration tasks need, which guarantees are enforced by the type system (verified bytes cannot reach verified-only sinks; `Send`/`Sync` where promised), what dependencies each adds, and how many semver-breaking changes each implies relative to the status-quo public API?

## 2. Hypotheses and falsification

- **H1 (type-level verification).** Only candidates that wrap verified data in a distinct type (`typed-verified-wrapper`, `separate-verified-and-raw-apis`, `combined-type-and-qualifier`) make the compile-fail test "pass unverified bytes to a verified-only sink" fail to compile; the status quo and `plain-ok-no-scope` compile it. *Falsified* if a wrapper candidate compiles the misuse or a non-wrapper candidate rejects it.
- **H2 (sans-io cost).** The sans-io core with a sync shell needs more integration LoC for the synchronous tasks IT01-IT04 than the status quo, and fewer for the async server task IT06 than any sync-only candidate. *Falsified* by the opposite on either comparison.
- **H3 (async dependency cost).** Candidates with a v1 async shell add an async runtime to the library's normal dependency closure unless the shell is runtime-agnostic. *Falsified* if every async-shell candidate is runtime-agnostic by construction.
- **H4 (semver break).** The SPEC four-object shape implies more breaking changes (`cargo-semver-checks`) relative to the status-quo public API than the layered low/high-level candidate. *Falsified* by the opposite.

## 3. Decisions informed

DEC-MOD-033, DEC-ACC-037, DEC-ACC-050, DEC-ACC-007 (API ergonomics and dependency parts only), DEC-LEG-071, DEC-INT-042 (compile-fail evidence), DEC-ACC-039 (presets versus raw limits, API form).

## 4. Candidates

| Decision | Candidates prototyped as sketches | Not prototyped (reason) |
|---|---|---|
| DEC-MOD-033 | status-quo-free-functions (the real public API at dev HEAD, no sketch); spec-four-object-shape; r1-plan-based; sans-io-core-with-shells; layered-low-and-high-level | cli-only-v1 (no library: integration tasks counted as subprocess scripts via EXP-ECO-018 rules) |
| DEC-ACC-050 | status-quo-sync-io-traits; sans-io-state-machine; async-shell-v1; trait-based-async-source | async-shell-post-v1 (same sketch as async-shell-v1; release timing is not measurable) |
| DEC-ACC-037 | status-quo-none; callback-byte-entry-progress; cancellation-token; transactional-cancel; reported-partial-state; declared-totals-only | — (kill and cancel injection behaviour is the scale and integrity domains') |
| DEC-ACC-007 | status-quo-single-threaded-sync; internal-thread-pools; async-runtime; sans-io-core-sync-async-shells; caller-provided-executor | — (throughput and determinism are the scale domain's) |
| DEC-LEG-071 | status-quo-none; import-backed-handles; typed-untrusted-streaming | defer-post-v1 |
| DEC-INT-042 | status-quo-report-structs; typed-verified-wrapper; diagnostic-qualifier-field; per-check-tristate-report; combined-type-and-qualifier; separate-verified-and-raw-apis; plain-ok-no-scope | — |
| DEC-ACC-039 | status-quo-defaults (raw `RandomAccessPolicy`); named-presets; scaled-to-declarations (constructor from declared sizes) | local-vs-remote-profiles and cli-flags are CLI surface (EXP-ECO-018) |

Sketches are Rust crates under `research/experiments/EXP-ECO-020/sketches/<candidate>/` in their own Cargo workspace (`unsafe_code = "forbid"`): public types, traits and signatures with bodies that delegate to the existing `entrybound` public API where possible and `unimplemented!()` otherwise (the experiment measures surface and compile-time guarantees, not runtime behaviour). Sketch authorship: the candidate owners (conventions §1), reviewed against the ledger description by a second session before any measurement.

## 5. Integration tasks (pre-registered)

| Task | Statement |
|---|---|
| IT01 | Pack a directory to a file with the balanced profile and report the archive's LAI |
| IT02 | Verify an archive and report, per check, whether it was performed and passed |
| IT03 | Read one entry's byte range from an HTTP URL with a total fetch budget, and refuse to use the bytes unless verified |
| IT04 | Extract an archive with a restrictive policy, progress reporting every 64 MiB, and cancellation after 1 s leaving a reported partial state |
| IT05 | Import a ZIP strictly and stream entries to a consumer, distinguishing untrusted legacy data from verified native data |
| IT06 | In an async HTTP server (tokio), stream one archive entry to a client without blocking the executor |
| IT07 | (compile-fail) pass bytes obtained without verification into `fn publish_verified(data: <verified type>)` |
| IT08 | Share an opened archive across 4 threads reading different entries concurrently |
| IT09 | Configure random-access limits appropriate for a remote 10 GiB archive |

Validation tasks (authored now): IT01v-IT09v with different parameters (e.g. STREAM layout, encrypted input, 1 GiB budget). Held-out tasks: sealed per conventions §5.

## 6. Environment and platform requirements

WSL2 Ubuntu 24.04 and Windows host (compile checks on both targets); toolchain 1.98.1 and nightly-2026-08-01 for rustdoc JSON; `cargo-semver-checks`, `tokei`, `trybuild` (dev-dependency of sketches) installed pinned.

## 7. Procedure and commands

Scripts in `research/tools/ecosystem/api/` (to be written):

1. `build_sketches.sh` — `cargo +1.98.1 check` of every sketch and its integration examples `examples/IT0x.rs` on both hosts; `cargo test` of `tests/compile_fail.rs` (`trybuild`) for IT07 and `static_assertions` for IT08 (`Send`/`Sync`).
2. `surface.py` — rustdoc JSON per sketch: public items, types, traits, methods, generic parameters, lifetimes in public signatures, error enum variants.
3. `task_complexity.py` — per (candidate, IT): `tokei` code lines of the example, distinct public API items referenced (from rustdoc JSON item ids resolved in the example via `rust-analyzer`-free name resolution on `use` paths and method calls), error variants handled, generic or lifetime annotations the user writes.
4. `deps.py` — `cargo tree -e normal` of each sketch (additions such as `tokio`, `futures`, `rayon`) with EXP-ECO-001 census fields.
5. `semver.sh` — `cargo semver-checks check-release --baseline-rev 9e44608` for sketches that build as a replacement of `entrybound`'s public API; count breaking changes by lint.
6. Simulated integrator arm (heuristic, SIMULATED): EXP-ECO-019 harness with IT01, IT03 and IT04 on three candidates (status quo, spec-four-object-shape, layered-low-and-high-level): 20 sessions per (task, candidate), agents see only rustdoc output of the sketch and the task statement, completion = `cargo check` passes and the example calls the required operations (checked by `task_checker_api.py`).
7. Output tables under `research/normalized/EXP-ECO-020/`.

## 8. Seeds, warmup, replication

Seed 2026091720 for session order. Compile and census steps run twice in fresh target directories; outputs must match. Sessions: at least 20 per cell (§4.6).

## 9. Metrics and measurement

| Metric | Canonical | Role |
|---|---|---|
| `flags_per_task` | yes (C6) | cost input, API analogue: distinct API items plus user-written type annotations per task (declared here as the API counting rule) |
| `task_success_rate`, `task_errors`, `task_steps` | yes (T-18) | heuristic (arm 6) |
| `decode_path_dependency_count` | yes (C3) | cost input per sketch |
| compile-fail outcome, `Send`/`Sync` assertion outcomes, public item counts, integration LoC, semver breaking-change count | no | record fields; compile-fail and assertion outcomes are binary design facts |

## 10. Normalization and aggregation

AGG-P per candidate; per task tables; no weighting across tasks except equal-weight mean and maximum as reported summaries.

## 11. Statistical analysis and use in the decision rule

Deterministic design facts; no inference. A candidate that promises a type-level verification guarantee and compiles IT07 fails its own stated property (R1 for that candidate's claim, HC-15 honesty of verification state). Dependency additions and public surface are cost inputs (C2, C3, C6). Semver-breaking counts are C7-style cost records. Simulated integrator results are heuristic only (§4.16).

## 12. Practical-significance thresholds

None (cost inputs, binary design facts, heuristic sessions; Appendix A.2).

## 13. Sensitivity analysis

LoC counts with and without error-handling boilerplate; semver checks against dev HEAD instead of `9e44608`; IT06 with `smol` instead of `tokio`.

## 14. Held-out (Phase D placeholder)

Conventions §5: sealed integration tasks counted once for every API candidate that reached the freeze.

## 15. Expected negative results worth recording

Type-level verification wrappers roughly doubling integration LoC for simple tasks; async shells pulling a runtime into the reader closure; the SPEC shape breaking most of the current public API.

## 16. Threats to validity

Sketches with `unimplemented!()` bodies can hide design problems that appear only in implementation; integration examples written by experts understate novice effort; name-resolution counting of API items is approximate.

## 17. Cost estimate

About 10 h compute (builds on two hosts, semver checks); about 20 GB disk; agent effort judgment for sketch authoring by candidate owners and review, scripted for measurement, plus about 180 simulated integrator sessions.

## 18. Gates and exposure

Conventions §1 and §2. No corpus family is used.
