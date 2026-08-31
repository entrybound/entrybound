# Entrybound cryptographic wire architecture v1

Status: **frozen implementation specification; encrypted-INDEXED subset
implemented**. This document assigns the records, feature bits, canonical
transcripts, limits, and reason codes that implementations must use. It extends
`ecf/bootstrap-v1` under required incompatibility features; it does not
reinterpret historical unencrypted bytes. See
[crypto-implementation-v1.md](crypto-implementation-v1.md) for implementation
status and the implemented Descriptor-v2 schema correction.

## Numeric registry

### Required incompatibility features

Existing bits `0x1` through `0x10` retain their meanings. The following values
are frozen; implemented crypto-v1 bits are included in the Rust crypto reader's
supported-feature mask, while signature bit `0x200` remains unsupported:

| Bit | Name | Rule |
|---:|---|---|
| `0x20` | `encrypted-indexed-v1` | selects the encrypted INDEXED physical schema; requires `0x40` and `0x400`; forbids `0x10` |
| `0x40` | `payload-suite-v1` | requires suite ID 1 and crypto version 1 |
| `0x80` | `recipient-xwing-v1` | at least one type-1 stanza; requires `0x20`; mutually exclusive with `0x100` |
| `0x100` | `recipient-password-v1` | exactly one type-2 stanza; requires `0x20`; mutually exclusive with `0x80` |
| `0x200` | `signature-ed25519-v1` | at least one embedded type-26 signature record; may also be used by unencrypted future writers |
| `0x400` | `crypto-padding-v1` | exactly one authenticated padding mode 0, 1, or 2 is present |
| `0x800` | `keyed-boundary-phte-v1` | boundary mode must be 2; requires `0x20` |
| `0x1000` | `private-resource-declaration-v1` | requires `0x20` and exactly one authenticated private Descriptor record type 1/version 2; forbidden for unencrypted archives |

Bits are compatibility declarations, not competing semantic authorities. The
suite/mode fields below are authoritative. A bit and its required field must
agree. Unknown incompatibility bits fail before recipient work. Every
algorithm/mode identifier is also inside commitment, envelope, wrap, record, or
signature transcripts; rewriting an ID cannot select another valid path.

### Algorithms and enums

```text
crypto_version                 1
payload_suite_id               1  entrybound-payload-suite-v1

layout                         1  INDEXED
archive_role                   1  Complete

protection_policy              1  HYBRID_ONLY
                                2  PASSWORD_ONLY
protection_class               1  HYBRID_PQ
                                2  PASSWORD

recipient_stanza_type          1  xwing-mlkem768-x25519-sha3-256/draft-10
                                2  argon2id-v19/aes256-gcm-siv-wrap-v1

padding_mode                   0  NONE
                                1  BUCKETED
                                2  MAXIMUM
boundary_mode                  1  SECRET_GEAR_TABLE
                                2  PHTE_AES128

segment_class                  1  CONTROL
                                2  PAYLOAD
protected_record_class         1  DATA
                                2  END

private_object_kind            1  CANONICAL_RECORD
                                2  CHUNK_FRAME
                                3  SEQUENCE_CONTAINER

sequence_container_kind        1  MANIFEST
                                2  TRANSFORM_PLANS
                                3  DICTIONARIES
                                4  CHUNK_GROUPS
                                5  RECONSTRUCTION_DATA_AND_AUDIT
                                6  RECONSTRUCTION_REGIONS_AND_AUDIT
                                7  EMBEDDED_SIGNATURES
                                8  RECIPIENT_DIRECTORY
                                9  ENCRYPTED_INDEX

signature_algorithm            1  Ed25519
signature_binding_mask         bit 0 CONTENT (mandatory)
                                bit 1 PHYSICAL
                                bit 2 ADDRESSING
```

Reserved numeric values and nonzero reserved bits are nonconforming. Unknown
recipient stanza *types* are the sole skippable algorithm namespace, subject to
the authenticated handling below; unknown protection classes are not.

### New canonical record and section numbers

The existing canonical record header/TLV rules in
[format-v0.md](format-v0.md) apply.

| Record | Type |
|---|---:|
| CryptoEnvelopeV1 | 20 |
| RecipientStanzaV1 | 21 |
| RecipientDirectoryEntryV1 | 22 |
| PrivateFragmentV1 | 23 |
| SegmentEndV1 | 24 |
| ArchiveFinalV1 | 25 |
| SignatureRecordV1 | 26 |
| EncryptedIndexEntryV1 | 27 |

Encrypted INDEXED layout uses only these public sections:

| Section | Type | Cardinality |
|---|---:|---:|
| CRYPTO_ENVELOPE | 32 | exactly one |
| ENCRYPTED_SEGMENTS | 33 | exactly one |

Both use the existing 64-byte `EBS1` section header and unkeyed SHA-256 payload
digest. That digest is an early corruption check, not an authentication
substitute. All Descriptor, TransformPlan, Dictionary, ChunkGroup,
ReconstructionData, Chunk frame, Manifest, Fidelity, ReconstructionRegion,
Index, provenance/audit, recipient-directory, and embedded-signature bytes are
inside encrypted objects in ENCRYPTED_SEGMENTS. The Index remains a
non-authoritative cache.

## Encrypted INDEXED container

The exact top-level order is:

```text
256-byte preamble
CRYPTO_ENVELOPE section
ENCRYPTED_SEGMENTS section
192-byte encrypted-layout footer v2
EOF
```

The preamble retains its current size and field offsets. Under feature `0x20`:

- layout is INDEXED and role is Complete;
- public `BudgetDeclared` is false;
- public decode requirements and eight ResourceBudget values are zero sentinel
  values and are explicitly non-authoritative;
- `stream_dedup_window` is zero;
- final authoritative budgets/requirements are encrypted in the Descriptor and
  checked before protected decoding that depends on them;
- hard crypto framing limits from this specification apply before decryption.

New conforming encrypted writers set `0x1000` and encode Descriptor v2 as the
sole producer declaration. Descriptor v2 without `0x1000`, or Descriptor v1
with `0x1000`, is nonconforming. The public `BudgetDeclared=false` byte remains
a privacy sentinel and is not an alternate semantic declaration.

