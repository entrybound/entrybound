# EXP-LEGACY-010: real-world legacy artifact prevalence, per-rule strict refusal rates and reference-reader ambiguity (tail-sampled public ecosystems)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (frame enumerators per stratum, seeded sampler, polite fetcher with checksum verification, recursive unwrapper, independent structural census parsers with positive controls, scan pipeline over Entrybound modes and reference readers, analysis `prevalence.py`) |
| Domain / program sections | legacy / §25 (strict defaults, anomaly classes), §26 (format prevalence for adapter scoring); inputs to §29 expectation classes and §35 |
| decision_ids | DEC-LEG-058, DEC-LEG-043, DEC-LEG-102, DEC-LEG-103, DEC-LEG-104, DEC-LEG-105, DEC-LEG-090, DEC-LEG-092, DEC-LEG-094, DEC-LEG-077, DEC-LEG-070, DEC-LEG-085, DEC-LEG-088, DEC-LEG-089, DEC-LEG-099, DEC-LEG-012, DEC-LEG-031, DEC-LEG-032, DEC-LEG-033, DEC-LEG-095, DEC-LEG-061, DEC-LEG-062, DEC-LEG-064, DEC-LEG-065, DEC-LEG-066, DEC-LEG-067, DEC-LEG-069, DEC-LEG-049, DEC-LEG-050, DEC-LEG-052, DEC-LEG-054, DEC-LEG-055, DEC-LEG-072, DEC-LEG-074, DEC-LEG-075, DEC-LEG-106, DEC-LEG-107, DEC-INT-022, DEC-PLT-021, DEC-PLT-051, DEC-LEG-006, DEC-LEG-076, DEC-LEG-022, DEC-LEG-093, DEC-CON-006, DEC-CRY-091, DEC-LEG-024, DEC-LEG-005, DEC-LEG-035, DEC-LEG-037, DEC-LEG-039, DEC-LEG-042, DEC-LEG-047, DEC-LEG-057, DEC-LEG-073 |
| Decision types (proposed) | EMPIRICAL (population rates and distributions); HUMAN-FACING parts of DEC-LEG-006/088/099 are not addressed beyond their mechanical premises |
| OD / HC (proposed) | OD-19 (M19.1, M19.2, M19.3), OD-17 (M17.3), OD-26 (M26.1 premises); HC-07, HC-17 |
| Shared rules / L7 / gates | C§ header, C§2. Network downloads of public artifacts are approved (PROGRESS standing decision; memory note "corpus downloads approved"). No authentication, no licence acceptance, no personal data retention (§6). |
| Timing | `timing: false` |

## 1. Question

In large seeded samples of legacy artifacts actually published by real ecosystems (package registries, distributions,
container registries, archives of the web and of software history), how often does each construct occur, how often
does each strict import rule refuse artifacts on which the reference readers unanimously agree (benign refusals), how
often do reference readers disagree (ambiguity), how concentrated are refusals in single producers, and what are the
distributions of the quantities that import limits bound?

## 2. Hypotheses

- **H1 (strict default, DEC-LEG-058).** For ZIP-family and tar-family artifacts, strict import accepts ≥ 99% of
  unambiguous artifacts in every stratum except those dominated by one producer construct (predicted: Java/JAR data
  descriptors and APK alignment/signing blocks) `[INFERRED]`. *Falsified by* any stratum with benign acceptance below
  0.99 whose refusals are not concentrated (> 50%) in one producer fingerprint.
- **H2 (per-rule bands, DEC-LEG-043/102).** Under the pre-registered band rule (§12.3), at least one current strict
  ZIP rule falls in a "lenient" or "lint" band and at least one rule has zero hits with n ≥ 3,000 (upper 95% bound
  below 0.1%).
- **H3 (ambiguity is rare but present).** Ambiguous artifacts (C§7) are below 1% of each format stratum and strict
  refuses or records all of them (`ambiguity_refusal_fraction` = 1). *Falsified by* one ambiguous artifact imported
  without a recorded conflict (binary, R1 evidence) or ambiguity ≥ 1% in a stratum.
- **H4 (format prevalence for §26).** Among archive-like artifacts reachable through the frames, cpio (RPM payloads,
  initramfs), ar (.deb, static libraries), OCI layers and squashfs each exceed 1% of their ecosystem strata, while
  RAR5, LHA, XAR and WIM are below 1% of every general stratum `[INFERRED]`.
- **H0.** Constructs targeted by strict rules never occur in real artifacts, so strictness costs nothing.

## 3. Decisions informed (premises supplied; selection happens with EXP-LEGACY-001..005, 041, 051)

