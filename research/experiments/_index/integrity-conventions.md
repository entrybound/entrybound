# Integrity domain design conventions (program sections 23, 24)

This file holds the definitions shared by the integrity-domain experiment protocols
`research/experiments/EXP-INT-001` to `EXP-INT-020`. Each protocol cites the
convention IDs below (C1, C2, ...) instead of restating them, so that a definition
exists in exactly one place. Where a protocol and this file disagree, the protocol
is wrong and is corrected by revision before any run (no result exists).

Governing documents: `research/decision-method.md` (authoritative for procedure,
thresholds, statistics, status and cost rules) at the pre-registration commit
`14b977c79f157c58278a70c0f281f195430ca4f0`, and `research/archetypal-objective.md`
(hard constraints, MVTs, metric definitions). Threshold values are never restated
here: every metric is cited by its canonical name from `decision-method.md`
Appendix A.2 and its threshold ID (T-xx), and the value is read from
`research/methods/thresholds.json` `metric_thresholds`.

---

## C0. Status of these protocols, gates and authorship

- **Phase C design artifacts, not yet §12.2 records.** A protocol becomes the
  pre-registration record of `decision-method.md` §12.1-§12.2 only when every gate
  it depends on holds. Gate **G-A** (§0.4) must hold before a record with non-empty
  `decision_ids` is committed as a record: the constraint crosswalk
  (`research/methods/constraint-crosswalk.csv`, §14 item 10), ledger schema v2 with
  independently assigned `decision_type`, `od_ids` and `hc_ids` (§14 item 12), and the
  validation-look registry (§14 item 14). Gate **G-B** (decision tooling §14 item 2,
  frozen held-out lock, workload mix, runner additions and calibration for timing)
  must hold before any decision-relevant result is committed. Until then every run of
  these specs is informational (`NOT_DECISION_GRADE`).
- **OD, HC and decision type.** The `od_ids`, `hc_ids` and `decision_type` shown in
  each protocol are the designer's expectation only. The binding values are assigned
  by an independent session (§1.2, §14 item 12). If the assigned OD set differs, the
  protocol's primary-metric table is amended by revision before execution; an OD that
  the protocol does not measure is listed as a gap (R3), never silently dropped.
- **Primary metrics.** Each protocol proposes at most one primary metric per OD per
  decision (R3). A second primary metric for one OD is marked "needs reviewed
  justification" and is secondary until a session not involved in the decision
  approves it.
- **Candidates.** Candidate sets are copied from the ledger (`candidate_set`). A
  candidate added by this designer is marked `[designer-added]`; it does not satisfy
  R0 item 6 (critic-proposed candidate), which the design review supplies.
- **Exclusions.** This designer only *proposes* L0 and R1(b) exclusions, with the
  counterexample named. Each proposal needs sign-off by a session not involved in the
  candidate set (objective §2.0 rule 3). Where running the excluded candidate is cheap,
  it is kept as a **negative control**: its measured violation is the committed
  counterexample artifact (objective §2.0 rule 2).
- **Author and exposure disclosure (L7, §5.4, §5.7).** Author: the Phase C-design
  integrity designer session (workflow `wf_47347bc4-611`, model Claude Opus 5). It
  designs experiments and analysis plans only; it proposes no new design candidate
  beyond those marked `[designer-added]`. While reading the required inputs it saw
  held-out item identifiers without content: `research/corpus/coverage.md` lists
  held-out item ids and independence groups, and a directory listing of
  `/root/eb-research` showed the name of the held-out root. No held-out content,
  listing of a held-out directory, statistic or held-out path below that root was
  opened. The corpus selectors of every protocol name tuning and validation items only.
  The design review must record this exposure in the L7 record (§5.8 h) and decide
  whether §5.4 item 3 restricts this session's analysis role for the exposed families.
- **Feasibility smokes.** Five smokes of at most 2 minutes each ran on tiny generated
  trees under `/root/eb-research/scratch/int-design-smoke*` (scripts in the session
  scratchpad, not committed; `ebound` from `/root/eb-research/target/baseline/release/ebound`,
  par2cmdline 0.8.1), including two repetitions of the EXP-INT-009 driver. They are
  **non-decision-grade**, produced no committed numbers, and only confirmed command shapes
  (for example that the addressing signature binding is refused for unencrypted archives,
  and that `verify --signature` prints one `signature ... cryptographic=... physical=...`
  line). Observations used to shape pre-registered cells, without claiming a violation:
  - unpacking an archive and re-packing the extracted tree with the same profile reproduced
    PCR but not AUX or PCI, and AUX differed between two extractions of the same archive
    (EXP-INT-009 H4 and its deterministic-view rule);
  - par2cmdline with its default thread pool used many CPU-minutes on a tiny file, so
    parity tools are pinned to one thread in recovery trials (EXP-INT-010);
  - a single-byte flip inside ChunkData made `ebound verify` report `CORRUPT` with a
    section-level code, and `ebound read` of an entry report a chunk-level code;
  - a single-byte flip inside the preamble ResourceBudget field made `verify` report
    `POLICY_REFUSED` rather than `CORRUPT`, which becomes adjudication cell A1 of C7;
  - tail truncation gave `TRUNCATED`; and a par2 repair of a 512-byte zero-fill restored
    identical bytes.

