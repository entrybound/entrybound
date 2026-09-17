# EXP-ECO-006: Release, distribution and legacy-migration workflow scenario matrix

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-006 |
| Domain | ecosystem (program §32; staged adoption path SPEC §21.1, objective OD-26 scenarios S1-S5) |
| Status | READY (all runnable variants use the implemented CLI and installable incumbents; scripts specified below) |
| Runner | `spec.yaml` (directory-input workflows) and `spec-legacy.yaml` (legacy-artifact workflows); W14 is a derived table |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |
| Timing | none (CI timing is EXP-ECO-008) |

## 1. Question

For each pre-registered adoption workflow a v1 user would run end to end (legacy audit, build-pipeline dual publishing, consumption by tools without Entrybound, Entrybound-aware partial retrieval, legacy fallback, CI hand-off, source releases, sidecars for existing artifacts, release comparison, incremental update, pipe conversion, lossy-export approval, publishing to filesystems without hard links), does the implemented CLI complete it under a mechanically checked criterion, with how many commands and flags compared with the capability-matched incumbent pipeline, which injected failures does it handle with an actionable diagnostic and a correct exit status and residual state, and which capabilities does each workflow need that the status quo lacks?

## 2. Hypotheses and falsification

- **H1 (completion).** Every runnable status-quo variant completes W02, W03, W04, W06, W08 and W12 on every applicable tuning item; W09 (archive-versus-directory), W10 (add an entry) and W11 (tar from stdin) fail by design and are recorded as capability gaps. *Falsified* per workflow by a completion failure on an item (W02, W03, W04, W06, W08, W12) or by an unexpected success (W09-W11).
- **H2 (friction).** `ebound publish` needs fewer commands and flags than the reproducible GNU tar plus Info-ZIP incantation for the same deterministic dual-publish outcome (W02). *Falsified* if `flags_per_task` for the publish variant is greater than or equal to the incumbent's.
- **H3 (consumer acceptance).** Every legacy artifact produced by W02 is read to an EAM-equivalent tree by every pre-registered incumbent reader, except where the migration report declares a loss for that reader class. *Falsified* by any undeclared non-acceptance (`export_acceptance_fraction` < 1 on undeclared cases).
- **H4 (failure handling).** Every injected failure (§7 fault list) ends with a non-zero exit status in the outcome-class mapping, a message naming the failed requirement and a remedy, and no destination object that could be mistaken for a complete output (HC-18). *Falsified* by any silent success, misleading exit status, or residual partial output under a final name.
- **H5 (no hard links).** On FAT-family and 9P-mounted output directories, `publish` either completes with its declared transaction guarantees or refuses before writing with a specific reason. *Falsified* by a partial publish or a generic I/O error class.

## 3. Decisions informed

DEC-ECO-002, DEC-ECO-004 (integration friction part), DEC-ECO-019, DEC-ECO-035, DEC-ECO-043 (mitigation friction part), DEC-ECO-057, DEC-ECO-062, DEC-ECO-065, DEC-ECO-075 (comparison with incumbent tools per value claim), DEC-CON-022, DEC-ACC-008, DEC-INT-036, DEC-INT-037, DEC-LEG-003, DEC-LEG-010, DEC-LEG-017, DEC-LEG-018, DEC-LEG-029 (reader interoperability of existing targets), DEC-LEG-030, DEC-LEG-036, DEC-LEG-038, DEC-LEG-056, DEC-LEG-060, DEC-MOD-043, DEC-PLT-037.

## 4. Candidates and workflow catalogue

The reference for every workflow is the status-quo `ebound` at dev HEAD, reported beside REF-PROD `9e44608`. Ledger candidates that are not implemented and cannot be expressed with existing commands are not run here; their command and flag counts come from their synopses in EXP-ECO-018, and their user-facing interpretation from EXP-ECO-019.

