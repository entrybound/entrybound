# EXP-EVAL-013: Claim-scope extension to macOS/APFS and native ARM64 (Apple AEA/AA security and signature probes, APFS metadata fidelity comparison, native ARM64 whole-system timing and determinism)

| Field | Value |
|---|---|
| Domain | evaluation (program sections 6, 7, 34, 35) |
| Status | **BLOCKED**. `PLATFORM_BLOCKED`: no macOS host (APFS, `aea(1)`, `aa(1)`, `ditto`) and no native ARM64 hardware. Hosted runners were declined by the program owner (standing decision 2026-09-12). The emulated ARM64 route (QEMU in Docker) is also unavailable, because Docker Desktop does not start (PROGRESS 2026-09-16). Even if available, it would serve determinism only, never timing (§4.1). |
| Kind | The macOS and ARM64 arms of EXP-EVAL-005 and EXP-EVAL-006, designed completely, so that remaining-unknowns and v1-scope dispositions (P5 in EXP-EVAL-008; DEC-PLT-062) can cite an exact missing platform |
| Method | `research/decision-method.md` §4.1 (scope rules), §8.2 item 13, §8.4 (`PLATFORM_BLOCKED` inside scope blocks) at `14b977c`; objective §2.0 rule 4 and §4.6 |

## Author, independence and exposure

Designed by the Phase C-design evaluation-domain session. The recorded L7 exposure (EXP-EVAL-001 protocol) is irrelevant here, because only generated fixtures are used.

## Question

1. Do the comparative claims about Apple Encrypted Archive (AEA) and Apple Archive (AA) survive capability probes on a real macOS host? The claims cover authenticated encryption, encrypted names, multi-recipient and PQ support, and signatures surviving recompression.
2. On APFS, which metadata classes do Entrybound, bsdtar (macOS), `ditto`, `aa` and 7-Zip for macOS capture and restore exactly? The classes include extended attributes (`com.apple.*`), resource forks, FinderInfo, birthtime, NFSv4-style ACLs, BSD file flags and clone and sparse topology.
3. On native ARM64 (Linux and macOS Apple silicon), do the EXP-EVAL-005 speed and access verdicts keep their direction? Are REF-PROD deterministic outputs (PCR, and unencrypted artifact bytes where claimed) identical to x86-64?

## Hypotheses and falsification

- **H1 (AEA/AA, DEC-CRY-042, DEC-CRY-043, DEC-CRY-044).**
  - AEA authenticated profiles detect every single-byte ciphertext flip; this is the EXP-EVAL-006 arm A procedure.
  - AEA hides entry names.
  - AEA supports asymmetric recipients (ECDHE P-256) but no hybrid PQ recipient at the pinned macOS version.
  - AEA signed profiles do not survive re-encoding with a different compression.
  - Each part is falsified by probe outcomes. Construction-security comparisons (commitment, misuse resistance of HKDF, AES-CTR and HMAC) are routed to external review and are not decided here.
- **H2 (APFS fidelity, DEC-PLT-024, DEC-PLT-035).** On the class list fixed before results from independent observer capability (`xattr`, `ls -le@O`, `GetFileInfo`, `stat -f %B`), `ditto` and `aa` restore at least as many classes as REF-PROD on APFS. REF-PROD is expected to declare, and not silently drop, every class it does not restore. Falsified per class by `restore_fidelity_fraction`. Any silent loss is an HC-07 violation.
- **H3 (ARM64 timing direction, DEC-CMP-011, DEC-ECO-070).** Every EXP-EVAL-005 validation verdict that is `DISTINGUISHABLE` or `NON_INFERIOR` on x86-64 has the same direction on native ARM64 Linux and on macOS Apple silicon. Falsified by any opposite-direction decision-grade verdict. The claim is then scoped to x86-64.
- **H4 (ARM64 determinism).** REF-PROD PCR and unencrypted `balanced` INDEXED artifact bytes are identical between native ARM64 Linux and x86-64 WSL for the same tree. LAI and AUX may differ per findings F-0001 and F-0002 on host-dependent metadata, and are reported. Falsified by a PCR difference, which is an HC-11/HC-13 cross-architecture determinism finding.

## Decisions informed

`DEC-CRY-042`, `DEC-CRY-043`, `DEC-CRY-044`, `DEC-PLT-024`, `DEC-PLT-035`, `DEC-PLT-062`, `DEC-CMP-011`, `DEC-ECO-070`, `DEC-ACC-002`.

## Candidates

- **macOS:**
  - REF-PROD `ebound`, built for `aarch64-apple-darwin` at `9e44608` with `--locked`;
  - `aea` (profiles: symmetric key, password, ECDHE-P256, signed variants);
  - `aa archive`/`aa extract`;
  - `ditto -c -k --sequesterRsrc`;
  - bsdtar (system);
  - 7-Zip for macOS (pinned release);
  - `zstd` and `xz` (Homebrew bottles pinned by SHA-256).
- **ARM64 Linux:** REF-PROD `aarch64-unknown-linux-gnu` plus the EXP-EVAL-005 incumbent set (Ubuntu 24.04 arm64 apt versions).
- **Status quo:** the SPEC §22.2 marks as written.
- **Excluded:** QEMU-emulated ARM64 for H3, never timing (§4.1). If Docker becomes available, it may serve H4 as an `EMULATED` determinism corroboration only.

