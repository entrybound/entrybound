# EXP-CRYPTO-005: Compression-context side channel under encryption (chosen-plaintext co-packing)

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

When the planner uses cross-file compression context (shared dictionaries, lookback groups, similarity cohorts) inside an encrypted archive, can a chosen-plaintext co-packer recover secret bytes of a co-packed victim file from public padded object lengths, in the manner of CRIME and BREACH? How much ratio does each candidate restriction (DEC-CRY-028) give up to stop it?

## 2. Hypotheses and falsification

- **H1.** Under status-quo planning (`*-enc-v1` inheriting the v6 cross-file context) and `NONE` padding, an adaptive co-packer confirms guessed 4-byte extensions of a secret token with `presence_advantage` `DISTINGUISHABLE` above 0 at the T-17 band. Full recovery of a 16-character base64 secret takes at most 10^4 archive creations on at least half of the victim contexts.
- **H2.** Under `BUCKETED` padding the advantage per query drops, but the query budget to full recovery grows by less than 10×.
- **H3.** `no-cross-file-context-when-encrypted` reduces the attack's advantage to `EQUIVALENT` with 0. Its `artifact_bytes` is `DISTINGUISHABLE_WORSE` than the status quo on F04, F05, F17 and F18, and `EQUIVALENT` on F09, F11, F12 and F13.
- **H0.** Cross-file context gives the co-packer no advantage above the band under any padding mode.
- **Falsification:** H1 fails if the status quo is `EQUIVALENT` to 0 advantage under `NONE` for every context. H3 fails if the restriction still yields `DISTINGUISHABLE` advantage (another channel exists) or is `EQUIVALENT` on size everywhere (the restriction is free).

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` attack lower bound (OD-16) plus size cost (OD-05). The claim that a restriction is sufficient is an external-review item.

## 4. Candidates and arms

| Arm | DEC-CRY-028 candidate | Implementation |
|---|---|---|
| C0 (reference) | `status-quo-same-planner` | production encrypted planning |
| C1 | `no-cross-file-context-when-encrypted` | research planner switch (research-internals, default-off): disable dictionaries, lookback groups and similarity cohorts when encrypting |
| C2 | `same-owner-cross-file-only` | research switch: context only among files whose source-owner label matches. Owner labels are synthetic, derived from the top-level directory of the context |
| C3 | `require-maximum-padding-for-cross-file` | C0 planning with `MAXIMUM` padding forced whenever any cross-file context object exists |
| C4 | `mandatory-compression-under-encryption` | research switch forbidding stored-only chunks (compression always attempted) |
| C5 | `profile-opt-in` | C1 for `fast` and `balanced`, C0 for `dense` and `extreme` (a scope partition, evaluated at R9 from per-profile results) |

Padding conditions: `NONE`, `BUCKETED`, `MAXIMUM`. Profiles: all four.

## 5. Corpus selectors

- **Victim contexts (tuning):** generated from tuning items by inserting a seeded secret file (formats: `.env` with a 32-hex API key, `id_ed25519`-style PEM block, JSON config with a 16-character base64 token) into:
  - `f04-tuning-maildir` (co-packer controls one mail message in the same maildir);
  - `f01-tuning-curl-8-19-0-git` (co-packer controls one source file);
  - `f19-tuning-real-home-snapshot` (co-packer controls one dotfile);
  - `f06-tuning-gen-structured-a-small` (co-packer controls one JSON record file).
  - 20 secrets per context; secret seeds in the spec.
- **Size cost (tuning):** every F01, F03, F04, F05, F06, F17 and F18 tuning item under C0-C4, encrypted, all profiles.
- **Validation:** the same construction on validation items (`f04-validation-objstore`, `f01-validation-redis-7-2-16-git`, `f19-validation-real-home-snapshot`, `f06-validation-gen-structured-b`) for one registered look.

## 6. Environment and platform requirements

`wsl-ubuntu`, Linux x86-64, `timing: false`. Planning is deterministic given inputs. Public lengths are computed exactly from private record lengths and the padding function, and a 1% sample of queries is checked against real encrypted packs (MVT-14(j)).

## 7. Commands and tooling

Tooling to build:
- research-internals planner switches C1, C2 and C4 (default-off; MVT-16(c) identity proof re-run);
- `ebr-crypto observe --planner-switch <c>` (as in EXP-CRYPTO-002);
- `research/tools/crypto/copack_attack.py`, an adaptive guess-extension loop over the observation oracle, with budgets of 10^2, 10^3 and 10^4 queries per secret.

`spec.yaml` holds one candidate per (arm × padding × profile). Each command runs the attack loop for one victim context and prints advantage, bits recovered, queries used and `artifact_bytes`.

## 8. Seeds, warmup, repetitions and adaptive rule

- 20 secrets per context; guess alphabets fixed by format; attack randomness seeded.
- Deterministic observations: 2 repetitions plus repeat-hash on a 1% sample.
- No warmups, no timing.
- Query budgets fixed in advance. No adaptive stopping on outcome.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `presence_advantage` (guess-confirmation test: TPR − FPR at FPR 0.01 for "guessed extension is correct") | T-17 | OD-16 | **primary**, per padding mode |
| `leak_shannon_bits` (secret bits recovered per 10^3 queries, estimated from the guess-confirmation channel) | T-17 | OD-16 | secondary |
| `artifact_bytes` | T-01 | OD-05 | **primary** for restriction cost |
| queries to full recovery | descriptive | — | reported |

## 10. Normalization and statistical analysis

- L1 is per victim context (secret-level results averaged).
- L2 is the independence group of the host item, then family and corpus per §4.9.
- Paired comparisons use the same secrets and contexts across arms.
- Welch-Satterthwaite corpus intervals and the family harm guard (§4.10-§4.12).
- Confirmatory W against S on validation.
- Size: exact item values; the T-01 relative band applies at the small tier.

## 11. Practical significance

`presence_advantage` and `leak_shannon_bits` per T-17, and `artifact_bytes` per T-01 (`research/methods/thresholds.json`). A new planner restriction under a new planner id is a T1 cost; §7.3 minimum gains apply.

## 12. Sensitivity analysis

§6 arms, plus: secret format and alphabet; co-packer control granularity (whole file versus appended bytes); query-budget truncation; profile.

## 13. Expected negative results worth recording

- Chunk-level keyed boundaries plus per-chunk compression already isolate the victim, so C0 is `EQUIVALENT` to 0 advantage. That would argue against paying C1's ratio loss.
- Similarity cohorts, not dictionaries, carry the entire channel, which points to a narrower restriction than C1.
- MAXIMUM padding (C3) removes the channel only for CONTROL objects while PAYLOAD dictionaries still leak.

## 14. Threats to validity

- The co-packer model assumes repeated archive creation over mostly unchanged content (backup-like). One-shot packing gives the attacker one query.
- Synthetic secrets and insertion positions.
- Exact-length modelling depends on the private-record accounting accessor, checked on the 1% sample.
- The attack is a lower bound. A more efficient adaptive strategy may exist.

## 15. Held-out placeholder (Phase D)

Candidates reaching Commit A run once on held-out host items with the same secret construction (seeded after unlock), under `EB_HELDOUT_UNLOCK`. No held-out item is named here.

## 16. Cost estimate and agent effort

- Compute: about 40 core-hours (planning-only oracle queries, about 1.2 million small-context plans) plus about 30 core-hours of size runs. Disk: about 30 GB transient.
- Agent effort: tooling **judgment** (planner switches, attack loop); execution **scripted**; analysis **judgment**.

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
