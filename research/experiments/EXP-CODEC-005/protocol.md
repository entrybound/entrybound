# EXP-CODEC-005: Codec and transform admission-criteria census: specification independence, test vectors, bound formulas, decoder size, float/VM dependence, incident and CVE history, registry governance

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CODEC-005 |
| Domain | codecs (8.6, 8.7) |
| Status | NEEDS_TOOLING (census scripts, section 7). Parts end in `EXTERNAL_REVIEW_REQUIRED`: final assurance that a specification is independently implementable (objective HC-12 note), and licence interpretation beyond SPDX compatibility lists (§7.5, §8.3). |
| Kind | Static census. Not runner-driven, so there is no `spec.yaml`. Outputs are generated, provenance-headed tables under `research/raw/EXP-CODEC-005/` and `research/normalized/EXP-CODEC-005/`, plus two-session classification records. |
| decision_ids | DEC-CMP-008 and DEC-CMP-009 (the SPEC §5.8 criteria audit per codec: normative specification independent of any implementation, published test vectors, bounded declarable decode memory as a written formula, independent implementations or a tractable second, minimal decoder size, float or VM dependence), DEC-CMP-010 (governance review of comparable registries: ZIP method IDs, IANA media types, OCI media types, xz filter IDs, 7-Zip method IDs; C1 words and wire items per registry candidate), DEC-CMP-043 (normative definition and specification size per transform family), DEC-CMP-044 (incident history of codec encoder and transform corruption bugs), DEC-INT-008 (decoder CVE history, for attack-surface weighting of the STREAM stored-bytes candidates) |
| Scope boundary | Crate-level dependency facts (licences via `cargo deny`, unsafe and native counts, RUSTSEC advisories, maintainers, MSRV, capability classes) are EXP-ECO-001. Its H5 public-specification table is reused as a cross-check, not re-derived. Encoder drift across versions is EXP-ECO-002. The clean-room reader's `independent_reader_loc` is conformance EXP-CONF-001/002. Legacy coder audits are EXP-LEGACY-021. This census adds the per-codec §5.8 criteria evidence that those experiments do not produce. |
| Proposed decision type | FORMAL (criteria from primary sources), EXTERNAL (specification-independence assurance, unusual licences), EMPIRICAL (LoC measured on pinned sources) |
| Proposed od_ids | OD-21, OD-22, OD-23, OD-24 (cost inputs, §7.6), with binary `baseline_codec_public_spec` and `unspecified_baseline_decode_steps` |
| Proposed hc_ids | HC-11, HC-12 (codec provenance, MVT-12(c)), HC-16 |
| Method, gates, author, L7 | EXP-CODEC-001 section 1 and Appendix C. Classification sessions must be uninvolved in the candidates (§7.2 C4-C7 assessor rule). This L7-exposed design session does not classify. |

## 2. Question

For every codec and structural transform under consideration in EXP-CODEC-001/004, what do pinned primary sources show about the SPEC §5.8 admission criteria?

- a normative specification independent of any implementation;
- published test vectors;
- a written bound formula for decode memory;
- independent implementations, or a tractable second one;
- decoder size;
- floating-point or virtual-machine dependence.

What encoder-corruption and decoder-vulnerability history do the primary sources document? How do comparable registries govern identifier allocation? What cost elements C1, C4, C5 and C7 follow for each candidate?

## 3. Hypotheses

These are predictions to be checked against primary sources `[INFERRED]`.

- **H1a.** Four codecs meet the normative-specification criterion: Zstandard (RFC 8878), DEFLATE (RFC 1951), Brotli (RFC 7932) and FLAC (RFC 9639). The rest do not:
  - The LZ4 block format is specified only in a document inside the reference implementation's repository.
  - LZMA and LZMA2 have only the LZMA SDK `lzma-specification.txt` plus the xz file format, which lists filter IDs, not the algorithm.
  - bzip2 and PPMd var.H have no normative specification.
  - libbsc, context-mixing codecs and OpenZL have implementation-defined formats.

  PPMd, bzip2, bsc, context-mixing and OpenZL therefore fail the baseline criterion (R2 for baseline membership) and are extended-set candidates at best.
