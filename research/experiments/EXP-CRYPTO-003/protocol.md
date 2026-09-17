# EXP-CRYPTO-003: Keyed-boundary CPU cost, boundary stability, dedup and keyed coincidence (HC-14(e)) on x86-64

<!-- BEGIN AUTO:meta -->
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
<!-- END AUTO:common -->