Encrypted archives emitted before the correction at commit `bb8cca9` may carry
`0x20` without `0x1000` and a private Descriptor v1. Current readers accept
that pair as the legacy experimental crypto-v1 compatibility form: public
crypto limits still apply before unlock, and actual authenticated EAM/decode
requirements are derived and caller-checked. Inspection reports that no
producer resource declaration was serialized. Writers never emit this form.

This feature-versioned sentinel rule prevents entry count, logical byte totals,
codec memory, and similar metadata from leaking through the historical public
preamble. It is not a duplicate declaration.

### Footer v2

Encrypted archives end in exactly 192 bytes:

| Offset | Value |
|---:|---|
| 0 | existing 8-byte footer magic `8E 45 42 46 0D 0A 1A 0A` |
| 8 | footer version `u16be = 2` |
| 10 | footer length `u16be = 192` |
| 12 | flags `u32be = 0` |
| 16 | exact total container length `u64be` |
| 24 | CRYPTO_ENVELOPE absolute offset `u64be` |
| 32 | CRYPTO_ENVELOPE complete section length `u64be` |
| 40 | ENCRYPTED_SEGMENTS absolute offset `u64be` |
| 48 | ENCRYPTED_SEGMENTS complete section length `u64be` |
| 56 | terminal segment absolute offset `u64be` |
| 64 | terminal ARCHIVE_FINAL record absolute offset `u64be` |
| 72 | SHA-256 of exact 256-byte preamble `[32]` |
| 104 | SHA-256 of exact ENCRYPTED_SEGMENTS section payload `[32]` |
| 136 | SHA-256 of canonical PublicCryptoContextV1 `[32]` |
| 168 | 24 zero reserved bytes |

`FooterCoreV1`, which avoids a circular dependency on ciphertext, is:

```text
T1("entrybound/encrypted-footer-core/v1", {
  1: footer_version:u16be = 2,
  2: total_container_length:u64be,
  3: envelope_offset:u64be,
  4: envelope_length:u64be,
  5: segments_offset:u64be,
  6: segments_length:u64be,
  7: terminal_segment_offset:u64be,
  8: archive_final_record_offset:u64be,
  9: preamble_sha256[32],
 10: public_crypto_context_sha256[32]
})
```

`ArchiveFinalV1` stores `SHA256(FooterCoreV1)`. The footer's unkeyed segments
digest is intentionally excluded from FooterCore because it includes the
ARCHIVE_FINAL ciphertext. It is checked as an exact corruption checksum; AEAD,
SegmentEnd, and ArchiveFinal supply security. Any footer locator/length used to
accept the archive must match the authenticated FooterCore.

## Public crypto records

### PublicCryptoContextV1

This transcript is derived, not serialized a second time:

```text
T1("entrybound/public-crypto-context/v1", {
  1: "ecf/bootstrap-v1",
  2: format_major:u16be,
  3: format_minor:u16be,
  4: crypto_version:u16be,
  5: payload_suite_id:u16be,
  6: archive_id[32],
  7: layout:u8,
  8: archive_role:u8,
  9: required_incompat_features:u64be,
 10: padding_mode:u8,
 11: boundary_mode:u8
})
```

The source fields are the preamble and CryptoEnvelope. They must agree.

### CryptoEnvelopeV1, record type 20

| Tag | Type and rule |
|---:|---|
| 1 | crypto version `u16be = 1` |
| 2 | payload suite `u16be = 1` |
| 3 | archive ID, 32 bytes |
| 4 | commitment, 32 bytes |
| 5 | protection policy `u8` |
| 6 | padding mode `u8` |
| 7 | boundary mode `u8` |
| 8 | exact `RecipientStanzaSequenceV1` bytes |
| 9 | envelope MAC, 32 bytes |

`RecipientStanzaSequenceV1` is exactly `count:u64be`, then repeated
`length:u64be || exact RecipientStanzaV1 record` in the canonical order below.
It is the field-local public-envelope sequence grammar, not an `EBCS` private
container. There is no magic because the enclosing type-20 field tag/type owns
dispatch. Limits are 1,024 stanzas, 64 KiB each, and 16 MiB total envelope
before allocation. Encrypted archives require a nonempty sequence.

`EnvelopeCoreV1` is:

```text
T1("entrybound/envelope-core/v1", {
  1: canonical PublicCryptoContextV1,
  2: commitment[32],
  3: protection_policy:u8,
  4: exact RecipientStanzaSequenceV1 bytes
})
```

The envelope MAC construction is in
[crypto-suite-v1.md](crypto-suite-v1.md).

### RecipientStanzaV1, record type 21

| Tag | Type and rule |
|---:|---|
| 1 | stanza version `u16be = 1` |
| 2 | stanza type `u16be` |
| 3 | protection class `u8` |
| 4 | random stanza ID, 16 bytes |
| 5 | recipient hint, exactly 16 zero bytes in v1 |
| 6 | canonical method parameters, bounded bytes |
| 7 | method encapsulation bytes; 1120 for X-Wing, empty for password |
| 8 | random wrap nonce, 12 bytes |
| 9 | wrapped AFK, 48 bytes |

For X-Wing, field 6 is exact ASCII
`xwing-mlkem768-x25519-sha3-256/draft-10`. For password, field 6 is the 36-byte
`A2ID` structure defined in the suite document.

The canonical stanza ordering key is:

```text
sort_key = SHA256(
  T1("entrybound/recipient-stanza-sort/v1", {
    1: exact RecipientStanzaV1 record bytes
  })
)
```

Sort ascending by `(sort_key, exact stanza bytes)`. Exact duplicate stanza
bytes are forbidden. The writer also deduplicates identical input recipient
public keys before encapsulation, using a private key fingerprint; a reader
cannot and need not identify semantically duplicate anonymous randomized
stanzas.

Unknown stanza types retain the same canonical outer fields and limits. They
may be skipped during matching but remain in the sorted sequence and MAC.

### Recipient-set digest

Let `S` be the exact sorted sequence encoding from EnvelopeCore. Then:

```text
recipient_set_digest = SHA256(
  T1("entrybound/recipient-set/v1", {1: S})
)
```

The digest therefore does not depend on input enumeration order, does include
unknown stanza bytes, and changes for any nonce, encapsulation, wrapped key,
type, class, parameter, or stanza-ID change. It is used in ArchiveFinal and the
addressing signature binding.

## Segment framing

### SegmentHeaderV1

Every segment begins with exactly 64 public bytes:

| Offset | Value |
|---:|---|
| 0 | magic `EBSG` |
| 4 | version `u16be = 1` |
| 6 | segment class `u8` |
| 7 | flags `u8 = 0` |
| 8 | global segment ordinal `u64be` |
| 16 | random segment salt `[16]` |
| 32 | DATA record count `u32be`, maximum `2^20 - 1` |
| 36 | reserved `u32be = 0` |
| 40 | reserved `u64be = 0`; unpadded private totals stay encrypted |
| 48 | complete segment extent including this header `u64be` |
| 56 | eight zero reserved bytes |

The complete header is the field-5 value in record associated data. Physical
segments are strictly ordinal-contiguous. The declared count/extent must match
actual framing and authenticated SegmentEnd. Exact unpadded byte totals exist
only in SegmentEnd.

### ProtectedRecordHeaderV1

Every protected record begins with exactly 32 public bytes:

| Offset | Value |
|---:|---|
| 0 | magic `EBC1` |
| 4 | version `u16be = 1` |
| 6 | protected-record class `u8` |
| 7 | flags `u8 = 0` |
| 8 | segment ordinal `u64be` |
| 16 | DATA counter or END count `u64be` |
| 24 | exact following ciphertext length `u64be` |

The exact header is field 6 of AEAD associated data and solely owns ciphertext
length. DATA counters are `0..n-1`; END has counter `n`. Ciphertext includes
the 16-byte tag and must fit the class limits before allocation.

### PrivateFragmentV1, record type 23

AEAD DATA plaintext is:

```text
private_length:u64be || exact PrivateFragmentV1 record || random padding
```

`private_length` includes only the canonical record and must fit within the
authenticated plaintext. Padding length is derived from AEAD plaintext length;
no second padding-size field exists.

| Tag | Type and rule |
|---:|---|
| 1 | encrypted object ID `[32]` |
| 2 | complete encrypted-object byte length `u64be` |
| 3 | fragment index `u32be`, zero based |
| 4 | fragment count `u32be`, nonzero |
| 5 | fragment byte offset `u64be` |
| 6 | exact fragment bytes |

```text
object_id = SHA256(
  T1("entrybound/encrypted-object/v1", {
    1: exact complete object bytes
  })
)
```

Fragments for one object are contiguous, ordered, gap-free, non-overlapping,
and collectively exact. They reassemble one `PrivateObjectV1`; no fragment is
parsed as an inner object before complete authenticated reassembly.

### PrivateObjectV1 dispatch envelope

Every complete encrypted object has this exact 12-byte header followed by its
payload through the reassembled object extent:

| Offset | Value |
|---:|---|
| 0 | magic ASCII `EBPO` |
| 4 | version `u16be = 1` |
| 6 | `private_object_kind:u16be` |
| 8 | flags `u32be = 0` |
| 12 | payload, extending to the exact complete-object length from `PrivateFragmentV1` tag 2 |

There is intentionally no payload-length field: the fragment declaration is
the sole owner of complete object extent. `PrivateObjectV1` is a standalone
crypto framing grammar, not an ECF canonical record. Its kind is physical
parser dispatch, not a second semantic record type.

- `CANONICAL_RECORD` payload is exactly one complete canonical ECF record,
  including its 16-byte record header, with no trailing bytes.
- `CHUNK_FRAME` payload is exactly one complete versioned `EBCH` frame. The
  frame owns its physical/logical fields and stored length.
- `SEQUENCE_CONTAINER` payload is exactly one complete `EBCS` canonical
  sequence container defined below.

The selected dispatch and payload grammar must agree. Nested `EBPO` values,
unknown object kinds, wrong inner magic/version, trailing bytes, and a complete
object over `2^30` bytes are rejected before semantic use. This explicit header
eliminates record/frame/container heuristics while leaving the inner record,
Chunk frame, or sequence kind as its fact authority. A protected public DATA
class still reveals only CONTROL versus PAYLOAD through its segment.

### CanonicalSequenceV1 (`EBCS`)

Crypto v1's phrase **canonical sequence container** means only this standalone
grammar; it never means a raw concatenation or the field-local ECF sequence
grammar from `format-v0.md`:

```text
magic[4] = ASCII "EBCS" ||
version:u16be = 1 ||
container_kind:u16be ||
flags:u32be = 0 ||
item_count:u64be ||
for each item in normative order:
    item_length:u64be || exact canonical ECF record bytes
```

The fixed header is 20 bytes. The complete extent is the remaining payload of
the enclosing `PrivateObjectV1`, so there is no duplicate total-length field.
Every item length is nonzero, at most 64 MiB, and must equal one complete ECF
record. Item count is at most 1,000,000 and the complete `EBCS` extent is at
most 1 GiB. Limits and checked arithmetic are enforced before allocation; a
caller may set lower limits. No item may be another `EBCS`, `EBPO`, or Chunk
frame. Nesting is forbidden in v1.

The empty sequence for kind `K` is exactly
`"EBCS" || 0001 || K:u16be || 00000000 || 0000000000000000`.
An empty sequence is canonical only when the kind-specific cardinality below
permits it. Unknown container kinds fail closed as
`EB_CRYPTO_SUITE_UNSUPPORTED`; malformed known containers use
`EB_CRYPTO_PRIVATE_OBJECT_INVALID`. Parsers first dispatch on the `EBPO` kind,
then on `EBCS.container_kind`, then require the item record types and ordering
in this table. They never infer a collection from its contents.

| Kind | Allowed exact item record types | Canonical order | Empty and duplicates |
|---:|---|---|---|
| 1 MANIFEST | Entry (3), then ContentObject (4) | Entries by canonical LogicalPath; then ContentObjects by logical digest | empty allowed; duplicate path/digest forbidden |
| 2 TRANSFORM_PLANS | TransformPlan (2) | plan ID ascending | nonempty; duplicate ID forbidden |
| 3 DICTIONARIES | Dictionary (11) | dictionary digest ascending | empty allowed; duplicate digest forbidden |
| 4 CHUNK_GROUPS | ChunkGroup (12) | group ID ascending | empty allowed; duplicate ID forbidden |
| 5 RECONSTRUCTION_DATA_AND_AUDIT | ReconstructionData (14), then ReconstructionFallback (16) | data by reconstruction digest; then fallback by Chunk digest | empty allowed; duplicate target/digest forbidden |
| 6 RECONSTRUCTION_REGIONS_AND_AUDIT | ReconstructionRegion (18), then ReconstructionAudit v2 (19) | regions by region ID; then audits by canonical `(target kind, target digest, transform ID)` byte tuple | empty allowed; duplicate region/target tuple forbidden |
| 7 EMBEDDED_SIGNATURES | SignatureRecordV1 (26) | exact canonical record bytes lexicographically | object, when present, is nonempty; exact duplicates forbidden |
| 8 RECIPIENT_DIRECTORY | RecipientDirectoryEntryV1 (22) | stanza ID ascending | HYBRID_ONLY object is nonempty and has exactly one unique entry for every authenticated stanza; absent in PASSWORD_ONLY |
| 9 ENCRYPTED_INDEX | EncryptedIndexEntryV1 (27) | Chunk digest ascending | empty allowed; duplicate digest forbidden |

