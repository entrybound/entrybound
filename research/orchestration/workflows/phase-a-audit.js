export const meta = {
  name: 'eb-research-phase-a-audit',
  description: 'Entrybound research Phase A: source-of-truth extraction, requirement/decision ledgers, completeness critique, pre-registered method',
  phases: [
    { title: 'Extract', detail: 'one agent per source slice writes JSONL extraction records' },
    { title: 'Merge', detail: 'one agent per topic cluster merges duplicate records and proposes decisions' },
    { title: 'Assemble', detail: 'deterministic ledger assembly with stable IDs' },
    { title: 'Critique', detail: 'completeness critics per source family, loop until dry' },
    { title: 'Method', detail: 'archetypal objective + pre-registered decision method, adversarially reviewed' },
  ],
}

const REPO = 'D:/Projects/entrybound/entrybound'
const SPEC = 'D:/Projects/entrybound/design/2026-08-29-entrybound-product-architecture.md'
const R1 = 'D:/Projects/entrybound/research/2026-08-29-archive-category-research.md'
const R2 = 'D:/Projects/entrybound/research/2026-08-29-entrybound-opportunity-validation.md'
const R3 = 'D:/Projects/entrybound/research/2026-08-29-entrybound-research-iii-final-gate.md'
const SRC = 'C:/Users/timdi/AppData/Local/Temp/claude/D--Projects-entrybound/9860c8f2-7fbf-42a7-994c-4bcb9932ac08/scratchpad/sources'
const APPX = SRC + '/appendix'
const R3I = SRC + '/r3-instrumentation'
const EXTRACT = REPO + '/research/audit/extract'
const MERGED = REPO + '/research/audit/merged'
const ADDENDA = REPO + '/research/audit/addenda'
const PY = 'C:/Python313/python.exe'

const CLUSTERS = [
  { key: 'model', code: 'MOD', desc: 'semantic model: EAM objects, EntryKind, LogicalPath rules, invariants I1-I31, identity roots LAI/PCR/AUX/PCI, canonical encoding/serialization, digest algorithm, determinism model, diagnostics/reason codes, library API surface, archive role, identity profiles' },
  { key: 'container', code: 'CON', desc: 'native container/ECF: preamble, feature bits/tiers, sections, footer, descriptor, manifest encoding & size, Index data structure, Merkle/outboard layout, declared resource budgets & default values, magic number, media type, file extension, format evolution/deprecation, decoder longevity' },
  { key: 'compression', code: 'CMP', desc: 'compression planner & objective/weights, four profiles, CDC chunking & parameters, deduplication scope, similarity clustering, dictionaries, ChunkGroups/lookback, codec registry/portfolio, structural transforms (delta/shuffle/BCJ), reconstructive transforms (DEFLATE/JPEG), planner determinism & versioning, input consistency during pack' },
  { key: 'access', code: 'ACC', desc: 'STREAM layout, INDEXED random access, remote/HTTP range access & fetch strategy, partial verification, streaming memory bounds, scalability beyond RAM, bounded staging, deterministic parallelism/concurrency model, repack/diff/inspect/explain tooling, multi-volume/append as access topics' },
  { key: 'platform', code: 'PLT', desc: 'metadata namespaces & criticality, timestamps, ownership (uid/gid/names/SIDs), permissions/ACLs, xattrs, sparse files, filename encoding & native path rules, forks/NTFS ADS/resource forks/FinderInfo, special files (devices/FIFOs/sockets), origin/quarantine markers, platform API feasibility (safe Rust), extraction confinement & dangerous-restoration policy, FidelityReport' },
  { key: 'crypto', code: 'CRY', desc: 'crypto primitive suite (AES-256-GCM-SIV, HKDF, HMAC commitment, X-Wing, Argon2id, Ed25519, SHA-256, RFC3161), envelope & key hierarchy, key commitment, nonces, recipients & passwords, padding schedules, encrypted CDC boundary leakage, encrypted metadata/privacy, signatures & bindings & trust semantics, timestamps, recipient mutation/key management, feature composition, threat model, external security review' },
  { key: 'legacy', code: 'LEG', desc: 'legacy import adapters (ZIP/tar/7z/compressed streams), Legacy Observation Model & reconciliation & conflict classes, --compat and --preserve modes & runtime matrices, legacy export & target profiles, LOSSLESS/LOSSY/REFUSED, dual publishing/migration, sidecars without new extension, conversion receipts & chained provenance, adapter priority (OCI, NAR, RAR5, cpio, ar, WIM, squashfs, lzip, LZ4 frame, others)' },
  { key: 'integrity', code: 'INT', desc: 'verification semantics & reporting, salvage/repair commands, corruption localization & blast radius, external parity binding (in-archive ECC refused), incremental archives & external content references & base chains, archive role Complete vs partial, network-backed archives' },
  { key: 'ecosystem', code: 'ECO', desc: 'CLI command set & UX & option complexity, adoption strategy & distribution wedge, opportunity/market/gatekeeper findings (Research I-III), conformance corpus & observed-outcome matrix, independent/minimal reader, fuzzing, dependency policy & longevity & implementation language, incumbent capability comparison & superiority claims, explicit non-goals' },
]
const CLUSTER_KEYS = CLUSTERS.map(c => c.key)

