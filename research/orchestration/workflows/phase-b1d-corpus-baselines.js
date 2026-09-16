export const meta = {
  name: 'eb-research-phase-b1d-corpus-baselines',
  description: 'Entrybound research Phase B1 continuation: finish/verify interrupted corpus groups, assemble corpus with stable fingerprints, completeness critic, incumbent baselines + capability matrix — with checkpoint commits',
  phases: [
    { title: 'Corpus', detail: 'resume g2, verify/finish g4 and g5' },
    { title: 'Assemble', detail: 'manifest, statistics, held-out lock (logical hashes)' },
    { title: 'Critique', detail: 'one completeness round + fixes' },
    { title: 'Baselines', detail: 'incumbent tooling, capability matrix, size/determinism pass' },
  ],
}

const REPO = 'D:/Projects/entrybound/entrybound'
const WREPO = '/mnt/d/Projects/entrybound/entrybound'
const DATA = '/root/eb-research'
const PY = 'C:/Python313/python.exe'
const CKPT = `${PY} ${REPO}/research/orchestration/checkpoint.py`

const RULES = `Shared rules for Entrybound research-infrastructure agents (resumed after a usage-limit interruption):
- Read ${REPO}/research/PROGRESS.md first (environment, pinned toolchain 1.98.1 invoked by explicit path, open integration items, interruption record).
- Repository: ${REPO} (Windows) = ${WREPO} (WSL). Large data OUTSIDE git in WSL ext4: ${DATA} (corpus ${DATA}/corpus/<split>/<item_id>, held-out ${DATA}/heldout/<item_id>, cache ${DATA}/cache, venv ${DATA}/venv). Only scripts, source definitions, pins, manifests, hashes, small raw results go in ${REPO}/research.
- Run Linux work via scripts: wsl.exe -d Ubuntu -- bash ${WREPO}/research/<path>.sh. Windows host is non-admin. Docker Desktop available (same WSL2 kernel); ARM64 only via QEMU emulation. No GitHub Actions.
- WSL has GNU tar 1.35, bsdtar 3.7.2, 7-Zip 23.01 as 7z/7za/7zr (no p7zip 16.02, no 7zz), zstd 1.5.5, xz 5.4.5, gzip 1.12, pigz, lz4, lzip, brotli, pixz, par2, hyperfine, cpio, squashfs-tools, xfsprogs, btrfs-progs, attr, acl, sqlite3, jq. apt/pip installs and downloads from reputable primary hosts are approved; record URL, date, size, SHA-256.
- NO decision-grade timing now (machine busy). Byte counts, determinism checks, capability probes, smoke tests are fine.
- Held-out: never read content or compute content statistics for ${DATA}/heldout; fingerprints only.
- Privacy: public or generated data only.
- The corpus framework is in ${REPO}/research/corpus/tools (run via research/corpus/tools/run.sh); provisioning is idempotent and hash-verified.
- Never run state-changing git commands directly. When your work is COMPLETE and VERIFIED, commit exactly your own paths with the serialized checkpoint tool (appends PROGRESS.md change-log line and pushes):
  ${CKPT} --subject "<imperative subject <=72 chars>" --body "<what and why>" --note "<one-line progress note>" -- <your paths...>
  Retry on lock timeout; report git errors instead of attempting manual git. Checkpoint intermediate progress too when a meaningful sub-part is verified (e.g. each family complete), so a future interruption loses little.`

const RESULT = { type: 'object', properties: { files: { type: 'array', items: { type: 'string' } }, commits: { type: 'array', items: { type: 'string' } }, summary: { type: 'string' }, problems: { type: 'array', items: { type: 'string' } } }, required: ['files', 'summary'] }

const COVERAGE = `Coverage requirements for EACH of your families: >=2 independent samples (distinct independence groups) in tuning, >=1 (prefer 2) in validation, >=2 in held-out, no independence group shared across splits within a family; small (<=16 MiB)/medium (<=512 MiB)/large (<=8 GiB) scales where meaningful; group total under ~12 GiB. Prefer real-world data; label generated data; record license per item and mark non-redistributable items. Report per-family coverage (split x scale counts, groups) in your summary and list inadequately covered families in problems.`

