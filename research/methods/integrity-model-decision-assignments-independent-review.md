# Independent INT/MOD decision assignment review

Reviewer session: `qualification_audit/2026-10-01`. This accepts prospective initial assignments only. It does not accept candidates, experiment results, registrations, release gates or `DECIDED` states.

## Exact source pins

- `research/decision-ledger.jsonl` SHA-256 `7ceb7c8c4d934c3cb5747b1dc6965657b09551cbbd3ceadf7e2f1644c3d77d90`
- `research/methods/decision-assignments-proposal.jsonl` SHA-256 `479d877e5247e00be3f03ab7e8a29945c9969c899efafb9306d0076cfe2e7446`
- `research/decision-method.md` SHA-256 `880e319c8d16e778f856cdbc738ecda088865c4d5e2499665fe058228a29426b`
- `research/archetypal-objective.md` SHA-256 `f81627d0171931aec960baa1ce0e1999f8d81802d207480803c575c692b30036`
- `research/methods/thresholds.json` SHA-256 `9cd4b5be3486ae671c54987df2c5ee4f8650e77f464145fe48fe829fe93ddc90`
- `research/experiments/decision-coverage.csv` SHA-256 `1c87648f7f2e8394988e26f0ca3e41acf49718778a36cd68e495b81779aeb5e0`

Each record binds the exact raw proposal JSONL row and canonical ten-field reviewed assignment digest. All 47 INT and 50 MOD questions and required-evidence lists were examined individually. Protected corpus, fingerprints, result bodies and private progress files were not inspected; source decision statuses were not changed.

P = registered primary, N = reviewed no-primary reason for this question, B = missing canonical instrument/role/band. A B accepts initial routing but blocks a decision-bearing preregistration, registered look, first result and DECIDED until independent canonical metric registration. Empirical, human-facing and external routes remain required as listed; no measured outcome was used to choose a candidate here.

Totals: 60 P, 188 N, 13 B across 97 decisions; 13 decisions contain blockers.

## Row-by-row adjudication

### DEC-INT-001

Question: What rules govern base chains: single base versus multiple bases, maximum dependency depth (declared budget or caller policy), stale-base handling, and cycle detection?

Required evidence: Task 23 scenario tests for cycles, stale and multiple bases; Resource-exhaustion analysis of chain resolution; Chain-length data from backup tools (restic, borg, kopia, zfs send)

Review: Scenario tests must distinguish single versus multiple bases and bound transitive resolution using backup-chain length data.

Additional HC screens: HC-09, HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-17 N Base-chain depth, cycles and stale-base refusal are binary resource/safety rules; declaration tightness cannot choose chain semantics; OD-22 N Reader and resolver upkeep is a permanent dependency cost

Pins: source row `48e7c352c7d1d40a85656044ded03082a29422697159e571f6980029dfdcbc0c`; proposal row `1cde7560de2f8f397ac5ab8af94b7e1b1c0aa85866cf5779dee423620935a034`; reviewed assignment `92201453f41a101a7a3b209f57b534fd1f99ab0e9b142dbb105983c877b249ac`.

### DEC-INT-002

Question: Is corruption blast radius defined as at most k following chunks, the transitive dependency closure, or the whole ChunkGroup (and dictionary users), and how is it reported and bounded by profile parameters?

Required evidence: Logical bytes and entries unreadable per corrupt chunk across lookback 0/1/2/4/8 and dictionary use (program §8.5); Formal dependency-closure definition checked against implementation; Report accuracy tests for dependency-affected entries

Review: Per-corrupt-chunk unreadable logical bytes across lookback 0/1/2/4/8 and dictionaries directly measure blast radius; affected-entry reports must match the formal closure.

Additional HC screens: HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-14 P `blast_logical_bytes_p95`; OD-22 N Dependency-closure reporting complexity is a permanent implementation cost, not graded locality

Pins: source row `3aa520d5f87f59a0699ee5ca273ef62d4f355b2c3601a2026c7a1b83ba65f7c2`; proposal row `7e1a9c569aee998e663c77f6d1979ea63be35efc2432f337816d0cadd2634635`; reviewed assignment `7225dfc7fcc000986259add6a460fe613be88e2eaffe632afe1571bdba1743b9`.

### DEC-INT-003

Question: Should every CLI output (pack INDEXED, export, publish, repack, sidecar, in-place mutations) use one crash-safe transaction pattern (temp+sync+link or rename-over, platform atomic replace), and is leftover cleanup or recovery required?

Required evidence: Crash-injection tests between each step on NTFS and ext4; Platform API feasibility of atomic replace in safe Rust (program §21); Disk-full fault injection for every output path

Review: Crash and disk-full injection across all output verbs plus primary platform API feasibility determine a single safe transaction rule.

Additional HC screens: HC-13, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-20 N Atomic publication, leftover cleanup and report truth are binary migration/transaction conditions; OD-25 N CLI collision/error behavior needs workflow evidence rather than a diagnostic proxy

Pins: source row `0201c389ac8ed3756ef3494096f759ca92a8e5bf85ac7d99452f29896bc15ce7`; proposal row `1754bafef019cc0d5e45dc9ec2359cd87141049f577d847f2baf064586991bcf`; reviewed assignment `c37c8c25a68eed634107bf70e6c3537421db27bf36286fc74fcbaf991add5a71`.

### DEC-INT-004

Question: Is 'never overwrite, no --force' the frozen v1 CLI policy, or does v1 adopt SPEC's caller-selected --on-collision policy (default refuse, overwrite via parent-descriptor unlink with bounded retry)?

Required evidence: Usability evaluation of refuse-only outputs (program §33); TOCTOU analysis of overwrite implementations per platform (program §21)

Review: The SPEC collision option must be reconciled with an actual no-clobber/TOCTOU design and user evaluation.

Additional HC screens: HC-15. Required routes: EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-25 N Refuse-only versus explicit overwrite is a human workflow choice; simulated task success cannot serve as a selecting primary; OD-26 N Extra flags and migration steps are descriptive adoption context

Pins: source row `5ca04d40b09eae243b58ace4b9a9e0a6ef3e214d6d8ee847caa303508fe849c8`; proposal row `e014c66b0c96cff606b25b9b2640ad4345289a8bdffae9d577b38014b1f4b7e3`; reviewed assignment `4af47fd4a1bfc9b5e79e82683a52665179dc615a2cf12f3b0933fa4f6153cda3`.

### DEC-INT-005

Question: How do unpack, list, diff, export, salvage, repack and random access behave on Sidecar and Incremental archives: refuse, require caller-supplied bases, resolve External refs by policy, preserve references, or materialize into Complete archives?

Required evidence: Behaviour matrix with failure scenarios (missing, stale, mismatched base); Conformance cases per command and role; Usability evaluation of refusal messages (program §33)

Review: Build a per-command role matrix for missing, stale and mismatched bases; random-entry latency and legacy import acceptance do not answer the question.

Additional HC screens: HC-09, HC-12, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-14 N Missing and stale base effects are binary dependency-locality scenario outcomes; OD-20 N Sidecar/Incremental role and External-reference identity are binary conformance; OD-25 N Refusal-message comprehension requires human-facing evidence

Pins: source row `dca97f097a46a2de32dde2dacb07cc103f275739fdf72200d642e0e0d98e74b4`; proposal row `9639492ea07342bdeb3d884e2af2fce291c2bf985932905664f61a59cc96e2b1`; reviewed assignment `228f8fc9984084c272f715e8d7e9c16131841948f15f0f018fe8bccc3cae2e32`.

### DEC-INT-006

Question: Which damage classes and prevalence weights (bit flips, bursts, sector erasures, missing chunks, truncation, footer/Index/metadata/dependency corruption, accidental versus crafted) define v1 detection and localisation guarantees and the recovery evidence base?

Required evidence: Failure-mode prevalence evidence (storage field studies, download failure data, incident reports); Corpus generator with ground-truth damage maps; Incumbent comparison runs over the same damage corpus (program §35)

Review: A ground-truth generator and same-corpus incumbent runs must weight bit flips, erasures, truncation and crafted damage without equating specificity to prevalence.

Additional HC screens: HC-09. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-14 P `localization_precision`; OD-04 N Damage class taxonomy and code correctness are binary ground-truth checks; OD-22 N Incumbent damage handling and field-study provenance are external comparison context

Pins: source row `ff2d161caf708ef7f12ee06f2a55a9d7bee1e05b55e1c0568e2b124f7b26b3c2`; proposal row `b4bbbfdadb15a99ac28798b41ee382808c84373753ab5a71ca6b5af17c77b08d`; reviewed assignment `1acc5c2794ffe84091ffaf228ba8aef9bd8896709be493eb7190e8fb482f48c0`.

### DEC-INT-007

Question: Must v1 verify continue past the first failure to report damaged chunks, affected ContentObjects and Entries, dependency-affected entries and intact entries, or remain fail-fast with localisation delegated to salvage?

Required evidence: Fault-injection accuracy of reported sets versus ground truth; Resource amplification analysis of continue-on-error on hostile archives; Evidence of user need for damage maps versus repair (see ECC refusal decision)

Review: Continue-on-error can be selected only if affected/intact entry sets match injected truth and hostile amplification stays bounded.

Additional HC screens: HC-09, HC-17. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-14 P `localization_precision`; OD-17 P `refusal_alloc_peak_bytes`; OD-25 N Damage-map need versus repair needs human workflow evidence

Pins: source row `00fee8a8e5abe7754d3182f290fa3da63a6b8041425ddfdb8be783a38faceecc`; proposal row `9274af4eaaeff7316b4d19694ea607177ee5e4e1808bb6b8f53f8d659465c9bd`; reviewed assignment `c0ec6d9f3f0ebfb07ff210546bc25bdffb2152c90c3be8b61f955aa1ef91fee8`.

### DEC-INT-008

Question: Must stored codec payloads be canonical single frames with no trailing bytes, and should STREAM chunk frames carry a stored-bytes digest checked before decoding?

Required evidence: Crafted payloads with skippable/data frames and trailing bytes per codec; Fuzzing campaigns on decode paths under memory ceilings (program §30); Size and CPU overhead of stored-bytes digests in STREAM; Decoder CVE history review for attack-surface weighting

Review: Stored-byte digests require measured STREAM size/verify cost while crafted extra frames and trailing bytes remain mandatory refusals.

Additional HC screens: HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-05 P `artifact_bytes`; OD-07 P `verify_wall_s`; OD-04 N Single canonical codec frame and trailing-byte refusal are binary parser correctness; OD-24 N Decoder fuzz/CVE history is a security and permanent attack-surface screen

Pins: source row `af05fcf9bebe0b3220986b862fdb110cd22eb334da6d15814ca82fedff58f351`; proposal row `d8299138b49882861d8383171ec7f4493c3786a77e8e4bc8fc8671246afaf412`; reviewed assignment `d5e6605ecba7b5e8db82c799e7b248512e7b8fc8febdb06894f2b907652aa217`.

### DEC-INT-009

Question: Will encrypted Sidecar or Incremental archives be supported after v1, under which crypto version and key-domain model (per-archive AFK, shared domain across chains), and with what leakage from external plaintext digests?

Required evidence: Demand assessment for encrypted incremental archives; Leakage analysis of external plaintext digests and cross-archive dedup side channels; Key-domain design review in the crypto external-review dossier

Review: External plaintext digest confirmation and cross-archive dedup leakage must be measured and independently reviewed before any post-v1 encrypted chain design.

Additional HC screens: HC-05, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-16 P `presence_advantage`; OD-20 N AFK/key-domain and External-reference identity are binary crypto/model rules; OD-25 N Encrypted incremental demand needs actual workflow evidence

Pins: source row `2211eb251335a2592d94a187a68ad91678faa8506caae6fc6e9040536443c400`; proposal row `7339d4c112b1ce02c7895023f61996166afa92fee38a601a84f100de79585133`; reviewed assignment `c948d7566c52fdb6c153030a99817b1a66a68bbf857fd8294aefd9c3330368f7`.

### DEC-INT-010

Question: What verification status is reported for encrypted random reads that do not verify segment END and the full segment-sequence digest, and is incremental remote whole-archive verification (PCR/PCI via ranges) required in v1?

Required evidence: Adversarial corpus tampering unread segments (delete, duplicate, reorder, splice sibling versions) with outcome of requested reads; Formal model of partial-read guarantees under per-object authentication plus ArchiveFinal; Bytes, requests and time for remote whole-archive verification under application-level network emulation; Crypto external-review dossier coverage of partial-read splicing

Review: Tamper unread encrypted segments and model partial guarantees, then measure remote whole-verification transfer cost without claiming full status from a slice.

Additional HC screens: HC-09, HC-10. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-12 P `http_bytes`; OD-11 N Requested-range versus whole-archive verification status is an exact scoped guarantee; OD-15 N Unread-segment splice and ArchiveFinal coverage are binary authentication conditions

Pins: source row `60913cf540665bb40abc5deb0cf1f88f1e4b756196e744d984af7623b2374338`; proposal row `d6744150db77ada6d6b7b9bb86d84291f0c48b0ad4fb0541a07dd48a9b823b87`; reviewed assignment `ad44afcf56589fb160003081901aff9ff01a1a2e2c0ebae2a234f5ca52369446`.

### DEC-INT-011

Question: How is an external parity artifact bound to and discovered from an archive (PCI, per-section or per-chunk digests, LAI plus signature, filename convention, receipt, wrapper index), and how is the binding authenticated and kept from silent separation by copy tools?

Required evidence: Separation and mismatch detection through cp, rsync, zip, HTTP and object stores; Staleness of each binding under repack and Index rebuild; Authentication analysis of parity-induced substitution; Adoption workflow evaluation (program §32)

Review: Test cp, rsync, zip, HTTP and object-store separation, then require a cryptographically bound identity and discoverable workflow.

Additional HC screens: HC-10, HC-16. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-15 N Parity binding, staleness and substitution are binary authenticity floors; OD-26 N Discovery and separation through copy tools are workflow/adoption context

Pins: source row `f381b8a61efa153d8eb02db73cec6df626d527cb78bdf349eb8782dd40c97e95`; proposal row `bc5aad5bd4670fa2d05f7883f27169050ba9a3706eb1d4dfc8a178867ad538aa`; reviewed assignment `88455940e4809008bded6e9c88061c32111f795ce243f34d2c40051958c7972d`.

### DEC-INT-012

