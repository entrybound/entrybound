# EXP-PLAT-024: Container-runtime capture and extraction semantics (rootless podman/crun route; Docker-parity arm blocked)

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **NEEDS_TOOLING** — Docker-parity arm BLOCKED: Docker Desktop fails to start on this host (stale AF_UNIX socket reparse points, Win32 error 1920; PROGRESS storage incident notes), so the moby default seccomp profile, userns-remap and Docker overlay upperdir cells cannot run; the podman/crun route in WSL is designed as the executable substitute and needs tooling. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-032, DEC-PLT-005, DEC-PLT-004, DEC-PLT-046, DEC-PLT-017, DEC-MOD-002, DEC-PLT-035, DEC-PLT-062
- **Program sections:** 15, 18, 19, 21
- **Decision type (proposed, not assigned):** EMPIRICAL; the Docker-parity arm is PLATFORM/ACCESS-blocked. Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03, OD-18, OD-25; HC-05, HC-07, HC-13 (container host class for MVT-13(a)).
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** Inside real OCI runtimes (rootless and rootful podman with crun; Docker when available), what ownership and xattrs does Entrybound capture from container filesystems (including idmapped and overlay upperdirs), which containment mechanisms (openat2 under the runtime's default seccomp profile, /proc masking, capability bounding set) are available to extraction, how are OCI whiteouts and opaque directories represented, and do extraction outcomes match containerd/umoci-style layer application?
- **H1:** Rootless capture sees subordinate-id-shifted ownership and cannot restore devices; the default seccomp profiles allow openat2 on current runtimes (so status-quo confinement is kernel-enforced) but an older-profile variant returning EPERM exercises the fallback; overlay upperdirs expose whiteouts as char 0/0 and opaque directories as trusted.overlay.opaque, which status-quo pack refuses or records differently from OCI `.wh.` markers.
- **H0 / equivalence:** Container runtimes do not change any capture or extraction outcome relative to WSL root and user-namespace results (EXP-PLAT-007/008/009/014).
- **Falsification of H1:** No outcome differs from the WSL namespace results.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-032 | V1_IMPORTANT | How is ownership represented and restored: numeric uid/gid only, plus user/group names, Windows owner/group SIDs, idmap or user-namespace context; does it live in port... | Rootless and rootful container capture/restore of ownership; podman userns=auto and idmapped volumes. | Docker rootless/userns-remap and NFS idmap (BLOCKED arm / EXP-PLAT-011 knfsd). |
| DEC-PLT-005 | V1_BLOCKING | Which containment primitive does Entrybound extraction (and capture) use on Linux, macOS and Windows, given cap-std 4.0.3 uses openat2 RESOLVE_BENEATH only on Linux (w... | Mechanism table under container default seccomp profiles and capability sets. | Docker moby profile (BLOCKED arm); Android (not reachable). |
| DEC-PLT-004 | V1_BLOCKING | How is the achieved confinement mode detected and reported, and must a weaker-than-kernel mode refuse, require explicit opt-in, or proceed with a printed report? | Mode detection and reporting under runtime seccomp profiles including an EPERM-for-openat2 profile. | Docker (BLOCKED arm). |
| DEC-PLT-046 | V1_BLOCKING | How are character/block devices, FIFOs and sockets handled in v1: first-class EntryKinds (SPEC §3.3), auxiliary snapshot metadata, FidelityReport-only omission with en... | OCI whiteout and opaque-directory representation in overlay upperdirs versus layer tars; extraction semantics versus umoci/containerd-style application. | Docker overlay2 driver (BLOCKED arm). |
| DEC-PLT-017 | V1_BLOCKING | Which handle-relative, no-follow operations must capture and extraction use for directory, file, symlink and special-file creation and metadata (mode, owner, xattrs, t... | Behaviour in minimal containers without /proc and with masked /proc paths. | API coverage (EXP-PLAT-016). |
| DEC-MOD-002 | V1_IMPORTANT | Should AUX avoid host-dependent inputs (sparse maps from allocation state, fidelity platform/filesystem strings, Windows creation times) in deterministic mode, and sho... | AUX host dependence for captures inside overlay and fuse-overlayfs filesystems. | Other hosts (EXP-PLAT-005). |
| DEC-PLT-035 | V1_BLOCKING | Which metadata classes does stable v1 guarantee to capture and restore on each supported OS/filesystem (Linux ext4/XFS/btrfs/tmpfs, Windows NTFS/exFAT/ReFS, macOS APFS... | Container (userns) column of the per-class fidelity matrix. | Other platforms (EXP-PLAT-004). |
| DEC-PLT-062 | V1_BLOCKING | How does stable v1 disposition capabilities that cannot be evaluated locally (macOS/APFS metadata, native ARM64 performance, ReFS, admin-only Windows operations): keep... | Disposition input: which container evidence is obtainable without Docker. | Docker parity (BLOCKED arm). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production `ebound` inside the container (bind-mounted binary), versus the WSL root/user-namespace results.
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-032** — candidates: `status-quo-numeric-aux-ignore`, `add-names-aux`, `names-with-map-by-name-restore`, `sids-via-security-descriptor`, `idmap-context-recording`, `ownership-in-strict-identity`, `override-ids-on-pack`, `remove-ownership` (status quo: `status-quo-numeric-aux-ignore`). Treatment: Measured under the container runtime with the treatment of the WSL counterpart experiment.
- **DEC-PLT-005** — candidates: `cap-std-4-status-quo`, `rustix-openat2-in-root-no-symlinks`, `manual-openat-nofollow-walk`, `macos-o-resolve-beneath`, `windows-ntcreatefile-reparse-walk`, `mount-namespace-or-chroot-sandbox`, `landlock-seccomp-complement`, `refuse-unsupported-platforms` (status quo: `cap-std-4-status-quo`). Treatment: Measured under the container runtime with the treatment of the WSL counterpart experiment.
  - excluded before execution (ledger): `string-prefix-validation` — SPEC §11.6 Pattern 1 forbids string validation followed by ordinary file operations; R1 and R2 classify prefix checking as a defect class (violates SPEC §11.6 L1413 frozen extraction doctrine); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-004** — candidates: `hardcoded-kernel-enforced-status-quo`, `runtime-detect-and-report`, `weak-mode-requires-opt-in`, `refuse-weak-platforms`, `static-platform-table` (status quo: `hardcoded-kernel-enforced-status-quo`). Treatment: Measured under the container runtime with the treatment of the WSL counterpart experiment.
  - excluded before execution (ledger): `silent-weaker-mode` — SPEC §11.6 L1436 says silence about a weaker confinement mode is an I25 violation (violates I25); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-046** — candidates: `status-quo-refuse-pack`, `omit-and-report-entry-scoped`, `first-class-kinds-spec`, `kinds-without-socket`, `aux-snapshot-metadata`, `policy-selectable-refuse-or-omit`, `post-v1-feature-bit` (status quo: `status-quo-refuse-pack`). Treatment: Measured under the container runtime with the treatment of the WSL counterpart experiment.
- **DEC-PLT-017** — candidates: `status-quo-mixed`, `rustix-at-nofollow`, `o-path-proc-self-fd-bridge`, `cap-std-ext-apis`, `audited-unsafe-ffi-boundary`, `decline-and-report-only`, `status-quo`, `handle-based-capture`, `kernel-resolve-beneath-macos`, `landlock-sandboxed-extraction`, `declare-concurrent-mutation-out-of-scope`, `private-mount-destination` (status quo: `status-quo-mixed`, `status-quo`). Treatment: Measured under the container runtime with the treatment of the WSL counterpart experiment.
  - excluded before execution (ledger): `path-based-with-checks` — SPEC §11.6 Pattern 2 forbids setting metadata by path; silently doing so is a conformance violation (violates SPEC §11.6 L1424-1426 frozen extraction pattern); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-MOD-002** — candidates: `status-quo-capture-all`, `deterministic-mode-normalise`, `sparse-maps-excluded`, `structured-fidelity-vocabulary`, `reject-duplicates` (status quo: `status-quo-capture-all`). Treatment: Measured under the container runtime with the treatment of the WSL counterpart experiment.
- **DEC-PLT-035** — candidates: `status-quo-bootstrap-matrix`, `status-quo-plus-declared-gaps`, `tar-pax-parity-target`, `apple-archive-parity-macos`, `wim-robocopy-parity-windows`, `minimal-portable-guarantee` (status quo: `status-quo-bootstrap-matrix`, `status-quo-plus-declared-gaps`). Treatment: Measured under the container runtime with the treatment of the WSL counterpart experiment.
- **DEC-PLT-062** — candidates: `status-quo-compiled-untested-plus-unavailable-declarations`, `exclude-blocked-platform-claims`, `experimental-tier-per-platform`, `declare-unavailable-until-tested`, `emulated-correctness-evidence-only`, `documentation-and-third-party-evidence`, `community-contributor-runs`, `acquire-hardware-or-admin-session` (status quo: `status-quo-compiled-untested-plus-unavailable-declarations`). Treatment: Measured under the container runtime with the treatment of the WSL counterpart experiment.
  - excluded before execution (ledger): `hosted-ci-runners` — The program owner declined hosted runners; platform experiments are local and Docker only (violates PROGRESS standing decision 2026-09-12 (no GitHub Actions or hosted runners)); L0/R1(b) counterexample and reviewer sign-off still required (R1).

## Corpus, fixtures and case sets

- **Images:** OCI layouts from `f16-oci-ubuntu-noble-multiarch-index` (tuning) and `f16-oci-fedora-44-zstd-layers` (validation), loaded locally.
- **Fixtures:** EXP-PLAT-007/008/009 fixtures (tuning 20260917, validation 20260918) bind-mounted into containers.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_id:** new `wsl-podman` capture (runtime versions, seccomp profile hashes, storage driver). Docker arm: `docker-ubuntu-24.04` (BLOCKED).

| Requirement | Needed? |
|---|---|
| Linux (WSL2, rootless and root) | yes |
| Windows | no |
| macOS | no |
| x86-64 | yes |
| ARM64 | no |
| Network | apt packages only (images loaded from the corpus) |
| Privileged | root for rootful arms; subordinate ids for rootless |
| Large storage | about 25 GB container storage on WSL ext4 |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Linux):** `rm -rf /root/eb-research/src/entrybound-plat && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/entrybound-plat && git -C /root/eb-research/src/entrybound-plat checkout <record-commit> && cd /root/eb-research/src/entrybound-plat && CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound` → `/root/eb-research/target/production/release/ebound` (production workspace, no `research-internals`; harness review R1-06).
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-024/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-024/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-024/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-024/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-024/spec.yaml'`
- **Scripts:** `scripts/container_arms.sh --runtime <podman-rootless|podman-rootful|docker> --arm <ownership|confinement|whiteouts|noproc> --item <path> --item-id <id> --scratch <dir>`; `seccomp_profiles/` holds the runtime default profile (copied from the installed containers-common package, hashed) and an EPERM-for-openat2 variant; `oci_apply_compare.sh` compares extraction trees with `umoci unpack` output (umoci pinned download).
- **Docker-parity (when available, currently BLOCKED):** `research/experiments/EXP-PLAT-024/spec-docker.yaml` driving `scripts/docker/container_arms.sh` with identical arms using `docker run --security-opt seccomp=<moby default>`.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

As in EXP-PLAT-007/008/009/014, observed inside and outside the container.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `capture_fidelity_fraction` | OD-03 | primary | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | observers |
| `restore_fidelity_fraction` | OD-03 | primary | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | observers |
| `hc05_mvt05_escapes` | OD-25 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-05 MVT-05(a)/(b) | confinement arm |
| `hc15_confinement_misreports` | OD-25 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-05/HC-15 (SPEC §11.6 L1436 reporting) | confinement arm |
| `unsafe_defaults` | OD-25 | binary screen | correctness | count | equal_zero | HC-05 (`metric_thresholds.unsafe_defaults`; A.2 role `binary`) | ownership/special arms |

## Normalization

As in the counterparts.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Runtime condition:** podman-rootless, podman-rootful and docker are separate evaluation conditions; podman results never substitute for Docker in a scope that names Docker.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- If podman results match WSL namespace results everywhere, container-specific decision scope adds nothing — a null result worth recording.

## Threats to validity

- crun and containers-common differ from runc and the moby seccomp profile; conclusions about Docker specifically remain blocked.
- Rootless networking and fuse-overlayfs can differ from kernel overlay; both recorded.

## Estimated cost and effort

- **Compute:** about 8 machine-hours; **disk:** about 25 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** apt: podman crun uidmap fuse-overlayfs slirp4netns (approved downloads); pinned OCI images from the F16 tuning/validation layouts loaded with `podman load`/`skopeo copy` (no registry pulls at run time); research/experiments/EXP-PLAT-024/scripts/{container_arms.sh,seccomp_profiles/,oci_apply_compare.sh} (mechanical); Docker-parity variants `scripts/docker/*.sh` kept for when Docker is available; a non-root WSL user with subuid/subgid ranges for rootless arms (created by the execution session; recorded in the environment capture)
- **Agent effort:** tooling scripted (package install and user setup); execution scripted; judgment for runtime-difference interpretation (crun/containers-common seccomp versus moby).

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
