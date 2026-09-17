# EXP-SCALE-005: Status-quo determinism invariance matrix (resources, environment, order, load, toolchain, host, encryption, repack)

| Field | Value |
|---|---|
| Status | **READY** (design draft until gate G-A; gate and L7 disclosure as EXP-SCALE-001) |
| Domain | scale (program §14; interfaces with model §7, planner §9, integrity §24, platform §15) |
| Platforms | Linux x86-64 (WSL2 ext4, root); Windows 11 NTFS host (arm `EXP-SCALE-005-WIN`); emulated arm64 is EXP-SCALE-014 |
| Requires timing | false |
| Compute estimate | about 70 machine-hours (Linux about 63 h, Windows about 7 h) |
| Disk estimate | about 40 GB (retained small-item archives for violation artifacts plus medium-item scratch) |
| Agent effort | scripted |
| Tooling to build | `research/tools/scale/det_cell.py` (variation driver, contract below); `research/tools/scale/shuffle_copy.py` (metadata-preserving copy in seeded creation order: mode, owner, mtime ns, xattrs, POSIX ACLs, symlinks, hard-link groups, sparse extents via SEEK_DATA); `research/tools/scale/bgload.py` (seeded background CPU load); `research/tools/scale/det_compare.py` (grouping and violation report); `ebound` built with toolchain 1.97.1 in WSL (`rustup toolchain install 1.97.1`, approved download); locale `tr_TR.UTF-8` (`locale-gen`) |

## Pre-registration

- **decision_ids:** DEC-MOD-011, DEC-MOD-012, DEC-MOD-013, DEC-CMP-019, DEC-CMP-021, DEC-CMP-041, DEC-INT-046.
- **Decision type:** EMPIRICAL for the determinism observations (binary HC-13 oracles); the scope decisions themselves are mixed (FORMAL definitions plus these measurements).
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-13 floor (HC-11, HC-13), OD-20 (M20.2 `migration_determinism_fraction` for repack), OD-18 M18.1 `cross_host_identity_agreement`; HC-13 (MVT-13(a), (b)), HC-10 (identity pattern under repack), HC-08.
- **Method, gate, author:** as EXP-SCALE-001. **L7:** this experiment reads tuning and validation corpus items of all families; see EXP-SCALE-001 and EXP-SCALE-003 notes (analysis provisional pending non-exposed sign-off).

## Question

Is the single-threaded production writer's output (PCI container bytes, and LAI, PCR, AUX identities) invariant under the variations HC-13 names and that this host can produce (available CPUs, memory limit, temporary-directory location, locale and time zone, source creation order, sustained background load, compiler toolchain, and Windows versus Linux host), are encrypted packs identity-stable while ciphertext differs, do layout round trips through repack reproduce the direct INDEXED bytes, and does a same-profile re-plan reproduce PCR and PCI (the feasibility and cost basis of `verify --reproducible`)?

## Hypotheses

