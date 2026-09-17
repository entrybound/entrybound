# EXP-ECO-004: Toolchain and MSRV matrix, local merge gate, release reproducibility and packaging channels

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-004 |
| Domain | ecosystem (program §31, §32; informs §34 release gates) |
| Status | READY (Windows x86-64, WSL x86-64, emulated aarch64 via user-mode QEMU without Docker). macOS, native ARM64 and container-image arms are EXP-ECO-005 (BLOCKED). |
| Runner | `spec.yaml` (WSL toolchain byte-identity arm), `spec-windows.yaml` (Windows arm), `spec-aarch64-emulated.yaml` (emulated arm); gate, reproducibility and packaging arms are scripts |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |
| Timing | no decision-grade timing; gate and build wall times are descriptive |

## 1. Question

Which MSRV, toolchain-pinning, merge-gate, release-reproducibility and distribution-channel candidates are feasible on the hosts this program has, and at what measurable cost: does the workspace build and pass tests at the declared MSRV and later toolchains on both hosts; are pack outputs, identity roots and frozen export-profile bytes identical across toolchains and hosts; can release binaries be rebuilt bit-identically in two independent build environments per target; how long and how complete is a local scripted merge gate; and which packaging channels can be produced and validated locally, with how many steps?

## 2. Hypotheses and falsification

- **H1 (declared MSRV holds).** `cargo +1.94.x build --workspace --locked` and `test --workspace` succeed on both hosts. *Falsified* by a build failure (a defect in the declared `rust-version`, reported as such).
- **H2 (toolchain-independent archive bytes).** For every tuning item, profile and layout, pack output SHA-256 is identical across toolchains 1.94.x, 1.97.1, 1.98.1, latest stable at execution and `nightly-2026-08-01` on the same host. *Falsified* by any difference; a difference means byte-determinism promises must name the toolchain (DEC-MOD-011 `pinned-tuple` versus `cross-minor-version-stable`).
- **H3 (known cross-host differences only).** Between Windows and WSL, PCR is identical for every cell, and LAI and AUX differ only in the ways recorded by findings F-0001 (platform-security feature bit) and F-0002 (executable bit unobservable on NTFS; AUX precision). *Falsified* by any PCR difference or by any LAI/AUX difference not explained by those findings.
- **H4 (frozen export profiles are toolchain- and host-independent).** `zip/portable-v1`, `tar/pax-v1` and the four compressed-tar pax-v1 profiles produce identical bytes across toolchains and hosts from the same `.eb`. *Falsified* by any difference (a frozen-profile break, HC-16).
- **H5 (release reproducibility requires explicit controls).** Two builds of the same commit in independent build environments on one target differ by default and are bit-identical with `--remap-path-prefix` for source, registry and toolchain paths, `SOURCE_DATE_EPOCH`, identical toolchain and `--locked`. *Falsified* if identical by default, or still different with those controls (then list the differing sections).
- **H6 (distro toolchain floor).** At least one currently supported stable release of Debian, Ubuntu (LTS), Fedora or Alpine ships a default `rustc` older than the workspace MSRV 1.94. *Falsified* if every supported release's default `rustc` is at least 1.94.

## 3. Decisions informed

DEC-ECO-073, DEC-ECO-071, DEC-ECO-003, DEC-MOD-011 (cross-OS and cross-toolchain byte comparison), DEC-LEG-008 (cross-platform golden vectors on Windows and WSL).

## 4. Candidates

