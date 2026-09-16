# Corpus completeness critique, round 1 (ebrc-2026.09-v1)

Status: **INCOMPLETE.** Five of the fifteen Entrybound use cases have no representative workload:
ML model/data distribution, database dumps, supply-chain verification, backups of home/project
directories, and real cross-platform metadata. Two more are covered only by in-house generators:
encrypted private archives and structurally adversarial archives. F12 fails the held-out
independence requirement. Validation has no workload larger than 575 MiB. Held-out has large-tier
items in seven families, so no scale-dependent decision can be validated before the held-out
evaluation.

Critic: adversarial completeness critic, Phase B1d (`wf_3615a3d7-fe4`), 2026-09-16.

## 0. Inputs and method

| Input | Identity |
|---|---|
| `manifest.json` | `manifest_sha256` `a71e7ee196520b2813c686b587d135fa1f0f8e5f185cb50b77352eb9fbbfa4ca`, 196 items, `complete: true`, `split_audit.errors` empty, 4 warnings |
| `coverage.md`, `statistics.json` (131 tuning+validation stats, none missing or stale), `licenses.md`, `sources/g1..g5*.json` | as committed at `3de5862` |
| Group problem reports | `git log --format=%B -- research/corpus`: commits `10743d7` g1, `d961869` g3, `4f9ca47` g4, `0760fb1` g5 (via `--grep g5`), `bf2a608`..`8f0c749` g2, `6400fc3` assembly |
| Program use-case obligations | `research/requirement-ledger.csv`, in particular REQ-CMP-0113, REQ-CMP-0149, REQ-LEG-0078, REQ-LEG-0104, REQ-LEG-0112, REQ-LEG-0169, REQ-LEG-0263, REQ-LEG-0272, REQ-PLT-0066, REQ-PLT-0093, REQ-PLT-0124, REQ-ECO-0131 and REQ-ECO-0174 |

Checks performed:

1. **Split, group, scale and real-vs-generated audit** for each family, recomputed from
   `manifest.json`.
2. **Format presence** by extension and libmagic MIME, aggregated over all 131 tuning+validation
   `stats` records.
3. **Hidden cross-split leakage.** I measured exact-content overlap (SHA-256 of files with at
   least one byte) between tuning and validation. The input was the non-held-out canonical line
   files `/root/eb-research/fingerprints/{tuning,validation}/*.lines.gz`. No held-out path,
   fingerprint listing or content was read.
4. **Existence of candidate fix sources.** Every URL below with a byte size was probed on
   2026-09-16 with HTTP HEAD, or with the Hugging Face and GitHub metadata APIs. No bodies were
   downloaded. A HEAD size is **not a pin**: provisioning must still declare or TOFU the SHA-256.

## 1. Use-case coverage

| # | Use case | Status | Evidence in the current corpus | Gaps |
|---|---|---|---|---|
| U1 | Source releases | COVERED | F01/F18 real in every split. Release tarballs in F18 and F11 (xorg). | G05 (language skew, no `.git`) |
| U2 | Package publishing | PARTIAL | F11 has deb, rpm, apk, wheel, jar and pkg.tar.zst. There are no registry source packages (`.crate`, npm `.tgz`, sdist, gem, nupkg) and no repository metadata (apt `InRelease`/`Packages`, PyPI simple). | G03 |
| U3 | CI artifacts | PARTIAL | F02 is Linux gcc/cargo build trees only. There are no JS/JVM build outputs, no test reports or coverage, no MSVC PDBs or split debug symbols, and all 9 build items are unpinned (`output_pin: null`). | G17 |
| U4 | ML model/data distribution | **MISSING** | No `.safetensors`, `.gguf`, `.onnx`, `.pt`/`.pth`, `.ckpt`, `.parquet`, `.arrow` or webdataset shards in any split. `.npy` appears only in generated F07 items; `.npz` only as tiny files inside site-packages and Loghub. REQ-CMP-0149 (a BF16/FP16 sign/exponent/mantissa split for model weights) has no input. | G01 |
| U5 | Scientific archives | PARTIAL | There is one real NetCDF file per split: NCEP reanalysis in tuning (NetCDF-4/HDF5 container), GISTEMP in validation, ETOPO in held-out. The only plain HDF5 file is held-out (GWOSC). Raw float/int arrays come from SDRBench and Silesia. No FITS, Zarr/N5 directory stores, genomics (VCF/BAM/CRAM/FASTQ) or GeoTIFF/COG. | G15 |
| U6 | Supply-chain verification | **MISSING** | No release directory with detached signatures (`.asc`/`.sig`/`.sigstore`), checksum manifests, SBOMs (SPDX/CycloneDX), provenance (in-toto/SLSA) or OCI referrers. REQ-ECO-0131 requires this workflow. | G03 |
| U7 | Backups of home/project directories | **MISSING** | There is no real mixed personal or office document tree. Office documents exist only as the 2.8 MiB generated `f14-gen-office-documents`. No working copy has `.git` (every F01 item is `git archive` or a tarball, with one mtime). Maildir and rsnapshot series are fully generated with synthetic content. REQ-CMP-0113 requires planner weights fitted to real file-type mixes. | G04, G05 |
| U8 | Legacy ZIP/tar/7z migration | PARTIAL (weak) | Tuning and validation contain **zero** `.7z` files, and 7z appears only in held-out. ZIPs come only from Java, Android and Python toolchains (jar, war, apk, wheel, sdist). There is no ZIP64, no ZipCrypto or WinZip AES, no CP437 or non-UTF-8 names, and no Windows or macOS writer output. Tar is ustar only. RAR, ISO9660, WIM, CAB/MSI and cpio outside RPM payloads are all absent. REQ-LEG-0078, 0169, 0263 and 0272 ask for a real-writer corpus. | G09 |
| U9 | Container/VM images | PARTIAL | F09 has flattened `docker export` rootfs trees; F16 has qcow2, raw and vmdk. There is no OCI image layout with layers, whiteouts or zstd layers (REQ-LEG-0104), no VHD/VHDX/VDI/OVA/ISO, and every F16 tuning item is converted or built. F16 validation has no large item. | G10, G18 |
| U10 | Media libraries | PARTIAL | Tuning is good (camera JPEG, video, audio, PDF). F12 validation and held-out hold only re-encodes from the same encoder-matrix generator, and held-out has one group. There are no camera RAW/DNG files, phone HEIC/HEVC, or sidecar XMP. The Blender, LibriVox and Musopen production pipelines repeat across splits. | G08, G21 |
| U11 | Logs/telemetry archival | PARTIAL | Text logs are covered. There are no metrics or telemetry time series, no binary logs (journald, EVTX) and no rotated `.gz` log directories. Validation and held-out max out at 153 MiB. | G11 |
| U12 | Database dumps | **MISSING** | Only live database files exist (SQLite, PGDATA, InnoDB). There are no logical dumps (pg_dump plain/custom/directory, mysqldump, tab/COPY exports). The known gap stands: F08 validation has no real-content database, because Sakila and Northwind hold fictitious rows and employees is synthetic. | G02 |
| U13 | Cross-platform metadata-rich trees | **MISSING** (real) | F19 is 6/6 generated on Linux ext4. No NTFS ADS, security descriptors, DOS attributes or reparse data. No macOS AppleDouble, resource forks or xar. No SELinux labels or real file capabilities, because `docker export` drops xattrs. REQ-PLT-0066, 0093 and 0124 need these. | G06 |
| U14 | Encrypted private archives | **MISSING** (workload) | F20 has ciphertext-like high-entropy inputs only. There is no secret-bearing private-document tree for evaluating compress-then-encrypt leakage, and no existing encrypted archives (ZipCrypto, AE-1/AE-2, 7z `-mhe`, gpg/age) to import or refuse. | G07 |
| U15 | Adversarial inputs | PARTIAL | High-entropy, CDC-adversarial and bomb inputs are covered, but all 10 F20 items come from in-house generators. There are no structurally malicious archives (zip-slip traversal, symlink or hardlink escape, overlapping-entry bombs, duplicate names, CD/LFH mismatch, malformed 7z/tar headers) and no third-party fuzz or CVE reproducers. | G12 |

