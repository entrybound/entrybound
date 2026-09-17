# EXP-CRYPTO-013: RFC 3161 timestamp verification profile, dependency attack surface and long-term validity

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** — Docker sandboxing unavailable (process-isolation substitute) |
| Kind | decision |
| Domain | crypto (program §22.1, §22.4) |
| Decisions informed | DEC-CRY-006, DEC-CRY-076, DEC-CRY-098, DEC-CRY-099, DEC-CRY-083, DEC-CRY-066, DEC-CRY-097, DEC-CRY-100 |
| Platforms | Linux/x86-64 |
| Requires timing | False |
| Estimates | 20 machine-hours; 5 GB disk |
| Tooling to build | ebr-crypto tsverify; real TSA token corpus (owner-fetched); negative generator; cargo-fuzz targets |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns no validation look |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
<!-- END AUTO:meta -->

## 1. Question

Which TSTInfo/CMS fields must the v1 timestamp verifier constrain, and does the constrained profile reject substitution and malformed-token attacks while accepting real TSA tokens (DEC-CRY-076, DEC-CRY-098, DEC-CRY-099)? Is shipping RFC 3161 verification on the pre-release `cms` crate acceptable, or must it be gated, vendored-and-fuzzed, or replaced by a minimal parser (DEC-CRY-006)? What are the right frozen signature/timestamp limits (DEC-CRY-083), how is timestamp evidence renewed (DEC-CRY-066), and how is validity established decades later without live infrastructure (DEC-CRY-097)?

## 2. Hypotheses and falsification

