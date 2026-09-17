# EXP-LEGACY-034: export verification oracle — LOSSY-prediction mutation testing, receipt completeness, derivation-chain tampering and receipt signing cost

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (research-only default-off export mutation hooks and comparator prototypes in `entrybound::research::legacy::export_verify` with identity proof; `ebr-legacy-export --mutate <id>`; chain prototypes `receipt_chain.py`; in-toto and DSSE tooling pinned in a dedicated venv; comparator code audit) |
| Domain / program sections | legacy / §27 (with §22.5 composition, §23 provenance, §24 integrity) |
| decision_ids | DEC-INT-023, DEC-LEG-045, DEC-CRY-093, DEC-LEG-018 |
| Decision types (proposed) | EMPIRICAL (mutation kill, tamper detection) with FORMAL parts (comparator code audit with `path:line`); cryptographic construction of signed receipts EXTERNAL (§8.3) |
| OD / HC (proposed) | OD-20 (M20.4), OD-15 (M15.1 for signed-receipt candidates), OD-05 (receipt bytes); HC-07 (MVT-07(c)), HC-15, HC-14 (signed candidates only) |
| Shared rules / L7 / gates | C§ header, C§2 |
| Timing | `timing: false` (signing and verification time recorded informationally in-process only) |

## 1. Question

Does export refuse publication whenever its output differs from the source in any way not predicted by a typed issue,
and which comparator design guarantees that; do receipts enumerate every degradation; and which derivation-chain and
receipt-signing designs detect substitution, replay and modification of chain elements, at what byte cost?

## 2. Hypotheses

- **H1 (status quo kill rate, DEC-INT-023).** The status-quo export pipeline refuses every semantic mutant M1-M7 and
  M10-M16 before publication (`adversarial_true_refusal_fraction` = 1 over semantic mutants). *Falsified by* one
  published semantic mutant (a production defect under HC-07 MVT-07(c)).
- **H2 (issue misattribution).** Mutants M15 (degradation applied to an entry other than the predicted one) and M16
  (degradation applied while its issue is dropped from the receipt) are the most likely to survive a count-based or
  class-based comparator; a field-mask comparator kills both.
- **H3 (chain, DEC-LEG-045).** Without a chain (status quo), substitution of the intermediate `.eb` by an archive with
  the same LAI but different AUX is undetectable from supplied artifacts; `embed-upstream-provenance` and
  `receipt-chain-command` detect every unsigned substitution attack A1-A6 but not a forged receipt A7; only signed
  candidates detect A7.
- **H4 (cost, DEC-CRY-093).** A detached signature over a receipt digest adds < 1 KiB; a DSSE/in-toto envelope adds
  < 4 KiB per receipt `[INFERRED]`.
- **H0.** Every comparator and chain design detects every mutation and attack.

## 3. Decisions informed

| Decision | Supplied here |
|---|---|
| DEC-INT-023 | Location of the predicted-difference comparator (audit); mutation kill matrix per comparator candidate |
| DEC-LEG-045 | Tamper/substitution detection matrix and chain assembly cost per chain design |
| DEC-CRY-093 | Forged-receipt threat cases A1-A7 against unsigned and signed designs; bytes and informational time of signing; wording census of "authenticated" sidecar claims in docs and CLI text |
| DEC-LEG-018 | Receipt completeness under M16 (dropped issues) for v1/v2 and per-entry versus per-class receipt prototypes |

## 4. Candidates

| Decision | Candidates | Treatment |
|---|---|---|
| DEC-INT-023 | status-quo-undescribed; predicted-issue-diff; field-mask-comparison; explicit-diff-report-gate; encoder-success-only | Status quo measured as built; three comparator prototypes; `encoder-success-only` is a proposed L0 exclusion (`docs/compressed-tar-export-v1.md` §Verification L44-49: encoder completion is insufficient) run as the zero-kill reference |
| DEC-LEG-045 | status-quo-no-chain; embed-upstream-provenance; receipt-chain-command; in-toto-layout-chain; defer-post-v1 | Prototypes; `defer-post-v1` has no artifact (cost of adding later recorded, C7) |
| DEC-CRY-093 | status-quo-unsigned; detached-ebsig-over-receipt; dsse-in-toto-receipt; signed-sidecar-native; wording-correction-only; embed-signature-in-legacy-target | `embed-signature-in-legacy-target` is a proposed L0 exclusion (`docs/legacy-export-v1.md` L51-52: legacy targets never embed Entrybound signatures); others prototyped; construction review EXTERNAL |
| DEC-LEG-018 | status-quo-v1-v2; per-entry-enumeration; per-class-counts-with-samples | completeness under M16 |

## 5. Corpus selectors