## C1. Builds and tools

| ID | Artifact | Build or source |
|---|---|---|
| B-PROD | `ebound` production CLI (status quo, REF-PROD behaviour) | `cargo +1.98.1 build --release -p entrybound-cli` in the production workspace at the experiment's pre-registration commit, `CARGO_TARGET_DIR=/root/eb-research/target/int-prod` (WSL) or `D:/eb-research/target/int-prod-win` (Windows). Before use, MVT-16(d) frozen-ID differential against `9e44608` is run by the platform or conformance domain; this domain records the binary SHA-256 in every raw row. |
| B-RES | `entrybound` with the default-off `research-internals` feature, used only through the harness crates | `research/harness/build.sh build --release`. Every HC result that uses B-RES as oracle counts only while `research/harness/internals-identity-check.sh` and `.ps1` pass (objective §2.0 rule 9, MVT-16(c)). |
| B-HOOK | Additive research-internals hooks needed by this domain (T11 below) | Added only as `#[cfg(feature = "research-internals")]` blocks, proven additive by `internals_identity_tools.py additive-check`; never enabled in production builds (PROGRESS standing decision). |
| TOOL-INC | Incumbent archivers | Exact versions as recorded in `research/baselines/tool-versions.json` (for example GNU tar 1.35, gzip 1.12, zstd 1.5.5, xz 5.4.5 and 7-Zip 23.01 per the PROGRESS environment record; Info-ZIP zip and unzip at their recorded package versions). Configs from `research/baselines/configs.json`. |
| TOOL-PAR | Parity tools | par2cmdline 0.8.1 (apt, present); others per EXP-INT-010. Versions recorded by extending `research/baselines/probe_tool_versions.py`. |

