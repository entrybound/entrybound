# EXP-LEGACY-006: baseline probe refusal findings — scope and cause of the symlink export refusal, non-UTF-8 name refusal and POSIX ACL/mode refusal

| Field | Value |
|---|---|
| Status | **READY** (instruments exist: `ebound` builds from `dev` with the pinned toolchain, `setfacl`/`getfacl`/`getfattr`, GNU tar 1.35, bsdtar 3.7.2, Info-ZIP `zip`/`unzip`, 7-Zip 23.01, `research/baselines/probes/make_fixtures.py`; the experiment-local driver `run_findings.py` and case builder `findings_cases.py` are fully specified in §7) |
| Domain / program sections | legacy / §27 (export fidelity), §25 (import asymmetry); origin: `research/baselines/README.md` "Findings worth carrying into Phase C" and PROGRESS open integration item |
| decision_ids | DEC-LEG-015, DEC-LEG-029, DEC-LEG-032, DEC-LEG-036, DEC-LEG-067, DEC-LEG-068, DEC-LEG-094, DEC-MOD-019, DEC-PLT-021; cross-domain (owners: model and platform domains): DEC-MOD-038, DEC-PLT-052 |
| Decision types (proposed) | EMPIRICAL (refusal scope and rates) with FORMAL parts (POSIX.1e ACL mask semantics per `acl(5)` of acl 2.3.2 at the pinned package; UTF-8 validity per RFC 3629 §3-§4) |
| OD / HC (proposed) | OD-19 (M19.4 export outcomes), OD-03 (M03.1 capture), OD-17 (M17.3 benign false refusal); HC-07, HC-08, HC-17 |
| Ledger note | The three findings have no dedicated ledger rows yet (PROGRESS "Baseline probe findings to add to the ledger at the next assembly"); results attach to the rows above and to the addendum rows once assembled |
| Shared rules / L7 / gates | C§ header, C§2 |
| Timing | `timing: false` |

## 1. Question

For the three refusals the Phase B1 capability probe observed at `dev` HEAD — (a) legacy export refuses the whole
conversion when any symlink Entry exists, (b) `ebound pack` refuses a tree containing a non-UTF-8 file name, (c)
`ebound pack` refuses a tree whose POSIX ACL and mode "disagree" — what exactly triggers each refusal, is the refusal
per entry or whole input, does it hold for every export profile, import path and filesystem, and is (c) a refusal of an
internally consistent kernel state (a false refusal) or of a genuinely inconsistent capture?

## 2. Hypotheses

- **H1 (symlink export scope).** For every symlink variant (relative in-tree, relative escaping, absolute, dangling,
  to directory, chain) and all six frozen export profiles, `convert --to <target>` refuses the whole export with
  `EB_LEGACY_EXPORT_REFUSED` / `ENTRY_KIND_UNSUPPORTED`, with and without `--allow-lossy` and under `--dry-run`, while
  `convert --strict` of a GNU-tar-written tar with the same symlinks imports them as Symlink entries (import/export
  asymmetry). *Falsified by* any profile emitting the tree, a per-entry refusal, or tar import refusing the symlinks.
- **H2 (non-UTF-8 scope).** Every invalid-UTF-8 byte class (lone Latin-1 byte, bad continuation, overlong, UTF-8-encoded
  surrogate, 0xFF) in any position (file name, directory name, symlink target) makes `pack` refuse the whole tree
  (`EB_EAM_INVALID_PATH_COMPONENT`); strict tar import of the same bytes refuses; ZIP import of the same bytes as an
  unflagged name succeeds through CP437 decoding. *Falsified by* any accepted pack, any refused ZIP unflagged import,
  or any accepted strict tar import.
- **H3 (ACL/mode, cause).** At least one refused ACL variant satisfies the POSIX.1e invariant independently observed on
  the source (`stat` group-class bits equal the `ACL_MASK` entry when a mask exists, otherwise the `ACL_GROUP_OBJ`
  entry; owner and other bits equal `ACL_USER_OBJ` and `ACL_OTHER`), i.e. the refusal is a capture defect rather than a
  genuine inconsistency. *Falsified by* every refused variant violating the invariant in the independent observation
  (then the refusal is correct and the fixture was inconsistent at capture time).
- **H0.** The refusals are narrow artifacts of the original fixture only.

## 3. Decisions informed

