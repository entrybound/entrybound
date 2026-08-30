# Entrybound cryptographic suite v1

Status: **frozen security architecture; not implemented**. The suite identifier
is `entrybound-payload-suite-v1`, numeric ID `1`. An encrypted archive uses this
one suite in its entirety. There is no cipher list, preference order, or
algorithm negotiation. A future suite requires a new suite and format feature;
an implementation MUST NOT silently redefine suite 1.

## Primitive suite

| Purpose | Frozen primitive | Sizes |
|---|---|---|
| Payload, control-record, and file-key wrapping AEAD | AES-256-GCM-SIV, [RFC 8452](https://www.rfc-editor.org/rfc/rfc8452) | key 32 bytes; nonce 12; tag 16 |
| Key derivation | HKDF-SHA-256, [RFC 5869](https://www.rfc-editor.org/rfc/rfc5869) | PRK/output 32 bytes unless stated |
| Commitment and envelope authentication | HMAC-SHA-256, [RFC 2104](https://www.rfc-editor.org/rfc/rfc2104) | full 32-byte output; no truncation |
| Structured and public digests | SHA-256, [FIPS 180-4](https://csrc.nist.gov/pubs/fips/180-4/upd1/final) | 32 bytes |
| Hybrid recipient | X-Wing draft 10: ML-KEM-768 + X25519 + SHA3-256 | public key 1216; encapsulation 1120; shared secret 32 |
| Password recipient | Argon2id v=19, [RFC 9106](https://www.rfc-editor.org/rfc/rfc9106) | salt 16; output 32 |
| Archive signature | pure Ed25519, [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032) | public key 32; signature 64 |
| Strong keyed boundary PRF | AES-128 over a universal polynomial hash | two independent 16-byte keys |
| Randomness | operating-system CSPRNG | sizes stated per object |

AES-256-GCM-SIV was selected because it is a standardized CFRG construction
with a large key, broad AES implementation support, fixed 96-bit nonces, and
nonce-misuse resistance. Nonce reuse remains forbidden; misuse resistance is
defense in depth, not permission to create deterministic ciphertext. No
plaintext or unauthenticated intermediate may be released before tag
verification. Entrybound caps one AEAD plaintext record at 64 MiB, far below
RFC 8452's `2^36`-byte bound.

AES-GCM-SIV is not assumed to commit to its key. The distinct commitment below
is mandatory; see [RFC 9771](https://www.rfc-editor.org/rfc/rfc9771) and the
review rationale in [crypto-review-v1.md](crypto-review-v1.md).

## Canonical cryptographic transcript

Every construction below uses `T1`, not an ECF runtime object or serializer:

```text
T1(label, fields) =
    u16be(byte_length(label)) || UTF8(label) ||
    u16be(field_count) ||
    for each field in strictly increasing tag order:
        tag:u16be || value_length:u64be || value
```

Labels are the exact lower-case ASCII strings shown. Field values are exact
bytes; integer encodings are stated at each use and are big-endian. No field is
optional unless its transcript defines an explicit presence byte. Duplicate,
unknown, or out-of-order tags are invalid. `T1` is cryptographic format
semantics and cannot change without a versioned label.

## Archive file key and root hierarchy

Each encryption creates:

```text
AFK        = random(32)       # archive file key
archive_id = random(32)       # public, unique encryption epoch
```

RNG failure is fatal. `archive_id` is not a key or content identity.

The root salt and extract are:

```text
root_context = T1("entrybound/root-salt/v1", {
  1: "ecf/bootstrap-v1",
  2: format_major:u16be,
  3: format_minor:u16be,
  4: crypto_version:u16be = 1,
  5: payload_suite_id:u16be = 1,
  6: archive_id[32]
})
root_salt = SHA256(root_context)
root_prk  = HKDF-Extract-SHA256(salt=root_salt, IKM=AFK)
```

All child material uses:

```text
Derived(purpose, context, length) =
  HKDF-Expand-SHA256(
    PRK=root_prk,
    info=T1("entrybound/derive/v1", {
      1: ASCII(purpose),
      2: context
    }),
    L=length)
```

The frozen children are:

| Purpose label | Length | Use |
|---|---:|---|
| `commitment-key` | 32 | key commitment only |
| `envelope-mac-key` | 32 | CryptoEnvelope MAC only |
| `control-segment-root` | 32 | metadata, manifest, Index, signature, and terminal control segments |
| `payload-segment-root` | 32 | Chunk/dictionary/reconstruction payload segments |
| `default-boundary-table-key` | 32 | default secret Gear-table derivation only |
| `strong-boundary-poly-key` | 16 | strong boundary polynomial key seed only |
| `strong-boundary-prf-key` | 16 | strong boundary AES-128 key only |

The context is the empty byte string for these seven roots. Implementations may
derive temporary implementation-internal keys only under a new purpose label;
they may not serialize or make those derivations format semantics. Keys are
zeroized where the Rust type permits, never logged, and never reused across the
purposes above.

## File-key commitment

The public stored commitment is:

```text
commitment_context = T1("entrybound/key-commitment/v1", {
  1: "ecf/bootstrap-v1",
  2: format_major:u16be,
  3: format_minor:u16be,
  4: crypto_version:u16be = 1,
  5: payload_suite_id:u16be = 1,
  6: archive_id[32],
  7: layout:u8 = 1,             # INDEXED
  8: padding_mode:u8,
  9: boundary_mode:u8
})
commitment = HMAC-SHA256(K_commitment, commitment_context)
```

The full 32 bytes are stored. After one recipient stanza authenticates and
unwraps a candidate AFK, the reader derives the hierarchy and compares the
commitment in constant time **before** accepting the AFK or decrypting archive
records. It then verifies the envelope MAC. A mismatch is a hard failure and
the candidate key is erased. This prevents two stanza paths from being
accepted as different file keys for one archive context and closes the suite's
key-substitution/ambiguity gap.

## CryptoEnvelope authentication

`PublicCryptoContextV1` contains the format namespace/version, crypto version,
suite ID, `archive_id`, INDEXED layout, archive role, complete required-feature
bitmap, padding mode, and boundary mode. It excludes mutable locators and the
envelope MAC itself.

`EnvelopeCoreV1` contains that context, the commitment, protection policy, and
the full canonical recipient sequence. The sequence uses the ordering in
[crypto-wire-v1.md](crypto-wire-v1.md), including unknown stanza bytes.

```text
envelope_mac = HMAC-SHA256(
  K_envelope_mac,
  T1("entrybound/envelope-mac/v1", {
    1: canonical PublicCryptoContextV1,
    2: canonical EnvelopeCoreV1-without-MAC
  })
)
```

The full 32-byte MAC is stored and compared in constant time. It prevents
recipient insertion, removal, reordering, type/class rewriting, commitment
replacement, and public crypto-context rewriting after an AFK is known. Public
locators are authenticated by the terminal archive-final/footer construction;
excluding them avoids a circular MAC.

## Segments, keys, and nonces

There are two public segment classes:

```text
CONTROL = 1
PAYLOAD = 2
```

Segments occur in physical order with a global `segment_ordinal:u64` equal to
`0, 1, ...`. Each has an independent `segment_salt=random(16)`. Duplicate or
non-monotonic ordinals are nonconforming.

```text
class_root = K_control_segment_root or K_payload_segment_root
segment_prk = HKDF-Extract-SHA256(
    salt=segment_salt,
    IKM=class_root)
segment_key = HKDF-Expand-SHA256(
    PRK=segment_prk,
    info=T1("entrybound/segment-key/v1", {
      1: archive_id[32],
      2: payload_suite_id:u16be = 1,
      3: segment_class:u8,
      4: segment_ordinal:u64be
    }),
    L=32)
```

A segment holds at most `2^20 - 1` DATA records and at most 1 GiB
(`2^30` bytes) of aggregate private DATA plaintext, whichever limit is reached
first. A record that would exceed either bound starts a new segment. One
PAYLOAD record is at most 64 MiB; one CONTROL record is at most 1 MiB. A large
physical object is fragmented into independently authenticated records with an
encrypted object identity, fragment ordinal/count, and exact total length.

For zero-based DATA counter `i`:

```text
data_nonce(i) = 0x00000000 || i:u64be
```

After `n` DATA records, every segment has exactly one END record:

```text
end_nonce(n) = 0xffffffff || n:u64be
```

`n <= 2^20 - 1`; the prefixes make DATA and END nonce domains disjoint. The
archive ID, class-specific root, random salt, and ordinal make segment-key
domains disjoint without mutable global state. A repeated random salt is not a
nonce collision when the ordinal or class differs. Repeating both salt and
ordinal is rejected structurally. Implementations still use fresh salts.

The END plaintext authenticates the DATA record count, aggregate private and
ciphertext lengths, and SHA-256 over the ordered exact public record headers
and ciphertexts. The final segment is CONTROL and contains exactly one
`ARCHIVE_FINAL` DATA record followed by END. `ARCHIVE_FINAL` binds the ordered
completed-segment digests, final resource totals, LAI/PCR/AUX, recipient-set
digest, and intended footer core. Missing EOF/footer is `TRUNCATED`; a complete
but unauthentic final marker is an integrity failure.

Random access first authenticates a recipient, commitment, envelope, encrypted
descriptor, and encrypted Index/segment directory. A requested record then
authenticates independently under its segment key; declared ChunkGroup or
reconstruction dependencies are also decoded and verified. No API may present
this as verified random access if those prerequisites have not succeeded.

## AEAD associated data and private records

The associated data is:

```text
T1("entrybound/aead-record/v1", {
  1: "ecf/bootstrap-v1",
  2: crypto_version:u16be = 1,
  3: payload_suite_id:u16be = 1,
  4: archive_id[32],
  5: exact canonical SegmentHeaderV1 bytes,
  6: exact fixed ProtectedRecordHeaderV1 bytes
})
```

The public protected-record header is the sole owner of ciphertext length and
contains only fixed magic/version, coarse class, zero flags, segment ordinal,
message counter, and ciphertext length. The encrypted `PrivateRecordV1` is the
sole owner of exact record type, logical/object ordinal, unpadded content
length, fragment information, TransformPlan reference, logical length, and
payload. Those semantic facts are not copied into associated data or public
framing.

AEAD authenticates the private record and every random padding byte. The
reader checks counter/segment structure before decryption, authenticates into a
private buffer, parses canonical private framing, removes padding, applies
codec/inverse transforms/reconstruction, and verifies final original-byte
Chunk identities before release.

## Recipient stanzas

Every stanza wraps the same random 32-byte AFK using AES-256-GCM-SIV. Limits
are checked before cryptographic work:

- at most 1,024 stanzas;
- at most 64 KiB per stanza;
- at most 16 MiB for the complete CryptoEnvelope;
- exactly 16 random bytes of `stanza_id`;
- exactly 16 zero bytes of `recipient_hint` in v1;
- exactly 12 random bytes of wrap nonce;
- exactly 48 bytes of wrapped AFK (32 ciphertext + 16 tag).

The public hint is deliberately anonymous. A reader tries caller-supplied
identities against candidate stanzas under a caller attempt limit. An encrypted
recipient directory maps `stanza_id` to the SHA-256 fingerprint of the
canonical recipient public key and an optional user label, allowing authorized
`key list/remove` without a public stable identifier.

For a method-produced 32-byte secret `method_secret`:

```text
method_context_digest = SHA256(
  T1("entrybound/recipient-method-context/v1", {
    1: stanza_version:u16be = 1,
    2: stanza_type:u16be,
    3: protection_class:u8,
    4: stanza_id[16],
    5: recipient_hint[16],
    6: canonical method parameters,
    7: method encapsulation bytes
  })
)

wrap_prk = HKDF-Extract-SHA256(salt=archive_id, IKM=method_secret)
wrap_key = HKDF-Expand-SHA256(
  PRK=wrap_prk,
  info=T1("entrybound/recipient-wrap-key/v1", {
    1: payload_suite_id:u16be = 1,
    2: stanza_type:u16be,
    3: protection_class:u8,
    4: stanza_id[16],
    5: method_context_digest[32]
  }),
  L=32)
```

Wrap associated data is `T1("entrybound/recipient-wrap-ad/v1", ...)` over the
format namespace, crypto version, suite, archive ID, and complete public stanza
header through the wrap nonce. The plaintext is exactly AFK. Neither wrap key
nor nonce is deterministic in production.

Unknown stanza types are skipped only for identity matching. Their lengths,
protection class, reserved bits, and canonical framing are still validated,
and their exact bytes remain covered by the envelope MAC and recipient-set
digest. Unknown protection classes fail closed.

## Hybrid post-quantum recipient

Stanza type `1`, protection class `1`, is
`xwing-mlkem768-x25519-sha3-256/draft-10`. It is the exact X-Wing construction
in [draft-connolly-cfrg-xwing-kem-10](https://datatracker.ietf.org/doc/html/draft-connolly-cfrg-xwing-kem-10):

- ML-KEM-768 as standardized by [FIPS 203](https://csrc.nist.gov/pubs/fips/203/final);
- X25519 from [RFC 7748](https://www.rfc-editor.org/rfc/rfc7748);
- X-Wing's SHA3-256 combiner over `ss_M || ss_X || ct_X || pk_X ||
  5c2e2f2f5e5c`;
- 1216-byte public key, 32-byte private seed, 1120-byte encapsulation, and
  32-byte combined secret.

Entrybound does not alter or reimplement the combiner. The method parameters
are the exact ASCII identifier above and the encapsulation is the exact
1120-byte X-Wing ciphertext. The hybrid is intended to preserve confidentiality
when either ML-KEM-768 or X25519 remains secure under X-Wing's stated model.

Normal v1 recipient archives have protection policy `HYBRID_ONLY`; every
stanza must declare class 1. A producer cannot add a password or future
classical-only stanza to that AFK. A recipient already holding AFK can disclose
it regardless, but any stanza-set edit is visible to the envelope MAC and an
expected addressing signature.

X-Wing draft 10 is active rather than a final RFC at this review date. The
wire identifier pins draft 10 permanently. The implementation release gate is
successful official draft KATs plus interoperability with an independent
implementation; a crate version alone is not sufficient.

## Classical recipient policy

Crypto v1 defines no creatable classical-only stanza. Numeric stanza type
`3..32767` remains unassigned rather than implying support. This avoids an
accidental downgrade path and keeps hybrid protection the normal public-key
mode. A later format review may define a separate classical protection class
and stanza type, but it may never share an AFK with hybrid-protected stanzas.

## Password recipient

Stanza type `2`, protection class `2`, is `argon2id-v19/aes256-gcm-siv-wrap-v1`.
Password archives have policy `PASSWORD_ONLY`, contain exactly one password
stanza, and contain no hybrid or other recipient.

The password is the exact UTF-8 byte sequence entered by the user. Entrybound
does not normalize Unicode, trim whitespace, append NUL, or accept a password
as a command-line argument. Creation prompts twice from a controlling terminal.

The creation parameters are frozen:

```text
algorithm   Argon2id
version     0x13 (19)
salt        random 16 bytes
memory      262144 KiB (256 MiB)
iterations  3
parallelism 4
output      32 bytes
```

The canonical parameter bytes are:

```text
"A2ID" || version:u32be || memory_kib:u32be ||
iterations:u32be || parallelism:u32be || salt[16]
```

Readers accept only version 19 and these conformance bounds:

```text
65536 <= memory_kib <= 1048576
3 <= iterations <= 10
1 <= parallelism <= 16
memory_kib >= 8 * parallelism
memory_kib mod (4 * parallelism) == 0
output length == 32
salt length == 16
```

Caller policy may impose lower maxima. All integers, products, and memory
requirements are validated for overflow and against policy before allocation
or Argon2 work. The 32-byte Argon2id result is `method_secret` for the common
stanza wrapping construction; it is not used directly as an AEAD key.

The parameters deliberately exceed RFC 9106's memory-constrained 64 MiB,
three-pass recommendation because an archive is an offline, low-frequency
operation. They remain below the 1 GiB wire ceiling. The public commitment and
wrapped-key tag create an unavoidable offline password-verification oracle, so
strong passwords and a meaningful KDF cost remain essential.

## Encrypted boundaries, padding, and deduplication

### Padding modes

Padding mode is public and authenticated:

```text
0 = NONE
1 = BUCKETED (creation default)
2 = MAXIMUM
```

For bucketed padding, define the ordered quarter-octave bucket set for powers
`k` as:

```text
2^k, 5*2^(k-2), 3*2^(k-1), 7*2^(k-2), then 2^(k+1)
```

Duplicates at octave boundaries are removed. CONTROL buckets run from 256
bytes through 1 MiB (`k=8..20`). PAYLOAD buckets run from 4 KiB through
64 MiB (`k=12..26`). The smallest bucket at least as large as the complete
unpadded `PrivateRecordV1` is selected; fresh random padding fills the rest.
The 16-byte AEAD tag is outside the bucket.

For inputs already above the smallest bucket, worst proportional padding is
less than 25%. Under the explicitly non-adversarial model in which unpadded
sizes are uniform within a bucket, mean overhead is roughly 6.7% to 11.1%
depending on the subinterval. Small records may pay the entire minimum bucket.
These are models, not a privacy guarantee.

`MAXIMUM` pads every CONTROL record to 1 MiB and every PAYLOAD record to
64 MiB. It still leaks class and record count and can be extremely expensive.
`NONE` adds no bytes after the private record and accepts exact-length leakage.
The writer refuses an object that cannot be fragmented within the class
capacity.

### Boundary modes

Boundary mode is public and authenticated:

```text
1 = SECRET_GEAR_TABLE
2 = PHTE_AES128
```

The creation default remains `SECRET_GEAR_TABLE` plus bucketed padding. It uses
the existing normalized Gear update and profile min/target/max values, but
derives each table value as the first eight bytes, interpreted big-endian, of:

```text
HMAC-SHA256(
  K_default_boundary_table,
  T1("entrybound/gear-table/v1", {1: i:u16be})
)
```

Its chunker ID is
`gear-norm-secret-table-v1/min-{n}/target-{n}/max-{n}`. Current research has
broken several analogous secret-table/folklore KCDC designs. Entrybound treats
this mode only as defense in depth whose residual boundary leakage is further
quantized by padding; it is not the strong KCDC claim.

`PHTE_AES128` is the optional stronger mode exposed as
`--chunk-boundary=keyed-prf`. It instantiates the Poly-hashing-then-Encrypt
construction from Truong et al., *Breaking and Fixing Content-Defined
Chunking* ([CCS 2025 paper](https://eprint.iacr.org/2025/558.pdf)):

- window `w=64` bytes, continuous within one ContentObject and not reset at a
  cut;
- field `F = Z/(2^127 - 1)Z`;
- `btof(byte)` is the integer 0..255 in `F`;
- `ftob(u)` is the 16-byte big-endian representation with the top bit zero;
- `Kpoly` is sampled uniformly from `F`: for `j=0,1,...`, compute the first
  16 bytes of `HMAC-SHA256(K_strong_boundary_poly,
  T1("entrybound/phte-poly-candidate/v1", {1: j:u32be}))`, clear the high bit,
  interpret big-endian, and reject the sole value `2^127 - 1`; the first
  accepted value is `Kpoly`;
- `KE` is the independently derived strong-boundary AES-128 key;
- rolling state is `u = sum(btof(B_i) * Kpoly^(w-1-i)) mod (2^127-1)` and uses
  the paper's remove/multiply/add update;
- the decision word is `AES-128(KE, ftob(u))`, interpreted as a big-endian
  128-bit integer;
- the existing normalized policy cuts on the low `b+1` zero bits before the
  target and low `b-1` zero bits from target to maximum, with forced maximum;
  `b=log2(target)` and min/target/max remain the selected profile values;
- no decision is evaluated before minimum, every byte still updates state, and
  the final non-empty tail is emitted.

Its chunker ID is
`phte-aes128-norm-v1/min-{n}/target-{n}/max-{n}`. The cited measurements found
one AES evaluation per input byte reduced throughput by about 53%--165% in the
patched Restic construction, so this stronger mode remains explicit rather
than default. It is recommended for a storage-provider adversary with known or
chosen plaintext and observable Chunk patterns.

Encrypted creation uses `fast-enc-v1`, `balanced-enc-v1`, `dense-enc-v1`, or
`extreme-enc-v1` planner IDs. Each inherits the exact corresponding frozen v6
codec/transform/reconstruction candidate set; only its versioned encrypted
chunker is substituted. This avoids silently redefining `fast-v6` through
`extreme-v6`.

### Deduplication domain

Exact dedup remains SHA-256 equality over plaintext Chunks, but only within one
AFK/archive domain. Each unique plaintext Chunk has one encrypted physical
representation and may be referenced many times inside that archive. No key is
content-derived; there is no convergent encryption, cross-archive dedup, or
cross-tenant equality oracle.

Keyed chunking changes boundaries before dedup. Padding applies after
compression/transforms and before encryption. References and logical equality
metadata are encrypted, although a provider may infer some within-archive
equality from storage/access behavior. AFK rotation derives new boundary keys,
rechunks content, and may change PCR.

## Signatures

Signature algorithm ID `1` is pure Ed25519. Wire public keys are canonical raw
32-byte compressed Edwards-Y encodings and signatures are 64-byte RFC 8032
values. Verification is strict: noncanonical encodings, small-order points, and
invalid scalar encodings are rejected. The 16-byte signer identifier is:

```text
SHA256(T1("entrybound/signer-id/v1", {1: public_key[32]}))[0..16]
```

A signature object always includes the content binding and may include physical
and addressing bindings. CLI creation defaults to all three. One Ed25519
signature covers the canonical `SignatureTranscriptV1`, including signature
version, algorithm ID, binding mask, signer public key, and exact binding
transcripts described in [crypto-wire-v1.md](crypto-wire-v1.md).

- Content binds LAI, AUX, `identity/v1`, and ECF namespace/version.
- Physical binds PCR.
- Addressing binds suite ID, recipient-set digest, commitment, and archive ID.

Verification first checks the signature over its stored values. Each present
binding is then compared independently with the current archive:

- bad signature/encoding is `INVALID` for the complete signature;
- a verified matching binding is `VALID`;
- a verified stored binding that differs from current state is `STALE` for
  that binding, not a forged signature.

Recompression, physical reordering, layout/index rebuilding, or signature
embedding leaves content valid; physical remains valid when PCR is unchanged.
Metadata or content identity changes make content stale. Rechunking makes
physical stale. Recipient edits, commitment/suite/archive-ID changes make
addressing stale.

Signatures are sign-then-encrypt when embedded, hiding signer identity and
values. A detached `.ebsig` is the exact same canonical SignatureRecord and
intentionally exposes them. Offline verification requires only the archive,
signature, and caller trust decision; Entrybound does not define a PKI.

### Timestamps

A SignatureRecord may carry one DER RFC 3161 TimeStampToken, maximum 64 KiB.
The `messageImprint` algorithm is SHA-256 and its value is
`SHA256(canonical SignatureCoreV1 bytes excluding the timestamp)`. Crypto v1
accepts a CMS signer using Ed25519 (`id-Ed25519`) and an embedded certificate
chain; other token signature algorithms are `TIMESTAMP_UNSUPPORTED`, not a
fallback path. Verification uses caller-provided trust anchors and time policy
and performs no required network fetch. Timestamp status is separate from the
archive signature: absent is allowed, invalid is reported, and unsupported
does not transform a valid archive signature into an invalid one.

For a cryptographically valid stored signature, current-binding status is
frozen as follows (`--` means the binding was not present):

| Operation/current change | Content | Physical | Addressing |
|---|---|---|---|
| no bound value changed | VALID | VALID | VALID |
| Index rebuild/repair only | VALID | VALID | VALID |
| codec, transform, dictionary, group, layout, or ciphertext change with identical logical Chunk organization | VALID | VALID | VALID unless suite/envelope changed |
| mtime/Fidelity/AUX change | STALE | VALID if PCR unchanged | VALID |
| file bytes, path, kind, executable bit, identity profile, or format version change | STALE | compare current PCR independently | VALID unless envelope changed |
| rechunk with the same semantics | VALID | STALE | VALID |
| add a recipient under the same AFK/archive ID | VALID | VALID | STALE |
| remove a recipient, change password, rotate AFK/archive ID, or change suite | VALID if semantics/AUX unchanged | STALE when keyed rechunking changes PCR | STALE |
| signature/public-key/encoding verification failure | INVALID | INVALID | INVALID |

Status is derived by comparing the verified stored transcript with current
values. A stale optional signature does not make the archive nonconforming; it
becomes a failure only when caller policy requires that binding to be current.

## Key lifecycle and signature status

`key add` requires AFK knowledge. For a `HYBRID_ONLY` archive it adds a hybrid
stanza, updates the encrypted recipient directory, recomputes the envelope MAC,
and re-authenticates the small terminal control/footer binding with fresh
segment salt/nonces. The AFK, archive ID, keyed boundaries, bulk payload
ciphertext, LAI, AUX, and PCR remain unchanged. Addressing signatures become
stale; content and physical bindings remain valid.

`key remove` creates a fresh AFK and archive ID, regenerates every remaining
stanza, re-derives keyed boundaries, and fully re-encrypts. LAI and AUX remain
the same if semantics/fidelity do; PCR may change because keyed boundaries
change; PCI changes. Physical and addressing bindings become stale. This is
the only honest removal operation and does not revoke old copies.

Password archives have exactly one stanza, so neither add nor remove is an
in-place recipient edit. Changing a password is full re-encryption with a new
AFK/archive ID.

## CLI security behavior reserved for implementation

The frozen future surface is:

```text
ebound pack ... --recipient <spec>
ebound pack ... --password
ebound unpack archive.eb --identity <spec>
ebound verify archive.eb --signatures
ebound inspect archive.eb --crypto

ebound sign archive.eb --identity <key>
ebound key list archive.eb
ebound key add archive.eb --recipient <spec>
ebound key remove archive.eb --recipient <spec>
```

`--recipient` means hybrid X-Wing in v1. Encrypted packing is metadata-private
by default. Password bytes are never accepted in argv, environment variables,
URLs, or diagnostic output; the CLI reads them from a controlling terminal and
confirms creation input. If stdin is archive data and no controlling terminal
exists, password prompting fails rather than reading from the archive stream.

Private-key files that are group/world-readable on Unix are refused by default;
platform ACLs are checked where practical and otherwise produce a prominent
warning. Secrets are not read from network services by default. `inspect
--crypto` without an identity reports only the deliberate public leakage; with
an identity it may report authenticated private detail. `verify --signatures`
requires decryption for embedded signatures.

Encrypted STREAM and encrypted stdout fail before writing. `key list` without
an identity shows only public stanza types/counts; after authentication it may
show the private recipient directory. Key add has the limited rewrite described
above. Key remove and password change always perform full re-encryption.

## Numbered security invariants

Future conformance tests SHALL cite these identifiers.

1. **CRYPTO-001:** An encrypted archive uses authenticated encryption for
   every protected byte; no unauthenticated encryption mode exists.
2. **CRYPTO-002:** Suite 1 is the sole payload suite for crypto v1; algorithm
   selection is not user-negotiated.
3. **CRYPTO-003:** Production AFK, archive ID, salts, stanza IDs, nonces, and
   padding randomness come from the OS CSPRNG; deterministic production
   ciphertext is forbidden.
4. **CRYPTO-004:** An AFK is accepted only after recipient-wrap authentication,
   constant-time commitment verification, and envelope-MAC verification.
5. **CRYPTO-005:** No purpose in the key hierarchy reuses another purpose's
   derived key material or domain label.
6. **CRYPTO-006:** No `(segment_key, nonce)` pair repeats; DATA and END nonce
   domains are disjoint.
7. **CRYPTO-007:** Global segment ordinals are contiguous and bound with class,
   archive ID, and salt into segment keys and associated data.
8. **CRYPTO-008:** Every segment has exactly one authenticated END and every
   archive has exactly one terminal authenticated ARCHIVE_FINAL.
9. **CRYPTO-009:** Truncation, record/segment reorder, and cross-archive splice
   cannot be accepted as a complete valid archive.
10. **CRYPTO-010:** Public framing that influences record decryption is exact
    AEAD associated data; public archive framing is checked against the
    authenticated terminal binding.
11. **CRYPTO-011:** A semantic fact has one authority inside authenticated
    private records and is not copied into public framing.
12. **CRYPTO-012:** Plaintext is buffered privately until AEAD authentication;
    decoded content is not verified until final original-byte digests pass.
13. **CRYPTO-013:** Names, paths, EAM, metadata, Index, identities, plans,
    reconstruction data, and embedded signatures are encrypted.
14. **CRYPTO-014:** The complete canonical recipient set, including unknown
    stanza bytes, is covered by the envelope MAC and recipient-set digest.
15. **CRYPTO-015:** Unknown crypto-critical suites, protection classes,
    feature bits, padding modes, or boundary modes fail closed.
16. **CRYPTO-016:** Unknown stanza types may be skipped only for identity
    matching; they cannot be removed, reordered, or ignored by authentication.
17. **CRYPTO-017:** A hybrid archive contains only hybrid-protection-class
    stanzas; a password archive contains exactly one password stanza.
18. **CRYPTO-018:** Crypto v1 creates no classical-only recipient and never
    mixes a weaker recipient with a hybrid AFK.
19. **CRYPTO-019:** Password parameters and all arithmetic are validated
    against format and caller limits before allocation or KDF execution.
20. **CRYPTO-020:** Passwords are not accepted on the command line and no
    secret-dependent detail is emitted as a public diagnostic.
21. **CRYPTO-021:** No content-derived encryption key, convergent encryption,
    cross-archive dedup, or cross-tenant dedup exists.
22. **CRYPTO-022:** Dedup equality is exact plaintext SHA-256 only and remains
    scoped to one AFK/archive domain.
23. **CRYPTO-023:** Padding and boundary modes are authenticated and inspectable;
    `NONE` is an explicit leak-accepting opt-out.
24. **CRYPTO-024:** Bucket padding is never described as complete size hiding;
    total size, record count, and access pattern remain observable.
25. **CRYPTO-025:** Strong keyed boundaries use the frozen PHTE construction;
    the default secret-table mode carries no equivalent security claim.
26. **CRYPTO-026:** Archive-controlled sizes, counts, expansion, decoder memory,
    KDF cost, and identity attempts cannot exceed caller policy.
27. **CRYPTO-027:** Signature transcripts bind signature algorithm/version and
    exact binding context; algorithm identifiers cannot be rewritten into an
    alternate verification path.
28. **CRYPTO-028:** Content, physical, and addressing signature status is
    evaluated independently as VALID, STALE, absent, unsupported, or invalid.
29. **CRYPTO-029:** Embedded signatures are encrypted; detached signatures are
    byte-identical canonical records and deliberately public.
30. **CRYPTO-030:** Recipient addition requires AFK knowledge and makes the
    addressing binding stale; removal rotates AFK/archive ID and fully
    re-encrypts.
31. **CRYPTO-031:** Key removal does not claim revocation of plaintext or old
    archive copies already obtained.
32. **CRYPTO-032:** Crypto v1 rejects STREAM/unseekable output rather than
    weakening metadata privacy or silently changing layout.
33. **CRYPTO-033:** PCI remains the exact-byte digest, not an authentication
    substitute; LAI/PCR/AUX retain their existing meanings.
34. **CRYPTO-034:** Whole-valid-archive replay freshness requires caller state;
    no self-contained freshness claim is made.
35. **CRYPTO-035:** Test-only fixed randomness is unreachable from production
    APIs and cannot be enabled by archive or CLI input.
