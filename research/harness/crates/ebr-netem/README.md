# ebr-netem

Application-level network emulation (RTT, bandwidth, jitter, loss-as-stall,
a CDN cache layer) plus a range-serving HTTP origin, for remote-access
research against `entrybound::random_access::HttpRangeSource`.

## Why this crate exists

`research/PROGRESS.md`'s execution-environment notes: `sch_netem` (Linux
kernel-level `tc qdisc add ... netem`) is unavailable in both WSL2 and
Docker on this host. Remote-access research needs latency, bandwidth-cap,
jitter, and loss emulation to produce anything resembling a real
remote-storage access pattern for `HttpRangeSource` to read through.
Without kernel `netem`, that has to be application-level: a proxy that sits
between the client and a real HTTP range-serving origin and deliberately
delays, throttles, or stalls traffic according to a configured profile.

**This is a documented threat to validity, not something this crate quietly
resolves.** See "Threats to validity" below, and carry the same caveat into
any experiment writeup that uses this crate.

## What's here

Two binaries plus a calibration tool, built on `tokio` 1.53.1 + `hyper`
1.11.1 + `hyper-util` 0.1.20 (exact pins, see `Cargo.toml`), no `unsafe`
(workspace-wide `unsafe_code = "forbid"`):

- **`ebr-origin`** -- a static HTTP/1.1 (and HTTP/2) range-serving file
  server for `.eb` archives (or any file), with the exact single-range
  contract `entrybound::random_access::HttpRangeSource` requires, plus
  configurable "object-store-like" misbehavior for testing client
  robustness.
- **`ebr-netem-proxy`** -- a reverse proxy applying per-connection RTT,
  bidirectional bandwidth token-bucket shaping, jitter, a connection-setup
  delay, a loss-as-stall model, a concurrent-connection cap, and an
  optional CDN cache layer.
- **`ebr-netem-calibrate`** -- runs both of the above in-process over a
  small RTT/bandwidth sweep and writes `calibration.md` (this directory)
  from raw JSONL under `research/raw/EXP-NETEM-CAL/`.

Shared, independently unit-tested building blocks: `src/range.rs`
(`Range` parsing, `multipart/byteranges` framing), `src/etag.rs`
(strong/weak ETag formatting, `If-Match` evaluation), `src/bandwidth.rs`
(the token bucket), `src/loss.rs` (the loss-as-stall model), `src/cache.rs`
(the CDN cache), `src/rng.rs` (the seeded PRNG jitter/loss draw from), and
`src/config.rs` (CLI flag parsing for both servers).

## `ebr-origin`

```text
ebr-origin --root <dir>
           [--listen 127.0.0.1:0] [--etag strong|weak|missing]
           [--multi-range true] [--max-ranges 16]
           [--full-body-fallback false] [--revision-churn-after <n>]
           [--h2 true] [--log <path>] [--env-id adhoc]
```

Serves every file under `--root` read-only. `HEAD` always returns `200`
with `Content-Length`, `Accept-Ranges: bytes`, and (unless `--etag missing`)
an `ETag`; never `Content-Encoding` (nothing here compresses).

A ranged `GET` sent with `If-Match: "<etag>"` gets `412 Precondition
Failed` if the tag no longer matches the file's current strong identity
(see "Revision churn" below); otherwise the multi-range decision table is:

| Ranges requested (`N`) | `--multi-range` | `N` vs `--max-ranges` | Response |
|---|---|---|---|
| 0 (no `Range`, or unparseable) | -- | -- | `200`, whole body |
| 1 | -- | -- | `206`, single part |
| ≥2, all unsatisfiable | -- | -- | `416` |
| ≥2 | `false` | -- | `206`, single part (first range only) |
| ≥2 | `true` | `N` > max | `416` |
| ≥2 | `true` | `N` <= max | `206`, `multipart/byteranges` |