Question: Which external parity format(s) are normative for verify --parity and repair --parity: PAR2, PAR3 draft, RaptorQ, a custom Reed-Solomon/erasure format, or no normative format?

Required evidence: Re-verified maturity and maintenance status (2026) of each format; Rust implementation availability, licence and patent review (RaptorQ); Recovery-probability versus overhead curves under scattered, burst and sector damage; Fuzzing parity parsers and CVE history (program §30); Dependency audit classification (program §31)

Review: Normative parity format cannot be selected by reason-code specificity; compare true recovery-overhead curves with security and licence review.

Additional HC screens: HC-06, HC-09, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-05 N External parity artifact overhead is reported separately; native archive artifact_bytes does not measure the parity format payload; OD-22 N Licence, patent, specification maturity and decoder maintenance are permanent external screens; OD-24 N Parity parser fuzz/CVE history is a safety screen; OD-14 B Recovery probability by damage class at equal parity overhead lacks a canonical repair-effectiveness selection instrument, role and band; truncation_salvage_fraction only covers tail truncation

Pins: source row `2149f86b7fa45f086d8275733fec2e4180cc5c25ea92e2d3e7e9f058b33b3e12`; proposal row `32f1bfc4d4c3327205f93a1ee8d2856239d8caa57a5fd7c16e67674b86ddcd09`; reviewed assignment `ef4e6a8fac57d8b94c48b12b003b76418d3bded9aa8fc988d6a82f3b6a609b45`.

### DEC-INT-013

Question: What are External.locator syntax and semantics (none, advisory-only, typed relative path/URL/CAS URI, multiple locators), is the locator excluded from identity, and may readers dereference it?

Required evidence: Security review of locator resolution (path traversal, SSRF, redirects, privacy); Signature stability under mirror changes and renamed originals; Task 23 scenario results for renamed and remote originals

Review: Path traversal, SSRF, redirects and privacy require an explicit resolver policy; mirror changes must not silently alter identity.

Additional HC screens: HC-16. Required routes: EXTERNAL, FORMAL.

Dispositions: OD-01 N Locator authority and identity exclusion are binary model semantics; OD-15 N Digest verification and mirror substitution are binary trust conditions

Pins: source row `ccb4dcc68a82b642ca0230c30f6939f640f670b5867d6c1947d325f961081035`; proposal row `64d651b011bee6c6f25480d551934d31f4fe48210e5cc93228d23bedee94963b`; reviewed assignment `72e0a8d4cb4d0071214c7b29efa918a57de435c377947a5c30d1755c3e7a3471`.

### DEC-INT-014

Question: How are missing, renamed, moved, truncated or digest-mismatched external originals and bases detected and reported (outcome class, reason codes, per-entry status), and may unaffected entries still be extracted?

Required evidence: Task 23 scenario matrix outcomes for missing, renamed and stale originals; Reason-code taxonomy extension review (model cluster); Usability evaluation of partial-dependency failures

Review: Replay renamed, moved, truncated and digest-mismatched bases with per-entry status rather than a generic corruption result.

Additional HC screens: HC-09. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-04 N Missing, stale and mismatched originals require exact outcome class and reason codes; OD-15 N Unaffected-entry extraction must not overstate verification scope; OD-25 N Partial-dependency failure comprehension needs human evidence

Pins: source row `f84ec3ab9dc9eee5de2b90889a3c1d9e9e43c5e07abbf4021c37819095845f72`; proposal row `36429ec707ef7ef469648ecd6d8f27b51700965a54bd351e7f46d7c368287007`; reviewed assignment `e0984b12a5b6df61ca1e4c29e09f06ab5c0d78d33c206ab7150773f3d807501e`.

### DEC-INT-015

Question: Is refusal of in-archive error correction confirmed as a permanent non-goal by evidence (recovery-parser attack surface, recovery effectiveness versus external parity, user need for repair versus assessment), or should it be reclassified as deferred?

Required evidence: Primary-source survey of recovery-parser vulnerabilities (CVE ids, versions) in RAR, PAR and similar tools; Recovery effectiveness of in-format versus external parity at equal overhead under the damage model; User and incident evidence on repair versus assessment needs; Resolution of SPEC §24 deferred-versus-refused categorisation

Review: Permanent non-goal versus deferral needs primary vulnerability evidence, actual recovery effectiveness and a SPEC category disposition.

Additional HC screens: HC-09, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-24 N Recovery parser vulnerabilities and unresolved fuzz findings are safety/cost screens; OD-25 N Repair versus assessment demand requires human or incident evidence; OD-14 B Comparing in-format with external-parity recovery at equal overhead lacks a canonical damage-stratified repair-effectiveness instrument, role and band

Pins: source row `04ade4276d413755ed3541f9cdad0330bb6fb1a10aa5bde8efde74e64320c5d2`; proposal row `13c4135010cf595a415ed06305a23dd19f88cf99a1ede6d261e5ccdf22ca1cca`; reviewed assignment `a9b4c9792c9d6e2e983782aef8ca289a768cd600e49ac76a6e6d0b88bd787c10`.

### DEC-INT-016

Question: By which identity is an Incremental's base referenced (LAI, PCR, PCI, archive_id, per-content digests only, signed reference), what verification is required before extraction, and does the losslessness contract extend to Incremental extraction with all bases verified?

Required evidence: Robustness of each identity under repack, recompression, Index rebuild and rechunk; Base-substitution attack tests; Losslessness round-trip experiments for Incremental extraction; Precedent review: OCI ChainID ordered composition, zfs/btrfs send streams, restic/borg snapshots

Review: Test each candidate reference under repack, recompression and substitution before permitting extraction from a chain.

Additional HC screens: HC-05, HC-09, HC-14, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-02 N Lossless Incremental extraction with verified bases is a zero-mismatch binary condition; OD-15 N Base substitution under signed references is a binary authenticity screen; OD-20 N LAI/PCR/PCI/base-id survival under repack and rechunk is a binary identity rule

Pins: source row `f46b1961ef2755faaedc2278533c4e2667834f90299f174090d55dfafb022e1b`; proposal row `eab36f9d1d6952be53155996addad5a2fa63b9a9e5f57f8e408ad81fc033e791`; reviewed assignment `751bf50c5fe105ed0786a906d1a64562a75077f80f61350b86bdc52223fb08f5`.

### DEC-INT-017

Question: How does an Incremental archive represent entry deletion (tombstone entries, whiteouts, opaque directory markers, full manifests, anti-items, additive-only)?

Required evidence: Size and identity analysis of each representation on realistic change sets; Reserved-name conflict analysis; Extraction correctness tests with deletions across chains

Review: Realistic change sets measure representation size while reserved-name collisions and multi-chain deletions must be exact.

Additional HC screens: HC-02, HC-10. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-05 P `artifact_bytes`; OD-03 N Deletion and opaque-directory extraction correctness is a binary fidelity condition; OD-20 N Tombstone, whiteout and full-manifest identity behavior is binary migration conformance

Pins: source row `782ef29df8e4a99c7ede4a4a99447a1f18bd5362ff9877f6dc5550e493841acc`; proposal row `6d5e273c9e50786e30cb6abfb8607608f18cd1442b64a848afa0bca2de2d9c8e`; reviewed assignment `68899252ac6611c9478abee02fc0a3d655b9f141be5cfd281f1b64a73f6c9382`.

### DEC-INT-018

Question: Should a reader ignore a structurally damaged trailing INDEX section (header magic, version, flags, reserved bytes, overrunning length) and rebuild locators, instead of rejecting the archive?

Required evidence: Mutation experiment over Index header versus payload bytes with outcomes; Differential/security analysis of section-skipping for parser ambiguity; Consistency review against SPEC §4.5 and docs/format-v0.md

Review: Mutation of header and payload bytes must align SPEC §4.5 and the existing format note without accepting parser-skipping ambiguity.

Additional HC screens: HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-04 N INDEX header damage versus rebuild/refusal is a binary canonical classification, not a graded reason-code preference

Pins: source row `ea4837aa1fc6f5360e6c33701dd7e8e1da9f849565d7553b43573c31010c6685`; proposal row `f6c22aad984b8ac25ebf1a19daf2ff2a5b1b84e80046b8e252241384306830a0`; reviewed assignment `34728cc8bdaf632c3550ef5fae687c5a6365b6ac49308d3da0c049e57391eee6`.

### DEC-INT-019

Question: Should unencrypted INDEXED archives bind a digest over section headers and footer fields (footer self-checksum, section-directory digest, or whole-body digest) in addition to per-section payload digests?

Required evidence: Fault injection on headers, length fields and footer bytes measuring detection and localisation precision; Differential fuzzing for undetected reinterpretation (program §30); Overhead and format-revision cost of each option (container Task 11)

Review: Compare header/footer binding schemes on added bytes, verification time and fault-localisation precision; fuzz undetected reinterpretation.

Additional HC screens: HC-09, HC-10, HC-13. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-05 P `artifact_bytes`; OD-07 P `verify_wall_s`; OD-14 P `localization_precision`; OD-04 N Header/footer reinterpretation detection is a binary integrity and deterministic-interpretation floor

Pins: source row `321255c7d4c671f251814893b17ee6f65bb1a7e0e7d09d1b82feadf497a04b8b`; proposal row `cc4825735600f64e5e1f2a672365cccfa699d5bd79e8bda82be45e015086dc66`; reviewed assignment `ed9b5e4b578afa331cbeccc3b59f386a5395108e77426bb8da02acdf10ed72a7`.

### DEC-INT-020

Question: Which diagnostic detail fields (section, offset, chunk ID, expected and actual digest, affected entries) are normative for CORRUPT and TRUNCATED outcomes, and must every EB_INTEGRITY_* code have a conformance case?

Required evidence: Conformance cases for every EB_INTEGRITY_* code (ContentDigestMismatch and ChunkRootMismatch currently unasserted); Independent reader parity of codes and details; Privacy review of digest disclosure in diagnostics for encrypted archives

Review: Independent reader parity and presently unasserted ContentDigestMismatch/ChunkRootMismatch details govern the normative record.

Additional HC screens: HC-10. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-04 N Each CORRUPT/TRUNCATED detail and reason code needs an exact conformance case; OD-16 N Encrypted diagnostic digest disclosure requires a field-level privacy review

Pins: source row `86e5de21c5f07710c72dbe4da4da8f247a3b25d79d88a66975e5a884e586d3bc`; proposal row `49e1dbb5ca4a5b48fb88c0b6e7feb28584dfe501aab970b95dedbd3197350edc`; reviewed assignment `1012abe99b11430d40de94765d51fd895ab7c94fef37d97ce1b0b36e539b5f2c`.

### DEC-INT-021

Question: Should verify of an encrypted archive without unlock material exit 0 with 'PRIVATE CONTENT UNVERIFIED', exit with a distinct status, or refuse, and is keyless public verification a named level with defined guarantees for custodians?

Required evidence: Survey of CI and script handling of verification exit codes in comparable tools (gpg, cosign, minisign, sha256sum -c); Simulated first-use evaluation of keyless verify output (program §33); Compatibility review against the exit-code contract decision (ecosystem cluster)

Review: Survey script conventions and reconcile the ecosystem exit-code contract before freezing keyless verify behavior.

Additional HC screens: HC-10. Required routes: EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-15 N Keyless public checks and PRIVATE CONTENT UNVERIFIED are typed assurance states, not verified-content success; OD-25 N CLI exit/status interpretation requires first-use human evidence

Pins: source row `178aa8e93a157a52be7db5668e7342b7d0deda1dc6cc83d79d7ac9142f98f2c6`; proposal row `123f65bec7a8841e43bcd97e204dc6028ecc80c16e5d7f123df739af483e8003`; reviewed assignment `5d959a7ab650a8cbd6d93a83b3edc62f98863846438c541a8c7fcb2a834d0903`.

### DEC-INT-022

Question: Should strict legacy import require integrity checks that formats make optional (zstd content checksums, xz check types other than None), treat CRC-32 and declared-size agreement as safety invariants for every compatibility profile, or accept and report integrity-unverified layers?

Required evidence: Fixtures: checksum-less zstd with corrupted literals, xz check None/CRC32/SHA-256/BCJ/delta, CRC-collision ZIP through every profile; Prevalence of checksum-less zstd and check=None xz in real corpora; Legacy runtime differential results (program §25)

Review: Checksum-less zstd, xz check=None and ZIP CRC-collision fixtures require separate security and prevalence dispositions.

Additional HC screens: HC-03, HC-12. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-15 N Optional checksum and CRC collision acceptance are binary integrity-honesty constraints; OD-19 B Per-format checksum-less prevalence and disputed runtime outcomes need a canonical rule-specific selection instrument, role and band; aggregate import acceptance cannot decide each integrity rule

Pins: source row `5ab02d36c298c9c66e4e5adacc3f4363a25e0cfe35df6764816a42f1fe8b950f`; proposal row `cd95f8cc735d3cc1ed487c3c236a6956a4f3b920e3d347f538573f1e4f2d4cf2`; reviewed assignment `7b1f2706ec071b466730b0973a1c7596b3b1ac26eb7cb3fd0b1a026fc5571841`.

### DEC-INT-023

Question: How is the rule that a LOSSY export may differ from its source only where typed issues predict mechanically verified before publication?

Required evidence: Code review locating the predicted-difference comparator; Mutation tests injecting unpredicted differences into export encoders; Legacy export fidelity experiments (program §27)

Review: Mutation-inject unpredicted encoder differences and require refusal before publishing a LOSSY target.

Additional HC screens: HC-03, HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 N Incumbent export acceptance cannot prove every observed difference was predicted by a typed issue; OD-20 N Source-to-output comparator completeness and LOSSY receipt truth are binary migration conditions

Pins: source row `3dd6453e5428c3ed59a0e68b98c2e31c776abf394af000ba1f0a9a40a35a82c8`; proposal row `6ad2de7c45cd520d802eb7ffca2488374c13993ad325741990b0cb542d2a16b9`; reviewed assignment `58de01e530d5cd6ff189ffdb1121aaef436fb0416790afcae6e6fb735773c5ac`.

### DEC-INT-024

Question: Is the explicit SHA-256 tree (named domains, largest-power-of-two split, no padding) free of known Merkle pitfalls with chunk count bound where range readers need it, should it adopt RFC 9162 byte-prefix hashing for interoperability, and is rejecting BLAKE3's internal tree documented?

