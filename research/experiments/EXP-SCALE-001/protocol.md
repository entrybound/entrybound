# EXP-SCALE-001: Native operation memory and scratch scaling beyond RAM (status quo)

| Field | Value |
|---|---|
| Status | **READY** (design draft; becomes a §12 experiment record only after gate G-A, see Pre-registration) |
| Domain | scale (program §13, §14) |
| Platforms | Linux x86-64 (WSL2 Ubuntu 24.04, root, ext4 under `/root/eb-research`, cgroup-v1 memory controller, loop mounts); large storage |
| Requires timing | false (wall and CPU times are descriptive only) |
| Compute estimate | about 50 machine-hours (stage 1 about 38 h, stage 2 about 12 h) |
| Disk estimate | stage 1 peak about 110 GB; stage 2 (64 GiB tier) peak about 260 GB on the WSL volume, which lives on host D: |
| Agent effort | scripted (driver written mechanically from this protocol); judgment only for analysis and the claim table |
| Tooling to build | `research/tools/scale/scale_cell.py` (driver, interface fixed below); `research/tools/scale/select_stage2.py` (stage-2 operation screen). Shared instruments already committed: `research/tools/scale/memcap_run.sh`, `research/tools/scale/synth_tree.py` |

## Pre-registration

- **decision_ids:** DEC-ACC-028, DEC-ACC-005, DEC-ACC-029, DEC-ACC-020, DEC-ACC-058, DEC-ACC-062, DEC-ACC-060, DEC-ACC-048, DEC-ACC-004, DEC-ACC-059, DEC-ACC-022, DEC-ACC-052, DEC-ACC-002, DEC-INT-043, DEC-CON-015, DEC-CON-018, DEC-CMP-017, DEC-CMP-041, DEC-MOD-004, DEC-MOD-026.
- **Decision type:** EMPIRICAL for the parts addressed here (memory, scratch and completion under caps are measured quantities, §1.2 forced types). Assignment by an independent session is pending (§1.2, §14 item 12).
- **od_ids / hc_ids:** PENDING-G-A. Proposal for the independent assignment (not binding): OD-08, OD-09, OD-17 (M17.6 scale limits), OD-06/OD-07 descriptive; HC-06, HC-15 (verified staging), HC-02 (round trip of every sample).
- **Method:** `research/decision-method.md` at the pre-registration commit resolved by subject "research: pre-register decision method and archetypal objective" (`14b977c`).
- **Gate status:** G-A is not met at design time (ledger schema v2 `od_ids`/`hc_ids`/`decision_type`, constraint crosswalk, look registry `research/decisions/validation-looks.jsonl`). This file is a design draft; the PENDING-G-A fields are filled, and this protocol re-committed as the §12 record, before any run whose results are cited by a decision.
- **Author and exposure disclosure (§5.4 item 3, §5.7 L7):** Phase C-design scale agent. While reading the required inputs this session saw held-out item *identifiers* (names only) in `research/corpus/coverage.md` (independence-group table) and upstream-source names in `research/PROGRESS.md`. It opened no held-out content, listing, statistic or path under the held-out roots, and printed only `split in {tuning, validation}` manifest rows. This experiment uses synthetic generated inputs only, so no corpus family's held-out items enter it. However, the decisions it informs are universal in scope, so their applicable families include families with exposed items; under §5.4 item 3 the **analysis plans of every EXP-SCALE protocol are provisional until a session without a recorded L7 exposure re-derives or signs them off** before gate G-A. The exposure is recorded for the §5.8(h) L7 record. (The same exposure arises for every Phase C-design agent that was instructed to read `coverage.md`; this is flagged to the orchestrator.)

## Question

For every native, unencrypted Entrybound operation in the production CLI, how do peak resident memory, scratch (temporary-directory) writes and completion under a memory cap scale with total plaintext, single-entry size, entry count, sparse logical size and duplicate content, and which operations therefore scale with plaintext rather than staying bounded?

## Hypotheses