const PROGRAM_QUESTIONS = {
  model: `- Digest algorithm & identity descriptor parameters (spec 25.1 #7); canonical serialization encoding choices (25.1 #1,#2); feature-bit assignment policy (25.1 #14); library API surface (25.1 #21).
- Whether non-UTF-8 / non-Unicode native names need LogicalPath representation (task 16), and what identity/strict-v1 must contain (task 19).
- Determinism guarantees under parallelism and encryption; which metadata participate in which identity root.`,
  container: `- Task 11: current manifest vs compressed vs segmented vs indexed/tree manifest; current Index vs range-friendly alternatives; Merkle outboard layouts; proof bytes/CPU/requests/cacheability/amplification; is a format revision justified; does the uncompressed-manifest default cost enough at very large entry counts to change it (spec 25.3).
- Default resource-budget values (25.1 #16, #23); magic-number confirmation against identification databases (25.1 #15); media-type registration path.`,
  compression: `- Task 8.1 CDC: normalized Gear vs fixed-size vs other credible CDC (FastCDC variants, Rabin, BuzHash, AE, RAM, SeqCDC etc.), min/target/max sweeps; ratio, dedup, index overhead, CPU, memory, boundary stability under insertion, update locality, random-access amplification; category-specific and aggregate Pareto frontiers; per-category parameters (25.1 #6, 25.3).
- Task 8.2 dedup scope: none/whole-file/exact-chunk/archive-wide/windowed; when dedup metadata cost exceeds benefit; tiny files; encrypted creation.
- Task 8.3 similarity clustering: bottom-k vs serious alternatives; accuracy, CPU/memory, false-group cost, cohort size sensitivity.
- Task 8.4 dictionaries: training size, dictionary size, level, cohort thresholds, reuse count; exact minimum-benefit rule charging dictionary bytes & decoder memory.
- Task 8.5 lookback 0/1/2/4/8+: gain vs first-byte latency, access amplification, dependency bytes, parallelism loss, corruption propagation (25.3).
- Task 8.6 codec portfolio: implemented + plausible additions; ratio/speed/memory/decoder complexity/dependency quality/spec availability/independent implementability; baseline vs extended codec set (25.1 #3,#4).
- Task 8.7 structural transforms: delta, shuffle, BCJ family, numeric transforms, combinations; false-positive cost (25.1 #5).
- Task 8.8 reconstructive transforms: DEFLATE, baseline JPEG, progressive JPEG, others only if prevalent; prevalence, exact roundtrip success, side-data, net savings, encode/decode/verify cost, burden (25.3).
- Task 9 planner optimality: regret vs exhaustive search by workload class; heuristic/DP/pruning/deterministic data-driven selection; is planner complexity justified (25.1 #17).
- Task 10 profiles: are four public profiles appropriate; is balanced the defensible default; per-profile chunking/codec breadth/dictionary/lookback/transform/working-set; stability of profile definitions under weight perturbation.`,
  access: `- Task 12 remote access: range coalescing thresholds, concurrency, cache sizes, eager vs lazy download, logical-range proof granularity, Index/metadata fetch strategy; bytes & requests under RTT/bandwidth/loss/CDN/object-store/warm-cold cache.
- Task 13 streaming/scalability: pack, STREAM & INDEXED creation, unpack, import, encryption, export, repack, multi-target publish on archives larger than RAM; which operations scale with total plaintext; bounded staging architectures; validity of bounded-memory claims.
- Task 14 deterministic parallelism: hashing, chunk analysis, codec candidate evaluation, compression, verification; queue structure, worker bounds, canonical emission ordering, diminishing returns; concurrency model (25.1 #20).`,
  platform: `- Task 15 cross-platform fidelity corpus results (non-UTF-8 POSIX names, UTF-16 names, case/normalization collisions, reserved names, trailing dot/space, symlinks, junctions/reparse, hardlinks, mode, uid/gid, ACLs, xattrs, sparse, security descriptors, Windows attributes, birth/creation/access/change times, macOS flags, forks, ADS, origin markers, devices, FIFOs, sockets) on real OS/filesystems.
- Task 16 native paths on ext4/XFS/btrfs/APFS/NTFS/ReFS: representation, invalid UTF-8, unpaired surrogates, case folding, normalization, reserved components, length, trailing chars, collisions; exact path encoding & extraction refusal rules.
- Task 17 streams/forks per class (NTFS ADS, Zone.Identifier, resource forks, FinderInfo, xattr analogues): LAI vs AUX vs separate named-content semantic vs refuse.
- Task 18 special files (char/block devices, FIFOs, sockets): first-class EntryKind vs auxiliary snapshot metadata vs FidelityReport-only vs refuse.
- Task 19 ownership & identity: numeric ids, names, user namespaces, containers, NFS idmap, AD SIDs, cross-machine restore; portable AUX vs platform metadata vs identity/strict-v1.
- Task 20 origin/provenance markers: preserve raw only, canonical cross-platform origin semantic, propagate to extracted members, caller policy.
- Task 21 platform API feasibility (safe Rust crates) for Windows security descriptors, opaque reparse data, no-follow creation/restoration, macOS ACLs, birthtime, forks, no-follow metadata ops.`,
  crypto: `- Task 22.1 primitive review of AES-256-GCM-SIV, HKDF-SHA-256, HMAC commitments, X-Wing, Argon2id (+default params, 25.1 #12), Ed25519, SHA-256 identities, RFC3161 against primary standards/current guidance (25.1 #9-#12, 25.2).
- Task 22.2 boundary leakage attacks reproduced: public CDC vs secret Gear vs PHTE/AES; leakage reduction, CPU, boundary stability, compression effect (25.2).
- Task 22.3 padding schedules NONE/BUCKETED/MAXIMUM/alternatives: privacy vs storage overhead (25.1 #13).
- Task 22.4 signature/trust: self-signed vs trusted publisher, allowed-key sets, multiple signatures, partial bindings, combining bindings across signers, recipient mutation, cross-context forwarding, stale physical/addressing bindings, timestamps; distinguish cryptographically valid / binding current / authorized signer / trusted timestamp (25.2).
- Task 22.5 composition matrix of conversion evidence, preservation, encryption, signatures, recipient changes, password rotation, random access, repack.
- External-review dossier: KDF hierarchy/commitment formal analysis, named adversary threat model (25.2).`,
  legacy: `- Task 25 legacy differential across ZIP runtimes, GNU tar, bsdtar/libarchive, Python tarfile, 7z implementations, dialect extensions: compatibility profiles derived from observed behavior (tar/7z compat & preservation currently ZIP-only); --compat behavioural models (25.1 #22).
- Task 26 adapter priority: OCI, NAR, RAR5, cpio, ar, WIM, squashfs, lzip, LZ4 framing, others discovered -> CORE_V1/POST_V1_CORE/ECOSYSTEM/NOT_JUSTIFIED.
- Task 27 export fidelity: standard tar, pax, GNU, star, ZIP Unix extras, platform extras; new target profiles and truthful guarantees; frozen profiles untouched; 7z export question.
- Import resource-budget default values (25.1 #23).`,
  integrity: `- Task 23 sidecars & incremental: content-free sidecars, local external refs, missing/renamed originals, remote locators, base chains, multiple/stale bases, cycles; locator semantics, advisory-only locators, max dependency depth, base identity requirements, network-fetch policy, stable-v1 membership.
- Task 24 recovery: bit flips, missing chunks, truncation, burst loss, footer/index/metadata/dependency corruption; identify damaged chunks/affected/intact entries/blast radius; external parity approach, binding, overhead, repair success, security; evidence-backed verify/salvage/repair semantics (salvage never implies verification).`,
  ecosystem: `- Task 28 independent reader from written spec; ambiguity log; production vs independent differential over native corpus.
- Task 29 versioned native conformance corpus with REQUIRED/FORBIDDEN/ACCEPTABLE/PROFILE_DEPENDENT/UNRESOLVED classes (UNRESOLVED empty for RC); spec 19.3 corpus as v1 deliverable.
- Task 30 persistent fuzz targets & differential fuzzing; findings minimized into regressions.
- Task 31 dependency audit & classification (baseline reader/extended decoder/writer only/adapter only/test-research only); influence on codec/feature choices (25.1 #19).
- Task 32 adoption workflows (source releases, package publishing, CI artifacts, model/data distribution, scientific archives, supply-chain verification, legacy audit, dual publishing, sidecar verification, remote partial access): commands/options, failure modes, migration friction, value before ecosystem adoption.
- Task 33 CLI/user complexity: which concepts hidden behind defaults vs explicit policy; can first-time users pack/unpack/verify/inspect/convert/choose profile/understand LOSSY-REFUSED/interpret signatures/handle metadata restoration/use remote access.
- Task 34 stable-v1 disposition for every unfinished capability; Task 35 final held-out system evaluation incl. workloads where Entrybound is not best; incumbent comparison fairness.`,
}

