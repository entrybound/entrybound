# Normalized CDC and exact deduplication

Status: frozen creation-time behavior introduced by planner v2 and inherited
unchanged by planner v3 and v4. Decoders do not implement
or invoke the chunker; they consume the ordered Chunk references recorded in
each ContentObject.

## Algorithm

New archives use `gear-norm-v1`, a normalized Gear-hash content-defined
chunker. It was selected because it provides deterministic integer-only linear
scanning, bounded state, practical insertion/deletion re-synchronization, and a
simple normalized distribution. Unlike fixed blocks, a small insertion does
not shift every later boundary. Unlike boundary-skipping variants, every input
byte updates the hash before any boundary is accepted.

The 256-entry `u64` Gear table is frozen by this construction:

```text
seed = 0x6a09e667f3bcc909
table[i] = splitmix64(seed + i), for i in 0..256

splitmix64(x):
  x = x + 0x9e3779b97f4a7c15
  x = (x xor (x >> 30)) * 0xbf58476d1ce4e5b9
  x = (x xor (x >> 27)) * 0x94d049bb133111eb
  return x xor (x >> 31)
```

All operations wrap modulo 2^64. Starting at each Chunk boundary, every byte
updates `hash = (hash << 1) + table[byte]`. No boundary is allowed before the
minimum. For a power-of-two target with `b = log2(target)`:

- from minimum through `target - 1`, cut when `hash & ((1 << (b + 1)) - 1) == 0`;
- from target through `maximum - 1`, cut when
  `hash & ((1 << (b - 1)) - 1) == 0`;
- force a cut at maximum.

The stronger early mask suppresses undersized chunks; the weaker late mask
encourages a boundary after the target, normalizing size variance. A final
trailing region is emitted even when smaller than the minimum. A non-empty file
is covered exactly once with no gaps or overlaps. A zero-length ContentObject
has no Chunks.

The `chunker_id` records the algorithm and selected parameters:

```text
gear-norm-v1/min-{bytes}/target-{bytes}/max-{bytes}
```

## Planner v2 policies

The frozen v2 planner IDs use these CDC policies. V3 and current v4 policies
reuse the same ordered candidates without altering any `gear-norm-v1`
parameter set:

| Planner | Candidate policies (minimum / target / maximum) | Selection |
|---|---|---|
| `fast-v2` | 512 KiB / 2 MiB / 8 MiB | Single coarse policy; minimizes frame and Index churn |
| `balanced-v2` | 128 KiB / 512 KiB / 2 MiB | Single general-purpose policy |
| `dense-v2` | balanced; 64 KiB / 256 KiB / 1 MiB | Lowest complete estimated physical cost |
| `extreme-v2` | fast; balanced; dense; 32 KiB / 128 KiB / 512 KiB | Lowest cost across the bounded full candidate set |

Candidate order wins ties, preferring the coarser policy. The estimate is:

```text
unique plaintext Chunk bytes
+ 164 bytes per unique Chunk (64-byte frame + 100-byte Index record)
+ 40 bytes per canonical manifest Chunk reference
```

Identical ContentObjects are counted once in manifest-reference cost. This rule
prevents smaller chunks from winning merely because they create more dedup
hits. Codec selection then runs once on the selected unique Chunk set and uses
the unchanged v1 STORE/Zstandard candidate levels and minimum-gain rule.

## Exact deduplication and identity

After chunking, every Chunk receives SHA-256 over exact plaintext bytes. The
ContentStore is a deterministic digest-keyed map, so repeated positions,
files, and ContentObjects reference one stored Chunk. A digest collision with
different captured bytes is a typed failure; similarity is never proof of
equality. Chunk-reference order remains authoritative for reconstruction.

ContentObject `logical_digest`, Entry identity, and LAI remain over logical
plaintext and are independent of boundaries. AUX remains metadata/fidelity
identity. PCR binds ContentObject chunk roots, unique Chunk count, and
`chunker_id`, so it changes when chunking changes. PCI changes with exact ECF
bytes.

Resource declarations keep total logical file bytes separate from unique
physical Chunk bytes. Duplicate references therefore cannot evade caller
logical-output limits. `inspect` and `explain` report both logical references
and unique physical storage.

## Compatibility and limitations

`ecf/bootstrap-v1` already carries ordered Chunk references and arbitrary
logical lengths, so no ECF wire change was required. Historical
`fixed-1mib/v1` and `*-v1` archives continue to open and verify without
reinterpretation.

Planner v3 adds similarity clustering, optional physical reordering,
Dictionaries, and bounded ChunkGroups without changing this chunker. See
[cross-file-compression-v1.md](cross-file-compression-v1.md). STREAM layout,
cross-archive deduplication, and keyed/encrypted boundaries remain unsupported.
