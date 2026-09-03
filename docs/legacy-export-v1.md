# Legacy export v1

Status: implemented and frozen as `entrybound/legacy-export-v1`.

## Boundary

Export consumes a fully verified Entrybound Archive Model. It does not inspect
ECF section ordering, source Chunk physical order, source codecs, or a temporary
filesystem extraction. The flow is:

```text
verified EAM
→ versioned target analysis
→ LOSSLESS | LOSSY | REFUSED
→ explicit caller policy
→ deterministic target bytes
→ ExportReceipt v1 or v2
```

The complete analysis, target compression decisions, framing, target length,
and target SHA-256 are computed before a destination is created.

## Outcomes

- `LOSSLESS`: every target-relevant Entry path, kind, regular-file byte,
  explicit directory, `core.executable`, and `core.mtime` claim is represented
  exactly.
- `LOSSY`: content remains safe and exact, but target-relevant metadata is
  degraded, omitted, or unavoidably introduced. Bytes require the explicit
  `--allow-lossy` policy.
- `REFUSED`: a safe coherent target is impossible under the profile. No policy
  can accept it and no partial target is created.

Stable issue classes are `PATH_UNREPRESENTABLE`, `PATH_COLLISION`,
`ENTRY_KIND_UNSUPPORTED`, `TIMESTAMP_RANGE_LOSS`,
`TIMESTAMP_PRECISION_LOSS`, `METADATA_UNSUPPORTED`, and
`TARGET_LIMIT_EXCEEDED`. Each issue includes scope, semantic field, source
value, target capability, disposition, and reason.

## Auxiliary evidence boundary

FidelityReport limitations, conversion provenance, preserved legacy source
bytes/observations, reconstruction planning audits, signatures, encryption,
recipients, and planner state are Entrybound governance or physical evidence.
They are reported by the receipt but are not target-relevant legacy semantics,
so their absence from ZIP/tar does not by itself turn a `LOSSLESS` export into
`LOSSY`.

Encryption is a security-state transition rather than metadata loss. An
encrypted source must be authenticated before export; the receipt records
`source encrypted = true` and `target encrypted = false`. Embedded signatures
are evaluated and summarized but are not embedded or re-signed in the target.

## Preflight and output safety

Preflight validates the EAM, reconstructs logical ContentObjects directly from
their authoritative Chunk-reference order, verifies each ContentObject digest,
checks paths/metadata/limits, and fully prepares target bytes. `--dry-run`
performs the same work but writes neither target nor receipt.

File destinations use exclusive creation. Encoding or receipt failure removes
new incomplete output; an existing target is never overwritten. Binary stdout
is prepared completely before the first byte is emitted and all status goes to
stderr.

## ExportReceipt v1

The optional receipt is canonical UTF-8 JSON with a fixed field order and final
newline. It records:

- format `entrybound/export-receipt-v1`, version 1, and exporter ID;
- source LAI, AUX, and PCR;
- source encryption and evaluated signature counts;
- Entrybound-only evidence summary;
- exact target format/profile and outcome;
- ordered typed issues;
- entry/logical-byte counts;
- target byte length and SHA-256;
- `deterministic = true`.

The receipt is external evidence. It is not embedded in the target and does not
participate in source EAM identity.

Wrapped tar profiles use ExportReceipt v2 rather than extending the frozen v1
JSON schema in place. V2 records the semantic tar/pax-v1 target, transport
profile, inner tar length/digest, and strict re-import result. See
[compressed tar export v1](compressed-tar-export-v1.md).

Multi-target operations use the external canonical
entrybound/migration-report-v1; sidecar and transaction behavior is frozen in
[migration workflows v1](migration-workflows-v1.md).

## Identity and determinism

Export is a function of verified EAM plus the target profile. INDEXED versus
STREAM, native compression profile, ciphertext, Chunk physical order, and PCI
do not affect target bytes. A lossless target strict-reimports with identical
LAI and exact ContentObject bytes. AUX normally changes because re-import adds
conversion provenance; PCR may change under new native planning.
