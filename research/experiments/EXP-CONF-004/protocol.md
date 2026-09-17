# EXP-CONF-004 — Native conformance corpus `ebcc-v1`: generators, expectation classes and production outcome census across every reader path

| Field | Value |
|---|---|
| Status | `NEEDS_TOOLING` (CC§2): `ebr-conformance` crate (`ebcc-gen`, `ebcc-census`), byte-level builders, case schema validator, environment-fault fixtures, local test CA for timestamp cases |
| Domain / program sections | conformance / §29 (primary), §28 (expected outcomes consumed by EXP-CONF-002), §30 (seeds for fuzzing) |
| Decision IDs informed | DEC-ECO-013, DEC-ECO-042, DEC-ECO-012, DEC-MOD-041, DEC-INT-040, DEC-INT-020, DEC-INT-006, DEC-INT-008, DEC-INT-042, DEC-CON-003, DEC-CON-004, DEC-CON-009, DEC-CON-011, DEC-CON-012, DEC-CON-014, DEC-CON-023, DEC-CON-024, DEC-CON-025, DEC-CRY-012, DEC-CRY-020, DEC-CRY-029, DEC-CRY-065, DEC-CRY-071, DEC-CRY-076, DEC-CRY-077, DEC-CRY-085, DEC-CRY-103, DEC-ACC-021, DEC-ACC-036, DEC-MOD-003, DEC-MOD-008, DEC-MOD-014, DEC-MOD-029, DEC-MOD-032, DEC-MOD-040, DEC-MOD-047, DEC-MOD-048, DEC-PLT-012, DEC-PLT-019, DEC-PLT-023, DEC-PLT-036, DEC-PLT-038, DEC-PLT-050, DEC-PLT-053, DEC-CMP-005, DEC-CMP-010 |
| Requirements | REQ-ECO-0136 (Task 29 obligation), REQ-ECO-0014, REQ-ECO-0021, REQ-ECO-0183 |
| HC oracles | MVT-01(b), MVT-01(d), MVT-03(a), MVT-03(c), MVT-04 items 1-10, MVT-15(b) (precedence table), MVT-17(b); proposal for the assigner: `hc_ids` HC-01, HC-03, HC-04, HC-15, HC-17; `od_ids` OD-04, OD-17 |
| Decision type | not assigned (gate G-A) |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1 |
| Depends on | EXP-CONF-003 (normative statement IDs `ns_ids`); produces the corpus used by EXP-CONF-002, -005, -007, -008, -009, -010 |

## 1. Question

For every normative rule and every frozen bound of the native format, what outcome (class, reason code, detail record,
verification qualifier, EAM and identity digests) does each production reader path and CLI command return on a minimal
generated case; does it match the expectation derived from the written rule; and do the reader paths agree with each
other?

## 2. Hypotheses and falsification

- **H1 (HC-03, HC-04, HC-15, HC-17).** Every `REQUIRED` and `FORBIDDEN` native case yields its expected outcome class,
  and its expected reason code where the documents define one, on every applicable path, including CLI exit status.
  *Falsified* by one mismatch on `ebound-head`.
- **H2 (cross-path agreement; DEC-CON-025, DEC-CRY-065).** All applicable paths return the same comparison key
  (CC§5) for every case, including preamble, segment and Index variants. *Falsified* by one cross-path disagreement.
- **H3 (native `UNRESOLVED` empty; REQ-ECO-0021, MVT-03(c)).** Every native case receives a decided class from written
  text. *Falsified* by one case whose expectation cannot be decided (recorded as a specification defect).
- **H4 (determinism of reading, HC-03/HC-11).** Two runs of the same build on the same case give identical tuples.
  *Falsified* by one differing pair (nondeterminism artifact rule, objective §2.0 rule 2).
- **H5 (canonical form, MVT-03(a)).** Every valid case decodes and re-encodes to identical bytes. *Falsified* by one
  byte difference.

## 3. What this experiment contributes per decision

