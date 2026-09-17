# EXP-CHUNK-007: Deduplication scope: net benefit, break-even, windows and extraction cost

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CHUNK-007 |
| Status | NEEDS_TOOLING: `scope_model.py`; `ebr-chunk tree-dedup --scope concatenated*`; xattr census; `ebr-chunk dedup-index-probe` with the counting allocator (§14 item 17); EXP-CHUNK-004 SQ occurrence tables must exist first |
| Arms | **S** scope comparison (`spec.yaml`: modelled scopes + actual STREAM windows); **I** dedup index memory (`spec-index-memory.yaml`, generated); **X** single-entry extraction under dedup indirection (inside S, modelled request counts); **R** redundancy-stratified incumbent comparison (analysis of `EXP-BASE-SIZE` outputs); **V1** validation look; **H** Phase D placeholder |
| decision_ids | DEC-CMP-017 (dedup scope), DEC-CRY-025 (space cost of disabling dedup; the encrypted part is in EXP-CHUNK-006 arm N), DEC-ACC-029 (windowed/external dedup loss and dedup-index memory, chunking side only) |
| Proposed decision types | DEC-CMP-017 EMPIRICAL; DEC-ACC-029 EMPIRICAL (scale domain owns bounded-memory claims) |
| Proposed od_ids | DEC-CMP-017: OD-05, OD-08, OD-10, OD-12, OD-16. DEC-ACC-029: OD-05, OD-08, OD-09. |
| Proposed hc_ids | HC-06 (declared bounds: chunk-count limit, window declarations), HC-09 (bounded dependency chains under windows), HC-10, HC-14 (no cross-archive equality) |
| Gates / author / L7 | As EXP-CHUNK-001 section 1 |

## 2. Question

Which deduplication scope maximizes net bytes saved across realistic trees? Net bytes saved means unique bytes saved minus frame, Index and reference metadata. Secondary questions:

- Where does chunk-level dedup metadata cost more than it saves (break-even file size)?
- What do windowed and streamed scopes lose?
- What does dedup indirection cost single-entry extraction?
- How does the status quo compare with redundancy-aware incumbents?

## 3. Hypotheses

- **H-S1.** `scope-archive-wide` (status quo) is not `DISTINGUISHABLE_WORSE` than any other scope on `artifact_bytes` at corpus level. **Falsified if** some scope is `DISTINGUISHABLE_BETTER` against that scope's tier band and `NON_INFERIOR` on the other primary metrics.
- **H-S2.** `scope-whole-file-only` and `scope-per-contentobject` are `DISTINGUISHABLE_WORSE` than archive-wide on F17 and F18 (family harm test), and `EQUIVALENT` on F12 and F13. **Falsified if** they are not worse on F17/F18.
- **H-S3.** Break-even: the median item-level `break_even_file_bytes` (the file size below which chunk records cost more than dedup saves) is under 4 KiB. `scope-tiny-inline-4096` is `DISTINGUISHABLE_BETTER` than archive-wide on F04 and F19 against the T3 band (a new record type), non-inferior elsewhere. **Falsified if** not better against that band.
- **H-S4.** `scope-windowed-256m` is within the T-01 band of archive-wide on every family except F17. Actual STREAM `auto` equals the modelled 256 MiB window within the band. **Falsified if** any non-F17 family is outside the band.
- **H-S5.** `scope-concatenated-stream` stores fewer unique bytes than per-file chunking on F04 and F18 (renames, small files). After its modelled reference overhead it is not better than archive-wide against the T4 band (new layout structure). **Falsified if** better against T4.
- **H-X.** The p95 physical ranges per single-entry extraction under archive-wide dedup exceed 1 (fragmentation) on F17 and F18 only.
- **H-I (DEC-ACC-029).** Status-quo dedup-table memory per unique chunk is roughly linear. The `alloc_peak_bytes` at 4,000,000 unique chunks is under 1 GiB for the digest map alone, excluding plaintext. `[INFERRED; the status-quo Archive also holds plaintext, which the scale domain measures]`
- **H0.** All scopes are within their bands everywhere.
- **Binary model-validity gate.** `scope_model.py` applied with `scope=archive-wide` must reproduce the actual EXP-CHUNK-004 SQ `artifact_bytes` exactly (`model_validity_exact = true`) on every item. Applied to the STREAM windows, it must reproduce the actual `stream-actual-window-*` archive bytes within the T-01 band on every item. Items where either fails are excluded from modelled-scope conclusions and reported. If more than 20% fail, arm S modelled results are not decision-grade.

