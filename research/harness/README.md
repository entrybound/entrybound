# Entrybound research harness

A Rust Cargo workspace for Phase B2 research measurements: end-to-end
archive operations, chunking, codecs, planner traces, and access patterns,
all built on entrybound's public API (and, where needed, its default-off
`research-internals` feature). This is its **own** `[workspace]`, entirely
separate from the production workspace at the repo root
(`D:/Projects/entrybound/entrybound/Cargo.toml`, `crates/entrybound` +
`crates/entrybound-cli`). See "Isolation from the production workspace"
below for how that separation is enforced and verified.

## Layout

```
research/harness/
  Cargo.toml              workspace root ([workspace], not a member of the root one)
  Cargo.lock              committed: exact resolved versions for the whole graph
  build.sh                WSL build/test entry point (pinned toolchain, target-dir convention)
  build.ps1               Windows equivalent
  README.md               this file
  research-internals.md   the entrybound research-internals feature (Internals agent)
  internals-identity-check.{sh,ps1,md}, internals_identity_tools.py
                          byte-identity proof for that feature (Internals agent)
  crates/
    ebr-common/    shared library: manifest reading, held-out guard, tree walking,
                   JSONL results, env/run identity, process self-measurement, timing, hashing
    ebr-pack/      end-to-end plan/encode/open/verify/unpack/random-read measurements
    ebr-chunk/     production gear-norm-v1 vs. a research-only alternative CDC
    ebr-codec/     production codec paths (research-internals) vs. candidate external codecs
    ebr-planner/   planner candidate-enumeration drift and regret checks (research-internals)
    ebr-access/    INDEXED random access and STREAM sequential access, with byte/memory accounting
    ebr-netem/     network emulation proxy and range origin -- skeleton only, built by a separate agent
```

Every crate under `crates/` has its own `README.md` describing its specific
purpose, what it does and does not cover yet, and (for the six with one) its
binary's exact flags. This file only covers what is common to all of them.

## Build commands

Both scripts default to `cargo +1.98.1 test --workspace` when given no
arguments, and otherwise pass every argument straight through to cargo.

**WSL** (`research/harness/build.sh`; requires `bash`):

```sh
wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh
wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh build --release
wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh clippy --workspace --all-targets
wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh fmt --all -- --check
```

(Run directly inside a WSL shell, drop the `wsl.exe -d Ubuntu -- ` prefix.)

**Windows** (`research/harness/build.ps1`; PowerShell 7):

```powershell
D:/Projects/entrybound/entrybound/research/harness/build.ps1
D:/Projects/entrybound/entrybound/research/harness/build.ps1 test -p ebr-pack
D:/Projects/entrybound/entrybound/research/harness/build.ps1 clippy --workspace --all-targets
```

Both scripts invoke the pinned research Rust toolchain explicitly
(`cargo +1.98.1 ...`), never plain `cargo`/`rustc` -- WSL's unqualified
`rustc` resolves to Ubuntu's packaged 1.75.0, and Windows floating `stable`
differs from the pinned toolchain too (see `research/PROGRESS.md`'s
"Execution environment"). `build.ps1` has no declared PowerShell
`param()` block on purpose, so a cargo flag that looks like a PowerShell
parameter (`-p ebr-common`) reaches cargo through `$args` untouched instead
of being caught by PowerShell's own parameter binder.

## Target-dir conventions

Neither script lets Cargo build into a directory under `research/harness/`
itself (that would put build output inside a directory `git status` walks).
Both set `CARGO_TARGET_DIR` if it is not already set:

| Host | Default `CARGO_TARGET_DIR` |
|---|---|
| WSL | `/root/eb-research/target/harness` |
| Windows | `D:/eb-research/target/harness` |

This is a **shared, stable** location for this one workspace across
sessions -- not the per-agent `.../target/b2-<label>` convention other Phase
B2 agents use while several of them build concurrently and would otherwise
contend for the same target directory's lock. If you are building this
workspace alongside another agent's concurrent work, set
`CARGO_TARGET_DIR`/`$env:CARGO_TARGET_DIR` yourself to a distinct path
before calling the script.

## Isolation from the production workspace

`research/harness/Cargo.toml` declares its own `[workspace]` table. Cargo's
workspace discovery stops at the nearest ancestor `Cargo.toml` with a
`[workspace]` table, so a cargo invocation anywhere under `research/harness/`
never sees the root workspace, and a cargo invocation at the repo root
(`cargo test --workspace` etc.) never sees this one -- `research/harness`
is not, and must never become, a path in the root `Cargo.toml`'s
`members`. Verified for this commit:

```sh
# From the repo root:
cargo +1.98.1 metadata --no-deps --format-version 1
# workspace_members is exactly [entrybound, entrybound-cli], both before
# and after this workspace was added -- unaffected either way.

cargo +1.98.1 test --workspace
# Same production test outcome as the pre-existing baseline
# (research/PROGRESS.md's baseline: 286 passed, 0 failed on Windows release;
# a debug run from this session matches with no new failures attributable
# to this workspace's addition).
```

## How binaries plug into `ebr` runner specs

