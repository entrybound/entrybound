# EXP-CONF-014 — Container, native-architecture, macOS and hosted-infrastructure arms of corpus regeneration, reader conformance, pinned incumbent matrix and continuous fuzzing (BLOCKED)

| Field | Value |
|---|---|
| Status | **`BLOCKED`**: Docker Desktop is unavailable on this host (startup fails on stale AF_UNIX socket reparse points, Win32 error 1920; `research/PROGRESS.md` 2026-09-16 storage incident); macOS/APFS and native ARM64 are `PLATFORM_BLOCKED` (no local hardware; hosted runners declined by the program owner 2026-09-12); OSS-Fuzz and other hosted fuzzing infrastructure are declined by the same standing decision; the ZipDiff harness requires ≥ 128 GB RAM and ≥ 300 GB storage (host 64 GB RAM, WSL VM 31 GB) |
| Domain / program sections | conformance / §29, §30, §28 |
| Decision IDs informed | DEC-ECO-042, DEC-ECO-047, DEC-ECO-029, DEC-ECO-014, DEC-ECO-041, DEC-LEG-063, DEC-LEG-082, DEC-CON-005, DEC-CRY-013 |
| HC oracles | HC-11 MVT-11(a) platform variation for readers; HC-08 cross-host identity of the corpus census; HC-16 regeneration identity |
| Decision type | not assigned (gate G-A) |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1 |
| Depends on | EXP-CONF-004 (corpus and census driver), EXP-CONF-005 (regeneration checker, CI-binary interface), EXP-CONF-008/009 (targets), EXP-CONF-011 (adapter cases) |

## 1. Question

Do the conformance corpus regeneration, the production and independent reader census, and the legacy observed-outcome
matrix hold on the platforms this host cannot provide (pinned Linux containers, native ARM64, macOS/APFS); does a pinned
containerized incumbent matrix (and ZipDiff's harness) change adapter expectation evidence; and what does continuous
hosted fuzzing add over local campaigns?

## 2. Arms, hypotheses and the exact missing access

| Arm | Hypothesis (falsified by one counterexample) | Decisions | Missing access |
|---|---|---|---|
| B1 Docker `linux/amd64` pinned image (digest recorded) | `ebcc-gen` regeneration is byte-identical to the committed manifest; production and `ebir` census tuples equal the WSL tuples for every case; the `ci-binary-with-junit-output` interface runs as a container image | DEC-ECO-042, DEC-ECO-014 | Docker Desktop (host fault, error 1920) |
| B2 Docker memory-limited containers | outcomes under container resident-memory limits (cgroup-enforced) equal the address-space-limit results of EXP-CONF-007-LM for the same limits, except where virtual reservations exceed resident use (reported) | DEC-CON-005, DEC-CRY-013 | Docker Desktop (systemd scope memory limits were found unenforced in this WSL2 configuration, so resident-memory limits need containers) |
| B3 native ARM64 (Linux and macOS) | regeneration identity and census tuple equality with x86-64; decode outcomes invariant (MVT-11(a) architecture axis, native not emulated); `ebir` and production reader agreement unchanged; native C decoder build requirements recorded (DEC-ECO-041) | DEC-ECO-042, DEC-ECO-041 | native ARM64 hardware (`PLATFORM_BLOCKED`) |
| B4 macOS/APFS | regeneration identity; CLI census path semantics (case-insensitive default volume, Unicode normalization) do not change outcome tuples; macOS Archive Utility as an additional ZIP runtime column | DEC-ECO-042, DEC-LEG-082 | macOS host (`PLATFORM_BLOCKED`) |
| B5 Dockerized pinned incumbent matrix | observed outcomes of pinned images (GNU tar, bsdtar, 7-Zip, Info-ZIP, Python, Go, Java, .NET, libzip, Rust readers, busybox tar) on the `ebcc-adapters-v1` cases reproduce the WSL/Windows observations where versions coincide; additional runtimes add distinguishing cases (DEC-LEG-082 marginal value) | DEC-ECO-047, DEC-LEG-063, DEC-LEG-082 | Docker Desktop |
| B6 ZipDiff harness reuse | ZipDiff's dockerized parser set produces divergences on the ZIP adapter cases consistent with B5 | DEC-ECO-047 (`reuse-zipdiff-dockerized-harness`) | Docker Desktop, ≥ 128 GB RAM, ≥ 300 GB storage |
| B7 hosted continuous fuzzing (OSS-Fuzz or equivalent) | continuous hosted campaigns find no deduplicated defect that EXP-CONF-009's local 72 CPU-hour campaigns missed over the same period | DEC-ECO-029 (`oss-fuzz-integration`) | hosted infrastructure (declined by standing decision); also public repository onboarding approval |

## 3. Design when unblocked

- B1/B2/B5/B6: images pinned by digest (`research/conformance/containers/images-v1.json`), built from committed
  Dockerfiles; runs through `research/experiments/EXP-CONF-014/spec-docker.yaml` (validated at design time), which
  executes the EXP-CONF-004 census and the EXP-CONF-005 regeneration checker inside the container via `docker run`.
- B3/B4: the same census and regeneration drivers on the native host; `ebir` cross-compiled (`GOARCH=arm64`,
  `GOOS=darwin|linux`); production built natively with toolchain `1.98.1`.
- B7: onboarding with the program owner's approval; weekly export of hosted findings into the regression ledger with the
  CC§8 handling; comparison window equal to EXP-CONF-009's campaign period.

## 4. Candidates, metrics, analysis

Candidates are platforms and harness hosts (not design candidates). Metrics: regenerated-case SHA-256 mismatches and
census tuple mismatches (binary by §4.13 and HC-11/HC-08 oracles), `reader_disagreements` (binary),
`import_agreement_fraction` (T-20) for B5, `unresolved_fuzz_findings_at_coverage_target` contributions (binary) for B7.
Analysis as in the arms' source experiments; EMULATED never applies (these arms exist to replace emulated evidence).

## 4a. Seeds, replication, thresholds and sensitivity

- Seeds and repetitions as in each arm's source experiment (generation seed `2809170401`; two repetitions with
  repeat-hash for deterministic arms; B7 hosted campaigns are continuous and not replicated).