- **H1 (status-quo classification, from code reading, `[INFERRED]`):** every pack variant (INDEXED and STREAM, file or stdout sink) is PLAINTEXT-PROPORTIONAL, because `entrybound::archive::plan_directory` retains every Chunk's plaintext (`research/harness/crates/ebr-access/src/probe.rs` module doc); INDEXED `unpack`, `verify`, `list`, `inspect`, `explain`, `read`, `repack` and `sign` are ARCHIVE-PROPORTIONAL (the INDEXED reader reads and decodes the whole file, `crates/entrybound/src/archive/filesystem.rs:489-503`); STREAM `verify`/`list` from stdin are METADATA-PROPORTIONAL (O(entries + chunks)); STREAM `unpack` is BOUNDED in resident memory (256 MiB staging cap) with scratch writes proportional to plaintext above 256 MiB.
- **H0:** an operation's peak RSS growth from the smallest to the largest probe of an axis stays within the bounded-memory tolerance of objective M08.3, max(4 MiB, 0.10 × R0).
- **Falsification of H1 for an operation:** its classification under the rule in "Statistical analysis" differs from the H1 class. Any difference is reported per operation; H1 is not a selection hypothesis and no decision is selected on it.

## Decisions informed and required-evidence items addressed

| Decision | Required evidence addressed here | Routed elsewhere |
|---|---|---|
| DEC-ACC-028 | Operation-by-operation peak RSS classification on inputs larger than the cap (Task 13 inventory), native operations | import/export/publish: EXP-SCALE-002; encryption: EXP-SCALE-010; workload size survey: EXP-SCALE-003 (corpus) and ecosystem domain |
| DEC-ACC-005 | Claim-by-claim measured values (RSS corroboration) for STREAM retention, "never holds the whole archive", staging bounds | allocator oracle, hostile maximal windows/regions/manifests: EXP-SCALE-006 |
| DEC-ACC-029 | Peak RSS vs input size per profile under memory caps for the current design | prototypes and ratio loss of windowed planning: EXP-SCALE-007 |
| DEC-ACC-020, DEC-ACC-058 | Writer peak RSS vs archive size (status quo row of each prototype table) | EXP-SCALE-007 |
| DEC-ACC-062 | RSS and time of verify/list/unpack of large INDEXED archives, status quo | prototypes: EXP-SCALE-008 |
| DEC-ACC-060 | Reader RSS vs entries/chunks for STREAM (1e3 to 9.99e5 entries) | 1M-10M entries under raised policy and POSIX-metadata retention: EXP-SCALE-006 |
| DEC-ACC-048, DEC-ACC-004, DEC-ACC-059 | Repack peak memory and full-rewrite cost vs archive size per mode and layout pair; repack-to-INDEXED cost | frame reuse and scan-and-cache prototypes: EXP-SCALE-008; demand survey: not measurable here (human participants) |
| DEC-ACC-022 | Memory of stdin/stdout pipeline variants; status-quo refusal behaviour for INDEXED to stdout | full non-seekable behaviour census: EXP-SCALE-004; first-use study: human participants |
| DEC-ACC-052 | Peak RSS and spill for representative synthetic archives under default limits | adversarial archives: EXP-SCALE-006; boundary behaviour: EXP-SCALE-004 |
| DEC-ACC-002 | Bounded-memory pipe marks: Entrybound STREAM pipelines vs `tar | zstd` pipelines at equal inputs | parallel decode marks: EXP-SCALE-013; other incumbents: evaluation domain |
| DEC-INT-043 | Time (descriptive), RSS and scratch of the only existing verification level on large inputs | other levels: EXP-SCALE-008 |
| DEC-CON-015, DEC-CON-018 | Open, list and inspect peak memory at 1e3-9.99e5 entries | byte census: EXP-SCALE-003; alternative structures: container domain |
| DEC-CMP-017, DEC-CMP-041 | Dedup and similarity memory at scale (dedup axis; dense/extreme on the entries axis) | external dedup index, windowed similarity: EXP-SCALE-007 |
| DEC-MOD-004 | Status-quo producer memory for canonical order at large entry counts | per-comparator sort cost: EXP-SCALE-007 |
| DEC-MOD-026 | Pack/extract memory, scratch and hashing cost scaling with logical (hole-including) size, status quo | hole-plan candidates: model/container domain |

## Candidates

This is a status-quo characterization: the "candidates" are operations of one design, plus incumbent references. No selection is made from this experiment alone; its rows are the status-quo candidate rows of the decisions above.

