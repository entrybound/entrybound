# Legacy-domain experiment conventions (EXP-LEGACY-*)

This file holds the rules every `EXP-LEGACY-*` protocol shares, so the protocols can stay specific. A protocol that
cites "C§n" means section n of this file. Where this file and `research/decision-method.md` disagree, the method
governs and this file is corrected.

| Field | Value |
|---|---|
| Domain / program sections | legacy / §25 (legacy import differential testing), §26 (adapter priority), §27 (export fidelity) |
| Author / independence / L7 | Phase C-design legacy-domain designer session. It did not construct the ledger candidate sets. **L7 disclosure:** as instructed, this session read `research/corpus/coverage.md`, whose independence-group table names held-out item identifiers and upstream sources for most families, including F11, F14, F16, F19 and F20 that these experiments use. No held-out content, listing, statistic or path under a held-out root was opened, and no held-out identifier is repeated in any `EXP-LEGACY-*` file. Under `decision-method.md` §5.4 item 3 a session with this exposure may not design *analyses* for decisions whose applicable families include the exposed items: the analysis plans in these protocols therefore need adoption (or replacement) by a non-exposed session before gate G-A, and the gate audit (§8.6) records the disposition. |
| Method | `research/decision-method.md` at pre-registration commit `14b977c` (or the revision in force when each `record.md` is instantiated) |

## C1. Status vocabulary used in `research/experiments/_index/legacy.jsonl`

- `READY`: every instrument exists at design time (ebound CLI build, installed runtimes, `ebr`, corpus items); only the
  experiment-local driver whose full behaviour the protocol specifies remains to be typed in.
- `NEEDS_TOOLING`: a named generator, driver, harness binary, research-only prototype (default-off, F-24) or pinned
  runtime installation must be built first. The protocol names it (`tooling_to_build`).
- `BLOCKED`: evidence needs a platform or access this host cannot provide (macOS/APFS, native ARM64, admin rights,
  human participants, external review, licence acceptance). The exact missing item is named.

Every status is independent of the method gates below: no experiment with non-empty `decision_ids` runs before G-A.

## C2. Gates and decision-grade conditions

- **G-A** (constraint crosswalk, ledger schema v2 `hc_ids`/`od_ids`/`decision_type`, validation-look registry): before
  `record.md` (§12.2) is instantiated from `protocol.md` and before any run. The OD/HC/decision-type values in the
  protocols are **proposals** for the independent assignment session.
- **G-B** (thresholds test updated, decision-analysis tooling §14 item 2, frozen held-out lock, workload mix,
  and for timing/memory: runner additions §14 item 3 and calibration `EXP-CAL-*-wsl-ubuntu` §4.7): before the first
  decision-relevant result. Deterministic correctness/coverage observations may be collected once G-A holds, but are
  labelled `NOT_DECISION_GRADE` until G-B holds for the analysis.
- **G-C / G-D**: held-out and `DECIDED` gates, Phase D/E.
- Harness use constraints (`research/harness/harness-review-round1.md` "Use constraints"): `lock_parity.py` and
  `harness-cli-parity.sh` must print `RESULT PASS` before any campaign; decision-grade timing against incumbents uses
  `ebound` built from the production workspace without research features; external-tool RSS only from
  `resource.max_rss_bytes` with GNU time present (R1-16).
- `research/orchestration/disk_guard.py` must pass before every campaign (C: ≥ 25 GB, D: ≥ 75 GB).

## C3. Builds of Entrybound

| Build id | Definition |
|---|---|
| `ebound-baseline` | `entrybound-cli` at research baseline `9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155`, `cargo +1.98.1 build --release --bin ebound -p entrybound-cli` from a fresh clone, target `/root/eb-research/target/legacy-baseline` (WSL) and `D:/eb-research/target/legacy-baseline` (Windows) |
| `ebound-head` | the same at the `dev` commit named in `record.md` (REF-PROD for the run) |
| `ebound-research` | `ebound-head` plus the default-off `research-internals` feature extended with module `entrybound::research::legacy` (prototypes named per protocol). Byte- and outcome-identity with the feature off is proven by extending `research/harness/internals-identity-check.sh` with the legacy import/export cells of C6 before any use (objective §2.0 rule 9; F-24). Never used for decision-grade timing. |
| `ebr-legacy` | new harness crate `research/harness/crates/ebr-legacy` (binaries `ebr-legacy-import`, `ebr-legacy-export`, `ebr-legacy-readers`, `ebr-legacy-wrap`, `ebr-legacy-decmem`), depending on `entrybound` by path with `research-internals`; counting global allocator with allocation-site attribution (§14 item 17); JSONL rows `ebr.harness.raw.v1`; held-out guard on every input path |