## 2. Family audit (from `manifest.json`)

Group counts are distinct `independence_group` values per split. Real means `real` or
`derived-from-real`. Scale cells in parentheses are present but misleading.

| Family | Tuning groups (real) | Validation groups (real) | Held-out groups (real) | Missing scale cells | Criteria | Principal problems |
|---|---|---|---|---|---|---|
| F01 source | 4 (4) | 2 (2) | 3 (3) | V:L | pass | C/C++-heavy. No JS/TS/Java source tree in any split. No `.git` history. Validation rests on cpython and redis. |
| F02 build | 3 (3) | 3 (3) | 3 (3) | V:L | pass | Linux gcc/cargo only. Validation (cpython, redis, cjson) and held-out (linux, musl, postgresql) share projects with F01/F18. All 9 items unpinned. |
| F03 vendor | 4 (4) | 3 (3) | 4 (4) | T:L, V:S, H:L | pass on labels, **fails on content** | Measured tuning/validation sharing: yq 61.2% of bytes (10.3 MiB) identical to fzf, bat 14.7% (79 MiB) identical to ripgrep's vendor tree, bootstrap 4.7% (13.3 MiB) identical to TypeScript's. Held-out alacritty, direnv and pdf.js are very likely similar and cannot be measured before the freeze. 14 items are unaudited `LicenseRef-mixed-per-package`. |
| F04 small files | 2 (**0**) | 2 (1) | 2 (2) | T:L, V:S, V:L, H:S, H:L | pass on counts | Tuning is 100% generated. `f04-validation-bootstrap-npm-modules` and `f04-heldout-pypi-site-packages` are byte-identical to F03 items (same `content_tree_sha256`). The largest file count is 34.7k, so the 10^5–10^6 range in the family intent is absent. |
| F05 logs | 5 (4) | 3 (2) | 4 (3) | V:L, H:L | pass | Largest validation/held-out item is 153 MiB. No telemetry or binary logs. Three Loghub items are non-redistributable public benchmarks. |
| F06 structured | 4 (3) | 3 (2) | 4 (4) | V:L, H:L | pass | No columnar or binary structured formats (Parquet, Arrow, Avro, protobuf, BSON). |
| F07 numeric | 4 (3) | 3 (2) | 3 (2) | V:L, H:L | pass | 3 of 7 tuning items are Silesia (public benchmark, non-redistributable). No ML weights, FITS, Zarr or genomics. |
| F08 databases | 4 (2) | 3 (2, fictitious rows) | 3 (2) | V:L | pass | No dumps. Validation has no real-content database (known gap). Real-content databases (Natural Earth, Lahman) exist only in held-out. |
| F09 executables | 3 (3) | 3 (3) | 4 (4) | V:S, T:L, V:L, H:L | pass | Tuning and validation are x86-64/amd64 only. Real Mach-O and arm64 executables exist only in held-out. The only tuning Mach-O objects are 544 LLVM test-fixture files. This blocks per-architecture BCJ filter decisions (REQ-CMP-0149). |
| F10 redundant | 3 (2) | 2 (2) | 3 (3) | no large anywhere | pass | No large tier, so long-range redundancy cannot be studied at scale. |
| F11 compressed | 3 (3) | 2 (2) | 3 (3) | no large, V:S | pass | OS and language packages only. No registry source packages or signed release directories. |
| F12 JPEG | 2 (2) | 1 (1) | **1 (1)** | V:L, H:L | **FAIL: held-out needs at least 2 groups** | Validation and held-out are 100% re-encodes by the tuning encoder-matrix script. There are no camera-originated JPEGs outside tuning. The Commons fetch (`commons-usgov-photos`, `commons-cc0-photos`) was abandoned after HTTP 429. |
| F13 media | 6 (6) | 4 (4) | 6 (6) | V:L | pass | Blender open movies appear in all splits, and `f13-heldout-tos-media-large` is tagged `public-benchmark` in held-out (methodology §11). LibriVox and Musopen pipelines repeat across splits. |
| F14 nested archives | 3 (2) | 2 (1) | 3 (2) | no large | pass | No real 7z in tuning/validation. No legacy-writer diversity (see U8). The office documents are generated. |
| F15 sparse | 2 (**0**) | 2 (**0**) | 2 (**0**) | T:S, T:L, V:L, H:S | pass on counts | 0 real items in every split. |
| F16 VM images | 2 (2, all derived) | 2 (1) | 3 (2) | V:S, V:L, H:S | pass | Tuning has no as-published image. Alpine raw and qcow2 are one filesystem in two containers. No OCI, ISO, VHD/VHDX, VDI or OVA. |
| F17 duplicates | 2 (**0**) | 2 (1) | 2 (**0**) | no small, no large | pass on counts | Tuning and held-out are fully generated. The only real item is eight `cp -a` copies of the F03 yq tree, whose bytes already overlap tuning by 61%. |
| F18 versioned | 5 (5) | 3 (3) | 3 (3) | none | pass | Source-release series only. No versioned binaries, images, datasets or model checkpoints. |
| F19 metadata | 2 (**0**) | 2 (**0**) | 2 (**0**) | no large | pass on counts | 100% generated, Linux-only. |
| F20 adversarial | 3 (**0**) | 3 (**0**) | 3 (**0**) | T:S, T:L, V:S (really), V:L (really), H:L | pass on counts | `f20-validation-bombs-compressed` is labelled large at 1.9 MiB (`fits_smaller_tier`), so the coverage table's F20 V:L cell is not a large workload. |

