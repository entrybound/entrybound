# EXP-ECO-015: CLI surface census — parser conformance, help coverage, exit statuses, diagnostics, machine-readable output and synopsis divergence

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-015 |
| Domain | ecosystem (program §33; cross-cuts §29 reason codes and §28 conformance) |
| Status | READY (black-box and source-census scripts over the implemented CLI; incumbent tools installed or pinned downloads) |
| Runner | script-driven census (`research/tools/ecosystem/clisurface/`); a scenario × build matrix, not a workload experiment |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |
| Timing | none |

## 1. Question

What exactly is the implemented CLI's surface and behaviour contract, measured mechanically: which options each command parses and in which syntactic forms (`--opt=value`, `--opt value`, repeats, `--` terminator, unknown and accepted-but-ignored options); whether help documents every parsed option; which exit status, outcome class and reason code each misuse, fault and environmental failure produces, compared with incumbent tools on equivalent failures; whether messages name the failed requirement and a remedy; which commands emit machine-readable output, how stable and canonical it is across builds; how the implementation diverges from the SPEC synopsis and from the repository documentation; which emitted reason codes lack tests; whether ambient environment settings change output bytes; how many warning and report lines typical trees produce; and whether unsalted digests printed by inspection can be inverted for low-entropy metadata?

## 2. Hypotheses and falsification

- **H1 (parser inconsistency).** At least one option accepts `--opt value` but not `--opt=value` (or the reverse), no command honours a `--` terminator, and at least one option is accepted and silently ignored by some command (REQ-ECO-0117). *Falsified* if every option accepts both forms, `--` works everywhere and no option is ignored.
- **H2 (help gaps).** At least one parsed option is absent from its command's help text (REQ-ECO-0059). *Falsified* if help coverage is complete.
- **H3 (actionability).** Fewer than 80% of refusal and failure messages across the scenario list meet the actionability rubric (§9). *Falsified* if at least 80% meet it (the measured fraction and its case list are the result either way).
- **H4 (environmental faults misclassified).** At least one environmental fault (disk full, permission denied, broken pipe, connection reset, proxy mangling) is reported as `CORRUPT` or `POLICY_REFUSED` rather than with an environment-specific reason code (DEC-MOD-032). *Falsified* if every environmental fault has a specific reason code distinct from data-integrity codes.
- **H5 (ambient determinism).** Pack output bytes are unchanged under variations of `TZ`, `LANG`/`LC_ALL`, `umask`, `SOURCE_DATE_EPOCH`, current directory, `HOME` and `TMPDIR` for the same input tree. *Falsified* by any change in PCI where no captured metadata changed (umask affecting captured modes of newly created inputs is excluded by constructing inputs once).
- **H6 (JSON canonicality).** Receipts and migration reports are valid RFC 8785 (JCS) canonical JSON (REQ-ECO-0085). *Falsified* by any non-canonical output (non-ASCII escaping rules, number formatting, key order).
- **H7 (digest inversion).** Unsalted digests of POSIX ACL entries keyed by numeric uid/gid and of short xattr values (≤ 8 bytes) are invertible by enumeration in under one CPU-hour on this host. *Falsified* if enumeration of the pre-registered value spaces exceeds one CPU-hour.

## 3. Decisions informed

DEC-ECO-006, DEC-ECO-007 (trace matrix from SPEC capabilities and workflows to commands), DEC-ECO-008, DEC-ECO-009 (ambient-setting reproducibility risk), DEC-ECO-010, DEC-ECO-024, DEC-ECO-036 (inventory of loss presentations), DEC-ECO-066 (divergence inventory), DEC-MOD-014, DEC-MOD-032, DEC-MOD-041 (emitted codes lacking tests; cross-reader consistency of codes for CLI-reachable faults), DEC-LEG-026 (doc-code drift of refusal classes), DEC-ACC-021 (digest inversion feasibility; inspect on refused archives), DEC-ACC-022 (distinct non-seekable diagnostic), DEC-ACC-049 (CLI tests for window violations and index-rebuild spelling), DEC-ACC-054 (window refusal messages), DEC-PLT-001 (refusal messages naming the grant), DEC-PLT-013 and DEC-PLT-015 (inventory of loss and refusal messages; class and reason strings emitted), DEC-PLT-022 (metadata class names consistent across CLI, registry and FidelityReport), DEC-PLT-059 (warning volume on real trees), DEC-INT-021 (keyless verify exit status against the exit-code contract), DEC-CRY-086 (synopsis reconciliation for signature commands).

## 4. Candidates

This census measures the status quo (dev HEAD, with REF-PROD `9e44608` beside it for stability across releases) and incumbent tools on equivalent scenarios. It evaluates candidates of these decisions only where they are *contracts over observable behaviour* that can be checked against the measured behaviour:

- DEC-ECO-008 exit-status candidates (`status-quo`, `status-quo-plus-distinct-usage-io`, `sysexits`, `diff-convention`, `tar-convention`, `reason-code-ranges`): each candidate's mapping table (committed before the census, transcribed from its description and the cited conventions) is applied to the scenario list, giving per candidate the number of scenarios whose status would change from the status quo and the number of statuses that collide (two distinguishable outcomes mapped to one status).
- DEC-ECO-010 output candidates: measured status-quo properties that each candidate would have to change (commands without JSON, non-canonical outputs, schema drift between REF-PROD and dev HEAD).
- DEC-ECO-006 parser candidates: conformance matrix of the status quo; `declarative-parser` footprint comes from EXP-ECO-001.

Incumbents for equivalent scenarios: GNU tar 1.35, bsdtar 3.7.2, Info-ZIP `zip` 3.0 and `unzip` 6.0, 7-Zip 23.01, `zstd`, `xz`, GNU `diff`, `curl`, `gpg`, `minisign`, `cosign` (pinned), `sha256sum`, `restic` (for JSON output practice).

## 5. Inputs

- Builds: dev HEAD and REF-PROD `ebound` (conventions §4).
- Fixtures generated by `fixtures.py --seed 2026091715`: small plain and encrypted INDEXED and STREAM archives; truncated, bit-flipped and unknown-feature archives (from the conformance corpus when available, else generated with the harness `ebr-pack` inspect-bytes offsets); legacy ZIP, tar, 7z inputs including symlinks, setuid entries, hostile names; a tree with symlinks and xattrs; an archive with an absolute symlink.
- Real trees for warning and report volume: tuning items `f19-tuning-real-home-snapshot`, `f19-tuning-real-ext4-acl-selinux` (mounted read-only), `f01-tuning-ripgrep-14-1-1-git`, `f02-tuning-zstd-cmake-build`; validation look: `f19-validation-real-home-snapshot`, `f19-validation-real-xfs-acl-selinux`, `f01-validation-redis-7-2-16-git`.
- SPEC at its recorded SHA-256 (read in place under `D:/Projects/entrybound/design/`; only option and command identifiers are extracted, no text is copied into the repository); repository `docs/*.md`; `crates/entrybound/src/diagnostics.rs`; `crates/entrybound-cli/src/lib.rs`; test sources.

## 6. Environment and platform requirements

WSL2 Ubuntu 24.04 (root and `setpriv --reuid=65534 --regid=65534 --clear-groups` for permission scenarios; 64 MiB loop mount for disk full; `ebr-netem` proxy for connection reset and header mangling); Windows host for the Windows-only scenarios (console code page and path forms) with `ebound.exe`. No network beyond localhost.

## 7. Procedure and commands

Scripts in `research/tools/ecosystem/clisurface/` (to be written):

1. `parsed_options.py` — extract every parsed option per command from `crates/entrybound-cli/src/lib.rs` (string literals in match arms and comparisons), cross-checked black-box by `probe_options.py`, which invokes each command with each option in forms F1 `--opt value`, F2 `--opt=value`, F3 repeated, F4 after a `--` terminator, F5 unknown option, F6 option valid for another command, F7 missing value; records exit status, stderr and whether the option took effect (effect oracle per option in `option-effects.yaml`, e.g. `--profile dense` changes the plan profile in `inspect --json`).
2. `help_coverage.py` — `ebound --help` and `ebound <command> --help` for every command; options parsed but not documented, and documented but not parsed.
3. `scenarios.py` — runs the scenario list `scenarios.yaml` (committed before any run) on both builds and on incumbents where an equivalent exists. Scenario groups: U (usage: missing argument, unknown command, conflicting options, value out of range), D (data: truncated, corrupt, unknown critical feature, encrypted without unlock, wrong password, wrong identity), P (policy: symlink escape, setuid entry, budget exceeded, lossy export without approval, window violation in STREAM repack, `--index present` versus `--rebuild-index` spelling), E (environment: output exists, disk full, permission denied on output, read-only source, SIGPIPE on stdout, connection reset, proxy mangles `Content-Range`, non-seekable input to a seeking command), S (signatures: no signatures present, stale binding, attacker key, keyless verify of an encrypted archive). For each: exit status, first stderr line, outcome class, reason code, whether any output object remains.
4. `actionability.py` — applies the rubric of §9 mechanically to every refusal message; two independently written rule sets (by two sessions) are applied and disagreements listed; the fraction reported is from the pre-registered primary rule set, with the second as sensitivity.
5. `exit_mappings.py` — applies each DEC-ECO-008 candidate mapping table to the scenario outcomes (§4).
6. `json_census.py` — which commands accept `--json` or `--format json`; validate outputs against RFC 8785 with an independent JCS implementation (pinned PyPI `rfc8785`); parse with Python `json`, Go `encoding/json` and `jq`; infer schemas (field names, types) for REF-PROD and dev HEAD on the same fixtures and diff them.
7. `divergence.py` — command and option identifiers from SPEC §15-§17 and §20.4 synopses versus implemented parsed options; repository `docs/*.md` refusal class and reason-code mentions versus `diagnostics.rs` enum and emitted codes; each divergence row classified missing-in-code, missing-in-doc, renamed, semantic-difference-suspected (the last flagged for human-free review by a second session).
8. `code_coverage.py` — every `ReasonCode` variant: emitted by some code path (static search), exercised by some test (static search of test sources for the variant name), exercised by the scenario list (dynamic) — lists of codes without tests.
9. `ambient.py` — for a fixed input tree created once, `ebound pack` under each ambient variation of H5 (one at a time) and PCI SHA-256 comparison; also `publish` and `convert --to` outputs.
10. `volume.py` — pack and unpack the real trees of §5 as root and as the unprivileged user; count stderr warning lines and restoration-report lines per class.
11. `invert_digests.py` — for every digest field printed by `inspect --json` over the fixtures, determine whether it is salted (by repeat-packing identical metadata and comparing); for unsalted metadata digests, enumerate the pre-registered value spaces (uid/gid 0-65535 × ACL permission sets; xattr values of length ≤ 4 over printable ASCII; ≤ 8 bytes over lowercase hex) and record time to invert on 8 threads (descriptive).
12. Output tables in `research/normalized/EXP-ECO-015/`.

