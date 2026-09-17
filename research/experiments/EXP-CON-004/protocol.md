# EXP-CON-004: Native ECF versus retrofit containers — capability-gap battery

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: `research/tools/container/retrofit/build_container.sh`, `research/tools/container/retrofit/capability_battery.py`, retrofit format builders and readers (section 7), CT-3 |
| Domain / program section | container / §11 (with §3, §6, §32, §34) |
| decision_ids | DEC-CON-022; input to DEC-CON-002 |
| Decision type (proposal) | EMPIRICAL (mechanical capability tests and bytes); the adoption part of DEC-CON-022 is not decided here |
| OD / HC (proposal) | OD-02, OD-04, OD-10, OD-11, OD-14, OD-17, OD-21, OD-26; HC-01, HC-03, HC-04, HC-06, HC-09, HC-10, HC-15 |
| Division of labour | This experiment measures which guarantees each container shape can deliver mechanically. Artifact size of native vs retrofit: `EXP-EVAL-002` H5; workflow need trace: `EXP-ECO-006` W14; remote open cost of retrofit layouts: not claimed here |
| Specs | `spec.yaml` (tuning, every item); `validation-items.txt` |
| Method, author, L7, gates | `../_index/container-conventions.md` §1-§2 |

## 1. Question

Research I's doctrine favours backward-compatible retrofits; SPEC ships a native ECF container. Which of the guarantees the native container is designed for (random entry access without a full scan, verified single-entry and partial-range reads from the format's own data, detection of truncation at every offset and of corruption in every region, declared resource bounds before payload, fail-closed refusal of unknown critical extensions, a unique canonical encoding, identities independent of compression parameters, single-file self-containment, write-only streaming, metadata-class coverage) can retrofit containers deliver on the same content, and what metadata access cost does each shape impose locally?

## 2. Hypotheses

- **H1.** Every retrofit and legacy-only candidate fails at least two capability tests that `native-ecf-balanced-indexed` passes. Expected failures: `declared_bounds_before_payload` (tar and ZIP declare no entry count or expansion bound before payload), `fail_closed_unknown_critical_extension` (tar and ZIP readers ignore unknown extension headers or extra fields), `unique_canonical_encoding` (tar headers and ZIP extra fields admit many encodings of one tree), and `truncation_detected_all_offsets` for gzip-member-concatenated eStargz blobs at member boundaries.
- **H2.** At least one retrofit (`retrofit-tar-zstd-chunked-toc` or `retrofit-estargz-toc`) matches native on `random_entry_without_full_scan` and `verified_single_entry_read` (their TOCs carry per-entry digests), and `sidecar-tar-zstd-plus-ecf-sidecar` matches native on verified reads but fails `single_file_self_contained`.
- **H3.** The native container passes every capability test in the battery on every tuning item that production packs (a native failure is a production defect, objective §2.0 rule 5, not a reason to relax the test).
- **H0 / falsification.** H1 is falsified if any retrofit passes every capability test that native passes on every applicable item; H3 by any native failure (recorded as a defect).

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CON-022 | Mechanical capability-gap table per container shape and item (AGG-C per family), local bytes to reach the first entry and to verify one entry, accepted alternative encodings | Size of native vs retrofit: EXP-EVAL-002; workflow need: EXP-ECO-006; adoption evidence and Research III findings: analytical, cited from primary sources; the supersession record is written at decision time |
| DEC-CON-002 | Whether any retrofit makes a native revision unnecessary for a given guarantee | EXP-CON-008 synthesis |

## 4. Candidates

| candidate_id | Ledger candidate | Definition (built from the same tuning item) |
|---|---|---|
| `native-ecf-balanced-indexed` | native-ecf-primary-status-quo | `ebound-prod pack --profile balanced --layout indexed` (REF-PROD) |
| `retrofit-tar-zstd-chunked-toc` | retrofit-only (zstd:chunked style) | Deterministic tar stream (`tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner`, GNU tar 1.35) compressed per file as zstd frames with a TOC (per-entry offsets, sizes and SHA-256 digests) in a zstd skippable frame at the end, following the containers/storage zstd:chunked layout description at a pinned commit (primary source recorded in `retrofit/FORMATS.md`) |
| `retrofit-estargz-toc` | retrofit-only (eStargz style) | Per-file gzip members of the tar stream plus `stargz.index.json` TOC entry and 51-byte footer per the eStargz specification at a pinned commit of containerd/stargz-snapshotter |
| `retrofit-zip-central-directory-digests` | retrofit-only (in-band ZIP index) | Info-ZIP `zip -X -y -9` archive plus a SHA-256 per entry in a private extra field (header ID in the user range) written by `retrofit/zip_digest_extra.py` |
| `sidecar-tar-zstd-plus-ecf-sidecar` | ecf-as-sidecar-to-legacy | `tar | zstd -19` plus `ebound-prod sidecar <artifact>` output |
| `dual-publish-ecf-plus-tar-zstd` | dual-publishing | native archive plus `ebound-prod convert --to tar.zst` of it (the pair is the candidate; capability is the native member's, cost is both artifacts' bytes) |
| `legacy-only-tar-zstd` | defer-native-format | `tar | zstd -19` only |
| `legacy-only-zip` | defer-native-format | `zip -X -y -9` only |

