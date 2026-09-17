# EXP-ECO-012: Format identification and naming — magic, extension, media type, executable-name collisions, polyglots and scanner interoperability

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-012 |
| Domain | ecosystem (program §32; cross-cuts §11 container identity) |
| Status | READY (identification databases and tools are public downloads or apt packages; scripts specified below) |
| Runner | `spec.yaml` for the dynamic identification arm (identifier × test-file set); other arms are scripts |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |
| Timing | none |

## 1. Question

Do the candidate magic numbers, trailer magic, file extensions (`.eb`, `.eba`, detached-signature and key-file extensions) and executable names collide with, or get misidentified by, the identification systems users and preservation tools rely on (libmagic, shared-mime-info, PRONOM/DROID, Apache Tika, TrID, OS extension associations, web-server MIME maps, package namespaces); do transport corruptions of the magic get detected; how do archive tools and scanners treat `.eb` files and `.eb`/ZIP polyglots; and how many identification registrations currently exist?

## 2. Hypotheses and falsification

- **H1 (magic prefix freedom).** No rule at offset 0 in the pinned libmagic Magdir, shared-mime-info, DROID signature file, Tika MIME types or TrID definitions matches every file that starts with the status-quo magic `8E 45 42 31 0D 0A 1A 0A`, and no existing signature is a prefix of it. *Falsified* by any such rule (counted per database).
- **H2 (transport corruption is detected).** Every pre-registered transport transformation of the header (LF→CRLF, CRLF→LF, high-bit stripping, UTF-8 re-encoding of 0x8E, Ctrl-Z truncation, NUL insertion) makes the implemented reader refuse the file before interpreting payload. *Falsified* by any transformed file the reader opens as a valid archive.
- **H3 (extension collision).** `.eb` is already mapped to another format by at least one of the surveyed extension sources (for example EasyBuild easyconfig files), while `.eba` is mapped by none. *Falsified* if `.eb` is unmapped everywhere, or `.eba` is mapped somewhere.
- **H4 (glob versus magic).** In shared-mime-info, a registered `application/vnd.*` type with a magic rule at priority 80 identifies an `.eb`-named binary Entrybound file correctly even when a text type claims the `*.eb` glob, and the text type still wins for an easyconfig text file named `.eb`. *Falsified* if either identification is wrong.
- **H5 (polyglot misreading).** At least one incumbent ZIP reader lists entries from an `.eb` file whose stored (uncompressed) chunk content ends with a ZIP end-of-central-directory structure. *Falsified* if every ZIP reader refuses such files.
- **H6 (scanner opacity).** An anti-malware scanner that inspects ZIP and tar contents does not detect the EICAR test string stored inside an `.eb` entry. *Falsified* if it detects it.
- **H7 (executable names).** At least one of `ebound`, `entrybound`, `eb` is already a packaged executable or package name in at least one surveyed namespace. *Falsified* if none is.

## 3. Decisions informed

DEC-CON-017, DEC-CON-019, DEC-CON-021, DEC-CON-023, DEC-ECO-025, DEC-ECO-043 (scanner and file-manager interoperability), DEC-ECO-020 (namespace collision search), DEC-CRY-018 (`.ebsig` extension and media type collisions), DEC-CRY-040 (EBK1 key-file extension and text-form collisions).

## 4. Candidates

| Decision | Candidates evaluated |
|---|---|
| DEC-CON-017 | freeze-candidate-status-quo (`8E 45 42 31 0D 0A 1A 0A`; footer `8E 45 42 46 0D 0A 1A 0A`); alternative-high-bit-byte (first byte 0x89, 0x8F, 0x9E with the remaining seven bytes unchanged); longer-magic-with-version (12-byte form `8E 45 42 4F 55 4E 44 31 0D 0A 1A 0A`); ascii-only-magic (`EBOUND01`); generation-digit-tied-to-major and generation-digit-fixed-forever are semantics, measured only through H1 prefix checks of digits `1`-`9` |
| DEC-CON-019 | vnd-tree (test entry `application/vnd.entrybound`), prs-tree (`application/prs.entrybound`), no-registration-magic-only (magic rules only), layout-media-type-parameter (identification unchanged; noted), ebsig-own-magic-and-media-type versus ebsig-record-header-only (detached-signature files identified or not), plus-zip-structured-suffix (`+zip`: checked only as a collision with ZIP identification, since the container is not ZIP) |
| DEC-CON-021 | eb-status-quo; eba-fallback; longer-extension (`.ebound`); eb-with-magic-priority-registrations (H4); encrypted-specific-extension (`.ebx` test value) |
| DEC-CON-023 | status-quo-offset0-and-length; inspect-warning-foreign-signatures and refuse-known-polyglot-patterns (implemented as a prototype scanner `foreign_signatures.py` over declared extents: false-positive rate on benign archives and cost); document-out-of-scope |
| DEC-ECO-020 | status-quo-ebound-plus-alias; entrybound-only; ebound-only; short-eb |
| DEC-ECO-025 | none-before-freeze and the registration candidates: identification-support census only (M26.4); acceptance criteria are `EXTERNALLY_SOURCED` from each registry's published submission documentation |
| DEC-CRY-018, DEC-CRY-040 | extension values `.ebsig`, `.ebk`, `.ebk1`, `.ebpub` in the extension survey |

