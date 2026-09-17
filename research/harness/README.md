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
  harness-review-round1.md  adversarial measurement-validity review, findings and dispositions
  lock_parity.py          checks this workspace resolves entrybound's dependencies like production
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
    ebr-netem/     range-serving origin, RTT/bandwidth/loss/cache netem proxy, and calibration binary
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

`ebr-common` does not depend on `entrybound`; `ebr-netem` uses it only as a
dev-dependency for its origin contract test.
`ebr-pack`/`ebr-chunk`/`ebr-access` only call entrybound's *public* API
today even though the feature is enabled (so future measurement code in
those crates can reach `entrybound::research` without a `Cargo.toml`
change); `ebr-codec`/`ebr-planner` already use
`entrybound::research::{codec,planner}` directly. See each crate's own
README for specifics.

### Two ways a harness build differs from the shipped CLI (review R1-06)

1. **Its own `Cargo.lock`.** This workspace resolves entrybound's
   transitive dependencies independently of the production workspace. At
   review round 1 it had drifted on 28 crates, including `zstd-sys`,
   `zstd-safe`, and `cc` (which builds libzstd), so harness "production"
   measurements did not run the dependency build `ebound` ships, and the
   research-internals identity proof (run in the production workspace) did
   not cover it. The lock was realigned, and
   `python research/harness/lock_parity.py` must report `RESULT PASS`
   before any measurement run and after any `Cargo.lock` change in either
   workspace (`--commands` prints the `cargo update --precise` fixes).
   Production's `cargo test` resolve also enables `flate2`'s `zlib-rs`
   backend through a dev-dependency; release builds of `ebound` and this
   harness both use `miniz_oxide`.
2. **`research-internals` is enabled.** The feature is proven additive for
   archive bytes, but exposing private functions through public wrappers
   can change code generation (inlining) of the functions production calls.
   Every `ebr-pack` row carries a `build_note` saying so. **Decision-grade
   timing comparisons against incumbents must use `ebound` built from the
   production workspace without the feature**, or first show timing parity
   between the two builds on the same inputs.

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

## Integration validation (2026-09-17, Phase B2 integration-validation agent)

Every check below was re-run this session, after all seven crates were
complete, with no harness-crate fix needed (the workspace already built and
tested clean end to end on both hosts):

- **Whole-workspace build + test, both hosts, pinned toolchain 1.98.1:**
  ```sh
  wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound/research/harness && CARGO_TARGET_DIR=/root/eb-research/target/harness /root/.cargo/bin/cargo +1.98.1 build --workspace'
  wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound/research/harness && CARGO_TARGET_DIR=/root/eb-research/target/harness /root/.cargo/bin/cargo +1.98.1 test --workspace'
  ```
  249 tests passed, 0 failed, on WSL (`ebr-access` 17, `ebr-chunk` 58,
  `ebr-codec` 27 + 1 integration test, `ebr-common` 26, `ebr-netem` 56 + 12
  `origin_contract` + 8 `proxy_netem`, `ebr-pack` 29, `ebr-planner` 15).
  `build --workspace --release` also verified on both WSL and Windows
  (`D:/eb-research/target/harness`, `%USERPROFILE%\.cargo\bin\cargo.exe
  +1.98.1`); Windows `test --workspace` (debug) was also run and passed.
- **`research-internals` byte-identity re-proof** (both hosts, re-running
  the already-committed scripts unchanged):
  ```sh
  wsl.exe -d Ubuntu -- bash research/harness/internals-identity-check.sh
  pwsh -NoProfile -File research/harness/internals-identity-check.ps1
  ```
  Both hosts: **PASS**, pack output SHA-256 identical with and without
  `research-internals` for all 8 profile x layout cells, on both hosts (see
  `internals-identity-check.md`, regenerated by this run). The pre-existing
  `cross_file_compression::small_cohort_stays_independent_and_v2_v3_remain_readable`
  WSL-only failure and pre-existing clippy `-D warnings` findings are
  unchanged and still `PRE-EXISTING` (identical with and without the
  feature).
- **Cross-host determinism smoke (`EXP-DET-SMOKE`)**: `ebr-pack` (release)
  packed the same checked-in-generator tree
  (`internals_identity_tools.py tree`) under all 4 profiles x 2 layouts on
  WSL x86_64 and Windows x86_64; native identities (LAI/AUX/PCR/PCI) came
  from `ebound inspect --json` (production CLI). Result: **PCR is
  byte-identical cross-host for all 8 cells** (content-defined chunking is
  platform-independent here); **LAI/AUX/artifact bytes differ cross-host by
  design**, matching the already-known Finding F-0001 (Windows requires an
  extra `0x10000` platform-security feature bit; entries carry
  `windows_file_attributes`/`windows_creation_time` on Windows vs.
  `posix.{mode,uid,gid}` on WSL). See `research/raw/EXP-DET-SMOKE/README.md`
  for the full method and `research/normalized/EXP-DET-SMOKE/summary.csv`
  for the per-cell comparison. linux/arm64 (emulated via Docker) is
  **DEFERRED**: Docker Desktop is unavailable on this host.
