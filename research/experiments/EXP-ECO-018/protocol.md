# EXP-ECO-018: Task-complexity metrics per first-use task for CLI candidates and incumbents

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-018 |
| Domain | ecosystem (program §33; cross-cuts §32 workflows, §22 crypto CLI, §15 extraction policy) |
| Status | READY (task scripts executed for runnable candidates and incumbents; synopsis-only counts for unimplemented candidates follow committed synopses) |
| Runner | `spec.yaml` (runnable candidate × task fixture, ad-hoc manifest); synopsis-only counts are computed by `count_synopsis.py` |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` (§5 human-facing evidence) |
| Timing | none |

## 1. Question

For each pre-registered first-use task, how many commands, option tokens, required concepts and unresolved decision points does each CLI candidate require to meet the task's mechanically checked completion criterion with its defaults, compared with the incumbent tools users already know — the mechanical, decision-capable part of the §33 usability evidence (C6, §4.16)?

## 2. Hypotheses and falsification

- **H1 (simple tasks stay one line).** For T01 pack and T02 unpack the status quo needs one command and zero options (REQ-ECO-0049). *Falsified* by any required option.
- **H2 (restoration load).** For T15 (restore ownership, xattrs and ACLs as root) the status quo needs at least three options, more than GNU tar's `--same-owner --xattrs --acls` equivalent or bsdtar's `-p`; a preset candidate (DEC-ECO-011 `preset-flag` or `tar-p-style`) needs one. *Falsified* if the status quo needs fewer than three or the preset candidate more than one.
- **H3 (authorized-signer verification).** Verifying that an archive was signed by a specific key (T11) needs more steps with the status quo than with `minisign -Vm <file> -P <key>` (one command), because the status quo has no allowed-key option. *Falsified* if the status quo meets the criterion in one command.
- **H4 (CI encryption).** Non-interactive encryption in automation (T17) cannot be completed with a password under the status quo (no non-terminal source) and needs a recipient-key workflow of at least two commands. *Falsified* if a one-command password path exists.

## 3. Decisions informed

DEC-ECO-011, DEC-ECO-023, DEC-ECO-049, DEC-ECO-005, DEC-ECO-007, DEC-ECO-066, DEC-ECO-062, DEC-ECO-009, DEC-ECO-055, DEC-ECO-063, DEC-ECO-044, DEC-ECO-083 (the `task-complexity-metrics` method), DEC-CMP-013, DEC-ACC-051, DEC-ACC-008, DEC-INT-043, DEC-INT-004, DEC-CRY-040, DEC-CRY-086, DEC-CRY-101, DEC-CRY-063, DEC-CRY-082, DEC-CRY-009, DEC-LEG-013, DEC-PLT-022, DEC-PLT-004, DEC-PLT-007, DEC-PLT-012.

## 4. Candidates

- **Runnable:** status-quo dev-HEAD `ebound`; REF-PROD `9e44608`; incumbents per task (GNU tar 1.35, bsdtar, Info-ZIP `zip`/`unzip`, 7-Zip, `zstd`, age and age-keygen, minisign, signify, gpg, `ssh-keygen -Y`, cosign, OpenSSL `ts` with `curl` for RFC 3161, `diff -r`, PowerShell `Compress-Archive`/`Expand-Archive` on Windows).
- **Synopsis-only (not implemented):** one synopsis file per ledger candidate that changes the CLI surface, committed under `research/experiments/EXP-ECO-018/synopses/<decision>/<candidate_id>.synopsis` **before any count is computed**, written by the candidate owner (or transcribed mechanically from the ledger description and the SPEC synopsis at its recorded hash, and then approved by the candidate owner). Candidates covered: DEC-ECO-005 (spec-aliases-only, aliases-plus-tar-flag-subset, compat-shim-binary, canonical-profile-flag, no-aliases-documented-mapping); DEC-ECO-007 (spec-17-2-set, union-set, core-four-plus-namespaces, minimal-v1-subset, tar-style-mode-flags); DEC-ECO-009 (minimal-output-flags, output-flags-plus-no-color, environment-variables, config-file); DEC-ECO-011 (preset-flag, tar-p-style, hidden-advanced, config-profiles); DEC-ECO-023 (spec-all-hidden, intent-presets, interactive-confirmation, tar-mirroring-defaults, config-file-defaults); DEC-ECO-044 (password-fd, password-file, environment-variable, password-command, recipient-keys-for-automation); DEC-ECO-049 (spec-exclude, gitignore-dialect, glob-dialect, file-list-input, creation-plan-file, capture-policy-flags); DEC-ECO-055 (workload-named-aliases, explain-recommendation, budget-driven-selection, level-scale, fewer-profiles); DEC-ECO-062 (distinct-sidecar-suffix, checksum-manifest-style); DEC-ECO-063 (content-only-opt-out, explicit-binding-list, policy-presets); DEC-ECO-066 (spec-authoritative synopsis); DEC-CMP-013 (spec-flags-bounded, expert-policy-file, lookback-override-only); DEC-ACC-008 (directory-and-legacy-inputs, spec-mode-flags, rename-detection); DEC-ACC-051 (include-exclude-full-pass, indexed-random-selective); DEC-INT-043 (spec-default-plus-deep, three-levels, full-plus-keyless-public, unpack-verify-only-alias); DEC-INT-004 (on-collision-policy, force-flag, interactive-prompt); DEC-CRY-009 (no-bind-physical-flag, explicit-binding-list); DEC-CRY-040 (generate-identity-ebk1, text-recipient-encoding, external-keygen-tool); DEC-CRY-063 (optional-directory, no-labels-by-default); DEC-CRY-082 (allowed-public-keys, allowed-signer-ids, require-key-with-require-flags, policy-file); DEC-CRY-086 (add-pack-sign, add-sign-list-remove, add-timestamp-attach); DEC-CRY-101 (emit-timestamp-request, optional-online-tsa-client, documented-openssl-recipe); DEC-LEG-013 (split-import-export-verbs, spec-flag-names); DEC-PLT-004 (weak-mode-requires-opt-in, refuse-weak-platforms); DEC-PLT-007 (named-grants-closed-set, bsdtar-style-p-flag, python-filter-model, named-presets); DEC-PLT-012 (five-way-overwrite-policy, staged-directory-then-rename); DEC-PLT-022 (spec-metadata-class-flags, profile-based-classes).
- Excluded: candidates that change no user-visible surface (they have identical counts to the status quo by construction; listed as such); interactive-only candidates are counted with their prompts as decision points.

## 5. Task catalogue (tuning set; mechanically checked completion criteria)

| Task | Statement given to counters and sessions | Completion criterion (`check_task.py`) |
|---|---|---|
| T01 | Pack directory `project/` into an archive | archive exists; `unpack` of it reproduces the tree's logical fingerprint |
| T02 | Unpack `project.eb` into a new directory | extracted fingerprint equals the source |
| T03 | Check that `release.eb` is intact | exit 0 on the intact file and non-zero on the pre-corrupted twin |
| T04 | Encrypt `secret/` for the holder of `alice.pub` | only `alice` identity unlocks; content matches |
| T05 | Unpack an encrypted archive with your identity | fingerprint equals source |
| T06 | Extract only `docs/guide.md` to stdout | stdout bytes equal the entry |
| T07 | Read `docs/guide.md` from `http://127.0.0.1:<port>/release.eb` | bytes equal; full archive not downloaded |
| T08 | Import `legacy.zip` rejecting ambiguous input | archive produced or specific refusal on the ambiguous twin |
| T09 | Export `release.eb` as a tar that loses nothing | tar produced losslessly or refusal naming losses |
| T10 | Find out which metadata a pack did not preserve | the answer file lists the classes the oracle knows were lost |
| T11 | Verify `release.eb` was signed by the key `maintainer.pub` | success for the genuine signature; failure for an attacker-key signature |
| T12 | Create a signing key | key usable by T11 |
| T13 | Publish `project/` as `.eb`, `.zip` and `.tar.zst` deterministically | three artifacts, byte-identical on rerun |
| T14 | Create a sidecar for `release.tar.gz` and later check the tarball against it | check passes for the original and fails for a modified copy |
| T15 | As root, unpack restoring ownership, xattrs and ACLs | oracle compares restored metadata |
| T16 | Extract only the `src/` subdirectory | extracted tree equals the subtree |
| T17 | In a non-interactive CI job, encrypt `artifacts/` so a deploy job can decrypt it | both jobs complete without a terminal |
| T18 | List what changed between `v1.eb` and `v2.eb` | answer equals ground-truth path sets |
| T19 | Make the smallest archive suitable for long-term release distribution | archive produced with the densest profile the candidate exposes for that intent |
| T20 | Rebuild the index of `release.eb` (or change it to STREAM) | output archive has the requested layout and identical LAI |
| T21 | Add recipient `bob.pub` to an encrypted archive, then remove `carol` | only alice and bob unlock |
| T22 | Check whether `release.tar.gz` corresponds to `release.eb` | correct yes/no on the matching and the mismatching pair |
| T23 | Attach a trusted timestamp to a signature | token attached and verified offline against the given anchor |
| T24 | Audit `upload.zip` for parser-disagreement hazards without creating output | verdict and named hazards on the pre-registered hazardous zip |
| T25 | Unpack an archive containing a setuid binary without granting setuid, then with granting it | first run has no setuid bit; second has it |
| T26 | Unpack into an existing non-empty directory keeping existing files | no existing file modified |
| T27 | Stream-pack to stdout and unpack from stdin in a pipeline | fingerprint equals source |
| T28 | Print the entry list as machine-readable JSON | output parses and lists every entry |

