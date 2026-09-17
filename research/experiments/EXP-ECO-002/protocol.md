# EXP-ECO-002: Encoder and decoder behaviour drift under dependency upgrades

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-002 |
| Domain | ecosystem (program §31; cross-cuts §7, §9, §27) |
| Status | NEEDS_TOOLING: per-arm research worktree builds and the `eco-drift-probe` binary must be built first |
| Runner | `ebr` spec (`spec.yaml`), one candidate per version arm, deterministic metrics |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |
| Timing | none |

## 1. Question

When a pinned codec or reconstruction dependency moves to another published version (a security fix, a minor release, or a backend swap), which compressed bytes, dictionary-training outputs, frozen export-profile bytes and golden vectors change, and do archives written with the pinned versions still decode, verify and reconstruct bit-exactly? The answer prices the update-policy candidates of DEC-ECO-051 and DEC-CMP-031 and the dependency-bound decode questions of DEC-CMP-038 and DEC-CON-007.

## 2. Hypotheses and falsification

- **H1 (encoder bytes drift).** Among the pre-registered arms (§4.2), at least one libzstd release step, one `flate2` backend or `miniz_oxide` step, or one `lzma-rust2` step changes compressed output bytes at fixed parameters on at least one tuning item. *Falsified* if every arm reproduces the pinned arm's output SHA-256 for every item and parameter set.
- **H2 (reconstruction data is version-bound).** Correction data produced by `preflate-rs =0.7.6`, or JPEG XL payloads produced by `jixel =0.2.19` and decoded by `jxl-oxide =0.12.6`, fail to reconstruct the original bytes (mismatch or refusal) under at least one other arm. *Falsified* if every arm reconstructs all pinned-arm outputs bit-exactly.
- **H3 (test and release resolve differ).** Frozen export profiles (`zip/portable-v1`, `tar.gz/pax-v1`) produce different bytes with `flate2` on `zlib-rs` (the production test resolve, harness README R1-06) than on `miniz_oxide` (the release resolve). *Falsified* if bytes are identical on every item.
- **H4 (declared memory drift).** `lzma-rust2` memory-usage estimates for identical LZMA2 parameters differ across arms. *Falsified* if identical in every arm.

No band-unit effects are predicted: every primary quantity here is binary or a count.

## 3. Decisions informed

DEC-ECO-051, DEC-CMP-031, DEC-MOD-011 (encoder output stability study part), DEC-LEG-008 (byte comparison across `flate2` backends and crate upgrades), DEC-CMP-021 (trainer output across libzstd 1.5.x), DEC-CMP-038 (decode compatibility of 0.7.6 corrections with other `preflate-rs` releases), DEC-CON-007 (cross-version differential decoding; `lzma2` memory estimates across versions), DEC-CMP-007 (encode output byte identity of candidate backends at fixed parameters).

## 4. Candidates

### 4.1 Policy candidates priced by the outcome table (not run as arms)

- DEC-ECO-051: status-quo-exact-pins-manual-bumps, advisory-triggered-bump-with-golden-vectors, exact-pin-everything, caret-with-lockfile-for-security-critical, vendor-and-freeze-encoders, semantic-profile-definitions, new-planner-id-on-encoder-change, retain-multiple-encoder-versions.
- DEC-CMP-031: status-quo-cargo-pins, record-encoder-versions, decision-only-freeze, vendored-encoders, new-id-on-any-encoder-change, toolchain-scoped-reproducibility.

Each policy's cost is computed from the arm outcomes: for every release in the observed 24-month history (from the arm snapshot), whether adopting it would change bytes (then: new planner or profile ID under `new-id-*`; golden-vector re-issue under `advisory-triggered-*`; nothing under `vendor-and-freeze-*` but a vendored-source C3/C4 burden from EXP-ECO-001; multiple retained encoders' dependency and binary-size increments from EXP-ECO-001 and EXP-ECO-003).

### 4.2 Version arms (run)

Arms were pre-registered from crates.io version lists retrieved 2026-09-17 (`arms-version-snapshot-2026-09-17.json`, retrieved by a design-time query; yanked versions excluded). The **reference arm** is the production pin set. Each other arm changes exactly one dependency (or one backend) and resolves everything else to the production lockfile (`cargo update -p <dep> --precise <v>` in a research worktree; if the precise update forces other changes, the arm records them and is labelled `multi-change`).

