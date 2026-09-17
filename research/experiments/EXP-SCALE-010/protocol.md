# EXP-SCALE-010: Encrypted archives at scale (status-quo memory, key mutation cost, crypto-v1 resource ceilings, bounded encrypted prototypes)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** |
| Domain | scale (program §13; interfaces with crypto §22.1-§22.5 and container §11) |
| Platforms | Linux x86-64 (WSL2, root, cgroup-v1 caps and `taskset` for low-end emulation) |
| Requires timing | true for the Argon2id and unlock-time parts (phase T); memory and outcome parts need none |
| Compute estimate | about 30 machine-hours |
| Disk estimate | about 120 GB peak |
| Agent effort | judgment (identity-file tooling, KDF-parameter archive generator, prototypes); scripted execution |
| Tooling to build | (1) `ebr-pack --encrypt recipients --recipients <n> --identity-out <dir>` (write X-Wing identity and recipient files so CLI `key add/remove` and `--identity` unlock can run) and `--kdf-memory-mib/--kdf-passes/--kdf-parallelism` for password archives at the wire ceiling (through the public creation options if exposed; otherwise a default-off research-internals wrapper, identity-checked); (2) `research/tools/scale/pty_password.sh` (feeds a password to `ebound key change-password` and `--password` prompts through `script -qec`, never logging it; the password comes from the harness sidecar); (3) `ebr-access --mode alloc-probe` operations for encrypted open/verify/unpack/range sessions (shared `ebr-alloc`, EXP-CONF-007); (4) research-only crate `research/harness/crates/ebr-scale-crypto` with the prototypes below; (5) `ebr-forge --case kdf-over-ceiling` and EBCS/segment boundary archives (format-building code, coordinated with the conformance domain's vectors) |

## Pre-registration

- **decision_ids:** DEC-ACC-009, DEC-ACC-011, DEC-ACC-028, DEC-CON-009, DEC-CRY-013, DEC-CRY-050, DEC-CRY-069.
- **Decision type:** EMPIRICAL for resource and outcome measurements; cryptographic soundness, leakage and final construction choices are EXTERNAL (`EXTERNAL_REVIEW_REQUIRED`, SPEC §25.2) and are not addressed here.
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-08, OD-15 (M15.3 encryption overhead), OD-17 (M17.2 refusal cost for KDF over ceiling); HC-14 (no weakening; MVT-14 vector gate must still pass on every prototype output), HC-06 (KDF refused before running), HC-13 (MVT-13(b)), HC-18.
- **Method, gate, author, L7:** as EXP-SCALE-001; crypto prototypes additionally require crypto-domain review of their constructions before any result is cited (they are resource prototypes, not proposals of new constructions).

## Question

How do encrypted creation, open, verification, extraction and range sessions scale in memory with plaintext, entries and session length; what do recipient addition, removal and password change cost in time and memory on multi-GiB archives; are the crypto-v1 structural limits and Argon2id and recipient ceilings reachable and safe on constrained hosts (time and memory at the ceiling, refusal before running for over-ceiling archives, unlock time at 1,024 stanzas); and what resource benefits would streamed segment emission, plaintext-discarding open, bounded decrypted-frame caches, spool-to-temp encrypted pipe output and boundary-preserving rotation bring?

## Hypotheses

- **H1a (status quo, from `crypto/container.rs`, `wire.rs`):** encrypted pack, open, verify and unpack are PLAINTEXT-PROPORTIONAL (whole-buffer write and open, cloned PAYLOAD reuse map); long-lived range sessions grow with the number of distinct frames read.
- **H1b:** `key add` rewraps without touching payload (time and memory independent of archive size, within the T-04 floor of an open); `key remove` and `change-password` re-encrypt and re-chunk (plaintext-proportional).
- **H1c:** opening a password archive at the wire ceiling (1 GiB, 10 passes, parallelism 16) completes under a 2 GiB cap on 2 CPUs and is OOM-killed or refused under a 1 GiB cap; an archive declaring KDF cost above caller policy is refused with zero Argon2id invocations (HC-06); unlock time at 1,024 recipient stanzas grows linearly with stanza count.
- **H1d:** crypto-v1 EBCS limits: 1,000,000 manifest items packs and opens; 1,000,001 is refused at creation with a typed code; the effective EBCS byte maximum boundary (2^30 − 12 vs 2^30 − 11) behaves as documented once the conformance vectors exist.
- **H1e:** `streamed-segment-writer` + `plaintext-discarding-open` make encrypted pack and verify M08.3-bounded on the bytes axis with ciphertext and identities satisfying MVT-13(b) and the HC-14 vector gate.
- **H0 / falsification:** a single counterexample per binary claim; confirmatory verdicts per §4.11 for graded costs.

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-ACC-009 | Status-quo refusal before output bytes for encrypted STREAM/pipe requests (zero stdout bytes, code); temporary disk and memory of `explicit-spool-to-temp` | workflow study: human participants; privacy design of encrypted sequential layouts and AEA comparison: external review / literature |
| DEC-ACC-011 | Peak memory and latency (phase T) for encrypting and opening 1-64 GiB archives and 1M entries; memory growth of long-lived encrypted range sessions; prototypes | leakage analysis of sharded encrypted metadata: crypto domain, external review |
| DEC-ACC-028 | Encrypted rows of the operation inventory | — |
| DEC-CON-009 | Segment count and directory size vs limits on large archives; peak memory per segment and object; million-entry encrypted archive tests; boundary archives at the EBCS limits (outcome only) | boundary conformance vectors: conformance domain |
| DEC-CRY-013 | Argon2id time and memory at the ceiling under memory caps and CPU restrictions (low-end emulation); envelope size and unlock time at 1,024 stanzas; hostile-archive (KDF over policy) resource use | final default choice rationale depending on attacker cost models: crypto domain |
| DEC-CRY-050 | Rotation cost of full rotation vs `boundary-preserving-rotation` prototype on multi-GiB archives; PCR and binding status after rotation (recorded values only) | leakage analysis of reused boundary keys: crypto domain, external review |
| DEC-CRY-069 | Time and peak memory of add, remove and change-password on multi-GiB archives | sibling-archive splicing analysis, composition with signatures, usability: crypto domain and human participants |

## Candidates

| Group | candidate | Definition |
|---|---|---|
| Status quo | `sq-pack-password`, `sq-pack-recipients` | `ebr-pack --encrypt password|recipients` (library `pack_directory_encrypted`, CLI-equivalent) |
| | `sq-open-verify`, `sq-unpack`, `sq-range-session` | `ebound verify/unpack --identity`, `ebr-access --mode alloc-probe --operation encrypted-range-session --reads 1..100000` |
| | `sq-key-add`, `sq-key-remove`, `sq-change-password` | CLI with identity files and `pty_password.sh` |
| | `sq-stream-password-stdout` | `ebound pack IN - --layout stream --password` (expected refusal before output) |
| Ceilings | `kdf-ceiling-open-cap<c>-cpu<n>` for c in {512 MiB, 1 GiB, 2 GiB, 24 GiB}, n in {1, 2, 16} | open a password archive created at 1 GiB / 10 passes / p = 16 under the cap with `taskset` |
| | `kdf-creation-default-open` | open at the creation default (256 MiB / 3 passes) under the same grid |
| | `kdf-over-policy` | forged archive declaring KDF cost above the caller ceiling; instrumented Argon2id counter |
| | `recipients-<k>` for k in {1, 16, 128, 1024} | unlock time and envelope bytes with the matching identity at the last stanza (worst case) and at the first |
| | `ebcs-items-<n>` for n in {999999, 1000000, 1000001} | encrypted pack and open of trees with n entries (64 B files) |
| | `segment-boundary` | single entry of 1 GiB + 1 byte and 2^20 DATA records + 1 (small-chunk profile) to exercise segment splits |
| Prototypes | `streamed-segment-writer`, `plaintext-discarding-open`, `bounded-decrypted-frame-cache-<b>` (b in {64 MiB, 256 MiB}), `explicit-spool-to-temp`, `boundary-preserving-rotation`, `lazy-rotation-on-repack` | as named in DEC-ACC-011, DEC-ACC-009, DEC-CRY-050 |

- **Excluded:** `silent-layout-switch-or-buffer` (L0: violates `docs/crypto-review-v1.md` L136-142 no silent layout change; counterexample is the specification clause; independent sign-off required); `convergent-encryption-mode` and `rewrap-only-removal` (L0: HC-14 statement forbids deterministic ciphertext and removal without file-key rotation per `docs/crypto-threat-model-v1.md` L105-107); `sharded-manifest-index-v2` resource measurements are included only if the crypto domain supplies a construction (otherwise excluded as not specified).

## Corpus selectors

Generated EXP-SCALE-001 shapes `b1g`, `b4g`, `b16g`, `b64g` (stage 2 rule as EXP-SCALE-001), `n1e5`, `n999k`; EBCS item trees (seeds 2001-2003); segment-boundary trees (seeds 2004-2005); forged KDF archive (seed 2006). Phase T: `b1g`, `b4g` and F01/F05 tuning items. **Held-out placeholder (Phase D):** any provisional selection (for example a changed default KDF open policy) re-run once on sealed generated seeds and held-out F01/F05 items after unlock (§5.5).

## Environment

As EXP-SCALE-001 (caps, loop mounts); low-end emulation by cap plus `taskset -c 2` or `-c 2-3`, labelled `EMULATED-LOW-END` (not a real device). Phase T: `timing: true`, WSL `st-v` and `mt-v-2`, calibration as EXP-SCALE-007.

## Commands

```sh
wsl.exe -d Ubuntu -- bash research/harness/build.sh build --release -p ebr-pack -p ebr-access -p ebr-scale-crypto --features ebr-alloc
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-SCALE-010/spec.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --timing --spec research/experiments/EXP-SCALE-010/spec-timing.yaml
```

Binary contract: `ebr-scale-crypto --design <candidate> --cell <id> --cells research/experiments/EXP-SCALE-010/cells.json --seed <n> --scratch <dir> --experiment-id EXP-SCALE-010 --env-name wsl-ubuntu`, one `ebr.harness.raw.v1` row; for status-quo CLI operations the driver is `scale_cell.py` with `memcap_run.sh` and `pty_password.sh`.

## Seeds

`seeds.order` 1310001, `seeds.bootstrap` 1310002, `seeds.command` 1310003.

## Warmup

Memory/outcome: 0. Phase T: §4.5.

## Measurement method and metrics

| Metric | Canonical name | Role | Family |
|---|---|---|---|
| `alloc_peak_bytes`, `peak_rss_bytes`, `bounded_memory_growth_bytes` | T-06; M08.3 | primary; binary | memory_peak; correctness |
| `artifact_bytes` encrypted vs unencrypted of the same EAM | T-01 (M15.3) | primary for overhead | size |
| `encode_wall_s`, `decode_wall_s`, `encrypted_random_entry_latency_s` | T-03, T-04, T-08 (in-process) | primary (phase T) | time_wall |
| `kdf_open_wall_s`, `kdf_open_peak_bytes`, `oom_kill` | T-04 (process), T-06 | primary for DEC-CRY-013 (phase T for wall) | time_wall, memory_peak |
| `argon2_invocations` | MVT-06(a) | binary (= 0 over policy) | correctness |
| `unlock_wall_s_by_stanza_position`, `envelope_bytes` | T-04; T-02 `side_data_bytes` | primary; diagnostic | time_wall; size |
| `outcome`, `reason_code`, `stdout_bytes_before_refusal` | none | binary for refusal-before-output | correctness |
| `lai_equal`, `aux_equal`, `pcr_equal_after_rotation`, `ciphertext_differs`, `vector_gate_pass` | MVT-13(b); HC-14 vector gate | binary (PCR equality descriptive) | correctness |
| `segment_count`, `directory_bytes` | none | descriptive | other; size |

## Replication

Memory probes 3 runs per size; outcomes 3 repetitions; phase T §4.6 (KDF opens are seconds long: large-tier rule 10/5/20 applied).

## Normalization and statistical analysis

Status-quo classification as EXP-SCALE-001; ceilings as outcome tables; prototypes and default comparisons as EXP-SCALE-007 (R3/R4 tuning, MDE gate, validation look, verdicts, computed tiers: resource-only prototypes that keep wire bytes are T0; anything touching crypto wire is T4 and external review).

## Practical-significance thresholds

Referenced by ID: T-01, T-02, T-03, T-04, T-06, T-08; M08.3; §7.3.

## Sensitivity analysis

Cap and CPU grid for KDF; stanza position arm (first vs last); size-tier arm; full §6 where a decision reaches R8/R9.

## Expected negative results worth recording

Encrypted operations unusable beyond RAM; key removal on a 16 GiB archive requiring plaintext-proportional memory; ceiling-cost archives OOM-killing readers on 1 GiB hosts instead of refusing; unlock time at 1,024 stanzas beyond interactive tolerance; the 1,000,000-item EBCS cap refusing trees the plain bootstrap policy accepts (1,000,000 entries plus directory entries).

## Threats to validity

1. Low-end emulation by cgroup cap and CPU pinning ignores memory bandwidth and cache sizes of real low-end devices.
2. X-Wing draft-10 and Argon2id implementations are pinned dependency versions; timing reflects them only.
3. Prototypes may diverge from reviewed constructions; they are resource probes and must pass the vector gate.
4. `pty_password.sh` adds a `script` process to the measured tree (small, recorded by a no-op control).

## Platform requirements

Linux x86-64 WSL2 root; about 120 GB; no external review for the parts addressed (constructions remain `EXTERNAL_REVIEW_REQUIRED`).

## Estimated compute and effort

About 30 h after tooling; tooling judgment.

## Deviations (append-only)

None.