Cross-cutting:

- **Validation scale.**
  - The largest validation item is 575 MiB (`f18-validation-cpython-3-12-point-releases`).
  - Real V:L cells exist only for F03 (544 MiB) and F18 (575 MiB).
  - Held-out has large items in F01, F02, F08, F13, F15, F16 and F18, up to 4.33 GiB apparent.
  - Split totals: tuning 20.60 GiB, validation 7.79 GiB, held-out 17.96 GiB.
  - No validation item falls between 0.6 and 4.3 GiB, so the largest validation input is 7.7x
    smaller than the largest held-out input.
  - Chunk-index memory, long-range windows, dedup scope, planner cost models and resource-budget
    defaults (REQ-CON-0073, REQ-LEG-0117) therefore cannot be validated at held-out scale.
- **Tier ceiling.** No real or derived-from-real item is larger than 3.5 GiB. Model weights and
  cloud images commonly are.
- **Same-project stacking.** The validation F01/F02/F18 cells are cpython, redis and cJSON. The
  held-out F01/F02/F18 cells are linux, musl, go, postgresql and lua. Pooling "source" results
  across these families counts one project several times. For example, the F02 redis build tree
  contains all 14.8 MiB of `f01-validation-redis-7-2-16-git`.
- **Cross-split byte sharing beyond vendor trees.** `f01-validation-redis-7-2-16-git`,
  `f02-validation-redis-intree-build` and `f18-validation-redis-7-2-point-releases` share
  3.0 MiB (20.5% of the F01 item) with `f03-tuning-ripgrep-cargo-vendor`, most likely bundled
  jemalloc. `f15-validation-edgecases` shares a 1.0 MiB file with `f10-gen-redundant-binary-blobs`.
- **License recording.** Every item has the four license keys. The problems are elsewhere:
  - The Silesia group is recorded inconsistently: `f09-silesia-mozilla-ooffice` says
    `redistributable: true`, while the four F07/F08 Silesia items say `false` under the same
    corpus terms.
  - The 14 `LicenseRef-mixed-per-package` items were declared "pending per-package license audit"
    in `10743d7`, and that audit was never done.
  - GH Archive has no data license.
  - Kodak has no formal licence text.
- **Public-benchmark tagging in held-out.** `f13-heldout-tos-media-large` is tagged
  `public-benchmark`. `f13-heldout-elephants-dream-media-medium` comes from the same Blender
  open-movie set but is not tagged.
- **Budget conflict for the fixes.** The fix-agent instructions carry "group total under
  ~12 GiB", but g3-data is already at 10.64 GiB and g4-binary at 11.93 GiB. G01, G10 and G11
  cannot be closed under that cap. WSL ext4 has about 706 GB free, so the orchestrator should
  raise the cap for round-1 fixes, for example to about 40 GiB per group, and should prefer
  held-out and validation additions.
- **Docker dependence.** Docker Desktop was unavailable to g2 because the service needs admin.
  Container-image fixes should use `skopeo` 1.13.3 (Ubuntu apt), which pulls registries over
  HTTPS without a daemon.

## 3. Gaps and fixes

Severity: BLOCKER means a use case is missing or a hard family criterion fails. MAJOR means a use
case is only partially represented or a split is not independent. MINOR is a quality issue.

Allocation rules that apply to every fix:

- New groups go in exactly one split.
- An existing group is reused only inside its own split. For example, `cpython` is validation
  everywhere and `almalinux` is held-out.
- `derive` items take sources from their own split.
- Held-out descriptions stay provenance-only.
- Hugging Face inputs pin the commit (`/resolve/<sha>/`) and declare `sha256` from
  `api/models/<id>/tree/<sha>` (`lfs.oid`).

### G01 [BLOCKER] F07: ML model and data distribution

Problem: no model weights, quantized weights, framework checkpoints or dataset shards exist in any
split.

Fix (g3-data; about 4.3 GiB tuning, 2.2 GiB validation, 9.1 GiB held-out, so the ~12 GiB group
cap must be raised):

- **Tuning**
  - `EleutherAI/pythia-160m` (Apache-2.0, commit `50f5173d932e8e61f858120bcb800b97af589f46`,
    group `eleutherai-pythia`): `model.safetensors` at `main` (374,998,696 B, FP16 by size) and at
    revision `step100000` (649,308,728 B, FP32 by size). This is a versioned checkpoint pair. For
    a same-dtype near-duplicate pair, add an adjacent revision such as `step99000` and verify
    dtypes in the safetensors headers.
  - `Qwen/Qwen2.5-1.5B` (Apache-2.0, commit `8faed761d45a263340a0528343f099c05c9a4323`, group
    `qwen2.5`): `model.safetensors` BF16 (3,087,467,144 B, `sha256`
    `a961db72e75d52b18e6b0c9d379e51a26973b233385e0e127fdda7d648aec796`), plus
    `Qwen/Qwen2.5-0.5B-Instruct-GGUF` `qwen2.5-0.5b-instruct-q4_k_m.gguf` (491,400,032 B, Apache-2.0)
    as a quantized, near-incompressible case.
  - `https://storage.googleapis.com/tensorflow/tf-keras-datasets/mnist.npz` (11,490,434 B;
    MNIST CC-BY-SA-3.0; group `mnist`): a small real `.npz`.
