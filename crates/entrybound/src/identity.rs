//! SHA-256 identity and integrity construction for `ecf/bootstrap-v1`.
//!
//! Plaintext Chunk and ContentObject logical digests are SHA-256 over the exact
//! plaintext bytes. Structured digests use `Entrybound hash v1`, a domain
//! string, a field count, and a sequence of `u64 length || field bytes` values.
//! Merkle leaves, interior nodes, and empty roots have separate domains and are
//! never padded.

use std::collections::BTreeMap;
use std::ops::Range;

use sha2::{Digest as ShaDigest, Sha256};

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, Chunk, ChunkRef, ContentObject, ContentRef, ConversionProvenance,
    ConversionResolution, Digest, Entry, EntryData, EntryIdentity, EntrySet, FidelityIssue,
    FidelityReport, LegacyPreservation, LinkTargetEncoding, LogicalPath, MetadataItem, MetadataSet,
    MetadataValue, PreservedLegacyValue, TimestampPrecision,
};

/// Fixed bootstrap chunk size used only by direct in-memory construction.
pub const BOOTSTRAP_CHUNK_SIZE: usize = 1024 * 1024;
/// Numeric plan reference for `bootstrap-store-v1`.
pub const STORE_PLAN_ID: u64 = 1;
pub const STORE_PLAN_IDENTIFIER: &str = "bootstrap-store-v1";
pub const STORE_CODEC_IDENTIFIER: &str = "store/v1";

const HASH_PREFIX: &[u8] = b"Entrybound hash v1\0";

/// The logically stable identity descriptor result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogicalArchiveIdentity(pub Digest);

/// The chunking-dependent physical content root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalContentRoot(pub Digest);

/// The non-identity metadata, fidelity, and provenance root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuxiliaryRoot(pub Digest);

/// The digest of every exact serialized container byte.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalContainerIdentity(pub Digest);

/// The four identities exposed by an opened archive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentitySet {
    pub lai: LogicalArchiveIdentity,
    pub pcr: PhysicalContentRoot,
    pub aux: AuxiliaryRoot,
    pub pci: PhysicalContainerIdentity,
}

/// Roots computable before final container bytes exist.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeRoots {
    pub lai: LogicalArchiveIdentity,
    pub pcr: PhysicalContentRoot,
    pub aux: AuxiliaryRoot,
}

impl NativeRoots {
    /// Combines native roots with the exact-byte container digest.
    #[must_use]
    pub const fn with_pci(self, pci: PhysicalContainerIdentity) -> IdentitySet {
        IdentitySet {
            lai: self.lai,
            pcr: self.pcr,
            aux: self.aux,
            pci,
        }
    }
}

/// A reader must carry verification state rather than invite inference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationState {
    ChunkVerified,
    PhysicalRootVerified,
    Unverified,
}

/// SHA-256 over exact bytes, used for plaintext content and section integrity.
#[must_use]
pub fn sha256_exact(bytes: &[u8]) -> Digest {
    let output = Sha256::digest(bytes);
    Digest::from_bytes(output.into())
}

/// PCI over every exact serialized container byte.
#[must_use]
pub fn physical_container_identity(bytes: &[u8]) -> PhysicalContainerIdentity {
    PhysicalContainerIdentity(sha256_exact(bytes))
}

/// Constructs a ContentObject and its naturally deduplicated plaintext Chunks.
pub fn build_content(
    plaintext: &[u8],
    chunk_size: usize,
    plan_ref: u64,
) -> Result<(ContentObject, BTreeMap<Digest, Chunk>)> {
    if chunk_size == 0 {
        return Err(resource("chunk size must be non-zero"));
    }
    let ranges = (0..plaintext.len())
        .step_by(chunk_size)
        .map(|start| start..start.saturating_add(chunk_size).min(plaintext.len()))
        .collect::<Vec<_>>();
    build_content_from_ranges(plaintext, &ranges, plan_ref)
}

/// Constructs content from validated creation-time chunk ranges.
pub fn build_content_from_ranges(
    plaintext: &[u8],
    ranges: &[Range<usize>],
    plan_ref: u64,
) -> Result<(ContentObject, BTreeMap<Digest, Chunk>)> {
    validate_complete_ranges(plaintext.len(), ranges)?;
    let logical_digest = sha256_exact(plaintext);
    let mut chunks = BTreeMap::new();
    let mut refs = Vec::new();
    for range in ranges {
        let bytes = &plaintext[range.clone()];
        let chunk_id = sha256_exact(bytes);
        let logical_len =
            u64::try_from(bytes.len()).map_err(|_| resource("chunk length exceeds u64"))?;
        refs.push(ChunkRef { chunk_id });
        match chunks.entry(chunk_id) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(Chunk {
                    chunk_id,
                    logical_len,
                    plan_ref,
                    group_ref: None,
                    plaintext: bytes.into(),
                });
            }
            std::collections::btree_map::Entry::Occupied(entry)
                if entry.get().plaintext.as_ref() != bytes =>
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::ChunkIdentityCollision,
                    format!("distinct plaintext regions produced Chunk ID {chunk_id}"),
                ));
            }
            std::collections::btree_map::Entry::Occupied(_) => {}
        }
    }
    let chunk_root = chunk_root(&refs, &chunks)?;
    Ok((
        ContentObject {
            logical_digest,
            chunk_root,
            chunks: refs.into_boxed_slice(),
        },
        chunks,
    ))
}

