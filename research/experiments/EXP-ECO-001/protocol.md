# EXP-ECO-001: Dependency census, capability classification and longevity signals

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-001 |
| Domain | ecosystem (program §31; feeds §7 cost element C3 for every domain) |
| Status | READY (design). Instruments are installable tools plus experiment-local scripts specified below; one judgment table (module capability map) needs an independent reviewer. |
| Runner | Script-driven census (not an `ebr` per-item spec). Outputs are AGG-P tables. |
| Method | `research/decision-method.md` at pre-registration commit `14b977c`; objective `research/archetypal-objective.md` OD-22, OD-21 (M21.6), OD-24 (C4 inputs), HC-12 (M22.5). |
| Timing | none |

## 1. Question

For every package resolved by the production workspace (`Cargo.lock` at research baseline `9e44608` and at the dev HEAD in force at execution), and for every alternative dependency named by a candidate in the §31 ledger decisions, what are its capability class (baseline reader, extended decoder, writer only, adapter only, test/research only; plus HTTP, crypto and platform-capture sub-classes), decode-path exposure, native and `unsafe` surface, licence, advisory history, maintenance, publisher, MSRV and build-requirement signals, and which dependency-policy candidates (DEC-ECO-017) admit, penalize (§7.2 maintenance penalty) or eliminate (§7.5 licence screen, F-23) which of them?

## 2. Hypotheses and falsification

These are directional expectations for cost inputs and binary screens. Cost inputs have no practical-significance band (Appendix A.2 role `cost_input`), so no band-unit effect size is predicted.

- **H1 (native code on the decode path).** Under the status-quo lockfile, at least two crates containing native C/C++/assembly (candidates: `zstd-sys`, and `aws-lc-sys` or `ring` through `reqwest`/`rustls`) are reachable from the baseline-reader or extended-decoder class on `x86_64-unknown-linux-gnu`. *Falsified* if the classification attributes fewer than two native crates to those classes.
- **H2 (maintenance penalty).** At least one reader-path dependency fails at least one §7.2 maintenance criterion (release within 24 months or feature-complete with stable spec; ≥ 2 publishers or organization ownership; no unresolved RUSTSEC advisory; MSRV ≤ workspace MSRV 1.94). Expected examples: `cms =0.3.0-pre.2` (pre-release), single-publisher reconstruction crates. *Falsified* if every reader-path crate passes all four.
- **H3 (HTTP stack share).** Crates reachable only through `reqwest` make up at least a quarter of the unique `name@version` set on the decode path. *Falsified* if the share is below 25%.
- **H4 (licence screen, binary).** No normal (non-dev, non-build) dependency of `entrybound` or `entrybound-cli` carries a strong-copyleft SPDX licence (GPL, LGPL, AGPL). *Falsified* by any such crate (an R2 elimination under §7.5 for the candidate that ships it).
- **H5 (baseline codec public specification, binary).** Of the codecs named by DEC-CMP-008 baseline-set candidates, Zstandard (RFC 8878), DEFLATE (RFC 1951) and LZ4 frame have freely available normative specifications independent of one implementation; PPMd, Deflate64 and the preflate correction format do not. *Falsified* per codec by the citation table (§7 step 9).

## 3. Decisions informed

Primary (section 31): DEC-ECO-017, DEC-ECO-041, DEC-ECO-074, DEC-ECO-051, DEC-ECO-067, DEC-ECO-071, DEC-ACC-016, DEC-CMP-007, DEC-CMP-008, DEC-CMP-021, DEC-CMP-026, DEC-CMP-038, DEC-CON-007, DEC-CRY-006, DEC-CRY-014, DEC-CRY-106, DEC-INT-012, DEC-LEG-024, DEC-LEG-035, DEC-LEG-037, DEC-LEG-047, DEC-LEG-052, DEC-LEG-075, DEC-LEG-085, DEC-PLT-017, DEC-PLT-030, DEC-PLT-040.