const EXTRACT_SCHEMA = {
  type: 'object',
  properties: {
    file: { type: 'string' },
    record_count: { type: 'integer' },
    counts_by_kind: { type: 'object', additionalProperties: { type: 'integer' } },
    open_questions_sample: { type: 'array', items: { type: 'string' } },
    api_notes_file: { type: 'string' },
  },
  required: ['file', 'record_count', 'counts_by_kind'],
}

const COMMON_EXTRACT = `You are one extraction agent in the source-of-truth audit for the Entrybound research program. Entrybound is a Rust archive system (repo ${REPO}, branch dev at 9e44608). Your job is EXHAUSTIVE, LOCATION-PRECISE extraction of every requirement, invariant, frozen decision, deferred matter, open/unresolved question, claim that depends on evidence, non-goal, and (for code) implementation fact or unimplemented capability, from the source slice assigned below. Later agents merge these into a Requirement Ledger and Decision Ledger; anything you omit may silently disappear from the research program, which is an automatic failure. Prefer too many fine-grained records over too few.

Write your records as JSON Lines (UTF-8, one JSON object per line) to the output file named below. Generate the file with a Python script (${PY}) to avoid escaping errors, then validate it by re-reading every line with json.loads. Do not write anywhere else except the optional api-notes file. Never run git commands that modify state (no commit/checkout/stash/reset/add).

Record fields (all required; use "" or [] when not applicable):
- local_key: unique within your file, e.g. "<your agent key>-0001".
- kind: one of INVARIANT, REQUIREMENT, FROZEN_DECISION, DEFERRED_QUESTION, OPEN_QUESTION, CLAIM_NEEDING_EVIDENCE, NON_GOAL, NOT_IMPLEMENTED, IMPLEMENTATION_FACT, EMPIRICAL_FINDING.
- source: the source id given below.
- location: section number/heading AND line range (e.g. "§7.3 L794-805") or "path:line-line" for code. Use Read line numbers or Grep -n; never guess.
- statement: faithful paraphrase, at most 60 words. Do not copy long passages; a quote, if needed, must be under 15 words.
- primary_cluster: exactly one of ${CLUSTER_KEYS.join(', ')}.
- tags: short topic tags (e.g. "cdc", "lookback", "padding", "ads", "zip-compat").
- refines_or_supersedes: if this source says it refines, freezes, corrects, supersedes, or contradicts another authority, name it with location; else "".
- implementation_hint: IMPLEMENTED, PARTIAL, MISSING, UNKNOWN, or N/A (only assert IMPLEMENTED/PARTIAL/MISSING if the source itself establishes it).
- evidence_in_source: what evidence the source offers (measured experiment, primary citation, inference, none).
- research_needed: what empirical/formal/security/platform/usability evidence would settle or validate it; "" if purely definitional and already settled.
- decision_needed: the product/technical decision still open, phrased as a question; "" if none.
- release_relevance: V1_BLOCKING, V1_IMPORTANT, POST_V1, or INFORMATIONAL.

Cluster guide:
${CLUSTERS.map(c => `- ${c.key}: ${c.desc}`).join('\n')}

Pay special attention to: anything labelled deferred, not implemented, future, open, unresolved, "must be measured", "should be checked", refused-for-now, experimental, frozen-by-version, a numeric parameter chosen without cited measurement, and every place a later document narrows or changes an earlier one.`

const EXTRACTORS = [
  { key: 'spec-01', source: 'SPEC', where: `${SPEC} lines 1-350 (TOC, §1 executive definition, §2 philosophy, §3 native semantic model including §3.9 complete invariant list)` },
  { key: 'spec-02', source: 'SPEC', where: `${SPEC} lines 351-644 (§4 native format architecture, §5 compression planner architecture)` },
  { key: 'spec-03', source: 'SPEC', where: `${SPEC} lines 645-996 (§6 four profiles, §7 dedup & chunking, §8 integrity & identity)` },
  { key: 'spec-04', source: 'SPEC', where: `${SPEC} lines 997-1330 (§9 encryption/authentication/signatures, §10 metadata architecture)` },
  { key: 'spec-05', source: 'SPEC', where: `${SPEC} lines 1331-1675 (§11 security model, §12 determinism, §13 streaming/random/remote access, §14 legacy import)` },
  { key: 'spec-06', source: 'SPEC', where: `${SPEC} lines 1676-2088 (§15 legacy export, §16 receipts & provenance, §17 CLI, §18 extension/media type/magic)` },
  { key: 'spec-07', source: 'SPEC', where: `${SPEC} lines 2089-2498 (§19 product & library surface, §20 extensibility, §21 adoption, §22 incumbent comparison, §23 superiority argument)` },
  { key: 'spec-08', source: 'SPEC', where: `${SPEC} lines 2499-2754 (§24 non-goals, §25 intentionally deferred matters — extract EVERY table row and bullet as its own record, §26 evidence base and both review-pass disposition tables, Appendix A invariants I1-I31 each as its own INVARIANT record, Appendix B)` },
  { key: 'r1-a', source: 'R1', where: `${R1} lines 1-605 (Research I §0-§8: archetype, lineage, incumbents, adoption, pain corpus, security analysis, edge cases)` },
  { key: 'r1-b', source: 'R1', where: `${R1} lines 606-1147 (Research I §9-§21: determinism, fragmented solutions, compatibility, falsification, successors, doctrine, conceptual API, v1 scope, future scope, opportunity, open research questions, bibliography, decisions we should not make yet)` },
  { key: 'r2', source: 'R2', where: `${R2} all lines (Research II opportunity validation; extract especially §3 corrections, §21 answers, §23 decision gate, §24-§26 open questions and decisions not to make yet)` },
  { key: 'r3-a', source: 'R3', where: `${R3} lines 1-686 (Research III §1-§13: experiment 1 method/corpus/prevalence/clustering/parser differentials/verdict, thirty-case corpus, expectation classes, policy profiles, maintainer precedent)` },
  { key: 'r3-b', source: 'R3', where: `${R3} lines 687-1333 (Research III §14-end: PyPI/Maven, licensing matrix, prior art, gatekeepers, outreach, product boundary, thesis, non-goals, NO-GO case, score, decision gate, unresolved questions, corrections, decisions not to make yet) AND the Research III instrumentation bundle at ${R3I} (source id for bundle records: R3-INSTR; read README.md, the scripts, cases/manifest.json, cases2/manifest.json, malo/manifest.json, out/FACTS.json; record what was measured, how, and what the raw outputs show)` },
  { key: 'appx-chunking', source: 'APPX/chunking.md', where: `${APPX}/chunking.md all lines (CDC incl. 2025 attacks, dedup systems, seekable formats, container layers, Merkle, parallelism)` },
  { key: 'appx-crypto', source: 'APPX/crypto.md', where: `${APPX}/crypto.md all lines (AEAD/commitment, nonces, KEM/multi-recipient, PQ, password KDFs, signatures/provenance, verified streaming, encryption+dedup, encrypted metadata, agility)` },
  { key: 'appx-metadata', source: 'APPX/metadata.md', where: `${APPX}/metadata.md all lines (timestamps, ownership, permissions/ACLs, xattrs, sparse, filename encoding, safe extraction per platform, media type, magic, long-term readability)` },
  { key: 'appx-incumbents', source: 'APPX/incumbents.md', where: `${APPX}/incumbents.md all lines (ZIP, tar, 7z, RAR5, zstd, gzip, xz, squashfs/EROFS, DwarFS, Apple AA/AEA, zpaq, WIM, NAR, OCI; fail-closed taxonomy; recovery records)` },
  { key: 'docs-format', source: 'docs', where: `${REPO}/docs/format-v0.md, ${REPO}/docs/stream-layout-v1.md, ${REPO}/docs/descriptor-vectors-v1.txt (use source "docs/<filename>" per record)` },
  { key: 'docs-fs', source: 'docs', where: `${REPO}/docs/filesystem-bootstrap.md, filesystem-fidelity-v1.md, posix-metadata-v1.md, posix-metadata-v1-vectors.txt, security-metadata-v1.md, security-metadata-v1-vectors.txt, platform-fidelity-v1.md (all under ${REPO}/docs; source "docs/<filename>")` },
  { key: 'docs-compress', source: 'docs', where: `${REPO}/docs/planner-v1.md, chunking-v1.md, cross-file-compression-v1.md, codec-transform-v1.md, reconstructive-transform-v1.md, jpeg-reconstruction-v1.md (source "docs/<filename>"; capture every frozen parameter value and the stated rationale or lack of measurement)` },
  { key: 'docs-access', source: 'docs', where: `${REPO}/docs/random-access-v1.md, http-range-access-v1.md, repack-v1.md, archive-diff-v1.md, inspection-explanation-v1.md (source "docs/<filename>")` },
  { key: 'docs-crypto-a', source: 'docs', where: `${REPO}/docs/crypto-threat-model-v1.md and ${REPO}/docs/crypto-suite-v1.md (source "docs/<filename>")` },
  { key: 'docs-crypto-b', source: 'docs', where: `${REPO}/docs/crypto-wire-v1.md and ${REPO}/docs/crypto-wire-v1-vectors.txt (source "docs/<filename>")` },
  { key: 'docs-crypto-c', source: 'docs', where: `${REPO}/docs/crypto-review-v1.md, crypto-implementation-v1.md, signing-key-management-v1.md (source "docs/<filename>"; capture every review finding, residual risk, and item routed to external review)` },
  { key: 'docs-legacy', source: 'docs', where: `${REPO}/docs/legacy-observation-model-v1.md, zip-import-v1.md, zip-compatibility-profiles-v1.md, legacy-preservation-v1.md, tar-import-v1.md, 7z-import-v1.md, compressed-stream-import-v1.md, legacy-export-v1.md, zip-export-v1.md, tar-export-v1.md, compressed-tar-export-v1.md, migration-workflows-v1.md (source "docs/<filename>")` },
]

