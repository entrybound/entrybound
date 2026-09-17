# EXP-SCALE-008: Bounded-memory read, verify, repack and staging prototypes

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** |
| Domain | scale (program §13; interfaces with integrity §21/§24, access §10, container §11) |
| Platforms | Linux x86-64 (WSL2, root, loop-mounted ext4/XFS/btrfs); Windows 11 NTFS host (staging arm and timing second environment) |
| Requires timing | true (phase T: verify, unpack, time to first file); phases M and F need no timing |
| Compute estimate | about 60 machine-hours (M about 25 h, F about 15 h, T about 20 h) |
| Disk estimate | about 300 GB peak (64 GiB probes with staging; the >64 GiB cumulative-staging archive about 70 GB plus spill) |
| Agent effort | judgment (prototypes); scripted execution |
| Tooling to build | research-only harness crate `research/harness/crates/ebr-scale-reader` (binary `ebr-scale-reader`) implementing the designs below over the public API and `research-internals`; `memmap2` dependency (pinned; contains `unsafe`, harness-only) for `mmap-input`; `ebr-pack --raise-limits` to create >64 GiB STREAM archives for the cumulative-staging case; shared counting allocator `ebr-alloc` (EXP-CONF-007); `fault_cell.py` scenarios of EXP-SCALE-004 parameterized with `--reader ebr-scale-reader --design <d>`; runner additions §14 item 3 and calibration §4.7 for phase T |

## Pre-registration

- **decision_ids:** DEC-ACC-062, DEC-ACC-060, DEC-ACC-048, DEC-ACC-004, DEC-ACC-059, DEC-ACC-006, DEC-ACC-053, DEC-ACC-055, DEC-INT-039, DEC-INT-043, DEC-ACC-022, DEC-CON-026.
- **Decision type:** EMPIRICAL (memory, disk, time, failure oracles); security parts of staging decisions are EXTERNAL (not addressed).
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-07 (T-04 `verify_wall_s`, `decode_wall_s`, `stream_unpack_wall_s`; M07.4), OD-08, OD-09 (M09.3), OD-20 (repack identity), OD-10 (scan-and-cache); HC-05 (MVT-05(d)), HC-06, HC-10, HC-15 (verification honesty of each level), HC-18.
- **Method, gate, author, L7:** as EXP-SCALE-001; the candidate set needs the R0 item 6 critic candidate and non-exposed sign-off before G-A.

## Question

For readers, repack and extraction staging, which designs bound memory for archives larger than RAM while keeping verification honest, crash and corruption behaviour safe, repack identities exact, and time to first verified output competitive; what do verification levels cost and guarantee; what does STREAM metadata retention cost under spill or incremental validation; what do scan-and-cache access to STREAM and frame-reusing repack cost; and does cumulative staging accounting falsely refuse long archives?

## Hypotheses

- **H1a:** `range-reader-verify` and `two-phase-verify-then-extract` bound INDEXED verify and unpack memory (M08.3 holds on the bytes axis) with `verify_wall_s` within 1 band unit (T-04) of the status quo; `two-phase` detects a source file replaced between phases (TOCTOU) in 100% of seeded replacements.
- **H1b:** `metadata-spill` and `incremental-sparse-map-validation` make STREAM verify bounded on the entries axis at a cost of more than 1 band unit of `verify_wall_s` at 1M entries.
- **H1c:** `streamed-representation-only-repack` and `chunk-reuse-repack-add` bound memory and preserve LAI, AUX, PCR exactly (HC-10), and PCI for representation-only.
- **H1d:** `per-entry-verified-streaming` and `quarantine-dir-per-object` cut time to first verified file by more than 2 band units (T-08 band) versus stage-all on multi-GiB STREAM archives while never exposing an unverified final object (MVT-05(d)); `opt-in-progressive-unverified` is faster still but reports UNVERIFIED objects (HC-15 honesty check).
- **H1e:** status-quo cumulative `total_bytes` accounting refuses extraction of a STREAM archive with 70 GiB plaintext even when concurrent staged bytes stay below 256 MiB; `concurrent-accounting-with-reuse` does not.
- **H0 / falsification:** per candidate, a binary counterexample (unbounded growth, identity mismatch, unverified object exposed as verified, TOCTOU miss, leftover plaintext spill after a normal exit) or a non-supporting confirmatory verdict.

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-ACC-062 | RSS and time of verify/list/unpack of large INDEXED archives per candidate; TOCTOU analysis for two-phase designs against file sources; crash-safety of staged extraction commit | status quo: EXP-SCALE-001, EXP-SCALE-006 |
| DEC-ACC-060 | Reader memory for 1M-10M entries / 4M-40M chunks under metadata spill, incremental sparse-map validation and declared metadata bound | — |
| DEC-ACC-048 | Repack peak memory vs archive size per mode and layout pair; identity equality of streamed repack vs reference | — |
| DEC-ACC-004 | Frame-reuse feasibility and cost of a chunk-reuse `repack --add` prototype vs full rewrite | demand survey: human participants (not measurable) |
| DEC-ACC-059 | Cost of repack vs scan-and-cache session index on large STREAM archives | trailing index and sidecar index are format changes: container domain |
| DEC-ACC-006, DEC-ACC-053, DEC-ACC-055, DEC-INT-039 | Disk and memory overhead, time to first file, completion and failure behaviour (disk full, kill, late corruption) per staging design; cross-filesystem rename; false refusal of cumulative accounting on long archives; spill encryption and unlink-on-create cleanup behaviour | security reviews: external |
| DEC-INT-043 | Time, peak memory and bytes read per verification level; what each level detects on the EXP-SCALE-004 corruption cases | fault-injection coverage across all damage classes: integrity domain |
| DEC-ACC-022 | Memory and refusal behaviour of `bounded-buffer-indexed-stdin` | first-use study: human participants |
| DEC-CON-026 | Reader cost (CPU, allocator peak) of computing the minimum dedup window while reading, as required by `reject-non-minimal-window` | — |

