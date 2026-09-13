export const meta = {
  name: 'eb-research-phase-a2-ledgers-method',
  description: 'Entrybound research Phase A continuation: remaining cluster merges (sharded), ledger assembly, completeness critics, pre-registered method docs — with checkpoint commits',
  phases: [
    { title: 'Merge', detail: '8 merge shards over pre-split inputs' },
    { title: 'Assemble', detail: 'stable-ID ledger assembly' },
    { title: 'Critique', detail: 'completeness critics, max 2 rounds' },
    { title: 'Method', detail: 'archetypal objective + decision method, adversarial review, thresholds' },
  ],
}

const REPO = 'D:/Projects/entrybound/entrybound'
const SPEC = 'D:/Projects/entrybound/design/2026-08-29-entrybound-product-architecture.md'
const R1 = 'D:/Projects/entrybound/research/2026-08-29-archive-category-research.md'
const R2 = 'D:/Projects/entrybound/research/2026-08-29-entrybound-opportunity-validation.md'
const R3 = 'D:/Projects/entrybound/research/2026-08-29-entrybound-research-iii-final-gate.md'
const APPX = 'D:/eb-research/sources/appendix'
const R3I = 'D:/eb-research/sources/r3-instrumentation'
const EXTRACT = REPO + '/research/audit/extract'
const BYC = REPO + '/research/audit/by-cluster'
const MERGED = REPO + '/research/audit/merged'
const ADDENDA = REPO + '/research/audit/addenda'
const PY = 'C:/Python313/python.exe'
const CKPT = `${PY} ${REPO}/research/orchestration/checkpoint.py`

const CHECKPOINT_RULE = `Never run state-changing git commands directly. When your output is COMPLETE and VALIDATED, commit exactly your own output paths with the serialized checkpoint tool (it appends a PROGRESS.md change-log line and pushes to origin/dev):
  ${CKPT} --subject "<imperative subject <=72 chars>" --body "<what and why>" --note "<one-line progress note>" -- <your paths...>
If it reports a lock timeout, retry. If it reports a git error, include that in your return value and do not attempt manual git. Report the commit hash printed by the tool.`

const CLUSTERS = [
  { key: 'model', code: 'MOD' }, { key: 'container', code: 'CON' }, { key: 'compression', code: 'CMP' },
  { key: 'access', code: 'ACC' }, { key: 'platform', code: 'PLT' }, { key: 'crypto', code: 'CRY' },
  { key: 'legacy', code: 'LEG' }, { key: 'integrity', code: 'INT' }, { key: 'ecosystem', code: 'ECO' },
]

