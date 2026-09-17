# EXP-CRYPTO-015: Feature-composition and mutation matrix — identity roots, binding status, nonce/key uniqueness under attacker-controlled input

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

For every combination of conversion evidence, preservation, encryption, signatures, recipient add/remove, password change, epoch rotation, random access and repack, what outcome (supported, refused, declared loss) and what identity roots (LAI/AUX/PCR/PCI) and binding statuses result, are they consistent with the identity definitions, and are they enforced by tests (DEC-CRY-010, DEC-MOD-029)? Under every mutation on an attacker-controlled input archive, does every produced record have a unique (key, nonce) and fresh salts (DEC-CRY-054), and which mutation contract do recipient changes and rotation use (DEC-CRY-050, DEC-CRY-069, DEC-CRY-031, DEC-CRY-045, DEC-CRY-105)?

## 2. Hypotheses and falsification

- **H1 (matrix completeness).** A normative executable matrix with a test per cell covers every operation combination; the status-quo per-operation behaviour has cells that are untested or inconsistent with the root definitions (for example AUX for `--add` per DEC-MOD-029's derived-from-definitions candidate).
- **H2 (nonce uniqueness).** Over 10,000 seeded mutation sequences on attacker-controlled inputs (duplicate salts, crafted ordinals), no produced record repeats a (derived-key, nonce) pair (MVT-14(c)). The status-quo reuse-payload/regenerate-control contract holds; `full-reencryption-every-mutation` trivially holds; a naive rewrap-only contract would repeat.
- **H3 (rotation cost vs safety).** Boundary-preserving rotation (DEC-CRY-050) keeps PCR stable but re-uses boundary secrets known to removed recipients, a measurable leakage against removed recipients; full rotation costs re-encryption time (EXP-CRYPTO-003) but leaks nothing to them.
- **Falsification:** any repeated (key, nonce) (H2) is an R1/HC-14 violation with the sequence as the artifact. H1/H3 are conformance/leakage verdicts.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` matrix outcomes, root/binding conformance (some `FORMALLY_DERIVED` from root definitions), and nonce-uniqueness property tests (binary, HC-14). A written uniqueness argument and mutation-sequence resistance go to external review (EXP-CRYPTO-022). This matrix also provides the composition cells for legacy provenance/preservation decisions and container mutation decisions owned by sibling domains.

## 4. Candidates and arms

- **DEC-CRY-010** composition matrix: `status-quo-per-operation`, `normative-executable-matrix`, `refuse-untested-compositions`, `declared-loss-records`, `restricted-v1-subset` (`silent-evidence-drop` excluded at R1).
- **DEC-MOD-029** per-operation binding outcomes: `spec-table-as-written` (excluded at R1: AUX must change on `--add`), `derived-from-definitions`, `repo-refined-table`, `executable-matrix`.
- **DEC-CRY-054** mutation nonce strategy: `status-quo-reuse-payload-regenerate-control`, `full-reencryption-every-mutation`, `epoch-rotation-on-untrusted-input`, `session-key-per-invocation`.
- **DEC-CRY-050** rotation: `status-quo-full-rotation-rechunk`, `boundary-preserving-rotation`, `separate-boundary-key-new-version`, `lazy-rotation-on-repack` (`rewrap-only-removal` excluded at R1).
- **DEC-CRY-069** recipient-mutation semantics; **DEC-CRY-031** encrypted-to-encrypted repack; **DEC-CRY-045** recipient-set integrity; **DEC-CRY-105** unlock-on-failure; **DEC-ACC-049** repack edge cases (encrypted repack cell); **DEC-CON-016** legacy crypto-v1 acceptance window (composition with mutation); **DEC-LEG-040** rewrite provenance chain (AUX/signature effect cell); **DEC-CRY-057** (POST_V1) multiple password stanzas, recipient addition to password archives and protection-mode conversion — the mutation-contract and re-encryption-cost cells are measured here (offline-guessing analysis of multi-password archives is in EXP-CRYPTO-017), with `mixed-hybrid-and-password` excluded at R1 (no-mixing freeze).

Reference candidate: the status quo of each decision.

## 5. Corpus selectors

- **Matrix base archives (tuning):** small real trees `f01-tuning-curl-8-19-0-git`, `f04-tuning-sourcelike`, `f19-tuning-zoo`, `f17-tuning-duptree-mixed`, plus generated trees, in each of {plain, encrypted, signed, converted-from-legacy, preserved} initial states. `f20-tuning-generated-private-vault` supplies attacker-controlled input archives (crafted salts/ordinals).
- **Mutation sequences (tuning):** up to length 5 over {key add, key remove, change-password, epoch rotation, sign --embed, repack --profile, repack --strip-provenance, interrupted write}. 10,000 seeded sequences for the nonce-uniqueness property test.
- **Validation:** the matrix on `f01-validation-redis-7-2-16-git`, `f17-validation-duptree-vendor` and generated archives, one registered look.

## 6. Environment and platform requirements

`wsl-ubuntu`, `timing: false` (matrix and nonce uniqueness are deterministic given seeds and use a research-only instrumented build that logs (derived key id, nonce) pairs, F-24, with MVT-16(c)). Rotation-cost timing is deferred to EXP-CRYPTO-003. Interrupted-write cells use the kill injector of the HC-18 harness (planned); where the injector is unavailable, those cells are marked NEEDS_TOOLING.

## 7. Commands and tooling

- `ebr-crypto compose`: applies a mutation sequence, records outcome class, all four roots (from `ebound inspect --json` with keys), binding statuses, and the (key, nonce) log from the research build.
- Nonce logger: research-internals accessor that records every (derived-key identifier, nonce) a mutation emits, per MVT-14(c).
- Oracle: root/binding expectations derived from the SPEC §8.0/§8.1 definitions by a session not involved in the writer; the derived-from-definitions candidate is the reference oracle for roots.
- Kill injector (shared with the HC-18 / EXP-CRYPTO-010 tooling) for the interrupted-write cells.

## 8. Seeds, warmup, repetitions and adaptive rule

- Deterministic matrix: 2 repetitions plus repeat-hash of the outcome/root/binding records (encryption randomness differs per run, so roots that must differ are checked to differ and roots that must be stable are checked stable across the two runs).
- Nonce uniqueness: 10,000 sequences; the detection bound is about 95% at a 3×10⁻⁴ per-sequence defect rate (rule of three, §D.5), and the (key, nonce) set is checked exhaustively within the run.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| repeated (key, nonce) count | HC-14 (M14(c)) | OD-15 | **binary** (R1) |
| root-conformance fraction (LAI/AUX/PCR/PCI per cell vs oracle) | T-20 / binary | OD-20 (M20.1/M20.2) | **primary** / binary where identity-rule |
| binding-status correctness fraction | T-20 | OD-04/OD-15 | **primary** |
| outcome-class correctness (supported/refused/declared-loss vs oracle) | T-20 | OD-04 | **primary** |
| `migration_identity_conformance_fraction`, `migration_determinism_fraction` | HC-10/HC-13 | OD-20 | binary |
| leakage against removed recipients (boundary-preserving rotation) | `presence_advantage` (T-17) | OD-16 | secondary (cross-ref EXP-CRYPTO-002) |
| declared-loss-record completeness | `receipt_completeness_fraction` (M20.4) | OD-07 | binary |

## 10. Normalization and statistical analysis

- Binary identity-rule and nonce metrics: any failure is R1 with the committed cell/sequence.
- Fractions over the fixed matrix: T-20 exact; AGG-C headline (min over strata).
- Confirmatory W against S on validation for the graded fractions; MDE gate.
- Leakage against removed recipients uses the EXP-CRYPTO-002 pipeline.

## 11. Practical significance

T-20 and T-17 bands and §7.2 tiers from `research/methods/thresholds.json`. `separate-boundary-key-new-version` and `full-reencryption` differ in tier and cost; the executable matrix itself is a T1 test-and-doc change. Wire-frozen exclusions enforced at R1.

## 12. Sensitivity analysis

Not weight-based. Reported arms: initial archive state, mutation-sequence length, attacker-controlled versus locally produced input, and rotation contract. CB triggers evaluated.

## 13. Expected negative results worth recording

- The SPEC §8.1 table as written is inconsistent with the AUX definition for `--add` (DEC-MOD-029), so the derived-from-definitions table is required.
- Boundary-preserving rotation leaks re-used boundary secrets to removed recipients, a real cost against DEC-CRY-050's cheaper option.
- Some status-quo composition cells are untested, supporting the normative executable matrix (DEC-CRY-010).

## 14. Threats to validity

- Oracle derivation is a judgement; independently reviewed against SPEC lines.
- The research nonce logger must be byte-identical to production when off (MVT-16(c)).
- Interrupted-write cells depend on the kill injector; missing, they are NEEDS_TOOLING.
- Mutation-sequence resistance to splicing is external (EXP-CRYPTO-022); only uniqueness and outcomes are measured.

## 15. Held-out placeholder (Phase D)

Binary identity and nonce checks run on held-out on every split (R1). Graded fractions get one held-out look. No held-out item named here.

## 16. Cost estimate and agent effort

- Compute: about 60 core-hours (10,000 mutation sequences plus the matrix). Disk: about 25 GB transient.
- Agent effort: tooling **judgment** (compose driver, nonce logger, oracle, kill injector); execution **scripted**; analysis **judgment**.

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