- **Validation**
  - `HuggingFaceTB/SmolLM2-360M` (Apache-2.0, commit `f8027fd0eaeea54caa13c31d31b9fdc459c38b49`):
    `model.safetensors` (723,674,912 B, `sha256`
    `7aaff6661428bed033abba9522bec81938678642cca3181fe752b6ca9e1e540f`, large tier), plus
    `SmolLM2-135M` (269,060,552 B). Group `huggingfacetb-smollm2`.
  - `google-bert/bert-base-uncased` (Apache-2.0, commit `86b5e0934494bd15c9632b12f734a8a67f723594`,
    group `google-bert`), the same weights in three containers: `pytorch_model.bin` zip-pickle
    (440,473,133 B), `model.safetensors` (440,449,768 B) and `tf_model.h5` (536,063,208 B).
    Large tier; this also adds HDF5 to validation.
- **Held-out**
  - `microsoft/Phi-3-mini-4k-instruct` (MIT, commit `f39ac1d28e925b323eae81227eaba4464caced4e`,
    group `microsoft-phi3`): the sharded BF16 checkpoint `model-00001-of-00002.safetensors`
    (4,972,489,328 B) and `model-00002-of-00002.safetensors` (2,669,692,552 B), 7.12 GiB total
    (large tier).
  - `HuggingFaceFW/fineweb-edu` (ODC-By, commit `87f09149ef4734204d70ed1d046ddc9ca3f2b8f9`, group
    `huggingfacefw-fineweb`): `sample/10BT/000_00000.parquet` (2,152,819,114 B), a
    dataset-distribution shard.

Acceptance:

- Each split has at least 2 ML groups and a large item.
- BF16/FP16 weights appear in every split.
- Quantized weights appear in tuning; add a GGUF to validation if budget allows.
- A dataset-shard format (parquet) appears in held-out, and G20 adds columnar data to tuning and
  validation.

### G02 [BLOCKER] F08: database dumps, and real-content databases in validation

Problem: no logical dumps exist. The only real-content databases are the held-out Natural Earth
and Lahman files.

Fix (g3-data):

- **Tuning**
  - Wikimedia `simplewiki` SQL table dumps: `https://dumps.wikimedia.org/simplewiki/20260901/`,
    files `simplewiki-20260901-{page,categorylinks,pagelinks}.sql.gz` (the `page` table is about
    33 MB gzipped). mysqldump format, CC-BY-SA-4.0/GFDL, group `wikimedia-simplewiki`,
    decompressed, medium tier.
  - `pg_dump -Fp`, `-Fc` and `-Fd` of the existing `f08-tuning-postgres17-pgbench-s40` as a
    same-split derive.
- **Validation**
  - Ensembl release-114 MySQL dump
    `https://ftp.ensembl.org/pub/release-114/mysql/saccharomyces_cerevisiae_core_114_4/`
    (`*.sql.gz` schema plus `*.txt.gz` tab-delimited tables; Ensembl data carry no use
    restrictions; group `ensembl`), as published and decompressed.
  - The same dump loaded into MariaDB 11.4, giving a real-content InnoDB directory and a
    `mysqldump`. This is a derive and closes the "no real-content DB" gap.
  - Stack Exchange `https://archive.org/download/stackexchange/cs.stackexchange.com.7z`
    (130,990,559 B; CC-BY-SA-4.0, to be verified against the dump's license file; group
    `stackexchange-cs`) loaded into SQLite by a derive loader.
  - `mysqldump` of `f08-validation-mariadb11-test-db-employees`.
- **Held-out**
  - Rfam 15.0 MySQL dump `https://ftp.ebi.ac.uk/pub/databases/Rfam/15.0/database_files/`
    (`*.sql` plus `*.txt.gz`; `full_region.txt.gz` alone is 135,765,791 B; CC0; group `ebi-rfam`),
    with a table selection that keeps the item at or below 8 GiB decompressed. This gives
    held-out a large dump.
  - Optionally, a `pg_dump -Fd` of Rfam loaded into PostgreSQL 17 (derive).

Acceptance:

- Every split has at least one real logical dump.
- Tuning and validation also have a custom-format or directory-format dump.
- Validation has at least one real-content database file.

### G03 [BLOCKER] F11: supply-chain verification release sets and package-publishing artifacts

Problem: nothing in the corpus carries signatures, checksums, SBOMs or provenance, and registry
source packages are absent.

Fix (g4-binary):

- **Tuning**
  - `sigstore/cosign` v2.5.0 GitHub release assets (Apache-2.0, group `sigstore-cosign`): binaries,
    `cosign_checksums.txt` (3,906 B), and the keyless `.sig`/`.pem` files. Enumerate the exact
    asset list from the GitHub releases API, and include any SBOM or provenance assets present.
  - The `.crate` files named by ripgrep 14.1.1's `Cargo.lock`, fetched from
    `static.crates.io` (ripgrep group).
  - An apt metadata snapshot (`InRelease`, `Packages.xz`, `by-hash/`) for noble from
    `snapshot.ubuntu.com` (ubuntu-noble group).
- **Validation**
  - python.org 3.13.7 release set: `Python-3.13.7.tar.xz`, `.tar.xz.sigstore` (4,951 B), `.asc`,
    `Python-3.13.7.tgz.spdx.json` (2,935,829 B) and `amd64/*.msi`, including `core_pdb.msi`
    (4,489,216 B). PSF-2.0, cpython group.
  - `npm pack` tarballs for bootstrap v5.3.8's lockfile (twbs-bootstrap group).
- **Held-out**
  - `https://archive.mozilla.org/pub/firefox/releases/155.0.1/`: `SHA256SUMS`,
    `SHA256SUMS.asc` (833 B), `KEY`, and the `linux-x86_64` tarball `.asc`
    (mozilla-firefox group).
  - PyPI sdists for `research/corpus/generators/g1-code/locks/pypi-web-service-stack.txt`
    (pypi-web-service-stack group).

### G04 [BLOCKER] F19: backups of home directories as real-content snapshot series

