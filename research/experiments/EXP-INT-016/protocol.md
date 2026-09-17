# EXP-INT-016: Encrypted archive integrity: unread-segment tampering, truncation, reorder and splice, encrypted salvage, and remote verification cost

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (crypto-v1 framing walker in T1, tamper operations in T2 for segments and stanzas, T11(a) and T11(e) research hooks for byte-verification logs and per-record decrypt, recipient identity generation for scripted runs, remote verification prototype over `ebr-origin`); the security-adequacy part of DEC-CRY-080 is EXTERNAL_REVIEW (dossier input only) |
| Domain | integrity (program §24, cross-reference crypto §22.1, §22.4, §22.5 and remote §12; objective HC-14, HC-15, HC-18, OD-11, OD-12, OD-15) |
| Decisions informed | DEC-INT-010, DEC-INT-033, DEC-CRY-080, DEC-CRY-073, DEC-CRY-033 (addressing-signature candidate) |
| Gates, method, author | conventions C0 |
| Spec | `spec.yaml` |

## 1. Question

For crypto-v1 encrypted INDEXED archives, which tampering of segments the reader does not read
(deletion, duplication, reordering, splicing from sibling archives, truncation at record
boundaries) can make a partial read report a stronger state than it verified or release
plaintext; how much can be salvaged with keys after damage without producing misleading output;
and what do remote signature verification and remote whole-archive verification cost in requests
and bytes, including under origin substitution over plain HTTP?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-CRY-080 | No enumerated truncation, reorder, splice or stanza insertion or removal releases any plaintext byte or yields an outcome other than an authentication-class failure (MVT-14(b)); `undetected_tampers` = 0 over the enumerated corpus. The security argument itself remains for external formal review. | Any plaintext release or non-authentication outcome (R1; defect; routed to the crypto dossier). |
| H2 | DEC-INT-010 | Under the status quo, a partial read of an intact object succeeds while unread segments are deleted or reordered, and it reports PCR as `DeclaredNotFullyVerified` and PCI as `NotComputed` (no overclaim); `verify-touched-segment-ends` additionally detects deletion or reordering of segments adjacent to touched records at extra bytes within the T-11 floor. | An overclaim cell (HC-15) under the status quo, or `verify-touched-segment-ends` cost above the T-11 band. |
| H3 | DEC-INT-010 | Remote whole-archive verification by streaming ranges needs bytes equal to the archive size and requests proportional to archive size over the range window, with modelled latency on NP-CONT more than 1 band unit of T-10 above a local download plus verify only when requests exceed one per RTT budget; memory stays bounded. | Memory not bounded (HC-06 binary) or no difference. |
| H4 | DEC-INT-033 | `authenticated-record-recovery` recovers every intact authenticated object after ciphertext or segment-header damage when envelope, commitment and recipient directory are intact, and never outputs spliced or reordered records as intact; `require-intact-envelope` recovers the same set with fewer cases; `fail-closed-no-salvage` recovers nothing. | Any misleading output (HC-15) or a recovered fraction not distinguishable from `require-intact-envelope`. |
| H5 | DEC-CRY-073, DEC-CRY-033 | Over plain HTTP without a pin, an origin that swaps the archive for another validly signed archive (attacker key) or for an older archive signed by the same key is accepted by the range opener, which skips signatures; `verify-signatures-on-range-open` detects the attacker-key swap given a trusted key but not the same-key rollback; `require-pin-for-plain-http` and `expected-pci-before-parse` detect both. The signature fetch costs one or two extra requests on first open. | Status quo range opener detects the swap. |