## 4. Decisions informed

| Decision | Contribution (ledger required evidence) | Elsewhere |
|---|---|---|
| DEC-CMP-017 | Net bytes saved per scope by file-size bucket on F01, F03, F04, F17, F18, F19 (and all families for aggregate); break-even file size; rename/move and version series for per-file vs concatenated chunking (arm S on F18 plus EXP-CHUNK-003 move arm); single-entry extraction cost with dedup indirection (arm X); dedup index memory at the 4,000,000-chunk limit (arm I); redundancy-stratified comparison against DwarFS, Borg, restic and zpaq (arm R) | Encrypted creation overhead (EXP-CHUNK-006 arms E and N); cross-archive equality (excluded) |
| DEC-CRY-025 | Plaintext space cost of `none` versus archive-wide (arm S); the encrypted counterpart is EXP-CHUNK-006 arm N | Leakage (crypto domain) |
| DEC-ACC-029 | "Ratio/dedup loss of windowed vs global planning": windowed dedup scopes and actual STREAM windows. External dedup-index design memory: arm I structures | Peak RSS vs input size under memory caps, streaming pack designs, two-pass source stability (scale domain; Docker `--memory` caps BLOCKED, cgroup limits in WSL used there) |

## 5. Candidates

| Candidate | Definition | Ledger id | Measurement | Preliminary tier |
|---|---|---|---|---|
| `scope-archive-wide` | SHA-256 exact chunk dedup over the archive | `exact-chunk-archive-wide` (**status quo; reference**) | actual (EXP-CHUNK-004 SQ) reproduced by the model | T0 |
| `scope-none` | Every chunk occurrence stored | `none-baseline` | modelled | **Excluded from selection** (frozen always-on exact dedup, SPEC §7.2 L784 / §5.2 L522; ledger exclusion); baseline only |
| `scope-whole-file-only` | Identical ContentObjects shared; no chunk sharing across distinct objects | `whole-file-only` | modelled | T1 |
| `scope-per-contentobject` | Chunk dedup only within one ContentObject | `exact-chunk-per-contentobject` | modelled | T1 |
| `scope-windowed-{64m,256m,1g}` | Chunk dedup only to an earlier occurrence within W logical bytes in manifest (emission) order, for INDEXED | `windowed-dedup` | modelled | T1 (if expressed by planner policy and the existing window declaration) |
| `stream-actual-window-{0,auto,64m,256m}` | Production `pack --layout stream --stream-window` (window units as production declares them; recorded) | `windowed-dedup` (STREAM instance; access decision DEC-ACC-054 owns defaults) | actual | T0/T1 |
| `scope-tiny-inline-{256,4096,65536}` | Objects ≤ threshold stored inline without separate chunk frame and Index record (no dedup for them) | `tiny-object-inline-or-aggregate` | modelled (per-object framing and Index savings from occurrence tables; inline record cost modelled at the frame header plus varint length) | T3 (new record type) |
| `scope-tiny-aggregate-65536` | Objects ≤ 64 KiB concatenated into ≤ 64 KiB aggregate chunks in manifest order, compressed with the profile's plans (`ebr-codec` over aggregates), no dedup below threshold | `tiny-object-inline-or-aggregate` | modelled with real compression of aggregates | T3/T4 |
| `scope-concatenated-stream` | CDC over the manifest-order concatenation of all file contents (`gear-norm-v1` balanced), references as (chunk, offset, length) triples (modelled at 40 B + 2 varints per reference); unique chunks compressed with the profile's plans | `concatenated-stream-chunking` (casync, Duplicacy pack-and-split) | modelled with real chunking and compression | T4 (new layout structure) |
| `scope-concatenated-stream-forced-file-cuts` | Same, with cuts forced at file boundaries | `concatenated-stream-chunking` (model variant) | modelled | T4 |
| `scope-large-xattr-as-contentobject-4096` | xattr/resource-fork values ≥ 4 KiB stored as deduplicated ContentObjects (F19 census of values from mounted images and trees) | `large-xattr-as-contentobject` | modelled | T2/T3 |
| `scope-local-seeding-potential` | Fraction of each F18 version's chunk bytes already present in the previous version's extracted tree (potential for casync `--seed`-style reuse; no cross-archive identity) | `local-seeding-post-v1` | potential only (not an archive size) | post-v1; add-later cost recorded (§7.1 C7) |