fn validate_complete_ranges(plaintext_len: usize, ranges: &[Range<usize>]) -> Result<()> {
    let mut expected = 0;
    for range in ranges {
        if range.start != expected || range.start >= range.end || range.end > plaintext_len {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::SectionStructure,
                "Chunk ranges overlap, contain a gap, are empty, or exceed plaintext",
            ));
        }
        expected = range.end;
    }
    if expected != plaintext_len || (plaintext_len != 0 && ranges.is_empty()) {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::SectionStructure,
            "Chunk ranges do not cover the complete plaintext",
        ));
    }
    Ok(())
}

/// Recomputes all authoritative native identities and returns a canonical copy.
/// Caller-provided Entry and descriptor digest placeholders are ignored.
pub fn apply_native_identities(archive: &Archive) -> Result<(Archive, NativeRoots)> {
    archive.validate()?;
    verify_chunks_and_content(archive)?;
    compute_native_identities(archive)
}

/// Recomputes native identities for a model whose Chunk plaintext digests and
/// ContentObject logical digests were already verified incrementally.
///
/// The sequential STREAM reader verifies every Chunk as its bytes arrive and
/// then releases the plaintext, so it cannot re-verify from a retained model.
/// The identity construction itself is byte-identical: LAI, PCR, and AUX are
/// derived from Entry fields, ContentObject `(logical_digest, chunk_root)`
/// pairs, Chunk `(chunk_id, logical_len)` leaves, and the FidelityReport, none
/// of which read plaintext.
pub(crate) fn native_identities_from_verified(archive: &Archive) -> Result<(Archive, NativeRoots)> {
    archive.validate_without_retained_plaintext()?;
    compute_native_identities(archive)
}

fn compute_native_identities(archive: &Archive) -> Result<(Archive, NativeRoots)> {
    let entries = archive
        .entry_set
        .entries()
        .iter()
        .map(|entry| {
            Entry::new(
                entry.path().clone(),
                entry.data().clone(),
                entry.metadata().clone(),
                EntryIdentity {
                    identity_digest: entry_identity_digest(entry),
                    aux_digest: entry_aux_digest(entry),
                },
            )
        })
        .collect::<Vec<_>>();
    let entry_set = EntrySet::from_canonical(entries)?;

    let mut canonical = archive.clone();
    canonical.entry_set = entry_set;
    canonical.fidelity = canonical_fidelity(&archive.fidelity);
    canonical.conversion = archive.conversion.as_ref().map(canonical_conversion);
    canonical.descriptor.pci = None;

    let manifest = manifest_root(&canonical);
    let total_logical = canonical.total_logical_size()?;
    let lai = lai(&canonical, manifest, total_logical)?;
    let pcr = pcr(&canonical)?;
    let aux = aux(&canonical);
    canonical.descriptor.lai = lai.0;
    canonical.descriptor.pcr = pcr.0;
    canonical.descriptor.aux = aux.0;

    Ok((canonical, NativeRoots { lai, pcr, aux }))
}

fn verify_chunks_and_content(archive: &Archive) -> Result<()> {
    for chunk in archive.content_store.chunks.values() {
        if chunk.logical_len != u64::try_from(chunk.plaintext.len()).unwrap_or(u64::MAX)
            || sha256_exact(&chunk.plaintext) != chunk.chunk_id
        {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ChunkDigestMismatch,
                chunk.chunk_id.to_string(),
            ));
        }
    }
    for object in archive.content_store.objects.values() {
        let mut hasher = Sha256::new();
        for chunk_ref in &object.chunks {
            let chunk = archive
                .content_store
                .chunks
                .get(&chunk_ref.chunk_id)
                .ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::UnknownChunk,
                        chunk_ref.chunk_id.to_string(),
                    )
                })?;
            hasher.update(&chunk.plaintext);
        }
        let logical = Digest::from_bytes(hasher.finalize().into());
        if logical != object.logical_digest {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ContentDigestMismatch,
                object.logical_digest.to_string(),
            ));
        }
        if chunk_root(&object.chunks, &archive.content_store.chunks)? != object.chunk_root {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ChunkRootMismatch,
                object.logical_digest.to_string(),
            ));
        }
    }
    Ok(())
}

fn chunk_root(refs: &[ChunkRef], chunks: &BTreeMap<Digest, Chunk>) -> Result<Digest> {
    let leaves = refs
        .iter()
        .map(|chunk_ref| {
            let chunk = chunks.get(&chunk_ref.chunk_id).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownChunk,
                    chunk_ref.chunk_id.to_string(),
                )
            })?;
            Ok((chunk.chunk_id, chunk.logical_len))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(chunk_root_from_leaves(&leaves))
}

/// Computes a ContentObject's chunk root from its ordered `(chunk_id,
/// logical_len)` leaves.
///
/// The sequential reader knows both values from the Chunk frame headers and
/// never needs retained plaintext to check a chunk root.
pub(crate) fn chunk_root_from_leaves(leaves: &[(Digest, u64)]) -> Digest {
    let leaves = leaves
        .iter()
        .map(|(chunk_id, logical_len)| {
            structured_hash(
                "chunk-tree/leaf",
                &[chunk_id.as_bytes(), &logical_len.to_be_bytes()],
            )
        })
        .collect::<Vec<_>>();
    merkle_root(&leaves, "chunk-tree/empty", "chunk-tree/node")
}

