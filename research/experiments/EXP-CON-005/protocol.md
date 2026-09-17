# EXP-CON-005: Production startup latency and parser memory at scale; bootstrap-policy scale limits

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: CT-1f `ebr-openers --mode bench` and `--policy`, CT-1a `ebr-eamgen tree`, CT-2, CT-3 (`pack_cache.sh`, `pack_cache.ps1`), `research/tools/container/scale_probe.sh`; timing needs §14 item 3 runner additions and passing `EXP-CAL-*` for `wsl-ubuntu` and `windows-host` |
| Domain / program section | container / §11 (with §13) |
| decision_ids | DEC-CON-018, DEC-CON-015, DEC-CON-028, DEC-CON-006 (bootstrap-policy scale limits), input to DEC-CON-002 |
| Decision type (proposal) | EMPIRICAL |
| OD / HC (proposal) | OD-07 (M07.3, M07.4), OD-08 (M08.1), OD-10 (M10.1, M10.3, M10.5), OD-17 (M17.3, M17.6); HC-06 |
| Division of labour | This experiment is the **production latency anchor** for the container structures of EXP-CON-001/002 (which compare prototypes among themselves). Production memory and scratch scaling to 9.99e5 entries: `EXP-SCALE-001`; production byte census: `EXP-SCALE-003`; encrypted archives at scale and crypto-v1 limits (DEC-CON-009): `EXP-SCALE-010`, `EXP-CRYPTO-009`; hostile-input refusal cost and policy candidates: `EXP-CONF-007`; remote open cost: `EXP-REMOTE-004` |
| Specs | `spec.yaml` (WSL timing, every tuning item), `spec-windows.yaml` (Windows host timing, NTFS copies of small and medium tuning items), `spec-scale.yaml` (materialized synthetic trees 1e3-4e6 entries, deterministic outcomes and memory); `scale-items.json`; `validation-items.txt` |
| Method, author, L7, gates | `../_index/container-conventions.md` §1-§2 |

## 1. Question

How do production open, list, first-entry and random-entry latency and parser memory scale with entry count, layout (INDEXED, STREAM, INDEXED without Index, encrypted INDEXED) on the WSL and Windows hosts today; how closely do the in-process harness timings track the production CLI; and at which entry counts do the bootstrap default resource policies (`bootstrap_resource_policy`: 1,000,000 entries, 1 GiB metadata bytes, 4,000,000 Chunks) refuse legitimate archives or production stop scaling?

## 2. Hypotheses

- **H1 (latency and memory scale).** For INDEXED and STREAM, `open_latency_s` and `alloc_peak_bytes__open` grow at least linearly with entry count (whole-Manifest residency and full re-encode canonical check at open; `docs/format-v0.md` "The reader re-encodes authoritative records"); the fitted log-log slope over the synthetic trees is ≥ 0.9 (descriptive, reported with its 0.99 interval).
- **H2 (Index).** `indexed-balanced-index-absent` has `open_latency_s` `DISTINGUISHABLE_WORSE` than `indexed-balanced` on items with ≥ 1e4 Chunks only when random access is used; full in-memory open rebuilds the locator map either way, so `list_latency_s` is `EQUIVALENT`.
- **H3 (bootstrap policy, DEC-CON-006).** On the F04-model synthetic trees, production packs succeed up to 4e6 entries under the 24 GiB memory cap only below some N_pack and the bootstrap policy refuses to open archives at an entry count below 1e6 because declared `max_metadata_bytes` exceeds 1 GiB (the feasibility smoke sized about 1.3 KiB of Manifest per tiny entry, non-decision-grade), with reason code naming the metadata budget.
- **H4 (parity).** Harness in-process `list` timing and production `ebound list` whole-process timing agree in direction and within 1 band unit of T-04 after subtracting the process floor (use constraint 2 of the harness review); otherwise every harness-only timing number is labelled `codegen-unverified`.
- **H0 / falsification.** H1 falsified if the slope interval lies entirely below 0.9; H3 if no benign synthetic archive below 1e6 entries is refused under the bootstrap policy; H4 if parity fails (recorded, not hidden).

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CON-018 | Production open/list/lookup latency and parser memory against entry count: the cost the alternatives of EXP-CON-001 must beat | Alternative representations: EXP-CON-001; remote: EXP-REMOTE-004; memory beyond RAM: EXP-SCALE-001 |
| DEC-CON-015 | Production open and random-entry latency with and without Index | Alternative Index structures: EXP-CON-002 |
| DEC-CON-028 | Production open latency share attributable to Manifest size (latency vs manifest bytes regression) | Trigger evaluation: EXP-CON-001 |
| DEC-CON-006 | Entry counts at which bootstrap policies refuse benign archives; which budget field refuses; process peak memory at the limits | Candidate policies and refusal cost on hostile cases: EXP-CONF-007; ecosystem population percentiles: EXP-CON-006 |
| DEC-CON-002 | Status-quo latency anchor for the revision synthesis | EXP-CON-008 |