- Practical-significance thresholds: §3.3 (binary) for regeneration, census and fuzz-finding checks; T-20
  (`metric_thresholds.import_agreement_fraction`) for B5 agreement fractions.
- Sensitivity: B1/B5 with two image digests of the same distribution release (userland drift); B3 on Linux ARM64 versus
  macOS ARM64 where both are available; B5 with and without runtimes whose versions differ from the WSL/Windows columns.

## 5. Split discipline

Conformance corpora and tuning-derived inputs only; no validation or held-out use.

## 6. Expected negative results worth recording

Container-specific differences (overlay filesystem metadata, locale defaults), path-semantics differences on APFS, ARM64
native-decoder build requirements, hosted-fuzzing findings missed locally.

## 7. Threats to validity

Container images pin userland, not the kernel (WSL kernel shared); macOS and ARM64 hosts would be single machines.

## 8. Cost estimates (when unblocked)

| Item | Estimate |
|---|---|
| Compute | 120 CPU-hours locally (B1 ≈ 30 h, B2 ≈ 10 h, B5 ≈ 60 h, B6 ≈ 20 h); B3/B4 ≈ 60 CPU-hours on the external hosts; B7 hosted |
| Disk | 150 GB (images, B6 ZipDiff storage profile excluded; B6 needs ≥ 300 GB on a suitable host) |
| Agent effort | **scripted**, with **judgment, low** for image pinning and result triage |
| Platform | Docker Desktop (`linux/amd64`), native ARM64 Linux, macOS/APFS, hosted fuzzing, ≥ 128 GB RAM host |

## 9. Unblocking conditions

- Docker: owner reboot and a clean Docker Desktop start with the relocated data disk (PROGRESS resume note), then
  `docker run --rm hello-world` succeeds; B1, B2, B5 become `NEEDS_TOOLING` (image build); B6 additionally needs a
  ≥ 128 GB RAM host.
- ARM64 / macOS: access to native hardware (owner decision; hosted runners are currently declined).
- Hosted fuzzing: owner approval of a hosted service for the public repository.
