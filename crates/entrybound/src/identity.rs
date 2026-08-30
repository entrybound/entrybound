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
    Archive, Chunk, ChunkRef, ContentObject, ContentRef, Digest, Entry, EntryData, EntryIdentity,
    EntrySet, FidelityIssue, FidelityReport, LogicalPath, MetadataItem, MetadataSet, MetadataValue,
    TimestampPrecision,
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
                entry.data(),
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
        } => (2, 1, Some(digest)),
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
    let conversion_absent = structured_hash("conversion/absent/v1", &[]);
    AuxiliaryRoot(structured_hash(
        "aux/v1",
        &[
            b"sha256",
            entry_aux_root.as_bytes(),
            fidelity.as_bytes(),
            conversion_absent.as_bytes(),
        ],
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
        .copied()
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
            encoded.push(u8::from(value));
        }
        MetadataValue::Timestamp(value) => {
            encoded.push(2);
            encoded.extend_from_slice(&value.seconds().to_be_bytes());
            encoded.extend_from_slice(&value.nanoseconds().to_be_bytes());
            encoded.push(timestamp_precision_id(value.source_precision()));
            encoded.push(u8::from(value.restorable()));
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
    use super::{build_content, physical_container_identity, sha256_exact};
    use crate::eam::Digest;

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
}