| Decision | Supplied here |
|---|---|
| DEC-LEG-015 | Whether export refusals for paths and entry kinds are typed consistently across ZIP and tar profiles (issue class, scope, reason) |
| DEC-LEG-029, DEC-LEG-068 | The size of the symlink gap every candidate profile must close; count of real tuning/validation items (§5) that cannot be exported at all today |
| DEC-LEG-032 | Import/export asymmetry: tar import projects symlinks, ZIP/7z imports refuse them, every export refuses them |
| DEC-LEG-036 | Share of freshly packed tuning/validation trees whose export is LOSSY or REFUSED only because of construction-equivalent metadata versus symlinks |
| DEC-LEG-067, DEC-LEG-094, DEC-MOD-019, DEC-MOD-038 | Exact byte classes refused on pack and tar import versus accepted through ZIP CP437 decoding: the same name bytes receive different treatment by entry path |
| DEC-PLT-021, DEC-PLT-052 | ACL variants accepted/refused, including minimal (trivial) access ACLs and SCHILY.acl-bearing tar import |

## 4. Candidates

This experiment characterises the **status quo** (the research baseline `9e44608` and the current `dev` build) and
compares it with incumbent behaviour; it does not select. Candidate ids of the informed decisions are unchanged
(ledger). Builds measured: `ebound-baseline` (C§3), `ebound-head`, and `ebound-probe` = the existing
`/root/eb-research/target/baseline/release/ebound` built by Phase B1 from clone HEAD `9efbc74` (the binary that produced
the finding). Incumbent context columns: GNU tar 1.35 `-c`, bsdtar 3.7.2, Info-ZIP `zip -y` and `zip` (dereferencing
default), 7-Zip 23.01 with and without `-snl`.

## 5. Corpus selectors

Generated case sets (`research/experiments/EXP-LEGACY-006/findings_cases.py`; F19; `S_T = 2509170601`,
`S_V = 2509170602` vary file contents, names and depths only):

| Set | Content |
|---|---|
| A-symlink | Trees of 1 and 1,000 regular files plus one symlink of each variant (relative in-tree, relative escaping, absolute, dangling, to directory, 3-link chain); a tree with only a hardlink pair (control); GNU tar and bsdtar tars and Info-ZIP `zip -y` ZIPs of the same trees |
| B-nonutf8 | Trees with one invalid name of each class (0xE9, 0xC3 0x28, 0xC0 0xAF, 0xED 0xA0 0x80, 0xFF) as a file name, a directory name (with children) and a symlink target; a valid non-NFC UTF-8 control; tars (GNU tar raw bytes, and pax `hdrcharset=BINARY` via bsdtar) and unflagged ZIPs (hand-built) carrying the same bytes |
| C-acl | On ext4, and on XFS and btrfs loop-mounted images (created fresh with `mkfs` under `/root/eb-research/scratch/legacy-006/`): files with (1) `setfacl -m u:1234:rwx` on 0644, (2) `setfacl -n -m u:1234:rwx`, (3) `setfacl -m m::r` after a named entry, (4) `chmod g-w` after (1), (5) default ACL on a directory with a file created inside, (6) minimal ACL equal to the mode (`setfacl -m u::rw,g::r,o::r`), (7) named-group entry with mask narrower than group_obj; the original `make_fixtures.py` ACL fixture; bsdtar and GNU tar `--acls` tars of each |

Real items (export-gap census for DEC-LEG-029/036; tuning/validation only): `f01-tuning-ripgrep-14-1-1-git`,
`f01-tuning-zstd-v1-5-7-git`, `f04-tuning-sourcelike`, `f19-tuning-real-home-snapshot`, `f19-tuning-rsnapshot-a`,
`f03-tuning-fzf-go-mod-vendor` — tuning; `f01-validation-redis-7-2-16-git`, `f19-validation-real-home-snapshot`,
`f19-validation-rsnapshot-b`, `f03-validation-yq-go-mod-vendor`, `f18-validation-cjson-releases` — validation.
Held-out: placeholder (C§5).

## 6. Environment

`wsl-ubuntu`, root, ext4 under `/root/eb-research`; XFS and btrfs via loop devices (research kernel supports them,
PROGRESS environment). Not `/mnt/*` (no metadata). No timing.

## 7. Commands

Experiment-local scripts (to be typed from this specification, no new instrument):

- `findings_cases.py --set A-symlink|B-nonutf8|C-acl --split tuning|validation --seed <S> --out <dir>` builds §5 trees;
  for each C-acl file it records the independent observation `{stat_mode, getfacl_n, getfattr_hex(system.posix_acl_access), invariant_holds}`.
- `run_findings.py --build <ebound path> --case-set <dir> --scratch <dir>` runs, per tree: `ebound pack <tree> <out.eb> --profile balanced`; if packed, `ebound convert <out.eb> <t> --to T --dry-run` and `--allow-lossy --receipt r.json` for T in `zip, tar, tar.gz, tar.zst, tar.xz, tar.bz2`; for tars/ZIPs in the set, `ebound convert <archive> <out.eb> --strict` and `ebound inspect <out.eb> --json --entries`; records exit code, outcome class, reason code, issue classes and scopes from stderr/receipt, entry kinds and path bytes; prints one JSON summary line.

