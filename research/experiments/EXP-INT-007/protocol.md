# EXP-INT-007: Incremental base-chain model: base references, chain resolution bounds, deletion representations and role confusion

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T9 `ebr-chain`, hostile chain generator, record-size model, role-descriptor model; `ebr-origin` wrapper) |
| Domain | integrity (program §23; objective HC-09, HC-06, HC-10, HC-02, OD-05, OD-12, OD-23) |
| Decisions informed | DEC-INT-001, DEC-INT-016, DEC-INT-017, DEC-INT-032, DEC-INT-005, DEC-INT-037, DEC-MOD-001, DEC-MOD-020, DEC-ACC-038 |
| Gates, method, author | conventions C0 |
| Evidence label | Every result is `MODEL` (conventions C2 T9): Incremental and Sidecar roles and External references do not exist on the wire at the baseline; results are at most `SUPPORTED_WITH_LIMITATIONS` until a wire implementation is measured. Identity robustness under operations uses real production identities (EXP-INT-009). |
| Spec | `spec.yaml` (one sample = the full scenario matrix for one series item and one candidate policy set) |

## 1. Question

If Entrybound adopted Incremental archives with base chains, which base-reference identity,
chain rules and deletion representation give correct extraction across realistic version
series, bounded and cycle-safe resolution on hostile chains, detection of base substitution
and role confusion, and what do they cost in bytes, verification work and remote requests
compared with independent Complete archives per version?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-INT-001 | Unbounded chains and caller-policy-only depth violate HC-06 on the hostile depth series (resolution memory or work not bounded before payload); `declared-max-depth` and `single-base-only` with a declared budget pass MVT-06 oracles; DAG-with-cycle-detection is bounded only with a declared node count. | `unbounded-chains` passes the depth bomb within declared bounds (then the hazard is not realized by the model). |
| H2 | DEC-INT-016 | Referencing bases by PCI or archive_id breaks resolution after a benign Index rebuild or repack (false "missing base") in 100% of such cases, while LAI keeps resolving; `content-digests-only` resolves but cannot detect a substituted base that contains the same content digests with different metadata; `base-archive-id` accepts a substituted archive with the same id. | LAI references fail after benign operations, or archive_id references detect substitution. |
| H3 | DEC-INT-017 | On INT-SERIES items with deletions, `additive-only` produces wrong extraction trees (extra entries) in 100% of versions with deletions (HC-02 or HC-07 failure for that use); `tombstone-entries` has the smallest total `metadata_bytes` among correct representations; `oci-whiteouts` collides with at least one real path in the tuning corpus (a `.wh.` prefix) or is shown collision-free with a count of zero. | `full-manifest-per-incremental` is within the T-02 band of tombstones on every series (no size reason to prefer tombstones). |
| H4 | DEC-INT-032 | With `role-outside-identity`, stripping content from a signed Complete archive and relabelling it as Sidecar or Incremental keeps the signature valid (undetected), whereas the status quo binding of role in LAI detects it. | Relabelling detected under `role-outside-identity`. |
| H5 | DEC-MOD-001 | The role-parameterized LAI descriptor model yields LAI(Sidecar of X) != LAI(X) for every item (property test), and equal EAMs with different roles never share an LAI. | Any collision. |
| H6 | DEC-MOD-020 | A path-independent entry digest detects 100% of pure renames in rename-heavy histories; deriving renames from ContentObject digests in tooling (`derive-in-tooling`) misattributes renames when identical contents exist at several paths (F17-like duplicates). | Tooling derivation equally accurate. |
| H7 | DEC-ACC-038 | Resolving a chain of depth d over HTTP needs at least d+1 metadata fetches before the first byte, so `remote_first_entry_latency_s` (modelled, T-10) grows linearly in d and exceeds a Complete archive by more than 1 band unit from d = 2 on NP-CONT. | Not distinguishable at d = 2. |

