# ebr-pack

End-to-end archive measurements using entrybound's **public** production API
(`research-internals` additionally enabled so the byte-accounting walk can
reach the one `entrybound::research::ecf` item it needs -- see below):
`capture -> chunk -> plan -> encode -> write -> open -> verify`, plus a
standalone `open -> verify -> extract` and a standalone
`open -> exact byte accounting` over any already-encoded archive file.

Four binaries share this crate's library code:

| Binary | What it does |
|---|---|
| `ebr-pack` | Packs a directory (INDEXED, STREAM, or encrypted-INDEXED), with per-phase wall-clock, exact byte accounting, logical/codec counts, an optional deterministic-repeat check, and peak memory. Writes the encoded archive to disk. |
| `ebr-unpack` | Full verify + extract of an already-encoded archive file into a scratch directory, with the scratch directory's peak on-disk byte usage sampled while extraction runs. |
| `ebr-verify` | Verifies an already-encoded archive file and reports every `VerificationReport` guarantee individually, timed. |
| `ebr-inspect-bytes` | Exact section-level byte accounting plus logical/codec counts for an already-encoded archive file -- the same accounting `ebr-pack` reports, but standalone, so it can double-check an archive `ebr-pack` produced earlier, or one this crate did not produce at all. |

`ebr-unpack`/`ebr-verify`/`ebr-inspect-bytes` never sniff a container's
layout or encryption from its bytes -- each takes `--layout`/`--encrypted`
explicitly, because the caller (typically a `ebr-pack` invocation earlier in
the same experiment) already knows those facts.

## What `ebr-pack` measures

Over one input directory (a corpus item's materialized path, or any other
directory), `--profile`/`--layout`/`--stream-window`/`--encrypt` selected:

1. **capture** -- `ebr_common::walk::read_tree_in_memory` reads every regular
   file's bytes (bounded at 8 GiB; a larger item needs a streaming approach
   this crate does not implement -- see "Scope" below).
2. **chunking** -- a *separate, additional* pass over each captured file's
   bytes through the same public chunker production uses
   (`entrybound::chunker::{select_parameters, chunk_ranges}`), timed on its
   own. This exists purely so chunking has its own visible timing figure;
   entrybound's single public directory-to-archive entry point
   (`entrybound::archive::plan_directory`) chunks internally too, and there
   is no public hook to time that internal chunking separately from
   planning. **`chunking_seconds` and `planning_seconds` are not two
   components of one total** -- summing every phase double-counts chunking
   by design; see `src/pack.rs`'s module docs for the full reasoning.
3. **planning** -- `entrybound::archive::plan_directory` (captures the
   filesystem again, chunks, and selects codecs/transforms/dictionaries/
   groups, all in one call).
4. **encoding** -- `entrybound::ecf::encode` (INDEXED) or `encode_stream`
   (STREAM), or `entrybound::crypto::pack_directory_encrypted`
   (`--encrypt true`, INDEXED only -- see "Scope").
5. **writing** -- `std::fs::write` of the encoded bytes to `--archive-out`
   (or a generated path under the OS temp directory, reported as
   `archive_path` in the row).

Then the encoded bytes are opened and verified
(`entrybound::ecf::open`/`open_stream`/
`entrybound::crypto::open_encrypted_authenticated`), and:

- **Exact section-level byte accounting** (`ebr_pack::bytes`): for a
  plaintext INDEXED archive, this walks `entrybound::ecf::
  open_indexed_random`'s own authenticated section directory
  (`RandomAccessMetadata::section_directory`) for every top-level section's
  exact on-disk extent (preamble, Descriptor, TransformPlans, Dictionaries,
  ChunkGroups, ReconstructionData, ReconstructionRegions, ManifestRecords,
  Fidelity -- this crate's "metadata records" -- Index, footer), and
  additionally walks *inside* the ChunkData section frame by frame
  (`entrybound::research::ecf::{chunk_frame_header_len,
  parse_chunk_frame_header}`, the one `research-internals` item this crate
  uses) to split every Chunk frame into its header and its stored payload,
  attributing each payload's bytes to the codec and transform pipeline its
  plan names. The accounted total is cross-checked against the file's exact
  size and **fails loudly** (a hard `Err`, never a silently wrong number) on
  any mismatch. STREAM and encrypted archives get a coarser, single-lump
  accounting instead -- production exposes no equivalent public per-item/
  per-segment directory for those -- still exactly cross-checked against the
  file size; see `src/bytes.rs`'s module docs for exactly why.
- **Logical/codec counts** (`ebr_pack::counts`): entry count, total logical
  bytes, planner id, chunker id, per-codec stored/logical bytes and Chunk
  count, unique-Chunk/logical-Chunk-reference/dedup statistics, dictionary
  and ChunkGroup counts, reconstruction-transform counts -- a `Serialize`
  mirror of `entrybound::archive::inspect`'s `ArchiveInspection` (which does
  not itself derive `Serialize`).