## 4. Candidates

Status quo only (census of the production behaviour; this experiment selects nothing by itself):

| candidate_id | Definition |
|---|---|
| `indexed-balanced` | `ebound-prod pack --profile balanced --layout indexed` (REF-PROD) |
| `stream-balanced` | `--layout stream` (stream window default `Ceiling(0)`) |
| `indexed-balanced-index-absent` | REF-PROD archive rewritten by `ebound-prod repack --index absent` |
| `indexed-balanced-encrypted-password` | harness `ebr-pack --encrypt true` (password; CLI prompts on the terminal); `encrypted_random_entry_latency_s` excludes the Argon2id key derivation, which `ebr-openers` times separately and reports (so KDF cost does not mask metadata cost) |
| scale arm `*--policy-bootstrap` / `*--policy-raised` | open under `bootstrap_resource_policy()` and under a raised caller policy (every field ×64, window and working set unchanged) |

## 5. Corpus selectors

- `spec.yaml`: every tuning item (112). `spec-windows.yaml`: tuning small and medium items (95) as NTFS copies staged by `../EXP-CON-007/stage_ntfs_copies.ps1` (large tier excluded for staging time and disk; the exclusion is a stated scope limit of the Windows arm).
- `spec-scale.yaml`: `scale-items.json` tuning rows: F04 model fitted on `f04-tuning-real-netbsd-pkgsrc` at 1e3, 1e4, 1e5, 5e5, 7e5, 1e6, 2e6, 4e6 entries and F19 model fitted on `f19-tuning-rsnapshot-a` at 1e4, 1e5, 1e6, materialized on ext4 by `ebr-eamgen tree` (files 0-64 bytes).
- Validation (CT-4): `validation-items.txt` (every validation item plus the validation scale trees fitted on `f04-validation-real-gentoo-repo-20260915` and `f19-validation-rsnapshot-b`; Windows arm: validation small and medium items).
- Held-out: none (section 15).

## 6. Environment and platform requirements

- `wsl-ubuntu`: `st-v` = {2}, `guard.max_loadavg_1m` 2.0, quiet window, `VIRTUALIZED`. Scale arm: every `ebound` process runs in a transient cgroup with `memory.max` = 24 GiB (`systemd-run --scope -p MemoryMax=24G` or a cgroup v2 directory created by `scale_probe.sh`; if cgroup v2 is unavailable in the WSL kernel the arm falls back to `prlimit --as` and labels peaks `address-space-limited`); an OOM kill is an outcome, not a failure.
- `windows-host`: `st-p` = {2}, AC power, runner additions of §14 item 3 required for decision-grade timing; `peak_commit_bytes` from the job object is the Windows memory corroboration.
- Platform matrix: Linux x86-64 and Windows x86-64 required; native ARM64 in EXP-CON-009 (BLOCKED); storage about 40 GB for scale trees (4e6 small files) plus archives; no privileges beyond root inside WSL.

## 7. Commands

```sh
python research/harness/lock_parity.py && bash research/harness/harness-cli-parity.sh
C:/Python313/python.exe research/orchestration/disk_guard.py
# scale trees (tuning)
python research/tools/container/materialize_generated.py --manifest research/experiments/EXP-CON-005/scale-items.json --split tuning --eamgen /root/eb-research/target/harness/release/ebr-eamgen
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CON-005/spec-scale.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CON-005/spec.yaml --timing
# Windows host (pwsh): stage copies, then run the Windows spec with the Windows ebr runner
pwsh -File research/experiments/EXP-CON-007/stage_ntfs_copies.ps1 -Split tuning
C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-CON-005/spec-windows.yaml --timing
# verify, normalize, analyze
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-CON-005/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-CON-005/spec.yaml
python research/tools/decide/analyze.py --experiment EXP-CON-005 --descriptive-scaling   # §14 item 2 tooling
```

Tooling to build: `ebr-openers --mode bench` (opens a production archive from bytes with the production API, runs `--batch` operations per round inside one process with CT-2 phase peaks, and, with `--prod-cli`, times `ebound list` as a whole process for the parity ratio), `ebr-openers --policy` (open under a named policy and report refusal code and field), `scale_probe.sh` (pack with a cgroup memory cap, record outcome and peak RSS by GNU time, open under both policies), `pack_cache.ps1`, `research/tools/container/materialize_generated.py`.

## 8. Seeds, warmups, repetitions

