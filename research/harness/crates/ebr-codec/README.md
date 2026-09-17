# ebr-codec

Codec/transform/reconstruction research: production internals reached
through `entrybound`'s default-off `research-internals` feature
(`entrybound::research::codec`), plus candidate external codecs not shaped
by `entrybound::eam::TransformPlan`'s model.

## What it provides

- `production` -- `roundtrip_zstd`/`roundtrip_lz4`/`roundtrip_lzma2`: build a
  `TransformPlan` exactly the way production's planner would (`zstd_plan`,
  `lz4_plan`, `lzma2_plan`), validate it (`validate_plan`), and round-trip
  through the exact `encode_payload`/`decode_payload` production uses.
  Nothing here changes production behavior or archive bytes -- see
  `research/harness/research-internals.md` for the identity guarantees this
  relies on.
- `external` -- `roundtrip_zstd_crate`: the plain `zstd` crate (pinned to the
  exact same `=0.13.3` production depends on) called directly, not through a
  `TransformPlan`. This reaches encoder parameters `TransformPlan` does not
  model at all (long-distance matching, an explicit window log decoupled
  from the level, multithreaded encoding). See the module docs for why that
  is the actual research question this crate exists to eventually answer.
- `compare_zstd_level(level, data)` -- runs both paths at the same level over
  the same bytes.

## Non-goals (for now)

- `external` wires up one directly-comparable baseline (plain zstd, default
  frame settings). It does not yet sweep long-distance matching or an
  explicit window log, and it does not yet add a codec production has never
  used (brotli, a different lzma binding, ...). Both are natural next steps
  that fit the existing `external` module without changing its shape.
- No dictionary- or prefix-mode comparison yet
  (`codec::zstd_dictionary_plan`/`zstd_prefix_plan` are exposed by
  `entrybound::research::codec` but not wrapped here).
- No `transform`/`reconstruction` (DEFLATE, JPEG) research yet, even though
  `entrybound::research::transform`, `::reconstruction`, and
  `::jpeg_reconstruction` are all available behind the same feature (see
  `research/harness/research-internals.md`'s "Exposed items" table). This
  crate's name reserves the space; filling it in is future work.

## The `ebr-codec` binary

```text
ebr-codec --item-path <file> --experiment-id <id> --env-name <name> \
          [--level <i32>] [--run-id <id>] [--out <path>]
```

`--level` must be one of `entrybound::research::codec::SUPPORTED_LEVELS`
(`1, 3, 5, 9, 15, 19, 22`); default `3`. See `research/harness/README.md` for
the shared flag contract.

## Testing

```sh
cargo +1.98.1 test -p ebr-codec
```

All tests use small in-memory buffers built from a repeating string;
nothing in this crate reads `research/corpus` or `/root/eb-research` unless
a caller points `--item-path` there.
