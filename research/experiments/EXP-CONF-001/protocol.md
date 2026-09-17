# EXP-CONF-001 — Clean-room independent minimal reader built from the written specification, with provenance control and ambiguity log

| Field | Value |
|---|---|
| Status | `NEEDS_TOOLING` (CC§2): clean-room snapshot builder, access/provenance logger, transcript audit script, ambiguity-log validator, LoC counter pins |
| Domain / program sections | conformance / §28 (primary), §29 (feeds corpus), §31 (reader dependency cost inputs) |
| Decision IDs informed | DEC-ECO-032, DEC-ECO-038, DEC-CMP-008, DEC-CMP-038, DEC-CON-002, DEC-CON-003, DEC-CON-012, DEC-CON-018, DEC-CON-026, DEC-CON-029, DEC-CON-030, DEC-CRY-020, DEC-CRY-047, DEC-ECO-012, DEC-ECO-045, DEC-INT-030, DEC-INT-042, DEC-LEG-003, DEC-LEG-004, DEC-LEG-034, DEC-MOD-003, DEC-MOD-004, DEC-MOD-006, DEC-MOD-018, DEC-MOD-042, DEC-PLT-015, DEC-PLT-050 |
| Requirements | REQ-ECO-0133 (Task 28 obligation), REQ-ECO-0030, REQ-ECO-0072, REQ-ECO-0093, REQ-ECO-0095 |
| HC oracles | HC-12 MVT-12(a) (with EXP-CONF-002), MVT-12(c); proposal for the assigner: `hc_ids` HC-12, HC-03; `od_ids` OD-21, OD-23 |
| Decision type | not assigned (gate G-A); this protocol does not choose it |
| Method / pre-registration | `research/decision-method.md` @ `14b977c`; this protocol is the design pre-registration (CC§3); `record.md` is instantiated after G-A |
| Author / exposure | Phase C-design conformance session; L7 disclosure and consequences in CC§1 |
| Depends on | nothing for construction; EXP-CONF-002 consumes the frozen reader; EXP-CONF-003 supplies the normative-word count used for `spec_defects_per_1000_words` |

## 1. Question

Can a session with no access to Entrybound source code implement a conforming baseline decoder (INDEXED and STREAM
layouts, unencrypted, baseline codec and transform set, digest verification, identity roots, outcome classes with
reason codes) from the SPEC, the repository's normative documents and the permanent vectors alone; where does the
written specification fail to determine behaviour; and how much reader code and how much reader fragmentation does
each feature level impose?

## 2. Hypotheses and falsification

- **H1 (implementability, HC-12 MVT-12(c)).** Every baseline decode step is implementable from public normative text
  and vectors. *Falsified* by one baseline step logged `UNSPECIFIED_DECODE_STEP` and confirmed by the adjudicator
  (`unspecified_baseline_decode_steps` > 0).
- **H2 (outcome determinacy).** For every negative condition in baseline scope the written text determines the outcome
  class and the reason code. *Falsified* by one confirmed `UNDOCUMENTED_REASON_CODE` or `MISSING_RULE` entry of
  severity S1 or S2 (§7).
- **H3 (codec provenance, MVT-12(c), objective M22.5).** Each baseline codec (`store/v1`, `zstandard/v1`, `lz4/v1`,
  `lzma2/v1`) and transform (`delta8/v1`, `byte-shuffle/v1`) is decodable from a public format definition plus the
  repository's parameter records. *Falsified* per codec by a decode requirement or parameter that is defined only by
  a library function or crate version (`baseline_codec_public_spec` = 0 for that codec).
- **H4 (fragmentation).** A reader at level L-base (§4) reads every archive the production CLI writes with default
  options on WSL ext4 and on the Windows host. *Falsified* by any such archive refused `UNSUPPORTED` for a feature
  outside L-base (`reader_fragmentation_count` > 0), measured in EXP-CONF-002 on the frozen reader.

No effect-size hypothesis applies: every quantity is a deterministic count over a design (AGG-P).

## 3. What this experiment contributes per decision