const CODE_EXTRA = `This is a CODE audit slice. In addition to the record kinds above, emphasise IMPLEMENTATION_FACT (what exists: frozen IDs/versions, parameters, limits constants, feature bits, reason codes, public entry points) and NOT_IMPLEMENTED (every refusal, unsupported branch, unimplemented!/todo!, "not supported" diagnostic, capability reported to FidelityReport instead of implemented). Cite path:line ranges relative to the repo root with source "code:<path>". Name the test files/tests that cover each capability (grep ${REPO}/crates/*/tests). ALSO write a markdown file ${EXTRACT}/<your key>-api-notes.md listing the public functions/types a separate research harness crate (path dependency on crates/entrybound) could call to run experiments on this area without modifying production code (e.g. chunk with given params, evaluate planner candidates, encode with a codec, build INDEXED/STREAM archives, random-access reads), and any internal-only functionality that WOULD require a research-only feature flag; return its path as api_notes_file.`

const CODE = [
  { key: 'code-model', where: `${REPO}/crates/entrybound/src/lib.rs, src/eam/*.rs, src/canonical.rs, src/identity.rs, src/diagnostics.rs` },
  { key: 'code-ecf', where: `${REPO}/crates/entrybound/src/ecf.rs, src/ecf/records.rs, src/ecf/container.rs, src/ecf/staging.rs` },
  { key: 'code-access', where: `${REPO}/crates/entrybound/src/ecf/stream.rs, src/ecf/random.rs, src/random_access.rs, src/archive.rs, src/archive/inspection.rs` },
  { key: 'code-fs', where: `${REPO}/crates/entrybound/src/archive/filesystem.rs and src/archive/tooling.rs` },
  { key: 'code-compress', where: `${REPO}/crates/entrybound/src/planner.rs, chunker.rs, codec.rs, transform.rs, reconstruction.rs, jpeg_reconstruction.rs, similarity.rs` },
  { key: 'code-crypto-a', where: `${REPO}/crates/entrybound/src/crypto/container.rs and src/crypto/random.rs` },
  { key: 'code-crypto-b', where: `${REPO}/crates/entrybound/src/crypto/mod.rs, signature.rs, timestamp.rs, wire.rs` },
  { key: 'code-legacy-a', where: `${REPO}/crates/entrybound/src/legacy/zip.rs and src/legacy/sevenz.rs` },
  { key: 'code-legacy-b', where: `${REPO}/crates/entrybound/src/legacy/tar.rs, export.rs, import.rs, stream.rs, migration.rs, lom.rs, mod.rs` },
  { key: 'code-cli', where: `${REPO}/crates/entrybound-cli/src/lib.rs, src/main.rs, src/bin/entrybound.rs, tests/*.rs (enumerate every command, option, default, and user-facing concept; note option counts per command)` },
  { key: 'code-deps-tests-tools', where: `${REPO}/Cargo.toml, Cargo.lock, crates/*/Cargo.toml, rust-toolchain.toml, .gitignore, crates/entrybound/tests/*.rs (inventory what each test file covers), tools/crypto-vector-helper/*, tools/zip-compat/*, tools/tar-strict/* (for every direct dependency record purpose, pinned version, and which module uses it; for tools record what runtimes/matrices were probed and exact versions)` },
]