Problem: no mixed personal or office content exists, and every snapshot series uses synthetic
bytes.

Fix (g2-generated): extend `hardlink_farm.py`, or add `home_snapshot.py`, to compose a home layout
from real same-split sources. The layout covers `Documents`, `Pictures`, `Downloads` with
duplicates, `Music`, dotfiles, `.config`, `.cache`, browser-profile SQLite, `.local/share/Trash`,
realistic modes and mtimes, and N daily snapshots with edits, renames and deletes. Real content
per split:

- **Tuning:** Digital Corpora govdocs1 threads 000–001
  (`https://digitalcorpora.s3.amazonaws.com/corpora/files/govdocs1/zipfiles/000.zip`,
  486,762,106 B). These are real doc, xls, ppt, pdf, jpg and html files from US government web
  sites. Group `digitalcorpora-govdocs1`.
- **Validation:** Apache POI `REL_5_4_1` `test-data/` (`github.com/apache/poi`, Apache-2.0; real
  OLE2, OOXML, Visio, Outlook `.msg` and Publisher files). Group `apache-poi`.
- **Held-out:** LibreOffice core `libreoffice-25.8.1.1` tag, `*/qa/*/data` test documents
  (MPL-2.0). Group `libreoffice-core`.

Use a different composition model for held-out, for example a Time-Machine-like model instead of
rsnapshot. This also gives F19 its first real content.

Licence recording for these items:

- govdocs1 files were crawled from US government web servers, and third-party content cannot be
  ruled out. Record the Digital Corpora terms and set `redistributable: false` unless per-file
  evidence says otherwise.
- The POI and LibreOffice test documents fall under the repository licence, but many arrived
  through bug reports. Record the repository licence, and set `redistributable: false` until
  someone audits the files individually.

### G05 [MAJOR] F01: project directories with `.git`, source-language breadth, and validation large tier

Problem: every F01 item has had its git history stripped. No JS/TS/Java source tree exists in any
split. Validation has no large item.

Fix (g1-code), with byte-reproducible acquisition by caching a `git bundle --all` created once
(pin its SHA-256) and then cloning from the bundle:

- **Tuning:** full clone of `facebook/zstd` with `.git` packfiles, plus the in-tree build products
  of `f02-tuning-zstd-cmake-build` (zstd group). Also a `git archive` of `microsoft/TypeScript`
  v5.9.3 (microsoft-typescript group).
- **Validation:** full clone of `python/cpython` at `v3.13.15` with `.git`, which puts it in the
  large tier and closes F01 V:L. Also a `git archive` of `twbs/bootstrap` v5.3.8
  (twbs-bootstrap group).
- **Held-out:** full clone of `github.com/lua/lua` (lua group). Also a `git archive` of
  `mozilla/pdf.js` v4.10.38 (mozilla-pdfjs group).

Checkouts have nondeterministic `.git/index` stat data, so set `output_pin: null` and reference the
recorded fingerprint.

### G06 [BLOCKER] F19: real cross-platform metadata (SELinux, capabilities, NTFS, macOS)

Fix (g2-generated):

- **(a) Real Linux rootfs trees with xattrs.** Derive by read-only loop-mount (root in WSL2), or
  with `guestfish --ro -i tar-out / - xattrs:true selinux:true acls:true` (apt `libguestfs-tools`
  1.52.0), from each split's images:
  - tuning: `f16-ubuntu-noble-cloudimg-20260911-raw` (ubuntu-noble)
  - validation: `f16-debian-13-nocloud-20260831-qcow2` (debian-trixie)
  - held-out: `f16-heldout-almalinux-10-genericcloud-qcow2` (almalinux; carries `security.selinux`)

  These give setuid binaries, `security.capability`, hardlink farms and usrmerge symlinks.
- **(b) NTFS metadata carried as a transport.**
  - Tuning: an image made with `mkntfs` and populated with `ntfs-3g` (ADS including
    `Zone.Identifier`, `system.ntfs_acl`, DOS attributes, reparse points), plus its
    `wimlib-imagex capture` WIM (apt `ntfs-3g`, `wimtools` 1.14.4).
  - Validation: Digital Corpora `nps-2009-ntfs1` E01 images, converted to raw with `ewfexport`
    (apt `ewf-tools`): `ntfs1-gen0.E01` (1,089,252 B), `ntfs1-gen1.E01` (9,332,369 B) and
    `ntfs1-gen2.E01` (36,083,007 B). Group `digitalcorpora-nps-ntfs1`.
  - Held-out: a different NTFS generator configuration.
- **(c) macOS metadata carried as a transport.** A generated AppleDouble (RFC 1740) and
  `__MACOSX/` ZIP writer with resource forks, FinderInfo and quarantine xattrs, using different
  parameters per split. Validation also gets the python.org 3.13.7 macOS universal2 `.pkg` (xar,
  Bom, Payload; cpython group).

### G07 [BLOCKER] F20: encrypted private-archive workload

Problem: no workload represents what users encrypt, or existing encrypted archives. The leakage
studies (crypto-leakage cluster) need compressible secrets placed next to attacker-influenced
content.

Fix (g2-generated): add `private_vault.py`. Items are seeded and use different generators for
held-out. Each contains:

- personal-style PDF, DOCX and XLSX statements with fake identities from a seeded locale
- a KeePass `.kdbx` made with pykeepass
- OpenSSH, age and GPG keys derived from a seeded DRBG
- `.env` and config files holding fake tokens
- a browser-profile SQLite
- legacy encrypted archives with passwords recorded in `notes`: `zip -P` (ZipCrypto),
  `7z -tzip -mem=AES256` (AE-2), `7z -t7z -mhe=on -p`, `gpg --symmetric` and `age -p`

Place one item per split, and add a large validation variant.

### G08 [BLOCKER] F12: held-out independence, and real camera JPEGs in validation and held-out

Fix (g5-media):

1. Retry `generators/g5-media/select.sh fetch` for pools `commons-usgov-photos` (held-out) and
   `commons-cc0-photos` (validation). The selections are already committed. Serialize requests to
   at most 1 per second, send a Wikimedia-policy User-Agent with contact, honour `Retry-After`,
   and make the fetch resumable.
