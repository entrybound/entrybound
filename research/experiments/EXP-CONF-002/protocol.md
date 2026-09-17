# EXP-CONF-002 — Production versus independent reader differential: conformance corpus pass, production-packed archives and exhaustive structural mutation class, with triage

| Field | Value |
|---|---|
| Status | `NEEDS_TOOLING` (CC§2): frozen `ebir` v1 (EXP-CONF-001), `ebcc-v1` (EXP-CONF-004), `ebcc-readtuple`, `ebcc-mutate`, `ebcc-diff`, field-level structural byte map (extension of `ebr-inspect-bytes`), Windows packing script |
| Domain / program sections | conformance / §28 (primary), §29 |
| Decision IDs informed | DEC-ECO-032, DEC-ECO-060, DEC-CON-002, DEC-CON-003, DEC-CON-012, DEC-CON-025, DEC-CON-026, DEC-CON-030, DEC-ECO-038, DEC-INT-020, DEC-INT-040, DEC-INT-042, DEC-LEG-004, DEC-MOD-004, DEC-MOD-006, DEC-MOD-027, DEC-MOD-041 |
| Requirements | REQ-ECO-0133, REQ-ECO-0037, REQ-ECO-0030 |
| HC oracles | HC-03 MVT-03(b); HC-12 MVT-12(a); proposal for the assigner: `hc_ids` HC-03, HC-12, HC-15; `od_ids` OD-04, OD-21 |
| Decision type | not assigned (gate G-A) |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1 |
| Depends on | EXP-CONF-001 (frozen reader v1, later v2), EXP-CONF-004 (corpus and expectations) |

## 1. Question

On the native conformance corpus, on archives that production packs from tuning items (and, in one registered look,
validation items), and on a structure-aware single-byte mutation corpus of those archives, do the production reader
paths and the clean-room readers produce identical outcome tuples; to what does independent triage attribute each
disagreement; and which default-written archives can a minimal L-base reader not read?

## 2. Hypotheses and falsification

- **H1 (MVT-03(b), MVT-12(a)).** No disagreement between production and `ebir` remains that triage (CC§6) does not
  attribute to `ebir`. *Falsified* by one cluster attributed `PRODUCTION_DEFECT`, `SPEC_AMBIGUOUS`, `SPEC_MISSING` or
  `BOTH_WRONG` (`reader_disagreements` > 0).
- **H2 (clean-room pass).** `ebir` v1 matches the expected outcome class and, where documented, the reason code on 100%
  of baseline-scope conformance cases. *Falsified* by one baseline case failing that is not attributed to `ebir`
  (`cleanroom_pass_fraction` < 1).
- **H3 (fragmentation, EXP-CONF-001 H4).** `ebir` at L-base reads every archive `ebound-head` writes with default
  options on WSL ext4 and on the Windows host. *Falsified* by one `UNSUPPORTED` outcome on such an archive.

## 3. Contribution per decision

| Decision | Contribution |
|---|---|
| DEC-ECO-032 | Differential results of the executed clean-room candidate; attribution counts (the evidence that the reader "tests something") |
| DEC-ECO-060 | Corpus-based equivalence results for a reimplementation in another language (candidate `independent-reimplementation`) |
| DEC-CON-025, DEC-INT-040, DEC-MOD-041, DEC-INT-020 | Disagreements on preamble, segment, Index, truncation and precedence cases across production paths and `ebir`; clusters decided by a citation versus by production behaviour only |
| DEC-CON-002, -003, -012, -026, DEC-MOD-004, -006, -027, DEC-LEG-004, DEC-INT-042 | Disagreement clusters by feature area (container structure, section/feature registries, version gate, STREAM record-set equivalence, comparator, TLV canonicality, identity roots recomputed by an independent reader, provenance registries, verification state) |
| DEC-CON-030, DEC-ECO-038 | `reader_fragmentation_count` on default-written archives from both hosts (H3) |

## 4. Candidates (readers compared)

