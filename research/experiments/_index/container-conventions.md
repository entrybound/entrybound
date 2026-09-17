# Container domain (program section 11): shared experiment conventions

This file holds the conventions every `EXP-CON-*` protocol cites, so that each
protocol states only what is specific to it. Where this file and
`research/decision-method.md` differ, the method governs; where it and a protocol
differ, the protocol states the deviation explicitly.

| Field | Value |
|---|---|
| Domain | container (program section 11) |
| Experiments | EXP-CON-001 to EXP-CON-010 |
| Method | `research/decision-method.md`, pre-registration commit `14b977c` (or the revision in force when a record is instantiated) |
| Designer | Phase C-design container-domain session, 2026-09-17. Did not construct any ledger candidate set. |
| State of evidence | No experiment in this domain has run. The only numbers in these files are the non-decision-grade feasibility smokes in `EXP-CON-001/feasibility-smoke/` and `EXP-CON-007/feasibility-smoke/` (sizing and tooling checks only, never results, §12.3 item 6). |

## 1. L7 identity-exposure disclosure

As instructed by the Phase C-design rules, this session read
`research/corpus/coverage.md`. Its independence-group table names held-out item
identifiers and upstream sources for every family. No held-out content, listing,
statistic or path under a held-out root was opened, and the session's own corpus
queries filtered `manifest.json` to `split in {tuning, validation}` before printing.
This is an L7 event (§5.7) for the §5.8(h) record. Because every family is exposed,
§5.4 item 3 would bar this session from designing analyses for any section-11
decision; the design review and gate audit decide the consequence (for example,
re-assigning the analysis plans to an unexposed session, or accepting the residual
risk with the `identity_aware_heldout` cap that already applies to every R5 pass).
Nothing in these designs uses held-out identities.

## 2. Gates and pre-registration state

- Every protocol here is the design pre-registration. The §12.2 `record.md` is
  instantiated from it only after gate G-A holds (constraint crosswalk, ledger
  schema v2 with `hc_ids`/`od_ids`/`decision_type`, validation-look registry).
- `od_ids`, `hc_ids` and `decision_type` in the protocols are **proposals for the
  independent assigner** (§1.2, §14 item 12). They bind nothing.
- No run with non-empty `decision_ids` starts before G-A. No decision-grade verdict
  exists before G-B (thresholds test update, decision tooling §14 item 2, runner
  additions §14 item 3 for timing, calibration §4.7 for timing and memory,
  frozen held-out lock, workload mix). Deterministic census runs may execute
  earlier as **informational** runs, labelled `NOT_DECISION_GRADE`, and must be
  re-run (or re-verified by repeat-hash against the same build) after G-B.
- Cost tiers (§7.2) are computed by the §14 item 7 scripts for C1-C3 and assessed
  for C4-C7 by a session not involved in the candidates, before the first
  validation look. Tiers written in protocols are expectations only.

## 3. Builds and identities

| Build | Identity | How it is built | Used for |
|---|---|---|---|
| `ebound-prod` (REF-PROD) | production workspace at `9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155`, no research features | `git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/prod-9e44608 && git -C /root/eb-research/src/prod-9e44608 checkout 9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155 && CARGO_TARGET_DIR=/root/eb-research/target/prod-9e44608 /root/.cargo/bin/cargo +1.98.1 build --release --locked --bin ebound -p entrybound-cli` (WSL); Windows: same commit cloned under `D:\eb-research\src\prod-9e44608`, `CARGO_TARGET_DIR=D:\eb-research\target\prod-9e44608`, `%USERPROFILE%\.cargo\bin\cargo.exe +1.98.1 build --release --locked --bin ebound -p entrybound-cli` | Every status-quo archive, every production timing number (harness review use constraint 2), every production reader outcome |
| `ebound-head` | production crates at the C-exec commit (must be additive-identical to `9e44608` for production paths, proven by `internals-identity-check`) | as above at the recorded commit | Only where a protocol needs a behaviour added after `9e44608`; the protocol names it |
| harness | `research/harness` at the C-exec commit, `CARGO_TARGET_DIR=/root/eb-research/target/harness` (WSL) or `D:/eb-research/target/harness` (Windows) | `research/harness/build.sh build --release` / `build.ps1 build --release` | In-process prototypes and measurements |

Before any campaign: `python research/harness/lock_parity.py` and
`research/harness/harness-cli-parity.sh` must both print `RESULT PASS` (harness
review use constraint 1), and `research/orchestration/disk_guard.py` must pass.
Every raw record carries the build SHA-256 of each binary it invoked (the run
command prints `sha256sum` of the binaries into the run meta via the spec's
`platform.requires` list plus a pre-run `build-identity.json` written by
`research/tools/container/build_identity.sh`).