- **H1b.** Minimal decoder LoC ranks, from smallest: LZ4 block < DEFLATE (puff) < LZMA2 (xz-embedded) < Zstandard (educational decoder) < Brotli (with its 122 KiB static dictionary) < bzip2 < PPMd.
- **H1c.** Every extended candidate has a written decode-memory bound formula except context-mixing, whose memory depends on model configuration stored in the stream (paq8px) or on a VM program (zpaq).
- **H1d.** Documented encoder corruption incidents exist for Zstandard, LZ4 and zlib, each fixed in a patch release.
- **H1e.** Every compared registry reserves experimental or vendor ranges. ZIP method IDs are a single-authority numeric registry with vendor squatting, the precedent for DEC-CMP-010's numeric-range candidate.
- **Falsification.** Any primary source contradicting a prediction. Contradictions are recorded as findings.

## 4. Subjects and candidates

### 4.1 Codecs

| Subject | Primary-source anchors to retrieve (pinned) | Minimal or independent decoders to measure |
|---|---|---|
| Zstandard | RFC 8878; `facebook/zstd` `doc/zstd_compression_format.md` at v1.5.7; decodecorpus and golden test files | `doc/educational_decoder/zstd_decompress.c` (v1.5.7); `ruzstd` (pin from EXP-ECO-003) |
| LZ4 block | `lz4/lz4` `doc/lz4_Block_format.md` at v1.10.0 | `lz4_flex` block decompress module (0.14.0) |
| LZMA / LZMA2 | LZMA SDK `lzma-specification.txt` (26.x); `tukaani-project/xz` `doc/xz-file-format.txt` | `xz-embedded` LZMA2 decoder; LZMA SDK `LzmaDec.c`/`Lzma2Dec.c`; `lzma-rust2` decoder modules (0.20.0) |
| DEFLATE | RFC 1951 | `madler/zlib` `contrib/puff/puff.c`; `miniz_oxide` inflate |
| Brotli (and large-window extension) | RFC 7932; large-window draft (status) | `google/brotli` `c/dec` plus dictionary; `brotli-decompressor` |
| bzip2 | no normative specification expected | `bzip2` 1.0.8 decompression sources |
| PPMd var.H | Shkarin's sources; 7-Zip `Ppmd7.c`/`Ppmd7Dec.c` | `ppmd-rust` (1.5.0) decoder |
| libbsc | repository documentation (3.3.x) | libbsc decoder sources |
| context-mixing | paq8px and cmix repositories; zpaq specification (`zpaq206.pdf`, ZPAQL VM) | LoC, licence and float/VM audit only |
| FLAC | RFC 9639 | `claxon`; libFLAC decoder |
| pcodec, fpzip, OpenZL | repository format documents (pinned tags) | decoder LoC |

### 4.2 Transforms

| Subject | Anchors |
|---|---|
| delta8, byte-shuffle | `docs/codec-transform-v1.md` |
| xz Delta and BCJ family | `xz-file-format.txt` filter IDs; `liblzma/simple/*.c` |
| ZipNN float split | ZipNN paper and repository |
| byte-plane split | Blosc shuffle documentation |

### 4.3 Candidate mapping

- **DEC-CMP-008/009.** Every portfolio's members (EXP-CODEC-001 section 5.2) get a criteria row. A baseline-set candidate fails R2 if any member intended for the baseline fails a baseline criterion; the second-implementation criterion is relaxed for the extended set (SPEC §5.8).
- **DEC-CMP-010.** Rows for status-quo-closed-string-registry, numeric-ids-reserved-ranges, namespaced-string-ids and compile-time-plugin-trait. Runtime-dynamic-plugins and in-archive-bytecode are excluded per the ledger: `docs/codec-transform-v1.md` closed registries, and SPEC §20.7. Those exclusions are recorded, not re-derived. Two things are measured per candidate:
  - C1 normative words and wire items, counted on a drafted registry-text diff written by an uninvolved session;
  - governance facts of comparable registries: reserved ranges, allocation policy, collision history.
