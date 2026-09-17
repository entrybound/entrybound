# EXP-PLAT-021: Primary-source cross-check of frozen platform registries, limits, time scales and mechanism availability (formal)

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-036, DEC-PLT-038, DEC-PLT-029, DEC-PLT-046, DEC-PLT-047, DEC-PLT-050, DEC-PLT-016, DEC-PLT-002, DEC-PLT-060, DEC-PLT-028, DEC-PLT-048, DEC-MOD-036
- **Program sections:** 15, 16, 21
- **Decision type (proposed, not assigned):** FORMAL (derivation from pinned primary sources with a committed counterexample-search protocol and independent reviewer sign-off, §1.2); R5 held-out validation not applicable (reason: no corpus content). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-21, OD-23 (normative text correctness inputs); HC-12, HC-16 (frozen identifiers are never edited in place; findings route to new registry revisions), HC-04 (unknown bits).
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** Do the frozen platform registries and bounds (Windows attribute mask 0x005af9a7, Windows ACE type registry 0x00–0x15, macOS flag mask 0x40bf80ef, NFS4 ACE4 masks 0x001f01ff/0x000000ff, WindowsReparsePointV1 tag handling, ACL/xattr/reparse bounds, Timestamp i64 seconds plus u32 nanoseconds) agree bit for bit with pinned primary sources; what are the documented filesystem limits, reserved-name lists, Unicode folding table versions and dev_t encodings; and at which OS/kernel versions do openat2 RESOLVE flags, O_NOFOLLOW_ANY, O_RESOLVE_BENEATH, Landlock ABIs and idmapped mounts exist?
- **H1:** Every bit in each frozen mask is defined by the pinned headers; at least one currently defined source bit (for example a newer Windows attribute or NFSv4.1 retention bit) is outside a frozen mask by design and must be listed as an explicit exclusion; the Microsoft reserved-name list includes COM¹–COM³/LPT¹–LPT³ and COM0/LPT0 variants that a naive list omits.
- **H0 / equivalence:** Masks and registries equal the union of source-defined bits exactly, and no documented reserved name is missing from the SPEC/doc lists.
- **Falsification of H1:** A frozen mask bit that no pinned source defines (a specification error), or exact equality with no exclusions anywhere.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-036 | V1_IMPORTANT | Are the frozen platform-security-v1 registry values correct and complete: Windows attribute mask 0x005af9a7, macOS flag mask 0x40bf80ef, NFS4 ACE4 permission mask 0x00... | Bit-level cross-check of every frozen mask and ACE type registry against windows-sys/win32metadata, MS-FSCC/MS-DTYP, xnu and RFC 8881. | Observed distributions (EXP-PLAT-011/013); reader behaviour for unknown bits (EXP-PLAT-019). |
| DEC-PLT-038 | V1_IMPORTANT | Which reparse tags are 'recognized' (mapped to Symlink entries) versus opaque ReparsePoints, is a recognized tag encoded as ReparsePoint noncanonical, does tag-2 data... | Enumeration of Microsoft reparse tags and payload layouts (MS-FSCC §2.1.2) with name-surrogate and directory bits. | Real payloads (EXP-PLAT-013); acceptance tests (EXP-PLAT-019). |
| DEC-PLT-029 | V1_IMPORTANT | Should the NFS4 ACL dialect remain frozen without any producing or consuming implementation, and which non-Linux ACL capture targets (macOS extended ACLs, FreeBSD/ZFS... | macOS ACL rights/flags (xnu `sys/acl.h`) mapped to RFC 8881 ACE4 bits; FreeBSD `sys/acl.h` NFSv4 constants. | Empirical macOS capture (EXP-PLAT-022, BLOCKED); FreeBSD ZFS observations (EXP-PLAT-011). |
| DEC-PLT-046 | V1_BLOCKING | How are character/block devices, FIFOs and sockets handled in v1: first-class EntryKinds (SPEC §3.3), auxiliary snapshot metadata, FidelityReport-only omission with en... | dev_t major/minor splits on Linux (kdev_t, glibc sysmacros), FreeBSD and macOS headers. | Round trips through tar formats (EXP-PLAT-009). |
| DEC-PLT-047 | V1_IMPORTANT | What minimum Linux kernel, macOS and Windows versions does v1 support for metadata capture and kernel-enforced confinement, and what fallback or report applies below t... | Primary release-note/man-page timeline for openat2 and RESOLVE_* (Linux 5.6), Landlock ABI versions, idmapped mounts, O_NOFOLLOW_ANY and O_RESOLVE_BENEATH (xnu tags). | Installed-base data (not measurable; analytical). |
| DEC-PLT-050 | V1_IMPORTANT | What is the normative time scale of Timestamp (Unix epoch, POSIX seconds without leap seconds, UTC), how are zone-less local times, leap seconds and values outside a t... | Normative time-scale definitions (POSIX 'Seconds Since the Epoch', NTFS FILETIME, HFS+ epoch, DOS date/time) and test vectors for negative, pre-1601 and post-2446 values. | Platform restore behaviour (EXP-PLAT-006). |
| DEC-PLT-016 | V1_IMPORTANT | Which file flags are captured and restored: a restorable subset of macOS UF/SF flags, exclusion of kernel-managed bits (SF_DATALESS, UF_COMPRESSED, SF_FIRMLINK), and a... | xnu UF_/SF_ flag definitions versus mask 0x40bf80ef; Linux FS_*_FL definitions. | Capture/restore (EXP-PLAT-004/008/022). |
| DEC-PLT-002 | V1_IMPORTANT | Are the frozen ACL bounds (at most 3 ACLs per Entry, 65,536 ACEs per ACL) justified, and may POSIX1E and NFS4 ACLs coexist on one Entry, and if so with what precedence... | Documented ACL size limits per filesystem (ext4 xattr block and ea_inode, XFS attr value limit, btrfs nodesize, NTFS ACL 64 KiB, ZFS). | Measured maxima (EXP-PLAT-011). |
| DEC-PLT-060 | V1_IMPORTANT | Are the frozen xattr bounds (at most 4096 attributes per entry, values at most 16 MiB, names 1-255 bytes) justified, or should they be derived from filesystem limits,... | Documented xattr maxima (Linux VFS XATTR_SIZE_MAX, ext4, XFS, btrfs, APFS/HFS+ resource forks). | Measured restores (EXP-PLAT-012). |
| DEC-PLT-028 | V1_BLOCKING | What exact rules map LogicalPath components to native names on each target (ext4, XFS, btrfs, APFS, NTFS, ReFS), and which components are refused or renamed: invalid U... | Microsoft 'Naming Files, Paths, and Namespaces' reserved list (version-aware) and path limits; Linux NAME_MAX/PATH_MAX. | Measured behaviour (EXP-PLAT-002). |
| DEC-PLT-048 | V1_BLOCKING | Which collision classes must extraction detect (case-fold, NFC/NFD normalization, Windows reserved aliases, trailing dot/space, 8.3 short names, confusables), against... | Unicode CaseFolding/normalization table versions (UCD 12.1.0 used by ext4 casefold utf8data, 15.0.0, 16.0.0) and differences between them for the case list. | Measured collisions (EXP-PLAT-002). |
| DEC-MOD-036 | V1_IMPORTANT | Which structural bounds belong in the model versus budgets and extraction policy: LogicalPath component length and depth, symlink target size (currently 1 MiB), and ca... | Documented component, depth, path and symlink-target limits per OS/filesystem. | Measured limits and corpus distributions (EXP-PLAT-002/003). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** The frozen values as written in docs/security-metadata-v1.md, docs/posix-metadata-v1.md, docs/format-v0.md and SPEC §10.
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-036** — candidates: `status-quo-frozen-masks`, `expand-masks-from-current-headers`, `opaque-unknown-bits-preserved`, `refuse-with-declared-loss` (status quo: `status-quo-frozen-masks`). Treatment: Not a selection experiment: candidates that depend on a registry, limit or version fact are checked against the derived table; any candidate whose rule contradicts a primary source is flagged for R1(b) review with the source citation as the counterexample.
- **DEC-PLT-038** — candidates: `undefined-status-quo`, `symlink-only-recognized`, `symlink-and-junction-recognized`, `registry-of-tags`, `remove-known-safe-option`, `guid-included-in-data`, `guid-excluded-separate-field` (status quo: `undefined-status-quo`). Treatment: Not a selection experiment: candidates that depend on a registry, limit or version fact are checked against the derived table; any candidate whose rule contradicts a primary source is flagged for R1(b) review with the source citation as the counterexample.
- **DEC-PLT-029** — candidates: `status-quo-frozen-unvalidated`, `validate-via-linux-nfs-client`, `macos-acl-adapter-v1`, `separate-macos-acl-dialect`, `defer-nfs4-post-v1`, `freebsd-zfs-adapter` (status quo: `status-quo-frozen-unvalidated`). Treatment: Not a selection experiment: candidates that depend on a registry, limit or version fact are checked against the derived table; any candidate whose rule contradicts a primary source is flagged for R1(b) review with the source citation as the counterexample.
- **DEC-PLT-046** — candidates: `status-quo-refuse-pack`, `omit-and-report-entry-scoped`, `first-class-kinds-spec`, `kinds-without-socket`, `aux-snapshot-metadata`, `policy-selectable-refuse-or-omit`, `post-v1-feature-bit` (status quo: `status-quo-refuse-pack`). Treatment: Not a selection experiment: candidates that depend on a registry, limit or version fact are checked against the derived table; any candidate whose rule contradicts a primary source is flagged for R1(b) review with the source citation as the counterexample.
- **DEC-PLT-047** — candidates: `status-quo-unstated`, `linux-5-6-openat2-minimum`, `linux-lts-5-10-minimum`, `macos-11-minimum`, `macos-latest-two`, `tiered-support-weaker-reported` (status quo: `status-quo-unstated`). Treatment: Not a selection experiment: candidates that depend on a registry, limit or version fact are checked against the derived table; any candidate whose rule contradicts a primary source is flagged for R1(b) review with the source citation as the counterexample.
- **DEC-PLT-050** — candidates: `status-quo-implicit-unix-utc`, `explicit-posix-time-no-leap`, `tai-based-scale`, `drop-timestamp-restorable-flag`, `validate-flag-against-registry`, `refuse-out-of-range-restore` (status quo: `status-quo-implicit-unix-utc`). Treatment: Not a selection experiment: candidates that depend on a registry, limit or version fact are checked against the derived table; any candidate whose rule contradicts a primary source is flagged for R1(b) review with the source citation as the counterexample.
- **DEC-PLT-016** — candidates: `status-quo`, `macos-user-flag-subset-restore`, `declare-linux-flags-unavailable`, `add-linux-flags-name`, `immutable-last-privileged-opt-in` (status quo: `status-quo`). Treatment: Not a selection experiment: candidates that depend on a registry, limit or version fact are checked against the derived table; any candidate whose rule contradicts a primary source is flagged for R1(b) review with the source citation as the counterexample.
- **DEC-PLT-002** — candidates: `status-quo-bounds-coexistence-allowed`, `forbid-mixed-dialects`, `platform-derived-ace-bounds`, `budget-only-bounds`, `keep-bounds-define-precedence` (status quo: `status-quo-bounds-coexistence-allowed`). Treatment: Not a selection experiment: candidates that depend on a registry, limit or version fact are checked against the derived table; any candidate whose rule contradicts a primary source is flagged for R1(b) review with the source citation as the counterexample.
- **DEC-PLT-060** — candidates: `status-quo-4096-16mib`, `linux-vfs-limits`, `budget-declared-only`, `per-platform-limit-table`, `status-quo-16mib-4096`, `linux-vfs-64kib`, `per-platform-maximum`, `large-values-as-content-objects` (status quo: `status-quo-4096-16mib`, `status-quo-16mib-4096`). Treatment: Not a selection experiment: candidates that depend on a registry, limit or version fact are checked against the derived table; any candidate whose rule contradicts a primary source is flagged for R1(b) review with the source citation as the counterexample.
- **DEC-PLT-028** — candidates: `status-quo-utf8-only-no-target-checks`, `refuse-per-target-hazard-list`, `refuse-portable-superset`, `declared-escape-renaming`, `native-bytes-on-posix`, `wtf8-surrogate-roundtrip`, `verbatim-long-path-prefix` (status quo: `status-quo-utf8-only-no-target-checks`). Treatment: Not a selection experiment: candidates that depend on a registry, limit or version fact are checked against the derived table; any candidate whose rule contradicts a primary source is flagged for R1(b) review with the source citation as the counterexample.
  - excluded before execution (ledger): `silent-sanitize` — Silent mangling makes faithful and relocated extraction indistinguishable; SPEC P8 requires refusal or a declared, reported renaming policy (violates SPEC §3.4 P8 L224 (frozen)); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-048** — candidates: `no-detection-status-quo`, `exclusive-create-only`, `pinned-unicode-full-casefold-nfc`, `probed-target-rules`, `filesystem-specific-tables`, `conservative-superset-plus-probe`, `confusables-advisory`, `confusables-refuse`, `enable-per-directory-case-sensitivity` (status quo: `no-detection-status-quo`). Treatment: Not a selection experiment: candidates that depend on a registry, limit or version fact are checked against the derived table; any candidate whose rule contradicts a primary source is flagged for R1(b) review with the source citation as the counterexample.
- **DEC-MOD-036** — candidates: `status-quo-bounds`, `platform-derived-bounds`, `budget-declared-only`, `format-limit-registry` (status quo: `status-quo-bounds`). Treatment: Not a selection experiment: candidates that depend on a registry, limit or version fact are checked against the derived table; any candidate whose rule contradicts a primary source is flagged for R1(b) review with the source citation as the counterexample.

## Corpus, fixtures and case sets

- **Sources (pinned in `sources.lock.json` before fetching):** `windows-sys` crate source at a pinned version and `microsoft/win32metadata` at a pinned commit (Win32 constants); MS-FSCC and MS-DTYP specification versions by publication date; `apple-oss-distributions/xnu` at a pinned tag (`bsd/sys/stat.h`, `bsd/sys/acl.h`, `bsd/sys/fcntl.h`, `bsd/sys/types.h`); FreeBSD `src` at a pinned release tag (`sys/sys/acl.h`, `sys/sys/types.h`); Linux kernel at a pinned tag (`include/uapi/linux/fs.h`, `include/uapi/linux/openat2.h`, `include/uapi/linux/landlock.h`, `include/linux/kdev_t.h`, `fs/unicode/`, `fs/ext4/`, `fs/xfs/libxfs/`), glibc `sysmacros.h`; RFC 8881 and RFC 7530 text; POSIX.1-2024 XBD §4.19; Unicode UCD 12.1.0, 15.0.0, 16.0.0; man-pages at pinned versions.
- **Counterexample search protocol:** exhaustive over every bit position of each 32-bit mask and every value of each small registry; for limits and version facts, every row requires two independent primary locators or is marked single-source.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_id:** `wsl-ubuntu` for fetching and parsing (no execution of fetched code). No timing.

| Requirement | Needed? |
|---|---|
| Linux (WSL2) | yes (parsing only) |
| Windows host | no |
| macOS | no (headers only) |
| x86-64 | yes |
| ARM64 | no |
| Network | downloads of pinned public sources (approved) |
| Privileged | no |
| Large storage | about 6 GB of source trees |
| External review | no (independent internal reviewer sign-off for FORMAL route) |

- **Storage discipline:** **PCP-6**.

## Commands

- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- `wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && PY=/root/eb-research/venv/bin/python && $PY research/experiments/EXP-PLAT-021/scripts/fetch_sources.py --lock research/experiments/EXP-PLAT-021/sources.lock.json --dest /root/eb-research/src/plat021 && $PY research/experiments/EXP-PLAT-021/scripts/registry_crosscheck.py --src /root/eb-research/src/plat021 --docs docs --out research/normalized/EXP-PLAT-021/decision/ && $PY research/experiments/EXP-PLAT-021/scripts/limits_table.py ... && $PY research/experiments/EXP-PLAT-021/scripts/mechanism_timeline.py ...'`
- Not runner-driven (no per-item samples); outputs carry the §12.3 provenance header (command, tool commit, source lock hash).

## Seeds, warmups and replication

- **Determinism:** the scripts run twice from a clean fetch; output tables must be byte-identical (repeat-hash).
- **Seeds:** none.

## Measurement method and metrics

- Per registry: bits defined by source, bits in frozen mask, symmetric difference with source locators.
- Per limit/version fact: value, unit, source locator(s), single- or dual-sourced.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `registry_mismatch_count` | OD-21 | secondary only (no Appendix A.2 band; decision-method §0.2 item 4 — reported, never eliminates or selects) | other | see definition | report | none | `registry_crosscheck.py` |

## Normalization

Constants normalized to integers; names to source spellings; every row carries `source`, `version`, `locator`.

## Statistical analysis

- FORMAL route (§1.2): the derivation tables plus the counterexample-search protocol are checked and signed off by a reviewer who did not produce them.
- A frozen value contradicted by a primary source is never edited in place (HC-16; objective §4.5): the finding routes to a new registry revision decision.
- Statuses: R5 and R7 not applicable (reasons recorded).

## Practical-significance thresholds

Not applicable (formal derivation; no practical-significance band).

## Sensitivity analysis

Not applicable (FORMAL; §8.2 condition 6 records 'not applicable' with this reason). Source-version sensitivity is reported: each table is regenerated against the next newer pinned source version as an informational arm.

## Held-out evaluation (Phase D placeholder)

Not applicable (FORMAL decision-type part; no corpus content).

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Excluded-by-design bits (for example directory, sparse and reparse authority bits removed from the attribute item under I1) are expected differences, documented as exclusions rather than errors.

## Threats to validity

- Header constants can differ from runtime behaviour; empirical experiments (EXP-PLAT-011/013) remain necessary.
- Microsoft documentation pages change without versioning; snapshots are hashed at fetch time.

## Estimated cost and effort

- **Compute:** about 1 machine-hours; **disk:** about 6 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-021/scripts/{fetch_sources.py,registry_crosscheck.py,limits_table.py,mechanism_timeline.py} and sources.lock.json (pinned URLs, tags/commits, SHA-256)
- **Agent effort:** scripted source parsing (about 1 machine-hour); judgment for mapping each source constant to a registry entry; independent reviewer sign-off required for the FORMAL route (§1.2).

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
