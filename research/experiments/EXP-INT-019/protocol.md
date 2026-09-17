# EXP-INT-019: macOS/APFS and native ARM64 integrity arms (BLOCKED)

| Field | Value |
|---|---|
| Status | **BLOCKED** (PLATFORM_BLOCKED) |
| Blocked reason | No macOS host and no native ARM64 host are available; hosted runners were declined by the program owner (PROGRESS standing decision 2026-09-12). APFS, HFS+, Finder copy, extended-attribute quarantine flags, `renamex_np`/`RENAME_SWAP`, `F_FULLFSYNC`, `clonefile`, and native ARM64 decode paths cannot be observed here. Emulated ARM64 (qemu user mode) supports correctness only and is used in EXP-INT-014 and EXP-INT-017. |
| Unblocks when | a macOS host on Apple silicon (covers both APFS and native ARM64) or separate macOS and Linux ARM64 hosts are made available |
| Domain | integrity (program §23, §24, cross-reference platform §15-§21) |
| Decisions informed | DEC-INT-034, DEC-INT-003, DEC-INT-039, DEC-LEG-056, DEC-INT-013, DEC-INT-046, DEC-INT-030 |
| Gates, method, author | conventions C0 |
| Spec | `spec.yaml` (runner-driven when the platform exists; `platform.os: [darwin]`, `arch: [arm64]`) |

## 1. Question

On macOS APFS and native ARM64, do (a) salvage unverified markers and sidecar naming survive
Finder-equivalent and command-line copies (`cp -p`, `ditto`, `rsync` from Homebrew, `xattr`
quarantine propagation, APFS clones); (b) the output transaction patterns of EXP-INT-005 remain
free of MVT-18(a) violations under process kill with `renamex_np` and `F_FULLFSYNC`; (c) atomic
tree moves for verified extraction behave as on NTFS and ext4; (d) sidecar creation under
concurrent modification binds one coherent version; (e) locators with Unicode normalization and
case-insensitive aliases stay confined; and (f) re-planning and reconstruction decoding reproduce
the same bytes natively on ARM64?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-INT-034 | xattr markers survive `cp -p` and `ditto` but not zip or HTTP, as on Linux; APFS clones preserve markers. | Marker lost by `ditto`. |
| H2 | DEC-INT-003, DEC-INT-039 | No MVT-18(a) violation for rename-based patterns under kill; `RENAME_SWAP` gives atomic in-place replacement. | Any violation. |
| H3 | DEC-INT-013, DEC-LEG-056 | NFD/NFC aliases on APFS are handled as distinct locators by confined resolvers (no silent merge). | A silent merge or escape. |
| H4 | DEC-INT-046, DEC-INT-030 | Native ARM64 PCR and reconstruction outputs equal x86_64 outputs (HC-11). | Any difference (HC-11 violation artifact). |

## 3. Candidates

As EXP-INT-011 (DEC-INT-034 carriers), EXP-INT-005 (DEC-INT-003 patterns, DEC-INT-039 policies),
EXP-INT-008 (DEC-INT-013 locator policies, DEC-LEG-056 snapshot candidates), EXP-INT-009
(DEC-INT-046 re-plan) and EXP-INT-014 (DEC-INT-030 vectors), with the same exclusions.

## 4. Corpus

Tuning: `f05-tuning-gutenberg-books`, `f19-tuning-zoo`, `f14-apache-maven-3916-bin-tarball`,
`f12-tuning-kodak-jpeg-edge-cases-small`, `f01-tuning-ripgrep-14-1-1-git`; validation items as in the
referenced experiments. Items mirrored with `logical_tree_sha256` verification.

## 5. Environment and platform requirements

macOS 15 or later on Apple silicon, APFS (case-insensitive default and a case-sensitive volume),
Xcode command-line tools, Rust 1.98.1, Python 3.13, Homebrew rsync and par2; no admin needed for most
arms (APFS volume creation with `diskutil apfs addVolume` needs admin and is optional).

## 6. Procedure and commands (designed)

Build B-PROD for `aarch64-apple-darwin`; run `spec.yaml` with ebr on the macOS host; compare vectors
and PCR with the WSL and Windows results of EXP-INT-009 and EXP-INT-014.

## 7. Seeds, warmups, replication

Seeds reserved: 2026091891/92/93. 2 repetitions with repeat-hash, as the referenced experiments.

## 8. Metrics

`carrier_survival_fraction` (C11), `mvt18a_violations` (C11), `confinement_escapes` (C11),
`reader_disagreements` (HC-03), `cross_host_identity_agreement` (HC-08), `unsafe_defaults` (HC-05).

## 9. Normalization

Exact tallies per arm; macOS is an evaluation condition (platform pair), never pooled.

## 10. Statistical analysis and sensitivity

Exact tallies; R1 on binaries; sensitivity by APFS case sensitivity.

## 11. Expected negative results worth recording

Quarantine xattrs propagate unexpectedly to extracted salvage output; Finder copies drop custom
xattrs in some cases.

## 12. Threats to validity

macOS version-specific behaviour; results do not transfer to HFS+ without a separate arm.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12.

## 14. Cost estimates

About 20 CPU-hours on the macOS host; disk about 30 GB; agent effort: setup judgment, execution scripted.

## 15. Deviations (append-only)

None.