| Decision | Candidates measured here | Excluded here, with reason |
|---|---|---|
| DEC-ECO-073 | status-quo-msrv-floating-stable-manual (reference); fixed-msrv-per-format-major and rolling-n-minus-2-msrv (toolchain arms 1.94.x, latest stable − 2 minors, latest stable); distro-toolchain-floor (distro survey + build at the lowest supported distro default `rustc`); exact-release-toolchain-pin (1.98.1 arm); local-scripted-merge-gate (gate arm) | hosted-ci-matrix: standing decision 2026-09-12 excludes hosted runners; its platform coverage is represented by the local gate plus EXP-ECO-005's BLOCKED cells |
| DEC-ECO-071 | status-quo-none (reference); reproducible-release-builds (reproducibility arm); signed-provenance-attestations (offline in-toto/SLSA-style attestation generated locally with a scratch key; steps counted); git-archive-only-sources (compare `cargo package` file list with `git archive` of the tag); maintainer-count-vetting (EXP-ECO-001 data) | trusted-publishing: needs a hosted OIDC identity; keyless signing with public transparency-log upload: publishing content requires owner approval and is not needed to measure binding |
| DEC-ECO-003 | status-quo-source-only; crates-io-plus-binstall (`cargo package`, `cargo publish --dry-run`, binstall metadata check); prebuilt-signed-binaries (release builds for both targets signed with minisign and cosign scratch keys; steps and files counted); distro-and-homebrew-packages (`cargo deb` + `lintian`; `cargo generate-rpm` + `rpmlint`; winget manifest + `winget validate` on Windows; Linux Homebrew formula lint only if `brew` can be installed under `/root/eb-research` without system changes, otherwise recorded as not measured); reproducible-release-with-provenance (reproducibility + attestation arms combined); self-signed-with-entrybound (`ebound sign` over the release archive; steps) | container-image and macOS Homebrew bottles: EXP-ECO-005 (Docker unavailable; macOS PLATFORM_BLOCKED); licence choice itself is an owner decision (§8.3 route for interpretation) |
| DEC-MOD-011 | lai-only, pinned-tuple-status-quo, cross-minor-version-stable, declared-determinism-levels: evaluated against the identity matrix (which identities are stable across which variation) | emulated and native ARM64 byte comparison beyond user-mode QEMU: EXP-ECO-005 |
| DEC-LEG-008 | status-quo-implicit, golden-vectors-ci, receipt-records-encoder-versions: evaluated against H4 | Docker and native ARM64 cells: EXP-ECO-005 |

## 5. Corpus

- Byte-identity arms (tuning): `f01-tuning-ripgrep-14-1-1-git`, `f02-tuning-lz4-intree-make-build`, `f04-tuning-sourcelike`, `f05-tuning-gutenberg-books`, `f06-tuning-gen-structured-a-small`, `f07-tuning-silesia-mr`, `f08-tuning-chinook-sqlite`, `f10-gen-redundant-binary-blobs`, `f12-tuning-kodak-jpeg-edge-cases-small`, `f13-tuning-nasa-ntrs-pdf-small`, `f14-gen-nested-archives-py`, `f18-tuning-lz4-releases-1-9-to-1-10`, `f19-tuning-zoo`, `f19-tuning-real-home-snapshot`, `f20-tuning-real-libarchive-testsuite`.
- Validation look (one): `f01-validation-redis-7-2-16-git`, `f02-validation-cjson-cmake-build`, `f05-validation-rfc-texts`, `f06-validation-osm-monaco-20250101-xml`, `f07-validation-gen-numeric-b-small`, `f08-validation-sakila-sqlite`, `f12-validation-fsa-jpeg-edge-cases-small`, `f13-validation-fsa-image-codecs-small`, `f14-gen-office-documents`, `f18-validation-cjson-releases`, `f19-validation-deep`, `f19-validation-real-home-snapshot`, `f20-validation-real-cpython-archivetestdata`.
- Windows arm: the same items materialized on NTFS under `D:/eb-research/corpus-win/<item_id>` by extracting a WSL-made `tar --format=pax` of each item with Windows `tar.exe` (bsdtar 3.8.8). The materialization loses what NTFS cannot hold (F-0002); that loss is part of the cross-host condition and is recorded per item.

## 6. Environment and platform requirements

- WSL2 Ubuntu 24.04 (`wsl-ubuntu`) and Windows 11 host (`windows-host`), both x86-64.
- Emulated `aarch64-unknown-linux-gnu`: cross build with `gcc-aarch64-linux-gnu` (apt) and test runner `qemu-aarch64 -L /usr/aarch64-linux-gnu` from `qemu-user` (apt), set through `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER`; no binfmt registration and no Docker. Labelled `EMULATED`; correctness and determinism only (§4.1).
- Independent build environment on Linux: a `debootstrap` Ubuntu 24.04 minimal root under `/root/eb-research/chroots/eco004-b` entered with `unshare --mount --uts --ipc --pid --fork chroot` (no host configuration change), with its own `rustup` install of the same toolchain, own `CARGO_HOME`, different build path and different locale and umask. On Windows the second environment is a different `CARGO_HOME`, `RUSTUP_HOME` and build path on `D:` (weaker; stated as a threat).
- Toolchains installed via `rustup toolchain install` (downloads approved): 1.94.x (latest patch), 1.97.1, 1.98.1, latest stable at execution, `nightly-2026-08-01`.
- Network for downloads and `cargo publish --dry-run`. Not privileged. Storage on D: only.

## 7. Procedure and commands

