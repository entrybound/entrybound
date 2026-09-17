# EXP-REMOTE-012: Remote random access versus incumbent remote readers and full download (claims evidence)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T8 incumbent remote readers and structure parsers; T1, T2, T5; incumbent archive creation reuses `research/baselines/configs.json`). Sub-arms BLOCKED: SOCI (needs containerd/Docker, unavailable), Apple Archive (macOS PLATFORM_BLOCKED), real endpoints (EXP-REMOTE-013). |
| Domain / program section | remote / §12 (§35 final evaluation claims; §22.2/§22.4 capability matrix) |
| decision_ids | DEC-ACC-041, DEC-ACC-002 |
| Decision type (provisional) | EMPIRICAL (claims must be evidence-backed); the claim wording itself is a synthesis act in Phase E |
| OD / HC (provisional until G-A) | OD-12 (M12.1-M12.3), OD-11 (M11.1, M11.3), OD-10 guard (local random access), OD-05 guard (artifact bytes from EXP-BASE-SIZE); HC-15 (Entrybound reads report verified states truthfully; incumbents' unverified reads are labelled), HC-18 (screen via EXP-REMOTE-009) |
| Author / L7 / gates | As EXP-REMOTE-001. External comparison claims need both reference arms (REF-PROD and REF-INC, objective §3.0) and follow `decision-method.md` §6.5 "Reference arm"; timing comparisons against incumbents use production-built `ebound` (harness review use constraint 2) for any local timing, while remote latency is modelled from exact request logs for every system identically. |

## 1. Question

What remote random-access performance can v1 honestly claim — requests, bytes and latency relative to full download and to capability-matched incumbent remote readers (ZIP central-directory range readers, 7z non-solid, indexed or seekable tar streams, eStargz, squashfs/EROFS/DwarFS images, indexed xz) — under the canonical network profiles and warm/cold caches, and which capability-matrix marks for remote access, partial retrieval, verified partial reads and lazy pull survive fair measurement?

## 2. Hypotheses

- **H1 (bounded requests).** On every tuning archive, status-quo Entrybound request counts per operation stay within the formula bound open-cost + closure-cost of EXP-REMOTE-002's `fetch-model` (binary claim check).
- **H2 (ZIP parity).** For WL-RAND1 and WL-FIRST, status-quo Entrybound is **not** `NON_INFERIOR` to a ZIP central-directory range reader at NP-CONT and NP-INTER (per-Chunk header requests and HEAD revalidations cost extra round trips), while the provisional remote-domain winner (EXP-REMOTE-003/004 W composed, `entrybound-provisional-w`) is `NON_INFERIOR`.
- **H3 (lazy pull vs eStargz).** For WL-SUBTREE and WL-FRACTION-0.1, `entrybound-provisional-w` is `EQUIVALENT` or better than an eStargz TOC reader and `DISTINGUISHABLE_BETTER` than full download at NP-CONT/NP-INTER; the SPEC "Exceed" mark requires `DISTINGUISHABLE_BETTER` vs eStargz, which H3 predicts is not reached.
- **H4 (verified partial reads).** Entrybound `verified_fraction` = 1 on every partial read while incumbent range readers verify nothing beyond CRC (ZIP/7z) or TOC digests (eStargz), so CC-07's OD-11 non-inferiority test is decided by bytes and latency overhead of verification.
- **H0 / falsification.** H1 falsified by one operation above the bound; H2 falsified if the status quo is already `NON_INFERIOR` to ZIP or the provisional W is not; H3 falsified if W is `DISTINGUISHABLE_WORSE` than eStargz or not better than full download; H4 falsified if any Entrybound partial read is reported verified without verification.

## 3. Decisions informed

| Decision | Supplied here | Remaining |
|---|---|---|
| DEC-ACC-041 (V1_BLOCKING) | Application-level emulation and model matrix over canonical profiles (loss excluded from performance claims, §4.15) with cold/warm sessions; incumbent baselines: ZIP range readers, indexed/seekable tar, eStargz, squashfs/EROFS/DwarFS (structure-derived request plans), indexed xz (pixz), full download; the evidence for each claim candidate | Real S3/R2/GitHub Releases/CDN endpoints: EXP-REMOTE-013 (BLOCKED); SOCI snapshotter: BLOCKED (containerd/Docker); held-out workload set frozen before tuning: evaluation domain's Phase D design (sealed supplement, `decision-method.md` §5.4) — placeholder in section 15 |
| DEC-ACC-002 | Measured remote-access, partial-retrieval, verified-partial-read and lazy-pull evidence per incumbent for the capability-matrix marks | Streaming and parallel-decode marks: scale domain (§13-§14); primary-source verification of incumbent characterisations: analytical citation work (§9.3); Apple Archive: PLATFORM_BLOCKED; held-out evaluation (Task 35): evaluation domain |

## 4. Candidates

### 4.1 Systems measured

| system_id | Archive creation | Remote reader | Realization |
|---|---|---|---|
| `entrybound-status-quo` (REF-PROD) | `ebound pack --profile balanced` (INDEXED) | production `HttpRangeSource` random reader | EXP-REMOTE-002 traces |
| `entrybound-provisional-w` | as above (or the W archive variant if a planner-order candidate won) | composition of the remote-domain tuning winners (coalescing, discovery/revalidation, cache, concurrency) fixed at the first validation look of those experiments | `ebr-remote` with source-layer prototypes |
| `zip-cd-range-reader` (REF-INC for CC-07) | `zip -6 -r -X -y` (`configs.json` `zip-6`) | research reader: fetch EOCD (tail 65,557 B max), central directory range, then local header + data per member; block readahead b ∈ {0, 64 KiB, 1 MiB}, best b per workload chosen on tuning and frozen | T8 `zip_range_reader.py` |
| `7z-nonsolid-range-reader` (REF-INC for CC-07) | `configs.json` `7z-mx9-nosolid` | research reader over an HTTP range file object (py7zr with the same readahead sweep) | T8 (pinned py7zr, download approval under PROGRESS standing decisions) |
| `tarzst-seekable-reader` | tar with one zstd frame per member plus a seek table (zstd seekable format) | seek table from the tail, then frame ranges | T8 writer and reader (pinned tool under download approval; otherwise a research writer following the published seekable-format specification) |
| `estargz-toc-reader` | eStargz layer (gzip member per file chunk + TOC JSON + footer) written by a research writer following the stargz-snapshotter eStargz specification | footer + TOC, then chunk ranges; TOC digest checked, per-chunk digests checked when present | T8 |
| `pixz-index-reader` | `configs.json` `tarxz-pixz` | xz block index from the tail, then block ranges (structure-derived plan) | T8 plan from `xz --robot --list -vv` |
| `squashfs-image-reader`, `erofs-image-reader`, `dwarfs-image-reader` | `configs.json` `squashfs-zstd`, `mkfs.erofs -zlz4hc` (Ubuntu erofs-utils, added config), `configs.json` `dwarfs-l7` (pinned v0.15.7) | structure-derived request plans (superblock, inode/directory tables, fragment and data blocks) from each tool's metadata dump | T8 `imgplan.py` (labelled `structure-derived`) |
| `full-download-then-extract` | `configs.json` `tarzstd-3-t1` | one GET of the whole object, local extraction of the requested members | model + measured local extraction CPU (quiet window) |
| `soci-ztoc` | OCI image layer + SOCI zTOC | snapshotter lazy pull | **BLOCKED**: SOCI tooling requires containerd/Docker, unavailable on this host |
| `apple-archive` | `aa` | – | **PLATFORM_BLOCKED** (macOS) |

### 4.2 Claim candidates (DEC-ACC-041) and the evidence each needs

| claim candidate | Supported only if |
|---|---|
| `claim-functional-only` (fallback) | always admissible |
| `claim-bounded-requests` | H1 holds on tuning, validation and held-out |
| `claim-zip-http-parity` | the claimed Entrybound configuration is `NON_INFERIOR` to `zip-cd-range-reader` on the OD-12 primary in every profile × workload condition of the claim's scope (R4 condition 3 guard per family) |
| `claim-lazy-pull-exceed` | `DISTINGUISHABLE_BETTER` than `estargz-toc-reader` (and SOCI, which is BLOCKED, so any "Exceed vs SOCI" wording stays unsupported) |
| `redesign-before-claims` | selected if only `entrybound-provisional-w` meets a claim that the status quo does not |

## 5. Corpus selectors

- Tuning items applicable to every system: all tuning items that every incumbent can archive losslessly for content (EXP-BASE-SIZE `roundtrip_ok` = 1 for that config; items failing an incumbent are excluded for that system only and listed); stratified by scale tier; eStargz and OCI-shaped comparisons additionally on F16 OCI items.
- Workloads: WL-OPEN (list), WL-FIRST, WL-RAND1, WL-RANDK16, WL-SUBTREE, WL-LARGE, WL-FRACTION-0.1, WL-FULL, cold and warm (WL-REVISIT session 2).
- Validation look: validation items under the same rule; held-out: section 15.

## 6. Environment and platform requirements

`wsl-ubuntu`; exact request logs for every system via `ebr-origin` over loopback (incumbent readers and structure-derived plans go through the same origin or the same `fetchsim`/model code path, so every system is priced by the identical latency model); emulated corroboration for Entrybound W, ZIP and eStargz at NP-CONT/NP-INTER on E12; local extraction CPU for full download in a quiet window with production-built tools. Linux x86-64; SOCI BLOCKED (Docker/containerd), Apple Archive PLATFORM_BLOCKED; about 60 GB for incumbent archives of the tuning corpus (shared with EXP-BASE-SIZE artifacts where their raw outputs retain them).

**Platform matrix.** Linux x86-64 required (WSL2); Windows: not used; macOS: Apple Archive arm PLATFORM_BLOCKED; ARM64: not required; network: loopback emulation (real endpoints in EXP-REMOTE-013); privileged: no; large storage: ~60 GB; external review: no (claims are internal evidence; wording reviewed at Phase E).

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound; PY=/root/eb-research/venv/bin/python; export PYTHONPATH=research/tools
python research/orchestration/disk_guard.py && python research/harness/lock_parity.py
$PY research/tools/remote/prepare_incumbents.py ensure --configs research/baselines/configs.json --systems zip-6,7z-mx9-nosolid,tarzstd-3-t1,tarzst-seekable,estargz,tarxz-pixz,squashfs-zstd,erofs-lz4hc,dwarfs-l7 \
    --items research/experiments/EXP-REMOTE-002/items-tuning-all.txt --cache /root/eb-research/cache/remote-incumbents
$PY -m ebr run --spec research/experiments/EXP-REMOTE-012/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-REMOTE-012/spec.yaml && $PY -m ebr normalize --spec research/experiments/EXP-REMOTE-012/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-REMOTE-012/spec-emulated.yaml --timing     # generated after tuning: W, zip, estargz on E12
$PY -m decide analyze --experiment EXP-REMOTE-012 --claims research/experiments/EXP-REMOTE-012/claims.json
```

## 8. Seeds, warmups, repetitions

Seeds `order` 20261020, `bootstrap` 20261021, `command` 20261022. Deterministic request-log arms: 2 repetitions + repeat-hash per system. Readahead sweeps for ZIP/7z readers are tuning-only selections frozen before validation. Emulated and local timing arms: warmups 2 (large 1), rounds min 10, tier step/cap, precision rule.

## 9. Metrics

| Metric | OD | Role | Threshold |
|---|---|---|---|
| `remote_task_latency_s` per (profile, workload) — modelled identically for all systems | OD-12 | **primary** | T-10 |
| `verified_range_fetch_bytes` (bytes to a verified result; for unverifying incumbents, bytes to an unverified result, labelled) | OD-11 | **primary** (CC-07 OD-11 view) | T-11 |
| `http_requests`, `http_bytes` | OD-12 | secondary | T-12, T-11 |
| `verified_fraction` | OD-11 | secondary (reported per system; incumbents' verification scope described) | T-20 |
| `random_entry_latency_s` local (in-process batches) | OD-10 | guard | T-08 |
| `artifact_bytes` of each system's archive (from EXP-BASE-SIZE raw or re-measured) | OD-05 | guard | T-01 |
| `requests_within_bound` (H1) | – | binary claim check | §3.3 |

## 10. Normalization and statistical analysis

- Reference arm rule (§6.5): every external claim reports REF-PROD (`entrybound-status-quo`) and REF-INC (per workload the capability-matched incumbent: ZIP/7z for single-entry reads, eStargz for lazy pull, full download for whole reads).
- Item = (item, system archive); L2-L4 with `workload-mix.json` and equal weights; strata per tier, profile, workload, cold/warm; conditions never averaged.
- Claims are evaluated with non-inferiority and superiority at the confidences of §4.12 on validation; family guard §4.9; a claim's scope is the intersection of conditions where it holds (R9-style narrowing, recorded as limitations).
- Negative results (workloads where an incumbent wins) are recorded per §10.1 item 2 with magnitudes from generated tables.

## 11. Practical significance

T-10, T-11, T-12, T-20, T-08, T-01 by reference.

## 12. Sensitivity analysis

Slow-start and connection-model arms (incumbent readers with large readahead are the most slow-start-sensitive); readahead b alternatives for ZIP/7z; warm vs cold; structure-derived vs measured plans (for DwarFS/squashfs if a FUSE-over-HTTP reader becomes available); byte-weighted arm; §6.5 family arms.

## 13. Expected negative results worth recording

- Status-quo Entrybound loses to a ZIP central-directory reader on single-small-file latency at high RTT (extra header and HEAD requests).
- eStargz and Entrybound W are equivalent for lazy pull; the SPEC "Exceed" mark is not supported.
- Full download wins for high read fractions on many-small-file archives.
- Incumbent readers verify less; Entrybound's verification costs bytes and time that the claims must declare.

## 14. Threats to validity

- Research readers and writers for incumbents may be less optimized than production clients (mitigated by readahead sweeps and by pricing all systems with the same latency model from exact request logs).
- Structure-derived plans for filesystem images are not measured reads.
- Emulated and modelled networks only; real CDN and object-store effects are in EXP-REMOTE-013 (BLOCKED).
- SOCI and Apple Archive absent.

## 15. Held-out (Phase D placeholder)

DEC-ACC-041 requires a held-out workload set frozen before tuning (Task 35). This experiment's held-out run is designed only as a placeholder: after the single program-wide freeze (`decision-method.md` §5.5), the frozen claim configurations and incumbent readers are rerun once on held-out items (and on the evaluation domain's sealed supplementary tranche if it exists, §5.4 sealing route, which lifts the `identity_aware_heldout` cap), report-only per §5.6, with held-out MDEs computed before unlock (R5).

## 16. Estimates

- Compute: about 20 machine-hours (incumbent archive creation over the tuning corpus about 10 h, reusing EXP-BASE-SIZE artifacts where kept; request-log runs about 6 h; emulated corroboration and local extraction timing about 4 h).
- Disk: about 60 GB peak for incumbent archives (removable, regenerable).
- Agent effort: scripted; judgment for reader fairness review, claims analysis and negative-results register entries.