`research/tools/ebr` is the existing Python experiment runner (spec schema
`ebr.spec.v1`, see `research/tools/ebr/spec.py`'s module docstring). A spec's
`command` is a template string with placeholders `{item_path} {item_id}
{split} {family} {scratch} {candidate} {rep} {seed} {input_bytes}` plus every
candidate's own `params` keys, e.g.:

```yaml
command: "gzip -{level} -c -n {item_path} > {scratch}/out.gz"
```

Every binary in this workspace (`ebr-pack`, `ebr-chunk`, `ebr-codec`,
`ebr-planner`, `ebr-access`) takes the same shape of flags, so an `ebr` spec
can invoke one directly as its `command`:

```yaml
command: >-
  /root/eb-research/target/harness/release/ebr-pack
  --item-path {item_path} --experiment-id EXP-PACK-001 --env-name wsl-ubuntu
  --profile {candidate}
```

Flag-to-placeholder mapping:

| Flag | Where it comes from when run under an `ebr` spec |
|---|---|
| `--item-path` | the spec's `{item_path}` placeholder |
| `--experiment-id` | a literal, hardcoded in the command template -- it is not a builtin placeholder; the spec already knows its own `experiment_id` |
| `--env-name` | a literal too, matching whichever `research/environment/index.json` name the spec's `platform` targets -- there is no `{env_name}` placeholder either |
| `--profile` (`ebr-pack`/`ebr-planner`/`ebr-access`) or the profile-shaped preset choice (`ebr-chunk`/`ebr-codec`'s `--level`) | usually `{candidate}` or a candidate `params` key |
| `--run-id` | usually omitted -- each binary generates a fresh id per invocation, which is fine because the runner's own wrapping row (schema `ebr.raw.v1`) is what actually correlates repetitions; pass `--run-id {rep}-{seed}` only if you specifically want a binary's own JSONL row's `run_id` to match |
| `--out` | **omit it** when driving a binary from an `ebr` spec. With `--out` left unset, the binary prints exactly one JSON line to stdout, and the runner's own `stdout_json` metric source (`research/tools/ebr/run.py`'s `_json_key`, `research/tools/ebr/spec.py`'s `metrics[].source: stdout_json`) reads a dotted key path out of that line's `payload`, e.g. a metric `{name: encoded_bytes, source: stdout_json, key: payload.encoded_bytes}`. Passing `--out` instead redirects the row to a file and stdout is empty, which breaks that extraction. |

The runner wraps the whole invocation with its own resource measurement
(`research/raw/*/*.jsonl`, schema `ebr.raw.v1`) regardless of what a binary
prints, so a binary does not need to duplicate wall-clock/RSS accounting
just to be usable from a spec -- see "Output schema" below for why each
binary still does its own timing and memory sampling anyway.

Each binary can also be run standalone (outside any `ebr` spec) for manual
research work, in which case its own JSONL row on stdout (or `--out`) is the
only record of the run.

## Output schema

Every binary appends rows through `ebr_common::results::ResultWriter`,
schema `ebr.harness.raw.v1`:

```json
{
  "schema": "ebr.harness.raw.v1",
  "experiment_id": "EXP-PACK-SMOKE",
  "run_id": "20260917T033505Z-11c31d",
  "env_id": "8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e",
  "seq": 0,
  "timestamp_utc": "2026-09-17T03:35:05Z",
  "item_id": "/root/eb-research/scratch/ebr-pack-cli-smoke",
  "payload": { "...": "crate-specific measurement fields" }
}
```

`env_id` is resolved from `--env-name` against `research/environment/index.json`
(`ebr_common::runenv::EnvIndex`) -- the same environment fingerprints
`research/environment/README.md` describes, not a value the binary invents.
This schema is a first cut for in-process Rust measurements and is **not**
byte-identical with the Python baselines' `ebr.raw.v1`
(`research/raw/*/*.jsonl`), which additionally hash-chains rows
(`row_sha256`/`prev_row_sha256`) and carries per-external-tool measurement
detail that does not apply the same way to something measured in-process.
Reconciling the two schemas (or deciding they should stay distinct) is
future work for whichever agent designs the C-exec experiment specs;
`ebr_common::results`'s module docs record the gap rather than guessing at a
reconciliation.

## Dependency on production

Every crate that needs `entrybound` depends on it by path, with the
default-off `research-internals` feature (`research/harness/research-internals.md`)
enabled:

```toml
entrybound = { path = "../../../../crates/entrybound", features = ["research-internals"] }
```

`ebr-common` and `ebr-netem` do not depend on `entrybound` at all.
`ebr-pack`/`ebr-chunk`/`ebr-access` only call entrybound's *public* API
today even though the feature is enabled (so future measurement code in
those crates can reach `entrybound::research` without a `Cargo.toml`
change); `ebr-codec`/`ebr-planner` already use
`entrybound::research::{codec,planner}` directly. See each crate's own
README for specifics.

## Third-party dependency pins

Every third-party crate this workspace depends on directly is pinned to an
exact version (`= x.y.z`) in its `Cargo.toml`, matching `crates/entrybound`'s
own style; `Cargo.lock` is committed and freezes the exact version of every
transitive dependency too. Two pins deliberately match `crates/entrybound`'s
own exact pins, for a reason each crate's README explains:
`sha2 = "=0.11.0"` (`ebr-common`, so fingerprints are directly comparable)
and `zstd = "=0.13.3"` (`ebr-codec`, comparing production's codec path
against the same underlying library called directly).

## Style and safety

Workspace lints (`research/harness/Cargo.toml`'s `[workspace.lints]`) mirror
the production workspace: `unsafe_code = "forbid"`, `clippy::all = "warn"`.
No crate in this workspace contains an `unsafe` block. Where that constrained
a design choice (`ebr_common::measure`'s Windows peak-memory reading), the
module's own docs say so rather than quietly enabling `unsafe_code` or
silently mislabeling a snapshot as a peak.