No exclusions. Retrofit readers are research prototypes written from the cited specifications (`retrofit/FORMATS.md`), cross-checked against incumbent tools (`tar`, `zstd`, `gzip`, `unzip`) for byte validity; a retrofit capability is credited only if the prototype reader implements the check the specification makes possible (the battery never credits a capability the format cannot express). Critic-proposed candidates enter at R1 (R0 item 6).

## 5. Corpus selectors

- Tuning: every tuning item (112; `spec.yaml`). Items production refuses (non-UTF-8 names) are recorded; retrofits still run on them and the asymmetry is reported (fair-comparison rule 1 of `research/baselines/README.md`).
- Validation (CT-4): `validation-items.txt` (every validation item). Held-out: none (section 15).
- Applicability: every capability test applies to every item; `metadata_classes_representable` uses the F19 items' observed metadata classes (from the corpus fingerprints; platform domain owns fidelity decisions).

## 6. Environment and platform requirements

`wsl-ubuntu` only (deterministic, no timing). Tools: GNU tar 1.35, zstd 1.5.5, gzip 1.12, Info-ZIP zip/unzip, Python 3.12 stdlib, `ebound-prod`. Linux x86-64; no privileges; storage section 16.

## 7. Commands

```sh
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CON-004/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-CON-004/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-CON-004/spec.yaml
python research/tools/decide/analyze.py --experiment EXP-CON-004 --coverage-mode AGG-C   # §14 item 2 tooling
```

Tooling to build: `retrofit/build_container.sh` (builds each candidate deterministically into `{scratch}/c`), `retrofit/capability_battery.py` (runs the battery, prints one JSON line), `retrofit/zstd_chunked.py`, `retrofit/estargz.py`, `retrofit/zip_digest_extra.py` (builders and prototype readers with read logging), and `retrofit/FORMATS.md` (primary-source citations with commit and section for every structure used).

Capability battery (each test is mechanical and applies the same oracle to every candidate):

