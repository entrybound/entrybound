# Conformance domain (program sections 28, 29, 30): shared experiment conventions

This file holds the rules every `EXP-CONF-*` protocol shares. A protocol that cites "CC§n" means section n of this file.
Where this file and `research/decision-method.md` differ, the method governs and this file is corrected. Where this file
and a protocol differ, the protocol states the deviation explicitly.

| Field | Value |
|---|---|
| Domain | conformance: §28 independent minimal reader and ambiguity log; §29 versioned native conformance corpus; §30 persistent and differential fuzzing |
| Experiments | EXP-CONF-001 to EXP-CONF-014 |
| Method | `research/decision-method.md` at pre-registration commit `14b977c79f157c58278a70c0f281f195430ca4f0` (or the revision in force when each `record.md` is instantiated) |
| Objective | `research/archetypal-objective.md` (HC-03, HC-04, HC-06, HC-12, HC-15, HC-16, HC-17 MVTs are the oracles used here) |
| Inputs read at design (SHA-256, working tree at repository HEAD `3c084ae`) | `research/decision-method.md` `7b00d9efe825f233e7431dc95fae6993f2e3020f0ed71e2eaf6273eb667c53ef`; `research/archetypal-objective.md` `e32450cd35700acfd3c2c9fab7253f642ddb24dda312326f118b81fccf0699ed`; `research/methods/thresholds.json` `9cd4b5be3486ae671c54987df2c5ee4f8650e77f464145fe48fe829fe93ddc90`; `research/corpus/manifest.json` `a9ce38bf9051e09f7b5504e7c63a13fb132818da5c5228f7bffccf4fc37d69fd`; `research/decision-ledger.jsonl` `7ceb7c8c4d934c3cb5747b1dc6965657b09551cbbd3ceadf7e2f1644c3d77d90` |
| Designer | Phase C-design conformance-domain session, 2026-09-17. It did not construct any ledger candidate set and wrote no production code. Of production source it saw only directory listings, `grep` counts of `EB_*` reason-code strings, and the `OutcomeClass` enum declaration in `crates/entrybound/src/diagnostics.rs`; it read repository documents under `docs/`, `tools/zip-compat/README.md`, `tools/crypto-vector-helper/README.md`, and SPEC §19-§20. It is therefore not eligible as a clean-room reader builder (CC11), which it is not. |
| State of evidence | No experiment in this domain has run. These files contain no measured result. |

## CC1. L7 identity-exposure disclosure

As instructed by the Phase C-design rules, this session read `research/PROGRESS.md` (whose change log names upstream
sources of held-out items in several families, including F20) and the head of `research/corpus/coverage.md`, whose
independence-group table names held-out item identifiers (families F03, F05, F06, F08, F09, F11, F15, F16, F20 among
the rows printed). No held-out content, listing, statistic or path under a held-out root was opened; every query of
`research/corpus/manifest.json` by this session filtered `split != heldout` before printing. No held-out identifier is
repeated in any `EXP-CONF-*` file. This is an L7 event (§5.7) for the §5.8(h) record.

Consequence under §5.4 item 3: this session may not design *candidates or analyses* for decisions whose applicable
families include the exposed items. Therefore (a) every candidate set used here is the ledger's `candidate_set`
verbatim, with no candidate added by this session (the R0 item 6 critic slot stays open for the completeness critic),
and (b) every analysis plan in these protocols must be adopted or replaced by a non-exposed session before gate G-A;
the gate audit (§8.6) records the disposition. The conformance corpus (EXP-CONF-004) and the independent reader
(EXP-CONF-001) are generated or built from normative text and do not depend on any corpus family, so they are the
least exposed parts of this design.

## CC2. Status vocabulary for `research/experiments/_index/conformance.jsonl`

- `READY`: every instrument exists; only an experiment-local driver whose behaviour the protocol fully specifies is
  still to be typed.
- `NEEDS_TOOLING`: a named generator, harness binary, independent reader, fuzz target, counting allocator,
  research-only prototype (default-off, F-24) or pinned runtime must be built first (`tooling_to_build`).
- `BLOCKED`: the evidence needs a platform, infrastructure or access this program cannot use (Docker Desktop,
  macOS/APFS, native ARM64, hosted fuzzing infrastructure, external human implementers or reviewers, more RAM than
  the host has). The exact missing item is named in `blocked_reason`.

The status is independent of the method gates in CC3.

## CC3. Gates and decision-grade conditions

- **G-A** (constraint crosswalk, ledger schema v2 `hc_ids`/`od_ids`/`decision_type`, validation-look registry
  `research/decisions/validation-looks.jsonl`): before a §12.2 `record.md` is instantiated from a `protocol.md`, and
  before any run with non-empty `decision_ids`. The OD, HC and decision-type values named in these protocols are
  **proposals** for the independent assignment session; they bind nothing.
