# Corpus completeness critique, round 2 (ebrc-2026.09-v2)

Status: **ADEQUATE_FOR_TUNING_AND_VALIDATION: YES, WITH CONDITIONS.** Every one of the fifteen
use cases that round 1 marked `MISSING` now has at least a partial real workload; the seven
BLOCKERs and the F12 held-out-independence failure are closed or substantially closed. Two
BLOCKERs (G02 database dumps, G03 supply-chain/package-publishing) and one MAJOR (G17 CI
artifacts / F02 validation scale) were closed less completely than their own acceptance criteria
require, because the Docker-dependent sub-fixes were explicitly skipped again this round. The
validation-scale blind spot is gone at the corpus level (largest validation item is now 4.74 GiB,
largest held-out 7.10 GiB — a 1.5x gap, down from 7.7x) but persists in eight individual families.
Cross-split content leakage in F03 is unchanged in magnitude and is now managed by a pre-registered
covariate rather than removed; a **new, unaudited** cross-split content overlap appears in F17
(kernel-header trees) as a side effect of round 1's own G19 fix. **Held-out coverage is NOT yet
adequate for final evaluation**: the design freeze has not happened, `heldout-lock.json` is still
`draft`, the program-level held-out-exposure decision (PROGRESS.md "Open integration items" G01)
is unresolved, and several post-freeze audits are only pre-registered, not run.

Critic: adversarial completeness critic, Phase B1e round-2 (this session), 2026-09-17.

## 0. Inputs and method

| Input | Identity |
|---|---|
| `manifest.json` | `manifest_sha256` `ed28b1b47333c186a8c80781d378b4c149eaaceea09a65098182f4c062380392`, 300 items, `complete: true`, `split_audit.errors` empty, 11 warnings (labeling only, see §3) |
| `coverage.md`, `statistics.json` (203 tuning+validation stats, `missing_or_stale: []`), `licenses.md`, `heldout-lock.json` (`status: "draft"`) | all reference the same `manifest_sha256` above; no drift between the four files |
| `git log 12b1510..HEAD -- research/corpus` | 10 corpus commits: `6dbd162` (F20 structural), `d0448b1` (F20 vault), `9038874` (F19 metadata), `79b41dc`+`89740fd` (F09/F10/F11/F14/F16), `ef35c3d` (F19 home snapshots), `4aef874` (ebr fingerprint fix), `7aab572`+`f362105`+`f132128` (integrity fixes), `250f7a3` (round-1 assembly) |
| Round-1 findings | `research/corpus/critique-round1.md`: G01-G25, §1 use-case table, §2 family audit |
| Program use-case obligations | `research/requirement-ledger.csv` (same REQ-* set round 1 cited) |
| Program-level open items | `PROGRESS.md` "Open integration items" (RD-4 gates, `(G01) Held-out exposure`) |

Checks performed:

1. **Independent recomputation from `manifest.json`** of family x split x scale x
   real-or-generated cells (not transcribed from `coverage.md`), to verify every G01-G25 fix
   against the corpus as materialized, not as described in commit messages or `PROGRESS.md`.
2. **Re-read of every gap-closing group's `notes` field** in `research/corpus/sources/*.json`
   (`g1-code.json`, `g2-generated.json`, `g3-data.json`, `g4-binary.json`, `g5-media.json`), which
   self-report what each pass did and did not close, including explicit "not included" /
   "Docker unavailable" disclosures.