A validation task set (V01-V28: same capabilities with different fixtures, names and parameters, for example two input directories for T01, an archive with 10,000 entries for T28) is authored now in `tasks/validation/` before any count. A held-out task set is authored by a session not involved in CLI candidate design and sealed (conventions §5). Task weights for aggregate reporting: equal, and, as a sensitivity arm, weights from EXP-ECO-013 incumbent operation frequencies (tasks below 1% at the upper bound reported unweighted).

## 6. Environment and platform requirements

WSL2 Ubuntu 24.04 for runnable Linux candidates and incumbents (root for T15 and T25); Windows host for PowerShell incumbents; local nginx for T07; no external network.

## 7. Procedure and commands

Scripts in `research/tools/ecosystem/tasks/` (to be written):

1. `fixtures.py --seed 2026091718` — per task fixture directories (tuning and validation).
2. For each (runnable candidate, task): the minimal command sequence is written as `scripts/<candidate>/<task>.sh` by one session, **independently re-derived** by a second session that sees only the tool's help or manual and the task statement; the shorter sequence that passes the checker is kept, and both are committed (the pair shows counter disagreement).
3. `run_task.py --candidate <c> --task <t>` executes the script in a fresh scratch, runs `check_task.py`, and prints counts per §9 (runner-driven, `spec.yaml`).
4. `count_synopsis.py --synopsis <file> --task <t>` — for synopsis-only candidates, a script sequence written against the synopsis under the same two-session rule; counted without execution; flagged `synopsis_only`.
5. `aggregate.py` writes `research/normalized/EXP-ECO-018/task-complexity.csv` (task × candidate) and per-decision comparison tables restricted to the decision's candidates and tasks.