`--full-body-fallback true` skips this table entirely: every `GET` gets
`200` with the whole body regardless of `Range`. Combined with
`--etag weak`/`--etag missing`, these three toggles exist specifically to
prove a client *refuses* a non-conformant origin instead of silently
misreading it -- exactly what
`crates/entrybound/src/random_access.rs`'s own unit tests already do
against a hand-rolled TCP fixture (weak ETag -> `HttpRangeUnsupported`;
`200` instead of `206` -> `HttpRangeUnsupported`), and what
`tests/origin_contract.rs` re-proves here against the real
`HttpRangeSource`, not a fixture.

### Revision churn

`--revision-churn-after <n>`: once the server has answered more than `n`
`GET`/`HEAD` requests in total (a whole-server counter, not per-file),
every file's served bytes and ETag deterministically flip to a second
"revision" (last byte XORed, hash recomputed once and cached). An
in-flight random-access session whose `If-Match` was captured before the
flip then gets `412` on its next range read, exactly like
`HttpRangeSource::read_exact_at`'s "HTTP source ETag changed during random
access" path. This is the closest an application-level, single-process
test server can get to "the remote object changed mid-session" without a
second, coordinated process.

### HTTP/2

`--h2 true` (the default) lets `hyper_util::server::conn::auto` accept
HTTP/2 over plain TCP via *prior knowledge* (the client sends the `PRI *
HTTP/2.0` connection preface directly, no ALPN/TLS negotiation needed).
This crate never sets up TLS anywhere (see "Threats to validity"), so this
is the only way HTTP/2 is "practical" here at all; a normal browser or a
client that only speaks h2-over-TLS will still get HTTP/1.1.

## `ebr-netem-proxy`

```text
ebr-netem-proxy --upstream <host:port>
                 [--listen 127.0.0.1:0] [--rtt-ms 0]
                 [--jitter none|uniform|normal]
                 [--jitter-max-ms <n>] [--jitter-mean-ms <n>] [--jitter-stddev-ms <n>]
                 [--bandwidth-up-mbit <n>] [--bandwidth-down-mbit <n>]
                 [--burst-up-bytes 65536] [--burst-down-bytes 65536]
                 [--setup-rtt-multiple 1.0]
                 [--loss-p 0.0] [--loss-rto-ms 200] [--loss-unit packet|chunk] [--seed 42]
                 [--max-connections 64]
                 [--cache-mode off|cold|warm] [--cache-capacity-bytes 16777216] [--cache-warm-list <path>]
                 [--chunk-size 16384] [--log <path>] [--env-id adhoc]
```

`--bandwidth-*-mbit` use the networking convention of decimal
megabits-per-second (1 Mbit/s = 125,000 bytes/s), not mebibytes.

### Per-request RTT and jitter