| Group | Decisions | Measured here |
|---|---|---|
| Strict default and rule bands | DEC-LEG-058, DEC-LEG-043, DEC-LEG-102, DEC-LEG-105 | Per-rule benign refusal rates with exact intervals per stratum; producer concentration; writer-variant rates |
| ZIP constructs | DEC-LEG-031, DEC-LEG-032, DEC-LEG-033, DEC-LEG-070, DEC-LEG-077, DEC-LEG-085, DEC-LEG-088, DEC-LEG-089, DEC-LEG-090, DEC-LEG-092, DEC-LEG-094, DEC-LEG-099, DEC-LEG-103, DEC-LEG-104, DEC-LEG-012, DEC-LEG-076, DEC-CRY-091 | SFX/prefix/gap/multi-disk rates, symlink attributes, ancestor synthesis need, DOS-vs-UT time differences, backslash/colon names by producer, method distribution, missing UT/NTFS extras, encryption types, EOCD multiplicity, extras prevalence, unflagged non-ASCII names by producer and locale hints, mixed methods, APK signing blocks and hidden local entries, stale Unicode Path extras, `__MACOSX/` members, signed JAR/APK prevalence |
| tar constructs | DEC-LEG-061, DEC-LEG-062, DEC-LEG-064, DEC-LEG-065, DEC-LEG-066, DEC-LEG-067, DEC-LEG-069, DEC-PLT-021, DEC-PLT-051 | EB-T007/T008-like end markers, pax/GNU override kinds (truncation, placeholder, unrelated), dialect signatures, `./` prefixes, hardlink topologies, non-UTF-8 names, padding/unterminated tars, SCHILY/LIBARCHIVE keys, GNU sparse variants, pax fractional digits |
| 7z constructs | DEC-LEG-049, DEC-LEG-050, DEC-LEG-052, DEC-LEG-054, DEC-LEG-055 | MTime-less entries by producer, strict 7z refusal causes, coder/filter distribution by 7-Zip version, backslash names, `0x8000` usage |
| Transports | DEC-LEG-072, DEC-LEG-074, DEC-LEG-075, DEC-LEG-106, DEC-LEG-107, DEC-INT-022 | Nested transports and magic collisions, false tar detection over compressed non-tar payloads, xz filters and checks, dictionary-ID frames, skippable/seekable frames, checksum-less zstd and check-None xz |
| Refusal evidence and annotation | DEC-LEG-095, DEC-LEG-006 | Share of refused artifacts that produce no LOM evidence; false-positive rate of construct-based "interpreted differently" annotations (annotation fires but reference readers agree) |
| Limit distributions | DEC-LEG-022, DEC-LEG-093, DEC-CON-006 | Per-artifact requirement vectors (entries, sizes, expansion ratios per unit, dictionary/window sizes, header and extra bytes, path depth, estimated observations) |
| Adapter prevalence | DEC-LEG-024, DEC-LEG-005, DEC-LEG-035, DEC-LEG-037, DEC-LEG-039, DEC-LEG-042, DEC-LEG-047, DEC-LEG-057, DEC-LEG-073 | Format shares per ecosystem stratum; OCI whiteout, Windows pax key, hardlink and duplicate-path rates in layers; NAR structure census |

## 4. Candidates

Candidates are the ledger ids of the decisions above; this experiment evaluates their **prevalence premises** and,
for DEC-LEG-043, pre-registers the band rules as candidate policies (§12.3): `status-quo-binary-strict`,
`r2-bands-as-is`, `r2-bands-closed`, `zero-hit-licensing`, `warn-then-reject-window`,
`lenient-default-deviation-list`. For DEC-LEG-058: `status-quo-strict-default`,
`strict-default-with-per-rule-relaxation`, `compat-default-for-unsigned`, `interactive-suggestion` (HUMAN-FACING part
not evaluable). No candidate is excluded here.

## 5. Corpus: sampling frames, strata, sizes and splits

All frames are public, unauthenticated HTTPS endpoints; the frame listing (identifiers only) is committed as
`research/experiments/EXP-LEGACY-010/frames/<stratum>.jsonl.gz` with retrieval date and SHA-256, except where an
identifier could be personal (§6). Target sizes (pre-freeze sample, tuning + validation):