## C4. Runtime registry and pinning

Machine-readable registry: `research/experiments/_legacy/runtime-registry.json`. Acquisition and pinning:
`research/tools/legacy/acquire_runtimes.py --registry research/experiments/_legacy/runtime-registry.json --lock research/experiments/_legacy/runtime-lock.json`
installs or builds each runtime under `/root/eb-research/tools/legacy-runtimes/<runtime_id>/` (WSL) or
`D:/eb-research/tools/legacy-runtimes/<runtime_id>/` (Windows), records the SHA-256 of every executable, jar,
package archive and source tarball, the exact `--version`/`java -version`/`dotnet --info` output and the build
command, and writes the lock file. Every driver refuses to run when the runtime's current hashes differ from the lock.
Downloads of public tools are approved (PROGRESS standing decision 2026-09-12); any download that requires accepting
licence terms (for example Microsoft media, RAR trial) is **not** performed by agents and is routed to the owner.

Windows-host runtimes are invoked from WSL through WSL interop (`/mnt/c/...exe`) for correctness observations only;
their extraction targets live on NTFS under `D:/eb-research/legacy-sandbox/`. Windows timing is never taken this way.

## C5. Case sets, splits and the held-out placeholder

- **Generators** live in `research/tools/legacy/cases/` and are pure Python (no Entrybound code, no runtime under
  test). Each case record carries `case_id`, `case_family`, `primitive` (the byte-level construction), `citation`
  (normative locator: PKWARE APPNOTE 6.3.10 section, POSIX.1-2024 `pax` rationale/extended header keywords, GNU tar
  manual node, 7-Zip `7zFormat.txt` at the pinned 7-Zip source release, RFC 1952/8878, XZ file format 1.2.1),
  `labels` (ground-truth construct flags used as positive controls, C9) and `sha256`.
- **Cases from unpublished inputs.** Research III case identifiers (EB-Z001..EB-Z025, EB-T001..EB-T013) are
  regenerated from their primitive descriptions, cited by document section and the SHA-256 in PROGRESS; no bytes or
  text are copied.
- **Splits.** Structural primitive classes are exhaustive and identical across splits; what varies by split is the
  seeded instantiation (payload bytes, name bytes, sizes, offsets, orderings, writer options). Seeds: tuning
  `S_T`, validation `S_V` (fixed in each protocol). **Held-out placeholder:** a sealed tranche per
  `decision-method.md` §5.4, generated after Commit A from an owner-held seed committed now only as its SHA-256 by the
  owner (not by this session), plus post-freeze real-archive samples (EXP-LEGACY-010 §5); run exactly once after
  Commit C with `EB_HELDOUT_UNLOCK` (§5.5); no held-out item is named in any legacy protocol.
- **Materialization.** `research/tools/legacy/materialize_cases.py --experiment <ID> --split tuning|validation`
  writes cases to `/root/eb-research/cases/<ID>/<split>/<item_id>/` and appends rows
  (`item_id`, `split`, `family`, `independence_group`, `path`, `sha256_tree`) to
  `research/experiments/<ID>/cases-manifest.jsonl`. Named **tuning/validation corpus items** from
  `research/corpus/manifest.json` are appended with their `materialized_relpath`, `logical_tree_sha256` and group.
  The spec's `corpus.manifest` points at that file; it never contains a held-out row.
- **Families and groups.** Generated adversarial case sets are family F20; archive-in-archive sets F14; real package
  items keep their manifest family. `independence_group` = `legacy-gen-<generator>-<case_family>` for generated sets.

## C6. Observation schema and raw artifacts

