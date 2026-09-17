# EXP-ECO-003: Crate and feature partition prototypes for a minimal-dependency reader

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-003 |
| Domain | ecosystem (program §31; cross-cuts §28 minimal reader, §12 HTTP, §8.6 codecs) |
| Status | NEEDS_TOOLING: research-worktree partition patches, `ebound-read` prototype binary, `ruzstd` decode switch and minimal HTTP client prototype |
| Runner | `spec.yaml` (deterministic identity and equivalence arm, tuning) and `spec-timing.yaml` (decode timing arm, tuning; validation copy generated for the look) |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |
| Timing | yes (timing arm only) |

## 1. Question

If the production library is partitioned so that a read-only reader builds without the writer, planner, legacy adapters, reconstruction encoders, HTTP stack or native decoders, how many decode-path dependencies, native and `unsafe` crates and binary bytes does each partition candidate carry; do the partitions preserve archive bytes and reader outcomes exactly; and is a pure-Rust Zstandard decoder correct on every production frame and non-inferior in decode throughput and memory to libzstd within the pre-registered bands?

## 2. Hypotheses and falsification

- **H1 (closure reduction).** A `read-baseline` feature partition (unencrypted list/verify/unpack/read, baseline codecs only, no HTTP) has a `decode_path_dependency_count` at most half of the status-quo monolithic build on `x86_64-unknown-linux-gnu`. *Falsified* if the reduction is below 50%.
- **H2 (bytes and outcomes preserved).** With default features the partitioned worktree produces pack output SHA-256 identical to production for every tuning item, profile and layout, and every read partition returns the production reader's (outcome class, reason code, extracted logical tree) on every test archive whose features it supports, and `UNSUPPORTED` with zero output on those it does not (HC-04). *Falsified* by any byte difference or disagreement (a prototype defect, not a trade-off: the prototype is fixed and re-run as a new version, R1 "Bugs").
- **H3 (pure-Rust Zstandard correctness).** `ruzstd` (version resolved in EXP-ECO-001) decodes every Zstandard frame emitted by production on tuning and validation items, including dictionary-bearing and ref-prefix frames of dense and extreme archives, bit-exactly. *Falsified* by any mismatch or refusal.
- **H4 (pure-Rust decode cost).** Reader partition decode with `ruzstd` is `DISTINGUISHABLE_WORSE` than with libzstd on `decode_throughput_bps` at corpus level (predicted 1 to 5 band units of T-05b). *Falsified* if the confirmatory comparison is `NON_INFERIOR` or `EQUIVALENT`.
- **H5 (HTTP client parity).** A minimal HTTP/1.1 client prototype passes every case of the `ebr-netem` origin contract and the EXP-ECO-009 local-server matrix that `reqwest` passes. *Falsified* by any case that `reqwest` passes and the prototype fails.

## 3. Decisions informed

DEC-ECO-016, DEC-ACC-016, DEC-ECO-041, DEC-ECO-038, DEC-CMP-008 (reader size per baseline set), DEC-CMP-007 (pure-Rust decode arm).

## 4. Candidates

Reference: **P0 `status-quo-monolithic-no-features`** (production at the execution SHA). Every other build is a research-worktree patch under `research/experiments/EXP-ECO-003/patches/`, never applied to production.