Sources (tuning/validation only): tuning `f04-tuning-sourcelike`, `f19-tuning-real-home-snapshot`,
`f01-tuning-ripgrep-14-1-1-git`; validation `f04-validation-objstore`, `f19-validation-real-home-snapshot`,
`f01-validation-redis-7-2-16-git`; plus EXP-LEGACY-030 generated stress trees (timestamps, paths, ownership, empty
directories; tuning/validation seeds) and EXP-LEGACY-001 C5 ZIPs for chains (zip → eb → tar.zst). Mutants M1-M16 are
applied to every (source, profile) where the mutated field exists. Held-out: placeholder (C§5).

## 6. Environment

`wsl-ubuntu`; research build `ebound-research` (C§3) with mutation hooks off proven byte- and outcome-identical to
`ebound-head`; signing keys generated locally for the experiment only (never user keys; no credentials entered
anywhere).

## 7. Commands and tooling

Tooling: `entrybound::research::legacy::export_verify` (mutation hook points after target analysis and before strict
re-import; comparator prototypes `predicted-issue-diff`, `field-mask`, `explicit-diff-report`), `ebr-legacy-export
--mutate M<n> --comparator <c>`; `research/tools/legacy/comparator_audit.md` (committed audit table, `path:line`);
`research/tools/legacy/receipt_chain.py --design embed|chain-command|in-toto --attack A<n>`; `receipt_sign.py --design
ebsig|dsse`; `wording_census.py` over `docs/` and CLI help text.

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
bash research/harness/internals-identity-check.sh   # extended with legacy export cells (C§3); must PASS with hooks off
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-034 --from-experiments EXP-LEGACY-030,EXP-LEGACY-001 --append-corpus-items research/experiments/EXP-LEGACY-034/real-items.txt
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-034/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-034/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-034/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/wording_census.py --paths docs crates/entrybound-cli/src/lib.rs --terms authenticated,verified,signed --out research/normalized/EXP-LEGACY-034/wording.csv
```

## 8. Seeds

`order 2509173411`, `bootstrap 2509173412`, `command 2509173413`; mutant target-entry selection seeded by the command
seed.

## 9. Replication

2 repetitions per (source, profile, mutant or attack, candidate) with repeat-hash of outcomes; a surviving mutant is a
committed counterexample without rerun (objective §2.0 rule 2).

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `adversarial_true_refusal_fraction` (semantic mutants refused before publication) | binary | OD-20 | §3.3 |
| `receipt_completeness_fraction` | binary | OD-20 | §3.3 |
| `undetected_tampers` (signed-receipt candidates claiming tamper evidence, attacks A1-A7) | binary for claiming candidates | OD-15 | §3.3 |
| `side_data_bytes` (receipt and signature/envelope bytes) | banded T-02 (primary for DEC-CRY-093 cost) | OD-05 | T-02 |
| kill matrix; attack detection matrix; wording census counts | non-canonical descriptive | — | none |

## 11-12. Normalization and analysis

Mutant outcomes: published or refused, with reason; semantic versus non-semantic classification of each mutant is fixed
here (M8 reorder is non-semantic for ZIP/tar targets only if the profile documents order as physical; otherwise
semantic). Binary results decide R1 eliminations per comparator; `side_data_bytes` compared per §4.9-§4.11 across chain
and signing candidates on the same receipts (deterministic, §4.13).

## 13. Practical significance

T-02 `side_data_bytes`; binary §3.3.

## 14. Sensitivity analysis

Profile arm (all six frozen profiles); mutation-position arm (first, middle, last entry); receipt-version arm (v1, v2,
prototypes).

## 15. Expected negative results worth recording

Status quo publishing a semantic mutant (defect); comparators that kill all mutants only because strict re-import fails
for unrelated reasons (checked by reason code); chain designs detecting nothing beyond what LAI comparison already gives.

## 16. Threats to validity

Mutation hooks at one pipeline point may miss defects earlier in target analysis (audit table documents coverage);
attacks are designer-chosen; cryptographic adequacy of signing designs is not assessed here (EXTERNAL).

## 17. Platform requirements

Linux x86-64 WSL2; no privileged operations; storage ~10 GB; network only for pinned in-toto/securesystemslib wheels.

## 18. Estimates

Compute ≈ 6 CPU-hours; disk ≈ 10 GB. Agent effort: comparator audit and prototype implementation judgment; execution
none.

## 19. Expected cost tiers `[INFERRED]`

Comparator change without wire effect: T0-T1; receipt v3 with embedded upstream provenance: T2; chain command: T2 (new
user-visible command); signed receipts: T4 if a new cryptographic construction, T2 if reusing an existing signature
record over a digest (to be assessed by an uninvolved session and external review).