const PROGRAM = {
  container: `- Task 11: current manifest vs compressed vs segmented vs indexed/tree manifest; current Index vs range-friendly alternatives; Merkle outboard layouts; proof bytes/CPU/requests/cacheability/amplification; is a format revision justified; does the uncompressed-manifest default cost enough at very large entry counts to change it (spec 25.3).
- Default resource-budget values (25.1 #16, #23); magic-number confirmation against identification databases (25.1 #15); media-type registration path.`,
  compression: `- Task 8.1 CDC: normalized Gear vs fixed-size vs other credible CDC (FastCDC variants, Rabin, BuzHash, AE, RAM, SeqCDC), min/target/max sweeps; ratio, dedup, index overhead, CPU, memory, boundary stability, update locality, random-access amplification; category-specific and aggregate Pareto frontiers (25.1 #6, 25.3).
- Task 8.2 dedup scope: none/whole-file/exact-chunk/archive-wide/windowed; when dedup metadata cost exceeds benefit; tiny files; encrypted creation.
- Task 8.3 similarity clustering: bottom-k vs alternatives; accuracy, CPU/memory, false-group cost, cohort size sensitivity.
- Task 8.4 dictionaries: training size, dictionary size, level, cohort thresholds, reuse count; minimum-benefit rule charging dictionary bytes & decoder memory.
- Task 8.5 lookback 0/1/2/4/8+: gain vs first-byte latency, access amplification, dependency bytes, parallelism, corruption propagation (25.3).
- Task 8.6 codec portfolio incl. plausible additions; ratio/speed/memory/decoder complexity/dependency quality/spec availability/independent implementability; baseline vs extended codec set (25.1 #3, #4).
- Task 8.7 structural transforms: delta, shuffle, BCJ family, numeric transforms, combinations; false-positive cost (25.1 #5).
- Task 8.8 reconstructive transforms: DEFLATE, baseline/progressive JPEG, others if prevalent; prevalence, roundtrip success, side-data, net savings, costs, burden (25.3).
- Task 9 planner optimality: regret vs exhaustive search by workload class; heuristic/DP/pruning/deterministic data-driven; is complexity justified (25.1 #17). Note: planning is currently in-memory and single-threaded.
- Task 10 profiles: are four appropriate; is balanced the defensible default; per-profile parameters; stability under weight perturbation.`,
  platform: `- Task 15 cross-platform fidelity corpus results on real OS/filesystems (names, links, reparse, hardlinks, mode, ids, ACLs, xattrs, sparse, security descriptors, Windows attributes, timestamps, macOS flags, forks, ADS, origin markers, devices, FIFOs, sockets).
- Task 16 native paths on ext4/XFS/btrfs/APFS/NTFS/ReFS: representation, invalid UTF-8, unpaired surrogates, case folding, normalization, reserved components, length, trailing chars, collisions; exact path encoding & extraction refusal rules.
- Task 17 streams/forks per class: LAI vs AUX vs separate named-content semantic vs refuse.
- Task 18 special files: first-class EntryKind vs auxiliary snapshot metadata vs FidelityReport-only vs refuse.
- Task 19 ownership & identity: numeric ids, names, user namespaces, containers, NFS idmap, AD SIDs, cross-machine restore; portable AUX vs platform metadata vs identity/strict-v1.
- Task 20 origin/provenance markers: preserve raw only, canonical origin semantic, propagate to extracted members, caller policy.
- Task 21 safe Rust API feasibility for Windows security descriptors, opaque reparse data, no-follow creation/restoration, macOS ACLs, birthtime, forks, no-follow metadata ops.`,
  'crypto-a': `- Task 22.1 primitive review of AES-256-GCM-SIV, HKDF-SHA-256, HMAC commitments, X-Wing, Argon2id (+default params, 25.1 #12), SHA-256 identities against primary standards/current guidance (25.1 #9-#12, 25.2).
- Task 22.2 boundary leakage attacks reproduced: public CDC vs secret Gear vs PHTE/AES; leakage reduction, CPU, boundary stability, compression effect (25.2).
- Task 22.3 padding schedules NONE/BUCKETED/MAXIMUM/alternatives: privacy vs storage overhead (25.1 #13).
- Encrypted metadata privacy, nonces, key hierarchy/commitment formal analysis for external review (25.2).`,
  'crypto-b': `- Task 22.1 (signatures part): Ed25519 and RFC3161 review.
- Task 22.4 signature/trust: self-signed vs trusted publisher, allowed-key sets, multiple signatures, partial bindings, combining bindings across signers, recipient mutation, cross-context forwarding, stale physical/addressing bindings, timestamps; distinguish cryptographically valid / binding current / authorized signer / trusted timestamp (25.2).
- Task 22.5 composition matrix of conversion evidence, preservation, encryption, signatures, recipient changes, password rotation, random access, repack.
- External-review dossier and named-adversary threat model (25.2).`,
  'legacy-a': `- Task 25 legacy differential across ZIP runtimes, GNU tar, bsdtar/libarchive, Python tarfile, 7z implementations, dialect extensions: compatibility profiles derived from observed behavior (tar/7z compat & preservation currently ZIP-only); --compat behavioural models (25.1 #22).
- Import resource-budget default values (25.1 #23); strict-mode conflict classes and observed prevalence (Research III).`,
  'legacy-b': `- Task 26 adapter priority: OCI, NAR, RAR5, cpio, ar, WIM, squashfs, lzip, LZ4 framing, others -> CORE_V1/POST_V1_CORE/ECOSYSTEM/NOT_JUSTIFIED.
- Task 27 export fidelity: standard tar, pax, GNU, star, ZIP Unix extras, platform extras; new target profiles and truthful guarantees; frozen profiles untouched; 7z export question.
- Dual publishing, sidecars, receipts/chained provenance (spec 15-16) as they affect stable v1.`,
  ecosystem: `- Task 28 independent reader from written spec; ambiguity log; production vs independent differential.
- Task 29 versioned native conformance corpus with REQUIRED/FORBIDDEN/ACCEPTABLE/PROFILE_DEPENDENT/UNRESOLVED classes (UNRESOLVED empty for RC).
- Task 30 persistent fuzz targets & differential fuzzing; findings minimized into regressions.
- Task 31 dependency audit & classification (baseline reader/extended decoder/writer only/adapter only/test-research only); influence on codec/feature choices (25.1 #19).
- Task 32 adoption workflows: commands/options, failure modes, migration friction, value before ecosystem adoption.
- Task 33 CLI/user complexity: which concepts hidden behind defaults vs explicit policy.
- Task 34 stable-v1 disposition for every unfinished capability; Task 35 final held-out system evaluation incl. workloads where Entrybound is not best; incumbent comparison fairness; Task 6/7 baselines & measurement framework obligations.`,
}