| Reader id | Definition |
|---|---|
| `PROD-MEM` (reference) | `ebound-head` library full open and verify, in-memory (EXP-CONF-004 P-MEM) through `ebcc-readtuple` |
| `PROD-STREAM` | EXP-CONF-004 P-STREAM, STREAM archives only |
| `PROD-CLI` | `ebound verify` exit status and JSON, corpus-pass mode only |
| `EBIR-V1` | EXP-CONF-001 frozen v1 (`FREEZE-v1.json`), `ebir batch` |
| `EBIR-V2` | EXP-CONF-001 frozen v2 (after errata) |
| `EBIR-PY-V1` | EXP-CONF-001 arm B framing reader; framing comparison-key subset only |

Design candidates of the informed decisions are the ledger's; none added (CC§1). `ebound-baseline` is not compared
here (EXP-CONF-004 covers baseline-versus-head behaviour).

## 5. Inputs and mutation classes

### 5.1 Base archives

- **B-CORPUS**: every case of `ebcc-v1` whose `applicable_paths` include P-MEM or P-STREAM.
- **B-TUNING**: `ebound-head` packs, in deterministic mode, unencrypted, with each profile {fast, balanced, dense,
  extreme} × layout {indexed, stream}, the small-tier tuning items `f01-tuning-ripgrep-14-1-1-git`,
  `f01-tuning-zstd-v1-5-7-git`, `f02-tuning-lz4-intree-make-build`, `f04-tuning-sourcelike`,
  `f06-tuning-census-popest-2023`, `f06-tuning-gen-structured-a-small`, `f07-tuning-gen-numeric-a-small`,
  `f08-tuning-chinook-sqlite`, `f12-tuning-kodak-jpeg-edge-cases-small`, `f14-apache-maven-3916-bin-tarball`,
  `f14-gen-nested-archives-py`, `f19-tuning-zoo`, `f20-tuning-generated-malicious-structures`,
  `f20-tuning-generated-private-vault` (packed as plain files; no decryption involved). The item list is verified
  against the manifest (`split == tuning`, `scale == small`) by `research/conformance/tools/check_items.py` before
  the record is committed; an id that fails is replaced by the next small tuning item of the same family in manifest
  order, recorded as a deviation. An item that `ebound-head` refuses to pack (for example a tree with non-UTF-8 names,
  as the baselines probe recorded) contributes no base archive; its refusal outcome is recorded, not replaced.
- **B-WINDOWS**: the same packs produced by `ebound.exe` (`ebound-head`, Windows build) from copies of those items on
  NTFS (`D:/eb-research/conf-pack/`), transferred to WSL by file copy; used for H3 and the corpus-pass mode only.
- **B-VALIDATION** (registered look, §12): small-tier validation items `f01-validation-redis-7-2-16-git`,
  `f06-validation-osm-monaco-20250101-xml`, `f07-validation-gen-numeric-b-small`, `f08-validation-sakila-sqlite`,
  `f19-validation-deep`, `f20-validation-generated-malicious-structures`, `f20-validation-real-cpython-archivetestdata`,
  `f15-validation-edgecases`, packed as for B-TUNING.

### 5.2 Structural byte map

`ebr-inspect-bytes` (harness crate `ebr-pack`) is extended to emit a field-level map: preamble, Descriptor, feature
bitmaps, section directory, section headers, record headers (type and length fields), footer, STREAM item headers,
Index record headers; everything else is payload. `ebir map --archive` emits the same map from the independent parse.
The **union** of both maps is the structural set (a byte classified structural by either reader), so the map does not
inherit one reader's misclassification.

### 5.3 Mutation classes (pre-registered)

- **M1 (exhaustive for the class as defined).** Every structural byte × all 255 alternative values, for archives whose
  every section holds ≤ 64 records. For larger archives: all bytes of preamble, Descriptor, feature bitmaps, section
  directory, section headers, footer and STREAM item headers × 255 values; record-header bytes of the first 32 and last
  32 records of each section × 255 values; all remaining record-header bytes × 8 single-bit flips. The exhaustiveness
  claim covers exactly this set.
