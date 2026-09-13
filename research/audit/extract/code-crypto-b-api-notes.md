# code-crypto-b: research-harness API notes

Slice: `crates/entrybound/src/crypto/mod.rs`, `crypto/signature.rs`, `crypto/timestamp.rs`,
`crypto/wire.rs` (dev @ 9e44608). Paths are relative to `crates/entrybound/src` unless stated.
`crypto` and `chunker` are public modules (`lib.rs:11`, `lib.rs:9`); `canonical` is private, so
`RecordBuilder`/`decode_record` (`canonical.rs:44`, `:285`) are not reachable from a harness.

## 1. Public entry points usable from a harness crate (no production changes)

### Archive creation and opening (re-exported from `crypto/container.rs`, `crypto/random.rs`)
| Item | Signature / location | Research use |
|---|---|---|
| `crypto::pack_directory_encrypted` | `fn(&Path, archive::PackOptions, EncryptedWriteOptions) -> Result<EncryptedArchive>` (mod.rs:408) | Only creation path that applies AFK-keyed CDC. Measure encrypted size vs plaintext per `CompressionProfile`, padding mode, boundary mode; PHTE vs Gear wall time. |
| `crypto::encrypt_archive` | `fn(&eam::Archive, EncryptedWriteOptions) -> Result<EncryptedArchive>` (container.rs:186) | Encrypts an existing plan without re-chunking. Useful for isolating padding/segment overhead from chunking effects; also demonstrates the public-boundary gap (record code-crypto-b-0035). |
| `crypto::EncryptedWriteOptions` | `{ recipients: &[XWingRecipient], password: Option<&[u8]>, padding: PaddingMode, boundary: BoundaryMode, include_index: bool, embedded_signatures: &[SignatureRecord] }` (container.rs:48-68) | Padding-overhead and privacy experiments (None/Bucketed/Maximum), Index on/off, recipient-count scaling (1..1024). No KDF-cost field. |
| `crypto::EncryptedArchive` | `{ bytes, archive, identities, public }` (container.rs:120-125) | `bytes.len()` for overhead; `identities` (LAI/PCR/AUX/PCI) for linkage experiments. |
| `crypto::open_encrypted`, `open_encrypted_authenticated`, `verify_encrypted` | container.rs:605, 611, 653 | Unlock latency (hybrid vs password), tamper/corruption corpora, `CryptoPolicy` refusal thresholds. `AuthenticatedEncryptedArchive { opened, embedded_signatures, recipient_directory, addressing }`. |
| `crypto::EncryptedOpenOptions` | `new(Option<Unlock>)`; pub fields `unlock`, `crypto_policy`, `resource_policy`, `decode_policy` (container.rs:70-88) | Lower caps to probe each `CryptoPolicy` limit. |
| `crypto::inspect_encrypted` | `fn(&[u8], Option<Unlock>, CryptoPolicy) -> Result<CryptoInspection>` (container.rs:999) | What an unauthenticated observer learns (public inspection) vs authenticated Descriptor-v2 declarations. |
| `crypto::add_recipient`, `reencrypt_recipients`, `change_password`, `embed_signature` | container.rs:762, 830, 867, 710 | Mutation cost (PAYLOAD reuse on add vs full rotation), PCR/PHYSICAL staleness after keyed rechunk, byte-diff size of recipient edits. |
| `crypto::open_indexed_random_encrypted`, `inspect_indexed_random_encrypted_public` | random.rs:155, 164; `EncryptedRandomAccessArchive::{metadata, metadata_report, read_entry}` (random.rs:542-551) | Encrypted random-access cost and fetch patterns (owned by the access slice; listed for completeness). |
| `crypto::CryptoPolicy` | pub fields + `Default` (mod.rs:131-165) | DoS experiments: Argon2 memory/passes/parallelism ceilings, stanza count/bytes, segments, record sizes, identity attempts. |
| `crypto::PaddingMode`, `crypto::BoundaryMode` | mod.rs:74-122 (`TryFrom<u8>`) | Enumerate modes; reason-code checks for unknown values. |
| Feature constants | `FEATURE_ENCRYPTED_INDEXED_V1` .. `FEATURE_PRIVATE_RESOURCE_DECLARATION_V1`, `CRYPTO_FEATURES` (mod.rs:49-65) | Preamble mutation corpora (feature bitmap at bytes 16..24 of the preamble, see tests/crypto_v1.rs:581). |

