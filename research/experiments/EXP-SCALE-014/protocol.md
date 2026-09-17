# EXP-SCALE-014: Emulated cross-architecture, CPU-feature-level and overlay-filesystem determinism without Docker

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** |
| Domain | scale (program §14; interfaces with platform §15 and determinism scope §7) |
| Platforms | Linux x86-64 WSL2 with `qemu-user` (EMULATED linux/arm64 and restricted x86-64 CPU models); overlayfs mounted as root |
| Requires timing | false (QEMU is never used for timing, §4.1) |
| Compute estimate | about 40 machine-hours (user-mode emulation is 5-30× slower) |
| Disk estimate | about 25 GB |
| Agent effort | scripted |
| Tooling to build | `apt-get install qemu-user gcc-aarch64-linux-gnu libc6-dev-arm64-cross` (approved downloads; versions recorded); `rustup target add aarch64-unknown-linux-gnu --toolchain 1.98.1`; `.cargo/config.toml` linker setting in a research-only build directory (not the production workspace config); `ebound` cross-built for aarch64 (`/root/eb-research/target/scale-arm64/aarch64-unknown-linux-gnu/release/ebound`) and `ebr-par` identity-passing prototypes from EXP-SCALE-012; `research/tools/scale/det_cell.py --runner qemu-aarch64|qemu-x86_64-<model>|overlay` extension of EXP-SCALE-005's driver |

## Pre-registration

- **decision_ids:** DEC-MOD-011, DEC-MOD-013, DEC-CMP-021, DEC-CMP-019, DEC-ACC-031, DEC-CMP-001.
- **Decision type:** EMPIRICAL (binary identity), qualifier `EMULATED`.
- **od_ids / hc_ids:** PENDING-G-A. Proposal: HC-11 (MVT-11(a) CPU-model variation: identical decoded output and outcome), HC-13 (MVT-13(a) host classes QEMU arm64 and Docker-overlay equivalent), OD-18 M18.1.
- **Method, gate, author, L7:** as EXP-SCALE-001; L7 note of EXP-SCALE-003 applies (tuning/validation items).

## Question

Are pack output bytes and identities, and decode output and outcomes, identical between native x86-64, emulated aarch64 and x86-64 restricted to older CPU feature levels, including dictionary-bearing profiles and identity-passing parallel prototypes; and does capture from an overlay filesystem (the Docker overlay host class, reproduced without Docker) change identities?

## Hypotheses

- **H1:** PCI and LAI of unencrypted `--deterministic`-equivalent packs are identical across native x86-64, `qemu-aarch64`, and `qemu-x86_64 -cpu` {`qemu64` (x86-64-v1), `Nehalem-v2` (v2), `Haswell-v4` (v3), `Icelake-Server` (v4)} for every item and profile (SHA-256 and codec SIMD dispatch must not change output); decode outputs and outcome codes are identical under every CPU model; overlayfs capture yields the same LAI as ext4 capture of the same tree with AUX differences explained by FidelityReport.
- **H0 / falsification:** any difference is a two-output artifact (objective §2.0 rule 2).

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-MOD-011 | Emulated ARM64 byte comparison of pack output over the corpus | native ARM64 and macOS: EXP-SCALE-015 (blocked) |
| DEC-MOD-013 | Parallel prototypes' identity under emulated ARM64 scheduling | — |
| DEC-CMP-021 | Byte identity of trained dictionaries and archives across x86-64 feature levels and emulated ARM64 | libzstd release drift: EXP-SCALE-011 |
| DEC-CMP-019 | Reproducibility matrix extended to architectures | — |
| DEC-ACC-031 | Byte-identical archives and identities across OSes and emulated arm64 for parallel hashing and CDC prototypes | speedups: EXP-SCALE-013 |
| DEC-CMP-001 | Scalar vs accelerated boundary and hash identity across ISAs (gear-norm-v1 is scalar; SHA-256 dispatch differs by CPU model) | SIMD chunker candidates: chunking domain |

## Candidates