| Decision | Contribution |
|---|---|
| DEC-ECO-032 | Executes candidates `clean-room-other-language-baseline-decoder` and `agent-clean-room-reader-with-ambiguity-log` (one construction satisfies both); ambiguity-log counts and severities; independence audit; reader size versus reference decoders |
| DEC-ECO-038 | LoC per conformance level and per section obligation; dependency count per level; which sections a minimal reader can skip with a reported verification scope (candidates `reader-conformance-levels`, `source-size-budget`, `dependency-count-budget`, `binary-size-budget` measured as stripped `ebir` binary bytes per level) |
| DEC-CMP-008 | Decoder LoC and external-decoder LoC per codec; H3 per codec |
| DEC-CMP-038, DEC-INT-030 | Time-boxed determination whether `deflate-reconstruct` (planner v5) and v6 JPEG reconstruction are implementable from documents (L-gated) |
| DEC-CON-002, -003, -012, -018, -026, -029, DEC-MOD-004, -006, -018, -042, DEC-CRY-020, -047, DEC-MOD-003, DEC-PLT-015, -050, DEC-LEG-003, -004, -034, DEC-INT-042 | Ambiguity entries tagged with the decision's feature area (section numbering and registries; version gate; manifest representation; STREAM record-set equivalence; normative-versus-experimental status; comparator; TLV encoding; EAM object list; record registry; EBCS versus LogicalPath order; `max_key_derivation_cost`; empty link target and tag 6; FidelityReport; timestamp scale; provenance types 28-29; preservation types 30-36; verification-state model) |
| DEC-CON-030 | LoC of the platform-security-metadata-v1 module and the fragmentation effect of reading without it (H4) |
| DEC-ECO-012 | Structured usability notes from the builder on vector files (unlabelled fields, REQ-ECO-0039) — `SIMULATED`-style heuristic evidence only |
| DEC-ECO-045 | Count of ambiguity entries caused by doc-to-doc inconsistency (category `CONFLICTING_TEXT`) |

## 4. Candidates

This experiment builds an instrument; the candidates it informs are the ledger's (CC§1: none added).

- **DEC-ECO-032 candidates.** `status-quo-none` is the reference (HC-12 stays `HC_UNVERIFIED`). Executed:
  `clean-room-other-language-baseline-decoder` together with `agent-clean-room-reader-with-ambiguity-log` (arm A, Go),
  plus a second, narrower clean-room reading (arm B, Python, framing only) that measures reading reliability of the
  framing rules. Not executed here, with reasons: `commissioned-external-full-reader` → EXP-CONF-013 (BLOCKED: no
  external implementer); `clean-room-rust-separate-author` → no separate human author exists, and an agent-built
  Rust reader would share the crate ecosystem (`sha2`, `zstd-sys`/libzstd, `lz4_flex`, `lzma-rust2`) that production
  uses, which defeats the independence the candidate is meant to buy; `same-team-minimal-reader-crate` → not
  independent by construction, its size is estimated from production reader LoC by the §14 item 7 C2 counter only;
  `post-v1-independent-implementation` → no measurement exists before v1 (its add-later cost is recorded under C7).
- **Language choice (pre-registered rubric, analytical, `[INFERRED]`).** Rust is excluded (production language).
  Scored for Go 1.22.2, Python 3.12.3, Java 21.0.12, Node 22.23.2 (all installed in WSL):
  1. different language and toolchain from production;
  2. baseline codecs decodable without any library production uses on its decode path (production: libzstd via
     `zstd-sys`, `lz4_flex`, `lzma-rust2`, `sha2`);
  3. memory safety;
  4. maintained pure implementations exist: Go `github.com/klauspost/compress/zstd` (dictionaries and raw-content
     prefixes), `github.com/pierrec/lz4/v4` (raw block), `github.com/ulikunitz/xz/lzma` (raw LZMA2 reader), `crypto/sha256`;
     Python's only mature zstd binding (`zstandard`) wraps libzstd, the same library as production, failing (2);
     Java passes (2) with `aircompressor` and `xz-java`; Node has no maintained pure decoder with dictionary support;
  5. in-process batch throughput for ~10^8 mutated inputs (EXP-CONF-002) and native coverage-guided fuzzing
     (`go test -fuzz`) for EXP-CONF-010.
  Go satisfies all five; Java is the pre-registered fallback if a required Go module is unavailable. Python is used for
  arm B only, whose framing scope needs no codec.

## 5. Inputs

