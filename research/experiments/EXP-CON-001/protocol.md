# EXP-CON-001: Manifest representation and canonical record encoding at 1e3 to 1e7 entries

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: CT-1 (`ebr-eamgen`, `ebr-manifest-repr`), CT-2 counting allocator, CT-3 pack cache, CT-4 spec derivation (see `../_index/container-conventions.md` §4) |
| Domain / program section | container / §11 (with §7, §12, §13 inputs) |
| decision_ids | DEC-CON-018, DEC-CON-028, DEC-MOD-006, DEC-MOD-047, DEC-MOD-004, DEC-ACC-018 (Manifest bytes per entry and whole-vs-paged fetch bytes and memory); input to DEC-CON-002 |
| Division of labour | Container domain owns structure bytes, local read logs, parser memory, canonicality and correctness. Remote requests and latency of these structures are owned by the remote domain (`EXP-REMOTE-004`), which consumes the per-item layout files this experiment exports (section 7); `http_requests__*` keys here are structure-level cross-checks only |
| Decision type (proposal for the independent assigner, §1.2) | EMPIRICAL for bytes, memory and latency parts; FORMAL for canonicality (unique encoding per EAM) of each encoding candidate |
| OD / HC (proposal; binds nothing until ledger schema v2, gate G-A) | OD-05, OD-07, OD-08, OD-10, OD-12, OD-21, OD-23, OD-24, OD-26; HC-01, HC-03, HC-06, HC-10, HC-12, HC-16 |
| Specs | `spec.yaml` (tuning, real items, deterministic), `spec-synthetic.yaml` (tuning, synthetic models, deterministic), `spec-timing.yaml` (tuning, synthetic models ≤ 1e6 entries, in-process timing); `synthetic-items.json`; `validation-items.txt` |
| Method, author, L7, gates | `../_index/container-conventions.md` §1-§2 |
| Feasibility smoke | `feasibility-smoke/` (non-decision-grade sizing only) |

## 1. Question

Which physical manifest representation (flat, compact-framed, whole-section compressed, segmented, paged, path-indexed, columnar) and which canonical record encoding (bootstrap TLV with u64 framing, varint TLV, deterministic CBOR, DER, NAR-style) minimize manifest bytes per entry, the bytes and requests needed to open, look up one path and list, and parser memory, from 1e3 to 1e7 entries, while every candidate decodes to exactly the canonical EAM record set, and at what deterministic trigger (if any) a non-default representation should activate?

## 2. Hypotheses

- **H1 (size, DEC-CON-018, DEC-MOD-006).** On the entry-heavy stratum (items with ≥ 1,000 entries) and on every synthetic stratum ≥ 1e5 entries, at least one alternative is `DISTINGUISHABLE_BETTER` than `m0-flat-tlv-u64` on `metadata_bytes_per_entry` beyond the minimum-gain band of its computed tier (§7.3), and `NON_INFERIOR` on `alloc_peak_bytes__open`. Predicted effect: compact framing (m1) about 2-4 band units, whole-section compression (m2) more than 10 band units, because the status quo spends fixed 16-byte record and 12-byte field headers with u64 lengths per field (`docs/format-v0.md` framing) and the feasibility smoke shows roughly 1.3 KiB of manifest per tiny entry (non-decision-grade sizing).
- **H2 (lookup and list).** At ≥ 1e5 entries, paged (m4) or segmented (m3) representations reduce `open_bytes_read__lookup_mean` and `http_requests__open_lookup` by more than 1 band unit of T-11 and T-12 against m0, which must fetch the whole manifest, while `open_bytes_read__list_all` is `NON_INFERIOR`.
- **H3 (stored digests, DEC-MOD-047).** Removing stored Entry identity and AUX digests reduces `metadata_bytes_per_entry` by at least 64 B per entry (exact, `FORMALLY_DERIVED` from field widths) but raises full-reader recomputation cost; the remote list/inspect saving of stored digests is measured by EXP-REMOTE-004 on the exported layouts.
- **H4 (order, DEC-MOD-004).** For paged representations, the per-component length-first order (o0) touches no more pages for a single-directory listing than bytewise order (o1) within 1 band unit, while parent-before-child (o4) touches fewer; the streaming reorder buffer from a readdir walk is largest for o5 (hash order).
- **H5 (transparency, DEC-CON-028).** m2, m3 and m6-fsst recover under 1% of path components by `strings -n 4`; m0, m1 and m4 recover over 95%.
- **H0.** Every alternative is `EQUIVALENT` to m0, or `INCONCLUSIVE` with demonstrated power (R10), on every primary metric in every stratum.
- **Falsification of H1-H2.** No alternative is `DISTINGUISHABLE_BETTER` beyond its minimum-gain band on the primary metric of its OD in any entry-count stratum, or every such alternative fails a correctness screen (section 9).