2. If Commons still returns 429, use these fallbacks:
   - **Held-out:** Open Images V7 Flickr originals. The CSV
     `https://storage.googleapis.com/openimages/2018_04/image_ids_and_rotation.csv`
     (3,348,497,077 B) has per-image `OriginalURL`, `License` (CC-BY-2.0), `Author`,
     `OriginalSize` and `OriginalMD5`. Stratify by EXIF make and model, including phones. Verify
     MD5, then record SHA-256. Group `openimages-flickr-originals`.
   - **Validation:** raw.pixls.us camera RAW samples (`https://raw.pixls.us/data/<Make>/<Model>/`,
     CC0; DNG, CR2/CR3, NEF, ARW, RAF, ORF and RW2 with embedded JPEG previews). Group
     `raw-pixls-us`. This also adds a validation large item and the RAW media-library case.

Acceptance: F12 held-out has at least 2 groups, and at least one validation group and one
held-out group contain camera-originated JPEGs that the corpus's own encoder matrix did not
produce.

### G09 [MAJOR] F14: legacy ZIP/tar/7z migration corpus

Problem: tuning and validation contain no 7z, ZIP64, encrypted or non-UTF-8 entries, no
Windows/macOS writers and no tar variants. RAR, WIM, CAB and MSI are absent.

Fix (g4-binary):

- **(a) `legacy_writer_matrix.py`.** Each split uses its own real input tree: tuning
  `f01-tuning-curl-8-19-0-git`, validation `f01-validation-redis-7-2-16-git`, held-out
  `f01-heldout-musl-1-2-5-src`. Writers:
  - Info-ZIP `zip` 3.0: `-0`/`-6`/`-9`, `-y`, `-X`, `-Z bzip2`, `-P`
  - 7-Zip 23.01:
    - `-tzip -mm=Deflate64|LZMA|PPMd|BZip2`, `-mem=AES256`, `-mcu=on`
    - `-t7z` LZMA2+BCJ2 solid and `-ms=off`, PPMd, `-mhe=on`, `-v` multi-volume
  - Python `zipfile` with forced ZIP64 and more than 65,535 entries
  - GNU tar `--format=v7|oldgnu|gnu|ustar|posix` with `--sparse` and `--xattrs`
  - bsdtar pax, `cpio newc/odc` and iso9660
  - `wimlib-imagex` XPRESS/LZX/LZMS solid
  - `gcab` and `msitools` CAB/MSI
  - On the Windows host (non-admin is enough): PowerShell `Compress-Archive`, .NET `ZipFile` and
    `tar.exe` (bsdtar 3.8.8)
  - Java 21 `jar` and `ZipOutputStream`
  - A CP437- or Shift-JIS-named ZIP written with the language-flag bit clear
- **(b) Real as-published 7z archives.**
  - Tuning: `https://dumps.wikimedia.org/tnwiki/20260901/tnwiki-20260901-pages-meta-history.xml.7z`
    (20,351,309 B; same group as `f06-tuning-wikimedia-tnwiki-20260901`) and Notepad++
    `npp.8.8.5.portable.x64.7z` (5,968,903 B; GPL-3.0).
  - Validation: `PortableGit-2.55.0.5-64-bit.7z.exe` as published (7z SFX; the same input as
    `f09-git-for-windows-portable-2550`, same split).
  - Held-out: the existing 7-Zip release archives suffice.
- **(c) RAR5 as a decode-only Tier 2 format (REQ-LEG-0132).** Use the RAR fixtures inside the
  libarchive test suite from G12 (tuning) rather than third-party downloads.

### G10 [MAJOR] F16: OCI image layouts, ISO/OVA/VHD formats, as-published tuning image, validation large tier

Fix (g4-binary):

- **(a) OCI layouts** with `skopeo copy docker://<repo>@sha256:<digest> oci:<dir>`, including a
  multi-arch index and at least one zstd-layer copy (`--dest-compress-format zstd`). Held-out also
  gets a built two-layer image with whiteouts. Reuse the exact repository and digest already
  pinned in the same split, because the upstream-identity rule forbids a docker repository across
  splits.
  - Tuning: the Ubuntu 24.04 image of `f09-ubuntu-noble-usr-binaries`
  - Validation: the Fedora 44 image of `f09-fedora-44-usr-binaries`
  - Held-out: the AlmaLinux 10 image of `f09-heldout-almalinux-10-usr-binaries`
- **(b) ISO images**
  - Tuning: `alpine-virt-3.24.1-x86_64.iso` (69,206,016 B; alpine-linux)
  - Validation: `debian-13.7.0-amd64-netinst.iso` (792,723,456 B; `SHA256SUMS` lists
    `a7ef94ac2fb9a7fec454552abd629b7cc9d5155c886165a45649f5ce6167e355`; debian-trixie). Large
    tier, which closes F16 V:L.
  - Held-out: `AlmaLinux-10-latest-x86_64-boot.iso` (1,056,202,752 B; almalinux). Pin the dated
    file name, not `latest`.
- **(c) Other virtual disk formats**
  - Tuning: `ubuntu-24.04-server-cloudimg-amd64.ova` (594,145,280 B; tar with OVF plus a
    streamOptimized VMDK; ubuntu-noble). Pin the dated `release-YYYYMMDD` path. This is tuning's
    first as-published image.
  - Held-out: `FreeBSD-15.1-RELEASE-amd64-ufs.vhd.xz` (665,167,912 B compressed; freebsd). Check
    that the decompressed apparent size is at most 8 GiB; otherwise keep it compressed as F11.
  - VDI and VHDX: `qemu-img convert -O vdi|vhdx` derives of same-split images.

### G11 [MAJOR] F05: telemetry and binary logs, and validation/held-out scale

Fix (g3-data):

- **Tuning:** Google cluster-usage trace 2011-2, `task_usage/part-0000{0..4}-of-00500.csv.gz`
  (91,723,415 B each compressed; CC-BY-4.0; group `google-clusterdata-2011-2`), decompressed.
  Large tier.
- **Validation**
  - One day of Wikimedia hourly pageviews,
    `https://dumps.wikimedia.org/other/pageviews/2026/2026-08/pageviews-20260801-HH0000.gz`
    (CC0; group `wikimedia-pageviews`), decompressed. Trim to the hours that fit 8 GiB. Large
    tier.
  - A binary `systemd-journald` journal generated in a systemd container (generated).