- **Clean-room snapshot** `/root/eb-research/cleanroom/snap-<head12>/`, built by
  `research/independent-reader/cleanroom/make_cleanroom.sh --commit <dev-sha> --spec /mnt/d/Projects/entrybound/design/2026-08-29-entrybound-product-architecture.md --spec-sha256 1f881c7b13f193b41353b90066d33ca6abcb7d4c3dcd4f58e22834801f0fe29c`:
  contents = `docs/*.md`, `docs/*-vectors*.txt`, `README.md`, `CONTRIBUTING.md` (from `git archive` of the recorded
  commit), and the SPEC copy (hash-verified; kept outside git, never committed). Excluded: `crates/`, `tools/`
  (including `tools/crypto-vector-helper/reference.py`, a same-project reference implementation), `research/`
  (ledgers cite code by line), `Cargo.*`, `target/`, git metadata. The script writes
  `research/independent-reader/cleanroom/snapshot-manifest.json` (path, bytes, SHA-256 per file) and mounts the
  snapshot read-only for the builder.
- **No conformance corpus** during phase A1 (§7): the builder never sees `ebcc-v1` expectations, so ambiguities are
  resolved by reading, not by fitting to expected outcomes.
- **Pinned third-party modules**, downloaded once through the Go module proxy and frozen in `go.sum`: klauspost/compress,
  pierrec/lz4/v4, ulikunitz/xz; versions are chosen by the builder and recorded before the first decode test.

## 6. Environment, platform and provenance controls

- WSL2 Ubuntu 24.04 x86-64, Go 1.22.2 (`CGO_ENABLED=0`), Python 3.12.3 (arm B, standard library only). No privileged
  operation. Network only for the one-time module download (GOPROXY), then `GOFLAGS=-mod=readonly`.
- **Session isolation (CC§11).** Arm A and arm B are separate fresh agent sessions. Their instructions permit reading
  only the snapshot and module documentation, forbid web requests whose URL contains `entrybound`, and forbid reading
  any other path.
- **Access log.** `research/independent-reader/cleanroom/run_builder.sh` starts each builder session with the snapshot
  as its working directory and records file accesses of child processes with `strace -f -e trace=%file -o` for every
  build/test command; the session transcript is exported after each session.
- **Transcript audit.** `research/independent-reader/cleanroom/audit_transcripts.py --allow /root/eb-research/cleanroom/snap-* --allow <GOMODCACHE> --allow research/independent-reader --forbid-url-substring entrybound`
  scans every builder transcript and strace log. Any read of a forbidden path or URL is a provenance violation: the
  modules written after it are labelled `NOT_CLEANROOM`, excluded from H1-H3 evidence, and reported.
- **Provenance log** `research/independent-reader/PROVENANCE.md`: per session: session id, start/end UTC, snapshot
  hash, documents consulted (from the access log), commits produced.

## 7. Procedure

1. **Phase A1 (specification only), arm A (Go `ebir`).** Implement levels in order, each module in its own package:
   - **L-base**: preamble and magic, version gate, Descriptor v1 and v2 (unencrypted), feature bitmaps and fail-closed
     handling, INDEXED sections, Index as a non-authoritative cache (rebuild when absent or invalid), STREAM layout v1,
     canonical records and EAM (entries, ContentObjects, Chunks, groups and lookback), codecs and transforms of H3,
     chunk SHA-256 verification, LAI/PCR/AUX/PCI and Merkle roots, outcome classes, reason codes and the UNVERIFIED
     qualifier, ResourceBudget and DecodeRequirements validation against a caller policy, extraction to an in-memory
     byte tree (no filesystem metadata restoration).
   - **L-meta**: POSIX metadata v1 (`0x8000`) and platform security metadata v1 (`0x10000`) parse and validate.
   - **L-legacy-records**: ConversionProvenance types 28-29 and preservation records 30-36 decode.
   - **L-crypto-public**: encrypted public framing parse without keys (no decryption; decryption is outside minimal
     reader scope and routed to crypto review).
   - **L-gated**: for `deflate-reconstruct` and v6 JPEG reconstruction, a time-boxed (one session) determination of
     whether the documents define the decode step; no implementation beyond that determination.
   The CLI is `ebir decode --archive PATH --policy default --json` (one `ebr.readtuple.v1` line) and
   `ebir batch --stdin` (length-prefixed inputs, one tuple per input) for EXP-CONF-002 and EXP-CONF-010.
   Permanent vectors in `docs/*-vectors*.txt` are part of the specification and are used as unit tests.