| Stratum | Frame | Artifacts (target) |
|---|---|---|
| Z-PYPI | PyPI Simple API v1 project list → per-project JSON → 1 latest + 1 random historical file (wheel or zip sdist) | 4,000 |
| Z-MAVEN | Maven Central search API (seeded groupId prefix walk) → jar | 4,000 |
| Z-NUGET | NuGet v3 catalog pages → `.nupkg` | 1,500 |
| Z-VSIX | Open VSX API → `.vsix` | 1,000 |
| Z-XPI | addons.mozilla.org API v5 → `.xpi` | 1,000 |
| Z-APK | F-Droid `index-v2.json` → `.apk` (subject to the §5.4 lineage check below) | 1,500 |
| Z-OFFICE | Common Crawl CDX index (OOXML/ODF/EPUB MIME types) → WARC record range fetch | 2,000 |
| Z-GOMOD | `index.golang.org` → `proxy.golang.org/<module>/@v/<v>.zip` | 1,000 |
| Z-RETRO | Internet Archive advanced search (software collections) → `.zip`, SFX `.exe` with ZIP payload | 1,500 |
| Z-WEB | Common Crawl CDX (`application/zip`, `application/x-zip-compressed`) | 500 |
| T-DEBSRC | Debian trixie `Sources.xz` → `orig.tar.*`, `debian.tar.*` | 3,000 |
| T-DEB | Debian trixie `Packages.xz` → `.deb` (ar → control/data tar) | 2,000 |
| T-CRATES | crates.io sparse index → `.crate` | 2,500 |
| T-NPM | npm registry package documents (replicate changes feed sample) → tarball | 2,500 |
| T-PYSDIST | PyPI (from Z-PYPI frame) `.tar.gz` sdists | 1,500 |
| T-ALPINE / T-ARCH | Alpine `APKINDEX`; Arch core/extra db → `.apk`, `.pkg.tar.zst` | 1,600 |
| T-OCI | Docker Hub official-image repositories (anonymous registry token) → linux layers (gzip/zstd) | 1,500 |
| T-UPSTREAM | ftp.gnu.org, kernel.org, archive.apache.org listings → release tarballs incl. `.tar.lz` | 1,400 |
| T-CONDA / T-GEM / T-CRAN / T-BREW | conda-forge repodata (`.conda`, `.tar.bz2`); rubygems names → `.gem`; CRAN `PACKAGES`; Homebrew bottle manifests (ghcr anonymous) | 2,000 |
| S-7Z | Internet Archive `.7z`; Scoop main/extras bucket manifests (public git) → `.7z` URLs; Wikimedia dumps `.7z` | 3,000 |
| X-STREAM | Wikimedia dumps (`.bz2`, `.gz`), GH Archive, OpenStreetMap diffs, Common Crawl `.br`/`.Z`/`.lzma`/`.lz4` objects | 3,000 |
| A-OTHER | Fedora RPMs (cpio payloads), Alpine netboot initramfs (cpio), Debian `-dev` static libraries (ar), AppImage feed and Snap store (squashfs), Internet Archive `.rar`/`.lha`/`.cab`/`.iso`, Aminet `.lha`, python.org macOS `.pkg` (xar), cache.nixos.org `.nar.xz` from a channel `store-paths` list | 3,000 |

- **Unwrapping** records parent-child relations: `.deb` → ar members → tars; RPM → cpio payload; `.apk` → gzip
  members; `.conda` → zip → tar.zst; `.gem` → tar → `data.tar.gz`; OCI blobs; SFX `.exe` → ZIP payload; nested
  archives inside ZIPs (jar-in-war) to depth 3. Children are analysed as their own formats and weighted to parents.
- **Independence groups** = publisher/project key (PyPI project, Maven groupId, Debian source package, npm package,
  crate, IA item, Scoop app, image repository, CC registered domain). At most 3 artifacts per group.
- **Size cap** 256 MiB per artifact; exclusions by cap are counted per stratum (selection bias reported).
- **Splits.** Pre-freeze sample: split = tuning if `SHA-256("ebr-legacy-010-v1" ‖ stratum ‖ group_key)` mod 5 ∈ {0,1,2},
  validation if ∈ {3,4}. Tuning is used to build and debug census predicates, rule-to-reason-code maps and limit
  candidates; validation is looked at once for confirmation (look registry).
- **Held-out placeholder (§5 of the method).** A sealed post-freeze sample: frames re-enumerated after Commit A, a draw
  by an owner-held seed (committed now by the owner only as SHA-256), restricted to independence groups absent from the
  pre-freeze sample, ≥ 30% of each stratum's pre-freeze size; not fetched before Commit C.
- **§5.4 lineage check.** The upstream sources of every stratum are declared above; before the first validation look a
  held-out-aware session records pass/fail per stratum against held-out lineages without revealing them; a failing
  stratum is excluded from any held-out-evidence claim for overlapping families.