- **H1 (from HC-13 statement and SPEC §12.1-§12.3, `[FORMALLY_DERIVED]` as claims; measured here):** within one host class, PCI and LAI are identical across every variation cell for every unencrypted (item, profile, layout); across WSL and Windows, LAI and PCR agree after F-0002's documented explanations, and every AUX difference is explained by FidelityReport entries; encrypted packs of the same input have equal LAI and AUX and different ciphertext; INDEXED → STREAM → INDEXED repack yields the direct INDEXED PCI; a same-profile replan yields identical PCR and PCI.
- **H0:** at least one within-host PCI or LAI difference exists (an R1 violation for the claiming candidate).
- **Falsification:** any single difference, committed as a two-output artifact (objective §2.0 rule 2; decision-method §4.13), falsifies H1 for that claim; the stress cell's detection statement follows Appendix D.5 (20 repetitions detect a per-run mismatch probability of at least 0.1391 with 95% probability).
- **Expected known exception:** REQ-CON-0055 (INDEXED/STREAM frame byte identity) is `CONTRADICTED` in the ledger; the repack round trip may show a PCI difference that is by design; it is recorded against the claim as written.

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-MOD-011 | Cross-OS (Windows/WSL) and cross-toolchain (rustc 1.97.1/1.98.1) byte comparison of pack output over tuning+validation items; repack representation-only byte identity | emulated ARM64 and CPU-feature models: EXP-SCALE-014; encoder version stability: EXP-SCALE-011; CI consumer needs: ecosystem domain |
| DEC-MOD-012 | Independent encrypted builds: LAI, AUX, PCR comparison; PCR divergence under keyed boundaries | confirmation-of-file risk of exposed LAI: crypto domain (external review) |
| DEC-MOD-013 | Sequential status-quo determinism baseline under CPU-count and load variation (the reference against which parallel prototypes are judged) | parallel prototypes: EXP-SCALE-012 |
| DEC-CMP-019 | Reproducibility matrix across platforms and thread (CPU-availability) counts; whether any output depends on the resource environment (memory limit, TMPDIR) | gains from adaptive planning: planner domain |
| DEC-CMP-021 | Byte identity of dictionary-bearing (dense, extreme) archives across Windows and WSL and CPU availability | trainer drift across libzstd releases and x86-64 feature levels: EXP-SCALE-011, EXP-SCALE-014; dependency audit: ecosystem domain |
| DEC-CMP-041 | Similarity/cohort determinism across CPU availability, order and platforms (dense and extreme exercise cohorts) | alternative similarity methods: crossfile domain |
| DEC-INT-046 | Cross-platform and cross-thread determinism runs; re-planning cost (descriptive wall and peak RSS of same-profile replan) and whether replan reproduces PCR and PCI | release-engineering workflow evaluation: ecosystem domain |

## Candidates (variation cells of the status quo)

Configurations: `bi` balanced INDEXED, `di` dense INDEXED, `bs` balanced STREAM (`--stream-window auto`), `xi` extreme INDEXED (small items only). Base environment: unpinned, `TMPDIR` on the run scratch (ext4), `LANG=C.UTF-8`, `TZ=UTC`, cap 24 GiB via `memcap_run.sh`, item read in place, rustc 1.98.1 build (EXP-SCALE-001 binary).

| Variation | Change from base | Applies to |
|---|---|---|
| `base` | none | small, medium |
| `aff1` | `taskset -c 2` | small |
| `aff8` | `taskset -c 2-9` | small |
| `aff32` | `taskset -c 0-31` | small, medium |
| `memcap-tight` | cap = max(512 MiB, 3 × item bytes + 256 MiB); an OOM outcome is recorded as not-comparable, not as a determinism result | small, medium |
| `tmpdir-alt` | `TMPDIR` on a fresh XFS loop mount | small |
| `locale-tz` | `LANG=tr_TR.UTF-8 LC_ALL=tr_TR.UTF-8 TZ=Pacific/Kiritimati` | small |
| `creation-shuffled` | item copied by `shuffle_copy.py --seed {seed}` to scratch, packed from the copy | small, medium |
| `toolchain-1971` | `ebound` built from the same commit with `cargo +1.97.1` into `/root/eb-research/target/scale-prod-1971` | small, medium |
| `stress20` | 20 sequential packs under `taskset -c 0-31` while `bgload.py --procs 31 --seed {seed}` runs seeded duty-cycle busy loops; payload reports the number of distinct PCI and LAI values | small and medium, configs `bi`, `di` only |
| `repack-roundtrip` | `pack bi` → `repack --layout stream --stream-window auto` → `repack --layout indexed`; compare with direct `bi` | small, medium |
| `replan-same-profile` | `pack bi` → `repack --profile balanced`; compare PCR and PCI with the original; record replan wall and peak RSS | small, medium |
| `encrypted-twice` | `ebr-pack --profile balanced --layout indexed --encrypt true` twice (harness binary, byte-identical to production for unencrypted cells per review; encryption path uses `pack_directory_encrypted`); identities via `ebr-inspect-bytes`/`ebound inspect --crypto` with the password sidecar | small, medium |

