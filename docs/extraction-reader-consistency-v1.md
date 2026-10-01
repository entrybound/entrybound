# Extraction reader consistency v1

This matrix records the safety contract shared by the current reader APIs. It
does not grant an archive permission to raise caller limits, change extraction
policy, or claim whole-archive verification from a partial read. The focused
regressions are in `crates/entrybound/tests/extraction_policy_consistency.rs`;
exact-head qualification evidence is required before calling any row PASS.

| Reader path | What it verifies before returning content or creating final objects | Caller limits and output boundary |
|---|---|---|
| Complete unencrypted INDEXED: `ecf::open_with_limits` then `archive::unpack` | Canonical preamble, section framing and digests, decoded chunks, EAM invariants, budget actuals, identity roots and footer binding | `ExtractionPolicy` resource and decode limits are checked before destination creation. `unpack` creates final objects only after full `open_with_limits` succeeds. |
| Complete unencrypted STREAM: `archive::unpack_stream` | One sequential pass checks framing, chunks, EAM invariants, identities, footer and exact EOF before materialization | Caller resource, decode, container, item and staging limits apply; content is held in bounded staging until full verification. Failure releases staging and creates no destination object. |
| Complete encrypted INDEXED: `crypto::open_encrypted` then `archive::unpack_opened` | Recipient unlock, authenticated envelope and segments, then the ordinary EAM and identity checks | Crypto, resource and decode limits apply at open. `unpack_opened` also honors its own extraction policy before final output, even if the earlier open used broader limits. `VerifiedArchive` exposes read-only inspection through `Deref`; consuming `into_inner` produces an ordinary model that needs fresh reader verification before extraction. Public report flags cannot authorize output, and replacing the verified model with one from another open is checked against the reader-owned authority before materialization. |
| Unencrypted INDEXED range: `ecf::open_indexed_random` and `read_entry` | Metadata and requested Chunk/dependency closure only, with source revision stability | Range, resource and decode limits apply. The API returns verified requested bytes; it does not unpack a tree. `whole_archive_verified` remains `false` without a full read. |
| Encrypted INDEXED range: `crypto::open_indexed_random_encrypted` and `read_entry` | Authenticated CONTROL metadata and requested encrypted payload/dependencies only | Crypto unlock, range, resource and decode limits apply. The private Descriptor v2 declaration must satisfy both `EncryptedOpenOptions` and `RandomAccessPolicy`; neither caller input raises the other. The API returns authenticated requested bytes and a partial-verification report; it does not unpack a tree or assert whole-archive verification. |

## Refusal and accepted cases

- A caller budget below the archive's declared entry count, or a decode limit
  below its declared requirement, must refuse before final output in every
  applicable full-reader path. Range readers refuse before returning bytes.
- A STREAM archive with `BudgetDeclared=false` has zero preamble budget values
  and is bounded incrementally by caller policy. The focused test covers both
  refusal and successful extraction for this supported form.
- Invalid magic, noncanonical structure, failed section or ciphertext
  authentication, failed chunk integrity, and truncated STREAM input must
  refuse before extraction creates final objects. A wrong crypto identity must
  not provide an opened archive to materialize.
- An archive that is verified but later encounters a destination collision may
  leave earlier newly created entries. Collision refusal preserves preexisting
  objects; extraction is not a filesystem transaction.
- Successful full INDEXED, STREAM and encrypted INDEXED extraction should
  produce the same logical tree for the same EAM under equivalent policy.
  Range reads deliberately report only the requested verified file and its
  dependency closure.

STREAM random access and encrypted STREAM are not implemented by these reader
APIs. A range read does not supply a whole-archive verification proof or an
extraction capability. Safe symlink metadata restoration without a held parent
handle is unavailable and must be recorded as an omission.

The current unencrypted INDEXED writer always declares a budget. The later
bootstrap note describes the shared preamble boolean but assigns no separate
unencrypted INDEXED `BudgetDeclared=false` producer/reader behavior; that
combination has no supported bootstrap path or positive vector. It must not be
silently inferred from STREAM's explicit undeclared-budget support. Encrypted
INDEXED uses a distinct public-false privacy sentinel under its crypto feature
and private Descriptor rules. Historical encrypted Descriptor v1 is accepted
under the explicit compatibility clause in `docs/crypto-wire-v1.md`; its
authenticated actuals and decoder needs remain subject to caller limits.
For that legacy form, encrypted range opening performs a full
verification using the existing unlocked keys before returning metadata or
content. It derives exact bounds with the common full reader, without another
recipient unlock or KDF. The source must fit the caller's individual range,
transfer and crypto memory limits; otherwise the fallback refuses before
output. Its report records whole-archive verification. Corrected Descriptor v2
range opening retains the partial-verification behavior described above.

The legacy range implementation currently screens physical source size at
64 times its length against the crypto working-memory limit. That screen is
not an aggregate allocation proof: retained controls, container capacity,
transcripts and temporary ordinary-container materialization also coexist.
Aggregate-memory qualification for this fallback remains open. A caller-owned
random source can also return a vector with excess capacity; exact returned
length alone does not bound that allocation. No hard working-memory guarantee
is claimed for the legacy fallback until those allocations are accounted for.

Corrected encrypted range reads retain authenticated Chunk locators instead
of decrypted Chunk frames between requests, including when rebuilding an
Index. Public protected-record headers are checked against caller ciphertext
limits before a whole segment is fetched. Envelope stanza count and encoded
stanza size are checked before stanza allocation/decoding, and a matching
identity-attempt limit is checked before password KDF or decapsulation work.

## Authority and compatibility

The original Product Architecture v1.2 §§11.5–11.7 establishes independent
caller bounds, full verification before normal materialization, descriptor
relative metadata writes, and policy-gated dangerous restoration. Its §11.7
table described absolute-target symlinks as written by default. The later,
accepted `docs/filesystem-fidelity-v1.md` extraction policy, introduced in
commit `136d01819b0f804c03d7f88a09e4a2e9f3c4bab8` on 2026-09-03,
explicitly makes `Safe` the default and requires `All` for absolute, rooted or
escaping targets. This is an extraction policy amendment: archived link target
bytes, wire format and identity participation are unchanged. The current
`ExtractionPolicy` API uses that later default; callers can opt into `All`.

This chronology records which public restoration policy applies. Later
semantic changes require a corresponding public authority amendment.
No row here extends the INDEXED/STREAM encoding or changes LAI, AUX, PCR or
PCI. Platform capability omissions remain explicit in extraction reports.