## 3. Decisions informed

| Decision | What this experiment supplies | What remains elsewhere |
|---|---|---|
| DEC-CON-018 | Bytes per entry, open/lookup/list bytes (exact read logs), parser memory and in-process latency per representation and stratum; correctness and canonical re-encode screens; exported layouts | Remote requests and latency: EXP-REMOTE-004 (remote domain); production latency anchor: EXP-CON-005; production memory at scale: EXP-SCALE-001; byte census: EXP-SCALE-003; differential fuzzing and independent-reader effort: conformance domain (§28-30); encrypted single-unit manifest interplay: EXP-CON-005 encrypted cell (latency) and EXP-SCALE-010 (scale) |
| DEC-ACC-018 | Manifest bytes per entry and the local bytes and memory of whole versus paged or segmented fetch, at 1e3-1e7 entries | Discovery and revalidation strategies, cold/warm request counts and latency vs RTT: EXP-REMOTE-004; server behaviour: EXP-REMOTE-009, EXP-REMOTE-013 (BLOCKED) |
| DEC-CON-028 | Manifest share of `artifact_bytes` against entry count; the transparency fraction per representation; deterministic evaluation of every trigger candidate on the same outputs (section 10.4) | Reader-compatibility cost of the incompat bit: EXP-CON-007 (feature-tier analysis, reader fragmentation); remote bytes and requests per trigger: EXP-REMOTE-004 |
| DEC-MOD-006 | Bytes, encode/decode CPU and memory, streaming-emission feasibility for each encoding | Independent-reader ambiguity log and differential fuzzing: conformance domain; historical stored-bytes corpus: EXP-CONF-006; migration-cost inventory: EXP-CON-008 |
| DEC-MOD-047 | Exact byte cost of stored digests per entry at every stratum; full-reader recomputation CPU | Remote list/inspect request savings: EXP-REMOTE-004 consumes the `-nodigests` layouts |
| DEC-MOD-004 | Pages touched per listing and streaming reorder-buffer size per order candidate | EBCS sort-key equivalence test: crypto domain (§22.5) |
| DEC-CON-002 | Manifest-structure input to the revision synthesis | EXP-CON-008 |

## 4. Candidates

All candidates encode exactly the same EAM (same entries, ContentObjects, metadata and identities); only the physical representation differs. Each non-status-quo representation has a written byte-level definition committed as `representations.md` in this directory before the first run (tooling task CT-1b); a candidate without it is not run.

