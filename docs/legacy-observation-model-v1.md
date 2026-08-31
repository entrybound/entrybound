# Legacy Observation Model v1

Status: implemented experimental adapter contract. This document freezes the
boundary used by `zip-strict/v1`; it does not add foreign semantics to EAM.

## Pipeline and authority

```text
foreign bytes
  -> adapter observations and byte evidence (LOM)
  -> explicit reconciliation policy
  -> valid Entrybound Archive Model projection + conversion provenance
  -> ordinary native planner and ECF writer
```

An adapter never constructs special-case EAM entries while parsing. It emits a
`LegacyArchiveObservation`, archive-level fields, and ordered
`LegacyEntryObservation` values. Each `LegacyFieldObservation<T>` names a
prospective semantic field and preserves:

- a format/structure/instance `LegacyAuthority`;
- the exact raw foreign value;
- an interpreted value when the adapter can establish one;
- an absolute `LegacyEvidenceLocation { offset, length }`;
- `Valid`, `Invalid`, or `Uninterpreted` parser state.

The generic report uses `LegacyObservedValue` for bytes, unsigned/signed
integers, text, and booleans. An adapter may retain richer private parsed types;
the reconciler consumes those parsed observations without reparsing the source.
This model contains no ZIP-specific authority names and is intended for future
tar and 7z adapters.

## Conflict classes

Every conflict identifies the semantic field, all authorities and values,
evidence locations, class, and any subsequent resolution.

- **Omission**: one structurally valid authority declares a fact and another
  omits it. A policy may accept the available claim.
- **Refinement**: both observations are compatible and one is more precise or
  structurally bound to the other. The binding must be validated before the
  refined value is selected.
- **Divergence**: authorities give different plausible semantic values.
  `strict` never chooses one.
- **Irreconcilable**: the evidence cannot describe one safe coherent object
  without invented semantics.

Classification and `LegacyResolution` belong to adapter/reconciliation code.
EAM receives only the resolved projection and continues enforcing its ordinary
path, ancestor, kind, content, and metadata invariants.

## Provenance and identity

Successful conversion stores one auxiliary `ConversionProvenance` object. It
records the source format, versioned adapter, SHA-256 of every exact source
byte, mode, source entry/observation counts, conflict counts, all automatic
Omission/Refinement decisions, synthesized ancestors, unsupported metadata,
and final outcome.

Provenance describes how EAM was obtained; it is not another authority for an
Entry, ContentObject, path, or digest. Its canonical digest contributes to AUX.
It never enters LAI or PCR. Two sources that resolve to identical semantic
content may therefore have equal LAI/PCR and different AUX. Index data remains
non-authoritative.

The native wire assignment is incompatibility feature `0x2000`
(`conversion-provenance-v1`), canonical record type 28. Type 29 is used only as
a nested canonical resolution record. In INDEXED ECF, type 28 follows the sole
type-5 FidelityReport record in the FIDELITY section. In STREAM, both records
are carried in the one FIDELITY manifest item. Absence of the feature preserves
the historical single-record payload exactly.

## Extension rules

Future compatibility profiles may resolve Divergence differently, and a
future preservation mode may retain more foreign evidence. They must consume
the same observations, name the policy/version in provenance, and must not
weaken EAM or silently redefine `strict`. A new adapter/parser behavior requires
a new adapter identifier.