const MERGE_FORMAT = `Requirement row fields (JSONL, record_type "requirement"):
- record_type: "requirement"; cluster: the CLUSTER key (not the shard name)
- canonical_key: short kebab slug unique within the cluster, stable and descriptive
- kind: INVARIANT | REQUIREMENT | FROZEN_DECISION | DEFERRED_QUESTION | OPEN_QUESTION | CLAIM_NEEDING_EVIDENCE | NON_GOAL | CAPABILITY
- requirement_or_question: precise paraphrase (<= 80 words)
- source: originating authority id (SPEC, R1, R2, R3, R3-INSTR, APPX/<file>, docs/<file>, code:<path>, PROGRAM)
- source_location: exact section + line range, with the primary record's local_key in brackets
- additional_sources: array of {source, location, local_key} for every merged duplicate/refinement
- superseding_authority: later authority that refines/freezes/overrides it, with location ("" if none). Authority order: repo docs and code at 9e44608 supersede SPEC where they explicitly freeze/refine; SPEC supersedes R1-R3 and APPX; PROGRAM (research-program task statement 2026-09-12) adds research obligations but does not override product semantics.
- supersession_note
- implementation_state: IMPLEMENTED | PARTIAL | MISSING | BLOCKED_PLATFORM | DEFERRED_APPROVED | REJECTED
- implementation_evidence: code path:line refs and test names, or "none found"
- evidence_state: PROVEN | EMPIRICALLY_SUPPORTED | SUPPORTED_WITH_LIMITATIONS | UNTESTED | INSUFFICIENT | CONTRADICTED | EXTERNAL_REVIEW_REQUIRED (strict: implementation + unit tests is not empirical support for an optimization claim; frozen parameters chosen without measurement are UNTESTED/INSUFFICIENT; primitive final selection routed to dedicated review is EXTERNAL_REVIEW_REQUIRED; PROVEN only for definitional facts or exact conformance vectors)
- research_needed; decision_needed (question or ""); decision_keys: array of decision_key slugs
- release_relevance: V1_BLOCKING | V1_IMPORTANT | POST_V1 | INFORMATIONAL
- program_sections: array from 2,3,5,6,7,8.1-8.8,9,10,11..21,22.1-22.5,23..35

Decision row fields (JSONL, record_type "decision"):
- record_type "decision"; cluster; decision_key (kebab slug unique within cluster)
- question; archetypal_objective (1-3 sentences); hard_constraints (I1-I31 ids and named repo freezes)
- candidate_set: [{candidate_id, description}] — EVERY serious candidate incl. status quo and literature/incumbent alternatives; do not prune
- candidate_exclusion_reasons: [{candidate_id, reason, invariant_violated}] only for hard-constraint exclusions evident from sources
- required_evidence: concrete experiments/platform runs/literature reviews/external reviews
- program_sections; requirement_keys (canonical_keys); initial_status INSUFFICIENT_EVIDENCE or EXTERNAL_REVIEW_REQUIRED (only where an authority routes to independent security review); external_review_requirement; release_relevance`

phase('Merge')
const MERGE_SCHEMA = { type: 'object', properties: { requirements_file: { type: 'string' }, decisions_file: { type: 'string' }, requirement_count: { type: 'integer' }, decision_count: { type: 'integer' }, input_record_count: { type: 'integer' }, unmapped_input_keys: { type: 'array', items: { type: 'string' } }, gaps: { type: 'array', items: { type: 'string' } }, commit: { type: 'string' } }, required: ['requirements_file', 'decisions_file', 'requirement_count', 'decision_count', 'input_record_count'] }

const SHARDS = [
  { shard: 'container', cluster: 'container', inputs: [`${BYC}/container.jsonl`, `${BYC}/container-secondary.jsonl`] },
  { shard: 'compression', cluster: 'compression', inputs: [`${BYC}/compression.jsonl`, `${BYC}/compression-secondary.jsonl`] },
  { shard: 'platform', cluster: 'platform', inputs: [`${BYC}/platform.jsonl`, `${BYC}/platform-secondary.jsonl`] },
  { shard: 'crypto-a', cluster: 'crypto', inputs: [`${BYC}/crypto-a.jsonl`, `${BYC}/crypto-secondary.jsonl`], scope: 'primitives, suite, envelope, KDF/key hierarchy, commitment, nonces, recipients and passwords (the recipient STANZA/KEM side), padding, encrypted CDC boundary leakage, encrypted metadata/privacy, crypto resource limits. The sibling shard crypto-b covers signatures, timestamps, trust, key management/recipient mutation, composition, threat model, review; use decision/canonical keys prefixed only by topic (never by shard), and if a record straddles both, own it only if its main subject is in your scope.' },
  { shard: 'crypto-b', cluster: 'crypto', inputs: [`${BYC}/crypto-b.jsonl`], scope: 'signatures and bindings, Ed25519, RFC 3161 timestamps, trust semantics, key management/recipient mutation/password rotation, feature composition, threat model, crypto review findings and external-review routing. The sibling shard crypto-a covers primitives, envelope, KDF, commitment, nonces, recipients/KEM, padding, CDC leakage, encrypted metadata.' },
  { shard: 'legacy-a', cluster: 'legacy', inputs: [`${BYC}/legacy-a.jsonl`, `${BYC}/legacy-secondary.jsonl`], scope: 'legacy IMPORT: ZIP/tar/7z/compressed streams, Legacy Observation Model, reconciliation and conflict classes, strict/--compat/--preserve modes, runtime matrices, import budgets. The sibling shard legacy-b covers export, target profiles, LOSSLESS/LOSSY/REFUSED, publish/migration, sidecars, receipts/provenance, adapter priority.' },
  { shard: 'legacy-b', cluster: 'legacy', inputs: [`${BYC}/legacy-b.jsonl`], scope: 'legacy EXPORT and migration: target profiles, LOSSLESS/LOSSY/REFUSED, ExportReceipt, dual publishing, MigrationReport, sidecars, conversion receipts & chained provenance, adapter priority classification. The sibling shard legacy-a covers import adapters, LOM, reconciliation, compat/preserve modes.' },
  { shard: 'ecosystem', cluster: 'ecosystem', inputs: [`${BYC}/ecosystem.jsonl`, `${BYC}/ecosystem-secondary.jsonl`] },
]