Scripts under `research/tools/ecosystem/release/` (to be written):

1. `toolchain_builds.sh` / `toolchain_builds.ps1` — per toolchain: fresh clone at the execution SHA, `cargo +<tc> build --release --locked -p entrybound-cli`, `cargo +<tc> test --workspace --release --locked`, `cargo +<tc> clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`; record pass/fail, failing tests, warnings count, build wall time (descriptive). Targets `/root/eb-research/target/eco004/<tc>`, `D:/eb-research/target/eco004-win/<tc>`.
2. `pack_matrix.py --ebound <bin> --item <path> --scratch <dir>` — pack in 4 profiles × 2 layouts; `ebound inspect --json` for LAI, PCR, AUX, PCI; then `ebound convert --to` each of the six frozen export profiles from the balanced INDEXED archive; print one JSON line with every SHA-256 and identity (used by all three specs).
3. `cross_host_compare.py` — join WSL and Windows rows per (toolchain, item, cell); classify each difference as `F-0001-explained`, `F-0002-explained` or `unexplained` using the pre-registered rules in `cross-host-rules.yaml` (feature bit 0x10000; executable-bit and timestamp-precision AUX members); also verify every WSL archive on Windows and every Windows archive on WSL (`ebound verify`, `ebound unpack` outcome equality).
4. `repro_release.sh` — build `ebound` release twice (environment A: host WSL; environment B: chroot) at the same commit and toolchain, first with defaults, then with the controls of H5; compare SHA-256; on difference run `diffoscope` (apt) and record the differing sections. `repro_release.ps1` does the Windows analogue with two `CARGO_HOME`/path pairs.
5. `merge_gate.sh` (+ `merge_gate.ps1` calls) — implements the local-scripted-merge-gate candidate: fmt; clippy both hosts; tests both hosts; `research/harness/internals-identity-check.sh`; fuzz smoke (`cargo +nightly-2026-08-01 fuzz run <target> -- -max_total_time=60` for each existing fuzz target, if any exist at the execution SHA, else recorded absent); determinism check (pack twice, cross-host PCR); golden vectors; `cargo deny check`; `cargo audit`; run three times; record per-step wall time, pass/fail, flaky steps (differing outcomes across the three runs).
6. `distro_survey.py` — download and hash package indexes at retrieval time: Ubuntu (all supported releases: `Packages.gz` for `rustc`, `rustc-1.*`), Debian (oldstable, stable, testing), Fedora (supported releases and rawhide `primary.xml`), Alpine (supported branches and edge `APKINDEX`), Arch (`extra.db`), Homebrew (`formula.json` for `rust`), winget (`Rustlang.Rust.MSVC` manifests in a sparse checkout of `microsoft/winget-pkgs`); output: default and available `rustc` versions per release with release support status from each distro's published support list (URL and access date recorded).
7. `distro_floor_build.sh` — build the workspace with the lowest default `rustc` found among supported releases, installed via `rustup` at that exact version (the distro binary itself is not used; the version is), recording the first failing crate.
8. `packaging_dryrun.sh` / `.ps1` — per channel candidate: execute the channel steps listed in §4 in a scratch copy; record commands (count), flags (count), produced files, validator outcome (`lintian`, `rpmlint`, `winget validate`, `cargo publish --dry-run`), and failures. Signing keys are generated in scratch and deleted; nothing is uploaded.
9. `attest_offline.sh` — produce an in-toto statement over the release archive and binaries (subjects by SHA-256) with a scratch key using a pinned `in-toto` Python package in the research venv, and verify it; steps counted.
10. `ebr run` of the three specs; `ebr verify-raw`; `ebr normalize`; `research/tools/ecosystem/release/identity_matrix.py` writes `research/normalized/EXP-ECO-004/identity-matrix.csv` (identity × variation → identical / explained difference / unexplained difference).

## 8. Seeds, warmup, replication

- Seeds `order` 2026091704, `bootstrap` 2026091704, `command` 2026091704.
- Deterministic metrics: 2 repetitions per (toolchain, item) in different processes + repeat-hash (§4.6, §4.13); a same-toolchain same-host digest mismatch where determinism is claimed is an R1 violation artifact (both outputs committed).
- Merge gate: 3 full runs (flakiness is itself a recorded outcome). Release reproducibility: 2 builds per environment per configuration.
- No warmups (no timing claims).

## 9. Metrics and measurement

