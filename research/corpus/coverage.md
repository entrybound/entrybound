# Corpus coverage (ebrc-2026.09-v1)

manifest_sha256 `a71e7ee196520b2813c686b587d135fa1f0f8e5f185cb50b77352eb9fbbfa4ca`

Cells show materialized/defined items. Columns: split (T=tuning, V=validation, H=heldout) : scale (S<=16 MiB, M<=512 MiB, L<=8 GiB).

| Family | T:S | T:M | T:L | V:S | V:M | V:L | H:S | H:M | H:L |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| F01 source-code repositories | 2/2 | 1/1 | 1/1 | 1/1 | 1/1 | - | 1/1 | 1/1 | 1/1 |
| F02 build trees | 1/1 | 1/1 | 1/1 | 1/1 | 2/2 | - | 1/1 | 1/1 | 1/1 |
| F03 dependency/vendor trees | 1/1 | 3/3 | - | - | 2/2 | 1/1 | 1/1 | 3/3 | - |
| F04 many-small-file trees | 1/1 | 1/1 | - | - | 2/2 | - | - | 2/2 | - |
| F05 logs/text | 2/2 | 3/3 | 1/1 | 1/1 | 2/2 | - | 1/1 | 3/3 | - |
| F06 JSON/XML/CSV/structured text | 2/2 | 3/3 | 1/1 | 1/1 | 3/3 | - | 1/1 | 3/3 | - |
| F07 scientific/numeric arrays | 4/4 | 2/2 | 1/1 | 1/1 | 3/3 | - | 1/1 | 3/3 | - |
| F08 databases | 3/3 | 1/1 | 1/1 | 1/1 | 2/2 | - | 1/1 | 2/2 | 1/1 |
| F09 executable/binary objects | 1/1 | 2/2 | - | - | 3/3 | - | 1/1 | 3/3 | - |
| F10 highly redundant binaries | 1/1 | 2/2 | - | - | 2/2 | - | 1/1 | 2/2 | - |
| F11 already-compressed files | 1/1 | 2/2 | - | - | 2/2 | - | 1/1 | 2/2 | - |
| F12 JPEG/images | 2/2 | 2/2 | 1/1 | 1/1 | 1/1 | - | 1/1 | 1/1 | - |
| F13 other media | 2/2 | 4/4 | 1/1 | 2/2 | 2/2 | - | 2/2 | 3/3 | 1/1 |
| F14 archive-inside-archive | 2/2 | 1/1 | - | 1/1 | 1/1 | - | 1/1 | 2/2 | - |
| F15 sparse files | - | 2/2 | - | 1/1 | 1/1 | - | - | 1/1 | 1/1 |
| F16 VM/disk-like images | 1/1 | 1/1 | 1/1 | - | 2/2 | - | - | 2/2 | 1/1 |
| F17 duplicate trees | - | 2/2 | - | - | 2/2 | - | - | 2/2 | - |
| F18 near-duplicate/versioned trees | 1/1 | 3/3 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 | 1/1 |
| F19 metadata-heavy filesystem trees | 1/1 | 1/1 | - | 1/1 | 1/1 | - | 1/1 | 1/1 | - |
| F20 adversarial/high-entropy inputs | - | 3/3 | - | - | 2/2 | 1/1 | 1/1 | 3/3 | - |

Empty family/split combinations: 0

## Real vs. generated, by split

| Split | real | generated | derived-from-real | Total |
|---|---:|---:|---:|---:|
| tuning | 49 | 22 | 7 | 78 |
| validation | 30 | 16 | 7 | 53 |
| heldout | 42 | 17 | 6 | 65 |

Overall: real=121, generated=55, derived-from-real=20

## Independence groups

An independence group is a set of items sharing upstream origin (same project, dataset, image or derivation lineage). Within a family a group appears in one split only; a group shared with `heldout` across families is a manifest error (`split_audit`); see methodology.md section 3.

147 groups over 196 items.

