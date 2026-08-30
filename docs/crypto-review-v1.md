# Entrybound cryptographic architecture review v1

Review date: 2026-08-30. Status: **security-design gate re-authorized after the
canonical-wire correction below; production implementation not started**.

This is the rationale and adversarial-review record for the frozen choices in
[crypto-suite-v1.md](crypto-suite-v1.md) and
[crypto-wire-v1.md](crypto-wire-v1.md). It is not a claim that documentation or
unit tests alone establish security. A production implementation requires code
review, standard and cross-implementation vectors, fuzzing, dependency review,
and an external cryptographic audit.

## Sources and review method

The review used current primary standards and specifications, not legacy
archive precedent:

- [RFC 8452](https://www.rfc-editor.org/rfc/rfc8452), AES-GCM-SIV;
- [RFC 9771](https://www.rfc-editor.org/rfc/rfc9771), AEAD property taxonomy
  including key commitment;
- [RFC 5869](https://www.rfc-editor.org/rfc/rfc5869), HKDF;
- [RFC 2104](https://www.rfc-editor.org/rfc/rfc2104) and
  [RFC 4231](https://www.rfc-editor.org/rfc/rfc4231), HMAC and vectors;
- [FIPS 180-4](https://csrc.nist.gov/pubs/fips/180-4/upd1/final), SHA-256;
- [FIPS 203](https://csrc.nist.gov/pubs/fips/203/final) and
  [NIST SP 800-227](https://csrc.nist.gov/pubs/sp/800/227/final), ML-KEM and
  KEM use;
- [X-Wing draft 10](https://datatracker.ietf.org/doc/html/draft-connolly-cfrg-xwing-kem-10),
  the concrete ML-KEM-768/X25519 hybrid;
- [RFC 7748](https://www.rfc-editor.org/rfc/rfc7748), X25519;
- [RFC 9106](https://www.rfc-editor.org/rfc/rfc9106), Argon2id;
- [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032), Ed25519;
- [RFC 3161](https://www.rfc-editor.org/rfc/rfc3161) and
  [RFC 5816](https://www.rfc-editor.org/rfc/rfc5816), timestamp tokens;
- Truong et al., [*Breaking and Fixing Content-Defined
  Chunking*](https://eprint.iacr.org/2025/558.pdf), CCS 2025;
- Albertini et al., [*How to Abuse and Fix Authenticated Encryption Without
  Key Commitment*](https://www.usenix.org/system/files/sec22-albertini.pdf),
  USENIX Security 2022.

Candidate selection considered standardized meaning, misuse behavior,
cryptanalytic history, implementation and interoperability availability,
hardware dependence, bounded resource behavior, license, and long-term
decodability. The second pass then treated the frozen draft as hostile and
looked for ambiguity, downgrade, replay, nonce, resource, and lifecycle gaps.

## Payload AEAD comparison

### Selected: AES-256-GCM-SIV

RFC 8452 gives a fixed 32-byte-key, 12-byte-nonce, 16-byte-tag construction,
published vectors, misuse resistance, and independent implementations. It can
use common AES acceleration, while remaining implementable portably. A
two-pass encryptor is acceptable for archive records because Entrybound already
has bounded independently framed physical objects.

Misuse resistance narrows the damage from an accidental repeated nonce, but
Entrybound still uses random segment salts, derived keys, structured nonces,
and rejects repeated ordinals. RFC 8452's instruction not to release plaintext
before tag confirmation matches the required reader architecture.

### Rejected payload candidates

| Candidate | Reason not selected for suite 1 |
|---|---|
| AES-256-GCM | Extremely mature and fast, but nonce reuse is catastrophic; the archive format can afford GCM-SIV's safer failure mode. |
| ChaCha20-Poly1305 | Strong portable software performance and an RFC, but ordinary nonce misuse is not resistant and the 96-bit nonce offers no v1 advantage over GCM-SIV's structured domain. |
| XChaCha20-Poly1305 | Large nonces are operationally attractive, but the construction does not provide GCM-SIV-style misuse resistance and lacks an IETF Standards Track/RFC definition comparable to the selected suite. |
| AES-SIV (RFC 5297) | Strong misuse resistance, but less AEAD ecosystem/interoperability support, larger key material, and deterministic operation unless an explicit nonce is included. V1 forbids deterministic ciphertext. |
| Ascon-AEAD128 | Standardized and attractive on constrained platforms, but offers a 128-bit-key profile, has a shorter deployment/cryptanalysis history, and had less mature Rust archive-scale interoperability at the freeze date. |
| AEGIS/Deoxys and other CAESAR-era candidates | Good properties in some profiles, but not enough standards plus stable independent Rust implementations to justify a long-lived archive suite over RFC 8452. |

No user cipher choice was retained. Algorithm agility is a new authenticated
suite/version, not negotiation within one archive.

## KDF, hash, MAC, and commitment review

HKDF-SHA-256 was selected because extract/expand semantics, salt handling,
limits, and vectors are stable in RFC 5869. SHA-256 already underlies
Entrybound's logical identities and has pervasive independent support. HMAC is
used only for the explicit commitment and envelope authentication; full
32-byte tags avoid a new truncation analysis.

A raw `SHA256(AFK || context)` commitment was rejected because it is an ad hoc
keyed-hash use. Treating AES-GCM-SIV's successful decryption as key commitment
was rejected because AEAD authentication and key commitment are separate
properties in RFC 9771 and the commitment literature. A separately derived
HMAC key and transcript makes the intended property explicit and testable.

BLAKE3 derive-key mode and KMAC were considered technically credible, but they
would introduce a second less-universal KDF/MAC family without a security need.
SHA-512/HKDF-SHA-512 adds no useful suite property here. Key separation is
achieved by HKDF labels and independent control/payload/boundary roots.

## Segment and nonce review

The design avoids mutable cross-process nonce state. A fresh archive ID enters
the root extraction context; independent class roots separate control and
payload; each segment has random salt and a globally unique ordinal; nonce
prefixes separate DATA from END. Even a repeated 16-byte segment salt cannot
repeat a key when class/ordinal differs. Duplicate ordinals are rejected before
decrypting.

Limits (`2^20-1` DATA messages, 1 GiB private data, 64 MiB maximum payload
record) are much lower than primitive maxima and bound counter, buffering, and
failure scope. SegmentEnd binds ordered record headers/ciphertexts. A terminal
ArchiveFinal binds prior completed segments, resources, identities, recipient
set, Descriptor/manifest objects, and intended footer. This covers reordering,
truncation, and splicing without putting a mutable global nonce counter outside
the file.

Absolute offsets are not AEAD AD for bulk records, allowing an envelope-size
change to move ciphertext without re-encrypting it. They are validated through
the authenticated footer core in ArchiveFinal. Exact segment and record
headers *are* AD. This division was checked for an unauthenticated field that
could change accepted meaning; none remains. At worst, a pre-authentication
locator edit causes bounded parsing work or a failure.

## Metadata and layout decision

Only crypto discovery, recipient discovery, and minimal ciphertext framing are
public. The old preamble budget fields become zero sentinels under the new
required feature; real budgets are encrypted. This avoids leaking entry count,
logical size, names, manifests, Index, codec choices, identity roots, and
signers.

Encrypted STREAM was rejected for crypto v1. Current STREAM intentionally puts
physical content before the manifest that describes it. Giving a sequential
reader equivalent name/metadata privacy and final authentication would require
staging or new ordering semantics, while stdin/stdout encryption would make
secure error recovery and key-envelope updates materially different. V1 fails
before output rather than buffering secretly, changing layout silently, or
leaking the manifest. This retains one confidentiality contract.

## Hybrid post-quantum recipient review

### Selected: X-Wing draft 10

X-Wing supplies the requested concrete hybrid without Entrybound inventing a
combiner. It combines standardized ML-KEM-768 with RFC 7748 X25519 and a fixed
SHA3-256 combiner. Its public key, ciphertext, and shared secret are fixed-size,
and the draft supplies a reference specification and deterministic KATs. The
security argument is explicitly about this concrete ML-KEM/X25519 combination;
Entrybound does not generalize its combiner.

The wire name includes `draft-10`. A later RFC or algorithm change does not
alter these bytes. X-Wing was still an active Internet-Draft on the review date,
so production release is gated on exact draft-10 KATs and at least one
independent implementation. This is a tracked implementation-maturity risk,
not an unresolved format decision.

### Rejected hybrid alternatives

- ML-KEM-768 alone fails the required classical hedge.
- X25519 alone fails harvest-now-decrypt-later confidentiality.
- Concatenating component secrets and applying an Entrybound-defined HKDF was
  rejected as a custom hybrid combiner.
- TLS-specific X25519/ML-KEM constructions were rejected because their security
  relies on a TLS transcript not present in an archive.
- A generic configurable KEM combiner was rejected because it adds algorithm
  combinations and negotiation when X-Wing provides a reviewed concrete
  profile.

No classical-only creator is included. This is simpler than offering a switch
that accidentally weakens hybrid confidentiality. Password mode is kept
separate because its security is governed by human entropy, not a public-key
recipient.

## Password construction review

Argon2id v19 was selected for memory hardness, hybrid data-independent/data-
dependent behavior, standard encoding semantics, and broad maintained
implementations. PBKDF2 was rejected for cheap GPU/ASIC parallelism; scrypt is
credible but Argon2id has the more current password-hashing standard and more
explicit parallelism/memory semantics. Argon2d alone has less side-channel
resistance; Argon2i alone requires more passes for comparable tradeoffs.

The 256 MiB, three-pass, four-lane creation profile is aimed at infrequent
offline archives, not high-throughput login servers. Wire bounds prevent a
malicious archive from demanding arbitrary resources, while caller maxima can
be lower. Raw-password bytes and raw-hash APIs avoid PHC-string parsing and
normalization ambiguity.

The public commitment and wrapped-key tag make offline guesses testable. The
design does not claim to remove that oracle. Password mode therefore cannot
borrow the security label of hybrid recipients and cannot be mixed with them.

## Padding and keyed-boundary review

Quarter-octave buckets were chosen over powers-of-two buckets to cap
proportional overhead below 25% for records above the minimum while still
coarsening length observations. Maximum padding is intentionally expensive and
does not hide record count. No padding exposes exact private-record length. All
three modes are authenticated and inspectable.

The CCS 2025 KCDC analysis recovers keys from several deployed folklore
secret-chunking schemes under weak known/chosen-plaintext conditions. For that
reason, the default secret Gear table is documented only as defense in depth.
The stronger option instantiates the paper's universal-polynomial-hash then AES
PRF construction with exact field/window/encoding rules. The paper's measured
AES-per-byte cost (roughly 53%--165% slowdown in its patched Restic experiments)
supports keeping the strong mode explicit rather than silently imposing it.

Padding and keyed boundaries mitigate different observations. Neither is
described as total traffic-flow or size hiding. The last Chunk, archive total,
record count, remote requests, and some within-archive equality remain visible.

## Signature review

Pure Ed25519 was selected for deterministic signing, fixed compact encodings,
widely implemented RFC vectors, simple key representation, and avoidance of a
per-signature random-nonce failure. ECDSA P-256 was rejected for more complex
encoding/nonce/malleability handling; RSA-PSS for much larger keys/signatures;
ML-DSA for substantially larger artifacts and a newer operational ecosystem.
The threat motivating post-quantum encryption (record now, decrypt later) does
not itself require a post-quantum signature today. A future signature algorithm
gets a new authenticated algorithm ID and feature.

Signing the canonical stored binding values, then separately comparing each to
the current archive, is important. It distinguishes forgery (`INVALID`) from a
legitimate historical statement whose content, physical organization, or
recipient addressing is now `STALE`. Addressing includes archive ID to stop a
signature over the same recipient set/commitment from being forwarded to a
different encryption epoch.

RFC 3161 timestamps are optional and separately statused. V1 limits imprint to
SHA-256 and CMS signer to Ed25519 to avoid an open-ended archive-controlled PKI
algorithm list. No network fetch is required or performed.

## Preferred Rust dependencies

Versions are exact intended pins for the implementation task. Cargo's project
MSRV is already Rust 1.94; all selected primitive crates declare MSRV 1.89 or
lower, so the freeze requires no MSRV increase. Default features should be
disabled where they add unused serializers, password strings, or RNG entry
points. Entrybound itself retains `unsafe_code = "forbid"`; dependency unsafe
and architecture-specific code is reviewed through the lockfile and audit
process.

| Purpose | Preferred crate/version | Review |
|---|---|---|
| AES-GCM-SIV | `aes-gcm-siv = =0.12.1` | Pure Rust, RFC 8452, MIT/Apache-2.0, MSRV 1.85, optional hardware acceleration, no native toolchain. Run every RFC vector and failed-tag case. |
| AES-128 for PHTE | `aes = =0.9.3` | Pure Rust, MIT/Apache-2.0, MSRV 1.89. Use only the block-encrypt API; never expose hazmat AES outside the chunker. |
| HKDF | `hkdf = =0.13.0` | Pure Rust, MIT/Apache-2.0, MSRV 1.85, RFC 5869 vectors. |
| HMAC | `hmac = =0.13.0` | Pure Rust, MIT/Apache-2.0, MSRV 1.85, RFC 4231 vectors, constant-time verification API. |
| SHA-256 | `sha2 = =0.11.0` | Pure Rust, MIT/Apache-2.0, MSRV 1.85; already aligns with ECF identity choice. |
| X-Wing | `x-wing = =0.1.0` with `zeroize` | Pure Rust, MIT/Apache-2.0, MSRV 1.85, `unsafe_code = deny`; published crate names draft 06. Draft 06--10 algorithm bytes are unchanged, but exact draft-10 KAT/interoperability is a mandatory gate. No independent audit is claimed. |
| ML-KEM transitive pin | `ml-kem = =0.3.2` | Pure Rust, FIPS 203, MIT/Apache-2.0, MSRV 1.85. Pin at or above the release containing public-key validation fixes and test malformed encodings/implicit rejection. |
| X25519 transitive pin | `x25519-dalek = =3.0.0` | Pure Rust, BSD-3-Clause, MSRV 1.85. Used only through X-Wing's frozen API. Dalek predecessors received a Quarkslab review in 2019; that is not represented as an audit of this exact release. |
| SHA3 transitive pin | `sha3 = =0.12.0` | Pure Rust, MIT/Apache-2.0, MSRV 1.85; used by the X-Wing implementation, not a local combiner. |
| Argon2id | `argon2 = =0.6.0` | Pure Rust, MIT/Apache-2.0, MSRV 1.85. Use raw `hash_password_into` with exact params; do not use PHC parsing for the wire construction. |
| Ed25519 | `ed25519-dalek = =3.0.0` | Pure Rust, BSD-3-Clause, MSRV 1.85, default zeroization. Use strict verification plus explicit public-key validation; RFC and Wycheproof vectors. Older Dalek code was independently reviewed, but this exact version requires normal dependency review. |
| OS randomness | `getrandom = =0.4.3` | Rust with platform backends, MIT/Apache-2.0, MSRV 1.85. RNG failure is propagated, never replaced with a PRNG seed. |
| Secret erasure | `zeroize = =1.9.0` | Pure Rust, MIT/Apache-2.0, MSRV 1.85. Apply to AFK, KDF output, segment/wrap keys, password buffers, and expanded private keys where ownership permits. |
| DER/CMS timestamp parsing | `der = =0.8.1`, `x509-cert = =0.3.0`, `cms = =0.3.0-pre.2` | Pure Rust, MIT/Apache-2.0, MSRV 1.85. `cms` is pre-release, so timestamp support is separately gated and may initially report `TIMESTAMP_UNSUPPORTED`; core signatures do not depend on it. |

No selected production path requires OpenSSL, liboqs, a subprocess, or another
native library. `libcrux-kem 0.0.9` and the X-Wing team's Apache-2.0 C reference
are independent test oracles, not runtime dependencies. Libcrux's high-assurance
work is valuable, but its `<0.1` KEM surface and unknown declared MSRV make it a
less stable primary API at this freeze. This decision can change before code is
merged only if the replacement passes the *same frozen draft-10 bytes*; it does
not reopen the wire construction.

No selected primitive crate's existence is treated as proof of security. The
implementation review must capture the exact lockfile, features, advisory
status, vector results, and unsafe/native transitive surface.

## Test and interoperability plan

The deterministic Entrybound vectors are in
[crypto-wire-v1.md](crypto-wire-v1.md). Implementation acceptance additionally
requires:

1. every applicable RFC/FIPS/X-Wing known-answer vector;
2. negative tags, noncanonical encodings, invalid ML-KEM/X25519 inputs, and
   strict Ed25519 vectors;
3. cross-check of X-Wing keygen/encapsulation/decapsulation against libcrux and
   the draft reference or C implementation;
4. cross-check of AES-GCM-SIV records with an implementation independent of
   RustCrypto;
5. root/commitment/envelope/segment/AD/recipient-set/signature vectors on at
   least two language implementations before the format is called stable;
6. property tests that no two generated records share `(key, nonce)`, direct
   and rolling PHTE evaluation agree, and stanza input order cannot affect the
   set digest;
7. fuzzing at public framing, canonical records, stanza parameters, fragment
   reassembly, DER/CMS, and authenticated-private parsing boundaries;
8. fault tests for RNG failure, partial writes, corrupted END/final/footer,
   interrupted key edits, and cleanup/zeroization paths;
9. policy tests proving KDF/memory/length/attempt rejection occurs before the
   expensive operation or allocation;
10. test-only fixed randomness APIs compiled out of non-test builds.

## Canonical-wire blocker resolution (2026-08-30)

The implementation review correctly stopped before adding production crypto.
Two security-critical byte grammars were not actually frozen:

1. `recipient-wrap-ad/v1` named concepts but did not assign T1 tags or exact
   stanza encodings;
2. `PrivateFragmentV1` allowed an undefined “canonical sequence container” and
   required a parser to distinguish records, Chunk frames, and collections
   without an explicit discriminator.

Repository history, the original review text, all checked-in documents, and
the empty vector-helper location contained no earlier field table or generator
from which one unique wrap-AD encoding could be recovered. The old V5/V6 AD
hashes therefore were evidence of one unrecorded generator execution, not
normative format semantics. No transcript search or brute-force preservation
was attempted.

The authorized minimal correction is a flat, required, 14-field T1 transcript.
It binds namespace and format version, crypto/suite, archive ID, and the exact
public stanza fields through nonce. Method parameters and encapsulation are
complete values, not digests or a partially serialized stanza. `wrapped_afk`
is excluded as the output. X-Wing uses the exact 39-byte draft-10 identifier
and 1,120-byte encapsulation; password uses the exact 36-byte A2ID value and a
zero-length encapsulation. The recipient public key remains an X-Wing KEM
input, not an undocumented transcript field. Comparison of the pinned crate's
three bundled draft-10 vector cases with the authoritative draft-10 vector data
found identical seed, encapsulation seed, keys, ciphertext, and shared-secret
values; this correction does not alter X-Wing.

The private-object correction adds a 12-byte authenticated `EBPO` envelope
whose kind explicitly dispatches a single canonical ECF record, one exact
Chunk frame, or one `EBCS` collection. `EBCS` has fixed magic/version/kind/zero
flags, a `u64be` count, and nonzero `u64be`-length-delimited canonical ECF
records. Nine collection kinds define allowed record types, ordering,
cardinality, and duplicate rules. It is self-delimiting through the enclosing
authenticated object's already-authoritative extent; it does not add another
total-length fact. Nesting and unknown items are forbidden. This preserves the
inner record/frame/collection kind as semantic authority while making the
outer kind purely physical parser dispatch.

The audit also found and corrected three adjacent specification defects:

- the signature feature row incorrectly said type 27 although
  `SignatureRecordV1` is type 26;
- record 22 and 27 collection items lacked canonical fields, so they are now
  explicitly `RecipientDirectoryEntryV1` and `EncryptedIndexEntryV1` inside
  typed EBCS collections;
- the timestamp prose named an undefined `SignatureCoreV1`; it now points to
  the exact canonical type-26 record through tag 8 already defined as
  `SignatureRecordWithoutTimestamp`.

V5 and V6 method-context hashes, HKDF PRKs, and wrap keys did not change. Their
wrap-AD hashes and AES-GCM-SIV ciphertext/tags did change because those bytes
had never had a normative definition. The old values are explicitly
superseded in the wire document. Complete transcript bytes and sequence vectors
are published in
[crypto-wire-v1-vectors.txt](crypto-wire-v1-vectors.txt). A standalone
RustCrypto helper and a Python implementation using independent T1/ECF/EBCS
construction plus `cryptography` and `argon2-cffi` agree on all 35 named
outputs. Neither helper is linked into Entrybound production code.

The focused adversarial re-review reached these conclusions:

- stanza-type, protection-class, stanza-ID, method-parameter, encapsulation,
  nonce, archive, format, and suite substitution changes wrap AD and, for the
  method fields, the wrap key context as well; no alternate nested encoding
  exists;
- canonical T1 and exact fixed/registered lengths prevent alternate stanza
  encodings from representing the same accepted v1 value;
- `EBPO` prevents record/Chunk/collection type confusion before inner parsing;
- `EBCS` magic/version/kind/flags/count/item lengths and exact outer extent
  prevent concatenation, count, length, and truncation ambiguity;
- kind-specific order and unique semantic keys reject reordering and
  duplicates; nested containers and unknown item types fail closed.

No blocker remains from this correction. The encrypted-INDEXED implementation
may proceed from the normative suite/wire documents without inventing crypto
wire bytes.

## Independent adversarial pass

The second pass began from the assumption that every public byte is controlled
by a persistent adversary and every decoded length/parameter by a malicious
producer. Findings are numbered for future audit traceability.

| Finding | Risk | Resolution | Status |
|---|---|---|---|
| AR-01 Nonce reuse through a global counter reset | Two writers or interrupted state could repeat GCM-style nonces. | No external state: class-separated roots, random segment salt, global ordinal, structured counter, disjoint END prefix; duplicate ordinals rejected. | Resolved |
| AR-02 Same salt across control/payload or archives | Could collide derived keys. | Class-specific roots plus archive ID in root/segment context; salt alone never defines the key. | Resolved |
| AR-03 AEAD treated as key committing | Multi-key ciphertext/recipient ambiguity would remain. | Separate derived 32-byte HMAC commitment verified before AFK acceptance. | Resolved |
| AR-04 Recipient stripping/injection/reordering | Could change who decrypts or what an addressing signature means. | MAC and recipient-set digest cover canonical full stanzas, including unknown; ordering is hash-key then raw bytes. | Resolved |
| AR-05 Unknown stanza downgrade | An unknown classical method could be mislabeled hybrid. | Authenticated protection class/policy, hybrid-only writer registry, unknown class fail closed. Unknown method is only skipped for matching; a producer/recipient already holding AFK can leak it regardless. Addressing signature detects edits. | Resolved |
| AR-06 Public header changes decode | Unauthenticated length/type/offset could create alternate meaning. | Exact segment/record headers are AD; semantic type/length is private; footer locators match authenticated FooterCore. Public edits can only cause bounded failure. | Resolved |
| AR-07 Truncate after a valid Chunk | Per-record AEAD alone would accept a prefix. | Mandatory SegmentEnd and one terminal ArchiveFinal/footer/EOF binding. | Resolved |
| AR-08 Reorder or cross-archive splice | Individually valid ciphertext might be moved. | Archive ID, segment class/ordinal/header, counters, ordered END digests, and ArchiveFinal root are bound. | Resolved |
| AR-09 Whole valid archive replay | Self-contained verification cannot know which historical copy is newest. | Explicit non-goal; caller expected PCI/signature/timestamp/external state supplies freshness. | Resolved by scope |
| AR-10 Password KDF denial of service | Archive chooses huge memory/time/parallelism. | Hard wire bounds, checked arithmetic, caller policy before allocation/work, attempt budget, separate reason code. | Resolved |
| AR-11 Password oracle obscured | Public commitment/wrap tag permits offline guesses. | Explicitly documented; meaningful default KDF, strong-password UX, no mixing with hybrid. | Resolved by disclosure/design |
| AR-12 Boundary privacy overclaim | Secret Gear tables in deployed systems have been broken; padding can be inverted statistically. | Default labeled defense in depth, exact bucket leakage documented, optional paper-based PHTE mode, no “hidden sizes” claim. | Resolved |
| AR-13 Cross-archive equality | Convergent encryption/content keys reveal known plaintext and equality. | Random AFK/archive ID, AFK-derived boundaries, archive-local exact dedup only. | Resolved |
| AR-14 Encrypted STREAM metadata leak | Current sequential order exposes data before private manifest and complicates final commit. | Crypto v1 is INDEXED-only and fails before any unseekable output. | Resolved |
| AR-15 Signature forwarding | Same identities/envelope values might be asserted for another encryption epoch. | Addressing transcript includes random archive ID as well as suite/set/commitment. | Resolved |
| AR-16 Stale versus invalid signature ambiguity | Legitimate reorganization could look forged or a forged value could look merely stale. | Verify signature over stored bindings first; only a valid signature receives independent current-value VALID/STALE statuses. | Resolved |
| AR-17 Recipient removal described as metadata edit | A removed recipient retains AFK and can decrypt unchanged ciphertext. | Removal rotates AFK/archive ID, rechunks keyed boundaries, regenerates stanzas, and fully re-encrypts; no revocation claim for old copies. | Resolved |
| AR-18 Recipient addition described inconsistently | Architecture sections disagreed whether add was envelope-only or full re-encryption. | Add preserves AFK/archive ID/bulk ciphertext and re-authenticates envelope/private directory/terminal footer control; addressing stale, PCR unchanged. | Resolved specification contradiction |
| AR-19 Recipient identification leaks stable public key ID | Public fingerprints allow correlation. | Fixed-zero public hints; authenticated private recipient directory for authorized management. | Resolved |
| AR-20 Key-add footer circularity/nonce reuse | Changed envelope length changes offsets; rewriting final plaintext under old nonce would violate nonce rules. | FooterCore excludes circular segments hash; terminal control segment is regenerated with fresh salt/nonces. Bulk payload remains untouched. | Resolved |
| AR-21 Duplicate semantic authority in AD | Copying logical length/plan/digest into headers could permit conflicting facts. | Public header owns ciphertext length only; encrypted inner ECF record/frame owns semantics. | Resolved |
| AR-22 Fragment mix-and-match | Large encrypted object fragments could be spliced. | Object digest, total length, ordinal/count/offset, contiguity, segment authentication, and complete inner canonical parse all required. | Resolved |
| AR-23 Resource declaration hidden until decrypt | Reader might allocate before seeing policy-relevant requirements. | Fixed small framing limits apply first; control records capped; encrypted Descriptor is authenticated and policy-checked before dependent payload decode. | Resolved |
| AR-24 Recipient error oracle | Distinct wrong-password, invalid-KEM, tag, commitment messages improve probing. | Stable internal reason codes, but public unlock wording collapses secret-dependent candidate failures and erases candidates. | Resolved |
| AR-25 X-Wing draft/library maturity | Preferred crate identifies draft 06 and has no claimed audit; draft 10 is not final. | Pin draft-10 wire, require official KATs plus two independent implementations and dependency review before shipping. If the crate fails, implementation is blocked rather than hand-rolling. | Resolved as release gate |
| AR-26 Deterministic test RNG escapes | Fixed vectors could accidentally become a CLI/API mode. | Fixed randomness exists only under `cfg(test)` and production constructors own OS randomness with fatal failure. | Resolved |
| AR-27 Legitimate recipient forwarding | Recipient can share AFK/plaintext or create another archive. | Explicit non-goal; addressing signatures express author intent but cannot provide DRM. | Resolved by scope |
| AR-28 Timestamp algorithm sprawl/network | CMS can import many algorithms, parsers, and online trust behavior. | 64 KiB DER bound, SHA-256 imprint, Ed25519 signer only, caller trust anchors, no network, separate unsupported status. | Resolved |
| AR-29 Segment summary defeats padding | A public aggregate *unpadded* byte count would reveal the value bucket padding was meant to coarsen. | SegmentHeader offset 40 is reserved zero; exact unpadded totals exist only in authenticated encrypted SegmentEnd. | Resolved |
| AR-30 Recipient wrap AD was not encoded | Independent implementations could authenticate different stanza/context bytes while claiming suite 1. | Flat 14-tag transcript with exact sources, widths, empty-value behavior, vectors, and a contiguous wrap/unwrap algorithm. | Resolved specification defect |
| AR-31 Private object dispatch was heuristic | Record, Chunk frame, and collection bytes could be confused or require parser probing. | Authenticated `EBPO` magic/version/kind/flags dispatches exactly one complete payload grammar. | Resolved specification defect |
| AR-32 Canonical private collection was undefined | Concatenation, ordering, duplicate, unknown-item, and truncation behavior could diverge. | `EBCS` grammar, nine semantic collection kinds, exact item records/order/cardinality, hard limits, and negative vectors. | Resolved specification defect |
| AR-33 Adjacent crypto wire names disagreed | Signature type and timestamp transcript names or directory/Index item bytes could diverge. | Corrected type 26 reference, exact type-26-through-tag-8 timestamp input, and canonical type-22/type-27 entry schemas. | Resolved specification defect |

No blocking adversarial finding remains in the specification. AR-25 is a
mandatory implementation/release gate: failure to satisfy it blocks production
crypto rather than authorizing a different combiner.

## Architecture consistency result

LAI, PCR, AUX, and PCI retain their established meanings:

- encryption, padding, recipient stanzas, segment salts, ciphertext, and
  signatures do not enter LAI or AUX;
- PCR continues to bind logical Chunk organization, not codec/encryption;
- AFK-derived chunk boundaries may change PCR when encrypting/rekeying, exactly
  as any chunker change does;
- PCI changes for every exact-byte encryption or envelope edit.

One genuine pre-existing architecture contradiction was found: recipient
addition was described both as an envelope-only operation and as full
re-encryption. The frozen resolution is AR-18 and the lifecycle rule in the
suite document. No other Entrybound architecture contradiction remains.
