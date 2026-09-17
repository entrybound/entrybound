# EXP-LEGACY-004: compressed-transport differential, integrity checks, detection ambiguity and decoder memory

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (generator `transport_cases.py` X1-X10; zstd 1.5.7 with legacy support, xz 5.8.x, bzip2 0.9.0-randomised fixture builder; `ebr-legacy-decmem` with counting allocator; stream decoder drivers; `ebound_import.py --format stream`) |
| Domain / program sections | legacy / §25 (with §26 transport classes, §31 decoder dependencies, §24 integrity) |
| decision_ids | DEC-LEG-007, DEC-LEG-037, DEC-LEG-069, DEC-LEG-072, DEC-LEG-074, DEC-LEG-075, DEC-LEG-106, DEC-LEG-107, DEC-INT-022 |
| Decision types (proposed) | EMPIRICAL (outcomes, allocator peaks) with FORMAL parts (RFC 1952, RFC 8878, XZ format 1.2.1, bzip2 and lzip format descriptions; declared-memory formulas) |
| OD / HC (proposed) | OD-19, OD-17 (M17.1 tightness, M17.2 refusal cost, M17.4), OD-04; HC-06, HC-15, HC-07 |
| Shared rules / L7 / gates | C§ header, C§2 (allocator metrics are exact counts: deterministic rule §4.13; process RSS columns are informational until memory calibration) |
| Timing | `timing: false` |

## 1. Question

For each transport framing, integrity-check and filter variant, nested-transport and magic-collision construction:
which decoders accept it, what content they produce, whether corruption is detected, how Entrybound strict transport
import classifies it; and does each decoder's peak memory follow a formula computable from headers before decoding?

## 2. Hypotheses

- **H1 (unverified layers, DEC-INT-022).** For zstd frames without content checksum and `.xz` with check `None`, at
  least one seeded single-bit corruption in compressed literals decodes without error in every decoder, and the status
  quo imports it with no integrity-unverified marker. *Falsified by* every such corruption being detected or reported.
- **H2 (filter coverage, DEC-LEG-075).** `lzma-rust2` 0.20.0 (production) decodes a different filter set than
  xz 5.8.x for at least one of ARM64, RISC-V, SPARC, IA-64, and check SHA-256 handling matches xz. *Falsified by*
  identical support on all filters and checks.
- **H3 (header precheck, DEC-LEG-007).** For zstd, xz/LZMA2, LZMA-alone and bzip2, the per-library counting-allocator
  peak is bounded by `k_lib × header_declared_requirement + c_lib` with constants fitted on tuning and holding on
  validation (`declared_tightness_ratio` ≥ 1 on every fixture). *Falsified by* one validation fixture above its bound.
- **H4 (magic collision, DEC-LEG-072).** A tar whose first member name begins with `BZh91AY&SY` (valid tar checksum)
  is read as tar by libarchive and refused by the status-quo detection; Python `tarfile.open(mode="r:*")` differs from
  libarchive. *Falsified by* unanimous treatment.
- **H5 (ambiguous child, DEC-LEG-074).** gzip of 1 MiB of zeros is listed as an empty archive by libarchive, GNU tar
  and Python `tarfile` and as a 1 MiB file by standalone decoders, so projection is ambiguous. *Falsified by* all tar
  readers refusing it.
- **H6 (gzip padding, DEC-LEG-069).** Zero padding after the last gzip member is accepted by GNU tar, libarchive and
  Python `gzip`, so a padding-only tolerance matches every reference reader `[INFERRED]`.
- **H0.** No divergence; all corruption detected; memory formulas trivially tight.

## 3. Decisions informed