| WF | Workflow (stage) | Variants run (candidate ids where the ledger names them) | Completion criterion (checked by `research/tools/ecosystem/workflows/check_<wf>.py`) |
|---|---|---|---|
| W01 | Legacy audit without writing `.eb` (S1) | `ebound convert --strict --dry-run` (DEC-ECO-035 status-quo-dry-run); `ebound convert --compat=<profile> --dry-run`; convert to scratch then `inspect --provenance --json`; incumbents `zipdetails`, `python3 -m zipfile -t`, `bsdtar -tvf`, GNU `tar -tvf`, `7z t`, warehouse ZIP validation function vendored at a pinned commit | per-artifact verdict (accept/refuse) and, for refusals, a named rule or reason code, emitted in a machine-readable form; exit status non-zero on refusal |
| W02 | Build pipeline dual publish (S2) | `ebound publish --native --target zip --target tar.zst` (DEC-ECO-019 status-quo-publish; DEC-LEG-010 status-quo-freeze-publish-v1); pack then one `convert --to` per target (DEC-ECO-019 per-target-convert); `publish` without `--native` (legacy-only-publish); `publish` then `sidecar` (publish-plus-sidecars); incumbent reproducible pipeline: GNU tar with `--sort=name --mtime=@$SOURCE_DATE_EPOCH --owner=0 --group=0 --numeric-owner --pax-option=exthdr.name=%d/PaxHeaders/%f,delete=atime,delete=ctime` piped to `zstd -19`, and `zip -X -r` after `touch -d @$SOURCE_DATE_EPOCH` plus `strip-nondeterminism` | all targets exist; a second run in a perturbed environment (different TZ, locale, umask, mtimes touched, directory creation order) produces byte-identical artifacts; W03 accepts every legacy artifact; migration report (if any) parses and matches outputs |
| W03 | Consumer without Entrybound (S3) | readers: Info-ZIP `unzip` 6.0, bsdtar 3.7.2 (WSL) and `tar.exe` 3.8.8 (Windows), GNU tar 1.35, 7-Zip 23.01, Python `zipfile`/`tarfile` (filter `data`), Java 21 `java.util.zip`/`jar`, PowerShell `Expand-Archive`, Go `archive/zip`/`archive/tar` (pinned Go toolchain) | extracted tree's logical fingerprint equals the source tree modulo losses declared in the migration report or receipt |
| W04 | Entrybound-aware consumer, partial retrieval (S4) | `ebound list <URL>`, `ebound read <URL> <path> --access-report` through local nginx (range-capable) | read bytes equal the source entry; reported verification state is verified; no full download |
| W05 | Optional legacy fallback (S5) | consumer script: try `release.eb`; on exit status 2 (`UNSUPPORTED`) or missing `ebound`, take `release.tar.zst` by the canonical names of `docs/migration-workflows-v1.md` | the fallback triggers exactly on `UNSUPPORTED`/tool absence and never on `CORRUPT`/`TRUNCATED` (which must stop) |
| W06 | CI artifact hand-off | `ebound pack --layout stream <dir> -` to a file (upload emulation) then `ebound unpack -` ; partial restore of one subdirectory from INDEXED by `ebound read` per entry; incumbent `tar -cf - | zstd -T0 --long=30` / `zstd -d --long=30 | tar -xf -` | restored tree fingerprint equals source (full) or subdirectory (partial) |
| W07 | Source release (DEC-ECO-065) | incumbent `git archive --format=tar --prefix=<p>/ <tag> | gzip -n` (DEC-ECO-065 git-archive-integration as incumbent); `git archive | xz -9`; `git archive` extracted then `ebound publish --target tar.gz` (legacy-only-deterministic / status-quo-publish-native-plus-legacy); `ebound sidecar` for the incumbent tarball (sidecar-for-existing) | regenerating from the same tag on the second environment (Windows host with `core.autocrlf=false`, or the EXP-ECO-004 chroot) gives byte-identical artifacts |
| W08 | Sidecar for an existing legacy artifact (DEC-ECO-062, DEC-INT-036, DEC-LEG-056) | `ebound sidecar <artifact>` with default name; `--preserve --compat=<profile>`; consumer check = `ebound verify <artifact>.eb` then source-digest comparison from `inspect --provenance --json` | consumer detects every modified, truncated, appended or substituted artifact; an unmodified renamed artifact is associated only by the explicit path the consumer supplies |
| W09 | Release comparison (DEC-ACC-008) | `ebound diff a.eb b.eb --json`; attempts: archive versus directory and archive versus legacy artifact (expected refusal); incumbents `diff -r` on extracted trees, `diffoscope --text` (small items only) | reported added/removed/changed path sets equal ground truth computed from the trees |
| W10 | Incremental update (DEC-MOD-043) | status quo: re-pack the modified directory (no add command); incumbents `tar --append`, `zip -u`, `7z u` | updated artifact contains the new file and every old file byte-identical; identity change reported or not (recorded) |
| W11 | Pipe-based conversion (DEC-LEG-030) | `curl -s file://<artifact> | ebound convert - out.eb --from tar.gz` (status-quo-file-only expected refusal); spool-to-temp recipe; incumbents `curl | tar -xz`, `curl | bsdtar -xf -` | `.eb` produced equivalent to file-based conversion, or refusal with a specific non-seekable diagnostic |
| W12 | Lossy export approval share (DEC-LEG-036) | for each tree: `pack` (default capture) then `convert --to <each of six profiles> --dry-run`; for tar-derived items: tar → `.eb` → tar round trip | outcome class LOSSLESS / LOSSY / REFUSED recorded per (item, profile); no completion criterion (descriptive distribution) |
| W13 | Publish without hard links (DEC-PLT-037, DEC-ECO-019) | `ebound publish` with the output directory on: ext4 (reference), vfat loop mount, exFAT loop mount (if the WSL kernel provides exFAT), tmpfs, drvfs `/mnt/d` (9P), NTFS (Windows `ebound.exe`), CIFS loopback (Samba `smbd` bound to 127.0.0.1 inside WSL, if the kernel provides CIFS) | success with all artifacts verified, or refusal before any final-name output with a specific reason |
| W14 | Workflow-need trace (DEC-INT-037, DEC-LEG-060, DEC-LEG-003, DEC-CON-022) | derived from W01-W13 scripts: every step that the status quo cannot perform carries a `needs:` capability tag (Sidecar role, Incremental role, tar/7z preservation, provenance fields, archive-versus-directory diff, stdin import); for W02, W03 and W08 the retrofit-only variant (legacy artifacts plus sidecar `.eb`) and the native-primary variant are scored on the pre-registered consumer-property checklist below | table of capability × workflows needing it; checklist scores per variant |