## Corpus selectors

- Generated fixtures only for H1 and H2: the EXP-EVAL-006 case lists, plus an APFS fixture builder that creates the class list above on a fresh APFS volume (`hdiutil create -fs APFS`).
- H3: the EXP-EVAL-005 validation selection rule applied to the tuning and validation splits (no held-out), copied to APFS or ext4 on the ARM64 hosts with `content_tree_sha256` verification.
- H4: the EXP-DET-SMOKE generated tree plus 10 small tuning items.

## Environment (required, currently unavailable)

- A macOS 14 or later host on Apple silicon: non-admin sufficient except `hdiutil` volume creation; AC power; exclusive window.
- A native ARM64 Linux host (for example a local ARM64 workstation or single-board server with at least 8 cores and 16 GiB): ext4, root.
- §4.2- and §4.3-style controls adapted per host: affinity by P- or E-core class on Apple silicon via `taskpolicy`, which is informational because pinning is unavailable. macOS timing is therefore `os-scheduled`-equivalent, and A/A calibration (EXP-EVAL-004 procedure) must pass per host before any timing claim.

## Commands (designed; TO BE WRITTEN when a platform exists)

```sh
# macOS
research/experiments/EXP-EVAL-013/macos/build_refprod.sh          # cargo +1.98.1 build --release --locked --target aarch64-apple-darwin
research/experiments/EXP-EVAL-013/macos/make_apfs_fixture.sh       # hdiutil APFS volume + class list
python3 research/experiments/EXP-EVAL-006/probes/tamper_campaign.py --tool aea --mode <profile> ...
python3 research/experiments/EXP-EVAL-006/probes/fidelity_matrix.py --tool {ebound,aa,ditto,bsdtar,7zz} --fixture <apfs> --platform macos
python3 research/experiments/EXP-EVAL-006/probes/signature_survival.py --system aea
# ARM64 Linux
research/experiments/EXP-EVAL-005/make_specs.py --env arm64-linux --op O1..O6 ; python -m ebr run --timing ...
research/experiments/EXP-EVAL-004/make_arm_specs.py --env arm64-linux   # calibration first
# determinism
python3 research/experiments/EXP-EVAL-013/det_compare.py --a research/raw/EXP-DET-SMOKE --b research/raw/EXP-EVAL-013/arm64
```

## Seeds

As in EXP-EVAL-004/005/006, with the constant 2,000,000 added to `order` and `bootstrap` for the new environments.

## Warmup

§4.5.

## Measurement method and metrics

`undetected_tampers`, `authenticated_coverage_fraction` (H1); `capture_fidelity_fraction`, `restore_fidelity_fraction`, `declared_gap_count`, `cross_platform_restore_fidelity_fraction` (H2); the EXP-EVAL-005 primary metrics (H3); PCR and repeat-hash equality (H4, binary). Thresholds are exactly as registered in `thresholds.json` (T-03, T-04, T-06, T-08, T-20; §3.3).

## Replication and adaptive rule

As in EXP-EVAL-004 (calibration), EXP-EVAL-005 (timing) and EXP-EVAL-006 (probes).

## Normalization

As in EXP-EVAL-005 and EXP-EVAL-006. Environments are never paired across hosts (§4.1).

## Statistical analysis

As in EXP-EVAL-005 and EXP-EVAL-006. Cross-platform claims need same-direction decision-grade support on every platform in scope (§4.1).

## Practical-significance thresholds

As registered in `thresholds.json`; no restatement.

## Sensitivity analysis

Per host, per core class (Apple silicon P or E, informational), and per macOS version where two are available.

## Expected negative results worth recording

- `ditto` and `aa` exceeding REF-PROD APFS fidelity.
- AEA matching Entrybound on tamper detection and name hiding.
- ARM64 decode direction reversals where x86-64 SIMD paths dominate an incumbent's speed.

## Consequence while BLOCKED (pre-registered)

- Every decision whose `decision_scope` includes macOS or native ARM64 stays `INSUFFICIENT_EVIDENCE` (§8.4), or excludes those platforms explicitly with a `known_limitations` entry citing this experiment (objective §4.6).
- Comparative claims naming AEA or AA are `BLOCKED:macOS` in the EXP-EVAL-012 claims table.
- The v1-scope rubric (EXP-EVAL-008 P5) cannot place any macOS or ARM64 support claim in `STABLE_V1`.
- `research/reports/remaining-unknowns` cites this experiment id with the missing platform named exactly: **"macOS 14+ on Apple silicon with APFS"**, and **"native aarch64 Linux host"**.

## Threats to validity (when unblocked)

- One Apple silicon host is not all macOS hardware.
- Rosetta must never be used; the binaries are native arm64 only.
- APFS behaviour differs between internal volumes and disk images; both are run if feasible.

## Platform requirements

macOS (Apple silicon, APFS); native ARM64 Linux; exclusive timing windows; network for tool bottles. All are unavailable, hence **BLOCKED**.

## Estimates (when unblocked)

- Machine time: about 150 h (macOS: calibration 20 h, timing 60 h, probes 10 h; ARM64 Linux: calibration 20 h, timing 40 h).
- Disk: about 150 GiB across hosts.
- Agent effort: scripted, with judgment for the claims tables.
