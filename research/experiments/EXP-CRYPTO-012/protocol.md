# EXP-CRYPTO-012: Digest, Merkle-tree and AEAD throughput and proof size (measured inputs to digest, tree and primitive selection)

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** — Requires timing calibration; final primitive/tree/digest selection is external review |
| Kind | decision |
| Domain | crypto (program §22.1) |
| Decisions informed | DEC-MOD-016, DEC-MOD-034, DEC-CRY-011, DEC-CRY-059 |
| Platforms | Windows+Linux/x86-64 |
| Requires timing | True |
| Estimates | 80 machine-hours; 20 GB disk |
| Tooling to build | ebr-crypto bench/tree; pinned candidate crates; lock_parity |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns look 1 for DEC-CRY-011, DEC-CRY-059, DEC-MOD-016; participates in looks owned by other experiments for DEC-MOD-034 |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
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
#### DEC-MOD-016 — Which single digest algorithm does Entrybound format version 1 use for chunk, content, structured, section, PCI and identity-descriptor digests?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-004, EXP-CRYPTO-020, EXP-CRYPTO-021, EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `sha-256-status-quo` | SHA-256 (FIPS 180-4) via RustCrypto sha2 =0.11.0, as implemented for every digest and bound as algorithm id 1 | ledger alternative | no | — |
| `sha-512-256` | SHA-512/256 (FIPS 180-4): faster on 64-bit CPUs, same standard family, length-extension resistant | ledger alternative | no | — |
| `blake2b-256` | BLAKE2b-256 (RFC 7693): fast in software on 64-bit CPUs without SHA extensions; the Borg incumbent's chunk-ID hash; no tree mode in the RFC profile | ledger alternative | no | — |
| `blake3` | BLAKE3 (C2SP BLAKE3 v1.0.0): tree-native, SIMD/parallel, not NIST/IETF standardised | ledger alternative | no | — |
| `sha3-256` | SHA3-256 (FIPS 202) | ledger alternative | no | — |
| `kangarootwelve-kt128` | KT128/KT256 (RFC 9861, IRTF informational): tree-capable Keccak variant | ledger alternative | no | — |
| `ascon-hash256` | Ascon-Hash256/Ascon-CXOF128 (NIST SP 800-232): lightweight, customization strings for domain separation | ledger alternative | no | — |
| `sha-256-with-hardware-accel-only-fallback` | SHA-256 retained but mandate SHA-NI/ARMv8 crypto-extension paths and parallel per-chunk hashing | ledger alternative | no | — |
| `dual-digest-transition` | SHA-256 for v1 plus a registered second algorithm id reserved for a future format version (no in-version negotiation) | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `flat-digest-list`.

**R0 checklist:** 1 status quo: no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 9 ledger candidates listed above; 4 strongest incumbent: `blake2b-256`, `kangarootwelve-kt128`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `sha-256-status-quo`, `sha-512-256`, `blake2b-256`, `blake3`, `sha3-256`, `kangarootwelve-kt128`, `ascon-hash256`, `sha-256-with-hardware-accel-only-fallback`, `dual-digest-transition`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-MOD-034 — Should Merkle roots over chunk lists and manifests remain an external RFC 6962-style tree with custom structured-hash domains layered over a flat hash, or be defined by a tree-native hash or a standardised sequence-hash construction?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CON-003, EXP-CONF-012, EXP-REMOTE-007.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `external-rfc6962-status-quo` | Status quo: largest-power-of-two split, no padding, domain-labelled leaf/node/empty structured hashes over SHA-256, leaves bind chunk_id and logical_len | status quo | no | EXP-CON-003, EXP-REMOTE-007 |
| `rfc6962-byte-prefix` | RFC 6962/9162 shape with 0x00/0x01 single-byte prefixes instead of ASCII domain labels | ledger alternative | no | EXP-CON-003, EXP-CONF-012, EXP-REMOTE-007 |
| `tree-native-hash-leaves` | Tree-native hash (BLAKE3/Bao, KT) with fixed internal chunking; CDC chunks referenced by offset into the tree | ledger alternative | no | EXP-CON-003, EXP-REMOTE-007 |
| `merkle-over-tree-hash-chunk-ids` | External Merkle over chunk ids computed with a tree-native hash (hybrid) | ledger alternative | no | EXP-CON-003, EXP-REMOTE-007 |
| `c2sp-sequencehash` | Adopt C2SP sequencehash v1.0.0 (length-prefixed, customization string) for multi-field inputs and node hashing | ledger alternative | no | EXP-REMOTE-007 |
| `flat-digest-list` | Flat per-chunk digest list without a Merkle tree (container-image style), requiring full index validation before random verification | ledger alternative | yes | EXP-CON-003, EXP-REMOTE-007 |
| `k-ary-tree` | Wider fan-out (k-ary) tree to reduce proof depth for large chunk counts | ledger alternative | no | EXP-REMOTE-007 |
| `content-defined-hashsplit-tree` | bup-style content-defined tree over the chunk list: extra rolling-hash bits choose interior boundaries, so an edit changes O(log n) nodes, interior nodes deduplicate across versions, and very large chunk lists page by subtree (APPX/chunking.md §2 bup L152) | ledger alternative | no | EXP-CON-003, EXP-REMOTE-007 |

