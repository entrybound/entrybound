# EXP-ECO-017: Secret material in automation — non-interactive password sources, unlock normalization, key-file permissions

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-017 |
| Domain | ecosystem (program §32, §33; cross-cuts §21 platform, §22.1 crypto CLI) |
| Status | READY (probe scripts, pty and ConPTY drivers from pinned packages, incumbent tools installed or pinned downloads) |
| Runner | script-driven matrices (`research/tools/ecosystem/secrets/`); outcomes are binary cells, not workload samples |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |
| Timing | none |

## 1. Question

(a) For each non-interactive way automation can supply a password or key (argument vector, environment variable, file, inherited file descriptor, stdin when not a terminal, password command, OS keychain, recipient key file), which exposure vectors does the operating-system mechanism have on Linux and Windows, how do incumbent tools support and guard each, and can the status-quo CLI be driven from CI at all? (b) Do passwords typed in different Unicode normalization forms, with trailing newlines, or through Windows console code pages unlock the same archive? (c) How do the status quo and incumbent tools treat identity and signing-key files with permissive permissions or inherited Windows ACLs, and what do key files created by each tool get by default?

## 2. Hypotheses and falsification

- **H1 (status quo not automatable with passwords).** The status-quo CLI cannot read a password from stdin, a file, a file descriptor or the environment when no terminal is attached (terminal-only prompt). *Falsified* if any non-terminal source unlocks.
- **H2 (argv and same-user environment leak).** On Linux, a password passed in argv is readable by another unprivileged uid through `/proc/<pid>/cmdline` for the process lifetime; an environment variable is readable by the same uid only; an inherited file descriptor and a 0600 file are readable by neither other uid. On Windows, a process's command line is readable by other processes of the same user. *Falsified* per cell by the opposite observation.
- **H3 (normalization mismatch).** A password containing a composed character entered in NFC does not unlock an archive created with the NFD form (no normalization in the status quo). *Falsified* if it unlocks.
- **H4 (permissive key files load silently).** The status-quo CLI loads a group- or world-readable identity or signing-key file on Linux with at most a warning, and applies no ACL check on Windows, while OpenSSH refuses a private key with group or other read permission. *Falsified* if the status quo refuses, or OpenSSH loads it.
- **H5 (Windows key files inherit broad ACLs).** A key file created by `ebound key generate-signing` under the user profile on Windows inherits ACL entries granting access to principals other than the owner (for example SYSTEM and Administrators). *Falsified* if the created file has an owner-only DACL.

## 3. Decisions informed

DEC-ECO-044, DEC-CRY-056, DEC-CRY-108, DEC-PLT-042, DEC-CRY-041, DEC-CRY-095.

## 4. Candidates

| Decision | Candidates and how each is evaluated |
|---|---|
| DEC-ECO-044 | status-quo-terminal-only (live CLI); password-fd, password-file, environment-variable, password-command, recipient-keys-for-automation: evaluated as OS mechanisms with the probe program `secret_source_probe.py` (a minimal reader of each source) plus the incumbents that implement them; os-keychain: Linux Secret Service needs a D-Bus session and keyring daemon in WSL (probed only if available without host changes); Windows Credential Manager would modify the user's credential store — excluded (host account state is not changed by agents) |
| DEC-CRY-056 | status-quo-raw-bytes-tty (live); nfc-normalise-cli, warn-non-ascii (evaluated by computing which inputs would unlock under each rule on the same test passwords); password-fd, password-file, env-variable, stdin-when-not-tty, argv-password (as DEC-ECO-044) |
| DEC-CRY-108 | status-quo-warn-unix-only (live); refuse-unix-default-with-override, refuse-without-override, warn-all-platforms-with-dacl-check, policy-knob-default-warn, no-check-caller-responsibility: evaluated against the CI secret-volume permission matrix (which realistic CI layouts each would refuse) |
| DEC-PLT-042 | status-quo-no-op-on-windows (live); owner-only-dacl-safe-wrapper (feasibility from EXP-ECO-001 API coverage of Windows ACL crates without workspace `unsafe`); warn-on-inherited-access; helper-process-icacls (steps and behaviour of `icacls /inheritance:r /grant:r <user>:F` on a scratch file); document-risk-only |
| DEC-CRY-041, DEC-CRY-095 | status-quo-raw-seed / status-quo-plaintext-ebsk (live); restrictive-permissions and windows-acl-restriction (permission matrices); passphrase-wrapped-identity and passphrase-encrypted-ebsk (usability cost as extra steps and prompts in automation, counted in EXP-ECO-018); os-keychain/os-keystore (excluded as above); hardware-token and PKCS#11/HSM (no hardware; excluded) |

Incumbents (pinned): gpg 2.4 (`--pinentry-mode loopback --passphrase-fd/--passphrase-file`), age (`-p` interactive, plugin and identity files), 7-Zip (`-p` argv), Info-ZIP `zip -P`, OpenSSL `enc -pass pass:|env:|file:|fd:|stdin`, restic (`--password-file`, `RESTIC_PASSWORD`, `--password-command`), borg (`BORG_PASSPHRASE`, `BORG_PASSCOMMAND`, `BORG_PASSPHRASE_FD`), cosign (`COSIGN_PASSWORD`), minisign, OpenSSH `ssh-keygen` (Linux and the Windows built-in OpenSSH client), signify.

## 5. Inputs