- **Excluded, not measured:** `convergent-cross-archive-dedup` (L0/R1: SPEC §7.2 L790; `docs/crypto-suite-v1.md` §Deduplication domain; I24; ledger exclusion).
- **Arm I structures** (dedup index for DEC-ACC-029 `external-dedup-index`):
  - status-quo `BTreeMap<Digest, Chunk metadata>`;
  - sorted `Vec` with binary search;
  - sharded open-addressing hash;
  - external sort with on-disk runs (spill).

  Each is measured at N ∈ {10^4, 10^5, 10^6, 4 × 10^6} unique digests.
- R0 item 6: critic candidate open.

## 6. Corpus

- **Arm S tuning:** all 112 tuning items (ids in `spec.yaml`). Targeted families F01, F03, F04, F17, F18 and F19 are analysed per family; all families enter the aggregate.
- **Arm S validation (V1):** all validation items for W and S of DEC-CMP-017, after the MDE gate, registered look.
- **Arm I:** generated digest streams (seeded SHA-256 of counters), no corpus.
- **Arm R:** `EXP-BASE-SIZE` normalized outputs for tuning items (existing baseline configs `borg`, `restic`, `dwarfs`, `zpaq`). Items are stratified by EXP-CHUNK-004 SQ `dedup_stored_fraction` tertiles.
- **xattr census:** F19 tuning items. The ext4/XFS/btrfs loop images are mounted read-only in WSL as root. Trees are read with `os.listxattr` without following links.
- **Held-out:** Phase D placeholder (§5.5; EXP-CHUNK-004 section 6).

## 7. Environment and platform

- WSL x86-64.
- **Privileged:** loop-mounting F19 filesystem images read-only (`mount -o ro,loop`), available as WSL root.
- Deterministic metrics. No network: request counts are modelled from range plans.
- The actual STREAM windows use the harness `ebr-pack` (SHA-256-identical to production per `harness-cli-parity.sh`).

## 8. Commands

```sh
# preconditions/builds as EXP-CHUNK-004; EXP-CHUNK-004 arm SQ must have produced occurrence tables
$PY -m ebr validate --spec research/experiments/EXP-CHUNK-007/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-CHUNK-007/spec.yaml --gzip
$PY -m ebr verify-raw --spec research/experiments/EXP-CHUNK-007/spec.yaml --refingerprint
$PY -m ebr normalize --spec research/experiments/EXP-CHUNK-007/spec.yaml
# runs as WSL root (read-only loop mounts of F19 images)
python3 research/experiments/EXP-CHUNK-007/xattr_census.py --items <F19 tuning ids> \
    --manifest research/corpus/manifest.json --out /root/eb-research/results/EXP-CHUNK-007/xattr-census.jsonl.gz
$PY research/experiments/EXP-CHUNK-007/make_index_memory_spec.py --out research/experiments/EXP-CHUNK-007/spec-index-memory.yaml
#   command: ebr-chunk dedup-index-probe --structure {structure} --entries {n} --seed 20260917 (counting allocator)
$PY research/experiments/EXP-CHUNK-007/incumbent_strata.py --base research/normalized/EXP-BASE-SIZE \
    --sq research/normalized/EXP-CHUNK-004 --out research/normalized/EXP-CHUNK-007/decision/arm-r.csv
$PY -m decide analyze --experiment EXP-CHUNK-007 --decisions DEC-CMP-017 --out research/normalized/EXP-CHUNK-007/decision/
```

