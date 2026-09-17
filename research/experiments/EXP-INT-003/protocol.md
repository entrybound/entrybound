# EXP-INT-003: Verification-state honesty and status-vocabulary conformance matrix

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T11(a) byte-verification log, matrix driver `ebr-int-statematrix`, origin wrapper around `ebr-origin`, typed-API compile-fail prototypes) |
| Domain | integrity (program §24; objective HC-15 MVT-15(a); OD-11, OD-04, OD-25) |
| Decisions informed | DEC-INT-042, DEC-INT-044, DEC-ACC-035, DEC-INT-021, DEC-INT-038, DEC-CRY-090, DEC-INT-020, DEC-MOD-014 |
| Gates, method, author | conventions C0 |
| Spec | `spec.yaml` (one sample = the whole matrix for one item and one archive type) |

## 1. Question

For every read API and CLI command, source kind, archive type and injected fault location,
does any reported verification state (library report fields, CLI text, JSON and exit status)
claim more than the bytes actually verified support; which state combinations are
reachable; and which candidate state representation can express every reachable oracle state
without overclaiming?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-INT-044, DEC-INT-042 | The status quo has overclaim cells: success paths hard-set report booleans (ledger: "VerificationReport all-true booleans", `semantic_metadata_sections_verified` true without a FIDELITY fetch), so random reads over archives with damaged unread FIDELITY, dictionaries or groups report those checks as passed. `mvt15a_overclaim_cells` > 0. | Zero overclaim cells over the full matrix (then the status quo is honest and the decision turns on cost and expressibility only). |
| H2 | DEC-ACC-035 | The number of reachable distinct (identity status tuple, per-read state) combinations is small enough (< 64) to be given normative interpretation wording, but at least one combination is currently printed identically to a strictly stronger one. | No collapsed pair. |
| H3 | DEC-INT-044 `verify-more-in-random-reader` | Verifying FIDELITY and AUX and the used dictionaries and groups in random reads adds at least 1 request and more than 16 KiB of `verified_range_fetch_bytes` per first read over HTTP (a cost above the T-11 and T-12 floors) but none per subsequent read in the same session. | Added cost inside the T-11 and T-12 bands. |
| H4 | DEC-INT-038, DEC-CRY-090 | Status quo binding statuses after a cryptographic signature failure are `STALE`, contradicting the suite specification table that requires `INVALID` (ledger candidate `spec-invalid-on-failure`); conformance mismatches > 0. | Zero mismatches against `docs/crypto-suite-v1.md` tables. |
| H5 | DEC-INT-021 | Keyless `ebound verify` of an encrypted archive exits 0; a script testing only the exit status accepts it as verified. Among comparable tools (gpg, minisign, cosign, sha256sum) at pinned versions, the "no key / not verifiable" condition exits non-zero. | Status quo exits non-zero, or the comparable tools exit 0 in the analogous condition. |

## 3. Decisions and candidates