fn entry_identity_digest(entry: &Entry) -> Digest {
    let path = encode_path(entry.path());
    let identity_metadata = encode_metadata(entry.metadata(), true);
    let (kind, content_kind, content_digest) = match entry.data() {
        EntryData::Directory => (1_u8, 0_u8, None),
        EntryData::File {
            content: ContentRef::Internal(digest),
        } => (2, 1, Some(*digest)),
        EntryData::Symlink { target } => {
            let encoding = [match target.encoding() {
                LinkTargetEncoding::Utf8 => 1,
                LinkTargetEncoding::PosixBytes => 2,
            }];
            return structured_hash(
                "entry/identity/v2",
                &[
                    b"identity/v1",
                    &path,
                    &[3],
                    &encoding,
                    target.bytes(),
                    &identity_metadata,
                ],
            );
        }
        EntryData::ReparsePoint { value } => {
            return structured_hash(
                "entry/identity/v3",
                &[
                    b"identity/v1",
                    &path,
                    &[4],
                    &value.tag().to_be_bytes(),
                    value.data(),
                    &identity_metadata,
                ],
            );
        }
    };
    let content = content_digest
        .as_ref()
        .map_or(&[][..], |digest| digest.as_bytes().as_slice());
    structured_hash(
        "entry/identity/v1",
        &[
            b"identity/v1",
            &path,
            &[kind],
            &[content_kind],
            content,
            &identity_metadata,
        ],
    )
}

fn entry_aux_digest(entry: &Entry) -> Digest {
    structured_hash("entry/aux/v1", &[&encode_metadata(entry.metadata(), false)])
}

/// Computes the inode-independent canonical hardlink group identifier.
pub fn hardlink_group_id(content: Digest, paths: &[LogicalPath]) -> Result<Digest> {
    if paths.len() < 2 {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::InvalidHardlinkGroup,
            "a hardlink group requires at least two members",
        ));
    }
    let mut members = paths.to_vec();
    members.sort();
    if members.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::InvalidHardlinkGroup,
            "a hardlink group cannot contain duplicate paths",
        ));
    }
    let mut encoded = Vec::new();
    append_len(&mut encoded, members.len());
    for path in &members {
        append_bytes(&mut encoded, &encode_path(path));
    }
    Ok(structured_hash(
        "entrybound/hardlink-group/v1",
        &[content.as_bytes(), &encoded],
    ))
}

/// Verifies all serialized Entry identity digests and the Descriptor LAI using
/// only authenticated/fetched semantic metadata. Chunk plaintext and physical
/// organization are deliberately outside this check.
pub(crate) fn verify_metadata_lai(
    entries: &crate::eam::EntrySet,
    total_logical: u64,
    expected_lai: Digest,
) -> Result<()> {
    let mut leaves = Vec::with_capacity(entries.len());
    for entry in entries.entries() {
        let recomputed = entry_identity_digest(entry);
        if recomputed != entry.identity().identity_digest {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::EntryIdentityMismatch,
                entry.path().to_string(),
            ));
        }
        leaves.push(structured_hash("manifest/leaf", &[recomputed.as_bytes()]));
    }
    let manifest = merkle_root(&leaves, "manifest/empty", "manifest/node");
    let entry_count =
        u64::try_from(entries.len()).map_err(|_| resource("entry count exceeds u64"))?;
    let actual = structured_hash(
        "lai/v1",
        &[
            b"sha256",
            manifest.as_bytes(),
            b"identity/v1",
            &[1],
            &entry_count.to_be_bytes(),
            &total_logical.to_be_bytes(),
        ],
    );
    if actual != expected_lai {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::LaiMismatch,
            "fetched semantic metadata does not match the declared LAI",
        ));
    }
    Ok(())
}

/// Verifies Entry auxiliary digests and AUX when every AUX-bearing semantic
/// record was fetched. Random access uses this only for authenticated private
/// metadata collections that are complete.
pub(crate) fn verify_metadata_aux(
    entries: &crate::eam::EntrySet,
    fidelity: &crate::eam::FidelityReport,
    expected_aux: Digest,
) -> Result<()> {
    let mut leaves = Vec::with_capacity(entries.len());
    for entry in entries.entries() {
        let recomputed = entry_aux_digest(entry);
        if recomputed != entry.identity().aux_digest {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::EntryAuxMismatch,
                entry.path().to_string(),
            ));
        }
        leaves.push(structured_hash(
            "aux-manifest/leaf",
            &[recomputed.as_bytes()],
        ));
    }
    let entry_aux_root = merkle_root(&leaves, "aux-manifest/empty", "aux-manifest/node");
    let fidelity = structured_hash("fidelity/v1", &[&encode_fidelity(fidelity)]);
    let conversion = structured_hash("conversion/absent/v1", &[]);
    let actual = structured_hash(
        "aux/v1",
        &[
            b"sha256",
            entry_aux_root.as_bytes(),
            fidelity.as_bytes(),
            conversion.as_bytes(),
        ],
    );
    if actual != expected_aux {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::AuxMismatch,
            "fetched auxiliary metadata does not match the declared AUX",
        ));
    }
    Ok(())
}

fn manifest_root(archive: &Archive) -> Digest {
    let leaves = archive
        .entry_set
        .entries()
        .iter()
        .map(|entry| {
            structured_hash(
                "manifest/leaf",
                &[entry.identity().identity_digest.as_bytes()],
            )
        })
        .collect::<Vec<_>>();
    merkle_root(&leaves, "manifest/empty", "manifest/node")
}

fn lai(
    archive: &Archive,
    manifest_root: Digest,
    total_logical: u64,
) -> Result<LogicalArchiveIdentity> {
    let entry_count =
        u64::try_from(archive.entry_set.len()).map_err(|_| resource("entry count exceeds u64"))?;
    Ok(LogicalArchiveIdentity(structured_hash(
        "lai/v1",
        &[
            b"sha256",
            manifest_root.as_bytes(),
            b"identity/v1",
            &[1],
            &entry_count.to_be_bytes(),
            &total_logical.to_be_bytes(),
        ],
    )))
}

