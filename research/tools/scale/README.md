# Scale-domain tooling (`research/tools/scale`)

Research-only instruments and drivers for the scale experiments `research/experiments/EXP-SCALE-*`
(program §13 memory/streaming/beyond-RAM and §14 deterministic parallelism). Nothing here is
compiled into production crates.

## Committed and smoke-tested at design time (non-decision-grade smoke, 2026-09-17)

| File | Purpose | Smoke result |
|---|---|---|
| `memcap_run.sh` | Runs one command in a fresh cgroup-v1 memory cgroup (WSL2, root) with `memory.limit_in_bytes` and `memory.memsw.limit_in_bytes` set to the cap, under GNU time; prints one `ebr.scale.memcap.v1` JSON line (GNU time max RSS of the child, cgroup peak, OOM-kill count, exact device write bytes of `--watch-mount` loop mounts). Exits 0 whenever it measured, so the command's outcome is data. | OOM kill detected (`signal` 9, `oom_kill` 1) for a 256 MiB allocation under a 64 MiB cap; a no-op wrote 0 device bytes; a 4 MiB `dd` wrote 4,255,744 device bytes (data plus ext4 metadata); `ebound pack` and a STREAM stdin/stdout round trip ran under the cap. |
| `synth_tree.py` | Deterministic, bounded-memory synthetic trees (`files`, `entries` with optional `--long-names`, `sparse`, `dedup` shapes; `half`/`incompressible`/`zeros` content from SHAKE256) with a streamed tree digest (`ebr.scale.tree.v1`) and a `digest` command. | Generated and re-read digests agreed for every shape; the sparse file had 268,435,456 apparent and 4,194,304 allocated bytes. |

The smoke outputs were not kept, and no number from them may be cited as a result.

## To be written (contracts are fixed in the named protocol)

| File | Experiment(s) | Effort |
|---|---|---|
| `scale_cell.py`, `select_stage2.py`, `classify_scaling.py` | EXP-SCALE-001, EXP-SCALE-002 | scripted |
| `legacy_counts.py` | EXP-SCALE-002 | scripted |
| `census_item.py`, `legacy_limit_stats.py`, `predict_refusals.py` | EXP-SCALE-003 | scripted |
| `fault_cell.py` | EXP-SCALE-004 (and the EXP-SCALE-008 fault phase) | scripted |
| `det_cell.py`, `shuffle_copy.py`, `bgload.py`, `det_compare.py` | EXP-SCALE-005, EXP-SCALE-014, EXP-SCALE-015 | scripted |
| `bound_check.py` | EXP-SCALE-006 | scripted |
| `mutate_during_pack.py` | EXP-SCALE-007 | scripted |
| `pty_password.sh` | EXP-SCALE-010 | scripted |
| `codec_invariance.py` | EXP-SCALE-011 | scripted |
| `gen_affinity_specs.py` | EXP-SCALE-013, EXP-SCALE-016 | scripted |
| `storage_cell.py` | EXP-SCALE-016 (blocked) | scripted |

Harness prototypes (`ebr-scale-writer`, `ebr-scale-reader`, `ebr-scale-legacy`, `ebr-scale-crypto`, `ebr-par`,
`ebr-forge`, the `ebr-access --mode alloc-probe` mode, the `ebr-codec` `codec-mt` feature) belong in
`research/harness/crates/` and are specified in EXP-SCALE-006 to EXP-SCALE-012. The counting allocator is the
shared `research/harness/crates/ebr-alloc` designed in EXP-CONF-007.

## Conventions

- Run inside WSL as root from the repository root through `/mnt/d/...` paths for scripts, with all inputs, outputs
  and loop images under `/root/eb-research` (never `/mnt/*` for measured I/O, `decision-method.md` §4.1).
- Drivers print exactly one JSON line with a `payload` object (the ebr `stdout_json` metric source) and exit
  non-zero only for infrastructure failures, which make the ebr sample invalid.
- Before a campaign: `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; drivers also refuse
  a sample when `/mnt/d` free space would fall below 75 GiB plus the cell's declared peak.