Consumer-property checklist for W14 (binary per variant, pre-registered): C1 exact bytes verifiable by the consumer; C2 single-entry retrieval without full download; C3 metadata loss declared to the consumer; C4 a signature that survives recompression of the payload; C5 derivation from the source verifiable; C6 consumable with preinstalled tools.

### Exclusions

| Excluded | Reason |
|---|---|
| goreleaser, cargo-dist and release-tool plugins (DEC-ECO-019 release-tool-plugin) as run variants | no Entrybound plugin exists; their archive steps are counted from their pinned documentation as incumbent configurations in EXP-ECO-018 |
| First-party CI actions, Bazel toolchain, Nix builder (DEC-ECO-004) | not implemented and need hosted CI or maintainer interest; friction counted as synopses in EXP-ECO-018; demand is EXP-ECO-022 |
| Uploading to registries or hosting services | publishing requires owner approval and accounts; W04 uses local servers (EXP-ECO-009 covers hosting behaviour, EXP-ECO-010 the authenticated arms) |
| Windows Explorer, macOS Finder and GUI archive managers as W03 readers | no GUI automation in scope; macOS PLATFORM_BLOCKED |

## 5. Corpus

- Directory-input workflows (`spec.yaml`, tuning): `f01-tuning-ripgrep-14-1-1-git`, `f01-tuning-zstd-v1-5-7-full-clone` (W07 source), `f02-tuning-lz4-intree-make-build`, `f02-tuning-zstd-cmake-build`, `f04-tuning-sourcelike`, `f10-gen-redundant-binary-blobs`, `f18-tuning-lz4-releases-1-9-to-1-10` (W09 pairs), `f18-tuning-zstd-releases-1-5-x` (W09 pairs), `f19-tuning-zoo` (hostile names), `f19-tuning-real-home-snapshot` (symlinks, hardlinks, metadata).
- Legacy-artifact workflows (`spec-legacy.yaml`, tuning): `f14-apache-maven-3916-bin-tarball`, `f14-gen-nested-archives-py`, `f14-jenkins-25683-war`, `f14-legacy-writer-matrix-tuning`, `f20-tuning-real-libarchive-testsuite`, `f20-tuning-generated-malicious-structures`.
- Validation look (one): directory items `f01-validation-redis-7-2-16-git`, `f01-validation-cpython-3-13-15-full-clone` (W07), `f02-validation-cjson-cmake-build`, `f04-validation-objstore`, `f18-validation-cjson-releases` (W09), `f19-validation-real-home-snapshot`, `f19-validation-deep`; legacy items `f14-gen-office-documents`, `f14-legacy-writer-matrix-validation`, `f14-pyspark-420-sdist`, `f20-validation-real-commons-compress-testdata`, `f20-validation-real-cpython-archivetestdata`.
- Minimum support: workflows are evaluation conditions (AGG-S), not family aggregates; the §4.9 corpus-level minimum applies only to `export_acceptance_fraction` and `export_outcome_distribution`, which use every tree item of the split (≥ 5 families required; the tuning list has F01, F02, F04, F10, F18, F19).