For kind 6, the audit ordering tuple is encoded for comparison as
`target_kind:u8 || target_digest[32] || transform_id_length:u64be ||
UTF8(transform_id)`; no locale or Unicode normalization is applied. All other
table keys are the exact byte encodings already frozen by their referenced ECF
record definitions.

Descriptor (type 1), FidelityReport (type 5), ArchiveFinalV1 (type 25), and
other singleton control records are `CANONICAL_RECORD` objects. Chunk data is
one `CHUNK_FRAME` object per physically stored Chunk, in the already-defined
physical order. TransformStep records remain nested inside the field-local
TransformPlan ECF sequence; Fidelity subrecords remain nested inside their
existing ECF fields. They are not `EBCS` items. Crypto v1 assigns no provenance
or ConversionRecord record, so no provenance sequence exists; adding one needs
a new record assignment and required feature rather than an unknown item.

### Descriptor v2, canonical record type 1/version 2

Descriptor v1 remains record type 1/version 1 with exactly tags 1 through 8.
Descriptor v2 retains those fields byte-for-byte and adds the authenticated
private declarations below. All 19 tags are required exactly once, use the
ordinary strictly-increasing canonical TLV grammar, and admit no unknown or
optional fields.

| Tag | Canonical type | Meaning |
|---:|---|---|
| 1 | UTF-8 | format namespace |
| 2 | `u8` | identity profile |
| 3 | `u8` | digest algorithm |
| 4 | UTF-8 | planner ID |
| 5 | UTF-8 | chunker ID |
| 6 | bytes, exactly 32 | LAI |
| 7 | bytes, exactly 32 | PCR |
| 8 | bytes, exactly 32 | AUX |
| 9 | `u64be` | `DecodeRequirements.window_bytes` |
| 10 | `u64be` | `DecodeRequirements.working_set_bytes` |
| 11 | `u32be` | `DecodeRequirements.flags` |
| 12 | `u64be` | `ResourceBudget.entry_count` |
| 13 | `u64be` | `ResourceBudget.total_logical_bytes` |
| 14 | `u64be` | `ResourceBudget.max_single_entry_logical_bytes` |
| 15 | `u64be` | `ResourceBudget.max_expansion_ratio_milli` |
| 16 | `u64be` | `ResourceBudget.chunk_count` |
| 17 | `u64be` | `ResourceBudget.max_path_depth` |
| 18 | `u64be` | `ResourceBudget.max_metadata_bytes` |
| 19 | `u64be` | `ResourceBudget.max_key_derivation_cost` |

Descriptor v2 itself means the producer declaration is present; no separate
private `budget_declared` field exists. After commitment and envelope-MAC
verification, the Descriptor object is the first authenticated private object.
Its declaration is checked against caller policy before dependent private
objects are accepted. The reader then independently aggregates decoder
requirements from authenticated plans/dictionaries/groups and derives archive
actuals from the complete EAM; requirements must match and actuals must remain
within every declared upper bound. Pre-unlock password/KDF safety continues to
come solely from caller `CryptoPolicy`, because Descriptor v2 is encrypted.

The exact historical, representative, and maximum-value encodings are frozen
in [descriptor-vectors-v1.txt](descriptor-vectors-v1.txt).

The deterministic encrypted-object emission order is: Descriptor;
TRANSFORM_PLANS; CHUNK_GROUPS; recipient directory when present; DICTIONARIES;
RECONSTRUCTION_DATA_AND_AUDIT; RECONSTRUCTION_REGIONS_AND_AUDIT; Chunk frames
in the authoritative physical order; MANIFEST; FidelityReport; embedded
signatures when present; ENCRYPTED_INDEX when present; then the terminal
ArchiveFinal object in its required final CONTROL segment. Empty collection
objects required by the active ordinary ECF feature schema are emitted even
when empty; optional audit/signature/Index objects are omitted when absent.
Object fragments remain consecutive in DATA-object order even when a segment
boundary inserts the prior segment's END and the next segment's header.

### Private recipient directory and encrypted Index entries

`RecipientDirectoryEntryV1` is canonical record type 22:

| Tag | Type and rule |
|---:|---|
| 1 | stanza ID, exactly 16 bytes |
| 2 | stanza type `u16be = 1` in v1 |
| 3 | recipient public-key fingerprint, exactly 32 bytes |
| 4 | required UTF-8 user label, 0..1,024 bytes; empty means no label |

The fingerprint is
`SHA256(T1("entrybound/recipient-public-key/v1", {1:
stanza_type:u16be, 2: exact canonical public key bytes}))`. The directory is
operational private metadata, not part of LAI/PCR/AUX or recipient-set digest.
Every ID/type must equal one stanza in the authenticated HYBRID_ONLY envelope;
PASSWORD_ONLY has no public key and therefore no directory object.

`EncryptedIndexEntryV1` is canonical record type 27:

| Tag | Type and rule |
|---:|---|
| 1 | Chunk digest, exactly 32 bytes |
| 2 | segment ordinal `u64be` |
| 3 | first protected-record counter `u64be` |
| 4 | fragment count `u32be`, nonzero |

Locators are relative to ENCRYPTED_SEGMENTS and never absolute container
offsets. The reader rebuilds the same map by scanning authenticated private
objects and ignores/rebuilds a missing or invalid kind-9 Index exactly as for
unencrypted INDEXED archives. This relative form allows CryptoEnvelope length
changes without rewriting bulk payload or creating a second semantic
authority.

### SegmentEndV1, record type 24

END plaintext uses the same `private_length || record || random padding`
framing and contains:

| Tag | Type |
|---:|---|
| 1 | segment ordinal `u64be` |
| 2 | segment class `u8` |
| 3 | DATA record count `u32be` |
| 4 | aggregate unpadded DATA private bytes `u64be` |
| 5 | aggregate DATA ciphertext bytes `u64be` |
| 6 | DATA transcript digest `[32]` |

The transcript digest is:

```text
SHA256(T1("entrybound/segment-data/v1", {
  1: exact SegmentHeaderV1,
  2: count:u64be || each(
       length:u64be || exact ProtectedRecordHeaderV1 || ciphertext
     ) in DATA-counter order
}))
```

The END record is excluded from its own field 6. A completed segment digest,
used by ArchiveFinal, is:

```text
SHA256(T1("entrybound/segment-digest/v1", {
  1: exact SegmentHeaderV1,
  2: exact DATA header/ciphertext sequence,
  3: exact END header,
  4: exact END ciphertext
}))
```

END padding uses CONTROL bucket/capacity rules even for a PAYLOAD segment.

### ArchiveFinalV1, record type 25

The last segment is CONTROL. It contains one DATA object whose complete bytes
are ArchiveFinalV1, then its END record, and nothing else.

| Tag | Type |
|---:|---|
| 1 | total segment count including terminal `u64be` |
| 2 | actual entry count `u64be` |
| 3 | actual total logical bytes `u64be` |
| 4 | actual unique Chunk count `u64be` |
| 5 | LAI `[32]` |
| 6 | PCR `[32]` |
| 7 | AUX `[32]` |
| 8 | recipient-set digest `[32]` |
| 9 | prior-segment sequence digest `[32]` |
| 10 | FooterCoreV1 digest `[32]` |
| 11 | authoritative encrypted Descriptor object ID `[32]` |
| 12 | authoritative encrypted manifest-root object ID `[32]` |

The prior sequence includes every completed nonterminal segment digest:

```text
SHA256(T1("entrybound/segment-sequence/v1", {
  1: nonterminal_count:u64be,
  2: each(length:u64be=32 || segment_digest[32]) in ordinal order
}))
```

Descriptor v2 itself carries the complete final resource/decode requirements
and semantic identities. ArchiveFinal repeats final identities/totals only as
the terminal physical authentication assertion required by the architecture;
the authenticated Descriptor/EAM remains semantic authority, and equality is
mandatory. A mismatch is corruption, never an alternate interpretation.

The reader verifies exact footer/EOF, public section digests, every relevant
segment and END, ArchiveFinal, Descriptor/EAM constraints and identities, and
PCI over the exact bytes. A missing footer or incomplete promised extent is
TRUNCATED. A present complete extent with a failed digest/tag/binding is
CORRUPT or INTEGRITY_MISMATCH as classified below.

## SignatureRecordV1, type 26

| Tag | Type and rule |
|---:|---|
| 1 | signature version `u16be = 1` |
| 2 | algorithm ID `u16be = 1` |
| 3 | binding mask `u8`; bit 0 required, no unknown bits |
| 4 | raw Ed25519 public key `[32]` |
| 5 | exact ContentBindingV1 transcript |
| 6 | optional exact PhysicalBindingV1 transcript; required iff bit 1 |
| 7 | optional exact AddressingBindingV1 transcript; required iff bit 2 |
| 8 | Ed25519 signature `[64]` |
| 9 | optional DER RFC 3161 token, maximum 64 KiB |

The signer identifier is derived from tag 4 and is not serialized.

```text
ContentBindingV1 = T1("entrybound/signature-content/v1", {
  1: LAI[32],
  2: AUX[32],
  3: "identity/v1",
  4: "ecf/bootstrap-v1",
  5: format_major:u16be,
  6: format_minor:u16be
})

PhysicalBindingV1 = T1("entrybound/signature-physical/v1", {
  1: PCR[32]
})

AddressingBindingV1 = T1("entrybound/signature-addressing/v1", {
  1: payload_suite_id:u16be,
  2: recipient_set_digest[32],
  3: commitment[32],
  4: archive_id[32]
})
```

The signed bytes are:

```text
T1("entrybound/signature/v1", {
  1: signature_version:u16be,
  2: signature_algorithm:u16be,
  3: binding_mask:u8,
  4: signer_public_key[32],
  5: ContentBindingV1,
  6: physical_present:u8,
  7: PhysicalBindingV1 or empty,
  8: addressing_present:u8,
  9: AddressingBindingV1 or empty
})
```

Ed25519 signs these bytes directly (pure mode), not a SHA-256 prehash. For a
timestamp, `SignatureRecordWithoutTimestamp` is the canonical type-26 record
through tag 8. The RFC 3161 `messageImprint.hashedMessage` is
`SHA256(SignatureRecordWithoutTimestamp)` and its algorithm is id-sha256. The
token itself is tag 9 and is excluded from that hash.

Embedded records are encrypted CONTROL objects. A detached `.ebsig` consists
of exactly one canonical type-26 record and no container wrapper.

## Recipient wrapping wire rules

The method context, wrap key, and flat 14-field wrap-AD formulas are normative
in [crypto-suite-v1.md](crypto-suite-v1.md). `RecipientStanzaV1` fields 1
through 8 are the exact sources for wrap-AD tags 7 through 14; fields 6 and 7
are copied as complete bytes, not digests or nested records. Field 9
`wrapped_afk` is the result and is not in AD. Additional method rules are:

- X-Wing field 6 is exactly the 39 ASCII bytes
  `xwing-mlkem768-x25519-sha3-256/draft-10`; field 7 is exactly 1120 draft-10
  encapsulation bytes. The 1216-byte recipient public key is not an implicit
  method-context/wrap-AD field: it determines the X-Wing shared secret through
  the frozen draft-10 KEM. Decapsulation returns a candidate
  32-byte secret even for an invalid KEM ciphertext according to the selected
  library's implicit-rejection contract; the wrap AEAD, commitment, and envelope
  MAC decide acceptance. Low-order/noncontributory X25519 behavior follows the
  pinned X-Wing draft and KATs, not an Entrybound special case.
- Password field 6 is exactly the 36 bytes `"A2ID" || 00000013 ||
  memory_kib:u32be || iterations:u32be || parallelism:u32be || salt[16]` and
  field 7 is exactly empty. T1 still emits tag 7 with `value_length = 0` in the
  method context and tag 13 with `value_length = 0` in wrap AD. The parameter
  structure is validated and policy checked before Argon2id. KDF/wrap failure
  is exposed as the same user-facing unlock failure as a wrong password.
