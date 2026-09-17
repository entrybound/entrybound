# ebr-access

Random/remote access and streaming/memory measurements, over entrybound's
public production API.

## What it provides

- `measure_random_access(encoded, paths)` -- opens Complete INDEXED bytes for
  metadata-first random access (`entrybound::ecf::open_indexed_random`) and
  reads back every path, recording each read's `bytes_fetched` and
  `range_request_count` (`RandomAccessVerificationReport`, unchanged from
  what production already computes). This is the same accounting a real
  remote backend (`entrybound::random_access::HttpRangeSource`) would pay
  for, even though this crate drives it over an in-memory source
  (`MemoryRandomReadSource`) so far.
- `measure_stream(archive)` -- STREAM sequential encode -> open -> verify
  (`entrybound::ecf::{encode_stream, open_stream, verify_stream}`), the
  layout meant for a source that cannot seek.
- Both attach `ebr_common::measure::peak_memory()` after the operation, so a
  caller can compare INDEXED random access's metadata-first footprint
  against STREAM's necessarily-more-sequential one.

## Non-goals (for now)

- Random access here runs over `MemoryRandomReadSource`, not
  `entrybound::random_access::HttpRangeSource` -- there is no remote origin
  in this measurement yet. `ebr-netem` (built separately) is the network
  emulation proxy/origin a later experiment would put in front of
  `HttpRangeSource` to get real range-request-count-under-latency numbers.
- No access-pattern sweep (sequential vs. scattered path order, warm vs.
  cold `RandomAccessArchive`, repeated reads of the same path) yet -- this
  crate reads every path once, in entry order.
- `measure_stream` runs against an in-memory `Vec<u8>` sink/source pair, not
  an actual unseekable pipe; the STREAM *format* does not require seeking,
  but this test harness does not yet prove it under one.

## The `ebr-access` binary

```text
ebr-access --item-path <dir> --experiment-id <id> --env-name <name> \
           [--profile fast|balanced|dense|extreme] [--run-id <id>] [--out <path>]
```

Plans and encodes `--item-path` once (INDEXED), then runs both measurements
against every regular-file entry, and appends one `ebr.harness.raw.v1` JSONL
row carrying both. See `research/harness/README.md` for the shared flag
contract.

## Testing

```sh
cargo +1.98.1 test -p ebr-access
```

Tests build their own small scratch input trees under `std::env::temp_dir()`
and remove them afterward; nothing in this crate reads `research/corpus` or
`/root/eb-research` unless a caller points `--item-path` there.
