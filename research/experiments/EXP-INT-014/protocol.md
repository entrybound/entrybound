# EXP-INT-014: Writer-side verification scope and reconstructive-transform differential decoding

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T11(b) encoder-output fault hook; independent decoder builds: C++ preflate reference, libjxl `djxl`; vector extractor; cross-compiled aarch64 `ebound` and `qemu-user-static` for the emulated arm) |
| Domain | integrity (program §24, cross-reference reconstruction §8.8 and conformance §28; objective HC-02, HC-11, HC-12, HC-16, OD-06) |
| Decisions informed | DEC-CMP-044, DEC-INT-030 |
| Gates, method, author | conventions C0; the arm A timing sub-arm also needs G-B timing gates |
| Specs | `spec.yaml` (arm A deterministic fault detection and arm B differential decoding); arm A timing is a sub-spec generated for an EXP-INT-004 quiet window when tooling exists |

## 1. Question

(A) Which writer verification scope catches injected encoder and transform faults before an
archive is published, and at what pack-time cost per profile? (B) Do independent decoders
reproduce byte-identical originals from the side data of registered reconstructive transforms
(DEFLATE reconstruction and JPEG reconstruction) across builds and architectures, so that
registered test vectors or differential testing can stand in for the writer's own-decoder round
trip?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-CMP-044 | Under the status quo (reconstructive dual check; structural steps length-checked), an injected bit flip in a codec encoder's output is not caught at pack time for non-reconstructive plans, so pack publishes an archive that later fails `verify` (detected at read time, HC-02 at write time violated); `verify-all-chunks` and `post-write-full-verify` catch 100%; `sampled-verification` at rate p catches a fraction near p per faulty chunk. | Status quo catches encoder output faults at pack time. |
| H2 | DEC-CMP-044 | `post-write-full-verify` costs more than 1 band unit of T-03 on the balanced profile (the tight-band profile) but is within the extreme profile's band. | Cost within T-03 band on balanced. |
| H3 | DEC-INT-030 | Independent decoders reproduce the production reconstruction outputs for every JPEG reconstruction side datum (JPEG XL reconstruction is specified in ISO/IEC 18181) but not for every DEFLATE reconstruction side datum (preflate-class side data is defined by an implementation, per objective HC-11 status), giving `reader_disagreements` > 0 for DEFLATE reconstruction across independent implementations or versions. | Zero disagreements for both. |
| H4 | DEC-INT-030 | Production outputs are identical across Windows x86_64, WSL x86_64 and emulated aarch64 for every vector (HC-11). | Any cross-build difference (HC-11 violation artifact). |

## 3. Candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-CMP-044 | `status-quo-reconstructive-only`, `verify-all-transformed`, `verify-all-chunks`, `sampled-verification` (p = 0.01 and 0.10, seeded deterministic sample), `post-write-full-verify` | B-RES with the T11(b) hook: fault classes injected into encoder output for one seeded chunk per pack (bit flip in compressed frame, truncated frame, wrong dictionary id, frame decoding to plaintext with one byte changed, transform output permutation for structural transforms, corrupted reconstruction side data). Each candidate scope is implemented as a hook-controlled verification pass. Fault classes are chosen from the incident survey `survey/encoder-incidents.md` (primary sources: upstream bug trackers, CVE, RUSTSEC, GHSA for zstd, lz4, xz and LZMA SDK, brotli, flate2 and miniz_oxide, preflate, libjxl, jxl-oxide), committed before the run. |
| DEC-INT-030 | `status-quo-own-decoder`, `registered-test-vectors`, `ci-differential-independent`, `pinned-reference-decoder`, `extended-tier-only`, `trust-encoder-no-verify` | Vectors: every reconstruction region and side datum in archives packed from the arm B items is extracted (research-internals) with the expected original bytes digest. Independent decoding: C++ preflate reference implementation at a pinned commit (licence recorded; reference tool only) for DEFLATE reconstruction; libjxl `djxl` at a pinned release for JPEG reconstruction from JPEG XL bitstreams. Cross-build: B-PROD on Windows and WSL, cross-compiled aarch64 under `qemu-aarch64-static` (EMULATED; correctness only, §4.1). Proposed L0 exclusion of `trust-encoder-no-verify` (HC-02 and I31 write-time verification); negative control via the hook (count of wrong archives published). Real-world prevalence and savings are the reconstruction domain's (§8.8), cited. |

## 4. Corpus