- **DEC-CMP-043.** Normative definition length per transform family (C1) and specification availability (R2).
- **DEC-CMP-044.** Incident rows per codec and transform backend.
- **DEC-INT-008.** CVE rows per decoder component.

## 5. Corpus

None (static). Inputs are pinned source archives and documents, retrieved by URL with retrieval date and SHA-256 (downloads approved by the PROGRESS standing decision). Storage: `/root/eb-research/src/codec-census/`, outside git.

## 6. Environment

WSL2 Ubuntu 24.04. Network is needed at retrieval time only; all later steps run offline on the pinned snapshot.

## 7. Tooling and commands

### 7.1 To build (`research/tools/codecs/census/`, stdlib Python plus pinned CLIs)

1. `fetch_sources.py`. Downloads each pinned tag, tarball or RFC text. Records URL, date, bytes and SHA-256 in `research/raw/EXP-CODEC-005/sources.jsonl`. Refuses unpinned references.
2. `loc_census.py`. Runs `tokei` at a pinned version (`cargo install tokei --version <pin> --locked`) on the explicit decoder file lists of section 4.1, committed in `decoder-file-lists.json`, excluding tests. Reports code lines per language plus `static_table_bytes` for embedded dictionaries.
3. `float_vm_audit.py`. For each decoder file list, a token scan for float types and operations (`float`, `double`, `f32`, `f64`, FPU intrinsics) and for interpreter or VM constructs. It emits every hit with `path:line` for the classifier; the scan does not decide.
4. `cve_census.py`. NVD API 2.0 queries by CPE (zstd, lz4, xz/liblzma, zlib, bzip2, brotli, 7-Zip coders) plus GHSA for the crates, with retrieval date. Each output record carries a component field for the classifier: decoder, encoder or container.
5. `incident_census.py`. GitHub REST search over the repositories committed in `incident-repos.json`: facebook/zstd, lz4/lz4, tukaani-project/xz, madler/zlib, zlib-ng/zlib-ng, google/brotli, and the Rust crates' repositories. It collects issues and release-note entries matching the committed query terms (`corruption`, `corrupt`, `incorrect output`, `data loss`, `roundtrip`, `decompression mismatch`). Two sessions classify each hit (7.3).
6. `registry_census.py`. Retrieves the IANA media-types registry, the PKWARE APPNOTE method table, OCI image-spec media types, the xz filter ID list and 7-Zip `Methods.txt` at pinned versions. Extracts the allocation policy, reserved ranges and entry counts.
7. `build_matrix.py`. Joins everything, plus EXP-ECO-001's crate table (reference by commit), into `research/normalized/EXP-CODEC-005/criteria-matrix.csv`: one row per subject × criterion, with an evidence locator. Also writes `cost-elements.csv` (C1, C4, C5, C7 per candidate), with provenance headers (§12.3).