Also informs (dependency-footprint parts only): DEC-ECO-016, DEC-ECO-038 (decoder crate size inputs), DEC-ECO-006 (argument-parser crate footprint), DEC-ACC-007 (async runtime and thread-pool crate footprint), DEC-CRY-101 (HTTP client footprint for an online TSA client).

What this experiment does **not** settle: licence interpretation beyond SPDX compatibility lists, patent review (RaptorQ, DEC-INT-012), and the UnRAR licence reading (DEC-LEG-047) are routed to external review (§7.5, §8.3); only the texts' SHA-256 and SPDX classification are recorded.

## 4. Candidates

### 4.1 Reference and status quo

- **REF-PROD dependency set:** `Cargo.lock` at `9e44608` (status quo for every decision below).
- **dev-HEAD dependency set:** `Cargo.lock` at the execution commit (reported beside REF-PROD; differences listed).

### 4.2 Policy candidates (DEC-ECO-017)

Each policy candidate is operationalized as a pre-registered admission-rule file `research/experiments/EXP-ECO-001/policies/<candidate_id>.yaml`, written before any census output is inspected, and applied mechanically to the census table:

| candidate_id | Operational rule set evaluated |
|---|---|
| status-quo-need-based-exact-pins | exact `=` pins required for normal deps; no class-specific rule |
| tiered-classification-with-admission-rules | per class: baseline reader and extended decoder require permissive SPDX, no native code, no pre-release, ≥ 2 publishers or org owner, release ≤ 24 months or declared feature-complete, MSRV ≤ 1.94, zero open RUSTSEC; writer-only relaxes native code; adapter-only relaxes publisher count; test/research-only screens licence only |
| cargo-deny-policy | `cargo deny check licenses advisories bans sources` with the class-free config `policies/cargo-deny.toml` |
| cargo-vet-with-imported-audits | `cargo vet` with imports from the audit sets published by Mozilla, Google, the Bytecode Alliance and the ISRG (pinned commits); outcome: count of crates without an imported or exempted audit |
| vendored-reviewed-sources | `cargo vendor` size (bytes, files, LoC) of the reader-path closure: the review burden |
| reimplement-baseline-reader-deps | reader-path non-crypto crates under a LoC threshold listed as reimplementation candidates; LoC totals reported |
| published-sbom | SPDX and CycloneDX SBOM generation succeeds for both targets (tool: `cargo-cyclonedx`, `cargo-sbom`); field completeness counted |
| no-prerelease-in-v1 | any normal dependency with a semver pre-release version fails |

### 4.3 Alternative-dependency candidates (other §31 decisions)

The crate names below are **search seeds taken from the ledger candidate descriptions**, not assertions that such crates exist. `resolve_candidates.py` resolves each seed against the crates.io sparse index at the recorded snapshot to the latest non-yanked version; a seed that does not resolve is recorded as `NOT_FOUND` (a negative result, not an omission).