Required evidence: Independent recomputation vectors including odd-length and truncated leaf lists; Proof that subtrees cannot pose as leaves under the named-domain encoding; Interop needs assessment (COSE Receipts, transparency logs)

Review: Odd/truncated leaf vectors and subtree-as-leaf attempts test the SHA-256 tree before considering a different proof shape.

Additional HC screens: HC-01, HC-09, HC-12, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-01 N Named-domain Merkle leaf/subtree separation and count binding are binary hash-soundness properties; OD-11 N Range-proof shape and RFC 9162 compatibility are binary verification-format questions here; no proof-byte preference is in the required evidence; OD-21 N RFC 9162 interop and independent root reproduction are formal/external specification checks

Pins: source row `158bb36af5a3827e44a13e06a761d975edb54728653ffb6aec82bc30065718d3`; proposal row `893a31b62b2f458588f393fa7d5c6fd0e8866350ce707dd0fbd7daa492216fa7`; reviewed assignment `5da673caf18e386e67dc113c3f58d2cba97f16b9a4be53f23557abe09a13fec7`.

### DEC-INT-025

Question: Should inclusion or range proofs be exportable, and in which format (none, RFC 9942 COSE Receipts, custom encoding, Bao slices, in-toto/SCITT statements)?

Required evidence: Tree-shape compatibility with RFC 9162 hashing; Demand from supply-chain transparency workflows; Standards status re-verification

Review: Exportable proof scope needs standards re-verification and real consumers before freezing COSE, Bao or custom bytes.

Additional HC screens: HC-09, HC-15. Required routes: EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-11 N Proof format and tree-shape compatibility are binary verification-protocol properties, not a graded fetch preference; OD-26 N Supply-chain transparency workflow demand is descriptive adoption context

Pins: source row `d3ba14a746a7f3f85253463ea5d876f006380e57aa593d82876b8e8c9cd0498c`; proposal row `f4c8a409316ab3607a57e751c976096200514f165f8d940013c3585526cf3a88`; reviewed assignment `4496903bde5a304ff593bf9446dcd67ee87a983232bbd37b49d7618abf53f1d1`.

### DEC-INT-026

Question: May readers fetch external content automatically, only on explicit caller opt-in with digest verification, only from local relative paths, or via a caller-supplied resolver, and when is a fetch protocol for network-backed archives specified?

Required evidence: Threat model for fetched content (SSRF, privacy leakage, TOCTOU, hostile servers); Latency and bytes under application-level network emulation (program §12); Offline decode conformance run for Complete archives

Review: Automatic versus opt-in network fetching must survive SSRF/privacy/TOCTOU threat review and application-level latency measurement.

Additional HC screens: HC-10, HC-16. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-12 P `remote_task_latency_s`; OD-15 N Fetched External bytes require exact digest verification and explicit partial assurance; OD-26 N Complete archive offline decode is a binary self-containment floor

Pins: source row `1f70bf60741f5b84eee4d3af69c515a060ded1a1419490ab75f590c98f81e1f5`; proposal row `82aa1011f24f52e41bc1be478e0d3522be06a1464c3ae267afc9a1e3c27b5bda`; reviewed assignment `b5bedf18383beb3721489beb36c904cc12be64e087d57449dbcbb39a1f8d6e75`.

### DEC-INT-027

Question: What parity redundancy does Entrybound recommend or default to (none, fixed percentage, damage-model target, per-medium profile, rateless top-up policy)?

Required evidence: Recovery probability versus overhead curves per candidate format and damage model; Storage overhead and parity creation time on large archives; User workflow evaluation (program §32)

Review: Recommend no parity percentage until recovery-overhead curves, creation cost and workflow needs are measured without substituting reason-code specificity.

Additional HC screens: HC-06, HC-15, HC-18. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-05 N External parity artifact overhead is not native archive artifact_bytes and remains a reported cost; OD-25 N Default redundancy preference and top-up workflow require human evidence; OD-14 B Recovery probability for each damage model versus parity percentage lacks a canonical damage-stratified repair-selection instrument, role and band

Pins: source row `0a7c0c874a7832464acc8106c74a90a2ee935ce4a9aad412dc03d45f535f5f4d`; proposal row `72f6f5ff841d448a0b6c4a6174ecfd6bf2f2e976a7cce88157c863cc60193601`; reviewed assignment `9bf124454766cf6c57bce5c5a88e592fbd197f50568034a6bbede851a8fb8440`.

### DEC-INT-028

Question: What does verifying PCI mean without an embedded or signed reference, may users supply an expected PCI or container digest checked before any parsing, and is that required in v1?

Required evidence: Use-case survey of container-digest verification in package managers, mirrors and receipts; Cost of verify-by-hash versus verify-by-decode on large archives; Parser-differential analysis of verifying bytes before parsing; Staleness analysis of PCI-bound claims under Index rebuild and repack

Review: Survey custodial expected-digest workflows and compare hash-before-parse cost and stale PCI claims under repack/Index rebuild.

Additional HC screens: HC-10, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-07 P `verify_wall_s`; OD-15 N PCI without an expected reference is only a computed container digest, not authenticated expected identity; OD-24 N Pre-parse hash versus decode parser exposure is a security cost screen

Pins: source row `9d64ad8ad73d97fc85aa6ad277797ba86734d07e9e20e2904b9de89df5f81bb5`; proposal row `bb9b538db0538085e4bc0ceacf8ff9872b641e117a3499ed95a762f8b0cb118a`; reviewed assignment `553a925d65a4ff4184b7e7076910d3e9b2be5a44c9b34ffe84a34e832e73dd75`.

### DEC-INT-029

Question: Which byte-range verification structure does v1 use (status quo flat per-chunk digests with per-object leaf lists, MERKLE_OUTBOARD balanced tree, per-object outboards for large objects, sister-list DAG, two-stage index-then-chunk, BLAKE3/Bao encoding), and which structure is authoritative if flat digests and a tree disagree?

Required evidence: Bytes, requests, RAM and CPU per verified range at 10^6-10^9 chunks for each structure (program §11, §12); Proof size and cacheability under signature; Disagreement corpus (flat versus tree) with reader outcomes; Specification and implementation complexity cost

Review: At 10^6-10^9 chunks compare range bytes, requests, RAM and CPU while forbidding two conflicting authentic authorities.

Additional HC screens: HC-06, HC-14, HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-08 P `alloc_peak_bytes`; OD-11 P `verified_range_fetch_bytes`; OD-12 P `http_requests`; OD-04 N Flat-digest versus tree disagreement must have one declared authority and binary refusal outcome; OD-15 N Signature and proof binding are binary trust conditions

Pins: source row `56ebd592655f15c6a6142584d87a6a614b9d4126ca568410f650f302de8a048d`; proposal row `258cb92775c767c8beb785b3f07f51d80b724707ee1dc9faeac366a6ca872658`; reviewed assignment `b8b21bbcce79356170a5497322a474f25ea08f065616e658f5037094a9a200a6`.

### DEC-INT-030

Question: Must registered reconstructive transforms carry test vectors or differential evidence proving that independent decoders reproduce identical original bytes, beyond the writer's own-decoder round trip?

Required evidence: Differential decoding with independent preflate and JPEG XL implementations; Vectors across builds and platforms (x86_64, emulated ARM64); Real-world corpus success and savings rates (program §8.8)

Review: Independent preflate/JPEG XL decoding across builds must precede any savings claim from reconstructive transforms.

Additional HC screens: HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-05 P `artifact_bytes`; OD-02 N Independent reconstruction of original bytes is a zero-mismatch losslessness gate; OD-21 N Clean-room decoder and vector reproduction are binary implementability checks

Pins: source row `79d34660b8fc41ccc7ba0e170723562f99d4c3d34887b68af1ed57575ca785e8`; proposal row `475ed3976b0660d04ed15ea8e76eac22dd43118df3d195c510385aac64aa78b9`; reviewed assignment `7b7463e577989698f74acdc258df62462b448e5d225a9c6844dc134b32cc9425`.

### DEC-INT-031

Question: What does repair produce and claim: a new --out file versus in-place atomic replacement, mandatory full re-verification of repaired bytes against authenticated digests, reporting of unrepaired damage, and dry-run assessment?

Required evidence: Wrong-reconstruction injection tests confirming refusal; Crash injection for in-place variants on NTFS and ext4; Repair success rates under the damage model; Parity-decoder fuzzing

Review: Wrong-reconstruction injection, NTFS/ext4 crash cuts and parity fuzzing govern repair output semantics.

Additional HC screens: HC-09, HC-13. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-15 N Repaired bytes require full authenticated digest verification and truthful unrepaired status; OD-20 N New-output versus in-place commit behavior is a binary crash-consistency rule; OD-14 B Repair success across the declared mixed damage model lacks a canonical damage-stratified repair-effectiveness instrument, role and band; tail-only truncation_salvage_fraction cannot substitute

Pins: source row `c0374ba7f1d2ccb17e83a204eb091cc3a6711e06f2b8d20a272d449cd2be8aed`; proposal row `2cd7f543f4967dbff9f580d7336a2637bf2e74b90daac14446bcb2ba65147117`; reviewed assignment `874afc0c2cf20d045f21e67be152738ebb20e798b702c09625f78f170ac2590e`.

### DEC-INT-032

Question: Is role participation in LAI, plus ContentRef kind in entry identity, sufficient against content-stripping and role-confusion attacks on signed archives, or must signatures or identities also bind base identities and sidecar describes_lai claims?

Required evidence: Attack tests converting Complete archives to Sidecar/Incremental once roles exist; Formal model of signature coverage over LAI fields and ContentRef kinds; Independent security review

Review: Run role-confusion attacks once roles exist and have independent security review of signature coverage.

Additional HC screens: HC-01, HC-05, HC-14, HC-15. Required routes: EXTERNAL, FORMAL.

Dispositions: OD-15 N Role stripping and sidecar/base-claim substitution are binary signature-authentication threats; OD-20 N Role and ContentRef identity binding are binary migration/model guarantees

Pins: source row `b13eecac7b8b66630a4fad4febb838d0f7ba519adcfc4dafc37b8e2ae4be5bea`; proposal row `9d5741340f31c7dbf748be109d0868d114257ced3a52344b9210b4a3c304aa73`; reviewed assignment `b745edd137b6219b00659d19e97fc86a7a77b3699bc5f0b9a7a1e3cf7c32fe18`.

### DEC-INT-033

Question: What salvage guarantees apply to damaged encrypted archives: fail closed, recover individually authenticated records with keys, require intact commitment and envelope, or rely on external parity?

Required evidence: Corruption injection per encrypted structure measuring recoverable entries; Security analysis that salvaged authenticated records cannot be spliced or truncated into misleading output; Crypto external-review dossier input on salvage semantics

Review: Corrupt each envelope, segment, commitment and payload region and require crypto review of the exact salvage claim.

Additional HC screens: HC-09. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-15 N No spliced or truncated unauthenticated plaintext may be reported intact; OD-14 B Recoverable individually authenticated records under mixed encrypted corruption need a canonical authenticated-salvage instrument, role and band; tail truncation salvage is narrower

Pins: source row `699d699ae2f5852805ca68d76ddd33e1ff2b6d67321e830b26acea6a207dfbc9`; proposal row `e62ee29ef7aff0da73bd84b0892884c85b02ea7fb61d1228cf467eada21c8495`; reviewed assignment `defb63a1841aa30815d92c5e57f95ec7fe7e87ecdb3f34e3c882601c45ae7dd8`.

### DEC-INT-034

Question: Which in-band unverified marker is used per destination type for salvage output (report only, filename suffix, xattr/ADS, marker manifest, quarantine layout, new archive flag), and how is it protected from being dropped?

Required evidence: Platform feasibility on ext4, NTFS and (PLATFORM_BLOCKED) APFS (program §21); Marker survival through cp, rsync, zip, cloud sync and archive re-packing; Simulated first-use evaluation of marker visibility

Review: Choose report, suffix, xattr/ADS, manifest or quarantine only after copy-tool survival and comprehension tests.

Additional HC screens: HC-08, HC-10. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-18 N Marker survival on NTFS/ext4 and blocked APFS is a binary platform capability and propagation claim; OD-20 N An unverified marker must remain identity/report truthful through copy and repack; OD-25 N Visibility of the marker requires human first-use evidence

Pins: source row `e78bb73321703e0d53b4fa354f8c5ab1fcc133c7c9f35874b35328b0d5f22857`; proposal row `fe22980ea95a1e528b71fa449821e96558eb1e419b03bfd03cbac2830ee81d47`; reviewed assignment `52fca1b050b36ac53bbcf1a16affc3d5c28f2e7e09dc73a1ea4636ef1b4cc35d`.

### DEC-INT-035

Question: Does salvage ship in v1, what does it recover (intact verified entries, best-effort chunk carving, legacy local-header scans), what happens without --best-effort, and how does its report state what was lost?

Required evidence: Damaged-archive corpora measuring recovery rate and false-recovery rate (must be zero for entries reported intact); Comparison with zip -FF, 7-Zip and RAR repair; Fuzzing of the salvage parser (program §30); CLI complexity and stable-v1 disposition review (programs §33, §34)

Review: Compare intact-entry recovery against damaged corpora and incumbents; false intact recovery must remain zero.

Additional HC screens: HC-06, HC-09. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-19 N Legacy local-header carving is a bounded salvage capability, not graded import acceptance; OD-24 N Salvage parser fuzz findings are a security floor; OD-25 N Best-effort CLI complexity and repair demand require human-facing evidence; OD-14 B Damage-stratified recovery and false-recovery rates for intact-entry claims lack a canonical salvage-selection instrument, role and band; blast radius measures unreadable bytes, not recovery success

Pins: source row `21dd52ec31aafd846ec5dd1ed0981d53df9919d857e3a29e25d3e97ac1c0d47e`; proposal row `42ff40e6a19f0f0569ab3c6a1535c78504cf001b7d1d9d51ad4a7dd8ff33b568`; reviewed assignment `08720b6106594326c2ee9dcbe1879a7c41df20ea4b47a572414ccfed667ea257`.

### DEC-INT-036

Question: Should v1 sidecars be content-free role=Sidecar archives with External refs (SPEC §15.4), content-bearing Complete archives with exact-source digest binding (repo status quo), both, or non-archive digest manifests?

Required evidence: Sidecar size overhead versus source for content-bearing and content-free designs; Verification workflow tests with modified, renamed and truncated legacy files; Adoption workflow evaluation of sidecar verification (program §32)