| candidate (ebr name) | Exact command (inside `memcap_run.sh`) | Prerequisite built by the driver outside the measured region |
|---|---|---|
| `pack-indexed-fast` | `ebound pack IN OUT.eb --profile fast` | none |
| `pack-indexed-balanced` | `ebound pack IN OUT.eb --profile balanced` | none |
| `pack-indexed-dense` | `ebound pack IN OUT.eb --profile dense` (applicable only to cells with logical bytes ≤ 4 GiB and entries ≤ 1e5) | none |
| `pack-indexed-extreme` | `ebound pack IN OUT.eb --profile extreme` (logical ≤ 1 GiB, entries ≤ 1e4) | none |
| `pack-stream-file` | `ebound pack IN OUT.ebs --layout stream --stream-window auto --profile balanced` | none |
| `pack-stream-stdout` | `ebound pack IN - --layout stream --stream-window auto --profile balanced`, stdout to a file on the destination mount | none |
| `pack-stream-stdout-window0` | as above with the CLI default window (`--stream-window` omitted) | none |
| `pack-indexed-stdout` | `ebound pack IN - --layout indexed --profile balanced` (records status-quo behaviour; refusal is a valid outcome) | none |
| `unpack-indexed` | `ebound unpack A.eb DEST` | balanced INDEXED archive A.eb |
| `unpack-stream-file` | `ebound unpack A.ebs DEST` | balanced STREAM archive A.ebs (window auto) |
| `unpack-stream-stdin` | `ebound unpack - DEST` with stdin = A.ebs | A.ebs |
| `verify-indexed` | `ebound verify A.eb` | A.eb |
| `verify-stream-stdin` | `ebound verify -` with stdin = A.ebs | A.ebs |
| `list-indexed` | `ebound list A.eb` (stdout to /dev/null) | A.eb |
| `list-stream-stdin` | `ebound list -` with stdin = A.ebs | A.ebs |
| `inspect-indexed` | `ebound inspect A.eb --json` | A.eb |
| `explain-indexed` | `ebound explain A.eb` | A.eb |
| `read-entry-indexed` | `ebound read A.eb <path> --output DEST/entry` for the entry chosen by `seeds.command` from the generator's path list | A.eb |
| `repack-representation` | `ebound repack A.eb B.eb` | A.eb |
| `repack-replan-fast` | `ebound repack A.eb B.eb --profile fast` | A.eb |
| `repack-to-stream` | `ebound repack A.eb B.ebs --layout stream --stream-window auto` | A.eb |
| `repack-to-indexed` | `ebound repack A.ebs B.eb --layout indexed` | A.ebs |
| `sign-detached-indexed` | `ebound sign A.eb --signing-key K --detached SIG` | A.eb; K from `ebound key generate-signing K` |
| `inc-tar-zstd-create` | `bash -c 'tar -C IN -cf - . | zstd -T1 -3 -q -o OUT.tar.zst'` (REF-INC streaming specialist, objective §1.3 CC-02) | none |
| `inc-tar-zstd-extract` | `bash -c 'zstd -dc -q IN.tar.zst | tar -C DEST -xf -'` | IN.tar.zst from the create command |

- **Reference candidate** for ratios: none within an operation class; each Entrybound pipeline operation is paired with its incumbent pipeline (`pack-stream-stdout` with `inc-tar-zstd-create`, `unpack-stream-stdin` with `inc-tar-zstd-extract`) for the DEC-ACC-002 mark only.
- **Excluded and why:** encrypted operations (`--password` prompts on a TTY; covered with harness tooling in EXP-SCALE-010); `convert`/`publish` (EXP-SCALE-002); `sign` over STREAM (a capability question, DEC-CRY-096, not a scale question; see `research/experiments/_index/scale-uncovered.jsonl`); `diff` and `sidecar` (no decision in §13/§14 asks for them); remote `read`/`list` over URLs (remote domain).
- Binary: `ebound` built from the production workspace (harness review use constraint 2) at the commit recorded in the run meta: `cd /root/eb-research/src/entrybound && git checkout <sha> && CARGO_TARGET_DIR=/root/eb-research/target/scale-prod /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --locked`. The SHA is the `dev` HEAD at the stage-1 launch and is recorded in `cells.json` `binary_commit` before the run.