- **M2 (regression evidence, no bound claimed).** 10,000 payload byte positions per archive drawn with seed
  `2809170001` + archive index, each replaced by a seeded different value.
- **M3 (truncation).** Every truncation length for archives ≤ 64 KiB; for larger archives, every section and item
  boundary ±1 and 1,000 seeded lengths (seed `2809170001` + 10^6 + archive index).

M1-M3 run on B-CORPUS valid cases and B-TUNING (and B-VALIDATION in the registered look). Mutations are applied
in memory: `ebcc-mutate` streams `(archive_id, class, offset, value)` records; `ebcc-readtuple` (production, in-process)
and `ebir batch` consume the same stream and emit tuple digests; `ebcc-diff` joins them and writes one disagreement
record per differing input plus up to 5 representative mutated archives per cluster (CC§8 applies).

## 6. Environment

WSL2 Ubuntu 24.04 x86-64, ext4 under `/root/eb-research`; Windows host only for B-WINDOWS packing. No timing
(`timing: false`). No network. Parallelism: 28 workers (`ebcc-diff --jobs 28`), leaving 4 vCPUs idle.

## 7. Commands

```sh
python3 research/conformance/tools/check_items.py --spec research/experiments/EXP-CONF-002/spec.yaml
# B-TUNING packs (deterministic, recorded with sha256)
python3 research/conformance/tools/pack_bases.py --ebound /root/eb-research/target/conf-head/release/ebound \
  --items research/experiments/EXP-CONF-002/items-tuning.txt --out /root/eb-research/conformance/bases/EXP-CONF-002/tuning
# Windows host (PowerShell 7): identical packs on NTFS
pwsh -File research/conformance/tools/pack_bases_windows.ps1 -Ebound D:/eb-research/target/conf-head/release/ebound.exe -Items research/experiments/EXP-CONF-002/items-tuning.txt -Out D:/eb-research/conf-pack/EXP-CONF-002
# runner
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CONF-002/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-002/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-CONF-002/spec.yaml
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-CONF-002/spec.yaml
python3 research/conformance/tools/diff_tables.py --exp EXP-CONF-002   # clusters, attributions, metrics (§12.3 provenance)
```

The spec's items are the B-CORPUS family bundles (conformance manifest) and the B-TUNING tuning items (corpus
manifest); the bundle of packed bases for a tuning item is produced by `pack_bases.py` and located by `item_id`.
Candidates are comparison modes; each sample compares `PROD-*` against the mode's `EBIR-*` reader on that item.

## 8. Seeds, warmup, replication

Seeds: M2/M3 `2809170001`; ebr order/bootstrap CC§12; triage reliability sample `2809170003`. No warmup. **Two
repetitions** with repeat-hash of `payload.artifact_sha256` (the sorted disagreement records). A differing pair from
one reader is a determinism violation of that reader (objective §2.0 rule 2); both artifacts are committed.

## 9. Metrics (CC§9)

| Metric | Role | Unit | Definition |
|---|---|---|---|
| `reader_disagreements` | binary (HC-03) | count | distinct (input, reader pair) comparison-key disagreements whose cluster is not attributed `OTHER_READER_BUG` to `ebir`; per reader pair and version |
| `cleanroom_pass_fraction` | binary (HC-12) | fraction | over baseline-scope `ebcc-v1` cases: `EBIR-V1` (and separately `EBIR-V2`) class matches expectation and code matches where `code_status: DOCUMENTED`; cases failing only through clusters attributed to `ebir` are re-counted after v2 |
| `reader_fragmentation_count` | cost input (C5) | count | distinct feature bits for which some B-TUNING or B-WINDOWS archive is `UNSUPPORTED` at `ebir` L-base |
| `decided_class_reach_fraction` | descriptive | fraction | fraction of the pre-registered decided (class, code) pairs of `ebcc-v1` that M1∪M2∪M3 reach on `PROD-MEM` |
| `reason_code_specificity_fraction` | banded (T-20), secondary here | fraction | per reader, as EXP-CONF-004 §8, over negative baseline cases |
| Cluster counts by attribution, feature area and mutation class; kappa of triage reliability sample; disagreements on `detail` only | descriptive, non-canonical | count | `diff_tables.py` |