phase('Extract')
const extractJobs = [
  ...EXTRACTORS.map(e => ({ ...e, prompt: `${COMMON_EXTRACT}\n\nYour agent key: ${e.key}\nSource id: ${e.source}\nSource slice: ${e.where}\nOutput file: ${EXTRACT}/${e.key}.jsonl\nRead the entire slice carefully (use Read with offset/limit for line ranges). Return the file path, record count, counts by kind, and up to 10 of the most important open questions.` })),
  ...CODE.map(c => ({ ...c, prompt: `${COMMON_EXTRACT}\n\n${CODE_EXTRA}\n\nYour agent key: ${c.key}\nSource slice: ${c.where}\nOutput file: ${EXTRACT}/${c.key}.jsonl\nReturn the file path, record count, counts by kind, api_notes_file, and up to 10 most important gaps.` })),
]
const extracted = await parallel(extractJobs.map(j => () => agent(j.prompt, { label: `extract:${j.key}`, phase: 'Extract', schema: EXTRACT_SCHEMA })))
const extractSummary = extractJobs.map((j, i) => ({ key: j.key, ok: !!extracted[i], count: extracted[i] ? extracted[i].record_count : 0 }))
const failed = extractSummary.filter(s => !s.ok).map(s => s.key)
log(`Extraction: ${extractSummary.reduce((a, s) => a + s.count, 0)} records from ${extractSummary.filter(s => s.ok).length}/${extractJobs.length} slices` + (failed.length ? `; FAILED: ${failed.join(', ')}` : ''))

// Retry failed extractors once
if (failed.length) {
  const retry = await parallel(extractJobs.filter(j => failed.includes(j.key)).map(j => () => agent(j.prompt + '\n\nA previous attempt at this slice failed; if a partial output file exists, overwrite it completely.', { label: `extract-retry:${j.key}`, phase: 'Extract', schema: EXTRACT_SCHEMA })))
  retry.forEach((r, i) => { if (r) log(`retry ok: ${failed[i]} (${r.record_count})`) ; else log(`retry FAILED: ${failed[i]}`) })
}

phase('Merge')
const MERGE_SCHEMA = {
  type: 'object',
  properties: {
    requirements_file: { type: 'string' },
    decisions_file: { type: 'string' },
    requirement_count: { type: 'integer' },
    decision_count: { type: 'integer' },
    input_record_count: { type: 'integer' },
    unmapped_input_keys: { type: 'array', items: { type: 'string' } },
    gaps: { type: 'array', items: { type: 'string' } },
  },
  required: ['requirements_file', 'decisions_file', 'requirement_count', 'decision_count', 'input_record_count'],
}

const MERGE_FORMAT = `Requirement row fields (JSONL, record_type "requirement"):
- record_type: "requirement"
- cluster: your cluster key
- canonical_key: short kebab slug unique within the cluster, stable and descriptive (e.g. "cdc-parameters-per-category")
- kind: INVARIANT | REQUIREMENT | FROZEN_DECISION | DEFERRED_QUESTION | OPEN_QUESTION | CLAIM_NEEDING_EVIDENCE | NON_GOAL | CAPABILITY
- requirement_or_question: precise paraphrase (<= 80 words)
- source: the originating authority id (SPEC, R1, R2, R3, R3-INSTR, APPX/<file>, docs/<file>, code:<path>, PROGRAM)
- source_location: exact section + line range in that source
- additional_sources: array of {source, location, local_key} for every merged duplicate/refinement
- superseding_authority: later authority that refines/freezes/overrides it, with location ("" if none). Authority order: repo docs and code at 9e44608 supersede SPEC where they explicitly freeze/refine; SPEC supersedes R1-R3 and APPX; PROGRAM (the research-program task statement dated 2026-09-12) adds research obligations but does not override product semantics.
- supersession_note: what changed and why ("" if none)
- implementation_state: IMPLEMENTED | PARTIAL | MISSING | BLOCKED_PLATFORM | DEFERRED_APPROVED | REJECTED (DEFERRED_APPROVED only when an authority explicitly defers it; REJECTED only when an authority explicitly refuses it; for non-implementable items such as pure research questions use the state of the capability they concern)
- implementation_evidence: code path:line refs and test names from code-* extraction records, or "none found"
- evidence_state: PROVEN | EMPIRICALLY_SUPPORTED | SUPPORTED_WITH_LIMITATIONS | UNTESTED | INSUFFICIENT | CONTRADICTED | EXTERNAL_REVIEW_REQUIRED (be strict: an implementation plus unit tests is not empirical support for an optimization claim; a frozen parameter chosen without measurement is UNTESTED or INSUFFICIENT; crypto primitive final selection routed to dedicated review is EXTERNAL_REVIEW_REQUIRED; PROVEN only for formally derivable definitional facts or exact conformance vectors)
- research_needed: concrete evidence required
- decision_needed: open decision phrased as a question, or ""
- decision_keys: array of decision_key slugs (from your decisions file) this requirement feeds
- release_relevance: V1_BLOCKING | V1_IMPORTANT | POST_V1 | INFORMATIONAL
- program_sections: array of research-program section numbers this maps to (e.g. ["8.1","10"]) from: 2,3,5,6,7,8.1,8.2,8.3,8.4,8.5,8.6,8.7,8.8,9,10,11,12,13,14,15,16,17,18,19,20,21,22.1,22.2,22.3,22.4,22.5,23,24,25,26,27,28,29,30,31,32,33,34,35

Decision row fields (JSONL, record_type "decision"):
- record_type: "decision"; cluster; decision_key (kebab slug unique within cluster)
- question: the exact product/technical question
- archetypal_objective: why the archetypal ideal of Entrybound cares (1-3 sentences)
- hard_constraints: array of invariant/constraint ids that bound it (I1-I31 from spec Appendix A, plus named repo freezes like "planner IDs frozen", "crypto-v1 wire frozen")
- candidate_set: array of {candidate_id, description} enumerating EVERY serious candidate including the status quo implementation and plausible alternatives from literature/incumbents; do not prune
- candidate_exclusion_reasons: array of {candidate_id, reason, invariant_violated} ONLY for candidates excluded purely by hard constraints already evident from sources; otherwise []
- required_evidence: array of concrete evidence items (experiments with metrics, platform runs, literature/primary-source reviews, external expert review)
- program_sections: as above
- requirement_keys: canonical_keys of the requirement rows feeding it
- initial_status: INSUFFICIENT_EVIDENCE, or EXTERNAL_REVIEW_REQUIRED only where an authority routes the question to independent security review
- external_review_requirement: "" or description
- release_relevance: V1_BLOCKING | V1_IMPORTANT | POST_V1 | INFORMATIONAL`