- **Positive controls (C§9).** Every census predicate is validated on EXP-LEGACY-001..004 tuning cases whose labels
  carry ground truth before any stratum is scanned.

## 6. Environment, politeness and privacy

`wsl-ubuntu`; fetcher `User-Agent` names the research program and a contact-free project URL; `robots.txt` honoured;
≤ 2 concurrent connections and ≤ 5 requests/s per host; registry-published checksums verified where offered (PyPI
sha256, Debian/Arch/Alpine index hashes, crates cksum, npm integrity, Maven `.sha1`). No authentication, cookies or
licence acceptance (Windows container layers, Microsoft media and trial-licensed tools are not fetched). **Privacy:**
Common Crawl documents (Z-OFFICE, Z-WEB, X-STREAM web objects) may contain personal data: their bytes are deleted after
the census, only structural records are kept, and their URLs are committed as SHA-256 only (raw URL lists stay under
`/root/eb-research/prevalence/private/`, never committed). Reference readers run in bubblewrap sandboxes (C§12). Linux
builds of the Windows anchors stand in for them (validated in EXP-LEGACY-001 H1); a seeded 2% sample is replayed on the
Windows anchors.

## 7. Commands and tooling

Tooling (`research/tools/legacy/prevalence/`): `frames/<stratum>.py` enumerators; `sample.py --stratum S --seed 2509171001 --target N --group-cap 3 --size-cap 268435456`;
`fetch.py --sample <jsonl> --cache /root/eb-research/prevalence/cache --verify-published-checksums`;
`unwrap.py --depth 3`; census parsers `research/tools/legacy/census/{zip_census.py, tar_census.py, sevenz_census.py, stream_census.py, other_census.py}`
(pure Python, independent of Entrybound, emitting construct flags, producer fingerprint and requirement vector);
`scan_artifact.py` (census → `ebound convert --strict --dry-run` and, for ZIP, `--compat=<each profile> --dry-run` and
`--preserve --compat=<p> --dry-run`; reference-reader listing/content digests); `shard.py` (writes shards of 500
artifacts as manifest items); analysis `research/tools/legacy/prevalence/prevalence.py`.

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python research/tools/legacy/prevalence/controls.py --cases EXP-LEGACY-001,EXP-LEGACY-002,EXP-LEGACY-003,EXP-LEGACY-004 --out research/normalized/EXP-LEGACY-010/controls.json   # gate: every predicate recall = precision = 1
for s in $(cat research/experiments/EXP-LEGACY-010/strata.txt); do
  /root/eb-research/venv/bin/python research/tools/legacy/prevalence/frames/$s.py --out research/experiments/EXP-LEGACY-010/frames/$s.jsonl.gz
  /root/eb-research/venv/bin/python research/tools/legacy/prevalence/sample.py --stratum $s --seed 2509171001 --frames research/experiments/EXP-LEGACY-010/frames/$s.jsonl.gz --out /root/eb-research/prevalence/samples/$s.jsonl
  /root/eb-research/venv/bin/python research/tools/legacy/prevalence/fetch.py --sample /root/eb-research/prevalence/samples/$s.jsonl --cache /root/eb-research/prevalence/cache --verify-published-checksums
