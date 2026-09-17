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
directory), `--profile`/`--layout`/`--stream-window`/`--encrypt` selected.
Harness review round 1 (`research/harness/harness-review-round1.md`,
findings R1-03 and R1-08) reworked what is timed and how bytes are checked;
the measured region now contains production work only:

1. **planning** -- `entrybound::archive::plan_directory` (captures the
   filesystem, chunks, and selects codecs/transforms/dictionaries/groups,
   all in one call, exactly as `ebound pack` does).
2. **encoding** -- `entrybound::ecf::encode` (INDEXED) or `encode_stream`
   (STREAM), or the whole `entrybound::crypto::pack_directory_encrypted`
   call (`--encrypt true`, INDEXED only; `planning_seconds` is then `0.0`).
3. **`pack_memory`** -- a phase-scoped memory reading
   (`ebr_common::measure::ScopedMemory`): on Linux the kernel high-water
   mark is reset immediately before planning and read immediately after
   encoding, so it is the pack's own peak, not the process-lifetime peak.
   On Windows it is a current-resident snapshot, labeled `scoped: false`.
4. **writing** -- `std::fs::write` of the encoded bytes to `--archive-out`
   (`writing_seconds`).

Earlier revisions first read the whole input tree into memory (up to 8 GiB)
and reported that read as a "capture" phase, kept that copy resident while
production packed, and measured peak memory once at the very end (after
verification, byte accounting, and any determinism repeats). None of those
harness costs are in the production figures any more.

After the measured region, and never timed as a production phase:

- the encoded bytes are opened and verified
  (`entrybound::ecf::open`/`open_stream`/
  `entrybound::crypto::open_encrypted_authenticated`) and inspected for
  counts;
- the input tree's **metadata** is walked for `input_logical_bytes`/
  `input_file_count` (`input_scan_seconds`; no file contents are read);
- with `--chunking-pass true` only, a harness-only per-file chunker pass runs
  (`chunking_pass_seconds`). It approximates, and must not be summed with,
  the chunking `plan_directory` already did.

Then:

- **Exact section-level byte accounting** (`ebr_pack::bytes`): for a
  plaintext INDEXED archive, this walks `entrybound::ecf::
  open_indexed_random`'s own authenticated section directory for every
  top-level section's exact on-disk extent, and additionally walks *inside*
  the ChunkData section frame by frame (`entrybound::research::ecf::
  {chunk_frame_header_len, parse_chunk_frame_header}`) to split every Chunk
  frame into its header and its stored payload, attributed to codec and
  transform. Two sums must equal the file size exactly or the run fails:
  the section extents, and the **named buckets** a consumer reads
  (`descriptor_bytes` ... `chunk_data_section_header_bytes`,
  `chunk_frame_header_bytes`, `chunk_payload_bytes`, `other_section_bytes`).
  An **independent cross-check** (`byte_accounting_cross_check`) then
  compares the frame walk's payload bytes and frame count against
  `entrybound::archive::inspect`'s per-codec `stored_bytes`/`chunk_count`,
  which production derives from the Index and Chunk table rather than from
  ChunkData; a mismatch fails the run. It is marked `applicable: false` for
  archives with whole-object ReconstructionRegions, whose representations
  live outside ChunkData. STREAM and encrypted archives get a coarser,
  single-lump accounting -- production exposes no public per-item/
  per-segment directory for those -- still exactly checked against the file
  size.
- **Logical/codec counts** (`ebr_pack::counts`): entry count, total logical
  bytes, planner id, chunker id, per-codec stored/logical bytes and Chunk
  count, unique-Chunk/logical-Chunk-reference/dedup statistics, dictionary
  and ChunkGroup counts, reconstruction-transform counts, and the
  whole-object region count.
- **`process_lifetime_peak_rss_bytes`** -- the whole process's peak at the
  end of the run, kept only as a diagnostic.
- **`build_note`** -- every row says it came from a harness build
  (entrybound with `research-internals`, `research/harness/Cargo.lock`);
  timing from this binary is not a substitute for the production CLI build.

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
         [--stream-window auto|<u64>] [--encrypt true|false] [--chunking-pass true|false]
         [--deterministic-check <N>] [--archive-out <path>]
         [--run-id <id>] [--out <path>]
```

- `--item-path`: a directory to pack.
- `--profile`: default `balanced`.
- `--layout`: default `indexed`.
- `--stream-window`: STREAM only; `auto` (`StreamWindow::Auto`) or a number
  (`StreamWindow::Ceiling(n)`). Default `Ceiling(0)`, forbidding cross-object
  historical Chunk dependencies, like `ebound pack --layout stream`.
- `--chunking-pass`: default `false`; see above.
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

`ebr-unpack` times the production entry point only (`unpack_seconds`:
reading the archive plus `archive::unpack`, `archive::unpack_stream`, or
`open_encrypted_authenticated` + `unpack_opened`) with a scoped
`unpack_memory`, while `src/scratch.rs` samples the destination directory
**and** production's STREAM staging spill files
(`entrybound-stage-<pid>-*.tmp` in the OS temp directory) --
`scratch.peak_total_bytes` is the per-sample sum. For STREAM,
`stream_staging` carries production's own `StreamReport` staging counters
next to the sampled peak. A separate verification pass runs afterwards for
`verified` (`verify_pass_seconds`, not additive with `unpack_seconds`).
Every binary refuses an input path under a held-out root before reading it.

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

- **Input size**: the harness no longer reads the input into memory, but
  production `plan_directory` itself holds every Chunk's plaintext in the
  planned `Archive`, and the encoded archive is held in memory before it is
  written; very large items need the STREAM layout's streaming sink
  (`entrybound::archive::pack_directory_stream`), which this crate does not
  drive yet.
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
both, `cargo clippy --all-targets` is clean on both. After harness review
round 1 the crate has 33 tests (named-bucket balance, independent
cross-check with a tampered inspection, and a forced STREAM staging spill
that the scratch sampler must observe).
