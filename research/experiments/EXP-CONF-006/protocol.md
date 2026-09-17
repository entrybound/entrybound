# EXP-CONF-006 — Historical-generation stored-bytes corpus and frozen-identifier decode regression

| Field | Value |
|---|---|
| Status | `NEEDS_TOOLING` (CC§2): generation-commit finder, per-commit builder (WSL and Windows), historical pack-and-check driver, repack identity checker, coverage-based retained-decode-surface counter |
| Domain / program sections | conformance / §29 (versioned corpus including historical generations), §28 |
| Decision IDs informed | DEC-CON-013, DEC-CMP-025, DEC-MOD-008, DEC-CON-012, DEC-ECO-042 |
| Requirements | REQ-ECO-0136; CONTRIBUTING freezes F-03, F-04, F-06, F-15, F-17, F-22, F-26 (objective Appendix B) |
| HC oracles | HC-16 MVT-16(a) (golden regression), MVT-16(d) (frozen-ID differential); HC-02 MVT-02(a) on historical archives; F-08 identity tiers on repack; proposal for the assigner: `hc_ids` HC-16, HC-02, HC-10; `od_ids` OD-20, OD-22 |
| Decision type | not assigned (gate G-A) |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1 |
| Depends on | none (uses repository history and generated inputs); feeds EXP-CONF-004 family `F-HIST` of `ebcc-v1.1` |

## 1. Question

Do archives written by past Entrybound commits, for every published identifier generation, still open under the
current build with unchanged outcome, EAM, identities and extracted bytes; do frozen planner and chunker identifiers
still produce byte-identical plans and containers; which historical identifiers (for example the replan
`entrybound/gear-cdc-v1` chunker id) exist in stored bytes; what identity changes does repack cause on them; and how
much reader code is reachable only for historical generations?

## 2. Hypotheses and falsification

- **H1 (MVT-16(a) on stored bytes).** Every stored historical archive decodes under `ebound-head` with the same outcome
  class, EAM digest, LAI/PCR/AUX/PCI and extracted tree as under its creating binary. *Falsified* by one difference.
- **H2 (MVT-02(a)).** Extraction of every stored historical archive by `ebound-head` equals the source tree fingerprint.
  *Falsified* by one byte mismatch (`roundtrip_mismatches` > 0).
- **H3 (MVT-16(d)).** For every frozen planner id (v1, v3, v4, v5, v6) and chunker id `gear-norm-v1`, `ebound-baseline`
  and `ebound-head` produce byte-identical plans, boundaries and containers on every input. *Falsified* by one
  difference.
- **H4 (F-08 repack identity tiers).** Representation-only repack of each stored archive preserves LAI, AUX and PCR;
  explicit replanning preserves LAI and AUX. *Falsified* by one violation
  (`migration_identity_conformance_fraction` < 1).
- **H5 (DEC-MOD-008).** Pack and replan of the same input record the same `chunker_id` and PCR at `ebound-head`.
  *Falsified* by one difference (the ledger status quo describes a divergence; recording it is the purpose).

## 3. Contribution per decision

| Decision | Contribution |
|---|---|
| DEC-CON-013 | Stored golden corpus of archives written by past commits (candidate evidence for `indefinite-read-support-status-quo`, `legacy-read-with-deprecated-diagnostic`, `repack-preserves-version-status-quo` versus `repack-upgrades-to-current`); identity effects of repack; retained decode surface (C2 cost line, §7.4); presence of `DEPRECATED` diagnostics (SPEC §20.4 rule 2) |
| DEC-CMP-025 | Number of cases and bytes per historical generation that the corpus must carry; inventory of internal and harness uses of historical creation APIs (`creation_api_inventory.py`, source grep of `crates/`, `research/harness`, tests) |
| DEC-MOD-008 | Which binaries and operations emit `entrybound/gear-cdc-v1` versus `gear-norm-v1`; PCR equality of pack versus replan (H5) |
| DEC-CON-012 | Stored bytes of bootstrap-namespace archives per generation, and the version-gate outcome of `ebound-head` on them |
| DEC-ECO-042 | How corpus versions tie to format generations (stored bytes versus hash-locked generator per generation; size per generation) |

## 4. Candidates