| candidate_id (spec) | Ledger candidate | Definition |
|---|---|---|
| `m0-flat-tlv-u64` | flat-uncompressed-status-quo; bootstrap-tlv-status-quo | Production MANIFEST_RECORDS bytes. The research encoder must reproduce production bytes exactly (validity gate G1, section 9) |
| `m0-flat-tlv-u64-nodigests` | derive-only (DEC-MOD-047) | m0 without Entry fields identity digest and auxiliary digest; readers derive them |
| `m1-flat-tlv-varint` | flat-with-compact-field-framing; tlv-varint-lengths | Same records and tags; canonical minimal LEB128 for lengths, counts and tags |
| `m1-flat-tlv-varint-nodigests` | derive-only combined with m1 | as named |
| `e2-deterministic-cbor` | deterministic-cbor | RFC 8949 §4.2.1 core deterministic encoding, integer map keys |
| `e3-asn1-der` | asn1-der | DER SEQUENCE per record, context tags per field |
| `e4-nar-padded` | nar-style-padded | Length-prefixed, 8-byte aligned strings, grammar-mandated order |
| `m2-compact-zstd3`, `m2-compact-zstd19` | compact-manifest-whole-section-compressed | m0 bytes compressed as one zstd frame (production-pinned `zstd =0.13.3`, window 1 MiB, levels 3 and 19) |
| `m3-segmented-zstd3-64k`, `-256k`, `-1m` | segmented-compressed-manifest | m0 record stream cut at record boundaries into segments of at most the named uncompressed size, each an independent zstd level-3 frame; seek table of (first LogicalPath, compressed offset, compressed length, uncompressed length, entry ordinal) |
| `m4-paged-16k`, `-64k`, `-256k` | paged-manifest-with-locator-table | Uncompressed m0 records packed into pages of at most the named size, page directory keyed by first LogicalPath per page |
| `m5-path-fanout-table` | path-indexed-tree (fanout variant) | m0 plus a 256-entry fanout over SHA-256(LogicalPath) and a sorted (path hash prefix 8 B, record offset u64) table |
| `m5-path-btree-256` | path-indexed-tree (B-tree variant) | m0 plus a static B-tree of fanout 256 over LogicalPath to record offsets |
| `m6-columnar-frontcoded` | columnar-bitpacked-manifest | DwarFS-Frozen2-style columns: front-coded component string table, bit-packed kind/mode/uid/gid, delta-coded mtimes, digests column; schema-declared bit widths |
| `m6-columnar-fsst` | columnar-bitpacked-manifest (FSST names) | as m6 with FSST-compressed string table (FSST reference algorithm reimplemented in Rust; symbol table stored) |

Order candidates for DEC-MOD-004 are analysis dimensions, reported as `desc_*__o0..o5` within each sample: `o0` length-first (status quo), `o1` bytewise-lexicographic components, `o2` Git flat-index order, `o3` encoded-byte order, `o4` depth-first parent-before-child, `o5` identity-digest order. They do not change the manifest bytes measured for other metrics (all byte metrics use o0, the frozen order).

Trigger candidates for DEC-CON-028 (`always-uncompressed-status-quo`, `profile-dependent`, `entry-count-threshold` at 1e4/1e5/1e6, `manifest-share-threshold` at 5%/10%/25%, `always-compact`, `uncompressed-plus-prologue`, `caller-opt-in-only`) are evaluated deterministically in section 10.4 from the outputs of the representation runs; `uncompressed-plus-prologue` is modelled as m2 plus a UTF-8 prologue of every LogicalPath component list truncated to 64 KiB.

**Critic-proposed candidate (R0 item 6).** Left to the completeness critic; any added candidate enters at R1 with its own `representations.md` definition before its first run.

**Exclusions.**
- `external-manifest-sidecar`: excluded before execution (ledger exclusion; a `Complete` archive must be self-contained, SPEC §13.5 L1551 and I1). Reviewer sign-off per R1(b) is recorded by the completeness critic.
- `protobuf-deterministic` and `flatbuffers-or-capnp-canonical` (DEC-MOD-006): excluded before execution (ledger exclusions, I15: not canonical across implementations without an extra canonicalisation, which would itself be a different candidate).

Expected cost tiers (assessor computes, §7.2): m0-nodigests, m1, e2-e4, m6: T4 (new canonical encoding of an identity-bearing record set) unless identities are shown unchanged; m2, m3, m4, m5: T3 (new section type or incompat feature) or T4 if they change the container structure. HC-16: every candidate is a new identifier; no in-place change of frozen Entry or Descriptor bytes is a candidate.

## 5. Corpus selectors