Review: Measure sidecar overhead and modified/renamed/truncated source behavior before resolving the SPEC versus repository role conflict.

Additional HC screens: HC-02, HC-05, HC-15, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-05 P `artifact_bytes`; OD-20 N Content-free role=Sidecar versus content-bearing Complete binding is binary identity/verification truth; OD-25 N Verification and discovery workflow need human evidence

Pins: source row `c5485c5b06264ce296cc8861fb6548b3cc3083dfe69e0700cfbfae3338c35973`; proposal row `8d929939a3b5c4e19e4feb07ac4189ca6b191c5f49f97e777e8d7c2a4f1d1f50`; reviewed assignment `04690b95db1f8b25c68f805fcde5928d3da31f7af3e3609283e0532698dfefbe`.

### DEC-INT-037

Question: Which non-Complete capabilities (Sidecar role, Incremental role, External ContentRef, base chains, network-backed references) are in stable v1, deferred post-v1, or non-goals, and where is the boundary with repository and backup semantics recorded?

Required evidence: Results of the Task 23 scenario matrix (missing, renamed, stale, multiple and cyclic bases); Specification and implementation complexity cost of each option (normative words, record types, LoC); Demand evidence from adoption workflows (program §32); Impact on conformance corpus and independent reader (programs §28, §29)

Review: The stable-v1 boundary must reconcile Task 23 failures, cost, reader scope and repository/backup non-goals.

Additional HC screens: HC-05, HC-09, HC-12, HC-15, HC-16. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-01 N Sidecar/Incremental/External role semantics are binary model coherence; OD-12 N Network-backed references require caller-controlled resolution and bounded remote trust, not an unspecified latency primary; OD-20 N Base-chain identities are binary migration rules; OD-23 N Normative words, records and independent-reader burden are permanent R10 costs; OD-25 N Adoption workflow demand requires human evidence

Pins: source row `5267a5650c294dc52bbe59af8754cfc5f547bc1aba036dc4123fb41ed1d3cb37`; proposal row `f511a44790a971ccc41e8a33797c014032892a61d36bc87b76ae0a8f5d46c98d`; reviewed assignment `58f1f0f3b8cc67b15922787c11944c8669047f6ea7f9ad93678b4617901e0992`.

### DEC-INT-038

Question: What is the complete signature status vocabulary (cryptographic validity, per-binding VALID/STALE/INVALID/ABSENT/NOT_BOUND), what evidence distinguishes STALE from INVALID, and what stale-binding policy is the CLI default?

Required evidence: Conformance tests per binding-status table row (only add-recipient currently tested); Simulated first-use evaluation of STALE versus INVALID (program §33); Signature/trust composition experiments (program §22.4)

Review: Complete each signature binding-state conformance row before choosing a CLI default for stale bindings.

Additional HC screens: HC-10. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-15 N VALID/STALE/INVALID/ABSENT/NOT_BOUND are binary per-binding truth states; OD-25 N STALE versus INVALID comprehension needs human evidence; error_actionability_fraction only measures refusal remedies

Pins: source row `27f1c3c4c8418dd38ce3fd67f8b602d4135f85310d71913af5bc1322b113f828`; proposal row `8f4f54b20d9097105ad91e6a7e753a23dcf80462fc6378be6b77ea876b239316`; reviewed assignment `df5e6b051e37eff87f785c6d4eb46781563cb940bfa418730d4c3e974bd4a48e`.

### DEC-INT-039

Question: Does extraction always stage until full-archive verification, or may streaming extraction materialize entries once their own chunks and content digest verify, reporting archive-level roots as pending or unverified?

Required evidence: Staging disk and memory cost versus archive size for STREAM and encrypted archives (program §13); Time-to-first-file and failure cleanup tests for each candidate; Security analysis of per-entry streaming given STREAM emits Entry records after chunk data; Platform feasibility of atomic tree moves (program §21)

Review: Measure staging disk/memory and time-to-first-file, then prove pending roots cannot be represented as fully verified output.

Additional HC screens: HC-06, HC-09, HC-13. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-07 P `first_entry_latency_s`; OD-08 P `scratch_peak_bytes`; OD-09 N Per-entry early output is a binary streaming capability; OD-15 N Full-archive versus per-entry assurance and cleanup are binary safety claims

Pins: source row `299ceb6c39407d02f96a3529ea498c1efa7410c0f1653042d3600c3b22c83dc8`; proposal row `4cb5b2ae8b07fbb34e23a1ab88409a89326ba7e8dff554787d19474fea02b438`; reviewed assignment `0c2a55dbb64ba1289fe7c72666c2b84e43be85f6c9e91d58f358deb778059271`.

### DEC-INT-040

Question: What exact state machine and precedence define TRUNCATED, CORRUPT, UNSUPPORTED and NONCONFORMING across INDEXED, STREAM and encrypted layouts, including cuts at item boundaries, preamble truncation codes, record-level truncation, appended bytes and concatenated archives?

Required evidence: Truncation-at-every-offset and item-boundary corpora for INDEXED, STREAM and encrypted layouts with outcome tables; Independent reader differential over the same corpus (program §28); Conformance corpus expectation classes for ambiguous cases (program §29); Review of xz and zstd truncation classification as incumbent precedent

Review: Offset-by-offset and boundary cuts across INDEXED, STREAM and encrypted layouts need identical expected class/code tables.

Additional HC screens: HC-01, HC-10. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-04 N TRUNCATED/CORRUPT/UNSUPPORTED/NONCONFORMING precedence is an exact state-machine and cross-reader conformance rule

Pins: source row `ac9de37a97a4ebdb13ab300c4a09f9f13dc79918164e70f6820a82c71576f898`; proposal row `3b9d0011b0058065622cd401248357bca3187feb71a1198cd5236f6ed210a286`; reviewed assignment `03a463be561e94d0dcfcaca1aa83f648fb22ab7299bc46e1b2cd4455e88ddc2e`.

### DEC-INT-041

Question: What mechanism lets a reader report 'truncated at N of M bytes, entries 1..k intact' when tail truncation removed the footer carrying total length and manifest locators?

Required evidence: Truncation corpus measuring recoverable and reportable information per mechanism; Size and CPU overhead of sync records on large archives; Writer feasibility for STREAM and non-seekable outputs; Failure-mode prevalence evidence weighting truncation (see damage-model decision)

Review: Sync-record alternatives need a truncation corpus and measured byte/CPU overhead, including non-seekable STREAM feasibility.

Additional HC screens: HC-09, HC-13. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-05 P `artifact_bytes`; OD-07 P `verify_wall_s`; OD-04 N Reporting N of M bytes without a surviving footer is a binary information-availability claim; reason-code specificity is not the missing total length

Pins: source row `d2192fb1b8810392de0a84b0c8ef7674c75542200e36a12205e468e8e68fe022`; proposal row `3b850751dcd1a3ac94a2ab15b3dee509cdb010d91c7b98e136c08cd8454e8459`; reviewed assignment `b52153e3c9ec314ca99d501c55bbae5f2688bc63c3584ea2e280c80cdd61b7cc`.

### DEC-INT-042

Question: How is verification state carried end to end: a typed per-result state (chunk digest, signed-PCR Merkle slice, keyless public, none) in the library API, an UNVERIFIED qualifier on OK diagnostics, and matching CLI text, JSON and exit semantics?

Required evidence: API design review with compile-fail tests showing unverified bytes cannot be passed where verified bytes are required; Conformance matrix of every command and API against each verification scope asserting reported state equals checks performed; Simulated first-use evaluation of CLI output and exit-status interpretation (program §33); Independent reader implements the state model from the written spec without ambiguity (program §28)

Review: Compile-fail types and a full command/API scope matrix must prevent unverified bytes from being passed as verified.

Additional HC screens: HC-10, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-15 N Typed per-result verification scope is a binary assurance invariant; OD-21 N Independent-reader implementation of the state model is a clean-room conformance gate; OD-25 N CLI/JSON/exit interpretation needs human evidence

Pins: source row `d4a5e2eda9a137b001c2a6b88ad37f7d6aa1722fdc43f64faeb079c384927d2a`; proposal row `ca2ecec6cf98e2110d9656c958991d378ad4e0d397ea3340a9e63235508dcc2f`; reviewed assignment `1a49850d4c2069f4f3d6edeb2b678530436638273f6b1b08cc3ea0006e231a88`.

### DEC-INT-043

Question: Which verification levels and modes ship in v1 (single full decode verify; structure-only; stored-digest-without-decode; --deep; unpack --verify-only; library verify without materialization; keyless public checks), and what does each guarantee for plaintext and encrypted archives?

Required evidence: Time, RSS and bytes read for each level on large corpora (program §13); Fault-injection detection coverage per level across damage classes; CLI complexity evaluation of adding levels (program §33); Review of incumbent test modes (unzip -t, 7z t, zstd -t, restic check --read-data) for user expectations

Review: Fault-injection coverage and large-corpus time/RSS must accompany exact plaintext/encrypted guarantee wording.

Additional HC screens: HC-09, HC-10. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-07 P `verify_wall_s`; OD-08 P `alloc_peak_bytes`; OD-25 N Adding levels and flags is a human workflow/complexity decision; OD-04 B Detection coverage by verification level and damage class lacks a canonical level-specific coverage selection instrument, role and band; reason_code_specificity_fraction measures a different outcome

Pins: source row `f986be396da2fdecd012a6ae6c7a56f17e7800f1981ed81e6f0a1cde1eb399ea`; proposal row `f3a2bf0c64a5107f80d93fcfda4fdb6e6fd6f63e16d219e928089d0e437e941c`; reviewed assignment `d9e26df0baa12a6276bb8606fa9a0622f97d9a94f8d63d74c1aad3446afa844f`.

### DEC-INT-044

Question: Must verification report flags be scoped to the checks and objects actually exercised (tri-state per check), including FIDELITY/AUX for random reads and dictionaries/groups actually used, and must inspection types encode verification scope and role?

Required evidence: Code audit mapping every flag to the check that sets it, for INDEXED, STREAM, random and encrypted readers; Mutation tests corrupting unread FIDELITY, dictionaries and groups during random reads, recording report output; Bytes and range requests added by verifying FIDELITY/AUX in random reads over HTTP; API review of OpenedArchive construction paths

Review: Corrupt unread metadata during random reads and measure the extra HTTP bytes/ranges if full AUX checking is required.

Additional HC screens: HC-09, HC-10, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-12 P `http_bytes`; OD-04 N Per-check tri-state report equality to work performed is binary cross-reader correctness; OD-15 N Unread FIDELITY/AUX or dependency claims must never exceed authenticated scope

Pins: source row `4a955d33df59b7b50267543ba74ae105eba16289095f7ce621b90f719cfe73ef`; proposal row `4824e85dd111bd7165300d135542bfba01e5352e5d4074e309eb107774620662`; reviewed assignment `4d884bf7308594c21b8c94b575496ceba107783e9eb2990a541ab5a9feb1a7e4`.

### DEC-INT-045

Question: Should verify accept --against <dir|archive> and legacy artifacts with --sidecar, what equality does each assert (LAI, AUX tiers, per-entry bytes, exact source digest), and how does this differ from diff?

Required evidence: Workflow tests with modified, renamed, extra and truncated files and legacy artifacts; Cost of re-import versus digest comparison on large legacy artifacts; CLI duplication analysis with diff (program §33) and adoption workflow evaluation (program §32)

Review: Modified, renamed, extra and truncated targets plus measured legacy re-import cost define each comparison mode.

Additional HC screens: HC-10, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-19 P `decode_wall_s`; OD-20 N LAI/AUX/entry byte/exact-source equality for each --against mode is binary identity truth; OD-25 N Verify-versus-diff duplication and adoption demand need human workflow evidence

Pins: source row `958ea5744c4da40aa6ee5577496011df082b6049c601f509181604d00020f2af`; proposal row `c28e9faa9b08910ca120dcc2b7d6e0ee0dd72ef2d6a0cc276d4539669d7e8ee1`; reviewed assignment `4759da3c6fe155db57c92e08e8586ef5e6af644ee0403d5bae93864297e38f65`.

### DEC-INT-046

Question: Does verify --reproducible ship, how is adaptive planning recorded in the archive, and what does reproduction verification re-plan and compare (PCR, PCI)?

Required evidence: Cross-platform and cross-thread determinism runs (program §14); Re-planning cost on large corpora; Release-engineering workflow evaluation (program §32)

Review: Cross-thread/platform determinism must show which adaptive planner fields are recorded and exactly which roots are compared.

Additional HC screens: HC-03, HC-11, HC-15, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-25 N Release-engineering use needs human workflow evidence; OD-20 B Replanning cost and PCR/PCI reproduction in verify --reproducible lack a canonical replan-verification cost instrument, role and band; generic encode_wall_s is not a plan-only verification metric

Pins: source row `c51c3a09433ce2d42b562c3b6d3ce1ba4667b54bf2cabe278596a24a617d74f5`; proposal row `84fbb81dbd48175428665b5b397c26ea7bde0c02979d145f39e63f88e065f5af`; reviewed assignment `bbabd9583a5a56121f0a55f5e79555153f76c00f3704f5caf11cb8a7ce929ae5`.

### DEC-INT-047

Question: Under what conditions would a crash-safe versioned-journal/append extension be designed (zpaq-style header-last commit, transactional updates), given append-in-place is rejected and Incremental archives exist?

Required evidence: Demand evidence for versioned archives versus incremental chains; Crash-safety study of journaling commit protocols; Complexity and identity-semantics cost analysis

Review: Keep append-in-place rejected until demand, crash protocol and identity costs justify a separate versioned extension.

Additional HC screens: HC-05, HC-10, HC-18. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-20 N Versioned-journal commit and LAI/PCR effects are binary crash/identity rules; OD-23 N Journal grammar and implementation size are permanent complexity costs; OD-26 N Versioned-archive versus Incremental demand is descriptive workflow context

Pins: source row `075c5df152a13d8b8146aba1ab99128a02dff37007ab02871c329b0e8b6bece0`; proposal row `cd51587427c95789c331622f40a15e34995a7bbaaa7682e7c9973f9e36590395`; reviewed assignment `22bf23f1895e36bde6d0b82135e5c4f25c2aee279d46b96749bc341800ac58e0`.

### DEC-MOD-001