## 6. Environment and platform requirements

- WSL2 Ubuntu 24.04 root (`wsl-ubuntu`) for W01-W13; loop mounts (vfat, exFAT, ext4 64 MiB for disk-full) inside the research VM as already used for corpus F19 images; Samba and CIFS only if available without host changes.
- Windows host (`windows-host`) for W03 readers (`tar.exe`, `Expand-Archive`, Java 21), W07 second environment, W13 NTFS arm.
- Incumbent tools pinned by version (apt, research venv, pinned Go toolchain and warehouse commit); `tool-versions.json` generated by a probe script.
- No network except local servers; no privileged host operations.

## 7. Procedure and commands

Scripts in `research/tools/ecosystem/workflows/` (to be written): `run_workflow.py --workflow <W> --variant <V> --item <path> --scratch <dir>` executes the variant's committed step file `steps/<W>/<V>.sh` (the step file **is** the counted command sequence), runs the workflow's fault cases, calls `check_<W>.py`, and prints one JSON line. Step files are written and committed before any run; changing one after results are visible creates a new variant id (CB-3).

Fault cases (pre-registered; each executed as a separate sub-run inside the sample):

| Id | Applies to | Injection | Expected safe outcome |
|---|---|---|---|
| FX-COLLIDE | W02, W08, W13 | a file already exists at a final output name | refusal before writing, name collision reported, existing file unchanged |
| FX-DISKFULL | W02, W06, W08 | output directory on a 64 MiB ext4 loop mount filled to 1 MiB free | failure with an I/O-class reason; no final-name output; temp files cleaned or reported |
| FX-READONLY | W02, W13 | output directory mode 0555 (as the unprivileged user via `setpriv --reuid=65534`) | refusal with a permission reason before any output |
| FX-LOSSY | W02, W12 | a target that is LOSSY for the item without `--allow-lossy` | refusal naming the lossy classes and the approval flag |
| FX-KILL | W02, W06, W08 | SIGKILL at three seeded points (after first output byte, mid-target, before final rename), seeds 2026091706-1..3 | no final-name output that verifies as complete (HC-18); leftovers recorded |
| FX-TRUNC | W01, W04, W08 | artifact truncated by 1 byte and by half | `TRUNCATED` class, exit status 3 |
| FX-FLIP | W01, W04, W08 | one seeded byte flipped | `CORRUPT` (or refusal naming the digest) |
| FX-STDIN | W01, W06, W11 | input supplied on stdin where seeking is required | specific non-seekable diagnostic |
| FX-ETAG | W04 | origin ETag changes between requests (`ebr-origin` churn mode) | refusal; never mixed-revision bytes reported as verified (`origin_protocol_safety`) |
| FX-NORANGE | W04 | server ignores Range (returns 200 whole body) | refusal or bounded full-download fallback, reported as such |
| FX-MODIFY | W08 | artifact modified, renamed, appended, or modified concurrently during sidecar creation (writer appends 4 KiB every 50 ms) | modification detected; concurrent modification refused or declared `unstable_source` |
| FX-UNKNOWN | W05 | `.eb` requiring a feature unknown to the reader build (conformance-corpus unknown-incompat case) | exit status 2 triggers fallback; a corrupt archive does not |

Run order: tool probe → `ebr run --spec spec.yaml` and `spec-legacy.yaml` (tuning) → verify-raw → normalize → `derive_w14.py` → validation look with `spec-validation.yaml` and `spec-legacy-validation.yaml` generated by `make_specs.py` from §5.

## 8. Seeds, warmup, replication

Seeds `order` 2026091706, `bootstrap` 2026091706, `command` 2026091706 (fault offsets derive from the command seed). Deterministic outcomes: 2 repetitions per (variant, item) in different processes plus repeat-hash of produced artifacts (§4.13); outcome classes and exit statuses must agree across repetitions, otherwise the cell is reported as nondeterministic (an R1 artifact when the variant claims determinism). No warmups.

## 9. Metrics and measurement