## 3. Decisions and candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-INT-001 | `single-base-only`, `declared-max-depth`, `caller-policy-depth`, `dag-with-cycle-detection`, `flatten-on-create`, `unbounded-chains` | T9 resolver per rule on realistic chains (INT-SERIES) and the hostile chain generator: depth 2^k (k = 0..12), fan-in {2, 4, 16, 256}, self-loop, 2-cycle, long cycle (length 1,024), stale base (base repacked), missing base, truncated base, base with mismatched digest. Oracles MVT-06(a) (counting allocator in the Rust resolver port, or `tracemalloc` peak with a declared bound for the Python model, labelled) and MVT-09(a) (acyclic, bounded closure). Proposed R2 exclusion of `unbounded-chains` (no bound computable before payload, R2 "unboundable"); kept as a negative control. Chain-length data from backup tools (restic, borg, kopia, zfs send) is a literature arm: primary-source documentation and the borg and restic repositories already produced by `research/baselines` (EXP-BASE-SIZE) inspected for chain or snapshot depth, descriptive only. |
| DEC-INT-016 | `base-lai`, `base-pcr`, `base-pci`, `base-archive-id`, `content-digests-only`, `lai-plus-pci`, `signed-base-reference` | Robustness input comes from EXP-INT-009 (identity change patterns under repack, recompress, Index rebuild, re-chunk, layout change, on real archives). Here: resolution outcomes on the chain scenarios under each reference kind; base-substitution attacks (a different archive with the same archive_id; same content digests with changed metadata; an older version signed by the same key); losslessness of incremental extraction with all bases verified (round trip against the version tree, `roundtrip_mismatches`). |
| DEC-INT-017 | `tombstone-entries`, `oci-whiteouts`, `opaque-directory-markers`, `full-manifest-per-incremental`, `anti-items`, `additive-only` | Exact byte-size model of each representation's records (canonical record rules) over the real change sets of INT-SERIES; extraction correctness across chains of length 2 to full series; reserved-name conflict count for `oci-whiteouts` and `opaque-directory-markers` over every path in INT-CORE-T, INT-SERIES-T and INT-LEGACY-T trees (paths beginning `.wh.`). `additive-only` is kept as a negative control for series with deletions. |
| DEC-INT-032 | `status-quo-role-in-lai`, `bind-base-identities`, `signature-explicit-role`, `signed-requires-complete`, `describes-lai-separation`, `role-outside-identity` | Model attack tests on signed archives (production `ebound sign` over the model's LAI descriptors computed by the T14-style independent descriptor model): content stripping, Complete to Sidecar or Incremental relabel, base identity swap, `describes_lai` conflation. The formal signature-coverage model and independent security review are external routes (§8.3) and recorded as gaps. |
| DEC-INT-005 | `refuse-all-content-commands`, `metadata-commands-only`, `resolve-by-policy`, `repack-preserve-references`, `repack-materialize-complete`, `per-command-matrix` | Per-command behaviour matrix over the model (list, inspect, unpack, diff, export, salvage, repack, random read) x scenarios (bases present, missing, stale, mismatched): outcome class, whether unaffected entries remain available, correctness. The usability of refusal messages is human-facing (ecosystem domain, simulated, §4.16). Sidecar cells are EXP-INT-008. |
| DEC-INT-037 | `status-quo-complete-only`, `complete-only-reserved-codepoints`, `sidecar-role-in-v1`, `all-roles-in-v1`, `incremental-only-in-v1`, `incremental-permanent-non-goal` | Scenario-matrix results from this experiment and EXP-INT-008, plus cost inputs C1-C4 per option counted from the model's normative-text drafts (checked-in counter, §7.1) and the record census. Demand evidence (program §32) is the ecosystem domain's; recorded as a gap here. |
| DEC-MOD-001 | `complete-only-status-quo`, `complete-plus-sidecar`, `all-three-roles`, `reserve-role-values` | Property test H5 over every INT-CORE-T item; cost inputs as DEC-INT-037. |
| DEC-MOD-020 | `path-dependent-status-quo`, `path-independent-entry-digest`, `derive-in-tooling` | Rename-heavy histories: INT-SERIES (real renames between versions) plus generated rename series (seeded moves of 1%, 10%, 50% of paths over `f04-tuning-sourcelike` and `f17-tuning-duptree-mixed`), ground truth from the generator. |
| DEC-ACC-038 | `status-quo-regular-files-complete-only`, `serve-hardlink-members`, `metadata-reads-for-links`, `non-complete-roles-with-bases`, `document-refusals` | Base-chain fetch and verification cost only (`non-complete-roles-with-bases`, H7) through `ebr-origin` with exact request and byte logs and the §4.15 latency model; hardlink and reparse-point random-access tests belong to the access and platform domains (recorded as their route). |

## 4. Corpus

- Tuning: INT-SERIES-T (version rule of conventions C3), INT-CORE-T (property and conflict
  tests), generated rename series (seed 2026091761). Validation: INT-SERIES-V, INT-CORE-V,
  generated rename series with seed 2026091762.
- Minimum support: INT-SERIES sets have 7 and 6 items over 3 families; banded corpus-level
  verdicts (`artifact_bytes`, `metadata_bytes`, `http_requests`) are secondary here (C3); binary
  and coverage results are not affected.

## 5. Environment and platform requirements

`wsl-ubuntu`, `timing: false`; `ebr-origin` on localhost for remote request and byte logs
(latency modelled per §4.15, not emulated). Offline decode check for Complete archives with
`unshare -n` (root in WSL) instead of Docker `--network none` (HC-09 MVT-09(c)).
No Windows arm (model is platform-independent; platform effects on LAI are EXP-INT-009 and
the platform domain).

## 6. Procedure and commands

1. Pack every version of every INT-SERIES item as a Complete archive with B-PROD (balanced,
   INDEXED); record identities with `ebound inspect --json`.
2. `ebr run` of `spec.yaml`: `python3 research/tools/integrity/ebr_chain.py campaign
   --series-item <item> --policy-set <candidate>` builds model incrementals per version and per
   candidate, runs the scenario matrix and hostile series, and prints one summary row.
3. `verify-raw`, `verify_detail.py`, `normalize`, `report`; decision analysis per C10.

## 7. Seeds, warmups, replication

Seeds 2026091763/64/65; hostile series and rename series per C8. Warmups 0; 2 repetitions with
repeat-hash of detail files (the model is deterministic).

## 8. Metrics

| Metric | ID | OD | Role |
|---|---|---|---|
| `roundtrip_mismatches` | HC-02 | OD-02 | binary (incremental extraction versus version tree) |
| `bounded_memory_growth_bytes` | HC-06 | OD-08 floor | binary (resolution on hostile chains) |
| `declared_cost_accuracy` | HC-09 | OD-10 floor | binary (measured resolution work over declared) |
| `adversarial_true_refusal_fraction` | HC-06 | OD-17 floor | binary (hostile chains refused) |
| `undetected_tampers` | HC-14 | OD-15 floor | binary (base substitution and role confusion on signed archives) |
| `artifact_bytes` | T-01 | OD-05 | **primary** for DEC-INT-017 and DEC-INT-037 size (sum over a series of incrementals versus Complete per version; MODEL bytes) |
| `metadata_bytes` | T-02 | OD-05 | diagnostic (deletion representation records) |
| `http_requests` | T-12 | OD-12 | **primary** for DEC-ACC-038 |
| `http_bytes` | T-11 | OD-12 | secondary |
| `remote_first_entry_latency_s` | T-10 | OD-12 | secondary (modelled from logs; would be a second OD-12 primary) |
| `verification_digest_ops` | T-12 | OD-11 | diagnostic |
| `hostile_cpu_scaling_exponent` | descriptive | OD-17 | descriptive |
| `rename_detection_fraction` | C11 proposed | OD-20 as assigned | secondary until named |
| reserved-name conflict count | none | - | descriptive |
| `normative_words_added`, `wire_items_added`, `untrusted_input_loc` | C1, C4 | OD-23, OD-24 | cost inputs |

## 9. Normalization

Series totals: ratio of the candidate's sum of artifact bytes over all versions to the sum of
Complete archives per version (T-01 floors per tier of the series total). Requests and bytes:
paired log ratio against a Complete archive of the same version (T-11, T-12 floors). Binary
counts summed (AGG-W).

## 10. Statistical analysis and sensitivity

C10, with banded verdicts secondary for lack of minimum support (section 4). Sensitivity:
network profile arm (NP-LAN to NP-INTER, never pooled, AGG-S), chain-length arm (2, 4, full
series), rename-rate arm (1%, 10%, 50%), and a slow-start arm of the latency model (§4.15 model
with and without slow-start rounds, harness constraint R1-14).

## 11. Expected negative results worth recording

- PCI and archive_id base references break on benign operations or accept substitutes.
- Incrementals save little over Complete archives with dedup on some series (small deltas are
  dominated by metadata), which argues for `complete-only-reserved-codepoints`.
- Remote first-byte latency grows linearly with chain depth.
- Tooling-derived rename detection fails on duplicate-content trees.

## 12. Threats to validity

- The model is not a wire implementation (label MODEL). Byte sizes come from a record-size
  model; parser attack surface and implementation defects are not measured.
- Real series in the corpus may under-represent deletions and renames; generated series
  supplement them but their change distribution is designer-chosen.
- Python memory accounting for the model is not the MVT-06 counting allocator; the Rust port of
  the resolver core is used for binary HC-06 claims, the Python model for the rest.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12. Because the roles are MODEL-only, held-out results are report-only unless a
wire implementation exists at the freeze.

## 14. Cost estimates

- Compute: packing about 60 versions across INT-SERIES (about 1.3 GiB logical) and the model
  campaign: about 25 CPU-hours; hostile series about 5 CPU-hours.
- Disk: about 20 GB of Complete version archives and model incrementals.
- Agent effort: model build requires judgment (record-size model, resolver rules) with review;
  execution none; analysis judgment.

## 15. Deviations (append-only)

None.
