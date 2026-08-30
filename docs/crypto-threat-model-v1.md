# Entrybound encrypted-archive threat model v1

Status: **frozen security architecture; not implemented**. This document is
normative for the first Entrybound cryptographic implementation. It is read
with [crypto-suite-v1.md](crypto-suite-v1.md),
[crypto-wire-v1.md](crypto-wire-v1.md), and
[crypto-review-v1.md](crypto-review-v1.md). Requirements expressed as MUST,
MUST NOT, REQUIRED, and SHALL are conformance requirements.

## Scope and protected assets

Crypto v1 protects a Complete, INDEXED Entrybound archive while it is at rest
or copied through an untrusted transport. The protected plaintext is the
archive file key, semantic manifest, names and directory structure, metadata,
content and Chunk identities, compression/transform plans, dictionaries,
reconstruction objects, FidelityReport, provenance, Index, identity roots,
embedded signatures, and decoded file bytes.

The design provides:

- confidentiality and authentication for every protected record;
- a separately verified commitment from the archive file key to the archive
  crypto context;
- authentication of the complete recipient set, including unknown stanzas;
- segment, record, and archive-final ordering and truncation protection;
- cross-archive splice resistance through a random archive identifier in every
  key and authentication domain;
- metadata and name confidentiality, subject to the leaks listed below;
- hybrid post-quantum recipient confidentiality for normally created
  recipient archives;
- bounded password and archive-controlled resource use under caller policy;
- self-contained offline opening with no key server or network dependency;
- signatures with independent content, physical, and addressing status.

Encrypted never means encryption without authentication. An implementation
MUST NOT expose plaintext from an AEAD operation, a reconstruction pipeline, or
a codec pipeline before authentication and the final plaintext identity check
succeed.

## Attacker classes

### Snapshot adversary

The attacker obtains one complete encrypted archive and may perform unlimited
offline analysis. It sees the public envelope, recipient method types, padded
record lengths, and other public framing. It may guess passwords, guessed
content, or recipient identities. With no recipient private key or password of
adequate entropy, it must not recover the file key, protected metadata, or
payload.

### Persistent storage adversary

The attacker can observe versions and can copy, replace, truncate, reorder,
replay, or splice bytes. It may recompute unkeyed section hashes and PCI. A
reader must reject altered records, reordered records or segments, missing
final markers, cross-archive ciphertext, and a recipient set that does not
authenticate under the recovered file key.

A self-contained file cannot distinguish a byte-for-byte replay of an older,
complete, valid archive from the same archive's current version. Rollback
detection requires caller-held state such as an expected PCI or addressing
signature, or a trusted timestamp/log outside the archive. Entrybound does not
claim freshness from archive bytes alone.

### Malicious archive producer

The producer controls every serialized field and can choose hostile KDF costs,
counts, lengths, nonces, salts, stanza types, and ciphertext. It may create
cryptographically valid but resource-hostile content. Readers therefore parse
canonical bounded framing, apply hard format limits, and apply caller policy
before password KDF work, large allocation, decompression, or reconstruction.
A valid authentication tag does not waive semantic, resource, or canonical
validation.

A producer that intentionally knows the plaintext and file key can disclose
them out of band. No archive format can prevent that. Signatures can attribute
the exact declared bindings; they do not make a malicious producer honest.

### Storage-provider adversary

The provider sees archive size, update cadence, public framing, recipient
stanza sizes and types, padded ciphertext-length patterns, and requests made to
its storage API. It may know a corpus of candidate plaintexts. Default padding
and secret-derived boundaries reduce some simple fingerprints; they do not
hide total size, record count, or access patterns. The stronger keyed-boundary
mode is provided for a threat model in which observed boundary patterns and
known/chosen plaintext matter.

### Known-plaintext adversary

The attacker knows or strongly guesses files, prefixes, metadata, or prior
archive contents. Such knowledge must not recover the file key, forge accepted
records, or enable cross-archive equality tests through convergent encryption.
The default boundary mode is expressly defense in depth, not a cryptographic
claim against boundary-key recovery. `--pad=none` accepts still greater length
leakage and is recorded in the authenticated archive.

### Recipient adversary

A legitimate recipient learns the archive file key and all protected content.
It can copy plaintext, forward the original archive, or make a new archive. It
can add a recipient and authenticate a new envelope because adding requires
file-key knowledge. Entrybound is not DRM and cannot prevent those acts.

An expected addressing signature detects recipient-set or commitment changes.
Removing a recipient rotates the file key and re-encrypts the archive, but it
cannot revoke plaintext, file keys, or old archive copies already obtained by
that recipient. A recipient cannot create a different interpretation that
verifies under an unchanged signed content/physical/addressing transcript
without forging the signature or breaking the underlying hashes/MACs.

## Required security properties

### Confidentiality

Every semantic and physical record not explicitly listed as public is inside
an AEAD-protected private record. Names, paths, entry counts, exact logical
lengths, EAM records, the Index, LAI/PCR/AUX, codecs, transforms, dictionaries,
reconstruction state, and embedded signatures are private. Recipient discovery
and minimal ciphertext framing remain public.