For every proxied request: sleep `rtt/2 + jitter/2` before the request
reaches the origin, then (after the real, unshaped local fetch) sleep
`rtt/2 + jitter/2` again before the response's first byte reaches the
client. One jitter value is sampled per request (from `--jitter`'s model)
and split evenly across the two legs, so total added latency for a request
with no cache involvement is `rtt + jitter`, never less than `rtt`.

### Bandwidth (token bucket, per direction)

`src/bandwidth.rs`'s `TokenBucket`: starts full up to `--burst-*-bytes`
(an initial burst is free), refills at the configured rate. `up_bucket`
paces the client's request body before it is forwarded to the origin;
`down_bucket` paces the response body sent back to the client. Because
`HttpRangeSource` only ever sends `GET`/`HEAD` (empty bodies), `up_bucket`
is real and wired through end to end but, for that workload, is always
metering `0` bytes.

**Minimum effective burst (harness review round 1, finding R1-13).** The
bucket's effective capacity is never below 5 ms of the configured rate
(`bandwidth::MIN_BURST_SECONDS`), for the same reason Linux `tc tbf`
requires `burst >= rate / HZ`: `tokio::time::sleep` resolves to whole
milliseconds and never wakes early, and the bucket can only credit that
oversleep back up to its capacity. The provisional calibration below
(`--burst-down-bytes 4096`, 16 KiB chunks) achieved 62 and 107 Mbit/s at a
configured 100 and 1000 -- the rates whole-millisecond sleeps of 16 KiB
chunks produce -- because the tiny burst discarded every oversleep credit.
With the minimum burst, an ignored smoke test
(`smoke_high_rate_bandwidth_is_not_limited_by_timer_granularity`, run with
`-- --ignored`) achieved 95.8% of a configured 100 Mbit/s with the same
4096-byte burst and 16 KiB chunks (non-decision-grade, 2026-09-17).
`calibration.md` predates this fix and must be regenerated.

**`--chunk-size` matters at high configured rates.** `TokenBucket::consume`
is called once per chunk, and each call's fixed cost (a mutex lock, an
`Instant::now()`, the async scheduling around it, one socket write) does
not shrink as the configured rate grows, while the time budget between
chunks does. `calibration.md`'s bandwidth sweep uses the default
`--chunk-size 16384` throughout and shows the achieved rate falling well
short of the configured one at 100 and 1000 Mbit/s; a spot check confirmed
the cause directly -- at a configured 1000 Mbit/s, `--chunk-size 16384`
achieved only about 79 Mbit/s, while `--chunk-size 1048576` (1 MiB)
achieved about 778 Mbit/s from the same origin, proxy, and client, back to
back. Pick `--chunk-size` with the configured rate in mind, not just the
16384 default; at settings above a few hundred Mbit/s, a chunk size in the
hundreds of KiB to low MiB range is what actually gets close to the
configured rate.

### Connection setup delay

`--setup-rtt-multiple` (default `1.0`): each **accepted client TCP
connection** pays `rtt * setup_rtt_multiple` once, before its first request
is served -- approximating a TCP handshake (`1.0`) or a TCP handshake plus
a simulated TLS handshake on top (`2.0`-`3.0`). This crate never actually
performs a TLS handshake (see "Threats to validity"); the cost is a
sleep, not a real negotiation.

### Loss as a stochastic stall

`src/loss.rs`: there is no way to drop bytes at the application layer
without either corrupting the transfer or reimplementing TCP
retransmission. Instead, before each chunk of the response body is
forwarded, every simulated TCP segment in it (1448 payload bytes,
`--loss-unit packet`, the default) is lost with probability `--loss-p`, and
the transfer stalls `--loss-rto-ms` per lost segment -- bytes are delayed,
never dropped or corrupted. Earlier revisions rolled once per `--chunk-size`
chunk, so the same `--loss-p` meant a 64x lower per-byte loss rate after
raising `--chunk-size` from 16 KiB to 1 MiB as advised above (review finding
R1-13); `--loss-unit chunk` keeps that legacy behavior for comparison. **This is an approximation
of packet loss's time cost, not of packet loss itself**: no
congestion-window reduction, no real retransmission, no reordering.

### Max concurrent connections

`--max-connections`: a `tokio::sync::Semaphore` permit is acquired *before*
`listener.accept()`. Once the limit is reached, further TCP connections
simply queue in the kernel's accept backlog instead of being serviced --
an application-layer approximation of a connection cap, not a kernel-level
one (a strict kernel cap would refuse or reset excess connections instead
of queuing them).

### CDN cache layer

`--cache-mode cold` (starts empty) or `--cache-mode warm` (pre-populates
from `--cache-warm-list` at startup, fetched directly from the origin,
bypassing RTT/bandwidth shaping -- warming is a startup cost, not a
proxied request). `--cache-warm-list` is a plain text file, one entry per
line: `<path>` or `<path> <range-header>`, e.g. `/archive.eb bytes=0-1023`;
blank lines and `#` comments are ignored.

Keyed by `(path, raw Range header)`, size-bounded LRU
(`--cache-capacity-bytes`), counting hits/misses/evictions/bypasses (an
entry larger than the whole configured capacity is served directly and
never cached). A cache hit skips the origin-facing leg of RTT/loss
entirely and serves straight from memory, still paced through the
downstream bandwidth/jitter model on the way to the client (a CDN edge is
closer to the origin, not to the client).

