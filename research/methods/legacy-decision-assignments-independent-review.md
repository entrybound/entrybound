# Independent LEG decision assignment review

Reviewer session: `qualification_audit/2026-10-01`. This review accepts prospective initial HC/OD/evidence-route assignments only. It does not accept a candidate, experiment result, registration, release gate or `DECIDED` status.

## Exact source pins

- `research/decision-ledger.jsonl` SHA-256 `7ceb7c8c4d934c3cb5747b1dc6965657b09551cbbd3ceadf7e2f1644c3d77d90`
- `research/methods/decision-assignments-proposal.jsonl` SHA-256 `479d877e5247e00be3f03ab7e8a29945c9969c899efafb9306d0076cfe2e7446`
- `research/decision-method.md` SHA-256 `880e319c8d16e778f856cdbc738ecda088865c4d5e2499665fe058228a29426b`
- `research/archetypal-objective.md` SHA-256 `f81627d0171931aec960baa1ce0e1999f8d81802d207480803c575c692b30036`
- `research/methods/thresholds.json` SHA-256 `9cd4b5be3486ae671c54987df2c5ee4f8650e77f464145fe48fe829fe93ddc90`
- `research/experiments/decision-coverage.csv` SHA-256 `1c87648f7f2e8394988e26f0ca3e41acf49718778a36cd68e495b81779aeb5e0`

Each review record additionally binds the exact raw proposal JSONL row and the ten-field canonical reviewed-assignment digest. The review register is `research/methods/legacy-decision-assignments-independent-review.jsonl`. All 108 source questions and required-evidence lists were read individually; the source ledger's status and evidence bodies were not changed. Protected corpus, identity fingerprints, result bodies and private progress files were not inspected.

`P` is a registered primary metric; `N` is a question-specific reviewed no-primary reason; `B` is a missing canonical instrument/role/band. A `B` accepts initial routing only and bars the affected decision from decision-bearing preregistration, registered look, first result and `DECIDED` until a public canonical metric is registered and independently reviewed. Human-facing and external routes remain mandatory where listed. No measured results were used in this review.

Disposition totals: 89 primary, 144 no-primary, 27 metric blockers across 108 rows; 27 rows contain at least one blocker.

## Row-by-row adjudication

### DEC-LEG-001