## 4. Tooling to build (shared; named precisely)

Status `NEEDS_TOOLING` in a protocol means at least one of these is missing.

| ID | Artifact | Contents |
|---|---|---|
| CT-1 | crate `research/harness/crates/ebr-container` | Library: streaming synthetic EAM model, the manifest/Index/outboard representation encoders and decoders, the exact read-log simulator, the §4.15 latency model (reusing `EXP-REMOTE-001/connection-model.json`), and the counting-allocator wrapper of CT-2. Binaries below. `unsafe_code = "forbid"`. |
| CT-1a | binary `ebr-eamgen` | `fit --source <item_path> --out <model.json>`; `model --model <model.json> --entries <N> --meta posix\|rich\|windows --seed <S> --out <dir>`; `tree --model ... --entries <N> --out <dir>` (materializes a filesystem tree with 0-64 byte files); `zip --model ... --entries <N> --out <file.zip>` (streaming, Python-free) |
| CT-1b | binary `ebr-manifest-repr` | EXP-CON-001 measurements over a packed archive (`--item-path` of a corpus item, packed through the pack cache) or a synthetic model |
| CT-1c | binary `ebr-index-repr` | EXP-CON-002 measurements |
| CT-1d | binary `ebr-outboard` | EXP-CON-003 measurements, including the construction counterexample search |
| CT-1f | binary `ebr-openers` | EXP-CON-005: in-process `bench` mode over production archives (open, list, first-entry and random-entry batches with CT-2 phases, plus a production-CLI parity arm) and the `--policy` probe used by the scale arm (open under a named caller policy, reporting refusal code and CT-2 peak) |
| CT-2 | module `ebr_common::alloc` behind harness feature `counting-alloc` | Counting global allocator with allocation-site attribution and an untrusted-size flag (§14 item 17; objective MVT-06(a)); exposes `alloc_peak_bytes` per phase |
| CT-3 | `research/tools/container/pack_cache.sh` (Windows: `pack_cache.ps1`) | Idempotent production pack: key = (`logical_tree_sha256`, `ebound-prod` binary SHA-256, profile, layout, encryption); archives under `/root/eb-research/cache/EXP-CON/packs/<key>.eb`; writes `<key>.json` with the `ebound inspect --json` output and the `ebr-inspect-bytes` row; a refusal is cached as `<key>.refused.json` with the reason code. Plain packs are deterministic, so an evicted archive is re-packed and must reproduce the recorded SHA-256 (a mismatch is a §4.13 determinism violation artifact). Eviction: archives above 64 MiB may be deleted once every consuming spec has finished its current pass; `disk_guard.py` runs before each pass. Encrypted cells are packed by harness `ebr-pack --encrypt true` (the CLI prompts on the terminal) and are never evicted mid-pass because they are not reproducible |
| CT-4 | `research/tools/container/derive_spec.py` | Mechanical derivation of validation and filtered specs (section 7) |
| CT-5 | scripts named inside individual protocols | Listed per protocol |

The prototypes are research-only code outside the production workspace (F-24).
Every prototype that re-implements a status-quo structure must first reproduce the
production bytes exactly (a per-experiment validity gate), so that alternative
candidates are compared with a status quo implemented under the same conditions.

## 5. Data locations

- Inputs: `/root/eb-research/corpus/{tuning,validation}/<item_id>` (WSL ext4, never
  `/mnt/*`), selected through `research/corpus/manifest.json`.
- Generated inputs: `/root/eb-research/generated/<EXP-ID>/<item_id>/`, described by an
  experiment-local manifest committed in the experiment directory (rows with
  `item_id`, `split`, `family`, `independence_group`, `path`, generator, parameters,
  seed). Their SHA-256 fingerprints are committed before the first run.
