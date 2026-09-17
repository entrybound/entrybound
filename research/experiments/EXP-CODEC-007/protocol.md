# EXP-CODEC-007: Canonical single-frame rule and STREAM stored-bytes checks: benign false refusals, size and CPU overhead, hostile-frame refusal cost

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CODEC-007 |
| Domain | codecs (8.6; shared with integrity 24) |
| Status | NEEDS_TOOLING: the `single-frame-rule` checker, `ebr-pack --stream-digest-overhead`, and the `ebr-codec stream-refusal` hostile-frame driver. |
| Stage | Deterministic benign-refusal census and size overhead (tuning, then a validation look). CPU overhead and refusal cost (timing, tuning, then a validation look). Held-out: Phase D placeholder. |
| decision_ids | DEC-INT-008 (must stored codec payloads be canonical single frames with no trailing bytes; should STREAM chunk frames carry a stored-bytes digest checked before decoding?) |
| Scope boundary | The HC-03/HC-04 case tests (codec slack: multiple frames, zstd skippable frames, trailing bytes, content-size lies, window above declaration; unknown codec and transform identifiers) are the conformance corpus's F-CODEC class, EXP-CONF-004. Their resource ceilings are EXP-CONF-007. This experiment supplies what those do not measure: whether a single-frame rule would falsely refuse any real payload, what each STREAM check costs in bytes and CPU on real archives, and how much decode work a STREAM reader performs before rejecting hostile frames under each candidate. |
| Proposed decision type | EMPIRICAL |
| Proposed od_ids | OD-05 (`artifact_bytes`), OD-07 (`decode_cpu_s`), OD-17 (`refusal_cpu_s`, `refusal_alloc_peak_bytes`, `refusal_bytes_read`, `benign_false_refusal_fraction`), OD-24 (cost input) |
| Proposed hc_ids | HC-03 (unique interpretation), HC-06 (refusal before unbounded work), HC-15 (distinguishable failure) |
| Method, gates, author, L7 | EXP-CODEC-001 section 1 and Appendix C. |

## 2. Question

- Would a rule rejecting extra frames, skippable frames and trailing bytes refuse any payload that production writes today?
- For STREAM archives of real items, what would a per-frame stored-bytes digest (BLAKE3 or SHA-256) or a per-frame CRC32C add in bytes and in decode CPU?
- How much CPU, memory and input does a STREAM reader consume before rejecting hostile frames under each candidate?

## 3. Hypotheses

- **H1a (benign).** The single-frame rule refuses no payload written by production at any profile and generation: `benign_false_refusal_fraction` = 0 over every tuning and validation chunk payload. Production writes one frame per chunk with no slack. *Falsified* by one refused benign payload, which is recorded with its bytes.
- **H1b (size).** A 32-byte digest per STREAM frame is `EQUIVALENT` to no digest on `artifact_bytes` for medium and large items. On small-tier items with many chunks it can be `DISTINGUISHABLE_WORSE` (relative-only band). A 4-byte CRC32C is `EQUIVALENT` everywhere.
- **H1c (CPU).** Checking a BLAKE3 digest over stored bytes adds less than 1 T-05 band unit to STREAM `decode_cpu_s` at balanced and extreme. SHA-256 adds 1-3 band units on highly compressible items, because there stored bytes are few relative to decode work.
- **H1d (refusal cost).** On hostile STREAM frames (decompression bombs, window lies, content-size lies), `hardened-decoders-and-budgets` (status quo) stops within declared budgets. Its `refusal_cpu_s` still exceeds that of `stream-stored-bytes-digest` by at least 1 T-05 band unit, because the digest check rejects a tampered frame before any decode. A CRC32C check rejects corruption but not a deliberately crafted frame with a valid CRC.
- **H0.** Every candidate is `EQUIVALENT` to the status quo on every primary metric.
- **Falsification.** The verdict class at the validation look differs from the one stated. H1a is binary.

## 4. Candidates

| candidate_id (ledger) | Measured as |
|---|---|
| status-quo-bulk-decode (reference) | production STREAM decode path at HEAD |
| enforce-single-frame-no-trailing | a harness checker applying the rule before the production decoder: benign false-refusal census, plus the rule's CPU cost (negligible, measured) |
| stream-stored-bytes-digest | a modelled wire change of 32 bytes per STREAM frame. BLAKE3 and SHA-256 are both measured; the digest is checked before decode. |
| per-frame-crc32c | 4 bytes per frame, covering corruption only; no authentication claim |
| hardened-decoders-and-budgets | the status-quo decode path under declared budgets, with plaintext digests checked after decode. It is the refusal-cost reference. |
| accept-trailing-bytes-silently | excluded in the ledger (I15, unique interpretation). The counterexample bytes are the EXP-CONF-004 F-CODEC slack cases. |

## 5. Corpus

- **Benign census.** Every stored chunk payload of production `ebound pack` archives of every tuning item (then validation). Profiles fast, balanced, dense and extreme; INDEXED and STREAM layouts. Payloads are extracted with `ebr-inspect-bytes`/`ecf::research::parse_chunk_frame_header`.
- **Overhead.** STREAM archives (`--stream-window auto`) of every tuning item at balanced and extreme.
- **Hostile frames.**
  - The F20 bomb items `f20-tuning-bombs-dense` and `f20-validation-bombs-compressed`, repacked as STREAM archives.
  - Generated STREAM archives whose frames are replaced by the EXP-CONF-004 F-CODEC hostile payload variants. The replacement is harness re-framing with the frame length field updated. For the digest candidates the modelled digest is computed over the original bytes, so the check fails as a real reader's would.
  - Generator `gen_stream_hostile.py`, seed 20260917, lock file committed before any result.