done
/root/eb-research/venv/bin/python research/tools/legacy/prevalence/shard.py --samples /root/eb-research/prevalence/samples --shard-size 500 --manifest research/experiments/EXP-LEGACY-010/cases-manifest.jsonl
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-010/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-010/spec.yaml
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-010/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/prevalence/prevalence.py --experiment EXP-LEGACY-010 --split tuning --out research/normalized/EXP-LEGACY-010/tuning/
# after tuning analysis is frozen in record.md and registered as a look:
/root/eb-research/venv/bin/python research/tools/legacy/prevalence/prevalence.py --experiment EXP-LEGACY-010 --split validation --out research/normalized/EXP-LEGACY-010/validation/
```

## 8. Seeds

Sampling `2509171001`; `order 2509171011`, `bootstrap 2509171012`, `command 2509171013`.

## 9. Warmup, replication

No warmup. Each shard is scanned twice (2 repetitions, different processes); per-artifact observation tuples must be
identical (repeat-hash). Fetches are not repeated; bytes are content-addressed and re-verified by SHA-256 on each scan.

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `benign_false_refusal_fraction` (per rule, per stratum and format condition, over unambiguous artifacts; rule attribution by first reason code and by census-predicted triggers) | T-20 | OD-17 | T-20 |
| `import_acceptance_fraction` (strict; each ZIP compat profile; unambiguous artifacts) | T-20 | OD-19 | T-20 |
| `import_agreement_fraction` / `import_majority_agreement_fraction` | T-20 / descriptive | OD-19 | T-20 / none |
| `ambiguity_refusal_fraction` (ambiguous artifacts) | binary | OD-19 | §3.3 |
| `scale_limits` (requirement-vector distributions) | descriptive | OD-17 | none |
| construct prevalence per stratum with Clopper-Pearson 95% intervals; producer concentration (max share of a rule's hits from one producer fingerprint and from one group); share of refusals without LOM evidence; annotation false-positive rate; format shares | non-canonical descriptive (secondary) | — | none |

## 11. Normalization

Rates are per artifact within a stratum; parent-level summaries weight each parent 1 and its children equally.
Stratum-pooled rates are reported both unweighted and weighted by the frame size of each stratum (sensitivity §14).

## 12. Statistical analysis

1. **Controls gate** (C§9) before any rate is reported.
2. **Rates** with exact Clopper-Pearson two-sided 95% intervals per stratum and format condition; per-rule rates by
   (a) production first-refusal reason code and (b) census-predicted triggers (all rules), reported side by side.
3. **DEC-LEG-043 band rule, pre-registered.** A rule's band is assigned only when its whole 95% interval lies inside one
   band of the candidate policy being evaluated; otherwise it is `band_ambiguous`. Producer concentration > 0.5 is
   reported and the band is also computed with that producer excluded. Bands of `r2-bands-as-is`: `< 0.1%` reject with
   named error and escape hatch; `[1%, 5%]` lenient with deviation; `> 10%` lint only; other intervals unaddressed.
   `r2-bands-closed` fills `[0.1%, 1%)` with warn-then-reject and `(5%, 10%]` with lenient. `zero-hit-licensing`
   rejects only rules with zero hits and n ≥ 3,000. Stratum results are never pooled into one band.
4. **Canonical metrics** per evaluation condition (format × stratum) with the family structure: strata map to corpus
   families (ZIP-family packages F11/F14, OCI F16, source tarballs F01/F18); the §4.9 support check is applied; groups
   are independence groups (§5).
5. **Look discipline.** Tuning results define predicates, maps and limit candidates; validation confirms rates once.

## 13. Practical significance

T-20 `a = 0.5/n_cases` for canonical fractions (`decision-method.md` §3.2); descriptive prevalence carries no band; the
DEC-LEG-043 bands are candidate policy definitions, not method thresholds.

## 14. Sensitivity analysis

Frame-weighting arm (unweighted vs frame-size weighted); group-cap arm (cap 1 vs 3); size-cap arm (exclude > 64 MiB);
historical-release arm (latest-only vs latest + historical for Z-PYPI/T-NPM/T-CRATES); reference-set arm (C§7 set vs
set minus 7-Zip); leave-one-stratum-out for pooled statements; Windows-anchor replay arm (2% sample).

## 15. Expected negative results worth recording

Strict rules with benign refusal rates far above 1% in major ecosystems (JAR descriptors, APK blocks); census
predicates failing controls (their prevalence is withheld); ambiguity near zero in all strata (differential corpora
are then the only ambiguity evidence); candidate adapter formats absent from every frame.

## 16. Threats to validity

Frames reachable without authentication under-represent private and enterprise archives (enterprise ZIPs, backups);
Common Crawl favours web-published documents; latest-release sampling favours modern writers (historical arm);
group caps hide producer concentration inside large publishers; reference readers themselves may share defects
(unanimity is not truth); census parsers could share blind spots with Entrybound (they are written without reading
Entrybound code, and controls are required).

## 17. Platform requirements

Linux x86-64 (WSL2), public network egress, ~200 GB free on D: via the WSL distro during scanning, no privileged host
operations; Windows anchors via interop for the 2% replay; owner-restricted substrata (Windows container layers,
licence-gated media) excluded unless the owner performs acquisition.

## 18. Estimates

Download ≈ 150 GB (peak cache ≈ 180 GB, pruned to hashes and structural records after C-analyze for web strata);
compute ≈ 200 CPU-hours scan (≈ 45,000 artifacts plus children × census, 6-10 Entrybound modes and 4-5 reference readers)
plus network-bound fetch time (1-3 days at politeness limits); disk ≈ 200 GB peak. Agent effort: enumerators, census
parsers and scan pipeline are scripted tooling (Sonnet); band assignment and interpretation judgment (non-exposed
analysis session, C§ header).

## 19. Expected cost tiers `[INFERRED]`

Not applicable (no candidate implementation measured); rule relaxations it motivates carry the tiers stated in
EXP-LEGACY-001/002/003.