- **Peak memory** -- `ebr_common::measure::peak_memory` (true OS-reported
  peak on Linux; current resident memory on Windows, labeled as such -- see
  that module's docs), sampled once after the whole pipeline completes.

`--deterministic-check N` (`N >= 2`) repeats the whole capture-through-
encode pipeline `N` times over the same input/profile/layout and SHA-256-
hashes each run's encoded bytes. Chunking, planning, and encoding are all
meant to be deterministic given the same input, so every run should produce
byte-identical output (down to every identity/digest the format embeds).
`ebr-pack` treats a mismatch as a hard failure (nonzero exit), since a
nondeterministic pack is a real problem for this research goal, not a
tolerable variance.

## `--encrypt`: generated test recipient and password

`--encrypt true` generates one fresh X-Wing recipient/identity pair
(`entrybound::crypto::XWingIdentity::generate`) and one fresh 32-byte test
password (`ebr_pack::encrypt::generate_test_password`, a SHA-256-based
entropy source good enough for a throwaway test secret, explicitly not a
reviewed RNG for anything real). Production refuses to encrypt with **both**
a recipient and a password set at once (`EB_CRYPTO_RECIPIENT_POLICY_INVALID:
encrypted creation requires recipients or one password, never both`), so
only the password actually unlocks the produced archive; the recipient is
still generated and its fingerprint reported (`credentials.
recipient_fingerprint_hex`) for completeness and any future identity-unlock
measurement. The password itself is never printed in the JSON row -- only
its length -- but is written to a sidecar file next to the archive
(`<archive_path>.testpassword`, best-effort, a warning on failure rather
than aborting the run) so a **separate** `ebr-verify`/`ebr-unpack`/
`ebr-inspect-bytes` process invoked later in the same experiment can unlock
the same archive; there is no other channel to share it, since `ebr-pack`
never keeps the password past its own process exit. That sidecar is plainly
named and documented here, never anything a caller is expected to protect
(it is fresh, ephemeral, throwaway test material for one measurement run,
not a credential to anything real).

`--encrypt true --layout stream` is refused (`PackError::Unsupported`):
entrybound's only public encrypted-pack entry point
(`pack_directory_encrypted`) always produces an INDEXED-shaped encrypted
container.

## Flags

### `ebr-pack`

```text
ebr-pack --item-path <dir> --experiment-id <id> --env-name <name>
         [--profile fast|balanced|dense|extreme] [--layout indexed|stream]
         [--stream-window <u64>] [--encrypt true|false]
         [--deterministic-check <N>] [--archive-out <path>]
         [--run-id <id>] [--out <path>]
```

- `--item-path`: a directory to pack.
- `--profile`: default `balanced`.
- `--layout`: default `indexed`.
- `--stream-window`: STREAM only; `StreamWindow::Ceiling(n)` (default
  `Ceiling(0)`, forbidding cross-object historical Chunk dependencies).
- `--encrypt`: default `false`. See above.
- `--deterministic-check`: omit for a single pack; `N >= 2` repeats and
  hash-compares (see above).
- `--archive-out`: where the encoded archive is written; default a
  generated path under the OS temp directory (reported as `archive_path`).
- `--experiment-id`/`--env-name`/`--run-id`/`--out`: shared flags, see the
  top-level `research/harness/README.md`.

### `ebr-unpack`

```text
ebr-unpack --archive-path <file> --experiment-id <id> --env-name <name>
           [--layout indexed|stream] [--encrypted true|false]
           [--unlock-password-file <path>] [--scratch-dir <dir>]
           [--keep-scratch true|false] [--run-id <id>] [--out <path>]
```

- `--archive-path`: an already-encoded archive file (typically one
  `ebr-pack --archive-out` produced).
- `--layout`/`--encrypted`: default `indexed`/`false`; must match the
  archive.
- `--unlock-password-file`: required when `--encrypted true` -- the
  `<archive_path>.testpassword` sidecar `ebr-pack` wrote.
- `--scratch-dir`: default a generated path under the OS temp directory.
- `--keep-scratch`: default `false` (the scratch directory is removed after
  measuring; set `true` to inspect the extracted tree afterward).

### `ebr-verify`

```text
ebr-verify --archive-path <file> --experiment-id <id> --env-name <name>
           [--layout indexed|stream] [--encrypted true|false]
           [--unlock-password-file <path>] [--run-id <id>] [--out <path>]
```

### `ebr-inspect-bytes`

```text
ebr-inspect-bytes --archive-path <file> --experiment-id <id> --env-name <name>
                   [--layout indexed|stream] [--encrypted true|false]
                   [--unlock-password-file <path>] [--run-id <id>] [--out <path>]
```

## Example `ebr` runner spec command templates

See the top-level `research/harness/README.md` for the full placeholder
list and output-schema conventions (`--out` omitted so the binary prints its
one JSON line to stdout for the runner's `stdout_json` metric source).

Pack a corpus item, INDEXED, balanced profile:

```yaml
command: >-
  /root/eb-research/target/harness/release/ebr-pack
  --item-path {item_path} --experiment-id EXP-PACK-001 --env-name wsl-ubuntu
  --profile {candidate} --layout indexed
  --archive-out /root/eb-research/scratch/{item_id}-{rep}.ecf
```

Pack encrypted, then verify from a separate spec step (the second step's
`command` reads the first step's `archive_path`/sidecar by the same fixed
scratch naming convention above rather than by piping JSON between specs):

```yaml
command: >-
  /root/eb-research/target/harness/release/ebr-pack
  --item-path {item_path} --experiment-id EXP-PACK-ENC-001 --env-name wsl-ubuntu
  --profile {candidate} --layout indexed --encrypt true
  --archive-out /root/eb-research/scratch/{item_id}-{rep}.ecf
```

```yaml
command: >-
  /root/eb-research/target/harness/release/ebr-verify
  --archive-path /root/eb-research/scratch/{item_id}-{rep}.ecf
  --experiment-id EXP-VERIFY-ENC-001 --env-name wsl-ubuntu
  --layout indexed --encrypted true
  --unlock-password-file /root/eb-research/scratch/{item_id}-{rep}.ecf.testpassword
```

Unpack with peak scratch-directory measurement, STREAM layout:

```yaml
command: >-
  /root/eb-research/target/harness/release/ebr-unpack
  --archive-path /root/eb-research/scratch/{item_id}-{rep}.ecfs
  --experiment-id EXP-UNPACK-STREAM-001 --env-name wsl-ubuntu --layout stream
```

Inspect an archive's exact byte accounting, independent of any pack step
(e.g. to spot-check an archive produced outside this harness):

```yaml
command: >-
  /root/eb-research/target/harness/release/ebr-inspect-bytes
  --archive-path {item_path} --experiment-id EXP-INSPECT-001 --env-name wsl-ubuntu
  --layout indexed
```

## Module map

- `src/pack.rs` -- the phase-timed pack pipeline and the deterministic-repeat
  check (`pack_once`, `check_determinism`).
- `src/bytes.rs` -- the exact section-level byte-accounting walk
  (`indexed_byte_accounting`, `stream_byte_accounting`,
  `encrypted_byte_accounting`).
- `src/counts.rs` -- the `Serialize` mirror of `ArchiveInspection`
  (`from_inspection`, `from_bytes`).
- `src/encrypt.rs` -- fresh test recipient/password generation
  (`TestCredentials`).
- `src/scratch.rs` -- the peak-scratch-directory-size sampler used by
  `ebr-unpack` (`ScratchSampler`).
- `src/unpack.rs` / `src/verify.rs` -- the standalone measurements
  `ebr-unpack`/`ebr-verify` report.
- `src/main.rs` -- the `ebr-pack` binary.
- `src/bin/ebr-unpack.rs`, `src/bin/ebr-verify.rs`,
  `src/bin/ebr-inspect-bytes.rs` -- the other three binaries (Cargo
  auto-discovers `src/bin/*.rs`; no explicit `[[bin]]` entries needed beyond
  the one this crate started with for `ebr-pack` itself).

## Scope (first cut, honestly bounded)

- **Input size**: `capture` reads the whole input tree into memory, bounded
  at 8 GiB. A validation-scale item too large for that needs a streaming
  capture path (`entrybound::archive::pack_directory_stream` is public and
  suited to it) this crate does not implement yet.
- **`--encrypt` + `--layout stream`**: refused (see above).
- **STREAM and encrypted byte accounting**: coarser than INDEXED's (a single
  lump body/segment figure, not a per-section/per-Chunk-frame breakdown) --
  see `src/bytes.rs`'s module docs for exactly which public APIs do and do
  not exist for each layout, and why this crate does not hand-parse a
  private wire format from outside the crate to close that gap.
- **Repack/diff** (`entrybound::archive::{prepare_repack, archive_diff}`)
  are also public API this crate does not measure yet.

## Testing

```sh
cargo +1.98.1 test -p ebr-pack
```

Every test builds its own small scratch input tree and/or encodes its own
small archive under `std::env::temp_dir()` and removes it afterward;
nothing in this crate reads `research/corpus` or `/root/eb-research` unless
a caller points a `--item-path`/`--archive-path` there. Verified on both
WSL Ubuntu (`/root/.cargo/bin/cargo +1.98.1`, `CARGO_TARGET_DIR=/root/eb-
research/target/b2-pack`) and the Windows host
(`%USERPROFILE%\.cargo\bin\cargo.exe +1.98.1`,
`CARGO_TARGET_DIR=D:/eb-research/target/b2-pack-win`): 29 tests pass on
both, `cargo clippy --all-targets` is clean on both.
