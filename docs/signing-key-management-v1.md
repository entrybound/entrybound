# Signing and key-management implementation v1

Status: experimental implementation note. The normative transcripts and wire
rules remain in `crypto-suite-v1.md` and `crypto-wire-v1.md`.

## Signature bindings and status

Entrybound uses pure Ed25519 (`algorithm_id = 1`). CONTENT is mandatory and
binds LAI, AUX, identity profile, and ECF namespace/version. PHYSICAL optionally
binds PCR. ADDRESSING optionally binds PayloadSuite, canonical recipient-set
digest, commitment, and archive ID, and is valid only for an authenticated
encrypted archive. PCI is intentionally not a signature binding.

Cryptographic verification and current-archive comparison are independent:

CLI creation follows the frozen default: unencrypted archives bind CONTENT and
PHYSICAL, while authenticated encrypted archives additionally bind ADDRESSING.
The explicit `--bind-physical` and `--bind-addressing` spellings are accepted as
confirmations of those defaults.

- `cryptographic = VALID` means Ed25519 verified the transcript stored in the
  record;
- each requested binding is separately `VALID` or `STALE` against the current
  verified archive;
- an omitted binding is `NOT_BOUND`;
- caller policy turns absence, invalidity, or a required stale binding into its
  stable `EB_SIGNATURE_*` diagnostic. Without such a policy, status remains
  inspectable rather than changing archive interpretation.

A detached `.ebsig` is exactly one canonical type-26 record, with no wrapper or
trailing bytes. Encrypted embedded signatures are sign-then-encrypt objects in
EBCS kind 7, lexically ordered by complete canonical record bytes. There is no
invented embedded placement for unencrypted ECF.

## Timestamps

An externally supplied RFC 3161 token is attached as type-26 tag 9. The SHA-256
message imprint covers the exact canonical record through tag 8. Verification
is offline and accepts only the frozen v1 profile: DER up to 64 KiB, SHA-256
imprint/digest, Ed25519 CMS signer/certificate signatures, embedded certificate
chain, and a signer certificate whose critical ExtendedKeyUsage contains only
`id-kp-timeStamping`.

Trust anchors and current-time policy are caller inputs. Entrybound performs no
TSA request, certificate download, OCSP, or CRL retrieval. An unsupported CMS
algorithm reports `EB_SIGNATURE_TIMESTAMP_UNSUPPORTED`; malformed DER, wrong
imprint/signature, untrusted chain, or invalid time reports
`EB_SIGNATURE_TIMESTAMP_INVALID`.

## Recipient transactions

`key list` authenticates the archive before reading RecipientDirectoryEntryV1.
Hybrid entries expose only authenticated stanza ID, public-key fingerprint,
optional private label, stanza type, and protection class. Password archives
report PASSWORD_ONLY and do not invent a public-key directory.

Adding a hybrid recipient retains AFK and archive ID. It creates a fresh X-Wing
encapsulation/stanza, canonicalizes and authenticates the stanza sequence,
regenerates recipient directory and ArchiveFinal, and rewrites affected CONTROL
segments/footer. PAYLOAD segments are reused exactly when their class/ordinal is
unchanged. Consequently LAI/AUX/PCR and CONTENT/PHYSICAL signatures remain
current, PCI changes, and ADDRESSING signatures become stale.

Removing a stanza without rotating AFK cannot revoke a recipient that already
learned AFK. `key remove` therefore requires the complete public keys of every
recipient to retain, generates fresh AFK/archive ID, re-runs encrypted-keyed CDC
and planning, rewraps only for retained recipients, and fully encrypts and
self-verifies a replacement. A missing retained public key is refusal, never a
fingerprint-based guess.

Password replacement follows the same fresh-key rule and also generates a new
Argon2 salt. The old password/removed identity cannot unlock the replacement.
For both operations LAI/AUX remain stable when semantics are unchanged, PCR may
change because AFK-derived chunking changes, and PCI changes. Existing signature
records remain as historical assertions: CONTENT stays current, PHYSICAL is
compared to the new PCR, and ADDRESSING is stale. Entrybound never resigns
automatically.

## Replacement safety and limitations

Mutation output is fully authenticated and EAM/identity-verified with the new or
retained AFK before being returned. Same-path CLI mutation writes and syncs a
sibling temporary file, preserves the original under a sibling backup, installs
the replacement, then removes the backup. Construction/authentication failure
does not touch the original. No online TSA, general PKI, keychain, HSM, classical-
only recipient, protection-mode conversion, encrypted STREAM, or remote key
service is implemented.
