# EXP-ECO-014: Ecosystem tree-digest and normalization interoperability

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-014 |
| Domain | ecosystem (program §32; cross-cuts §7 identity model, §27 export normalization) |
| Status | READY (reference implementations are public downloads or packages; scripts specified below) |
| Runner | `spec.yaml` (digest producer × tuning item, including rebuild-like variants generated inside each sample); the perturbation sensitivity matrix is a script |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |
| Timing | descriptive digest-computation cost only (not decision-grade) |

## 1. Question

How do Entrybound's minted identities (LAI under identity/v1, AUX) relate to the tree digests other ecosystems already verify — Go module `h1:` (dirhash Hash1), Git tree object IDs, Nix NAR hashes, Software Heritage directory SWHIDs, and in-toto/SLSA subject digests over archives — in which semantic facts each covers, which perturbations change each, what each costs to compute, and whether identity/v1 equality matches the equality notion of reproducible-build comparison use cases; and how do Entrybound's export normalizations compare with the normalization dimensions applied by the OSS Rebuild `stabilize` tool?

## 2. Hypotheses and falsification

- **H1 (sensitivity differs).** For at least three pre-registered perturbations, LAI and at least one ecosystem digest disagree on whether the tree changed (for example: mtime-only changes, empty directories, file mode bits other than executable, extended attributes, Unicode normalization form of names). *Falsified* if disagreements occur on fewer than three perturbations.
- **H2 (no single digest replaces LAI).** No ecosystem digest changes on exactly the same perturbation set as LAI. *Falsified* if one does (then `replace-lai-with-dirhash1`-style candidates are not excluded by semantics).
- **H3 (reproducible-build equality).** For the pre-registered rebuild-variant use cases, identity/v1 LAI equality agrees with the expected "same content for reproducible-build comparison" verdict in at least 90% of cases, and every disagreement is due to a metadata class the use case ignores. *Falsified* if agreement is below 90% or a disagreement is due to content.
- **H4 (normalization parity).** `stabilize` applies at least one normalization dimension to ZIP or tar artifacts that no frozen Entrybound export profile applies, or vice versa. *Falsified* if the dimension sets are identical.

## 3. Decisions informed

DEC-MOD-017, DEC-MOD-005, DEC-ECO-072 (which digests can serve as in-toto subjects), DEC-LEG-014, DEC-ECO-065 (Git tree identity versus `.eb` identity for source releases).

## 4. Candidates

| Decision | Candidates evaluated |
|---|---|
| DEC-MOD-017 | minted-lai-only-status-quo; emit-dirhash1; emit-multiple-tree-digests (dirhash1, Git tree, NAR hash, SWHID directory); replace-lai-with-dirhash1; document-divergence-only (measured as the divergence table itself) |
| DEC-MOD-005 | identity-profiles-only-status-quo (identity/v1 as implemented); named-normalization-profiles, sde-clamp-at-pack, order-preserving-time-normalization and export-only-normalization evaluated by applying their stated normalization to the tree before packing (emulation by a pre-pack normalizer script per candidate, committed before any run) and recomputing identities; validator-keyword-vocabulary emulated with BSD `mtree` specifications (keyword set per candidate) |
| DEC-LEG-014 | status-quo-profile-id-is-rule-set; stabilize-dimension-parity (dimension matrix); explicit-rule-set-version and disclose-rewrite-in-receipts (receipt field presence check) |
| DEC-ECO-065 | git-archive-integration: Git tree ID of the tag versus LAI of the exported tree |

Digest producers (reference implementations, pinned): Go `golang.org/x/mod/sumdb/dirhash` Hash1 via a 20-line Go program built with a pinned Go toolchain; `git write-tree` in a temporary repository with `core.fileMode=true`; `nix-hash --type sha256 --base32` or `nix hash path` from a pinned static Nix binary run without a daemon (`--store` in scratch); Software Heritage `swh.model` (pinned PyPI version) directory SWHID; SHA-256 of canonical `tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner` output (the common in-toto subject practice); Entrybound LAI and AUX from `ebound inspect --json`.

Excluded: OCI layer `diffID` as a *tree* digest (it is the digest of a tar serialization, covered by the canonical-tar producer); Bazel remote-execution Directory digests (no stable command-line reference implementation in scope).

## 5. Corpus

- Tuning items: `f01-tuning-ripgrep-14-1-1-git`, `f01-tuning-zstd-v1-5-7-git`, `f02-tuning-lz4-intree-make-build`, `f04-tuning-sourcelike`, `f18-tuning-lz4-releases-1-9-to-1-10`, `f19-tuning-zoo`, `f19-tuning-real-home-snapshot`, `f19-tuning-real-ext4-acl-selinux` (loop image mounted read-only for ACL and SELinux labels).
- Generated perturbation base: `perturb_base.py --seed 2026091714` builds a 200-file tree with files, empty directories, symlinks (relative, absolute, dangling), a hardlink pair, executable and non-executable modes, xattrs and a POSIX ACL (on ext4 under `/root/eb-research`).
- Perturbations (pre-registered, applied one at a time to copies of the base and of each tuning item where applicable): P01 mtime of one file; P02 atime only; P03 mode 0644→0664; P04 mode 0644→0755; P05 uid/gid change (root); P06 add empty directory; P07 remove empty directory; P08 symlink target change; P09 symlink replaced by its target file; P10 hardlink pair broken into two identical files; P11 add xattr `user.test`; P12 change ACL entry; P13 file content one byte; P14 rename file (same content); P15 NFC→NFD name; P16 case-only rename; P17 directory mtime; P18 sparse versus dense file with identical content; P19 file order of creation; P20 umask at creation; P21 CRLF→LF in a text file; P22 add `.gitignore`d file; P23 SELinux label change; P24 file capability `security.capability`; P25 non-UTF-8 byte name.
- Reproducible-build use cases (pre-registered expected verdicts from reproducible-builds.org documentation, cited with access date): RB1 rebuild with different timestamps (expected same); RB2 different build path embedded only in metadata (expected same); RB3 different umask (expected same for permission bits beyond executable); RB4 different file order (expected same); RB5 different uid/gid (expected same); RB6 content difference (expected different).
- Validation look: `f01-validation-redis-7-2-16-git`, `f02-validation-cjson-cmake-build`, `f04-validation-objstore`, `f18-validation-cjson-releases`, `f19-validation-deep`, `f19-validation-real-xfs-acl-selinux`.

