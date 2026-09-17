# EXP-EVAL-010: Held-out readiness audits before the lock freeze and the design freeze (content overlap and lineage, transcript exposure, materialization, same-upstream tuning, public-benchmark tagging, §5.8 audits)

| Field | Value |
|---|---|
| Domain | evaluation (program sections 6, 7, 34, 35) |
| Status | NEEDS_TOOLING (audit scripts; execution by a held-out-aware corpus session, never by a design or analysis session) |
| Kind | Mechanical gate audits. Held-out eligibility (R5) of every EMPIRICAL decision depends on them. They produce pass/fail records and aggregate fractions, never held-out identities, statistics or content. |
| Gates served | G-B: `heldout-lock.json` frozen before the first decision-relevant result (§5.3). The content-overlap audit runs **before** that lock freeze, so any regrouping it forces is still allowed; after the freeze, relocking is integrity-only. G-C: design freeze, §14 item 15 and §5.8(a)-(h). |
| Method | `research/decision-method.md` §5.3, §5.4, §5.7, §5.8, §14 items 5 and 15 at `14b977c`; `thresholds.json` `protocol.content_overlap_max_fraction` |

## Author, independence and exposure

Designed by the Phase C-design evaluation-domain session. Its L7 exposure is recorded in the EXP-EVAL-001 protocol. **Role separation (§5.4 item 3):** the scripts read held-out identifiers and fingerprint-level fields internally, so a held-out-aware corpus session executes them. That session may not design candidates, experiments or analyses. Design and analysis sessions read only the aggregate outputs specified below, which contain no held-out item id, source, path, listing or content statistic.

## Question

Before the held-out lock freeze and the design freeze, are these conditions true?

- (1) Content shared across splits is at most 0.01 of logical bytes per family per split pair, and upstream lineages are grouped consistently.
- (2) Every program session's transcript exposure of held-out identifiers or paths is recorded (L7), and no content read (L1) occurred.
- (3) Held-out content is not present on disk in usable form.
- (4) Candidates' public tuning sources do not overlap held-out upstream lineages.
- (5) Public-benchmark contamination of held-out is tagged.
- (6) Every §5.8 pre-freeze audit (a)-(h) passes.

## Hypotheses and falsification

- **H1 (overlap, §5.3).** The known covariate overlaps in round-2 corpus critique §2 (F03 shared vendor packages, F17 kernel-header trees) exceed 0.01 for at least one (family, split pair) that involves held-out. Every other (family, split pair) is at most 0.01.
  - Falsified in the safe direction if no family exceeds the limit.
  - Falsified in the unsafe direction if a family not already flagged exceeds it.
  - Every exceedance must be resolved by regrouping and relocking **before** the lock freeze, then re-audited. After the lock freeze it becomes an L6 event handled per §5.7.
- **H2 (transcripts).** The transcript audit finds L7 hits (held-out identifiers displayed) in at least these sessions: the three disclosed in §0.1, and the Phase C-design sessions that read `PROGRESS.md` or corpus critiques, including this designer's disclosed exposure. It finds zero L1 events (no tool call reading a path under a held-out root, no held-out file content in a tool result).
  - Falsified by any L1 event. Every decision whose evidence comes from the leaked families then loses R5 eligibility, and the items are relocked as `spent` (§5.7).
  - Any L7 hit missing from the exposure record is added, and that session's design and analysis role is restricted (§5.4 item 3).
- **H3 (materialization, §5.4).** At the design freeze, `/root/eb-research/heldout` and `/root/eb-research/fingerprints/heldout` content either does not exist or exists only as owner-key-encrypted `*.age` bundles. Falsified by any readable held-out file, which blocks Commit A until fixed.
- **H4 (same-upstream tuning, §5.4 item 2).** For each decision's candidates, the declared public tuning data sources pass the held-out lineage check. Falsified per decision. A failing candidate cannot be DECIDED on held-out evidence from the overlapping families.
- **H5 (public benchmark, L5).** Held-out items from public benchmarks are tagged `public-benchmark` (count per family), and every held-out analysis reports with and without them (EXP-EVAL-012). Falsified by an untagged public-benchmark lineage found by the lineage check. That is an L5 event: the decision is blocked until resolved.
- **H6 (§5.8 audits).** (a) through (h) all pass at Commit A. Falsified by any failure, which blocks the freeze.