- A matching identity attempt budget defaults to 4,096 `(identity, stanza)`
  trials and is caller-lowerable. Exceeding it is POLICY_REFUSED.

## Feature combinations

For a current v6-capable hybrid archive using bucketed/default boundaries, the
required bitmap is the archive's ordinary required features plus `0x20 | 0x40
| 0x80 | 0x400`. Strong boundaries additionally set `0x800`. Password mode
uses `0x100` instead of `0x80`. Embedded signatures add `0x200`.

The encrypted schema forbids `stream-layout-v1`. A detached signature does not
alter the archive feature bitmap. A future feature may add a recipient type but
cannot change the interpretation of any numeric value above.

## Stable diagnostic taxonomy

Diagnostics use the existing top-level classes. Public CLI output is
`<reason-code>: <concise detail>` and never Rust debug formatting.

| Reason code | Class | Meaning |
|---|---|---|
| `EB_CRYPTO_SUITE_UNSUPPORTED` | UNSUPPORTED | crypto/suite version or critical algorithm is unknown |
| `EB_CRYPTO_LAYOUT_UNSUPPORTED` | UNSUPPORTED | encrypted STREAM/unseekable output requested in v1 |
| `EB_CRYPTO_NO_MATCHING_RECIPIENT` | POLICY_REFUSED | supplied identities cannot unlock any supported stanza, or attempt budget exhausted |
| `EB_CRYPTO_RECIPIENT_STANZA_INVALID` | NONCONFORMING | stanza framing, sizes, reserved fields, type/class, or policy is invalid |
| `EB_CRYPTO_RECIPIENT_POLICY_INVALID` | NONCONFORMING | hybrid/password mixing, stanza cardinality, or protection policy mismatch |
| `EB_CRYPTO_ENVELOPE_AUTH_FAILED` | INTEGRITY_MISMATCH | authenticated wrap/envelope check failed; wrong-secret detail is suppressed |
| `EB_CRYPTO_KEY_COMMITMENT_FAILED` | INTEGRITY_MISMATCH | authenticated candidate AFK does not match stored commitment; same public unlock wording |
| `EB_CRYPTO_PASSWORD_KDF_POLICY_REFUSED` | POLICY_REFUSED | valid encoded Argon2id cost exceeds caller policy |
| `EB_CRYPTO_AEAD_AUTH_FAILED` | INTEGRITY_MISMATCH | protected-record tag failed; no plaintext released |
| `EB_CRYPTO_SEGMENT_STRUCTURE_INVALID` | NONCONFORMING | ordinal, counter, salt reuse, count, END, terminal, or ordering rule violated |
| `EB_CRYPTO_PADDING_INVALID` | NONCONFORMING | mode, bucket, private length, or padding framing invalid |
| `EB_CRYPTO_PRIVATE_OBJECT_INVALID` | NONCONFORMING | EBPO/EBCS magic, version, flags, kind agreement, extent, item type/order/cardinality, or inner canonical bytes are invalid |
| `EB_CRYPTO_BOUNDARY_MODE_UNSUPPORTED` | UNSUPPORTED | authenticated required boundary mode is not implemented |
| `EB_CRYPTO_RESOURCE_POLICY_REFUSED` | POLICY_REFUSED | crypto framing, identity attempts, or decrypted declared requirements exceed caller policy |
| `EB_SIGNATURE_ABSENT` | POLICY_REFUSED | signature was required by caller policy but none exists |
| `EB_SIGNATURE_UNSUPPORTED` | UNSUPPORTED | signature version/algorithm is unknown |
| `EB_SIGNATURE_INVALID` | INTEGRITY_MISMATCH | signature bytes or key encoding fail strict verification |
| `EB_SIGNATURE_STALE_CONTENT` | POLICY_REFUSED | valid signature's content binding differs and caller required it current |
| `EB_SIGNATURE_STALE_PHYSICAL` | POLICY_REFUSED | valid signature's physical binding differs and caller required it current |
| `EB_SIGNATURE_STALE_ADDRESSING` | POLICY_REFUSED | valid signature's addressing binding differs and caller required it current |
| `EB_SIGNATURE_TIMESTAMP_INVALID` | INTEGRITY_MISMATCH | supported timestamp token fails imprint, signature, chain, or time policy |
| `EB_SIGNATURE_TIMESTAMP_UNSUPPORTED` | UNSUPPORTED | token uses a non-v1 CMS/imprint/signature algorithm |

Wrong passwords, wrong private keys, KEM implicit-rejection outcomes, and wrap
tag failures all present the same public unlock message. Detailed internal
telemetry, if enabled, contains no secret/candidate material and is not emitted
across a trust boundary. Absent or stale signatures are ordinary reported
statuses when caller policy does not require them; the reason codes above are
returned only when that policy refuses the archive.

## Deterministic construction vectors

These vectors use lowercase hexadecimal and the exact `T1` definition in the
suite document. They are documentation/conformance inputs, not production
fixed randomness. The reference values were generated independently with
Python's SHA-256/HMAC/HKDF definitions and RustCrypto `aes-gcm-siv 0.12.1`.
The password output was cross-checked byte-for-byte with RustCrypto `argon2`
0.5.3 and the selected 0.6.0; the final signature was generated with
`ed25519-dalek 3.0.0`.

### V1: root hierarchy and commitment

```text
format             ecf/bootstrap-v1, version 0.1
crypto/suite       1 / 1
archive_id         000102030405060708090a0b0c0d0e0f
                   101112131415161718191a1b1c1d1e1f
AFK                202122232425262728292a2b2c2d2e2f
                   303132333435363738393a3b3c3d3e3f
layout/role         01 / 01
padding/boundary    01 / 01

root_salt          627e156c6c9792cbda3477909712769e
                   aa683efa2c938c689737c3e1fea6f8ff
root_prk           47fe02a0218467666c70f5aca6c8fc01
                   e67b3b75ceec9c405498c750745b2f0e
commitment_key     8ebf3989bc145eea11a334641019095b
                   ec09fa264f55d3f4365e899a49d60cdb
envelope_mac_key   65479e34546cacf6083804c20f1b3bc0
                   df802c926f37684c4069cf19bed2689e