Excluded: browsers' download sniffing (no headless browser in scope; server `Content-Type` maps are measured instead); Windows Defender scanning of EICAR on the host (would trigger host security remediation; host security settings are not touched); submitting any registration (owner action); Software Heritage or GitHub code-search file counts (need authenticated APIs) — EasyBuild's public repository is counted instead.

## 5. Inputs

- Identification databases pinned at retrieval: `file` 5.45 (installed) and the upstream `file/file` repository `magic/Magdir` at HEAD; freedesktop `shared-mime-info` `freedesktop.org.xml` at its latest tag; The National Archives DROID signature file and container signature file (latest versions); Apache Tika `tika-mimetypes.xml` and `tika-app` jar (latest release); TrID definitions package (only if its published terms permit research use without click-through acceptance, otherwise excluded and recorded); IANA media-types registry XML and CSVs; nginx and Apache `mime.types`; Python `mimetypes` defaults; GitHub Linguist `languages.yml`; Windows host registry `HKEY_CLASSES_ROOT\.<ext>` (read-only `reg query`).
- Test files: for each magic candidate, 200 files = candidate header + seeded bodies (100 random, 100 derived from real status-quo `ebound pack` outputs of `f01-tuning-ripgrep-14-1-1-git` and `f04-tuning-sourcelike` with the header replaced); transport-transformed copies of the status-quo archives; EasyBuild easyconfig files from a public clone of `easybuilders/easybuild-easyconfigs` at HEAD (collision counts and H4 text case).
- Polyglots: `polyglot_gen.py --seed 2026091712` stores a ZIP file, a tar header block, an ELF header and a PE header as entry contents packed with a profile that stores them uncompressed (detected by `inspect --plans`), and places a ZIP EOCD within the final 64 KiB.
- Benign archives for foreign-signature false positives: tuning items `f14-jenkins-25683-war`, `f14-gen-nested-archives-py`, `f14-legacy-writer-matrix-tuning`, `f12-tuning-kodak-jpeg-edge-cases-small` packed in all four profiles. Validation look: `f14-gen-office-documents`, `f14-legacy-writer-matrix-validation`, `f14-pyspark-420-sdist`.
- Namespaces: Debian and Ubuntu `Contents-amd64` and package lists; Fedora `filelists`; Alpine `APKINDEX`; Arch `extra.files`; Homebrew formula JSON; `microsoft/winget-pkgs` manifests index (sparse checkout); Chocolatey community package search API; Scoop main and extras buckets; crates.io database dump; npm registry package documents; PyPI JSON API.

## 6. Environment and platform requirements

WSL2 Ubuntu 24.04 for all arms except the Windows registry query and Windows `tar.exe` polyglot listing (Windows host, non-admin). Java 21 in WSL (apt `openjdk-21-jre-headless`) for Tika and DROID. ClamAV (apt `clamav`, signatures via `freshclam` into `/root/eb-research/cache/eco012/clamav` without enabling the daemon). Network for downloads. Not privileged.

## 7. Procedure and commands

Scripts in `research/tools/ecosystem/identification/` (to be written):