| Decision | Candidate ids | Seeds resolved and censused |
|---|---|---|
| DEC-ACC-016 | status-quo-unconditional-reqwest; cargo-feature-gate; separate-crate; minimal-http11-client; caller-supplied-source-only | reqwest (current features); ureq; minreq; attohttpc; hyper (client-only features); none (caller-supplied) |
| DEC-CMP-007 | status-quo-mixed; pure-rust-default-c-optional; c-reference-everywhere; pure-rust-decode-c-encode; vendored-frozen-encoders | ruzstd; zlib-rs; libbz2-rs-sys; lzma-rust2; lz4_flex; zstd-sys; libz-sys; bzip2-sys; liblzma-sys (xz2); lz4-sys |
| DEC-ECO-041 | status-quo-c-zstd-and-aws-lc; pure-rust-zstd-decoder-for-readers; pure-rust-default-native-behind-feature; rustls-pure-rust-provider; remove-tls-from-core; sandboxed-native-decoders | ruzstd; rustls with a pure-Rust crypto provider seed (rustls-rustcrypto); `sandboxed-native-decoders` is censused only as the native crates it would isolate (its process-isolation design cost is C4, assessed elsewhere) |
| DEC-CMP-026, DEC-CMP-038, DEC-CON-007 | status-quo-v6; libjxl-backend; lepton-class; brunsli; vendor-and-freeze; extended-set-crate-reference | jixel; jxl-oxide; preflate-rs; jpegxl-rs / jpegxl-sys (libjxl); lepton_jpeg; brunsli seeds |
| DEC-CRY-006 | status-quo-prerelease-cms; pin-stable-release; minimal-profile-parser; vendor-and-fuzz | cms (all versions: release history); der; spki; x509-cert |
| DEC-CRY-014, DEC-CRY-106 | dependency-review; replace-crate-with-audited-impl | hkdf; hmac; argon2; aes-gcm-siv; x-wing; ml-kem; x25519-dalek; libcrux-ml-kem |
| DEC-INT-012 | par2; raptorq-rfc6330; custom-rs-erasure-format | raptorq; reed-solomon-erasure; reed-solomon-simd; par2 seeds |
| DEC-LEG-024, DEC-LEG-035, DEC-LEG-047 | libarchive-parity; sandboxed-libarchive; unrar-binding-sandboxed; clean-room-rust-decoder; libarchive-backed | compress-tools; libarchive3-sys; unrar; unrar_sys; C library CVE history for libarchive, UnRAR, 7-Zip (NVD) |
| DEC-LEG-037, DEC-LEG-052, DEC-LEG-075, DEC-LEG-085 | add-deflate64; add-ppmd; add-zstd-93-and-20; add-bzip2-lzma-xz; add-bcj-filters; import-lzip-lz4-only | deflate64; ppmd-rust; sevenz-rust2; lzma-rust2 filter features; lz4_flex frame; brotli; weezl (LZW) |
| DEC-PLT-017, DEC-PLT-030, DEC-PLT-040, DEC-ECO-074 | rustix-at-nofollow; cap-std-ext-apis; audited-unsafe-ffi-boundary; windows-rs-safe-wrappers; third-party-safe-wrappers; workspace-isolated-ffi-crate | rustix; cap-std; cap-fs-ext; cap-primitives; xattr; nix; windows-sys; windows; exacl |
| DEC-ECO-006 | declarative-parser | clap; lexopt; pico-args; argh; bpaf |
| DEC-ACC-007 | internal-thread-pools; async-runtime | rayon; tokio; smol |

The full seed list is committed as `research/experiments/EXP-ECO-001/candidate-crates.toml` before execution. Adding a seed after results exist is allowed and logged (R0).

### 4.4 Exclusions (before execution)

| Excluded | Reason |
|---|---|
| Hosted dependency-intelligence services requiring accounts (e.g. commercial SCA dashboards) | Account creation is outside agent authority; the public crates.io, RustSec, OSV and NVD sources give the §7.1 C3 fields. |
| Legal interpretation of licences and patents | §7.5 and §8.3 route these to external review; this experiment records SPDX expressions and licence-text hashes only. |
| `trusted-publishing` style provenance checks of upstream crates | crates.io trusted-publishing metadata is recorded when the API exposes it; judging its sufficiency is DEC-ECO-071 analysis, not census. |

## 5. Corpus and inputs

No workload corpus: every metric is per design (AGG-P). Inputs:

- repository at `9e44608` and at the execution HEAD (read-only clones under `/root/eb-research/src/eco001-<sha>`);
- crates.io sparse-index responses for every censused crate, stored with retrieval UTC time and SHA-256 under `/root/eb-research/cache/eco001/index/` (the "index snapshot");
- `rustsec/advisory-db` at a pinned commit recorded in the run manifest;
- crates.io API `/api/v1/crates/<name>`, `/owner_user`, `/owner_team` responses (stored, hashed);
- upstream repositories named by crate metadata, cloned `--filter=blob:none` (history only) at retrieval time;
- NVD CVE API 2.0 results for C libraries (libzstd, liblzma/xz-utils, libarchive, UnRAR, 7-Zip, libjxl, zlib, aws-lc, BoringSSL/ring C portions), stored and hashed;
- the repository's own fixture and vector files (for DEC-ECO-067): `crates/*/tests/data/**`, `docs/*vectors*.txt`, `tools/*/**` fixtures.

## 6. Environment and platform requirements

- **WSL2 Ubuntu 24.04** (`wsl-ubuntu`): census scripts, `cargo metadata`/`cargo tree` for all targets, `cargo audit`, unsafe scanner, `tokei`, rustdoc JSON (nightly-2026-08-01) for platform crates. `cargo fetch --locked` first: the feasibility check showed Windows-only crates are not in the WSL registry cache, so `--offline` metadata for `x86_64-pc-windows-msvc` fails until fetched.
- **Windows host** (`windows-host`): `cargo bloat` for `x86_64-pc-windows-msvc`, verification that native build requirements recorded from `build.rs` scans actually hold (MSVC, cmake, perl, nasm) by a clean build in a fresh `CARGO_TARGET_DIR` on `D:`.
- Targets censused: `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin` (metadata and rustdoc only for the non-host targets; no linking, so macOS targets are not blocked for this census).
- Network: crates.io, GitHub/GitLab (public clone), NVD, OSV, advisory-db. No privileged operations. Storage on D:/WSL ext4 only (`disk_guard.py` must pass).

## 7. Procedure and commands

All scripts live in `research/tools/ecosystem/depcensus/` (to be written; stdlib Python 3.12 in the research venv plus `tomllib`), and every output carries a provenance header (command, tool versions, snapshot hashes).