const merged = await parallel(SHARDS.map(s => () => agent(
`You are the merge agent for shard "${s.shard}" of cluster "${s.cluster}" in the Entrybound research program's source-of-truth audit (resumed after a usage-limit interruption; no prior output exists for this shard).
${s.scope ? `Shard scope: ${s.scope}\n` : ''}
Inputs (pre-split by research/tools/ledger/split_by_cluster.py from ${EXTRACT}/*.jsonl): primary records ${s.inputs[0]}${s.inputs[1] ? `; secondary (other clusters' records with overlapping tags — take one only if your cluster's topic clearly owns it, otherwise cross-reference by local_key in text) ${s.inputs[1]}` : ''}. The *-api-notes.md files in ${EXTRACT} describe code areas. Load records with ${PY} scripts; read the records in batches rather than dumping everything into context at once. Already-merged clusters for cross-reference: ${MERGED}/{model,access,integrity}-{requirements,decisions}.jsonl.

Task:
1. Merge semantically duplicate or refining records into canonical requirement rows. Every primary input record MUST be referenced by exactly one requirement row (as source or in additional_sources); list any deliberately dropped key in unmapped_input_keys with a reason in gaps. Verify coverage with a script before finishing.
2. Determine supersession chains (spec -> repo docs -> code) and current implementation/evidence states using code:* records as implementation evidence; spot-check high-impact claims in the code at ${REPO}/crates.
3. Produce decision rows: one per distinct unresolved or evidence-dependent question in scope, including "frozen" values never empirically justified. Also create decision rows for EVERY research-program question below even if no source mentions it (source PROGRAM):
${PROGRAM[s.shard]}
4. Add requirement rows with source PROGRAM for research-program obligations in scope not otherwise represented.

${MERGE_FORMAT}

Write requirement rows to ${MERGED}/${s.shard}-requirements.jsonl and decision rows to ${MERGED}/${s.shard}-decisions.jsonl (cluster field = "${s.cluster}"). Validate every line parses, every decision_keys reference resolves within your two files, and every requirement_keys reference resolves.
${CHECKPOINT_RULE}
Checkpoint paths: research/audit/merged/${s.shard}-requirements.jsonl research/audit/merged/${s.shard}-decisions.jsonl. Return counts, input_record_count, unmapped keys, gaps, commit.`,
  { label: `merge:${s.shard}`, phase: 'Merge', schema: MERGE_SCHEMA })))
merged.forEach((m, i) => log(m ? `merge ${SHARDS[i].shard}: ${m.requirement_count} reqs, ${m.decision_count} decisions (from ${m.input_record_count}) ${m.commit || ''}` : `merge ${SHARDS[i].shard}: FAILED`))
const failedShards = SHARDS.filter((s, i) => !merged[i]).map(s => s.shard)
if (failedShards.length) log(`WARNING failed shards will be missing from assembly: ${failedShards.join(', ')}`)