## 8. Seeds, warmup, replication

Seed 2026091715 for fixtures and scenario order. Every scenario runs twice in different processes; outcome tuples must be identical, else reported as nondeterministic behaviour (an HC-03/HC-15 concern for the status quo). No warmups; `invert_digests.py` times are descriptive.

## 9. Metrics and measurement

**Actionability rubric (primary rule set, pre-registered; used by EXP-ECO-006, -009, -016, -017).** A refusal or failure message is actionable if and only if all hold: (A1) it contains a stable reason code or an outcome class token; (A2) it names the failed requirement as one of: the violated policy or grant, the missing input (unlock material, file), the exceeded budget field, or the damaged structure; (A3) it names at least one remedy that exists in the CLI's help text (an option, a command, or an explicit "cannot be overridden" statement); (A4) it names the object (path, entry or URL) when one exists. The fraction is over the pre-registered scenario list; n_cases is the list length.

| Metric | Canonical | Role |
|---|---|---|
| `error_actionability_fraction` | yes (T-20, M25.3) | banded |
| `reason_code_specificity_fraction` | yes (T-20, M04.2) | banded: fraction of scenarios whose reason code identifies the injected fault class uniquely |
| `unsafe_defaults` | yes (HC-05) | reported where a scenario reveals one (full census in EXP-ECO-016) |
| `flags_per_task` | yes (C6) | not measured here (EXP-ECO-018) |
| parser conformance cells, help gaps, exit status per scenario, status collisions per mapping candidate, JSON commands, JCS validity, schema diff, divergence rows, codes without tests, ambient PCI changes, warning and report line counts, inversion time | no | descriptive record fields |

## 10. Normalization and aggregation

AGG-S per scenario group (U, D, P, E, S) with the worst group as headline for `error_actionability_fraction`; AGG-P for surface facts; volume counts per tree and user.

## 11. Statistical analysis and use in the decision rule

Deterministic census, no inference. Candidate comparisons (exit mappings; later, candidate CLIs implemented by owners) use exact T-20 fractions over the same scenario list with the `0.5/n_cases` floor; one validation look on the validation trees and a validation scenario list (variants of each scenario with different fixtures, authored before any run). Status-quo defects found (e.g. ignored options, non-canonical JSON claimed canonical, unspecific environment codes) are recorded as defects under objective §2.0 rule 5 and as counterevidence. H7 results feed DEC-ACC-021's redaction candidates as feasibility evidence; the privacy judgement itself is not made here.

## 12. Practical-significance thresholds

T-20 for the two banded fractions (§3.2, Appendix A.2); none for descriptive fields.

## 13. Sensitivity analysis

Actionability under the second independent rule set; with and without A4; exit mappings evaluated with incumbent-derived scenario weights from EXP-ECO-013 (scenarios weighted by incumbent failure-handling frequency) as a descriptive alternative.

## 14. Held-out (Phase D placeholder)

Conventions §5: a sealed held-out scenario list (same groups, new fixtures and wording-independent faults) authored by a session not involved in CLI design, opened after the freeze, run once on every CLI candidate that reached the freeze.

## 15. Expected negative results worth recording

Exit status 1 unused while usage errors and I/O errors share codes with data classes; some commands lacking help; JSON outputs non-canonical; codes without tests; environment faults reported as corruption; unsalted short metadata digests trivially invertible.

## 16. Threats to validity

Static extraction of parsed options from a hand-written parser can miss dynamically built option names (mitigated by the black-box probe); the actionability rubric is mechanical and can pass unhelpful messages or fail helpful ones (two rule sets, reported); SPEC synopsis extraction depends on formatting.

## 17. Cost estimate

About 6 h compute (scenarios are small); about 10 GB disk; agent effort scripted plus judgment for `scenarios.yaml`, `option-effects.yaml`, the second actionability rule set and divergence classification (two sessions).

## 18. Gates and exposure

Conventions §1 and §2. No exposed family is used (F01, F02, F19 only).