phase('Corpus')
const groups = await parallel([
  () => agent(`${RULES}

Task: RESUME corpus group g2-generated: F04 many-small-file trees, F15 sparse files, F17 duplicate trees, F19 metadata-heavy filesystem trees, F20 adversarial/high-entropy inputs. A previous agent was interrupted after writing generator scripts in ${REPO}/research/corpus/generators/g2-generated/ (committed as WIP in 6ef6659) but before writing ${REPO}/research/corpus/sources/g2-generated.json or materializing items. Also inspect the stray probes ${REPO}/research/corpus/generators/.g2probe.py and .g2probe.sh and ${DATA}/generated (an extra directory another agent created) — reuse or delete them and record which.
Review the existing generators for correctness and determinism (fixed seeds, streaming for multi-GB outputs rather than in-memory), complete them, write the source definitions, provision and fingerprint every item.
Hints: F04 real /usr/include and /usr/share/doc trees exported from pinned Docker images plus generated small-file trees with realistic size distributions. F15 generated sparse files with varied hole layouts (VM-like preallocation, database preallocation, huge-hole single extent, many small extents) verified with SEEK_HOLE. F17 exact duplicate trees (2x/8x copies, duplicated vendor dirs). F19 deep nesting, many empty files/dirs, symlinks, hardlinks, user.* xattrs, POSIX ACLs, varied modes/uids, long and Unicode names, plus a real /etc from a pinned Docker image. F20 seeded CSPRNG data, AES-CTR ciphertext, CDC-adversarial patterns that force min-/max-size chunks for gear-norm-v1 (see ${REPO}/docs/chunking-v1.md and crates/entrybound/src/chunker.rs), compression-bomb-like inputs, maximal-entropy files with sparse exact duplication.
${COVERAGE}
Checkpoint per family as each becomes complete (paths: research/corpus/sources/g2-generated.json, research/corpus/generators/g2-generated, and research/corpus/pins|fingerprints files for the family prefixes f04-, f15-, f17-, f19-, f20-). Return files, commits, summary, problems.`, { label: 'corpus:g2-resume', phase: 'Corpus', schema: RESULT, model: 'sonnet' }),
  () => agent(`${RULES}

Task: VERIFY and FINISH corpus group g4-binary: F09 executables, F10 highly redundant binaries, F11 already-compressed files, F14 archive-inside-archive, F16 VM/disk-like images. A previous agent was interrupted after writing ${REPO}/research/corpus/sources/g4-binary.json (42 items), generators in research/corpus/generators/g4-binary, and pins/fingerprints for all 42 items (committed as WIP in 6ef6659), but before its final verification and coverage report.
Do: run provision --check and a verify pass for g4 items (use logical_tree_sha256 for identity; tree_sha256 differences caused only by ext4 st_blocks are a known framework defect being fixed by the assembler, not an item failure); re-materialize any item whose data is missing or incomplete under ${DATA}; confirm idempotence; check license records; evaluate coverage against the requirements and close gaps within budget.
${COVERAGE}
Checkpoint the verified group (subject noting it supersedes the WIP checkpoint; paths: research/corpus/sources/g4-binary.json, research/corpus/generators/g4-binary, pins/fingerprints files with prefixes f09-, f10-, f11-, f14-, f16-). Return files, commits, summary, problems.`, { label: 'corpus:g4-verify', phase: 'Corpus', schema: RESULT, model: 'sonnet' }),
  () => agent(`${RULES}

Task: VERIFY and FINISH corpus group g5-media: F12 JPEG/images, F13 other media. A previous agent was interrupted after writing ${REPO}/research/corpus/sources/g5-media.json (26 items), generators in research/corpus/generators/g5-media, and pins/fingerprints for all 26 items (WIP commit 6ef6659), before final verification and coverage reporting.
Do: provision --check and verify g5 items (logical_tree_sha256 identity); re-materialize missing/incomplete data; confirm the JPEG sets span encoders and modes that matter for JPEG reconstruction eligibility (libjpeg-turbo baseline, mozjpeg, progressive, arithmetic-coded if available, 4:2:0/4:4:4, restart markers, EXIF-heavy, CMYK/grayscale, truncated/corrupt, trailing data) and close gaps; check licenses (NASA/Wikimedia/Blender/LibriVox etc.).
${COVERAGE}
Checkpoint the verified group (subject noting it supersedes the WIP checkpoint; paths: research/corpus/sources/g5-media.json, research/corpus/generators/g5-media, pins/fingerprints files with prefixes f12-, f13-). Return files, commits, summary, problems.`, { label: 'corpus:g5-verify', phase: 'Corpus', schema: RESULT, model: 'sonnet' }),
])
groups.forEach((g, i) => log(`${['g2', 'g4', 'g5'][i]}: ${g ? g.summary.slice(0, 200) : 'FAILED'}`))

