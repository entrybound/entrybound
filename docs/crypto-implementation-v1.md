# Crypto v1 implementation note

Status: experimental implementation of the frozen encrypted-INDEXED subset.
The normative construction remains `crypto-suite-v1.md` and
`crypto-wire-v1.md`; this note records implementation choices and verification,
not a second wire authority.

## Implemented surface

The Rust library creates and opens Complete, INDEXED crypto-v1 archives. The
public envelope supports either one or more X-Wing draft-10 hybrid recipients,
or exactly one password recipient. Password and hybrid stanzas cannot be mixed.
Encrypted STREAM output is rejected before filesystem capture or output-file
creation.

The public container is the frozen 256-byte crypto preamble, one canonical
`CRYPTO_ENVELOPE` section, one `ENCRYPTED_SEGMENTS` section, and the 192-byte
footer v2. Descriptor, manifest, paths, metadata, fidelity, plans,
dictionaries, groups, reconstruction state and regions, Chunk frames, optional
Index, logical identities, and final totals are carried only inside
authenticated encrypted private objects. `EBPO` selects record, Chunk-frame,
or `EBCS` parsing without heuristics; all nine `EBCS` collection kinds use the
frozen ordering, cardinality, duplicate, and item-type rules.

The writer compresses and transforms before encryption. It fragments a private
object only at the frozen protected-record boundary and reassembles it only
after every fragment authenticates and its encrypted-object digest matches.
Each current writer segment contains one complete private object. This is a
simple valid v1 organization, not a new semantic boundary.

The implementation derives a fresh 32-byte AFK and archive ID from the OS CSPRNG
for every archive. X-Wing encapsulation and each 16-byte segment salt also use
fresh randomness. No production deterministic-encryption option exists.

## Dependencies

The direct crypto dependencies are pinned exactly:

| Crate | Version | Purpose | License |
|---|---:|---|---|
| `aes` | 0.9.3 | AES-128 for PHTE boundary decisions | MIT OR Apache-2.0 |
| `aes-gcm-siv` | 0.12.1 | Payload and AFK-wrap AEAD | MIT OR Apache-2.0 |
| `argon2` | 0.6.0 | Argon2id password recipient | MIT OR Apache-2.0 |
| `getrandom` | 0.4.3 | OS entropy | MIT OR Apache-2.0 |
| `hkdf` | 0.13.0 | HKDF-SHA-256 hierarchy | MIT OR Apache-2.0 |
| `hmac` | 0.13.0 | commitment and envelope MAC | MIT OR Apache-2.0 |
| `subtle` | 2.6.1 | constant-time commitment and MAC comparison | BSD-3-Clause |
| `ml-kem` | 0.3.2 | X-Wing ML-KEM-768 component and KAT API | Apache-2.0 OR MIT |
| `sha2` | 0.11.0 | SHA-256 | MIT OR Apache-2.0 |
| `sha3` | 0.12.0 | frozen X-Wing combiner validation | MIT OR Apache-2.0 |
| `x-wing` | 0.1.0 | draft-10 X-Wing KEM | Apache-2.0 OR MIT |
| `x25519-dalek` | 3.0.0 | reviewed X-Wing dependency surface | BSD-3-Clause |
| `zeroize` | 1.9.0 | owned secret erasure | Apache-2.0 OR MIT |
| `rpassword` | 7.4.0 | controlling-terminal CLI prompts | Apache-2.0 |

Entrybound adds no unsafe Rust and implements no cryptographic primitive. The
workspace remains Rust 1.94 MSRV; these dependencies did not require an MSRV
increase. The X-Wing test suite imports all three authoritative draft-10 KATs
and checks deterministic public-key generation, encapsulation, decapsulation,
and shared secrets byte-for-byte.

## Keys, recipients, and local key files

One `KeyHierarchy` centralizes every frozen HKDF label. It owns commitment,
envelope-MAC, control-segment, payload-segment, secret-Gear, strong-boundary
polynomial, and strong-boundary AES keys as distinct `ZeroizeOnDrop` values.
AFKs, method secrets, segment/wrap keys, password buffers, and candidate
unwrapped AFKs are likewise held in non-printing zeroizing types where owned.
No secret type implements `Debug` or `Display`; public-recipient debug output
contains only its fingerprint and non-secret label.

The experimental local `EBK1` wrapper is deliberately separate from `.eb`:
version 1/type 1 contains exactly a 1216-byte X-Wing public recipient and
version 1/type 2 contains exactly the 32-byte X-Wing identity seed. Length,
version, type, flags, and reserved bytes are strict. It is the minimum local
serialization needed by `--recipient` and `--identity`, not a general key store;
callers remain responsible for restricting identity-file permissions.

