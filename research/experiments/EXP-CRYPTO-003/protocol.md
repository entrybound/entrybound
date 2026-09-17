# EXP-CRYPTO-003: Keyed-boundary CPU cost, boundary stability, dedup and keyed coincidence (HC-14(e)) on x86-64

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** |
| Kind | decision |
| Domain | crypto (program §22.2) |
| Decisions informed | DEC-CRY-023, DEC-CRY-005, DEC-CMP-001, DEC-MOD-039 |
| Platforms | Windows+Linux/x86-64; requires timing calibration + runner additions |
| Requires timing | True |
| Estimates | 220 machine-hours; 85 GB disk |
| Tooling to build | ebr-chunk --key-seed; ebr-crypto chunk; counting allocator; runner additions (method §14 item 3) |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns look 1 for DEC-CRY-005, DEC-CRY-023; participates in looks owned by other experiments for DEC-MOD-039 |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
<!-- END AUTO:meta -->

## 1. Question

On the reference x86-64 host (Windows and WSL), what does each keyed-boundary construction cost relative to unkeyed `gear-norm-v1`? The costs measured are chunking throughput, end-to-end encrypted pack time, dedup and stored bytes, chunk-size distribution, and boundary resynchronization after edits. Do the keyed constructions satisfy the HC-14 keyed-boundary test (MVT-14(e))?

## 2. Hypotheses and falsification

- **H1 (cost).** `phte-aes128-norm-v1` chunking throughput is `DISTINGUISHABLE_WORSE` than `gear-norm-secret-table-v1` by at least 3 band units on `encode_throughput_bps` (chunking stage, diagnostic). End-to-end `encode_wall_s` of `ebound pack --chunk-boundary keyed-prf` is `DISTINGUISHABLE_WORSE` than the default on the `fast` and `balanced` profiles and `EQUIVALENT` on `extreme`, where codec time dominates. The truth of the published 53%-165% chunking overhead (Truong et al., as cited in `docs/crypto-suite-v1.md`) is tested here, not assumed.
- **H2 (software AES).** A software-AES build (`--cfg aes_force_soft`, verified from the `aes` =0.9.3 crate documentation before use) increases the PHTE overhead by at least another 2 band units.
- **H3 (stability and dedup).** At matched realized `mean_chunk_bytes` (harness use constraint 4), secret Gear and PHTE are `EQUIVALENT` to `gear-norm-v1` on `artifact_bytes` and `dedup_stored_fraction` for F17 and F18, and on resynchronization distance after edits. Fixed-size chunking is `DISTINGUISHABLE_WORSE` on `artifact_bytes` for F18 edit series.
- **H4 (HC-14(e)).** Neither keyed mode shows identical boundary sets under two distinct keys. For each keyed mode, no more than 1 key in 1,000 exceeds the 0.999 quantile of the item's distinct-key null distribution for keyed-versus-unkeyed coincidence.
- **Falsification.** H1 is falsified if PHTE is `NON_INFERIOR` to secret Gear on end-to-end `encode_wall_s` for `balanced`. H3 is falsified if either keyed mode is `DISTINGUISHABLE_WORSE` on `artifact_bytes` at matched means. H4 failing is an R1 violation, recorded with the violating key pair as the artifact.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
#### DEC-CRY-023 — Which chunk-boundary construction is the default for encrypted archives (and per profile or threat setting), given secret Gear tables are known-breakable and PHTE/AES costs more?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CHUNK-006, EXP-CHUNK-008, EXP-CHUNK-012, EXP-CHUNK-013, EXP-CRYPTO-002, EXP-CRYPTO-004, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-secret-gear-default-phte-opt-in` | Status quo: SECRET_GEAR_TABLE default (defense in depth), PHTE_AES128 via --chunk-boundary keyed-prf | status quo | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `phte-default-everywhere` | PHTE_AES128 default for all encrypted archives; secret Gear retained only for reading/compatibility | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `phte-default-for-selected-profiles` | PHTE default for remote/storage-provider oriented or Dense/Extreme profiles; secret Gear elsewhere | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `widened-keyed-rolling-hash` | Hardened secret table (wider state, e.g. 256-bit buzhash as recommended to Borg) with mandatory compression; new boundary mode id | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `fixed-size-chunks-under-encryption` | Fixed-size chunking for encrypted archives (no content-defined boundaries), losing shift-resistant dedup | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `keyed-cdc-with-coarse-rechunking` | Keyed CDC followed by packing chunks into fixed-size padded containers so boundaries are not observable | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `public-unkeyed-cdc` | Public unkeyed gear-norm-v1 boundaries under encryption | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |

**Ledger exclusions:** {"candidate_id": "public-unkeyed-cdc", "reason": "Encrypted boundary modes are keyed by frozen registry; unkeyed boundaries directly expose plaintext-derived structure", "invariant_violated": "I24; docs/crypto-wire-v1.md L56-57"}.

**R0 checklist:** 1 status quo: `status-quo-secret-gear-default-phte-opt-in`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: `widened-keyed-rolling-hash`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-005 — Should EncryptedBoundaryKey (Gear table, PHTE polynomial, AES key) implement ZeroizeOnDrop and avoid Clone, and what lifetime do boundary secrets have during planning?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-018.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-clone-no-zeroize` | Status quo: derive(Clone, Eq, PartialEq) without Zeroize | status quo | no | EXP-CRYPTO-018 |
| `zeroize-on-drop-no-clone` | ZeroizeOnDrop, remove Clone, pass by reference | ledger alternative | no | EXP-CRYPTO-018 |
| `secrecy-wrapper` | Wrap secrets in a secrecy-style type with explicit expose | ledger alternative | no | EXP-CRYPTO-018 |
| `derive-on-demand` | Derive boundary material inside the chunker and drop immediately | ledger alternative | no | EXP-CRYPTO-018 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-clone-no-zeroize`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CMP-001 — Which CDC algorithm, normalization level and boundary-mask construction should the v1 chunker and its keyed variants use: keep frozen gear-norm-v1 (Gear hash, contiguous low-bit masks, one-bit normalization, every byte hashed) or register a measured successor such as spread-mask NC-2 Gear, FastCDC-2020, Rabin, BuzHash, AE, RAM, SeqCDC, VectorCDC or Chonkers?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-003, EXP-CHUNK-004, EXP-CHUNK-006, EXP-CHUNK-008, EXP-CHUNK-009, EXP-CHUNK-012, EXP-CHUNK-013, EXP-CRYPTO-021, EXP-EVAL-012, EXP-SCALE-012, EXP-SCALE-014.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-gear-norm-v1` | Frozen gear-norm-v1: Gear table from splitmix64, hash reset per chunk, low b+1/b-1 bit masks (NC-1 equivalent), no cut before minimum, forced maximum | status quo | no | — |
| `gear-spread-mask-nc2` | Gear with FastCDC-2020 spread (high-bit) masks at NC-2, same min/max semantics, new chunker_id | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-006 |
| `fastcdc-2016` | Original FastCDC (USENIX ATC 2016): Gear hash with normalized chunking and sub-minimum cut-point skipping, one byte per step; the Phase B2 harness plan lists FastCDC-2016 separately from 2020 | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002 |
| `fastcdc-2020` | Full FastCDC 2020: normalized spread masks, hash skipping before minimum, rolling-two-bytes optimization with identical boundaries | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002 |
| `tttd` | Two Thresholds Two Divisors (Eshghi and Tang 2005): Rabin-based chunking with a backup divisor; the classic literature baseline for chunk-size variance | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-008 |
| `rabin-fingerprint` | Rabin polynomial rolling hash over a 48-64 byte window (restic-style), optionally per-archive polynomial | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-006 |
| `buzhash` | Cyclic-polynomial BuzHash rolling hash (casync, Borg 1.x buzhash, kopia) | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002 |
| `ae-extremum` | Asymmetric Extremum hashless chunking with corrected h~mu-256 parameterization (lowest size variance) | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-006 |
| `ram-hashless` | Rapid Asymmetric Maximum hashless chunking (high throughput; pathological on low-entropy data) | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-006 |
| `seqcdc` | SeqCDC hashless monotonic byte-run chunking | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-008 |
| `vectorcdc-simd` | VectorCDC/VRAM SIMD hashless chunker with a scalar reference and cross-ISA boundary identity | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-008, EXP-CHUNK-013 |
| `chonkers` | Chonkers CDC with provable chunk-size and edit-locality bounds (unoptimised reference today) | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-008 |
| `quickcdc-feature-acceptance` | QuickCDC/RapidCDC-style fast-forwarding that accepts memoized lengths on partial feature-vector matches | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-003, EXP-CHUNK-004, EXP-CHUNK-008, EXP-CHUNK-013 |
| `fixed-size-blocks` | Fixed-size blocks (Duplicati, MSIX, Nydus) as measurement baseline | ledger alternative | no | EXP-CHUNK-002 |

**Ledger exclusions:** {"candidate_id": "quickcdc-feature-acceptance", "reason": "Accepts a chunk boundary on a partial feature match without content-defined scanning; SPEC excludes this class regardless of throughput as a silent-corruption risk", "invariant_violated": "SPEC §7.3 L803 / §25.1 row 6 boundary-skipping exclusion"}; {"candidate_id": "fixed-size-blocks", "reason": "Frozen SPEC rejection of fixed blocking for new archives; retained only as a measurement baseline", "invariant_violated": "SPEC §7.2 L792"}.

**R0 checklist:** 1 status quo: `status-quo-gear-norm-v1`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 14 ledger candidates listed above; 4 strongest incumbent: `rabin-fingerprint`, `buzhash`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-gear-norm-v1`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-MOD-039 — Can one archive contain multiple chunker algorithms or parameter sets (per category, per profile), how must PCR bind them, and is profile- or key-dependent PCR acceptable to users comparing physical arrangement?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CHUNK-002, EXP-CHUNK-004, EXP-CHUNK-005, EXP-CHUNK-011, EXP-CHUNK-012, EXP-PLANNER-001, EXP-PLANNER-005, EXP-PLANNER-012.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `single-chunker-id-status-quo` | Status quo: one chunker_id string per archive; profiles choose different CDC; AFK-keyed boundaries change PCR | status quo | no | EXP-CHUNK-012 |
| `per-object-chunker-id` | Bind chunker id per ContentObject in the PCR list | ledger alternative | no | EXP-CHUNK-012 |
| `chunker-set-in-descriptor` | Bind the set of chunker ids used, with per-object references | ledger alternative | no | EXP-CHUNK-012 |
| `uniform-chunking-across-profiles` | Use one CDC policy for all profiles so PCR is profile-independent | ledger alternative | no | EXP-CHUNK-005, EXP-CHUNK-012, EXP-PLANNER-005 |
| `pcr-excludes-chunker-id` | Bind only chunk roots and counts (chunk boundaries already determine roots) | ledger alternative | no | EXP-CHUNK-012 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `single-chunker-id-status-quo`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED`. Timing carries `VIRTUALIZED` on WSL (soft cap for Linux-native claims). HC-14(e) is a binary HC test (R1). Keyed-variant feasibility of non-Gear CDC algorithms is partly `FORMALLY_DERIVED` (whether a key slot exists) and partly measured.

## 4. Candidates and arms

| Arm | Chunker | Source |
|---|---|---|
| U0 (reference) | `gear-norm-v1` (unkeyed; status quo for plaintext) | `chunk_ranges` |
| K1 | `gear-norm-secret-table-v1` (status-quo encrypted default) | `chunk_ranges_encrypted` + `SecretGearTable` |
| K2 | `phte-aes128-norm-v1` (hardware AES autodetected) | `chunk_ranges_encrypted` + `PhteAes128` |
| K2s | `phte-aes128-norm-v1` built with software AES | same code, separate harness build |
| K3 | research `buzhash256-keyed-v0` (widened keyed rolling hash) | `ebr-crypto` |
| K4 | fixed-size at the profile target | `ebr-chunk --algorithm fixed-size` (existing) |
| KV-* | keyed variants of the harness CDC algorithms (FastCDC-2016/2020 with secret Gear table, Rabin with secret polynomial, BuzHash with secret table; AE, RAM and SeqCDC flagged "no key slot") | `ebr-chunk` extended with `--key-seed` |
| Z1 | `EncryptedBoundaryKey` with ZeroizeOnDrop and without Clone (DEC-CRY-005 candidate) | research patch of `crates/entrybound` in `research/harness/wip/zeroize-boundary-key.patch`, applied only in a separate worktree export |

End-to-end arms (production `ebound`, built from the production workspace per harness use constraint 2):
- `ebound pack <tree> <out> --recipient <r.pub>` (default boundary);
- `--chunk-boundary keyed-prf`;
- unencrypted `ebound pack` as the OD-06 reference.

Each runs at `--pad bucketed` and every profile.

## 5. Corpus selectors

- **Timing (tuning):** at least 10 families and all three tiers, following the calibration coverage of §4.7:
  - `f01-tuning-zstd-v1-5-7-git`, `f01-tuning-typescript-5-9-3-git`, `f01-tuning-llvm-project-20-1-8-git`;
  - `f03-tuning-ripgrep-cargo-vendor`, `f04-tuning-sourcelike`, `f04-tuning-maildir`;
  - `f05-tuning-enwik8`, `f05-tuning-loghub-hdfs-v1`, `f06-tuning-nyc-tlc-yellow-tripdata-2024`;
  - `f07-tuning-pythia-160m-safetensors-main`, `f09-ubuntu-noble-usr-binaries`, `f10-linux-firmware-20260910-subset`;
  - `f11-tuning-ubuntu-noble-debs`, `f12-tuning-nasa-ivl-photos-medium`, `f13-tuning-bbb-video-medium`;
  - `f16-alpine-3241-minirootfs-ext4-raw`, `f17-tuning-real-ubuntu-kernel-headers-2ver`, `f18-tuning-curl-8-x-release-series`, `f20-tuning-csprng`.
- **Dedup and stability (tuning, deterministic):** every F03, F17 and F18 tuning item, plus synthetic edits (insert, delete, replace of 1, 64 and 4,096 bytes at 100 seeded offsets) applied to `f05-tuning-enwik8` and `f09-silesia-mozilla-ooffice`.
- **MVT-14(e):** every tuning item of at least 1 MiB (the test's own definition), for K1 and K2.
- **Validation:** matching families from the validation split for the confirmatory look (registered), meeting §4.9 minimum support.

## 6. Environment and platform requirements

- `windows-host`: affinity `st-p` for chunking throughput; end-to-end `ebound pack` on `st-p` and `mt-p-4`.
- `wsl-ubuntu`: `st-v` and `mt-v-4`, ext4 inputs under `/root/eb-research`, never `/mnt/*`.
- Both need calibration experiments `EXP-CAL-AA-<env>`, `EXP-CAL-AAP-<env>`, `EXP-CAL-KE-<env>`, `EXP-CAL-BG-<env>` passing for `time_wall`, `time_cpu` and `throughput` (§4.7), and the runner additions of §14 item 3.
- Deterministic parts (dedup, stability, MVT-14(e)) run in `wsl-ubuntu` with `timing: false`, outside timing windows.
- Native ARM64 is out of scope here (EXP-CRYPTO-004, BLOCKED).

## 7. Commands and tooling

Tooling to build:
- `ebr-crypto chunk` (throughput with in-process timers; boundary lists; stability and dedup accounting; `--mvt14e` loop over 1,000 seeded distinct-key pairs, computing the null distribution per item and emitting violations with the offending key seeds);
- `ebr-chunk --key-seed` for KV-* variants;
- research-internals accessor `boundary_key_for_afk` (as in EXP-CRYPTO-002), so MVT-14(e) keys go through the production HKDF derivation;
- `ebr-crypto keygen` (X-Wing test recipients for `ebound pack --recipient`);
- the software-AES harness build profile in `research/harness/build.sh` (`RUSTFLAGS='--cfg aes_force_soft'`, target dir `/root/eb-research/target/harness-softaes`).

Specs:
- `spec.yaml`: WSL chunking and end-to-end timing, `timing: true`.
- `spec-windows.yaml` (experiment id `EXP-CRYPTO-003.windows`): the same on `windows-host`.
- `spec-mvt14e.yaml` (experiment id `EXP-CRYPTO-003.mvt14e`): deterministic HC-14(e) and dedup/stability, `timing: false`.

Commands follow the harness README (`validate`, `run`, `verify-raw`, `normalize`, `report`), with `lock_parity.py` and `harness-cli-parity.sh` passing first.

## 8. Seeds, warmup, repetitions and adaptive rule

- Timing:
  - warmups 2 (small and medium) and 1 (large);
  - rounds per §4.6: minimum 10, step 10 up to cap 30 (small), step 10 up to cap 20 (medium), step 5 up to cap 20 (large), and never below n_min(c);
  - adaptive precision rule of §4.6, on item-level exact order-statistic half-widths;
  - `order: randomized_blocks`, `pairing: item_repetition`, cooldown and drift canary per §4.2 and §4.3.
- Deterministic: 2 repetitions plus repeat-hash of boundary lists.
- MVT-14(e): 1,000 distinct-key pairs per item per mode, with key seeds `sha256(seeds.command || item_id || mode || i)`.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role | Measurement |
|---|---|---|---|---|
| `encode_wall_s` | T-03 | OD-06 | **primary** (end-to-end `ebound pack`, process timer; small tier uses the in-process timer, per T-03) | runner `resource` / harness in-process |
| `encode_cpu_s` | T-05 | OD-06 | secondary | runner `resource` |
| `encode_throughput_bps` | T-05b | OD-06 | diagnostic (chunking stage only, in-process) | `ebr-crypto chunk` |
| `artifact_bytes` | T-01 | OD-05 | **primary** (encrypted packs at matched realized means) | `path_size` |
| `dedup_stored_fraction` | T-01 | OD-05 | diagnostic | harness accounting |
| `alloc_peak_bytes` | T-06 | OD-08 | secondary (planning peak with boundary key) | counting allocator (gate §14 item 17) |
| `decode_wall_s` | T-04 | OD-07 | guard (decoders never run the chunker, I30; expected `EQUIVALENT`) | `ebound unpack` |
| resynchronization distance, chunk-size mean, sigma, forced-max fraction | descriptive | — | reported | `ebr-crypto chunk` |
| MVT-14(e) violations | binary (HC-14) | — | R1 screen | `--mvt14e` |
| PCR chunk-root overlap between two keys for identical content | descriptive (DEC-MOD-012, DEC-MOD-039) | — | reported | derived from boundary lists |

## 10. Normalization and statistical analysis

- **Timing (§4.9-§4.12):**
  - L1 is the median of per-round paired log ratios, with the exact order-statistic interval;
  - L2 to L4 as §4.9, with Welch-Satterthwaite at corpus level;
  - family harm guard at 0.90;
  - confirmatory W against S on validation with k_sup-adjusted confidence;
  - MDE gate at most 1 band unit before the look.
- **Environment claims:** Windows and WSL need same-direction support (§4.1).
- **Speedup comparisons:** none here.
- **Size:** exact values with the repeat-hash check, item-level exact comparison against the T-01 band and floor.
- **MVT-14(e):** exact per item, no statistics beyond the pre-registered 0.999 quantile and the 1-in-1,000 rule.
- **Keyed-variant feasibility table:** formal (key slot present or absent, attack status from EXP-CRYPTO-001 and 002). No verdicts.

## 11. Practical significance

`encode_wall_s` and `encode_cpu_s` use the per-profile bands of T-03 and T-05, `encode_throughput_bps` T-05b, `artifact_bytes` T-01 (small tier relative only), `decode_wall_s` T-04 and `alloc_peak_bytes` T-06, all from `research/methods/thresholds.json`. Minimum-gain multipliers apply by computed tier (§7.2, §7.3).

## 12. Sensitivity analysis

§6 arms on validation, including the environment arm (Windows versus WSL) and tier-stratified results. Experiment-specific:
- AES backend (hardware versus software);
- realized-mean matching tolerance (±2% versus ±5%);
- edit size classes.

## 13. Expected negative results worth recording

- PHTE end-to-end overhead is within the band on `dense` and `extreme` profiles. That would weaken the cost argument for keeping PHTE opt-in there, which feeds the R9 profile partition of DEC-CRY-023.
- Secret-Gear planning shows a measurable cost from table derivation (256 HMAC calls) on tiny archives (small tier, in-process).
- Zeroization (Z1) is `EQUIVALENT` on every timing metric. That removes the performance objection to DEC-CRY-005's candidate.
- A keyed variant of FastCDC-2020 (spread masks) is cheaper than K1 with identical stability. That candidate would then need a new chunker id (T1 planner cost plus T4 if considered a new crypto construction).

## 14. Threats to validity

- Hybrid-core scheduling and WSL virtualization (§4.1-§4.3).
- Defender scanning on Windows outputs (`defender_on` labels).
- Harness build inlining differs from `ebound`. Mitigated because the decision-grade end-to-end timing uses production `ebound` (use constraint 2) and in-process chunking timing is diagnostic only.
- AES-NI availability: the host has it. No-AES hosts are represented only by the software-AES build, not by real hardware (EXP-CRYPTO-004).
- Matched-mean tuning of research chunkers could be over-fit to tuning items. Matching parameters are frozen before validation.

## 15. Held-out placeholder (Phase D)

Every candidate reaching Commit A runs once on held-out with frozen builds and affinity configurations under `EB_HELDOUT_UNLOCK`. The held-out MDE (at most 2 band units) is computed from validation variance and held-out group counts. MVT-14(e) also runs on held-out items of at least 1 MiB (the R1 check on every split). No held-out item is named here.

## 16. Cost estimate and agent effort

- Compute: about 120 machine-hours of exclusive timing windows (both hosts), plus about 100 core-hours for MVT-14(e) and dedup/stability. Disk: about 80 GB transient and 5 GB retained.
- Agent effort: tooling **scripted** (mostly mechanical harness work) plus **judgment** for research chunkers; execution **scripted** (overnight windows); analysis **judgment**.

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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-003; edit the inputs, not this block._
<!-- END AUTO:common -->