- **G-B** (thresholds test updated, decision-analysis tooling §14 item 2, frozen held-out lock, workload mix; for timing
  and memory also runner additions §14 item 3 and calibration `EXP-CAL-*-<env>` §4.7; counting allocator §14 item 17
  for HC-06 records): before the first decision-relevant result. Deterministic correctness observations (outcome
  tuples, disagreement counts, vector reproduction, fuzz findings) may be collected once G-A holds, but are labelled
  `NOT_DECISION_GRADE` until G-B holds for their analysis, and are re-verified by repeat-hash against the same build
  afterwards.
- **G-C, G-D**: Phase D and E gates (§0.4).
- Harness use constraints (`research/harness/harness-review-round1.md`): `python research/harness/lock_parity.py` and
  `research/harness/harness-cli-parity.sh` print `RESULT PASS` before any campaign; decision-grade timing uses `ebound`
  built from the production workspace without research features; external-tool RSS only from
  `resource.max_rss_bytes` with GNU time present (R1-16).
- `research/orchestration/disk_guard.py` passes before every campaign. Fuzzing campaigns (EXP-CONF-008/009/010)
  saturate the CPU and must never overlap a timing window (§4.2 exclusivity).

## CC4. Builds

| Build id | Definition | Used for |
|---|---|---|
| `ebound-baseline` | `entrybound-cli` at research baseline `9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155`, built from `git archive 9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155` extracted to `/root/eb-research/src/conf-baseline` with `/root/.cargo/bin/cargo +1.98.1 build --release --locked --bin ebound -p entrybound-cli`, `CARGO_TARGET_DIR=/root/eb-research/target/conf-baseline` (WSL); Windows equivalent under `D:/eb-research/src/conf-baseline`, `D:/eb-research/target/conf-baseline` | Status-quo outcomes, frozen-ID differential reference |
| `ebound-head` | the same at the `dev` commit recorded in `record.md` (REF-PROD for the run), source `/root/eb-research/src/conf-head`, `CARGO_TARGET_DIR=/root/eb-research/target/conf-head` (WSL) and `D:/eb-research/target/conf-head` (Windows) | Census of the current product |
| `ebound-research` | `ebound-head` with the default-off `research-internals` feature extended only by the hooks a protocol names (allocation tags, KDF invocation counter, key/nonce log, byte-verification log). Before any use, `research/harness/internals-identity-check.sh` is extended with the conformance-corpus valid cases and passes (objective MVT-16(c), §2.0 rule 9; F-24). | HC oracles that need instrumentation; never decision-grade timing |
| `ebr-conformance` | new harness crate `research/harness/crates/ebr-conformance`, depending on `entrybound` by path (public API; `research-internals` only where a protocol says so). Binaries: `ebcc-gen`, `ebcc-census`, `ebcc-readtuple`, `ebcc-mutate`, `ebcc-diff`, `ebcc-resource`, `ebcc-verify`. Held-out guard on every input path (`ebr_common::heldout`). Public-API builds use `CARGO_TARGET_DIR=/root/eb-research/target/harness`; builds that enable research features use `/root/eb-research/target/harness-research`. | Corpus generation, census, differential drivers |
| `ebound-fuzz` | source tree from `git archive <commit>` extracted to `/root/eb-research/src/conf-fuzz-<commit12>` (the production tree is never modified), fuzz crate `research/harness/fuzz` copied in as `fuzz/`, built with `/root/.cargo/bin/cargo +nightly-2026-08-01 fuzz build --release --sanitizer address` (cargo-fuzz 0.13.2); digest-bypass and injected-defect variants are patches applied only to that copy (EXP-CONF-008) | EXP-CONF-008, -009, -010 |
| `ebir` | independent Go reader built by EXP-CONF-001 under `research/independent-reader/go`, Go 1.22.2 (`/usr/bin/go`), modules pinned by `go.sum`, `GOFLAGS=-mod=readonly`, `CGO_ENABLED=0` | EXP-CONF-001, -002, -005, -010 |