## 9. Seeds

- **Spec seeds:** `{order: 2026091771, bootstrap: 2026091772, command: 2026091773}`.
- The model is deterministic. Arm I digests are seeded with 20260917.
- **Single-entry sampling:** all entries for items with ≤ 10,000 entries; otherwise 10,000 entries sampled with seed 20260917, candidate-independent.
- Warmups 0.

## 10. Metrics

| Metric | Canonical | OD | Role (DEC-CMP-017) | Threshold |
|---|---|---|---|---|
| `artifact_bytes` (actual or `MODELLED`; the `artifact_bytes_is_modelled` flag is carried into every table) | yes | OD-05 | **primary** | T-01 |
| `index_bytes`, `dedup_stored_fraction` | yes | OD-05 | diagnostic | T-02, T-01 |
| `metadata_bytes_per_entry` | yes | OD-05 | secondary | T-02b |
| `http_requests` (single-entry extraction, modelled from the physical range plan with the production default coalescing gap; `http_requests_single_entry_modelled_p95`) | yes | OD-12 | **primary** for OD-12 (modelled, §4.15; emulated corroboration in EXP-CHUNK-009) | T-12 |
| `access_amplification` whole-entry stratum | yes | OD-10 | **primary** (taken from EXP-CHUNK-004 for actual scopes; modelled here for others) | T-22 |
| `alloc_peak_bytes` (arm I, counting allocator, exact) | yes | OD-08 | **primary** for OD-08 (index component) | T-06 |
| `net_bytes_saved_vs_none`, `break_even_file_bytes`, `single_entry_ranges_p95` | no | — | secondary | none |
| `model_validity_exact` | no | — | binary model gate | — |
| `result_sha256` | no | — | repeat-hash | §3.3 |

OD-16 for DEC-CMP-017 (equality leakage within an archive under encryption) is measured by the crypto domain. This experiment records it as an unmeasured OD for its own analysis (R3 coverage gap), resolved by citation of that experiment.

## 11. Replication

2 repetitions and repeat-hash for all deterministic arms. Arm I is deterministic under the counting allocator (exact), so 2 repetitions apply. RSS is not used there. No adaptive rule.

## 12. Analysis

1. **Model gate first** (section 3).
2. **L1-L4** as EXP-CHUNK-004 section 12. Strata: scale tier; file-size bucket (≤ 4 KiB, ≤ 64 KiB, ≤ 1 MiB, ≤ 16 MiB, larger) for net savings.
3. **Break-even** per item: the largest bucket upper bound at which bucket-restricted chunk dedup has negative net savings. Reported per family (median, IQR).
4. **Tuning:** exploratory R4 → S and W for DEC-CMP-017 (universal scope; R9 partition by policy (STREAM/INDEXED, encrypted/plain) then family). MDE gate (§4.12). V1 on validation: W vs S, superiority at 1 − 0.01/k_sup, non-inferiority 0.99, harm guard 0.90, R6 minimum gain per computed tier, R10.
5. **Modelled evidence cap.** A selection resting on `MODELLED` `artifact_bytes` for a non-expressible scope cannot be `DECIDED` on that evidence alone. The winning scope must be implemented as a research writer and re-measured (actual bytes) before the freeze. This is recorded as a condition in the decision record.
6. **Arm R:** within each redundancy tertile, the ratio of SQ `artifact_bytes` to each incumbent (REF-INC), reported with capability notes: Borg and restic repositories include encryption and chunk-index structures; DwarFS is a mountable image; zpaq journaling. **Informational only;** feeds DEC-ECO-031 and the phase E report on workloads where Entrybound is not best.
7. **Arm I:** `alloc_peak_bytes` per unique digest, fitted as a linear slope with an exact order-statistic interval over the 4 N values (descriptive), plus the absolute value at 4,000,000.