fn pcr(archive: &Archive) -> Result<PhysicalContentRoot> {
    let mut objects = Vec::new();
    objects.extend_from_slice(
        &u64::try_from(archive.content_store.objects.len())
            .map_err(|_| resource("ContentObject count exceeds u64"))?
            .to_be_bytes(),
    );
    for object in archive.content_store.objects.values() {
        objects.extend_from_slice(object.logical_digest.as_bytes());
        objects.extend_from_slice(object.chunk_root.as_bytes());
    }
    let chunk_count = u64::try_from(archive.content_store.chunks.len())
        .map_err(|_| resource("Chunk count exceeds u64"))?;
    Ok(PhysicalContentRoot(structured_hash(
        "pcr/v1",
        &[
            b"sha256",
            &objects,
            &chunk_count.to_be_bytes(),
            archive.descriptor.chunker_id.as_bytes(),
        ],
    )))
}

fn aux(archive: &Archive) -> AuxiliaryRoot {
    let leaves = archive
        .entry_set
        .entries()
        .iter()
        .map(|entry| {
            structured_hash(
                "aux-manifest/leaf",
                &[entry.identity().aux_digest.as_bytes()],
            )
        })
        .collect::<Vec<_>>();
    let entry_aux_root = merkle_root(&leaves, "aux-manifest/empty", "aux-manifest/node");
    let fidelity = structured_hash("fidelity/v1", &[&encode_fidelity(&archive.fidelity)]);
    let conversion = archive.conversion.as_ref().map_or_else(
        || structured_hash("conversion/absent/v1", &[]),
        |value| structured_hash("conversion/provenance/v1", &[&encode_conversion(value)]),
    );
    let base = structured_hash(
        "aux/v1",
        &[
            b"sha256",
            entry_aux_root.as_bytes(),
            fidelity.as_bytes(),
            conversion.as_bytes(),
        ],
    );
    archive
        .preservation
        .as_ref()
        .map_or(AuxiliaryRoot(base), |preservation| {
            let evidence = structured_hash(
                "legacy-preservation/v1",
                &[&encode_preservation(preservation)],
            );
            AuxiliaryRoot(structured_hash(
                "aux-preservation/v1",
                &[base.as_bytes(), evidence.as_bytes()],
            ))
        })
}

fn encode_preservation(value: &LegacyPreservation) -> Vec<u8> {
    let mut encoded = Vec::new();
    append_bytes(&mut encoded, value.preservation_format.as_bytes());
    append_bytes(&mut encoded, value.source_format.as_bytes());
    encoded.extend_from_slice(value.source_digest.as_bytes());
    append_bytes(&mut encoded, &value.source_bytes);
    append_len(&mut encoded, value.observations.len());
    for item in &value.observations {
        encoded.push(item.scope);
        encoded.extend_from_slice(&item.subject_ordinal.to_be_bytes());
        encoded.extend_from_slice(&item.observation_ordinal.to_be_bytes());
        append_bytes(&mut encoded, item.semantic_field.as_bytes());
        append_bytes(&mut encoded, item.authority.format.as_bytes());
        append_bytes(&mut encoded, item.authority.structure.as_bytes());
        encoded.extend_from_slice(&item.authority.instance.to_be_bytes());
        append_bytes(&mut encoded, &item.raw_value);
        encode_preserved_value(&mut encoded, item.interpreted_value.as_ref());
        encoded.extend_from_slice(&item.evidence.offset.to_be_bytes());
        encoded.extend_from_slice(&item.evidence.length.to_be_bytes());
        encoded.push(match item.validity {
            crate::eam::PreservedLegacyValidity::Valid => 1,
            crate::eam::PreservedLegacyValidity::Invalid => 2,
            crate::eam::PreservedLegacyValidity::Uninterpreted => 3,
        });
    }
    append_len(&mut encoded, value.conflicts.len());
    for conflict in &value.conflicts {
        encoded.extend_from_slice(&conflict.ordinal.to_be_bytes());
        append_bytes(&mut encoded, conflict.semantic_field.as_bytes());
        append_len(&mut encoded, conflict.authorities.len());
        for authority in &conflict.authorities {
            append_bytes(&mut encoded, authority.format.as_bytes());
            append_bytes(&mut encoded, authority.structure.as_bytes());
            encoded.extend_from_slice(&authority.instance.to_be_bytes());
        }
        append_len(&mut encoded, conflict.observed_values.len());
        for observed in &conflict.observed_values {
            encode_preserved_value(&mut encoded, Some(observed));
        }
        append_len(&mut encoded, conflict.evidence.len());
        for location in &conflict.evidence {
            encoded.extend_from_slice(&location.offset.to_be_bytes());
            encoded.extend_from_slice(&location.length.to_be_bytes());
        }
        append_bytes(&mut encoded, conflict.classification.as_bytes());
        if let Some(resolution) = &conflict.resolution {
            encoded.push(1);
            append_bytes(&mut encoded, resolution.action.as_bytes());
            if let Some(authority) = &resolution.selected_authority {
                encoded.push(1);
                append_bytes(&mut encoded, authority.format.as_bytes());
                append_bytes(&mut encoded, authority.structure.as_bytes());
                encoded.extend_from_slice(&authority.instance.to_be_bytes());
            } else {
                encoded.push(0);
            }
        } else {
            encoded.push(0);
        }
    }
    append_len(&mut encoded, value.selected_resolutions.len());
    for resolution in &value.selected_resolutions {
        append_bytes(
            &mut encoded,
            &encode_conversion_resolution_identity(resolution),
        );
    }
    encoded
}