phase('Assemble')
const ASSEMBLE_SCHEMA = { type: 'object', properties: { requirement_rows: { type: 'integer' }, decision_rows: { type: 'integer' }, new_ids_assigned: { type: 'integer' }, validation_errors: { type: 'array', items: { type: 'string' } }, counts_by_implementation_state: { type: 'object', additionalProperties: { type: 'integer' } }, counts_by_evidence_state: { type: 'object', additionalProperties: { type: 'integer' } }, counts_by_status: { type: 'object', additionalProperties: { type: 'integer' } }, commit: { type: 'string' } }, required: ['requirement_rows', 'decision_rows', 'validation_errors'] }
const ASSEMBLE_PROMPT = (round) => `You are the ledger assembler for the Entrybound research program (round ${round}).

Write (or update if it exists) a deterministic, rerunnable Python 3 tool at ${REPO}/research/tools/ledger/assemble.py (stdlib only) plus ${REPO}/research/tools/ledger/README.md, then run it with ${PY}. The tool must:
1. Load ${MERGED}/*-requirements.jsonl and ${MERGED}/*-decisions.jsonl (files are per shard; several shards may share one cluster, e.g. crypto-a and crypto-b both have cluster "crypto") and every ${ADDENDA}/*.jsonl (addenda rows use the same formats; an addendum row whose cluster+canonical_key/decision_key already exists UPDATES that row by merging additional_sources and replacing non-empty fields; a row with "correction_reason" is a correction).
2. Detect key collisions across shards within a cluster: if two rows share a key, merge them when they have the same meaning (union sources, keep the stricter evidence_state), otherwise disambiguate deterministically; also detect near-duplicate rows across shards of the same cluster (same question in different words) and merge them, recording each merge in ${REPO}/research/audit/assembly-log.md. Deduplicate records claimed by more than one cluster by primary local_key (keep the row in the record's primary_cluster).
3. Assign stable IDs via ${REPO}/research/audit/id-map.json (create if absent): requirement "<cluster>/<canonical_key>" -> "REQ-<CODE>-<NNNN>", decision "<cluster>/<decision_key>" -> "DEC-<CODE>-<NNN>", codes ${CLUSTERS.map(c => c.key + '=' + c.code).join(', ')}. Existing IDs never renumbered or reused; new keys get the next number in sorted key order; merged-away keys map to their survivor's ID (record alias).
4. Write ${REPO}/research/requirement-ledger.csv (RFC 4180, UTF-8) with columns exactly: req_id, cluster, kind, requirement_or_question, source, source_location, additional_sources, superseding_authority, supersession_note, implementation_state, implementation_evidence, evidence_state, research_needed, decision_needed, decision_ids, release_relevance, program_sections ("source@location" joined by " | "; lists joined by ";"), sorted by req_id.
5. Write ${REPO}/research/decision-ledger.jsonl sorted by decision_id with exactly: decision_id, status, blocker_class (NONE|EVIDENCE|PLATFORM|EXTERNAL_REVIEW|HUMAN_PARTICIPANTS|RESOURCES), cluster, question, requirement_ids, program_sections, release_relevance, archetypal_objective, hard_constraints, candidate_set, candidate_exclusion_reasons, required_evidence, experiment_ids [], raw_result_refs [], normalized_result_refs [], heldout_result_refs [], counterevidence "", sensitivity_analysis "", complexity_cost "", dependency_cost "", security_cost "", wire_cost "", selected_decision "", decision_scope "", confidence "", known_limitations "", reopen_trigger "", external_review_requirement, remaining_unknowns_ref "", history [{"date":"2026-09-13","change":"created by Phase A audit"}]. Status comes from initial_status unless an existing ledger row with the same decision_id already has later status/evidence fields — preserve those (never clobber later research).
6. Write JSON Schemas ${REPO}/research/tools/ledger/{decision-ledger,requirement-ledger}.schema.json and validate both ledgers (implementation_state {IMPLEMENTED, PARTIAL, MISSING, BLOCKED_PLATFORM, DEFERRED_APPROVED, REJECTED}; evidence_state {PROVEN, EMPIRICALLY_SUPPORTED, SUPPORTED_WITH_LIMITATIONS, UNTESTED, INSUFFICIENT, CONTRADICTED, EXTERNAL_REVIEW_REQUIRED}; status {DECIDED, INSUFFICIENT_EVIDENCE, EXTERNAL_REVIEW_REQUIRED}). Fix invalid enumerations minimally in the merged/addenda inputs, logging changes in the assembly log, then rerun.
7. Referential integrity: every decision references >=1 requirement; every requirement with non-empty decision_needed references >=1 decision; every research-program section in [8.1..8.8, 9..35, 22.1..22.5] appears in >=1 decision; fix by minimal addendum rows in ${ADDENDA}/assembler-round${round}.jsonl (source PROGRAM) and rerun.
8. Write ${REPO}/research/supersession-ledger.md generated by the tool (per cluster table: req_id, original authority@location, superseding authority@location, nature of change, governing authority).
9. Write ${REPO}/research/source-inventory.md generated by the tool: every source (SPEC ${SPEC}; R1 ${R1}; R2 ${R2}; R3 ${R3}; appendix members at ${APPX} from D:/Projects/entrybound/design/research-appendix/entrybound-architecture-research-appendix.tgz; Research III instrumentation members at ${R3I}; repo docs, crate sources/tests, tools, Cargo manifests at 9e44608) with path, SHA-256, bytes, lines, role, authority rank, extraction slice keys, and count of ledger rows citing it; state that spec and Research I-III are external unpublished inputs referenced by hash and not copied into the repository.
${CHECKPOINT_RULE}
Checkpoint paths: research/tools/ledger research/audit/id-map.json research/audit/assembly-log.md research/audit/addenda (if present) research/requirement-ledger.csv research/decision-ledger.jsonl research/supersession-ledger.md research/source-inventory.md. Return counts, validation errors, commit.`