### Keys and unlock material
| Item | Location | Research use |
|---|---|---|
| `crypto::XWingIdentity::{generate, from_seed, recipient, encode_file, read_file}` | mod.rs:244-289 | Deterministic identities via `from_seed` for reproducible benchmarks; cross-implementation X-Wing interop (encapsulate elsewhere, decapsulate via an archive stanza, or feed seeds to an independent draft-10 implementation). |
| `crypto::XWingRecipient::{from_bytes, bytes, label, fingerprint, encode_file, read_file}` | mod.rs:184-231 | Malformed-public-key acceptance tests (ML-KEM modulus checks); label length limit. |
| `crypto::Unlock::{Identity, Password}` | mod.rs:295-298 | Wrong-credential timing experiments (all collapse to `EB_CRYPTO_NO_MATCHING_RECIPIENT`). |
| `crypto::SigningKey::{generate, from_seed, public_key, encode_file, read_file}` | signature.rs:47-106 | Deterministic signing keys; EBSK round-trip/malformed-file tests (currently untested). |

### Keyed chunking (public `chunker` module)
| Item | Location | Research use |
|---|---|---|
| `chunker::EncryptedBoundaryKey::{SecretGearTable(Box<[u64;256]>), PhteAes128 { polynomial: u128, aes_key: [u8;16] }}` | chunker.rs:35-38 | A harness can construct arbitrary boundary keys (random tables/polynomials) without the AFK hierarchy, enabling boundary-leakage experiments (Truong et al. style attacks) on secret-Gear vs PHTE. |
| `chunker::chunk_ranges_encrypted(&[u8], ChunkingParameters, &EncryptedBoundaryKey)` | chunker.rs:116-130 | Throughput (bytes/s) PHTE vs Gear vs unkeyed `chunk_ranges`; chunk-size distribution under keyed modes. |
| `chunker::select_parameters_encrypted`, `chunker::evaluate` | chunker.rs:266, 287 | Dedup/overhead cost of keyed chunking vs unkeyed. |
| `chunker::ChunkingParameters { chunker_id: &'static str, minimum_size, target_size, maximum_size }` | chunker.rs:42-47 | Parameter sweeps. |

### Signatures and timestamps
| Item | Location | Research use |
|---|---|---|
| `crypto::current_bindings(&ecf::OpenedArchive, Option<AddressingBinding>)` | signature.rs:324 | Binding transcripts for plaintext (`ecf::open`) or encrypted (`open_encrypted_authenticated`) archives; staleness matrix experiments (repack, recompress, rechunk, metadata edits, rotation). `OpenedArchive` has public fields, so a harness can also fabricate identities to test binding comparison. |
| `crypto::sign_archive(&CurrentBindings, &SigningKey, bind_physical, bind_addressing)` | signature.rs:366 | Produce records for corpora. |
| `crypto::verify_signature(&SignatureRecord, &CurrentBindings, Option<&TimestampPolicy>)` | signature.rs:401 | Status matrix; Wycheproof EdDSA vectors can be injected by editing `public_key`/`signature` public fields of `SignatureRecord` (transcript is fixed, so only negative vectors and self-generated positives apply). |
| `crypto::SignatureRecord::{encode, encode_without_timestamp, decode, with_timestamp_token, signer_id}` + pub fields | signature.rs:109-220 | Canonical decode fuzzing; RFC 3161 imprint = SHA-256(`encode_without_timestamp()`), so a harness can build TimeStampReq messages for real TSAs and attach responses. |
| `crypto::SignaturePolicy::enforce(&[SignatureStatus])` | signature.rs:461 | Policy outcome matrix. |
| `crypto::read_detached_signature(&Path)` | signature.rs:506 | `.ebsig` corpus loading. |
| `crypto::TimestampPolicy { trust_anchors, verification_unix_seconds }`, `TimestampTrustAnchor { der }` | timestamp.rs:61-73 | Interop runs with real TSA tokens and anchors; deterministic verification time. |
| Status enums `CryptographicStatus`, `BindingStatus`, `TimestampStatus`, `SignatureStatus` | signature.rs:281-313 | Reporting experiments. |

Diagnostics: `diagnostics::Diagnostic::{code(), class()}` and `ReasonCode::*` (diagnostics.rs:118-139, 274-295) give stable `EB_CRYPTO_*` / `EB_SIGNATURE_*` strings for outcome matrices.

## 2. Experiments feasible today without internal access
1. Padding overhead and size leakage: `encrypt_archive`/`pack_directory_encrypted` over a corpus, all three `PaddingMode`s, compare `bytes.len()` and public segment/record sizes parsed from the documented footer v2 and segment headers (docs/crypto-wire-v1.md L158-201, L316-356).
2. Keyed-CDC leakage and throughput: construct `EncryptedBoundaryKey` values directly and call `chunk_ranges_encrypted`.
3. Unlock latency and DoS: `open_encrypted` with passwords (fixed 256 MiB/3/4 creation cost) and with many recipients; lower `CryptoPolicy` fields to find refusal points.
4. Tamper/truncation corpora: mutate bytes and re-hash public digests using the helper patterns in `tests/crypto_v1.rs:597-785`.
5. Signature/timestamp interop: `sign_archive` -> external TSA over SHA-256(`encode_without_timestamp`) -> `with_timestamp_token` -> `verify_signature` with real anchors (expect UNSUPPORTED for RSA/ECDSA TSAs).
6. Linkage/privacy: compare `EncryptedArchive.identities` and detached-signature bindings across re-encryptions of identical content.