- **Held-out**
  - Backblaze Drive Stats `data_Q1_2025.zip` (1,020,483,699 B; Backblaze terms of attribution
    and no resale, so `redistributable: false`; group `backblaze-drive-stats`). Large tier.
  - `sbousseaden/EVTX-ATTACK-SAMPLES` Windows `.evtx` files (GPL-3.0; group
    `evtx-attack-samples`).
- **All splits:** a logrotate-style directory of daily `.gz` rotations, derived from each split's
  real logs.

### G12 [MAJOR] F20: real structurally adversarial archives

Fix (g2-generated):

- **Tuning:** `libarchive/libarchive` v3.8.1 test-suite archives (`libarchive/test`, `tar/test`,
  `cpio/test`, `unzip/test`, with `*.uu` decoded). This covers malformed tar, cpio, zip, 7z, RAR,
  RAR5, xar, lha and cab. Record license per file: `COPYING` is BSD-2-Clause for most files, and
  GitHub reports `NOASSERTION`. Group `libarchive`.
- **Validation**
  - CPython `Lib/test/archivetestdata` and the zipfile/tarfile test data, derived from
    `f01-validation-cpython-3-13-15-git` (same split).
  - `apache/commons-compress` `rel/commons-compress-1.28.0` `src/test/resources` (Apache-2.0;
    group `apache-commons-compress`), which includes COMPRESS-* fuzzer reproducers.
- **Held-out**
  - `src/archive/zip/testdata` and `src/archive/tar/testdata`, derived from
    `f01-heldout-go-1-25-0-src` (same split).
  - The `fastzip/malo` corpus (BSD-2-Clause).
  - Fifield overlapping-entry zip bombs regenerated with
    `https://www.bamsoftware.com/hacks/zipbomb/zipbomb-20210121.zip` (18,130 B). REQ-ECO-0174
    says to regenerate these, never vendor them.
- **All splits:** a generated set of traversal (`../`, absolute, drive-letter and backslash
  paths), symlink- and hardlink-escape, duplicate-name and CD/LFH-mismatch archives, using a
  distinct generator for held-out.

### G13 [MAJOR] F09: architecture and object-format skew

Fix (g4-binary):

- **Tuning**
  - `linux/arm64` exports of the already-pinned Ubuntu 24.04 and Alpine 3.24 images. Export does
    not execute anything, so no emulation is needed. Use skopeo plus `umoci` or the docker
    export path.
  - ripgrep 14.1.1 release assets for `aarch64-apple-darwin`, `x86_64-apple-darwin` and
    `aarch64-unknown-linux-gnu` (ripgrep group).
- **Validation**
  - Node.js v22.20.0 `node-v22.20.0-darwin-arm64.tar.gz` (49,838,299 B), the `linux-arm64` tarball
    and the `win-arm64` zip (MIT; group `nodejs`).
  - Fedora 44 `linux/arm64` image binaries (fedora).

### G14 [MAJOR] F04: real many-small-file trees at 10^5 scale

Problem: tuning is all generated, and validation and held-out duplicate F03 bytes.

Fix (g2-generated):

- **Tuning:** NetBSD `https://cdn.netbsd.org/pub/pkgsrc/pkgsrc-2026Q2/pkgsrc.tar.xz`
  (60,074,916 B; BSD-2-Clause; group `netbsd-pkgsrc`).
- **Validation:** a dated Gentoo ebuild repository snapshot
  `https://distfiles.gentoo.org/snapshots/gentoo-YYYYMMDD.tar.xz` (the latest is 49,333,192 B;
  GPL-2.0; group `gentoo-repo`). Pin a dated name; `-latest` rotates.
- **Held-out:** `freebsd/freebsd-ports` `git archive` at a pinned commit (BSD-2-Clause; group
  `freebsd-ports`; held-out only, alongside the existing held-out freebsd group).

Keep the two byte-identical derive items, but tag them `duplicate-of:<item>` so that analyses do
not count them as independent F04 samples.

### G15 [MAJOR] F07: scientific archive formats beyond NetCDF and raw arrays

Fix (g3-data):

- **Tuning**
  - SDSS DR17 imaging frames, e.g.
    `https://data.sdss.org/sas/dr17/eboss/photoObj/frames/301/756/1/frame-g-000756-1-0206.fits.bz2`
    (3,572,185 B each), several runs and camcols, decompressed FITS (SDSS public data; group
    `sdss-dr17`).
  - `https://ann-benchmarks.com/fashion-mnist-784-euclidean.hdf5` (227,528,288 B; MIT;
    group `ann-benchmarks-fashion-mnist`). Tuning's only HDF5 container is the NetCDF-4 reanalysis
    file.
- **Validation:** the CMIP6 Zarr store
  `https://storage.googleapis.com/cmip6/CMIP6/CMIP/NCAR/CESM2/historical/r1i1p1f1/Amon/tas/gn/v20190308/`
  (consolidated `.zmetadata` present; CC-BY-4.0; group `cmip6-ncar-cesm2`). This is a directory
  store of many chunk objects. Enumerate keys from `.zmetadata`.
- **Held-out:** 1000 Genomes phase 3 `ALL.chr22...genotypes.vcf.gz` (205,612,353 B; IGSR open
  data; group `igsr-1000genomes`), kept as published BGZF plus a decompressed region subset of at
  most 8 GiB.

### G16 [MAJOR] F03: hidden cross-split package sharing and unaudited licences

Measured tuning/validation overlap: yq and fzf 61.2%, bat and ripgrep 14.7% (79 MiB), bootstrap
and TypeScript 4.7%.

Fix (g1-code):

- Run a provenance-level audit that reads no held-out tree. Intersect the `name@version` sets
  from each item's lockfile (`Cargo.lock`, `go.sum`, `package-lock.json`, pip lock) across all
  three splits, and commit the result. Held-out lockfiles are upstream provenance, not
  materialized content.
- For any held-out tree that shares more than 5% of its lock entries with tuning or validation,
  either replace it with a project that has a disjoint dependency graph, or add a `shared_packages`
  covariate and pre-register reporting held-out results both with and without the shared
  packages.