- **H1 (profile).** The status-quo minimal checks (imprint, signed attributes, EKU, chain) accept at least one certificate-substitution or attribute-omission attack from the negative corpus. Adding ESS signing-certificate-v2, TSA-name match, policy-OID allowlist and critical-extension rejection reduces `benign_false_refusal_fraction` acceptably while raising `import_acceptance_fraction` on real tokens to 1.
- **H2 (dependency surface).** The `cms =0.3.0-pre.2` parser has more untrusted-input LoC and attacker-controlled parameters, and more fuzzing findings at a fixed coverage target, than a minimal profile parser limited to the frozen fields.
- **H3 (fuzzing).** A local cargo-fuzz campaign on `verify_timestamp` finds zero unresolved findings at the coverage target for the minimal profile parser; the pre-release `cms` path may not.
- **Falsification:** H1 fails if the minimal checks reject nothing extra the corpus contains and the constrained profile rejects real tokens. H3 fails if the minimal parser also has unresolved findings.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
#### DEC-CRY-006 — Is shipping v1 RFC 3161 verification on cms =0.3.0-pre.2 (with der, spki and x509-cert) acceptable, or must timestamp verification be gated to TIMESTAMP_UNSUPPORTED, vendored and fuzzed, replaced by a minimal profile parser, or isolated?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-009, EXP-ECO-001.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-prerelease-cms` | Status quo: cms =0.3.0-pre.2 pinned | status quo | yes | EXP-ECO-001 |
| `pin-stable-release` | Wait for and pin a stable cms release before stable v1 | ledger alternative | yes | EXP-ECO-001 |
| `gate-timestamps-unsupported` | Ship with timestamp verification disabled until audited | ledger alternative | yes | — |
| `minimal-profile-parser` | Hand-written DER parser limited to the frozen profile | ledger alternative | yes | EXP-ECO-001 |
| `vendor-and-fuzz` | Vendor the crate and run continuous local fuzzing | ledger alternative | yes | EXP-ECO-001 |
| `sandboxed-parse` | Parse tokens in an isolated process with resource limits | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-prerelease-cms`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-076 — Which TSTInfo and CMS fields must the v1 timestamp verifier constrain (policy OID allowlist, TSA name, nonce, accuracy and skew, critical extensions, ESS signing-certificate-v2), and which negative tests make the profile normative?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-004.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-minimal-checks` | Status quo: imprint, signed attributes, EKU, chain; TSTInfo policy/nonce/accuracy/extensions and ESS ignored | status quo | yes | — |
| `require-ess-signing-certificate-v2` | Require and check ESS signing-certificate-v2 (RFC 5035/5816) | ledger alternative | yes | — |
| `policy-oid-allowlist` | Caller policy carries acceptable TSA policy OIDs | ledger alternative | yes | — |
| `tsa-name-match` | Check TSTInfo tsa GeneralName against the signer certificate | ledger alternative | yes | — |
| `reject-unknown-critical-extensions` | Reject tokens and certificates with unknown critical extensions | ledger alternative | yes | — |
| `accuracy-and-skew-window` | Apply accuracy and allowed clock skew to genTime comparisons | ledger alternative | yes | — |
| `full-rfc3161-rfc5816-profile` | Implement every MUST in RFC 3161 and RFC 5816 verification | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-minimal-checks`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: `require-ess-signing-certificate-v2`, `full-rfc3161-rfc5816-profile`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-098 — Which TSA signature and digest algorithms does the v1 RFC 3161 profile admit: Ed25519 with SHA-256 only (frozen), SHA-512 for Ed25519 per RFC 8419, ECDSA and RSA signers and chains, NULL-parameter digest identifiers, or timestamps gated off?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-ed25519-sha256-only` | Status quo: Ed25519 CMS signer, SHA-256 digests, Ed25519-signed chain | status quo | yes | — |
| `ed25519-with-sha512` | Admit SHA-512 CMS digest for Ed25519 signers per RFC 8419 | ledger alternative | yes | — |
| `add-ecdsa` | Admit ECDSA P-256/P-384 signers and chains | ledger alternative | yes | — |
| `add-rsa` | Admit RSA PKCS#1 v1.5 and RSA-PSS signers and chains | ledger alternative | yes | — |
| `accept-null-parameters` | Accept SHA-256 AlgorithmIdentifiers with explicit NULL parameters | ledger alternative | yes | — |
| `separately-versioned-profile` | Keep v1 narrow and add a broader timestamp profile under a new feature | ledger alternative | yes | — |
| `gate-timestamps-unsupported` | Report all tokens TIMESTAMP_UNSUPPORTED until a reviewed profile exists | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-ed25519-sha256-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: `ed25519-with-sha512`, `add-rsa`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-099 — Given the RFC 3161 token lies outside the signed transcript, should caller policy be able to require a valid timestamp, should some signing profiles mandate timestamps, and should verify detect or report stripping and substitution (for example records differing only by token)?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-optional-unrequirable` | Status quo: optional token, no policy can require it | status quo | yes | — |
| `require-timestamp-policy` | Add require-timestamp (optionally with anchors) to SignaturePolicy and CLI | ledger alternative | yes | — |
| `profile-mandated-timestamps` | Release-signing profile requires a valid timestamp | ledger alternative | yes | — |
| `report-transcript-duplicates` | Report records sharing a signed transcript with differing tokens | ledger alternative | yes | — |
| `unique-transcript-on-embed` | Refuse embedding a record whose transcript is already present | ledger alternative | yes | — |
| `timestamp-inside-v1-transcript` | Move the timestamp claim inside the signed transcript (minisign trusted-comment style) | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "timestamp-inside-v1-transcript", "reason": "The token is defined over the signed record and excluded from the Ed25519 transcript", "invariant_violated": "crypto-v1 wire frozen (docs/crypto-wire-v1.md L704-708)"}.

**R0 checklist:** 1 status quo: `status-quo-optional-unrequirable`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: `timestamp-inside-v1-transcript`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `timestamp-inside-v1-transcript`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-083 — Are the frozen signature limits justified: 64 KiB maximum timestamp token, 16-certificate chain depth, and 16-byte signer identifier truncation?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-limits` | Status quo limits | status quo | yes | — |
| `raise-token-limit-new-version` | Larger token limit in a new version | ledger alternative | no | — |
| `lower-token-limit` | Lower reader-side limit via policy | ledger alternative | no | — |
| `full-key-display` | Display and pin full 32-byte public keys | ledger alternative | no | — |
| `pin-by-public-key-only` | Use signer id for display only, never for trust | defer / no change | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-limits`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `pin-by-public-key-only`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `raise-token-limit-new-version`, `lower-token-limit`, `full-key-display`, `pin-by-public-key-only`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-066 — What operation renews signature and timestamp evidence before classical algorithms become forgeable (add a new signature, re-timestamp an existing record, evidence-record chains, an archive timestamp over the signature collection), and does it rewrite archive bytes?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-none` | Status quo: no renewal operation | status quo | no | — |
| `add-new-signature` | Add a new detached or embedded signature under a newer algorithm | ledger alternative | no | — |
| `re-timestamp-existing-record` | Attach a newer token over the same record | ledger alternative | no | — |
| `evidence-record-sidecar` | RFC 4998 evidence-record chain sidecar | ledger alternative | no | — |
| `archive-timestamp-over-collection` | Timestamp over the whole signature collection | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-none`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `evidence-record-sidecar`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-none`, `add-new-signature`, `re-timestamp-existing-record`, `evidence-record-sidecar`, `archive-timestamp-over-collection`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-097 — How is timestamp validity established decades later without live infrastructure: genTime-evaluated chains with no revocation (status quo), verification-time validity, caller-supplied offline revocation data, embedded revocation evidence, renewable evidence records, or a TSA trust list with indefinite validity?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-gentime-no-revocation` | Status quo: validity at TSA-asserted genTime, exact-DER anchors, no revocation | status quo | no | — |
| `verification-time-validity` | Evaluate certificate validity at verification time | ledger alternative | no | — |
| `caller-supplied-offline-revocation` | Caller supplies CRLs or OCSP responses as files | ledger alternative | no | — |
| `embedded-revocation-evidence` | Carry revocation evidence captured at signing time (new record version) | ledger alternative | no | — |
| `evidence-record-renewal` | RFC 4998-style evidence records renewed before algorithm or key expiry | ledger alternative | no | — |
| `tsa-trust-list-indefinite` | C2PA-style TSA trust list with indefinitely valid timestamps | ledger alternative | no | — |
| `transparency-log-anchoring-sidecar` | Optional transparency-log inclusion proof sidecar | ledger alternative | no | — |
| `online-revocation-fetch` | Fetch OCSP/CRL during verify | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "online-revocation-fetch", "reason": "Makes verification depend on live revocation infrastructure", "invariant_violated": "SPEC §24 item 7 L2509 verification never requires live infrastructure"}.

**R0 checklist:** 1 status quo: `status-quo-gentime-no-revocation`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 8 ledger candidates listed above; 4 strongest incumbent: `evidence-record-renewal`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-gentime-no-revocation`, `verification-time-validity`, `caller-supplied-offline-revocation`, `embedded-revocation-evidence`, `evidence-record-renewal`, `tsa-trust-list-indefinite`, `transparency-log-anchoring-sidecar`, `online-revocation-fetch`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-100 — What are the default timestamp trust anchors (none, bundled list, platform store, TSA trust list) and reference time (system clock, explicit --at-time, genTime only), and how is a token reported when no anchors are supplied?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-caller-files-system-clock` | Status quo: --timestamp-trust files, system clock, UNSUPPORTED without policy | status quo | no | — |
| `explicit-reference-time` | Add an explicit reference-time option | ledger alternative | no | — |
| `bundled-tsa-trust-list` | Ship a curated TSA trust list | ledger alternative | no | — |
| `platform-trust-store` | Use platform trust stores for timeStamping anchors | ledger alternative | no | — |
| `report-not-evaluated` | Report NOT_EVALUATED when no anchors are supplied | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-caller-files-system-clock`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-caller-files-system-clock`, `explicit-reference-time`, `bundled-tsa-trust-list`, `platform-trust-store`, `report-not-evaluated`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` accept/reject fractions over pre-registered case sets (T-20), attack-surface cost inputs (C4), and fuzzing findings (M24.2, binary at the coverage target). Long-term-validity semantics (DEC-CRY-097) and renewal (DEC-CRY-066) are largely analytical/standards comparisons; the mechanical parts (what verify reports, whether bytes are rewritten) are measured, and the trust semantics feed external review.