phase('Assemble')
const assemble = (round) => agent(`${RULES}

Task: corpus assembly (round ${round}).
1. Fix the known fingerprint defect first: tree_sha256 includes st_blocks, which is unstable on ext4 under delayed allocation. Make corpus identity, held-out locking, and verification use logical_tree_sha256 (content, paths, types, modes, link targets, hardlink topology, xattrs, logical sparse layout via SEEK_HOLE/SEEK_DATA extents — but not st_blocks), keeping allocation data as a non-identity observation. Update ${REPO}/research/corpus/tools accordingly with a regression test, and make sure the ebr runner's corpus/held-out guard (${REPO}/research/tools/ebr/corpus.py) reads research/corpus/manifest.json and delegates held-out refusal to corpuslib.assert_not_heldout() rather than a separate format (integration item from PROGRESS.md); run the ebr tests.
2. Run assemble.py and stats.py (tuning+validation only) over all source definitions (g1-code, g2-generated, g3-data, g4-binary, g5-media) and fingerprints. Produce ${REPO}/research/corpus/manifest.json, statistics.json, heldout-lock.json (logical hashes), coverage.md (family x split x scale, independence groups, real vs generated), licenses.md, and finalize methodology.md (no hand-entered numbers; reference generated files). Spot-check 10% of fingerprints by recomputation.
Checkpoint: research/corpus (manifest.json, statistics.json, heldout-lock.json, coverage.md, licenses.md, methodology.md, tools) and research/tools/ebr + tests if changed. Return files, commits, summary with totals, problems.`, { label: `corpus:assemble:r${round}`, phase: 'Assemble', schema: RESULT, model: 'sonnet' })
const as0 = await assemble(0)
log(`assemble r0: ${as0 ? as0.summary.slice(0, 300) : 'FAILED'}`)

phase('Critique')
const CRIT = { type: 'object', properties: { gaps: { type: 'array', items: { type: 'object', properties: { family: { type: 'string' }, problem: { type: 'string' }, fix: { type: 'string' } }, required: ['family', 'problem', 'fix'] } }, summary: { type: 'string' }, commit: { type: 'string' } }, required: ['gaps', 'summary'] }
const crit = await agent(`${RULES}

Task: adversarial corpus completeness critic. Read ${REPO}/research/corpus/manifest.json, coverage.md, statistics.json, licenses.md, sources/*.json and each group's reported problems in the git log (git log --format=%B -- research/corpus). The corpus is INCOMPLETE if a major Entrybound use case has no representative workload. Use cases: source releases, package publishing, CI artifacts, ML model/data distribution (large safetensors/numpy weights), scientific archives, supply-chain verification, backups of home/project directories, legacy ZIP/tar/7z migration, container/VM images, media libraries, logs/telemetry archival, database dumps, cross-platform metadata-rich trees, encrypted private archives, adversarial inputs. Check every family F01-F20: >=2 independent tuning samples, >=1 validation, >=2 held-out, no independence group across splits, scale coverage (note known gaps: large-tier validation items largely absent; F08 validation lacks real-content DBs), real vs generated balance, license recording, no single workload standing in for a category. Write the critique to ${REPO}/research/corpus/critique-round1.md and checkpoint it. Return concrete gaps with specific fixes (sources), summary, commit.`, { label: 'corpus:critic', phase: 'Critique', schema: CRIT })
if (crit && crit.gaps.length) {
  log(`corpus critic: ${crit.gaps.length} gaps`)
  const FAM_GROUP = { F01: 'g1-code', F02: 'g1-code', F03: 'g1-code', F18: 'g1-code', F04: 'g2-generated', F15: 'g2-generated', F17: 'g2-generated', F19: 'g2-generated', F20: 'g2-generated', F05: 'g3-data', F06: 'g3-data', F07: 'g3-data', F08: 'g3-data', F09: 'g4-binary', F10: 'g4-binary', F11: 'g4-binary', F14: 'g4-binary', F16: 'g4-binary', F12: 'g5-media', F13: 'g5-media' }
  const byGroup = {}
  for (const gap of crit.gaps) { const g = FAM_GROUP[gap.family.slice(0, 3).toUpperCase()] || 'g2-generated'; (byGroup[g] = byGroup[g] || []).push(gap) }
  await parallel(Object.entries(byGroup).map(([gk, gaps]) => () => agent(`${RULES}\n\nTask: close corpus gaps for group ${gk}. Gaps:\n${gaps.map(x => `- ${x.family}: ${x.problem} -> ${x.fix}`).join('\n')}\nAppend items to (never remove existing items from) ${REPO}/research/corpus/sources/${gk}.json and its generators; provision and fingerprint; ${COVERAGE}\nCheckpoint your added/changed paths. Return files, commits, summary, problems.`, { label: `corpus:fix:${gk}`, phase: 'Critique', schema: RESULT, model: 'sonnet' })))
  await assemble(1)
}