| Test key | Oracle |
|---|---|
| `random_entry_without_full_scan` | For 1,000 seeded regular-file entries, the reader returns the exact bytes while reading at most (entry stored bytes + format metadata directly addressed by the format's own index) and never the payload of any other entry (read log) |
| `verified_single_entry_read` | The reader verifies the entry against a digest carried by the format and bound to a format-level structure without reading other entries' payloads; a one-byte change in the entry's stored bytes is detected (seeded) |
| `verified_partial_range_read` | As above for a 4 KiB range inside an entry of ≥ 1 MiB, without reading the whole entry |
| `truncation_detected_all_offsets` | Truncation at every structural boundary and 1,000 seeded offsets: the format's reader never reports success; fraction refused = `adversarial_true_refusal_fraction__truncation` |
| `corruption_detected_all_regions` | 200 seeded single-byte flips per region (payload, per-entry headers, index/TOC, footer/trailer): every flip detected by the reader; `localization_precision` over detected flips |
| `declared_bounds_before_payload` | The format declares entry count, total logical size and maximum expansion before the first payload byte (native preamble; retrofit headers), and the declaration equals the actual values |
| `fail_closed_unknown_critical_extension` | An extension marked critical by the format's own grammar (or, where the grammar has no criticality, an unknown extension carrying a semantic change such as an unknown pax keyword that renames the path) makes the reference reader refuse rather than proceed |
| `unique_canonical_encoding` | The format-building mutator produces alternative encodings of the same tree (tar: base-256 vs octal sizes, pax vs GNU long names, padding blocks; ZIP: extra-field order, data descriptors; native: the canonical-encoding mutation generators of EXP-CONF-004); `desc_accepted_alternative_encodings` = count accepted by the reader as the same content; pass iff 0 |
| `identity_independent_of_compression` | Rebuild with a different compression level; the format's content identity (native LAI; retrofit TOC digest set) is unchanged |
| `single_file_self_contained` | Every check above uses only the one artifact |
| `streamable_write_only_sink` | The builder writes to a pipe (`| cat >`) with no seek and produces identical bytes |
| `metadata_classes_representable` | Fraction of the item's observed metadata classes that the format can represent (descriptive; binary pass iff 1) |

## 8. Seeds, warmups, repetitions

Seeds 14001-14003 (order, bootstrap, command; the command seed drives every seeded choice above). Deterministic: 2 repetitions with repeat-hash of the JSON payload and of each built artifact (builders must be deterministic; a nondeterministic builder is recorded and its capability rows are invalid).

## 9. Measurement method and metrics

| Metric | OD | Role | Family | T-ID |
|---|---|---|---|---|
| `desc_capability__<test>` (12 binary tests) | per test | descriptive capability rows (AGG-C per family) | correctness | none (binary rows, not screens) |
| `verified_fraction` | OD-11 | **primary** for the verification guarantee | other | T-20 |
| `truncation_salvage_fraction` | OD-14 | primary for truncation reporting | other | T-20 |
| `localization_precision` | OD-14 | secondary | other | T-20 |
| `adversarial_true_refusal_fraction__truncation`, `__corruption` | OD-15/OD-17 | binary per candidate (reported; eliminates only the native candidate if it fails, as a defect) | correctness | HC-15 |
| `open_bytes_read__wl_first`, `open_bytes_read__wl_rand1_mean` | OD-10 | **primary** (local access cost) | size | T-11 |
| `artifact_bytes` | OD-05 | reported; primary comparison belongs to EXP-EVAL-002 | size | T-01 |
| `single_file_self_contained` | OD-26 | binary row | correctness | HC-09 |
| `desc_accepted_alternative_encodings`, `desc_bytes_read_to_verify_one_entry_mean` | OD-04, OD-11 | descriptive | other | none |

The capability tests are not R1 screens for the retrofit candidates: DEC-CON-022 asks which guarantees require a native format, so a retrofit's failure is the evidence, not an elimination. The native candidate's failures are defects (objective §2.0 rule 5).

## 10. Normalization and statistical analysis

Capability rows: AGG-C per family (fraction of applicable items passing), headline minimum over families; exact deterministic counts. Graded metrics (`verified_fraction`, `truncation_salvage_fraction`, `open_bytes_read__*`): L1-L4 aggregation (§4.9) against `native-ecf-balanced-indexed`, confirmatory W-vs-S only on validation (CT-4), with the §4.12 confidences. The DEC-CON-022 record states, per guarantee, the set of candidates delivering it and the evidence reference; the selection among container strategies is made by the decision owner under R4-R10 with the cost tiers of each strategy (native format T4; retrofit T3-T4 for new in-band structures; dual publishing costs both artifacts).

## 11. Practical-significance thresholds

T-01, T-11, T-20 (`0.5/n_cases`) as transcribed in `research/methods/thresholds.json`. Binary rows have no band.

## 12. Sensitivity analysis

Conventions §8 defaults where graded metrics apply, plus: (a) retrofit reader strictness arm: each retrofit prototype also run in a "maximally strict" mode that applies every check its specification permits (for example refusing unknown pax keywords), so that a failure attributable only to lax incumbent behaviour is separated from a format limitation; (b) native profile arm (`dense`, STREAM layout) for the native rows; (c) item-size stratification (small tier vs rest).

## 13. Adversarial search and expected negative results

- `adversarial-search/`: per candidate, a search for inputs where the native container loses a capability that a retrofit keeps (for example streaming write of entries larger than memory, metadata classes the native model lacks, non-UTF-8 names that production refuses), and where a retrofit passes a test by accident (items with one file, empty files).
- Negative results worth recording: retrofits matching native on verified reads and random access (which narrows the native format's justification to bounds, canonicality and fail-closed evolution); native refusals of real trees (non-UTF-8 names, ACL masks) that retrofits accept.

## 14. Threats to validity

Retrofit prototypes are written by this program from specifications, not by their upstream projects; a capability may be under- or over-credited by implementation choices (mitigated by strictness arm (a), incumbent byte-validity checks and `FORMATS.md` citations). Capability tests encode the native design's goals; guarantees that retrofits provide and native does not (ecosystem reach, zero-install consumption) are outside this battery and are reported by EXP-ECO-006 and EXP-EVAL-002. L7 exposure (conventions §1).

## 15. Phase D held-out placeholder

As EXP-CON-001 section 15 (the battery re-runs once on the frozen held-out items under R5 for the capability rows used by the decision).

## 16. Cost and effort

Compute: about 12 h tuning (8 candidates × 112 items × 2 repetitions; builds dominate on large items), about 10 h validation. Disk: about 180 GB transient per pass if all candidates of a large item are kept (8 artifacts); the builder deletes each artifact after its battery row is written and keeps only hashes, so peak about 30 GB. Agent effort: judgment for `FORMATS.md` and the strictness definitions; scripted otherwise.
