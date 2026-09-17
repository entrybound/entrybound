# EXP-CON-008: Container format-revision justification — synthesis rule and migration-cost inventory

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: decision tooling (§14 item 2), cost-accounting scripts for C1-C3 (§14 item 7), `research/tools/container/revision/binding_inventory.py`, `revision_synthesis.py` |
| Domain / program section | container / §11 (with §28, §34) |
| decision_ids | DEC-CON-002, DEC-CON-012 (namespace and version-binding inventory and the coupling between a revision and the version identity) |
| Decision type (proposal) | EMPIRICAL for the measured inputs it aggregates; FORMAL for the add-later (post-v1 incompat feature) feasibility checks; the owner decides whether a V1_BLOCKING revision proceeds (§8.2 item 18 for T3/T4) |
| OD / HC (proposal) | OD-05, OD-07, OD-08, OD-10, OD-11, OD-12, OD-21, OD-22, OD-23; HC-10, HC-12, HC-16 |
| Runner | Scripted synthesis over committed outputs of other experiments plus one scripted census; no `spec.yaml` |
| Inputs (experiments) | EXP-CON-001 (manifest), EXP-CON-002 (Index), EXP-CON-003 (outboard, tree construction), EXP-CON-004 (retrofit capability gaps), EXP-CON-005 (production anchor), EXP-CON-007 (feature tiers), EXP-REMOTE-004 and EXP-REMOTE-007 (remote legs), EXP-CONF-001 and EXP-CONF-002 (independent-reader ambiguity log attributable to container structure), EXP-CONF-006 (historical stored-bytes corpus), EXP-EVAL-002 (small-archive fixed overhead vs incumbents, H4) |
| Method, author, L7, gates | `../_index/container-conventions.md` §1-§2 |

## 1. Question

Do the measured results for the container structures (Manifest representation, Index, Merkle outboard and tree construction, section locator and registry structure, fixed framing) justify a container format revision before the v1 freeze, and if so of what scope (additive incompat features after v1, a pre-freeze cleanup revision, or a new generation namespace), once the permanent migration cost of every byte, transcript, identity descriptor and vector that binds the current namespace and version is counted?

## 2. Hypotheses

- **H1 (scope).** At least one structure (expected: Manifest representation at ≥ 1e5 entries, or the outboard for verified ranges in objects of ≥ 1e4 Chunks) passes the R6 minimum-gain rule of its computed tier in its scope on validation, and every such structure can be added as a post-v1 incompat feature without changing an existing identifier (FORMAL add-later check), so `additive-incompat-features-post-v1` is the lowest-tier candidate that captures the gain.
- **H2 (inventory).** The status-quo namespace `ecf/bootstrap-v1` and format version 0.1 are bound by at least the Descriptor v1 vectors, the crypto-v1 transcripts (`crypto/container.rs` bindings), the signature CONTENT transcript and the historical corpus; a rename (`rename-to-v1-major-1`) re-issues every permanent vector that binds them and changes every signed or encrypted archive's transcript.
- **H0 / falsification.** H1 falsified if no structure passes its minimum-gain rule in any scope (then `freeze-status-quo` or `defer-revision-to-v2` stands under R10), or if a passing structure cannot be added post-v1 without changing an existing identifier (then a pre-freeze revision is the only way to capture it). H2 falsified if the inventory finds fewer bindings than listed.

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CON-002 | The pre-registered synthesis rule applied to the measured inputs; the revision-scope table with computed tiers, minimum-gain outcomes and add-later feasibility; the migration-cost inventory (C7) | All measurements (inputs list); owner acceptance for T3/T4 selections |
| DEC-CON-012 | Inventory of every binding of namespace and version (code, docs, vectors, transcripts, identity descriptors, historical corpus); coupling table between revision candidates and version-identity candidates | Version-gate behaviour and minor-version tolerance tests: EXP-CONF-004, EXP-CRYPTO-016; independent-reader version gate: EXP-CONF-001/002 |

## 4. Candidates

DEC-CON-002: `freeze-status-quo`; `additive-incompat-features-post-v1`; `pre-freeze-cleanup-revision` (stable section ids, registry, ignorable class, framing widths, sequence caps); `new-generation-namespace`; `defer-revision-to-v2`. No exclusions.