- **Builds** (the experiment's candidates): one `ebound` binary per generation commit (§5.1), `ebound-baseline`
  (`9e44608`) and `ebound-head`. Reference: the creating binary for H1-H2; `ebound-baseline` for H3.
- **Design candidates** of the informed decisions are the ledger's (CC§1: none added). `drop-pre-v1-at-freeze` and
  `reinterpret-historical-records` are not executed: dropping decode support or reinterpreting published identifiers is
  outside the program's authority (objective HC-16, §4.5) and is escalated, not decided; this experiment only records
  the stored population and retained surface such a proposal would affect.

## 5. Inputs

### 5.1 Generation commits (selection rule fixed now; list committed before any build)

`research/conformance/historical/find_generation_commits.py` selects, from `git log --first-parent dev` (read-only):
the first commit where each identifier's defining document or implementation appears (planner v1, CDC `gear-norm-v1`,
cross-file planner v3, codec/transform planner v4, DEFLATE reconstruction v5, JPEG reconstruction v6, STREAM layout v1,
encrypted Descriptor v2, crypto v1 wire, POSIX metadata v1, platform security metadata v1, legacy export profiles,
filesystem bootstrap), the last commit before the next generation's introduction when it changes an emitted identifier,
the research baseline `9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155`, and `ebound-head`. At design time the add-commits of
the defining documents are `06451ec`, `a5a1fff`, `932f69d`, `a908507`, `e2dc5bc`, `3a5bd34`, `289a160`, `cf6b7e6`,
`09a5765`, `3be3689`, `e35241b`, `2a07828`, `136d018`, `9e44608` (from `git log --diff-filter=A -- docs/…`); the script's
output, not this list, is authoritative and is committed as `generation-commits-v1.json`.

### 5.2 Build of each generation

`build_generations.sh` extracts `git archive <commit>` to `/root/eb-research/src/hist/<commit>`, builds
`ebound` with `/root/.cargo/bin/cargo +1.98.1 build --release --locked` and a per-commit `CARGO_TARGET_DIR`, copies the
binary to `/root/eb-research/conformance/historical/bin/<commit>/ebound`, records its SHA-256, and deletes the target
directory. If a commit fails to build with `1.98.1`, the pre-registered fallback is the stable toolchain released
closest before the commit date (installed with `rustup toolchain install <version>`), recorded per commit. A commit that
builds with neither is recorded `BUILD_FAILED` and its identifiers are covered by the nearest later commit that emits
them. The Windows host repeats the builds (`D:/eb-research/src/hist`, `D:/eb-research/target/hist-<commit>`) for commits
at or after `136d018` (metadata capture differs by host, F-0001).

### 5.3 Inputs packed by every generation

- Generated items (ebr generators): `hist-text-64k` (`text-v1`, 65,536 B, seed 2809170601), `hist-random-4m`
  (`random-v1`, 4,194,304 B, seed 2809170602), `hist-zeros-1m` (`zeros-v1`, 1,048,576 B, seed 2809170603), and the
  metadata tree of `research/harness/internals_identity_tools.py tree` (seed recorded by that tool).
- Small tuning items: `f01-tuning-ripgrep-14-1-1-git`, `f06-tuning-census-popest-2023`, `f08-tuning-chinook-sqlite`,
  `f12-tuning-kodak-jpeg-edge-cases-small` (JPEG path for v6), `f14-gen-nested-archives-py` (DEFLATE path for v5).
- Every creation option that selects a generation in that binary (profiles, planner selection flags, layouts,
  encryption with the public test password where supported, export profiles), enumerated per binary by
  `enumerate_options.py` from `--help` output and recorded.

### 5.4 Frozen-ID differential inputs (H3)

All small- and medium-tier `tuning` items of `research/corpus/manifest.json`; validation small and medium items in one
registered look (§12).

## 6. Environment

WSL2 Ubuntu 24.04 x86-64 on ext4 under `/root/eb-research`; Windows host (NTFS) for the Windows generation arm. No
timing. Network only for a fallback toolchain install.

## 7. Commands

```sh
python3 research/conformance/historical/find_generation_commits.py --repo /mnt/d/Projects/entrybound/entrybound --out research/conformance/historical/generation-commits-v1.json
bash research/conformance/historical/build_generations.sh research/conformance/historical/generation-commits-v1.json
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CONF-006/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-006/spec.yaml            # H1, H2, H4, H5 (stored corpus)
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-006/spec-frozen-id.yaml  # H3 (raw id EXP-CONF-006-FID)
python3 research/conformance/historical/retained_surface.py --head-src /root/eb-research/src/conf-head --stored /root/eb-research/conformance/historical/stored
python3 research/conformance/historical/creation_api_inventory.py --repo /mnt/d/Projects/entrybound/entrybound
```