| Group | Families | Splits | Items | Item ids |
|---|---|---|---:|---|
| 7-zip | F11 | heldout | 1 | f11-heldout-7zip-2603-release-archives |
| alacritty | F03 | heldout | 1 | f03-heldout-alacritty-cargo-vendor |
| almalinux | F09, F16 | heldout | 2 | f09-heldout-almalinux-10-usr-binaries, f16-heldout-almalinux-10-genericcloud-qcow2 |
| alpine-linux | F09, F11, F16 | tuning | 4 | f09-alpine-324-rootfs-binaries, f11-alpine-324-apks, f16-alpine-3241-minirootfs-ext4-qcow2, f16-alpine-3241-minirootfs-ext4-raw |
| apache-maven | F14 | tuning | 1 | f14-apache-maven-3916-bin-tarball |
| apache-spark | F14 | validation | 1 | f14-pyspark-420-sdist |
| archlinux | F11 | heldout | 1 | f11-heldout-archlinux-pkgs |
| bts-ontime-performance | F06 | heldout | 1 | f06-heldout-bts-ontime-2024-01-csv |
| census-popest-v2023 | F06 | tuning | 1 | f06-tuning-census-popest-2023 |
| chinook-sample-db | F08 | tuning | 1 | f08-tuning-chinook-sqlite |
| cjson | F02, F18 | validation | 2 | f02-validation-cjson-cmake-build, f18-validation-cjson-releases |
| cpython | F01, F02, F09, F18 | validation | 4 | f01-validation-cpython-3-13-15-git, f02-validation-cpython-build, f09-cpython-3147-embed-win-amd64, f18-validation-cpython-3-12-point-releases |
| curl | F01, F18 | tuning | 2 | f01-tuning-curl-8-19-0-git, f18-tuning-curl-8-x-release-series |
| debian-trixie | F10, F16 | validation | 2 | f10-debian-13-edk2-firmware-images, f16-debian-13-nocloud-20260831-qcow2 |
| direnv | F03 | heldout | 1 | f03-heldout-direnv-go-mod-vendor |
| ebrc-g2-f04-maildir | F04 | tuning | 1 | f04-tuning-maildir |
| ebrc-g2-f04-objstore | F04 | validation | 1 | f04-validation-objstore |
| ebrc-g2-f04-sourcelike | F04 | tuning | 1 | f04-tuning-sourcelike |
| ebrc-g2-f15-coredump | F15 | heldout | 1 | f15-heldout-coredump |
| ebrc-g2-f15-dbprealloc | F15 | tuning | 1 | f15-tuning-dbprealloc |
| ebrc-g2-f15-edgecases | F15 | validation | 1 | f15-validation-edgecases |
| ebrc-g2-f15-fragmented | F15 | validation | 1 | f15-validation-fragmented |
| ebrc-g2-f15-hugehole | F15 | heldout | 1 | f15-heldout-hugehole |
| ebrc-g2-f15-vmraw | F15 | tuning | 1 | f15-tuning-vmraw |
| ebrc-g2-f17-duptree-mixed | F17 | tuning | 1 | f17-tuning-duptree-mixed |
| ebrc-g2-f17-duptree-mono | F17 | heldout | 1 | f17-heldout-duptree-mono |
| ebrc-g2-f17-duptree-nested | F17 | tuning | 1 | f17-tuning-duptree-nested |
| ebrc-g2-f17-duptree-renamed | F17 | heldout | 1 | f17-heldout-duptree-renamed |
| ebrc-g2-f17-duptree-vendor | F17 | validation | 1 | f17-validation-duptree-vendor |
| ebrc-g2-f19-deep | F19 | validation | 1 | f19-validation-deep |
| ebrc-g2-f19-linkstore | F19 | heldout | 1 | f19-heldout-linkstore |
| ebrc-g2-f19-matrix | F19 | heldout | 1 | f19-heldout-matrix |
| ebrc-g2-f19-rsnapshot-a | F19 | tuning | 1 | f19-tuning-rsnapshot-a |
| ebrc-g2-f19-rsnapshot-b | F19 | validation | 1 | f19-validation-rsnapshot-b |
| ebrc-g2-f19-zoo | F19 | tuning | 1 | f19-tuning-zoo |
| ebrc-g2-f20-aesctr | F20 | validation | 1 | f20-validation-aesctr |
| ebrc-g2-f20-bombs-compressed | F20 | validation | 1 | f20-validation-bombs-compressed |
| ebrc-g2-f20-bombs-dense | F20 | tuning | 1 | f20-tuning-bombs-dense |
| ebrc-g2-f20-bombvariants | F20 | heldout | 1 | f20-heldout-bombvariants |
| ebrc-g2-f20-chacha | F20 | heldout | 2 | f20-heldout-chacha-files, f20-heldout-chacha-volume |
| ebrc-g2-f20-csprng | F20 | tuning | 1 | f20-tuning-csprng |
| ebrc-g2-f20-gear-extremes | F20 | tuning | 1 | f20-tuning-gear-extremes |
| ebrc-g2-f20-gearstraddle | F20 | heldout | 1 | f20-heldout-gearstraddle |
| ebrc-g2-f20-sparsedup | F20 | validation | 1 | f20-validation-sparsedup |
| ebrc-gen-logs-a | F05 | tuning | 2 | f05-tuning-gen-logs-a-medium, f05-tuning-gen-logs-a-small |
| ebrc-gen-logs-b | F05 | validation | 1 | f05-validation-gen-logs-b |
| ebrc-gen-logs-c | F05 | heldout | 1 | f05-heldout-gen-logs-c |
| ebrc-gen-numeric-a | F07 | tuning | 2 | f07-tuning-gen-numeric-a-medium, f07-tuning-gen-numeric-a-small |
| ebrc-gen-numeric-b | F07 | validation | 2 | f07-validation-gen-numeric-b, f07-validation-gen-numeric-b-small |
| ebrc-gen-numeric-c | F07 | heldout | 2 | f07-heldout-gen-numeric-c, f07-heldout-gen-numeric-c-small |
| ebrc-gen-sqlite-churn | F08 | heldout | 2 | f08-heldout-gen-sqlite-churn, f08-heldout-gen-sqlite-churn-small |
| ebrc-gen-sqlite-tpch | F08 | tuning | 2 | f08-tuning-gen-sqlite-tpch-medium, f08-tuning-gen-sqlite-tpch-small |
| ebrc-gen-structured-a | F06 | tuning | 2 | f06-tuning-gen-structured-a-medium, f06-tuning-gen-structured-a-small |
| ebrc-gen-structured-b | F06 | validation | 1 | f06-validation-gen-structured-b |
| f-droid-apps | F14 | heldout | 1 | f14-heldout-fdroid-apks |
| fedora | F09, F10, F11 | validation | 3 | f09-fedora-44-usr-binaries, f10-fedora-44-static-libraries, f11-fedora-44-rpms |
| freebsd | F16 | heldout | 1 | f16-heldout-freebsd-151-ufs-vmdk |
| g5-blender-bbb | F13 | tuning | 2 | f13-tuning-bbb-video-large, f13-tuning-bbb-video-medium |
| g5-blender-elephants-dream | F13 | heldout | 1 | f13-heldout-elephants-dream-media-medium |
| g5-blender-sintel | F13 | validation | 1 | f13-validation-sintel-media-medium |
| g5-blender-tears-of-steel | F13 | heldout | 1 | f13-heldout-tos-media-large |
| g5-federal-register-pdf | F13 | validation | 1 | f13-validation-federal-register-pdf-small |
| g5-kodak-photocd-suite | F12, F13 | tuning | 3 | f12-tuning-kodak-jpeg-edge-cases-small, f12-tuning-kodak-jpeg-matrix-medium, f13-tuning-kodak-image-codecs-small |
| g5-librivox-1900-last-president | F13 | tuning | 1 | f13-tuning-librivox-audiobook-medium |
| g5-librivox-reynolds-sf-stories | F13 | heldout | 1 | f13-heldout-librivox-audiobook-medium |
| g5-librivox-tenn-sf-stories | F13 | validation | 1 | f13-validation-librivox-audiobook-medium |
| g5-loc-fsa-owi-color | F12, F13 | validation | 3 | f12-validation-fsa-jpeg-edge-cases-small, f12-validation-fsa-jpeg-matrix-medium, f13-validation-fsa-image-codecs-small |
| g5-loc-highsmith | F12, F13 | heldout | 3 | f12-heldout-highsmith-jpeg-edge-cases-small, f12-heldout-highsmith-jpeg-matrix-medium, f13-heldout-highsmith-image-codecs-small |
| g5-musopen-chopin-flac | F13 | heldout | 1 | f13-heldout-musopen-chopin-flac-medium |
| g5-musopen-kickstarter-flac | F13 | tuning | 1 | f13-tuning-musopen-flac-medium |
| g5-nasa-ivl-photos | F12 | tuning | 3 | f12-tuning-nasa-ivl-photos-large, f12-tuning-nasa-ivl-photos-medium, f12-tuning-nasa-ivl-photos-small |
| g5-nasa-ntrs-pdf | F13 | tuning | 1 | f13-tuning-nasa-ntrs-pdf-small |
| g5-prelinger-archives | F13 | tuning | 1 | f13-tuning-prelinger-video-medium |
| g5-usgs-pubs-pdf | F13 | heldout | 1 | f13-heldout-usgs-pdf-small |
| gen-g4-fat-image | F16 | validation | 1 | f16-gen-fat32-image |
| gen-g4-gpt-disk | F16 | heldout | 1 | f16-heldout-gen-gpt-disk |
| gen-g4-nested-archives-cli | F14 | heldout | 1 | f14-heldout-gen-nested-archives-cli |
| gen-g4-nested-archives-py | F14 | tuning | 1 | f14-gen-nested-archives-py |
| gen-g4-office-docs | F14 | validation | 1 | f14-gen-office-documents |
| gen-g4-redundant-blobs | F10 | tuning | 1 | f10-gen-redundant-binary-blobs |
| gharchive-github-events | F06 | tuning | 2 | f06-tuning-gharchive-2015-01-01-h15-h16, f06-tuning-gharchive-2024-01-15-h15 |
| git-for-windows | F09 | validation | 1 | f09-git-for-windows-portable-2550 |
| golang-go | F01 | heldout | 1 | f01-heldout-go-1-25-0-src |
| govinfo-congressional-record | F05 | heldout | 1 | f05-heldout-congressional-record-2024-01 |
| govinfo-federal-register | F06 | heldout | 1 | f06-heldout-federal-register-xml-2024-01 |
| gutenberg-pd-books | F05 | tuning | 1 | f05-tuning-gutenberg-books |
| gwosc-strain-data | F07 | heldout | 1 | f07-heldout-gwosc-gw150914-h1-4khz-4096s |
| hutter-enwik8 | F05 | tuning | 1 | f05-tuning-enwik8 |
| ietf-rfc-texts | F05 | validation | 1 | f05-validation-rfc-texts |
| jenkins | F14 | tuning | 1 | f14-jenkins-25683-war |
| junegunn-fzf | F03 | tuning | 1 | f03-tuning-fzf-go-mod-vendor |
| lahman-baseball-db | F08 | heldout | 1 | f08-heldout-lahman-baseball-sqlite-2022 |
| linux-firmware | F10 | tuning | 1 | f10-linux-firmware-20260910-subset |
| linux-kernel | F01, F02, F18 | heldout | 3 | f01-heldout-linux-6-6-src, f02-heldout-linux-6-6-defconfig-build, f18-heldout-linux-6-6-stable-subset-series |
| linux-kernel-documentation | F04 | heldout | 1 | f04-heldout-linux-docs |
| llvm-project | F01 | tuning | 1 | f01-tuning-llvm-project-20-1-8-git |
| loghub-hdfs-v1 | F05 | tuning | 1 | f05-tuning-loghub-hdfs-v1 |
| loghub-openssh | F05 | validation | 1 | f05-validation-loghub-ssh |
| loghub-openstack | F05 | tuning | 1 | f05-tuning-loghub-openstack |
| lua | F18 | heldout | 1 | f18-heldout-lua-5-4-releases |
| lz4 | F02, F18 | tuning | 2 | f02-tuning-lz4-intree-make-build, f18-tuning-lz4-releases-1-9-to-1-10 |
| maccdc2012-zeek-logs | F05 | heldout | 1 | f05-heldout-maccdc2012-zeek-logs |
| maven-central-jvm-jars | F11 | validation | 1 | f11-maven-central-jvm-jars |
| microsoft-typescript | F03 | tuning | 1 | f03-tuning-typescript-npm-node-modules |
| mikefarah-yq | F03 | validation | 1 | f03-validation-yq-go-mod-vendor |
| mozilla-firefox | F14 | heldout | 1 | f14-heldout-firefox-15501-linux-tarball |
| mozilla-pdfjs | F03 | heldout | 1 | f03-heldout-pdfjs-npm-node-modules |
| musl | F01, F02, F18 | heldout | 3 | f01-heldout-musl-1-2-5-src, f02-heldout-musl-1-2-5-build, f18-heldout-musl-releases-1-2-x |
| mysql-test-db-employees | F08 | validation | 1 | f08-validation-mariadb11-test-db-employees |
| nasa-giss-gistemp | F07 | validation | 1 | f07-validation-nasa-gistemp-1200km |
| natural-earth-vector | F08 | heldout | 1 | f08-heldout-natural-earth-vector-gpkg |
| ninja-build | F09 | heldout | 1 | f09-heldout-ninja-1132-multiplatform |
| nist-nvd-cve | F06 | heldout | 1 | f06-heldout-nvd-cve-2016-json |
| noaa-ncei-etopo2022 | F07 | heldout | 1 | f07-heldout-noaa-etopo2022-60s-surface |
| noaa-psl-ncep-reanalysis | F07 | tuning | 1 | f07-tuning-noaa-ncep-reanalysis-air-sig995-2020 |
| noaa-storm-events | F06 | validation | 1 | f06-validation-noaa-storm-events-2016 |
| northwind-sample-db | F08 | validation | 1 | f08-validation-northwind-sqlite |
| openstreetmap-planet | F06 | validation | 2 | f06-validation-osm-andorra-20250101-xml, f06-validation-osm-monaco-20250101-xml |
| openwrt | F10 | heldout | 1 | f10-heldout-openwrt-25125-ath79-images |
| postgres-pgbench | F08 | tuning | 1 | f08-tuning-postgres17-pgbench-s40 |
| postgresql | F02 | heldout | 1 | f02-heldout-postgresql-17-6-build |
| powershell | F09 | heldout | 1 | f09-heldout-powershell-766-win-x64 |
| pypi-binary-wheels | F11 | tuning | 1 | f11-pypi-binary-wheels-cp312 |
| pypi-scientific-stack | F03 | tuning | 1 | f03-tuning-pypi-scientific-site-packages |
| pypi-web-service-stack | F03 | heldout | 1 | f03-heldout-pypi-web-site-packages |
| pypi-web-service-stack-sitepkgs | F04 | heldout | 1 | f04-heldout-pypi-site-packages |
| quickjs | F10 | heldout | 1 | f10-heldout-quickjs-20260604-multibuild |
| raspberrypi-firmware | F10 | heldout | 1 | f10-heldout-raspberrypi-firmware-pi4-variants |
| real-yq-vendor-8x | F17 | validation | 1 | f17-validation-real-yq-vendor-8x |
| redis | F01, F02, F18 | validation | 3 | f01-validation-redis-7-2-16-git, f02-validation-redis-intree-build, f18-validation-redis-7-2-point-releases |
| ripgrep | F01, F02, F03, F18 | tuning | 4 | f01-tuning-ripgrep-14-1-1-git, f02-tuning-ripgrep-cargo-target, f03-tuning-ripgrep-cargo-vendor, f18-tuning-ripgrep-commit-snapshots |
| rust-fd-find | F10 | tuning | 1 | f10-rust-fd-find-1050-multiprofile |
| sakila-sample-db | F08 | validation | 1 | f08-validation-sakila-sqlite |
| sdrbench-exafel | F07 | validation | 1 | f07-validation-sdrbench-exafel |
| sdrbench-hurricane-isabel | F07 | tuning | 1 | f07-tuning-sdrbench-hurricane-isabel |
| secrepo-self-weblogs | F05 | heldout | 1 | f05-heldout-secrepo-self-weblogs-2017-01 |
| sharkdp-bat | F03 | validation | 1 | f03-validation-bat-cargo-vendor |
| silesia-corpus | F07, F08, F09 | tuning | 5 | f07-tuning-silesia-mr, f07-tuning-silesia-sao, f07-tuning-silesia-xray, f08-tuning-silesia-osdb, f09-silesia-mozilla-ooffice |
| sqlite | F18 | tuning | 1 | f18-tuning-sqlite-amalgamation-series |
| twbs-bootstrap | F03 | validation | 1 | f03-validation-bootstrap-npm-node-modules |
| twbs-bootstrap-node-modules | F04 | validation | 1 | f04-validation-bootstrap-npm-modules |
| ubuntu-noble | F09, F11, F16 | tuning | 3 | f09-ubuntu-noble-usr-binaries, f11-ubuntu-noble-debs, f16-ubuntu-noble-cloudimg-20260911-raw |
| unicode-ucd | F06 | heldout | 1 | f06-heldout-unicode-ucd-16-0-0-text |
| uutils-coreutils | F09 | heldout | 1 | f09-heldout-uutils-coreutils-0110-macos |
| wikimedia-tnwiki | F06 | tuning | 1 | f06-tuning-wikimedia-tnwiki-20260901 |
| xorg | F11 | heldout | 1 | f11-heldout-xorg-release-tarballs |
| zstd | F01, F02, F18 | tuning | 3 | f01-tuning-zstd-v1-5-7-git, f02-tuning-zstd-cmake-build, f18-tuning-zstd-releases-1-5-x |