| Decision | Contribution |
|---|---|
| DEC-ECO-013 | Classification of every native case under the five-class taxonomy with profile binding; count of native cases that cannot be decided (H3) |
| DEC-ECO-042 | The executed prototype of `bytes-generator-and-hashes` (exported bytes, manifest, generator identity); regeneration determinism is measured in EXP-CONF-005 |
| DEC-ECO-012 | The executed schema (§5.2) and its traceability fields |
| DEC-MOD-041, DEC-INT-020, DEC-MOD-014 | Reason code and detail record per negative case per path; multi-fault precedence family; codes emitted versus expected |
| DEC-INT-040, DEC-INT-006 | F-TRUNC family: truncation at every section/item boundary and precedence cases for INDEXED, STREAM and encrypted layouts, with ground-truth fault maps |
| DEC-CON-025, DEC-CRY-065 | Cross-path tuples over F-INDEX and F-CRYPTO variant families (H2) |
| DEC-CON-003, -004, -009, -011, -012, -014, -023, -024 | Feature-bit swap cases; all-ones flag and budget vectors; crypto structural limit boundary vectors; ro_compat/compat operation-by-command matrix; version gate cases; unknown-element and ignorable-item authentication cases; native polyglot cases; stale advisory-field cases (production outcome, inspect and salvage behaviour) |
| DEC-CRY-012, -029, -071, -076, -077, -085, -103, DEC-CRY-020 | Security-report-field falsity cases; huge `total_len` private objects; unknown stanza types and protection classes; RFC 3161 constrained-field negative tokens; wrong key/password/tag failure code census; signature fault-class cases; unknown signature version/algorithm/mask/critical attribute; mixed-encoding order cases |
| DEC-ACC-021, DEC-ACC-036 | `inspect` outcome on every refused and nonconforming case; refusal-path code for `BudgetDeclared=false` streams exceeding policy |
| DEC-MOD-003, -008, -029, -032, -040, -047, -048 | Empty LinkTarget and omitted tag 6; pack-versus-replan chunker_id and PCR; identity outcome per operation (SPEC §8.1 table with F-08); environment-fault classes; `plan_ref` 0 on non-region chunks; stored derivable digest disagreement (cache mutants); orphan Chunks in both layouts |
| DEC-PLT-012, DEC-PLT-023 | Residual destination state (objects created, partial files, pre-existing objects changed) after each F-ENV fault on WSL ext4 and on the Windows-host NTFS CLI arm; `ebound diff` report records for archives differing only in the executable bit (one SEMANTIC record, AUXILIARY record, or both); the user-interpretation parts of both decisions are not addressed |
| DEC-PLT-019, -036, -038, -050, -053 | Hardlink-alias inode-scoped metadata disagreement; unknown ACE types and undefined mask bits; symlink-tagged ReparsePoint uniqueness; Timestamp negative, pre-1601, post-2446 and out-of-range nanoseconds; unknown Critical/Optional metadata items |
| DEC-CMP-005, DEC-CMP-010 | `chunker_id` grammar variants; unknown codec and transform identifiers and malformed parameter records |