control_root       73026777ba7515733ec28c7f69050cb1
                   1372a73d06ccd50cfeeb01faf0ef6d0c
payload_root       ebe5cab2cbf934472ab7f96785b8ab44
                   68c7f67118d8d3b511e1aa3ecdb3cbf0
default_table_key  2659b0e17cf2ee880df551bf040cb58f1
                   47471d615acb46268fede0c3c723cbc
strong_poly_key    035551a60590525f9529aab1ce233e41
strong_prf_key     f35f2c09669f609e858fc8ff02d81150
commitment         16bb3e788dce7f99545ae0fc098ccf2c
                   8c0087b8cf539095af977b30c3c7dcfc
```

### V2: segment derivation, nonces, and associated data

Using V1, PAYLOAD class `02`, ordinal 7, salt
`a0a1a2a3a4a5a6a7a8a9aaabacadaeaf`:

```text
segment_key        896e85aced3989236dff9b96e3f08ea3
                   7456b61c720de93b82db7391b16243bc
data_nonce(3)      000000000000000000000003
end_nonce(4)       ffffffff0000000000000004
```

For the exact 64-byte segment header below (four DATA records, complete extent
16,944) and protected header
`4542433100010100000000000000000700000000000000030000000000001010`:

```text
SegmentHeaderV1    45425347000102000000000000000007
                   a0a1a2a3a4a5a6a7a8a9aaabacadaeaf
                   00000004000000000000000000000000
                   00000000000042300000000000000000
SHA256(record_AD)  cf1e885dfd548e088f5405eb8c9a44d4
                   ba1573dabce0743a4fbaadcad8db841a
```

The conformance source retains and compares the complete AD bytes, whose hex
is:

```text
0019656e747279626f756e642f616561642d7265636f72642f7631000600010000
0000000000106563662f626f6f7473747261702d76310002000000000000000200
010003000000000000000200010004000000000000002000010203040506070809
0a0b0c0d0e0f101112131415161718191a1b1c1d1e1f00050000000000000040
45425347000102000000000000000007a0a1a2a3a4a5a6a7a8a9aaabacadaeaf
0000000400000000000000000000000000000000000042300000000000000000
000600000000000000204542433100010100000000000000000700000000000000
030000000000001010
```

### V3: envelope and recipient-set canonicalization

This small vector deliberately uses synthetic raw stanza byte strings `010203`
and `a0a1` to isolate set ordering.

```text
sort_key(010203)   06325ef9e18049f0b66560576cf54bb2
                   da8ede7fdbe7263f4b530986ea1ebdba
sort_key(a0a1)     c5db6f6e2d472c827118afe3f09a0ccf
                   db6e3e031ee6201cc659b128a3591169
canonical order    010203, a0a1
recipient_set      d25792586b6102e7d8d19e4dd96cbbef
                   18a409f05f0dc921166a5f0607bbc61c
```

Using V1 commitment/key, HYBRID_ONLY, required bitmap `0x4ef`, and the
synthetic sequence:

```text
public_ctx_sha256  2828ad02a7c05a014611cd130dfaca8e
                   40659a65e6afeea1c20eec60920539f1
envelope_core_sha  412b4b7317c38c303fbd78411b2bc9cb
                   db8e2caccad504a09632f16854489917
envelope_mac       d8ddfd1d6625436a0433bd0c8a6ed002
                   f214bbdd215761774c53962b1e47b2ee
```

### V4: X-Wing combiner

Use `ss_M=00..1f`, `ss_X=20..3f`, `ct_X=40..5f`, `pk_X=60..7f`, and label
`5c2e2f2f5e5c`:

```text
SHA3-256(combiner input)
  0acca09f2fb739bc89668dbcd01ae5ae
  bf9b72c6fe013297e3baa96854468491
```

The full KEM MUST also pass every draft-10 X-Wing KAT, including deterministic
key generation and encapsulation, from the draft's appendix. This compact
combiner vector is not a replacement for those KATs.

### V5: hybrid stanza wrapping

Use V1 AFK/archive ID; synthetic method secret `b0..cf`; stanza ID `40..4f`;
nonce `50..5b`; method parameters
`xwing-mlkem768-x25519-sha3-256/draft-10`; and 1120 encapsulation bytes where
byte `i` is `i mod 256`. Stanza type/class are `1/1`, stanza version is 1,
recipient hint is 16 zero bytes, and format version is 0.1. The complete
method-context and wrap-AD transcript hex are published as
`V5_METHOD_CONTEXT` and `V5_WRAP_AD` in
[crypto-wire-v1-vectors.txt](crypto-wire-v1-vectors.txt); they are normative,
not abbreviated hashes.

```text
encapsulation_sha  0de689b3c273e26569eaac258d83fd685
                   f7bd0c0e383e6c5e50895b65dbc8982
method_ctx_sha256  3e48bbd23eb809b4b599abe0beb3561b
                   749790ce9e01115e2d9f4250d1b854a3
wrap_prk           1692c5a48fea3da45fb59b22f9072534
                   f8390d8b24ec47a47779dbaacb3d1c76
wrap_key           0762e9b9abfdeb51baa1d748f538d802
                   8de80db7e0296b062a476f58cc3098f2
wrap_AD_sha256     12334742b8d3e5457ae34a7c72ae30c1
                   26dd506587a681f6a24b5c6c2fe171a0
wrapped_AFK        db49535140afd435d92500c4eaf6e77f
                   72bcf0fc469aa36c76775a44503ba19d
                   192b1705f345960f1a72c4cec9541603
```

The previous V5 wrap-AD hash
`14ba444ac3a7cb223d6dcc90d21740bbcf35bac726b2b20903d5532573f3493d`
and its `c550...7d73` wrapped AFK are superseded. They were produced before
`recipient-wrap-ad/v1` had normative fields, so they could not define or test
an independently implementable wire construction. Method context, wrap PRK,
and wrap key are unaffected.

### V6: password KDF and wrapping

Use V1 AFK/archive ID; password exact UTF-8 `correct horse battery staple`;
salt `60..6f`; default Argon2id parameters; stanza ID `70..7f`; and nonce
`80..8b`. Stanza type/class are `2/2`, stanza version is 1, recipient hint is
16 zero bytes, and format version is 0.1. The complete `A2ID`, method-context,
and wrap-AD bytes are `V6_A2ID`, `V6_METHOD_CONTEXT`, and `V6_WRAP_AD` in
[crypto-wire-v1-vectors.txt](crypto-wire-v1-vectors.txt).

```text
parameter_bytes    41324944000000130004000000000003
                   00000004606162636465666768696a6b
                   6c6d6e6f