| Decision | Supplied here | Remaining elsewhere |
|---|---|---|
| DEC-LEG-007 | Measured decoder peaks per library/parameter; header-derivable bounds; outcome class and reason code of over-policy fixtures; `refusal_bytes_read` before refusal | Import-limit defaults EXP-LEGACY-041 |
| DEC-LEG-037 | Integrity semantics per extra transport (lzip, LZ4 frame, lzma-alone, zlib, `.Z`, brotli): detection of seeded corruption per decoder; pure-Rust decoder availability smoke | Prevalence EXP-LEGACY-010; class scoring and dependency audit EXP-LEGACY-021 |
| DEC-LEG-069 | gzip member padding and trailing data behaviour per reader | Other tar end-marker behaviour EXP-LEGACY-002 |
| DEC-LEG-072 | Collision fixtures (`BZh1`..`BZh9`, gzip, zstd, xz, `PK\3\4`, 7z magics as first tar names); nested transports depth 1-8 per reader | Prevalence EXP-LEGACY-010 |
| DEC-LEG-074 | Zero-payload fixtures; false tar-detection count over non-tar compressed tuning/validation items | Prevalence-weighted false detection EXP-LEGACY-010 |
| DEC-LEG-075 | Filter and check support per decoder including `lzma-rust2` 0.20.0; block-index observations available per decoder | Distribution filter/check survey EXP-LEGACY-010; dependency audit EXP-LEGACY-021 |
| DEC-LEG-106 | Dictionary-ID frames with and without the dictionary per decoder | Prevalence EXP-LEGACY-010 |
| DEC-LEG-107 | Skippable (all 16 magics), seekable-format index frames, legacy v0.5-0.7 frames, size-less frames per decoder | Prevalence EXP-LEGACY-010 |
| DEC-INT-022 | Checksum-less corruption fixtures; check types None/CRC32/CRC64/SHA-256 | ZIP CRC part EXP-LEGACY-001; prevalence EXP-LEGACY-010 |

## 4. Candidates

| Decision | Candidates (ledger ids) evaluated as rules over observations |
|---|---|
| DEC-LEG-007 | status-quo-integrity-mismatch; header-precheck-policy-refused; dedicated-reason-code; policy-out-of-evidence; measured-multipliers |
| DEC-LEG-037 | core-v1; post-v1-core; ecosystem; not-justified; import-lzip-lz4-only; import-and-export (integrity and decoder premises only) |
| DEC-LEG-069 | status-quo-strict-refuse; tolerate-padding-only; compat-profile-per-runtime; last-wins-with-evidence (gzip part) |
| DEC-LEG-072 | status-quo-single-layer; bounded-nesting; structural-from-override; tar-checksum-before-weak-magic; refuse-ambiguous-detection |
| DEC-LEG-074 | status-quo-heuristic; explicit-projection-flag; stricter-tar-detection |
| DEC-LEG-075 | status-quo-lzma-rust2-default; document-and-test-set; add-bcj-filters; preserve-block-index-observations; withdraw-spec-block-index; align-chunks-to-blocks |
| DEC-LEG-106 | status-quo-refuse; caller-supplied-dictionary |
| DEC-LEG-107 | status-quo; retain-skippable-content; seekable-format-aware; accept-legacy-frames; refuse-unrecognised-skippable |
| DEC-INT-022 | status-quo-present-checks-only; strict-require-checksums; report-unverified-layers; refuse-xz-check-none; crc-plus-size-invariant |

Candidates needing Entrybound behaviour changes (`header-precheck-policy-refused`, `stricter-tar-detection`,
`retain-skippable-content`, `report-unverified-layers`) are evaluated as rules over observations here; if the rule
ranking is close, a research-only prototype in `entrybound::research::legacy::transport` (C§3) is measured in a
follow-up run under a new experiment id.

## 5. Corpus selectors

Generated (`research/tools/legacy/cases/transport_cases.py`; F20; `S_T = 2509170401`, `S_V = 2509170402`):