Question: For --compat behavioural models in general (SPEC §25.1 #22), does a profile refuse or generalise when an input's conflict pattern is absent from the observed-outcome matrix, how are models checked for generalisation, and how many runtimes per format can v1 resource?

Review: The recorded generalisation question needs pinned-runtime prediction, unmatched-pattern and maintenance evidence. OD-17 tightness measures resource declarations and is unrelated.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-21 N Effort to build and maintain the tar and 7z models is permanent implementation cost under C1-C7 rather than graded product utility; OD-19 B Compatibility-model prediction on ambiguous conflict patterns lacks a canonical selection instrument with a committed role and band because M19.2 excludes reader disagreements

Pins: source row `8d5b5a97c093341c08f79c5461d192802b2a09d98fbc885ba1e9b1fdb4b80f59`; proposal row `150bb3edd2bfaac6c6d4c28983e06390de2ae4549b935a1215df35669e3fd281`; reviewed assignment `def0181b8f788d7a497f4d7f48a159d7b89720d648d06bc6145cd1796bebce96`.

### DEC-LEG-002

Question: Should structurally absent local-header fields be excluded from conflicts and per-entry resolutions be aggregated so conversion evidence scales to 1,000,000-entry archives within the 256 MiB cap?

Review: One-million-entry conversion evidence must fit the byte and memory cap while retaining conflicts. The AUX time is an import-time comparison. Auditor preference for itemised evidence is a human claim.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-05 P `side_data_bytes`; OD-08 P `alloc_peak_bytes`; OD-19 P `decode_wall_s`

Pins: source row `bd038f441a0a7679b356b84e6ebea4835fedf11f52ed1216c1ac9a3eaab279b2`; proposal row `7bccdd81140cf2f73651942b078b04e0d4f7ea184dfb82a3d50de8cd5a260f1b`; reviewed assignment `eb9d6d1ee3228c8d3b28fcfe40d9494ffc884298c5276cc2f758460d66f38844`.

### DEC-LEG-003

Question: Which SPEC §16.1 import-record fields missing from ConversionProvenance (source size, before/after normalisations, per-datum discarded or preserved dispositions, result LAI and identity profile, evidence offsets of resolved conflicts) are required in v1, and is type 29 one nested schema despite its duplicated table row?

Review: Field coverage and nested type 29 grammar are normative; large-import record size and consumer needs require separate empirical and human evidence.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-05 P `side_data_bytes`; OD-20 N Import-record completeness and before/after identity fields are binary provenance conformance, not a graded migration preference; OD-21 N An independent reader decoding types 28-29 is a binary clean-room/specification check

Pins: source row `408e0c9e6bf25cd7a6faa9f077e864c677ca9d5726c1d8e74dcee52d48a4aba9`; proposal row `ff7ff5a3762986442de5b19596f3dc1d7a1b5c2dfbaccfeab284ae020e7bd0f7`; reviewed assignment `10a110880ea23141ff75903c875bcb3f9c737f8dab27432ac5e21ca12feb06fc`.

### DEC-LEG-004

Question: Which conversion record strings become closed registries (source_format, adapter_id grammar, import_mode, outcome, semantic_field, action, typed dialect), must every counted conflict carry a resolution, how are byte observations rendered, and must provenance record tool version, runtime and producer strings?

Review: Enumerate every emitted registry value and independently reproduce LOM decoding before freezing identifiers.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-01 N Closed conversion vocabularies and conflict-resolution completeness are canonical semantic conformance, not a graded validation-obligation count; OD-20 N LAI stability and provenance producer strings across importer releases are binary migration identity and receipt-truth checks

Pins: source row `c631444d0b2ec74ffd43ae6c20377a91220db20f95f25ee3831dc747558c68e3`; proposal row `c3dae47808fe8b4f3daea47a9ec55b19c84c25aaca7b42fce08c29db4705f305`; reviewed assignment `ec700e0016be4f999a97cac5f64d8bf25d0aa49cb066e9adb2698adb1eed0656`.

### DEC-LEG-005

Question: Which classes do cpio (newc, odc, bin, crc; initramfs, RPM payloads) and ar (.deb, static libraries) adapters get?

Review: Adapter class depends on packaging workflow demand and safe cpio/ar semantics, not generic import acceptance alone.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-19 N Cpio and ar fidelity and hazard mapping require exact format conformance, while M19.1 import acceptance does not measure ecosystem demand; OD-26 B Distribution-package prevalence used to classify cpio/ar adapter priority lacks a canonical adoption-demand selection instrument, role and band; OD-26 adoption proxies remain descriptive

Pins: source row `5ccf06c70a82ab397bd29028db9172f164b54a02aa6d969361c49ae6fabe2521`; proposal row `d7daf1e9eaeb250c5d5e1018ddfa12354074d715bb8c7d876c23c04dd0981ec0`; reviewed assignment `f8df1536c6a72723260236f37276c553ac411f6868e9ffb157d28324bf42d8b2`.

### DEC-LEG-006

Question: Should Entrybound import or inspect annotate legacy inputs whose constructs are known to be interpreted differently by other runtimes, and if so from which evidence and in which output?

Review: Cross-parser warning value depends on annotation precision and human comprehension, with a truthful-reporting floor.

Additional HC screens: HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-25 B False-positive construct annotations and user interpretation lack a canonical annotation-quality selection instrument, role and band; error_actionability_fraction measures refusal remedies instead

Pins: source row `dc17aec2626e2c02860ee52d754418df464563c928f8243732f68dd56ad512af`; proposal row `675dace1b764dbd9a26943a0efd0bba56c710832f64b39f8889424ace3c83759`; reviewed assignment `880471f77d2649b6891cc28ebad4a0ff18eee0008007f7403a349d6ea4d7ddf9`.

### DEC-LEG-007

Question: When a transport or codec needs more decoder memory than policy allows (zstd window, xz dictionary, bzip2 working set, 7z LZMA dictionary), which outcome class and reason code are reported, are limits checked from headers before decoding, and may caller policy values appear in foreign LOM evidence?

Review: Header-first resource refusal needs exact reason codes, measured decoder peak and hostile pre-refusal allocation. M19.1 acceptance is not the policy-bound refusal criterion.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-04 P `reason_code_specificity_fraction`; OD-08 P `alloc_peak_bytes`; OD-17 P `refusal_alloc_peak_bytes`

Pins: source row `172ff313dc4563191ce2d21f85769611c0e3569e1c6ff4e74e1026c9dacf1d89`; proposal row `85ec07a7351ac7f6f713881b2194c4181a92cc3556cdd4f72d779f6d799435a1`; reviewed assignment `f7eac8281ab6dbe0f7352416a0626f7c2438b90a97e7a2de9524b840a4801a74`.

### DEC-LEG-008

Question: What is the scope of byte-identical export (same build, same profile across tool versions, across platforms and dependency upgrades), and how is it enforced: golden vectors, vendored encoders, receipt-recorded encoder versions, or re-versioned profiles when bytes change?

Review: Cross-platform and backend vectors must define the reproducibility scope before any profile can be called byte-stable.

Additional HC screens: HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-20 N Byte-identical export under each declared build, platform and dependency scope is binary migration determinism; OD-22 N Encoder vendoring and dependency-version retention are permanent C1-C7 costs rather than a graded utility metric

Pins: source row `36bf3bcee9e4552cccb0a772cb1c8c02b686be6e174b74e291a9eb64ff41ddc6`; proposal row `31b4c7ce3b9fb489ecfa6b279301486b2cee9db2b20805279d4c6d1fd180cdeb`; reviewed assignment `2f1f4bc97a851f755e832271e17ac0527f0d67d0a603de9e44643a6eaacc74a4`.

### DEC-LEG-009

Question: How does a consumer holding the .eb verify a legacy artifact was produced from it: deterministic re-export and byte comparison, logical comparison against LAI via strict import, a signed receipt, or a combination?

Review: Verification method choice includes measured cost on large artifacts plus a signed-receipt threat analysis.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-07 P `verify_wall_s`; OD-15 N Receipt forgery and artifact substitution are binary trust checks, not graded encryption-overhead comparisons; OD-20 N Deterministic re-export or strict import must prove the claimed source-to-target relation and identity rule

Pins: source row `60dcc4da4d683703d21d88bd584423053d697a73423cf5375c7fdc7f5040073b`; proposal row `fb7a311e829728e2dc027e841d7373dfd09ac03a570bcb8a0e4f5c6d4a82f8fc`; reviewed assignment `44329456436d63a2b25c3becbd1c2e24aac8fb31ae752e4711959619e51a3583`.

### DEC-LEG-010

Question: Which dual-publishing surfaces are stable v1 (publish command, migration-report-v1, target set, native relations, receipts per target), and do they deliver value before ecosystem adoption given converter bridges elsewhere went unused?

Review: Stable publish surfaces and bridge value depend on release workflows and honest failure handling.

Additional HC screens: HC-15. Required routes: EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-20 N Publish transaction and per-target receipt truth are binary migration/receipt conformance; OD-26 N Bridge adoption history and zero-install consumption are descriptive context, not a graded utility primary; OD-25 N Release-engineer workflow preference requires human-facing evidence because simulated task success cannot decide it

Pins: source row `b474b132a3d4da9ae3e52df0c3aea2825796e52bb9524bc0681fa48a15315adb`; proposal row `eda0c383524b0765619c34db8116e243af95283a5bdff0d46391f541b7831545`; reviewed assignment `46f0224933bc6b7c7e137ed3a267e846b594e7794290d33ceae9f4047c2cff9c`.

### DEC-LEG-011

Question: Must encrypted archives carry conversion provenance and preservation evidence in v1, and via which crypto record assignment and required feature, given crypto-v1 refuses archives with conversion data?

Review: A new encrypted record assignment cannot re-interpret frozen crypto bytes; current OD-19 import acceptance is unrelated.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EXTERNAL, FORMAL.

Dispositions: OD-15 N Encrypted conversion records must remain authenticated and truthful as a binary crypto composition condition; OD-16 N Provenance source digests and dropped-field names require a field-level leakage inventory, with any protection-effectiveness claim sent to external review; OD-20 N Conversion and preservation evidence presence and identity effects are binary migration conformance

Pins: source row `d062e0cc50219e07cf46cfd997ea7f29dc587dafc9b824bda92753024b709f09`; proposal row `13a5761add3c5b64823c9cb9c4980d4284c1d17feac285c878ab446908d2fae7`; reviewed assignment `9f6fce63cbb604a01573a28faa9717d1ee4919579af6147614529b3bd1650a21`.

### DEC-LEG-012

Question: Should encrypted legacy inputs (ZipCrypto, WinZip AES, 7z AES, RAR5) remain refused, be decrypted with typed integrity warnings in provenance, or be preserved opaquely?

Review: Encrypted legacy-format scope uses prevalence, bounded KDF refusal and cryptographic review, including typed provenance warnings.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-17 P `refusal_cpu_s`; OD-19 P `import_acceptance_fraction`; OD-15 N Legacy decrypt paths and unauthenticated-output warnings are security and verification-honesty floors, not graded encryption overhead

Pins: source row `b24e4aa63d7c6aa202874d65bd54233e8715df5828f1372dc8a1fd232b9348ca`; proposal row `c4dc8dfab1937b6471d4a401553b614bfd70dbfefd143c94e59151f6d4d5bea7`; reviewed assignment `b76443e7365cca99d8bced797832f127f61dc44a71b9065dcf81d4a1f192ddd9`.

### DEC-LEG-013

Question: Should export remain a bidirectional convert verb with --to/--target-profile and --allow-lossy (versus SPEC --accept-loss and --deterministic), and what do unversioned aliases resolve to once v2 profiles exist?

Review: The SPEC flag mismatch and future v2 alias drift cannot be resolved by import/export acceptance metrics.

Additional HC screens: HC-16. Required routes: EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-25 N CLI verb and alias comprehension require human evidence; simulated first-use success is heuristic and cannot be primary; OD-20 N Versioned alias resolution and deterministic receipt naming are binary migration-interface conformance

Pins: source row `cd257e9578dede72f7da5e0dca974318828d8e329bd569ff80b289381111070a`; proposal row `22b55358f86c78feda0ab35c6a66e6bc3a84f69408b4a90bc3a7fc377d5b636f`; reviewed assignment `a9a545c4835fb5777512e865d08c7f78ba259d0ebbdb92284ced8defeda16037`.

### DEC-LEG-014

Question: Are per-target normalisation rule sets identified and versioned separately from profile ids and recorded in receipts, and does import-then-export constitute the rejected canonical-rewrite service?

Review: A new rule-set identifier must not silently revise a frozen profile or turn import/export into an undeclared rewrite service.

Additional HC screens: HC-13, HC-16. Required routes: FORMAL, HUMAN-FACING.

Dispositions: OD-20 N Receipt-identified normalisation rules and re-rendered digest identity are binary migration/determinism checks; OD-25 N Consumer interpretation of changed digests is a human claim, not a graded proxy metric

Pins: source row `f0a0c534ccf49b76d1b4bd82a31a6b5868d903af4144e0b43f96ee5a62b00ca4`; proposal row `88113dfea29a83eb65e2987ed5bbf5dd8d14b426be262673bb9f6e7f617854ed`; reviewed assignment `04705c0653859c434506f91e67aa122aff8aaa5925c6ef1f5c6dcba8e0a4c544`.

### DEC-LEG-015

Question: Is export path refusal keyed to target format or declared extraction runtime, is the ZIP/tar portability asymmetry intentional, are backslash/colon/newline paths refused or typed consistently, and is any rename option permitted?

Review: Target-runtime extraction and hostile-name collision outcomes set the refusal rule. OD-07 decode performance is not the path safety question.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-18 P `cross_platform_restore_fidelity_fraction`; OD-19 P `export_acceptance_fraction`

Pins: source row `2a554955d252dfa41aabfb28da750678f734e6c54e383bbdb5af0bbab217ab88`; proposal row `149f0047134b44e4fd7ba83c9b58d23372f63e7a790ab88dd3293c4e18bd872b`; reviewed assignment `ef58102f07839746fd48cec60ff442c51b3367a8780d0ebc39be0a276b668159`.

### DEC-LEG-016

Question: Are physical-only differences (per-entry codecs, chunk groups, random access) compatibility notes rather than LOSSY or REFUSED outcomes, confirming the repo resolution of SPEC §15.2?

Review: Measure source chunk-group read cost only if it decides whether a physical note is material; semantic LOSSY/REFUSED classification remains binary.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-10 P `random_entry_latency_s`; OD-19 N Physical-only differences become compatibility notes unless a target actually cannot read or reconstruct the EAM tree

Pins: source row `7a04fb0261f327f70cc50266353e61124eb839b3f14ca067749c9983bbe11338`; proposal row `5952b999186537a47e29d1d90db240e9cf514061154e3d2e98c91f5fc619d97d`; reviewed assignment `1cbc9d6dd93e7132d4f91f492dc21842f76a874c3290928a89e3d444f6743b16`.

### DEC-LEG-017

Question: Are export receipts always produced, where do they live by default (sidecar file name, stdout, embedded in a native sidecar), may they accompany stdout targets, and do refused exports emit receipts?

Review: An export receipt location cannot be selected by M19.1 import acceptance.

Additional HC screens: HC-15. Required routes: FORMAL, HUMAN-FACING.

Dispositions: OD-20 N Receipt emission for success, refusal and stdout targets is binary completeness and transaction conformance; OD-25 N Default receipt discovery and audit usefulness require consumer workflow evidence

Pins: source row `642104acd28a1a1de672341eec9cfb3caaaa772a5ad055b1ab58850bcded0428`; proposal row `50341d8895ae4987bd8f9d56bef224ca3883cdf7a0be4f6628ff1c04494b555b`; reviewed assignment `11c95306e30b514ec3fe952bc6caf45f50f0df0489ae89731e09315843811af5`.

### DEC-LEG-018

Question: Should a new receipt version record strict re-import validation for bare targets, tool build and normalisation rule-set versions, export options, source PCR/digest, per-entry loss enumeration versus per-class counts, compatibility notes, and multiple result digests?

Review: One-million-entry receipt size and pipeline consumer needs force measured and human routes beyond formal schema review.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-05 P `side_data_bytes`; OD-20 N Receipt fields, strict re-import validation and loss enumeration are binary completeness/determinism checks

Pins: source row `5331922c26f28d79cde3fbe854c166521b131847dbb9b8822afbd4029b7f7c2c`; proposal row `6bc62604ad6983a7d720800a75974b82e3f60008f14ece9e27218c0c0997f416`; reviewed assignment `27b8801a473b075f3bd07069433bc43fc59b0d9d068a2d00d7cc7ee46bf23d39`.

### DEC-LEG-019

Question: Should export self-validation and strict re-import use limits derived from the source EAM (instead of default import policies), with failures reported as typed per-target REFUSED issues?

Review: Source-derived self-validation limits must permit legitimate large exports and refuse typed failures within measured memory and time.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-07 P `decode_wall_s`; OD-08 P `alloc_peak_bytes`; OD-19 P `export_acceptance_fraction`

Pins: source row `dadd183392f59d26ef9ec24941398999cd7fadd33ee938da66200a46d057529c`; proposal row `6ebde629690e32c3f23bc9e7406db914b4546821f61d1f587c7eb0f85e7b7d4d`; reviewed assignment `3a7a41751eb62ac104e2fe015f3bad7fd6c09901518756d984e5723559945546`.

### DEC-LEG-020

Question: Must strict tar import refuse members carrying any GNU.sparse.* pax keys (formats 0.0, 0.1, 1.0) until sparse projection is frozen, or project them into native sparse metadata?

Review: The source asks for format-specific fixtures and mapping, not a measured prevalence choice.

Additional HC screens: HC-17. Required routes: FORMAL.

Dispositions: OD-03 N Sparse projection must preserve logical holes and report representability or refuse, a binary fidelity condition in this fixture scope; OD-19 N GNU sparse variants require exact strict-import conformance cases rather than a graded corpus acceptance preference

Pins: source row `1ae542b2a4393f0b719ec3cb5bfd1789cf459eb825361c28765af49bc52d0f74`; proposal row `b06b104db192f4764ea6aba99252f2fc68becf4b1f69d5b326d800786c0fccad`; reviewed assignment `72513a159f86480f86d51b0bb0aedc45638c53c73d74d557e8ffa34238d76b8e`.

### DEC-LEG-021

Question: Should gzip ISIZE verification be fixed to compare length modulo 2^32 before v1, and with what regression evidence?

Review: A wrong length comparison is a correctness defect; the large-member vector is the required proof.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: FORMAL.

Dispositions: OD-19 N Gzip ISIZE modulo-2^32 and above-4-GiB re-import are exact conformance regressions, with no graded interoperability preference

Pins: source row `d52b9e72175e1cb8534d28c3b282a46d351d24a76412353da0386a106c990b03`; proposal row `40390922cf331d27aa7ed5089518cc1f0ab516d42215d28c2ab3926b04152f9d`; reviewed assignment `cd37bbeaf79d909da6fa3abf7a4d90f2de50eb16dfd458dd28080ed036c2f607`.

### DEC-LEG-022

Question: What default values ship in v1 for the adapter-specific import limits of tar, 7z, compressed streams and preservation (entries, observations, per-entry, per-folder and total decoded bytes, expansion ratios, LZMA dictionary, zstd window and decoder memory, header sizes, evidence and exact-source bytes), and do they admit 7-Zip ultra presets and other legitimate large archives (SPEC §25.1 #23)?

Review: Legacy default caps must be screened on real tail distributions and bombs, including false refusals, hostile cost, memory and time.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-07 P `decode_wall_s`; OD-08 P `alloc_peak_bytes`; OD-17 P `benign_false_refusal_fraction`; OD-19 P `import_acceptance_fraction`

Pins: source row `22d2bac3d4cf6f430acb476fee20f6e7be17783aaf7715623fe731910175eba1`; proposal row `f73dcaa3d80142429a1e366a85df5901fa1f24b665538001e3a8762f6194f18b`; reviewed assignment `69bdb469ff9d47743778bbdd79d5f8223acea416bdfcd6049db83d1ed915fe38`.

### DEC-LEG-023

Question: Should legacy adapters share one outcome-class mapping for malformed foreign structure and truncation (ZIP Corrupt/ZipStructureInvalid vs 7z Nonconforming/TruncatedStream) and one adapter-identifier namespace in ConversionProvenance ('zip-strict/v1', 'tar-strict/v1', 'entrybound/sevenz-strict-v1')?

Review: The source asks for exact strings and malformed-case assertions, not graded import coverage.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: FORMAL.

Dispositions: OD-04 N Unified malformed-structure reason taxonomy is a binary registry/conformance obligation, not a preference for maximal specificity; OD-19 N Malformed ZIP and 7z class mapping must match exact adapter cases and preserve refusal honesty

Pins: source row `9f44356822c5389ff5bfb76fd864e0368a0ea7317abcd6e02824beba6b575ead`; proposal row `fe5b0830232085628759d61cd8c62d8c1712633e7110222e7923486b1a96bdd2`; reviewed assignment `318ac3fba8291de7c56f0e89483eed290b0cbbc332b169e57f5a60f128e6a0b1`.

### DEC-LEG-024

Question: What evidence rule classifies each candidate legacy import adapter as CORE_V1, POST_V1_CORE, ECOSYSTEM or NOT_JUSTIFIED, and does strict 7z import remain core given SPEC names only ZIP, tar-family and stream codecs?

Review: Classifying strict 7z and other adapters needs measured prevalence, security/licence review and real workflow demand.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-19 N Candidate format fidelity and safety are exact adapter admission screens, while import acceptance does not measure prevalence or workflow demand; OD-22 N Licence and dependency support are binary and C1-C7 permanent-cost screens; OD-24 N Parser CVE and fuzz history enter safety and attack-surface cost rather than graded product utility; OD-26 B Choosing CORE_V1 by format prevalence and user workflow lacks a canonical adoption-demand selection instrument, role and band; OD-26 proxies are descriptive

Pins: source row `a2c5639edc7cac725df10539a78e0eebccd4d1949e34a9c67be0e50c92071bc8`; proposal row `1c1b26c0dfe5917e9b401d62ca2b87011d2ffe2f2e3a21d2ff6409329bb7239a`; reviewed assignment `f8cb20c12c8ac536ead1dcd14bf098e3508aa19b4191a27da8b3be67d5ceb21f`.

### DEC-LEG-025

Question: Should legacy import retain enough evidence to re-emit byte-identical legacy artifacts for tar and 7z (tar-split style header side data) beyond ZIP exact-source preservation?

Review: Preservation scope compares storage overhead and exactness beyond ZIP while respecting legacy source bytes.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-05 P `artifact_bytes`; OD-20 N Byte-identical source recovery or exact recomposition is binary migration/identity conformance; OD-25 N OCI and packaging demand is a human workflow claim, not a corpus-size result

Pins: source row `af9a6fd2918ca2933b96c3684f110fb00f6b5ae1fcc258283bb835bc1264c6e0`; proposal row `f5f95972c84e1ce942c44f152810b41fc3bc407e9716f900a349d8dca8a2b133`; reviewed assignment `bd64f39ba0a15c1b58b637658a171462b24fcfc17a8613ea44a7488802ea198c`.

### DEC-LEG-026

Question: Must v1 publish a single per-format capability and limitations matrix (import modes, refused features, export profiles and losses), and must it be generated from code refusal paths?

Review: A generated capability matrix is a truthfulness and drift obligation, not graded acceptance.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: FORMAL.

Dispositions: OD-19 N Per-format modes, refusals and limitations must match code refusal paths and conformance fixtures; OD-20 N Declared export losses and migration outcomes must be reported without omission

Pins: source row `1c7c6366389934bc0aa3fbe49118fba66b31409303fdfcd71dc7b6d97290ec3d`; proposal row `2bc511b2e7a0469b0e2787d71e02902ac2c99ac6d1d5bf4bd7201884ccce13a6`; reviewed assignment `5ad5d595fa2f4665d7a8778668338446983864c56698e8c230faea817279ab7c`.

### DEC-LEG-027

Question: Are the frozen export codec parameters (ZIP DEFLATE level 6 when smaller, gzip level 6, zstd level 9 with checksum, xz preset 6 single block CRC64, bzip2 900k) justified, or should new profile versions choose different levels?

Review: Frozen export codec parameters may change only under new profile IDs after size, encode/decode time and memory comparison. Current OD-19 export_acceptance_fraction is not a codec-level preference in the required evidence.

Additional HC screens: HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-05 P `artifact_bytes`; OD-06 P `encode_wall_s`; OD-07 P `decode_wall_s`; OD-08 P `alloc_peak_bytes`

Pins: source row `e10a7b06cab2510753a2d8a28a79adc69c687079624fba064aa5fcb9c7792a07`; proposal row `8910067377b8dd15578c98e195fe8dddc1efd69d6a7acf79de026449eb7878f6`; reviewed assignment `270343d889278e6c7aa6bd010fd9dac49908ef1e00ab166938134208abd41644`.

### DEC-LEG-028

Question: Which readers and platforms must each export profile open without errors or stray PaxHeaders files, and do tar dialect choices (pax when needed, PaxHeaders naming, no blocking-factor padding, ustar-first) change if they fail?

Review: The profile-specific Windows and Linux reader matrix tests real export acceptance and stray PaxHeaders output; dialect details remain pinned conformance conditions.

Additional HC screens: HC-07, HC-13, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 P `export_acceptance_fraction`

Pins: source row `519fa7e229228c9542021fb81dcab24d112d1a8b2cc6ed44074c90e8d3bf8115`; proposal row `3b14aa511ff6f7727b412644147a995148eb8151acdf65368d31be96121b70e6`; reviewed assignment `eb3d971375309ec3f3178afed112c6b8e2b469aa83c99370966955c1f525556a`.

### DEC-LEG-029

Question: Which additional export target profiles are required for stable v1 or post-v1 (tar/pax-v2, ZIP with Unix/platform extras, registry-ingest ZIP, 7z, OCI layer, cpio, standalone compressed streams, tar.lz/tar.lz4)?

Review: Additional export profiles need explicit consumer requirements, per-profile fidelity and reader interoperability before profile freeze.

Additional HC screens: HC-13, HC-16. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-19 P `export_acceptance_fraction`; OD-03 N Per-profile fidelity and loss/refusal classification are normative admission floors rather than a graded aggregate; OD-26 N Registry, distribution, CI and OCI requirement surveys describe demand but the adoption proxies are not decision primaries

Pins: source row `1ce8068562f664adcd776b03c999155f1fa00f4848ec7960a426756a831940b2`; proposal row `d94e3ae06e59749028c2d8c2ade662d317be0b02543afc0e0095f7dce1378afa`; reviewed assignment `55269d5162651be7932a6ab6a959e96bac290c09b24d3307f7491aada61a823b`.

### DEC-LEG-030

Question: Should convert import accept legacy input from stdin or other non-seekable sources for tar and compressed tar (buffered under the import budget or streamed), and must 7z and ZIP refuse non-seekable input?

Review: The question concerns stdin buffering/streaming and truthful format refusal; random-entry latency measures a different access pattern.

Additional HC screens: HC-12, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-08 P `alloc_peak_bytes`; OD-09 N Non-seekable tar acceptance and ZIP/7z refusal are binary streaming-capability claims; OD-17 N Import-budget enforcement is a binary pre-output resource floor, not declaration tightness; OD-19 N Which format accepts stdin is binary adapter conformance, not a graded corpus fraction; OD-25 N Pipe first-use failures and remedy comprehension require human evidence

Pins: source row `7531a4e9f79eb67f120a895fe6967da1bf2d7dcdeda3fffbaab725b78c422f0e`; proposal row `700b4f5658be3b75e9f458a926b0b0c3971279608cc03527f4878d50afffe88e`; reviewed assignment `faa5cc1022f8e035acc8ccdf88dc8bf3bcadba6b4634f2daf99b29ddb002765e`.

### DEC-LEG-031

Question: How must strict import treat SFX prefixes, inter-entry gaps, trailing data, concatenated archives, multi-disk ZIP and multi-volume 7z: refuse, record as evidence, or support?

Review: SFX, gaps, trailing bytes, concatenation and split volumes require format-specific prevalence and differential reader outcomes; hidden data/evasion remains a safety floor.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 B Selecting separate SFX, gap, trailing-data, concatenation and split-volume policies from class-specific prevalence and reader disagreement lacks a canonical per-class selection instrument, role and band; aggregate M19.1 cannot stand in

Pins: source row `dc79f1d7518f6fa79139b4bb13a67f8fec937ab9ffb02598eb5253d45181b706`; proposal row `188e0c271f7fc05736c1e58bef96bebd0d7c82096aca303e5c1a1e68d4cc2d45`; reviewed assignment `b62e8323f7c739bfbf87841f48dd6f3de05600aa3088b975a169b37e45d6e478`.

### DEC-LEG-032

Question: Should ZIP and 7z adapters project Unix symlinks (and reparse points) into EAM Symlink entries, matching tar import, or keep refusing special files?

Review: The archived link projection requires prevalence, platform restoration and link-target extraction safety review before widening accepted entry kinds.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-18 P `cross_platform_restore_fidelity_fraction`; OD-19 P `import_acceptance_fraction`; OD-03 N Symlink/reparse target mapping and declared loss are binary fidelity conditions

Pins: source row `2e4ddc76ba026dc1a639c6ab4517d296a6e9e0ded394edd282f3c5c1fbc18dcc`; proposal row `0786bea8cb54f7cf8e346d8659423b318fffbf86534f7154eeb82b56d49fd328`; reviewed assignment `ca4fc1957c43af9d117e088f1571e281e6518f1c7b0d5a3b7343f91e589a4736`.

### DEC-LEG-033

Question: What metadata do ZIP-synthesised ancestor directories receive, and must provenance/AUX distinguish synthesised from explicit directories so exports and diffs can reproduce or report the omission?

Review: ZIP ancestor synthesis must distinguish recovered metadata from invented directory attributes and test export outcomes against incumbents.

Additional HC screens: HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 P `export_acceptance_fraction`; OD-03 N Ancestor metadata and implicit-versus-explicit status are binary fidelity/LOSSY-reporting conditions; OD-20 N Provenance classification and exact re-export identity are binary migration/receipt claims

Pins: source row `8fb82721d059b116fff6e11fd86590c1e65c4911cea1c79bd5a4fcc3460b342c`; proposal row `c790d55144c5374dfa7e055c77582d29044c6b1c2c05d34da538b84aaec4c481`; reviewed assignment `ea3dcf95f860a8d492b0cc56bc29eb2471a493e14d75c40458d89969b04f96df`.

### DEC-LEG-034

Question: Is the Legacy Observation Model general and stable enough to freeze for v1: which parts (preserved wire records types 28-36, public adapter API, vocabularies, layer addressing) are stable, and does one schema serve ZIP, tar, 7z, transports and future adapters without becoming a universal facade?

Review: Map cpio and OCI whiteout cases on paper, enumerate deviations and preserve the frozen public adapter/wire boundary.

Additional HC screens: HC-13. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-21 N Independent clean-room decoding of LOM types 30-36 and schema stability are binary specification tests; OD-23 N Adapter-specific schema maintenance is permanent implementation cost under C1-C7, not a graded product primary

Pins: source row `b1fd93a8b57007bf4126b014d3d184a0fce7d522bf60616ea7a2455313a77e8c`; proposal row `bc36b3478500ff6f779d36dd44d850b6ce29d250a5b2802508124d70f6a4bfa7`; reviewed assignment `93a47de9978871181e158b9008e33e73450ce6b97a00a2ebbd5b0bb847877a84`.

### DEC-LEG-035

Question: For long-tail formats (ISO9660, CAB, LHA, XAR, RAR5), should Entrybound wrap libarchive in a sandbox, reimplement readers, or refuse?

Review: Long-tail wrap/reimplement/refuse selection requires prevalence, measured sandbox overhead and external decoder security evidence.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-07 P `decode_wall_s`; OD-08 P `alloc_peak_bytes`; OD-19 P `import_acceptance_fraction`; OD-22 N Sandbox packaging and platform support enter permanent dependency and integration cost; OD-24 N Parser CVEs and fuzz findings are safety/attack-surface screens

Pins: source row `fd15aeadef580af8199467a25fa758ab11ac960855f004dd9801725e744acc33`; proposal row `c5e70d563039851dd595e843792a4d52f19af6a6294f2dc74daf0421ff76f08e`; reviewed assignment `4c71554bb12e66a124d7ba6ed0a7e382737bd5335d825230508101146931f422`.

### DEC-LEG-036

Question: Should construction-equivalent values (POSIX mode matching the executable projection, uid/gid 0, whole-second or aligned mtimes) count as LOSSLESS, or is LOSSY-by-default for tar round trips and freshly packed trees acceptable?

Review: The question defines a LOSSLESS semantic boundary and requires user approval evidence before a changed equivalence rule can be frozen.

Additional HC screens: HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-19 N Share of exports needing explicit LOSSY approval is descriptive friction context; source-value equivalence and profile versioning are binary fidelity conditions; OD-20 N Construction-equivalent values and truthful loss approval are binary migration/receipt conformance; OD-25 N Approval friction is a human task claim, not a simulated-task primary

Pins: source row `6818dc64d8303a831251888e13188b9523dca9a836da4170e458729854964f42`; proposal row `5f956bbaf908f4901c79a9f7b6fe79a05b16a5439f50f79d2046ddfb6366b908`; reviewed assignment `33c1ddf737b7e58e63525b1181aada71ff1f046a7934ead0c65dbfac08950a3f`.

### DEC-LEG-037

Question: Which classes do additional compressed transports get (lzip, LZ4 frame, lzma-alone, zlib, Unix compress .Z, brotli), including tar.lz and tar.lz4 import and possible export?

Review: Compressed transport classes require .tar.lz/.tar.lz4 prevalence and a primary-source decoder/dependency audit.

Additional HC screens: none beyond the proposal and governed crosswalk. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-19 P `import_acceptance_fraction`; OD-22 N Decoder availability, integrity semantics and support burden are permanent dependency/safety screens

Pins: source row `55f5c6b185d3a3a79f8858fe5ca95fb86631458a7add46b8b042f52f4db3826a`; proposal row `72bfd979b41c7863c064cf36cabee3619fe805ba7def601e1b00c7d6239766c1`; reviewed assignment `73a3c6c5b8c05b749e89f041e57b771ce0be889ace809c6dca1f68a70142a507`.

### DEC-LEG-038

Question: Should migration reports emit FAILED on failed transactions (or drop the state), report per-target validation failures instead of aborting, set lossy_approved only for LOSSY targets, and record relative artifact names instead of machine-dependent display paths?

Review: Disk-full and rename-failure tests must demonstrate that reports do not claim success or hide target-level failure.

Additional HC screens: HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-20 N FAILED transaction states, per-target failures, relative paths and report reproducibility are binary receipt-truth conditions

Pins: source row `97acd02550ef6aa089c8207fa52bb8e3a602adb1c432e3b20cee145261c0d686`; proposal row `d1eba460642f2ac62bec36f0b7df689c1c941b0c117e0c4011f51251d0689156`; reviewed assignment `a35f39a69448531d3f4fc4fb8e17e0a639da7714d737419a39570050c329e05f`.

### DEC-LEG-039

Question: Which class does a Nix NAR import adapter get, and is a NAR export profile justified as a canonical-serialisation conformance check?

Review: Nix demand is human-facing, while NAR-to-EAM mapping and NarHash feasibility are formal/empirical conformance questions.

Additional HC screens: HC-13, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-19 N NAR class and export-profile admission depend on fidelity/round-trip conformance and user demand rather than a graded import fraction; OD-20 N NarHash derivability and canonical serialization identity are binary migration/determinism checks

Pins: source row `d168f76ba4ad4c8c8078e00735fac6a5e76ba779deecf60c209ff978947a70d1`; proposal row `7fef1f118f15bb05b14b41b27e1ae8dfda637a72db3eaa60b9d6ec6a73a5a093`; reviewed assignment `ce27e6f20ee73bf52792b526f92424140f251e640304ad29ad998e170bb201e3`.

### DEC-LEG-040

Question: Do physical or trust rewrites (repack, transcode, re-encryption, key add/remove, strip-provenance) append provenance records to a chain?

Review: A provenance-chain policy must account for every physical/trust rewrite and auditor value without changing LAI truth.

Additional HC screens: HC-15. Required routes: FORMAL, HUMAN-FACING.

Dispositions: OD-15 N Signature invalidation and chain tamper resistance are binary trust conditions; OD-20 N Rewrite provenance and identity continuity are binary migration/receipt conditions

Pins: source row `c2b55d7d4a1e6a3523ba910b666ca2819672b4593ba2efff165a03931aed5e15`; proposal row `66d90855d7bfcc6836ba69e120808e795ca92c034cb651dcf24b9aa98494f52f`; reviewed assignment `75a7bd11187f9e4d4133fe92a1d809a221f719ce5338b868c4a108b61cd450b5`.

### DEC-LEG-041

Question: Should Entrybound define a container-layer export profile with its own pinned tar dialect (sorted pax, densified sparse, no duplicates) or wait for OCI to pin a dialect?

Review: The OCI unpacker matrix measures acceptance; upstream image-spec roadmap is an external standards input.

Additional HC screens: HC-13, HC-16. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-19 P `export_acceptance_fraction`; OD-20 N Digest stability across builds and profile tar dialect pinning are binary export-identity conditions

Pins: source row `5a38b23f9cd5b0bdc320b145b2a23681567f09b23d682470d4efd54bbccbfd0f`; proposal row `c8c46efce316d0a13260f18a4031d31786622a81a7450440cfa9e7eeffe901db`; reviewed assignment `ceee8bf32b150c61424a03db8fd76974a1d59dedc7618e8df6ef8047ba726ff2`.

### DEC-LEG-042

Question: Which class does an OCI image-layer import adapter get, and how are whiteouts, OCI Windows pax keys and duplicate paths represented (EntryKind, metadata, refusal)?

Review: OCI layer whiteouts, Windows pax keys, duplicate paths and hardlinks need measured representation/fidelity plus workflow demand.

Additional HC screens: HC-13, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-18 P `cross_platform_restore_fidelity_fraction`; OD-19 P `import_acceptance_fraction`

Pins: source row `ee0c537edb26c3368b5a130631a0d2cfde02ba1eb89df40c2d8fa329f1a00eac`; proposal row `78bf2c5cd2c58594ecf25c5b1d7ea7fa28d0aef4fab69ad011f28af551981a41`; reviewed assignment `1316746196c81f6d79deaf76ea4e57c03d6251466444fc39fac79aa4878379fa`.

### DEC-LEG-043

Question: What pre-registered violation-rate bands and rollout rules govern per-rule strict defaults across tar, compressed-tar and 7z import (R2: under 0.1% reject with named error, escape hatch and producer-concentration check; 1-5% lenient with deviations; above 10% lint only; unaddressed 0.1-1% and 5-10% bands; warn before reject)?

Review: The 0.1-1% and 5-10% gaps and warn-before-reject state must be preregistered before corpus observations.

Additional HC screens: HC-03, HC-12. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-17 P `benign_false_refusal_fraction`; OD-19 B Per-rule strict violation rate, exact binomial uncertainty and producer-concentration rollout need a canonical per-rule instrument with committed role and band; aggregate import_acceptance_fraction cannot substitute

Pins: source row `fe715226ea6f7af080048357e0c2880c2b1475e75dd8f01a919b56dae9895b96`; proposal row `ee501c89fc272883d300056acf96c57050d012c5d3ea54909e513012a49ebeb0`; reviewed assignment `43aa17ccd24122764a999302704ea923acd64ec3f254da35836b6459819f97c3`.

### DEC-LEG-044

Question: Will Entrybound offer re-projection of preserved LOM evidence under a different compatibility profile or newer adapter without the original source, and how is the result's provenance versioned?

Review: Prototype re-projection must bind adapter version and observation evidence, rather than claim agreement through a metric that excludes disputed cases.

Additional HC screens: HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-20 N Adapter-version drift and chained provenance identity are binary record-truth checks; OD-19 B Re-projection agreement on preserved ambiguous observations versus direct import lacks a canonical disagreement-case instrument, role and band; M19.2 import_agreement_fraction excludes them

Pins: source row `466641b0b414d85e1880c545a94c6faef766c136dfe121b6747e159138084783`; proposal row `5fd8264bb0b99acd19c12bc916c1335b74e8f8372051162110bf37251f89e3f2`; reviewed assignment `e78ad1135a855f49bd1afe52dd7e004594b9122e1102eb5cd2b6d8399d184c14`.

### DEC-LEG-045

Question: How is a derivation chain (zip to eb to tar.zst) assembled and presented: export receipts embedding upstream conversion provenance, a receipt --chain command over supplied artifacts, or no chain in v1?

Review: Receipt embedding versus a supplied-artifact chain command needs supply-chain consumer requirements and tamper tests.

Additional HC screens: HC-15. Required routes: FORMAL, HUMAN-FACING.

Dispositions: OD-15 N Upstream receipt/artifact substitution and chain tampering are binary authenticity floors; OD-20 N Derivation-chain assembly and source-target identities are binary migration/receipt conformance

Pins: source row `a62d41a356ce95aac58fe67d934b715c75b7351592029d0d36425f8810586fb9`; proposal row `251fe4da0587e816668b05d4adccfd590460405b5e51e7f9416214bd6287a58c`; reviewed assignment `374f989f6723376c9394117f6875f6e80888a1be9cb8c851ed8a601099153f89`.

### DEC-LEG-046

Question: Should convert import offer a no-embed option or an external import receipt, and should repack --strip-provenance (LAI unchanged, AUX and signatures invalidated) ship in v1?

Review: Privacy need and warning usability govern the option; frozen encrypted/provenance semantics cannot be silently altered.

Additional HC screens: HC-15. Required routes: EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-15 N Stripping AUX or removing keys invalidates signatures under a binary trust rule; OD-16 N Source digests and resolution details require a field-level leakage review, not a proxy privacy metric; OD-20 N No-embed/external receipt and LAI-stable strip-provenance semantics are binary migration/identity rules; OD-25 N Signature-invalidation comprehension requires human workflow evidence

Pins: source row `a72f1e5ef01bf20b9759779c35a6152f6fbdbed2d0c949bf6ea70683d40e629c`; proposal row `381a4cfc5a3e35c01323a1c3e3e473c785ddaaef7226667ac9da0d9824cbc1fd`; reviewed assignment `21bfadfc1d741c11f88bd1ffe442abf590689652d3798d52ebe1776e21be10ca`.

### DEC-LEG-047

Question: Is a RAR5 decode-only adapter legally, securely and practically justified, and if so must it refuse unknown records, map PUA names, bound dictionary sizes and report unauthenticated encryption?

Review: RAR5 admission needs primary licence/legal review, prevalence and hostile 64-GB dictionary refusal, not resource-declaration tightness.

Additional HC screens: HC-03, HC-15, HC-18. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-17 P `refusal_alloc_peak_bytes`; OD-19 P `import_acceptance_fraction`; OD-22 N UnRAR licensing and decoder support are legal/dependency screens; OD-24 N CVE/fuzz record and unknown-record refusal are security screens

Pins: source row `1d53cf84bba60aadc2597e11b4fac8df028f0140e2ec7788a81fab89fb584624`; proposal row `6e75a0a8c9c41efbda100df407336fb655f92b6a5079e5ae34444bf76945938b`; reviewed assignment `f19bb815c14d8c56b63dfd89211be7293b4a66da05ea00b35f0d9ce3b586aa66`.

### DEC-LEG-048

Question: Which normative canonical JSON rules govern receipts, migration reports and sidecar reports (RFC 8785 JCS, a documented custom grammar with fixed field order, or CBOR deterministic encoding)?

Review: The choice between JCS, custom JSON and deterministic CBOR requires exact current-byte compatibility and Unicode/control-character vectors.

Additional HC screens: HC-12, HC-13, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-20 N Canonical receipt/report bytes and cross-implementation reproduction are binary determinism/identity checks; OD-21 N Independent grammar implementation and Unicode fixture agreement are binary clean-room specification checks

Pins: source row `968515c0941592fbb33171e7cc4775ad3f2d80b6345eaab71ca31f87a9483403`; proposal row `f554c1e7fd01913dfae4d21325fe152697026299a17bd8187697145a969edf9f`; reviewed assignment `6c7fe5bcd84d9bd49a5dbfbbd8d8e98fadf963dd95e43cfc53a820a1f0a7d90c`.

### DEC-LEG-049

Question: Should 7z entries lacking MTime omit core.mtime and report it unavailable, instead of the status quo epoch placeholder marked non-restorable yet reported as captured in the FidelityReport?

Review: The epoch placeholder cannot be called captured metadata without confirming model allowance and loss reporting.

Additional HC screens: HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-03 N Absent 7z MTime and LAI/FidelityReport truth are binary captured-versus-synthesized metadata conditions; OD-19 N MTime-less archive prevalence is descriptive context, not a graded import preference

Pins: source row `d34f01c2c5799092da351e5d468c79280a38925aaa0f07ef64836eaadee1145d`; proposal row `f7ba83cdba470832ff3fa96efb8a1d0fdf303b8d575294332a188c5ec4e9dccb`; reviewed assignment `efce44ef29a1a7ac133afe1db2218cbc0dc6600758f5b848597892fa7bf2a931`.

### DEC-LEG-050

Question: Are 7z compatibility profiles needed (for example 7-Zip 26.x, p7zip 16.02, py7zr, libarchive), and which divergences (backslash names, SFX offsets, anti items, trailing data, CRC-less streams) could they resolve?

Review: Distinct 7z runtime divergence cases and real strict-refusal prevalence inform whether profiles improve useful acceptance; exact LOM evidence is a prerequisite.

Additional HC screens: HC-13. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 B Selecting 7z compatibility profiles for conflicting runtime outcomes needs a registered disputed-case agreement instrument, role and band; M19.2 excludes reader disagreements and M19.1 only measures format acceptance

Pins: source row `308e7cf8dff7a014f9412e6cc5cd6c6af661c480ccbb54fc0d1fae0b82218926`; proposal row `abda9b895f311cd1178b9765fd1fcca4a3dbe638cdc43709312404546caeb1b7`; reviewed assignment `eebbdc5ceaad8581fed4efce53ab662fd769871b6d346d7ed957e41d5d2f089a`.

### DEC-LEG-051

Question: Does 7z export remain NOT_JUSTIFIED for v1, become a post-v1 profile, or stay a permanent non-goal?

Review: Demand for .7z deliverables and a multi-reader export matrix are needed before changing NOT_JUSTIFIED status.

Additional HC screens: HC-13, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-19 P `export_acceptance_fraction`; OD-22 N LZMA2 writer determinism and header-writer maintenance are permanent costs, not graded product utility

Pins: source row `710706d1c5288cba861dc90698a15d3a2e27d8bb6b3500ef8ede780a8a81db51`; proposal row `2fc2b95396531b2f8fd0e42341706143bb7f4c11a8445ceda2e3ffaf3cdbaddb`; reviewed assignment `959f6226797700ee405859f0bb718707768c40a4ea138f920ced804576c15652`.

### DEC-LEG-052

Question: Which 7z coders and filters are in v1 import scope beyond COPY/LZMA/LZMA2/BZip2/DEFLATE/Delta/x86-BCJ (ARM64, RISCV, BCJ2, PPMd, fork-only Zstd 04F71101/Brotli/LZ4), and is AES-encrypted 7z (with unconfirmed KDF iteration count) supported?

Review: Coder/filter distribution and differential decode support scope; AES cannot be admitted on an unconfirmed KDF rule.

Additional HC screens: HC-14, HC-16. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-19 P `import_acceptance_fraction`; OD-15 N 7z AES KDF iteration and authenticated-output truth require primary-source crypto review and binary trust checks; OD-22 N Decoder dependency support is a permanent cost/supply-chain screen; OD-24 N Decoder fuzz and malformed-filter safety are attack-surface screens

Pins: source row `3d97b4fa9ce2d9468913cbb868415f8ee661069bffc64151a39e7bc77ee51994`; proposal row `45be6475e9a4b44336a4c7834d2a5a75b489f5094b6821ac09b76c64368e11d6`; reviewed assignment `2c5ce47c9c884b4f7c7e864bbb2015df33587d5eaf2aaed3867c8fd779d95cf8`.

### DEC-LEG-053

Question: Must 7z LOM observations carry exact source extents and raw source bytes (ZIP parity), record structural contradictions as classified LOM conflicts rather than diagnostics, and give decoded-header evidence its own layer coordinates?

Review: The 7z LOM parity question needs evidence-exactness fixtures and measured record-size impact on large archives.

Additional HC screens: HC-13, HC-15, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-05 P `side_data_bytes`; OD-20 N Exact source extents, raw bytes and decoded-header layer coordinates are binary provenance-evidence truth conditions; OD-21 N Independent observation decoding and contradiction classification are binary specification checks

Pins: source row `27d7cc6ba59071874d3f5af21c3e35f0f98bf66d4577211be8c93340efb1f81a`; proposal row `71a14f13c418af07a0ca6f2e510daf972cd189cb081bb707e3a9ad70c7d6c1f8`; reviewed assignment `f4a98110cabfe00d2c22dbb1809b832ec9bce6d4b9d2285d0e74a08ec7780544`.

### DEC-LEG-054

Question: How are 7z names with literal backslashes handled: split as separators (status quo), refused as ambiguous, split only for Windows-origin archives, split with a recorded resolution, or per profile?

Review: OS-origin indicators and corpus prevalence must be measured while exact refused/split semantics remain a safety and path-identity floor.

Additional HC screens: HC-12, HC-13. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 B Literal-backslash 7z name resolution across Linux and Windows needs a registered runtime-disagreement selection instrument, role and band; M19.2 excludes conflicting cases

Pins: source row `3e397dcb180433620bda1e4cf2642fcbd4557d5f6390cabb8f360c70d2f38af9`; proposal row `90cdf4e31235ff81f1ad9d77a02766486283bda90afa59027ed64c3819c9ce14`; reviewed assignment `1defcac82c0d7d367d0a37800f45fadf4d1249a9619c1e78801a37dea536b03a`.

### DEC-LEG-055

Question: Must the 7-Zip FILE_ATTRIBUTE_UNIX_EXTENSION flag (0x8000) gate interpretation of high-word POSIX mode bits in 7z attributes?

Review: Source-defined 0x8000 interpretation, Windows placeholder fixtures, false executable projection and false refusal must all agree before changing mode mapping.

Additional HC screens: HC-08, HC-17. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-03 P `restore_fidelity_fraction`; OD-17 P `benign_false_refusal_fraction`; OD-19 N 7z attribute-flag handling is exact importer conformance rather than a graded format-coverage choice

Pins: source row `4c67e261e1c029b201dc92397bb71e36c6317332782932da8b6c8d8214253664`; proposal row `88ae75bf9876ebb9a4f4f0d9f62292d6e9d00e75f068206e6b8c64922ef66773`; reviewed assignment `4c8e8470050944b536ad4cd12bb761b7e03f0d77398145f1969d5ebf9af49650`.

### DEC-LEG-056

Question: Which sidecar workflow is stable v1: source formats and modes (tar/7z compat or preserve), dry-run, encryption, normative <artifact>.eb naming for discovery, snapshot stability mechanism, preservation overhead, and relation to the SPEC content-free Sidecar role?

Review: Measure preservation overhead and test concurrent modification, encryption composition and normative artifact naming against a content-free Sidecar role.

Additional HC screens: HC-07, HC-13. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-05 P `artifact_bytes`; OD-20 N Snapshot stability, named discovery and signed/encrypted sidecar identity are binary preservation and receipt conditions; OD-25 N Verifier discovery workflow needs human evidence, not a simulated task-success primary

Pins: source row `6abd826f38e214fe977d943117198fe9b35ec1c1fb14a2d1a121bc989156b181`; proposal row `76976fd7f34561be2957868c48b8c178e3bc8fe7112272370d1f588c41756d73`; reviewed assignment `634e8de87f6dbf2d9808d5693cff1b45c49872433762d37d87f247e32e78cca8`.

### DEC-LEG-057

Question: Which class does a squashfs import (and possibly export) adapter get given its uid/gid, name and inode limits and past unsquashfs traversal bugs?

Review: Squashfs class requires name/inode/uid fidelity, traversal fixtures and actual user demand before widening format scope.

Additional HC screens: HC-12, HC-13, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-03 P `restore_fidelity_fraction`; OD-19 P `import_acceptance_fraction`; OD-25 N AppImage, snap and live-image demand is a human workflow claim

Pins: source row `177acdcd7bd347e9b69393bfd5dec8b50d98d9fbec8f73fa502692913c5f47db`; proposal row `cef6cad278f0062461cf41721147d588363fa1098b2218829224a04724ac1005`; reviewed assignment `d99009f411bd26fdea40e0a664097251b9cbd005d3bee6d0069a690d3fb9f181`.

### DEC-LEG-058

Question: Do measured real-world strict refusal rates confirm strict as the default for legacy import, per rule?

Review: The 45,000-artifact scan, descriptor JAR replay and tail sampling must be preregistered for each rule before observing data.

Additional HC screens: HC-03, HC-12. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 B Per-rule real-world strict refusal rates and uncertainty lack a canonical per-rule selection instrument with committed role and band; aggregate M19.1 import_acceptance_fraction cannot stand in

Pins: source row `df3d312cdf5d92d9220df79e50a3d46ab9ae3fd1a1503326f6768ced102369c4`; proposal row `8e40735c140bea60a96ea8574ad25d81c1cc56fc9c508598f683f646dcee4964`; reviewed assignment `149e1b2067ac029008f5c36e1fe7c803f23fe2c0a6b93c41f003a95f67ae6f8a`.

### DEC-LEG-059

Question: Must tar, 7z and wrapper import policies add a conversion-provenance byte bound and reconcile observation caps with entry caps (tar's 19 observations per header make 8,000,000 observations bind near 420k entries; unknown 7z FilesInfo bytes are copied per file), or count shared observations differently?

Review: Measure provenance growth on 500k/1M-entry tar and 100k-file 7z fixtures, including adversarial replicated FilesInfo evidence.

Additional HC screens: HC-12, HC-13. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-05 P `side_data_bytes`; OD-17 P `benign_false_refusal_fraction`; OD-19 N Parser observation and entry caps are binary bound-admission rules, not general legacy-format acceptance

Pins: source row `49312910f96d1e68daa4bfec2915ea62b7edd45f25c1e8a43e91f3400cddcf8c`; proposal row `5026cf75ad5a8ae5cddd9b52813c128e7049d3079e0edb56d9ad0a11b5e8866e`; reviewed assignment `4b4abeca083291303702540369fc02afff241b90889f5964162747550b858b41`.

### DEC-LEG-060

Question: Should preservation (exact source plus structured LOM evidence under feature 0x4000) be available for tar, compressed-stream and 7z sources via strict+preserve, format-specific compat profiles, or not at all in v1?

Review: Compare tar/compressed/7z preserved size while closing 7z evidence-exactness and encryption/sign/repack composition gaps.

Additional HC screens: HC-13, HC-14, HC-15, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-05 P `artifact_bytes`; OD-20 N Exact-source preservation, LOM completeness and LAI/crypto composition are binary identity and trust claims; OD-25 N Legacy-audit and dual-publishing need must be established with workflow evidence

Pins: source row `9ceef10b775dbe8fd68576c5a8206c8138dbce371e39f8c23b6975681c6bbfc5`; proposal row `e92515647ec2f97849ac53cf49f451b9b328a47f9c8586aa393fc6dd30bb3d46`; reviewed assignment `ba5cc48514809d12163f78e17a0456d7830c08282981efed6518ea0902f0dadf`.

### DEC-LEG-061

Question: What outcome (accept, refuse with reason code, recorded conflict) and expectation class does Entrybound assign the non-path adversarial tar cases (EB-T001 duplicates, T002 hardlink before target, T005/T009/T010 pax size desync, T006/T012 GNU long names, T007/T008/T013 end-marker cases), especially where Research III disagrees with tar-strict/v1 (T007 ACCEPTABLE vs refused; T012 FORBIDDEN vs Refinement)?

Review: T007/T012 disagreements require an explicit authority disposition and regenerated fixtures, not an aggregate acceptance score.

Additional HC screens: HC-03, HC-12, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-04 N Each adversarial tar case must produce the specified reason code or classified conflict, an exact taxonomy check; OD-19 N Accept/refuse outcome against EB-T and incumbents is binary case conformance; prevalence informs scope only

Pins: source row `496accf02e33dfc603d99a701d01100825571f911050f19207a1970d6c651bcf`; proposal row `9348bcab546922f31f452a5f3a6da78d5fb511d4908293563aa8c01941ed5c95`; reviewed assignment `2ec50bb12d50fbfc6ec3c1857fe860722e3872b9058ad67ef3b37c64aba5e19c`.

### DEC-LEG-062

Question: When tar authorities disagree (pax path, size or linkpath vs ustar fields; GNU long name vs ustar name; local vs global pax), is the override a Refinement only after validating a binding (ustar value is a truncation or placeholder) or always; and which disagreements (EB-T012 unrelated names, EB-T005/T009/T010 size desync, global pax path) must be Divergence refusals?

Review: Validate binding/truncation before accepting overrides; replay size desync and unrelated-name cases including the cited CVE class.

Additional HC screens: HC-03, HC-12, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-04 N Refinement versus Divergence classification for pax/GNU and ustar conflicts is an exact semantic rule; OD-19 N Tail prevalence and differential readers contextualize the rule but do not grade correctness

Pins: source row `a513d9c66f09ab0dcaf02f996d8b648f5f246a24985d7cd1b89d7cca4dcb48f8`; proposal row `0d3314400df3d541515e31842dd29518c551c03227528450e572317cab28a12a`; reviewed assignment `af35e59bf862b3a5eb2f4a55afa45f43505dc1940b5aea8e51e93ca1ac435ef4`.

### DEC-LEG-063

Question: Which versioned tar compatibility profiles, if any, are defined for v1, against which pinned runtimes (GNU tar, bsdtar/libarchive, Python tarfile, Go archive/tar, Rust tar crates, busybox), and which divergent behaviours (duplicates, end markers, './', forward hardlinks, pax/GNU conflicts) may they resolve?

Review: Runtime matrix, per-behaviour prevalence, strict refusal and profile upkeep must be evaluated separately; restore_fidelity_fraction does not prove runtime-faithful projection.

Additional HC screens: HC-12, HC-13, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-22 N Per-profile maintenance remains a C1-C7 permanent cost, not graded product utility; OD-19 B Pinned-runtime outcome agreement for divergent tar profiles lacks a canonical selection instrument on the disputed cases; M19.2 excludes reader disagreements

Pins: source row `b3f279d414799c39bfa2d60a66e21d9374678672e6161a862e633c43d0cc27bb`; proposal row `f43893d12364e0cde844ed466187108a7f18e45f66a92872a1046a299c3fac50`; reviewed assignment `6ae23e8c9cabad35085333fdf6a00f82c09153afffb675a1504875fbb9803f01`.

### DEC-LEG-064

Question: Which tar dialects and header variants must strict import accept for v1 beyond ustar, pax, GNU L/K and base-256: v7 without magic, old-GNU and GNU incremental layouts (atime/ctime in prefix bytes), star/xstar and Solaris 'X' headers, volume labels and multi-volume records; and must the parser dispatch on magic?

Review: V7, old-GNU, incremental, star and Solaris header coverage requires dialect prevalence, exact fixtures and magic-dispatch safety.

Additional HC screens: HC-15, HC-18. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 P `import_acceptance_fraction`; OD-24 N Added dialect fuzz outcomes are parser-safety gates, not graded utility

Pins: source row `f1a156dac9abfdcff676e9ba6c859cb28375e4665d88405d4953067bcea0fe8c`; proposal row `a82fb6df39ea25063e85bf0dc0bdccc49808afb6a8d8a2474e8eaa870ed0e793`; reviewed assignment `4b3905c531c86f52742bf20a7bbe5252ab6ac7b09ca7b91e7bf698ed72f548fb`.

### DEC-LEG-065

Question: Should strict or a compatibility profile canonicalise a leading "./" member prefix and the "." root entry (and V7 trailing-slash directories) instead of refusing them?

Review: Leading-dot and V7 trailing-slash acceptance can improve legitimate import coverage only if normalization collision tests preserve path identity.

Additional HC screens: HC-03, HC-12. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 P `import_acceptance_fraction`

Pins: source row `4e365b3785ac7d9d971f6e70a22db358463bb4d6e7a803d0d4440598739fd78c`; proposal row `4b9f908d41473cd172697896772f4501a63a4c5319a9e47494178c6e7574f654`; reviewed assignment `1d426343dc47f7f735768ee4584bf2849444694b852f8f50472cf7432f5c66c2`.

### DEC-LEG-066

Question: What are the strict and profile rules for tar hardlinks that precede their target, chain to other hardlinks, target symlinks or directories, name repeated targets, or carry data, and what bounds unresolved-link state?

Review: Forward/chained hardlink prevalence and worst-case unresolved-reference memory decide bounded profile support after cross-reader differential tests.

Additional HC screens: HC-08, HC-12, HC-13. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-08 P `alloc_peak_bytes`; OD-19 P `import_acceptance_fraction`; OD-03 N Hardlink target kinds and content identity are exact restore-fidelity conditions

Pins: source row `dc92b36073914d732dc3334a786f3538801ea963546c61d3dd17ab71622bc5d6`; proposal row `3b4967a7a623dcd3a2ef3a8f7eedd633d0b34e61e7207eb14c647f38c4a31600`; reviewed assignment `bd492a38c9227e3a1c0cd6e9df4d62a2f2831d95db94d1d546f66b6dd8d27f4b`.

### DEC-LEG-067

Question: Should strict tar import refuse non-UTF-8 member names (hdrcharset=BINARY, legacy filesystems) or offer an explicit policy mapping them to declared encodings?

Review: Non-UTF-8 name prevalence may justify a declared-encoding profile; exact raw-name mapping and collision refusal remain identity floors.

Additional HC screens: HC-03, HC-12. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 P `import_acceptance_fraction`

Pins: source row `f25ec84656ee519d3f834d7f4dd02efd42645d131364742a4058fa80655b0bc8`; proposal row `9917b4c63cff1b686509b320eb260c6a2ddddcc11189b82e511397c696f061e2`; reviewed assignment `1d426343dc47f7f735768ee4584bf2849444694b852f8f50472cf7432f5c66c2`.

### DEC-LEG-068

Question: If a tar/pax-v2 profile is added, which semantics does it carry and how: symlinks, hardlink records versus copies, uid/gid and names (octal, base-256 or pax), xattrs (SCHILY.xattr or LIBARCHIVE.xattr), ACLs (SCHILY.acl), birthtime, Windows attributes and descriptors (OCI MSWINDOWS keys), sparse (GNU PAX 1.0 or densify)?

Review: Export profile selection needs per-key reader acceptance and round-trip metadata fidelity, including ACL, xattr, sparse and Windows keys.

Additional HC screens: HC-13, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-03 P `restore_fidelity_fraction`; OD-18 P `cross_platform_restore_fidelity_fraction`; OD-19 P `export_acceptance_fraction`; OD-01 N Tar/pax-v2 field encodings must preserve frozen canonical metadata semantics, a binary conformance condition

Pins: source row `44798637109e038ae0003e45d70247ae9235d6d3410c5f862d9a975426a663f9`; proposal row `544390133b5109a8455b3e705a1b20e4f984a0fb4c52df44206528f61fc3ba95`; reviewed assignment `c31f55732764c19174ff0c64164bfa5cb54eb537c2afbc467e64037b7c8b64b1`.

### DEC-LEG-069

Question: Should a versioned tar/compressed-tar compatibility profile tolerate single-zero-block termination, trailing garbage, unterminated tars, zero padding after gzip members, and last-member-wins duplicates?

Review: Padded/unterminated tar prevalence and pinned GNU outcomes support only an explicit versioned tolerance profile with duplicate-safety checks.

Additional HC screens: HC-07, HC-13. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 B A tolerance profile for termination, padding, trailing data and duplicates needs a registered per-behaviour disputed-runtime agreement instrument, role and band; M19.1 aggregate acceptance cannot establish faithful GNU projection

Pins: source row `2a3be4cd45ddf99f6a50a52729267e58820e4052e08ca173a1a6cfb047531b34`; proposal row `ec4e0b061bd4604da2e083f4a2df7f1a944a4a3945319127c37653f060a256ac`; reviewed assignment `2c0c988ffe2ebbb42c20cdf10d07ca564e770c5f1e28db306f984ccd7f3e83a2`.

### DEC-LEG-070

Question: What rule classifies two timestamp (and flag) assertions as compatible Refinement versus Divergence (DOS local 2 s time versus UTC extended timestamps), should compat profiles model flag and DOS-time divergences, and what version granularity identifies runtimes?

Review: Measure DOS/extended mtime and local/central flag differences at pinned patch versions, preserving exact classification rules.

Additional HC screens: HC-08, HC-17. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 B Timestamp/flag Refinement versus Divergence across runtime versions lacks a canonical disputed-case agreement instrument with committed role and band; M19.2 excludes reader disagreements

Pins: source row `0fd94f61d03451f18ef670405c93de87e2656c455f907a208560e571c999ab21`; proposal row `2014192d9f353024e304d82464f4bdd3663fea73f023a70243f68ac16d563dc0`; reviewed assignment `fa4d707d1ad6e36b6e73ab142d1f5bdd9f16ea11ad948cdde15d455142f98890`.

### DEC-LEG-071

Question: Do transparent legacy read adapters (one library interface over .eb, .zip, .tar.*) ship in stable v1, and how are strict default, source format, import policy and untrusted lenient streaming results typed?

Review: A transparent read interface must prove bounded import-beyond-RAM behavior and refuse untrusted lenient results truthfully.

Additional HC screens: HC-06, HC-12, HC-13. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-08 P `alloc_peak_bytes`; OD-09 N Typed streaming availability and strict-default behavior are binary API claims; OD-19 N Supported formats and explicit source-policy typing are conformance claims, not corpus-acceptance preferences; OD-25 N Library consumer demand and API usability require human-facing evidence

Pins: source row `5059420420be55f134ce8eac2392b8865ae65ceab12a5704f569f58ea0f4146a`; proposal row `27b015d66b57f19b7163b5d7608b9eef58f6c32c0ad5699a7c80d4a7027fddc3`; reviewed assignment `3212753863e293adc146812b2455058ab253ea09ec23b67656656bffc8f6c17b`.

### DEC-LEG-072

Question: Must import compose nested transports (tar.gz inside a transport, gzip of xz) with per-layer budgets, and should detection order or an explicit structurally valid --from override weak magic collisions (a tar whose first name starts 'BZh9' is detected as bzip2 and unimportable)?

Review: Nested transport prevalence and BZh/PK prefix collision fixtures decide support only after deterministic detection and explicit --from semantics.

Additional HC screens: HC-05, HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 P `import_acceptance_fraction`; OD-17 N Per-layer budgets and magic-collision refusal are binary pre-output safety rules, not declaration tightness

Pins: source row `f8e3a3829be623b721c69c97c83de9337a360bb01b1b9701f95c6a0e002da769`; proposal row `29649f3e8937a3bc7595246d1aa06e7a2ce969a40aea25e641f6bda7c49d0a4d`; reviewed assignment `0d286c2d89914e81ab511537f7d138da0420879af31b53f7d2bca23bc6603128`.

### DEC-LEG-073

Question: Which class does a WIM/ESD adapter get, and must encrypted ESDs be refused outright given all evidence derives from wimlib?

Review: Independent [MS-WIM] reconciliation must precede reliance on wimlib; LZMS memory bounds and workflow demand govern class.

Additional HC screens: HC-05, HC-13, HC-15, HC-16. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-08 P `alloc_peak_bytes`; OD-19 P `import_acceptance_fraction`; OD-15 N Encrypted ESD refusal and untrusted decoder truth are binary trust conditions; OD-25 N Windows deployment demand is a human workflow claim

Pins: source row `e5ff2212a4af125693b7bec2223075a9b001da82a23f0d4a51cda3384951ebbb`; proposal row `41a1c7a500e9fb147c5785f5cbf48daeaba2ce3356644ca6277540ad811d6487`; reviewed assignment `2db692b9b6bc671ed5d508a7bc8021056d2731da0bdf30bfb6edd9613f255c52`.

### DEC-LEG-074

Question: When a decoded compressed-stream child is ambiguous (e.g. all-zero payload that looks like an empty tar), must the caller explicitly choose tar versus standalone projection?

Review: The 1-MiB-zero child and broader non-tar corpus must be tested with explicit caller choice and deterministic refusal semantics.

Additional HC screens: HC-08, HC-12. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 B False tar detection among compressed non-tar children lacks a canonical classifier-specific selection instrument, role and band; M19.1 format acceptance is not false detection

Pins: source row `52ca4c5c5efb9c7cc71cb1ce828f9f899ce5df99f4df4a8626814151b9599d44`; proposal row `4e37ca4fd3baa6165613a49225b39da3ce459fd5b3bca4fae05f5ff9e83d96b5`; reviewed assignment `9ae8d75b675ecbb35fd3c875b8092f0a7ab697d0825b69fa3beed1b009adcb6c`.

### DEC-LEG-075

Question: Which xz filters (LZMA1/2, delta, BCJ x86, PowerPC, IA-64, ARM, ARM-Thumb, ARM64, SPARC, RISC-V) and check types (none, CRC32, CRC64, SHA-256) does v1 accept, and is SPEC's preserved block index with per-block integrity carried into native digests retained or withdrawn?

Review: Enumerate pinned lzma-rust2 filters/checks and .tar.xz usage before retaining or withdrawing the SPEC block-index claim.

Additional HC screens: HC-06, HC-13. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-19 P `import_acceptance_fraction`; OD-15 N Transport check-type integrity and native-digest preservation are binary verification truths; OD-22 N Filter/check decoder support and audit are dependency costs

Pins: source row `2a88143dd491590cc4353c75168529dcc80bad0dcac2f80a6e4b6cee520a078d`; proposal row `8e79e47051a335a48c67c320ce95bffabd8e18122ba04b24a2a500b52d6c3c42`; reviewed assignment `d27286ce3e5f8fd221119d8ca0b45df1b497c147e4020591517947a5bc957ba8`.

### DEC-LEG-076

Question: Should ZIP import reconcile __MACOSX/ AppleDouble members (ditto --sequesterRsrc) into native resource forks/xattrs, keep them as ordinary entries (status quo), or report them, and should ZIP export emit them?

Review: Finder/ditto samples and the platform fork/xattr rule govern whether __MACOSX remains ordinary entries or maps to metadata.

Additional HC screens: HC-08, HC-16, HC-17. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-03 P `restore_fidelity_fraction`; OD-19 N AppleDouble prevalence and ZIP format acceptance contextualize whether reconciliation is needed; OD-20 N Sidecar/resource-fork mapping and export identity are binary preservation conditions

Pins: source row `16b4559d15a3bb1a3d17da7142f5d165bc51fef6d742e99e19a5dcd9c52d8479`; proposal row `eecaca86c5fdf252349e1c6936c2bfcdd394a3d96dc531a76c33f7ac2a7759fa`; reviewed assignment `ef244c09f95a54c20180c9123169aabbecf297ce9b20435959969877c2693318`.

### DEC-LEG-077

Question: Should ZIP import keep refusing every backslash in names (status quo), interpret backslash as a separator (as 7z import does), do so only under a named compat profile, or preserve it as a literal component character; and should ZIP refuse ':' in components like 7z?

Review: Producer prevalence and runtime extraction must distinguish literal characters, separators and unsafe collisions before a profile selects projection.

Additional HC screens: HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 B Backslash/colon name behavior and normalization collisions across pinned Windows/Linux ZIP readers lack a registered disputed-case agreement instrument; M19.2 excludes conflicting cases

Pins: source row `212d8d01680fa9f3c74785f4fdf0aec3b4ca8d00d1a8c16aa9f3ca4cbdc9cc0c`; proposal row `5640ac94e73f2d52e270a41cbccbc744304794f8cbc3e43515d33604bbee0bd3`; reviewed assignment `7eb0b276c41e5f3a24a59d925d7dca7f0a62f763de547701bd108f75af5f6409`.

### DEC-LEG-078

Question: When a runtime's observed content selection differs from other readings (CPython truncating to declared size with a colliding CRC, Java ZipFile returning the full stream, bsdtar concatenating duplicates), should the profile reproduce that selection, refuse it as a documented safety override, or import it only with an explicit recorded Divergence?

Review: Pinned CPython, Java and bsdtar outcomes plus security review must govern each source-content conflict; generic acceptance cannot decide it.

Additional HC screens: HC-08, HC-17. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-15 N Content-substitution attack resistance is a binary integrity floor; OD-25 N Compat users' expectation of faithful output versus explained refusal needs human evidence; OD-19 B Selecting runtime-faithful content under disputed CRC/declared-size cases lacks a canonical conflict-case agreement instrument, role and band

Pins: source row `806f91c35876999e598d04f97ad08acdffce1d035401906d469c7a394b35d49d`; proposal row `d9dc0c5d18d185d6234a41a3d3181e3a75c9193bf1840298e551af1288e27e9c`; reviewed assignment `cfeebe9c921b9ae3a1263ce4614e93a861c317c342cf27eafafffafafb261280`.

### DEC-LEG-079

Question: Under compat and preserve, which duplicate member does each profile project (central vs local order, first vs last), should duplicates be refused instead, and what exactly is a refused 'extraction collision' (case-fold, Unicode normalisation) versus a projected duplicate?

Review: Replay reordered central-directory fixtures against stream and disk extractors; resolve Research III inconsistency before profile freeze.

Additional HC screens: HC-08, HC-14. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-18 N Case-fold and Unicode extraction collisions are platform safety/fidelity floors, not a graded duplicate preference; OD-19 B First/last duplicate projection and runtime-specific order disagreement need a canonical disputed-case agreement instrument with role and band

Pins: source row `f405497fe2d180f2db57a7424c4cc822a2e3c847ca0962dd2c3f9ef7825fbdaf`; proposal row `5d39471580eb0496e58f1a7f5f8e43e64bf82731d6adf30c16d1d4c9aa672a30`; reviewed assignment `1125208d6711f4ec04cea0b8f25c1dc4bcffdab284f482c66f21b83d90b5479e`.

### DEC-LEG-080

Question: What lifecycle policy governs ZIP compat profiles as runtimes release new versions: cadence for adding newer-version IDs, deprecation or removal of old IDs, and guidance or matching behaviour when a user's runtime version differs from any frozen profile?

Review: Adjacent CPython, JDK and libarchive release comparisons must preserve interpretable frozen ConversionProvenance IDs.

Additional HC screens: HC-07. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-22 N Profile cadence and old-ID maintenance are permanent lifecycle costs; OD-19 B Cross-version compat-prediction drift on disputed behavior lacks a canonical version-specific agreement instrument, role and band; M19.2 excludes disputed runtime outcomes

Pins: source row `80ff06700211538a772e4c018b71cc5328105059ae9cf820136e8fb85515cb72`; proposal row `e1439e1dacc483311a9916000603096193b6c7b151536bfda581489c159d6263`; reviewed assignment `6ff9a4dbe3709b4ae2eec58da89d440444abb7503b3456484eb9e43edfe23c24`.

### DEC-LEG-081

Question: How are frozen ZIP compat rule tables derived from and continuously validated against observed runtime behaviour, and how are detected contradictions (python-zipfile declared-size and Unicode Path rules versus Research III) resolved?

Review: R3 and EB-Z fixtures plus differential fuzzing must reconcile documented contradictions at pinned versions on both platforms.

Additional HC screens: HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-21 N Two-case traceability per cell and independent rule replay are binary specification checks; OD-19 B Rule-table disagreement rate on distinguishing divergent cases lacks a canonical runtime-specific selection instrument and band; M19.2 excludes reader disagreements

Pins: source row `2b4f118a679ee98a32207565393b5818a0fc4cd150e0a8f22838dae8c4eaea77`; proposal row `92da8a823d315afbaaac8447604fdf72a740f50a19e269557aa006f2cb969aae`; reviewed assignment `0d9b465ab2c5c3a107163e57c99509041963fcef7893850fef01b994bd184b6a`.

### DEC-LEG-082

Question: Which ZIP runtimes and exact versions receive frozen --compat profiles for v1 (status quo four; add Info-ZIP unzip, 7-Zip, Go archive/zip, .NET System.IO.Compression, Windows Explorer, Rust zip2, Node yauzl, Commons Compress), in what priority, and must each be measured on more than one platform before freezing?

Review: Freeze priority only after pinned Windows/Linux runtime replay, byte-identical four-profile reproduction and actual pipeline-reader demand.

Additional HC screens: HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-26 N Runtime adoption and workflow importance are descriptive/human selection context rather than graded adoption proxies; OD-19 B Marginal runtime-profile distinguishing value and disputed-case agreement lack a registered profile-specific selection instrument, role and band; M19.1 format acceptance and M19.2 unambiguous agreement do not measure it

Pins: source row `ec28f69e669c10ab686cd2b4ccfccd925a2c0d435551d95dabace8b195a8e807`; proposal row `83dcb71fb8efdd4347420d3e08f4924167d327ba717497b4b2f80156f11a7245`; reviewed assignment `4144bbfd9a255731f44fd643b63d0e0a38123afd6d4286d360d1110588e5b51a`.

### DEC-LEG-083

Question: Must compat re-run Entrybound safety validation on profile-selected values, specifically overlap and extent checks on local/descriptor-selected compressed extents and expansion-ratio checks on actual decoded output, and where in the pipeline?

Review: Adversarial local-size and declared-size bombs plus descriptor archives test pre-output bounds and benign refusal, not declaration tightness.

Additional HC screens: HC-03, HC-12, HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-08 P `alloc_peak_bytes`; OD-17 P `benign_false_refusal_fraction`; OD-19 N Selected compressed extents and decoded expansion must pass binary safety revalidation; generic import acceptance is not an integrity check

Pins: source row `4ac5b7ccad527a22c86842a5d7d7b0cd9bf8af6920877c6bb36eae528cfd34e7`; proposal row `de61cc74cb1b427fe85d6814d44f094eb057d06d5733ec39e996ae8a03ee3357`; reviewed assignment `9182efab7f14238e8222fc0363eb153f43c424c595464f4c079447c9d7ca1e4d`.

### DEC-LEG-084

Question: Must every ZIP compatibility profile (Python zipfile, both Java profiles, libarchive) refuse non-regular Unix entry types before projection, as the documented safety override states?

Review: Each frozen ZIP profile must replay 0o120777 and 0o010644 attributes and record any deliberate override of runtime behavior.

Additional HC screens: HC-03, HC-12, HC-17. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-19 N Non-regular Unix type refusal is an exact compatibility safety override, not a graded coverage preference

Pins: source row `95d8bcc165a0d96eb1921509b006c55eb41c3461706825217d16a5b6744fa835`; proposal row `c1aadd5f6bef93e9e9bab49cb34f60e06a38723f67973b98a59a193d8d40df8d`; reviewed assignment `aefdd12dc6b53210e38c2a60d1b74ec4aa8942e94641f12ece9ced2e7c797b9f`.

### DEC-LEG-085

Question: Which ZIP compression methods must v1 import decode beyond STORE and DEFLATE: Deflate64 (Explorer >2 GB), bzip2 (12), LZMA (14), zstd (93 and legacy 20), XZ (95), PPMd (98)?

Review: Method distribution across real writer ecosystems supports scope only after dependency audit and hostile decode tests.

Additional HC screens: HC-12, HC-15, HC-18. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-19 P `import_acceptance_fraction`; OD-22 N Decoder dependency maintenance is a permanent cost/supply-chain screen; OD-24 N Method-specific fuzz and decompression-bomb behavior are safety screens

Pins: source row `657ea03d90e37fbda60de653266b691cea967662eeb1eb5f092baf317dd12437`; proposal row `3599bd3849f372915333b77fec0972ba62e01571d344f2a369ab1505f9728e3c`; reviewed assignment `a8fed0e48545347494326564f1cb8b5cfb0df6e4ab3c4fb3a0f0c3ae7ee2abf3`.

### DEC-LEG-086

Question: How should ZIP import determine data-descriptor field width: infer ZIP64 width from central sentinels (status quo), from the local ZIP64 extra, by trying both widths and raising an ambiguity conflict when both parse, or by requiring agreement among all?

Review: Replay Java, Go, 7-Zip, Commons Compress and .NET width variants; both-width parses must remain explicit ambiguity evidence.

Additional HC screens: HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 B Choosing ZIP descriptor width under conflicting local and central ZIP64 assertions requires a registered disputed-case runtime-agreement instrument, role and band; M19.2 excludes disagreements

Pins: source row `f59416d598c2153c0b98131b65a4ad7ca1dfc856990f5493f6f6367a11523d3b`; proposal row `dd536a3b841940caaf7f46709bbd2d5655d9369c6ef7a7bfc584406c0e49440a`; reviewed assignment `8da256800f8abe08576e11b55997d5ae45ffd7b66bfcd1c367311acacadaa49a`.

### DEC-LEG-087

Question: What is the canonical ZIP differential corpus and observed-outcome matrix for v1: which cases (tools/zip-compat, R3 EB-Z001..Z025 and malo fixtures, ZipDiff types, GHSA wheel classes, real archives), which columns (pinned runtimes plus Entrybound strict and each profile), which expectation classes, and how is it versioned and bound to tests?

Review: Licence/source inventory, positive detector controls and reason-code replay must bind each matrix cell to exact cases.

Additional HC screens: HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-19 N The versioned differential corpus and outcome matrix are method evidence infrastructure, not a graded import-acceptance decision

Pins: source row `5039b8c2dfb02c23f729cc0803ad42420b9aa249c432424164c0f7645e356ec8`; proposal row `c47c76569b8c211f978f37a1d4b13e129d6ac7ca33946b46e297dc2817a6091c`; reviewed assignment `1de28b4f36afa7670180d055a0c55f985994d79a869b0ece49519cf97ce48cf4`.

### DEC-LEG-088

Question: Should ZIP import keep dropping DOS local timestamps (status quo), import them under an explicit recorded timezone assumption, store them as zone-less AUX metadata, and should access/creation times from 0x5455 and NTFS extras be captured or reported?

Review: Zone-less DOS time must not be presented as captured UTC; UT/NTFS access and creation times need explicit metadata truth.

Additional HC screens: HC-08, HC-17. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-03 P `restore_fidelity_fraction`; OD-18 N Cross-platform timestamp restoration must state timezone assumptions or unavailability before it can be compared; OD-19 N Frequency of missing UT/NTFS extras is descriptive input to fidelity policy; OD-25 N Lost-mtime impact in convert/extract tasks requires human evidence

Pins: source row `025711027bbf87399ed79ad99c1d14fdd22acd8179f986152c0c076fbcf0e9d0`; proposal row `64dd6fedbb797970b4eff38c1170cf09cacdad7b14d9d510c32c91cd6c56c078`; reviewed assignment `f2052e0c12507a7daf432dbba1bd8bc059dac47d4c512d0e7c2e1dd954cb739b`.

### DEC-LEG-089

Question: Should ZIP import support encrypted entries: keep refusing (status quo), support WinZip AES (AE-1/AE-2) with caller-supplied password, accept ZipCrypto with a broken-crypto warning, or list metadata only?

Review: Corpus prevalence, primary WinZip AES rules and cryptanalysis govern encrypted-entry scope before any decrypt path is offered.

Additional HC screens: HC-03, HC-06, HC-12. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-19 P `import_acceptance_fraction`; OD-15 N ZipCrypto warning, AES authentication and credential handling are binary crypto/integrity floors; OD-22 N Decoder and credential dependency support are permanent costs

Pins: source row `7bf55d09dee93aec4cb1af433b4022a133f05ec454c1a5ebc3f2a1638be12ac7`; proposal row `21789830aae7df67fce1d0a12d990a36c69c4a6ccb2a8cbf57f7f50c72f9284d`; reviewed assignment `f026cfd21af8260c17547e065f1c22cf8278fcee1ac97b7243031c52dbb8b106`.

### DEC-LEG-090

Question: How should ZIP import treat EOCD ambiguity: multiple EOF-aligned EOCD candidates, a decoy EOCD inside a comment, concatenated or zip-in-zip archives, and trailing data: select the last aligned candidate (status quo), select the majority reader's candidate, record competing candidates as conflicts, or refuse?

Review: Replay EB-Z and malo cases, measure aligned/nonaligned signatures, and refuse hidden-source ambiguities unless profile rules are evidenced.

Additional HC screens: HC-08, HC-12. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 B EOCD candidate selection under decoys, concatenation and reader disagreement needs a registered disputed-case agreement instrument and band; M19.2 excludes conflicting observations

Pins: source row `730bd881bfdff2374ee69851a000a9f08607a6ec3bab3cd8c0f73bf4b3e83d3c`; proposal row `c87eae477c75df3f5776ee16c8580b0ae3a01bb62b93a698e1ae5b0ae127daa6`; reviewed assignment `09c91234d6d15e8f0989d9103b94d07207f90c6c935dbb41c32977fdc6e9bb3f`.

### DEC-LEG-091

Question: Should a new ZIP profile write Info-ZIP Unix extras (0x7875 ids), NTFS times (0x000a), symlinks via Unix mode, xattr/ACL extras, or keep declaring these losses?

Review: Each Unix/NTFS/symlink/ACL extra needs a reader matrix and registry-ingest safety check before a new ZIP export profile may claim fidelity.

Additional HC screens: HC-13, HC-16, HC-17. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-03 P `restore_fidelity_fraction`; OD-18 P `cross_platform_restore_fidelity_fraction`; OD-19 P `export_acceptance_fraction`

Pins: source row `1fe541ff05c8a05127e48db23fe1e9e8bead76b6e38ad6858fb7194391d98a6f`; proposal row `71ce6505d60fa2b1700a97db8c1b24cf943f67d538b11bd8a82041f68ab5426c`; reviewed assignment `4acb1ee01cb7e9a2ffc3f59792703906d92e94bcf0f80d1fdf234757c5298dd7`.

### DEC-LEG-092

Question: Which ZIP extras and attributes must v1 interpret into EAM metadata (Unix mode from external attributes, 0x7875 UID/GID, 0x756e ASi Unix, symlink attributes, 0xCAFE, 0xd935 alignment, 0x9901 AES, comments) versus retain as evidence and report in FidelityReport?

Review: UID/GID, mode, symlink, alignment, AES and comments must be classified against platform metadata decisions and explicit FidelityReport gaps.

Additional HC screens: HC-08, HC-13, HC-17. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-03 P `restore_fidelity_fraction`; OD-19 N Extra-field prevalence is descriptive scope context; exact import mapping versus retained evidence is a fidelity rule; OD-20 N Round-trip source evidence and declared metadata losses are binary preservation/migration conditions

Pins: source row `f2354fa111ae5ad59b125d9ba8caafc802aab8ad9461911f144b437d34ac1358`; proposal row `38e83622c905011a958a69b39bc3d3ee7eeb573a1ab6e88ced95bd92aee542b5`; reviewed assignment `73567ac4f13f767036ca9d994837f14950105109330e0508494cca4fe87bc765`.

### DEC-LEG-093

Question: What are the frozen v1 default ZipImportPolicy limits (archive, entries, CD, extras, per-entry/total bytes, expansion ratio, observations, conflicts, evidence, preserved source), should the expansion-ratio unit and default be harmonised across legacy adapters, and should CLI users be able to tune them?

Review: Measure legitimate tail, 1/4-GiB and bomb cases, including time and RSS; error_actionability_fraction does not answer cap selection.

Additional HC screens: HC-03, HC-12. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-08 P `alloc_peak_bytes`; OD-17 P `benign_false_refusal_fraction`; OD-19 N Default ZIP limits and expansion-ratio units must enforce binary safety floors; general import acceptance cannot substitute for benign-refusal distribution

Pins: source row `c5436c8a3cfa8c364ba8c40ddb3910103056156b57dcbe6150972ccfe5ddcd46`; proposal row `6e4bebc8249d404e5092a3cf617d5b9830934e5ebb307cabe23124625a5dd894`; reviewed assignment `42e0498cdff6aed20b747a5277f0afe139eb7d87d93c8cf0eeb56563ecfc6b78`.

### DEC-LEG-094

Question: For ZIP names without the UTF-8 flag or a valid Unicode Path extra, should import keep deterministic CP437 decoding (status quo), accept an explicit caller code-page override, apply heuristic detection, preserve raw bytes as opaque components, or refuse non-ASCII unflagged names?

Review: Pin the full CP437 control table and producer/locale sample before changing raw-name semantics or heuristically guessing.

Additional HC screens: HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-19 B Choosing CP437, explicit code page or opaque name projection across locale-conflicting ZIPs needs a canonical correct-name/disputed-runtime selection instrument, role and band; M19.2 excludes disputed cases

Pins: source row `dfe741add974b76fb03ceb2db9cf6d7c7d69b741f10a117fdd70108e0053f099`; proposal row `fb1d01cad4dc7ad223d1a439a32993f252dbf89b8ec333f32520ffa7016f3d3f`; reviewed assignment `49cd93e11e43e0bafdaabfbf9aa2cdb5a073d71ac0f53d137231a431af725666`.

### DEC-LEG-095

Question: Must every multi-authority contradiction and unsupported-feature condition (duplicate ZIP64 extras, descriptor ambiguity, competing EOCD candidates, ZIP64 extensible data, unsupported method/encryption) be recorded in the LOM before refusal, so dry-run and preserve can report it, or may observe() fail fast?

Review: Map observe()-time errors to conflict classes and fuzz record-then-refuse bounds before promising forensic completeness.

Additional HC screens: HC-13, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-17 P `refusal_alloc_peak_bytes`; OD-19 N Recording every contradiction before refusal is binary observation completeness; share of archives lacking evidence is an audit census, not graded format acceptance; OD-20 N Preserved LOM evidence and dry-run report truth are binary provenance conditions

Pins: source row `45389c43dada19289bac563c43adbd82e454428410ba848de1fc32fd4df5a58e`; proposal row `02a8bb8ae01d3f8f4e937cd665e5b6787fbb375cc3de98aca9f3741c72fe855c`; reviewed assignment `0850befed39aecb94c64fc405421c3c99c74483451edfee311addf9d71ffd1e3`.

### DEC-LEG-096

Question: How should preserved source bytes be stored (raw record, codec-planned, deduplicated against decoded content, external sidecar) and what round-trip/forensic guarantees may v1 claim (exact-source recovery only, regenerated ZIP equivalence, 'everything preserved' including non-chosen readings)?

Review: Compare raw, planned, deduplicated and sidecar storage on size/time; exact recovery in INDEXED/STREAM cannot be inferred from a generic import fraction.

Additional HC screens: HC-06, HC-13, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-05 P `artifact_bytes`; OD-06 P `encode_wall_s`; OD-20 N Byte-exact source recovery, recomposition and evidence completeness are distinct binary identity claims

Pins: source row `64c295a92846e3e68f8ba134bd249882b4dba31b9c6e35f1053544c92bce8c23`; proposal row `fd04a215fad511aeadb19d8fa5dbe12b559785a5aaccba4c12b5d51a715d759f`; reviewed assignment `e272b30fbe89efd51d114584202111c467ef38fb84d64c9172345f91d33798e0`.

### DEC-LEG-097

Question: Should --preserve remain coupled to an exact --compat profile (status quo), or also be allowed with strict reconciliation or with a LOM-only projection that records evidence without selecting a runtime model?

Review: The option cannot imply a runtime projection when only strict reconciliation or observation-only evidence exists.

Additional HC screens: HC-13, HC-15, HC-16. Required routes: FORMAL, HUMAN-FACING.

Dispositions: OD-20 N Strict-plus-preserve changing AUX only and explicit LegacyPreservation mode are binary wire/identity conditions; OD-25 N Forensic strict-preservation need is a human workflow claim

Pins: source row `a6fb0cdc9424841d2a969a0e49d97012c129e873f7100ad33007424847a9b444`; proposal row `49ac571da2dc2e4710984d1d0e59d3ded39bb39e984dce555d56096cc59092a5`; reviewed assignment `62578b5e380d43b3cf644eac73121bc3311b4f9e1dd8d826b2d240a4a51d3fba`.

### DEC-LEG-098

Question: Where should preserved-source recovery live: ZIP-module function only (status quo), a format-neutral library API, a CLI command with source-digest verification, or export integration; and which adapters must produce preservation evidence?

Review: Recovery location follows the library API surface and must verify source bytes before presenting them as original.

Additional HC screens: HC-13, HC-16. Required routes: FORMAL, HUMAN-FACING.

Dispositions: OD-20 N Source-digest verification and format-neutral recovery truth are binary preservation/identity conditions; OD-25 N Forensic API/CLI usability requires human workflow evidence; OD-26 N Adapter demand is descriptive adoption context without a selecting adoption metric

Pins: source row `1fdd6c706952e787a01599d269b58276dccaad973c63012a346a72640852767c`; proposal row `3f315615c84ecc33e7e82745345b2797a3c812c54698a0bd6272a6675fe4c805`; reviewed assignment `d2787383a36819c373d6a26a95357cd428da5a722af6e61559400c86c33c0e7d`.

### DEC-LEG-099

Question: Should ZIP import refuse the whole archive when any entry uses an unsupported method, encryption, flag or disk number (status quo), or import supported entries with typed LOSSY records for skipped ones?

Review: Mixed-method/encrypted ZIP prevalence can support a partial policy only if it never silently drops unsupported entries.

Additional HC screens: HC-13, HC-16. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-19 P `import_acceptance_fraction`; OD-20 N Typed LOSSY records and skipped-entry accounting are binary migration truth conditions; OD-25 N Partial-conversion expectations require human evidence

Pins: source row `a64cfccd35a2df4cab90ea264557f153976ff4838cf520dc17bf2daefdfafc5b`; proposal row `d078ad56b7436e9ba709cea723fbe893c6073bddf78f9b7cff83bbc49a8d312b`; reviewed assignment `e1c31ea14cf2834283a49ac8b5826645ac6df688ce18776735c0a5332ebecf8f`.

### DEC-LEG-100

Question: What does the Stage-1 legacy audit surface do in v1: convert --strict --dry-run stopping at the first refusal (status quo) or reporting every ambiguity with offsets and authorities; what does verify legacy.zip --canonical mean; and is legacy-vs-legacy diff in scope?

Review: Prototype first-refusal versus all-ambiguity reporting after observation-completeness dependency; import acceptance is unrelated.

Additional HC screens: HC-13, HC-16. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-04 N Offset/authority and reason-code completeness are binary audit-report conformance; OD-20 N Audit/provenance report truth and legacy-vs-legacy diff scope are binary interface boundaries; OD-25 N Release-engineer comprehension and workflow usefulness require human testing

Pins: source row `5202d0ab6dfca19cb90b07edddac4a863ce499d8a5b53f37cb9f36cde925a637`; proposal row `da9e162f37e52af4faa1d67180a6ad2ae175f387c571eb924019d318903729e7`; reviewed assignment `a5340eff49f849a66032b411c87bec60e8a0c57b9295480ac3b6b4792263e804`.

### DEC-LEG-101

Question: Should Entrybound offer a streaming ZIP import mode driven by local headers (for pipes/non-seekable input), with results labelled untrusted until the central directory is reconciled, or keep central-directory-driven import over a complete source only?

Review: A local-header stream cannot be trusted before central-directory reconciliation; security and bounded-memory proof precede output.

Additional HC screens: HC-06, HC-12, HC-13, HC-16. Required routes: EMPIRICAL, EXTERNAL, FORMAL, HUMAN-FACING.

Dispositions: OD-08 P `alloc_peak_bytes`; OD-09 N Pipe-fed ZIP availability and staged/untrusted typing are binary streaming-capability claims; OD-19 N Central-directory reconciliation and exact import outcome are conformance/security conditions, not random-entry latency; OD-25 N Pipe workflow demand needs human evidence

Pins: source row `cfd8b6d96da6fea15f2ed60429bf933cb91c97f19e80dcb02916dee46b84fb9b`; proposal row `5aa12a70f24f7f016222ffad722c36d50f16eab5b89f0fe2396bd38b3e8c5a71`; reviewed assignment `d96501f93d5d39e85f830ee80191b155d629bfda6fb115b8c604b7f56ac271da`.

### DEC-LEG-102

Question: Which ZIP anomaly classes must strict import refuse, record as resolved conflicts, or tolerate: data descriptors, LFH/CD extra divergence, alignment padding, duplicate extra IDs, trailing data, CD comments, ZIP64 extensible data, method/CRC/size/name mismatches; and must the classification be justified by measured rejection rates on real corpora?

Review: APPNOTE citations, JAR/APK/wheel/docx/SFX tail rates and reader differentials must be preregistered per anomaly class.

Additional HC screens: HC-08, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-19 B Per-anomaly strict rejection rates and materiality across pinned runtimes lack a canonical rule-specific selection instrument, role and band; M19.1 aggregate acceptance cannot proxy each rule

Pins: source row `d86e66f7b034d3c8706a2b6468b785d837be8eaf2dd92efef652cbb1fb138def`; proposal row `4a20cad8e6de5e9c46fc5aa6f1f5fdf501b11df25c388fa7b51a838ff93aa6fc`; reviewed assignment `ea1ed691af51f9f7e43d08642e83919797a67816a4ed44079ccf506bed396a12`.

### DEC-LEG-103

Question: How should ZIP import treat bytes not accounted for by referenced local entries, descriptors, CD and EOCD: SFX prefixes, APK Signing Blocks, hidden local entries absent from the CD, and gaps; tolerate silently (status quo), record as observations, refuse in strict, or special-case known structures?

Review: Import signed/aligned APK and SFX fixtures, count unreferenced intervals, and test EOCD sniff on non-ZIP binaries.

Additional HC screens: HC-08, HC-14, HC-15. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-15 N APK signing-block integrity and hidden local-entry ambiguity are binary authenticity/safety conditions; OD-19 B EOCD tail-sniff false-positive rate and gap-class-specific prevalence lack a canonical detector/rule-specific selection instrument and band

Pins: source row `7cd51a42cd12f70ce8e08f5741af1cf239acd8d8532d7febb09c83b6a7e459aa`; proposal row `a58c77c9615301f2ab8237af621d8eb7cb655a816917f531fff80862d5e127e6`; reviewed assignment `80a0fae0a1865fcfacb9c84ed34ed5f3e42d029ad44b73080a89603d95a3a9dc`.

### DEC-LEG-104

Question: How should ZIP import treat Info-ZIP Unicode Path extras (0x7075): ignore stale-CRC, wrong-version or invalid extras as Omission (APPNOTE/R1 reading), refuse them (status quo), and how are flag-set conflicts and multiple extras resolved in strict and per profile?

Review: APPNOTE/R1 authority, stale-extra frequency and pinned EB-Z019 replay must resolve Omission versus Divergence before a profile claims behavior.

Additional HC screens: HC-08, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-19 B Stale Unicode Path and flag-conflict projection across disagreeing runtimes needs a registered disputed-case agreement instrument, role and band; M19.2 excludes reader disagreement

Pins: source row `e97d99ba966254cc151e3fe8bb7755c520e3281de7bfd5191f1fa7260e3469aa`; proposal row `4f2c5b9ab00644301e9c28bb3722f41367d7acd9748a364b554e132c5a8f0f6c`; reviewed assignment `7cfe24b434e0b0f185df7546df8ee4cedacd369b4b17e9b59a48a7d94fbea39e`.

### DEC-LEG-105

Question: Which benign ZIP writer variants must strict import accept: surplus or zero-filled local ZIP64 extras, unconditional ZIP64 end records, central-directory digital signature records, reserved or rarely used flag bits, and which APPNOTE version is the baseline?

Review: Survey Java, .NET, Go, libzip, Info-ZIP and 7-Zip against the exact APPNOTE version and differential outcomes.

Additional HC screens: HC-08, HC-15. Required routes: EMPIRICAL, EXTERNAL, FORMAL.

Dispositions: OD-19 B Per-writer benign ZIP variant refusal rates lack a registered rule-specific selection instrument and band; generic M19.1 acceptance cannot decide each strict rule

Pins: source row `c6f54aadd3b018c7e889bcc5716772e438ed97a2204bbaf2080ff81902fb8bd2`; proposal row `b0172032b3ee130840cc08abdcd7b3953d51883d4a13c15f3a957dc1983c10c1`; reviewed assignment `b3edc7d4d61932c8bbf8c4f6dc68abb897117291e22a3258139df8b15829f1f5`.

### DEC-LEG-106

Question: Should zstd transport import accept frames declaring dictionary IDs when the caller supplies the dictionary?

Review: Dictionary-frame prevalence can justify caller-supplied dictionary support only with dictionary ID binding and bounded decoder behavior.

Additional HC screens: HC-06, HC-13. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-19 P `import_acceptance_fraction`

Pins: source row `6b4218e6118699893681e957d84835364619b75d0393a7e012c0a1a1c75160df`; proposal row `57f94a403aca0a1f77f2aba683693e905c443072e9e47b3241bd9396495cf6fd`; reviewed assignment `3dc79945583ac3ed86576d2e8e418a39011ff19c843d998b6f292a48c879d217`.

### DEC-LEG-107

Question: Which Zstandard framing variants does strict import accept and record: skippable-frame payload retention, seekable-format index frames, legacy pre-1.0 frames, and frames without content size (checksum-less frames are decided in legacy-integrity-check-strictness)?

Review: Frame variants need pinned writer import cases and prevalence; random-entry latency has no bearing on skippable or size-less framing.

Additional HC screens: HC-03, HC-12, HC-13, HC-16. Required routes: EMPIRICAL, FORMAL.

Dispositions: OD-05 P `artifact_bytes`; OD-19 P `import_acceptance_fraction`; OD-20 N Skippable payload retention and native-digest truth are binary provenance conditions

Pins: source row `16d83478519cbd058af09ea4f5c90d459c715f29152c4ee7ed52be0e062b32ed`; proposal row `54769a9ed9dc5bdd00722b030e68f30fecc8e3d0de278a04948dbfc808096877`; reviewed assignment `a6615c8e5abef1019a777e4210185de547c1609a3d3249b613592bcfb9216a01`.

### DEC-LEG-108

Question: Which ZIP64 emission policy should ZIP export use when output cannot be prepared in memory or goes to a non-seekable sink (R1 §8.3 Tier 3; §21 item 14): whole-archive in-memory AsNeeded (zip/portable-v1), per-entry buffering, temp-file staging, an Always-ZIP64 streaming profile with data descriptors, Never-ZIP64 with pre-write refusal, fail-late AsNeeded, or refusing large non-seekable ZIP output?

Review: Compare multi-GiB and >65,535-entry paths for memory, scratch and time, then test Always-ZIP64 descriptors across named readers.

Additional HC screens: HC-06, HC-13, HC-16. Required routes: EMPIRICAL, FORMAL, HUMAN-FACING.

Dispositions: OD-06 P `encode_wall_s`; OD-08 P `alloc_peak_bytes`; OD-19 P `export_acceptance_fraction`; OD-09 N Pipe and non-seekable ZIP emission is a binary streaming capability; OD-20 N Byte identity of buffered/staged zip/portable-v1 and profile versioning are binary export determinism; OD-25 N Release/CI demand for pipe output needs human workflow evidence

Pins: source row `6af81e86be6cec8cb91a2049b08f4a5c13de06c289a80be2fe2a39ba001c2e18`; proposal row `c4b75e428f0b0c701dcc6cebdb28eb402356c4c0aaa2cc38445e3717f52cfbf4`; reviewed assignment `1f808d649ec2def66f147a837682bed74ec8602d154463a67823709cd0dc200b`.

## Controlled LF source rebinding

On 2026-10-01, `policy_review/2026-10-01` normalized only CRLF to LF in `research/decision-method.md` and `research/archetypal-objective.md`, after verifying exact decoded-text equality. The canonical proposal was regenerated with only those two file-byte source hashes changed. Every reviewed assignment field, canonical assignment digest, status and per-record reviewer identity remained unchanged; only the raw proposal-row source pins were rebound. This is source-byte rebinding of the accepted prospective review, not a new candidate, experiment, outcome, gate PASS or live ledger migration.

The preceding method/objective byte identities were `fe27bcb0c078d700463916c12ef5f07b9c7cd5870ae2c25894e02fe2590dbb7c` / `ab00418cdfb40cb3ca283324d30d01e10a3dac4f6cb733b11d104dc0d567664a`; the preceding proposal identity was `4099090f862a03490ff19fac279bce217b72badcc4e028b108a03552f1efe7bd`. These are superseded byte identities, not current-source receipts. Their retention here does not assert that historical bytes are durably archived in the public repository.

The preceding separate review register and note identities were `a26f1d2cbf6f4bb5c3c000cd139c5813c7054c6979ce09365bbbf162a3e23c35` / `dd52ba11dd9304ed495338d06d3bb7f03f631ed75fdd55a1768c8766916a4558`. The question-specific adjudications and original independent reviewer identity are preserved.