- Scratch: `/root/eb-research/scratch/<EXP-ID>/` (WSL), `D:\eb-research\scratch\<EXP-ID>\`
  (Windows, outside the repository). Pack cache: CT-3.
- NTFS copies for Windows arms: `D:\eb-research\corpus-ntfs\<split>\<item_id>`, made
  by `research/experiments/EXP-CON-007/stage_ntfs_copies.ps1`.
- Held-out: never. Every protocol's Phase D section is a placeholder under §5.5.

## 6. Metric naming, roles and strata

- Metric names in specs are canonical names from `decision-method.md` Appendix A.2
  wherever a canonical metric exists. A stratum or evaluation condition is appended
  with a double underscore: `<canonical>__<stratum>` (for example
  `verified_range_fetch_bytes__4k__p95`, `open_bytes_read__lookup_mean`).
  The decision tooling maps the prefix to the canonical threshold (T-ID) and treats
  the suffix as a stratum or condition (never pooled, §4.9, AGG-S).
- Non-canonical metrics are prefixed `desc_` and are descriptive only: they never
  eliminate or select (§0.2 item 4). Binary screens without an Appendix A.2 entry
  (for example `desc_capability__declared_bounds_before_payload`) are reported to the constraint
  crosswalk session; they become R1/R2 screens only if the crosswalk classifies the
  underlying SPEC clause as an HC or freeze.
- `diagnostic` metrics (for example `metadata_bytes`, `index_bytes`,
  `side_data_bytes`) are declared primary only in decisions about that component
  (§1.1 role table), with the justification in the protocol.
- At most one primary metric per OD per decision (R3). Guards are declared as
  non-inferiority metrics.

## 7. Split discipline and derived specs

- `spec.yaml` (and any `spec-*.yaml`) in an experiment directory is the **tuning**
  pass. It selects `splits: [tuning]` only.
- The validation look is **derived, not hand-edited**, by CT-4:
  `python research/tools/container/derive_spec.py --spec <spec> --split validation --items <EXP-ID>/validation-items.txt --candidates <W-and-S.txt> --out <EXP-ID>/spec-validation-look<k>.yaml`.
  The item list is pre-registered now (`validation-items.txt`, generated from
  `research/corpus/manifest.json` by the selection rule stated in the protocol); the
  candidate list is W and S from the tuning record (§4.12). The look is appended to
  `research/decisions/validation-looks.jsonl` before `ebr run`.
- Timing specs run only on candidates not eliminated by R1, R2 or the deterministic
  R4 screen; CT-4 `--candidates` filters them by that recorded rule, never by
  inspection of timing results.
- Synthetic models used on a split are fitted only on items of that split.
- Held-out (Phase D): after Commit C, CT-4 `--split heldout --items-from-lock` derives
  the held-out spec from the frozen record; run once under `EB_HELDOUT_UNLOCK` (§5.5,
  R5). The held-out MDE is computed before unlock from validation group variance and
  held-out group counts.

## 8. Protocol defaults (apply unless a protocol overrides)

- **Deterministic metrics first** (§13): 2 repetitions in different rounds and
  processes, repeat-hash on every artifact or JSON payload (`desc_payload_sha256`),
  `order: randomized_blocks`. Any hash mismatch where determinism is claimed is an
  R1 elimination artifact (§4.13).
- **Timing**: in-process harness timers or batches of at least 1,000 operations
  (T-08); rounds per §4.6 tier table with n_min(c); warmups 2 (small, medium) or 1
  (large); `vm-cold` never used for claims; affinity `st-v` = {2} in WSL
  (`guard.max_loadavg_1m` = 2.0) and `st-p` = {2} on Windows; cooldown, drift canary
  and frequency covariate per §4.2 once §14 item 3 exists; calibration
  `EXP-CAL-AA-*`, `EXP-CAL-AAP-*`, `EXP-CAL-KE-*`, `EXP-CAL-BG-*` for the
  environment and families used must pass first (§4.7). WSL timing is `VIRTUALIZED`
  (soft cap MEDIUM for Linux-native claims); a universal-scope timing claim needs
  same-direction support on the Windows host too (§4.1).
- **Memory**: `alloc_peak_bytes` from CT-2 is primary for library-level parse and
  decode memory; `peak_rss_bytes` from GNU time (never the runner's rusage fallback,
  review R1-16) and Windows job peak commit are secondary corroboration.
- **Remote**: this domain measures no remote latency. Structure experiments
  (EXP-CON-001, EXP-CON-002, EXP-CON-003) export exact per-item layouts under
  `research/normalized/<EXP-ID>/layouts/`; the remote domain composes requests, bytes and
  modelled latency from them (EXP-REMOTE-004, EXP-REMOTE-007) with the connection models
  and slow-start arms of `research/experiments/EXP-REMOTE-001/connection-model.json`.
  `http_requests__*` keys in container specs are structure-level cross-checks.
- **Statistics**: L1-L4 aggregation (§4.9) with the cited workload mix; exact
  order-statistic item intervals and Welch-Satterthwaite corpus intervals (§4.10);
  verdict classes and `NON_INFERIOR` (§4.11); confirmatory set W against S with
  `1 - 0.01/k_sup` superiority confidence, 0.99 non-inferiority, 0.90 family harm
  guard, MDE gate at 1 band unit (§4.12). Practical-significance bands and floors are
  exactly those of `research/methods/thresholds.json` `metric_thresholds` for the
  canonical metric; protocols cite T-IDs and never restate values differently.
- **Minimum support**: at least 10 independence groups across at least 5 applicable
  families per corpus-level verdict (§4.9). Synthetic models form their own
  evaluation condition ("entry-count stratum", AGG-S) and are never pooled with real
  items at L4.
- **Sensitivity** (§6): weight grid and Dirichlet on per-OD weights, SMAA report,
  threshold ×0.5/×2 with the resolvability condition, all §6.5 arms including
  leave-one/three-families-out, workload mixes WM-*, tier-stratified and
  byte-weighted arms, environment arm where both hosts are in scope, and the
  §6.7 selection-stability bootstrap (seed = spec `seeds.bootstrap`).
- **Outcome record** (§10.1): `outcome.json` for every committed spec, including
  abandoned runs; negative, null and inconclusive outcomes go to the register and the
  counterevidence of every decision informed.
- **Adversarial search artifact** (§8.2 item 5): each protocol names the directory
  `research/experiments/<EXP-ID>/adversarial-search/` and what it searches for.

## 9. Evidence qualifiers used here

`VIRTUALIZED` (WSL timing), `EMULATED` (application-level network, QEMU),
`PRE_FREEZE`, `NOT_DECISION_GRADE` (runs before G-B), `PLATFORM_BLOCKED`
(EXP-CON-009, EXP-CON-010), `EXTERNAL_REVIEW_REQUIRED` (registrations with external
authorities, §8.3; tree-construction soundness sign-off).

## 10. Division of labour with other domains (section-11 decisions)

Designed in parallel, the other domains' pre-registrations already carry the evidence
routes for several section-11 decisions. To avoid duplicate pre-registration and
duplicate compute, the container domain designs no separate experiment for them; each
is listed with its route in `container-uncovered.jsonl`. Where a container experiment
and another domain's experiment both touch a decision, the protocol's "Division of
labour" row states which part each owns.

| Topic | Owner experiments |
|---|---|
| Registry, section numbering, normative status, doc-code contradictions (DEC-CON-003, DEC-CON-029, DEC-MOD-042) | EXP-CONF-001, EXP-CONF-003, EXP-CONF-004 |
| Cross-opener accepted byte sets, fixed framing fields, unknown-element and tier behaviour, version gate (DEC-CON-011, DEC-CON-014, DEC-CON-024, DEC-CON-025) | EXP-CONF-002, EXP-CONF-003, EXP-CONF-004, EXP-CONF-010, EXP-CRYPTO-016 |
| Historical stored-bytes corpus (DEC-CON-013) | EXP-CONF-006 |
| Decode requirements and default decoder ceilings on x86-64 (DEC-CON-004, DEC-CON-005) | EXP-CONF-007, EXP-RECON-001, EXP-CODEC-003, EXP-CROSSFILE-003, EXP-CROSSFILE-004 |
| Hostile resource cases and allocator oracle (DEC-CON-001 dynamic part, DEC-CON-006 corpus arm) | EXP-CONF-004, EXP-CONF-007 |
| Dependency-defined decoding across versions (DEC-CON-007) | EXP-ECO-002, EXP-RECON-003, EXP-RECON-004, EXP-CODEC-005 |
| Identification, extensions, media type, polyglots on Linux and Windows (DEC-CON-017, DEC-CON-019, DEC-CON-021, DEC-CON-023) | EXP-ECO-012 (macOS and browsers: EXP-CON-010, BLOCKED) |
| Crypto-v1 limits and legacy encrypted forms (DEC-CON-009, DEC-CON-016) | EXP-SCALE-010, EXP-CRYPTO-009, EXP-CRYPTO-015 |
| STREAM ordering and window canonicality (DEC-CON-026), incremental producers (DEC-CON-027) | EXP-SCALE-003, EXP-SCALE-007, EXP-SCALE-008, EXP-CONF-001, EXP-CONF-002 |
| Index header damage, header/footer digests, truncation reporting (DEC-INT-018, DEC-INT-019, DEC-INT-041) | EXP-INT-001, EXP-INT-002, EXP-CONF-010 |
| Legacy import evidence at 1e6 entries (DEC-LEG-002) | EXP-SCALE-002, EXP-SCALE-009 |
| Digest algorithm and identity descriptor bindings (DEC-MOD-016, DEC-MOD-027) | EXP-CRYPTO-004, EXP-CRYPTO-012, EXP-CRYPTO-014, EXP-CRYPTO-020, EXP-CRYPTO-021, EXP-CONF-002, EXP-CONF-012 |
| Remote legs of manifest, Index and proof structures (DEC-ACC-018, DEC-ACC-019, DEC-ACC-024 remote parts) | EXP-REMOTE-004, EXP-REMOTE-007, EXP-REMOTE-008 |
| Production memory and byte census at scale (DEC-CON-015, DEC-CON-018 status quo) | EXP-SCALE-001, EXP-SCALE-003 |