DEC-CON-012 (evaluated for coupling only; the decision's gate tests are elsewhere): `keep-bootstrap-identity`, `rename-to-v1-major-1`, `rename-with-dual-acceptance-window`, `minor-tolerant-readers`, `strict-version-gate-status-quo`; `negotiated-version-fallback` excluded before execution (ledger exclusion; SPEC §9.9 rule 2 L1159).

## 5. Inputs

- Validation-look outputs (never tuning-only results) of the input experiments, at the commits recorded in their outcome records.
- Binding inventory census over the repository at `9e44608`: `crates/**` (string constants `ecf/bootstrap-v1`, `FORMAT_NAMESPACE`, `format_major`, `format_minor`, transcript builders), `docs/*.md` and every permanent vector file (`docs/descriptor-vectors-v1.txt`, `docs/posix-metadata-v1-vectors.txt`, `docs/security-metadata-v1-vectors.txt`, `docs/crypto-wire-v1-vectors.txt`), plus the historical corpus manifest of EXP-CONF-006 (counts of archives per generation).
- No corpus items are read by this experiment. Held-out: none (section 15).

## 6. Environment and platform requirements

Any host with Python 3.12 and the repository; no timing, no privileges, negligible disk.

## 7. Commands

```sh
python3 research/tools/container/revision/binding_inventory.py --repo . --commit 9e44608 --historical-manifest <EXP-CONF-006 manifest path> --out research/normalized/EXP-CON-008/binding-inventory.csv
python3 research/tools/container/revision/revision_synthesis.py --rule research/experiments/EXP-CON-008/synthesis-rule.json --inputs research/experiments/EXP-CON-008/inputs.json --tiers <§14 item 7 cost outputs> --out research/normalized/EXP-CON-008/decision/
```

`synthesis-rule.json` and `inputs.json` are committed with this protocol's first tooling commit, before any input experiment's validation look, and encode section 10 exactly.

## 8. Seeds and repetitions

None needed (deterministic scripts over committed outputs); scripts run twice and outputs must be byte-identical (§12.3 item 4).

## 9. Metrics

| Quantity | Source | Role |
|---|---|---|
| Per structure and scope: confirmatory verdicts and R6 minimum-gain outcome of the provisional winner against the status quo on its primary metrics | input experiments' decision outputs | synthesis input |
| Computed cost tier per candidate structure | §14 item 7 scripts (C1-C3) and the independent C4-C7 assessment | synthesis input |
| `normative_words_added`, `wire_items_added` per revision candidate | §14 item 7 C1 counter over the candidate specification diffs | cost input (R10 key 2) |
| `independent_reader_loc`, `reader_fragmentation_count` | EXP-CONF-001, EXP-CON-007 | cost inputs |
| Binding inventory counts: code sites, doc statements, vector records, transcript fields, identity descriptor fields, historical archives per generation | `binding_inventory.py` | C7 migration cost (descriptive) |
| Container-attributable ambiguity-log entries | EXP-CONF-001/002 | counterevidence and cost input |
| `fixed_overhead_bytes` on the small tier | EXP-EVAL-002 H4 | guard input |

## 10. Synthesis rule (pre-registered)

1. **Per structure S** in {Manifest representation, Index structure, verification structure and outboard layout, tree construction and count binding, section locator table, section-id and registry structure, fixed framing fields}: take the input experiment's R4-R8/R9 outcome on validation. S is **justified in scope P** iff its provisional winner W_S ≠ status quo, W_S passes R6 against the best lower-tier candidate in P, and the R7 bars hold at P (MEDIUM band allowed, recorded as a soft cap).
2. **Add-later feasibility (FORMAL).** For each justified S, a written check (reviewed by a second session) answers: can W_S be introduced after v1 as a new incompat (or ro_compat) feature and new identifiers only, with every existing identifier's behaviour unchanged (HC-16) and without changing LAI/AUX/PCR of archives that do not use it? Record `add-later-feasible` or the violated identifier.
3. **Candidate costs.** `freeze-status-quo`: no wire cost; the unrealized gains of justified structures are recorded as known limitations. `additive-incompat-features-post-v1`: the sum over justified, add-later-feasible structures of their tiers, plus `reader_fragmentation_count` increments. `pre-freeze-cleanup-revision`: justified structures that are not add-later-feasible plus the cleanup items that pass their own decisions (registry and section-id decisions of EXP-CONF-003/004), plus the migration cost C7 from the binding inventory restricted to the bindings the cleanup touches. `new-generation-namespace`: all of the above plus every binding in the inventory (full C7). `defer-revision-to-v2`: as freeze, plus the add-later cost of every justified structure.
4. **Selection.** R4 across candidates uses the aggregated benefit per scope (each justified structure's validated verdict counts only in its scope; there is no cross-structure utility sum beyond §6.2 per-OD weights) and the computed tiers; R6 applies between candidates of different tiers; R10 keys resolve ties (lower tier, fewer normative words and wire items, fewer reader LoC, status quo). A candidate whose benefit rests on a structure with an `INCONCLUSIVE` verdict without demonstrated power cannot be selected (R10 power rule).
5. **Coupling to DEC-CON-012.** A revision candidate that changes any binding in the inventory requires a version-identity candidate other than `keep-bootstrap-identity`; the joint table lists the permitted pairs and their combined C7 cost.

## 11. Practical-significance thresholds

Those of each input metric in `research/methods/thresholds.json`, with minimum-gain bands per computed tier (§3.1, §7.3). Nothing is restated or re-banded here.

## 12. Sensitivity analysis

(a) Tier-dispute arm: every disputed tier resolved to the higher tier (§7.2) and, separately, to the lower, reporting whether the selection changes; (b) scope arm: remove the synthetic entry-count strata from EXP-CON-001/002/003 inputs (real items only); (c) remote-weight arm: selection with and without the remote legs (EXP-REMOTE-004/007) as evidence for OD-12; (d) R7 arm: treat MEDIUM-band stability as failure.

## 13. Adversarial search and expected negative results

- `adversarial-search/`: for each justified structure, search for an existing identifier whose behaviour would have to change to add it post-v1 (the add-later counterexample), and for bindings missed by `binding_inventory.py` (grep of hex encodings and length-prefixed forms of the namespace string in vectors).
- Negative results worth recording: no structure justified (the container freezes as is); justified structures that are all add-later-feasible (no pre-freeze revision needed); a rename whose migration cost exceeds every measured gain.

## 14. Threats to validity

The synthesis inherits every input's limitations and soft caps; it adds no measurement. The rule weighs structures independently, which can miss interactions (for example paged Manifest plus paged Index sharing a locator table); interactions are handled only where an input experiment measured the combination (EXP-REMOTE-004 composes them). Tier assessments involve judgment (sensitivity (a)). L7 exposure (conventions §1).

## 15. Phase D held-out placeholder

The synthesis is re-applied after Phase D to the held-out outcomes of the justified structures' experiments (R5 per structure); a structure that fails R5 loses its justification and the selection is recomputed by the same rule without re-selection of candidates (§5.6).

## 16. Cost and effort

Compute under 1 h. Disk negligible. Agent effort: judgment for the add-later feasibility checks (two sessions) and the synthesis record; the rule itself is scripted.