2. **Ambiguity log** `research/independent-reader/ambiguity-log.jsonl`, schema `ebr.ambiguity.v1`, written at the
   moment of encounter: `id`, `utc`, `session_id`, `document`, `section`, `lines`, `feature_area`, `category`
   (`MISSING_RULE`, `CONFLICTING_TEXT`, `UNDEFINED_TERM`, `IMPLEMENTATION_DEFINED_REFERENCE` (text defers to a
   function, crate or code path), `UNDOCUMENTED_REASON_CODE`, `UNDOCUMENTED_LIMIT`, `VECTOR_CONFLICT`,
   `UNSPECIFIED_DECODE_STEP`, `EDITORIAL`), `readings` (≥ 1 alternative interpretation), `chosen_reading`,
   `severity` (S1: the readings can yield different comparison-key tuples on some input; S2: only class, code or
   detail differ; S3: editorial), `vector_disambiguated` (bool). Validated by
   `research/independent-reader/tools/validate_ambiguity_log.py`.
3. **Freeze v1.** Commit hash, `go.sum`, `ebir` binary SHA-256 per level build tag recorded in
   `research/independent-reader/FREEZE-v1.json`. No change after freeze except through phase A2.
4. **Arm B (Python framing reader `ebir-py`).** Scope: preamble, Descriptor, feature bitmap, section directory and
   headers, canonical TLV framing and minimality, footer, STREAM item framing, `store/v1` chunk digests; outputs
   `ebr.readtuple.v1` with codec-dependent fields null. Same phase A1 rules, own ambiguity log
   `research/independent-reader/python-framing/ambiguity-log.jsonl`, frozen as `FREEZE-py-v1.json`.
5. **Adjudication.** An adjudicator session (CC§11) classifies each entry as `CONFIRMED_SPEC_DEFECT` or
   `BUILDER_MISREADING` (with a citation that unambiguously decides it), and merges duplicates across arms. A second
   adjudicator re-classifies a seeded 30% sample (seed CC§12 triage sample).
6. **Phase A2 (errata), after EXP-CONF-002's first triage.** Fixes are allowed only when a cluster is attributed
   `OTHER_READER_BUG` with a citation, or when the program records an erratum for a `SPEC_*` cluster. Produces
   `FREEZE-v2.json`; every A2 change cites its triage cluster. A1 and A2 counts are reported separately; H1-H3 are
   evaluated on A1.
7. **Measurements** (scripts under `research/independent-reader/tools/`):
   - `count_loc.sh`: `tokei` 12.1.2 (installed by `/root/.cargo/bin/cargo +1.98.1 install tokei --version 12.1.2 --locked`)
     per package and level, excluding `_test.go` files and vendored modules; external module LoC counted separately
     for the packages actually linked (`go list -deps`); `cloc` 2.x (apt) as a second counter (sensitivity).
   - `binary_size.sh`: stripped `ebir` binary bytes per level build tag (`go build -tags <level> -ldflags="-s -w"`).
   - `deps_audit.py`: `go list -m all` versus production `cargo tree -e normal --target all` (from `ebound-head`):
     count of shared native libraries and shared algorithm implementations (independence audit; expected 0).
   - `ambiguity_stats.py`: counts by category, feature area and severity, A1 and A2, confirmed only.
   - Reference sizes: `tokei` over `doc/educational_decoder` of tuning item `f01-tuning-zstd-v1-5-7-git`, and over
     the pinned `nix` NAR reader source (`src/libutil/archive.cc` at the pinned release tag, downloaded) — descriptive
     comparators for DEC-ECO-038 and REQ-ECO-0072.

## 8. Seeds, warmup, replication

Deterministic construction; no warmup. LoC, binary size and audit scripts run twice (repeat-hash of their outputs,
§4.13). Adjudication reliability sample seed: CC§12.

## 9. Metrics (CC§9)

| Metric | Role | Unit | Source | Level |
|---|---|---|---|---|
| `unspecified_baseline_decode_steps` | binary (HC-12) | count | confirmed `UNSPECIFIED_DECODE_STEP` entries in L-base | AGG-P |
| `baseline_codec_public_spec` | binary per codec (HC-12) | binary | H3 determination per codec/transform | AGG-P |
| `independent_reader_loc` | cost input (C2) | LoC | `count_loc.sh`, per level and module; external module LoC reported beside | AGG-P |
| `reader_fragmentation_count` | cost input (C5) | count | features whose absence at L-base makes default-written archives unreadable (measured by EXP-CONF-002 §9 on this reader) | AGG-P |
| `spec_defects_per_1000_words` | descriptive | count/1000 words | confirmed S1+S2 entries ÷ normative words of the snapshot (EXP-CONF-003 grammar) × 1000 | AGG-P |
| Ambiguity counts by category/area/severity; kappa; stripped binary bytes; shared-library count; builder sessions and wall hours | descriptive, non-canonical | count, bytes, h | §7 scripts | AGG-P |