| Metric | Canonical | Role here |
|---|---|---|
| `build_availability` | yes (M26.3) | descriptive: build and test success per (toolchain, host, target), build wall time |
| `cross_host_identity_agreement` | yes (HC-08, M18.1) | binary for PCR; LAI/AUX differences classified per H3 |
| `migration_determinism_fraction` | yes (HC-13, M20.2) | binary per frozen export profile across toolchains and hosts |
| `roundtrip_mismatches` | yes (HC-02) | binary for cross-host verify/unpack |
| `migration_steps_count` | yes (M26.2) | descriptive: commands per packaging channel |
| `flags_per_task` | yes (C6) | cost input: flags per packaging channel task |
| `toolchain_pack_identity`, `release_binary_reproducible`, `gate_step_wall_s`, `gate_flaky_steps`, `distro_default_rustc`, `distro_floor_first_failure`, `unexplained_cross_host_differences` | no | descriptive record fields |

## 10. Normalization and aggregation

AGG-S per evaluation condition (platform pair, toolchain pair); headline is the worst condition (objective §3.0). Identity outcomes are per (item, cell) exact values, aggregated as counts and AGG-C fractions per family (minimum over families headline). Packaging and gate outputs are AGG-P per candidate.

## 11. Statistical analysis and use in the decision rule

- No inferential statistics (deterministic facts and censuses).
- R1 evidence: an unexplained PCR difference across hosts (HC-08), an export-profile byte difference (HC-16 for frozen profiles), or a same-configuration nondeterminism (HC-13) is a violation artifact attached to every candidate that claims the corresponding property.
- Scope: DEC-MOD-011 candidates are screened by which identity survives which variation; a candidate that promises stability over a variation shown to change bytes is eliminated at R1 for that scope, or must declare the variation in its tuple.
- Costs: gate runtime and packaging steps are C6/C7-style cost records (descriptive), not selection metrics; `flags_per_task` enters C6.

## 12. Practical-significance thresholds

None applied: binary (§3.3), descriptive and cost-input roles only (Appendix A.2).

## 13. Sensitivity analysis

- Identity matrix with and without F19 metadata-heavy items (isolates AUX-driven differences).
- Cross-host comparison with the Windows materialization replaced by a direct WSL-path read from Windows (`\\wsl.localhost\Ubuntu\...`) for three items, to separate materialization loss from capture differences.
- Release reproducibility with each control removed one at a time (which control is necessary).
- Distro floor using "latest available versioned `rustc-1.*` package" instead of the default `rustc`.

## 14. Held-out (Phase D placeholder)

Conventions §8. The identity matrix is re-run once on held-out items after unlock with the frozen build; the toolchain set is the one frozen at Commit A.

## 15. Expected negative results worth recording

- Declared MSRV 1.94 failing because a dependency raised its own MSRV (then the declared MSRV is stale).
- Nightly or a newer stable changing pack bytes (e.g. through `std` float formatting or hash iteration in writer code).
- Release binaries non-reproducible even with controls (e.g. embedded build IDs from native C builds).
- Distribution channels failing validation (`lintian` errors on vendored C sources, winget validation requirements).
- The local gate taking hours, with flaky steps.

## 16. Threats to validity

- The chroot shares the kernel and CPU with environment A; reproducibility across genuinely different machines is not tested (EXP-ECO-005 would add a Docker/other-machine arm).
- Windows second environment differs only in paths and homes.
- User-mode QEMU does not emulate every CPU feature path; `EMULATED` results are correctness-only.
- Materializing items on NTFS changes the input; cross-host LAI comparisons are about the materialized trees, which is the real cross-host situation but not a controlled one.
- Distro support status is taken from each distro's published lists at the access date (`EXTERNALLY_SOURCED`).

## 17. Cost estimate

- Compute: toolchain builds and tests ≈ 5 toolchains × 2 hosts × 40 min ≈ 7 h; pack matrices ≈ 5 × 2 × 28 items × 8 cells ≈ 4 h; emulated aarch64 build and tests ≈ 4 h; reproducibility ≈ 3 h; merge gate 3 runs ≈ 4 h; packaging ≈ 2 h; distro survey ≈ 0.5 h. Total about 25 h.
- Disk: about 80 GB (target directories per toolchain on both hosts, chroot ≈ 2 GB, emulated build), deleted after verification except outputs.
- Agent effort: scripted; judgment only for the `cross-host-rules.yaml` classification rules (pre-registered before any cross-host output is inspected) and review of packaging validator messages.

## 18. Gates and exposure

Conventions §1 and §2. Applicable families include exposed families (F04, F05, F12, F13, F20); analysis by an unexposed session.