Runners (each an ebr candidate × config `bi`, `di`, `xi`, `bs` as EXP-SCALE-005): `native` (reference), `qemu-aarch64`, `qemu-x86_64-qemu64`, `qemu-x86_64-Nehalem-v2`, `qemu-x86_64-Haswell-v4`, `qemu-x86_64-Icelake-Server`, `overlay-capture` (item copied into an overlay lower directory, packed from the merged mount), plus `decode-<runner>` candidates that unpack the native reference archive under each runner and compare the content digest and outcome, and `par-<design>-<runner>` for identity-passing EXP-SCALE-012 prototypes at workers {1, 8}.

**Excluded:** Docker-based host classes (Docker unavailable; the overlay mount reproduces the capture layer, and cgroup caps reproduce memory limits, EXP-SCALE-005); timing of any kind; encrypted packs (nondeterministic by design; LAI/AUX equality under emulation is covered by `enc` variants only if EXP-SCALE-005 finds them stable natively).

## Corpus selectors

Manifest `research/corpus/manifest.json`, splits `tuning` and `validation`, small items only (59 items, EXP-SCALE-005 selection) for emulated runners; plus two medium dense-profile items per family with dictionaries (F01, F03, F05) under `qemu-aarch64` only. **Held-out placeholder (Phase D):** re-run of `qemu-aarch64` and `qemu-x86_64-qemu64` on held-out small items after unlock (§5.5).

## Environment

`env_id` `wsl-ubuntu` with QEMU user-mode (`EMULATED`); binfmt is not required (runners invoke `qemu-aarch64 -L /usr/aarch64-linux-gnu` explicitly). Overlayfs via `mount -t overlay overlay -o lowerdir=L,upperdir=U,workdir=W M` on ext4. `LANG=C.UTF-8`, `TZ=UTC`.

## Commands

```sh
apt-get install -y qemu-user gcc-aarch64-linux-gnu libc6-dev-arm64-cross   # records versions to research/environment
rustup toolchain install 1.98.1 && rustup target add aarch64-unknown-linux-gnu --toolchain 1.98.1
cd /root/eb-research/src/entrybound && CARGO_TARGET_DIR=/root/eb-research/target/scale-arm64 \
  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
  /root/.cargo/bin/cargo +1.98.1 build --release --locked -p entrybound-cli --target aarch64-unknown-linux-gnu
cd /mnt/d/Projects/entrybound/entrybound && PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-SCALE-014/spec.yaml
/root/eb-research/venv/bin/python research/tools/scale/det_compare.py --raw research/raw/EXP-SCALE-014 --reference research/raw/EXP-SCALE-005 \
   --out research/normalized/EXP-SCALE-014/decision/emulated-identity.csv --artifacts research/raw/EXP-SCALE-014/violations
```

## Seeds

`seeds.order` 1314001, `seeds.bootstrap` 1314002, `seeds.command` 1314003.

## Warmup

0.

## Measurement method and metrics

`output_sha256`, `lai`, `pcr`, `aux`, `pci` (binary comparisons vs native), `decode_content_sha256_equal`, `decode_outcome_equal` (MVT-11(a)), `cpu_model_verified` (the runner prints `/proc/cpuinfo` flags seen by the guest; a model whose flags do not match its definition invalidates the sample, MVT-11(a) verified variation), `cross_host_identity_agreement` (M18.1).

## Replication

3 repetitions per cell (MVT-13(a)); no stress cell under emulation (the native stress cell is EXP-SCALE-005's).

## Normalization and statistical analysis

Binary identity per (item, config, runner) against native; violations committed as artifacts; counts only.

## Practical-significance thresholds

None (binary oracles, §3.3).

## Sensitivity analysis

Profile arm; item-family arm; CPU-model ladder (a difference appearing only below a feature level identifies the dispatching dependency).

## Expected negative results worth recording

A codec or trainer whose output changes with CPU feature level; overlay capture changing AUX (for example inode-derived hard-link grouping); a prototype whose identity holds natively but not under emulation (schedule-dependent).

## Threats to validity

QEMU user-mode emulates the CPU but not the kernel (host 5.15 kernel syscalls); ISA coverage of emulation may hide JIT/SIMD paths that real hardware would take (native ARM64 remains EXP-SCALE-015); `-cpu` models may still expose host features through CPUID leaves QEMU passes through (checked by `cpu_model_verified`).

## Platform requirements

Linux x86-64 WSL2 root; packages above; about 25 GB; no Docker; no network after installation.

## Estimated compute and effort

About 40 h (emulation overhead dominates); scripted.

## Deviations (append-only)

None.
