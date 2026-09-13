# code-crypto-a API notes: encrypted INDEXED container and encrypted range access

Scope: `crates/entrybound/src/crypto/container.rs` and `crates/entrybound/src/crypto/random.rs`
(branch dev at 9e44608). These notes list what a separate research harness crate
(`[dependencies] entrybound = { path = "../../crates/entrybound" }`) can call today without
modifying production code, and which internals would need a research-only feature flag.

## 1. Public entry points usable from a harness crate

All paths are re-exported from `entrybound::crypto` (see `crates/entrybound/src/crypto/mod.rs:31-47`).

### Creating encrypted archives
| Item | Signature (abridged) | Notes for experiments |
|---|---|---|
| `archive::plan_directory` | `(&Path, PackOptions) -> Result<Archive>` | Unkeyed (public Gear) plan. Input to `encrypt_archive`. |
| `crypto::encrypt_archive` | `(&Archive, EncryptedWriteOptions) -> Result<EncryptedArchive>` | Uses the plan's existing chunk boundaries (no keyed re-chunking). Fresh random AFK/archive ID. |
| `crypto::pack_directory_encrypted` | `(&Path, PackOptions, EncryptedWriteOptions) -> Result<EncryptedArchive>` | Keyed CDC (secret Gear or PHTE) plus `*-enc-v1` planner IDs. Use this for boundary-leakage and dedup-loss experiments. |
| `EncryptedWriteOptions` | pub fields `recipients`, `password`, `padding`, `boundary`, `include_index`, `embedded_signatures` | Vary `PaddingMode::{None,Bucketed,Maximum}`, `BoundaryMode::{SecretGearTable,PhteAes128}`, `include_index`. |
| `EncryptedArchive` | pub `bytes`, `archive`, `identities`, `public` | `public.segment_count`, `bytes.len()` give overhead and leakage figures directly. |
| `XWingIdentity::generate` / `from_seed([u8;32])` / `recipient()` | key generation | `from_seed` gives reproducible identities; encryption itself stays nondeterministic. |
| `XWingRecipient::from_bytes(&[u8], label)` / `fingerprint()` | recipient material | Build N-recipient envelopes for unlock-cost scaling. |

### Opening, verifying, inspecting
| Item | Notes |
|---|---|
| `crypto::open_encrypted(&[u8], EncryptedOpenOptions)` -> `ecf::OpenedArchive` | Full authenticated open; time/memory benchmarks. |
| `crypto::open_encrypted_authenticated` -> `AuthenticatedEncryptedArchive` | Adds embedded signatures, recipient directory, `AddressingBinding`. |
| `crypto::verify_encrypted` -> `ecf::VerificationReport` | Requires unlock. |
| `crypto::inspect_encrypted(&[u8], Option<Unlock>, CryptoPolicy)` -> `CryptoInspection` | `None` unlock = keyless public inspection (public digests and framing checks only). |
| `EncryptedOpenOptions::new(Option<Unlock>)` | pub fields `crypto_policy`, `resource_policy`, `decode_policy` are overridable. |
| `CryptoPolicy` | all 12 fields pub; set tight limits to probe refusal thresholds (`max_segments`, `max_identity_attempts`, Argon2 caps, `max_working_memory_bytes`, ...). |
| `Unlock::{Identity(&XWingIdentity), Password(&[u8])}` | |

### Mutation / key management
| Item | Notes |
|---|---|
| `crypto::embed_signature(bytes, options, SignatureRecord)` | AFK-preserving; PAYLOAD ciphertext reused. Pair with `current_bindings`, `sign_archive`, `SigningKey::from_seed`, `verify_signature`. |
| `crypto::add_recipient(bytes, options, &XWingRecipient)` | AFK-preserving; addressing binding becomes Stale. |
| `crypto::reencrypt_recipients(bytes, options, &[XWingRecipient])` | Fresh epoch, full replan and re-encryption; benchmark cost vs archive size. |
| `crypto::change_password(bytes, options, &[u8])` | Fresh epoch. |

### Encrypted range / remote access
| Item | Notes |
|---|---|
| `crypto::open_indexed_random_encrypted(source, RandomAccessPolicy, EncryptedOpenOptions)` -> `EncryptedRandomAccessArchive` | `.metadata()`, `.metadata_report()`, `.read_entry(&LogicalPath)` -> `ecf::RandomAccessRead { bytes, report }`. |
| `crypto::inspect_indexed_random_encrypted_public(source, RandomAccessPolicy, CryptoPolicy)` | Keyless; `access_trace` lists every range read. Entries with `purpose == SectionHeader` and `length == 64` after the segments section header are the per-segment header reads, so segment offsets/extents (public leakage profile) are recoverable without a private parser. |
| `random_access::{MemoryRandomReadSource::new, LocalFileRandomReadSource::open, HttpRangeSource::open}` | Sources; implement `RandomReadSource` yourself to inject latency, count requests, or simulate S3. |
| `random_access::RandomAccessPolicy` | all fields pub (`max_range_requests`, `max_individual_range_bytes`, `max_encrypted_segment_header_walk`, `coalesce_gap_bytes`, ...). |
| `ecf::RandomAccessVerificationReport` | `bytes_fetched`, `range_request_count`, `access_trace`, `index_status`, verification flags. |