Hybrid recipient archives use X-Wing (ML-KEM-768 plus X25519). Entrybound does
not create a classical-only stanza in v1. A hybrid archive may contain only
hybrid-protection-class stanzas. Password mode is a separate archive protection
class and cannot be mixed with hybrid or future stronger recipients.

### Integrity and unique interpretation

Every ciphertext record uses AEAD. Public headers needed to locate or frame a
record are associated data. Exact semantic record kind, logical ordinals,
logical lengths, TransformPlan references, and identities live only in the
encrypted private record, avoiding competing authorities. Canonical ECF and
EAM validation still runs after authentication.

Segment keys, nonces, associated data, authenticated segment endings, and one
terminal archive-final record bind record order, segment order, totals,
identity roots, and the intended footer. A corrupt prerequisite Chunk,
Dictionary, reconstruction object, or region prevents every dependent result
from being reported verified.

### Key commitment and recipient-set integrity

AES-GCM-SIV is not assumed to be key committing. After a stanza authenticates
and produces a candidate archive file key, the reader verifies a 32-byte
HMAC-SHA-256 commitment before accepting that key. It then verifies the
envelope MAC over the full canonical recipient set and public crypto context.
Unknown stanza types may be skipped while looking for a matching identity, but
their exact bytes remain in both the envelope MAC and recipient-set digest.

### Nondeterminism

Production encryption uses a fresh random archive file key, archive identifier,
segment salts, stanza identifiers, wrap nonces, and padding bytes. A production
API MUST NOT offer a deterministic encryption mode. Fixed randomness is
permitted only behind test-only interfaces for conformance vectors.

### Resource safety

Format maxima are not allocation instructions. Caller policy is checked first.
Password parameters, stanza/envelope lengths, record sizes, segment counts,
logical output, decoder memory, reconstruction expansion, and dependency costs
remain bounded. Secret-dependent errors are collapsed at the user-facing
boundary where distinguishing them would create an avoidable oracle.

## Deliberately public information

Crypto v1 leaves the following observable:

- Entrybound magic, base format version, encrypted-archive feature bits,
  `INDEXED` layout, Complete role, crypto version, and PayloadSuite ID;
- one random 32-byte archive identifier;
- CryptoEnvelope location and length, recipient stanza count, stanza type and
  protection class, X-Wing encapsulations, password KDF salt/parameters,
  commitment, envelope MAC, and padding/boundary modes;
- recipient hints, which are fixed to zero in v1; no stable public-key
  fingerprint is serialized;
- public segment class, ordinal, random salt, ciphertext extent, record count,
  coarse record class, per-record ciphertext length, and necessary locators;
- fixed footer, exact total archive length, and unkeyed ciphertext/section
  digests;
- filesystem attributes, timestamps, update cadence, transport metadata, and
  remote access patterns exposed by systems outside `.eb`;
- record-count and bucket information from which approximate content size,
  Chunk distribution, or repeated access may sometimes be inferred;
- whether the creator selected default bucket padding, maximum padding, no
  padding, or the stronger keyed-boundary mode.

Default bucket padding **quantizes** record sizes. It does not fully hide Chunk
sizes. Maximum padding hides the exact size within a record class but still
leaks record count and total size. Secret chunk boundaries do not conceal the
last Chunk length or all equality/access information. No mode claims oblivious
storage.

## Non-goals

Crypto v1 does not provide:

- total archive-size hiding, constant-rate traffic, ORAM, PIR, or remote access
  pattern hiding;
- self-contained rollback/freshness detection for replay of a whole valid old
  archive;
- deniability, anonymity against endpoint observation, traffic-flow secrecy,
  or recipient-count/type secrecy;
- protection after a recipient private key, password, archive file key, or
  plaintext is compromised;
- revocation of copies already possessed by a removed recipient;
- prevention of recipient forwarding or re-archiving;
- cross-tenant or cross-archive convergent encryption/deduplication;
- protection from a compromised host while plaintext or keys are in use;
- encrypted STREAM archives in crypto v1;
- post-quantum signature security in v1.

## Trust and operational assumptions

- Randomness comes from the operating system CSPRNG and failures are fatal.
- Recipient public keys and signer public keys are authenticated by the caller
  or its trust policy; the archive is not a PKI.
- Password secrecy is limited by password entropy and the encoded Argon2id
  cost. The public commitment and authenticated wrapped key provide an offline
  password-verification oracle.
- Rust dependencies are pinned, standard vectors are mandatory, and X-Wing
  additionally has a production-release interoperability gate described in
  [crypto-review-v1.md](crypto-review-v1.md).
- The caller supplies resource and signature policy. Cryptographic validity is
  not permission to extract, overwrite, trust a signer, or accept stale
  bindings.

## INDEXED-only decision

Crypto v1 supports INDEXED archives only. The current STREAM order places
physical data before the manifest that names it; equivalent metadata privacy
would require buffering or a different authenticated sequential design. V1
does not weaken privacy or create a second semantic authority to claim STREAM
support. An encryption request combined with `--layout stream`, output `-`, or
another unseekable destination fails before output with
`EB_CRYPTO_LAYOUT_UNSUPPORTED`. It does not silently buffer, switch layout, or
emit a less-private archive.