## Decisions informed

`DEC-ECO-076` (audit log of held-out access before freeze; permissible post-unlock policy rests on it), `DEC-ECO-077` (independence audit per family, split and scale; contamination tagging). Gate evidence for every EMPIRICAL decision's R5 eligibility.

## Candidates

Audit procedures, not product candidates.

- **Overlap measure** (pre-registered): per family f and split pair (A, B), `overlap = max(bytes_A(hash in B)/bytes_A, bytes_B(hash in A)/bytes_B)`. Bytes are logical file bytes. Hashes are file content SHA-256 from the fingerprint records. There is no exclusion of small or common files; the conservative verdict is reported, with a ≤ 4 KiB-file exclusion shown as sensitivity. Cross-family overlap per split pair is also reported. Derived items of one lineage in different families must share a split.
- **Lineage check.** Normalize each item's provenance URL to (host, owner or project path). If the same normalized lineage appears in two splits under different independence groups, it is flagged to the corpus owner for regrouping.
- **Transcript scan scope.** Every `*.jsonl` transcript under `C:\Users\<user>\.claude\projects\D--Projects-entrybound\`, including subagent transcripts, plus workflow journals under the orchestration directories. Patterns: every held-out `item_id`; every held-out fingerprint file name; the held-out root paths (`/root/eb-research/heldout`, `fingerprints/heldout`, `\\wsl.localhost\...\heldout`); held-out source-file selection-list names.
- **L1 heuristic.** A tool call whose input path resolves under a held-out root, or a tool result containing ≥ 64 consecutive bytes of a held-out file (content hash probe over fingerprinted file prefixes).

## Corpus selectors

Fingerprint-level fields of all splits (item_id, family, split, scale, independence group, file content hashes, byte counts, provenance URLs), read **inside the audit scripts only**. No content reads.

## Environment

- The overlap and lineage audits run in WSL.
- The transcript audit runs on the Windows host (read-only over the user profile's transcript directory; no writes to C:).
- Outputs go to `research/decisions/heldout-audit/` (committed).

## Commands (TO BE WRITTEN)

```sh
# 1 overlap + lineage (corpus session; prints only aggregates)
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && /root/eb-research/venv/bin/python research/corpus/tools/overlap_audit.py --manifest research/corpus/manifest.json --fingerprints /root/eb-research/fingerprints --max-fraction 0.01 --out research/decisions/heldout-audit/overlap.csv --owner-detail /root/eb-research/private/overlap-detail.json'
#    overlap.csv columns: family, split_a, split_b, overlap_fraction, overlap_fraction_excl_small, verdict   (no item ids)
# 2 transcripts (corpus session, Windows host)
C:/Python313/python.exe research/corpus/tools/transcript_audit.py --transcripts "%USERPROFILE%/.claude/projects/D--Projects-entrybound" --lock research/corpus/heldout-lock.json --manifest research/corpus/manifest.json --out research/decisions/heldout-audit/transcripts.csv
#    transcripts.csv columns: session_file_sha256, session_role, l7_id_hits, l7_path_hits, l1_events   (no matched text)
# 3 materialization
wsl.exe -d Ubuntu -- bash -lc '/root/eb-research/venv/bin/python /mnt/d/Projects/entrybound/entrybound/research/corpus/tools/materialization_audit.py --roots /root/eb-research/heldout,/root/eb-research/fingerprints/heldout --out /mnt/d/Projects/entrybound/entrybound/research/decisions/heldout-audit/materialization.json'
# 4 same-upstream tuning check (invoked per decision before its first validation look)
C:/Python313/python.exe research/corpus/tools/upstream_lineage_check.py --declared-sources research/experiments/<EXP>/record.md --out research/decisions/heldout-audit/lineage-<DEC>.json   # PASS/FAIL only
# 5 public-benchmark tagging
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && /root/eb-research/venv/bin/python research/corpus/tools/benchmark_tag_audit.py --out research/decisions/heldout-audit/public-benchmark.csv'   # per family counts
# 6 section 5.8 audits
C:/Python313/python.exe research/decisions/tools/freeze_audit.py --repo . --out research/decisions/heldout-audit/section-5-8.json
#    (a) git log --all -- research/corpus/heldout-unlock.json research/decisions/design-freeze.json is empty
#    (b) grep -rn allow_heldout research/ callers == {provision.py, assemble.py --verify-heldout}
#    (c) no research/raw/*/*.jsonl[.gz] row with split == heldout
#    (d) each decision experiment raw meta git sha descends from its spec's pre-registration commit and dirty == false
#    (e) transcripts.csv exists at the audited commit
#    (f) overlap.csv all verdicts PASS
#    (g) research/decisions/validation-looks.jsonl agrees with raw records (experiment, items, commit)
#    (h) research/decisions/l7-exposures.jsonl lists every exposure from §0.1, from transcripts.csv and from protocol disclosures
```

## Seeds

Not applicable: every audit is exhaustive and deterministic.

## Warmup

Not applicable.

## Measurement method and metrics

`overlap_fraction` (bounded by `thresholds.json` `protocol.content_overlap_max_fraction` = 0.01); L7 hit counts; L1 event count; readable held-out file count; lineage PASS/FAIL; public-benchmark counts per family; §5.8 booleans. No Appendix A.2 metric applies.

## Replication and adaptive rule

- Each audit runs once per gate: before the lock freeze (1, 5), and at Commit A (1-3, 6).
- Audit 4 runs before each decision's first validation look.
- Any fix triggers a full re-run of the affected audit.

## Normalization

None.

## Statistical analysis

Pass/fail against the pre-registered limits; no inference.

## Practical-significance thresholds

Content overlap at most 0.01 (§5.3, Appendix C.1). Zero tolerance for L1 events, readable held-out files, failed §5.8 items and untagged public-benchmark lineages.

## Sensitivity analysis

- Overlap with and without files of 4 KiB or less.
- Cross-family overlap.
- The transcript scan repeated with case-insensitive and path-normalized patterns (Windows backslashes, UNC forms).

## Expected negative results worth recording

- F03 and F17 overlap exceedances, which force regrouping before the lock freeze.
- L7 hits in many Phase A-C sessions, because `PROGRESS.md` names held-out upstreams. Every R5 pass on identity-aware held-out therefore keeps the `identity_aware_heldout` cap, and only the sealed tranche (EXP-EVAL-011) lifts it.
- Held-out content still materialized in readable form at audit time, which forces removal or encryption before Commit A.

## Threats to validity

- The transcript audit cannot see memory outside transcripts (for example, content an agent saw in another tool without a transcript), and pattern matching can miss paraphrased identities. It is a lower bound on exposure.
- Content hashes of files miss near-duplicates (a re-compressed or re-encoded copy). The lineage check partially covers these.
- Copies and hard links of held-out content outside the held-out roots cannot be detected by path audits (harness review L-13). The materialization audit also hashes large files outside the corpus roots against held-out fingerprint prefixes as a best-effort check.

## Platform requirements

WSL2 root; Windows host read access to the user profile transcripts; no network; no privileges beyond those.

## Estimates

- Machine time: about 3 h (overlap over roughly 300 items' file-hash sets, transcript scan over the program's transcripts, and a materialization hashing pass).
- Disk: under 1 GiB.
- Agent effort: scripted, executed by the corpus session; judgment only for the corpus owner's regrouping when H1 flags an exceedance.
