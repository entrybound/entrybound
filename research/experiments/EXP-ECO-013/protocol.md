# EXP-ECO-013: Incumbent archive and signing CLI usage census from public build, packaging and CI scripts

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-013 |
| Domain | ecosystem (program §32, §33) |
| Status | READY (public downloads; parser and extraction scripts specified below) |
| Runner | script-driven census (one pass per sample); no per-item timing or candidate commands, so no `ebr` spec |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` (§6 sampled external populations) |
| Timing | none |

## 1. Question

In real build, packaging, release and CI scripts, how are incumbent archive, compression, verification and encryption tools invoked: which flags are used and how often (creation, extraction, restoration, compression level and codec parameters, quiet/verbose/progress/thread options), how often are they used in pipes (stdin/stdout), how do scripts handle their exit statuses, which of their machine-readable outputs are parsed and which fields, and how do scripts supply passwords and keys non-interactively?

## 2. Hypotheses and falsification

- **H1 (small flag vocabulary).** At least 80% of `tar` creation invocations use only flags from a set of at most 12 distinct normalized options. *Falsified* if more than 12 options are needed to cover 80%.
- **H2 (pipes are common).** At least 20% of archive-tool invocations in CI and build scripts read from stdin or write to stdout. *Falsified* if the upper 0.99 bound is below 20%.
- **H3 (exit statuses are mostly ignored or binary).** Fewer than 5% of archive-tool invocations distinguish non-zero exit statuses (for example tar's 1 versus 2, or diff's 1 versus 2). *Falsified* if the lower 0.99 bound exceeds 5%.
- **H4 (codec-level flags).** Compression-level or codec-parameter flags (for example `-19`, `--long`, `-9e`, `-mx=9`) appear in at least 30% of compressor invocations, more often than any named preset option. *Falsified* if below 30% (upper bound) or presets dominate.
- **H5 (secrets in argv).** Among scripts that supply passwords to archive or encryption tools non-interactively, at least one quarter pass them on the command line or through environment variables rather than file descriptors or files. *Falsified* if the upper 0.99 bound is below 25%.

## 3. Decisions informed

DEC-ECO-005 (flags used in real CI scripts; install counts), DEC-ECO-008 (scripting study of failure handling), DEC-ECO-009 (CI usage of global flags), DEC-ECO-010 (which machine-readable output fields are parsed), DEC-ECO-011 (restoration flags usage), DEC-ECO-049 (pack options used in CI and release scripts), DEC-CMP-013 (codec-level override usage versus presets), DEC-ACC-022 (pipe usage frequency), DEC-INT-021 (handling of verification exit codes for `gpg`, `cosign`, `minisign`, `sha256sum -c`), DEC-ECO-044 (non-interactive secret sources in automation), DEC-LEG-030 (pipe-based conversion usage), DEC-ECO-004 (CI cache usage patterns).

## 4. Candidates

This is a census of incumbent practice; it does not run decision candidates. Its outputs are the task frequencies and flag vocabularies that EXP-ECO-018 uses to weight and choose realistic tasks, and the descriptive inputs to the decisions above. Tools censused: `tar`, `bsdtar`, `gtar`, `zip`, `unzip`, `7z`/`7za`/`7zr`, `gzip`, `pigz`, `zstd`, `xz`, `bzip2`, `lz4`, `cpio`, `ar`, `gpg`, `age`, `minisign`, `signify`, `cosign`, `sha256sum`/`shasum`, `openssl` (`enc`, `ts`, `dgst`), `restic`, `borg`, `rclone`, `curl`/`wget` piped into archive tools, and container `docker save`/`load` piped into compressors.

## 5. Sampled populations (not corpus items)

Conventions §6 apply.

| Stratum | Frame | Unit extracted | Tuning n | Validation n |
|---|---|---|---:|---:|
| ubuntu-src | Ubuntu noble main+universe `Sources` index | source package: `debian/rules`, `debian/*.install`, maintainer scripts, plus upstream scripts inside `.orig.tar.*` (`*.sh`, `Makefile*`, `*.mk`, `CMakeLists.txt`, `Dockerfile*`, `justfile`, `package.json` scripts, `.github/workflows/*.yml`, `.gitlab-ci.yml`, `.travis.yml`, `azure-pipelines.yml`, `Jenkinsfile`, `.circleci/config.yml`, `release*`/`dist*` scripts) | 2,000 | 2,000 |
| corpus-trees | tuning or validation corpus source trees | same file patterns | tuning: `f01-tuning-ripgrep-14-1-1-git`, `f01-tuning-zstd-v1-5-7-git`, `f01-tuning-curl-8-19-0-git`, `f01-tuning-typescript-5-9-3-git`, `f01-tuning-llvm-project-20-1-8-git` | validation: `f01-validation-redis-7-2-16-git`, `f01-validation-bootstrap-5-3-8-git`, `f01-validation-cpython-3-13-15-git` |
| popcon | Debian popularity-contest `by_inst` public statistics | install counts for the tools above | full list | same list re-fetched at the validation date |

Sampling: seeded permutation of source package names (seed 2026091713), first block tuning, second validation; held-out: third block from a fresh index after the freeze with an owner-held sealed seed (§5.4).

## 6. Environment and platform requirements

WSL2 Ubuntu 24.04; downloads from an Ubuntu mirror (politeness: ≤ 4 concurrent connections); `bashlex` and `PyYAML` pinned in the research venv; disk streamed (tarballs deleted after extracting matching script files). Not privileged.

## 7. Procedure and commands

Scripts in `research/tools/ecosystem/usage/` (to be written):

1. `frame.py` — fetch and hash `Sources` indexes; `sample.py --seed 2026091713` writes `samples/{tuning,validation}.txt` (committed before extraction).
2. `fetch_extract.py` — download each sampled package's `.dsc`-listed files, verify SHA-256 from `Sources`, extract only the file patterns of §5 into `/root/eb-research/cache/eco013/scripts/<split>/<package>/`, delete tarballs.
3. `commands.py` — extract command invocations: shell files and `run:`/`script:` blocks of CI YAML parsed with `bashlex` (fallback: tokenizer on parse failure, flagged); Makefile recipes via `make -pn`-free recipe line extraction; `package.json` scripts; Dockerfile `RUN` lines. Output rows: (package, file, line, tool, argv tokens, pipeline position, redirections, enclosing control construct, `set -e`/`pipefail` state when determinable).
4. `normalize_flags.py` — per tool, a committed normalization table `flag-tables/<tool>.yaml` derived from the tool's own manual at the version shipped in noble (short/long equivalences, bundled short options such as `-czvf`, attached values) written before any counting; unknown tokens reported, not guessed.
5. `classify.py` — exit handling class per invocation: `ignored` (`|| true`, `; true`, not last in a non-`pipefail` pipeline), `set-e-binary`, `explicit-nonzero` (`if ! cmd`, `|| exit`), `status-distinguishing` (`$?` compared to specific values, `case $?`); output parsing: piped into or captured by `grep`, `awk`, `sed`, `jq`, `cut`, `python -c`, and which option produced machine-readable output (`--json`, `-slt`, `--format`, `--porcelain`); secret supply: `--passphrase-fd`, `--passphrase-file`, `--password-file`, `-p<value>`, `-P <value>`, environment variables named `*PASS*`/`*KEY*`, `pass:`, `env:`, `file:`, `fd:` forms; global flags (`-q`, `-v`, `--progress`, `-T`, `--threads`, `--no-color`).
6. `report.py` — frequencies per stratum with exact Clopper-Pearson two-sided 0.99 intervals; flag coverage curves (smallest option set covering x% of invocations); co-occurrence of restoration flags (`-p`, `--same-owner`, `--no-same-permissions`, `--xattrs`, `--acls`, `--selinux`, `--numeric-owner`).
7. Validation pass: steps 2-6 on the validation sample; frequencies reported beside tuning; a tuning statement "reversed" on validation (non-overlapping intervals in the opposite direction) is recorded as counterevidence.

## 8. Seeds, warmup, replication

Sampling seed 2026091713. The extraction and counting pipeline is deterministic: it runs twice on the same stored files in different processes and the output tables must be byte-identical (§4.13). No warmups.

## 9. Metrics and measurement

No canonical metric applies directly (Appendix A.2 has no census-of-practice metric); every output is a descriptive record field: `invocations`, `flag_usage_fraction` per normalized option, `flag_coverage_set_size_80`, `pipe_usage_fraction`, `exit_handling_class_distribution`, `machine_output_parsed_fraction` and parsed field names, `secret_supply_class_distribution`, `global_flag_usage_fraction`, `codec_parameter_flag_fraction`, `popcon_installs`. These feed task selection and weights in EXP-ECO-018 and are cited as `EMPIRICALLY_MEASURED` context; they never select a candidate on their own (they are not banded metrics, §0.2 item 4).

## 10. Normalization and aggregation

Per stratum (ubuntu-src, corpus-trees) and per script kind (CI YAML, build files, packaging rules, shell scripts); invocation-level counts with package-level clustering reported (fraction of packages with at least one occurrence) to avoid one package dominating.

## 11. Statistical analysis and use in the decision rule

Descriptive estimation with exact intervals; no candidate comparison. The outputs inform EXP-ECO-018's task weights (pre-registered there as a rule: tasks whose incumbent operation occurs in fewer than 1% of invocations at the upper 0.99 bound are reported but not weighted) and the problem statements of the decisions listed in §3.

## 12. Practical-significance thresholds

None (descriptive; Appendix A.2 roles). The 1% task-weighting rule in §11 is an EXP-ECO-018 design rule, not a decision threshold.

## 13. Sensitivity analysis

Package-level versus invocation-level frequencies; excluding packages whose scripts are vendored copies of the same upstream file (deduplicated by file SHA-256); excluding `bashlex` fallback-tokenized lines.

## 14. Held-out (Phase D placeholder)

Conventions §6 and §8: a sealed third sample after the freeze; its frequencies are reported beside tuning and validation and used only to check stability of EXP-ECO-018 task weights.

## 15. Expected negative results worth recording

Scripts almost never parse archive tools' machine-readable output; restoration flags rarely used; secrets frequently on the command line; exit statuses almost never distinguished (weakening the value of a rich exit-code contract).

## 16. Threats to validity

Debian packaging scripts over-represent Linux distribution practice and under-represent application CI on hosted services; upstream CI files inside orig tarballs are often stripped by packagers; static parsing misses generated commands and variables holding flags (flagged as `dynamic` and reported).

## 17. Cost estimate

About 8 h compute (download and extraction dominate); about 60 GB downloaded, peak disk about 40 GB; agent effort scripted plus judgment for the per-tool flag normalization tables (one session, reviewed against the manuals).

## 18. Gates and exposure

Conventions §1, §2 and §6. Only F01 corpus items are used (not in the exposure record).