const merged = await parallel(CLUSTERS.map(c => () => agent(
`You are the merge agent for cluster "${c.key}" (${c.desc}) in the Entrybound research program's source-of-truth audit.

Inputs: every *.jsonl file in ${EXTRACT} (extraction records from the product architecture spec, Research I/II/III, research appendix, repo docs, and code audit at commit 9e44608) and the *-api-notes.md files there. Use ${PY} scripts to load all records; select those with primary_cluster == "${c.key}", and ALSO scan records of other clusters whose tags/statement clearly concern your cluster (take them only if their own cluster's topic would not own them; cross-reference instead of duplicating).

Task:
1. Merge semantically duplicate or refining records into canonical requirement rows. Every input record with primary_cluster "${c.key}" MUST be referenced by exactly one requirement row (as source or in additional_sources); list any you deliberately drop in unmapped_input_keys with a reason in gaps.
2. Determine supersession chains (spec -> repo docs -> code) and the current implementation and evidence state for each row, using code-* records as implementation evidence. Be skeptical and exact.
3. Produce decision rows: one per distinct unresolved or evidence-dependent product/technical question in your cluster, including questions that are currently "frozen" but whose frozen values were never empirically justified (those still need evidence-backed confirmation or a new-version proposal). Also create decision rows for EVERY research-program question below even if no source mentions it (source PROGRAM):
${PROGRAM_QUESTIONS[c.key]}
4. Add requirement rows with source "PROGRAM" for research-program obligations in your cluster not otherwise represented.

${MERGE_FORMAT}

Write requirement rows to ${MERGED}/${c.key}-requirements.jsonl and decision rows to ${MERGED}/${c.key}-decisions.jsonl (overwrite). Validate every line parses and every decision_keys reference resolves to a decision row in your file and every requirement_keys reference resolves. Do not edit other files; no git state changes. Return counts, input_record_count considered, unmapped keys, and gaps.`,
  { label: `merge:${c.key}`, phase: 'Merge', schema: MERGE_SCHEMA })))
merged.forEach((m, i) => log(m ? `merge ${CLUSTERS[i].key}: ${m.requirement_count} reqs, ${m.decision_count} decisions (from ${m.input_record_count})` : `merge ${CLUSTERS[i].key}: FAILED`))

phase('Assemble')
const ASSEMBLE_SCHEMA = {
  type: 'object',
  properties: {
    requirement_rows: { type: 'integer' },
    decision_rows: { type: 'integer' },
    new_ids_assigned: { type: 'integer' },
    validation_errors: { type: 'array', items: { type: 'string' } },
    counts_by_implementation_state: { type: 'object', additionalProperties: { type: 'integer' } },
    counts_by_evidence_state: { type: 'object', additionalProperties: { type: 'integer' } },
    counts_by_status: { type: 'object', additionalProperties: { type: 'integer' } },
  },
  required: ['requirement_rows', 'decision_rows', 'validation_errors'],
}
const ASSEMBLE_PROMPT = (round) => `You are the ledger assembler for the Entrybound research program (round ${round}).

Write (or update if it exists) a deterministic, rerunnable Python 3 tool at ${REPO}/research/tools/ledger/assemble.py (standard library only) plus ${REPO}/research/tools/ledger/README.md, then run it with ${PY}. The tool must:
1. Load ${MERGED}/*-requirements.jsonl, ${MERGED}/*-decisions.jsonl, and every ${ADDENDA}/*.jsonl (addenda rows use the same record formats with record_type "requirement" or "decision"; an addendum row whose cluster+canonical_key/decision_key already exists UPDATES that row by merging additional_sources and replacing non-empty fields).
2. Assign stable IDs via ${REPO}/research/audit/id-map.json (create if absent): requirement key "<cluster>/<canonical_key>" -> "REQ-<CODE>-<NNNN>", decision key "<cluster>/<decision_key>" -> "DEC-<CODE>-<NNN>", cluster codes ${CLUSTERS.map(c => c.key + '=' + c.code).join(', ')}. Existing IDs are never renumbered or reused; new keys get the next number in sorted key order. Detect key collisions across files and disambiguate deterministically, reporting them.
3. Write ${REPO}/research/requirement-ledger.csv (RFC 4180, UTF-8, header row) with columns exactly: req_id, cluster, kind, requirement_or_question, source, source_location, additional_sources, superseding_authority, supersession_note, implementation_state, implementation_evidence, evidence_state, research_needed, decision_needed, decision_ids, release_relevance, program_sections. additional_sources as "source@location" joined by " | "; decision_ids and program_sections joined by ";". Sort by req_id.
4. Write ${REPO}/research/decision-ledger.jsonl sorted by decision_id with exactly these fields: decision_id, status (from initial_status unless an existing ledger row already has a later status — preserve evidence fields and status from any existing ${REPO}/research/decision-ledger.jsonl rows with the same decision_id so later research is never clobbered), blocker_class (NONE|EVIDENCE|PLATFORM|EXTERNAL_REVIEW|HUMAN_PARTICIPANTS|RESOURCES; default EVIDENCE, EXTERNAL_REVIEW when status is EXTERNAL_REVIEW_REQUIRED), cluster, question, requirement_ids (resolved REQ ids), program_sections, release_relevance, archetypal_objective, hard_constraints, candidate_set, candidate_exclusion_reasons, required_evidence, experiment_ids [], raw_result_refs [], normalized_result_refs [], heldout_result_refs [], counterevidence "", sensitivity_analysis "", complexity_cost "", dependency_cost "", security_cost "", wire_cost "", selected_decision "", decision_scope "", confidence "", known_limitations "", reopen_trigger "", external_review_requirement, remaining_unknowns_ref "", history [{"date":"2026-09-12","change":"created by Phase A audit"}] for new rows.
5. Write ${REPO}/research/tools/ledger/decision-ledger.schema.json (JSON Schema for the decision rows, enumerating status values DECIDED, INSUFFICIENT_EVIDENCE, EXTERNAL_REVIEW_REQUIRED) and requirement-ledger.schema.json, and validate both ledgers against the allowed enumerations: implementation_state {IMPLEMENTED, PARTIAL, MISSING, BLOCKED_PLATFORM, DEFERRED_APPROVED, REJECTED}; evidence_state {PROVEN, EMPIRICALLY_SUPPORTED, SUPPORTED_WITH_LIMITATIONS, UNTESTED, INSUFFICIENT, CONTRADICTED, EXTERNAL_REVIEW_REQUIRED}. Invalid enumerations are validation errors to FIX in the merged inputs (edit the offending merged/addenda row minimally, log what you changed in ${REPO}/research/audit/assembly-log.md) and then rerun.
6. Validate referential integrity: every decision references >=1 requirement; every requirement with a non-empty decision_needed references >=1 decision; every research-program section in [8.1..8.8, 9..35 plus 22.1..22.5] appears in at least one decision's program_sections; report any violations and fix them by adding minimal addendum rows in ${ADDENDA}/assembler-round${round}.jsonl (with source PROGRAM) and rerunning.
7. Write ${REPO}/research/supersession-ledger.md: for every requirement with a superseding_authority or supersession_note, a table row (req_id, original authority@location, superseding authority@location, nature of change, current governing authority), grouped by cluster, generated by the tool.
8. Write ${REPO}/research/source-inventory.md generated by the tool: for each source (spec, R1, R2, R3, each research-appendix member, each Research III instrumentation member, each repo doc, each crate source/test file, tools, Cargo manifests) list path, SHA-256, byte size, line count, role, authority rank, extraction slice keys, and number of ledger rows citing it. Sources: ${SPEC}; ${R1}; ${R2}; ${R3}; D:/Projects/entrybound/design/research-appendix/entrybound-architecture-research-appendix.tgz (members extracted at ${APPX}); D:/Projects/entrybound/research/entrybound-research-iii-instrumentation.tgz (members at ${R3I}); ${REPO} tracked files at 9e44608. State explicitly that the spec and Research I-III are external, unpublished inputs referenced by hash and are NOT copied into the repository.
Return row counts, counts by implementation state, evidence state and status, and any unresolved validation errors.`