| Build id | Implements ledger candidate(s) | Definition |
|---|---|---|
| P0 | DEC-ECO-016 status-quo-monolithic-no-features; DEC-ACC-016 status-quo-unconditional-reqwest; DEC-ECO-041 status-quo-c-zstd-and-aws-lc; DEC-CMP-008 status-quo-all-registered-mandatory | production |
| P1-default | DEC-ECO-016 features-in-single-crate (default = everything) | features `writer`, `legacy`, `crypto`, `http`, `reconstruct-deflate`, `reconstruct-jpeg`, `platform-capture`; default enables all; must be byte-identical to P0 (H2 guard) |
| P1-read-baseline | features-in-single-crate; separate-read-only-format-crate and reader-crate-plus-feature-gates (dependency closure proxy, see §16); caller-supplied-source-only | `--no-default-features --features read` + baseline codecs; `ebound-read` binary with list/verify/unpack/read |
| P1-read-extended | as above + `reconstruct-*` decode, crypto decode | extended decode without writer and HTTP |
| P1-read-remote | cargo-feature-gate; separate-http-crate (closure proxy) | P1-read-baseline + `http` (reqwest as production configures it) |
| P1-read-remote-min | minimal-http11-client | P1-read-baseline + prototype HTTP/1.1 range client on a minimal client crate resolved in EXP-ECO-001 (first choice `ureq`; fallback `minreq`) |
| P1-read-remote-rustcrypto | rustls-pure-rust-provider | P1-read-remote with rustls on a pure-Rust provider crate resolved in EXP-ECO-001; `NOT_FOUND` records a negative result |
| P1-read-remote-notls | remove-tls-from-core | P1-read-remote with TLS features disabled (plain HTTP only) |
| P1-read-baseline-ruzstd | pure-rust-zstd-decoder-for-readers; pure-rust-default-native-behind-feature; DEC-CMP-007 pure-rust-decode-c-encode | P1-read-baseline with the Zstandard decode path on `ruzstd` behind feature `zstd-decode-rust` |
| B-store, B-store-zstd, B-store-zstd-lz4, B-store-zstd-lzma2, B-store-deflate-zstd, B-store-deflate | DEC-CMP-008 baseline-set candidates | P1-read-baseline with only the named codecs compiled; `B-store-deflate*` add a DEFLATE decoder (`miniz_oxide` inflate) that production does not use as a chunk codec (decoder-size measurement only) |

Measured-only quantities for DEC-ECO-038 candidates `binary-size-budget`, `source-size-budget` and `dependency-count-budget`: stripped binary bytes, reader-path Rust LoC (own plus dependencies), and `decode_path_dependency_count` for each build; `all-sections-mandatory` versus `reader-conformance-levels` is not a build difference and is left to the conformance domain.

### Exclusions

| Excluded | Reason |
|---|---|
| DEC-ECO-016 alloc-only-baseline-decoder-crate as a build | a `no_std` port is a rewrite, not a partition; measured statically instead: the fraction of reader-path crates that declare `no_std`/`alloc` support (EXP-ECO-001 metadata) and the list of `std`-only blockers |
| DEC-ECO-041 sandboxed-native-decoders | a process-isolation design; its C4 cost is assessed by the security assessor; no build here |
| Real crate split (moving modules into a new crate) | API and maintenance cost belongs to EXP-ECO-020 and the C2 assessor; dependency closure and binary size equal the feature-gated closure, which is what this experiment measures (threat §16.1) |

## 5. Corpus

- Identity and equivalence arm (`spec.yaml`, tuning): the tuning items listed in `spec.yaml` (text, build trees, many-small-file, logs, structured, numeric, database, firmware, PDF, versioned trees, metadata-heavy home snapshot, adversarial libarchive testsuite and generated malicious structures), each packed by P0 in 4 profiles × 2 layouts.
- Validation look (one): the corresponding validation list (`spec-validation*.yaml`, generated by `make_specs.py` from the lists in this protocol): `f01-validation-redis-7-2-16-git`, `f02-validation-cjson-cmake-build`, `f04-validation-objstore`, `f05-validation-rfc-texts`, `f06-validation-gen-structured-b`, `f07-validation-gen-numeric-b`, `f08-validation-sakila-sqlite`, `f10-debian-13-edk2-firmware-images`, `f13-validation-federal-register-pdf-small`, `f18-validation-cjson-releases`, `f19-validation-real-home-snapshot`, `f20-validation-generated-malicious-structures`, `f20-validation-real-cpython-archivetestdata`.
- Conformance archives: the native conformance corpus from the conformance domain when it exists (`research/conformance/`); until then the production test archives under `crates/entrybound/tests/data`.
- Timing arm: `spec-timing.yaml` tuning items (small and medium tiers); the validation copy uses the medium- and small-tier validation items above.

## 6. Environment and platform requirements

- WSL2 Ubuntu 24.04 (`wsl-ubuntu`) for all arms; timing on `st-v` = vCPU {2} (§4.3), `VIRTUALIZED` qualifier, cache `hot`.
- Windows host: builds and stripped sizes for `x86_64-pc-windows-msvc` for P0, P1-read-baseline, P1-read-remote and P1-read-remote-min (no timing on Windows until §14 item 3 runner additions exist).
- Calibration dependency for the timing arm: `EXP-CAL-AA-wsl-ubuntu`, `EXP-CAL-AAP-wsl-ubuntu`, `EXP-CAL-KE-wsl-ubuntu`, `EXP-CAL-BG-wsl-ubuntu` passing for `throughput`, `time_wall` and `memory_peak` families (§4.7; designed by the evaluation domain).
- Network: local only (the `ebr-netem` origin and EXP-ECO-009 local servers).

