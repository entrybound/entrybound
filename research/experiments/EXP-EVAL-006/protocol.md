# EXP-EVAL-006: Capability-claim verification probes against incumbents (tamper detection, encrypted names and index, size hiding, multi-recipient and PQ, signatures surviving recompression, metadata fidelity, safe extraction, streaming and access marks, ubiquity)

| Field | Value |
|---|---|
| Domain | evaluation (program sections 6, 7, 34, 35) |
| Status | NEEDS_TOOLING (probe drivers, fixture generators, format region mappers, extraction sandbox, tool acquisition; gates G-A and G-B for the decision-relevant Entrybound rows; §14 item 19 specialist configurations) |
| Kind | Deterministic capability probes (binary outcomes and counts over pre-registered case lists). Incumbent-only rows are capability evidence. Entrybound rows are decision-relevant. |
| External review | Security comparisons of constructions (for example Apple AEA's HKDF/AES-CTR/HMAC composition, commitment, misuse resistance) and the effectiveness of padding are **not** decided here. They go to the crypto domain's external-review dossier (§8.3). This experiment supplies only mechanically observed behaviour. |
| Blocked sub-arms | Apple AEA/AA and APFS rows: EXP-EVAL-013 (macOS `PLATFORM_BLOCKED`) |
| Method | `research/decision-method.md` §3.3, §4.13, §9.1 and §9.3 (primary-source rule; PROBE outranks DOC) at `14b977c`; `research/baselines/README.md` fair-comparison rules 1-5 |

## Author, independence and exposure

Designed by the Phase C-design evaluation-domain session. Every case list below is fixed here, before any probe result (T-20: case lists fixed before results). The L7 exposure is recorded in the EXP-EVAL-001 protocol; the F19 and F20 analyses run in an unexposed session.

## Question

Which comparative capability claims of SPEC §22.2-§23 survive a reproducible, capability-matched probe of pinned incumbent versions and of the REF-PROD build? The claims concern authenticated encryption, encrypted names and index, size hiding, multi-recipient and hybrid PQ encryption, signatures that survive recompression, metadata fidelity, safe extraction, streaming creation, random access, partial retrieval and preinstalled ubiquity. Where does an incumbent match or beat Entrybound?

## Hypotheses and falsification (per arm; each claim is falsified by a single reproducible counterexample under objective §2.0 rule 2)

- **A. Tamper detection (DEC-CRY-042).**
  - **Entrybound.** `refprod-balanced-indexed-enc-*` detects 100% of injected single-byte ciphertext flips in every region stratum (`undetected_tampers = 0`, HC-14). Falsified by any flip that yields wrong plaintext without a `CORRUPT` outcome.
  - **Incumbent predictions** (`INFERRED` from format documentation, to be probed):
    - ZipCrypto has undetected wrong-output flips (only a CRC-32 check);
    - WinZip AE-2 detects payload flips (HMAC-SHA1) but not flips in unauthenticated local-header or central-directory fields;
    - 7z AES detects no payload flips before CRC (no MAC; the CRC is inside the encrypted stream);
    - `tar | age` and `tar | gpg --symmetric` (AEAD) detect all payload flips.
  - Every prediction is falsifiable by the probe counts.
- **B. Names, index and size visibility (DEC-CRY-042).** Without the key:
  - `7z -mhe=on` hides names;
  - 7z without `-mhe` exposes names (default verification on the pinned release);
  - ZIP (both modes) exposes names and sizes;
  - `tar | age` hides names but exposes total length;
  - Entrybound INDEXED encrypted hides names and entry count.
  - Length leakage per padding mode: `leak_min_entropy_bits` of the length-only reference observer over the pre-registered size-set case list. `bucketed` and `max` are expected to leak fewer bits than `none` and than every incumbent. This row is labelled `NOT_DECISION_GRADE` until the OD-16 observer model is committed (§14 item 16).
- **C. Multi-recipient and PQ (DEC-CRY-043).** Predictions to probe:
  - `age` (pinned newest release) supports multiple recipients and, at versions that ship hybrid ML-KEM recipients, PQ encryption;
  - GnuPG 2.4.4 supports multiple public-key recipients but no PQ;
  - GnuPG 2.5.x (pinned build) supports Kyber hybrid;
  - 7z, RAR5 and ZIP support a single shared password only.
  - The claim "only Entrybound fully provides multi-recipient plus hybrid PQ among archive incumbents" is falsified for the class "archive plus file-encryption pipelines" if `tar | age` or `tar | gpg-2.5` passes both functional tests. The narrowed claim (archive formats only) is then the only survivable wording.
- **D. Signatures surviving recompression (DEC-CRY-044).**
  - **Entrybound.** A content signature (`ebound sign`) still verifies after `ebound repack` to another profile or layout, and the physical binding is reported stale rather than silently valid (`migration_identity_conformance_fraction = 1`, HC-10).
  - **Incumbent predictions:**
    - JAR/ZIP `jarsigner` signatures survive re-zipping at another level, because they sign per-entry uncompressed digests. That falsifies "only container layers offer signatures surviving recompression" if observed.
    - A cosign signature over a gzip layer blob fails after gzip-to-zstd recompression.
    - An EROFS fs-verity digest signature fails after rebuilding with another compressor.
- **E. Metadata fidelity comparison (DEC-PLT-024).** On the pre-registered class list (below), GNU tar 1.35 pax (`--xattrs --acls --selinux`) and bsdtar restore at least as many classes as Entrybound on Linux ext4. Entrybound is expected ahead only in declared-loss reporting (`declared_gap_count` explicit versus silent). Falsified per class by the restore fidelity fractions. The "full POSIX/Windows fidelity" mark is contradicted by any class Entrybound restores less exactly than the specialist.
- **F. Safe extraction (DEC-PLT-039).** With default flags, Entrybound refuses or neutralizes 100% of the hostile-structure cases (`hostile_name_handling_fraction = 1`, HC-08). At least one incumbent writes outside the destination on at least one case with default flags. Falsified by an Entrybound escape (HC violation, R1 artifact), or by every incumbent refusing every case. In that case the "structurally impossible versus validated" comparative claim is not supported on this case list.
- **G. Streaming, random-access and partial-retrieval marks (DEC-ACC-002).** Per configuration, binary probes:
  - (g1) creation from a non-seekable input stream;
  - (g2) extraction from a non-seekable input stream;
  - (g3) single-member extraction reading less than 10% of archive bytes for a member at the end of a 256-member archive (`/proc/<pid>/io` `read_bytes`);
  - (g4) verification of one member's integrity without reading the whole archive.
  - Predictions: tar pipelines g1/g2 yes, g3/g4 no; 7z non-solid g3 yes, g1/g2 no; ZIP g3 yes, g4 no (CRC only after full member read, no archive-level authenticity); squashfs and dwarfs g3 yes; Entrybound STREAM g1/g2 yes; Entrybound INDEXED g3/g4 yes.
  - Each mark is falsified by the probe.
- **H. Ubiquity (DEC-ECO-070 missing rows).** On stock Windows 11 and stock Ubuntu 24.04 server and cloud images, `.zip` and `.tar.gz` are consumable with preinstalled tools, `.7z` and `.tar.zst` are not on both, and `.eb` is not on either. Measured as `zero_install_scenario_fraction` (descriptive).

## Decisions informed

`DEC-CRY-042`, `DEC-CRY-043`, `DEC-CRY-044`, `DEC-PLT-024`, `DEC-PLT-039`, `DEC-ACC-002`, `DEC-ECO-069`, `DEC-ECO-070`, `DEC-ECO-031`.

## Candidates (configurations probed; versions pinned in run meta; new downloads recorded with URL, date, size and SHA-256 in `research/baselines/tool-versions.json` extension `probe_tools`)

| Arm | Entrybound | Incumbents |
|---|---|---|
| A, B | `refprod-balanced-indexed-enc-{bucketed,none,max}` (X-Wing recipient) | `7z-aes-mhe` (`7z a -p -mhe=on`), `7z-aes-nomhe` (`7z a -p`), `zip-zipcrypto` (`zip -e`), `zip-winzip-ae2` (`7z a -tzip -mem=AES256 -p`), `rar5-hp` (`rar a -ma5 -hp`; RAR for Linux trial binary, see Exclusions), `tar-age` (age pinned newest release binary), `tar-gpg-sym-aead` (GnuPG 2.4.4 `--symmetric --force-aead`), `tar-gpg-pub` (GnuPG 2.4.4), `borg-repokey-blake2`, `restic`, `gocryptfs` and `cryfs` (apt; ciphertext directory tamper; FUSE) |
| C | X-Wing multi-recipient pack; `ebound key add/remove` | `age` (multi `-r`; PQ recipient type if present at pinned version), GnuPG 2.4.4 and GnuPG 2.5.x (source build pinned), 7z/ZIP/RAR5 (password-only; functional test of the single-secret limit) |
| D | `ebound sign` then `ebound repack --profile dense`, `--layout stream` | `jarsigner` (Java 21, Windows host) re-zip at level 0 and 9; `cosign sign-blob --tlog-upload=false` over a gzip OCI layer, recompressed to zstd; `mkfs.erofs` (erofs-utils) + `fsverity digest` + openssl signature, rebuilt with `-zlz4hc` versus `-zlzma` |
| E | `ebound pack` then `ebound unpack --xattrs restore --acls restore --restore-owner` (WSL root); Windows `--windows-security restore` | GNU tar 1.35 pax `--xattrs --acls --selinux`, bsdtar 3.7.2 (WSL) and 3.8.8 (Windows), 7-Zip 23.01 `-snl`, Info-ZIP `-y`, squashfs-tools, DwarFS, borg, restic |
| F | `ebound unpack` defaults | `tar -xf` (GNU default), `bsdtar -xf`, `7z x`, `unzip`, Python `zipfile.extractall`, Python `tarfile.extractall` with the default filter of the pinned CPython |
| G | STREAM and INDEXED `balanced` | the 13 large-tier configurations of EXP-EVAL-001 |
| H | `.eb` | `.zip`, `.tar.gz`, `.tar.zst`, `.tar.xz`, `.7z` |

- **Exclusions and blocks.**
  - Apple AEA/AA and APFS fidelity: `PLATFORM_BLOCKED`, see EXP-EVAL-013.
  - `rar5-hp` creation requires the RARLAB trial binary. If the owner does not approve its evaluation licence, the arm falls back to reading the encrypted RAR5 fixtures present in `f20-tuning-real-libarchive-testsuite` with `unrar` (tamper on decryptable fixtures only). Otherwise it is recorded `BLOCKED: tool licence`.
  - WinZip itself: Windows GUI and commercial; the AE-2 format is produced by 7-Zip, and only format-level behaviour is claimed.
  - EROFS kernel mount: not required; digest computation only.
- **Status quo** for each claim: the SPEC §22.2 mark as written (the ledger candidate `status-quo-*`).

## Case lists (fixed now; T-20 `a = 0.5/n_cases`)

- **A.** 3 generated plaintext trees (seeded `text-v1` split into 16, 256 and 1,024 files; 64 KiB, 4 MiB and 64 MiB totals). 200 seeded single-byte flips per region stratum per artifact. Strata are mapped per format by `probes/region_map.py`: preamble/header, index/metadata/central directory, payload, trailer/footer. Where a format lacks a stratum it is `NOT_APPLICABLE`.
- **B.** The same trees for name visibility; a size set of 1,000 generated trees with total logical sizes drawn log-uniformly from 1 KiB to 64 MiB (seed 20260930) for the length-only observer.
- **C.** Functional tests: encrypt to 3 recipients and decrypt with each; add a 4th and remove the 1st; check whether payload ciphertext bytes change; generate a PQ or hybrid key and round-trip.
- **D.** One generated tree, 3 recompression variants per system.
- **E.** The metadata class list, fixed from independent observer capability on each platform before results (objective OD-03): mode bits, owner and group numeric ids, owner and group names, mtime (ns), atime, symlink target bytes, hardlink topology, POSIX ACL access and default, `user.*` xattrs, `security.selinux`, `security.capability`, sparse extents, empty directories, non-UTF-8 names. Windows: DOS attributes, ADS, security descriptor owner and DACL, creation time, reparse (symlink, junction). Inputs are `probes/make_fixtures.py` plus `f19-tuning-real-ext4-acl-selinux` and `f19-validation-real-xfs-acl-selinux` (metadata-bearing, tuning and validation only), plus an NTFS fixture built on the Windows host.
- **F.** The malicious-structure generators' cases from `f20-tuning-generated-malicious-structures` and `f20-validation-generated-malicious-structures`, plus `f20-validation-real-cpython-archivetestdata` and `f20-tuning-real-libarchive-testsuite` path cases. Each case is labelled with its hazard class (absolute path, `..` traversal, symlink-then-file, hardlink escape, device node, Windows reserved name, case collision).
- **G.** One 256-member generated tree of 256 MiB (seed 20260931), plus the incumbent large-tier artifacts.
- **H.** Scenario list S1-S5 of objective M26.1. Platforms: stock Windows 11 host inventory; the official Ubuntu 24.04 server and cloud image package manifests (primary source, fetched and hashed).

## Corpus selectors

Generated fixtures (above) plus these tuning and validation items only: `f19-tuning-real-ext4-acl-selinux`, `f19-validation-real-xfs-acl-selinux`, `f19-tuning-real-ntfs-metadata`, `f19-validation-real-ntfs-metadata`, `f20-tuning-generated-malicious-structures`, `f20-validation-generated-malicious-structures`, `f20-tuning-real-libarchive-testsuite`, `f20-validation-real-cpython-archivetestdata`, `f20-tuning-generated-private-vault`, `f20-validation-generated-private-vault` (existing encrypted fixtures, for reader-side checks).

## Environment

- `wsl-ubuntu` root, for arms A-G. Arm F runs inside `unshare --mount --user --map-root-user` with a tmpfs root, bind-mounted tool binaries, and canary files outside the destination. FUSE for gocryptfs, cryfs and the dwarfs/squashfs mounts.
- `windows-host` for D (`jarsigner`), E (NTFS), F (relative-traversal cases only, in a 64-level-deep throwaway directory with canaries; absolute-path cases WSL only), and H (inventory).
- No timing.

## Commands (all TO BE WRITTEN unless noted)

```sh
# tool acquisition: research/experiments/EXP-EVAL-006/acquire_tools.sh (age, cosign, gnupg-2.5 source, rar trial if approved) -> /root/eb-research/tools/eval006/ + sha256 record
# probes (each prints one JSON line per sample for ebr stdout_json):
#   probes/tamper_campaign.py     --tool T --mode M --tree DIR --scratch DIR --flips-per-region 200 --seed S
#   probes/region_map.py          --format F --artifact PATH -> region extents
#   probes/visibility_probe.py    --tool T --mode M --tree DIR  (names/sizes/count visible without key)
#   probes/length_observer.py     --tool T --mode M --sizes sizes.json -> leak_min_entropy_bits, anonymity_set_median (length-only reference observer)
#   probes/recipients_probe.py    --tool T
#   probes/signature_survival.py  --system {entrybound,jar,cosign,erofs}
#   probes/fidelity_matrix.py     --tool T --fixture DIR --platform {linux,windows} (reuses research/baselines/probes/make_fixtures.py)
#   probes/safe_extract.py        --tool T --case-dir DIR --sandbox unshare
#   probes/stream_access_marks.py --config C --tree DIR
#   probes/ubiquity_inventory.py  --platform {windows-host,ubuntu-24.04-server,ubuntu-24.04-cloud}
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-EVAL-006/spec.yaml'
# ebr run/verify-raw/normalize per arm spec; arm specs generated by research/experiments/EXP-EVAL-006/make_specs.py
# analysis: research/experiments/EXP-EVAL-006/claims_table.py -> research/normalized/EXP-EVAL-006/claims.csv
#   (claim_id, SPEC section, system, outcome SUPPORTED|QUALIFIED|CONTRADICTED|NOT_APPLICABLE|BLOCKED, evidence row keys, method PROBE|DOC)
```

`spec.yaml` is the arm A tamper campaign on the generated trees.

## Seeds

- `seeds: {order: 20260929, bootstrap: 20260929, command: 20260929}`.
- Flip positions derive from `command` seed plus the sample `rep`, which makes each repetition's flip set distinct and recorded.
- Size set: 20260930. Stream and access tree: 20260931.

## Warmup

0 (no timing).

## Measurement method and metrics

| Canonical metric | HC / OD | Role | Aggregation |
|---|---|---|---|
| `undetected_tampers` | HC-14 / OD-15 | binary for Entrybound; count for incumbents (claim evidence) | AGG-W |
| `authenticated_coverage_fraction` | HC-14 | binary for Entrybound; descriptive fraction for incumbents | AGG-W |
| `leak_min_entropy_bits`, `anonymity_set_median` | OD-16 | secondary, `NOT_DECISION_GRADE` until §14 item 16 | AGG-S per padding mode |
| `padding_overhead_pp` | OD-16 | secondary | per mode |
| `migration_identity_conformance_fraction` | HC-10 | binary (Entrybound) | AGG-C |
| `capture_fidelity_fraction`, `restore_fidelity_fraction` | OD-03 | primary for the fidelity-claim comparison, per class and platform pair | AGG-S (min over listed classes) |
| `declared_gap_count` | OD-03 | descriptive | per class |
| `cross_platform_restore_fidelity_fraction` | OD-18 | secondary (Linux-to-Windows and Windows-to-Linux pairs) | AGG-S |
| `hostile_name_handling_fraction` | HC-08 | binary (Entrybound); descriptive for incumbents | AGG-C |
| `streaming_capability` | HC-06/F-09 | binary per configuration | AGG-C |
| `zero_install_scenario_fraction` | OD-26 | descriptive | AGG-S |

Non-canonical descriptive fields, which never select: `released_before_detection_bytes` (plaintext emitted before a tamper was reported; relevant to I25), `names_visible_without_key`, `multi_recipient_ok`, `pq_hybrid_ok`, `payload_rewritten_on_recipient_change`.

## Replication and adaptive rule

- 2 repetitions per (configuration, case) with distinct seeded flip sets (§4.6 deterministic).
- Detection bound: 200 flips per stratum per artifact detects a per-flip undetected probability of at least 0.015 at 95% confidence (rule of three: 3/200). Totals over 3 trees x 2 repetitions (1,200 flips per stratum) detect at least 0.0025. These are stated bounds, not proofs (objective §2.0 rule 2).
- No adaptive extension.
- Any single Entrybound failure is an R1 artifact, committed without a confirming rerun.

## Normalization

`ebr normalize`, then `claims_table.py`. Fractions are computed over the fixed case lists (T-20), per class, stratum or hazard, and are never pooled across platform pairs or padding modes (AGG-S).

## Statistical analysis

No interval statistics: every metric is a deterministic count over a fixed case list. A claim is `SUPPORTED` if every probed counterexample search fails to falsify it and a PROBE row exists. It is `CONTRADICTED` if any case falsifies it. It is `QUALIFIED:<scope>` if it holds only on a named subset (profile, layout, version, flag). It is `BLOCKED` if the platform or tool is unavailable. DOC rows, meaning primary-source documentation at the pinned version (§9.3), are reported beside PROBE rows and never override them (README rule 3). For fidelity (arm E), the per-class differences between Entrybound and the specialist use T-20 `a = 0.5/n_cases`: a one-case difference is material.

## Practical-significance thresholds

- Binary metrics: §3.3.
- Fractions: T-20.
- Leakage: T-17. Reported only; not decision-grade before §14 item 16.
- `padding_overhead_pp`: T-16.

All as registered in `thresholds.json`.

## Sensitivity analysis

- Per stratum (A).
- Per hazard class (F).
- Per metadata class and platform pair (E).
- Per tool version where two versions are probed (GnuPG 2.4 and 2.5; 7-Zip Linux 23.01 and Windows current).
- With and without the `-mhe` and `--force-aead` flags, as the defaults-versus-configured views of DEC-ECO-031.

## Expected negative results worth recording

- `tar | age` matching Entrybound on tamper detection and multi-recipient support (and on PQ at a recent age version). This contradicts uniqueness wording.
- JAR signatures surviving recompression.
- GNU tar pax restoring SELinux and capability xattrs that Entrybound declares non-restorable, or refuses.
- 7z `-mhe` hiding names as well as Entrybound.
- DwarFS or squashfs single-member access matching INDEXED.

Every one of these narrows a SPEC mark. Each goes to the negative-results register and to EXP-EVAL-012's "not best" section.

## Threats to validity

- Region mapping is format-parser-dependent; `region_map.py` is cross-checked against each tool's own listing offsets.
- Flip campaigns on 64 MiB artifacts under-sample large payloads. Metadata strata are the claim-relevant part and are fully sampled.
- Sandbox escape risk in arm F: WSL-only for absolute paths, and canaries detect escapes. Windows relative cases run in a deep throwaway directory.
- Tool versions: Ubuntu apt versions lag upstream. Current-release claims use the pinned newest downloads, and both versions are reported.
- DOC-only rows for unavailable tools (AEA, RAR if blocked) cannot support a `SUPPORTED` outcome for comparative wording.

## Platform requirements

Linux x86-64 WSL2 root (`unshare`, FUSE, loop devices for F19 images); Windows 11 host non-admin (Java 21 `jarsigner`, NTFS fixtures); network for tool acquisition only (approved downloads); macOS for the AEA/AA/APFS rows (`PLATFORM_BLOCKED`, EXP-EVAL-013); external cryptographic review for construction-security comparisons (routed, not run).

## Estimates

- Machine time: about 30 h. Arm A dominates: about 14 configurations x 3 trees x 2 reps x about 800 flips x extraction.
- Disk: about 40 GiB.
- Agent effort: scripted execution; judgment for the claims table and for DOC citations at pinned versions (§9.3), by an unexposed session.