**Not revision-aware:** a cached entry is served as-is until evicted, with
no revalidation against the origin's current ETag. Running `ebr-origin`
with `--revision-churn-after` set *and* the proxy's cache enabled at the
same time is out of scope and will silently serve stale bytes past the
churn point -- a known, documented limitation, not a bug to chase.

## Where emulation is applied (and where it is not)

All of the above -- RTT, jitter, bandwidth, loss, connection setup delay --
applies to the **client-facing leg only**. The proxy-to-origin leg is
treated as "inside the datacenter": fast, local, and unshaped, the same
simplification as running `tc qdisc` on a single interface of a real
network emulator. This keeps the whole model at the HTTP body-framing
level (pacing bytes handed to the client) instead of needing a custom
`AsyncRead`/`AsyncWrite` wrapper around raw sockets in both directions.

## Logging

Both servers append one JSON line per request (`ebr.harness.raw.v1` via
`ebr_common::results::ResultWriter` -- see that module's doc comment for
why this differs from the Python baselines' `ebr.raw.v1` schema) to
`--log <path>` when given; logging is off by default. Each row's `payload`
carries `method`, `path`, `range`, `status`, `bytes_sent`, and:
`ebr-origin` adds `churned`; `ebr-netem-proxy` adds `elapsed_ms` and
`cache_status` (`"hit"`, `"miss"`, `"bypass"` for a non-`GET` method, or
`"error"` on an upstream failure, which the proxy answers with `502`).

## Calibration

```sh
CARGO_TARGET_DIR=/root/eb-research/target/b2-netem \
  /root/.cargo/bin/cargo +1.98.1 run -p ebr-netem --bin ebr-netem-calibrate
```

Runs one `ebr-origin` and, per setting, a fresh `ebr-netem-proxy` in
process against a generated scratch archive, over the task's example
settings (RTT 1/20/100/300 ms; bandwidth 1/10/100/1000 Mbit/s), and writes:

- Raw per-request JSONL under `research/raw/EXP-NETEM-CAL/<run_id>.jsonl`.
- `calibration.md` (this directory), generated from that raw data.

The RTT sweep uses unmetered bandwidth and a 1-byte range (isolating RTT
from bandwidth pacing); the bandwidth sweep uses a fixed `--rtt-ms 1` and a
range sized to take about a second at the configured rate (isolating
bandwidth from the RTT floor). Both are described in more detail, alongside
the actual results, in `calibration.md`'s own methodology section --
**every number there is labeled provisional** (this binary does not check
for a quiet machine) and the report says, in its own words, that it must
be re-run in a confirmed quiet window before being treated as
decision-grade. This is exactly the "smoke timings... labeled
non-decision-grade" the task allows, not a substitute for that re-run.

## Threats to validity

Everything above is a documented, deliberate simplification relative to
real network conditions and kernel-level `netem`, in addition to the
missing-`sch_netem` reason this crate exists at all:

- **No slow start (use constraint).** A fresh real TCP connection needs
  about `log2(response / (10 * MSS))` extra round trips before a large
  response reaches full rate; this proxy pays one RTT per request
  regardless of size. Emulated results therefore favor fewer, larger range
  requests on new connections. Any decision about range coalescing
  (`coalesce_gap_bytes`, request counts versus sizes) made on this emulator
  must include a sensitivity analysis showing the conclusion survives an
  added `log2`-scaled per-request RTT penalty, or be confirmed on a real
  path.
- **Low-RTT bias.** The provisional calibration shows about +3 ms at a
  configured 20 ms and noisy results at 1 ms (timer rounding plus loopback
  proxying). Use the achieved `elapsed_ms` from `--log`, not the configured
  RTT, and do not draw conclusions from configured RTTs below 20 ms.
- **No real congestion control.** Real TCP's throughput under loss and
  latency is shaped by its congestion-control algorithm (slow start,
  congestion avoidance, cwnd reduction on loss) interacting with the
  kernel's socket buffers. This proxy's token bucket and fixed RTT sleep
  produce a *smoother*, more *deterministic* shape than that -- there is no
  slow-start ramp, no cwnd, and a "loss" event costs a fixed RTO stall
  rather than a cwnd halving that then has to ramp back up. A client
  tuned against this emulator may behave differently against a real,
  congestion-controlled network path.
- **Loss is a stall, not a drop.** `src/loss.rs`'s doc comment: bytes are
  delayed by one RTO, never actually dropped, corrupted, or reordered.
  Real packet loss forces retransmission (extra round trips, not just a
  fixed delay) and can reorder later segments; none of that is modeled.
- **No head-of-line blocking difference between HTTP/1.1 and HTTP/2.**
  Both this proxy and `ebr-origin` buffer each response fully in memory and
  hand it to whichever protocol the connection negotiated; real HOL
  blocking effects (HTTP/1.1 serializing requests per connection; HTTP/2
  multiplexing streams but still head-of-line-blocked at the *TCP* layer
  under real loss) are not reproduced, because there is no real TCP-layer
  loss to block on in the first place.
- **RTT/loss/setup-delay only apply to the client-facing leg**, never the
  proxy-to-origin leg (see "Where emulation is applied" above) -- a real
  network path's asymmetry (e.g. the origin is also remote) is not
  modeled unless the operator runs a second proxy in front of the origin
  too.
- **Both legs typically run on one host over loopback** in this program's
  environment (no netem-capable kernel path exists to send real packets
  over, per the reason this crate exists). Loopback has no real NIC
  queueing, no real MTU/fragmentation behavior, and effectively unlimited
  underlying bandwidth and near-zero underlying latency -- everything
  observed is *this crate's* emulation, not any interaction with a real
  link.
- **Bodies are fully buffered in memory**, both in `ebr-origin` (the whole
  file, once, cached) and in `ebr-netem-proxy` (each response, per
  request, for pacing and possible caching). Fine at the file sizes this
  research program's corpus uses; not a design that scales to
  multi-gigabyte objects without becoming a memory-pressure experiment of
  its own.
- **The seeded RNG's reproducibility is per-draw, not per-run.** `src/rng.rs`'s
  doc comment: a fixed `--seed` reproduces the *distribution* jitter/loss
  draws come from, but which concurrent request's task calls the shared,
  mutex-guarded RNG first on a given poll depends on the Tokio scheduler,
  not the seed. A multi-connection run is not expected to be byte-for-byte
  reproducible from the seed alone.

## Testing

```sh
CARGO_TARGET_DIR=/root/eb-research/target/b2-netem /root/.cargo/bin/cargo +1.98.1 test -p ebr-netem
```

`src/*.rs`'s `#[cfg(test)]` modules unit-test the pure logic (`Range`
parsing and `multipart/byteranges` framing, ETag formatting and `If-Match`
evaluation, the token bucket's timing under `tokio::time::pause`, the loss
model's roll rate, the cache's LRU eviction and hit/miss accounting, the
seeded RNG's reproducibility) without a running server.

`tests/origin_contract.rs` starts a real `ebr-origin` on an ephemeral
loopback port and checks the exact wire contract: single-range `206` with
correct `Content-Range`/`Content-Length`, `416` on an unsatisfiable range,
`412` on a stale `If-Match` after `--revision-churn-after`, `multipart/byteranges`
framing for a genuine multi-range request, the weak-ETag and
full-body-fallback misbehavior toggles, **and** a round trip through the
real, unmodified `entrybound::random_access::HttpRangeSource` (a
dev-dependency on `crates/entrybound`, default features only -- this never
changes production semantics, it only calls entrybound's existing public
API from a test) to prove the origin actually satisfies the production
client, not just this crate's own reading of the contract.

`tests/proxy_netem.rs` starts a real `ebr-origin` and `ebr-netem-proxy`
pair and checks observable behavior at small (test-fast) settings: a
configured RTT is actually paid, a configured bandwidth cap actually slows
a transfer, the loss model actually stalls at a high configured
probability, and the cache actually counts a hit after a miss and serves
without incrementing the origin's own request count.