await agent(ASSEMBLE_PROMPT(0), { label: 'assemble:round0', phase: 'Assemble', schema: ASSEMBLE_SCHEMA }).then(r => log(r ? `assembled: ${r.requirement_rows} requirements, ${r.decision_rows} decisions, errors=${r.validation_errors.length}` : 'assemble FAILED'))

phase('Critique')
const CRITIC_SCHEMA = {
  type: 'object',
  properties: {
    additions_file: { type: 'string' },
    additions_count: { type: 'integer' },
    updates_count: { type: 'integer' },
    notes: { type: 'array', items: { type: 'string' } },
  },
  required: ['additions_count', 'updates_count'],
}
const FAMILIES = [
  { key: 'spec', what: `the Product Architecture Specification ${SPEC} (all 2754 lines; especially §4.9, §5.7a, §7.3-§7.6, §9.6-§9.11, §10, §11.5, §13, §14.3-§14.4, §15.4, §16, §17.2, §18, §19.2-§19.5, §20, §21, §22.4-§22.5, §23.4, §24, §25, §26)` },
  { key: 'research', what: `Research I ${R1}, Research II ${R2}, Research III ${R3} (especially their open-question, unresolved, falsification, NO-GO, and decisions-not-to-make-yet sections)` },
  { key: 'appendix', what: `the research appendix files ${APPX}/chunking.md, crypto.md, metadata.md, incumbents.md and the Research III instrumentation at ${R3I}` },
  { key: 'docs', what: `every document in ${REPO}/docs (40 files), especially sections titled non-goals, limitations, deferred, future, residual risk, open, not implemented` },
  { key: 'code', what: `the code at ${REPO}/crates (grep for refusals, unsupported, NotSupported/Unsupported reason codes, todo!, unimplemented!, FidelityReport capability-unavailable paths, hard-coded limits, frozen IDs) and ${REPO}/tools` },
  { key: 'program', what: `the research program's own explicit questions, listed here by cluster:\n${CLUSTERS.map(c => `[${c.key}]\n${PROGRAM_QUESTIONS[c.key]}`).join('\n')}\nplus program obligations: incumbent baselines (task 6: ZIP/DEFLATE, tar+gzip, tar+zstd, tar+xz, 7z/LZMA2, others; capability-matched), whole-system measurement framework (task 7), corpus completeness (task 5, 20 workload families, tuning/validation/held-out), archetypal objective & decision method (task 3), reproducibility (task 37), evidence classification (38), negative results (39), stable-v1 dispositions (34), final held-out evaluation (35)` },
]

let round = 1
while (round <= 3) {
  const results = await parallel(FAMILIES.map(f => () => agent(
`You are a completeness critic (round ${round}) for the Entrybound research program's Requirement and Decision Ledgers. Automatic failure condition: a research question or deferred/unresolved architecture question disappears instead of receiving a ledger record.

Ledgers: ${REPO}/research/requirement-ledger.csv and ${REPO}/research/decision-ledger.jsonl (read them fully; grep them). Source family to check: ${f.what}.

Read the source family directly (not summaries). Find every requirement, invariant, deferred matter, open or unresolved question, evidence-dependent claim, unimplemented capability, or research obligation that is NOT represented by any ledger row (represented = a row whose meaning covers it, even if phrased differently), and every ledger row whose implementation_state/evidence_state/supersession is demonstrably wrong against the sources. Be adversarial and specific; do not add near-duplicates of existing rows.

For each gap, write an addendum row to ${ADDENDA}/round${round}-${f.key}.jsonl using the merged row formats:
${MERGE_FORMAT}
For a correction to an existing row, emit a row with the SAME cluster and canonical_key/decision_key (look up the key in ${REPO}/research/audit/id-map.json) containing only the corrected fields plus the key fields, and add "correction_reason". If there are no gaps and no corrections, do not create a file and return additions_count 0 and updates_count 0. Validate JSON lines. No git state changes. Return counts and short notes naming the most significant gaps.`,
    { label: `critic:${f.key}:r${round}`, phase: 'Critique', schema: CRITIC_SCHEMA })))
  const adds = results.filter(Boolean).reduce((a, r) => a + r.additions_count + r.updates_count, 0)
  log(`critique round ${round}: ${adds} additions/updates (${results.map((r, i) => `${FAMILIES[i].key}=${r ? r.additions_count + '+' + r.updates_count : 'FAIL'}`).join(', ')})`)
  if (adds === 0) break
  await agent(ASSEMBLE_PROMPT(round), { label: `assemble:round${round}`, phase: 'Assemble', schema: ASSEMBLE_SCHEMA }).then(r => log(r ? `reassembled r${round}: ${r.requirement_rows} requirements, ${r.decision_rows} decisions, errors=${r.validation_errors.length}` : 'reassemble FAILED'))
  round++
}
if (round > 3) log('critique loop stopped at 3 rounds without a dry round — residual gaps possible; flag for next phase')

phase('Method')
const METHOD_SCHEMA = { type: 'object', properties: { files: { type: 'array', items: { type: 'string' } }, summary: { type: 'string' } }, required: ['files', 'summary'] }
const objective = agent(
`Write ${REPO}/research/archetypal-objective.md for the Entrybound research program. Read ${REPO}/research/requirement-ledger.csv (INVARIANT and REQUIREMENT rows), ${SPEC} §1-§3.9, §6.5, §8, §11, §12, §23, Appendix A, and ${REPO}/CONTRIBUTING.md.

Required content:
1. Governing question: what design most completely realizes the archetypal ideal of Entrybound as the definitive next-generation archive system, subject to semantic, security, fidelity, determinism, interoperability, bounded-resource, longevity, and usability invariants.
2. Hard constraints (HC-01..): at minimum one authority per semantic fact; exact logical losslessness; deterministic native interpretation; fail-closed unknown critical semantics; caller-owned extraction/security policy; declared resource bounds; no silent metadata/semantic loss; archive semantics independent of host filesystem interpretation; bounded dependency chains; canonical identities; no runtime-dependent decoder interpretation; independently implementable native specification; reproducible deterministic modes; crypto correctness never traded for performance. Map each HC to spec invariants I1-I31 and repo freezes, and state a mechanical test for violation (how an experiment or review detects that a candidate violates it). Every I1-I31 must map to at least one HC.
3. Optimization dimensions (OD-01..) covering at least: semantic coherence, exact losslessness, preservation fidelity, deterministic interpretation, compression/storage efficiency, creation performance, decoding performance, memory/scratch, streaming, random access, verified partial retrieval, remote access, parallelism, corruption locality, confidentiality/authenticity, metadata privacy, resource-bounded decoding, cross-platform fidelity, legacy interoperability, deterministic migration, independent implementability, dependency longevity, specification complexity, implementation attack surface, user complexity, adoption friction. For EACH: precise metric definition(s), unit, measurement procedure/instrument, direction (min/max), aggregation across workloads (e.g. geometric mean of ratios per family then median across families), and which dimensions are binary constraints versus graded objectives. No vague labels.
4. Relationship between hard constraints and dimensions (a candidate violating an HC is invalid regardless of scores).
Mark each material claim with an evidence class tag: FORMALLY_DERIVED, EMPIRICALLY_MEASURED, EXTERNALLY_SOURCED, INFERRED, EXPERT_REVIEW_REQUIRED, UNRESOLVED. Return files and a summary.`,
  { label: 'method:objective', phase: 'Method', schema: METHOD_SCHEMA })