**Harness use constraints** (`research/harness/harness-review-round1.md`, "Use
constraints") bind every experiment: `lock_parity.py` and `harness-cli-parity.sh`
must print `RESULT PASS` before a campaign; decision-grade timing uses B-PROD;
memory metrics use scoped memory fields, never `process_lifetime_peak_rss_bytes`;
netem results use achieved RTT and carry the slow-start constraint (R1-14);
external-tool RSS comes only from GNU time (R1-16).

## C2. Tooling to build (shared by several experiments)

All Rust tools live in a new harness crate `research/harness/crates/ebr-integrity`
(added to the harness workspace `members` only when it builds). All Python tools
live in `research/tools/integrity/` (stdlib only unless stated). Every binary calls
`ebr_common::heldout::assert_inputs_not_heldout` before reading an input, prints
exactly one `ebr.harness.raw.v1` JSON line on stdout when `--out` is omitted, and
writes per-event detail to the detail store (C9).

| ID | Tool | Purpose | Used by |
|---|---|---|---|
| T1 | `ebr-int-map` | Region and ownership map of an archive: INDEXED section extents, section headers, preamble, footer, Chunk frames (header and stored bytes, `chunk_id`, `plan_ref`, `group_ref`), dictionaries, ChunkGroups, reconstruction data and regions; STREAM tagged items (walker over `STREAM_BODY` via research-internals); crypto-v1 INDEXED public framing (preamble, envelope, stanzas, segments, SegmentEnd, ArchiveFinal, footer). Emits stratum byte ranges (C4) and, per byte range, the owning entries through the dependency closure (C5). Its section sums must equal the file size and agree with `ebr-inspect-bytes` named buckets, else the item is invalid. | 001, 002, 003, 012, 013, 016 |
| T2 | `ebr-int-inject` | Seeded injector for the damage classes of C6 on any file, driven by a T1 map (native) or an incumbent map (T4). Seed derivation C8. | 001, 002, 010, 012, 016 |
| T3 | `ebr-int-probe` | Runs every probe mode of C7 on an injected archive and computes the C5 oracle sets and per-event metrics. Implements the **overlay optimization**: payload-stratum events re-decode only the damaged frames and the frames whose recorded closure includes them, reading every other byte from a cached pristine decode. The optimization is admitted for an experiment only after an **equivalence precondition** passes: on every small-tier item of that experiment, 100% of events give identical per-entry outcomes, reason codes and blast values with the optimization on and off. Otherwise every event re-opens the full archive. | 001, 002, 012, 016 |
| T4 | `ebr-int-incumbent` | Region maps and per-entry probes for incumbent formats: ZIP (local headers, data, central directory, EOCD), tar.gz and tar.zst and tar.xz (container header, compressed payload, trailer, frame checksums), 7z solid and non-solid (signature header, packed streams, header database). Probe: the incumbent's own test and extract commands (`unzip -t`, `gzip -t`, `zstd -t`, `xz -t`, `7z t`, then extraction), per-entry content comparison against the source tree. | 001, 012 |
| T5 | `ebr-int-verify-levels` | Research prototypes of verification levels and modes (EXP-INT-004 candidates): structure-only, section-digest-without-decode, stored-bytes-digest (using a research-only per-frame stored-bytes digest map as a stand-in for the proposed wire change), full decode, container-digest-before-parse, continue-and-collect with a bounded diagnostic list, damage map. Built on the public API plus B-HOOK. | 001, 004 |
| T6 | `ebr-int-salvage` | Salvage prototypes: intact-entry extraction, best-effort with unverified marking, Chunk-frame carving by `EBCH` magic and `chunk_id` matching, legacy ZIP local-header scan. | 002, 012, 016 |
| T7 | `ebr-int-kill` | Crash injector. Linux: a ptrace syscall stepper that stops the target process tree at the Nth file-mutating syscall (`openat` with `O_CREAT`/`O_TRUNC`, `write`, `pwrite64`, `ftruncate`, `fsync`, `fdatasync`, `rename`, `renameat2`, `link`, `linkat`, `unlink`, `unlinkat`, `close` of a written descriptor) and sends SIGKILL; a recording pass enumerates the sequence. Linux crash-state generator in the ALICE style (strace record, persistence model of ext4 `data=ordered`: unsynced data may be lost, metadata operations persist in order up to the crash point). Windows: a job-object launcher that calls `TerminateJobObject` at seeded delays and on the first `ReadDirectoryChangesW` event for the destination; no admin rights. | 005 |
| T8 | `ebr-int-fsfault` | Linux fault filesystems as root inside WSL on loop devices under `/root/eb-research/scratch`: small ext4, XFS and btrfs images pre-filled to a target free space (disk full), and a `dm-linear` + `dm-error` table that returns EIO for a seeded sector range (`CONFIG_BLK_DEV_DM=y`; `dm-flakey` and `dm-log-writes` are not in the WSL2 kernel, checked 2026-09-17). Windows rename failure by holding the destination open without `FILE_SHARE_DELETE`. | 005 |
| T9 | `ebr-chain` (Python) | Research model of non-Complete roles over production Complete archives: reference manifests (entry, base identity, content digest, length, locator), resolvers per candidate, hostile chain generator (cycles, depth, fan-in, stale, substituted and missing bases), exact byte-size model of candidate canonical records from `docs/format-v0.md` record rules. It is a model, never a wire implementation; every result from it carries the `MODEL` label in the protocol and is `SUPPORTED_WITH_LIMITATIONS` at most until a wire implementation is measured. | 007, 008 |
| T10 | `ebr-int-resolver-suite` | Hostile locator and origin fixtures: path traversal, absolute and UNC paths, symlink and NTFS junction escape, case-fold aliases, `file://`, HTTP redirects to loopback, link-local and private addresses (address classes injected through the resolver's own DNS hook, no system DNS change), oversized and slow bodies, lying `Content-Length`, changing ETags, time-of-check to time-of-use swaps. Built on `ebr-netem` `ebr-origin`. | 008, 016 |
| T11 | B-HOOK hooks | (a) byte-verification log: which digests, AEAD tags and signatures were checked over which byte ranges; (b) encoder-output fault hook for writer verification scope; (c) export-encoder mutation hook for the LOSSY gate; (d) reader options for prototype policies (ignore damaged Index section, continue after chunk failure, verify touched segment ends); (e) crypto-v1 per-record decrypt access for encrypted salvage. | 001, 003, 004, 014, 016, 017 |
| T12 | `ebr-int-reseal` | Re-computes section payload digests, STREAM body digest and footer fields after a deliberate payload edit, so that only deeper checks (chunk digests, decoder canonicality) can detect it. | 013, 015 |
| T13 | `ebr-int-carrier` | Creates out-of-band carriers (names, xattrs, NTFS ADS, suffixes, marker manifests, quarantine directories, receipts, wrapper indexes, parity sets) and passes them through transfer tools. | 011 |
| T14 | `ebr-int-merkle` (Python) | Independent model of the SHA-256 named-domain tree written only from `docs/format-v0.md` "Hash construction" with a provenance log, plus proof-cost models of the candidate range-verification structures. | 015 |
| T15 | `ebr-int-fixtures` (Python) | Crafted codec payloads and legacy fixtures (zstd skippable, multi-frame, checksum-less frames; xz check types and filters; gzip multi-member and ISIZE; ZIP CRC-collision size cases reusing `tools/zip-compat/generate_corpus.py` read-only). | 013 |

## C3. Corpus sets

Only `tuning` and `validation` items of `research/corpus/manifest.json` (corpus
`ebrc-2026.09-v1`, manifest SHA-256 `ed28b1b47333c186a8c80781d378b4c149eaaceea09a65098182f4c062380392`)
are named. Item identity of record is `logical_tree_sha256` (§5.3); every raw row
records it. Groups are the manifest `independence_group`.

- **INT-CORE-T (tuning, 21 items, 21 family-group pairs, 14 families, 0.43 GiB):**
  `f01-tuning-ripgrep-14-1-1-git`, `f01-tuning-curl-8-19-0-git`,
  `f03-tuning-fzf-go-mod-vendor`, `f04-tuning-sourcelike`,
  `f05-tuning-gutenberg-books`, `f05-tuning-loghub-openstack`,
  `f06-tuning-census-popest-2023`, `f06-tuning-wikimedia-tnwiki-20260901`,
  `f07-tuning-silesia-xray`, `f07-tuning-noaa-ncep-reanalysis-air-sig995-2020`,
  `f08-tuning-chinook-sqlite`, `f09-alpine-324-rootfs-binaries`,
  `f12-tuning-kodak-jpeg-edge-cases-small`, `f17-tuning-duptree-mixed`,
  `f18-tuning-lz4-releases-1-9-to-1-10`, `f18-tuning-zstd-releases-1-5-x`,
  `f19-tuning-zoo`, `f19-tuning-real-home-snapshot`,
  `f20-tuning-real-libarchive-testsuite`, `f20-tuning-csprng`,
  `f10-gen-redundant-binary-blobs`.
- **INT-CORE-V (validation, 21 items, 21 family-group pairs, 14 families, 0.39 GiB):**
  `f01-validation-redis-7-2-16-git`, `f01-validation-bootstrap-5-3-8-git`,
  `f03-validation-yq-go-mod-vendor`, `f04-validation-objstore`,
  `f05-validation-rfc-texts`, `f05-validation-logrotate-from-loghub-ssh`,
  `f06-validation-osm-monaco-20250101-xml`, `f06-validation-noaa-ghcn-daily-parquet-2023`,
  `f07-validation-gen-numeric-b-small`, `f07-validation-nasa-gistemp-1200km`,
  `f08-validation-sakila-sqlite`, `f08-validation-northwind-sqlite`,
  `f09-cpython-3147-embed-win-amd64`, `f12-validation-fsa-jpeg-edge-cases-small`,
  `f12-validation-commons-cc0-photos-small`, `f13-validation-federal-register-pdf-small`,
  `f17-validation-duptree-vendor`, `f18-validation-cjson-releases`,
  `f19-validation-real-home-snapshot`, `f19-validation-deep`,
  `f20-validation-real-cpython-archivetestdata`.
- **INT-LARGE-T / INT-LARGE-V (scale arms):** tuning `f05-tuning-loghub-hdfs-v1`,
  `f01-tuning-llvm-project-20-1-8-git`; validation
  `f18-validation-cpython-3-12-point-releases`, `f07-validation-smollm2-360m-safetensors`.
- **INT-SMALL-T / INT-SMALL-V (combinatorial matrices where content matters little):**
  tuning `f19-tuning-zoo`, `f01-tuning-ripgrep-14-1-1-git`, `f06-tuning-census-popest-2023`,
  `f12-tuning-kodak-jpeg-edge-cases-small`, `f09-alpine-324-rootfs-binaries`,
  `f05-tuning-gutenberg-books`; validation `f19-validation-deep`,
  `f01-validation-redis-7-2-16-git`, `f05-validation-rfc-texts`,
  `f18-validation-cjson-releases`, `f12-validation-fsa-jpeg-edge-cases-small`,
  `f08-validation-sakila-sqlite`.
- **INT-SERIES-T / INT-SERIES-V (version series for incremental and rollback arms):**
  tuning `f18-tuning-lz4-releases-1-9-to-1-10`, `f18-tuning-zstd-releases-1-5-x`,
  `f18-tuning-sqlite-amalgamation-series`, `f18-tuning-ripgrep-commit-snapshots`,
  `f19-tuning-real-home-snapshot`, `f19-tuning-rsnapshot-a`,
  `f17-tuning-real-ubuntu-kernel-headers-2ver`; validation `f18-validation-cjson-releases`,
  `f18-validation-redis-7-2-point-releases`, `f18-validation-cpython-embeddable-3-14-series`,
  `f19-validation-real-home-snapshot`, `f19-validation-rsnapshot-b`,
  `f17-validation-real-fedora-kernel-devel-2ver`. **Version rule, pre-registered:** the
  versions of an item are its top-level directories in the order given by the item's
  manifest `description` (release order), or, if the description gives no order, by
  byte-wise name order. An item whose top level is not one directory per version or
  snapshot is excluded from the chain arms before any run, and the exclusion is logged.
- **INT-LEGACY-T / INT-LEGACY-V (legacy artifacts):** tuning
  `f14-legacy-writer-matrix-tuning`, `f14-apache-maven-3916-bin-tarball`,
  `f14-gen-nested-archives-py`, `f14-jenkins-25683-war`, `f11-pypi-binary-wheels-cp312`,
  `f11-ubuntu-noble-debs`, `f11-alpine-324-apks`, `f16-oci-ubuntu-noble-multiarch-index`,
  `f20-tuning-real-libarchive-testsuite`; validation `f14-legacy-writer-matrix-validation`,
  `f14-gen-office-documents`, `f14-pyspark-420-sdist`, `f11-maven-central-jvm-jars`,
  `f11-cpython-3137-release-set`, `f11-fedora-44-rpms`, `f16-oci-fedora-44-zstd-layers`,
  `f20-validation-real-commons-compress-testdata`.
- **INT-HOSTILE-T / INT-HOSTILE-V:** tuning `f20-tuning-generated-malicious-structures`,
  `f20-tuning-bombs-dense`, `f20-tuning-gear-extremes`; validation
  `f20-validation-generated-malicious-structures`, `f20-validation-bombs-compressed`,
  `f20-validation-sparsedup`.

**Minimum support (§4.9).** INT-CORE-T and INT-CORE-V each exceed 10 independence
groups across 5 families; they are the only sets on which banded corpus-level verdicts
are drawn. INT-SMALL, INT-SERIES, INT-LEGACY and INT-HOSTILE feed binary (R1/R2)
tests, coverage fractions over pre-registered case sets (T-20) and descriptive
metrics, for which §4.9 minimum support does not apply; any banded verdict attempted
on them is secondary. Known covariates: `alpine-linux` spans F09/F11/F16 tuning,
`cpython` spans F01/F09/F11/F18 validation, `redis` spans F01/F18 validation; each
counts as one group per family and cross-family sharing is reported, per
`research/corpus/critique-round2.md`.

## C4. Region strata (T-15, objective OD-14)

Every stored byte of an archive belongs to exactly one stratum. The mapping is
pre-registered here and implemented by T1.

| Stratum | INDEXED plaintext | STREAM plaintext | crypto-v1 INDEXED |
|---|---|---|---|
| R-PAY payload | Chunk frame stored bytes in CHUNK_DATA | Chunk frame stored bytes | segment ciphertext bytes of CHUNK class records |
| R-IDX Index | INDEX section header and payload | not applicable (reported, weight renormalized) | encrypted Index record bytes |
| R-META metadata and manifest | DESCRIPTOR, TRANSFORM_PLANS, MANIFEST_RECORDS, FIDELITY payloads | the corresponding tagged items | envelope, stanzas, recipient directory, CONTROL records other than ArchiveFinal |
| R-DICT dictionaries and dependency data | DICTIONARIES, CHUNK_GROUPS, RECONSTRUCTION_DATA, RECONSTRUCTION_REGIONS payloads | corresponding items | corresponding records |
| R-INT integrity records and footer | 256 B preamble, 64 B section headers, Chunk frame headers, 128 B footer | preamble, item tags and headers, Chunk frame headers, footer | preamble, segment headers, SegmentEnd, ArchiveFinal, footer |

The headline combines strata with equal weights 0.2 over applicable strata; a stratum
that is structurally absent for a configuration (R-IDX in STREAM, R-DICT when the
archive has no dictionary, group or reconstruction data) is reported as not applicable
and the remaining weights are renormalized equally. The byte-proportional combination
is a sensitivity arm (objective OD-14 aggregation).

## C5. Ground-truth oracle sets

For an injection event `x` (a set of modified, removed or appended byte positions)
and an entry `e`:

- **closure(e)** is the set of Chunk frames in `e`'s ContentObject, plus every frame
  those frames depend on under the recorded plans (ChunkGroup lookback members,
  transitively), plus referenced dictionaries and reconstruction regions and data,
  computed by T1 from the pristine archive.
- **oracle_affected(e, x)** is true iff `x` overlaps (a) any byte of a frame in
  closure(e); (b) `e`'s Entry record, ContentObject record, or a metadata or FIDELITY
  record bound to `e`; or (c) an authoritative global structure: preamble, DESCRIPTOR,
  section headers of authoritative sections, footer, TRANSFORM_PLANS records used by
  `e`, or crypto framing needed to authenticate `e`. Case (c) marks every entry the
  structure governs as affected. Damage confined to the non-authoritative Index or to
  records bound to no entry marks no entry affected.
- **reported_affected(e, x, mode)** is true iff probe mode `mode` (C7) does not
  deliver `e` as verified intact.
- Per event: `blast_logical_bytes` = sum of logical bytes of entries with
  reported_affected; `blast_entries` = their count; localization **recall** =
  |reported ∩ oracle| / |oracle| (defined as 1 when oracle is empty);
  localization **precision** = |reported ∩ oracle| / |reported| (defined as 1 when
  reported is empty). An event with a non-empty oracle set, or a change to any
  authoritative byte, that every probe of a mode reports as `OK` without qualification
  is an **undetected injection** (I21, binary).
- Per item and stratum: p95 and max of `blast_logical_bytes` and p95 of
  `blast_entries` over events (Hyndman-Fan type 7, §4.8); precision is the mean over
  events and its minimum is reported beside it.

## C6. Damage classes

| ID | Class | Placement | Count per archive (unless the protocol says otherwise) |
|---|---|---|---|
| D-P1 | single byte XOR with a nonzero seeded byte (**primary class of T-15**) | uniform over the stratum's bytes | 200 per applicable stratum (all positions if the stratum has fewer than 200 bytes) |
| D-S1 | single bit flip | uniform per stratum | 200 per stratum |
| D-S2 | 512 B sector erasure (zero fill), aligned to 512 | uniform over aligned offsets | 100 |
| D-S3 | 4096 B sector erasure, aligned to 4096 | uniform | 100 |
| D-S4 | 64 KiB random overwrite | uniform | 50 |
| D-S5 | 1 MiB random overwrite | uniform; skipped when archive < 2 MiB | 20 |
| D-S6 | missing Chunk: a whole frame's stored bytes zero-filled, length unchanged | uniform over frames | 50 |
| D-S7 | structural field edits: every length, offset, count and locator field in preamble, section headers, frame headers and footer set to 0, +1, and past end of file | exhaustive over fields (frame fields: all frames if at most 1,000, else 1,000 seeded frames) | all |
| D-S8 | two independent D-P1 events in different strata | uniform stratum pairs | 100 |
| D-T1 | truncation at every structural boundary b, at b-1 and b+1, and at every byte of preamble and footer | exhaustive (frame boundaries: all if at most 10,000 frames, else first and last 100 plus 10,000 seeded) | all |
| D-T2 | truncation at seeded offsets | uniform over the file | 1,000 |
| D-T3 | appended bytes: 1 zero byte, 4096 random bytes | end of file | 2 |
| D-T4 | concatenation: the archive followed by itself; followed by another archive of the same configuration | end of file | 2 |

Incumbents use the same classes over their own region maps (T4); strata that do not
exist for an incumbent format are reported as not applicable.

## C7. Probe modes and the fault-class oracle

| Mode | What it runs |
|---|---|
| M-SQ-VERIFY | status quo `entrybound` library `verify` (fail-fast) and, on a seeded 10% subsample of events plus every D-S7 and D-T1 event, the B-PROD CLI `ebound verify` exit status |
| M-SQ-READ | status quo verified random read of every entry (INDEXED, Index rebuild permitted) or verified sequential unpack to a staging directory (STREAM); per-entry outcome |
| M-CONT | T5 continue-and-collect verify with a bounded diagnostic list |
| M-MAP | T5 damage map: per-entry verified, affected or dependency-affected |
| M-IDXGUIDE | T5 Index-guided localisation, fail-fast otherwise |
| M-INC | T4 incumbent test plus per-entry extraction comparison |

**MVT-15(b) fault-class diagonal.** Expected outcome classes are those of the
precedence table in `archetypal-objective.md` HC-15, applied to the event by T3 from
the T1 map. The count of events whose library class or CLI exit status is off that
diagonal is the HC oracle count `mvt15b_offdiagonal_events` (binary, R1). Pre-registered
adjudication cells, where the table does not decide the expected class without a
normative citation (objective §2.0 rule 8), are reported separately and never counted as
passes until adjudicated by a session involved in neither the reader nor this design:
- **A1.** A preamble field that a budget check reads before the footer's preamble digest
  is checked, changed so that the declared budget exceeds policy. Candidates:
  `POLICY_REFUSED` (reader order) or `CORRUPT` (table row 2: a digest available from
  the present bytes covers the field).
- **A2.** Appended bytes or concatenation after a complete footer (D-T3, D-T4):
  `CORRUPT`, `NONCONFORMING` or `OK` with a qualifier; this is the subject of
  DEC-INT-040 candidate `appended-bytes-nonconforming`.
- **A3.** A length field edited so that it points inside the file but to a different
  well-formed boundary (D-S7 "+1" on a frame chain): `CORRUPT` or `NONCONFORMING`.

## C8. Seeds

`seeds.order` and `seeds.bootstrap` are in each spec. Event seeds are derived, never
drawn from a clock: `event_seed = first 8 bytes (big-endian) of SHA-256("ebr-int/v1" ||
experiment_id || item_id || candidate || class_id || stratum || event_index)`, then
PCG64 as in `research/tools/ebr/stats.py`. The derivation does not include the
repetition number, so both repetitions inject identical events, which is what the
repeat-hash check (§4.13) requires.

## C9. Raw detail store

Per-event rows (one JSON object per event, gzip-compressed JSONL) go to
`/root/eb-research/raw-detail/<EXP-ID>/<run_id>/<candidate>/<item_id>/rep<k>.jsonl.gz`,
outside git like the corpus. The hash-chained `ebr.raw.v1` row of the sample records
the detail file's SHA-256 (`detail_sha256`, string metric) and byte length, and
`research/raw/<EXP-ID>/detail-manifest.jsonl` (committed) lists every detail file with
its hash. Every number used for a decision is regenerated from detail files by the
decision tooling (§12.3); `ebr verify-raw` plus a detail re-hash is required before
`normalize`. Detail files are never deleted (§4.8).

## C10. Default statistical plan (cited by protocols; protocols state deviations)

- **Deterministic metrics** (blast radius, localization, coverage fractions, byte
  counts, outcome classes): 2 repetitions in different rounds and processes plus the
  repeat-hash check of the detail file (§4.6, §4.13). A mismatch where the reader
  claims determinism (outcome of a reader is a function of bytes, HC-03) is itself the
  violation artifact; otherwise the metric is reclassified as stochastic with
  timing-style repetitions.
- **Timing and memory metrics:** §4.6 minimum rounds, steps and caps per tier, with
  n_min for the confidence in use; the precision-based adaptive rule of §4.6; warmups
  §4.5; affinity `st-v` in WSL (§4.3) or `st-p` on Windows (§4.2); guard values in the
  spec only tighter than `thresholds.json` (§3.7, §4.4); calibration experiments
  `EXP-CAL-*` for the environment and family (§4.7) must have passed. Windows timing
  is not decision-grade until the runner additions of §14 item 3 exist.
- **Levels and intervals:** L1 item exact value or median paired log ratio; L2 group;
  L3 family; L4 corpus with the cited workload-mix weights (§4.9); item intervals exact
  order-statistic, corpus Welch-Satterthwaite, family pooled (§4.10); verdict classes
  and `NON_INFERIOR` (§4.11); confirmatory set W versus S with the confidences and MDE
  gate of §4.12; family harm guard (R4 condition 3), with the worst-stratum guard for
  T-15 metrics.
- **Rules:** R1-R10 in order through the decision tooling (§14 item 2). Binary metric
  failures are R1 or R2 eliminations and, for the status quo, production defects
  (objective §2.0 rule 5), never trade-offs.
- **Validation looks:** only after tuning names W and S and the MDE gate passes; each
  look is registered in `research/decisions/validation-looks.jsonl` (§5.2); at most two
  looks per decision.
- **Sensitivity:** §6 in full (weight grid and Dirichlet, SMAA report, threshold
  perturbation with the resolvability condition, every §6.5 arm, selection-stability
  bootstrap), plus each protocol's experiment-specific arms.
- **Negative results:** every run gets `outcome.json` (§10.1); negative, null and
  inconclusive outcomes go to the register under `research/reports/`.

## C11. Metric-name gaps for the method's round-2 review

These quantities are needed by the integrity decisions but have no canonical name in
`decision-method.md` Appendix A.2. Under §0.2 item 4 a banded metric without a band and
role committed before its first decision-relevant result is secondary forever, so the
design review should either add them by revision now (no decision-relevant result
exists) or accept that they stay secondary.

| Proposed name | Kind | Proposed threshold class | Used by |
|---|---|---|---|
| `damage_detection_fraction` | coverage over a pre-registered damage case set, per verification level | T-20 (a = 0.5/n_cases), maximize | 004, 014 |
| `parity_repair_success_fraction` | fraction of damage trials fully repaired and re-verified | T-20, maximize | 010 |
| `salvage_recovered_fraction` | fraction of oracle-intact logical bytes recovered as verified by salvage under non-truncation damage (the truncation case is `truncation_salvage_fraction`) | T-20, maximize | 012, 016 |
| `carrier_survival_fraction` | fraction of (carrier, transfer) cases in which the carrier survives intact | T-20, maximize | 011 |
| `binding_survival_fraction` | fraction of rewrite operations after which a binding still matches (or is correctly reported stale) | T-20, maximize | 009 |
| `rename_detection_fraction` | fraction of renames in a history identified as renames | T-20, maximize | 007 |
| `undetected_injections` | I21 detection failures | binary (§3.3 bullet 3), count = 0 | 001, 002, 016 |
| `mvt15a_overclaim_cells` | HC-15 MVT-15(a) reported state stronger than oracle state | binary, count = 0 | 003, 004, 016 |
| `mvt15b_offdiagonal_events` | HC-15 MVT-15(b) class confusion | binary, count = 0 | 001, 002 |
| `mvt18a_violations` | HC-18 MVT-18(a): verifying partial output, partial publish, modified pre-existing destination | binary, count = 0 | 005 |
| `confinement_escapes` | HC-05 / I16 confinement failures (§3.3 bullet 6) | binary, count = 0 | 008 |

Until the review disposes of this table, protocols use these names as HC oracle counts
(binary rows) or secondary metrics (coverage rows), never as primary metrics.

## C12. Held-out evaluation (Phase D placeholder, shared)

Held-out evaluation is designed only as a placeholder here. It follows
`decision-method.md` §5 and R5 exactly: the single program-wide design freeze (§5.5
Commit A and B), the held-out MDE gate computed from validation group-level variance
and fingerprint-level held-out group counts only (§5.4, R5), one run of every candidate
that reached the freeze with `EB_HELDOUT_UNLOCK` equal to the recorded
`design_freeze_sha` after Commit C, R1, R2, R4, R5 and R7 re-run with no re-selection
(§2.11), the `identity_aware_heldout` soft cap unless the evidence comes only from a
sealed tranche (§5.4), and report-only publication with re-runs only for infrastructure
failures (§5.6). Held-out item selection is fixed by the frozen lock, not by these
protocols; the held-out specs are written at the freeze by copying each experiment's
validation spec and replacing only the corpus selector with `splits: [heldout]` and the
frozen families. No held-out item, path or statistic is named in this domain's files.

## C13. Cross-domain overlaps noted at design time (for the design review)

Other domain designers wrote index fragments concurrently. Where their experiments inform the
same decisions, the review should merge or assign one confirmatory source per comparison, so
that a decision does not receive two validation looks from near-duplicate experiments (§5.2).
Overlaps visible when this domain's fragment was written:

| Integrity experiment | Overlapping experiments (other domains) | Shared decisions | Suggested split |
|---|---|---|---|
| EXP-INT-001 | EXP-CROSSFILE-004, 006; EXP-EVAL-012 | DEC-INT-002, DEC-CMP-027, DEC-INT-006 | crossfile owns lookback candidates and their packing; integrity owns injection, oracle and blast-radius measurement over whatever archives crossfile produces |
| EXP-INT-002 | EXP-CONF-002, 004, 010 | DEC-INT-040, DEC-INT-019 | conformance owns normative conformance cases; integrity owns exhaustive cut campaigns and intact-prefix mechanisms |
| EXP-INT-003 | EXP-CONF-001, 002, 004; EXP-CRYPTO-014, 022; EXP-REMOTE-010 | DEC-INT-042, DEC-INT-020, DEC-MOD-014, DEC-CRY-090, DEC-INT-038, DEC-INT-021, DEC-INT-044 | integrity owns the MVT-15(a) oracle matrix; crypto owns signature-status semantics; remote owns byte and request cost at scale |
| EXP-INT-004, EXP-INT-006 | EXP-SCALE-001, 004, 008 | DEC-INT-043, DEC-INT-039 | scale owns memory and staging growth beyond RAM; integrity owns detection coverage per level and failure cleanup |
| EXP-INT-009 | EXP-SCALE-005; EXP-CRYPTO-015 | DEC-INT-046, DEC-LEG-040 | scale owns cross-thread determinism; integrity owns identity and binding staleness |
| EXP-INT-012 | EXP-EVAL-008 | DEC-INT-035 and several section-23 decisions | evaluation owns stable-v1 scoring; integrity owns recovery measurements |
| EXP-INT-013 | EXP-CODEC-005, 007; EXP-CONF-004, 009, 011; EXP-SCALE-002 | DEC-INT-008, DEC-LEG-021 | codecs owns decoder resource profiles; conformance owns fuzzing; integrity owns crafted canonicality and legacy check strictness |
| EXP-INT-014 | EXP-CODEC-005, 006; EXP-CONF-001 | DEC-CMP-044, DEC-INT-030 | codecs owns pack-time cost; integrity owns fault-injection detection and independent-decoder differentials |
| EXP-INT-015 | EXP-CONF-012; EXP-REMOTE-007 | DEC-INT-024, DEC-INT-029, DEC-MOD-007, DEC-CON-020 | conformance owns permanent vectors; remote owns emulated-network cost; integrity owns the counterexample search and exact proof-cost model |
| EXP-INT-016 | EXP-CRYPTO-008, 010, 021; EXP-REMOTE-009, 010 | DEC-INT-010, DEC-CRY-080, DEC-CRY-073, DEC-INT-033 | crypto owns constructions and the external-review dossier; integrity owns tamper corpora against partial-read states and encrypted salvage |
| EXP-INT-017 | EXP-CRYPTO-019 | DEC-LEG-009 | crypto owns receipt signing and forgery analysis; integrity owns reproducibility and gate mutation tests |