## 8. Seeds, warmup, replication

Seed 2026091718 for fixtures and order. Runnable scripts executed twice in different processes; the checker verdict and counts must agree. Counts are deterministic properties of committed scripts. No warmups.

## 9. Metrics and measurement

**Counting rules (pre-registered; also used by EXP-ECO-006, -007, -017).**

- `commands`: number of executable invocations in the script (pipelines count each program; shell built-ins and `cd` excluded; environment-variable assignments prefixing a command are counted as options).
- `flags_per_task`: number of option tokens across commands; an option with an attached or following value counts once; bundled short options count per letter; positional arguments (paths, URLs) are not options; a required subcommand word is not an option.
- `concepts`: number of distinct entries from the pre-registered concept vocabulary `concepts.yaml` that the script requires the user to name or choose (for example layout, profile, recipient, identity, grant, target profile, lossy approval, binding, policy file, verification level, pattern dialect, timestamp authority).
- `decision_points`: number of choices the task requires for which the candidate offers no default that meets the completion criterion.
- `keystrokes`: characters of the script with paths, URLs and values replaced by one-character placeholders (descriptive).

| Metric | Canonical | Role |
|---|---|---|
| `flags_per_task` | yes (C6, M25.2) | cost input; aggregated as mean and maximum over tasks (objective M25.2) |
| `migration_steps_count` | yes (M26.2) | descriptive for migration tasks T08, T09, T13, T14, T22 |
| `commands`, `concepts`, `decision_points`, `keystrokes`, `task_completed`, `synopsis_only`, `counter_disagreement` | no | record fields |

## 10. Normalization and aggregation

AGG-S per task (a task is an evaluation condition); per candidate the mean and maximum `flags_per_task` over applicable tasks; comparisons are restricted to the tasks a decision's candidates affect. No averaging of executed and synopsis-only counts into one figure without the flag.

## 11. Statistical analysis and use in the decision rule

Exact deterministic counts; no intervals. `flags_per_task` and `concepts` feed the C6 cost element: a new user-visible flag or mode is at least T2 and a new required user concept is T3 (§7.2), and counts enter R10 key ordering through cost tiers. They are EMPIRICAL measurements of CLI designs, not of people (§4.16): they may decide HUMAN-FACING decisions' mechanical parts; any claim that fewer flags means users succeed more often is a human claim (EXP-ECO-021). Held-out task counts are required for the mechanical parts' R5 (§1.2 HUMAN-FACING row).

## 12. Practical-significance thresholds

None: `flags_per_task` is a cost input (Appendix A.2), and the other counts are descriptive; integer differences are exact.

## 13. Sensitivity analysis

Equal versus usage-frequency task weights; counts from the longer of the two independently derived scripts; excluding root-only tasks; counting environment variables as zero options.

## 14. Held-out (Phase D placeholder)

Conventions §5: sealed held-out task set, opened after the freeze, counted once for every CLI candidate that reached the freeze.

## 15. Expected negative results worth recording

Status quo more complex than bsdtar for restoration; no one-command authorized-signer verification; CI password encryption impossible; synopsis candidates that reduce flags by adding concepts.

## 16. Threats to validity

Minimal scripts by experts understate novice sequences (EXP-ECO-019 is the heuristic check); synopsis-only candidates may hide complexity their implementation would add; the concept vocabulary is a judgment (committed before counting, reviewed by a second session).

## 17. Cost estimate

About 3 h compute; about 5 GB disk; agent effort judgment for writing and independently re-deriving scripts (two sessions, about 28 tasks × about 10 runnable tools and about 90 synopsis candidates, restricted per decision to relevant tasks), scripted for execution and aggregation.

## 18. Gates and exposure

Conventions §1, §2 and §5. No corpus family is used.