## 4. Candidates and arms

- **DEC-CRY-076 profile fields:** `status-quo-minimal-checks`; `require-ess-signing-certificate-v2`; `policy-oid-allowlist`; `tsa-name-match`; `reject-unknown-critical-extensions`; `accuracy-and-skew-window`; `full-rfc3161-rfc5816-profile`. Evaluated as a cumulative set and per field.
- **DEC-CRY-098 algorithms:** `status-quo-ed25519-sha256-only`; `ed25519-with-sha512`; `add-ecdsa`; `add-rsa`; `accept-null-parameters`; `separately-versioned-profile`; `gate-timestamps-unsupported`.
- **DEC-CRY-006 dependency:** `status-quo-prerelease-cms`; `pin-stable-release`; `gate-timestamps-unsupported`; `minimal-profile-parser`; `vendor-and-fuzz`; `sandboxed-parse`.
- **DEC-CRY-099 requirability/stripping:** `status-quo-optional-unrequirable`; `require-timestamp-policy`; `profile-mandated-timestamps`; `report-transcript-duplicates`; `unique-transcript-on-embed`.
- **DEC-CRY-083 limits:** `status-quo-limits` (64 KiB token, 16-cert chain, 16-byte signer id) versus alternatives, measured against real TSA token/chain-size distributions.
- **DEC-CRY-066 renewal, DEC-CRY-097 long-term, DEC-CRY-100 anchors/reference-time:** the mechanical behaviour (report at genTime, NOT_EVALUATED without anchors, whether bytes are rewritten) is measured; the trust policy is analytical/external.