fn encode_conversion_resolution_identity(value: &ConversionResolution) -> Vec<u8> {
    let mut encoded = Vec::new();
    append_bytes(&mut encoded, value.conflict_class.as_bytes());
    append_bytes(&mut encoded, value.semantic_field.as_bytes());
    encode_string_list(&mut encoded, &value.authorities);
    encode_string_list(&mut encoded, &value.observed_values);
    append_bytes(&mut encoded, value.action.as_bytes());
    encoded
}

fn encode_preserved_value(encoded: &mut Vec<u8>, value: Option<&PreservedLegacyValue>) {
    match value {
        None => encoded.push(0),
        Some(PreservedLegacyValue::Bytes(value)) => {
            encoded.push(1);
            append_bytes(encoded, value);
        }
        Some(PreservedLegacyValue::Unsigned(value)) => {
            encoded.push(2);
            encoded.extend_from_slice(&value.to_be_bytes());
        }
        Some(PreservedLegacyValue::Signed(value)) => {
            encoded.push(3);
            encoded.extend_from_slice(&value.to_be_bytes());
        }
        Some(PreservedLegacyValue::Text(value)) => {
            encoded.push(4);
            append_bytes(encoded, value.as_bytes());
        }
        Some(PreservedLegacyValue::Boolean(value)) => {
            encoded.push(5);
            encoded.push(u8::from(*value));
        }
    }
}

fn encode_conversion(value: &ConversionProvenance) -> Vec<u8> {
    let mut encoded = Vec::new();
    append_bytes(&mut encoded, value.source_format.as_bytes());
    append_bytes(&mut encoded, value.adapter_id.as_bytes());
    encoded.extend_from_slice(value.source_digest.as_bytes());
    append_bytes(&mut encoded, value.import_mode.as_bytes());
    for count in [
        value.source_entry_count,
        value.observation_count,
        value.omission_count,
        value.refinement_count,
        value.divergence_count,
        value.irreconcilable_count,
    ] {
        encoded.extend_from_slice(&count.to_be_bytes());
    }
    append_len(&mut encoded, value.resolutions.len());
    for resolution in &value.resolutions {
        append_bytes(&mut encoded, resolution.conflict_class.as_bytes());
        append_bytes(&mut encoded, resolution.semantic_field.as_bytes());
        encode_string_list(&mut encoded, &resolution.authorities);
        encode_string_list(&mut encoded, &resolution.observed_values);
        append_bytes(&mut encoded, resolution.action.as_bytes());
    }
    append_len(&mut encoded, value.synthesized_ancestors.len());
    for path in &value.synthesized_ancestors {
        append_bytes(&mut encoded, &encode_path(path));
    }
    encode_string_list(&mut encoded, &value.unsupported_metadata);
    append_bytes(&mut encoded, value.outcome.as_bytes());
    encoded
}

fn canonical_conversion(value: &ConversionProvenance) -> ConversionProvenance {
    let mut canonical = value.clone();
    let mut resolutions = canonical.resolutions.into_vec();
    resolutions.sort_by(resolution_order);
    resolutions.dedup();
    canonical.resolutions = resolutions.into_boxed_slice();
    let mut ancestors = canonical.synthesized_ancestors.into_vec();
    ancestors.sort();
    ancestors.dedup();
    canonical.synthesized_ancestors = ancestors.into_boxed_slice();
    let mut unsupported = canonical.unsupported_metadata.into_vec();
    unsupported.sort();
    unsupported.dedup();
    canonical.unsupported_metadata = unsupported.into_boxed_slice();
    canonical
}

fn resolution_order(
    left: &ConversionResolution,
    right: &ConversionResolution,
) -> std::cmp::Ordering {
    (
        &left.conflict_class,
        &left.semantic_field,
        &left.authorities,
        &left.observed_values,
        &left.action,
    )
        .cmp(&(
            &right.conflict_class,
            &right.semantic_field,
            &right.authorities,
            &right.observed_values,
            &right.action,
        ))
}

