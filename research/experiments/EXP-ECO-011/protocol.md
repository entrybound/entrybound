# EXP-ECO-011: Registry artifact tail scan — legacy-audit yield, strict-refusal prevalence and checker agreement

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-011 |
| Domain | ecosystem (program §32; cross-cuts §25-§27 legacy import rules, §5 corpus prevalence) |
| Status | READY (public downloads approved; every checker is installed, installable or vendored at a pinned commit; sampling and scan scripts specified below) |
| Runner | `spec.yaml` (checker × sampled artifact, ad-hoc manifest of the tuning sample); arms B and C are scripts |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` (§6 sampled external populations) |
| Timing | none |

## 1. Question

Across a pre-registered, seeded, stratified sample of real published package artifacts (PyPI sdists and wheels, npm tarballs, crates.io crates, Maven Central jars, RubyGems gems, Go module zips, Debian source and binary packages, OCI image layers), how often does each strict legacy-import rule of the implemented reader fire, how concentrated are refusals by producer, how often do incumbent parsers disagree about the same artifact, how often does the implemented strict or compatibility import accept an artifact on which reference readers disagree without declaring it, how many benign artifacts do default resource budgets refuse, and how do these verdicts compare with incumbent checkers (PyPI warehouse validation, `zipdetails`, Python `zipfile`, bsdtar, GNU tar, 7-Zip)?

## 2. Hypotheses and falsification

- **H1 (tail differs from head).** At least one strict rule's violation rate in the tail strata exceeds its rate in the head strata such that the two exact 0.99 intervals do not overlap (Research III's Experiment 1 was head-biased: 446 ZIPs, 81% Gradle and Maven distributions, REQ-ECO-0044). *Falsified* if every rule's head and tail intervals overlap.
- **H2 (refusals are producer-concentrated).** For every rule with ≥ 20 violations, the top producer accounts for ≥ 50% of that rule's violations. *Falsified* by any such rule with a top-producer share below 50%.
- **H3 (no silent acceptance of ambiguity).** On every artifact where the unanimous-reference condition fails (reference readers disagree on names, sizes, count or content), strict import either refuses or records the ambiguity (`ambiguity_refusal_fraction` = 1). *Falsified* by any silent acceptance (an HC-07 violation of the status quo).
- **H4 (dot-slash prevalence).** `./`-prefixed member names occur in more than 1% of Debian binary-package `data.tar` members' archives and of source tarballs in at least one stratum (DEC-LEG-065). *Falsified* if the upper 0.99 bound is below 1% in every stratum.
- **H5 (default budgets rarely refuse benign artifacts).** `benign_false_refusal_fraction` under default budgets is below 0.001 in every stratum (upper 0.99 bound). *Falsified* if any stratum's lower bound exceeds 0.001.
- **H6 (upstream issue reproduction, arm B).** The two drafted warehouse behaviours (a `UnicodeDecodeError` on names with the general-purpose UTF-8 flag and invalid UTF-8; acceptance of a short `usize` field) reproduce on the latest CPython 3.14.x and 3.13.x and on warehouse HEAD. *Falsified* per behaviour and version by non-reproduction.

## 3. Decisions informed

DEC-ECO-001 (feasibility and outcome of the registry tail scan study), DEC-ECO-035, DEC-ECO-075 (legacy audit value versus incumbent checkers), DEC-ECO-050 (pre-upload validation prototype against warehouse validation), DEC-ECO-015 (arm B reproduction), DEC-ECO-030 (arm C policy drift), DEC-LEG-006 (false-positive rate of construct-based annotations), DEC-LEG-043 (per-rule violation rates with exact bounds and producer concentration), DEC-LEG-065, DEC-LEG-042 (OCI layer whiteout, Windows pax key, hardlink and duplicate prevalence), DEC-LEG-024 and DEC-LEG-037 (format prevalence in registries and distribution sources), DEC-LEG-075 (xz filter and check usage), DEC-LEG-085 (ZIP method distribution).

The decisions about *which* bands and defaults to adopt (DEC-LEG-043 candidates `r2-bands-as-is`, `r2-bands-closed`, `zero-hit-licensing`, `warn-then-reject-window`, `lenient-default-deviation-list`) are applied by the legacy-domain analysis to these rates; this experiment pre-registers the rates' measurement, not the bands.

## 4. Candidates

Checkers run on every sampled artifact (the "candidates" of the spec; reference: the implemented strict import):

| Candidate | Definition | Ledger mapping |
|---|---|---|
| `ebound-strict-dry-run` | `ebound convert <artifact> - --strict --dry-run` (format inferred; `--from` given for container-in-container layers) | DEC-ECO-035 status-quo-dry-run; DEC-LEG-043 status-quo-binary-strict |
| `ebound-compat-dry-run` | `--compat=<profile>` for each frozen compatibility profile applicable to the format | DEC-LEG-043 lenient-default-deviation-list (as implemented profiles) |
| `ebound-default-budget` | strict import under the default caller `ResourceBudget` | budget false refusals (H5) |
| `warehouse-validate` | the PyPI warehouse distribution-file validation function vendored at a pinned commit (Apache-2.0), wheels and sdists only | DEC-ECO-050 pre-upload-validation; DEC-ECO-030 pypi-profile-only |
| `zipdetails-parse` | `zipdetails` exit status and warnings | DEC-ECO-035 comparison |
| `python-zipfile`, `python-tarfile` | Python 3.12 stdlib listing (and `tarfile` `data` filter verdict) | reference reader |
| `bsdtar-list` | `bsdtar -tvf` (libarchive 3.7.2) | reference reader |
| `gnutar-list` | GNU tar 1.35 `-tvf` | reference reader |
| `sevenzip-list` | 7-Zip 23.01 `7z l -slt` | reference reader |
| `go-archive` | Go `archive/zip` and `archive/tar` listing (pinned Go toolchain) | reference reader |
| `java-zip` | Java 21 `java.util.zip.ZipFile` listing | reference reader (ZIP family) |
| `repo-zip-compat`, `repo-tar-strict` | `tools/zip-compat` and `tools/tar-strict` rule checkers at the execution SHA (read-only use; never regenerating tracked outputs) | per-rule attribution |

Excluded: `zipalign -c` (Android build tools require accepting licence terms; accepting terms needs owner approval); commercial malware scanners; any upload or issue filing (sending messages is outside agent authority; DEC-ECO-015 filing is an owner action).

## 5. Sampled populations (not corpus items)

Conventions §6 apply. Frames downloaded, hashed and stored under `/root/eb-research/cache/eco011/frame/` before sampling:

| Stratum | Frame | Unit | Tuning n | Validation n |
|---|---|---|---:|---:|
| pypi-head | top 5,000 projects of the public monthly `top-pypi-packages` dataset | latest release: sdist and one wheel | 500 projects | 500 |
| pypi-tail | PyPI simple index (JSON form) minus pypi-head | as above | 2,500 projects | 2,500 |
| npm | npm registry replication `_all_docs` keys (dropped and recorded if the endpoint is unavailable) | latest version tarball | 2,000 | 2,000 |
| crates | crates.io daily database dump | latest non-yanked `.crate` | 2,000 | 2,000 |
| maven | Maven Central repository index | latest version main jar | 2,000 | 2,000 |
| rubygems | RubyGems compact index `versions` | latest `.gem` | 1,000 | 1,000 |
| go | Go module index (`index.golang.org`) | latest module zip from `proxy.golang.org` | 1,000 | 1,000 |
| debian-src | Ubuntu noble main+universe `Sources` | `.orig.tar.*` (plus `.debian.tar.*`) | 1,000 | 1,000 |
| debian-bin | Ubuntu noble main+universe `Packages` (amd64) | `.deb` (`data.tar.*` inside) | 1,000 | 1,000 |
| oci | official-images library definitions (docker-library/official-images) | every layer of the linux/amd64 image of the default tag, pulled with `skopeo` from a public mirror respecting anonymous pull limits | 150 images | 150 |

Sampling: one seeded permutation per stratum (seed 2026091711, stratum name hashed into the stream), first block tuning, second block validation; sample lists committed before any artifact is scanned. Held-out: a third block drawn **after the design freeze** from a fresh frame with an owner-held seed whose SHA-256 is committed before the freeze (sealing route, §5.4); nobody draws it before then. Expected totals: about 15,800 artifacts per split (≈ 47,000 over three splits; the "45,000-artifact registry tail scan" named in DEC-ECO-001).

## 6. Environment and platform requirements

WSL2 Ubuntu 24.04 (`wsl-ubuntu`); downloads to `/root/eb-research/cache/eco011/` streamed and deleted after scanning (hashes and extracted metadata kept; artifacts not redistributed); per-host politeness: ≤ 4 concurrent requests per host, exponential back-off on 429/503, `User-Agent` naming the research project without personal data; Windows host for nothing. Not privileged. `disk_guard.py` checked every 1,000 artifacts; the scan pauses if D: free space falls below the guard.

## 7. Procedure and commands

Scripts in `research/tools/ecosystem/registry/` (to be written):

1. `frames.py fetch` — download frames; `frames.py hash`; commit `frames.json` (URLs, retrieval UTC, SHA-256, sizes).
2. `sample.py --seed 2026091711 --frames frames.json --out samples/{tuning,validation}.jsonl` — resolve each sampled unit to concrete artifact URLs with expected digests where the registry publishes them (PyPI, npm `dist.integrity`, crates.io checksum, Maven `.sha1`, Go sumdb `h1:`, Debian `SHA256` fields).
3. `fetch_scan.py --sample samples/tuning.jsonl --workers 8` — download, verify digest against the registry's published digest (mismatch recorded and excluded as `integrity_mismatch`), write an ad-hoc manifest row `{item_id, split: tuning, path, family: <stratum>}` to `/root/eb-research/cache/eco011/manifest-tuning.jsonl`, and hand off to the runner in batches.
4. `ebr run --spec spec.yaml` (checker × artifact); `check_artifact.py` normalizes each checker's output to: accepted/refused, reason codes, rule ids, listing digest (sorted names, sizes, types, link targets, content hashes where extracted), features (§9), producer string.
5. `analyze.py` — per stratum: rule violation counts and exact Clopper-Pearson two-sided 0.99 intervals; producer concentration per rule (top-producer share, Herfindahl index); reference-reader agreement (unanimous, majority, split); `ambiguity_refusal_fraction`, `import_acceptance_fraction`, `import_agreement_fraction`, `benign_false_refusal_fraction`; checker-pair confusion matrices (ebound strict versus warehouse, versus zipdetails, versus stdlib).
6. Arm B `reproduce_upstream.py` — obtain CPython latest 3.13.x and 3.14.x as pinned python-build-standalone releases; clone warehouse at HEAD; regenerate the two Research III fixtures with the generator scripts from the hash-verified external instrumentation archive (`research/tools/sources/extract_external_sources.ps1` output under `D:/eb-research/sources`), never copying the unpublished text into the repository; run the behaviours; record reproduced yes/no per version.
7. Arm C `policy_drift.py` — `git log --follow` over warehouse's distribution-validation source files since 2018; count commits changing validation logic (non-comment, non-test diff lines) per year; list rule additions and removals by diffing the raise sites.
8. Validation look: steps 3-5 on `samples/validation.jsonl` with `spec-validation.yaml`, registered in the look registry.

## 8. Seeds, warmup, replication

Sampling seed 2026091711; runner seeds `order` 2026091711, `bootstrap` 2026091711, `command` 2026091711. Checker outcomes are deterministic metrics on stored bytes: 2 repetitions per (checker, artifact) in different processes and rounds, plus the repeat-hash check on normalized outputs (§4.6, §4.13); a disagreement marks that (checker, artifact) `unstable`, it is reported, and it is excluded from rates with the exclusion counted. Artifacts are kept on disk until both repetitions of every checker have run, then deleted. No warmups.

## 9. Metrics and measurement

| Metric | Canonical | Role |
|---|---|---|
| `import_acceptance_fraction` | yes (T-20, M19.1) | banded; per format × policy (strict, each compat profile) over the sampled case set |
| `import_agreement_fraction` | yes (T-20, M19.2) | banded; over unambiguous artifacts, imported EAM equals the unanimous reference result |
| `import_majority_agreement_fraction` | yes (M19.2) | descriptive |
| `ambiguity_refusal_fraction` | yes (HC-07, M19.3) | binary: must be 1 under strict |
| `benign_false_refusal_fraction` | yes (T-20, M17.3) | banded (minimize); default budgets |
| `rule_violation_count`, `rule_violation_rate_ci99`, `top_producer_share`, `producer_hhi` | no | descriptive record fields per rule and stratum |
| `reader_disagreement_class` | no | unanimous / majority / split per artifact |
| feature prevalences: `dot_slash_prefix`, `dot_root_entry`, `v7_trailing_slash_dirs`, `tar_format`, `xz_filters`, `xz_check`, `zip_methods`, `zip64`, `data_descriptor`, `utf8_flag_invalid_names`, `duplicate_names`, `absolute_or_dotdot_names`, `symlinks`, `hardlinks`, `oci_whiteouts`, `windows_pax_keys`, `long_tail_format` | no | descriptive record fields |

## 10. Normalization and aggregation

Per stratum first (strata are evaluation conditions, AGG-S); population-weighted estimates across strata only as a secondary report with weights = frame sizes. Headline for T-20 fractions: minimum over strata (AGG-C style). No pooling of tuning and validation samples.

## 11. Statistical analysis and use in the decision rule

- Prevalence estimates: exact Clopper-Pearson intervals at two-sided 0.99 (the base per-comparison confidence of §4.12) per stratum; head-versus-tail comparisons are descriptive (non-overlap reported), not decision verdicts.
- Candidate comparisons (strict versus compatibility profiles; ebound strict versus warehouse policy for DEC-ECO-050 scopes): exact paired counts on the same case set against the T-20 floor `0.5/n_cases`; confirmatory on the validation sample (one look); R4 per stratum with scoped winners at R9 only where the product can express the scope (a named policy profile).
- `ambiguity_refusal_fraction` < 1 is an R1 violation (HC-07) of the policy that silently accepted, with the artifact digest as the counterexample (the artifact itself is not committed; its registry URL and digest are).
- Minimum support: every stratum's n is pre-registered above; strata that end with fewer than 80% of planned artifacts after integrity exclusions are reported as under-sampled.

## 12. Practical-significance thresholds

T-20 (`import_acceptance_fraction`, `import_agreement_fraction`, `benign_false_refusal_fraction`) as in §3.2 and Appendix A.2; binary per §3.3; descriptive fields have none.

## 13. Sensitivity analysis

Excluding artifacts produced by the top producer of each stratum; excluding artifacts whose frame age exceeds 5 years; unweighted versus frame-weighted cross-stratum estimates; alternative reference-reader sets (dropping each reader in turn from the unanimity condition).

## 14. Held-out (Phase D placeholder)

Sealed third block drawn after the design freeze (§5 above; §5.4 sealing route); run once with the frozen checkers; report-only after unlock (§5.6). Because the frame is re-downloaded after the freeze, drift between frames is reported.

## 15. Expected negative results worth recording

Strict rules that fire on large shares of legitimate Java or Python artifacts (would make strict defaults unusable for gatekeepers without profiles); warehouse and ebound strict verdicts disagreeing widely; OCI layers with whiteouts and hardlinks common enough that refusal is impractical; `./` prefixes ubiquitous in Debian binary packages.

## 16. Threats to validity

- Frames are not the population of artifacts people download; head and tail strata bracket the difference.
- "Latest version" sampling misses historical producers.
- Registries may throttle; partial strata are reported, not silently shrunk.
- Registry samples are not corpus items: before decision use, a corpus-methodology addendum must record the sample lists as immutable (L4-style change control) and name them in each record.
- Legal and ethical: artifacts are analysed, not redistributed; if a sample contains a security-relevant finding, disclosure is an owner decision and no agent contacts third parties.

## 17. Cost estimate

- Compute: about 80 h for tuning and validation (≈ 31,600 artifacts × 14 checkers × 2 repetitions; most checkers run in well under a second; large jars and OCI layers dominate).
- Network: about 150 GB downloaded over the two splits; disk peak about 30 GB (streamed deletion).
- Agent effort: scripted; judgment only for mapping checker messages to rule ids (`rule-map.yaml`, one session plus review) and for arm C's classification of validation-logic commits.

## 18. Gates and exposure

Conventions §1, §2 and §6. No corpus family is used, so the session's L7 exposure does not apply to the sampled populations; the analysis still runs in a session not involved in candidate construction.