For each of these decisions, where the ledger's candidates imply different outcomes on a case, the case records the
expected outcome **per candidate id** (`PROFILE_DEPENDENT` bound to `DEC-…:candidate_id`), so the number of cases whose
outcome each candidate changes relative to status quo is computed mechanically (descriptive; an input to C1 counts of
added reason codes and to the decision's candidate analysis).

## 4. Candidates

- **Reader builds compared** (not design alternatives): `ebound-head` (census of the current product) and
  `ebound-baseline` (status quo at `9e44608`, CC§4). The reference is `ebound-baseline`; differences between the two are
  reported as behaviour changes since the research baseline.
- **Design candidates informed** are the ledger's (§3), evaluated through per-candidate expectations; none added (CC§1).
- **Exclusions:** none of the reader paths is excluded; a path the product does not expose for a case family is
  recorded `NOT_APPLICABLE` with the reason, never silently skipped.

## 5. The corpus

### 5.1 Generation

- `ebcc-gen` (Rust, `research/harness/crates/ebr-conformance`) is format-building code (F-18) with two builders:
  (a) **valid-archive builder**: packs seeded generated trees through the production public API in deterministic mode
  (unencrypted, and encrypted with fixed public test secrets documented in the manifest); (b) **byte builder**
  (`ebcc::bytes::{Preamble, Descriptor, FeatureBitmap, SectionDirectory, Section, Record, Footer, StreamItem, Segment}`)
  that writes fields from the documented layouts and computes digests with the `sha2` crate directly from the
  documented hash constructions, never through production helpers.
- **Builder self-check**: every byte-builder output of a valid case is parsed by the frozen arm-B Python framing reader
  of EXP-CONF-001 (`ebir-py`), which shares no code with the builder; any disagreement blocks the family until the
  generator or a documented erratum resolves it (recorded in the deviation log).
- Export: `ebcc-gen export --out /root/eb-research/conformance/ebcc-v1 --manifest research/conformance/corpus/ebcc-v1/manifest.json --ebr-manifest research/conformance/corpus/ebcc-v1/ebr-manifest.jsonl --seed 2809170401`.
  Cases larger than 256 MiB (some F-BOUND cases) are exported as **hash-locked generator entries** (generator
  command, parameters, SHA-256) and materialized on demand.

### 5.2 Case schema (`ebcc.case.v1`, JSON)

`case_id` (`EBCC-<FAMILY>-<nnnnn>`, never reused), `family`, `ns_ids` (normative statement IDs from EXP-CONF-003),
`property` (one sentence), `bytes_file`, `bytes_sha256`, `bytes_length`, `generator` (`name`, `version`,
`builder_sha256`, `seed`, `params`), `layout` (`INDEXED`, `STREAM`, `ENCRYPTED_INDEXED`, `ENCRYPTED_STREAM`, `NONE`),
`features` (bit values per tier), `secrets` (public test identity/password reference or null),
`expectation` = { `taxonomy`: `five-class-v1`, `class` (`REQUIRED`, `FORBIDDEN`, `ACCEPTABLE`, `PROFILE_DEPENDENT`,
`UNRESOLVED`), `outcome_class`, `reason_code` (string or null), `code_status` (`DOCUMENTED`, `UNDOCUMENTED`),
`detail_required` (list of SPEC §20.5 detail fields), `unverified` (bool or null), `eam_sha256` (valid cases),
`identities` (LAI/PCR/AUX/PCI for valid cases, computed from the documented constructions by the builder),
`per_candidate` ({`DEC-…`: {`candidate_id`: outcome}}) }, `applicable_paths` (§6.2), `policy` (caller policy used),
`resource_ceiling` (filled by EXP-CONF-007), `fault_map` (for F-TRUNC: injected operation, offset, region, ground-truth
first-reached fault), `security` (flags), `expectation_author`, `expectation_reviewer`, `notes`.
Validated by `research/conformance/tools/validate_manifest.py` (JSON Schema `research/conformance/corpus/ebcc.case.v1.schema.json`).

### 5.3 Expectation discipline

1. The expectation author (CC§11) writes each expectation from the cited normative statements **before** any census
   of that family. When the text decides the class but not the code, `code_status: UNDOCUMENTED`.
2. A reviewer session checks every expectation against its citation; disagreements are resolved by citation or the
   case is marked `UNRESOLVED` (a specification defect under H3, never silently decided).
3. Expectations are frozen (manifest SHA-256 committed) before the census. A census result never changes an expectation
   except through an erratum with a citation, logged as a deviation with `results_visible: yes`, and both the original
   and amended match counts are reported.

### 5.4 Case families (pre-registered enumeration rules)

| Family | Enumeration rule |
|---|---|
| F-PRE | magic variants; namespace string variants; major and minor version values (current, current+1, 0, max); preamble length variants; each reserved byte individually nonzero; encrypted preamble sentinels |
| F-DESC | Descriptor v1 and v2: every tag missing, duplicated, out of order, unknown tag, non-minimal length; each budget field at 0, documented maximum, all-ones; each decode flag bit set individually and all-ones; `BudgetDeclared` true/false combinations |
| F-FEAT | every unassigned bit of each tier (incompat, ro_compat, compat) set individually; stale feature-bitmap digest; `0x10000` without `0x8000`; each assigned bit swapped with a neighbour under fixed section ids |
| F-SECT | missing, duplicated, reordered and unknown section types in critical and non-critical positions; per-feature section numbering variants |
| F-REC | every canonical record type: non-minimal TLV length, duplicate TLV, unknown TLV, unknown record version, field width overflow; orphan Chunks (INDEXED and STREAM); `plan_ref` 0 on non-region Chunks; stored derivable digest disagreeing with the authority; stale INDEXED footer hint; nonzero hostility-summary slot; footer totals disagreeing with manifest |
| F-EAM | MVT-17(b) hazards: `.` and `..` components under every declared encoding including UTF-16LE `2E 00 2E 00`; absolute and rooted paths; separator inside a component; empty component; duplicate LogicalPath; non-Directory proper prefix; missing ancestor; Directory with content; empty LinkTarget; empty tag-6 sequence omitted and present; canonical-order violations including mixed encodings; Timestamp negative, pre-1601, post-2446, nanoseconds ≥ 10^9; hardlink aliases disagreeing on each inode-scoped metadata name |
| F-META | unknown Critical and Optional items at entry and archive scope; archive-scope item at entry scope and the reverse; registered item with wrong value type; Critical item in an unregistered non-`x-` namespace (MVT-04 items 4, 6, 8, 9, 10); unknown Windows ACE types; undefined mask bits in each security-metadata mask; symlink-tagged ReparsePoint; single-byte change inside an ignored optional item (authentication check) |
| F-BOUND | at the bound, bound−1 and bound+1 for every frozen count and size limit named in the documents, including: canonical sequence cap; symlink target size; path component length and depth; xattr count, value size, name length; ACL count and ACE count; sparse extent count; crypto segment DATA-record count and plaintext size; EBCS item count, item size, total size and the effective maximum (2^30−12, 2^30−11, 2^30); CONTROL and PAYLOAD record caps; recipient stanza size, envelope size, directory label length, recipient count, identity-attempt budget; KDF parameters at and above the wire ceiling; reconstruction limits; codec window and dictionary sizes at and above declared requirements (resource ceilings are measured in EXP-CONF-007) |
| F-RES | MVT-06(a) enumeration: declared counts or sizes above default policy; declarations below reality; expansion-ratio bombs; path-depth and metadata-byte bombs; long and transitive lookback chains; overflowing arithmetic fields; STREAM with `BudgetDeclared=false` exceeding policy mid-stream; element counts differing from the Descriptor in both directions; private objects with huge declared `total_len` |
| F-CODEC | per codec: multiple frames, zstd skippable frames, trailing bytes, content size absent or wrong, dictionary id mismatch, declared output length mismatch, window above declaration; unknown codec and transform identifiers; malformed parameter records; `chunker_id` grammar variants (empty, unknown algorithm, non-power-of-two target, unordered sizes, overlong) |
| F-TRUNC | truncation at every section boundary, every STREAM item boundary and every record boundary of the valid base cases, plus offsets 1..64 into the preamble; appended bytes; concatenated archives; native polyglots (ZIP end-of-central-directory inside a stored chunk, tar header prefix, executable prefix); every row of the objective MVT-15(b) precedence table; two- and three-fault combinations whose first-reached fault differs by read order; digest damage in a requested versus an unrequested chunk |
| F-INDEX | Index absent, zeroed, truncated, disagreeing with authority in each locator field (REQ-ECO-0183 twins); segment-spanning objects; Index validation variants across openers |
| F-CRYPTO | public framing: CONTROL/PAYLOAD order (PAYLOAD first), Descriptor not at ordinal 0, per-stanza size variants; unknown stanza types and protection classes; wrong identity, wrong password, tag failure, structure invalid after decryption, policy refusal; `max_key_derivation_cost` variants; archives on which each security-reporting field (`private_metadata_authenticated`, `secret_material_exposed`, `independently_validated`, `payload_suite`) would be false |
| F-SIG | signature records with unknown version, algorithm, mask bits, critical attributes; embedded algorithm-2 record; detached `.ebsig` version 2; malformed binding transcripts; ADDRESSING requested on plaintext; missing trust anchors; RFC 3161 tokens violating each constrained field (policy OID, TSA name, nonce, accuracy/skew, unknown critical extension, ESS signing-certificate-v2 absent), built with a local test CA (`openssl` 3.0.13) whose keys are public test material |
| F-TIER | each unknown ro_compat and compat bit × each command (`repack`, `repack --add`, recipient add, recipient remove, password rotation, signature add, `strip-provenance`, `export`, random read) |
| F-OPS | identity outcome per operation: pack, representation-only repack, replan, layout change, key add/remove, signature add, export then re-import; expected LAI/PCR/AUX/PCI change pattern and binding states from SPEC §8.1 as refined by F-08; `ebound diff --json` between archives differing only in the executable bit (records and tiers per `docs/archive-diff-v1.md`) |
| F-VSTATE | verification state per API or command (list, inspect, read_entry, range read, verify, unpack, diff) × source (Memory, LocalFile, HttpRange via in-process `ebr-origin`, STREAM pipe) × fault (none, requested chunk, unrequested chunk, Index damage, footer truncation) (MVT-15(a) black-box variant) |
| F-ENV | environment faults with a valid archive: destination on a full 16 MiB loop-mounted ext4 (ENOSPC mid-staging), permission-denied destination, closed stdout pipe, HTTP connection reset, short 206, ETag change mid-session and `Content-Range` mismatch from `ebr-origin` misbehaviour modes; after each extraction fault the destination tree (including a pre-existing sibling object) is fingerprinted before and after to record residual state |

The per-family case lists are generated, and their counts are recorded in the manifest before the census; the design
does not fix numeric totals.

## 6. Environment, paths and commands

### 6.1 Platform

WSL2 Ubuntu 24.04 x86-64, ext4 under `/root/eb-research` (never `/mnt/*`); root is available for loop mounts (F-ENV).
Windows host (NTFS, non-admin) runs the CLI paths P-CLI-* on the same case bytes to record exit status and messages
(`ebound.exe` from `ebound-head` Windows build). No timing (`timing: false`); `wall_ns` is informational.

### 6.2 Reader paths (role names; the census binary prints the exact production API symbol for each in its output header)

| Path id | Role |
|---|---|
| P-MEM | library full open and verify from in-memory bytes |
| P-FILE | library full open and verify from a LocalFile source |
| P-RAND-MEM, P-RAND-FILE | random-access opener; list, then read every entry and one seeded range per entry |
| P-HTTP | random-access opener over an HttpRange source served by in-process `ebr-origin` |
| P-STREAM | sequential STREAM reader from a Read-only pipe |
| P-ENC-FULL, P-ENC-RANGE | encrypted full open and encrypted range opener with the case's public test secret |
| P-CLI-VERIFY, P-CLI-INSPECT, P-CLI-UNPACK | `ebound verify`, `ebound inspect --json`, `ebound unpack` into scratch (exit status, JSON, created objects) |
| P-CANON | valid cases only: decode to EAM and re-encode with the same identifiers; byte comparison (MVT-03(a)) |

### 6.3 Commands

```sh
# once per build (CC§4), after lock_parity.py and harness-cli-parity.sh print RESULT PASS
wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh build --release -p ebr-conformance
# generation (not part of the spec; its outputs are inputs)
/root/eb-research/target/harness/release/ebcc-gen export --out /root/eb-research/conformance/ebcc-v1 \
  --manifest research/conformance/corpus/ebcc-v1/manifest.json \
  --ebr-manifest research/conformance/corpus/ebcc-v1/ebr-manifest.jsonl --seed 2809170401
python3 research/conformance/tools/validate_manifest.py research/conformance/corpus/ebcc-v1/manifest.json
# census (runner-driven)
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate  --spec research/experiments/EXP-CONF-004/spec.yaml
/root/eb-research/venv/bin/python -m ebr run       --spec research/experiments/EXP-CONF-004/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-CONF-004/spec.yaml
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-CONF-004/spec.yaml
# decided analysis (checked-in; provenance headers per §12.3)
python3 research/conformance/tools/census_tables.py --exp EXP-CONF-004 --manifest research/conformance/corpus/ebcc-v1/manifest.json
```

Each sample runs `ebcc-census` over one family bundle with one build, all applicable paths, writing one
`ebr.readtuple.v1` line per (case, path) to the raw-artifact directory and printing the summary (CC§5).

## 7. Seeds, warmup, replication

Generation seed `2809170401`; ebr order/bootstrap seeds CC§12. No warmup. **Two repetitions** in different rounds and
processes (§4.6 deterministic metrics) with the repeat-hash check on `payload.artifact_sha256` (§4.13). A differing
pair is the H4 violation artifact; both tuple files are committed (objective §2.0 rule 2).

## 8. Metrics (CC§9)

| Metric | Role | Unit | Definition |
|---|---|---|---|
| `reason_code_specificity_fraction` | banded (T-20) | fraction | over negative native cases with `code_status: DOCUMENTED`: fraction whose production code on P-MEM uniquely identifies the violated rule (one code ↔ one `ns_id` group) |
| `equivalence_assertion_pass_fraction` | binary | fraction | over F-OPS and INDEXED/STREAM twin cases: fraction whose equivalence assertions (F-07, F-08) hold |
| `adversarial_true_refusal_fraction` | binary | fraction | over F-RES under the default policy: fraction refused with a non-OK class |
| `reader_disagreements` | binary | count | distinct (case, path pair) comparison-key disagreements among production paths (H2) |
| MVT violation counts: MVT-01(b), MVT-01(d), MVT-03(a), MVT-03(c), MVT-04, MVT-15(b), MVT-17(b) | binary (HC oracles, CC§9) | count | per the objective's oracle for each family |
| `error_actionability_fraction` | banded (T-20), secondary here | fraction | over refused cases on P-CLI-*: message names the failed requirement and a remedy (scripted census rules pre-registered in `research/conformance/tools/message_census.py`) |
| Expectation match counts (class; code where documented; detail fields present) per family and path; `UNDOCUMENTED` expected codes; `UNRESOLVED` native cases; per-candidate outcome deltas; nondeterministic cases | descriptive, non-canonical | count | `census_tables.py` |

## 9. Normalization and statistical analysis

Deterministic census: no intervals (§4.13). Aggregation AGG-W for violation counts and AGG-C (per family, minimum over
families as headline) for fractions. The binary metrics screen at R1 (§3.3). `reason_code_specificity_fraction`
comparisons between `ebound-head` and `ebound-baseline` or between candidate expectation profiles use the T-20
absolute band `a = 0.5 / n_cases`; the case set is fixed before results (objective OD-04). No corpus-family weighting
applies: conformance families are not corpus families, and §4.9 minimum support is not claimed; results are scoped
to the conformance corpus population. The analysis plan needs adoption by a non-exposed session (CC§1).

## 10. Practical-significance thresholds

§3.3 for binary metrics; T-20 (`thresholds.json` `metric_thresholds.reason_code_specificity_fraction`,
`…error_actionability_fraction`) for the two fractions; descriptive counts have no band.

## 11. Sensitivity analysis

- Expectation reviewer agreement: a second reviewer re-derives a seeded 20% sample of expectations (seed `2809170402`);
  kappa and the match counts under the second reviewer's expectations are reported.
- Match counts with `UNDOCUMENTED`-code cases counted as class-only matches versus excluded.
- Path subsets: library paths only; CLI paths only; Windows-host CLI versus WSL CLI (same bytes).
- Builder self-check failures included versus excluded.

## 12. Split discipline and held-out placeholder

The conformance corpus is generated from normative text and is not a corpus split; no tuning, validation or held-out
item is read. F-OPS and F-VSTATE use seeded generated trees only. Held-out placeholder: none required for this
population; the census is re-run on the frozen build at Commit A (§5.5) so that Phase D evidence cites the frozen
build's tuples.

## 13. Expected negative results worth recording

- Reason codes emitted but undocumented, and documented codes never emitted.
- Cross-path divergence between separately implemented encrypted readers (ledger status quo "divergent").
- TRUNCATED versus CORRUPT precedence differences between INDEXED, STREAM and encrypted layouts.
- Advisory fields (footer hint) that change outcomes although documented as never trusted.
- `inspect` refusing archives that extraction policy refuses (DEC-ACC-021 status quo).
- CLI exit status collapsing distinct outcome classes.

## 14. Threats to validity

- **Generator bugs** create false mismatches; mitigated by the arm-B framing self-check and by reviewing every
  mismatch cluster before attributing it to production (triage, CC§6).
- **Circular oracles**: valid encrypted and signed bases are built with the production encryptor and signer; the
  census tests public framing, outcome classes and codes, not cryptographic correctness (routed to crypto review).
- **Expectation bias**: the expectation author may unconsciously follow production behaviour; expectations are written
  before the census and reviewed by a second session, and the author may read production source only after writing.
- **Path coverage**: some paths may need `research-internals` exposure; any such path is run on `ebound-research` only
  after the MVT-16(c) identity proof passes.
- **Environment faults** are emulated (loop device, origin misbehaviour modes), not observed in the field.
- **Shared base model** across author, reviewer and triage sessions (CC§11).

## 15. Cost estimates

| Item | Estimate |
|---|---|
| Compute | 40 CPU-hours (generation of large F-BOUND cases 6 h; census 2 builds × 2 repetitions × all families and paths ≈ 34 h, 32 parallel workers) |
| Disk | 60 GB peak (large bound cases materialized transiently; exported small cases < 2 GB; tuple artifacts compressed ≈ 5 GB) |
| Agent effort | **judgment, medium-high** (expectation author and reviewer sessions per family; mismatch triage); generation and census scripted |
| Platform | Linux x86-64 (WSL, root for loop mounts); Windows host for CLI exit-status arm; no network beyond loopback; no timing |

## 16. Outcome obligations

`outcome.json` (§10.1). Every mismatch attributed to production becomes a defect work item (objective §2.0 rule 5);
every `UNRESOLVED` native case becomes a specification finding; every family's tuples are the expected-output baseline
for EXP-CONF-002 and the seeds for EXP-CONF-008/009/010. The corpus version `ebcc-v1` is frozen by the manifest SHA-256
in the outcome record; later versions (`ebcc-v1.1`, …) record added, removed and re-expected cases.
