# EXP-CRYPTO-007: Encrypted public-byte leakage inventory and count inference

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

In a crypto-v1 encrypted archive, which bytes are public, what plaintext-derived information does each public field carry, and how accurately can an observer infer entry count, unique chunk count, total logical size and within-archive duplication from them? Do the documented privacy claims survive: metadata-private, entry count hidden, per-entry sizes hidden, no plaintext digests exposed? Also, what do detached signatures, content-dependent feature bits and recipient fields add, and how does the status quo compare with restic, borg, age, gpg and 7z encrypted headers on the same inputs?

## 2. Hypotheses and falsification

- **H1 (inventory).** Every public field falls in exactly one class: constant, option-derived, content-derived or random. The content-derived fields are exactly those listed in `docs/crypto-threat-model-v1.md` "Deliberately public information" (L165-192). A content-derived public field missing from that list falsifies H1 and is recorded as an undocumented leak (DEC-CRY-030).
- **H2 (no plaintext digests).** A byte scan of every public region finds zero occurrences of any plaintext-derived digest (LAI, AUX, PCR, chunk SHA-256, file SHA-256, logical digests) and zero plaintext substrings of 8 or more bytes. Any hit falsifies the SPEC §9.8 claim, is an R1-relevant finding under HC-14, and is escalated.
- **H3 (count inference).** Under status-quo BUCKETED padding and one-object-per-segment layout, an estimator trained on tuning archives predicts entry count within a factor of 2 for at least 80% of validation archives and unique chunk count within ±10% for at least 80%. The "entry count hidden" and "per-entry sizes hidden" claims are then quantised, not hidden (DEC-CRY-026 `downgrade-to-quantised`).
- **H4 (detached signatures).** A detached `.ebsig` over an encrypted archive lets an observer who holds a candidate file set confirm the archive's logical content with `presence_advantage` `DISTINGUISHABLE` above 0 whenever the candidate set contains the exact tree, as `docs/crypto-suite-v1.md` L684-686 documents.
- **Falsification:** H3 is falsified if inference accuracy is below those rates, which would support the status-quo claims as worded.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` (inventory, byte scan, inference accuracy) plus `FORMALLY_DERIVED` field classification from `docs/crypto-wire-v1.md` with line citations. Acceptance of residual leakage is an external-review input (SPEC §25.2, dossier EXP-CRYPTO-024).

## 4. Candidates and arms

**Archive option factorial (status-quo writer):**

| Factor | Levels |
|---|---|
| Protection | X-Wing recipients 1, 2, 16, 128, 1,024; password (one stanza) |
| Boundary mode | default (secret Gear), `keyed-prf` (PHTE) |
| Padding | `none`, `bucketed`, `max` |
| Signatures | none; embedded (`sign --embed`); detached `.ebsig` |
| Content-dependent metadata | trees with and without symlinks (POSIX feature 0x8000); captured on `windows-host` (platform security 0x10000, F-0001) and on `wsl-ubuntu` |
| Profile | `fast`, `balanced`, `dense`, `extreme` |

**Candidate policies (as exact observation transforms, or research writer where noted):**
- DEC-CRY-030: `status-quo-documented-public-framing`; `hide-signature-presence-new-version` (0x200 removed from the public bitmap; transform); `pad-record-counts` (dummy records to quarter-octave count buckets; transform); `maximum-padding-default` (P6); `declare-accepted-leakage` (inventory only).
- DEC-CRY-026: `status-quo-claims`; `downgrade-to-quantised`; `coarser-control-padding`; `publish-leakage-inventory`; `keyed-public-digests` (the byte scan tests whether any unkeyed public ciphertext digest links archives, for example footer or context digests shared between sibling archives after `key add`).
- DEC-CRY-064: `status-quo-public-count-types`; `dummy-stanza-padding` (stanza count bucketed: 1, 2, 4, 8, … 1,024; size and unlock cost in EXP-CRYPTO-021); `uniform-stanza-encoding` (transform: all stanza types padded to the maximum stanza size); `restrict-keyless-reporting` (CLI output census); `document-only`.
- DEC-CRY-008: `status-quo-content-dependent-bits`; `always-set-under-encryption`; `private-declaration` (transform: generic public bit); `document-as-public-leak`.
- DEC-CRY-081 / DEC-CRY-079: layouts L0-L4 as in EXP-CRYPTO-002 (count-inference impact).
- DEC-CRY-019 / DEC-MOD-012: detached versus embedded signatures; `keyed-lai-commitment` and `encrypted-roots-only` (formal: the public linkage disappears by construction; the residual is measured).
- DEC-CRY-025: `status-quo-single-archive-domain` versus `no-dedup-option` (inference of within-archive duplication from PAYLOAD object count versus total logical size).

**Incumbent references (same trees):**
- `restic` 0.16.4 (pack files and index sizes);
- `borg` 1.2.8 `repokey` with `none`, `lz4` and `obfuscate`;
- `age` 1.1.1 over a tar stream;
- `gpg` 2.4.4 symmetric and pubkey over a tar stream;
- `7z` 23.01 `-mhe=on -p`.

The same inventory schema is applied to each.

## 5. Corpus selectors

- **Tuning:** `f04-tuning-sourcelike`, `f04-tuning-maildir`, `f04-tuning-real-netbsd-pkgsrc`, `f01-tuning-curl-8-19-0-git`, `f03-tuning-ripgrep-cargo-vendor`, `f05-tuning-gutenberg-books`, `f09-alpine-324-rootfs-binaries`, `f11-tuning-ubuntu-noble-debs`, `f13-tuning-librivox-audiobook-medium`, `f17-tuning-duptree-mixed`, `f18-tuning-zstd-releases-1-5-x`, `f19-tuning-zoo`, `f19-tuning-real-ntfs-metadata`, `f19-tuning-real-ext4-acl-selinux`, `f20-tuning-generated-private-vault`, plus generated trees with controlled entry counts (10^1 to 10^6, log-spaced, `text-v1` files of seeded sizes) for the inference training grid.
- **Validation:** `f04-validation-objstore`, `f04-validation-real-gentoo-repo-20260915`, `f01-validation-redis-7-2-16-git`, `f03-validation-yq-go-mod-vendor`, `f05-validation-rfc-texts`, `f09-fedora-44-usr-binaries`, `f11-maven-central-jvm-jars`, `f13-validation-librivox-audiobook-medium`, `f17-validation-duptree-vendor`, `f18-validation-redis-7-2-point-releases`, `f19-validation-deep`, `f19-validation-real-ntfs-metadata`. One registered look; the inference estimator is frozen before it.

## 6. Environment and platform requirements

- `wsl-ubuntu` for most cells.
- `windows-host` for Windows-captured metadata bit cells (F-0001): the Windows `ebound` build packs the NTFS-image-derived trees mounted read-only through `ntfs-3g` in WSL and then copied to NTFS scratch. The procedure is recorded in the run README.
- `timing: false`.

## 7. Commands and tooling

Tooling to build:
- `research/tools/crypto/public_inventory.py`: a clean-room parser of crypto-v1 public framing written only from `docs/crypto-wire-v1.md` by a session not involved in the production writer. It emits `(offset, length, field, value, class)` rows and passes every public byte through a coverage check: every byte is attributed to exactly one field, otherwise the run fails.
- `research/tools/crypto/bytescan.py`: builds the plaintext-derived digest dictionary per archive from the plaintext tree (SHA-256 of files and chunks via `ebr-crypto observe --emit-digests`; LAI/AUX/PCR from `ebound inspect --json` with keys) and scans public regions (Aho-Corasick over digests and all 8-byte plaintext windows sampled at 1 in 64 offsets).
- `research/tools/crypto/count_inference.py`: quantile-regression and gradient-boosting estimators over public features, trained on tuning only.
- `research/tools/crypto/incumbent_inventory.sh`: the same schema for restic, borg, age, gpg and 7z outputs.
- `ebr-crypto keygen` (1,024 test recipients).

`spec.yaml`: candidates are option cells. The command packs, parses, scans and prints the inventory summary JSON.

## 8. Seeds, warmup, repetitions and adaptive rule

- Deterministic public structure given options: 2 repetitions (fresh randomness each). Field classes and lengths must agree. Random fields must differ, which gives a negative check for accidental determinism (HC-14 "no deterministic ciphertext").
- Inference estimator seeds are fixed. Tuning uses 5-fold cross-validation grouped by independence group.
- No warmups, no timing.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `leak_min_entropy_bits` per public field and per archive (uniform prior over the tuning population of the family, deterministic fields) | T-17 | OD-16 (M16.1) | **primary** |
| `anonymity_set_median` over entry-count classes and over trees (known-tree universe) | T-17 | OD-16 | secondary |
| `presence_advantage` (known-tree confirmation from counts and lengths; from detached signatures) | T-17 | OD-16 (M16.2) | **primary** for H4 and the count-based fingerprint |
| byte-scan hits | binary (HC-14-relevant finding) | OD-15/OD-16 | screen |
| unattributed public bytes | binary (tool validity) | — | screen |
| inference error ratios (entry count, unique chunks, logical size) | descriptive | — | reported |
| `metadata_bytes` of CONTROL objects | T-02 | OD-05 | diagnostic |
| `fixed_overhead_bytes` of the envelope by recipient count | T-21 | OD-05 | diagnostic (EXP-CRYPTO-021 owns recipient cost) |

## 10. Normalization and statistical analysis

- **Inventory and byte scan:** exact census, no statistics.
- **Channel metrics:** exact for deterministic fields. Aggregation per §4.9 on absolute differences in bits (absolute-only metric).
- **Inference:** per-archive error, group-level accuracy fractions, family and corpus means. Intervals per §4.10 on group-level values.
- **Candidate comparisons** (for example `pad-record-counts` against the status quo): paired per archive; corpus Welch-Satterthwaite; confirmatory W against S at validation.
- **Incumbents:** a reference arm (REF-INC) reported beside every external comparison claim (§6.5). No superiority claim unless capability-matched: only encrypted, authenticated incumbents are compared on leakage.

## 11. Practical significance

T-17 (`leak_min_entropy_bits`, `anonymity_set_median`, `presence_advantage`), T-02 and T-21 from `research/methods/thresholds.json`. Wire-changing candidates (`hide-signature-presence-new-version`, `private-declaration`, `uniform-stanza-encoding`, `keyed-public-digests`) are at least T2 and possibly T4 (crypto construction), with the assessor computing the tier (§7.2).

## 12. Sensitivity analysis

§6 arms. Experiment-specific:
- prior (uniform versus population-frequency);
- estimator family (quantile regression versus gradient boosting);
- training-grid composition (generated only versus generated plus real);
- F-0001 host arm (Windows versus WSL capture).

## 13. Expected negative results worth recording

- Entry count is inferable within quarter-octave classes from CONTROL padded lengths. "Entry count hidden" is then contradicted as worded (DEC-CRY-026).
- The embedded-signature bit 0x200 reveals signing practice publicly.
- Content-dependent bits 0x8000/0x10000 reveal source-platform class (Windows versus Linux) for every Windows-created encrypted archive.
- restic or borg leak strictly more (per-chunk sizes) than Entrybound BUCKETED. A context claim, recorded with its capability match.
- A sibling-archive linkage exists through an unkeyed public digest after `key add`.

## 14. Threats to validity

- The clean-room parser could share a misreading with the production writer. Mitigated by the full-byte attribution check and by a second session reviewing field classification against the doc lines.
- Population priors from tuning items are not real-world priors.
- Inference estimators are lower bounds on inference power.
- Windows-capture arms depend on the copy procedure preserving security descriptors (checked by `ebound inspect` on the source tree).

## 15. Held-out placeholder (Phase D)

The frozen inventory tool, estimator and candidates run once on held-out items after unlock (`EB_HELDOUT_UNLOCK`). Byte-scan and inventory checks are also R1-relevant on held-out. No held-out item is named here.

## 16. Cost estimate and agent effort

- Compute: about 40 core-hours (option factorial on about 27 items, incumbents, inference training). Disk: about 60 GB transient and 5 GB retained.
- Agent effort: tooling **judgment** (clean-room parser by an independent session); execution **scripted**; analysis **judgment**.

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