- Arm A: INT-CORE-T under fast, balanced, dense and extreme (every codec and transform path the
  planner selects is exercised; paths not selected on any item are listed as uncovered). Validation:
  INT-CORE-V.
- Arm B tuning: `f12-tuning-kodak-jpeg-edge-cases-small`, `f12-tuning-kodak-jpeg-matrix-medium`,
  `f12-tuning-nasa-ivl-photos-small`, `f13-tuning-kodak-image-codecs-small`,
  `f11-pypi-binary-wheels-cp312`, `f11-alpine-324-apks`, `f14-apache-maven-3916-bin-tarball`,
  `f14-jenkins-25683-war`. Validation: `f12-validation-fsa-jpeg-edge-cases-small`,
  `f12-validation-commons-cc0-photos-small`, `f13-validation-fsa-image-codecs-small`,
  `f11-maven-central-jvm-jars`, `f11-cpython-3137-release-set`, `f14-gen-office-documents`.

## 5. Environment and platform requirements

`wsl-ubuntu` (arm A, arm B), `windows-host` (arm B cross-build), emulated aarch64 via
`qemu-user-static` (apt install under the approved download policy; Docker not required).
Native ARM64 is PLATFORM_BLOCKED (EXP-INT-019). `timing: false` except the arm A timing sub-spec.

## 6. Procedure and commands

1. Commit `survey/encoder-incidents.md`; build independent decoders and record versions.
2. `ebr run` of `spec.yaml` (candidates `arm-a-<scope>` and `arm-b-<decoder>`), `verify-raw`,
   `verify_detail.py`, `normalize`, `report`.
3. Arm B cross-build: the same vector set decoded on Windows and under qemu; comparison by
   `research/tools/integrity/vector_compare.py`.
4. Arm A timing sub-spec in an EXP-INT-004 quiet window (pack with each scope, no faults).

## 7. Seeds, warmups, replication

Seeds 2026091841/42/43; faulty chunk selection per C8. Warmups 0; 2 repetitions with repeat-hash.
Timing sub-spec per C10.

## 8. Metrics

| Metric | ID | OD | Role |
|---|---|---|---|
| `roundtrip_mismatches` (archives published by pack that fail round trip) | HC-02 | OD-02 floor | binary for candidates claiming write-time guarantees |
| `damage_detection_fraction` (fault classes caught at pack time) | C11 proposed | OD-02 or as assigned | secondary until named |
| `encode_wall_s` | T-03 | OD-06 | **primary** for DEC-CMP-044 cost (timing sub-spec) |
| `verification_cpu_share` | descriptive (M06.3) | OD-06 | descriptive |
| `reader_disagreements` | HC-03 | OD-04 floor | binary for DEC-INT-030 (production versus independent decoders, and across builds) |
| `unspecified_baseline_decode_steps` | HC-12 | OD-21 floor | binary census |
| vectors produced per transform | none | - | descriptive (C7 permanent-vector cost input) |

## 9. Normalization

Arm A detection: exact fractions per fault class and profile. Timing: paired log ratios against
the status quo scope within rounds. Arm B: exact disagreement counts.

## 10. Statistical analysis and sensitivity

Exact tallies for binaries; timing per C10. Sensitivity: profile arm (fast, balanced, dense,
extreme reported separately; T-03 band depends on profile), fault-class weighting arm (uniform
versus incident-frequency weights from the survey when numeric), decoder-version arm (two libjxl
releases).

## 11. Expected negative results worth recording

- Status quo publishes archives with encoder-output faults that only read-time verification detects.
- DEFLATE reconstruction side data cannot be decoded identically by an independent implementation
  (implementation-defined format), supporting the gated T4 classification.
- Full post-write verification is too costly for the fast profile's band.

## 12. Threats to validity

- Injected faults are synthetic; real encoder bugs may be data-dependent and rarer.
- The C++ preflate reference may differ in version from production's port; version mismatch is
  itself evidence for HC-11 concerns and is recorded, not treated as noise.
- Emulated aarch64 checks correctness only; native ARM64 decoders may use different SIMD paths.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12.

## 14. Cost estimates

- Arm A: 4 profiles x 21 items x about 6 fault classes x 5 scopes x 2 repetitions: about 40
  CPU-hours; timing sub-spec about 8 hours of exclusive wall time.
- Arm B: vector extraction and independent decoding about 15 CPU-hours; emulated decoding about
  30 CPU-hours.
- Disk: about 25 GB.
- Agent effort: hook and decoder builds with review (judgment); survey judgment; execution none.

## 15. Deviations (append-only)

None.