fn structured_hash(domain: &str, fields: &[&[u8]]) -> Digest {
    let mut hasher = Sha256::new();
    hasher.update(HASH_PREFIX);
    hasher.update(
        u64::try_from(domain.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    hasher.update(domain.as_bytes());
    hasher.update(
        u64::try_from(fields.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for field in fields {
        hasher.update(u64::try_from(field.len()).unwrap_or(u64::MAX).to_be_bytes());
        hasher.update(field);
    }
    Digest::from_bytes(hasher.finalize().into())
}

fn merkle_root(leaves: &[Digest], empty_domain: &str, node_domain: &str) -> Digest {
    match leaves {
        [] => structured_hash(empty_domain, &[]),
        [leaf] => *leaf,
        _ => {
            let split = largest_power_of_two_less_than(leaves.len());
            let left = merkle_root(&leaves[..split], empty_domain, node_domain);
            let right = merkle_root(&leaves[split..], empty_domain, node_domain);
            structured_hash(node_domain, &[left.as_bytes(), right.as_bytes()])
        }
    }
}

fn largest_power_of_two_less_than(value: usize) -> usize {
    debug_assert!(value > 1);
    let highest = 1_usize << (usize::BITS - 1 - value.leading_zeros());
    if highest == value {
        highest / 2
    } else {
        highest
    }
}

fn encode_path(path: &LogicalPath) -> Vec<u8> {
    let mut encoded = Vec::new();
    append_len(&mut encoded, path.components().len());
    for component in path.components() {
        encoded.push(1);
        append_bytes(&mut encoded, component.bytes());
    }
    encoded
}

fn encode_metadata(metadata: &MetadataSet, identity: bool) -> Vec<u8> {
    let items = metadata
        .items()
        .iter()
        .filter(|item| item.name().participates_in_identity_v1() == identity)
        .cloned()
        .collect::<Vec<_>>();
    let mut encoded = Vec::new();
    append_len(&mut encoded, items.len());
    for item in items {
        encode_metadata_item(&mut encoded, item);
    }
    encoded
}

fn encode_metadata_item(encoded: &mut Vec<u8>, item: MetadataItem) {
    append_bytes(encoded, item.name().as_str().as_bytes());
    encoded.push(match item.criticality() {
        crate::eam::Criticality::Optional => 0,
        crate::eam::Criticality::Critical => 1,
    });
    encoded.push(match item.restorability() {
        crate::eam::Restorability::Restorable => 1,
        crate::eam::Restorability::CaptureOnly => 2,
    });
    match item.value() {
        MetadataValue::Bool(value) => {
            encoded.push(1);
            encoded.push(u8::from(*value));
        }
        MetadataValue::Timestamp(value) => {
            encoded.push(2);
            encoded.extend_from_slice(&value.seconds().to_be_bytes());
            encoded.extend_from_slice(&value.nanoseconds().to_be_bytes());
            encoded.push(timestamp_precision_id(value.source_precision()));
            encoded.push(u8::from(value.restorable()));
        }
        MetadataValue::PosixMode(value) => {
            encoded.push(3);
            encoded.extend_from_slice(&value.to_be_bytes());
        }
        MetadataValue::PosixUid(value) => {
            encoded.push(4);
            encoded.extend_from_slice(&value.to_be_bytes());
        }
        MetadataValue::PosixGid(value) => {
            encoded.push(5);
            encoded.extend_from_slice(&value.to_be_bytes());
        }
        MetadataValue::HardlinkGroup(value) => {
            encoded.push(6);
            encoded.extend_from_slice(value.as_bytes());
        }
        MetadataValue::Xattrs(values) => {
            encoded.push(7);
            append_len(encoded, values.len());
            for value in values {
                append_bytes(encoded, value.name());
                append_bytes(encoded, value.value());
            }
        }
        MetadataValue::SparseMap(value) => {
            encoded.push(8);
            encoded.extend_from_slice(&value.logical_size().to_be_bytes());
            append_len(encoded, value.extents().len());
            for extent in value.extents() {
                encoded.extend_from_slice(&extent.offset.to_be_bytes());
                encoded.extend_from_slice(&extent.length.to_be_bytes());
            }
        }
        MetadataValue::Acls(values) => {
            encoded.push(9);
            append_len(encoded, values.len());
            for acl in values {
                encoded.push(match acl.dialect() {
                    crate::eam::AclDialect::Posix1e => 1,
                    crate::eam::AclDialect::Nfs4 => 2,
                });
                encoded.push(match acl.scope() {
                    crate::eam::AclScope::Access => 1,
                    crate::eam::AclScope::Default => 2,
                });
                append_len(encoded, acl.entries().len());
                for entry in acl.entries() {
                    encoded.push(match entry.entry_type() {
                        crate::eam::AclEntryType::Allow => 1,
                        crate::eam::AclEntryType::Deny => 2,
                        crate::eam::AclEntryType::Audit => 3,
                        crate::eam::AclEntryType::Alarm => 4,
                    });
                    encode_acl_principal(encoded, entry.principal());
                    encoded.extend_from_slice(&entry.permissions().to_be_bytes());
                    encoded.extend_from_slice(&entry.flags().to_be_bytes());
                }
            }
        }
        MetadataValue::WindowsSecurityDescriptor(value) => {
            encoded.push(10);
            append_bytes(encoded, value.bytes());
        }
        MetadataValue::WindowsFileAttributes(value) => {
            encoded.push(11);
            encoded.extend_from_slice(&value.to_be_bytes());
        }
        MetadataValue::WindowsReparseOriginal(value) => {
            encoded.push(12);
            encoded.extend_from_slice(&value.tag().to_be_bytes());
            append_bytes(encoded, value.data());
        }
        MetadataValue::MacosFlags(value) => {
            encoded.push(13);
            encoded.extend_from_slice(&value.to_be_bytes());
        }
    }
}

fn encode_acl_principal(encoded: &mut Vec<u8>, principal: &crate::eam::AclPrincipal) {
    match principal {
        crate::eam::AclPrincipal::UserObj => encoded.push(1),
        crate::eam::AclPrincipal::User(value) => {
            encoded.push(2);
            encoded.extend_from_slice(&value.to_be_bytes());
        }
        crate::eam::AclPrincipal::GroupObj => encoded.push(3),
        crate::eam::AclPrincipal::Group(value) => {
            encoded.push(4);
            encoded.extend_from_slice(&value.to_be_bytes());
        }
        crate::eam::AclPrincipal::Mask => encoded.push(5),
        crate::eam::AclPrincipal::Other => encoded.push(6),
        crate::eam::AclPrincipal::OwnerAt => encoded.push(7),
        crate::eam::AclPrincipal::GroupAt => encoded.push(8),
        crate::eam::AclPrincipal::EveryoneAt => encoded.push(9),
        crate::eam::AclPrincipal::Uuid(value) => {
            encoded.push(10);
            encoded.extend_from_slice(value);
        }
    }
}

fn encode_fidelity(fidelity: &FidelityReport) -> Vec<u8> {
    let mut encoded = Vec::new();
    encode_string_list(&mut encoded, &fidelity.captured);
    encode_issues(&mut encoded, &fidelity.unavailable);
    encode_issues(&mut encoded, &fidelity.degraded);
    append_bytes(&mut encoded, fidelity.platform.as_bytes());
    encode_string_list(&mut encoded, &fidelity.filesystem);
    encoded
}

fn encode_string_list(encoded: &mut Vec<u8>, values: &[String]) {
    append_len(encoded, values.len());
    for value in values {
        append_bytes(encoded, value.as_bytes());
    }
}

fn encode_issues(encoded: &mut Vec<u8>, issues: &[FidelityIssue]) {
    append_len(encoded, issues.len());
    for issue in issues {
        append_bytes(encoded, issue.class.as_bytes());
        append_bytes(encoded, issue.reason.as_bytes());
        if let Some(path) = &issue.entry_scope {
            encoded.push(1);
            append_bytes(encoded, &encode_path(path));
        } else {
            encoded.push(0);
        }
    }
}

fn canonical_fidelity(fidelity: &FidelityReport) -> FidelityReport {
    let mut canonical = fidelity.clone();
    let mut captured = canonical.captured.into_vec();
    captured.sort();
    captured.dedup();
    canonical.captured = captured.into_boxed_slice();
    let mut filesystem = canonical.filesystem.into_vec();
    filesystem.sort();
    filesystem.dedup();
    canonical.filesystem = filesystem.into_boxed_slice();
    let mut unavailable = canonical.unavailable.into_vec();
    unavailable.sort_by(issue_order);
    unavailable.dedup();
    canonical.unavailable = unavailable.into_boxed_slice();
    let mut degraded = canonical.degraded.into_vec();
    degraded.sort_by(issue_order);
    degraded.dedup();
    canonical.degraded = degraded.into_boxed_slice();
    canonical
}

fn issue_order(left: &FidelityIssue, right: &FidelityIssue) -> std::cmp::Ordering {
    (&left.class, &left.reason, &left.entry_scope).cmp(&(
        &right.class,
        &right.reason,
        &right.entry_scope,
    ))
}

fn timestamp_precision_id(value: TimestampPrecision) -> u8 {
    match value {
        TimestampPrecision::Second => 1,
        TimestampPrecision::Centisecond => 2,
        TimestampPrecision::Microsecond => 3,
        TimestampPrecision::Hectonanosecond => 4,
        TimestampPrecision::Nanosecond => 5,
    }
}

fn append_len(encoded: &mut Vec<u8>, value: usize) {
    encoded.extend_from_slice(&u64::try_from(value).unwrap_or(u64::MAX).to_be_bytes());
}

fn append_bytes(encoded: &mut Vec<u8>, value: &[u8]) {
    append_len(encoded, value.len());
    encoded.extend_from_slice(value);
}

fn resource(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ResourceLimit,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        build_content, entry_aux_digest, entry_identity_digest, hardlink_group_id,
        physical_container_identity, sha256_exact,
    };
    use crate::eam::{
        Acl, AclDialect, AclEntry, AclEntryType, AclPrincipal, AclScope, ContentRef, Digest, Entry,
        EntryData, EntryIdentity, LinkTarget, LogicalPath, MetadataItem, MetadataSet, SparseExtent,
        SparseMap, WindowsReparsePoint, WindowsSecurityDescriptor,
    };

    fn file_entry(path: &str, metadata: MetadataSet) -> Entry {
        Entry::new(
            LogicalPath::from_utf8([path]).unwrap(),
            EntryData::File {
                content: ContentRef::Internal(sha256_exact(b"same content")),
            },
            metadata,
            EntryIdentity::default(),
        )
    }

    #[test]
    fn sha256_known_answer() {
        assert_eq!(
            sha256_exact(b"abc").to_string(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn zero_length_content_has_no_chunks() {
        let (content, chunks) = build_content(&[], 1024, 1).unwrap();
        assert_eq!(content.logical_digest, sha256_exact(&[]));
        assert!(content.chunks.is_empty());
        assert!(chunks.is_empty());
        assert_ne!(content.chunk_root, Digest::ZERO);
    }

    #[test]
    fn pci_changes_with_any_exact_byte_change() {
        assert_ne!(
            physical_container_identity(b"one"),
            physical_container_identity(b"two")
        );
    }

    #[test]
    fn posix_identity_tiers_are_frozen() {
        let base = file_entry(
            "a",
            MetadataSet::new(vec![MetadataItem::executable(false)]).unwrap(),
        );
        let mode = file_entry(
            "a",
            MetadataSet::new(vec![
                MetadataItem::executable(false),
                MetadataItem::posix_mode(0o640),
            ])
            .unwrap(),
        );
        assert_eq!(entry_identity_digest(&base), entry_identity_digest(&mode));
        assert_ne!(entry_aux_digest(&base), entry_aux_digest(&mode));

        let executable = file_entry(
            "a",
            MetadataSet::new(vec![
                MetadataItem::executable(true),
                MetadataItem::posix_mode(0o750),
            ])
            .unwrap(),
        );
        assert_ne!(
            entry_identity_digest(&mode),
            entry_identity_digest(&executable)
        );

        let sparse = file_entry(
            "a",
            MetadataSet::new(vec![
                MetadataItem::executable(false),
                MetadataItem::sparse_map(
                    SparseMap::new(
                        12,
                        vec![SparseExtent {
                            offset: 4,
                            length: 4,
                        }],
                    )
                    .unwrap(),
                ),
            ])
            .unwrap(),
        );
        assert_eq!(entry_identity_digest(&base), entry_identity_digest(&sparse));
        assert_ne!(entry_aux_digest(&base), entry_aux_digest(&sparse));

        let link_a = Entry::new(
            LogicalPath::from_utf8(["link"]).unwrap(),
            EntryData::Symlink {
                target: LinkTarget::canonical(b"a".to_vec().into_boxed_slice()).unwrap(),
            },
            MetadataSet::default(),
            EntryIdentity::default(),
        );
        let link_b = Entry::new(
            LogicalPath::from_utf8(["link"]).unwrap(),
            EntryData::Symlink {
                target: LinkTarget::canonical(b"b".to_vec().into_boxed_slice()).unwrap(),
            },
            MetadataSet::default(),
            EntryIdentity::default(),
        );
        assert_ne!(
            entry_identity_digest(&link_a),
            entry_identity_digest(&link_b)
        );
    }

    #[test]
    fn hardlink_group_is_path_sorted_and_inode_independent() {
        let content = sha256_exact(b"payload");
        let left = LogicalPath::from_utf8(["a"]).unwrap();
        let right = LogicalPath::from_utf8(["b"]).unwrap();
        let expected = hardlink_group_id(content, &[left.clone(), right.clone()]).unwrap();
        assert_eq!(
            expected,
            hardlink_group_id(content, &[right, left]).unwrap()
        );
        assert_eq!(
            expected.to_string(),
            "e7dad4aaaea2d134d9a56c71f5dfa84be64c7b40a2a4a9c953fbeede3437dc61"
        );
    }

    #[test]
    fn platform_security_identity_tiers_are_frozen() {
        let base = file_entry(
            "a",
            MetadataSet::new(vec![MetadataItem::executable(false)]).unwrap(),
        );
        let acl = Acl::new(
            AclDialect::Posix1e,
            AclScope::Access,
            vec![
                AclEntry::new(AclEntryType::Allow, AclPrincipal::UserObj, 6, 0).unwrap(),
                AclEntry::new(AclEntryType::Allow, AclPrincipal::GroupObj, 4, 0).unwrap(),
                AclEntry::new(AclEntryType::Allow, AclPrincipal::Other, 0, 0).unwrap(),
            ],
        )
        .unwrap();
        let acl_entry = file_entry(
            "a",
            MetadataSet::new(vec![
                MetadataItem::executable(false),
                MetadataItem::acls(vec![acl]).unwrap(),
            ])
            .unwrap(),
        );
        assert_eq!(
            entry_identity_digest(&base),
            entry_identity_digest(&acl_entry)
        );
        assert_ne!(entry_aux_digest(&base), entry_aux_digest(&acl_entry));

        let mut descriptor = vec![0_u8; 20];
        descriptor[0] = 1;
        descriptor[2..4].copy_from_slice(&0x8000_u16.to_le_bytes());
        let secured = file_entry(
            "a",
            MetadataSet::new(vec![
                MetadataItem::executable(false),
                MetadataItem::windows_security_descriptor(
                    WindowsSecurityDescriptor::new(descriptor).unwrap(),
                ),
            ])
            .unwrap(),
        );
        assert_eq!(
            entry_identity_digest(&base),
            entry_identity_digest(&secured)
        );
        assert_ne!(entry_aux_digest(&base), entry_aux_digest(&secured));

        let flagged = file_entry(
            "a",
            MetadataSet::new(vec![
                MetadataItem::executable(false),
                MetadataItem::macos_flags(0x2).unwrap(),
            ])
            .unwrap(),
        );
        assert_eq!(
            entry_identity_digest(&base),
            entry_identity_digest(&flagged)
        );
        assert_ne!(entry_aux_digest(&base), entry_aux_digest(&flagged));

        let left = Entry::new(
            LogicalPath::from_utf8(["rp"]).unwrap(),
            EntryData::ReparsePoint {
                value: WindowsReparsePoint::new(0x8000_001b, b"one".to_vec()).unwrap(),
            },
            MetadataSet::default(),
            EntryIdentity::default(),
        );
        let right = Entry::new(
            LogicalPath::from_utf8(["rp"]).unwrap(),
            EntryData::ReparsePoint {
                value: WindowsReparsePoint::new(0x8000_001b, b"two".to_vec()).unwrap(),
            },
            MetadataSet::default(),
            EntryIdentity::default(),
        );
        assert_ne!(entry_identity_digest(&left), entry_identity_digest(&right));

        let symlink = |original: &[u8]| {
            Entry::new(
                LogicalPath::from_utf8(["link"]).unwrap(),
                EntryData::Symlink {
                    target: LinkTarget::canonical(b"target".to_vec()).unwrap(),
                },
                MetadataSet::new(vec![MetadataItem::windows_reparse_original(
                    WindowsReparsePoint::new(0xa000_000c, original.to_vec()).unwrap(),
                )])
                .unwrap(),
                EntryIdentity::default(),
            )
        };
        let symlink_a = symlink(b"original-a");
        let symlink_b = symlink(b"original-b");
        assert_eq!(
            entry_identity_digest(&symlink_a),
            entry_identity_digest(&symlink_b)
        );
        assert_ne!(entry_aux_digest(&symlink_a), entry_aux_digest(&symlink_b));
    }
}