| Dependency | Reference (production pin) | Arms |
|---|---|---|
| libzstd via `zstd-sys` (with the matching `zstd-safe`/`zstd`) | as locked at the execution SHA (`zstd =0.13.3`) | `zstd-sys` 2.0.6+zstd.1.5.2; 2.0.7+zstd.1.5.4; 2.0.9+zstd.1.5.5; 2.0.13+zstd.1.5.6; 2.0.16+zstd.1.5.7; 2.1.0+zstd.1.5.7; and `zstd 0.14.0` (API-major arm, probe adapted, labelled `api-major`) |
| `lz4_flex` | `=0.14.0` | 0.13.1; 0.12.2; 0.11.6; 0.10.0 |
| `lzma-rust2` | `=0.20.0` | 0.20.1; 0.19.0; 0.18.1; 0.17.0; 0.16.5; 0.15.8 |
| `flate2` backend | `=1.1.9` with `miniz_oxide` as locked | `miniz_oxide` 0.8.9; 0.9.0; 0.9.1; `zlib-rs` backend 0.5.5; 0.6.0; 0.6.8; `flate2` 1.1.10 (default backend) |
| `bzip2` | `=0.6.1` as locked | `bzip2` 0.6.0; 0.5.2 (C `bzip2-sys` backend); `libbz2-rs-sys` 0.2.0; 0.2.5 |
| `preflate-rs` | `=0.7.6` | 0.7.1; 0.7.0; 0.6.4 |
| `jixel` (encode) | `=0.2.19` | 0.2.20; 0.2.22; 0.2.27 |
| `jxl-oxide` (decode) | `=0.12.6` | 0.12.5; 0.12.0; 0.11.4 |

Arms released after 2026-09-17 and before execution are added by the same rule (latest patch of each newer minor) and logged as additions before any output is inspected.

### 4.3 Exclusions

| Excluded | Reason |
|---|---|
| `sevenz-rust2` | dev-dependency only (test fixture writer); not on a production encode or decode path |
| Pure-Rust Zstandard **encoders** | no candidate in the ledger names one for the writer path; pure-Rust decode is EXP-ECO-003 |
| Cross-toolchain and cross-host drift | EXP-ECO-004 (toolchains, Windows vs WSL, emulated arm64) and EXP-ECO-005 (BLOCKED arms) |
| CPU-feature (SIMD dispatch) drift | included only as the sensitivity arm §13.2 on the reference arm |

## 5. Corpus

Tuning items (search and arm screening), all small or medium tier:

`f01-tuning-ripgrep-14-1-1-git`, `f01-tuning-zstd-v1-5-7-git`, `f02-tuning-lz4-intree-make-build`, `f04-tuning-sourcelike`, `f05-tuning-gutenberg-books`, `f06-tuning-census-popest-2023`, `f06-tuning-gen-structured-a-small`, `f07-tuning-silesia-mr`, `f08-tuning-chinook-sqlite`, `f10-gen-redundant-binary-blobs`, `f12-tuning-kodak-jpeg-edge-cases-small`, `f12-tuning-nasa-ivl-photos-small`, `f13-tuning-nasa-ntrs-pdf-small`, `f14-apache-maven-3916-bin-tarball`, `f14-gen-nested-archives-py`, `f18-tuning-lz4-releases-1-9-to-1-10`, `f20-tuning-real-libarchive-testsuite`.

Validation items (confirmatory counts, one look):

`f01-validation-redis-7-2-16-git`, `f02-validation-cjson-cmake-build`, `f05-validation-rfc-texts`, `f06-validation-osm-monaco-20250101-xml`, `f07-validation-gen-numeric-b-small`, `f08-validation-sakila-sqlite`, `f12-validation-fsa-jpeg-edge-cases-small`, `f12-validation-commons-cc0-photos-small`, `f13-validation-fsa-image-codecs-small`, `f14-gen-office-documents`, `f18-validation-cjson-releases`, `f20-validation-real-cpython-archivetestdata`.

Rationale: DEFLATE-bearing inputs (F14 jars and zips, F13 PDFs, F12 JPEGs for the JPEG XL path) exercise `preflate-rs` and `jixel`; text, structured, numeric and database items exercise Zstandard, LZ4 and LZMA2 planning. Minimum support (§4.9) for any corpus-level statement: at least 10 independence groups across at least 5 families in the split used; the lists above meet it on both splits.

## 6. Environment and platform requirements

