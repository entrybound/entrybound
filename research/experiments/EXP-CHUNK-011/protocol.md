# EXP-CHUNK-011: chunker_id producer audit and pack/replan identity

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CHUNK-011 |
| Status | READY. It uses the production `ebound` CLI (built at the pre-registration commit), the harness `ebr-pack` (encrypted producer), GNU tar 1.35 and the checked-in wrapper `audit_item.py`. |
| Type of test | Binary HC test (HC-10 identity pattern and HC-16 frozen grammar), run on tuning now and on validation at the owning decisions' HC screen (objective §2.0 rule 3: HC tests must pass on tuning and validation before freeze, and on held-out after it) |
| decision_ids | DEC-MOD-008 (replan chunker_id divergence), DEC-CMP-005 (audit of every chunker_id producer against the grammar), DEC-MOD-039 (PCR divergence for identical content across producers; the profile/rekey census is in EXP-CHUNK-012) |
| Proposed decision types | DEC-MOD-008: FORMAL + EMPIRICAL test (candidate choice among fix/alias/reject is FORMAL with an identity-rule argument; the defect's existence is this test). DEC-CMP-005: FORMAL + conformance. |
| Proposed od_ids | DEC-MOD-008: OD-20 (identity-rule conformance), OD-23. DEC-CMP-005: OD-04, OD-21, OD-23. |
| Proposed hc_ids | HC-10, HC-13, HC-16 |
| Gates | G-A unmet (draft) |
| Author / L7 | As EXP-CHUNK-001 section 1 |
| **Results-visible feasibility smoke (NOT_DECISION_GRADE, disclosed per §0.2 and CB-3)** | Before writing this protocol, this session ran `audit_item.py` on a 3-file generated tree at `balanced` with the program's baseline `ebound` build (`/root/eb-research/target/baseline/release/ebound`) and the review-round-1 `ebr-pack` build. Results: `pack`, `retain` and `convert-tar` recorded `gear-norm-v1/min-131072/target-524288/max-2097152` with equal PCR. `replan-same` and `replan-cross` recorded `entrybound/gear-cdc-v1/min-131072/target-524288/max-2097152`, with LAI equal and PCR different from the direct pack. `encrypted-pack` recorded `gear-norm-secret-table-v1/min-131072/target-524288/max-2097152`. **H1 below was written after seeing this smoke.** The experiment's decision-grade role is to replicate it on corpus items and every profile with the pre-registration-commit build. Smoke observation: `ebound inspect --json` carries no chunker or planner identifier (only the text output does); recorded for DEC-CMP-005 inspection-surface work. |

## 2. Question

Does every production producer of chunked archives record the frozen chunker_id grammar for the algorithm and parameters it used? The producers audited are:

- `pack`;
- current-v6 replan through `repack --profile`;
- boundary-retaining `repack`;
- tar import through `convert`;
- encrypted `pack`.

And does identical input, chunked with identical boundaries, yield identical chunker_id and PCR across producers?

## 3. Hypotheses

- **H1 (smoke-informed, section 1).** `replan-same` and `replan-cross` record a non-frozen identifier for `gear-norm-v1` boundaries, and PCR differs from a direct pack of the same profile while LAI is equal. `pack`, `retain` and `convert-tar` record `gear-norm-v1/min-{n}/target-{n}/max-{n}` (`docs/chunking-v1.md`). `encrypted-pack` records `gear-norm-secret-table-v1/min-{n}/target-{n}/max-{n}` (`docs/crypto-suite-v1.md` §Boundary modes).
  - **Falsified** (for replan) if replan records the frozen grammar with equal PCR on every applicable item and profile.
- **H2 (identity pattern).**
  - `retain` keeps LAI, AUX and PCR equal (per `ebound repack --help`: "recorded Chunk boundaries … retained and LAI/AUX/PCR must remain equal").
  - `convert-tar` of a deterministic pax tar keeps chunker_id and PCR equal to a direct pack, whenever the conversion accepts the item. LAI may differ where tar metadata differs; the SPEC §8.1 table decides.
  - **Violation** is any `retain` cell with a PCR/LAI/AUX difference, or any accepted `convert-tar` cell with a chunker_id difference.
- **H0.** All producers record frozen grammars with equal PCR for equal boundaries.
- **Oracle and triage.** The expected cell values are the SPEC §8.1 identity table and the frozen grammars. Any disagreement is triaged under objective §2.0 rule 8 by a session not involved in either reader. The triage log is committed with one citation per case.

## 4. Decisions informed

| Decision | Contribution |
|---|---|
| DEC-MOD-008 | Ledger evidence "Test PCR and chunker_id equality between pack and replan for identical input" on the whole tuning subset, all profiles. The inventory of published archives carrying the replan identifier is outside this experiment. It is recorded in `known_limitations` for the decision record (the program has no access to user archives; the inventory is limited to repository fixtures by `grep -rn "gear-cdc-v1"` over committed vectors). |
| DEC-CMP-005 | Audit of chunker_id producers against the grammar. Parsing, fuzzing, vectors and clean-room reimplementation are EXP-CHUNK-012. |
| DEC-MOD-039 | Producer-caused PCR divergence for identical content (one of the divergence sources the decision asks about) |

## 5. Candidates

This experiment tests **producers**, not design candidates.

- **Decision candidates it discriminates, DEC-MOD-008:**
  - `fix-replan-emit-gear-norm`;
  - `accept-both-as-aliases`;
  - `reject-legacy-replan-id`;
  - `status-quo-divergent`: already excluded in the ledger (R1 against the frozen grammar, `docs/chunking-v1.md` L45-49); this test supplies its mechanical counterexample artifact.

  The test establishes whether the defect exists in production and on which producers. Choosing among the remaining candidates is FORMAL (HC-10 canonical identity, HC-16 immutability of published identifiers: an alias rule changes PCR computation for existing archives, which is costed at T4 as an identity participation change).
- **Producers:** 6 operations × 4 profiles = 24 spec candidates (`{op}-{profile}`). The reference producer is `ebound pack --profile P --layout indexed`.
- **Not audited here** (listed for the DEC-CMP-005 producer inventory):
  - STREAM layout pack (the same writer path is expected; added as an amendment if a code census finds a distinct chunker_id assignment);
  - ZIP and 7z import;
  - `publish`;
  - `sidecar`;
  - encrypted replan (unsupported by the CLI).

  The code census in section 8 step 3 lists every assignment site.

## 6. Corpus

- **Tuning:** all tuning items ≤ 64 MiB, 53 items, 19 families, 0.98 GiB (ids in `spec.yaml`). Items that production refuses (for example non-UTF-8 names in `f19-tuning-zoo`, `EB_EAM_INVALID_PATH_COMPONENT`) are recorded with `applicable = 0` and their refusal reason; they are not failures.
- **Validation:** `spec-validation.yaml` with the same selection rule on validation, run as the HC screen and recorded in the look registry as a non-confirmatory HC screen.
- **Held-out:** HC tests re-run after unlock (Phase D placeholder, §5.5).
- **Generated vectors:** none.

## 7. Environment and platform

- WSL x86-64, ext4. Production CLI build at `/root/eb-research/target/exp-chunk-prod` (use constraint 2). Harness `ebr-pack` at `/root/eb-research/target/exp-chunk` (encrypted producer only).
- Windows-host replication: `spec-windows.yaml` runs on the Windows host with the Windows production build. PCR and chunker_id are expected host-independent (F-0002). LAI and AUX differ by host by design (F-0001, F-0002), so cross-host comparisons are PCR/chunker_id only.
- No privileges, network, or timing.

## 8. Commands

```sh
# 1. Builds at the pre-registration commit (clean worktree)
CARGO_TARGET_DIR=/root/eb-research/target/exp-chunk-prod /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli
CARGO_TARGET_DIR=/root/eb-research/target/exp-chunk bash research/harness/build.sh build --release -p ebr-pack
# 2. Run
export PYTHONPATH=research/tools; PY=/root/eb-research/venv/bin/python
$PY -m ebr validate --spec research/experiments/EXP-CHUNK-011/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-CHUNK-011/spec.yaml --gzip
$PY -m ebr verify-raw --spec research/experiments/EXP-CHUNK-011/spec.yaml --refingerprint
$PY -m ebr normalize --spec research/experiments/EXP-CHUNK-011/spec.yaml
# 3. Code census of chunker_id producers (committed output)
grep -rn --include=*.rs -E 'chunker_id|gear-cdc-v1|gear-norm' crates/ > research/experiments/EXP-CHUNK-011/producer-census.txt
# 4. Cell table against the SPEC section 8.1 oracle (to be written, stdlib)
$PY research/experiments/EXP-CHUNK-011/cell_table.py --normalized research/normalized/EXP-CHUNK-011 \
    --out research/normalized/EXP-CHUNK-011/decision/cells.csv
```

## 9. Seeds

- **Spec seeds:** `{order: 2026091811, bootstrap: 2026091812, command: 2026091813}`.
- Encrypted pack uses fresh randomness (keys). Its chunker_id is deterministic; its PCR is key-dependent and not compared.
- Warmups 0.

## 10. Metrics

| Metric | Canonical | Role |
|---|---|---|
| `op_chunker_grammar_ok` (the produced id matches the frozen grammar for its boundary mode, with a power-of-two target and ordered sizes) | HC-16 binary (reported under `migration_identity_conformance_fraction` for replan/convert, M20.1) | binary |
| `chunker_id_equal`, `pcr_equal`, `lai_equal`, `aux_equal` (vs the direct pack) | HC-10 binary (`migration_identity_conformance_fraction` cells) | binary per SPEC §8.1 cell |
| `chunker_params_equal` | no | secondary (distinguishes wrong-identifier from wrong-boundaries) |
| `applicable`, `op_exit_code`, `op_reason` | no | refusal accounting |
| `result_sha256` | no | repeat-hash |

## 11. Replication

2 repetitions and repeat-hash (§4.6). Encrypted cells repeat-hash on the chunker_id-derived fields only (the payload excludes key-dependent identities). A plaintext cell whose two repetitions differ is an HC-13 nondeterminism artifact (§4.13).

## 12. Analysis

- **Per (operation, profile):** the fraction of applicable items whose cell matches the oracle (AGG-C per family; headline minimum over families). A single mismatching item is a violation with its artifact: both archives' inspect outputs plus the item id and command, committed under `research/raw/EXP-CHUNK-011/violations/`.
- **No statistics** (binary HC test, objective §2.0 rule 1).
- **Output:** a cell table (operation × profile × {grammar, chunker_id, PCR, LAI, AUX}) with counts, cited by DEC-MOD-008 and DEC-CMP-005.

## 13. Thresholds

None: binary HC test (`decision-method.md` §3.3). The pass criterion is the objective's MVT-10(a) cell equality.

## 14. Sensitivity

- Windows-host replication (PCR and chunker_id only).
- Validation-split replication.
- Tar format `gnu` versus `pax` for `convert-tar` (on 10 items).

## 15. Expected negative results worth recording

- Replan identifier divergence confirmed on all profiles (a production defect under objective §2.0 rule 5; ledger work item, not an HC relaxation).
- `convert-tar` refuses a share of items (symlinks, special files), limiting the import audit's coverage.
- The encrypted chunker_id grammar deviates from docs on some profile.

## 16. Threats to validity

- `inspect` text parsing depends on the `chunker: ` line format. The parser fails closed (None), which yields `op_chunker_grammar_ok = 0` and is visible in `op_chunker_id`.
- `encrypted-pack` uses the harness build, whose encrypted outputs are not byte-comparable with the CLI. Only the recorded identifier is audited, and it comes from production library code.
- Deterministic tar construction may still differ from other tar producers. The import path is tested for identifier recording only.
- Items > 64 MiB are excluded (runtime bound). Chunker_id recording is size-independent in the code paths `[INFERRED; census step 3 verifies]`.

## 17. Estimates

| Item | Estimate |
|---|---|
| Compute | 24 candidates × 53 items (0.98 GiB) × 2 reps; each sample packs the item 1-3 times (extreme slowest): about **6-10 CPU-hours** |
| Disk | < 5 GB transient |
| Agent effort | scripted run; judgment for `cell_table.py` (small) and triage of any disagreement by an uninvolved session |

## 18. Tooling to build

- `research/experiments/EXP-CHUNK-011/cell_table.py` (stdlib). Everything else exists: `audit_item.py` is checked in with this protocol and was feasibility-smoked.

## 19. Deviations

(append-only)