Each (variation, config) pair is one ebr candidate named `<variation>-<config>` (for example `aff32-di`); pairs outside the "Applies to" rules report `applicable: 0`.

**Windows arm (`EXP-SCALE-005-WIN`)**: items copied once to `D:\eb-research\scale\det-items\<item>` with `robocopy /E /COPY:DT /DCOPY:T` from the WSL path (content and mtimes; POSIX metadata is not representable on NTFS, which F-0002 already documents); variations `base`, `aff1` (`cmd /c start /wait /affinity 4`), `toolchain-1971` (Windows floating 1.97.1 and pinned 1.98.1 builds), `stress20`; configs `bi`, `di`, `bs`. Comparison against the WSL base uses PCR (content-chunking identity) and LAI/AUX with FidelityReport explanation.

**Excluded:** thread-count variation of a multi-threaded writer (none exists; EXP-SCALE-012); Docker memory-limit variation (equivalent cgroup cap used; Docker unavailable); QEMU arm64 (EXP-SCALE-014); macOS host class (EXP-SCALE-015, blocked).

## Corpus selectors

- Manifest `research/corpus/manifest.json`, splits `tuning` and `validation`; `item_ids` = every small-scale item plus, for each (split, family, independence group) with medium items, the smallest medium item (deterministic rule implemented by `research/experiments/EXP-SCALE-005/make_items.py`, which writes the `item_ids` list into `spec.yaml`). Large items are excluded for compute; the large tier's determinism is exercised by EXP-SCALE-001's repeat hashes (b16g/e16g) on synthetic data.
- **Minimum support:** determinism claims are binary per item; no corpus-level verdict.
- **Held-out placeholder (Phase D):** the `base`, `aff32`, `creation-shuffled` and `stress20` variations are re-run once on held-out items after unlock (R5 (a): no R1 failure on any held-out item), per §5.5; spec to be added to Commit A.

## Environment

`env_id` `wsl-ubuntu` and `windows-host`; no timing. Loop-mounted XFS for `tmpdir-alt`. Background load from `bgload.py` only in `stress20` cells; the orchestrator runs no other workload during `stress20` so that the load is the seeded one. Encrypted packs use fresh randomness by design.

## Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound; export PYTHONPATH=research/tools; PY=/root/eb-research/venv/bin/python
$PY research/experiments/EXP-SCALE-005/make_items.py     # rewrites item_ids in spec.yaml; commit before running
$PY -m ebr validate --spec research/experiments/EXP-SCALE-005/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-SCALE-005/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-SCALE-005/spec.yaml --refingerprint
$PY -m ebr normalize --spec research/experiments/EXP-SCALE-005/spec.yaml
$PY research/tools/scale/det_compare.py --raw research/raw/EXP-SCALE-005 --raw-win research/raw/EXP-SCALE-005-WIN \
    --out research/normalized/EXP-SCALE-005/decision/determinism.csv --artifacts research/raw/EXP-SCALE-005/violations