## 10. Normalization and statistical analysis

Deterministic comparisons: no intervals. AGG-W for counts; AGG-C per case family for fractions. Binary metrics screen at
R1 (§3.3): a `PRODUCTION_DEFECT` cluster is an HC-03 violation of the production build; a `SPEC_*` cluster is an HC-03
violation of the specification until the text is fixed (objective MVT-03(b)). Tuning-derived and validation-derived
results are reported separately and never pooled. The analysis plan needs adoption by a non-exposed session (CC§1).

## 11. Practical-significance thresholds

§3.3 for binary metrics; T-20 for `reason_code_specificity_fraction` (`thresholds.json`
`metric_thresholds.reason_code_specificity_fraction`); cost inputs and descriptive counts have no band.

## 12. Split discipline and held-out placeholder

- Tuning packs (B-TUNING) drive triage and `ebir` v2 errata. No change to `ebir`, the mutation classes or the
  comparison key is allowed after the validation look.
- **Validation look** (B-VALIDATION): one run of `EBIR-V2` and `PROD-*` in the M1-M3, corpus-pass and fragmentation
  modes after v2 is frozen, defined by `spec-validation.yaml` (raw experiment id `EXP-CONF-002-VAL`, so its raw data
  never mixes with tuning runs); registered in `research/decisions/validation-looks.jsonl` before running; a regression
  check, not a search.
- **Held-out placeholder (Phase D, §5.5):** after Commit C, the same modes on small-tier held-out items with
  `EB_HELDOUT_UNLOCK` set, run exactly once with the frozen readers; the spec is written at Commit A and references
  §5 of `decision-method.md`. No held-out item is named here.

## 13. Sensitivity analysis

- Attribution under the second triager (30% sample) versus the first.
- M1 over the production-only structural map versus the union map.
- `EBIR-V1` versus `EBIR-V2`; with and without `detail` in the comparison key.
- Disagreements counted per input versus per cluster.

## 14. Expected negative results worth recording

- `UNDOCUMENTED` reason codes dominating `ebir` disagreements (a documentation gap, attributed `SPEC_MISSING`).
- Truncation classification differences at section boundaries between the two readers.
- Windows-written archives unreadable at L-base because of the required `0x10000` bit (F-0001).
- Mutations accepted by one reader and rejected by the other at TLV minimality or reserved-field rules.

## 15. Threats to validity

- **Shared model knowledge** may make both readers agree on an unwritten convention (CC§11); agreement is weaker
  evidence than disagreement.
- **Structural map completeness**: bytes neither reader classifies as structural are sampled only by M2.
- **Harness bugs** in `ebcc-readtuple` or `ebcc-diff` look like reader disagreements; each cluster's representative is
  re-run through `ebound verify` and `ebir decode` as separate processes before triage.
- **Production-packed bases** exercise only what the packer emits; M1-M3 and B-CORPUS widen that.
- **Triage is judgment** (CC§6); reliability is measured, not assumed.

## 16. Cost estimates

| Item | Estimate |
|---|---|
| Compute | 150 CPU-hours (M1 dominates: ≈ 5.5 × 10^8 mutated decodes per reader across bases; 28 workers ≈ 6 wall hours per repetition) |
| Disk | 25 GB (compressed tuple digests, disagreement records, ≤ 5 representatives per cluster, base packs) |
| Agent effort | **scripted** execution; **judgment, medium** for triage (1-3 sessions) and reliability re-triage (1 session) |
| Platform | Linux x86-64 (WSL); Windows host for B-WINDOWS packing; no privileged operation; no timing; no network |

## 17. Outcome obligations

`outcome.json` (§10.1). Each disagreement cluster becomes a conformance case in `ebcc-v1.1` family `F-DIFF` with the
triaged expectation, or an `UNRESOLVED` specification finding. Clusters attributed to production are defects
(objective §2.0 rule 5).