| Set | Content | Approx. cases |
|---|---|---|
| X1 | zstd: skippable frames (16 magics, payload 0 B..1 MiB, before/between/after), seekable-format index (zstd `contrib/seekable_format`), legacy v0.5-v0.7 frames, no content size, checksum off with seeded literal corruption, dictionary-ID frames (trained dictionary present/absent), windowLog 10..31, multi-frame, empty frames | 60 |
| X2 | xz: checks None/CRC32/CRC64/SHA-256/reserved ids; filters LZMA2, delta, BCJ x86/PowerPC/IA-64/ARM/ARM-Thumb/ARM64/SPARC/RISC-V; multi-block with index; dictionary up to 1536 MiB; stream padding; concatenated streams; corrupted index | 50 |
| X3 | gzip: multi-member, zero padding after members, FEXTRA/FNAME/FCOMMENT/FHCRC, trailing garbage, ISIZE and CRC mismatch, reserved flags, BGZF, `pigz -i` | 25 |
| X4 | bzip2: multi-stream, block sizes 1-9, randomised blocks, stream CRC mismatch, truncation | 16 |
| X5 | Magic collisions: tar with first member names starting `BZh1`..`BZh9`, `\x1f\x8b`, `\x28\xb5\x2f\xfd`, `\xfd7zXZ\0`, `PK\3\4`, `7z\xbc\xaf\x27\x1c` | 14 |
| X6 | Nested transports depth 1-8: gzip(xz(tar)), zstd(gzip(tar)), tar.gz inside zstd | 16 |
| X7 | Ambiguous children: gzip of 1 MiB zeros, zstd of 10240 zero bytes, gzip of an empty tar (1024 zero bytes), gzip of 512 zero bytes | 8 |
| X9 | Extra transports with seeded corruption: lzip (members, trailing data), LZ4 frame (content/block checksum on/off, legacy frame, skippable), lzma-alone (known/unknown size), zlib, Unix compress `.Z`, brotli | 40 |
| X10 | Memory-policy fixtures: zstd windowLog 10..31, xz dict 64 KiB..1536 MiB, LZMA-alone dict, bzip2 1-9 and `-s` | 40 |

X8 (non-tar compressed payloads for false tar detection), tuning/validation corpus items only:
`f05-tuning-logrotate-from-loghub-hdfs`, `f11-alpine-324-apks` (concatenated gzip), `f11-ubuntu-noble-debs` (inner
compressed members) — tuning; `f05-validation-logrotate-from-loghub-ssh`, `f11-fedora-44-rpms` (zstd cpio payloads),
`f16-oci-fedora-44-zstd-layers` (zstd tar layers) — validation. Held-out: placeholder (C§5).

## 6. Environment

`wsl-ubuntu`; Windows `tar.exe` via interop for the libarchive 3.8.8 column. No timing. Allocator metrics are exact
(deterministic, §4.13). CLI RSS through GNU time is recorded as informational (R1-16; not decision-grade before
`EXP-CAL-*-wsl-ubuntu` memory calibration).

## 7. Commands and tooling