## 13. Thresholds

T-01, T-02, T-02b, T-06, T-12, T-22 as transcribed in `research/methods/thresholds.json` `metric_thresholds`, plus `cost_tiers` multipliers. Not restated.

## 14. Sensitivity

- §6 in full for DEC-CMP-017.
- Window ordering: manifest order versus physical (digest) order.
- Reference-triple cost for concatenated chunking at 40 B ± 50%.
- Inline record cost at +0 B and +16 B.
- Aggregate size 16 KiB versus 64 KiB.
- `balanced` versus `dense` profile for the model.
- F17 generated duplicate trees in versus out (their duplication rates are generator parameters, not measured populations).

## 15. Expected negative results

- Archive-wide exact dedup wins or ties everywhere.
- Tiny-object inlining saves bytes on F04 and F19 but not enough for a T3 band.
- Concatenated chunking's unique-byte advantage on renames is erased by reference overhead.
- STREAM `auto` windows equal archive-wide on every family except F17.
- Dedup indirection fragments single-entry reads only on F17 and F18.
- Incumbents (zpaq, Borg) beat SQ on high-redundancy strata.

## 16. Threats to validity

- **Modelled scopes** ignore planner interactions (cohort dictionaries and groups depend on which chunks exist). Mitigation: the model gate plus the implementation condition in section 12 item 5.
- **Generated F17 trees:** duplication rates are generator choices. They are reported separately and excluded in a sensitivity arm.
- **Single-entry request counts are modelled** from range plans. The emulator lacks slow start (R1-14), so remote latency claims are EXP-CHUNK-009's.
- **xattr census on WSL-mounted images:** the SELinux/ACL values are real, but kernel support for some xattr namespaces in WSL may hide values. Unreadable namespaces are recorded as census gaps.
- **Arm R capability mismatch** (encryption, repository structure): informational only.

## 17. Estimates

| Arm | Compute |
|---|---|
| S | 15 modelled scopes × 112 items × 2 reps. Most are occurrence-table arithmetic (minutes per item); concatenated scopes re-chunk and compress (about 36 GiB each, 2 variants); aggregates compress small files. Actual STREAM windows are 4 balanced packs. About **40 CPU-hours** |
| I | < 1 CPU-hour |
| R | minutes |
| V1 | about **25 CPU-hours** |

- **Disk:** about 5 GB (xattr census, aggregate scratch).
- **Agent effort:** scripted; judgment for `scope_model.py` (careful accounting), the xattr census and analysis.

## 18. Tooling to build

1. **`research/experiments/EXP-CHUNK-007/scope_model.py`.**
   - **Input:** EXP-CHUNK-004 occurrence tables (chunk digest, object digest, offsets, stored frame bytes, header bytes, plan refs, group refs) plus byte accounting.
   - **Output:** per-scope `artifact_bytes`, section components, net savings, break-even, and single-entry range plans (coalescing gap from `RandomAccessPolicy::default`, read from production source and cited).
   - **Validation:** `model_validity_exact` against actual SQ archives; unit tests on synthetic tables.
2. **`ebr-chunk tree-dedup --scope concatenated|concatenated-forced-file-cuts`** (extends EXP-CHUNK-002's subcommand), emitting unique chunk digests for compression by `ebr-codec`.
3. **`ebr-codec` aggregate compression mode** (`object-matrix` over concatenated tiny objects with the profile's plan selection via research-internals).
4. **`xattr_census.py`** (read-only loop mounts; per-entry value sizes and SHA-256 digests; no values stored).
5. **`ebr-chunk dedup-index-probe`** plus the **counting global allocator** in `ebr-common` (§14 item 17; allocation-site attribution not required for this arm).
6. **`make_index_memory_spec.py`** and **`incumbent_strata.py`.**

## 19. Deviations

(append-only)
