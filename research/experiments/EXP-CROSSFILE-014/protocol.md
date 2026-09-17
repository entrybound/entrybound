# EXP-CROSSFILE-014: Prevalence of dictionary-dependent Zstandard frames in real artifacts (census for dictionary-aware zstd import)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING**: `research/experiments/EXP-CROSSFILE-014/zstd_dictid_census.py` (stdlib-only recursive frame census, specified below) is not written |
| Domain / program sections | crossfile / 8.4 (legacy import, program §25) |
| decision_ids | DEC-LEG-106 |
| Evidence type | descriptive census (deterministic counts); no candidate selection by statistics |
| Method | `research/decision-method.md`, pre-registered |
| Record | Design only; `record.md` (§12.2) instantiated once gate G-A holds. |
| Design session and disclosure | Phase C-design crossfile designer; L7 disclosure as in EXP-CROSSFILE-001. |

## 1. Question

How common are Zstandard frames that declare a Dictionary_ID (and so cannot be decoded without an
external dictionary) among the real artifacts of the tuning and validation corpus (packages,
container layers, compressed tarballs, logs, database dumps), which is the evidence DEC-LEG-106
names ("prevalence of dictionary-compressed .zst artifacts") for accepting such frames when the
caller supplies the dictionary?

## 2. Hypotheses

- **H1.** Dictionary-dependent frames are absent or below 0.1% of zstd frames in every family:
  real distribution formats (Arch/Fedora/Ubuntu packages, OCI zstd layers, `.tar.zst` releases)
  do not use dictionaries, so the post-v1 capability has no corpus-evidenced demand.
- **H0.** Dictionary-dependent frames occur in at least one family at >= 1% of frames.
- **Falsification of H1.** Any family with dictionary-dependent frames above 0.1% of its zstd frames.

## 3. Decision informed; proposed sets

| Decision | Proposed type | Proposed od_ids | Proposed hc_ids |
|---|---|---|---|
| DEC-LEG-106 | EMPIRICAL (census) + FORMAL/cost for the import path | OD-19 (`import_acceptance_fraction` on dictionary-framed cases, only if the capability is built later) | HC-06, HC-07, HC-12 |

## 4. Candidates

`status-quo-refuse` (EB_WRAPPER_DICTIONARY_UNSUPPORTED) and `caller-supplied-dictionary`. The
census does not compare candidates; it supplies the demand input. If H1 holds, `caller-supplied-
dictionary` has no measured benefit, and R10 keeps the status quo by cost (a new supported path is
T2); if H0 holds, a follow-up import experiment (legacy domain) is required before selection.

## 5. Corpus selectors

- Manifest `ed28b1b4…`. `spec.yaml` covers the **tuning** split, every family and tier. The census
  has no search step; the validation split is then censused once (`spec-validation.yaml`,
  registered in `research/decisions/validation-looks.jsonl` as DEC-LEG-106's first look, §5.2) to
  confirm the tuning finding on independent groups. Held-out is excluded before unlock.
- Unit of census: every regular file, and every member reachable by recursion into tar, zip, ar
  (`.deb`), RPM payloads, gzip/xz/zstd transport layers and OCI blobs, to depth 4, with a per-member
  decode bound of 256 MiB and a total decoded-bytes bound of 4x the item size (bounds recorded;
  members beyond them are counted as `unscanned`).

## 6. Environment and platform requirements

`wsl-ubuntu`, x86-64, ext4; stdlib Python 3 plus the system `zstd` binary only for frame-header
cross-checks; `timing: false`; no network; no privilege.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
PY=/root/eb-research/venv/bin/python
$PY -m ebr validate --spec research/experiments/EXP-CROSSFILE-014/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-CROSSFILE-014/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-CROSSFILE-014/spec.yaml
$PY -m ebr normalize --spec research/experiments/EXP-CROSSFILE-014/spec.yaml
```

Tool specification (`zstd_dictid_census.py --item-path P --item-id I`, one JSON line): detect
Zstandard frames by magic `28 B5 2F FD` at member start and at every frame boundary within a
concatenated stream (following `Frame_Content_Size`/block headers per RFC 8878 §3.1.1, cited),
parse `Frame_Header_Descriptor` (`Dictionary_ID_flag`, `Single_Segment_flag`) and the
`Dictionary_ID` field, also count skippable frames (`0x184D2A5?`). Output counts:
`zstd_frames`, `zstd_frames_with_dictid` (Dictionary_ID present and non-zero),
`distinct_dictids`, `zstd_members`, `zstd_members_with_dictid`, `skippable_frames`,
`unscanned_members`, `decode_errors`. Unit tests over generated frames made with `zstd -D` and
without, before first use.

## 8. Seeds, warmups, repetitions

No randomness. Warmups 0. Repetitions 2 plus repeat-hash of the canonical output JSON.

## 9. Metrics

| Metric | Role | ebr family |
|---|---|---|
| `zstd_frames`, `zstd_frames_with_dictid`, `zstd_members`, `zstd_members_with_dictid`, `distinct_dictids`, `skippable_frames`, `unscanned_members`, `decode_errors` | descriptive census (non-canonical; no band) | other |
| `census_sha256` | repeat-hash | correctness |

## 10. Normalization and statistical analysis

Descriptive: per family and overall, frames-with-Dictionary_ID / zstd frames and members with such
frames / zstd members, with exact binomial (Clopper-Pearson) 0.95 intervals computed by the
analysis script and reported with the independence-group structure (items of one group are not
independent). No R4 comparison.

## 11. Practical-significance thresholds

None (descriptive census). The H1 threshold of 0.1% is a pre-registered decision input for this
record, not a canonical band.

## 12. Sensitivity analysis

Recursion depth 2 versus 4; decode bound 64 MiB versus 256 MiB; excluding generated items.

## 13. Validation look and held-out placeholder

See §5; held-out placeholder `EXP-CROSSFILE-014-HO` at Commit A (§5.5) for the same census after
unlock (report-only).

## 14. Expected negative results worth recording

No dictionary-dependent frames anywhere in the corpus (demand not evidenced), or frames present
only in synthetic adversarial items (F20), which is a hostile-input test case, not demand.

## 15. Threats to validity

The corpus is curated, not a random sample of the world's `.zst` files; dictionary use concentrates
in private systems (databases, messaging) that public corpora rarely contain. The census therefore
bounds prevalence in public distribution artifacts only; demand elsewhere stays an adoption-domain
question (program §32).

## 16. Cost

About 6 CPU-hours (decompression of package payloads dominates); disk: transient decode buffers in
memory, raw about 20 MB. Agent effort: scripted tooling (small), none for execution.

## 17. Evidence this decision needs that this experiment does not produce

- **DEC-LEG-106:** demand outside public distribution artifacts (private databases and message
  systems) is an adoption-workflow question (ecosystem domain, program §32); import correctness of a
  caller-supplied-dictionary path would need its own legacy-domain experiment if the census finds
  demand.