1. `fetch_dbs.sh` — download and hash every source in §5; `sources.json`.
2. `static_prefix.py` — parse offset-0 (and offset ranges covering 0) rules from Magdir (via `file -C`-compiled test plus a Magdir parser), shared-mime-info XML, DROID XML byte sequences, Tika XML, TrID definitions; report rules that match every candidate-prefixed file and signatures that are prefixes of a candidate or vice versa; negative-offset (trailer) rule support per database.
3. `ebr run --spec spec.yaml` (dynamic identification: `file --mime-type -b`, `file -b`, `xdg-mime query filetype` against a scratch `XDG_DATA_DIRS` MIME database built with `update-mime-database`, `java -jar tika-app.jar --detect`, DROID command line with the pinned signature files, TrID if permitted) over the test-file sets listed in an ad-hoc manifest.
4. `transport.py` — apply the transformations of H2 to the status-quo archives; run `ebound verify` and `ebound list`; record class and reason code; also run libmagic with a draft Entrybound magic rule (`draft-magic/entrybound`) to see whether corrupted headers are still identified as Entrybound (desired: not identified).
5. `glob_magic.py` — build a scratch MIME database with a candidate type (magic priority 50 and 80; glob weights 50 and 60) and a competing `text/x-easybuild` type claiming `*.eb`; query the four (name, content) combinations; repeat for `.eba`.
6. `extensions.py` — look up each extension value in every extension source; count EasyBuild `*.eb` files at HEAD.
7. `polyglot.py` — for each polyglot, run `unzip -l`, `bsdtar -tf` (WSL and Windows `tar.exe`), `7z l`, `python3 -m zipfile -l`, Java `ZipFile`, `file`, Tika, and `ebound verify`; record which interpret it as a foreign format. `foreign_signatures.py` prototype over benign archives: flagged count, scan time (descriptive).
8. `scanner.py` — EICAR string in a stored `.eb` entry, in a compressed `.eb` entry, and control copies in `.zip` and `.tar.gz`; `clamscan --scan-archive=yes`; record detections.
9. `namespaces.py` — exact-name and executable-path searches for `ebound`, `entrybound`, `eb` in every namespace source.
10. `registrations.py` — identification support census: whether any entry for Entrybound exists in IANA, Magdir, shared-mime-info, PRONOM, Tika, Linguist (expected none at design time), plus the published submission requirements and turnaround statements with URL and access date (`EXTERNALLY_SOURCED`).

## 8. Seeds, warmup, replication

Seeds `order` 2026091712, `bootstrap` 2026091712, `command` 2026091712; test-file bodies from seed 2026091712. Deterministic tool outputs: 2 repetitions + repeat-hash per (identifier, file set). No warmups.

## 9. Metrics and measurement

| Metric | Canonical | Role |
|---|---|---|
| `identification_support_count` | yes (M26.4) | descriptive |
| `benign_false_refusal_fraction` | yes (T-20, M17.3) | banded (minimize): refuse-known-polyglot-patterns prototype on benign archives |
| `magic_prefix_collisions`, `misidentified_files`, `transport_corruption_undetected`, `glob_magic_correct`, `extension_mappings`, `easybuild_eb_file_count`, `polyglot_foreign_interpretations`, `scanner_detections`, `namespace_collisions` | no | descriptive record fields per candidate |

`transport_corruption_undetected` > 0 for the status quo is also recorded as HC-15 counterevidence (a corrupted file opened without a failure).

## 10. Normalization and aggregation

AGG-P per candidate for database facts; AGG-S per identification system (headline: worst system); benign false-refusal fraction per family with minimum-over-families headline.

## 11. Statistical analysis and use in the decision rule

Deterministic facts, no inference. For DEC-CON-017 a candidate magic with a prefix collision or with an undetected transport corruption is excluded at R1 against the HC-04/HC-15 fail-closed requirements (a corrupted or foreign file must not be interpreted). DEC-CON-023 prototype scanning candidates are compared on `benign_false_refusal_fraction` (T-20) with confirmatory validation look. All other outcomes are descriptive inputs to owner and external-registration decisions (§8.3: registrations with external authorities are `EXTERNAL_REVIEW_REQUIRED`).

## 12. Practical-significance thresholds

T-20 for `benign_false_refusal_fraction`; none for descriptive fields (Appendix A.2).

## 13. Sensitivity analysis

Draft magic rule strength variants (libmagic `!:strength` +0, +50); DROID with and without container signatures; namespace search exact versus case-insensitive; polyglot EOCD at 0, 1 KiB and 64 KiB from the end.

## 14. Held-out (Phase D placeholder)

Conventions §8: the benign-archive false-refusal comparison runs once on held-out F14 items after unlock. Database facts are re-fetched at the freeze and changes reported.

## 15. Expected negative results worth recording

`.eb` collides with EasyBuild; `application/vnd.*` registration timelines longer than a release cycle; scanners are blind to `.eb` contents; some ZIP readers accept `.eb`/ZIP polyglots.

## 16. Threats to validity

Identification databases change continuously; results are at the access date. Test bodies with replaced headers may trigger secondary rules differently from real archives. Windows extension associations on one host reflect installed software, not a clean OS.

## 17. Cost estimate

About 6 h compute; about 20 GB disk (Magdir, DROID, Tika, EasyBuild clone, namespace indexes, winget sparse checkout); agent effort scripted plus judgment for the Magdir and TrID parsers' validation (one session).

## 18. Gates and exposure

Conventions §1 and §2. F04, F12 are in the exposure record; the benign-false-refusal analysis is run by an unexposed session.