- **Wired into the `ebr` runner** (`research/experiments/EXP-HARNESS-SMOKE-*`):
  one spec per crate with a binary (`PACK`, `CHUNK`, `CODEC`, `PLANNER`,
  `ACCESS`, `NETEM`), each running that crate's binary once on a tiny
  `text-v1`-generated item, `timing: false`. All six completed
  `validate` -> `plan` -> `run` -> `verify-raw` -> `normalize` -> `report`
  with zero failed/invalid samples:
  ```sh
  wsl.exe -d Ubuntu -- bash -lc '
    export PYTHONPATH=/mnt/d/Projects/entrybound/entrybound/research/tools PYTHONDONTWRITEBYTECODE=1
    cd /mnt/d/Projects/entrybound/entrybound
    for exp in PACK CHUNK CODEC PLANNER ACCESS NETEM; do
      SPEC=research/experiments/EXP-HARNESS-SMOKE-$exp/spec.yaml
      /root/eb-research/venv/bin/python -m ebr validate --spec "$SPEC"
      /root/eb-research/venv/bin/python -m ebr run --spec "$SPEC"
      /root/eb-research/venv/bin/python -m ebr verify-raw --spec "$SPEC" --refingerprint
      /root/eb-research/venv/bin/python -m ebr normalize --spec "$SPEC"
      /root/eb-research/venv/bin/python -m ebr report --spec "$SPEC"
    done'
  ```

## Harness review round 1 (2026-09-17)

`harness-review-round1.md` records an adversarial measurement-validity
review of this workspace: 14 HIGH/MEDIUM harness defects fixed (held-out
guard never called by any binary; cumulative random-access byte counters
reported per read; pack/probe/codec memory and timing dominated by harness
work; STREAM staging spill missing from peak scratch; a drifted
`Cargo.lock`; single-chunk planner regret with mixed cost units; byte
accounting without an independent cross-check; mis-calibrated or
unfaithful CDC baselines; unfair or unmeasured codec configurations;
netem loss and bandwidth artifacts), plus the use constraints that remain.
Read its "Use constraints" section before designing a C-exec experiment.

Every binary refuses any input path under a held-out root before reading
it (`ebr_common::heldout::assert_inputs_not_heldout`), independently of the
`ebr` runner's own spec-level guard.

## Known limitations

- **`ebr-pack --stream-window auto`** is supported since review round 1
  (earlier revisions only accepted a fixed `Ceiling(n)`; `EXP-DET-SMOKE` used
  `100000` as a workaround).
- **`ebr-origin`/`ebr-netem-proxy` serve forever; they are not one-shot `ebr`
  spec commands on their own.** Every other harness binary prints one JSON
  row and exits, which is what an `ebr` spec's `command` expects. The two
  network-emulation servers instead bind a port and run until killed.
  `research/experiments/EXP-HARNESS-SMOKE-NETEM/origin_probe.sh` is the
  minimal wiring this session added: start `ebr-origin` in the background,
  poll its log for the bound address, make one real HTTP GET through `curl`,
  kill it, and print one summary JSON line (schema `ebr.harness.smoke.v1`,
  not `ebr.harness.raw.v1`, since it is a shell wrapper's output, not a
  `ResultWriter` row) for the spec's `stdout_json` metrics to read. A real
  `EXP-NETEM-*` experiment measuring the proxy's RTT/bandwidth/loss/cache
  behavior needs a similarly-shaped wrapper (or a small purpose-built Rust
  driver, the way `ebr-netem-calibrate` already is for its own fixed
  RTT/bandwidth sweep -- see `crates/ebr-netem/README.md`); it is not
  something the generic `ebr` spec `command` template alone can express for
  a long-running server.
- **linux/arm64 determinism (emulated via Docker) is deferred**, not run,
  because Docker Desktop is unavailable on this host (stale AF_UNIX socket
  error 1920 at startup; see `research/PROGRESS.md`'s 2026-09-16 storage
  incident). `EXP-DET-SMOKE` only covers WSL x86_64 and Windows x86_64.
- **A pre-existing, unrelated line-ending drift.** `git status` shows eight
  `crates/entrybound/*` files as modified with an all-CRLF-vs-LF diff (every
  line content-identical, only the line terminator changed) that predates
  this session's work (it was already present in the previous
  `internals-identity-check.md` run recorded at `2026-09-17T03:02Z`,
  before this agent started). It does not affect any build, test, or
  identity-check result recorded here or previously, and this agent did not
  edit `crates/` to fix it (only the Internals agent may change production
  files, and this is a whitespace-only condition on disk, not a diff this
  agent introduced).