## Corpus selectors (generated inputs only)

Inputs are synthetic trees from `research/tools/scale/synth_tree.py` (content model and digest defined in its docstring; SHAKE256-based, library-version-independent). They are not corpus items and have no split; held-out discipline does not apply to them. The ebr corpus selector is the ad hoc manifest `research/experiments/EXP-SCALE-001/cells.jsonl`, one row per cell, each row pointing at `research/experiments/EXP-SCALE-001/cells.json` (the committed cell table); `{item_id}` names the cell.

| Cell id | Shape and parameters (`synth_tree.py materialize`) | Logical bytes | Cap(s) |
|---|---|---|---|
| `b1g` | `--shape files --files 64 --total-bytes 1073741824 --content half --seed 1301` | 1 GiB | cap24 |
| `b4g` | files 64, 4294967296, half, seed 1302 | 4 GiB | cap24, cap2 |
| `b16g` | files 64, 17179869184, half, seed 1303 | 16 GiB | cap24, cap2 |
| `b64g` (stage 2) | files 64, 68702699520 (2^36 − 2^24), half, seed 1304 | 64 GiB − 16 MiB | cap24, cap2 |
| `e1g` | files 1, 1073741824, half, seed 1311 | 1 GiB single entry | cap24 |
| `e4g` | files 1, 4294967296, half, seed 1312 | 4 GiB single entry | cap24, cap2 |
| `e16g` | files 1, 17163091968 (2^34 − 2^24), half, seed 1313 | 16 GiB − 16 MiB single entry | cap24, cap2 |
| `n1e3` | `--shape entries --entries 1000 --entry-bytes 4096 --content half --seed 1321` | 3.9 MiB | cap24 |
| `n1e4` | entries 10000, 4096 B, seed 1322 | 39 MiB | cap24 |
| `n1e5` | entries 100000, 4096 B, seed 1323 | 391 MiB | cap24, cap2 |
| `n999k` | entries 999000, 4096 B, seed 1324 | 3.81 GiB | cap24, cap2 |
| `s1g` | `--shape sparse --total-bytes 1073741824 --island-bytes 1048576 --island-period 67108864 --seed 1331` | 1 GiB logical, 16 MiB data | cap24 |
| `s16g` | sparse, 17163091968 logical, same islands, seed 1332 | 16 GiB − 16 MiB logical, 256 MiB data | cap24, cap2 |
| `d1g` | `--shape dedup --files 256 --file-bytes 4194304 --unique 16 --seed 1341` | 1 GiB logical, 64 MiB unique | cap24 |
| `d16g` | dedup, files 4096, 4194304 B, unique 256, seed 1342 | 16 GiB logical, 1 GiB unique | cap24, cap2 |
| `i4g` | files 64, 4294967296, `--content incompressible`, seed 1351 | 4 GiB | cap24, cap2 |

- **Caps:** `cap24` = 25,769,803,776 B (a safety cap that keeps the 31 GiB WSL VM alive; it is the "uncapped" arm); `cap2` = 2,147,483,648 B (beyond-RAM emulation: 2× to 32× smaller than the inputs). A cell id plus cap forms the ebr item id, for example `b16g-cap2`.
- **Size choices against policy:** the bootstrap policy (`crates/entrybound/src/archive.rs:35-46`) caps total logical bytes at 64 GiB, a single entry at 16 GiB and entries at 1,000,000. The top probes sit 16 MiB (or 1,000 entries) below those caps so that this experiment measures growth rather than policy refusal; the caps themselves are probed at and above the boundary in EXP-SCALE-004. **Declared deviation from Appendix C.1:** the 64 GiB bounded-memory probe uses 2^36 − 2^24 bytes (0.025% below nominal), to be dispositioned by the method review.
- **Minimum support (§4.9)** does not apply: no corpus-level verdict is computed; classification is per (operation, axis, cap).
- **Held-out placeholder (Phase D):** none. Generated inputs have no held-out split. If a decision later relies on this experiment's classification, its R5 obligation is met by EXP-SCALE-006's held-out re-run on post-freeze generated seeds (`decision-method.md` §5.4 sealing route: seeds committed only as SHA-256 before the freeze, revealed after unlock).

## Environment