- WSL2 Ubuntu 24.04 (`wsl-ubuntu`), toolchain `+1.98.1`, `CARGO_TARGET_DIR=/root/eb-research/target/eco002/<arm>`.
- Windows host subset: reference arm plus libzstd and `flate2` arms rebuilt with MSVC to detect compiler-dependent C output (`D:/eb-research/target/eco002-win/<arm>`).
- Network once (crate downloads). Not privileged. Storage about 60 GB of build targets (arms are built, measured, and their targets deleted after the raw rows are verified).

## 7. Procedure and commands

Tooling (to build; NEEDS_TOOLING):

- `research/tools/ecosystem/drift/make_arm.sh <arm-id>` — clone the repository at the execution SHA into `/root/eb-research/src/eco002/<arm-id>`, apply `cargo update -p <dep> --precise <version>` (and for backend arms the committed Cargo feature patch `research/experiments/EXP-ECO-002/patches/<arm-id>.patch`), record `cargo tree -e normal` diff against the reference lock, build `ebound` and the probe.
- `research/tools/ecosystem/drift/eco-drift-probe/` — a Rust binary in its own Cargo workspace depending on the arm's `entrybound` by path with `research-internals` enabled (codec entry points only; bytes are unaffected by the feature per `research/harness/internals-identity-check.md`). For one input item it: (a) runs production chunking and records each chunk's planner-selected codec and parameters from the **reference arm's plan log** (`--plan-from <reference plan jsonl>`), so every arm encodes identical chunk bytes with identical parameters; (b) encodes each chunk with the arm's codec implementation and prints per-codec output SHA-256 lists; (c) trains a Zstandard dictionary from the reference arm's sample selection and prints its SHA-256; (d) prints `lzma2` memory estimates for the reference parameter set; (e) decodes the **reference arm's stored outputs** (`--decode-from <reference output dir>`) and reports mismatches; (f) for DEFLATE and JPEG objects, applies the reference arm's stored correction/JPEG XL payloads through the arm's reconstruction path and reports byte equality with the original object. One JSON line on stdout (schema `ebr.harness.raw.v1` style, flat keys listed in §9).
- `research/tools/ecosystem/drift/golden_vectors.sh <arm-id>` — in the arm worktree: `cargo +1.98.1 test --workspace --release --locked` and the vector files (`docs/descriptor-vectors-v1.txt`, `docs/crypto-wire-v1-vectors.txt`, `docs/posix-metadata-v1-vectors.txt`, `docs/security-metadata-v1-vectors.txt`, test golden archives) → failing test names and count.
- `research/tools/ecosystem/drift/export_bytes.sh <arm-id> <item>` — `ebound pack` with the reference build, then `ebound convert --to zip|tar|tar.gz|tar.zst|tar.xz|tar.bz2` with the arm build; SHA-256 per profile.

Run order:

1. Build the reference arm; run the probe on all tuning items twice (seeded order) to create the reference plan logs and stored outputs; repeat-hash must pass.
2. For each arm: `make_arm.sh`, then `ebr run` (spec below) over tuning items; `golden_vectors.sh`; `export_bytes.sh` for `flate2`, `bzip2`, `lzma-rust2` and libzstd arms.
3. `ebr verify-raw`, `ebr normalize`; `research/tools/ecosystem/drift/outcome_table.py` joins each arm's digests with the reference arm's and writes `research/normalized/EXP-ECO-002/drift-outcomes.csv`.
4. Validation look (one): steps 1-3 on the validation items for every arm that changed nothing on tuning, plus every arm whose change set was non-empty (confirming the direction), recorded in the look registry.

## 8. Seeds, warmup, replication

- Seeds: `order` 2026091702, `bootstrap` 2026091702, `command` 2026091702 (the probe is deterministic; the seed only orders samples).
- Warmups 0 (deterministic metrics, §4.5 does not apply).
- 2 repetitions per (arm, item) in different processes, plus the §4.13 repeat-hash check. An intra-arm digest mismatch where determinism is claimed is itself an R1 violation artifact (§4.13): both outputs are committed and no confirming rerun is required.

## 9. Metrics and measurement

