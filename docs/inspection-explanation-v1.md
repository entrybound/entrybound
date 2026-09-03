# Structured inspection and explanation v1

Status: implemented without changing ECF.

## Inspection JSON

`inspect --json` emits the versioned schema `entrybound/inspection-v1` with a
fixed field order. It groups format/layout/features, identity values and
verification state, resource/decode declarations, entries, plans, Chunk
statistics, reconstruction, provenance, security, and access information.
Focused switches (`--entries`, `--plans`, `--chunks`, `--reconstruction`,
`--provenance`, `--security`, and `--access`) select surfaces without changing
their interpretation.

A fully opened archive says `whole archive verified`. A range-backed URL says
`range-backed metadata; whole_archive_verified=false`, gives the status of each
identity, and reports bytes/range requests. Locked encrypted JSON contains only
the already-public crypto framing. Secret key, password, AFK, and private
recipient data have no JSON field.

## Explanation evidence

Every structured explanation fact has one evidence class:

- `RECORDED`: serialized archive facts, including plan IDs, codecs,
  dictionaries, group bounds, and region access declarations.
- `DERIVED`: arithmetic or dependency closure computed from recorded facts.
- `AUDIT`: an explicitly persisted reconstruction/fallback audit.
- `NOT_RECORDED`: creation-time history that cannot be recovered honestly.

Entry-level explanation follows its ContentObject references and reports Chunk
count/size, selected plan and codec, dedup sharing, dictionary dependencies,
bounded same-group predecessors, intersecting reconstruction regions, declared
worst reconstruction bytes, and a locally derived range-count estimate.
Candidate rankings that were never stored are explicitly `NOT_RECORDED`; the
current planner is never rerun and presented as historical intent.

`repack --profile ... --dry-run` is prospective rather than archival evidence.
It compares source and target planner, Chunk counts, unique Chunks, stored
bytes, decode working set, dictionary/group/region counts, PCR, and prospective
container size while predicting and then verifying LAI/AUX equality.

