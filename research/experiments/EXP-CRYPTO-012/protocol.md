# EXP-CRYPTO-012: Digest, Merkle-tree and AEAD throughput and proof size (measured inputs to digest, tree and primitive selection)

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

For each candidate digest algorithm, Merkle/tree-hash construction and payload AEAD, what is the throughput (pack and full verify), the proof size and verification CPU for verified range reads, and the peak buffering, on x86-64 (Windows and WSL, with and without hardware acceleration)? These are the measured inputs the digest selection (DEC-MOD-016), the tree construction (DEC-MOD-034), the SHA-256 crypto-transcript coupling (DEC-CRY-011) and the payload-AEAD selection (DEC-CRY-059) need; the final primitive selections are routed to external review.

## 2. Hypotheses and falsification

- **H1.** With SHA-NI, SHA-256 `verify_throughput_bps` is `NON_INFERIOR` to BLAKE3 single-thread and `DISTINGUISHABLE_WORSE` than BLAKE3 with parallel leaf hashing on large inputs. SHA-512/256 is `DISTINGUISHABLE_BETTER` than SHA-256 without SHA-NI.
- **H2 (tree).** A tree-native hash (BLAKE3/Bao) gives smaller proof size and lower verification CPU for random ranges at 10^5-10^7 chunks than the external RFC 6962-style tree over a flat hash; the external tree is `NON_INFERIOR` at small chunk counts.
- **H3 (AEAD).** AES-256-GCM-SIV `encode_throughput_bps` with AES-NI is `NON_INFERIOR` to AES-256-GCM and `DISTINGUISHABLE_BETTER` than ChaCha20-Poly1305 on this host; without AES-NI the order reverses (measured via the software-AES build).
- **Falsification:** each H is a directional verdict on validation at the T-05b/size bands; the opposite verdict falsifies it.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` throughput, proof size and memory. **Final digest, tree and AEAD selections are `EXTERNAL_REVIEW_REQUIRED`** (SPEC §8.8, §25.2; DEC-MOD-016, DEC-CRY-059). This experiment produces the measured inputs; the selection is not made here. DEC-MOD-016 and DEC-MOD-034 are primarily model-cluster; this crypto experiment supplies the throughput/proof-size measurements and is cross-referenced from the model domain.

## 4. Candidates and arms

- **Digest (DEC-MOD-016 / DEC-CRY-011):** SHA-256, SHA-512/256, BLAKE2b-256, BLAKE3, SHA3-256, KT128/KT256, Ascon-Hash256, SHA-256-with-hardware-accel-only. Each measured for pack (hash all chunks) and full verify.
- **Tree (DEC-MOD-034):** external RFC 6962-style (status quo, ASCII domain labels), RFC 6962 byte-prefix, tree-native (BLAKE3/Bao), Merkle-over-tree-hash chunk ids, C2SP sequencehash, k-ary tree, content-defined hashsplit tree. (`flat-digest-list` excluded at R1: no O(log n) verified range reads.)
- **AEAD (DEC-CRY-059):** AES-256-GCM-SIV (status quo), AES-256-GCM, ChaCha20-Poly1305, XChaCha20-Poly1305, XAES-256-GCM, AEGIS-256, AES-SIV, Deoxys-II-256, Ascon-AEAD128, committing-AEAD-construction, DNDK-GCM. Correctness/selection is external; here only throughput and buffering at 64 MiB records, with and without AES-NI.

Reference candidate: SHA-256, external RFC 6962 tree, AES-256-GCM-SIV.

## 5. Corpus selectors

- **Throughput (tuning):** the EXP-CRYPTO-003 timing set (at least 10 families, all tiers), plus `random-v1` buffers of 4 KiB, 1 MiB, 64 MiB and 1 GiB for raw primitive throughput.
- **Proof size / verify CPU (tuning, deterministic):** generated chunk lists of 10^3, 10^4, 10^5, 10^6 and 10^7 entries; random range requests of 1, 16 and 256 chunks.
- **Validation:** matching validation-split families and generated chunk lists, one registered look.

## 6. Environment and platform requirements

- `windows-host` and `wsl-ubuntu`, hardware acceleration on (SHA-NI, AES-NI present on the i9-14900HX) and a software-only build (`aes_force_soft`, and disabling the sha2 asm path) for the no-acceleration arm.
- Calibration (§4.7) for `throughput`, `time_wall`, `time_cpu` and `memory_peak` must pass before decision-grade timing; runner additions (§14 item 3) required.
- ARM64 hardware-accel throughput is EXP-CRYPTO-004 (BLOCKED).

## 7. Commands and tooling

- `ebr-codec` already covers codec matrices; extend `ebr-crypto bench --primitive {digest,aead} --impl ... --accel {on,off}` for raw throughput, and `ebr-crypto tree --construction ... --chunks N --ranges ...` for proof size and verify CPU.
- Pinned crates for each candidate (added to `research/harness/crates/ebr-crypto/Cargo.toml`, exact `=` pins matching production where shared: `sha2 =0.11.0`, `aes-gcm-siv =0.12.1`); `lock_parity.py` re-run.
- Proof-size accounting is exact (bytes of the audit path); verify CPU uses in-process timers (T-08 style) for the small work items.

## 8. Seeds, warmup, repetitions and adaptive rule

- Timing rounds and warmups per §4.6; adaptive precision rule; `order: randomized_blocks`.
- Proof size: deterministic, 2 repetitions plus repeat-hash.
- `random-v1` buffers use fixed seeds.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `verify_throughput_bps`, `encode_throughput_bps` | T-05b | OD-06/OD-07 | **primary** |
| `decode_wall_s`, `verify_wall_s` | T-04 | OD-07 | secondary |
| proof `verified_range_fetch_bytes` (audit-path bytes) | T-11 | OD-11 | **primary** (tree proof size) |
| `verification_digest_ops` | T-12 | OD-11 | diagnostic |
| `alloc_peak_bytes` at 64 MiB records | T-06 | OD-08 | secondary (AEAD buffering) |
| `peak_rss_bytes` | T-06 | OD-08 | secondary |

## 10. Normalization and statistical analysis

- Timing/throughput per §4.9-§4.12 with the environment arm (Windows and WSL same-direction) and the acceleration condition (AES-NI on/off, never averaged).
- Proof size: exact per (construction, chunk count, range size); no aggregation across chunk counts (each is a condition).
- Confirmatory W against S on validation; MDE gate.
- Selection is not made here; the report is an input table for the external-review dossier and for the model-cluster decision tooling.

## 11. Practical significance

T-04, T-05b, T-11, T-12, T-06 bands from `research/methods/thresholds.json`. A digest or tree change is a new identifier that supersedes frozen Descriptor-v1 vectors (T4); §7.3 minimum gains for a new baseline construction apply, but the selection is external.

## 12. Sensitivity analysis

§6 arms plus: acceleration on/off, chunk-count strata, range-size strata, and the environment arm.

## 13. Expected negative results worth recording

- SHA-256 with SHA-NI is competitive with BLAKE3 single-thread, so the throughput argument for changing the digest is weak on this host (and the change costs a frozen-identity supersession).
- The external RFC 6962 tree's proof size is acceptable at realistic chunk counts, so a tree-native change is not justified by proof size alone.
- ChaCha20-Poly1305 wins only on the no-AES-NI build, which is the ARM/low-end case (BLOCKED here).

## 14. Threats to validity

- Hybrid cores and WSL virtualization; software-AES/SHA is a proxy for no-acceleration hosts, not real ARM.
- Harness build inlining; decision-grade end-to-end throughput uses production `ebound` where a whole-archive claim is made (use constraint 2).
- Selection is external, so these numbers never on their own decide the primitive.

## 15. Held-out placeholder (Phase D)

Throughput and proof-size confirmatory verdicts on held-out large items after unlock. No held-out item named here.

## 16. Cost estimate and agent effort

- Compute: about 80 machine-hours of timing windows across both hosts and acceleration settings. Disk: about 20 GB transient.
- Agent effort: tooling **scripted** plus **judgment** for candidate crate integration; execution **scripted**; analysis **judgment** (feeds ER dossier).

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