Every driver emits one JSON object per (case, runtime or mode, operation) in schema `ebr.legacy.observation.v1`:
`case_id`, `case_sha256`, `runtime_id`, `runtime_lock_sha256`, `platform`, `operation` (`list`, `read`, `extract`,
`import`, `export`, `verify`), `exit_status`, `error_class`, `listing[]` (`name_b64`, `kind`, `size`, `mode`,
`mtime_ns`, `link_target_b64`), `selected[]` (`name_b64`, `sha256`, `length`, `error_class`), `extraction`
(`tree_fingerprint`, `created[]`, `refused[]`, `escape_attempts[]`, `stray_files[]`), `entrybound`
(`outcome_class`, `reason_code`, `lai`, `aux`, `pcr`, `conflicts[]`, `provenance_bytes`, `receipt_sha256`),
`stderr_sha256` (full stderr kept in the artifact). Observations are written gzip-compressed to
`/root/eb-research/raw-artifacts/<EXP-ID>/<run_id>/<candidate>/<item_id>/rep<k>.jsonl.gz`; the driver prints one
summary line (`payload.*`) whose `observations_sha256` the ebr raw row records, and the checkpoint commits the
artifact under `research/raw/<EXP-ID>/observations/` when it is ≤ 50 MiB, otherwise only its SHA-256 and the
data-root path (re-fetchable per §12.3 regeneration).

## C7. Divergence, ambiguity and agreement definitions (DEC-ECO-047)

- **Outcome divergence:** one runtime accepts (lists and reads every entry) and another refuses or errors.
- **Listing divergence:** the ordered sequence of (`name_b64`, `kind`) differs.
- **Content divergence:** for a common name, `selected.sha256` or `length` differs, or one side errors.
- **Extraction divergence:** `extraction.tree_fingerprint` differs, or one side records an escape attempt.
- **Strict divergence** = any of the above; **broad divergence** = content or extraction divergence only.
- **Reference readers for unanimity** (objective M19.2 names): ZIP {`python-zipfile-3.13.5-win`,
  `java-zipfile-21.0.12.1-win`, `libarchive-3.8.8-win`, `7zip-23.01-linux`, `infozip-unzip-6.00-linux`}; tar
  {`gnu-tar-1.35-linux`, `libarchive-3.8.8-win`, `python-tarfile-3.13.5-win`, `7zip-23.01-linux`}; 7z
  {`7zip-23.01-linux`, `libarchive-3.8.8-win`, `py7zr-pinned-linux`}; transports {the format's reference CLI,
  the Python stdlib module, `libarchive-3.8.8-win`}. An input is **unambiguous** when every reference reader of its
  format accepts it with no strict divergence; otherwise **ambiguous**.
- **Profile agreement:** for compatibility profile P modelled on runtime R, a case agrees when P's projected
  (listing, per-entry content digest) equals R's, or P refuses with a reason code listed as a documented safety
  override in `docs/zip-compatibility-profiles-v1.md`.
- **Triage** of Entrybound-versus-runtime disagreements follows objective §2.0 rule 8 (cited normative sentence,
  adjudicating session involved in neither reader, committed triage log).

## C8. Metrics

- Canonical names and roles come from `decision-method.md` Appendix A.2; bands from §3.2 by threshold ID; they are
  referenced, never restated with different values.
- T-20 fractions (`import_acceptance_fraction`, `import_agreement_fraction`, `export_acceptance_fraction`,
  `benign_false_refusal_fraction`, `reason_code_specificity_fraction`, `restore_fidelity_fraction`,
  `capture_fidelity_fraction`, `error_actionability_fraction`) are computed over the pre-registered case list of the
  evaluation condition (legacy format × platform × runtime or profile), aggregated per objective AGG-S/AGG-C.
- Binary metrics (`ambiguity_refusal_fraction`, `adversarial_true_refusal_fraction`, `roundtrip_mismatches`,
  `migration_identity_conformance_fraction`, `migration_determinism_fraction`, `receipt_completeness_fraction`,
  `bounded_memory_growth_bytes`, `scratch_write_bytes`, `streaming_capability`,
  `unresolved_fuzz_findings_at_coverage_target`): one committed counterexample is an R1/R2 elimination (§3.3).
- **Non-canonical descriptive statistics** (construct prevalence with exact Clopper-Pearson 95% intervals, producer
  concentration, disagreement counts, distinguishing-case counts, detector precision/recall) have no Appendix A.2
  band. Under §0.2 item 4 they are **secondary for every decision**: they may motivate candidates and bound claims,
  never eliminate or select.
