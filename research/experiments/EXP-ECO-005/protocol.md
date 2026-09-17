# EXP-ECO-005: macOS, native ARM64 and container-image arms of the toolchain, reproducibility and packaging matrix

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-005 |
| Domain | ecosystem (program §31, §32) |
| Status | **BLOCKED** |
| Blocked by | (1) no macOS host (macOS/APFS `PLATFORM_BLOCKED`, PROGRESS standing constraints; hosted runners declined 2026-09-12); (2) no native ARM64 machine (native ARM64 `PLATFORM_BLOCKED`); (3) Docker Desktop fails to start on this host (stale AF_UNIX socket reparse points, Win32 error 1920; PROGRESS storage-incident notes), so pinned distribution images and a container-image channel cannot be built or run |
| Unblocked by | a macOS machine (Apple silicon and Intel) with Xcode command-line tools; a native ARM64 Linux machine; Docker Desktop verified to start with its relocated disks (then the Docker arms become READY on this host) |
| Runner | `spec-macos-arm64.yaml` (validated design spec), plus the EXP-ECO-004 scripts reused unchanged |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |

## 1. Question

The same questions as EXP-ECO-004, for the platforms this host cannot provide: does the workspace build and pass tests at MSRV and later toolchains on `aarch64-apple-darwin`, `x86_64-apple-darwin` and native `aarch64-unknown-linux-gnu`; are pack outputs, identity roots and frozen export-profile bytes identical to the x86-64 Windows and Linux results; does a release binary rebuild bit-identically in a pinned container image; what does building the native C dependencies (libzstd, aws-lc) require on those platforms; and can container-image and Homebrew (macOS bottle) channels be produced and validated?

## 2. Hypotheses and falsification

- **H1.** Build and tests pass at MSRV 1.94 and at 1.98.1 on both macOS targets and native ARM64 Linux. *Falsified* by any failure.
- **H2.** PCR on macOS and native ARM64 equals x86-64 PCR for every tuning cell; LAI and AUX differ only where macOS metadata capture adds classes (documented per class). *Falsified* by any PCR difference or unexplained LAI difference.
- **H3.** Frozen export-profile bytes on macOS and native ARM64 equal the x86-64 bytes (HC-16). *Falsified* by any difference.
- **H4.** A release binary built twice in the pinned image `docker.io/library/rust:<1.98.1>-bookworm@<digest>` (digest recorded at unblock time) is bit-identical, and equal to the chroot build of EXP-ECO-004 when the same controls are applied. *Falsified* by a difference (sections recorded with `diffoscope`).
- **H5.** Distribution images whose default `rustc` is below 1.94 fail the build at the first crate raising its MSRV, matching EXP-ECO-004's `distro_floor_build.sh` prediction. *Falsified* if an image below 1.94 builds.

## 3. Decisions informed

DEC-ECO-073, DEC-ECO-003, DEC-ECO-071, DEC-MOD-011, DEC-LEG-008, DEC-ECO-041 (native build-requirement report on ARM64 and macOS).

## 4. Candidates

As EXP-ECO-004 §4, restricted to the cells this host cannot run: toolchain arms (1.94.x, 1.98.1, latest stable) on macOS and native ARM64; `container-image` (DEC-ECO-003) built from a committed `Dockerfile` pinned by base-image digest; `distro-and-homebrew-packages` Homebrew bottle on macOS (`brew install --build-bottle` of a local formula, `brew audit --strict`); `reproducible-release-builds` second environment in Docker. Exclusions as in EXP-ECO-004 (hosted CI, trusted publishing, public transparency-log upload).

## 5. Corpus

The EXP-ECO-004 tuning and validation lists, materialized on each platform by extracting WSL-made pax tarballs with the platform's native `tar` (on macOS the APFS materialization loss is itself part of the condition and is recorded per item).

## 6. Environment and platform requirements

| Arm | Requires |
|---|---|
| macOS arm64 and x86-64 | macOS host(s) with APFS, Xcode CLT, rustup; non-admin sufficient for build/test/pack; admin not required |
| native ARM64 Linux | an aarch64 Linux machine (any distribution with glibc), rustup, gcc |
| Docker images | Docker Desktop starting on this host (or any Docker/Podman engine), network for image pulls, base images pinned by digest |

## 7. Procedure and commands

Unchanged EXP-ECO-004 scripts: `toolchain_builds.sh`, `pack_matrix.py`, `cross_host_compare.py` (with the macOS rules added to `cross-host-rules.yaml` before any macOS output is inspected), `repro_release.sh --env docker --image <digest>`, `distro_floor_build.sh --docker <image list>`, `packaging_dryrun.sh --channel container-image|homebrew-bottle`. Distribution images: Debian oldstable/stable/testing, Ubuntu supported LTS releases, Fedora supported releases, Alpine supported branches, pinned by digest at unblock time.

## 8. Seeds, warmup, replication

Seeds `order` 2026091705, `bootstrap` 2026091705, `command` 2026091705; 2 repetitions per (platform, toolchain, item) + repeat-hash; reproducibility 2 builds per configuration; no warmups.

## 9. Metrics, 10. aggregation, 11. analysis, 12. thresholds

Identical to EXP-ECO-004 §9-§12 (`build_availability`, `cross_host_identity_agreement`, `migration_determinism_fraction`, `roundtrip_mismatches`, `migration_steps_count`, `flags_per_task`; AGG-S by platform pair with worst-condition headline; binary facts drive R1 for candidates claiming the property; no bands apply).

## 13. Sensitivity analysis

As EXP-ECO-004 §13; additionally macOS with and without extended attributes captured (`com.apple.*` classes) to separate metadata-driven AUX differences.

## 14. Held-out (Phase D placeholder)

Conventions §8; the macOS and ARM64 identity matrix is re-run once on held-out items after unlock if the platforms are available by then. If not, every decision whose scope includes macOS or native ARM64 excludes that platform explicitly (§4.1: a `PLATFORM_BLOCKED` platform inside a scope prevents `DECIDED`).

## 15. Expected negative results worth recording

aws-lc or libzstd build requirements differing on macOS (e.g. assembler or cmake versions); APFS normalization of names changing LAI for F19 items; container builds non-reproducible due to base-image drift unless pinned by digest.

## 16. Threats to validity

Different machines introduce hardware and OS-version variation beyond the toolchain; each platform arm records its exact OS build, CPU model and toolchain. Rosetta-translated x86-64 on Apple silicon is not native x86-64 and is labelled `EMULATED` if used.

## 17. Cost estimate

About 30 h of machine time across platforms once unblocked; about 80 GB disk per platform for targets; agent effort scripted (the scripts exist from EXP-ECO-004).

## 18. Gates and exposure

Conventions §1 and §2. This protocol exists so that `remaining-unknowns` can cite the exact missing platforms for DEC-ECO-073, DEC-ECO-003, DEC-ECO-071, DEC-MOD-011, DEC-LEG-008 and DEC-ECO-041.