## 6. Environment and platform requirements

WSL2 Ubuntu 24.04 root on ext4 under `/root/eb-research` (never `/mnt/d`, which lacks metadata); XFS item via read-only loop mount; pinned Go toolchain, static Nix binary, `swh.model`, `stabilize` built from the OSS Rebuild repository at a pinned commit, BSD `mtree` (apt `mtree-netbsd`), `diffoscope` (apt) for RB checks. Not privileged beyond the research VM root already used for corpus images.

## 7. Procedure and commands

Scripts in `research/tools/ecosystem/digests/` (to be written):

1. `digests.py --tree <path>` — compute every producer's digest; print one JSON line with digests, per-producer wall time (descriptive) and warnings (e.g. producer refused non-UTF-8 names).
2. `perturb_matrix.py` — for each (base or tuning item, perturbation), compute all digests before and after; cell = changed / unchanged / producer-error.
3. `rb_usecases.py` — generate RB1-RB6 variant pairs for each tuning item; compare LAI equality and each digest's equality with the pre-registered expected verdict.
4. `normalizers/<candidate>.py` — pre-pack normalizers for DEC-MOD-005 candidates; `identities_under_normalization.py` recomputes LAI and AUX after each normalizer over the RB variants.
5. `stabilize_parity.py` — run `stabilize` on the six frozen export profiles' outputs of each tuning item and on incumbent `zip -X` and GNU tar archives; diff inputs and outputs to enumerate the dimensions `stabilize` changed; compare with the export profile rule documentation (`docs/legacy-export-v1.md`, `docs/compressed-tar-export-v1.md`) by dimension identifier.
6. `ebr run --spec spec.yaml`; `verify-raw`; `normalize`; `matrix.py` writes `research/normalized/EXP-ECO-014/{sensitivity,rb-agreement,stabilize-dimensions,coverage}.csv`.

## 8. Seeds, warmup, replication

Seeds `order` 2026091714, `bootstrap` 2026091714, `command` 2026091714; perturbation base seed 2026091714. Digests are deterministic: 2 repetitions per (producer, item) + repeat-hash; perturbation matrix computed twice. No warmups; wall times are descriptive only.

## 9. Metrics and measurement

| Metric | Canonical | Role |
|---|---|---|
| `cross_host_identity_agreement` | yes (HC-08) | not measured here (EXP-ECO-004) |
| `equivalence_assertion_pass_fraction` | yes (HC-01/HC-03, M01.3) | binary: LAI equal across INDEXED and STREAM packs of every perturbed tree (a consistency guard) |
| `digest_sensitivity_cell`, `rb_usecase_agreement_fraction`, `semantic_facts_covered`, `digest_wall_s`, `stabilize_dimensions` | no | descriptive record fields |

## 10. Normalization and aggregation

AGG-P per digest producer for the sensitivity matrix (perturbation × producer); AGG-C per family for RB use-case agreement (minimum over families headline).

## 11. Statistical analysis and use in the decision rule

Deterministic facts, no inference. For DEC-MOD-017, a candidate that replaces LAI with a digest whose sensitivity set lacks a fact that SPEC §8.1 assigns to LAI is excluded at R1 (HC-10) with the perturbation as counterexample; emitting additional digests is costed (C1 wire items and normative words if stored; C2 code; per-pack compute as descriptive). For DEC-MOD-005 the agreement fractions are descriptive inputs; any normalizer that changes content bytes (P21 excepted by design) violates HC-02 and is excluded.

## 12. Practical-significance thresholds

None applied (binary and descriptive roles, Appendix A.2). The 90% figure in H3 is a hypothesis statement, not a decision threshold.

## 13. Sensitivity analysis

Git with `core.fileMode=false`; Nix NAR with and without executable-bit canonicalization; SWHID with and without symlink following; digests computed on the extracted `.eb` tree instead of the source tree (round-trip consistency).

## 14. Held-out (Phase D placeholder)

Conventions §8: the sensitivity and RB matrices run once on held-out F01/F02/F19 items after unlock.

## 15. Expected negative results worth recording

LAI insensitive to perturbations that downstream verifiers treat as significant, or sensitive to ones they ignore; producers failing on non-UTF-8 names; `stabilize` dimensions absent from Entrybound profiles.

## 16. Threats to validity

Reference implementations evolve; digests over trees depend on how the tree was materialized (ext4 versus other filesystems); reproducible-build expected verdicts are documentation readings (`EXTERNALLY_SOURCED`), reviewed by a second session.

## 17. Cost estimate

About 6 h compute; about 20 GB disk; agent effort scripted plus judgment for the RB expected-verdict table and the stabilize dimension mapping (one session plus review).

## 18. Gates and exposure

Conventions §1 and §2. F04 is in the exposure record; analysis by an unexposed session.
