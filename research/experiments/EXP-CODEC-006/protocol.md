# EXP-CODEC-006: Write-time verification scope: fault-injection detection and pack-time cost

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CODEC-006 |
| Domain | codecs (8.7; shared with integrity section 24) |
| Status | NEEDS_TOOLING. Needs a harness pack pipeline with selectable verification scope and a seeded fault injector (`ebr-pack --verify-scope`, `--inject-fault`). It is harness-only: production planning via the public `plan_archive_v6` API, per-chunk encoding via `entrybound::research::codec`, and assembly via the public encoder. No production hook is added (research-internals "no hooks" rule). Timing arms also need §14 items 3 and 4. |
| Stage | Detection arm: deterministic, tuning, then a validation look. Timing arm: tuning, then a validation look (W and S). Held-out: Phase D placeholder. |
| decision_ids | DEC-CMP-044 (write-time verification scope), DEC-CMP-043 (verification cost of structural transforms, the "including verification cost (I31)" clause of §7.3) |
| Proposed decision type | EMPIRICAL (cost, detection); the interpretation of I31/T2 scope (REQ-CMP-0150, an open SPEC question) is a SPEC clarification outside this experiment |
| Proposed od_ids | OD-06 (`encode_wall_s`, `encode_cpu_s`) |
| Proposed hc_ids | HC-02 ("injected reconstruction error is never written"), HC-15 (verification honesty: an archive must never claim verification it did not do) |
| Method, gates, author, L7 | EXP-CODEC-001 section 1 and Appendix C. |

## 2. Question

For each candidate verification scope, which classes of seeded encoder, transform and parameter faults are detected before the archive is written or published? What does each scope cost in pack wall time and CPU time per profile?

## 3. Hypotheses

- **H1a (detection; FORMALLY_DERIVED expectations to be confirmed empirically).**
  - Status quo (reconstructive dual check only) detects 0% of injected codec-payload faults (FM1) and structural-transform faults (FM2) before write.
  - verify-all-transformed detects 100% of FM2 and 0% of FM1 on untransformed chunks.
  - verify-all-chunks and post-write-full-verify detect 100% of FM1-FM3.
  - sampled-verification at fraction q detects each single fault with probability q (the observed fraction lies within the binomial 0.99 interval around q).
  - No scope detects FM4 (plaintext corrupted before digest computation). That is the negative control showing what write-time verification cannot catch.
- **H1b (cost).**
  - At fast and balanced, verify-all-chunks is `DISTINGUISHABLE_WORSE` than the status quo on `encode_wall_s` at the profile's T-03 band: decode speed is comparable to encode speed at low levels.
  - At dense and extreme, it is `EQUIVALENT` or within 1 band unit (the extreme band is 0.25): decode is small against the codec search.
  - verify-all-transformed is `EQUIVALENT` to the status quo at every profile.
- **H0.** Every scope is `EQUIVALENT` to the status quo on `encode_wall_s` at every profile.
- **Falsification.** Any detection fraction outside the predicted value or interval (detection arm). The verdict class at the validation look (cost arm).

## 4. Candidates

| candidate_id (ledger) | Harness scope |
|---|---|
| status-quo-reconstructive-only (reference) | reconstructive dual check (production `verified_forward`), structural steps length-checked, chunk digests verified before planning |
| verify-all-transformed | + decode and compare every chunk whose plan has a structural step |
| verify-all-chunks | + decode and compare every stored chunk for every codec |
| sampled-verification | + decode and compare a deterministic sample, the chunks whose plaintext digest mod 2^16 < q·2^16, for q ∈ {1/64, 1/16, 1/4} (`OPERATIONALIZATION` of "deterministic sample") |
| post-write-full-verify | + production `verify` of the complete encoded archive in-process before the output is renamed into place (publication) |