argon2id_output    b954ca2999c51dfbd1810dad53340641
                   d507696a416ec59334f7e18bf823ea2d
method_ctx_sha256  fa8bc3f2394fabcf754b735a54fd953b
                   04f01096584001b455fb41ecd94d6486
wrap_prk           f04d722afb3931d82d594cd7400d1d59
                   70abcbe2f3941e40ee5ad2c961529dd2
wrap_key           46b449b0eb4d76fa2ff054606c7a0f2d
                   3ee44b4661a2ccae24d1d9828a0a2945
wrap_AD_sha256     56e84a074663fc268c5f5298cbdad6f7
                   cc84731814c6113e136f484588c105df
wrapped_AFK        e183d98d5694ee3dc723e9d43cdd56d9
                   2889035d3f7c11e9492236f43115fe20
                   a53c6725b498e1288e8fb12aae7ff573
```

The previous V6 wrap-AD hash
`a8aee7d38f9756adf84cd953e73f3a812b01afa174101d386fdde3b570427a4e`
and its `f393...2c81` wrapped AFK are superseded for the same missing-normative-
encoding defect. The A2ID bytes, Argon2id output, method context, wrap PRK, and
wrap key are unaffected.

### V7: signature transcript hashes

Use LAI `00..1f`, AUX `20..3f`, PCR `40..5f`, V3 recipient-set digest, V1
commitment/archive ID, all three bindings, signature version/algorithm 1.

```text
SHA256(ContentBindingV1)
  6eca79bc2e0c661220b93b39d01b84be
  6133c6768bf4e46498e48f94fd9cf3bf
SHA256(PhysicalBindingV1)
  db7470304e63ef04aa2a83e90b252ccf
  9a0462304579f9822584ae8af36458ef
SHA256(AddressingBindingV1)
  f37f674b7927d755ed3fbbf223315222
  74b6825869d10499bc2941cca62837ab
```

Use binding mask `07` and RFC 8032 test-key-1 public key
`d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a`.
The exact 600-byte final signature transcript has:

```text
SHA256(SignatureTranscriptV1)
  68b91370f571119013e73530df0996aa
  51845387d839f136ae569ca279587f50
Ed25519 signature
  3ba203ed99bc3050b9815966f4d05da1
  2b7133a9416457280090d063b2282e13
  bb3e3be1c0ea737136f52838bf43050d
  57f6c394d8df4040bf204cec9f583a0f
```

The seed is RFC 8032 test-key-1
`9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60`.
The implementation must reproduce the signature and also pass all applicable
RFC 8032 section 7.1 vectors.

### V8: private-object and canonical-sequence containers

The complete canonical and deliberately malformed byte strings are published
without abbreviation as `S1_*` through `S7_*` in
[crypto-wire-v1-vectors.txt](crypto-wire-v1-vectors.txt). The generators build
the ECF records and container framing independently rather than consuming those
hex strings.

`S1` is an empty MANIFEST container and is the canonical empty encoding:

```text
S1_EMPTY         4542435300010001000000000000000000000000
SHA256           6d025124515cac937fc17001ee46b15c
                 11ee51ace780180438b4fb65f5e1666f
```

`S2` is one RECIPIENT_DIRECTORY item. Its type-22 record uses stanza ID
`00..0f`, stanza type 1, fingerprint `20..3f`, and label `alice`. `S3` adds a
second correctly ordered item with stanza ID `10..1f`, fingerprint `40..5f`,
and an empty label. `S4` wraps the exact S3 bytes in an `EBPO` kind-3 private
object. Their hashes are:

```text
S2_ONE_SHA256                    8da189b0c457fdf77f938d124b11a2f1
                                 c6bca6359579bc0d1f78a118fd13e973
S3_MULTI_SHA256                  b05bb05eb84ebfa6ba8ff8b79e35c518
                                 0c7c0be9212ab196adf68ef0a1ed4bb6
S4_PRIVATE_CONTROL_OBJECT_SHA256 50e9456d665b00eccc1d3ede310d2e12
                                 6b75b695f4a982329b6adab390340305
```

The negative vectors retain structurally explicit bytes so a parser can prove
it rejected the intended rule:

```text
S5_OUT_OF_ORDER_INVALID_SHA256  8a551fca35b6efe614874cd00ddfb796b
                                d0cd765de7e4355bf1bf97752a55adc
expected                        EB_CRYPTO_PRIVATE_OBJECT_INVALID

S6_DUPLICATE_INVALID_SHA256     2d3c0c6868d7adb4e56c27f17127cc22
                                3947477cd924e20935549aed5ec3febb
expected                        EB_CRYPTO_PRIVATE_OBJECT_INVALID

S7_TRUNCATED_INVALID_SHA256     91f9a7f2275c8669a490f74b3b09b346
                                a53a55a9585a118f62346e9ff4b6b3ec
expected                        EB_CRYPTO_PRIVATE_OBJECT_INVALID
```

S5 reverses S3's items, S6 repeats its first record exactly, and S7 removes the
last byte from S2 while retaining the declared item length. If the containing
archive itself ends before its promised fragment/ciphertext/footer extent that
is `TRUNCATED`; S7 is instead a complete authenticated object whose inner item
length exceeds its bytes and is therefore nonconforming.

### Required external primitive vectors

Before production release, the implementation test suite must include:

- AES-256-GCM-SIV vectors in RFC 8452 Appendix C.2, including failed-tag and
  counter-wrap cases;
- HKDF-SHA-256 RFC 5869 Appendix A vectors;
- HMAC-SHA-256 [RFC 4231](https://www.rfc-editor.org/rfc/rfc4231) vectors;
- Argon2id RFC 9106 section 5.3 and V6;
- ML-KEM-768 NIST ACVP/FIPS 203 vectors and invalid-ciphertext behavior;
- every X-Wing draft-10 vector and at least one independent implementation;
- Ed25519 RFC 8032 vectors plus Project Wycheproof invalid encodings;
- boundary vectors for table derivation, PHTE field/rejection sampling,
  rolling update versus direct evaluation, normalization, and empty/tail cases.

Fixed test randomness is accepted only through `cfg(test)` constructors that
are absent from production builds.