- **Tuning, real (`spec.yaml`):** every tuning item in `research/corpus/manifest.json` (112 items, 20 families, 89 independence groups), packed by `ebound-prod` with REF-PROD settings (balanced, INDEXED, unencrypted) through the pack cache. Items production refuses (for example non-UTF-8 names, `EB_EAM_INVALID_PATH_COMPONENT`) are recorded as `pack_refused` with the reason code and excluded from representation metrics; the refusal list is reported.
- **Tuning, synthetic (`spec-synthetic.yaml`):** the 42 tuning rows of `synthetic-items.json`: models fitted on tuning items F04 `f04-tuning-real-netbsd-pkgsrc`, F01 `f01-tuning-llvm-project-20-1-8-git`, F03 `f03-tuning-typescript-npm-node-modules`, F17 `f17-tuning-real-ubuntu-kernel-headers-2ver`, F19 `f19-tuning-rsnapshot-a` (posix metadata at 1e3, 1e4, 1e5, 1e6, 4e6, 1e7 entries), plus rich (xattr/ACL, fitted on `f19-tuning-real-ext4-acl-selinux`) and Windows (security descriptors and attributes, fitted on `f19-tuning-real-ntfs-metadata`) metadata for the F04 and F19 models at 1e4, 1e6, 1e7. `ebr-eamgen fit` records per-model distributions: path depth, component length and alphabet, siblings per directory, kind mix, chunk count per file (per profile chunker), metadata item presence and value entropy. Chunk plaintexts are 8-16 byte seeded values so that digests are real and payload is negligible.
- **Timing (`spec-timing.yaml`):** synthetic tuning models ≤ 1e6 entries (larger strata are deterministic-only because a timing round would exceed the §4.6 cap budget).
- **Validation look (derived by CT-4):** `validation-items.txt`: every validation item plus the 42 validation synthetic models fitted on validation items (gentoo, CPython, bootstrap node_modules, Fedora kernel-devel, rsnapshot-b; rich on `f19-validation-real-xfs-acl-selinux`, Windows on `f19-validation-real-ntfs-metadata`).
- **Minimum support:** real tuning 89 groups/20 families; entry-heavy stratum 23 groups/12 families (tuning), 19/9 (validation). Synthetic strata are an evaluation condition (AGG-S), never pooled at L4.
- **Held-out:** none (section 15).

## 6. Environment and platform requirements

- `wsl-ubuntu` (x86-64, ext4 under `/root/eb-research`). Deterministic metrics are platform-independent byte counts; a 10% seeded subset is re-run on `windows-host` with the harness Windows build to confirm hash identity of `desc_payload_sha256` (cross-host determinism check, §4.13).
- Timing arm: `wsl-ubuntu` `st-v` = {2} (`VIRTUALIZED`), plus the Windows host `st-p` = {2} replicate derived by CT-4 (`--env windows-host`) for universal-scope claims (§4.1).
- Memory: at 1e7 entries the production cross-check is infeasible in 31 GiB WSL (smoke: about 9 KiB RSS per entry for production pack), so `--production-crosscheck-max-entries 1000000`; above it the research encoder runs streaming and the validity gate G1 relies on the byte identity proven at ≤ 1e6.
- Platform matrix: Linux x86-64 required; Windows x86-64 for the timing replicate and hash cross-check; macOS and native ARM64 not in scope for this experiment (ARM64 open latency and memory: EXP-CON-009, BLOCKED); no privileged operations; storage about 120 GB peak (section 16).

## 7. Commands

```sh
# 0. identities and guards (conventions §3)
python research/harness/lock_parity.py && bash research/harness/harness-cli-parity.sh
C:/Python313/python.exe research/orchestration/disk_guard.py
# 1. fit and materialize synthetic models (tuning)
/root/eb-research/target/harness/release/ebr-eamgen fit --source /root/eb-research/corpus/tuning/<source_item_id> --out research/experiments/EXP-CON-001/models/<key>.json
/root/eb-research/target/harness/release/ebr-eamgen model --model research/experiments/EXP-CON-001/models/<key>.json --entries <N> --meta <meta> --seed <seed> --out /root/eb-research/generated/EXP-CON-001/<item_id>
python research/tools/container/fingerprint_generated.py --manifest research/experiments/EXP-CON-001/synthetic-items.json --write   # fills fingerprint_sha256; committed before step 2
# 2. deterministic passes
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CON-001/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CON-001/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CON-001/spec-synthetic.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-CON-001/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-CON-001/spec.yaml
# 3. deterministic screen (R1, R2, deterministic R4) and timing filter
python research/tools/decide/screen.py --experiment EXP-CON-001 --out research/normalized/EXP-CON-001/decision/screen.json   # §14 item 2 tooling
python research/tools/container/derive_spec.py --spec research/experiments/EXP-CON-001/spec-timing.yaml --candidates-from research/normalized/EXP-CON-001/decision/screen.json --out research/experiments/EXP-CON-001/spec-timing.filtered.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CON-001/spec-timing.filtered.yaml --timing
# 4. decision analysis (conventions §8) and report
python research/tools/decide/analyze.py --experiment EXP-CON-001 --record research/experiments/EXP-CON-001/record.md
/root/eb-research/venv/bin/python -m ebr report --spec research/experiments/EXP-CON-001/spec.yaml
```