Question: Which archive roles (Complete, Sidecar, Incremental) are in stable v1, and how is role bound into identity and verified?

Required evidence: Task 23 evidence: sidecar and incremental use cases, locator security, base-chain verification; Property test LAI(Sidecar of X) != LAI(X); Stable-v1 disposition review (Task 34)

Review: The role list must reconcile Task 23 chains and require LAI(Sidecar of X) to differ from LAI(X).

Additional HC screens: HC-01, HC-02, HC-09. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-20 N Role participation in LAI and verified Sidecar/Incremental base binding are binary identity rules; OD-25 N Use-case need is human workflow context

Pins: source row `f48bb72c6ab9cd63387adfc3beaddf4e13be366e8f129b4e832427fbd678b050`; proposal row `46439ee92aa9e11c7c8bc2e4621e5bb29bacd8120c3e0dca91e570df60f62fea`; reviewed assignment `e23d91338125a0bcf7fc6ddd208aa9a3a0d2fa4e62d3decfc570f8e847b73fd2`.

### DEC-MOD-002

Question: Should AUX avoid host-dependent inputs (sparse maps from allocation state, fidelity platform/filesystem strings, Windows creation times) in deterministic mode, and should duplicate fidelity/conversion declarations be rejected rather than silently collapsed?

Required evidence: Pack identical trees on ext4, XFS, btrfs, tmpfs, NTFS, WSL and after cp --sparse variants; compare AUX roots; Privacy review of fidelity fields in unencrypted archives; Count of AUX differences attributable to each host-dependent field

Review: Compare identical trees on ext4/XFS/btrfs/tmpfs/NTFS/WSL and sparse-copy variants, with privacy review of emitted fields; declaration tightness is unrelated.

Additional HC screens: HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-01 N Duplicate fidelity/conversion declarations and host-dependent AUX fields are binary authority/canonicality rules; OD-20 N Cross-filesystem AUX stability in deterministic mode is an identity conformance condition

Pins: source row `3b2c3a8b3799080cea10c7f72c80119f33d308ae779823e41be82c289b49c56c`; proposal row `96c6d962881c5b40487c36d2228c2e0ceb1c1f3dc0ac7fed103fefb16f9e6207`; reviewed assignment `78c8782df2ee37b3779e50838529f3a928b803e8192efb83b9d7dc0132c10d3f`.

### DEC-MOD-003

Question: What are the canonical rules for edge cases the docs leave open: empty LinkTarget validity and extraction, and whether an empty Entry-v2 metadata sequence (tag 6) may be omitted?

Required evidence: Implementation behaviour audit and negative vectors for each case; Platform behaviour for empty symlink targets

Review: Negative vectors and platform behavior determine whether omission is legal without changing frozen wire meaning silently.

Additional HC screens: HC-04, HC-17. Required routes: EXTERNAL, FORMAL.

Dispositions: OD-01 N Empty LinkTarget and empty Entry-v2 metadata record are binary canonical grammar and extraction-validity cases

Pins: source row `6ff86f160f21a7685cb12e7774812a14ca124effe6d70a8231834806bffcd796`; proposal row `6e55dd131dbc1f2bbfbc9fde9a7e329f95f1341b774d6febb3e67c06bc9d8322`; reviewed assignment `90401d0a8dc5285c246c617fa557bb83920364b66ba2916af508b69abfd709f9`.

### DEC-MOD-004

Question: Is the frozen canonical LogicalPath comparator (component by component, byte length first, then bytes) the right order for identity/v1, given alternatives and the encrypted EBCS sort-key discrepancy?

Required evidence: STREAM producer memory/buffering needed to emit canonical order for large trees per candidate (Task 13); Diff/lookup locality: cost of prefix queries and incremental tree diff per order; Equivalence test between EAM order and EBCS manifest sort key for mixed encodings; Independent reimplementation error study

Review: Measure large-tree STREAM buffering and locality before any comparator change, with exact mixed-encoding equivalence vectors.

Additional HC screens: HC-06, HC-10. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-08 P `alloc_peak_bytes`; OD-01 N LogicalPath byte-length ordering and EBCS sort-key agreement are binary identity/canonicality requirements; OD-21 N Independent sort implementation is a binary clean-room check; OD-10 B Prefix-query and incremental-diff locality by path order lack a canonical path-order lookup instrument, role and band; random_entry_latency_s measures content access instead

Pins: source row `3fcc07a57bcd695629a848fbbabc55dd9f1360891d1efada4ef3853967713c83`; proposal row `c293b2075c091ca74c820cc432bd84c90428eb07392e9210dfe67ec7ad35c17c`; reviewed assignment `31088fd3a2a5578722b3efea245d40531d7e7deb82de0bc67136f8a1b993d971`.

### DEC-MOD-005

Question: Should Entrybound ship canonical metadata normalization profiles (R1 strict-canonical/portable/preserve), an mtree-style validator keyword vocabulary (R2), SOURCE_DATE_EPOCH clamping with order-preserving time normalization, or rely solely on identity profiles (SPEC)?

Required evidence: Consumer study with reproducible-builds ecosystems on metadata keep/collapse/drop; Survey of consumers depending on relative mtime ordering; Measure whether identity/v1 alone satisfies reproducible-build comparison use cases on the corpus

Review: Corpus comparison must test whether identity/v1 serves the use case rather than invent a generic normalisation success metric.

Additional HC screens: HC-10, HC-13. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-01 N Metadata normalisation and validator vocabulary must declare one authority and exact identity effect; OD-20 N SOURCE_DATE_EPOCH clamping and relative-mtime ordering are binary reproducibility rules; OD-25 N Reproducible-build consumer preference requires human evidence

Pins: source row `71659047d43c5e542f72bb6a0a5fe286ee66809efa4ba013a855f1e92b23ef75`; proposal row `abf6f17658894cc45192cc17d1a0fa90b4d46bf80d739a39101103fdc6c83f6d`; reviewed assignment `fd00b82774349d3bbbf650f4f0219c9853d97a37ac3f69fcbfbd971c45262a9b`.

### DEC-MOD-006