```sh
cd /mnt/d/Projects/entrybound/entrybound
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/legacy-baseline
git -C /root/eb-research/src/legacy-baseline checkout 9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155
(cd /root/eb-research/src/legacy-baseline && CARGO_TARGET_DIR=/root/eb-research/target/legacy-baseline /root/.cargo/bin/cargo +1.98.1 build --release --bin ebound -p entrybound-cli)
git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/legacy-head   # HEAD = the dev commit named in record.md
(cd /root/eb-research/src/legacy-head && CARGO_TARGET_DIR=/root/eb-research/target/legacy-head /root/.cargo/bin/cargo +1.98.1 build --release --bin ebound -p entrybound-cli)
/root/eb-research/venv/bin/python research/experiments/EXP-LEGACY-006/findings_cases.py --all --split tuning --seed 2509170601 --out /root/eb-research/cases/EXP-LEGACY-006/tuning
/root/eb-research/venv/bin/python research/experiments/EXP-LEGACY-006/findings_cases.py --all --split validation --seed 2509170602 --out /root/eb-research/cases/EXP-LEGACY-006/validation --manifest research/experiments/EXP-LEGACY-006/cases-manifest.jsonl --append-corpus-items research/experiments/EXP-LEGACY-006/real-items.txt
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-LEGACY-006/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-006/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-006/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-006/spec.yaml
```

The builds are the C§3 `ebound-baseline` and `ebound-head` definitions; the clone commands above write only under
`/root/eb-research` (never to the repository).

## 8. Seeds

`order 2509170611`, `bootstrap 2509170612`, `command 2509170613`; case seeds §5.

## 9. Warmup, replication

No warmup; 2 repetitions (§4.6 deterministic) plus repeat-hash of outcome tuples and exported bytes.

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `benign_false_refusal_fraction` (pack refusals of C-acl trees whose independent observation satisfies the invariant) | T-20 | OD-17 | T-20 |
| `capture_fidelity_fraction` (ACL and name classes captured when packed) | T-20 | OD-03 | T-20 |
| `export_outcome_distribution` (LOSSLESS/LOSSY/REFUSED per profile over §5 real items and A-symlink trees) | descriptive | OD-19 | none |
| `reason_code_specificity_fraction` (refusals naming the triggering entry and class) | T-20 | OD-04 | T-20 |
| `import_acceptance_fraction` (tar and ZIP imports of the same byte classes) | T-20 | OD-19 | T-20 |
| refusal granularity (whole input vs per entry), real items exportable only after symlink removal | non-canonical descriptive | — | none |

## 11-12. Normalization and analysis

Outcome tuples are exact; the ACL invariant is computed from `getfattr` hex and `stat` by `findings_cases.py`, never by
Entrybound. H3 is decided per variant; a refused variant satisfying the invariant is a committed counterexample
(input generator, command, build identity, output; objective §2.0 rule 2) and a production defect record, not a method
relaxation (objective §2.0 rule 5). Real-item census counts are exact per item and aggregated per C§10.

## 13. Practical significance

T-20 `a = 0.5/n_cases`; binary counterexamples need no band.

## 14. Sensitivity analysis

Filesystem arm (ext4, XFS, btrfs); build arm (`ebound-baseline`, `ebound-probe`, `ebound-head`); fixture-construction
order arm for C-acl (ACL set before versus after `chmod`, and `os.utime` before versus after `setfacl`).

## 15. Expected negative results worth recording

The ACL refusal is correct (fixture inconsistent at capture time) — then the original probe note is withdrawn; symlink
refusal limited to some profiles; non-UTF-8 refusal limited to some positions.

## 16. Threats to validity

Loop-mounted XFS/btrfs inside WSL may differ from native kernels; setfacl behaviour depends on acl 2.3.2; the probe
build `9efbc74` is not the research baseline (both are measured).

## 17. Platform requirements

Linux x86-64 WSL2 as root (loop devices, `setfacl`); no Windows; no network; storage < 2 GB.

## 18. Estimates

Compute < 1 CPU-hour; disk ≈ 1 GB; agent effort: scripted execution, judgment only to write the finding records.

## 19. Feasibility smoke (non-decision-grade)

On 2026-09-17 the design session ran a single plain two-file generated tree (no symlink, ACL or invalid name) through
`/root/eb-research/target/baseline/release/ebound pack` and `convert --to tar --dry-run` inside
`/root/eb-research/scratch/legacy-006-smoke/`, only to confirm the commands and argument forms in §7 run end to end.
**Labelled NOT_DECISION_GRADE**; no outcome of any hypothesis was observed and nothing was committed under
`research/raw/`.