## 7. Procedure and commands

Tooling to build (NEEDS_TOOLING):

1. `research/experiments/EXP-ECO-003/patches/*.patch` — feature gates in `crates/entrybound` and a `crates/entrybound-read` binary target in the research worktree only; authored against the execution SHA; reviewed by a session that did not write them against the rule "gate, never change behaviour".
2. `research/tools/ecosystem/partition/build_all.sh` — for each build id: clone, apply patch, `cargo +1.98.1 build --release --locked` with the build's feature set, `strip` copy, record `cargo tree -e normal --target <t>`, feature list, build wall time and peak RSS (GNU time).
3. `research/tools/ecosystem/partition/probe_partition.py --build <id> --item <path> --scratch <dir>` — for `P1-default`: pack the item in 4 profiles × 2 layouts and compare PCI SHA-256 with P0's stored outputs; for read builds: run `list`, `verify`, `unpack` (to scratch) and one `read` per entry sample on each stored P0 archive of the item and compare (class, reason code, logical tree fingerprint) with P0; prints one JSON line.
4. `research/tools/ecosystem/partition/zstd_frames.py` — extract every Zstandard frame and dictionary reference from P0 archives via `ebound inspect --json --chunks` plan records; decode each with the `ruzstd` harness and compare with P0 decode.
5. `research/tools/ecosystem/partition/http_parity.sh` — run `research/harness/crates/ebr-netem` origin contract cases and the EXP-ECO-009 local-server cases against `ebound-read read <URL>` for P1-read-remote, -min, -rustcrypto, -notls.
6. Census: `research/tools/ecosystem/depcensus/graph.py` and `surface.py` (EXP-ECO-001) run on each build's `cargo tree` output.

Run order: build → `ebr run --spec spec.yaml` → verify-raw → normalize → frames and HTTP parity scripts → timing campaign in a quiet window (`ebr run --spec spec-timing.yaml --timing`) → validation look copies.

## 8. Seeds, warmup, replication

- Deterministic arm: seeds `order` 2026091703, `bootstrap` 2026091703; 2 repetitions + repeat-hash (§4.6, §4.13); warmups 0.
- Timing arm: seeds `order` 2026091713, `bootstrap` 2026091713; warmups 2 (small and medium tiers, §4.5); 10 minimum measured rounds, step 10, cap 30 (small) or 20 (medium), with the §4.6 precision rule (half-width ≤ 0.5 × `log1p(r)` of the primary metric; the rule never looks at effect direction); items still above target at the cap are flagged `imprecise`.

## 9. Metrics and measurement

| Metric | Role here | Source |
|---|---|---|
| `decode_path_dependency_count` | cost_input (C3) | census on each build |
| `decode_path_native_unsafe_count` | cost_input (C3/C4) | census on each build |
| `reader_disagreements` | binary (HC-03) | probe (P0 versus read builds) |
| `roundtrip_mismatches` | binary (HC-02) | frames script and unpack comparison |
| `origin_protocol_safety` | binary (HC-18) | HTTP parity script |
| `decode_throughput_bps` | banded (T-05b), primary for the timing arm (OD-07) | logical bytes / `wall_s` of `ebound-read unpack` to tmpfs-free ext4 scratch |
| `peak_rss_bytes` | banded (T-06), primary for the timing arm (OD-08) | GNU time `max_rss_kib` (harness review R1-16: GNU time mandatory) |
| `decode_wall_s` | banded (T-04), secondary | resource `wall_s` |
| `stripped_binary_bytes`, `build_wall_s`, `build_peak_rss_bytes`, `reader_rust_loc` | non-canonical descriptive | build script, `tokei` |
| `http_parity_cases_passed` | non-canonical descriptive | HTTP parity script |

## 10. Normalization and aggregation