`retained_surface.py` builds `ebound-head` with `RUSTFLAGS="-C instrument-coverage"` (toolchain `nightly-2026-08-01`,
`llvm-cov` from `llvm-tools-preview`), runs `ebound verify` and `ebound unpack` over (a) stored archives of historical
generations only and (b) archives written by `ebound-head`, and reports lines covered in (a) but not (b), per module
(descriptive C2 line).

## 8. Seeds, warmup, replication

Generated item seeds above; ebr seeds CC§12. No warmup. **Two repetitions** (§4.6) with repeat-hash on the stored
archive digests (a historical binary whose two packs differ in deterministic mode is recorded as a determinism finding
of that historical build, not averaged).

## 9. Metrics (CC§9)

| Metric | Role | Unit | Definition |
|---|---|---|---|
| `roundtrip_mismatches` | binary (HC-02) | count | files or symlinks whose extracted bytes differ from the source fingerprint (H2) |
| `migration_identity_conformance_fraction` | binary (HC-10) | fraction | repack operations whose LAI/PCR/AUX change pattern matches F-08 (H4) |
| MVT-16(a) differences; MVT-16(d) differences | binary (HC-16 oracle, CC§9) | count | H1; H3 |
| Stored archives and bytes per generation; historical identifiers found per operation; `DEPRECATED` diagnostics emitted; retained decode surface LoC; creation-API call sites | descriptive, non-canonical | count, B, LoC | §7 scripts |

## 10. Normalization and statistical analysis

Deterministic comparisons (§4.13); AGG-W counts and AGG-C per generation. H3 on corpus items is reported per family and
tier (no aggregation into verdicts: any difference is a violation). The analysis plan needs adoption by a non-exposed
session (CC§1).

## 11. Practical-significance thresholds

§3.3 for all binary checks; descriptive counts have no band.

## 12. Split discipline and held-out placeholder

Tuning items for H3 exploration; one registered validation look (`spec-frozen-id-validation`, written from
`spec-frozen-id.yaml` by replacing `splits: [tuning]` with `[validation]` and registered in
`research/decisions/validation-looks.jsonl` before running). Held-out placeholder (Phase D, §5.5): MVT-16(d) on held-out
items after unlock, once, with the frozen binaries; the spec is written at Commit A.

## 13. Sensitivity analysis

- Historical binaries built with `1.98.1` versus the date-matched fallback toolchain for the commits where both build
  (detects compiler-dependent output in historical builds).
- H1 with and without metadata classes that differ by host (Windows versus WSL generation arms, F-0002).

## 14. Expected negative results worth recording

- Replan emitting a chunker id different from pack (DEC-MOD-008 status quo).
- Historical builds that no longer compile with the pinned toolchain.
- `DEPRECATED` diagnostics absent for retired constructs (SPEC §20.4 rule 2 not implemented).
- Repack of bootstrap-namespace archives changing identities.

## 15. Threats to validity

- **Coverage of history**: only commits on the first-parent line are sampled; intermediate commits that briefly emitted
  other identifiers may be missed (the selection rule is fixed and its output committed).
- **Toolchain drift**: building old commits with a newer compiler can change behaviour (sensitivity §13).
- **Option enumeration** from `--help` can miss hidden creation paths (library-only APIs are listed by
  `creation_api_inventory.py`).
- **Coverage instrumentation** changes code generation; used only for the descriptive retained-surface count.

## 16. Cost estimates

| Item | Estimate |
|---|---|
| Compute | 45 CPU-hours (≈ 16 WSL and 5 Windows builds × 6 min; stored-corpus packs and checks ≈ 8 h; H3 over small+medium tuning items × 6 frozen ids × 2 binaries × 2 repetitions ≈ 30 h) |
| Disk | 25 GB (stored archives ≈ 15 GB; transient build targets ≈ 8 GB) |
| Agent effort | **scripted** (one short review session for the commit list and option enumeration) |
| Platform | Linux x86-64 (WSL), Windows x86-64 host; no timing; network only for fallback toolchains |

## 17. Outcome obligations

`outcome.json` (§10.1). Stored archives become family `F-HIST` of `ebcc-v1.1` with expectations derived from their
creating generation's documents; every H1-H3 difference is a defect or an HC-16 finding.