**Ledger exclusions:** {"candidate_id": "flat-digest-list", "reason": "Cannot provide O(log n) verified range reads against a signed root required by SPEC §8.6 L977 and the three-root model", "invariant_violated": "SPEC §8.6 range verification / I25 verification state"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `flat-digest-list`.

**R0 checklist:** 1 status quo: `external-rfc6962-status-quo`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 8 ledger candidates listed above; 4 strongest incumbent: `external-rfc6962-status-quo`, `rfc6962-byte-prefix`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-011 — Is SHA-256 confirmed for suite-1 crypto transcripts, encrypted object IDs and public digests, and must it stay coupled to the identity digest chosen under model decision digest-algorithm-v1?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-020, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-sha-256-coupled` | Status quo: SHA-256 for crypto transcripts and identities | status quo | no | — |
| `sha-256-crypto-decoupled` | Keep SHA-256 in suite 1 regardless of identity-digest changes (suite-versioned) | ledger alternative | no | — |
| `sha-512-256` | SHA-512/256 for transcripts | ledger alternative | no | — |
| `blake3` | BLAKE3 for transcripts | ledger alternative | no | — |
| `sha3-256` | SHA3-256 for transcripts | ledger alternative | no | — |
| `follow-identity-decision` | Adopt whatever digest-algorithm-v1 selects, changing suite id if needed | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `flat-digest-list`.

**R0 checklist:** 1 status quo: `status-quo-sha-256-coupled`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-sha-256-coupled`, `sha-256-crypto-decoupled`, `sha-512-256`, `blake3`, `sha3-256`, `follow-identity-decision`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-059 — Does independent review confirm AES-256-GCM-SIV as the suite-1 payload, control and wrap AEAD, or must suite 1 change before v1 freeze?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-020, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-aes-256-gcm-siv` | Status quo: AES-256-GCM-SIV (RFC 8452), 12-byte structured nonces, aes-gcm-siv =0.12.1 | status quo | no | — |
| `aes-256-gcm` | AES-256-GCM (SP 800-38D) with the same structured nonces | ledger alternative | no | — |
| `chacha20-poly1305` | ChaCha20-Poly1305 (RFC 8439) | ledger alternative | no | — |
| `xchacha20-poly1305` | XChaCha20-Poly1305 (no RFC) | ledger alternative | no | — |
| `xaes-256-gcm` | XAES-256-GCM (C2SP) with 192-bit nonces | ledger alternative | no | — |
| `aegis-256` | AEGIS-256/256X (RFC pending) | ledger alternative | no | — |
| `aes-siv` | AES-SIV (RFC 5297) with explicit nonce | ledger alternative | no | — |
| `deoxys-ii-256` | Deoxys-II-256-128 (CAESAR final portfolio, defense-in-depth use case): nonce-misuse-resistant tweakable-block-cipher AEAD and the direct nonce-misuse-resistant alternative to GCM-SIV | ledger alternative | no | — |
| `ascon-aead128` | Ascon-AEAD128 (SP 800-232) | ledger alternative | no | — |
| `committing-aead-construction` | Status quo AEAD plus construction-level key commitment (commitment decisions owned by crypto-keys-recipients) | status quo | no | — |
| `dndk-gcm` | DNDK-GCM (IETF CFRG draft): AES-GCM with double-nonce key derivation and optional built-in key commitment; named with AES-XGCM and XAES-256-GCM in NIST's SP 800-38D Rev. 1 first pre-draft call (2025-01-06) | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `flat-digest-list`.

**R0 checklist:** 1 status quo: `status-quo-aes-256-gcm-siv`, `committing-aead-construction`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 11 ledger candidates listed above; 4 strongest incumbent: `status-quo-aes-256-gcm-siv`, `chacha20-poly1305`, `aes-siv`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-aes-256-gcm-siv`, `aes-256-gcm`, `chacha20-poly1305`, `xchacha20-poly1305`, `xaes-256-gcm`, `aegis-256`, `aes-siv`, `deoxys-ii-256`, `ascon-aead128`, `committing-aead-construction`, `dndk-gcm`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.
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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-012; edit the inputs, not this block._
<!-- END AUTO:common -->