## Candidates

| Group | candidate | Definition |
|---|---|---|
| INDEXED read | `sq-indexed` | production open/verify/unpack (reference) |
| | `range-reader-verify` | verify through `LocalFileRandomReadSource`, discarding plaintext after digest checks |
| | `per-object-staged-extraction` | verify each ContentObject into bounded staging, commit at end |
| | `mmap-input` | memory-map the archive, decode lazily |
| | `two-phase-verify-then-extract` | full plaintext-discarding verify, then extract re-decoding per entry; re-checks file identity (size, mtime, inode, and a digest of the footer) between phases |
| | `declared-ceiling` | status quo plus typed refusal above a caller ceiling before reading |
| Verify levels | `level-structure` | structure and section digests, no chunk decode |
| | `level-stored-digest` | manifest-level digests without decoding (feasible only as far as the current wire carries digests; the prototype records which classes it cannot check) |
| | `level-full` | status-quo full decode verify |
| STREAM metadata | `sq-stream-retain` | production STREAM reader (reference) |
| | `metadata-spill` | Entry/ContentObject metadata spilled to staging |
| | `incremental-sparse-map-validation` | sparse maps validated as ContentObjects arrive |
| | `declared-metadata-bound` | enforce a declared metadata retention bound |
| Repack | `sq-repack` | production repack (reference) |
| | `streamed-representation-only-repack` | copy frames and records with bounded memory, recompute Index and footer |
| | `streamed-replan-with-spill` | replan with spilled plaintext and EXP-SCALE-007's windowed planner |
| | `verify-output-by-streaming-reader` | output verification with plaintext-discarding readers |
| | `chunk-reuse-repack-add` | full logical rewrite adding entries while reusing encoded frames |
| STREAM access | `scan-and-cache-explicit` | explicit sequential scan building a session index for a seekable STREAM file (reported as Sequential per SPEC §19.4) |
| Staging | `sq-resident-plus-spill` | production (reference) |
| | `quarantine-dir-per-object` | verified objects materialized into a hidden sibling directory, renamed at the end |
| | `destination-adjacent-spill` | spill files on the destination filesystem |
| | `memory-only` | refuse above the resident bound |
| | `two-pass-reread` | verify pass, then extraction pass re-reading a seekable source |
| | `per-entry-verified-streaming` | materialize each entry after its chunks and content digest verify; archive roots reported at end |
| | `temp-tree-then-atomic-move` | private temp tree moved into place after full verification |
| | `opt-in-progressive-unverified` | emit immediately, report UNVERIFIED until completion |
| | `concurrent-accounting-with-reuse` | `total_bytes` bounds live staged bytes with space reuse |
| | `ephemeral-key-spill` | spill encrypted with an in-memory ephemeral key |
| | `unlink-on-create` | unlinked temp files (Linux `O_TMPFILE`; Windows `FILE_FLAG_DELETE_ON_CLOSE`) |
| Non-seekable | `bounded-buffer-indexed-stdin` | buffer INDEXED stdin up to a declared bound |
| Window check | `min-window-on-read` | compute the minimum dedup window while reading STREAM |
| References | `inc-zstd-tar` | `zstd -dc | tar -x` (REF-INC for time to first file) |

- **Excluded:** `progressive-reported-verified` and `unverified-unmarked-streaming` (L0: they present unverified output as verified, HC-15/I25; counterexample: any late-corruption case of EXP-SCALE-004; exclusion needs independent sign-off per objective §2.0 rule 3); `silent-scan-fallback` (L0: SPEC §19.4 complexity-visible rule); `unbounded-silent-buffering` (R2: unboundable).

## Corpus selectors

Phase M/F: generated EXP-SCALE-001 shapes (`b1g`..`b64g`, `n1e5`, `n999k`), raised-policy `n4m`, `n10m`, POSIX-metadata `b4g`, and `stream-70g` (70 GiB plaintext, 64 files, seed 1801, produced with `ebr-pack --raise-limits`). Phase T: `b4g`, `b16g`, and tuning items of F01, F05, F13, F15 (manifest, split `tuning`), then one registered validation look on the same families. **Held-out placeholder (Phase D):** provisional selections re-run once on held-out items of the same families after unlock (§5.5); phase M on sealed generator seeds.