## 6. Environment

WSL2 x86-64. Size and benign census are deterministic. CPU arms follow CC-8.

## 7. Tooling and commands

### 7.1 To build

1. `ebr-codec single-frame-check --archive-path <eb> --summary-stdout`: applies the rule to every payload and reports refusals with payload SHA-256.
2. `ebr-pack --layout stream --stream-digest-overhead none|crc32c|blake3|sha256 --timing-mode in-process --sink memory`:
   - computes the exact `artifact_bytes` with the modelled per-frame field;
   - times STREAM decode plus the check in-process;
   - reports `stream_frame_count`.
3. `ebr-codec stream-refusal --archive-path <hostile.eb> --candidate <c> --summary-stdout`: in-process `refusal_cpu_s`, `refusal_alloc_peak_bytes` (with the `ebr-alloc` build), `refusal_bytes_read`, and the outcome class and reason code.
4. `gen_stream_hostile.py` and its lock file.

### 7.2 Commands

```sh
PY=/root/eb-research/venv/bin/python
for S in spec.yaml spec-overhead.yaml spec-refusal.yaml; do $PY -m ebr validate --spec research/experiments/EXP-CODEC-007/$S; done
```

## 8. Seeds

Order, bootstrap and generator seeds: 20260917.

## 9. Metrics

| Metric | OD | Role | Family | Threshold |
|---|---|---|---|---|
| `benign_false_refusal_fraction` | OD-17 | primary for enforce-single-frame-no-trailing | other | T-20 (0.5/n_cases) |
| `artifact_bytes` | OD-05 | primary for the digest and CRC candidates | size | T-01 |
| `decode_cpu_s` | OD-07 | primary (STREAM decode plus check) | time_cpu | T-05 |
| `refusal_cpu_s` | OD-17 | primary (hostile frames; AGG-W) | time_cpu | T-05 |
| `refusal_alloc_peak_bytes`, `refusal_bytes_read` | OD-17 | secondary | memory_peak, size | T-06, T-02 |
| outcome class and reason code on hostile frames | — | binary (HC-15: distinguishable failure; never OK) | correctness | §3.3 |

At most one primary metric per OD: OD-17 carries `refusal_cpu_s` for the digest candidates and `benign_false_refusal_fraction` for the rule candidate. These are separate candidate comparisons, and the second primary metric carries a written justification for the reviewer.

## 10. Replication

- Benign census and size: deterministic; 2 repetitions and a repeat-hash (CC-7).
- CPU: §4.6 rounds with the adaptive rule (CC-8).

## 11. Normalization

CC-9. `refusal_cpu_s` uses AGG-W (worst hostile frame per item) and AGG-G across families.

## 12. Statistical analysis

- H1a is a binary census. Any benign refusal eliminates `enforce-single-frame-no-trailing` as specified, and a revised rule becomes a new candidate version.
- R4 among the surviving candidates on OD-05, OD-07 and OD-17, with cost tiers: a stored-bytes digest field in the STREAM frame is T2 (new field with decode semantics); a single-frame rule that is normative text only is T1; hardened decoders are T0.
- §7.3 minimum gain for the T2 row.
- CC-10.

## 13. Practical-significance thresholds

T-01, T-02, T-05, T-06, T-20; §7.3 "New field, enum value, reason code or feature bit".

## 14. Sensitivity

- CC-11.
- BLAKE3 against SHA-256.
- Small-tier-only stratum for size.
- Hostile generator variants excluded one class at a time.

## 15. Held-out (Phase D placeholder)

CC-12.

## 16. Expected negative results

- The single-frame rule costs nothing on benign data, so rejecting slack is free. The status quo's acceptance of slack (EXP-CONF-004) is then a pure defect.
- The digest's size cost matters only for tiny many-chunk STREAM archives.
- CRC32C gives no protection against crafted frames.

## 17. Threats to validity

- The hostile STREAM archives are harness re-framed, and real archive-layer checks may reject some of them earlier. This is reported as a class; it may favour the status quo.
- Digest timing on this host (SHA extensions present) does not generalise to hardware without them. ARM64 is blocked (EXP-CODEC-009).
- Modelled wire fields assume a fixed per-frame layout; a real design could batch digests (flagged to the critic as a candidate variant).

## 18. Platform and cost

| Item | Value |
|---|---|
| Platforms | Linux x86-64 WSL2. No network, no privileges. |
| Compute | Benign census: production packs at 4 profiles × 2 layouts × 2 repetitions dominate, ≈ 120 CPU-h (dense and extreme packs at single-digit MB/s over 62.7 GiB; the census check itself ≈ 4 CPU-h). Packs are not assumed to be shared with other domains. Overhead size ≈ 2 CPU-h. CPU timing ≈ 12 h exclusive. Refusal timing ≈ 3 h exclusive. |
| Disk | Transient archives ≈ 30 GB (deleted after hashing); raw < 1 GB. |
| Agent effort | Scripted; analysis by a session without the L7 exposure. |

## 19. Deviations (append-only)

None.