### 7.2 Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound
PY=/root/eb-research/venv/bin/python; T=research/tools/codecs/census; R=research/raw/EXP-CODEC-005
$PY $T/fetch_sources.py --manifest $T/subjects.json --out /root/eb-research/src/codec-census --log $R/sources.jsonl
$PY $T/loc_census.py --lists $T/decoder-file-lists.json --out $R/loc.jsonl
$PY $T/float_vm_audit.py --lists $T/decoder-file-lists.json --out $R/float-vm-hits.jsonl
$PY $T/cve_census.py --cpes $T/cpes.json --out $R/cves.jsonl
$PY $T/incident_census.py --repos $T/incident-repos.json --out $R/incidents.jsonl
$PY $T/registry_census.py --out $R/registries.jsonl
$PY $T/build_matrix.py --raw $R --eco research/normalized/EXP-ECO-001 --classifications research/experiments/EXP-CODEC-005/classification --out research/normalized/EXP-CODEC-005
```

### 7.3 Judgment steps (agent sessions other than this design session)

1. Classifier A, uninvolved in the candidates, classifies each criterion cell from primary sources with `path:line` or `§` locators. The cells cover specification independence, vector existence, bound-formula existence, independent-implementation lineage, float/VM confirmation and incident classification.
2. Classifier B, a second uninvolved session, classifies independently.
3. A third session adjudicates disagreements. The agreement rate is reported.
4. Unusual licences, weak copyleft, and any uncertainty about specification independence go to the external-review dossier `research/experiments/EXP-CODEC-005/external-review-dossier.md` as `EXPERT_REVIEW_REQUIRED`.

## 8. Seeds

None. Retrieval is pinned by tag, commit or URL plus SHA-256.

## 9. Metrics

| Metric | Role | Notes |
|---|---|---|
| `baseline_codec_public_spec` | binary (HC-12) | per codec proposed for the baseline |
| `unspecified_baseline_decode_steps` | binary (HC-12) | per baseline-set candidate |
| `license_compatibility` | binary (R2, §7.5) | cross-check against EXP-ECO-001 |
| `normative_words_added`, `wire_items_added` | cost_input (C1) | per registry, transform and codec candidate, from normative text drafted by an uninvolved session |
| `untrusted_input_loc` | cost_input (C4) | decoder LoC reachable from archive bytes (file lists) |
| `reader_fragmentation_count` | cost_input (C5) | per baseline and extended set |
| minimal decoder LoC (external implementations), `static_table_bytes`, CVE counts by component, incident counts, registry governance facts | descriptive, non-canonical | proxies. `independent_reader_loc` proper comes from the conformance clean-room reader. |

## 10. Replication

Scripts are deterministic on the pinned snapshot, so a rerun must regenerate byte-identical tables (§12.3 item 4). Classification is done by two independent sessions (7.3).

## 11. Normalization

None. AGG-P per design (objective §3.0).

## 12. Analysis

- R1/R2 screens (specification, unbounded or unspecifiable decode, float or VM dependence) are reported per candidate with evidence and reviewer.
- Cost tiers follow the §7.2 rules together with EXP-ECO-001's C3 inputs and the maintenance penalty. They must agree with the §14 item 7 scripts where those exist; a disputed tier resolves to the higher tier.
- Outputs feed R4 condition 5, R6 and R10 of every informed decision.

## 13. Practical-significance thresholds

Not applicable: cost inputs carry no band (§7.6). Tiers T0-T4 and multipliers follow §7.2.

## 14. Sensitivity

Tier assignment under the "disputed resolves higher" rule, compared with classifier A alone and classifier B alone.

## 15. Held-out

Not applicable (no corpus). FORMAL and EXTERNAL parts record the reason (§1.2).

## 16. Expected negative results

- LZ4 and LZMA2 lack independent normative specifications. Their v1 baseline status therefore needs either written specifications (C1 cost) or extended-set placement.
- Brotli's static dictionary inflates minimal-reader size.
- The incident searches find few encoder corruption bugs, so a base rate cannot be estimated.

## 17. Threats to validity

- LoC is only a proxy for implementability.
- Existing "independent" implementations may share lineage; lineage is classified, not assumed.
- Keyword searches miss incidents and their recall is not measured (regression evidence).
- NVD CPE coverage is incomplete for embedded copies.
- The two classifier sessions share one base model and are correlated. Their independence is procedural, not statistical, which is why specification-independence assurance goes to external review.

## 18. Platform and cost

| Item | Value |
|---|---|
| Platforms | Linux (WSL2); network for retrieval only. External review is REQUIRED for specification-independence assurance and unusual licences; it is not available internally, so the dossier is prepared. |
| Compute | ≈ 2 CPU-h (tokei, API queries paced to rate limits) |
| Disk | ≈ 8 GB source checkouts under `/root/eb-research` |
| Agent effort | Judgment-heavy classification (two classifiers plus an adjudicator); scripted retrieval and counting |

## 19. Deviations (append-only)

None.