const a0 = await agent(ASSEMBLE_PROMPT(0), { label: 'assemble:round0', phase: 'Assemble', schema: ASSEMBLE_SCHEMA })
log(a0 ? `assembled: ${a0.requirement_rows} requirements, ${a0.decision_rows} decisions, errors=${a0.validation_errors.length} ${a0.commit || ''}` : 'assemble FAILED')

phase('Critique')
const CRITIC_SCHEMA = { type: 'object', properties: { additions_file: { type: 'string' }, additions_count: { type: 'integer' }, updates_count: { type: 'integer' }, notes: { type: 'array', items: { type: 'string' } }, commit: { type: 'string' } }, required: ['additions_count', 'updates_count'] }
const FAMILIES = [
  { key: 'spec', what: `the Product Architecture Specification ${SPEC} (all 2754 lines; especially §4.9, §5.7a, §7.3-§7.6, §9.6-§9.11, §10, §11.5, §13, §14.3-§14.4, §15.4, §16, §17.2, §18, §19.2-§19.5, §20, §21, §22.4-§22.5, §23.4, §24, §25, §26)` },
  { key: 'research', what: `Research I ${R1}, Research II ${R2}, Research III ${R3} (open-question, unresolved, falsification, NO-GO, decisions-not-to-make-yet sections)` },
  { key: 'appendix', what: `the research appendix files ${APPX}/chunking.md, crypto.md, metadata.md, incumbents.md and the Research III instrumentation at ${R3I}` },
  { key: 'docs', what: `every document in ${REPO}/docs (40 files), especially non-goals, limitations, deferred, future, residual risk, open, not implemented sections` },
  { key: 'code', what: `the code at ${REPO}/crates (refusals, unsupported/NotSupported reason codes, todo!/unimplemented!, FidelityReport capability-unavailable paths, hard-coded limits, frozen IDs) and ${REPO}/tools` },
  { key: 'program', what: `the research program's explicit questions by cluster:\n${Object.entries(PROGRAM).map(([k, v]) => `[${k}]\n${v}`).join('\n')}\nplus obligations already merged under model/access/integrity (determinism under parallelism; Task 12 remote access; Task 13 streaming/scalability; Task 14 deterministic parallelism; Task 23 sidecars/incremental; Task 24 recovery/parity) and program obligations: incumbent baselines (task 6), whole-system measurement framework (task 7), corpus completeness (task 5), archetypal objective & decision method (task 3), reproducibility (37), evidence classification (38), negative results (39), stable-v1 dispositions (34), final held-out evaluation (35)` },
]
let round = 1
while (round <= 2) {
  const results = await parallel(FAMILIES.map(f => () => agent(
`You are a completeness critic (round ${round}) for the Entrybound research program's Requirement and Decision Ledgers. Automatic failure condition: a research question or deferred/unresolved architecture question disappears instead of receiving a ledger record.

Ledgers: ${REPO}/research/requirement-ledger.csv and ${REPO}/research/decision-ledger.jsonl (use scripts and grep; do not load them whole into context). Source family: ${f.what}.

Read the source family directly. Find every requirement, invariant, deferred matter, open question, evidence-dependent claim, unimplemented capability, or research obligation NOT represented by any ledger row (represented = covered in meaning), and every row whose implementation_state/evidence_state/supersession is demonstrably wrong. Be adversarial and specific; no near-duplicates.

For each gap write an addendum row to ${ADDENDA}/round${round}-${f.key}.jsonl using:
${MERGE_FORMAT}
For a correction, emit a row with the SAME cluster and canonical_key/decision_key (look up in ${REPO}/research/audit/id-map.json) with only corrected fields plus keys and "correction_reason". If no gaps and no corrections, create no file and return 0/0. Validate JSON lines.
${CHECKPOINT_RULE}
Checkpoint path (only if you created it): research/audit/addenda/round${round}-${f.key}.jsonl. Return counts, notes, commit.`,
    { label: `critic:${f.key}:r${round}`, phase: 'Critique', schema: CRITIC_SCHEMA })))
  const adds = results.filter(Boolean).reduce((a, r) => a + r.additions_count + r.updates_count, 0)
  log(`critique round ${round}: ${adds} additions/updates (${results.map((r, i) => `${FAMILIES[i].key}=${r ? r.additions_count + '+' + r.updates_count : 'FAIL'}`).join(', ')})`)
  if (adds === 0) break
  const ar = await agent(ASSEMBLE_PROMPT(round), { label: `assemble:round${round}`, phase: 'Assemble', schema: ASSEMBLE_SCHEMA })
  log(ar ? `reassembled r${round}: ${ar.requirement_rows} requirements, ${ar.decision_rows} decisions, errors=${ar.validation_errors.length}` : 'reassemble FAILED')
  round++
}
if (round > 2) log('critique stopped after 2 rounds without a dry round — residual gaps must be re-checked in a later completeness pass (recorded for PROGRESS.md)')