- `env_id` `wsl-ubuntu` (research/environment), qualifier `VIRTUALIZED`; kernel 5.15.167.4, cgroup-v1 memory controller at `/sys/fs/cgroup/memory` (hybrid hierarchy; verified at design time by a non-decision-grade smoke).
- **Memory cap:** every measured command runs under `research/tools/scale/memcap_run.sh --cap <cap>`, which sets `memory.limit_in_bytes` and `memory.memsw.limit_in_bytes` to the same value (no swap escape) and reports GNU time's `max_rss_kib` for the child, the cgroup peak (`memory.max_usage_in_bytes`, includes page cache) and the OOM-kill counter.
- **Mounts per cell** (created by the driver with `mkfs.ext4 -q -F` on sparse image files under `/root/eb-research/scratch/EXP-SCALE-001/<cell>/`, mounted `-o loop,noatime`): `tmp` (exported as `TMPDIR`; production spills STREAM staging to `std::env::temp_dir()`), `dst` (all outputs and extraction destinations). Image sizes: `tmp` = logical bytes + 1 GiB, `dst` = 2 × logical bytes + 1 GiB (sparse files). Inputs and prerequisites live on the plain ext4 volume outside both mounts.
- **Cache state:** `hot` (§4.5). Before each sample the driver reads the sample's input or archive once with `cat >/dev/null` outside the measured region. No cold-cache claim is made.
- **Exclusivity:** timing is descriptive, so no quiet window is required; the orchestrator still does not schedule another memory-heavy workflow concurrently, because memory pressure from other processes on the VM can change reclaim behaviour under the cap. The driver records `/proc/loadavg` and `MemAvailable` before each sample.
- **Disk safety:** the driver refuses a sample (infrastructure exit 75, sample invalid) unless `df -B1 --output=avail /mnt/d` reports at least 75 GiB plus the cell's declared peak; after each cell the driver deletes the cell directory and runs `fstrim -v /` so the sparse distro VHDX can shrink. `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass before launch (program constraint).

## Commands

```sh
# once, before stage 1 (inside WSL, root)
cd /root/eb-research/src/entrybound && git fetch /mnt/d/Projects/entrybound/entrybound dev && git checkout --detach FETCH_HEAD
CARGO_TARGET_DIR=/root/eb-research/target/scale-prod /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --locked
cd /mnt/d/Projects/entrybound/entrybound
export PYTHONPATH=research/tools PYTHONDONTWRITEBYTECODE=1
PY=/root/eb-research/venv/bin/python
$PY -m ebr validate --spec research/experiments/EXP-SCALE-001/spec.yaml
$PY -m ebr plan --spec research/experiments/EXP-SCALE-001/spec.yaml > /root/eb-research/logs/EXP-SCALE-001.plan.tsv
$PY -m ebr run --spec research/experiments/EXP-SCALE-001/spec.yaml --scratch-root /root/eb-research/scratch
$PY -m ebr verify-raw --spec research/experiments/EXP-SCALE-001/spec.yaml
$PY -m ebr normalize --spec research/experiments/EXP-SCALE-001/spec.yaml
$PY -m ebr report --spec research/experiments/EXP-SCALE-001/spec.yaml
# stage 2: screen, commit the generated stage-2 spec, then run it
$PY research/tools/scale/select_stage2.py --raw research/raw/EXP-SCALE-001 \
    --out research/experiments/EXP-SCALE-001/spec-stage2.yaml
```

