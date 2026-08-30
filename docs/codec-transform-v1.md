# Codec and structural-transform generation v1

Status: experimental frozen implementation note for the v4 planners. This
note extends `ecf/bootstrap-v1`; it does not redefine v1-v3 archive bytes.

## Closed registries and wire feature

Codec and transform identifiers are matched only against compile-time
registries. A registry entry owns its identifier, format version, required
incompatibility feature, parameter validation, deterministic encode/decode
dispatch, and decoder-resource derivation. Transform entries additionally own
their reversibility class and forward/inverse operations. Archive-controlled
strings cannot select arbitrary executable code.

V4 archives require incompatibility bit `0x2`, named `codec-transform-v1`, in
addition to v3's `cross-file-compression-v1`. With bit `0x2`, TransformPlan
field 3 contains ordered canonical type-13 TransformStep records:

```text
transform identifier(1) | canonical parameter bytes(2)
```

Both fields are required. Unknown identifiers, duplicate transform families,
or invalid parameters fail closed. Without bit `0x2`, the historical field is
required to remain empty. This feature gate avoids silently assigning new
meaning to the old transform-string placeholder, which released planners never
populated.

## Registered codecs

| Codec | Canonical parameters | Declared decoder requirement |
|---|---|---|
| `store/v1` | empty | none |
| `zstandard/v1` | frozen `ZP01`, `ZD01`, or `ZX01` records from v1/v3 | 1 MiB window, 4 MiB working set |
| `lz4/v1` | `"L401"`, block format `1`, three zero reserved bytes | 64 KiB window, 128 KiB working set |
| `lzma2/v1` | `"LM21"`, preset `u8`, three zero flag/reserved bytes, dictionary bytes `u32be` | recorded dictionary window and `lzma2_get_memory_usage(dict) * 1024` working set |

LZ4 is a raw independent block encoded by pinned `lz4_flex` 0.14.0 with its
safe default block encoder. No frame defaults or external dictionary are
implicit. LZMA2 is a raw independent stream encoded by pinned `lzma-rust2`
0.20.0 with default features disabled and only `std` plus `encoder` enabled.
Its frozen `(preset, dictionary bytes)` pairs are `(4, 1 MiB)`, `(6, 4 MiB)`,
and `(9, 8 MiB)`; chunk size is automatic, operation is single-threaded, and
there is no external dictionary. The selected crates are permissively licensed
(MIT for `lz4_flex`, Apache-2.0 for `lzma-rust2`) and Entrybound itself contains
no unsafe codec implementation.

The decoder validates the full parameter value and caller memory policy before
dispatch. It allocates only the declared logical output length, rejects
trailing/malformed payloads and length mismatch, then applies inverse
transforms and verifies the authoritative plaintext Chunk SHA-256.

## Exact transforms

All v1 structural transforms are length-preserving bijections over every byte
string.

### `delta8/v1`

Parameters are empty. Let `previous = 0`. Forward output byte `i` is
`input[i] - previous (mod 256)`, after which `previous = input[i]`. Inverse
decoding starts at zero and cumulatively adds each delta modulo 256.

### `byte-shuffle/v1`

Parameters are exactly one byte with value 2, 4, or 8. For width `w`, let
`n = floor(length / w)`. Forward encoding emits lane 0 for all `n` complete
records, then lane 1, through lane `w-1`; the final `length mod w` bytes are
copied unchanged. Inverse decoding restores the interleaving and copies the
same trailing bytes. Thus every possible tail length is deterministic.

Encoding applies TransformSteps in recorded order and then the codec. Decoding
performs codec decode, then inverses in reverse order.

## Frozen v4 creation policies

V4 retains the v3 CDC, exact deduplication, similarity, dictionary, physical
ordering, and bounded-lookback policies. `balanced-v4` still has lookback zero
by contract. The independent candidate sets below are ordered; earlier,
simpler candidates win equal complete cost.

| Planner | Independent candidates | Structural candidates when the frozen probe qualifies |
|---|---|---|
| `fast-v4` | STORE, LZ4, Zstandard level 1 | none |
| `balanced-v4` | STORE, LZ4, Zstandard levels 1/3/5 | delta8 → Zstandard 3; byte-shuffle-4 → Zstandard 3 |
| `dense-v4` | STORE, Zstandard 5/9/15, LZMA2 4+1MiB, LZMA2 6+4MiB | delta8 and byte-shuffle-2/4/8, each with Zstandard 9 and LZMA2 6+4MiB |
| `extreme-v4` | STORE, LZ4, Zstandard 9/15/19/22, all three LZMA2 pairs | delta8 and byte-shuffle-2/4/8, each with Zstandard 15, LZMA2 6+4MiB, and LZMA2 9+8MiB |

The integer-only structural eligibility probe is the existing v1 probe:
within its deterministic sample, a candidate search runs when distinct bytes
are at most 192, the most common byte reaches 12,000 parts per million, or
adjacent equal bytes reach 20,000 parts per million. Fast never searches
transforms. The probe only bounds work; actual candidate bytes decide selection.

After independent selection, the unchanged profile-specific v3 strategy may
replace a similarity cohort with Zstandard shared-dictionary encoding
(`balanced`, `dense`, `extreme`) or bounded prefix lookback (`dense`,
`extreme`). Those representations do not accept structural transforms in this
generation. Their full existing Dictionary/ChunkGroup costs remain part of the
comparison.

## Complete-cost rule

For an independent candidate the measured cost is:

```text
exact encoded payload bytes + exact canonical TransformPlan record bytes
```

Non-STORE candidates must improve on STORE by more than the larger of 32 bytes
and one percent of the STORE complete cost. A structural pipeline must then
clear that same margin against the best non-transform candidate. Strict
comparison makes STORE and simpler candidates win ties. Dictionary and group
selection continues to include payloads, full Dictionary/TransformPlan/group
records, and section overhead as documented for v3.

## Identity and limitations

Codec identifiers, codec parameters, TransformSteps, Dictionary choice,
ChunkGroup layout, and physical ordering remain physical facts. With unchanged
chunking, content, and metadata they do not enter ContentObject logical digest,
LAI, AUX, or PCR; exact container PCI may change. No format-specific parser,
reconstructive transform, executable transform, new CDC algorithm, unbounded
solid stream, encryption, or STREAM layout is introduced here.