`cleanroom_pass_fraction` and `reader_disagreements` for this reader are measured in EXP-CONF-002.

## 10. Normalization and analysis

No statistics: deterministic counts over one design (AGG-P). H1-H3 are binary determinations (§3.3). Analysis
tables: ambiguity entries per decision's feature area (the §3 mapping), LoC per level and per codec, binary bytes per
level, fragmentation per feature. Numbers are produced only by the checked-in scripts (§12.3). The analysis plan is
subject to adoption by a non-exposed session (CC§1).

## 11. Practical-significance thresholds

Binary metrics: §3.3 (no band). Cost inputs and descriptive metrics: no band (Appendix A.2 roles `cost_input`,
`descriptive`); they enter §7.2 tiers and R10 key 3 only.

## 12. Sensitivity analysis

- Ambiguity counts with and without S3, with and without entries later marked `BUILDER_MISREADING`.
- Adjudicator agreement (second adjudicator, 30% sample); counts under the second adjudicator's labels reported.
- LoC under `tokei` versus `cloc`; per-level partitions L-base only, L-base+L-meta, all levels.
- Arm A versus arm B on the framing scope: entries logged by only one arm (reading reliability of framing rules).

## 13. Split discipline and held-out placeholder

No corpus split is read during construction (§5). The frozen reader is exercised on the conformance corpus and on
tuning/validation-derived archives in EXP-CONF-002 under its own split rules. Held-out: none; a held-out re-run of the
frozen reader belongs to EXP-CONF-002's Phase D placeholder (§5.5).

## 14. Expected negative results worth recording

- Declared decoder working-set formulas defined by a library function (for example an LZMA2 memory-usage function)
  rather than by arithmetic in the text (`IMPLEMENTATION_DEFINED_REFERENCE`, H3).
- Reason codes emitted by production but absent from the documents (H2).
- Gated reconstruction steps not specifiable from documents (expected and allowed only under HC-11 gating; recorded).
- Fragmentation of Windows-created archives through the required `0x10000` bit (F-0001; H4).
- Doc-to-doc contradictions already named in the ledger (for example CHUNK_DATA ordering, type-18 field numbering).

## 15. Threats to validity

- **Shared model knowledge.** Builder, production authors and adjudicators may share a base model; the public
  repository may be in training data. The reader can agree with production for reasons other than the text,
  which understates ambiguity. Mitigations: provenance controls, arm B, and EXP-CONF-013 (external implementer).
- **Builder skill.** Misreadings inflate counts; adjudication separates them, but adjudication is itself judgment.
- **Snapshot drift.** Documents may change during construction; the snapshot is pinned by hash and every entry cites
  the snapshot.
- **Third-party decoder bugs** in Go modules can masquerade as specification defects; triage (EXP-CONF-002) requires a
  citation, and codec disagreements are cross-checked against the codec's own reference (RFC 8878 test frames).
- **Level boundaries** are a design choice; LoC per level depends on them (sensitivity §12).
- **Counting** LoC rewards terse code; reported with binary size and external LoC to limit that.

## 16. Cost estimates

| Item | Estimate |
|---|---|
| Compute | 3 CPU-hours (builds, unit tests, counters) |
| Disk | 3 GB (snapshot, module cache, builds, logs) |
| Agent effort | **judgment, high**: arm A 4-8 builder sessions; arm B 1-2 sessions; adjudication 1-2 sessions; A2 1-2 sessions |
| Platform | Linux x86-64 (WSL); network once for pinned modules; no privileged operation; no timing |

## 17. Outcome obligations

`research/experiments/EXP-CONF-001/outcome.json` per §10.1; ambiguity entries that are confirmed defects become
UNRESOLVED-native findings for EXP-CONF-004 (each must become a specification fix or a decided case) and appear in the
`counterevidence` of the decisions in §3.