Generated only: test passwords (ASCII; Latin-1 range composed characters such as "é" in NFC and NFD; Hangul; emoji; trailing newline variants; 1-byte and 1,024-byte lengths) from `passwords.yaml` (committed; synthetic); scratch identities and signing keys generated per sample and deleted; a small plain tree (100 files, seed 2026091717) for archives.

## 6. Environment and platform requirements

- WSL2 Ubuntu 24.04: pty driving with Python `pty`; second uid via `setpriv --reuid=65534`; `/proc` default mount options recorded (`hidepid`).
- Windows host (non-admin): ConPTY driving via `pywinpty` (pinned, installed into a venv under `D:/eb-research/venv-win`); console code pages 437, 850, 1252, 65001 set per child console only (never system-wide); `icacls` read and scratch-file modification only; `Get-CimInstance Win32_Process` for command-line visibility.
- CI secret-volume emulation: files created with modes and ownerships typical of Kubernetes secret volumes (0644 and 0444 root-owned, 0440 with fsGroup), Docker secrets (0444), GitHub-style temporary files (0600) — as files on ext4; no container runtime needed.

## 7. Procedure and commands

Scripts in `research/tools/ecosystem/secrets/` (to be written):

1. `sources_matrix.py` — for each source × OS: start the probe (and each incumbent) receiving a known marker secret; from a second process (other uid on Linux; same user on Windows) attempt to read the secret via `/proc/<pid>/cmdline`, `/proc/<pid>/environ`, `/proc/<pid>/fd/*`, shell history (`bash -i` with `HISTFILE` in scratch), `ps -ef`, `Win32_Process.CommandLine`; record binary exposure per vector; record whether each tool supports the source and whether it warns about unsafe forms.
2. `automation_check.py` — status-quo `ebound pack --password` and `unpack --password` with stdin not a terminal (pipe, `/dev/null`), with `script -qc` pty, with `setsid` no controlling terminal; outcome and message.
3. `unicode_unlock.py` — create archives through a pty with each password form; attempt unlock with every other form; on Windows through ConPTY with each code page; record unlock success matrix; the same matrix for gpg, 7-Zip and OpenSSL where they accept typed passwords.
4. `keyfile_matrix.py` — for modes 0600, 0640, 0644, 0444, 0666 and owner other-than-current-user (root-owned 0644 read by uid 65534 where readable): load identity (`ebound unpack --identity`) and signing key (`ebound sign --signing-key`), and the incumbents' equivalent loads; record refuse/warn/silent.
5. `windows_acl.ps1` — `ebound key generate-signing` into `%USERPROFILE%\eb-research-scratch\` and `D:\eb-research\scratch\`; `icacls` output parsed into principals and rights; the same for `ssh-keygen.exe -t ed25519 -f`, age-keygen.exe and minisign.exe; then load each key after adding an extra `Users:(R)` ACE on a scratch copy, and record behaviour.
6. Summary tables to `research/normalized/EXP-ECO-017/`.

## 8. Seeds, warmup, replication

Seed 2026091717 for trees and ordering. Every cell runs twice in different processes; results must agree (else reported nondeterministic). No warmups.

## 9. Metrics and measurement

| Metric | Canonical | Role |
|---|---|---|
| `unsafe_defaults` | yes (HC-05) | binary: silent load of a permissive key file and argv-only secret support counted as unsafe defaults where the tool offers no safer default |
| `flags_per_task` | yes (C6) | cost input for the CI encryption task per source (feeds EXP-ECO-018) |
| `error_actionability_fraction` | yes (T-20) | banded over refusals (rubric EXP-ECO-015 §9) |
| exposure vector cells, tool support cells, unlock matrix cells, key-file behaviour cells, Windows ACE lists | no | record fields |

## 10. Normalization and aggregation

AGG-S per OS and per source; headline the worst exposure per source; AGG-W for unsafe defaults per tool.

## 11. Statistical analysis and use in the decision rule

Deterministic binary facts. A candidate source whose OS mechanism exposes the secret to other uids by default (argv on Linux) cannot be the recommended automation path without a documented mitigation; this is a security design input, and the final security judgement for passwords and key protection is `EXTERNAL_REVIEW_REQUIRED` under §8.3 where it concerns cryptographic key protection. The unlock matrix screens DEC-CRY-056 candidates by which inputs each rule unlocks (a rule that makes visually identical passwords fail is recorded as counterevidence; a rule that changes accepted passwords of existing archives violates HC-16 for frozen KDF input definitions and needs a new identifier).

## 12. Practical-significance thresholds

Binary per §3.3; T-20 for actionability; `flags_per_task` has no band.

## 13. Sensitivity analysis

`/proc` mounted with `hidepid=2` inside a private mount namespace (`unshare -m`) to show the mitigation's effect; WSL `setpriv` other uid versus same uid; Windows standard user versus the same user in a second session.

## 14. Held-out (Phase D placeholder)

Not applicable to OS mechanism facts (no workload items); the live CLI cells are re-run once on the frozen build at Commit A.

## 15. Expected negative results worth recording

The status quo cannot be used for password-based encryption in CI; incumbents commonly accept argv passwords; key files created on Windows inherit broad ACLs for every tool including incumbents; code page 850 input changes password bytes.

## 16. Threats to validity

WSL2 is not a typical CI runner; ConPTY emulation differs from a real console host; only one Windows host configuration; exposure through core dumps, swap and crash reports is not tested.

## 17. Cost estimate

About 3 h compute; about 1 GB disk; agent effort scripted plus judgment for `passwords.yaml` and the incumbent command tables (one session).

## 18. Gates and exposure

Conventions §1 and §2. No corpus family is used.