## 3. Candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-INT-010 | `status-quo-per-object-plus-archivefinal` (B-PROD), `verify-touched-segment-ends` (T11(d) reader option), `per-segment-digests-in-archivefinal` (wire change under frozen crypto-v1: evaluated only as a detection model, labelled MODEL, tier T4 new crypto version), `remote-streaming-whole-verify` (prototype over `ebr-origin`), `full-download-only` (curl plus local verify), `merkle-over-segments` (MODEL, T4) | Tamper corpus plus MVT-15(a) oracle states for encrypted partial reads (shared matrix conventions with EXP-INT-003); formal model of partial-read guarantees is an EXTERNAL dossier input (gap). |
| DEC-INT-033 | `fail-closed-no-salvage`, `authenticated-record-recovery` (T11(e) prototype), `require-intact-envelope`, `parity-required-for-encrypted` (recovery from EXP-INT-010's ciphertext arm, cited), `public-framing-recovery-only` | Damage per encrypted structure (envelope, each stanza, recipient directory, commitment, segment headers, ciphertext, SegmentEnd, ArchiveFinal, footer) with D-P1 and D-S2 to D-S4; splice and reorder attempts against salvage. |
| DEC-CRY-080 | `status-quo-segmentend-archivefinal` (measured); `stream-construction`, `final-flag-in-nonce`, `merkle-over-segments`, `per-record-counter-only` (constructions under external review; not implemented; the corpus is reusable for them) | Enumerated corpus: truncation at every record boundary; every adjacent record swap and 1,000 seeded permutations; splicing records between two archives with the same recipient set; stanza insertion, removal and reordering; all single-bit flips in public framing, headers and stanzas (exhaustive) and 10,000 seeded ciphertext flips (MVT-14(b)). |
| DEC-CRY-073 | `status-quo-no-signatures-remote`, `verify-signatures-on-range-open` (prototype over the library), `require-pin-for-plain-http`, `https-only-default` (TLS origin: `ebr-origin` behind a local TLS terminator with a generated test CA; cost is setup round trips per §4.15 connection model), `expected-pci-before-parse` | Substitution attacks (H5); request and byte cost from exact origin logs; modelled latency per network profile. |
| DEC-CRY-033 | `expected-addressing-signature` (plaintext arm impossible, EXP-INT-009) | Rollback detection with an addressing-bound embedded signature on encrypted archives. |

## 4. Corpus

- Tuning: `f01-tuning-ripgrep-14-1-1-git`, `f05-tuning-gutenberg-books`, `f07-tuning-silesia-xray`,
  `f12-tuning-kodak-jpeg-edge-cases-small`, `f19-tuning-zoo`, `f18-tuning-lz4-releases-1-9-to-1-10`
  (consecutive versions packed separately for splice and rollback cases).
- Validation look: `f01-validation-redis-7-2-16-git`, `f05-validation-rfc-texts`,
  `f07-validation-gen-numeric-b-small`, `f12-validation-fsa-jpeg-edge-cases-small`,
  `f19-validation-real-home-snapshot`, `f18-validation-cjson-releases`.
- Archives: B-PROD `pack --recipient` with generated X-Wing test recipients (recipients avoid an
  Argon2id KDF per open); a small password arm (5 items, 1 repetition per trial class) records
  password-specific framing behaviour; padding modes bucketed (default), none and max.

## 5. Environment and platform requirements

`wsl-ubuntu`, `timing: false`; `ebr-origin` on localhost, plain HTTP and local TLS (test CA in the
work directory only; no system trust-store change). Remote latency is modelled from exact logs
(§4.15). Real CDNs are EXP-INT-020 (BLOCKED). No Windows arm (crypto framing is platform
independent; Windows reader identity is the platform domain's).

## 6. Procedure and commands

1. Generate recipient identities with the harness (`ebr-pack --encrypt` identity export, tooling).
2. `ebr run` of `spec.yaml`, `verify-raw`, `verify_detail.py`, `normalize`, `report`.
3. Dossier extract: `research/tools/integrity/crypto_dossier_extract.py --experiment EXP-INT-016`
   writes the tamper corpus summary for the crypto domain's external-review dossier.

## 7. Seeds, warmups, replication

Seeds 2026091861/62/63; tamper selection per C8. Warmups 0. Encrypted packs are nondeterministic
by design (HC-13(b)): each repetition packs anew; the repeat-hash applies to the outcome vector
over structurally equivalent tamper positions (as EXP-INT-002). 2 repetitions.

## 8. Metrics

| Metric | ID | OD | Role |
|---|---|---|---|
| `undetected_tampers` | HC-14 | OD-15 floor | binary |
| `authenticated_coverage_fraction` | HC-14 | OD-15 floor | binary (every byte after the public preamble under AEAD or bound as associated data) |
| `mvt15a_overclaim_cells` | C11 | HC-15 | binary |
| `verified_fraction` | T-20 | OD-11 | **primary** for DEC-INT-010 |
| `http_requests` | T-12 | OD-12 | **primary** for DEC-CRY-073 and DEC-INT-010 remote verification |
| `http_bytes` | T-11 | OD-12 | secondary |
| `remote_task_latency_s` | T-10 | OD-12 | secondary (modelled per network profile; AGG-S) |
| `bounded_memory_growth_bytes` | HC-06 | OD-08 floor | binary for remote streaming verification |
| `origin_protocol_safety` | HC-18 | OD-12 floor | binary |
| `salvage_recovered_fraction` | C11 proposed | OD-14 | secondary until named; **primary candidate** for DEC-INT-033 |

## 9. Normalization

Exact counts and fractions; requests and bytes as paired log ratios against the status quo opener
(T-11, T-12 floors); latency per network profile never pooled.

## 10. Statistical analysis and sensitivity

Binary metrics decide R1; banded metrics are secondary for lack of minimum support (six items;
C3) unless extended by revision before the first look. Sensitivity: padding-mode arm (never
pooled; objective AGG-S), recipient versus password arm, network profile arm, slow-start arm of the
latency model (R1-14).

## 11. Expected negative results worth recording

- Remote range openers accept substituted archives over plain HTTP without a pin.
- Keyless or partial reads cannot detect deletion of unread segments; only whole verification can.
- Encrypted salvage yields little without intact envelope structures.

## 12. Threats to validity

- Internal tamper testing is regression evidence, not a security proof; construction adequacy
  needs external review (§8.3).
- Research decrypt hooks add code paths; MVT-16(c) identity checks bound their effect on bytes,
  not on side channels.
- Local TLS termination does not model real TLS stacks' session resumption.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12.

## 14. Cost estimates

- Compute: about 20,000 tamper cases per archive (exhaustive framing flips dominate) x 12 archives
  x 3 padding modes x 2 repetitions, each an open and partial read of a small archive: about 40
  CPU-hours; salvage arm about 10 CPU-hours; remote arm about 5 CPU-hours.
- Disk: about 20 GB.
- Agent effort: crypto-aware tooling requires judgment and review by a session familiar with
  crypto-v1 wire docs; execution none; dossier extraction scripted; analysis judgment.

## 15. Deviations (append-only)

None.