Recipient wrapping uses the corrected flat 14-field
`recipient-wrap-ad/v1`. The executable V5/V6 tests reproduce the authoritative
method-context, wrap-key, AD, and wrapped-AFK values in
`crypto-wire-v1-vectors.txt`; superseded pre-`1c9c925` values are not accepted.
Unlock verifies AEAD wrapping, then constant-time key commitment, then envelope
MAC before accepting the AFK. Candidate failures collapse to the public
no-matching-recipient result except for structurally malformed public input.

Password creation uses the frozen Argon2id v1.3 defaults: 256 MiB, three
passes, parallelism four, 16-byte random salt, and 32-byte output. The reader
checks encoded bounds and caller limits before invoking Argon2. Passwords are
read from a controlling terminal and creation requires confirmation; argv never
contains a password.

## Segments, padding, and chunk boundaries

CONTROL and PAYLOAD segments use their own derived roots, random salts, global
strictly increasing ordinals, up to `2^20 - 1` DATA messages, at most 1 GiB of
private DATA plaintext, and exactly one authenticated END. DATA nonces are
`00000000 || counter:u64be`; END nonces are
`ffffffff || data_count:u64be`. Associated data is the frozen T1 transcript over
suite/archive context and the exact segment/protected headers. ArchiveFinal and
footer binding validate the ordered completed-segment digest, recipient set,
descriptor and manifest object identities, final roots/totals, locators, total
length, and EOF.

`BUCKETED` is the default padding mode and uses the normative quarter-octave
schedule. `NONE` authenticates exact private length but exposes record size;
`MAXIMUM` pads to the maximum class. Padding bytes are random and authenticated,
and readers validate the private length and canonical class after AEAD opens.

Encrypted filesystem creation derives either the default 256-entry secret Gear
table from the AFK hierarchy or the optional PHTE polynomial and AES-128 key.
The resulting planner IDs are `fast-enc-v1`, `balanced-enc-v1`,
`dense-enc-v1`, and `extreme-enc-v1`. Compression candidates are inherited from
the matching frozen v6 profile. Unencrypted v6 chunking is unchanged.

## Resource and materialization policy

`CryptoPolicy` independently caps stanza count/size, envelope bytes, identity
attempts, Argon2 memory/passes/parallelism, segment count, messages per segment,
ciphertext record size, private record size, and crypto working memory. These
checks happen before the corresponding KDF, allocation, or segment work where
the public framing permits it. After authentication, the ordinary caller-owned
ResourceBudget and DecodeRequirements are enforced before codec decoding.

Encrypted extraction calls the same capability-relative, exclusive-create
materializer only after envelope, all AEAD records, segment finality, private
canonical structure, EAM semantics, codec/transforms/reconstruction, and all
identities have verified. Wrong credentials, corruption, or truncation therefore
leave the destination untouched.

## Frozen-wire issue discovered during implementation

The normative crypto prose says the encrypted Descriptor carries the complete
authoritative ResourceBudget and DecodeRequirements. The frozen canonical ECF
Descriptor record (type 1/version 1) has exactly fields 1 through 8 and contains
neither structure, and crypto v1 assigns no other private record for them.
`EBPO` requires the inner record to remain an exact canonical ECF record, so the
missing bytes cannot be added without a new normative record/version decision.

The implementation does not invent fields. It authenticates all private bytes
under the independent public `CryptoPolicy`, derives actual EAM and decoder
requirements from authenticated records, applies the caller ResourceBudget and
DecodeRequirements before codec decode, and reconstructs the ordinary
Descriptor model with those checked values. This preserves memory safety and
the existing canonical record, but it cannot satisfy the prose claim that a
producer-authored final resource declaration is present inside Descriptor.
That specification contradiction remains open and must be resolved by a
separate wire correction before crypto v1 can be called fully format-conformant.

## Conformance gates

Tests retain RFC 8452 AES-256-GCM-SIV, RFC 5869 HKDF, RFC 4231 HMAC-SHA-256,
RFC 9106 Argon2id, all X-Wing draft-10, V1-V6 transcript, secret-Gear, and PHTE
vectors. Generated integration tests cover hybrid and password unlock,
multi-recipient behavior, padding modes, metadata privacy, keyed chunking,
ciphertext/envelope/commitment/footer mutation, truncation, destination safety,
and real CLI pack/inspect/verify/unpack. Historical unencrypted INDEXED and
STREAM tests remain in the normal targeted suite.

Signatures, recipient add/remove, classical-only recipients, encrypted STREAM,
KMS/HSM integration, and deterministic encryption remain unimplemented.