3. **Cross-split leakage remeasurement**, same method as round 1: exact-content SHA-256 overlap
   between tuning and validation only, computed from `/root/eb-research/fingerprints/{tuning,
   validation}/*.lines.gz` (203 items, up from round 1's 131). No held-out path, fingerprint
   listing or content was read for this measurement.
4. **`provision.py --check`** (definitions-only, no materialization): `OK: 300 item(s) in 5 source
   file(s)`.
5. **Independent full-fingerprint reverification** of 3 sampled items spanning tuning, validation
   and a held-out item added this round (`f17-tuning-real-ubuntu-kernel-headers-2ver`,
   `f20-validation-bombs-compressed`, `f19-heldout-real-btrfs-acl-selinux`): `provision: 3 ok, 0
   failed`. Held-out fingerprint reverification reads only bytes for hashing (identity), consistent
   with the "fingerprints only" held-out rule and with round 1's own 30-item spot-check method.
6. **`disk_guard.py`**: C: 372.8 GB free, D: 422.4 GB free, distro on D: — comfortably passing.
7. Scale-tier mismatch scan (declared `scale` vs `bytes` against `scale_tiers` boundaries) over
   all 300 items: 0 mismatches (round 1's single F20 mislabel, G23, is fixed).

## 1. Round-1 gap disposition

Severity and numbering as in `critique-round1.md`. "Evidence" cites item ids and bytes from the
current `manifest.json` unless noted.

| Gap | Severity | Verdict | Evidence |
|---|---|---|---|
| G01 ML/data distribution | BLOCKER | **CLOSED** | Real weights/checkpoints in every split with a large item: tuning `f07-tuning-pythia-160m-safetensors-step100000` (649,308,728 B, large), `f07-tuning-qwen2-5-1-5b-safetensors-bf16` (3,087,467,144 B, BF16), `f07-tuning-qwen2-5-0-5b-instruct-gguf-q4km` (491,400,032 B, GGUF quantized), `f07-tuning-mnist-npz`; validation `f07-validation-smollm2-360m-safetensors` (723,674,912 B, large) plus `bert-base-uncased` in 3 containers (bin/safetensors/h5); held-out `f07-heldout-phi3-mini-safetensors-p{1,2}of2` (4.97 + 2.67 GiB, BF16 sharded) and `f07-heldout-fineweb-edu-parquet-sample10bt` (2,152,819,114 B, dataset-shard parquet). Every acceptance bullet in G01 is met. |
| G02 database dumps + real-content validation DB | BLOCKER | **PARTIAL** | Real logical dumps now exist in every split: `f08-tuning-wikimedia-simplewiki-sql-dump` (783,019,629 B, mysqldump SQL), `f08-validation-ensembl-scerevisiae-core-sql-dump` (66,379,337 B, real MySQL table dumps), `f08-heldout-ebi-rfam-15-0-sql-dump` (2,564,211,859 B, real, large). Validation's real-content-DB gap is closed via `f08-validation-stackexchange-cs-sqlite` (606,966,077 B, derived-from-real, large) instead of the planned MariaDB-loaded Ensembl DB. **Not done**, self-disclosed in `sources/g3-data.json`: *"Docker Desktop's Linux engine was unavailable this session ..., so the Docker-dependent parts of G01/G02 (pg_dump derive of postgres17-pgbench, Ensembl loaded into MariaDB, employees mysqldump) are not included."* The G02 acceptance bullet "Tuning and validation also have a custom-format or directory-format dump" is **not met** — every new dump is plain-SQL/text format, none is `pg_dump -Fc`/`-Fd`. |
| G03 supply-chain verification + package-publishing | BLOCKER | **PARTIAL** | Use case U6 (supply-chain verification) is now closed with real artifacts in every split: `f11-cosign-v250-signing-assets` (keyless sigs, checksums, per-platform SBOMs), `f11-cpython-3137-release-set` (sdist + `.asc` + Sigstore bundle + SPDX SBOM + MSI components), `f11-heldout-firefox-15501-signing-metadata` (SHA256SUMS + `.asc` + KEY). Use case U2 (registry source packages: `.crate`, npm pack, PyPI sdist, apt `InRelease`/`Packages`) is **still absent** — `sources/g4-binary.json`'s own gap-closing note lists "F11 tuning ripgrep .crate files, an apt metadata snapshot, npm pack tarballs for bootstrap's lockfile, and PyPI sdists ... NOT closed in this pass," and no later commit adds them. |
| G04 real home/project backups | BLOCKER | **CLOSED** | `f19-tuning-real-home-snapshot` (real git clone of isaacs/node-mkdirp + mathiasbynens/dotfiles + a reused F13 PDF), `f19-validation-real-home-snapshot` (chalk/ansi-styles + holman/dotfiles + federal-register PDF), `f19-heldout-real-home-snapshot` (jonschlinkert/is-number + necolas/dotfiles + usgs PDF) — independent upstreams per split as required. |
| G05 F01 `.git` history, language breadth, validation large | MAJOR | **CLOSED** | Full-clone items with packfiles: `f01-tuning-zstd-v1-5-7-full-clone`, `f01-validation-cpython-3-13-15-full-clone` (1,047,462,722 B — closes F01 V:L), `f01-heldout-lua-5-4-8-full-clone`. JS/TS breadth: `f01-tuning-typescript-5-9-3-git` (391,819,141 B). |
| G06 real cross-platform metadata | BLOCKER | **CLOSED**, with a labeling nuance | Real filesystem constructs per split: NTFS (`f19-{tuning,validation,heldout}-real-ntfs-metadata`, distinct ADS/DOS-attribute/reparse-point/security-descriptor configurations per split, built with `mkntfs`+`ntfs-3g`), and Linux ACL/SELinux/capability trees on three distinct filesystems (ext4 tuning, XFS validation, btrfs held-out). macOS stays correctly `PLATFORM_BLOCKED` (not faked). **Nuance:** these items carry `real_or_generated: "generated"` in the manifest even though they are built with real filesystem tools and carry genuine metadata semantics (not fabricated data). This is a conservative, non-deceptive label choice, but it means `coverage.md`'s "real vs. generated by split" rollup understates F19's real-metadata coverage; an analyst reading only that table would not see this fix. |
| G07 encrypted private-archive workload | BLOCKER | **CLOSED**, one acceptance bullet unmet | One `private_vault` item per split (`f20-tuning-generated-private-vault`, `f20-validation-generated-private-vault`, `f20-heldout-generated-private-vault-v2`), each with seeded fake-identity documents, a KeePass KDBX4, OpenSSH/age/GPG keys, `.env` secrets, and legacy encrypted archives (ZipCrypto, 7z AES-256/`-mhe`, gpg, age). **Not done:** "add a large validation variant" — the validation vault is 424,910 B (small). |
| G08 F12 held-out independence + real camera JPEGs | BLOCKER | **CLOSED** | Held-out now has 2 independence groups (`g5-loc-highsmith`, 2 items, derived-from-real; `g5-commons-usgov-photos`, 3 items, **real**, not re-encoded by the tuning matrix) — the round-1 `FAIL` is reversed. Validation likewise gained `g5-commons-cc0-photos` (2 real items) alongside `g5-loc-fsa-owi-color`. The Commons fetch evidently succeeded on retry (items are still named `commons-*`, not the `openimages`/`raw-pixls-us` fallback). |
| G09 legacy ZIP/tar/7z/RAR/WIM/CAB corpus | MAJOR | **CLOSED**, sub-item (b) not done | `f14-legacy-writer-matrix-{tuning,validation,heldout}` (real source tree through 7z/zip/tar/cpio/WIM/CAB/Windows-host writers, ZIP64, non-UTF-8 names, tar dialects, over zstd/redis/musl respectively) — this closes the literal "zero `.7z` in tuning/validation" complaint, since the matrix itself emits real `.7z` files. **Not done:** G09(b), real *as-published* upstream `.7z` distributions in tuning/validation (no `tnwiki-*.7z`, no Notepad++ portable `.7z`, no PortableGit SFX kept as a `.7z`); `f09-git-for-windows-portable-2550` is the *extracted* PE tree, not the archive. Only held-out has a native upstream `.7z` source (`f11-heldout-7zip-2603-release-archives`, pre-existing). |
| G10 F16 OCI/ISO/other virtual-disk formats | MAJOR | **CLOSED** (formats substituted, requirement met) | OCI layouts in every split (`f16-oci-ubuntu-noble-multiarch-index` tuning, `f16-oci-fedora-44-zstd-layers` validation with zstd layers, `f16-heldout-oci-almalinux-10-whiteout-derive` heldout with whiteouts). ISOs: `f16-alpine-virt-3241-iso` (tuning), `f16-debian-13-netinst-iso` (validation, 792,723,456 B, large — closes F16 V:L), `f16-heldout-almalinux-102-boot-iso` (heldout, large). Other disk formats: VHDX (`f16-alpine-3241-minirootfs-ext4-vhdx`, tuning) and VMDK (`f16-heldout-freebsd-151-ufs-vmdk`, heldout, large) instead of the VDI/VHD named in the gap text — same U9 coverage goal met with an equivalent format. |
| G11 F05 telemetry/binary logs + scale | MAJOR | **CLOSED** | `f05-tuning-google-clusterdata-2011-2-task-usage` (large), `f05-validation-wikimedia-pageviews-20260801` (4,738,688,282 B, large — closes F05 V:L), `f05-heldout-backblaze-drive-stats-2025-q1` (7,097,027,357 B, large — closes F05 H:L, and is the corpus's single largest item), `f05-heldout-evtx-attack-samples` (binary EVTX), `f05-validation-gen-journal-systemd` (binary journald), plus per-split logrotate derives. |
| G12 real structurally adversarial archives | MAJOR | **CLOSED** | Real fixtures matching the fix list exactly: tuning `f20-tuning-real-libarchive-testsuite`; validation `f20-validation-real-commons-compress-testdata` + `f20-validation-real-cpython-archivetestdata`; held-out `f20-heldout-real-go-archive-testdata` + `f20-heldout-real-fastzip-malo` + `f20-heldout-real-fifield-zipbomb-regen`. F20 real-group counts per split rose from 0/0/0 to 1/2/3. |
| G13 F09 architecture/object-format skew | MAJOR | **CLOSED** | arm64/riscv64 added to tuning (`f09-alpine-324-{arm64,riscv64}-rootfs-binaries`, `f09-ubuntu-noble-{arm64,riscv64}-usr-binaries`, `f09-ripgrep-1411-arm64-darwin-multiplatform` for Mach-O) and validation (`f09-fedora-44-arm64-usr-binaries`, `f09-nodejs-v2220-arm64-multiplatform` covering darwin-arm64/linux-arm64/win-arm64), via skopeo OCI export — no Docker daemon needed, as the gap text suggested. |
| G14 F04 real many-small-file trees at 10^5-10^6 scale | MAJOR | **CLOSED** | `f04-tuning-real-netbsd-pkgsrc` (234,505 files), `f04-validation-real-gentoo-repo-20260915` (132,706 files), `f04-heldout-real-freebsd-ports` (177,413 files) — the round-1 "largest file count is 34.7k" complaint and the "0 real items" complaint are both resolved. |
| G15 F07 scientific formats beyond NetCDF/raw arrays | MAJOR | **CLOSED** | `f07-tuning-sdss-dr17-frames` (real FITS), `f07-tuning-ann-benchmarks-fashion-mnist-hdf5` (a second, non-NetCDF HDF5 container), `f07-validation-cmip6-ncar-cesm2-tas-zarr` (Zarr directory store), `f07-heldout-1000genomes-chr22-vcf-{bgzf,region-subset}` (VCF/BGZF genomics). BAM/CRAM/FASTQ/GeoTIFF remain absent, but the gap text did not ask for those. |
| G16 F03 cross-split package sharing + unaudited licenses | MAJOR | **PARTIAL** — see §2 for the remeasurement | License side: **closed**. `f03_license_audit.py` flipped 5 of the 14 `LicenseRef-mixed-per-package` items to `redistributable: true` with per-package evidence (`f03-tuning-{fzf,pypi-scientific,ripgrep,typescript}*`, `f03-validation-yq-go-mod-vendor`); the 2 that stayed non-redistributable (`bat-cargo-vendor`, `bootstrap-npm-node-modules`) did so because the audit found a specific unresolved/copyleft package, recorded in `licenses.md`. The 4 held-out F03 items and their F04 derives are correctly left un-audited pre-freeze. Content-overlap side: **not reduced**, only measured and covariate-flagged (see §2) — the same byte-level cross-split overlaps round 1 measured (yq/fzf, bat/ripgrep, bootstrap/typescript) are present at essentially the same magnitude today. |
| G17 F02 CI artifact breadth + validation large tier | MAJOR | **OPEN** | Only one item was added to F02 across every fix in this gap: `f02-validation-nodejs-22-20-0-win-x64-pdb` (the MSVC PDB, 361,672,704 B, medium). The TypeScript `hereby local` build output (tuning JS build artifacts), the CPython `--junit-xml` + coverage report, a second CPython build variant to reach the large tier, and the PostgreSQL `make check` + lcov report were **not added**. F02 validation's largest item is still 361,672,704 B (medium) — **F02 is the only family, of the ones the must-fix set targeted for scale, still without a real validation large item**, and coverage.md confirms `V:L` is `-` for F02. |
| G18 F15 sparse files real layouts | MAJOR | **CLOSED** | `f15-tuning-real-ubuntu-noble-ova-raw` (3,758,097,030 B, large), `f15-validation-real-debian-nocloud-raw` (3,221,225,739 B, large — also closes F15 V:L), `f15-heldout-real-almalinux-genericcloud-raw` (1,073,742,438 B, large), all `derived-from-real`, `SEEK_DATA`/`SEEK_HOLE`-verified. |
| G19 F17 real duplicate trees | MINOR | **CLOSED, but introduces a new unaudited leak (see §3)** | `f17-tuning-real-ubuntu-kernel-headers-2ver`, `f17-validation-real-fedora-kernel-devel-2ver`, `f17-heldout-real-almalinux-kernel-devel-2ver` — two ABI versions of kernel headers/devel packages per split, as specified. |
| G20 F06 columnar formats + scale | MINOR | **CLOSED** | `f06-tuning-nyc-tlc-yellow-tripdata-2024` (parquet) and `f06-validation-noaa-ghcn-daily-{csv,parquet}-2023` close both the columnar-format gap and F06's T:L/V:L cells. |
| G21 F13 pipeline repetition, benchmark tagging, validation large | MINOR | **CLOSED** | `f13-heldout-nasa-video-naca-large` (real, non-benchmark, large) added alongside the existing benchmark items; `f13-heldout-elephants-dream-media-medium` now correctly carries `tags: ["public-benchmark", ...]` matching `f13-heldout-tos-media-large` (round 1's inconsistent-tagging complaint fixed); every Blender item now carries `pipeline:blender-open-movie` as a reportable covariate; `f13-validation-sintel-media-large` (1,861,375,870 B) closes F13 V:L; phone-style HEVC/HEIC matrices added to validation and held-out. |
| G22 F18 versioned non-source series | MINOR | **CLOSED** | `f18-tuning-ubuntu-noble-cloud-images` (versioned binary images, large), `f18-validation-cpython-embeddable-3-14-series`, `f18-heldout-firefox-155-releases`. |
| G23 F20 scale label | MINOR | **CLOSED** | `f20-validation-bombs-compressed` is now `scale: "small"` (1,969,137 B); the scale-tier mismatch scan (§0.7) confirms 0 mislabeled items corpus-wide. |
| G24 F09 Silesia license inconsistency | MINOR | **CLOSED** | `f09-silesia-mozilla-ooffice` is `redistributable: false`, matching `f07-tuning-silesia-{mr,sao,xray}` and `f08-tuning-silesia-osdb` under the same `silesia-corpus` group; `licenses.md` groups all 5 under consistent non-redistributable entries. |
| G25 F10/F11 large tier | MINOR | **CLOSED** | `f10-linux-firmware-20260910-full-tree` (2,015,065,171 B, tuning large) and `f11-fedora-44-large-rpm-set` (708,538,475 B, validation large). |

## 2. Cross-split leakage remeasurement (tuning vs. validation, no held-out read)

Recomputed over the current 112+91=203 tuning+validation items (round 1 measured 131), using the
same SHA-256-content-hash method. 466 distinct cross-split item pairs share at least one byte; the
material ones are unchanged from round 1:

| Tuning item | Validation item | Shared bytes | % of tuning item | % of validation item |
|---|---|---:|---:|---:|
| `f03-tuning-ripgrep-cargo-vendor` | `f03-validation-bat-cargo-vendor` | 82,668,523 | 78.4% | 14.5% |
| `f03-tuning-fzf-go-mod-vendor` | `f03-validation-yq-go-mod-vendor` | 10,805,788 | 70.7% | 61.2% |
| `f03-tuning-typescript-npm-node-modules` | `f03-validation-bootstrap-npm-node-modules` | 13,961,514 | 9.0% | 4.3% |
| `f03-tuning-ripgrep-cargo-vendor` | `f01-validation-redis-7-2-16-git` | 3,193,954 | 3.0% | 20.5% |
| `f10-gen-redundant-binary-blobs` | `f15-validation-edgecases` | 1,048,576 | 6.9% | 7.3% |

These five are exactly what round 1 reported (the first three are the ones `f03_lockfile_audit.py`
and the `shared_packages` covariate pre-registration now document). **New this round**, not
previously measured or covariate-flagged anywhere:

| Tuning item | Validation item | Shared bytes | Files | % of tuning item | % of validation item |
|---|---|---:|---:|---:|---:|
| `f17-tuning-real-ubuntu-kernel-headers-2ver` | `f17-validation-real-fedora-kernel-devel-2ver` | 17,860,077 | 6,895 | 8.8% | 5.2% |

This pair exists because G19 (round 1, MINOR) added real Linux kernel header/devel trees to F17 in
three splits without checking pairwise content overlap the way the F03 fix did — kernel headers
for a given upstream version are largely identical byte sequences (macros, ioctl numbers, struct
layouts) regardless of the downstream distribution repackaging them. This is a real, non-trivial
(8.8% of the tuning item) cross-split overlap in a family whose entire criterion is duplicate-tree
structure, and no `shared_packages`-style covariate covers F17. `f01-tuning-typescript-5-9-3-git`
also shares ~7.7 MB (2.0%/2.4%) with the two bootstrap-derived validation items via a bundled JS
library, likewise unflagged, though smaller.

## 3. New problems (beyond the round-1 gap list)

1. **Unaudited new cross-split leakage in F17 (kernel-header trees).** See §2. Not a `split_audit`
   rule violation (the rule only forbids a *group* spanning splits, not *content* overlap between
   different groups), but it is exactly the kind of independence violation round 1's G16 was
   written to catch, and it was introduced by round 1's own G19 fix without a corresponding check.
2. **G02/G03 acceptance criteria not fully met, self-disclosed as Docker-blocked.** `pg_dump`
   custom/directory-format dumps, a MariaDB-loaded real-content InnoDB directory, registry source
   packages (`.crate`, npm pack, PyPI sdist) and apt repository metadata were all skipped again
   this round for the same reason as round 1 (`com.docker.service` needs admin). This is
   consistent with the program's standing Docker-unavailability constraint, not a new failure, but
   it means the corpus still cannot exercise a `pg_dump -Fc`/`-Fd` reconstruction workflow or a
   package-registry-publishing workflow (U2) in any split.
3. **G17 essentially untouched.** F02 (build trees / CI artifacts) got one item (a Windows PDB)
   out of five planned fixes. It is the only family in the round-1 "must-fix" scale list that still
   has no real validation-split large item (`coverage.md` F02 row: `V:L` = `-`).
4. **11 `split_audit` warnings are naming-only, not lineage-tracked.** Every warning is of the form
   *"item X's `independence_group` differs from its same-split derive source's group"* (e.g.
   `f04-validation-bootstrap-npm-modules` vs. its source `f03-validation-bootstrap-npm-node-modules`
   / `twbs-bootstrap`). These are same-split derives, so they do not violate the held-out
   independence rule, but an analyst who groups F04/F17/F19/F20 items by `independence_group` alone
   (without following `provenance`/description text back to the source item) would silently treat
   a byte-identical or near-identical derived tree as an independent sample. This list did not
   shrink from round 1 to round 2; it grew (11 vs. an unstated number, since round 1 predates most
   of these derive items).
5. **`real_or_generated: "generated"` undercounts real-metadata fixes.** See G06 in §1. Not a
   defect in the data itself, but `coverage.md`'s real/generated rollup by split is the only
   quick-look summary most future readers will see, and it will make F19 look less "real" than the
   underlying construction actually is.
6. **Corpus growth this phase modestly exceeds the informal ~60 GiB "new data" guidance.** Round 1
   totals were tuning 20.60 + validation 7.79 + held-out 17.96 = 46.35 GiB; round 2 totals are
   36.12 + 26.56 + 47.02 = 109.70 GiB, i.e. ~63.35 GiB added in this closure phase. The *binding*
   constraint (`disk_guard.py`, D: >= 75 GB free) is nowhere near threatened (422.4 GB free), so
   this is a budget-tracking note, not a corpus or storage defect.
7. **No new mislabeled-scale or mislabeled-family items found** (§0.7 scan: 0 mismatches), and **no
   new license inconsistency found** beyond the already-fixed Silesia case — both categories the
   task asked me to check specifically came back clean.
8. **No case found of generated data silently standing in for a real-data claim.** Every generated
   or derived-from-real item in the sampled families (F08 dumps, F19 metadata, F20 vault) is
   labeled and described accurately; the closest thing to a mislabeling risk is item 5 above, which
   understates rather than overstates realism.

## 4. Held-out coverage

- `heldout-lock.json.status` is `"draft"` — the held-out set is not frozen, correctly, since no
  `research/decisions/design-freeze.json` exists yet.
- Held-out large-tier families rose from 7 (round 1: F01, F02, F08, F13, F15, F16, F18) to 10
  (adds F05, F07, F12); validation large-tier families rose from 2 (F03, F18) to 11 (adds F01, F05,
  F06, F07, F08, F11, F13, F15, F16). F02, F04, F09, F10, F12, F14, F17, F19, F20 still have no
  real held-out-*and*-validation large item pair for a same-family scale comparison.
- Held-out statistics remain correctly fingerprint-only (`statistics.json.heldout.note`); no
  content statistics were computed for held-out items in this critique either, per program
  discipline.
- **PROGRESS.md's own open item stands unresolved:** *"(G01) Held-out exposure: held-out item
  identities appear in committed corpus sources, PROGRESS.md, and git history, and most items are
  public upstream data ... Before Phase D, decide on and build a sealed supplementary held-out set
  ... per `decision-method.md` §5."* This is a program-level decision, not something this critique
  can close, but it directly gates whether the current held-out set can be used for a final,
  uncontaminated evaluation.
- Pre-registered post-freeze work that has not run: the F03 `shared_packages` audit against the 4
  held-out F03/F04 items (`f03_shared_packages_covariate.md` §"Status"); the per-package license
  audit of those same 4 items plus `f17-validation-real-yq-vendor-8x`'s held-out analogue; no
  equivalent covariate exists yet for the F17 kernel-headers overlap found in §2 for held-out at
  all (it was only measured tuning vs. validation).
- Independent full-fingerprint reverification of one held-out item this round
  (`f19-heldout-real-btrfs-acl-selinux`) matched its recorded fingerprint (§0.5), consistent with
  round 1's 30-item, 0-mismatch spot-check — identity integrity is not in question, only content
  completeness and the exposure decision.

## 5. Verdicts

**ADEQUATE_FOR_TUNING_AND_VALIDATION: YES, WITH CONDITIONS.** No use case is `MISSING` any more
(round 1's five `MISSING` and two `MISSING (workload)` rows are all now at least `PARTIAL`/real);
all seven BLOCKERs and the F12 held-out-independence `FAIL` are closed or substantially closed;
tuning+validation statistics are complete (`missing_or_stale: []`); the validation-scale blind spot
is closed at the corpus level. Conditions before relying on results from the still-open areas:

- Do not treat F03 vendor-tree items (or the F17 kernel-header pair identified in §2) as
  independent samples without applying/extending a `shared_packages`-style covariate; report with
  and without the flagged items, per the existing F03 pre-registration, and build the equivalent
  check for F17 before any F17 aggregate result is reported.
- Do not draw an F02 (build-tree / CI-artifact) scale conclusion that requires a validation-split
  large item; none exists yet (G17 open).
- Do not exercise a `pg_dump` custom/directory-format restore path or a package-registry
  publishing workflow from this corpus; neither has a real fixture yet (G02/G03 partial).

**HELD-OUT COVERAGE: NOT YET ADEQUATE FOR FINAL EVALUATION.** Remaining work, in order:

1. Resolve the program-level held-out-exposure decision (PROGRESS.md `(G01)`): build the sealed
   supplementary held-out set, or explicitly accept and document the current exposure as a
   threat-to-validity, before Phase D.
2. Run the design-freeze process and populate `research/decisions/design-freeze.json`; only then
   unlock and freeze `heldout-lock.json` (currently `draft`).
3. Run the two pre-registered post-freeze audits: the F03 `shared_packages` measurement against
   the 4 held-out items, and the per-package license audit of those same items (plus
   `f17-validation-real-yq-vendor-8x`'s held-out analogue) — both are scripted and waiting.
4. Build and run an equivalent overlap check for the F17 kernel-header items against their
   held-out counterpart (`f17-heldout-real-almalinux-kernel-devel-2ver`), since none exists today
   even for the tuning/validation pair (§2), and held-out cannot be measured pre-freeze regardless.
5. Decide whether the nine families still lacking a held-out large item paired with a validation
   large item (F02, F04, F09, F10, F12, F14, F17, F19, F20) need one before any scale-dependent
   decision in those families is validated at held-out scale — this mirrors the validation-blind-
   spot problem round 1 found, one tier up.

## 6. What changed for the better since round 1 (summary)

Real-vs-generated groups per split rose across nearly every family that round 1 flagged (F04:
0/1/2 real groups -> 1/2/3; F07: 3/2/2 -> 8/5/5; F15: 0/0/0 -> 1/1/1; F17: 0/1/0 -> 1/2/1; F19: 0/0/0
real-metadata + 0 derived-from-real -> 1 derived-from-real group in every split plus real-tool-built
metadata in every split; F20: 0/0/0 -> 1/2/3 real groups). `provision.py --check` passes 300/300;
`statistics.json.missing_or_stale` is empty; the scale-tier mismatch scan is clean; the license
audit closed 5 of 14 previously-unaudited F03 items with evidence; the corpus's own largest item
(Backblaze Drive Stats, 7.10 GiB) and smallest-to-largest span now cover the full declared `large`
tier (up to the 8 GiB ceiling) in three different families.