const method = agent(
`Write ${REPO}/research/decision-method.md for the Entrybound research program. This document is PRE-REGISTERED: it will be committed before any experiment result is observed, so every threshold must be fixed now. Read ${REPO}/research/decision-ledger.jsonl (skim candidate sets to calibrate), ${REPO}/research/requirement-ledger.csv, and the spec ${SPEC} §5.4, §6, §22.1.

Required content:
1. Decision rule, in this order: (1) eliminate invariant violations; (2) eliminate designs that cannot be bounded or independently specified; (3) measure surviving candidates; (4) eliminate Pareto-dominated candidates (define epsilon-dominance using the practical-significance thresholds); (5) validate survivors on held-out workloads; (6) account for permanent implementation/specification/dependency cost; (7) sensitivity analysis; (8) choose a universal default only when evidence supports one; (9) otherwise profile/policy/workload-specific winners; (10) prefer the simpler design when differences are statistically or practically negligible.
2. Pre-registered practical-significance thresholds with units for every measured metric family: total artifact bytes, metadata/index/dictionary/side-data bytes, encode/decode wall time, CPU time, peak RSS, peak scratch, first-entry and random-entry latency, HTTP bytes and request count, verification overhead, parallel speedup, corruption blast radius, leakage measures (e.g. attack success rate / bits of information), padding overhead, usability task success. Give both a relative and an absolute floor where appropriate, and justify each briefly.
3. Statistical protocol: repetitions (minimum count and adaptive rule), warmup policy, CPU affinity/power/thermal controls for a laptop i9-14900HX (hybrid P/E cores) running Windows 11 and WSL2 Ubuntu 24.04, reporting median/p90/p95/dispersion (IQR, MAD) with raw samples, bootstrap confidence intervals for medians and for median ratios (paired where the same input is used), a noise rule (if CIs overlap the threshold band, report "not distinguishable" and do not claim a winner), multiple-comparison handling, and treatment of deterministic byte-count metrics (single run with determinism verified by repeat hash).
4. Corpus split discipline: tuning vs validation vs held-out; the held-out set is frozen by manifest hash and may be accessed only after a recorded design-freeze commit; any leakage invalidates the affected decisions.
5. Sensitivity analysis protocol: objective weight perturbations (e.g. each weight ±25% and ±50%, random Dirichlet samples), threshold perturbations, workload-mix reweighting; stability metric (e.g. fraction of perturbations preserving the winner, Kendall tau of rankings) and the pre-registered stability bar required to freeze a default.
6. Cost accounting for permanent costs: specification complexity (normative spec words/sections/new record types/wire fields), implementation complexity (LoC, cyclomatic proxies), dependency cost (transitive crate count, unsafe count, C/C++ code, license, maintenance signals), attack surface (parser states, attacker-controlled allocations), independent-implementability; how these trade against measured gains (e.g. minimum gain required to justify a new codec or record type).
7. Decision record status rules: DECIDED only when all serious candidates enumerated, exclusions justified, evidence linked, held-out validation for empirical decisions, counterevidence discussed, sensitivity measured, practical significance considered, complexity/dependency cost included, and the rule followed; otherwise INSUFFICIENT_EVIDENCE; EXTERNAL_REVIEW_REQUIRED for questions needing independent expert review. Confidence scale definitions (e.g. HIGH/MEDIUM/LOW with criteria).
8. Evidence classification tags: FORMALLY_DERIVED, EMPIRICALLY_MEASURED, EXTERNALLY_SOURCED, INFERRED, EXPERT_REVIEW_REQUIRED, UNRESOLVED; primary-source citation rule.
9. Negative-results obligation and confirmation-bias review trigger.
10. Experiment record template (experiment ID, question, hypothesis, candidates, corpus IDs, environment fingerprint, commands/code, seeds, warmup, measurement method, replication count, raw results, normalization, statistical summary, plots/tables, interpretation, threats to validity, decisions informed) and the rule that every number in a report must be generated from raw artifacts by a checked-in command.
Return files and a summary.`,
  { label: 'method:decision-method', phase: 'Method', schema: METHOD_SCHEMA })
const drafts = await Promise.all([objective, method])
log(`method drafts: ${drafts.filter(Boolean).length}/2`)

const review = await agent(
`Adversarially review two pre-registration documents for the Entrybound research program: ${REPO}/research/archetypal-objective.md and ${REPO}/research/decision-method.md. Also consult ${REPO}/research/decision-ledger.jsonl and ${SPEC} Appendix A.

Look for: vague or unmeasurable metrics; missing optimization dimensions or hard constraints; invariants I1-I31 not mapped; thresholds that are unjustified, gameable, inconsistent across metric families, or so loose/tight that every comparison becomes a tie/win; statistical protocol gaps (noise, laptop thermal/hybrid-core confounds, WSL2 virtualization, paired designs, multiple comparisons); held-out leakage paths; sensitivity analysis that cannot detect instability; cost accounting that ignores permanent spec/dependency cost; status rules that allow a decision to be marked DECIDED prematurely; anything that conflicts with the spec or repo freezes. Write concrete findings with required fixes to ${REPO}/research/methods/method-review-round1.md. Return files and a summary listing the top findings.`,
  { label: 'method:adversarial-review', phase: 'Method', schema: METHOD_SCHEMA })

await agent(
`Revise ${REPO}/research/archetypal-objective.md and ${REPO}/research/decision-method.md to resolve every finding in ${REPO}/research/methods/method-review-round1.md. For any finding you reject, justify it in an appended "Review disposition" section of the review file. Add a "Revision history" section to each document. Keep all thresholds concrete and pre-registered. Return files and a summary of changes.`,
  { label: 'method:revise', phase: 'Method', schema: METHOD_SCHEMA })

return { extractSummary, merged: merged.map((m, i) => ({ cluster: CLUSTERS[i].key, ...(m || { failed: true }) })), critiqueRounds: round, review: review ? review.summary : null }