Tooling to build for this experiment: CT-1a `ebr-eamgen` (`fit`, `model`), CT-1b `ebr-manifest-repr`, CT-2, CT-3, CT-4, `research/tools/container/fingerprint_generated.py`, `representations.md` (byte-level definitions), and a pinned FSST port inside CT-1 (reference: Boncz, Neumann, Leis, "FSST: Fast Random Access String Compression", PVLDB 13(11), 2020).

`ebr-manifest-repr` contract (one JSON line): for the selected representation it (a) re-encodes the manifest from the production-decoded EAM (or synthetic model), (b) decodes it back and compares the canonical record multiset and order with the EAM (`roundtrip_mismatches`), (c) for m0 compares bytes with production MANIFEST_RECORDS (`reader_disagreements`, gate G1), (d) simulates the pre-registered read plans below over an in-memory byte source that logs every (offset, length) read, and (e) measures `alloc_peak_bytes` per phase with CT-2, and (f) writes the representation's exact layout (every directory, page, segment and record extent with purpose labels) to `research/normalized/EXP-CON-001/layouts/<item_id>/<candidate>.json`, the hand-off consumed by EXP-REMOTE-004.

Read plans (identical for every candidate; the reader reads only what the representation's own directory structures require):
- `open_lookup`: fetch the minimum structures to resolve one seeded random LogicalPath to its Entry and ContentObject records (1,000 seeded paths per item; mean and max over paths).
- `list_dir`: list the direct children of 100 seeded directories.
- `list_all`: materialize every Entry (as `ebound list`).
- Requests are counted from the read log under the declared connection model of `EXP-REMOTE-001/connection-model.json` (contiguous reads within one plan step coalesce only if the representation's directory lets the reader know the extent in advance).

## 8. Seeds, warmups, repetitions

- Seeds: `spec.yaml` order 11001, bootstrap 11002, command 11003; `spec-synthetic.yaml` 11011-11013; `spec-timing.yaml` 11021-11023. The command seed selects lookup paths and listing directories; model seeds are in `synthetic-items.json`.
- Deterministic passes: 2 repetitions, repeat-hash of the JSON payload (`desc_payload_sha256`) and of each encoded representation; a mismatch is an R1 elimination artifact for the candidate (every candidate claims determinism).
- Timing: warmups 2; rounds per §4.6 tier table with the tier assigned by entry-count stratum (1e3-1e4 small, 1e5-1e6 medium); adaptive rule §4.6 on each primary timing metric; in-process batches of 1,000 lookups per round (T-08).

## 9. Measurement method and metrics

| Metric (canonical name or `desc_`) | OD | Role in this experiment | Family | T-ID | Source |
|---|---|---|---|---|---|
| `metadata_bytes_per_entry` | OD-05 | **primary** (DEC-CON-018, DEC-MOD-006, DEC-MOD-047) | size | T-02b | exact encoder output |
| `metadata_bytes` | OD-05 | diagnostic (manifest component) | size | T-02 | exact |
| `artifact_bytes` | OD-05 | non-inferiority guard | size | T-01 | archive bytes with the representation substituted (other sections unchanged) |
| `open_bytes_read__lookup_mean` | OD-10 | **primary** (DEC-CON-018 lookup) | size | T-11 | read log |
| `open_bytes_read__list_all` | OD-07 | guard | size | T-11 | read log |
| `http_requests__open_lookup` | OD-12 | secondary cross-check (the OD-12 primary belongs to EXP-REMOTE-004) | other | T-12 | read log under connection model |
| `alloc_peak_bytes__open`, `__list_all` | OD-08 | **primary** `__open`; `__list_all` guard | memory_peak | T-06 | CT-2, exact |
| `open_latency_s`, `list_latency_s`, `random_entry_latency_s__lookup` | OD-07, OD-10 | secondary until the timing filter; primary for OD-07 only if no byte metric separates survivors | time_wall | T-08 | in-process timers, batches |
| `encode_wall_s__manifest_in_process` | OD-06 | secondary (streaming emission cost) | time_wall | T-03 | in-process |
| `roundtrip_mismatches` | OD-02 | binary screen (R1) | correctness | HC-02 | decode vs EAM |
| `reader_disagreements` | OD-04 | binary screen (gate G1 for m0 vs production) | correctness | HC-03 | byte compare |
| `desc_transparency_path_recovery_fraction` | OD-26 | descriptive (DEC-CON-028 transparency) | other | none | fraction of path components (≥ 4 bytes) found by a `strings -n 4` scan of the archive bytes |
| `desc_pages_touched_list_dir__o0..o5`, `desc_reorder_buffer_entries__o0..o5` | OD-10, OD-09 | descriptive (DEC-MOD-004) | other | none | read log; readdir-order walk of the fitted model |

**Screens before any comparison (R1/R2).** (a) `roundtrip_mismatches = 0`; (b) gate G1: `m0-flat-tlv-u64` bytes equal production MANIFEST_RECORDS on every item ≤ 1e6 entries (else the whole prototype is invalid, not the candidate); (c) canonicality counterexample search (section 13) finds no second accepted encoding of one EAM; (d) R2 boundability: every decoder's allocation is bounded by a written formula in declared counts, checked against CT-2 on every item including F20; (e) HC-10 identity effect: LAI, AUX and PCR recomputed from the decoded EAM equal the production identities (a candidate changing any identity root is recorded as an identity-participation change, cost T4, not eliminated).

## 10. Normalization and statistical analysis

1. **Units.** Deterministic item values (§4.13); log ratios against `m0-flat-tlv-u64` per item; L2 group means; L3 family means; L4 with `research/methods/workload-mix.json` weights (§4.9). Absolute floors at item level (§3.1): for example an item whose manifest differs by at most 0.5 B/entry contributes log ratio 0 on T-02b.
2. **Strata and conditions.** Scale tiers (small/medium/large) and entry-count strata (real: <1e3, 1e3-1e4, ≥1e4 entries; synthetic: each N) are strata; metadata model (posix/rich/windows) is an evaluation condition (AGG-S). The R4 condition-3 guard applies per stratum and condition.
3. **Tuning.** R4 without multiplicity control forms S and W per scope; W is the continuous band-unit utility winner (§6.2) with per-OD base weights equal over {OD-05, OD-08, OD-10} (OD-12 enters the joint decision through EXP-REMOTE-004's record); cost tiers enter at R4 condition 5, R6 and R10 only.
4. **DEC-CON-028 triggers.** For each trigger candidate the activation decision per archive is computed deterministically from pre-pack observables (entry count, profile, manifest share of the m0 archive); the resulting per-item `metadata_bytes_per_entry`, `open_bytes_read__lookup_mean` and `desc_transparency_path_recovery_fraction` are aggregated as a derived candidate. A trigger that depends on a quantity not knowable before writing the manifest (for example the final manifest share in a single-pass STREAM writer) is flagged `not-single-pass` (formal check), recorded as a cost, not eliminated.
5. **Confirmatory.** Validation look #1 via CT-4 on W and S; superiority confidence `1 - 0.01/k_sup`, non-inferiority 0.99, family harm guard 0.90; MDE gate ≤ 1 band unit computed from tuning group variance and the validation group structure before the look (§4.12). Synthetic strata use the exact deterministic values per N; their "groups" are the source models (5 per split), so synthetic strata support scoped claims per entry-count condition only, never a corpus-level verdict.
6. **R6/R8/R9.** Minimum gain per computed tier (§7.3); universal default only per R8; otherwise R9 partitions in the order profile, policy (INDEXED/STREAM, plain/encrypted), workload mix, family, with expressibility checked (an entry-count trigger is a policy-level partition, cost at least T1).

## 11. Practical-significance thresholds

Exactly the `metric_thresholds` entries of `research/methods/thresholds.json` for T-01, T-02, T-02b, T-03, T-06, T-08, T-11 and T-12 (canonical metric = the prefix before `__`). Minimum-gain bands: `[(1+r)^-k, (1+r)^k]` with k from the computed tier (§3.1, §7.3, Appendix D.6). Nothing is restated here.

## 12. Sensitivity analysis

All defaults of conventions §8, plus: (a) profile arm: re-run the deterministic pass for the synthetic F04/F19 models and 20 seeded real tuning items with `dense` and `fast` chunking (ContentObject chunk-reference counts change); (b) metadata-model arm (posix vs rich vs windows) reported as conditions; (c) byte-weighted arm on `metadata_bytes`; (d) read-plan arm: lookup sample drawn Zipf(1.0) over paths instead of uniform (`--lookup-distribution zipf`), to test that paged wins are not an artefact of uniform lookups; (e) page and segment size arms are separate candidates (no tuning of sizes on validation, §5.2).

## 13. Adversarial search and expected negative results

- `adversarial-search/` (§8.2 item 5): (i) **canonicality counterexample search** per encoding candidate: a generator enumerates alternative encodings of 10,000 seeded small EAMs (non-minimal varints, alternative CBOR float/int forms, DER length forms, padding bytes, segment/page boundary shifts, duplicate directory entries) and records any byte string that the candidate's decoder accepts and that is not the canonical encoding; any hit eliminates the encoding at R1(b) unless the definition is revised as a new candidate version; (ii) **worst-case inputs** for bytes per entry: deepest paths, maximal xattr/ACL sets, 255-byte components, many hardlink groups; (iii) inputs where compressed manifests expand (random component names).
- Negative results worth recording: compression gains that vanish on rich-metadata or random-name models; paged manifests that lose on `list_all`; varint framing gains below the T4 minimum gain; hash order (o5) breaking streaming emission; every representation whose transparency collapses.

## 14. Threats to validity

- **Prototype quality.** Alternatives are research encoders, not production code; the status quo is re-implemented in the same framework and must match production bytes (G1), and timing compares prototypes only with each other; production absolute latency comes from EXP-CON-005.
- **Synthetic realism.** Models resample path and metadata statistics of single source items; correlations (for example between names and metadata) may be lost. Mitigation: real items dominate L4; synthetic strata support only entry-count-scoped claims; the fitted distributions are committed.
- **Read-log model.** Bytes and requests are exact for the declared reader behaviour; a real reader might read more (implementation overhead) or cache across operations (covered by EXP-REMOTE-004 `WL-REVISIT`).
- **Allocator determinism.** CT-2 peaks are assumed deterministic single-threaded; the repeat-hash check covers the JSON payload including peaks, so nondeterminism is detected.
- **Chunking dependence.** ContentObject sizes depend on the chunker and profile (sensitivity arm a).
- **Canonical order is frozen** for identity (HC-16); order candidates other than o0 can only be new identity profiles (T4); the experiment measures locality, not re-ordering of existing archives.
- **L7 exposure** of the designer session (conventions §1).

## 15. Phase D held-out placeholder

After Commit C (§5.5), CT-4 derives `spec-heldout.yaml` from the frozen record (W, S, the synthetic-model rule fitted on held-out sources selected by the frozen lock, never named here), runs once under `EB_HELDOUT_UNLOCK`, and applies R5 with the held-out MDE computed beforehand from validation group variance and the lock's group counts.

## 16. Cost and effort

- Compute: real deterministic pass about 10 h (dominated by packing 36 GiB of tuning items once; the pack cache is shared with EXP-CON-002/003/004/006/007/011), synthetic deterministic pass about 12 h (19 candidates × 42 models × 2 repetitions; 1e7-entry models streaming), timing arm about 20 h WSL plus 20 h Windows host; validation look about 25 h. Total about 90 machine hours.
- Disk: pack cache about 40 GB (shared); synthetic models about 60 GB transient (1e7-entry m0 manifests about 13 GB each are streamed and discarded after hashing); peak about 120 GB on the D:-hosted WSL disk; raw JSONL under 1 GB.
- Agent effort: judgment for `representations.md` review and the tuning/validation analysis records; everything else scripted.