Reference candidate: status quo of each decision.

## 5. Corpus selectors

- **Real tokens:** a committed corpus `research/tools/crypto/tsa_tokens/` of RFC 3161 tokens obtained by the program owner from at least six public/commercial TSAs (algorithms, digest params, chain algorithms, token sizes), each with provenance and hash. No live TSA is contacted during the run. Because network fetching is a program-owner action, this is a precondition, not an agent step.
- **Negative corpus:** `research/tools/crypto/tsa_negatives/`, generated per constrained field (substituted certificate, omitted ESS attribute, wrong TSA name, disallowed policy OID, unknown critical extension, out-of-skew genTime, malformed DER, oversized token, deep chain).
- No tuning/validation/held-out corpus items (this is CLI/parser behaviour over a fixed case set). Case lists are fixed before results (T-20).

## 6. Environment and platform requirements

`wsl-ubuntu`, Linux x86-64. `timing: false` for accept/reject; a `time_cpu` sub-run reports fuzzing throughput only. cargo-fuzz 0.13.2 and nightly-2026-08-01 are available (PROGRESS). Docker sandboxing (DEC-CRY-006 `sandboxed-parse`) is BLOCKED (Docker unavailable); a process-isolation prototype with resource limits is the substitute and is labelled.

## 7. Commands and tooling

- `ebr-crypto tsverify --profile <candidate> --token <der> --anchors <der...>`: runs the verifier at each candidate profile and prints outcome, per-field checks and exit status.
- Negative generator `research/tools/crypto/tsa_negatives/gen.py` (the committed counterexample-search protocol).
- `research/tools/crypto/dep_surface.py`: counts untrusted-input LoC, attacker-controlled parameters and native/unsafe surface of each parser candidate from `cargo tree`/`cargo metadata` and source (C4 cost inputs).
- `cargo fuzz` targets `fuzz_targets/verify_timestamp_*.rs` for the pre-release `cms`, the minimal parser and the vendored path; a fixed coverage target (line coverage percentage recorded before results) defines M24.2.

## 8. Seeds, warmup, repetitions and adaptive rule

- Accept/reject: deterministic, run once, repeat-hash of the outcome table.
- Fuzzing: fixed corpus seeds, a fixed wall budget (for example 4 CPU-hours per target) and a fixed coverage target; findings are triaged; the run stops at the budget or the coverage target.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `import_acceptance_fraction` (real tokens accepted) | T-20 | OD-19 | **primary** |
| `benign_false_refusal_fraction` (real tokens wrongly refused) | T-20 | OD-17/OD-25 | **primary** |
| `error_actionability_fraction` (refusal messages name the field) | T-20 | OD-25 | secondary |
| negative-corpus rejection fraction | T-20-style binary (must be 1) | OD-04 | **binary screen** |
| `unresolved_fuzz_findings_at_coverage_target` | M24.2 | OD-24 | **binary** (= 0) |
| `untrusted_input_loc`, `attacker_controlled_parameters`, `decode_path_native_unsafe_count` | C4 cost inputs | OD-24 | cost |
| `rustsec_advisory_count`, `maintainer_count`, `release_age_years`, `msrv_gap` for `cms` | C3 cost inputs | OD-22 | cost |
| real TSA token/chain sizes | descriptive | — | reported (DEC-CRY-083) |

## 10. Normalization and statistical analysis