## Environment

Phase M/F as EXP-SCALE-001 and EXP-SCALE-004 (caps, loop mounts including XFS and btrfs, kill and fault scenarios). Windows NTFS for staging designs (`quarantine-dir-per-object`, `temp-tree-then-atomic-move`, `destination-adjacent-spill`, `unlink-on-create`). Phase T: `timing: true`, WSL `st-v`, Windows `st-p`, calibrations as EXP-SCALE-007.

## Commands

```sh
wsl.exe -d Ubuntu -- bash research/harness/build.sh build --release -p ebr-scale-reader -p ebr-pack --features ebr-alloc
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-SCALE-008/spec.yaml            # phase M
/root/eb-research/venv/bin/python research/tools/scale/fault_cell.py --experiment EXP-SCALE-008 --reader ebr-scale-reader ...       # phase F via spec-fault.yaml (generated from EXP-SCALE-004 scenarios)
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --timing --spec research/experiments/EXP-SCALE-008/spec-timing.yaml
```

Binary contract: `ebr-scale-reader --design <candidate> --cell <id> --cells <cells.json> --archive <path> --dest <dir> --isolate true --seed <n> --experiment-id EXP-SCALE-008 --env-name <env>` prints one `ebr.harness.raw.v1` row.

## Seeds

`seeds.order` 1308001, `seeds.bootstrap` 1308002, `seeds.command` 1308003.

## Warmup

M and F: 0. T: §4.5.

## Measurement method and metrics

| Metric | Canonical name | Role | Family |
|---|---|---|---|
| `alloc_peak_bytes`, `bounded_memory_growth_bytes` | T-06; M08.3 | primary; binary | memory_peak; correctness |
| `scratch_write_bytes`, `scratch_peak_bytes` | §4.14; T-07 | primary | correctness; scratch_peak |
| `verify_wall_s`, `decode_wall_s`, `stream_unpack_wall_s` | T-04 | primary (phase T) | time_wall |
| `first_entry_latency_s` (time to first verified file, in-process) | T-08 | primary for staging (phase T) | time_wall |
| `open_bytes_read` | T-11 | primary for levels and scan-and-cache | size |
| `verified_fraction` | T-20 (M11.3) | honesty per level and progressive mode | other |
| `dest_objects_before_verify`, `verified_ok_partial_outputs`, `spill_leftover_bytes`, `toctou_detected` | HC-05, HC-18, cleanup, TOCTOU | binary | correctness |
| `identity_lai_equal`, `identity_aux_equal`, `identity_pcr_equal`, `identity_pci_equal` | HC-10, repeat-hash | binary | correctness |
| `false_refusal_long_archive` | `benign_false_refusal_fraction` input | binary per archive | correctness |
| `min_window_cpu_s`, `min_window_alloc_peak_bytes` | T-05 in-process; T-06 | primary for DEC-CON-026 | time_cpu; memory_peak |

## Replication

M08.3 probes: 3 runs per size. Failure scenarios: 3 repetitions × 25 kill points (as EXP-SCALE-004). Phase T: §4.6 with the precision rule.

## Normalization and statistical analysis

As EXP-SCALE-007: per decision, R3/R4 on tuning to name W and S, MDE gate, one validation look, §4.10-§4.12 verdicts, family and tier guards, §7.2 computed tiers (for example `range-reader-verify` T0 if byte-compatible and behaviour-identical; `quarantine-dir-per-object` T1 policy; `interleaved` or level candidates needing new digests T2-T4), R6, binary oracles by counterexample.

## Practical-significance thresholds

Referenced by ID: T-04, T-06, T-07, T-08, T-11, T-20; M08.3; §7.3 minimum gains.

## Sensitivity analysis

Full §6 for decisions reaching R8/R9; environment arm WSL vs Windows for staging; filesystem arm ext4/XFS/btrfs/NTFS; archive-size tier arm.

## Expected negative results worth recording

Bounded INDEXED verify slower than the status quo beyond the T0 minimum gain; metadata spill doubling verify time at 1M entries; quarantine designs doubling destination disk; `mmap-input` giving no memory benefit under cgroup accounting (page cache charged); cumulative accounting refusing a legitimate 70 GiB archive.

## Threats to validity

1. Prototype constant factors (as EXP-SCALE-007).
2. cgroup-v1 charges mapped file pages to the cgroup, penalizing `mmap-input` relative to native hosts; reported with the allocator peak (which excludes mappings) beside RSS.
3. Kill oracles are regression evidence without research-build phase boundaries.
4. Windows rename and Defender interactions (as EXP-SCALE-004).

## Platform requirements

Linux x86-64 WSL2 root; Windows 11 NTFS non-admin; about 300 GB; calibration and runner additions for phase T.

## Estimated compute and effort

About 60 h after tooling; tooling judgment-heavy.

## Deviations (append-only)

None.