1. `install_tools.sh` — `cargo +1.98.1 install --locked` of pinned `cargo-deny`, `cargo-vet`, `cargo-bloat`, `cargo-cyclonedx`, `cargo-sbom`, `tokei`, `askalono-cli`; versions recorded. `cargo-audit` 0.22.2 and `osv-scanner` are already installed.
2. `snapshot.py --seeds candidate-crates.toml --lock <sha>:Cargo.lock ... --out /root/eb-research/cache/eco001/` — fetch sparse-index entries, crates.io API documents, advisory-db clone at pinned commit; write `snapshot-manifest.json` (every response hash).
3. `resolve_candidates.py --seeds candidate-crates.toml --snapshot ... --out candidates.lock.json` — resolve seeds; for each resolved seed create a scratch Cargo project `/root/eb-research/scratch/eco001/cand-<name>` depending on exactly that version with default features, and run `cargo generate-lockfile` against the snapshot (records the candidate's transitive closure).
4. `graph.py --workspace <clone> --targets <list> --out graph.jsonl` — per target: `cargo metadata --format-version 1 --locked --filter-platform <t>` and `cargo tree -e normal,build,dev --target <t> --prefix depth --format "{p} {f}"`; one row per (target, dep-kind, parent, child, enabled features).
5. `module_usage.py --src crates/entrybound/src crates/entrybound-cli/src --out module-usage.csv` — for each source file, every external crate path referenced (`use <crate>`, `<crate>::`, `extern crate`), by token scan that skips comments and strings.
6. **Judgment table:** `module-capability-map.csv` maps each source file (and each CLI subcommand handler) to capability classes {baseline-reader, extended-decoder, writer-only, adapter-only, crypto, http, platform-capture, platform-restore, cli-surface, test-research-only}. Authored by one session and **independently re-classified** by a second session that has not seen the first; disagreements resolve to the class set with the higher cost tier (§7.2 "disputed tier resolves to the higher tier"), and both versions are committed.
7. `classify.py --graph graph.jsonl --usage module-usage.csv --map module-capability-map.csv --out classes.csv` — a crate's class set is the union over files that use it; transitive dependencies inherit their parents' class sets; dev-only and build-only edges give `test-research-only` and `build-only`.
8. `surface.py --registry-src ~/.cargo/registry/src --crates classes.csv --out surface.csv` — per crate package: `unsafe` blocks, `unsafe fn`, `unsafe impl`, `unsafe trait`, `extern` blocks (counted by `research/tools/ecosystem/unsafe-census/`, a small `syn`-based Rust binary built in its own Cargo workspace, `unsafe_code = "forbid"` itself); `links` key; `-sys` name; `build.rs` invoking `cc`, `cmake`, `bindgen`, `pkg-config`; bytes and files of `*.c *.cc *.cpp *.h *.hpp *.S *.s *.asm`; `tokei` Rust LoC; build-requirement flags (C compiler, cmake, perl, nasm, go, clang).
9. `licenses.py` — SPDX expression from metadata; `askalono` identification of shipped licence files with confidence; classes permissive / weak-copyleft / strong-copyleft / unknown; mismatches between declared and detected listed. **Codec specification table:** `codec-specs.csv` (judgment, cited per §9.3: standard number, version, locator, access date) for every decode-path codec and transform → `baseline_codec_public_spec`.
10. `maintenance.py` — from the snapshot: last release date, release count in the last 24 months, yanked versions, pre-release flag, declared `rust-version`, owners (users, teams), repository URL; from the blob-less clone: commits and distinct authors in the last 12 and 24 months, share of commits by the top author in the last 24 months.
11. `msrv_probe.sh` — for reader-path crates and candidate crates without a declared `rust-version`: `cargo +1.94.x check` of the scratch project (step 3); MSRV gap in minor versions.
12. `advisories.py` — `cargo audit --db <pinned advisory-db> --no-fetch --json -f <lock>` per lockfile and candidate lock; full advisory history per crate (any version) from advisory-db files; `osv-scanner --lockfile` cross-check; NVD CVE counts per year for the C libraries.
13. `api_coverage.py` — rustdoc JSON (`cargo +nightly-2026-08-01 rustdoc -Z unstable-options --output-format json --target <t>`) for the platform crates; matched against the pre-registered capability list `api-capabilities.yaml` (capability → required OS primitive patterns, e.g. `openat2`/`RESOLVE_BENEATH`, `*at` with `AT_SYMLINK_NOFOLLOW`, `fgetxattr`/`lsetxattr` by fd, `utimensat`, `mknodat`, `DeviceIoControl`+`FSCTL_GET_REPARSE_POINT`, `GetSecurityInfo`/`SetSecurityInfo`, `FindFirstStreamW`, macOS `getattrlist`/`fgetxattr` with `XATTR_NOFOLLOW`, `acl_get_fd_np`); cell = SAFE_API (callable without `unsafe`), UNSAFE_ONLY, ABSENT.
14. `bloat.sh` / `bloat.ps1` — `cargo bloat --release --crates -n 0` for `ebound` on Linux and Windows; per-crate `.text` bytes (descriptive).
15. `fixture_provenance.py` — every tracked fixture/vector file: first-add commit, generator reference if any (script path that regenerates identical bytes, verified by regeneration into scratch), embedded third-party markers (copyright strings, known magic of camera/encoder outputs), classification generated-in-repo / third-party-with-source / unknown.
16. `policy_eval.py --policies policies/ --census census.parquet --out policy-outcomes.csv` — apply each §4.2 rule set; also compute the §7.2 maintenance penalty per crate and the resulting tier raise for every candidate dependency set.
17. `gate_cost.sh` — run `cargo deny check` and `cargo vet` once each in the local gate on WSL and Windows; record wall time (descriptive, not decision-grade timing) and findings counts.
18. `repeat_check.sh` — rerun steps 4-16 in a fresh process tree against the same snapshot; every output table must be byte-identical (§4.13 repeat-hash rule for deterministic metrics).

## 8. Seeds, warmup, replication

- No randomness. Deterministic metrics: 2 repetitions in different processes plus the repeat-hash check (§4.6, §4.13). A mismatch invalidates the table until explained (snapshot drift is excluded by construction because both runs read the stored snapshot).
- No warmup. The live-network step (2) runs once; its stored responses are the evidence.

## 9. Metrics and measurement

Canonical metrics (Appendix A.2):

| Metric | Role | OD / M | Measured as |
|---|---|---|---|
| `decode_path_dependency_count` | cost_input (C3) | OD-22 M22.1 | unique `name@version` in {baseline-reader ∪ extended-decoder} class, per target |
| `decode_path_native_unsafe_count` | cost_input (C3/C4) | OD-22 M22.2 | count of decode-path crates with native code; and total `unsafe` blocks+fns in decode-path crates (both reported) |
| `rustsec_advisory_count` | cost_input (C3) | OD-22 M22.3 | open advisories affecting the resolved versions |
| `license_compatibility` | binary (R2) | OD-22 M22.3 | 1 iff no strong-copyleft normal dependency ships in production crates |
| `maintainer_count` | cost_input (C3) | OD-22 M22.4 | crates.io owners (users; teams reported separately) |
| `msrv_gap` | cost_input (C3) | OD-22 M22.4 | minor versions above workspace MSRV 1.94 |
| `release_age_years` | descriptive | OD-22 M22.4 | snapshot date minus last non-yanked release |
| `baseline_codec_public_spec` | binary (HC-12) | OD-22 M22.5 | per codec in a baseline-set candidate |
| `build_availability` | descriptive | OD-26 M26.3 | build requirements per target (partial; full matrix in EXP-ECO-004) |
| `reader_fragmentation_count` | cost_input (C5) | OD-21 M21.6 | optional/extended features whose decode dependencies are absent from the baseline-reader class (static count) |

Non-canonical descriptive record fields (never primary, §0.2 item 4): class set, native source bytes, build requirement flags, publishers-as-org flag, commits and authors in 12/24 months, top-author share, advisory history count, NVD CVE counts per year, pre-release flag, yanked-version count, `.text` bytes per crate, SBOM completeness, cargo-vet unaudited count, API coverage cells, fixture provenance class.

## 10. Normalization and aggregation

AGG-P (objective §3.0): one value per candidate dependency set and target; decode path primary (OD-22). No family weighting. Class-specific counts are reported for every class and for the union, per target; the headline per candidate is the worst target.

## 11. Statistical analysis and use in the decision rule

- No inferential statistics: census facts are exact given the snapshot.
- **R1 screens:** any `unsafe` added to workspace crates by a candidate is an F-23 violation (§7.5); recorded for platform-FFI candidates (DEC-ECO-074, DEC-PLT-017/030/040) as the design fact that the unsafe must live in a non-workspace crate.
- **R2 screens:** `license_compatibility` = 0 eliminates the shipping candidate (§7.5); `baseline_codec_public_spec` = 0 for a baseline-set codec eliminates that baseline-set candidate under HC-12/SPEC §5.8 admission (R2 unspecifiable).
- **§7.2 tiers:** census fields feed C3 and the maintenance penalty (tier +1 per failing reader-path dependency, capped at T4). The policy outcomes table states, per decision candidate, the computed C3 tier. Cost tiers act only at R4 condition 5, R6 and R10 key 1 (§2.0).
- This experiment's scripts are the C3 half of §14 item 7 ("cost-accounting scripts for C1-C3"); they are delivered as reusable tooling for every domain.

## 12. Practical-significance thresholds

None apply: every metric here is `cost_input`, `descriptive` or `binary` (Appendix A.2). Maintenance-penalty criteria are those of §7.2 and `thresholds.json` `cost_tiers.maintenance`; binary screens follow §3.3 and §7.5. Nothing is restated here with different values.

## 13. Sensitivity analysis

1. **Snapshot-date sensitivity:** recompute the maintenance penalty at snapshot date + 6 months holding release data fixed; crates that would cross the 24-month criterion are flagged `penalty_imminent`.
2. **Classification sensitivity:** penalties and counts under each of the two independent module-capability maps and under their union; crates whose class differs are listed.
3. **Target sensitivity:** per-target results; a candidate's tier is the worst target within the decision's platform scope.
4. **Feature-unification sensitivity:** `cargo tree` with the workspace's actual feature resolution versus `--no-default-features` on `entrybound` (static) — reveals features pulled only by unification.
5. **Publisher definition:** users only; users + teams; organization-owned repository as a pass (as §7.2 permits).

## 14. Held-out (Phase D placeholder)

Not applicable to per-design census facts (AGG-P, no workload items). The C3 tier is recomputed on the frozen lockfile at Commit A (§5.5), and any change between the tuning-time census and the frozen census is reported as a deviation.

## 15. Expected negative results worth recording

- Seeds with no maintained pure-Rust implementation (likely candidates: RAR5 decode, Deflate64, PPMd with more than one publisher).
- cargo-vet imported audits covering only a minority of reader-path crates.
- Many crates without a declared `rust-version`.
- A policy candidate that eliminates the status-quo reader (e.g. `no-prerelease-in-v1` against `cms`).
- Declared SPDX licence differing from the shipped licence text.

## 16. Threats to validity

- File-level reachability over-approximates call-level reachability; a crate used only by writer code inside a mixed file is classed as reader. Mitigation: partition prototypes in EXP-ECO-003 measure real closures.
- crates.io owners are publish rights, not active maintainers; commit authorship in monorepos is misattributed.
- Advisory-db and NVD completeness; CPE ambiguity for C libraries.
- Rustdoc JSON format depends on the pinned nightly.
- The snapshot ages; the reopen rule §11 item 2 covers later advisories and maintenance loss.
- Candidate crate seeds come from ledger text; an unnamed better alternative is an R0 completeness gap for a critic to fill.

## 17. Cost estimate

- Compute: about 6 h (fetch and blob-less clones about 2 h; unsafe/tokei/rustdoc about 1.5 h; bloat builds on both hosts about 1.5 h; repeat run about 1 h).
- Disk: about 15 GB (registry sources, blob-less clones, two release builds), all on D:/WSL ext4.
- Agent effort: scripted (Sonnet-class) for tooling and runs; judgment for the module capability map (two independent short sessions) and the codec specification citation table (one session plus review).

## 18. Gates, pre-registration status, exposure disclosure

- This protocol is a Phase C-design document, not the §12.2 `record.md`. Before execution: gate G-A (constraint crosswalk; ledger schema v2 with `hc_ids`, `od_ids`, `decision_type` assigned by an independent session; validation-look registry) must hold, and `record.md` is instantiated from this protocol with those assignments and commit SHAs and committed before any run (§12.1, R3).
- Designer: Phase C-design ecosystem agent (2026-09-17), not involved in constructing any ledger candidate set. L7 exposure (§5.7): this session read `research/PROGRESS.md` change-log entries and the first rows of the `research/corpus/coverage.md` independence-group table, which name held-out items or upstream sources in families F03, F04, F05, F09, F11, F12, F13, F15, F16, F17 and F20. No held-out content, listing, statistic or path was opened. This experiment uses no corpus family.
- Feasibility smoke (NOT_DECISION_GRADE, 2026-09-17, WSL): `cargo metadata --locked --offline` resolved the Linux target and exposed the license, rust_version and links fields; the Windows target failed offline until crates are fetched; `cargo audit --no-fetch --json` produced its JSON report; crates.io owner and OSV query endpoints answered HTTP 200. No numbers from the smoke are used.