| Decision | Candidates | Evaluation |
|---|---|---|
| DEC-INT-042 | `status-quo-report-structs` (measured), `typed-verified-wrapper`, `diagnostic-qualifier-field`, `per-check-tristate-report`, `combined-type-and-qualifier`, `separate-verified-and-raw-apis`, `plain-ok-no-scope` | Status quo measured by the matrix. For each alternative, an **expressibility census**: a research schema for the candidate (JSON schema plus Rust types in `research/harness/crates/ebr-integrity/proto/state_api/`) and an automated mapping from every reachable oracle state to the candidate's representation; metric = fraction of reachable oracle states represented without overclaim (must be 1; T-20 coverage). Typed candidates also get **compile-fail tests** (`trybuild`): each pre-registered misuse (passing unverified bytes where verified bytes are required, reading content before verification) must fail to compile. Proposed L0 exclusion of `plain-ok-no-scope`: violates HC-15 ("`UNVERIFIED` qualifies `OK`"); counterexample = any random read, which it must report as plain `OK`; kept as a negative control. |
| DEC-INT-044 | `status-quo-all-true-flags` (measured), `tristate-per-check`, `narrow-flag-definitions`, `verify-more-in-random-reader`, `typed-opened-archive-scope`, `scope-enum-only` | Matrix plus census as above; `verify-more-in-random-reader` is also **measured** with a T11(d) reader option (bytes and requests over `ebr-origin`). A flag-to-check **code audit** table (every report field mapped to the check that sets it, per reader) is produced by a session not involved in the reader and committed as `audit/flag-check-map.csv` before the matrix runs; the matrix then tests each mapping. |
| DEC-ACC-035 | `status-quo-per-root-statuses`, `typed-verification-state-per-read`, `interpretation-matrix`, `outcome-qualifiers`, `aux-verification-for-random` | Enumeration of reachable status combinations from the matrix; expressibility census per candidate; `aux-verification-for-random` measured like `verify-more-in-random-reader`. The ledger's "user comprehension study" is a human claim routed to external review or the ecosystem domain's simulated evaluation (§4.16); not claimed here. |
| DEC-INT-021 | `status-quo-exit-zero-label`, `distinct-exit-code`, `require-public-only-flag`, `refuse-without-unlock`, `json-qualifier-exit-zero`, `policy-flag-require-full` | Mechanical census: for each candidate, over the pre-registered script patterns of `patterns/exit-status-scripts.txt` (for example `ebound verify x && deploy`, `if ebound verify x; then`, JSON field checks), the count of patterns that would treat keyless output as fully verified (`unsafe_defaults`, HC-05 count). Comparable-tool survey: primary-source documentation at pinned versions plus black-box exit statuses of installed tools (`gpg --verify` without the key, `sha256sum -c` with a missing file, `minisign -V` and `cosign verify` if downloaded under the approved download policy). Human interpretation is not claimed. |
| DEC-INT-038 | `status-quo-stale-on-failure`, `spec-invalid-on-failure`, `add-not-verifiable`, `default-require-none`, `default-require-content-current`, `default-require-all-present-current` | One archive per binding-status table row (valid; stale after `repack --index absent`, `repack --layout stream`, `key add`; invalid after a signature byte flip; not bound; absent), plus partial-read cases for `add-not-verifiable`. Conformance mismatches of the status quo against `docs/crypto-suite-v1.md` counted; default-policy candidates evaluated by the count of rows whose default exit status accepts a stale or invalid binding. |
| DEC-CRY-090 | `status-quo-three-dimensions`, `add-authorization-dimension`, `overall-verdict-enum`, `structured-json-only`, `explicit-no-signatures-line`, `verdict-words-require-policy`, `silent-when-no-signatures` | Conformance per dimension combination (cryptographic validity x binding x timestamp x authorization under a caller key policy file) and the census of reachable combinations. `silent-when-no-signatures` is checked against SPEC §17.3 L1891 as cited in the ledger ("no signatures present", never "verified"); proposed R1 exclusion pending sign-off, kept as a negative control. Simulated first-use evaluation belongs to the ecosystem domain. |
| DEC-INT-020, DEC-MOD-014 | as in EXP-INT-001 | Census of `EB_INTEGRITY_*` and other reason codes reached by the matrix faults; codes never reached by any case are listed (candidates for new conformance cases); detail fields per code. |

## 4. Corpus and matrix

- Items: INT-SMALL-T (tuning) for the full matrix; INT-SMALL-V for the validation look;
  `f17-tuning-duptree-mixed` and `f17-validation-duptree-vendor` added under the dense profile
  so that dictionaries and ChunkGroups exist.