Seeds 15001-15003 (WSL), 15011-15013 (scale), 15031-15033 (Windows). Timing: warmups 2 (small/medium) or 1 (large), rounds per §4.6 with n_min(c), adaptive precision rule; 1,000 operations per round for list, first-entry and random-entry batches (T-08); open is timed per round as one operation inside the process after warmups, reported with the in-process floor. Scale arm: 2 repetitions, deterministic outcomes and repeat-hash of archives.

## 9. Measurement method and metrics

| Metric | OD | Role | Family | T-ID |
|---|---|---|---|---|
| `open_latency_s` | OD-10 (M10.3) | **primary** descriptive anchor | time_wall | T-08 |
| `list_latency_s` | OD-07 (M07.3) | primary descriptive anchor | time_wall | T-08 |
| `first_entry_latency_s`, `random_entry_latency_s`, `encrypted_random_entry_latency_s` | OD-07, OD-10 | anchors | time_wall | T-08 |
| `alloc_peak_bytes__open`, `alloc_peak_bytes__list` | OD-08 | anchors | memory_peak | T-06 |
| `peak_rss_bytes__ebound_list`, `peak_rss_bytes__pack`, `peak_rss_bytes__open` | OD-08 | corroboration (GNU time; never the runner rusage fallback, R1-16) | memory_peak | T-06 |
| `desc_refused_under_policy`, `desc_budget_field_that_refused`, `desc_scale_limit_reason_code` | OD-17 (M17.3, M17.6) | descriptive `scale_limits` | other | none |
| `desc_timing_parity_ratio_harness_vs_prod` | — | validity check H4 | other | none |

The scaling analysis is descriptive (the status quo is the only candidate). Comparisons between layouts (H2) use the §4.10-§4.12 machinery with the layouts as candidates and `indexed-balanced` as reference; they inform decisions but cannot select a new structure.

## 10. Normalization and statistical analysis

- Per item: exact order-statistic interval of the median per metric (§4.10). Scaling: least-squares fit of ln(metric) on ln(entry count) over the synthetic trees, 0.99 interval, reported per layout (descriptive; objective M17.5-style, not an HC test).
- Layout comparisons (H2): paired log ratios per round (`item_repetition` pairing), L1-L4 aggregation, verdict classes (§4.11), environment arm: a claim covering both hosts needs same-direction support in both (§4.1).
- DEC-CON-006: smallest N at which each policy refuses, per model and layout, with the refusing field; AGG-W.
- Timing validity: invalid-sample balance rule (§4.4); `VIRTUALIZED` soft cap for WSL-only claims.

## 11. Practical-significance thresholds

T-06 and T-08 entries of `research/methods/thresholds.json` (in-process floor 0.1 ms for T-08; T-04 process floor for the parity arm). Not restated.

## 12. Sensitivity analysis

(a) Cache-state arm: `vm-cold` on WSL for 10 seeded items, labelled "VM cache dropped", no cold-cache claim (§4.5); (b) affinity arm on Windows `st-e` = {18} for 10 seeded items (low-end core class, reported separately, never paired with `st-p`); (c) profile arm: `dense` packs of 20 seeded items (more Chunks per entry); (d) metadata-model arm in the scale series (F04 vs F19 models); (e) the §6.5 family arms for the H2 layout comparisons.

## 13. Adversarial search and expected negative results

- `adversarial-search/`: the tree shapes that maximize open cost per entry within benign bounds (deepest paths, longest components, maximal sibling counts per directory), generated by `ebr-eamgen tree --adversarial-benign` at 1e5 entries, to find the worst benign case for each policy limit.
- Negative results worth recording: superlinear open cost (for example from canonical re-encode or path-collision checks); a Windows/WSL direction disagreement; harness/production parity failure; bootstrap policy refusing real corpus items.

## 14. Threats to validity

WSL virtualization and Windows Defender scanning (conventions §8; labelled); synthetic trees with tiny files underweight content-dependent costs (real items dominate the anchor); NTFS copies carry different metadata than the WSL originals (F-0001, F-0002: expected and reported); the 24 GiB cgroup cap bounds what "scale limit" means on this host; the encrypted arm uses the harness packer; open timing inside one process after warmups excludes process start-up (reported separately by the CLI parity arm). L7 exposure (conventions §1).

## 15. Phase D held-out placeholder

As EXP-CON-001 section 15, for the layout comparisons that enter a decision.

## 16. Cost and effort

Compute: WSL timing pass about 30 h (4 layouts × 112 items × 10-30 rounds), Windows pass about 25 h, scale arm about 20 h (packing 4e6-entry trees is the long pole; tree materialization about 6 h), validation about 45 h: about 120 machine hours, all in quiet windows except the scale arm. Disk: scale trees about 25 GB, scale archives about 15 GB, NTFS copies about 20 GB on D:. Agent effort: scripted; judgment only for the analysis record.