- Replace or annotate `f17-validation-real-yq-vendor-8x` and `f03-validation-yq-go-mod-vendor`,
  which are 61% tuning bytes.
- Resolve the 14 `LicenseRef-mixed-per-package` items with generated per-package license lists
  (`cargo metadata`, `go-licenses`, `license-checker`, `pip-licenses`). Mark items redistributable
  only with that evidence.
- Add one missing ecosystem in any split: a Maven `~/.m2` repository, NuGet `packages/` or Bundler
  `vendor/bundle`.

### G17 [MAJOR] F02: CI artifact breadth, and validation large tier

Fix (g1-code):

- **Tuning:** TypeScript v5.9.3 `npx hereby local` output (`built/local` JS, `.d.ts` and source
  maps; microsoft-typescript).
- **Validation**
  - CPython 3.13.15 `python -m test --junit-xml` reports plus a `gcov`/`lcov` coverage report
    over `f02-validation-cpython-build`.
  - `https://nodejs.org/dist/v22.20.0/win-x64/node_pdb.zip` (91,339,479 B; MSVC PDB; nodejs
    group, same split as G13).
  - A second CPython build variant (`--with-pydebug`, or `--enable-optimizations --with-lto`) to
    reach the large tier.
- **Held-out:** PostgreSQL 17.6 `make check` regression outputs plus a `--enable-coverage` lcov
  HTML report (postgresql).

### G18 [MAJOR] F15: sparse files have no real layouts

Problem: F15 has 0 real items, and validation has no large item.

Fix (g2-generated): `qemu-img convert -O raw` derives, verified with `SEEK_DATA`/`SEEK_HOLE`:

- Tuning: the VMDK inside the Ubuntu noble OVA from G10 (ubuntu-noble).
- Validation: `f16-debian-13-nocloud-20260831-qcow2` to sparse raw (debian-trixie, large tier).
- Held-out: `f16-heldout-almalinux-10-genericcloud-qcow2` to sparse raw (almalinux).

### G19 [MINOR] F17: real duplicate trees in tuning and held-out

Fix (g2-generated): unpack kernel header or devel packages for two ABI versions side by side.

- Tuning: Ubuntu noble `linux-headers-6.8.0-<N>` and `-<N>-generic` for two N (ubuntu-noble).
- Validation: Fedora 44 `kernel-devel` for two versions (fedora).
- Held-out: AlmaLinux 10 `kernel-devel` for two minor releases (almalinux).

### G20 [MINOR] F06: columnar formats, and validation/held-out large tier

Fix (g3-data):

- **Tuning:** NYC TLC `https://d37ci6vzurychx.cloudfront.net/trip-data/yellow_tripdata_2024-01.parquet`
  (49,961,641 B) and several months (NYC Open Data terms; group `nyc-tlc`).
- **Validation:** NOAA GHCN-Daily parquet
  `https://noaa-ghcn-pds.s3.amazonaws.com/parquet/by_year/YEAR=2023/` (public domain; group
  `noaa-ghcn-daily`), plus `csv.gz/by_year/2023.csv.gz` (172,261,078 B) decompressed for a large
  CSV.
- **Held-out:** reuse the G01 fineweb-edu parquet, which is F07 in the same split, or add a
  distinct large JSON dataset.

### G21 [MINOR] F13: production-pipeline repetition, benchmark in held-out, validation large tier

Fix (g5-media):

- Retag or replace `f13-heldout-tos-media-large`: add a non-benchmark large held-out item, and tag
  Elephants Dream consistently.
- Add validation and held-out media from pipelines that no split uses yet. Candidates are phone
  HEVC/HEIC encodes produced by an x265/libheif matrix from same-split sources (generated), and a
  large validation item such as the higher Sintel renditions from `download.blender.org/durian`
  (same group).
- Report LibriVox, Musopen and Blender results with the production pipeline as a covariate.

### G22 [MINOR] F18: versioned non-source series

Fix (g1-code):

- Tuning: two consecutive Ubuntu noble cloud image releases (qcow2 as published; ubuntu-noble).
- Validation: CPython Windows embeddable series 3.14.0 through 3.14.7 (cpython).
- Held-out: Firefox 155.0 and 155.0.1 Linux tarballs, extracted (mozilla-firefox).

The G01 pythia checkpoints cover the model-version case in tuning.

### G23 [MINOR] F20: scale label

`f20-validation-bombs-compressed` is labelled large at 1.9 MiB (`fits_smaller_tier`). Relabel it
small. Record the expansion size in `notes` or `tags`, not in `scale`, so that `coverage.md` stops
reporting an F20 V:L cell.

### G24 [MINOR] F09: Silesia licence inconsistency

`f09-silesia-mozilla-ooffice` is marked `redistributable: true`, while the same Silesia
distribution (`silesia-corpus` group) is `false` for `f07-tuning-silesia-*` and
`f08-tuning-silesia-osdb`. Mark it `false` (hashes only) to match, or justify redistribution per
file for all five items.

### G25 [MINOR] F10: no large tier in F10 or F11

Fix (g4-binary):

- F10 tuning large: the full `linux-firmware` 20260910 tree instead of the subset (same group).
- F11 validation large: an expanded Fedora 44 RPM set above 512 MiB (fedora).

## 4. What is adequate

- F01, F02 and F18 have real, distinct upstreams in every split with all three scales in tuning.
  F18 has V:L and H:L.
- F13 has 6, 4 and 6 real groups per split.
- F09, F10 and F11 have real, distinct distributions per split.
- F12 tuning's encoder matrix and edge cases are thorough for JPEG-reconstruction eligibility.
- The framework is sound: groups, upstream-identity checks, same-split derivation, the held-out
  guard, and the regression-tested `logical_tree_sha256`.

## 5. Must-fix set before corpus freeze

G01, G02, G03, G04, G06, G07 and G08 are blockers. G05, G09, G10, G11, G12 and G16 close
partially covered use cases and the leakage problem. After those, every split has at least one
large item in F01, F05, F07, F15 and F16 (G01, G05, G10, G11, G18), which removes the
validation-scale blind spot. Re-run assembly and a round-2 critic afterwards. Round 2 should
check the §1 table row by row, the §2 criteria, and the tuning/validation overlap measurement.