```

Driver contract: `det_cell.py --item {item_path} --item-id {item_id} --candidate {candidate} --rep {rep} --seed {seed} --scratch {scratch} --manifest research/corpus/manifest.json` prints one `ebr.scale.det.v1` line; every produced archive is verified (`ebound verify`, or the password sidecar for encrypted) before identities are recorded; archives of small items are retained under `/root/eb-research/artifacts/EXP-SCALE-005/<item>/<candidate>/<rep>.eb` until `det_compare.py` finishes, so that both outputs of any difference can be committed.

## Seeds

`seeds.order` 1305001, `seeds.bootstrap` 1305002, `seeds.command` 1305003 (creation shuffles and background-load duty cycles derive from the per-sample seed).

## Warmup

0 (deterministic outputs).

## Measurement method and metrics

| Payload key | Canonical name | Role | Family |
|---|---|---|---|
| `output_sha256` | repeat-hash (§4.13) | binary | correctness (string) |
| `lai`, `pcr`, `aux`, `pci` | identities (SPEC §8.0) | binary comparisons | correctness (string) |
| `distinct_pci`, `distinct_lai` (stress20) | none; > 1 is a violation | binary | correctness |
| `repack_pci_equal_direct`, `repack_lai_pcr_aux_equal` | `migration_determinism_fraction` inputs (HC-13) and `migration_identity_conformance_fraction` inputs (HC-10) | binary | correctness |
| `replan_pcr_equal`, `replan_pci_equal` | none | binary | correctness |
| `enc_lai_equal`, `enc_aux_equal`, `enc_pcr_equal`, `enc_ciphertext_differs` | MVT-13(b) inputs | binary (PCR equality descriptive) | correctness |
| `outcome`, `reason_code` | none | outcome | correctness |
| `replan_wall_s`, `replan_peak_rss_bytes` | none (descriptive) | descriptive | time_wall, memory_peak |

Aggregate: `cross_host_identity_agreement` (M18.1, binary per archive) computed by `det_compare.py` from WSL and Windows base rows with the FidelityReport explanation rule of MVT-13(a).

## Replication

3 repetitions per (variation, config, item) (MVT-13(a) minimum); `stress20` performs 20 packs inside each of its 3 samples (60 per cell), exceeding the 20-repetition stress requirement. No adaptive rule.

## Normalization and statistical analysis

Group by (item, config): the set of distinct PCI and LAI over all within-host variation samples. Violation list = groups with more than one value, each with the minimal pair of variations exhibiting it. Cross-host: LAI equality and AUX-difference explanation per item. Encryption: per item. Repack and replan: per item. No statistics beyond counts and detection statements (Appendix D.5); every violation is an artifact.

## Practical-significance thresholds

None applicable: all oracles are binary (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- Scale arm: small vs medium items (reported separately).
- Profile arm: `bi` vs `di` vs `xi` (dictionary and cohort code paths).
- Order arm: `creation-shuffled` repeated with three seeds (the three repetitions use different seeds by construction).

## Expected negative results worth recording

A PCI difference under `toolchain-1971` (cross-compiler byte instability of dependencies or codegen affecting output); a dense/extreme dictionary archive differing across Windows and WSL; `repack-roundtrip` PCI not equal to the direct INDEXED pack (REQ-CON-0055); replan not reproducing PCI (limits `verify --reproducible`); any within-host nondeterminism under `stress20` or `locale-tz`.

## Threats to validity

1. The production writer is single-threaded; invariance under CPU availability shows only that no environment probe changes planning, not that a parallel writer would be deterministic (EXP-SCALE-012).
2. `shuffle_copy.py` must preserve every captured metadata class; a copy defect would appear as an AUX difference and is triaged by comparing a `getfacl`/`getfattr`/`stat` census of source and copy before attributing a difference to Entrybound.
3. `robocopy` to NTFS changes metadata by design; Windows comparisons are restricted to PCR and explained AUX/LAI.
4. Emulated architectures and CPU-feature levels are not covered here.
5. The harness `ebr-pack` path for encryption is not the CLI; identity equality there is evidence about the library entry point `pack_directory_encrypted`.

## Platform requirements

Linux x86-64 WSL2 root; Windows 11 NTFS non-admin; rustc 1.97.1 and 1.98.1 on both hosts; about 40 GB; locales package; no network after tool installation.

## Estimated compute and effort

About 70 h (59 small items × 41 cells × 3, plus 109 medium items × 13 applicable cells × 3, plus 60-pack stress cells; dense medium packs dominate). Scripted. If the budget must shrink, the pre-registered reduction is to drop `toolchain-1971` and `memcap-tight` on medium items first (recorded as a deviation), never to cut repetitions below 3.

## Deviations (append-only)

None.