- Coverage fractions over fixed case sets use the T-20 absolute floor `0.5/n_cases`: a one-case difference exceeds the band (deterministic; exact).
- Cost inputs are counts (AGG-P, per design), reported per candidate.
- Fuzzing findings are binary at the coverage target.
- No corpus aggregation (no tuning/validation items). No sensitivity weighting (fixed case set).

## 11. Practical significance

T-20 bands and the cost tiers of §7.2 from `research/methods/thresholds.json`. Adding an algorithm or a wire field is at least T2; the minimal-parser choice trades attack surface (C4) against maintenance (C3). Ed25519-only status quo is a frozen wire fact (SHA-256 imprint); alternatives are new versions.

## 12. Sensitivity analysis

Not weight-based (fixed case sets). Reported arms: profile field subsets; anchor-set presence/absence; reference-time source (system clock, `--at-time`, genTime only); coverage-target level for the fuzzing comparison.

## 13. Expected negative results worth recording

- The status-quo minimal checks accept a certificate-substitution token, which selects the ESS/TSA-name additions.
- The pre-release `cms` crate carries a larger untrusted-input surface and at least one fuzzing finding, supporting a minimal profile parser.
- Real TSA tokens exceed 16 KiB or chains exceed a low depth, informing the DEC-CRY-083 limits.

## 14. Threats to validity

- Real-token corpus completeness (six TSAs is a sample); reported as a limitation.
- Fuzzing time budget bounds coverage; findings are a lower bound on defects.
- Docker sandboxing unavailable; the process-isolation substitute is not the shipped design.
- Trust/long-term-validity semantics (DEC-CRY-097, DEC-CRY-066) are analytical and route to external review; only mechanical behaviour is measured.

## 15. Held-out placeholder (Phase D)

Not applicable (fixed case sets, no corpus). The accept/reject and fuzzing gates are re-run at the freeze commit for the frozen builds.

## 16. Cost estimate and agent effort

- Compute: about 20 core-hours (fuzzing dominates). Disk: about 5 GB.
- Agent effort: **judgment** (profile design, negative generation, dependency-surface assessment, fuzz triage); execution **scripted**. Long-term-validity analysis is **judgment** feeding external review.

<!-- BEGIN AUTO:common -->
**Shared provisions (binding; `research/experiments/_index/crypto-conventions.md`).**

- **Author and L7 exposure (CR1):** designed by the Phase C-design crypto session, which read `research/corpus/coverage.md` and `research/PROGRESS.md` under the shared design instructions and is recorded as L7-exposed for all families (`research/experiments/_program/l7-exposures-design-phase.jsonl`). Its candidate operationalizations, exclusions and analysis plans are re-signed by an unexposed session before G-A (PA-01); decision analyses are executed by unexposed sessions only.
- **Gates (CR0):** runs before G-A/G-B are `NOT_DECISION_GRADE`; specs with non-empty `decision_ids` launch only through `research/tools/experiments/run_guarded.py` (PA-13).
- **Candidates (CR2):** the tables above are generated; R0 item 6 is open until the crypto-cluster critic pass (PA-12).
- **Security claims (CR3):** attack, leakage and tamper results are lower bounds; selections resting on a security claim, sufficiency of a mitigation or acceptance of leakage are provisional until the EXP-CRYPTO-021 external review is received.
- **Metrics (CR6):** component throughput uses `__component_<name>` strata; chunk-stage throughput is owned by EXP-CHUNK-008 T1 (PA-17); HC oracle counts are binary under their MVT ids pending the PA-02 metric-registry disposition.
- **Timing (CR5):** affinity and guard per §4.2-§4.4 (lint-checked), staged helpers (PA-16), calibration identity and instrument-class calibration (PA-04, PA-15).
- **Split discipline (CR7):** committed specs are tuning-only or binary HC screens; graded looks use look specs derived at look time with candidates restricted to W ∪ S, registered by the owner in `research/experiments/_program/look-plan.csv` (PA-03, PA-09).
- **Held-out (CR8):** generated only by EXP-EVAL-012 at Commit A (PA-20).
- **Power (PA-06):** a banded comparison enters its full tuning run only after `research/tools/experiments/mde_feasibility.py` projects an MDE of at most 1 band unit on a tuning proxy.

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-013; edit the inputs, not this block._
<!-- END AUTO:common -->
