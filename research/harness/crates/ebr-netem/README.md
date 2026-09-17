# ebr-netem

Network emulation proxy and range origin for remote-access research.

**Status: skeleton only.** This crate exists in the harness workspace (its
`Cargo.toml`, workspace lints, and dependency on `ebr-common` are set up and
it compiles) so the workspace has a place for it, but its implementation is
built out by a separate agent, as scoped by the Phase B2 harness-skeleton
task. Nothing in `src/lib.rs` beyond a placeholder constant and a smoke test
should be read as a design decision.

## Why this crate needs to exist

`research/PROGRESS.md`'s execution-environment section records that
`sch_netem` (Linux's kernel-level network emulation, `tc qdisc add ... netem`)
is unavailable in both WSL2 and Docker on this host. Remote-access research
(`entrybound::random_access::HttpRangeSource` against `ebr-access`) needs
latency, bandwidth-cap, and loss emulation to produce anything resembling a
real remote-storage access pattern. Without kernel `netem`, that has to be an
application-level proxy: something that sits between `HttpRangeSource` and a
real HTTP range-serving origin, and deliberately delays, throttles, or drops
requests according to a configured profile.

This is a documented threat to validity (`PROGRESS.md`'s "Known blockers and
limitations": "Packet-level loss/latency emulation (`netem`) is unavailable;
remote-access experiments use an application-level proxy"), not something a
future implementation of this crate can quietly resolve -- whatever it
builds should say so in its own README and in the eventual experiment
writeups that use it.

## What the eventual implementation needs to provide

- **A range origin**: an HTTP server that serves byte-range requests over a
  fixed archive file (or a directory of them), for `HttpRangeSource` to read
  from -- this is the "origin" half of the name.
- **A network emulation proxy**: sits in front of that origin (or wraps the
  client side) and applies a configured latency/bandwidth/loss profile per
  request, at the application layer, since kernel `netem` is not available.
- Enough of a Rust API (or a binary this workspace's `ebr-access` /
  `ebr-netem` binaries can point `HttpRangeSource::open` at) that an
  experiment can compare random-access behavior under different network
  profiles, the same way `ebr-access` already compares INDEXED vs. STREAM
  access.

## Testing

```sh
cargo +1.98.1 test -p ebr-netem
```

Only a placeholder smoke test exists today.