Tooling: `cases/transport_cases.py`; decoder drivers `readers/stream_cli.py` (zstd, xz, gzip, pigz, bzip2, lbzip2,
lzip, lz4, brotli, ncompress, GNU tar auto-detection, bsdtar), `readers/py_stream.py` (gzip, bz2, lzma, zstd on 3.14),
`readers/goread` (compress/gzip, compress/bzip2, klauspost zstd), `ebr-legacy-readers` (production crates flate2 with
miniz_oxide, zstd, lzma-rust2 0.20.0, bzip2; plus ruzstd, lz4_flex, brotli, zlib-rs for comparison);
`ebr-legacy-decmem --lib <crate> --fixture <path> --policy-bytes <n>` (counting allocator peak, site attribution,
`header_declared_bytes`); `ebound_import.py --format stream --policy-memory <n>` (library-level policy via
`ebr-legacy-import` since the CLI exposes no policy flag).

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
/root/eb-research/venv/bin/python research/tools/legacy/acquire_runtimes.py --registry research/experiments/_legacy/runtime-registry.json --lock research/experiments/_legacy/runtime-lock.json --only-formats gzip,bzip2,xz,zstd,lzip,lz4,brotli,lzma
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-004 --generator transport_cases --sets X1-X10 --split tuning --seed 2509170401
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-004 --generator transport_cases --sets X1-X10 --split validation --seed 2509170402 --append-corpus-items research/experiments/EXP-LEGACY-004/real-items.txt
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-004/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-004/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-004/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/matrix.py --experiment EXP-LEGACY-004 --format stream --reference-set transports --memory-fit tuning --memory-check validation --out research/normalized/EXP-LEGACY-004/matrix/
```

## 8. Seeds

`order 2509170411`, `bootstrap 2509170412`, `command 2509170413`; corruption positions from `S_T`/`S_V`.

## 9. Warmup, replication

No warmup. 2 repetitions plus repeat-hash of observations and allocator peaks (exact; any difference reclassifies that
decoder's allocator metric as stochastic and triggers §4.6 timing-style repetitions for it).

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `adversarial_true_refusal_fraction` (corrupted fixtures where the format carries a check; over-policy fixtures) | binary | OD-17 | §3.3 |
| `declared_tightness_ratio` (header-declared bound / allocator peak, per library) | banded T-23 | OD-17 | T-23 |
| `refusal_bytes_read` | banded T-02 | OD-17 | T-02 |
| `refusal_alloc_peak_bytes` | banded T-06 | OD-17 | T-06 |
| `alloc_peak_bytes` (successful decode, per library and parameter) | banded T-06 (diagnostic here) | OD-08 | T-06 |
| `import_acceptance_fraction` (benign fixtures and X8 items) | T-20 | OD-19 | T-20 |
| `reason_code_specificity_fraction` | T-20 | OD-04 | T-20 |
| corruption detection rate for checksum-less layers, false tar-detection count, filter support table, fitted `k_lib`/`c_lib` | non-canonical descriptive | — | none |
| `peak_rss_bytes` of CLI decoders | banded T-06, informational here (R1-16, calibration) | OD-08 | T-06 |

## 11. Normalization

Decoded content compared by SHA-256 and length; allocator peaks in bytes with site attribution; header-declared
requirement computed by `transport_cases.py` from the fixture header per the format specification (not by any decoder).

## 12. Statistical analysis

Exact counts per (transport × decoder) condition; H3 fit uses tuning fixtures only (least-squares over the maximum
ratio, constants rounded up to the next power of two), checked once on validation (look registry). Binary failures are
counterexamples. Real X8 items aggregate per C§10 with the support limitation recorded.

## 13. Practical significance

T-20, T-23, T-02, T-06 per `decision-method.md` §3.2; binary §3.3.

## 14. Sensitivity analysis

C§13 arms; decoder-version arm (zstd 1.5.5 vs 1.5.7, xz 5.4.5 vs 5.8.x); policy arm (16/64/256/1024 MiB); rounding arm
for `k_lib` (no rounding versus power-of-two).

## 15. Expected negative results worth recording

No header-derivable memory bound for some library (forces streaming budgets instead of prechecks); `lzma-rust2`
lacking a filter present in distribution tarballs; skippable frames carrying data some tools rely on; padding
tolerated by some but not all readers.

## 16. Threats to validity

Allocator peaks from Rust crates do not describe C decoders used by incumbents (reported separately); corruption
positions may miss sensitive regions (seeded, counts reported, regression evidence only); legacy zstd frame writers
require an old zstd build whose exact output is recorded by hash.

## 17. Platform requirements

Linux x86-64 (WSL2); Windows `tar.exe` via interop; storage ~40 GB (decoded large-dictionary outputs streamed to
`/dev/null` where possible); memory up to 4 GiB per decode; no privileged operations.

## 18. Estimates

Compute ≈ 20 CPU-hours; disk ≈ 40 GB transient. Agent effort: scripted; judgment for rule evaluation and the memory
formula review.

## 19. Expected cost tiers `[INFERRED]`

Header precheck with a dedicated reason code: T2; new transport import (lzip, LZ4): T2 with pure-Rust decoder, T3 if
`unsafe`/native; retaining skippable-frame content in provenance: T2-T3; seekable-format awareness affecting
chunk alignment: T3.