### Chunk-boundary studies without the container
| Item | Notes |
|---|---|
| `chunker::chunk_ranges(&[u8], ChunkingParameters)` | Public Gear CDC. |
| `chunker::chunk_ranges_encrypted(&[u8], ChunkingParameters, &EncryptedBoundaryKey)` | `EncryptedBoundaryKey::SecretGearTable(Box<[u64;256]>)` and `PhteAes128 { polynomial: u128, aes_key: [u8;16] }` are publicly constructible, so a harness can use synthetic random keys to measure boundary leakage, dedup ratio, and throughput. Reproducing the exact AFK-derived key for a real archive is not possible publicly (see section 2). |

## 2. Experiments runnable today (no production changes)
1. Padding overhead and leakage: pack a corpus with each `PaddingMode`; compare `EncryptedArchive.bytes.len()` against `ecf::encode` output; record `public.segment_count` (approximately unique Chunk count plus control objects).
2. MAXIMUM padding cost: same as 1 on small corpora (expect about 65 MiB per unique Chunk).
3. Unlock latency vs recipient count (1..1024) and matching-stanza position; password unlock time with the fixed Argon2id m=256 MiB, t=3, p=4.
4. Range-access scaling: build archives with increasing Chunk counts, open with `MemoryRandomReadSource` or a latency-injecting source; record `range_request_count` at open and per `read_entry`, and find the `max_range_requests` (default 100,000) failure threshold.
5. Default-policy refusals: MAXIMUM-padded archive or >64 MiB Manifest opened via range reader with default `RandomAccessPolicy`.
6. Index-less fallback: `include_index: false`; measure bytes fetched on first `read_entry` and the 1 GiB retained-bytes failure point.
7. Mutation cost: time and peak memory for `add_recipient` (reuse) vs `reencrypt_recipients` / `change_password` (full replan) on multi-GB inputs.
8. Signature binding status after each mutation via `verify_signature` (content/physical/addressing).
9. Keyed-CDC dedup loss: `pack_directory_encrypted` vs `plan_directory` on versioned corpora; PHTE vs secret Gear throughput using `chunk_ranges_encrypted` with synthetic keys.
10. Tamper corpus (bit flips, truncation, segment reorder/splice between two archives) on `encrypt_archive` output, comparing `open_encrypted` and `open_indexed_random_encrypted` reason codes.

## 3. Internal-only functionality that would need a research-only feature flag
These are `pub(super)`, `pub(crate)`, or private, and are required for hostile-sender /
conformance-divergence experiments (crafting authenticated but malformed archives):

| Internal item | Location | Needed for |
|---|---|---|
| `encrypt_with_file_key` (fixed AFK and archive ID) | container.rs:200-211 (pub(super)) | Deterministic encrypted fixtures; reproducible leakage baselines. |
| `encrypt_with_material` (custom stanza list, reusable segments) | container.rs:221-520 (private) | Unknown stanza types, crafted envelopes, splicing across AFK-sharing siblings. |
| `KeyHierarchy::derive`, `boundary_key` | mod.rs:320-404 (private) | Reproduce the exact keyed chunker of a real archive; key-schedule vectors. |
| `build_segment`, `segment_key`, `record_ad`, `data_nonce`, `end_nonce`, `pad_private`, `padded_len`, `bucket_lengths` | container.rs:1926-2376 | Craft segments with multiple objects, objects spanning segments, reordered CONTROL objects, PAYLOAD at ordinal 0, oversize `total_len` fragments. |
| `parse_public`, `scan_public_segments` | container.rs:1043-1199 | Keyless segment table without re-implementing the parser. |
| `decrypt_segments`, `ObjectCollector` | container.rs:1292-1855 | Differential full-reader tests on crafted object streams. |
| `walk_segments`, `decrypt_segment`, `EncryptedRandomAccessArchive.{segments,index}` | random.rs:1203-1438, 136-152 | Range-reader differential tests; index-fallback edge cases. |
| `ecf::prepare_encrypted_plain_parts`, `open_encrypted_plain_parts`, `private_descriptor_declaration`, `prepare_legacy_encrypted_plain_parts` (cfg(test)) | ecf.rs:85-90 | Build/modify Descriptor v1/v2 plain parts (under-declared budgets). |
| `archive::plan_directory_encrypted`, `replan_archive_encrypted` | archive/filesystem.rs:114-249 (pub(crate)) | Keyed planning of an in-memory EAM without the filesystem. |
| `crypto::wire::*` (record kinds, `private_object`, `sequence_container`, `encode_private_fragment`, `t1`) | wire.rs (pub(crate)) | Independent-writer conformance vectors. |

Suggested shape: a non-default cargo feature (for example `research-internals`) exposing a
`entrybound::crypto::research` module that re-exports the items above behind `#[cfg(feature = ...)]`,
never compiled into release builds, so production APIs keep the no-deterministic-encryption guarantee
(docs/crypto-threat-model-v1.md L150-155; docs/crypto-suite-v1.md CRYPTO-035).