## 3. Internal-only functionality that would need a research-only feature flag
A flag such as `#[cfg(feature = "research-internals")] pub mod research { pub use ... }` in `crypto/mod.rs` would be needed for:

| Internal item | Location | Why a harness needs it |
|---|---|---|
| `KeyHierarchy::derive`, `KeyHierarchy::boundary_key` | mod.rs:320-404 (private struct) | Derive the actual AFK-bound boundary key/planner inputs for a known AFK to compare against `EncryptedBoundaryKey` experiments; independent-vector regeneration. |
| `commitment`, `public_crypto_context`, `envelope_mac`, `recipient_fingerprint`, `method_context`, `wrap_key`, `wrap_ad`, `seal_afk`, `open_afk` | mod.rs:445-640 | Crafting adversarial but correctly MACed envelopes (unknown protection classes, maximal stanza sets, custom Argon2 parameters within wire bounds) for DoS and forward-compat tests; independent implementation cross-checks (V1-V6). |
| `a2id`, `parse_a2id`, `password_secret` | mod.rs:680-734 | Argon2 cost sweeps at non-default parameters (writer hard-codes 262144 KiB/3/4 at container.rs:555); requires either this flag or a `kdf` field in `EncryptedWriteOptions`. |
| `aead_seal`, `aead_open`, `hmac`, `derive` | mod.rs:431-443, 642-678 | Wycheproof-style primitive runs through Entrybound's exact wrappers and reason-code mapping. |
| `wire::t1`, `wire::decode_t1` | wire.rs:44-98 | Build/parse transcripts for independent vector generation and fuzz oracles. |
| `wire::RecipientStanza`, `encode_stanza_sequence`, `decode_stanza_sequence`, `CryptoEnvelope::{encode, decode}` | wire.rs:100-280 | Fuzz targets and limit-boundary tests on the attacker-controlled public envelope. |
| `wire::private_object`, `decode_private_object`, `sequence_container`, `decode_sequence_container`, `encode_private_fragment`, `decode_private_fragment`, `encrypted_object_id` | wire.rs:283-537 | EBPO/EBCS/fragment fuzzing, peak-memory measurement of 1 GiB containers, manifest sort-key vs `LogicalPath` ordering property tests. |
| `timestamp::verify_timestamp` | timestamp.rs:75 (`pub(super)`) | Returns the detailed `Diagnostic` that `verify_signature` discards (it only yields Valid/Invalid/Unsupported); essential for diagnosing real-TSA interop failures. |
| `CurrentBindings` construction (fields `pub(crate)`) | signature.rs:34-39 | Signing arbitrary binding transcripts (e.g., future identity profiles) and binding-validator fuzzing. |
| `container::segment_key`, `data_nonce`, `end_nonce`, `record_ad` | used by mod.rs:995-1017 tests | Segment-level crafting (owned by the container slice). |
| `canonical::RecordBuilder`, `decode_record` | canonical.rs:44, 285 (private module) | Any record-level corpus generation (signature records with unknown algorithms, stanza records with unknown classes). |

## 4. Existing tests a harness can mirror
- `crypto/mod.rs` unit tests: `descriptor_v2_public_context_vector` (L832), `corrected_v5_and_v6_wrapping_vectors` (L871), `frozen_v1_to_v4_construction_vectors` (L952), `local_key_files_are_typed_and_secret_debug_is_redacted` (L1052), `encrypted_boundary_derivation_and_phte_ranges_are_frozen` (L1070), `argon2_policy_is_checked_before_kdf_work` (L1121).
- `crypto/signature.rs` unit tests: L644-824 (V7, RFC 8032 7.1, strict verification, staleness, policy, timestamp bounds, EBSK).
- `crypto/timestamp.rs` unit test: `generated_ed25519_rfc3161_token_verifies_offline` (L461) contains a reusable Ed25519 TSA/certificate builder.
- `crypto/wire.rs` unit tests: L596-680.
- `tests/crypto_primitives.rs` (RFC 8452, 5869, 4231, 9106, X-Wing draft-10 KATs), `tests/crypto_v1.rs` (round trip, privacy, tamper, splicing, policy, mutation), `entrybound-cli/tests/crypto_cli.rs` (CLI flows).