R0 gaps for the critic: "defer" is status quo; the SPEC-proposed design is ambiguous (REQ-CMP-0150: T2's heading says every transform, its body says reconstructive only); a critic candidate.

## 5. Fault models (seeded; `configs/fault-models.json`, committed before results)

| ID | Injected where | Examples |
|---|---|---|
| FM1 | encoder output, after encode and before framing | single bit flip at a seeded offset; truncation of the last k bytes; zeroed tail; valid-but-wrong frame (payload of a neighbouring chunk with the same length class) |
| FM2 | structural transform forward | lane order swapped in byte-shuffle; delta seed 1 instead of 0; tail bytes shuffled; plan still records the correct step |
| FM3 | parameter/plan mismatch | LZMA2 payload encoded with a different dictionary size than the plan records; zstd payload encoded at another window log |
| FM4 (negative control) | plaintext before chunk digest | single bit flip in the plaintext buffer before digest computation |
| FM5 | dictionary/prefix context | encode with the neighbouring cohort's dictionary or prefix (dictionary and prefix plans only; shared with crossfile) |

For each (item, profile, fault model), 20 seeded injections are applied, each to a distinct chunk and in isolation. Production plans are unchanged, only the injected artefact differs. Each injection is followed by the scope's check and, when the check does not refuse, by `verify` and unpack of the written archive to classify the outcome: `detected-at-write`, `detected-at-verify` (written but corrupt), or `undetected` (verify passes with wrong bytes, an HC-15/HC-02 violation artifact).

## 6. Corpus

- **Detection arm.** Every tuning item of the small and medium tiers (95 items, 20 families), then validation small and medium (80 items). All 4 profiles.
- **Timing arm.**
  - All tuning items at fast and balanced.
  - Small- and medium-tier items at dense and extreme.
  - Large-tier dense/extreme packs are a declared limitation: one pack can take hours, and the §4.6 rounds are infeasible. The R4 condition 3 tier guard therefore cannot be evaluated for large at dense/extreme, and the decision scope excludes that stratum explicitly.
  - Validation look on the same tier coverage.

## 7. Tooling and commands

### 7.1 To build (harness)

1. `ebr-pack --verify-scope status-quo|transformed|all|sampled:<q>|post-write --timing-mode in-process`. The timed region is plan + encode + verification + write to a tmpfs or in-memory sink (writing excluded from the time via a `Vec` sink).
2. `ebr-pack --inject-fault <FMx> --fault-seed <s> --fault-count 20 --classify-outcome true`. It emits per-injection outcome classes and the detection fraction per scope.
3. `configs/fault-models.json` and `research/experiments/EXP-CODEC-006/check_detection.py`, which compares detection against the H1a formal expectations and writes a mismatch report.

### 7.2 Commands

```sh
PY=/root/eb-research/venv/bin/python
for S in spec-detection.yaml spec.yaml spec-large.yaml; do $PY -m ebr validate --spec research/experiments/EXP-CODEC-006/$S; done
# detection arm -> check_detection.py -> timing arm in scheduled exclusive windows
```

## 8. Seeds and warmups

- Fault seeds 20260917 + injection index. Order and bootstrap seeds 20260917.
- Timing warmups follow CC-8.

## 9. Metrics

| Metric | OD | Role | Family | Threshold |
|---|---|---|---|---|
| `encode_wall_s` | OD-06 | primary (profile-scope: T-03 band of each profile) | time_wall | T-03 |
| `encode_cpu_s` | OD-06 | secondary | time_cpu | T-05 |
| `roundtrip_mismatches` | OD-02 | binary (no scope may write an archive that verifies with wrong bytes) | correctness | §3.3 |
| detection fraction per fault model at write, at verify, undetected | — | descriptive, non-canonical. It is not a band decision: the benefit side is a risk-reduction count, whose weight is a SPEC question (REQ-CMP-0150). | other | — |

## 10. Replication

- Detection: deterministic, 2 repetitions plus an outcome-digest check.
- Timing: §4.6 by tier, adaptive rule (CC-8).

## 11. Normalization

CC-9, per profile and tier.

## 12. Statistical analysis

- **R1.** Any scope under which an injected fault yields an archive that `verify` reports as OK with wrong bytes violates HC-15/HC-02. The expected source is FM3 under the status quo, if the plaintext digest does not cover the codec parameter record. Such a violation is recorded as a production defect under objective §2.0 rule 5, not merely a candidate elimination.
- **R4.** Profile-scope form per profile (Appendix B.1): no size difference exists among scopes (all write identical bytes when no fault occurs). The decision is therefore a cost minimisation subject to the detection requirement the SPEC clarification sets. The experiment reports the frontier (cost against detected fault classes). The selection is made only after the I31 scope is fixed (the dependency is recorded), otherwise R10 (simplest candidate under `EQUIVALENT` cost).
- CC-10 for confidences and looks.

## 13. Practical-significance thresholds

T-03 per profile; T-05.

## 14. Sensitivity

- CC-11 threshold perturbations.
- Sampled q grid.
- In-memory sink against a real ext4 write (the write cost is diluted in the real write; report both).
- Extreme's band (0.25) under threshold perturbation ×0.5 (resolvability condition applies).

## 15. Held-out (Phase D placeholder)

CC-12 (detection arm on held-out small and medium items; timing arm per the same tier coverage).

## 16. Expected negative results

- Write-time verification cannot detect plaintext corruption before digesting (FM4). This limits any "verified at write" claim.
- verify-all-chunks is materially expensive at fast.
- Sampling offers probabilistic protection only.

## 17. Threats to validity

- Harness pack assembly must be byte-identical to production for the no-fault case (identity check against `ebound pack` on small tuning items per profile, the harness-cli-parity pattern of harness review round 1).
- Injected faults are synthetic, and real encoder bugs may be correlated with inputs (EXP-CODEC-005 incident census gives context only).
- In-process timing excludes file close and Defender scan-on-close (reported via the real-write arm).
- WSL `VIRTUALIZED` qualifier; Windows needs §14 item 3.

## 18. Platform and cost

| Item | Value |
|---|---|
| Platforms | Linux x86-64 WSL2 (detection and timing); Windows host (timing; decision scope covers both, §4.1). No network, no privileges. |
| Compute | Detection ≈ 175 items × 4 profiles × 5 fault models × 20 injections, each re-encoding one chunk and verifying the archive: ≈ 25 CPU-h. Timing: 5-7 scopes × about 200 (item, profile) cells in scope × about 15 rounds; dense/extreme small and medium packs dominate. ≈ 100 h exclusive on WSL and ≈ 100 h on Windows. |
| Disk | Scratch ≈ 10 GB (injected archives deleted after classification); raw < 1 GB. |
| Agent effort | Scripted; judgment only for the I31 scope linkage and the analysis (a session without the L7 exposure). |

## 19. Deviations (append-only)

None.