**Driver contract** (`research/tools/scale/scale_cell.py`, to be written; the spec's `command` calls it):

```
scale_cell.py --experiment EXP-SCALE-001 --cells research/experiments/EXP-SCALE-001/cells.json
              --cell {item_id} --op {candidate} --rep {rep} --seed {seed} --scratch {scratch}
```

1. Look up the cell (shape, parameters, cap, declared disk peak) and the operation (command template, prerequisites, applicability predicate) in `cells.json`. If the predicate is false, print `{"payload": {"applicable": 0}}` and exit 0.
2. Materialize the input with `synth_tree.py materialize` into `/root/eb-research/generated/EXP-SCALE-001/<cell>/input` if absent; verify `<input>.synth.json` `tree_sha256` equals `cells.json` `expected_tree_sha256` (filled from the first materialization and committed before stage 1; a mismatch is an infrastructure failure, exit 75).
3. Build prerequisites (archives, signing key, incumbent `.tar.zst`) once per cell with uncapped commands, verify each with `ebound verify`, and cache them next to the input. Prerequisite build time is not measured.
4. Create and mount `tmp` and `dst`; pre-read the input or archive; run the operation under `memcap_run.sh --label <cell>-<op>-<rep> --cap <cap> --watch-mount tmp --watch-mount dst --stdin/--stdout as the template requires`, with `TMPDIR=<tmp>`, `LANG=C.UTF-8`, `TZ=UTC`.
5. After the command: for `unpack-*`, `inc-tar-zstd-extract` and `read-entry-indexed`, compute `synth_tree.py digest` of the destination (or the SHA-256 of the entry) and compare with the generator record (content round trip; metadata is outside this experiment); for pack and repack outputs, run `ebound inspect <out> --json` uncapped and record identities. Record whether any file remains under `tmp` (leftover spill).
6. Print exactly one JSON line `{"schema": "ebr.scale.cell.v1", "payload": {...}}` with the fields in "Metrics"; unmount and remove `tmp`/`dst`; exit 0. Exit non-zero only for infrastructure failures (input digest mismatch, disk guard, mount failure), which make the ebr sample invalid.

## Seeds

- `seeds.order` 1301001 (ebr randomized blocks), `seeds.bootstrap` 1301002 (informational ebr intervals only), `seeds.command` 1301003 (entry choice for `read-entry-indexed`).
- Generator seeds per cell are in the cell table and `cells.json`.

## Warmup

`warmups: 0`. Peak memory and write counts are not subject to JIT or cache warmup; page-cache warmth is controlled by the hot pre-read (§4.5). Kendall τ between repetition index and `peak_rss_bytes` is still computed per (operation, cell) as the §4.5 adequacy check; if |τ| > 0.3 in more than 10% of cells, one warmup round is added for stage 2 and logged as protocol strengthening (§0.2 item 3).

## Measurement method and metrics

| Metric (payload key) | Canonical name (Appendix A.2) | Role here | Family | Unit | Source |
|---|---|---|---|---|---|
| `peak_rss_bytes` | `peak_rss_bytes` (T-06) | primary for classification | memory_peak | B | GNU time `max_rss_kib` × 1024 of the measured child tree (§4.14; review R1-16) |
| `cgroup_max_usage_bytes` | none (diagnostic, not canonical) | secondary | memory_peak | B | cgroup v1 peak; includes page cache |
| `oom_kill` | none | outcome | correctness | count | cgroup `memory.oom_control` |
| `outcome` | none | outcome (string) | correctness | enum OK, REFUSED_TYPED, FAILED_OTHER, OOM_KILLED, KILLED_SIGNAL | exit status (CLI map `crates/entrybound-cli/src/lib.rs` main_entry), signal, `oom_kill` |
| `reason_code` | none | outcome (string) | correctness | `EB_*` | first `EB_[A-Z0-9_]+` token in stderr |
| `scratch_write_bytes` | `scratch_write_bytes` (binary, §4.14) | primary for streaming/zero-scratch claims | correctness | B | device writes of the `tmp` loop mount (tolerance 0) |
| `dest_write_bytes` | none | secondary | io | B | device writes of the `dst` loop mount |
| `scratch_leftover_bytes` | none | secondary | scratch_peak | B | bytes remaining under `tmp` after exit |
| `roundtrip_ok` | `roundtrip_mismatches` (HC-02) as 1 − mismatches | gating | correctness | 0/1 | content tree digest equality |
| `output_sha256`, `lai`, `pcr`, `aux`, `pci` | none (repeat-hash §4.13) | gating | correctness | string | SHA-256 of output; `ebound inspect --json` identities |
| `input_logical_bytes`, `input_files`, `archive_bytes` | none | covariates | size | B, count | generator record, `stat` |
| `op_wall_s`, `op_cpu_s` | none (descriptive, **not** `encode_wall_s`/`decode_wall_s`) | descriptive | time_wall, time_cpu | s | memcap elapsed; GNU time user + sys |

- Correctness gating (objective §3.0): a sample with `roundtrip_ok = 0`, or a pack/repack output that fails `ebound verify`, is reported as an HC-02/HC-15 violation artifact and excluded from memory analysis.
- Streaming capability (M09.1, `streaming_capability`) is derived per operation: 1 if every applicable pipe variant completes with `outcome = OK` and `scratch_write_bytes = 0`; otherwise 0, with the cell that breaks it.

## Replication count and adaptive rule

- **3 repetitions** per (operation, cell), the objective M08.3 count; deterministic outputs are additionally repeat-hashed across the 3 (§4.13).
- **Stage 2 rule (pre-registered, direction-blind with respect to decisions):** an operation enters the 64 GiB tier iff, on the `b16g-cap24` and `b16g-cap2` cells, it is not classified PLAINTEXT-PROPORTIONAL or OOM_KILLED. A plaintext-proportional operation has already falsified any bounded-memory claim, so a 64 GiB probe adds no information about it. `select_stage2.py` applies this rule mechanically to verified raw rows, writes `spec-stage2.yaml` (same schema, candidate list reduced, cells `b64g-cap24` and `b64g-cap2`), and that file is committed before stage 2 runs.
- No other adaptive step. Memory metrics are not used for banded comparisons in this experiment, so the §4.6 timing/memory round minimums (10 rounds) do not apply; any later banded comparison of these operations against a prototype uses EXP-SCALE-006/007/008 with §4.6 rounds.

## Normalization

Per (operation, axis, cap): `peak_rss_bytes` versus the axis variable (logical bytes, single-entry bytes, entries, sparse logical bytes, dedup logical bytes). Growth `G = max_rep peak(largest probe) − min_rep peak(smallest probe)`. Per-byte slope `k = (median peak(largest) − median peak(smallest)) / (x_largest − x_smallest)` (bytes of memory per input byte or per entry). R0 for the M08.3 tolerance is the median `peak_rss_bytes` of the same operation on the smallest probe of the axis (an RSS stand-in; the allocator R0_max of MVT-06(a) is EXP-SCALE-006's).

## Statistical analysis

- **Classification (exact rule, per operation and axis, on cap24 cells):**
  - `BOUNDED` if G ≤ max(4 MiB, 0.10 × R0) (objective M08.3 tolerance applied to RSS);
  - `PLAINTEXT-PROPORTIONAL` (bytes axes) if k ≥ 0.5 B/B; `ARCHIVE-PROPORTIONAL` if k ≥ 0.5 × (archive bytes / logical bytes) on the `i4g`/`b*` pair and the operation reads an archive;
  - `METADATA-PROPORTIONAL` (entries axis) if the entries-axis slope exceeds the tolerance while the bytes-axis class is BOUNDED;
  - `SUBLINEAR-GROWTH` otherwise (reported with G and k).
- **Beyond-RAM completion (cap2 cells):** per operation, the largest probe with `outcome = OK` in all 3 repetitions, and the first probe with `OOM_KILLED` or `REFUSED_TYPED` (with its reason code). A typed refusal before exhausting memory is recorded separately from an OOM kill (HC-06 prefers the former).
- **Exact intervals:** for G the order-statistic interval over the 3 repetitions is degenerate at n = 3 (§4.10 n_min 8 at 0.99); the classification therefore uses the conservative max-minus-min form above, and a `BOUNDED` label from RSS is explicitly **not** the binary M08.3 claim, which requires the allocator oracle and is decided only by EXP-SCALE-006.
- **Scale limits (M17.6, `scale_limits`, descriptive):** per operation, the largest completed probe per axis under each cap.
- **Incumbent comparison (DEC-ACC-002, descriptive):** ratio of `peak_rss_bytes` of `pack-stream-stdout` to `inc-tar-zstd-create` and of `unpack-stream-stdin` to `inc-tar-zstd-extract`, per bytes-axis cell; reported with both absolute values, no verdict.
- ebr's `normalize` output is informational (§3.6); the classification table is produced by `research/tools/scale/classify_scaling.py` (to be written with the driver; reads verified raw only) into `research/normalized/EXP-SCALE-001/decision/classification.csv`.

## Practical-significance thresholds

Referenced, not restated: objective M08.3 bounded-memory tolerance and probe sizes (Appendix C.1); T-06 for any later graded memory comparison; T-07 and §4.14 for scratch (binary claims use `scratch_write_bytes` with tolerance 0). No new threshold is introduced.

## Sensitivity analysis

- **Content model:** `i4g` (incompressible) versus `b4g` (half): classification must not change; a change is reported as content-dependent.
- **File-count shape:** `e*` (single entry) versus `b*` (64 entries) at equal logical bytes.
- **Cap:** cap2 versus cap24 peak RSS for operations that complete under both (reclaim pressure can lower RSS without changing the algorithm; a lower cap2 RSS is not evidence of boundedness).
- **Page-cache charge:** classification recomputed with `cgroup_max_usage_bytes` in place of `peak_rss_bytes`; differences are reported, the RSS classification governs.
- §6 weight, mix and selection sensitivities are not applicable (no selection is made here).

## Expected negative results worth recording

- Pack in any layout, including STREAM to stdout, cannot complete a 4 GiB input under a 2 GiB cap (plaintext-proportional pack), which contradicts any reading of SPEC §4.6/§13.2 as "streaming creation in bounded memory".
- INDEXED verify/list/inspect of a large archive needs memory proportional to the archive.
- STREAM unpack writes plaintext-proportional scratch, so "streaming extraction" is not zero-scratch.
- Tar-plus-zstd pipelines stay bounded where Entrybound STREAM creation does not (a workload where the incumbent is better, §10.1 item 2).
- Any operation that is OOM-killed instead of refusing with a typed code under a cap.

## Threats to validity

1. **cgroup-v1 page-cache accounting:** heavy buffered writes under a small cap can trigger memcg OOM from unreclaimable dirty pages rather than anonymous memory. Control: `inc-tar-zstd-*` run in the same cells; if an incumbent pipeline is OOM-killed in a cell, every OOM outcome of that cell is labeled `cap_artifact_possible` and excluded from beyond-RAM completion claims.
2. **VIRTUALIZED environment:** WSL2 memory reclaim and the VHDX-backed ext4 differ from native Linux; results carry the `VIRTUALIZED` qualifier and a Linux-native claim is soft-capped (§4.1).
3. **RSS is not the allocator oracle:** allocator arenas can retain freed memory (inflating RSS) or RSS can miss transient allocations sampled between page faults (it cannot: `ru_maxrss` is a high-water mark). Binary claims are EXP-SCALE-006's.
4. **Loop mounts add a filesystem layer;** device write counts include ext4 metadata and journal blocks, so non-zero small values need the no-op baseline (smoke: 0 B for `true`, 4,255,744 B for a 4 MiB write).
5. **Synthetic inputs** lack real metadata diversity (xattrs, ACLs, hard links); memory contributions of rich metadata are measured on corpus items in EXP-SCALE-003 and EXP-SCALE-006 (POSIX-metadata retention).
6. **Harness vs production build:** only the production `ebound` binary is measured here (use constraint 2).
7. **Host D: capacity** may block stage 2; that outcome is recorded as a deviation, not silently skipped.

## Platform requirements

Linux x86-64 (WSL2), root for cgroup-v1 memory control and loop mounts, about 260 GB free on the WSL volume with host D: above the 75 GB guard, no network, no external review. Windows memory scaling (job peak commit) is not covered here; the Windows host arm is a documented gap for DEC-ACC-028 (memory is never compared across OSes, §4.14) and would reuse this cell table with `ebr` Windows job accounting and no cap (non-admin Windows cannot set job memory limits on the runner's behalf without a runner addition, §14 item 3).

## Estimated compute and effort

- Compute: about 50 h (roughly 10 TiB of operation input over stage 1 at about 100 MiB/s single-threaded, plus generation and prerequisites; stage 2 about 12 h).
- Disk: stage 1 about 110 GB peak (largest cap24 cell inputs plus two archives plus one extraction plus spill); stage 2 about 260 GB peak.
- Agent effort: scripted (driver, classifier); judgment for interpretation and ledger text only.

## Deviations (append-only)

- 2026-09-17 (design): the 64 GiB probe is 2^36 − 2^24 bytes and the 16 GiB single-entry probe is 2^34 − 2^24 bytes, to stay below the bootstrap policy caps (see Corpus selectors). Results visible: no.