phase('Method')
const METHOD_SCHEMA = { type: 'object', properties: { files: { type: 'array', items: { type: 'string' } }, summary: { type: 'string' }, commit: { type: 'string' } }, required: ['files', 'summary'] }
const objective = agent(
`Write ${REPO}/research/archetypal-objective.md for the Entrybound research program. Read ${REPO}/research/requirement-ledger.csv (INVARIANT and REQUIREMENT rows, via grep/scripts), ${SPEC} §1-§3.9, §6.5, §8, §11, §12, §23, Appendix A, and ${REPO}/CONTRIBUTING.md.
Required content:
1. Governing question: what design most completely realizes the archetypal ideal of Entrybound as the definitive next-generation archive system, subject to semantic, security, fidelity, determinism, interoperability, bounded-resource, longevity, and usability invariants.
2. Hard constraints (HC-01..): at minimum one authority per semantic fact; exact logical losslessness; deterministic native interpretation; fail-closed unknown critical semantics; caller-owned extraction/security policy; declared resource bounds; no silent metadata/semantic loss; archive semantics independent of host filesystem interpretation; bounded dependency chains; canonical identities; no runtime-dependent decoder interpretation; independently implementable native specification; reproducible deterministic modes; crypto correctness never traded for performance. Map each HC to spec invariants I1-I31 and repo freezes, with a mechanical violation test. Every I1-I31 maps to >=1 HC.
3. Optimization dimensions (OD-01..) covering at least: semantic coherence, exact losslessness, preservation fidelity, deterministic interpretation, compression/storage efficiency, creation performance, decoding performance, memory/scratch, streaming, random access, verified partial retrieval, remote access, parallelism, corruption locality, confidentiality/authenticity, metadata privacy, resource-bounded decoding, cross-platform fidelity, legacy interoperability, deterministic migration, independent implementability, dependency longevity, specification complexity, implementation attack surface, user complexity, adoption friction. For EACH: precise metric(s), unit, measurement procedure/instrument (name the research tooling: ebr runner, research/harness crates, corpus manifest), direction, aggregation across workloads, and whether binary constraint or graded objective.
4. HC vs OD relationship (HC violation invalidates regardless of score).
Tag each material claim: FORMALLY_DERIVED, EMPIRICALLY_MEASURED, EXTERNALLY_SOURCED, INFERRED, EXPERT_REVIEW_REQUIRED, UNRESOLVED.
${CHECKPOINT_RULE}
Checkpoint path: research/archetypal-objective.md (subject should say it is the pre-registration draft). Return files, summary, commit.`,
  { label: 'method:objective', phase: 'Method', schema: METHOD_SCHEMA })