- Census and binary outcomes: AGG-P per build and target; AGG-W for disagreements (sum).
- Timing arm: L1 item median of per-round paired log ratios (ruzstd / libzstd), L2 group mean, L3 family mean, L4 weighted by `research/methods/workload-mix.json` (equal-weight arm in sensitivity), strata by tier (§4.9).

## 11. Statistical analysis and use in the decision rule

- Deterministic parts: any H2 or H3 failure is a prototype bug (R1 "Bugs": fix without design change, new build version, restart at R1) or, for H3, an R1 elimination of `pure-rust-zstd-decoder-for-readers` as specified.
- Timing arm (§4.10-§4.12): on tuning, W and S are formed (expected W = libzstd reader, S = {W, ruzstd reader}); confirmatory on validation: W versus ruzstd at corpus level; superiority metrics declared from tuning (k_sup per pair, confidence 1 − 0.01/k_sup); non-inferiority at two-sided 0.99; family harm guard at two-sided 0.90; MDE gate ≤ 1 band unit before the look. Minimum support: ≥ 10 groups across ≥ 5 families.
- Cost tiers: `ruzstd` adds a reader-path pure-Rust dependency (T2 if without `unsafe`, T3 with `unsafe`; computed by EXP-ECO-001); feature partitions are T0-T1 (no wire change). R6 minimum gain applies across tiers; R10 keys order ties.
- HC-04 check: a read partition that silently decodes an unsupported feature, or returns a class other than `UNSUPPORTED` with zero output, is an R1 violation of the partition candidate.

## 12. Practical-significance thresholds

`decode_throughput_bps` T-05b, `decode_wall_s` T-04, `peak_rss_bytes` T-06, as defined in §3.2 and Appendix A.2 (no values restated). Binary metrics have no band (§3.3). Cost inputs have none.

## 13. Sensitivity analysis

- Timing arm: §6.5 equal-weight arm, leave-one-family-out, leave-three-families-out, family Dirichlet, WM-* mixes, tier-stratified, byte-weighted; selection-stability bootstrap (§6.7, 2,000 replicates, seed 2026091723); threshold perturbation ×0.5 (only where the A/A half-width condition of §6.4 holds) and ×2.
- Census: per target; with and without `build.rs`-only dependencies; with `multi-change` lock differences excluded.
- HTTP parity: with and without TLS cases.

## 14. Held-out (Phase D placeholder)

Conventions §8: the timing comparison and the equivalence arm are re-run once on held-out items after unlock, every candidate that reached the freeze is run, and the held-out MDE gate is computed before unlock.

## 15. Expected negative results worth recording

- A read-only partition that still pulls a large share of the closure through types shared with the writer (for example planner types used by the reader's plan interpretation).
- `ruzstd` refusing dictionary or ref-prefix frames, or being slower beyond the band.
- No pure-Rust `rustls` provider resolvable at the snapshot.
- Minimal HTTP client failing redirect or multipart cases that `reqwest` handles.

## 16. Threats to validity

1. A feature-gated closure equals a separate crate's closure only if the split crate needs no additional shared-types crate; the separate-crate candidates' extra API and maintenance cost is outside this measurement.
2. Patch quality: gating mistakes can remove or add code on the default path; the H2 identity guard and the review in §7 step 1 detect byte-visible mistakes but not dead-code differences.
3. `VIRTUALIZED` WSL timing: soft cap MEDIUM for Linux-native performance claims (§4.1); no Windows timing until runner additions exist.
4. The minimal HTTP client prototype is a strawman risk; its implementation is reviewed against the origin contract before measurement, and its failures are reported per case.
5. Harness review R1-16: peak RSS only from GNU time.

## 17. Cost estimate

- Compute: builds ≈ 16 configurations × 2 targets × 15 min ≈ 8 h; identity and equivalence arm ≈ 6 h; frames ≈ 2 h; HTTP parity ≈ 1 h; timing arm ≈ 12 h in quiet windows (2 builds × 12 items × up to 30 rounds, tuning; similar for validation). Total about 29 h.
- Disk: about 40 GB (targets, stored P0 archives for 25 items × 8 configurations).
- Agent effort: judgment for patch authoring and independent patch review (two sessions); scripted for runs.

## 18. Gates and exposure

Conventions §1 and §2. The timing arm additionally needs G-B timing gates and §4.7 calibration for WSL. Applicable families include exposed families (F04, F05, F13, F20); analysis by an unexposed session.