| Metric | Canonical? | Role | Source |
|---|---|---|---|
| `roundtrip_mismatches` | yes (HC-02, M02.1) | binary | probe (e), (f): reference outputs decoded by arm |
| `declared_tightness_ratio` | yes (T-23, M17.1) | reported for LZMA2 estimates only (< 1 is an HC-06 violation) | probe (d) versus measured peak by the counting allocator in EXP-ECO-003 where available |
| `artifact_bytes` | yes (T-01) | descriptive here (size effect of an upgrade) | probe per item |
| `chunk_output_changed_count`, `chunk_output_total` | no | descriptive record fields | probe (b) versus reference digests |
| `dictionary_changed` | no | descriptive (binary per item) | probe (c) |
| `lzma2_memory_estimate_bytes` | no | descriptive | probe (d) |
| `golden_vector_failures` | no | HC-16 MVT outcome count (any > 0 is a frozen-identifier break) | `golden_vectors.sh` |
| `export_profile_bytes_changed` | no | per frozen export profile, binary per item | `export_bytes.sh` |
| `output_sha256` | string | repeat-hash | probe |

## 10. Normalization and aggregation

- Per (arm, item): changed/unchanged per codec; per arm: fraction of items with any change (reported with counts, not intervals, because the quantity is deterministic).
- Per dependency: ordered release timeline with a change/no-change mark per arm; "drift frequency" = arms with any change / arms tested, over the last 24 months of releases.
- AGG-C style headline per dependency: minimum over families of the unchanged fraction (objective §3.0 AGG-C), with families reported.

## 11. Statistical analysis and use in the decision rule

- Deterministic facts; no inferential test. Split use: tuning determines the change sets; the single validation look confirms that no arm reported unchanged on tuning changes bytes on validation items (a reversal is counterevidence, §10.2 CB-6).
- R1 evidence: any `roundtrip_mismatches > 0` or intra-arm nondeterminism is an HC-02/HC-13 violation for a candidate that relies on that dependency version; any `golden_vector_failures > 0` shows that an in-place upgrade breaks a frozen identifier (HC-16), which is what the `new-id-*` candidates exist to prevent.
- Cost: the outcome table becomes the C7 "planner-ID churn caused by pinned encoder versions" line (§7.1) for each policy candidate, and the C3 lines for vendored or retained encoders come from EXP-ECO-001.

## 12. Practical-significance thresholds

None are applied to the binary and count outcomes (§3.3; Appendix A.2 roles). `artifact_bytes` differences are reported against the T-01 band only as description; they select nothing in this experiment.

## 13. Sensitivity analysis

1. **Thread count:** reference and libzstd arms with Zstandard worker threads 1 and 4 (where production exposes it through the plan); bytes must match across thread counts if determinism is claimed.
2. **CPU feature level:** reference arm under QEMU user-mode `qemu-x86_64 -cpu qemu64` (x86-64 baseline) versus native (`EMULATED` label; correctness only); only if `qemu-user` is installable in WSL without Docker.
3. **Windows MSVC build** of libzstd and `flate2` arms versus Linux GCC build.
4. **Precise-update side effects:** results with `multi-change` arms excluded and included.

## 14. Held-out (Phase D placeholder)

Conventions §8. At Commit A the frozen lockfile is re-probed on the tuning items; if any pinned version differs from this experiment's reference arm, the drift table is regenerated before the freeze.

## 15. Expected negative results worth recording

- Patch-level libzstd releases that change bytes at fixed levels (would show exact pins are necessary for any byte-identity promise).
- `lz4_flex` block output stable across minor versions (would show that pin was not the drift source).
- `preflate-rs` 0.6/0.7 corrections mutually undecodable.
- Test-resolve versus release-resolve export bytes differ (a latent frozen-profile hazard).

## 16. Threats to validity

- Replaying the reference plan isolates encoder drift but hides planner decisions that would change if encoder sizes change; a second "full pack" pass per arm (`ebound pack` with the arm build) reports archive-level PCI changes as a secondary record.
- `cargo update --precise` may be impossible for old versions with incompatible APIs; such arms are recorded `not-buildable` (a negative result) rather than patched.
- Probe correctness: the probe's encode path must match production; `eco-drift-probe --self-check` compares the reference arm's archive bytes from the probe with `ebound pack` bytes for every tuning item before any arm runs.
- Tuning items are a sample; drift absent on them may appear on other content (validation look mitigates).

## 17. Cost estimate

- Compute: about 30 h (≈ 36 arms × build 20 min + tests 10 min + probe over 17 items; Windows subset about 4 h).
- Disk: about 60 GB peak build targets (deleted per arm after verification); raw outputs under 1 GB.
- Agent effort: scripted for arms and runs after the probe is written; judgment for the probe design review (one session checking §16 self-check).

## 18. Gates and exposure

Conventions §1 and §2. Applicable families include F05, F12, F13 and F20, which appear in this session's L7 exposure record; the analysis of results for those families is executed by an unexposed session.