const method = agent(
`Write ${REPO}/research/decision-method.md for the Entrybound research program. PRE-REGISTERED: committed before any experiment result is observed, so every threshold must be fixed now. Read ${REPO}/research/decision-ledger.jsonl (skim candidate sets via scripts), ${REPO}/research/requirement-ledger.csv, ${SPEC} §5.4, §6, §22.1, and the runner's threshold template ${REPO}/research/methods/thresholds.json and statistics code ${REPO}/research/tools/ebr/stats.py and thresholds.py (so the document's parameters map onto machine-readable keys).
Required content:
1. Decision rule in order: (1) eliminate invariant violations; (2) eliminate designs that cannot be bounded or independently specified; (3) measure survivors; (4) eliminate Pareto-dominated candidates (epsilon-dominance using practical-significance thresholds); (5) validate survivors on held-out workloads; (6) account for permanent implementation/specification/dependency cost; (7) sensitivity analysis; (8) universal default only when evidence supports one; (9) otherwise profile/policy/workload-specific winners; (10) prefer the simpler design when differences are statistically or practically negligible.
2. Pre-registered practical-significance thresholds with units for every metric family: total artifact bytes, metadata/index/dictionary/side-data bytes, encode/decode wall time, CPU time, peak RSS, peak scratch, first-entry and random-entry latency, HTTP bytes and request count, verification overhead, parallel speedup, corruption blast radius, leakage measures, padding overhead, usability task success. Relative and absolute floors; brief justification each.
3. Statistical protocol: repetitions (minimum and adaptive rule), warmups, CPU affinity/power/thermal controls for a laptop i9-14900HX (P-cores = Windows logical 0-15, E-cores 16-31) under Windows 11 with Defender real-time protection and VBS, and WSL2 Ubuntu 24.04 (32 vCPUs without P/E mapping; host load appears as steal); quiet-machine guard limits; median/p90/p95/IQR/MAD with raw samples; bootstrap CIs (paired where possible); noise rule (CI overlapping threshold band => "not distinguishable", no winner); multiple-comparison handling; deterministic byte-count metrics via single run with repeat-hash determinism check.
4. Corpus split discipline: tuning vs validation vs held-out; held-out frozen by manifest hash (logical_tree_sha256, since tree_sha256 depends on unstable ext4 st_blocks) and accessible only after a recorded design-freeze commit in research/decisions/design-freeze.json (key design_freeze_sha); leakage invalidates affected decisions.
5. Sensitivity analysis: weight perturbations (±25%, ±50%, Dirichlet samples), threshold perturbations, workload-mix reweighting; stability metric (fraction preserving winner, Kendall tau) and the pre-registered stability bar to freeze a default.
6. Permanent cost accounting: spec complexity (normative words/sections/record types/wire fields), implementation complexity (LoC, cyclomatic proxies), dependency cost (transitive crates, unsafe count, C/C++ code, license, maintenance), attack surface (parser states, attacker-controlled allocations), independent implementability; minimum gains required to justify a new codec, transform, record type, or profile.
7. Status rules: DECIDED only when candidates enumerated, exclusions justified, evidence linked, held-out validation for empirical decisions, counterevidence discussed, sensitivity measured, practical significance considered, costs included, rule followed; else INSUFFICIENT_EVIDENCE; EXTERNAL_REVIEW_REQUIRED for independent expert review. Confidence scale with criteria.
8. Evidence classification tags and primary-source citation rule. 9. Negative-results obligation and confirmation-bias trigger. 10. Experiment record template (ID, question, hypothesis, candidates, corpus IDs, environment fingerprint, commands/code, seeds, warmup, measurement method, replication count, raw results, normalization, statistical summary, plots/tables, interpretation, threats, decisions informed) and the rule that every reported number is generated from raw artifacts by a checked-in command.
${CHECKPOINT_RULE}
Checkpoint path: research/decision-method.md (subject should say pre-registration draft). Return files, summary, commit.`,
  { label: 'method:decision-method', phase: 'Method', schema: METHOD_SCHEMA })
const drafts = await Promise.all([objective, method])
log(`method drafts: ${drafts.filter(Boolean).length}/2`)

const review = await agent(
`Adversarially review the pre-registration documents ${REPO}/research/archetypal-objective.md and ${REPO}/research/decision-method.md (consult ${REPO}/research/decision-ledger.jsonl via scripts and ${SPEC} Appendix A). Look for: vague/unmeasurable metrics; missing dimensions or hard constraints; unmapped invariants I1-I31; unjustified, gameable, inconsistent, too-loose or too-tight thresholds; statistical gaps (noise, laptop thermal/hybrid-core confounds, WSL2 virtualization, Defender, paired designs, multiple comparisons); held-out leakage paths; sensitivity analysis that cannot detect instability; cost accounting ignoring permanent spec/dependency cost; status rules allowing premature DECIDED; conflicts with spec or repo freezes. Write concrete findings with severity and required fixes to ${REPO}/research/methods/method-review-round1.md.
${CHECKPOINT_RULE}
Checkpoint path: research/methods/method-review-round1.md. Return files, summary (top findings), commit.`,
  { label: 'method:adversarial-review', phase: 'Method', schema: METHOD_SCHEMA })

const revise = await agent(
`Revise ${REPO}/research/archetypal-objective.md and ${REPO}/research/decision-method.md to resolve every finding in ${REPO}/research/methods/method-review-round1.md; for rejected findings append a justified "Review disposition" section to the review file. Add "Revision history" sections. Keep thresholds concrete. Then fill ${REPO}/research/methods/thresholds.json with the exact machine-readable values from the revised decision-method.md (every key the ebr runner reads: practical-significance bands per metric family, CI level, bootstrap resamples, Pareto epsilons, quiet-machine guard limits, sensitivity perturbation grid and stability bar), keeping a "source" pointer to the decision-method.md section for each value; run the ebr test suite in WSL (bash /mnt/d/Projects/entrybound/entrybound/research/tools/run_tests.sh) to confirm the runner accepts the file.
${CHECKPOINT_RULE}
Checkpoint paths: research/archetypal-objective.md research/decision-method.md research/methods/method-review-round1.md research/methods/thresholds.json, subject "research: pre-register decision method and archetypal objective". This commit is the pre-registration record: state in the body that no decision-relevant experiment result exists at this commit. Return files, summary, commit.`,
  { label: 'method:revise', phase: 'Method', schema: METHOD_SCHEMA })

return { merged: merged.map((m, i) => ({ shard: SHARDS[i].shard, ok: !!m, reqs: m && m.requirement_count, decisions: m && m.decision_count, gaps: m && m.gaps, commit: m && m.commit })), assemble0: a0, critiqueRounds: round, review: review && review.summary, revise: revise && { summary: revise.summary, commit: revise.commit } }