- Matrix axes (pre-registered, every applicable cell):
  - operations: library `open`, `list`, `read_entry`, range read, `verify`, `unpack`;
    CLI `list`, `inspect --json` (plain, `--access`, `--security`, `--entries`), `read`
    (with `--access-report`), `verify` (plain, `--signatures`, `--signature <file>`), `unpack`,
    `diff --json` (plain, `--public`), `explain`; `salvage` (records `EB_CLI_NOT_IMPLEMENTED`
    until EXP-INT-012's prototype exists);
  - sources: Memory, LocalFile, HttpRange via `ebr-origin` on localhost (plain http), STREAM
    pipe (stdin);
  - archive types: unsigned INDEXED; unsigned STREAM; signed (content binding; plus
    `--bind-physical`; plus `--bind-addressing`); signed then stale (operations of H4);
    detached `.ebsig`; crypto-v1 with key; crypto-v1 without key; crypto-v1 signed;
  - faults: none; D-P1 in a requested chunk; D-P1 in an unrequested chunk; Index payload
    damage; Index header damage; FIDELITY damage; used dictionary or group damage; unused
    dictionary or group damage; footer truncation; signature record byte flip; stale binding.
- Requested entries: 40 seeded entries per (item, archive type, source) (§4.6 operation
  sampling), or all entries if fewer.

## 5. Environment and platform requirements

`wsl-ubuntu` for the full matrix; `windows-host` for the CLI subset (text, JSON, exit
status on LocalFile and HttpRange) because exit-status and console text semantics are
platform-sensitive. `timing: false`. `ebr-origin` on 127.0.0.1 (no netem; counts are exact).
Signing keys generated with `ebound key generate-signing` into the work directory; test
recipients generated by `ebr-pack --encrypt` conventions. No external services.

## 6. Procedure and commands

1. Commit `audit/flag-check-map.csv` (independent session) and
   `patterns/exit-status-scripts.txt` before any run.
2. `ebr run` of `spec.yaml` (driver `ebr-int-statematrix`): for each (item, archive type) it
   builds the archive, applies each fault, runs every operation on every source, captures
   reported state (library report structs serialized, CLI stdout, stderr, JSON, exit status)
   and the oracle state from T11(a) logs, and writes one detail row per cell.
3. `research/tools/integrity/state_census.py --experiment EXP-INT-003 --candidates
   research/harness/crates/ebr-integrity/proto/state_api/` computes expressibility per
   candidate; `cargo +1.98.1 test -p ebr-integrity --test state_api_compile_fail` runs the
   compile-fail tests.
4. Comparable-tool survey script `research/tools/integrity/exit_status_survey.sh` (pinned
   tool versions recorded) for DEC-INT-021.

## 7. Seeds, warmups, replication

Seeds 2026091721/22/23; entry selection by C8 derivation with `class_id` = operation name.
Warmups 0; 2 repetitions; repeat-hash on the cell outcome vector (`detail_sha256`). Crypto
archives are packed once per repetition; comparison is on state vectors, not bytes.

## 8. Metrics

| Metric | ID | OD | Role | Notes |
|---|---|---|---|---|
| `mvt15a_overclaim_cells` | C11 | HC-15 | binary (R1 for candidates; defect for status quo) | reported state stronger than oracle state |
| `verified_fraction` | T-20 | OD-11 | **primary** for DEC-INT-044 and DEC-ACC-035 | fraction of partial-read results in the strongest state the archive supports (objective M11.3) |
| `verified_range_fetch_bytes` | T-11 | OD-11 | secondary (would be a second OD-11 primary; needs reviewed justification) for `verify-more-in-random-reader` cost | exact, per logical operation |
| `http_requests` | T-12 | OD-12 | **primary** for DEC-INT-044 OD-12 cost | exact from origin log |
| `reason_code_specificity_fraction` | T-20 | OD-04 | **primary** for DEC-INT-020 | as EXP-INT-001 |
| `unsafe_defaults` | HC-05 | OD-25 floor | binary for DEC-INT-021 and DEC-INT-038 default policies | count of pre-registered script patterns accepting unverified or stale results |
| expressibility fraction per candidate | proposed T-20 coverage (C11 table if the review adds it) | OD-04 | secondary until named | must equal 1 |
| reachable combination count | none | - | descriptive | enumeration |
| compile-fail misuse patterns rejected | none | OD-24 | descriptive (cost and assurance input C4) | count |
| `normative_words_added`, `wire_items_added`, `flags_per_task` | C1, C6 | OD-23, OD-25 | cost inputs | checked-in counters |

## 9. Normalization

Coverage fractions and counts: exact per item; differences against the status quo candidate;
aggregation over the case set (AGG-C: per family minimum as headline). Byte and request costs:
paired log ratios per item against the status quo reader (T-11, T-12 floors).

## 10. Statistical analysis and sensitivity

Per C10; binary metrics decide R1; coverage fractions are exact over pre-registered cases
(no intervals needed at item level; corpus-level verdicts use §4.10 only for the banded byte
and request metrics). The INT-SMALL sets do not meet §4.9 minimum support for banded
corpus-level verdicts (C3), so `verified_range_fetch_bytes` and `http_requests` verdicts from
this experiment are secondary; the banded cost comparison is repeated on INT-CORE items by
the remote domain's access experiments, which this protocol cites as the confirmatory source
when available. Sensitivity: §6 where banded metrics are used; plus a **source arm** (Memory
versus LocalFile versus HttpRange reported separately; never pooled) and a **fault-mix arm**
(no-fault cells only versus all cells).

## 11. Expected negative results worth recording

- Overclaiming report booleans in the status quo (production defect, HC-15).
- Keyless verify exit 0 accepted by naive scripts.
- Binding-status vocabulary non-conformance to the suite table.
- Some reason codes unreachable by any constructed fault (dead codes or missing cases).

## 12. Threats to validity

- The oracle state relies on the T11(a) hook logging every check; a missed check makes the
  oracle weaker than reality and inflates overclaim counts. Mitigation: the hook is validated
  by showing that, on unfaulted archives, the oracle state for full `verify` equals
  whole-archive verification (all digests logged) before any matrix run.
- The black-box CLI subset on Windows cannot use the research hook; oracle states there are
  known by construction only.
- Expressibility census uses research schemas of API candidates written by this program; a
  better API within the same candidate could exist. Results bound the candidate as specified.
- Human comprehension of any output is not evaluated (external or simulated, §4.16).

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12.

## 14. Cost estimates

- Compute: about 2,600 applicable cells x 40 entries x 14 items x 2 repetitions, mostly
  sub-second operations on small items: about 25 CPU-hours (WSL) plus about 4 CPU-hours for the
  Windows CLI subset.
- Disk: about 10 GB scratch; detail about 1 GB.
- Agent effort: independent audit table (judgment, a session not involved in readers), tooling
  build scripted with review, execution none, census and conformance analysis judgment.

## 15. Deviations (append-only)

None.