phase('Baselines')
const baselines = await agent(`${RULES}

Task: incumbent baseline tooling, capability matrix, and a non-timing size/determinism pass.
1. ${REPO}/research/baselines/configs.json: pinned incumbent configurations, at minimum ZIP/DEFLATE (Info-ZIP zip -6/-9 via apt 'zip'; 7z -tzip -mx=9), tar+gzip (-6, -9; pigz single vs multi-thread), tar+zstd (-3, -19, --ultra -22 --long=27; -T1 vs -T0), tar+xz (-6, -9e; pixz indexed xz), 7z/LZMA2 (7z -mx=5 and -mx=9, solid and -ms=off), tar+lz4, tar+brotli -q11, plus materially relevant containers from ${REPO}/research/requirement-ledger.csv (if present) and the incumbent list: squashfs (mksquashfs zstd/xz), WIM (wimtools LZX/LZMS solid), borgbackup and restic (dedup repositories), DwarFS (official static release binary if obtainable), zpaq (upper bound if packaged), zstd --patch-from for versioned trees. Record exact versions in ${REPO}/research/baselines/tool-versions.json (generated by a script).
2. ${REPO}/research/baselines/run_baselines.py using the ebr runner (${REPO}/research/tools/ebr): for each config and corpus item from research/corpus/manifest.json (tuning+validation only), create the artifact, verify full round-trip (logical tree fingerprint equality where representable; otherwise record exactly what differed), record complete artifact bytes, determinism (two runs byte-identical?), and single-member extraction support (command used). Timing/memory fields only under --timing with the quiet-machine guard; in THIS phase run timing-disabled over all tuning+validation items at small/medium scale and a subset of large items, raw rows to ${REPO}/research/raw/EXP-BASE-SIZE/ and normalized CSV to ${REPO}/research/normalized/EXP-BASE-SIZE/ (checkpoint partial raw results periodically).
3. Capability probes ${REPO}/research/baselines/probes/: fixtures with non-UTF-8 and Unicode paths, mtime/atime/ctime/birth precision, permissions, ownership, symlinks, hardlinks, POSIX ACLs, xattrs, sparse files; check what each incumbent config preserves after create+extract in WSL as root on ext4; plus encryption, authentication, random access, streaming creation/extraction, deterministic output, partial retrieval, verification by probe where possible else by primary documentation citation. Output ${REPO}/research/baselines/capability-matrix.csv (system_config, capability, support {YES,NO,PARTIAL,N/A}, method {PROBE,DOC}, evidence_ref, notes), generated from probe results, including Entrybound (ebound built from a clone ${DATA}/src/entrybound at the current dev HEAD with /root/.cargo/bin/cargo +1.98.1 and CARGO_TARGET_DIR=${DATA}/target/baseline) probed the same way.
4. ${REPO}/research/baselines/README.md: configs, capability-matching rules for fair comparison (a baseline lacking a capability is stated, not used to exclude Entrybound's cost), reproduction commands.
Checkpoint research/baselines and the EXP-BASE-SIZE raw/normalized outputs. Return files, commits, summary, problems.`, { label: 'baselines:tooling', phase: 'Baselines', schema: RESULT, model: 'sonnet' })

return { groups: groups.map((g, i) => ({ group: ['g2', 'g4', 'g5'][i], summary: g && g.summary, problems: g && g.problems })), assemble: as0 && as0.summary, critic: crit && { gaps: crit.gaps.length, summary: crit.summary }, baselines: baselines && { summary: baselines.summary, problems: baselines.problems } }