Every raw record names the SHA-256 of each binary it invoked (`build-identity.json` written next to the run, listed
in the spec's `platform.requires`).

## CC5. Shared artifacts and schemas

- **Conformance corpus `ebcc-v1`** (defined in EXP-CONF-004 §5): case bytes under `/root/eb-research/conformance/ebcc-v1/<family>/`
  (outside git); committed manifest `research/conformance/corpus/ebcc-v1/manifest.json` (every case with SHA-256,
  generator identity, `ns_ids`, expectation) and ebr manifest `research/conformance/corpus/ebcc-v1/ebr-manifest.jsonl`
  with one row per case-family bundle: `{"item_id": "ebcc-v1-<family>", "split": "conformance", "family": "EBCC-<FAMILY>", "path": "/root/eb-research/conformance/ebcc-v1/<family>"}`.
  The split name `conformance` is not a corpus split in the §5.1 sense; the conformance corpus is the separate
  population on which objective §2.0 rule 3 requires HC MVTs to pass.
- **Outcome tuple `ebr.readtuple.v1`**, one JSON line per (input, reader path): `input_id`, `input_sha256`,
  `reader_id`, `reader_build_sha256`, `path_id`, `outcome_class` (one of the six SPEC §20.5 classes),
  `reason_code` (string, or `UNDOCUMENTED` for the independent reader when the written specification gives none),
  `unverified` (bool), `detail` (object, canonical key order), `entry_set_sha256`, `content_digests_sha256`,
  `metadata_digests_sha256`, `lai`, `pcr`, `aux`, `pci` (hex or null), `wall_ns` (informational only).
  The comparison key is every field except `reader_id`, `reader_build_sha256`, `path_id`, `detail` and `wall_ns`;
  `detail` is compared in a second, diagnostic pass.
- **Ambiguity log `ebr.ambiguity.v1`** (EXP-CONF-001 §7), **triage log `ebr.triage.v1`** (CC6), **fuzz finding
  `ebr.fuzzfinding.v1`** (EXP-CONF-009 §9), **regression ledger** `research/conformance/regressions.jsonl`.
- Every summary printed by a driver is one JSON line `{"schema": "ebr.conformance.summary.v1", "payload": {...}}`
  so ebr `stdout_json` metrics use `key: payload.<field>`. Per-input detail goes to
  `/root/eb-research/raw-artifacts/<EXP-ID>/<run_id>/<candidate>/<item_id>/rep<k>/` and the summary carries its
  SHA-256 (`payload.artifact_sha256`), which the spec records for the repeat-hash check.

## CC6. Triage of reader disagreements (objective §2.0 rule 8)

- A **triage session** is involved in neither reader under comparison and did not assign the case's expectation.
- Every disagreement cluster (same pair of differing comparison-key fields, same case family or mutation class, same
  pair of codes) receives exactly one attribution: `OTHER_READER_BUG` (production mandated by a cited normative
  sentence: document, section, line), `PRODUCTION_DEFECT` (the independent reader is mandated by a cited sentence),
  `SPEC_AMBIGUOUS` (two readings both consistent with the text), `SPEC_MISSING` (no text decides), `BOTH_WRONG`.
- A cluster without a citation is not `OTHER_READER_BUG`; under the rule it counts as a specification violation of
  the HC. Counts per attribution are reported for every run.
- A second triage session re-attributes a seeded 30% sample of clusters (seed per protocol); Cohen's kappa and the
  confusion table are reported as descriptive reliability evidence.
- Triage records: `research/experiments/<EXP-ID>/triage/triage-log.jsonl`, schema `ebr.triage.v1`
  (`cluster_id`, `representative_input_sha256`, `fields`, `attribution`, `citation`, `session_id`, `notes`).

## CC7. Inputs and exclusions before the design freeze

- Only `tuning` and `validation` items of `research/corpus/manifest.json` are named. Seeds for fuzzing and mutation
  come from `tuning` items and from generated conformance cases only; `validation` items are used only for registered
  confirmation or regression looks (§5.2), never for search, seeding or harness tuning.
- **Upstream-lineage exclusion (§5.4 item 2).** Before Phase D no experiment here uses test fixtures, cases, or
  regenerations derived from the fastzip/malo corpus, the Go standard library `archive/zip` and `archive/tar`
  testdata, or published zip-bomb constructions by D. Fifield, because this session knows (CC1) that held-out F20
  items derive from those upstreams. Research III EB-Z/EB-T cases are regenerated from their own primitive
  descriptions only (as in `research/experiments/_legacy/conventions.md` C5).
- No input path may resolve under `/root/eb-research/heldout`; every driver calls the held-out guard.
- `tools/zip-compat` and `tools/tar-strict` scripts are executed only from copies in scratch; tracked files are never
  regenerated in place (PROGRESS integration note on `observed-outcomes-v1.json`).

## CC8. Handling of security-relevant findings

The repository is pushed to a public remote. A fuzz or differential finding classified as memory-safety, resource
exhaustion, confinement, verification-honesty or cryptographic impact is recorded in git by SHA-256, target, class and
triage only; its minimized bytes stay under `/root/eb-research/conformance/findings-private/` until the program owner
approves publication (the approval is recorded in the regression ledger). This handling delays publication; it never
removes a finding from any count.

## CC9. Metric conventions

- Canonical names and roles come from `decision-method.md` Appendix A.2; bands and floors are referenced by threshold
  ID and `thresholds.json` key, never restated with different values.
- Binary metrics (§3.3): `reader_disagreements`, `cleanroom_pass_fraction`, `normative_rule_coverage_fraction`,
  `unspecified_baseline_decode_steps`, `baseline_codec_public_spec`, `unresolved_fuzz_findings_at_coverage_target`,
  `equivalence_assertion_pass_fraction`, `adversarial_true_refusal_fraction`, `ambiguity_refusal_fraction`,
  `roundtrip_mismatches`, `migration_identity_conformance_fraction`: one committed counterexample is an R1/R2
  elimination for the candidate that produced it, or an HC violation of the specification when triage says so.
- HC MVT oracles that have no Appendix A.2 name (MVT-01(b)/(d), MVT-03(a)/(c), MVT-04, MVT-15(b), MVT-16(a)/(d),
  MVT-17(b)) are reported as violation counts under the MVT's own identifier; they are binary by objective §2.0 rule 1.
  Adding canonical names for them is flagged for the round-2 method review.
- Non-canonical quantities (ambiguity-log counts, disagreement cluster counts, injected-defect detection, coverage
  curves, adapter LoC, kappa) are **descriptive and secondary for every decision** (§0.2 item 4): they may motivate
  candidates, bound claims and feed cost assessments; they never eliminate or select.
- Cost inputs (`independent_reader_loc`, `reader_fragmentation_count`, `untrusted_input_loc`) feed §7 tiers only.

## CC10. Coordination with other domains

- **Legacy domain** (`research/experiments/_legacy/conventions.md`): EXP-CONF-011 reuses its runtime registry (C4),
  case generators and materialization (C5), observation schema `ebr.legacy.observation.v1` (C6), divergence
  definitions (C7), positive controls (C9) and sandboxing (C12). An observation already produced by an `EXP-LEGACY-*`
  run for the same (case SHA-256, runtime lock SHA-256, operation) is reused by SHA-256 and not re-run.
- **Container domain** (`research/experiments/_index/container-conventions.md`): budgets, reader-path and framing
  decisions (DEC-CON-*) receive their size, timing and migration evidence there; this domain supplies conformance
  cases, cross-path outcome tuples, resource ceilings of refusal and fuzz findings.
- **Crypto, integrity, platform, codecs, reconstruction domains**: crypto construction correctness, leakage, damage
  prevalence, extraction confinement, codec performance and reconstruction memory are measured there; this domain
  supplies clean-room vector reproduction, conformance cases, cross-reader tuples and fuzzing of their parsers.
- Merge rule for the index: when two domains design the same measurement, the design review keeps one and records the
  other as an alias; decision coverage is the union.

## CC11. Agent roles and session separation

| Role | Constraint |
|---|---|
| Reader builder (EXP-CONF-001 arm A; arm B; EXP-CONF-012 clean-room arms) | Fresh session; reads only the clean-room snapshot and pinned third-party module documentation; never production source, `tools/`, `research/` (other than its own output directory), the conformance corpus expectations, or web pages about Entrybound. Transcript audited (EXP-CONF-001 §6). |
| Expectation author (EXP-CONF-004) | May read production source only after writing each expectation from normative text; expectations are frozen before the census runs. |
| Expectation reviewer | A different session; reviews every expectation against the cited sentence before the census. |
| Adjudicator / triager (CC6) | Involved in neither reader and in no expectation of the cases it triages. |
| Labeller (EXP-CONF-003 recall sample) | Involved in neither the specification, the extraction script, nor either reader. |
| Analyst (C-analyze) | Not this design session (CC1); not a builder of the candidate or reader analysed. |

All sessions share one base model. That limits independence; it is recorded as a threat in every protocol and is the
reason EXP-CONF-013 (external implementer and reviewer) exists.

## CC12. Seeds registry

| Use | Seed |
|---|---|
| ebr order / bootstrap (all specs unless stated) | `2026091728` / `2026091729` |
| Mutation positions (EXP-CONF-002 M2) | `2809170001` |
| Recall sample (EXP-CONF-003) | `2809170002` |
| Triage reliability sample (CC6) | `2809170003` |
| Fuzz campaign libFuzzer seeds (EXP-CONF-008/009/010) | `{2809171001, 2809171002, 2809171003, 2809171004, 2809171005}` |
| Compat-model generator (EXP-CONF-011) | tuning `2809170011`, validation `2809170012` |
| Taxonomy rating sample (EXP-CONF-005) | `2809170005` |
