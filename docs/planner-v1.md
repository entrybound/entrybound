# Compression planner v1

Status: frozen codec-selection behavior retained for historical fixed-chunk
archives and inherited unchanged by the CDC-aware v2 and cross-file-aware v3
policies for their independent baseline candidates. V4 preserves those IDs and
adds a separately frozen multi-codec baseline.

The planner accepts validated plaintext Chunks and one public profile. It emits
explicit TransformPlans and per-Chunk plan references before ECF serialization.
It is never used to open, verify, list, inspect, explain, or unpack an archive.
Those operations dispatch solely from the recorded plans.

The original public names mapped to these frozen planner IDs:

| Public profile | Planner ID | Measured Zstandard levels |
|---|---|---|
| `fast` | `fast-v1` | level 1 only when the deterministic probe predicts useful compression |
| `balanced` | `balanced-v1` | levels 1, 3, and 5 when the probe predicts compression; otherwise level 1 |
| `dense` | `dense-v1` | levels 5, 9, and 15 |
| `extreme` | `extreme-v1` | levels 9, 15, 19, and 22 |

Every candidate is compressed on the real Chunk. Search is bounded by the
table; it does not use elapsed time, core count, available memory, filesystem
order, floating point, filenames, or extensions. The probe samples at most
4,096 deterministically spaced bytes and uses integer symbol diversity,
maximum-frequency, and adjacent-repeat measures. Inputs shorter than 64 bytes
always use STORE.

Zstandard wins only when this strict inequality holds:

```text
encoded bytes + 16 bytes attributable plan cost
              + max(32 bytes, ceil(1% of plaintext bytes))
  < plaintext bytes
```

The smallest qualifying candidate wins and STORE wins all ties. The fixed
16-byte charge prevents plan choice from considering payload size in isolation;
the absolute and relative thresholds reject negligible savings. This is a
planner rule, not a promise that every surrounding container byte is assigned
to an individual Chunk.

Zstandard plans use codec identifier `zstandard/v1`, a 1 MiB window, content
size in the frame, no checksum, no dictionary identifier, and no long-distance
matching. The decoder enforces the plan's 1 MiB window and the archive declares
a conservative 4 MiB working-set requirement. STORE remains `store/v1` and has
zero decoder-memory declaration. Planner v3 aggregate working-set declarations
may additionally include stored Dictionary bytes and bounded-group access.

LAI, ContentObject logical digests, and AUX exclude creation policy and codec
parameters. Under `ecf/bootstrap-v1`, PCR deliberately binds plaintext Chunk
organization and the chunker but excludes TransformPlans, so changing only the
codec profile while retaining fixed chunking preserves PCR too. Current public
v2 and v3 profiles may select different CDC policies, in which case PCR
changes. PCI hashes every exact container byte and therefore captures planner,
plan, chunking, and encoded-payload differences.

Current public profiles create `*-v4` archives. Frozen v3 preserves this
independent selection behavior before complete-cost Dictionary and bounded
ChunkGroup choices. V4 broadens the independent baseline as documented in
[codec-transform-v1.md](codec-transform-v1.md), then retains the v3 cross-file
strategy. No v1, v2, or v3 planner behavior was redefined.