Question: Which canonical serialization encoding and field-width policy do manifest, descriptor and metadata records use in v1 (spec 25.1 #1, #2)?

Required evidence: Manifest bytes per entry and total manifest size at 1e2..1e7 entries per candidate on the corpus; Encode/decode throughput and peak memory; streaming emission feasibility; Independent reader implementation effort and ambiguity log from written spec (Task 28); Differential fuzzing for accepted non-canonical encodings per candidate (Task 30); Migration cost given frozen Descriptor-v1, Entry-v1 and crypto-v1 bytes

Review: Measure 10^2..10^7-entry manifest alternatives but retain frozen Descriptor/Entry/crypto bytes unless a governed new version is accepted.

Additional HC screens: HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-05 P `artifact_bytes`; OD-06 P `encode_wall_s`; OD-07 P `decode_wall_s`; OD-08 P `alloc_peak_bytes`; OD-09 N STREAM emission feasibility is a binary bounded-output requirement; OD-21 N Independent reader ambiguity and canonical parse are clean-room conformance checks

Pins: source row `d1406893577019558398023418a7a348c676e02c0b57b0eb480525e9acab5cd7`; proposal row `012eedc4692d0adf9b23f4931da0691b19e1146e5f75b2fe128c0e843c594ffa`; reviewed assignment `e17e2197aaa57eaac5ad31fc7477c4c00efe37b1cf26d9f7724024d18ff990e6`.

### DEC-MOD-007

Question: Should per-ContentObject chunk_root be descriptor-bound with its chunk count and logical length (or otherwise authenticated) so a range reader can verify bytes against one object root without trusting unauthenticated counts?

Required evidence: Formal soundness proof sketch of range verification against truncated/extended lists per candidate; Proof bytes and request count for remote range reads (Task 12); Wire cost per ContentObject at large entry counts

Review: Formal proof and remote request/proof bytes must accompany any per-ContentObject root change.

Additional HC screens: HC-01, HC-09. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-05 P `artifact_bytes`; OD-11 P `verified_range_fetch_bytes`; OD-15 N Untrusted chunk count/list truncation and descriptor binding are binary range-authentication soundness conditions

Pins: source row `b86f60bc81f75df0ee17bc81646bac39c7c983e07918fe9da2855ab78070f4c0`; proposal row `170a6b4b1e282a29f4a70422c0e45ad549ddc98112ea0111651fd9f3b813b740`; reviewed assignment `ba0caa2bd4c465ca47d3eb0df1b267fd16db0f7089c786fbcc1ab7d29cff00fd`.

### DEC-MOD-008

Question: Must replan_archive be corrected to record the frozen gear-norm-v1 chunker identifiers used by pack, and how are archives already carrying the divergent entrybound/gear-cdc-v1 identifier treated?

Required evidence: Test PCR and chunker_id equality between pack and replan for identical input; Inventory of any published archives carrying the replan identifier

Review: Compare same-input pack/replan bytes and inventory published replan IDs without reusing a frozen identifier.

Additional HC screens: HC-03, HC-13. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-01 N Pack/replan chunker identifier and PCR agreement are exact frozen-identifier conformance; OD-20 N Published divergent IDs require explicit migration/compatibility truth

Pins: source row `0608291b490942e00278f9a0511f838d0b88ced5e5bb03b75f108a47dfb2460b`; proposal row `743e56ce2806973c3f6d7f93266b346559f76d57b5f3225657638b3c178d24ad`; reviewed assignment `4aa9724e3c22c93d9855261f71047c72e916dd085ab445494f0a724010003185`.

### DEC-MOD-009

Question: How is metadata criticality exercised in v1 (which names are Critical, how readers treat unknown Critical items) and is restorability a per-name constant validated by readers or a per-item fact?

Required evidence: Identify metadata whose loss must fail extraction on each platform (Task 15/17); Fail-closed tests with constructed unknown Critical items (needs research feature); Wire/identity cost of per-item fields

Review: Unknown Critical refusal and per-platform loss cases need exact vectors before criticality can be frozen.

Additional HC screens: HC-15, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-01 N Metadata criticality and restorability authority are binary model semantics; OD-23 N Per-item wire fields and conditional rules are permanent specification cost

Pins: source row `9239d7a6b25f75cc5b778ef37201a41754b928223d06c423e68d39170f0678c8`; proposal row `d084e8b207d977d60b44a62c7c1a968c3a5296f8b16a0d2b87f7a365bb98443a`; reviewed assignment `2dc58ba50c924729d7f7bbd499e6b2750e114bb79833e356f3b737d22717de63`.

### DEC-MOD-010

Question: Should the ArchiveDescriptor carry path hostility and collision-class summaries, and if so are they advisory caches or validated authorities, with which enumerated classes and Unicode version?

Required evidence: Collision/hostility prevalence on corpora and cost of reader-side computation at large entry counts; Unicode case-fold/normalisation version stability across platforms (Task 16)

Review: Measure Unicode-version drift, collision prevalence and reader computation cost without making a cached claim authoritative by default.

Additional HC screens: HC-08, HC-12. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-01 N Descriptor path hostility summaries are advisory caches or validated authorities by a binary I1 rule; OD-17 N Path and collision refusal remain binary structural floors; OD-07 B Large-tree reader-side collision/hostility computation lacks a canonical summary-construction time instrument, role and band; list_latency_s is a broader list operation

Pins: source row `6c0ca3bb859da6da080ee5295738c99c98150d7b1010e00bf29376ad843e4985`; proposal row `c629ced53a7cfe72c4c08e785c8e04c23a3ac4f22ad22cf700ad358636441050`; reviewed assignment `b29bf301f3517c803cd3df9313d87df5122bac93248e06847b38f2ffd5eb95ba`.

### DEC-MOD-011

Question: What byte-level determinism does Entrybound guarantee: LAI only, byte-identical unencrypted containers for a pinned (planner id, format, writer version, codec builds) tuple, cross-writer identical canonical manifest bytes, and/or byte stability across Entrybound minor versions?

Required evidence: Cross-OS (Windows/WSL/Docker), emulated ARM64, cross-toolchain (rustc 1.97/1.98) and cross-dependency-version byte comparison of pack output over the corpus; Encoder output stability study for zstd/lz4_flex/lzma-rust2/preflate-rs/jixel versions; Repack representation-only byte identity over corpus; CI consumer needs assessment for cross-version stability

Review: Cross-version and ARM64 runs must name the exact tuple before claiming byte-identical output.

Additional HC screens: HC-15. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-01 N LAI versus pinned tuple byte stability across OS/toolchain/codec versions is a binary declared-scope reproducibility rule; OD-20 N Representation-only repack PCR/PCI identity is exact conformance; OD-25 N CI consumer stability needs human workflow evidence

Pins: source row `c36f590a25ce5293adb63effd69ea70297c0563249004629f35bb9958e34a617`; proposal row `8f57f04e7150e2a12f428690f21e2094f87cc893d2c7f27c2999686eaebf045c`; reviewed assignment `90dfe269d61de97cf8361b660988b9778eebf85e32bf3ee11656cd27022e8fe0`.

### DEC-MOD-012

Question: Given that encrypted bytes are intentionally non-reproducible, what reproducibility and identity guarantees apply to encrypted archives (LAI/AUX stability, PCR under keyed boundaries), and what identity material may be exposed outside encryption?

Required evidence: Independent encrypted builds on separate machines/keys: compare LAI, AUX, PCR; Quantify PCR divergence under keyed CDC and rekey; Confirmation-of-file risk analysis for detached signatures exposing LAI (crypto Task 22.4)

Review: Separate-machine/key comparisons and detached-LAI confirmation analysis must not claim ciphertext reproducibility.

Additional HC screens: HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-16 P `presence_advantage`; OD-01 N Encrypted LAI/AUX and keyed PCR behavior are binary identity rules; OD-15 N Ciphertext non-repeatability and signed-root exposure are trust conditions

Pins: source row `6833259c93d217c90baceb69279b47bf4f5734fd9a1dc9b72a86704f5dadd5a1`; proposal row `406cc4e1ede25181278ab194222238f67d07773ab60972b93d8eb5a57e3ddcd9`; reviewed assignment `13a24e4925f0644d7ac90a65ca6927a78732a6675a86f2f37c17a7285c741794`.

### DEC-MOD-013

Question: What determinism guarantees hold when hashing, chunk analysis, codec candidate evaluation, compression and verification run in parallel, and how is the worker/thread configuration kept from changing output bytes or identities?

Required evidence: Research-harness prototype of parallel pack/verify: byte and LAI/PCR/AUX comparison across worker counts 1..32 and machines; Speedup and memory versus worker count; diminishing returns (Task 14); zstd/xz multithreaded output differences vs single-thread at pinned parameters

Review: Measure workers 1..32 on pinned machines and codec settings; speedup never waives identity equivalence.

Additional HC screens: HC-06. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-08 P `alloc_peak_bytes`; OD-13 P `parallel_speedup`; OD-01 N Output bytes and LAI/PCR/AUX invariance across worker counts is binary determinism

Pins: source row `94be5012d430fe663d2db24d582e1625f3f93d9c294b4ff0bbb99168dddc87be`; proposal row `c4f6ee473e620740915d261f32664ccd8d8303d06f4172efeb13327b5706b85a`; reviewed assignment `f62b50830ee0b7618f8d6ae16affaf78f186565cf23a32a3c30379429b36e519`.

### DEC-MOD-014

Question: How are machine-readable detail records (invariant, entry, byte offset, budget field), the UNVERIFIED qualifier, DEPRECATED notices and warning severity represented in the typed library result and CLI output?

Required evidence: Audit of every emitted diagnostic for available entry/offset context; Conformance cases asserting detail fields; Usability review of error messages (Gradle empty-zip lesson)

Review: Audit emitted diagnostics and test user comprehension of UNVERIFIED and DEPRECATED notices.

Additional HC screens: HC-10. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-04 N Typed invariant/entry/offset details and warning severity are exact diagnostic conformance; OD-25 N Machine-readable detail, warning severity, UNVERIFIED and DEPRECATED comprehension need human evidence; error_actionability_fraction only scores refusal remedies

Pins: source row `8811eb410594027e8805c2b87b5ee24538b15044a89ba137187d69d0a75e5462`; proposal row `4aedf9a4a9319cc927775181414add12f9ce5b963a1fab6e8ebe9127f7f16b93`; reviewed assignment `3de77e6a8b18624e51aed2cb2faed263cbf99fef70b7fc9af3cf60e1c3746e27`.

### DEC-MOD-015

Question: Should remote LAI SAME be qualified as declared (unread payload), what AUX status applies to remote diff, and which diff tiers do Entry-v3 changes (ACLs, Windows security, ReparsePoint, macOS metadata) land in?

Required evidence: User-comprehension test of SAME under partial verification; Diff tests over Entry-v3 archives aligned with repack identity rules

Review: Partial remote reads cannot silently advertise full equality, and ACL/security/Reparse changes need exact tier fixtures.

Additional HC screens: HC-08, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-15 N Remote SAME must state which payload and AUX bytes were actually verified; OD-20 N Entry-v3 diff tiers and repack identity effects are binary semantic classification; OD-25 N Qualified SAME wording requires human comprehension evidence

Pins: source row `20888538956eb92d64ea3b1171d15f085ac0f8fa8a36013525e08614ee64086f`; proposal row `5c41ef879c1b61c1ddc24b0e5969b66e5ea7cc943a62ec43dc5c420d6c5669c5`; reviewed assignment `ca498460105e84d56b5362bc9847848409dc19ee6eff9f52621f958d72dabfb2`.

### DEC-MOD-016

Question: Which single digest algorithm does Entrybound format version 1 use for chunk, content, structured, section, PCI and identity-descriptor digests?

Required evidence: Throughput benchmark (MiB/s, CPU-s) of pack and full verify per candidate on the corpus, Windows and WSL, single- and multi-core, with and without hardware acceleration; Tree-hash/proof-size comparison for verified range reads over CDC chunk lists; Standards, FIPS/compliance and long-term availability review from primary sources (FIPS 180-4, FIPS 202, C2SP BLAKE3, RFC 9861, SP 800-232); Dependency audit of each implementation crate (unsafe, maintenance, license); Independent security review dossier as required by SPEC §25.2

Review: Compare pack/full-verify throughput and tree proof size, then require primary standards and independent security review before a hash change.

Additional HC screens: HC-09, HC-12, HC-14, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-07 P `verify_wall_s`; OD-11 P `verified_range_fetch_bytes`; OD-15 N Digest security and long-term compliance are external binary trust screens; OD-22 N Hash crate maintenance, unsafe and licence findings are permanent costs

Pins: source row `93c230d64d3db99acf6dce6e4c679cd9b491b2931f9c6bbc2750261d5f844e5b`; proposal row `2735a59dcf5b997e6e5a05fd2e62ed88cc6f0d467b6202987211df460fbffce5`; reviewed assignment `b611a453eaee10e7cbf9a0f1f462a529e4751fd34818b72d97dd9a146dc1fdfc`.

### DEC-MOD-017

Question: Should Entrybound compute and publish ecosystem tree digests (in-toto dirHash1, gitTree, NarHash) alongside its minted LAI, or document why minted roots replace them?

Required evidence: Semantic coverage comparison of LAI vs dirHash1/gitTree/NarHash on corpus edge cases; Cost of computing each digest during pack/verify; Consultation with in-toto and reproducible-builds communities

Review: Measure added pack cost on corpus edge cases and consult affected standards communities before publishing another tree digest.

Additional HC screens: HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-06 P `encode_wall_s`; OD-20 N dirHash1/gitTree/NarHash versus minted LAI semantic coverage is binary identity truth; OD-26 N Ecosystem digest demand and interoperability are human/external context

Pins: source row `bb843999b651b71b1d8ba807c7280f4a566e92204ca2e43cbc69072c865a1820`; proposal row `1cc2732657365eb3c93bcf6e8fff49844200e505eb304732f60be6433fe2aa06`; reviewed assignment `f74c93e17c66ddbd741a0589e36cdb77fad35cff824a1bcd2dc51e14307dea30`.

### DEC-MOD-018

Question: What is the normative v1 EAM object list (twelve vs fifteen named objects), and are ChunkGroup and Segment first-class model objects?

Required evidence: Enumerate SPEC §3 objects/fields missing from code and classify V1-blocking vs deferrable; Independent-reader ambiguity log

Review: Enumerate missing SPEC fields and record V1-blocking versus deferred status without inventing a new object identity.

Additional HC screens: HC-12, HC-16. Required routes: FORMAL.

Dispositions: OD-01 N The normative twelve-versus-fifteen object list and ChunkGroup/Segment first-class status are binary model registry choices; OD-21 N Independent-reader ambiguity is a clean-room specification gate

Pins: source row `3a9b5e3fc5a0b14cfa9689e9bb93c9c05aa0959dc58a183ef1371cb09f305a19`; proposal row `6e4a92c284801f121ae4d1b8a30176e3968ab732465f09b87521f7c37e287d57`; reviewed assignment `53d3e422fe3b020fbd22102e02bbb86aae4ff76a7193685ec3713a2f3ccf8167`.

### DEC-MOD-019

Question: Should declared path encodings participate in entry identity and LAI, and how must legacy adapters choose declarations deterministically?

Required evidence: Corpus measurement of legacy archives where identical name bytes receive different declarations across adapters/modes; Security analysis: can declaration exclusion enable two names that decode differently to share identity

Review: Cross-adapter identical-byte/different-declaration cases and security analysis must decide whether exclusion permits identity aliasing.

Additional HC screens: HC-08, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-01 N Path-encoding declaration in Entry identity/LAI is a binary name-equivalence and collision rule; OD-19 N Corpus frequency of declaration disagreement is scope context, not format import acceptance

Pins: source row `55996f3b13802c1f178d7ce496c858af97a72bd8e7c4be35a7c132e7e7119ce6`; proposal row `5e87580f044cdabb58734978722f76dcd707dcedbbae9d7837a96ee9f7b3fe78`; reviewed assignment `42fac1c503aec8ff7035c59517bd434d55a71139f799a2669f91ef778695d21f`.

### DEC-MOD-020

Question: Is entry identity meant for cross-archive comparison only, and should a path-independent content-and-metadata identity exist for rename detection, dedup and incremental update?

Required evidence: Diff/incremental use-case evaluation with rename-heavy histories

Review: Use rename-heavy histories to test consumer need without silently redefining frozen entry identity.

Additional HC screens: HC-02, HC-13, HC-16. Required routes: FORMAL, HUMAN-FACING.

Dispositions: OD-01 N Path-dependent entry identity versus path-independent auxiliary identity is a binary model construction; OD-20 N Rename/dedup/Incremental update identity behavior is migration conformance

Pins: source row `983841f582166d27ade4852c6671afd014a46979f3df9ead746c069f7528b458`; proposal row `ac607d4300339a929691364796720fa0e4617af2b11fd5498fde30cd9cb994a6`; reviewed assignment `d69accea5d18f5b560e43e8109bef5eea2ac52d3f7ccf473ee51cd2c47514538`.

### DEC-MOD-021

Question: What is the v1 EntryKind set (implemented Directory/File/Symlink/ReparsePoint vs SPEC Fifo/Socket/BlockDevice/CharDevice, whiteouts, junctions) and how are unknown kinds handled?

Required evidence: Task 18 special-file prevalence and restoration feasibility per platform; Security review of device/FIFO extraction policy; SPEC amendment for ReparsePoint

Review: Platform prevalence and special-file restoration tests must precede expanding the four implemented kinds.

Additional HC screens: HC-06, HC-08, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-03 P `restore_fidelity_fraction`; OD-01 N EntryKind list and unknown-kind handling are binary wire/model rules; OD-24 N Device/FIFO extraction and parser surface require security review rather than a graded utility proxy

Pins: source row `00f8fdc03cc71c1e82644558d7f03ec3b8862ec142e28847341353891a82acb0`; proposal row `29a543cbcef3e34a573e0f223a30e4c966ea07a4d59f4e87f07913a3c8c9a1e9`; reviewed assignment `868d7b2e9e33feedc45ed26c3839ebebb4e1d77f6e54f439ea2d0c5e08c41727`.

### DEC-MOD-022

Question: How is core.executable derived from POSIX mode (owner vs any x bit) and from non-POSIX sources (Windows, DOS/NTFS ZIP, 7z), and must it always be present on File entries so absent and false cannot yield different LAIs?

Required evidence: Audit of every writer and adapter for core.executable emission behaviour; Pack the same tree on Linux, Windows (NTFS) and via ZIP/7z/tar import; compare LAI; Survey executable derivation in git, Nix, ZIP external attributes

Review: Compare the same tree across NTFS, POSIX and legacy imports and preserve the absent-versus-false distinction truthfully.

Additional HC screens: HC-02, HC-08, HC-10, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-03 P `capture_fidelity_fraction`; OD-01 N core.executable derivation and presence on File entries are binary LAI canonicality rules; OD-19 N ZIP/7z/tar source mapping is exact adapter conformance, not graded import acceptance

Pins: source row `b2af5bdc8b4558d4cdb0977fb9f0561e93ef967088e566973aec2dd195e4305d`; proposal row `d8bc769513b21d98ad75e921282dcfbd690a98a7bd48b6bcaeacbbd1ed9ff519`; reviewed assignment `0515b8c4fa47da1feeebd5228c16ab882eaa443fb0f27715bc283118b14500bb`.

### DEC-MOD-023

Question: Should structured explanations add distinct NOT_APPLICABLE and PROSPECTIVE evidence classes instead of overloading NOT_RECORDED?

Required evidence: First-use comprehension study of explain output (simulated, labeled)

Review: Avoid overloading NOT_RECORDED when a fact cannot apply or has only planned evidence.

Additional HC screens: HC-01, HC-16. Required routes: EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-01 N NOT_APPLICABLE and PROSPECTIVE are exact evidence-class semantics; OD-25 N First-use explanation comprehension requires human evidence, not a simulated-success primary

Pins: source row `18ac820cf2688ae4fd44c6020629192cadb7f3426dfacb4e5a7395eb4a37f12a`; proposal row `20be3fe8c673fa50157eff4d8dce55bede4f0467059434e876849685fb3e2b39`; reviewed assignment `ff761dcd1cee81666ffabbb6bbce714829c12ecabced91df6f485f21a4cba69b`.

### DEC-MOD-024

Question: What feature-bit assignment policy governs model-semantic features in v1 (spec 25.1 #14): which features are incompat vs ro_compat vs compat, cumulative vs independent dependencies, and content-dependent activation?

Required evidence: Inventory of assigned bits with a per-bit analysis of whether an old reader ignoring it could misinterpret meaning; Measure fraction of real archives (Windows packs, v6 profiles) that require each bit and the resulting old-reader refusal rate; Determinism impact of content-dependent activation across planner versions

Review: Audit every assigned bit against silent reinterpretation and measure real archive activation/refusal before changing the frozen bit policy.

Additional HC screens: HC-03, HC-11, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-01 N incompat/ro_compat/compat and feature dependency activation are binary old-reader semantic safety rules; OD-21 N Old-reader refusal is a compatibility conformance case, not a clean-room utility score; OD-26 B Content-dependent feature-bit activation and old-reader acceptance need a canonical native-version interoperability selection instrument, role and band; descriptive preinstalled-tool proxies do not measure old-reader compatibility

Pins: source row `3933034a84d9a29744d1526fe678eab34cbdefb4c9415757b3dcee107b2b71a1`; proposal row `284f191030e10cddbe7c955ff7a92d2e8fc76129e26360f20bb26b06d4b90c7e`; reviewed assignment `9064c7ba8c27976038f51dbc5c073aacd466e9353acffeb6ff3adb20c0738a18`.

### DEC-MOD-025

Question: Is the frozen path-derived hardlink group identity (hash of content digest and sorted member paths) the right construction, given AUX churn on alias renames?

Required evidence: Diff experiment renaming/adding one alias across candidates (AUX changes, report size); Determinism test copying trees across filesystems; I1 analysis of each representation

Review: Compare alias-rename report size and filesystem-copy determinism while keeping the canonical group hash unambiguous.

Additional HC screens: HC-07, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-03 N Hardlink alias restoration stays an exact fidelity condition; OD-05 N Diff report bytes are not native archive artifact_bytes; OD-20 B Selecting a hardlink-group construction by AUX churn and diff-report growth on alias renames lacks a canonical churn/report-size instrument, role and band

Pins: source row `0f41537671de68a4f921607036434b1c819a96e5aa4840c33200103b437725f8`; proposal row `67c58c183b65ef54fc0b08f154be9e27fd85ef613f461c0e3b40a196e0f3e575`; reviewed assignment `438df85a19fb100e14520f4974441c4847b26772808613672e18946ce5d4c002`.

### DEC-MOD-026

Question: Should holes be represented by a registered hole TransformPlan (SPEC), by ordinary zero-filled chunks plus AUX sparse-map metadata (repo), or by another scheme?

Required evidence: Size, pack/extract time and hashing cost on sparse corpora per candidate; Extraction hole fidelity per platform (Task 15)

Review: Hole TransformPlan versus zero chunks plus sparse AUX needs storage, hash/pack cost and extraction-hole fidelity on sparse corpora.

Additional HC screens: HC-07, HC-08, HC-17. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-03 P `restore_fidelity_fraction`; OD-05 P `artifact_bytes`; OD-06 P `encode_wall_s`; OD-07 P `decode_wall_s`

Pins: source row `55d6e7b9070f0c2e6a321c2191466111a4d13a5c1045499766824618bc49d895`; proposal row `027e4fbfce93dd4d8e2641da134c2764243f68df0758306db39802ddb80a6b91`; reviewed assignment `2adfe0727c4496512fb7f3f37760879ecca84377ca33a2d3245fbdfa0a9e77c9`.

### DEC-MOD-027

Question: Exactly which parameters must the LAI, PCR and AUX descriptors bind (algorithm id, namespace, format version, identity profile, archive role, entry count, total logical size, chunker id, planner id, transform plans, dictionaries), and is the current field set frozen for v1?

Required evidence: Formal analysis: can two distinct semantic states share any root under each candidate field set; Survival-matrix tests per candidate (which operations change which root); Security analysis of signed PCR not authenticating decode path (crypto cross-review); Identity-root conformance vectors recomputed by an independent reader

Review: Survival-matrix and independent-root vectors must expose any distinct semantic state sharing a root before fields freeze.

Additional HC screens: HC-01, HC-09, HC-15. Required routes: EXTERNAL, FORMAL.

Dispositions: OD-01 N Each descriptor field's LAI/PCR/AUX root participation is a binary semantic-binding rule; OD-15 N Signed PCR must authenticate every decode-path claim used by a verifier

Pins: source row `a49284cd6cac809c8a3461a1780cb67118e91c23ecf91c9ad83cc8c84d879a04`; proposal row `74f6e12fcf2072514e88e71a6501510ae9170bbe4b4582003c7233ee8fe901f5`; reviewed assignment `c19cb3a79d082c9391797236dc2522619580949e437e92e51ff4bdf40ce7557e`.

### DEC-MOD-028

Question: What must identity/strict-v1 contain (mode bits, ownership names vs numeric ids vs SIDs, mtime precision and normalisation, hardlink grouping, xattrs, ACLs, platform metadata), and does it ship in v1 (task 19)?

Required evidence: Cross-machine/container/user-namespace capture of the same tree: stability of names, ids, SIDs, mtime precision (Task 19, Task 15); Consumer requirement survey (backup, package, reproducible-build users) for strict identity; Implementation cost: missing uname/gname metadata names

Review: Cross-user-namespace and platform capture must separate unknown names/SIDs/mtime precision from invented facts before identity/strict-v1 scope.

Additional HC screens: HC-02, HC-07, HC-08, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-03 P `capture_fidelity_fraction`; OD-18 P `cross_platform_restore_fidelity_fraction`; OD-25 N Strict identity metadata needs a consumer requirement survey

Pins: source row `4a68046b2abd8df63e6947770fad17c6b62830a5a28e6cdc432846b3e1d05532`; proposal row `a13c2769277ec0f2d1ec9db1e57907bc002be28d29c1c92228a0c8a927032851`; reviewed assignment `a5fae024cdd45c3321e7df06526f0ad48673b4928637864fa41e3b3b9e7b5e26`.

### DEC-MOD-029

Question: What are the exact LAI/PCR/AUX/PCI and signature-binding outcomes for every archive operation, including operations omitted from SPEC §8.1 and the inconsistent --add/content-change AUX cells, and are they enforced by tests?

Required evidence: One integration test per operation asserting all four roots and three binding states; Formal derivation of each cell from descriptor definitions

Review: Derive each omitted operation cell from descriptors and assert all four roots and three binding statuses.

Additional HC screens: HC-01, HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-15 N Signature binding VALID/STALE/INVALID states per operation are binary trust conformance; OD-20 N LAI/PCR/AUX/PCI survival cells are binary identity-rule conformance

Pins: source row `2196d09dc1b537a506a5713f9001b813c30e91b65b874362bd6cd9b07bdecebd`; proposal row `89d0587b55593d2172086a64ebb1d80063f04218ac56c56c96dde73b96b2ed3b`; reviewed assignment `bc2bf7fab715b6b6ebef0e92bcb94bf817470cbece19ad633c990060cffe5f8a`.

### DEC-MOD-030

Question: Which metadata participate in LAI under identity/v1: only core.executable (status quo), none, a NAR-like set, or mode/ownership/ACLs?

Required evidence: LAI stability across hosts/filesystems for each candidate over the corpus; Consumer mapping of fidelity levels 0/1/2 to ecosystems (Go, Nix, Git, npm, OCI); Threat model of permission tampering preserving LAI under content-binding signatures

Review: Cross-host LAI stability and Go/Nix/Git/npm/OCI semantics must be reconciled before expanding logical identity.

Additional HC screens: HC-01, HC-07, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-01 N Which metadata participate in LAI is a binary identity-profile construction; OD-03 N Metadata capture/restoration fidelity remains separately reported, not a proxy for LAI membership; OD-16 N Permission tampering under content-only signatures requires external threat review; OD-25 N Consumer fidelity-level expectations require human evidence

Pins: source row `bddd1707cd60d893cb6e2f37b9fe43344b2980eae4ee7944038fc23c4dabf4af`; proposal row `a3eb0e8a7a79c82a0b6383c891773a905abaf9cb456e550f4bf45463a13fbf53`; reviewed assignment `ef2326c54e10a9adc49c0330d559fed7097188faef8b46743bdcf4a073f20d61`.

### DEC-MOD-031

Question: Should model rules omitted from the I1-I31 gating list (P2, P5, P8, directory carries no content, hole semantics) be promoted, and how is each invariant traced to conformance cases?

Required evidence: Traceability audit of I1-I31 and P1-P8 to code and tests; Review of past feature additions (feature bits 0x1-0x10000) against gate coverage

Review: Audit I1-I31/P1-P8 versus code and conformance cases, including past feature-bit additions.

Additional HC screens: HC-01, HC-03, HC-12, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-01 N Promoting P2/P5/P8, directory-content and hole rules is a binary invariant-traceability decision

Pins: source row `5d2372827096dd64c8217a6fd74a80fcf68b1f2b4188d3d68e20734964573466`; proposal row `9e39e7893460ea0ee23cbdb40f58d9c8d6605fc024beba5566bcbaaf0e021d5b`; reviewed assignment `ca8b7aaddc6aeb58364bcc6f88ca9e0482eb154c9e4efe4709d7501d795a7da5`.

### DEC-MOD-032

Question: Should environmental I/O failures, transient network/transport faults and misbehaving range servers get their own outcome class or reason codes rather than POLICY_REFUSED or CORRUPT?

Required evidence: Fault-injection tests (disk full, permission denied, pipe close, connection reset, proxy mangling) recording reported class/code; Usability review of resulting diagnostics

Review: Inject disk full, permission, pipe-close, reset and proxy mangling to ensure no I/O failure is mislabeled as hostile bytes.

Additional HC screens: HC-10. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-04 N Environmental I/O versus data-corruption class and code are exact diagnostic semantics; OD-12 N Network fault typing is a remote assurance floor, not an HTTP latency primary; OD-25 N Reported failure comprehension needs user review

Pins: source row `c588efe2500a00aa7f1e18356fa9c8c4555dfe0b9dcad3a6045beca3cc0271d8`; proposal row `5e1c7fd8a3a87e660055e1284b4f7fe0b58eb40bcf88e8d602701e9241abb97b`; reviewed assignment `5ae96d6a7692ed7a8a3eda8188e76fc1f75131e1fedbc7dfc8bbfa8d35f363aa`.

### DEC-MOD-033

Question: What is the stable v1 library API surface (spec 25.1 #21): concrete types, signatures, error enum and stability promise, and does it adopt the SPEC four-object shape?

Required evidence: API usability review with first-time integrators (simulated, labeled) for pack/verify/range-read/extract tasks; Mapping of every SPEC normative behaviour to a type-level guarantee in each candidate; Downstream consumer inventory and semver-break cost estimate

Review: Map each SPEC behavior to type-level pack/verify/range/extract guarantees before a stable promise.

Additional HC screens: HC-01, HC-10, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-15 N Public API types must carry truthful verification authority as a binary safety property; OD-23 N Semver/LoC cost is permanent API complexity; OD-25 N First-time integrator usability needs human-facing evidence

Pins: source row `b1f8d99a79ecd52f9f5ea934c0103f9111c1b2eedbfe11bd421c5f6c2166d8d7`; proposal row `340470fe9da730ffa94aa00c7a7334e8fc63e988dcf94ab488d0976a06329d9c`; reviewed assignment `541f78be7be3367807bf9aeb812d6162494709984b6363b7deaa3172355432f4`.

### DEC-MOD-034

Question: Should Merkle roots over chunk lists and manifests remain an external RFC 6962-style tree with custom structured-hash domains layered over a flat hash, or be defined by a tree-native hash or a standardised sequence-hash construction?

Required evidence: Proof size and verification CPU for random ranges at 1e3..1e7 chunks per candidate; Second-preimage and domain-separation analysis of status quo vs byte prefixes; Independent-implementation exercise reproducing roots from written spec for n=0..N; Hashing throughput with parallel leaf hashing

Review: Compare proof bytes and verification CPU at 10^3..10^7 chunks with independent n=0..N root reproduction.

Additional HC screens: HC-01, HC-09, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-07 P `verify_wall_s`; OD-11 P `verified_range_fetch_bytes`; OD-01 N Domain separation and root agreement are binary hash-soundness conditions

Pins: source row `691d09ad82f305c1f29e09ec145baf185e984a4c2304a90de3ae404aef67076b`; proposal row `e48c774be1c5a49aa2bbc20348412ec443c771fb8c82de9301be3686dca10a27`; reviewed assignment `9b29bdab06c76e817e7f74098784aa2ca87aa9b61dae5756d2ade3035a640621`.

### DEC-MOD-035

Question: Which metadata classes and model objects participate in which identity root (LAI, AUX, PCR, PCI or none), including pending classes such as ownership names, NTFS ADS/forks, origin/quarantine markers, special-file attributes and sparse layout?

Required evidence: Per-class fidelity and stability experiments across platforms (Tasks 15, 17, 18, 19, 20); LAI/AUX stability measurement when each class is captured on different hosts; Security analysis of AUX-only security metadata under content-binding signatures

Review: Per-class platform capture and root stability must separate logical and auxiliary semantics for ADS, forks, quarantine and sparse layout.

Additional HC screens: HC-08, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-03 P `capture_fidelity_fraction`; OD-18 P `cross_platform_restore_fidelity_fraction`; OD-01 N Metadata-to-LAI/AUX/PCR/PCI assignment is a binary root-authority map; OD-15 N AUX-only security metadata and signature binding require threat review

Pins: source row `e507a7cfb74503ae4cb9171053e2254e12c8eed009ae1fc58b5a55711f25b7c5`; proposal row `510aa10abc3f8cc1416ae5e55af979ff278e130ace4f5988cf0c585014b94031`; reviewed assignment `f700418681a71fb653a37bf512405d30f2e46fdac06613bfe93d9d6acf7e7fcc`.

### DEC-MOD-036

Question: Which structural bounds belong in the model versus budgets and extraction policy: LogicalPath component length and depth, symlink target size (currently 1 MiB), and canonical sequence item cap (1,000,000)?

Required evidence: Distribution of component lengths, depths, target sizes and sequence lengths in corpora; Platform limits survey (Linux, Windows, macOS where available); Adversarial allocation tests at each bound

Review: Corpus tail lengths/depths and hostile allocation tests select structural caps without using declared_tightness_ratio as a path-size proxy.

Additional HC screens: HC-08, HC-12, HC-17. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-08 P `alloc_peak_bytes`; OD-17 P `benign_false_refusal_fraction`

Pins: source row `fe8a9c5158b1d19206c8bf064e498206b548ac0eff439662de4970c5dc135fc6`; proposal row `8d1b01113c1531021c6dfc1abccc48bdd6c14668843816f032ab2f68bbf43f2a`; reviewed assignment `ea7b3c467c1ad0fb30bb202c6c2dc55b81e1cdd7814c22dd62bd35177ee76c74`.

### DEC-MOD-037

Question: Can one archive carry signatures under different identity profiles, and how does verify report them?

Required evidence: Signature trust semantics analysis (Task 22.4) once a second profile exists

Review: Defer concrete multi-profile composition until a second profile exists and external trust analysis can assess it.

Additional HC screens: HC-14, HC-15. Required routes: EXTERNAL, FORMAL.

Dispositions: OD-15 N Multiple profile signatures and verification reporting are binary trust-scope semantics

Pins: source row `b6f35ff43f356ed124895469b3c020539ccbbd3f3113f814b804c933868c7c06`; proposal row `6a92c0e1abca2da55ad3ade9c3b890852e44c71e8209154eba2007411b8eaea1`; reviewed assignment `b77adf7de760c57845a16eaf6274af5b9bf048ad5c45ec1d78114ff9361c2db7`.

### DEC-MOD-038

Question: Do non-UTF-8 / non-Unicode native names (raw POSIX bytes, unpaired UTF-16 surrogates, legacy code pages) need LogicalPath representation in v1, and if so with which encoding declarations and validation rules (task 16)?

Required evidence: Native-path corpus on ext4/XFS/btrfs/NTFS/WSL (APFS/ReFS PLATFORM_BLOCKED): frequency of invalid UTF-8, unpaired surrogates and legacy code pages in real trees and legacy archives; Round-trip fidelity and extraction behaviour per candidate on each platform; Traversal-safety fuzzing of decoded-form ./.. checks per encoding; Identity impact (LAI divergence) of declarations under import heuristics

Review: Measure invalid UTF-8/surrogates on available native filesystems, test round trip and fuzz decoded ./.. traversal.

Additional HC screens: HC-02, HC-07, HC-12, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-03 P `capture_fidelity_fraction`; OD-18 P `cross_platform_restore_fidelity_fraction`; OD-19 N Legacy code-page mapping and declared encoding are exact name-conformance rules, not general import acceptance

Pins: source row `c359b4b96c770d1181833f5f5a7bc5a254a36e7263feb61027608129e3416c13`; proposal row `8e158f25d7c41280da756d48579b5b3213d14f4f49734a95cfd5783e7eabe9e3`; reviewed assignment `64f88d26deb74d813a4d6f2a1f9aab70558ef4aa5b9b9d434e90516349df4439`.

### DEC-MOD-039

Question: Can one archive contain multiple chunker algorithms or parameter sets (per category, per profile), how must PCR bind them, and is profile- or key-dependent PCR acceptable to users comparing physical arrangement?

Required evidence: Frequency of PCR divergence for identical content across profiles, categorisers and rekey (Task 8.1, 10, 22.2); Dedup/ratio impact of uniform vs per-category chunking; Verified-range soundness when chunker id is absent

Review: Compare uniform versus per-category dedup/ratio and PCR divergence without treating key-dependent physical roots as logical identity.

Additional HC screens: HC-01, HC-09, HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-05 P `artifact_bytes`; OD-11 N Verified range soundness with multiple chunkers is a binary authentication condition; OD-20 N PCR binding and comparison semantics across profiles or rekey are binary identity rules

Pins: source row `3e8adc23f4ff64ea808e0928fa9b74a940cc875cf91811a94dac5ff5a6ae4cc5`; proposal row `f9aaf6c2eb5d06f2c017804645061813af766f7fcf62eb8cda3c9755938edbc0`; reviewed assignment `e0ae21ee53f5d53f084a212f016c22ca8d67b4c10bdd32efed7e479c7745be00`.

### DEC-MOD-040

Question: Should region-member chunks use a plan_ref sentinel distinct from UNPLANNED_PLAN_ID (both 0) before format freeze?

Required evidence: Conformance tests of non-region chunks with plan_ref 0 in both layouts; Wire compatibility impact on frozen v6 archives

Review: Both-layout conformance vectors and frozen v6 compatibility decide whether a new sentinel can be introduced.

Additional HC screens: HC-01, HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-01 N Region-member versus ordinary plan_ref 0 requires a binary unambiguous record grammar

Pins: source row `6a4ca0bb08d9d1e929dfb2e3db0661349bb50928e3ebee62feb1e64112da4ed7`; proposal row `4a964ebeed87b53eb1e05e93a295f2e4734aba0915f6976b3eaa2dfc17a81148`; reviewed assignment `a993de505524f8d0991a530c9cf58595c34dad51c447a4736e68426cd1decbad`.

### DEC-MOD-041

Question: What is the v1 reason-code registry (codes, classes, stability and deprecation policy, prefix organisation, missing codes) and the precedence order when several outcome classes apply?

Required evidence: Multi-fault conformance corpus measuring which code each reader reports; Cross-reader (INDEXED, STREAM, random, encrypted) consistency audit; Coverage audit: emitted codes lacking tests (27 found) and missing codes

Review: Run the multi-fault corpus through INDEXED, STREAM, random and encrypted readers and close emitted-code test gaps.

Additional HC screens: HC-01, HC-10, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-04 N Reason-code names, classes, multi-fault precedence and deprecation are exact registry/conformance obligations; fraction of generic codes cannot choose precedence

Pins: source row `7ad676a6cfc1a6b9278ff2b4cd3fd2d8ab04ec7033065823bad0269fafbda1dc`; proposal row `e72bc850e495397295b9c79700b1cffcfb7420685c315643a30b3ad6684da8e4`; reviewed assignment `0e5b79835e985e237d8f7fb0d1543eb868c88e72975c1cb36ade4096947ddf31`.

### DEC-MOD-042

Question: Which document is the normative registry for record types, record versions, TLV type codes and minimality rules, and hash domains, and which record-evolution convention (new type number vs version field) is normative for future records?

Required evidence: Independent reader built from docs only, counting ambiguities and mismatches against production reader; Collision and gap audit of all type/version/domain assignments across code and docs; Review of precedent registries (EROFS, zstd, IANA-style) for maintenance cost

Review: Audit every assigned wire/domain value against code and docs before choosing new-type versus version-field evolution.

Additional HC screens: HC-01, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-01 N Record type/version/TLV/domain registry and evolution convention are binary frozen-specification rules; OD-21 N Independent reader ambiguity count and pass cases are clean-room conformance

Pins: source row `6446eb14fd2a33435eb2694390b7f987e0cf2f35ca5350fca3bde3cf85c423d9`; proposal row `b57c70d9875366dfdcfa18846ac4ae53ed0e78eb92c515c419a469083605c852`; reviewed assignment `7c2c656d017d2f650f549166db3734efda83e64e55bcfcefa510fb9f3f11ab0f`.

### DEC-MOD-043

Question: Should LAI-changing entry addition exist in v1, and if so under repack --add, a separate command, or not at all given the append non-goal?

Required evidence: Adoption workflow study (release updates, datasets) for incremental additions; User comprehension test of identity-change reporting

Review: Keep append non-goal explicit while testing a distinct add command or no-add v1 boundary.

Additional HC screens: HC-05, HC-10, HC-18. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-20 N LAI-changing addition and receipt truth are binary migration/identity conditions; OD-25 N User comprehension of changed identity and release-update demand need human evidence

Pins: source row `536ce7cc84890f391029ec201ae05e809622dfa80d6832f121bbb6c98e61b5b1`; proposal row `71f8e2e264bf2ee235403f6f70f9d5b11490266d86f216d372ebfaf5b612f162`; reviewed assignment `2f6fb95719105d7ae86b528748d98089512f24cbea261a20511231c6f49856eb`.

### DEC-MOD-044

Question: Can ReparsePoint entries have descendant Entries or carry content (directory junctions, cloud placeholders, dedup files), and is Entry-v3 tag 9 forbidden on non-ReparsePoint kinds?

Required evidence: Survey Windows reparse tag semantics on NTFS (junctions, mount points, cloud files, dedup) (Task 17/21); EntrySet validation tests for tag 9 on other kinds and ReparsePoint descendants

Review: Survey exact NTFS tags and assert invalid descendant and tag combinations before permitting any reparse projection.

Additional HC screens: HC-05, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-01 N ReparsePoint descendants/content and Entry-v3 tag 9 kind restriction are binary model grammar; OD-17 N Reparse traversal and cloud-placeholder behavior are structural path/safety floors

Pins: source row `be950e5b2e630d4cdce4c0b3740d1a78489e6d23323924c71cdbefb1ec718fee`; proposal row `ad42b4222af5e0c47903d63b0f988f5622f61580a3de72669eda9cb76d243582`; reviewed assignment `368555d07d55a1d266b75c63623a04159994b896d001d7a583cfbd519f4f4391`.

### DEC-MOD-045

Question: Should a default-off research-internals cargo feature expose canonical record, codec, transform, reconstruction and identity/Merkle internals, or must experiments use whole-archive encode?

Required evidence: List of experiments needing each internal and overhead of whole-archive path; Byte-identity test with feature disabled vs baseline build

Review: Inventory actual experiments and compare feature-disabled bytes before accepting a default-off public internals surface.

Additional HC screens: HC-03, HC-11, HC-13. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-01 N Research-internals feature must leave canonical bytes and identity unchanged as a binary model invariance; OD-21 N Whole-archive overhead versus exposed internals is an independent-implementation cost input

Pins: source row `fa5ea372933236163a691cb60ce455109921e08656a8eb5879cdac809c08f3b0`; proposal row `1979866bcb7c3ff3186387c9c0006f2850dd235686dc6ac8aaa76ceb03e7ad7d`; reviewed assignment `56cba8235a8b39cb5b4482118d42a7439c46d135b21e7a2a1773b1f1a443de15`.

### DEC-MOD-046

Question: Which import mode and identity profile define a sidecar describes_lai, and does verify --sidecar check it or report it as an unverified claim?

Required evidence: Measure LAI divergence between strict and compat imports of ambiguous ZIPs; Task 23 sidecar workflow evidence

Review: Pin import mode and identity profile to the sidecar claim; ambiguous ZIP experiments must never turn a digest claim into verified content.

Additional HC screens: HC-01, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 N Strict-versus-compat LAI divergence on ambiguous ZIPs is a binary identity counterexample; M19.1 import acceptance does not measure the sidecar claim; OD-20 N Sidecar describes_lai binding and verify --sidecar claim status are binary migration/trust conditions

Pins: source row `0c2a0569c3596b83d54c2cec3e771284bca214d69751a9a50b1d5df42bd0dbff`; proposal row `d4cd998795606620ddf1b2eebccbb76ccc757c622ccfddf78ac33eaaed4c5793`; reviewed assignment `44e421646927e6b2507e6393e3a5f76037531e2152e802ee08a0bbd50f30c958`.

### DEC-MOD-047

Question: Should Entry records keep storing derivable identity and auxiliary digests (verified on read), or should readers derive them only?

Required evidence: Manifest size cost of 64 bytes per entry at 1e3..1e7 entries; Remote list/inspect request savings from stored digests (Task 12)

Review: At 10^3..10^7 entries measure the 64-B-per-entry manifest cost and remote list/inspect request savings.

Additional HC screens: HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-05 P `metadata_bytes_per_entry`; OD-12 P `http_requests`; OD-01 N Stored versus derived Entry identity/AUX digest remains one binary authority rule

Pins: source row `dae060429e4f85407624215b86d4faeca59c227e68f3f7ce7516852f1f05fc6e`; proposal row `ad440a5ee9c0cf2884639b78e3ec7226616f8004264cad24ca76ca263c0cb986`; reviewed assignment `2842e27bd33a4f7ea76e9e2a80212cb7326be5bea8d5ee118490baba30c463f3`.

### DEC-MOD-048

Question: Are Chunks not referenced by any ContentObject valid in a canonical archive, and how do they count toward PCR unique chunk count and STREAM dedup windows?

Required evidence: Code audit and conformance cases with orphan Chunks in INDEXED and STREAM

Review: INDEXED/STREAM orphan vectors must agree without inventing an implicit ContentObject reference.

Additional HC screens: HC-09, HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-01 N Orphan Chunk validity and PCR unique count are binary canonical rules; OD-09 N STREAM dedup window accounting for orphan frames is exact streaming conformance

Pins: source row `ceb7ee6c6b4737442253c61dcec4ed57f5e3b478e8301de1df6a935364a0cd63`; proposal row `cb115d8325e4670df84b2337b7af18e14f8b4781a63a7b4f541a4585f0e28115`; reviewed assignment `29f4a579711fcefe04dc6e034b9e738a98eccc71626678963ebcc9dfd02c8571`.

### DEC-MOD-049

Question: How should identity-participating semantics that a capture platform cannot observe (e.g. POSIX executable state on NTFS) be represented so that LAI is canonical across packing hosts without inventing facts?

Required evidence: Cross-host LAI stability experiment over real corpus trees (Windows NTFS vs Linux ext4/XFS/btrfs); Inventory of LAI-participating fields and their observability per platform; Wire and identity cost of a tri-state encoding; Extraction semantics for unknown executable state

Review: Cross-host NTFS versus ext4/XFS/btrfs LAI experiments must bind observability, wire cost and extraction of unknown executable state.

Additional HC screens: HC-02, HC-08, HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-01 N Unknown identity-participating semantics need a canonical tri-state representation, not invented false values; OD-03 N An unobservable executable fact cannot be scored as captured without ground truth; unknown-versus-false is a binary fidelity distinction; OD-18 N Cross-platform LAI stability for unknown facts is a binary identity requirement, not a restore-fidelity preference

Pins: source row `262bd5fedf39cea8b08c9f96d52b8a71dd16a3441b56d1d1e219fae7f06de56e`; proposal row `67cfaab47fdf3656031e6c3a1e2df02512859307bb4520da539098d5031fb8d5`; reviewed assignment `f7d8f4c4a0552de102dd4b3b8f37f1baa11190b02bd845ee00c0def2ba020005`.

### DEC-MOD-050

Question: Should timestamp precision recorded from the capturing filesystem be canonicalized (or excluded) so identical instants yield identical AUX across hosts?

Required evidence: AUX stability experiment across NTFS, ext4, XFS, btrfs captures of identical instants; Restoration fidelity implications per platform

Review: Cross-filesystem same-instant captures must quantify AUX drift and test whether precision canonicalization preserves restoration fidelity.

Additional HC screens: HC-01, HC-10. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-03 P `restore_fidelity_fraction`; OD-01 N Timestamp precision participation in AUX is a binary canonical-identity rule

Pins: source row `e9850adccb7db3eb7afe24f7365fe3fc6dc531675ccc69cdcfebbb4d9f460a80`; proposal row `517f0bd1f89e26a2f71ee0d2dc570893744a14078738d7e961822f99a4bea491`; reviewed assignment `bcf270fee9441ecf4c82d4dfd376ac1cd26ad0ed6634ad2d4a3bdd40f1d5bf29`.

## Controlled LF source rebinding

On 2026-10-01, `policy_review/2026-10-01` normalized only CRLF to LF in `research/decision-method.md` and `research/archetypal-objective.md`, after verifying exact decoded-text equality. The canonical proposal was regenerated with only those two file-byte source hashes changed. Every reviewed assignment field, canonical assignment digest, status and per-record reviewer identity remained unchanged; only the raw proposal-row source pins were rebound. This is source-byte rebinding of the accepted prospective review, not a new candidate, experiment, outcome, gate PASS or live ledger migration.

The preceding method/objective byte identities were `fe27bcb0c078d700463916c12ef5f07b9c7cd5870ae2c25894e02fe2590dbb7c` / `ab00418cdfb40cb3ca283324d30d01e10a3dac4f6cb733b11d104dc0d567664a`; the preceding proposal identity was `4099090f862a03490ff19fac279bce217b72badcc4e028b108a03552f1efe7bd`. These are superseded byte identities, not current-source receipts. Their retention here does not assert that historical bytes are durably archived in the public repository.

The preceding separate review register and note identities were `f498e7098edbaec9aa7fc518d69c3c44a9762e78ce8c6738bdf617c43eefb57a` / `bd1bf78f07a9c2e5dd1b36d9da81003c1589e7155aa1d54225a95d27d5e987e2`. The question-specific adjudications and original independent reviewer identity are preserved.