- `import_majority_agreement_fraction`, `export_outcome_distribution`, `scale_limits`, `declared_gap_count`,
  `hostile_cpu_scaling_exponent` are `descriptive`.

## C9. Positive controls for detectors (DEC-ECO-047, M19.2)

Every census predicate and divergence detector used in a prevalence or divergence claim is run first over its
synthetic controls (C5 `labels`): at least one positive and one negative control per predicate. A predicate whose
control recall or precision is below 1 is reported as a negative result and its prevalence numbers are not reported.

## C10. Statistics conventions

- Deterministic observations: 2 repetitions in different processes and rounds plus the repeat-hash check (§4.6,
  §4.13). A runtime whose repeated observation differs is reclassified as nondeterministic for that case, both
  observations are committed, and the case is reported as a finding (not averaged).
- T-20 metrics: exact counts; item-level values are exact; aggregation per C8. Family-level (§4.9) aggregation
  applies only to real-archive tiers, after the minimum-support check (≥ 10 groups across ≥ 5 applicable families for
  corpus-level verdicts; ≥ 2 groups for a family guard); unmet support is a recorded limitation that narrows the
  decision scope.
- Timing and memory: §4.6 table (small 10/10/30, medium 10/10/20, large 10/5/20 rounds), n_min for the confidence in
  use, precision-based adaptive rule, warmups 2/2/1 (§4.5), cache state `hot`, WSL affinity `st-v` = {2} unless a
  protocol names `mt-v-n`, `guard.max_loadavg_1m` = 1.0 + threads, `guard.max_wait_s` = 300,
  `guard.max_other_cpu_percent` = 3.0 (never looser). Intervals §4.10, verdicts §4.11, confidences and MDE gate §4.12.
  WSL timing carries `VIRTUALIZED` (soft cap MEDIUM for Linux-native performance claims).
- Every verdict is produced by the decision-analysis tooling (§14 item 2); `ebr normalize` outputs are informational
  (§3.6).

## C11. Candidates and cost

- Candidate ids are the ledger `candidate_set[].candidate_id` values. The R0 item 6 critic slot is left open for the
  completeness critic pass before G-A; a candidate added then enters at R1.
- Proposed L0 exclusions (candidates whose own definition contradicts an HC) need a written counterexample and
  sign-off by a session not involved in the candidate set (objective §2.0 rule 3). Until that sign-off exists, a
  proposed-excluded candidate that is cheap to run **is run** in the arm that measures it.
- Cost tiers are computed by the §14 item 7 scripts (C1-C3) and assessed by an uninvolved session (C4-C7) before the
  first validation look. Tiers stated in protocols are expectations `[INFERRED]` only.

## C12. Safety of adversarial extraction

- WSL: every runtime that extracts runs inside `bwrap --unshare-all --bind <sandbox> /work --ro-bind <runtime> ...`
  (package `bubblewrap`, pinned in the lock) with the sandbox at
  `/root/eb-research/scratch/legacy-sandbox/<run_id>/<case>/d1/d2/d3/d4/d5/d6/d7/d8/`; the eight-level nesting keeps
  relative traversal inside the per-case sandbox root, which is fingerprinted before and after. Network is unshared.
- Windows: extraction under `D:/eb-research/legacy-sandbox/<run_id>/<case>/d1/.../d8/`; the per-case root and
  `D:/eb-research/legacy-sandbox/` are fingerprinted before and after; absolute-path and drive-letter cases are run
  only for runtimes whose listing-only observation shows they would not write outside the sandbox root, otherwise the
  extraction cell is recorded `not_run_unsafe`.
- No extraction ever targets the repository, `/mnt/*` outside the sandbox, or a held-out root.

## C13. Sensitivity conventions

For coverage-fraction decisions: leave-one-case-family-out and leave-one-generator-group-out (fraction preserving
≥ 0.90, §6.6 bars), runtime-version arm (adjacent versions of the same runtime), platform arm (Windows versus WSL,
same-direction support §4.1), and a threshold arm scaling T-20 `a` by ×0.5 and ×2 (§6.4, with the resolvability
condition not applicable to exact counts). Banded continuous metrics use the full §6 set.