| Metric | Canonical | Role |
|---|---|---|
| `migration_steps_count` | yes (M26.2) | descriptive: commands in the step file |
| `flags_per_task` | yes (C6) | cost input: option tokens in the step file (counting rules of EXP-ECO-018 §9) |
| `export_acceptance_fraction` | yes (T-20, M19.4) | banded; per reader over the pre-registered case list (items × targets) |
| `export_outcome_distribution` | yes (M19.4) | descriptive (W12) |
| `zero_install_scenario_fraction` | yes (M26.1) | descriptive: S1-S5 scenarios served with preinstalled tools |
| `single_file_self_contained` | yes (HC-09, M26.5) | binary: W04/W06 archives decode offline with no sidecar |
| `migration_determinism_fraction` | yes (HC-13, M20.2) | binary: W02/W07 rebuild identity |
| `receipt_completeness_fraction` | yes (HC-07, M20.4) | binary: every changed or dropped item in W02/W12 recorded in the receipt or report |
| `error_actionability_fraction` | yes (T-20, M25.3) | banded over the fault cases (rubric of EXP-ECO-015 §9) |
| `origin_protocol_safety` | yes (HC-18, M12.4) | binary (W04) |
| `artifact_bytes` | yes (T-01) | descriptive: sidecar size and dual-publish total bytes |
| `workflow_completed`, `fault_expected_outcome`, `exit_status`, `residual_state_class`, `needs_capability` | no | record fields |

## 10. Normalization and aggregation

AGG-S per workflow (evaluation condition = user task, objective §3.0), headline the worst workflow; fault cases reported per (workflow, fault). `export_acceptance_fraction` AGG-C per reader with minimum over families headline. W12 distribution per profile and family.

## 11. Statistical analysis and use in the decision rule

- Deterministic counts and binary outcomes; no confidence intervals. T-20 fractions compare candidates exactly against the `0.5/n_cases` floor.
- Binary failures (HC-13, HC-18, HC-09, HC-07) are R1 evidence against the variant's candidate for its claimed property.
- Scope: workflows are evaluation conditions; a variant preferred in one workflow and worse in another is scoped at R9, never averaged (AGG-S).
- `flags_per_task` and `migration_steps_count` feed C6 and R10 keys; they never select alone (§2.0).
- Validation: one look for the fault-case outcomes and acceptance fractions on validation items; a reversal is counterevidence (CB-6).

## 12. Practical-significance thresholds

T-20 (`export_acceptance_fraction`, `error_actionability_fraction`); binary metrics per §3.3; cost inputs and descriptive metrics have none (Appendix A.2).

## 13. Sensitivity analysis

- W03 with and without Windows readers (host effects).
- W02/W07 rebuild identity with each environment perturbation removed in turn.
- W12 distribution with F19 metadata-heavy items excluded.
- Fault outcomes with the unprivileged user versus root.

## 14. Held-out (Phase D placeholder)

Conventions §5 and §8: a held-out set of workflow variants of the same stages (different artifacts, names and fault offsets) is authored by a session not involved in CLI design and sealed before candidate iteration; held-out corpus items run once after unlock.

## 15. Expected negative results worth recording

- `publish` refusing trees with symlinks for legacy targets (baseline probe finding) making W02 fail on F19 and source trees with symlinks.
- F-0001: archives packed on Windows requiring the platform-security feature bit, which could make W05 fall back unnecessarily for older readers.
- Sidecars roughly the size of the artifact (content-bearing design).
- No stdin import; no archive-versus-directory diff; no incremental add.
- Hard-link transactions refusing on vfat and 9P.

## 16. Threats to validity

- Step files written by the experiment designer may not reflect what real users type; EXP-ECO-013 (public script census) and EXP-ECO-019 (simulated sessions) are the checks.
- Loop-mounted vfat/exFAT inside WSL is not a USB stick or a Windows share; CIFS loopback is not a real SMB server.
- The warehouse validation function is vendored out of its service context.
- Workflow catalogue completeness against the program's Task 32 list cannot be checked from the repository (the program text is not committed); the design reviewer reconciles it.

## 17. Cost estimate

- Compute: about 20 h (≈ 16 tree and 6 legacy items per split, ≈ 40 variants with fault sub-runs, 2 repetitions; Windows readers and W13 NTFS about 3 h).
- Disk: about 100 GB transient scratch (published artifacts of medium items), cleaned per sample after hashing.
- Agent effort: scripted after step files exist; judgment to author the step files (one session) and to review them against incumbent documentation (a second session).

## 18. Gates and exposure

Conventions §1 and §2. Applicable families include F04 and F20 from the exposure record; analysis by an unexposed session.
