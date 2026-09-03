use std::collections::{BTreeMap, BTreeSet};

use super::{
    AclDialect, AclPrincipal, AclScope, Archive, ArchiveRole, ContentRef, Digest, Entry, EntryData,
    EntryKind, EntrySet, Layout, MetadataName,
};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::identity::sha256_exact;

fn platform_metadata_limit(message: &'static str) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ResourceLimit,
        message,
    )
}

impl EntrySet {
    /// Sorts enumeration-independent input into canonical order and validates P4/P6.
    pub fn new(mut entries: Vec<Entry>) -> Result<Self> {
        entries.sort_by(|left, right| left.path().cmp(right.path()));
        Self::from_canonical(entries)
    }

    /// Validates entries already encoded in claimed canonical order.
    pub fn from_canonical(entries: Vec<Entry>) -> Result<Self> {
        for pair in entries.windows(2) {
            match pair[0].path().cmp(pair[1].path()) {
                std::cmp::Ordering::Equal => {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::DuplicateLogicalPath,
                        pair[1].path().to_string(),
                    ));
                }
                std::cmp::Ordering::Greater => {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::NoncanonicalEncoding,
                        "entries are not in canonical LogicalPath order",
                    ));
                }
                std::cmp::Ordering::Less => {}
            }
        }

        let by_path = entries
            .iter()
            .map(|entry| (entry.path(), entry.kind()))
            .collect::<BTreeMap<_, _>>();
        for entry in &entries {
            for depth in 1..entry.path().depth() {
                let ancestor = entry.path().prefix(depth);
                match by_path.get(&ancestor) {
                    None => {
                        return Err(Diagnostic::new(
                            OutcomeClass::Nonconforming,
                            ReasonCode::MissingAncestor,
                            ancestor.to_string(),
                        ));
                    }
                    Some(EntryKind::File | EntryKind::Symlink | EntryKind::ReparsePoint) => {
                        return Err(Diagnostic::new(
                            OutcomeClass::Nonconforming,
                            ReasonCode::FileAsAncestor,
                            ancestor.to_string(),
                        ));
                    }
                    Some(EntryKind::Directory) => {}
                }
            }
        }
        Ok(Self {
            entries: entries.into_boxed_slice(),
        })
    }
}

impl Archive {
    /// Validates semantic references and the supported bootstrap profile.
    ///
    /// This is the complete check and requires every Chunk to carry its
    /// retained plaintext.
    pub fn validate(&self) -> Result<()> {
        self.validate_without_retained_plaintext()?;
        self.validate_retained_plaintext_lengths()
    }

    /// Validates every EAM invariant that does not read retained Chunk
    /// plaintext.
    ///
    /// The sequential STREAM reader verifies each Chunk's plaintext digest as
    /// the bytes arrive and does not retain plaintext afterwards, so it
    /// validates the model through this entry point instead. Every invariant
    /// checked here is identical for both layouts.
    pub fn validate_without_retained_plaintext(&self) -> Result<()> {
        if self.descriptor.role != ArchiveRole::Complete {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnsupportedRequiredFeature,
                "only Complete archives are supported by the bootstrap profile",
            ));
        }
        if self.descriptor.layout == Layout::Indexed && self.descriptor.stream_dedup_window != 0 {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::NoncanonicalEncoding,
                "INDEXED archives must declare a zero STREAM dedup window",
            ));
        }
        let conversion_feature =
            self.descriptor.features.incompat & crate::ecf::FEATURE_CONVERSION_PROVENANCE_V1 != 0;
        if conversion_feature != self.conversion.is_some() {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                "conversion-provenance-v1 feature and ConversionProvenance presence disagree",
            ));
        }
        let preservation_feature =
            self.descriptor.features.incompat & crate::ecf::FEATURE_LEGACY_PRESERVATION_V1 != 0;
        if preservation_feature != self.preservation.is_some()
            || preservation_feature && !conversion_feature
        {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                "legacy-preservation-v1 requires exactly one ConversionProvenance and one preservation object",
            ));
        }
        let posix_feature =
            self.descriptor.features.incompat & crate::ecf::FEATURE_POSIX_METADATA_V1 != 0;
        let uses_posix = self.entry_set.entries().iter().any(Entry::uses_posix_v1);
        let platform_feature = self.descriptor.features.incompat
            & crate::ecf::FEATURE_PLATFORM_SECURITY_METADATA_V1
            != 0;
        let uses_platform = self
            .entry_set
            .entries()
            .iter()
            .any(Entry::uses_platform_security_v1);
        if platform_feature != uses_platform || uses_posix && !posix_feature {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::UnsupportedRequiredFeature,
                "metadata feature declarations disagree with Entry-v2/v3 semantics",
            ));
        }
        if platform_feature && !posix_feature {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::UnsupportedRequiredFeature,
                "platform-security-metadata-v1 requires posix-metadata-v1",
            ));
        }
        if posix_feature && !uses_posix && !platform_feature {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::UnsupportedRequiredFeature,
                "posix-metadata-v1 is set without versioned metadata semantics",
            ));
        }
        if let Some(preservation) = &self.preservation {
            if preservation.preservation_format != "legacy-preservation/v1"
                || preservation.source_format != "ZIP"
                || crate::identity::sha256_exact(&preservation.source_bytes)
                    != preservation.source_digest
                || self
                    .conversion
                    .as_ref()
                    .is_none_or(|conversion| conversion.source_digest != preservation.source_digest)
                || self.conversion.as_ref().is_none_or(|conversion| {
                    conversion.resolutions != preservation.selected_resolutions
                })
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::AuxMismatch,
                    "legacy preservation source identity or format is invalid",
                ));
            }
            if preservation.observations.windows(2).any(|pair| {
                (
                    pair[0].scope,
                    pair[0].subject_ordinal,
                    pair[0].observation_ordinal,
                ) >= (
                    pair[1].scope,
                    pair[1].subject_ordinal,
                    pair[1].observation_ordinal,
                )
            }) || preservation
                .conflicts
                .windows(2)
                .any(|pair| pair[0].ordinal >= pair[1].ordinal)
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::NoncanonicalEncoding,
                    "legacy preservation observations/conflicts are not canonically ordered",
                ));
            }
            let source_len = u64::try_from(preservation.source_bytes.len()).unwrap_or(u64::MAX);
            if preservation.observations.iter().any(|item| {
                !matches!(item.scope, 0 | 1)
                    || item.scope == 1
                        && self.conversion.as_ref().is_none_or(|conversion| {
                            item.subject_ordinal >= conversion.source_entry_count
                        })
                    || item
                        .evidence
                        .offset
                        .checked_add(item.evidence.length)
                        .is_none_or(|end| end > source_len)
            }) || preservation.conflicts.iter().any(|conflict| {
                conflict.authorities.is_empty()
                    || conflict.authorities.len() != conflict.observed_values.len()
                    || conflict.authorities.len() != conflict.evidence.len()
                    || conflict.evidence.iter().any(|location| {
                        location
                            .offset
                            .checked_add(location.length)
                            .is_none_or(|end| end > source_len)
                    })
            }) {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::NoncanonicalEncoding,
                    "legacy preservation scope, evidence extent, or conflict cardinality is invalid",
                ));
            }
        }

        let plans = self
            .transform_plans
            .iter()
            .map(|plan| plan.plan_id)
            .collect::<BTreeSet<_>>();
        if plans.len() != self.transform_plans.len() {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                "TransformPlan identifiers must be unique",
            ));
        }
        let referenced_dictionaries = self
            .transform_plans
            .iter()
            .filter_map(|plan| plan.dictionary)
            .collect::<BTreeSet<_>>();
        for dictionary_id in &referenced_dictionaries {
            if !self.content_store.dictionaries.contains_key(dictionary_id) {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownDictionary,
                    dictionary_id.to_string(),
                ));
            }
        }
        if referenced_dictionaries.len() != self.content_store.dictionaries.len() {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                "every stored Dictionary must be referenced by a TransformPlan",
            ));
        }
        let referenced_reconstruction = self
            .transform_plans
            .iter()
            .flat_map(|plan| plan.transforms.iter())
            .filter_map(|step| step.reconstruction_ref)
            .collect::<BTreeSet<_>>();
        for reconstruction_id in &referenced_reconstruction {
            if !self
                .content_store
                .reconstruction_data
                .contains_key(reconstruction_id)
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownReconstructionData,
                    reconstruction_id.to_string(),
                ));
            }
        }
        if referenced_reconstruction.len() != self.content_store.reconstruction_data.len() {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                "every stored ReconstructionData object must be referenced by a TransformPlan",
            ));
        }

        let physical = self
            .content_store
            .physical_order
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if physical.len() != self.content_store.physical_order.len()
            || physical.len() != self.content_store.chunks.len()
            || physical
                != self
                    .content_store
                    .chunks
                    .keys()
                    .copied()
                    .collect::<BTreeSet<_>>()
        {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::InvalidGroupOrdering,
                "physical Chunk order must contain every unique Chunk exactly once",
            ));
        }

        for (digest, dictionary) in &self.content_store.dictionaries {
            if digest != &dictionary.dictionary_id {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::DuplicateSemanticDeclaration,
                    "Dictionary map key differs from its authoritative dictionary_id",
                ));
            }
            if sha256_exact(&dictionary.bytes) != dictionary.dictionary_id {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::DictionaryDigestMismatch,
                    dictionary.dictionary_id.to_string(),
                ));
            }
        }

        for (digest, data) in &self.content_store.reconstruction_data {
            if digest != &data.reconstruction_id {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::DuplicateSemanticDeclaration,
                    "ReconstructionData map key differs from its authoritative identity",
                ));
            }
            crate::reconstruction::validate_data(data)?;
        }

        let mut region_ranges = BTreeMap::<Digest, Vec<(u64, u64, Digest)>>::new();
        let mut region_owned_chunks = BTreeMap::<Digest, Digest>::new();
        for (region_id, region) in &self.content_store.reconstruction_regions {
            if region_id != &region.region_id
                || crate::jpeg_reconstruction::region_identity(region) != region.region_id
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::InvalidReconstructionRegion,
                    "ReconstructionRegion identity does not match its canonical physical fields",
                ));
            }
            let object = self
                .content_store
                .objects
                .get(&region.content_object)
                .ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::UnknownContentObject,
                        region.content_object.to_string(),
                    )
                })?;
            let representation_len = u64::try_from(region.representation.len()).map_err(|_| {
                Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::ResourceLimit,
                    "ReconstructionRegion representation exceeds u64",
                )
            })?;
            if representation_len == 0
                || representation_len
                    > u64::try_from(crate::jpeg_reconstruction::MAX_JXL_BYTES).unwrap_or(u64::MAX)
                || region.logical_bytes
                    > representation_len
                        .saturating_mul(crate::jpeg_reconstruction::MAX_REGION_EXPANSION_RATIO)
            {
                return Err(Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::ResourceLimit,
                    "ReconstructionRegion exceeds the v1 representation/expansion bounds",
                ));
            }
            let start = usize::try_from(region.start_chunk_index).map_err(|_| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidReconstructionRegion,
                    "ReconstructionRegion start exceeds usize",
                )
            })?;
            let count = usize::try_from(region.chunk_count).map_err(|_| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidReconstructionRegion,
                    "ReconstructionRegion count exceeds usize",
                )
            })?;
            let end = start.checked_add(count).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidReconstructionRegion,
                    "ReconstructionRegion range overflows",
                )
            })?;
            if count == 0
                || region.chunk_count > crate::jpeg_reconstruction::MAX_REGION_CHUNKS
                || end > object.chunks.len()
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidReconstructionRegion,
                    "ReconstructionRegion range is outside its ContentObject",
                ));
            }
            let plan = self
                .transform_plans
                .iter()
                .find(|plan| plan.plan_id == region.plan_ref)
                .ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Unsupported,
                        ReasonCode::UnknownTransformPlan,
                        format!("region {region_id} references plan {}", region.plan_ref),
                    )
                })?;
            if plan.dictionary.is_some()
                || !plan.transforms.first().is_some_and(|step| {
                    crate::transform::is_whole_object_reconstructive(step).unwrap_or(false)
                })
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidReconstructionRegion,
                    "ReconstructionRegion requires an independent self-contained reconstructive plan",
                ));
            }
            let mut logical_bytes = 0_u64;
            for chunk_ref in &object.chunks[start..end] {
                let chunk = self
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
                logical_bytes = logical_bytes
                    .checked_add(chunk.logical_len)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::PolicyRefused,
                            ReasonCode::ResourceLimit,
                            "ReconstructionRegion logical length overflows",
                        )
                    })?;
                if chunk.plan_ref != crate::jpeg_reconstruction::REGION_MEMBER_PLAN_REF
                    || chunk.group_ref.is_some()
                {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::InvalidReconstructionRegion,
                        "region-owned Chunks cannot carry an independent plan or ChunkGroup",
                    ));
                }
                if let Some(other) = region_owned_chunks.insert(chunk.chunk_id, *region_id)
                    && other != *region_id
                {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::OverlappingReconstructionRegion,
                        format!("Chunk {} belongs to conflicting regions", chunk.chunk_id),
                    ));
                }
            }
            if logical_bytes != region.logical_bytes
                || region.access.logical_bytes != region.logical_bytes
                || region.access.logical_chunks != region.chunk_count
                || region.access.worst_reconstructed_bytes != region.logical_bytes
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidRegionAccess,
                    format!("ReconstructionRegion {region_id} access declaration is inconsistent"),
                ));
            }
            let region_end = region
                .start_chunk_index
                .checked_add(region.chunk_count)
                .ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::InvalidReconstructionRegion,
                        "ReconstructionRegion range overflows",
                    )
                })?;
            region_ranges
                .entry(region.content_object)
                .or_default()
                .push((region.start_chunk_index, region_end, *region_id));
        }
        for ranges in region_ranges.values_mut() {
            ranges.sort_by_key(|range| range.0);
            if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::OverlappingReconstructionRegion,
                    "ReconstructionRegions overlap within one ContentObject",
                ));
            }
        }

        for (target, audit) in &self.content_store.reconstruction_audits {
            if target != &audit.target || audit.transform_id.is_empty() {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::DuplicateSemanticDeclaration,
                    "ReconstructionAudit key and explicit target must agree",
                ));
            }
            let exists = match target {
                super::ReconstructionAuditTarget::Chunk(digest) => {
                    self.content_store.chunks.contains_key(digest)
                }
                super::ReconstructionAuditTarget::ContentObject(digest) => {
                    self.content_store.objects.contains_key(digest)
                }
                super::ReconstructionAuditTarget::Region(digest) => self
                    .content_store
                    .reconstruction_regions
                    .contains_key(digest),
            };
            if !exists {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownReconstructionRegion,
                    "ReconstructionAudit target does not exist",
                ));
            }
        }

        for (digest, group) in &self.content_store.chunk_groups {
            if digest != &group.group_id || group.group_id == Digest::ZERO {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::DuplicateSemanticDeclaration,
                    "ChunkGroup map key and non-zero authoritative group_id must agree",
                ));
            }
            if group.max_lookback == 0 {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::LookbackViolation,
                    "stored ChunkGroups must declare non-zero bounded lookback",
                ));
            }
        }

        for (digest, chunk) in &self.content_store.chunks {
            if digest != &chunk.chunk_id {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::DuplicateSemanticDeclaration,
                    "Chunk map key differs from the authoritative chunk_id",
                ));
            }
            if !region_owned_chunks.contains_key(digest)
                && chunk.plan_ref == crate::jpeg_reconstruction::REGION_MEMBER_PLAN_REF
                && self.descriptor.features.incompat
                    & crate::ecf::FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1
                    != 0
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownReconstructionRegion,
                    format!("region-owned Chunk declaration {digest} has no region"),
                ));
            }
            if !region_owned_chunks.contains_key(digest) && !plans.contains(&chunk.plan_ref) {
                return Err(Diagnostic::new(
                    OutcomeClass::Unsupported,
                    ReasonCode::UnknownTransformPlan,
                    format!("chunk {digest} references plan {}", chunk.plan_ref),
                ));
            }
            if region_owned_chunks.contains_key(digest)
                && chunk.plan_ref != crate::jpeg_reconstruction::REGION_MEMBER_PLAN_REF
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidReconstructionRegion,
                    format!("region-owned Chunk {digest} has an independent plan"),
                ));
            }
            if let Some(group_id) = chunk.group_ref
                && !self.content_store.chunk_groups.contains_key(&group_id)
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidGroupReference,
                    format!("chunk {digest} references unknown group {group_id}"),
                ));
            }
        }

        for chunk_id in self.content_store.reconstruction_fallbacks.keys() {
            let chunk = self.content_store.chunks.get(chunk_id).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownChunk,
                    format!("ReconstructionFallback references unknown Chunk {chunk_id}"),
                )
            })?;
            let plan = self
                .transform_plans
                .iter()
                .find(|plan| plan.plan_id == chunk.plan_ref)
                .expect("Chunk plan reference was validated above");
            if plan
                .transforms
                .iter()
                .any(|step| step.reconstruction_ref.is_some())
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::DuplicateSemanticDeclaration,
                    format!(
                        "Chunk {chunk_id} cannot carry both a selected reconstruction and a fallback audit"
                    ),
                ));
            }
        }

        self.validate_group_access_costs()?;

        for (digest, object) in &self.content_store.objects {
            if digest != &object.logical_digest {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::DuplicateSemanticDeclaration,
                    "ContentObject map key differs from its logical_digest",
                ));
            }
            for chunk in &object.chunks {
                if !self.content_store.chunks.contains_key(&chunk.chunk_id) {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::UnknownChunk,
                        chunk.chunk_id.to_string(),
                    ));
                }
            }
        }

        for entry in self.entry_set.entries() {
            if let EntryData::File {
                content: ContentRef::Internal(digest),
            } = entry.data()
                && !self.content_store.objects.contains_key(digest)
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownContentObject,
                    entry.path().to_string(),
                ));
            }
        }
        self.validate_posix_metadata()?;
        self.validate_platform_security_metadata()?;
        Ok(())
    }

    /// Checks the one invariant that requires retained Chunk plaintext.
    fn validate_retained_plaintext_lengths(&self) -> Result<()> {
        for chunk in self.content_store.chunks.values() {
            if chunk.logical_len != u64::try_from(chunk.plaintext.len()).unwrap_or(u64::MAX) {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::SectionStructure,
                    "Chunk logical_len differs from its plaintext byte length",
                ));
            }
        }
        self.validate_sparse_holes()?;
        Ok(())
    }

    fn validate_posix_metadata(&self) -> Result<()> {
        let mut hardlinks = BTreeMap::<Digest, Vec<&Entry>>::new();
        for entry in self.entry_set.entries() {
            if let Some(mode) = entry.metadata().posix_mode()
                && entry
                    .metadata()
                    .items()
                    .iter()
                    .any(|item| item.name() == MetadataName::CoreExecutable)
                && entry.metadata().executable() != (mode & 0o111 != 0)
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidPosixMetadata,
                    format!(
                        "core.executable and posix.mode disagree for {}",
                        entry.path()
                    ),
                ));
            }
            if entry.metadata().sparse_map().is_some()
                && !matches!(entry.data(), EntryData::File { .. })
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidSparseMap,
                    format!("sparse map is valid only for File Entry {}", entry.path()),
                ));
            }
            if let Some(group) = entry.metadata().hardlink_group() {
                if !matches!(entry.data(), EntryData::File { .. }) {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::InvalidHardlinkGroup,
                        format!(
                            "hardlink metadata requires a File Entry at {}",
                            entry.path()
                        ),
                    ));
                }
                hardlinks.entry(group).or_default().push(entry);
            }
        }
        for (stored_group, members) in hardlinks {
            if members.len() < 2 {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidHardlinkGroup,
                    "a hardlink group must contain at least two File Entries",
                ));
            }
            let content = match members[0].data() {
                EntryData::File {
                    content: ContentRef::Internal(value),
                } => *value,
                _ => unreachable!(),
            };
            let baseline = members[0]
                .metadata()
                .items()
                .iter()
                .filter(|item| item.name() != MetadataName::PosixHardlinkGroup)
                .collect::<Vec<_>>();
            let mut paths = Vec::with_capacity(members.len());
            for member in members {
                let member_content = match member.data() {
                    EntryData::File {
                        content: ContentRef::Internal(value),
                    } => *value,
                    _ => unreachable!(),
                };
                let metadata = member
                    .metadata()
                    .items()
                    .iter()
                    .filter(|item| item.name() != MetadataName::PosixHardlinkGroup)
                    .collect::<Vec<_>>();
                if member_content != content || metadata != baseline {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::InvalidHardlinkGroup,
                        "hardlink members must share ContentObject and inode-scoped metadata",
                    ));
                }
                paths.push(member.path().clone());
            }
            if crate::identity::hardlink_group_id(content, &paths)? != stored_group {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::InvalidHardlinkGroup,
                    "hardlink group ID does not match canonical membership",
                ));
            }
        }
        Ok(())
    }

    fn validate_platform_security_metadata(&self) -> Result<()> {
        let mut aggregate = 0_u64;
        for entry in self.entry_set.entries() {
            for acl in entry.metadata().acls() {
                if acl.scope() == AclScope::Default && !matches!(entry.data(), EntryData::Directory)
                {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::InvalidAcl,
                        format!("DEFAULT ACL requires Directory Entry {}", entry.path()),
                    ));
                }
                if acl.dialect() == AclDialect::Posix1e && acl.scope() == AclScope::Access {
                    let mode = entry.metadata().posix_mode().ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::Nonconforming,
                            ReasonCode::InvalidAcl,
                            format!("POSIX1E ACCESS ACL requires posix.mode on {}", entry.path()),
                        )
                    })?;
                    let permission = |principal: &AclPrincipal| {
                        acl.entries()
                            .iter()
                            .find(|candidate| candidate.principal() == principal)
                            .map(super::AclEntry::permissions)
                    };
                    let owner = permission(&AclPrincipal::UserObj).unwrap_or_default();
                    let other = permission(&AclPrincipal::Other).unwrap_or_default();
                    let group = permission(&AclPrincipal::Mask)
                        .or_else(|| permission(&AclPrincipal::GroupObj))
                        .unwrap_or_default();
                    if owner != (mode >> 6) & 7 || group != (mode >> 3) & 7 || other != mode & 7 {
                        return Err(Diagnostic::new(
                            OutcomeClass::Nonconforming,
                            ReasonCode::InvalidAcl,
                            format!("POSIX1E ACL and posix.mode disagree for {}", entry.path()),
                        ));
                    }
                }
                aggregate = aggregate
                    .checked_add(u64::try_from(acl.entries().len()).unwrap_or(u64::MAX) * 32)
                    .ok_or_else(|| platform_metadata_limit("ACL metadata length overflow"))?;
            }
            if entry.metadata().windows_reparse_original().is_some()
                && !matches!(entry.data(), EntryData::Symlink { .. })
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidReparsePoint,
                    format!(
                        "windows.reparse-original requires Symlink Entry {}",
                        entry.path()
                    ),
                ));
            }
            if let Some(value) = entry.metadata().windows_security_descriptor() {
                aggregate = aggregate
                    .checked_add(u64::try_from(value.bytes().len()).unwrap_or(u64::MAX))
                    .ok_or_else(|| platform_metadata_limit("security metadata length overflow"))?;
            }
            if let Some(value) = entry.metadata().windows_reparse_original() {
                aggregate = aggregate
                    .checked_add(u64::try_from(value.data().len()).unwrap_or(u64::MAX))
                    .ok_or_else(|| platform_metadata_limit("reparse metadata length overflow"))?;
            }
            if let EntryData::ReparsePoint { value } = entry.data() {
                aggregate = aggregate
                    .checked_add(u64::try_from(value.data().len()).unwrap_or(u64::MAX))
                    .ok_or_else(|| platform_metadata_limit("reparse Entry length overflow"))?;
            }
        }
        // A zero budget is the in-memory pre-serialization sentinel. The ECF
        // writer replaces it with the exact canonical metadata bound, and
        // readers compare that authenticated declaration with independently
        // derived section sizes before this model-level validation runs.
        if self.descriptor.budget.max_metadata_bytes != 0
            && aggregate > self.descriptor.budget.max_metadata_bytes
        {
            return Err(platform_metadata_limit(
                "platform/security metadata exceeds declared ResourceBudget",
            ));
        }
        Ok(())
    }

    fn validate_sparse_holes(&self) -> Result<()> {
        for entry in self.entry_set.entries() {
            let Some(map) = entry.metadata().sparse_map() else {
                continue;
            };
            let EntryData::File {
                content: ContentRef::Internal(digest),
            } = entry.data()
            else {
                continue;
            };
            let object = &self.content_store.objects[digest];
            let logical_size = object.chunks.iter().try_fold(0_u64, |total, reference| {
                total
                    .checked_add(self.content_store.chunks[&reference.chunk_id].logical_len)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::PolicyRefused,
                            ReasonCode::ResourceLimit,
                            "sparse ContentObject size overflows u64",
                        )
                    })
            })?;
            if map.logical_size() != logical_size {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidSparseMap,
                    format!("sparse logical size disagrees for {}", entry.path()),
                ));
            }
            let mut offset = 0_u64;
            for reference in &object.chunks {
                let chunk = &self.content_store.chunks[&reference.chunk_id];
                map.validate_range(offset, &chunk.plaintext)?;
                offset = offset.checked_add(chunk.logical_len).ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::PolicyRefused,
                        ReasonCode::ResourceLimit,
                        "sparse validation offset overflows u64",
                    )
                })?;
            }
        }
        Ok(())
    }

    fn validate_group_access_costs(&self) -> Result<()> {
        let mut positions = BTreeMap::<Digest, Vec<usize>>::new();
        for (position, chunk_id) in self.content_store.physical_order.iter().enumerate() {
            if let Some(group_id) = self.content_store.chunks[chunk_id].group_ref {
                positions.entry(group_id).or_default().push(position);
            }
        }
        if positions.len() != self.content_store.chunk_groups.len() {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::InvalidGroupReference,
                "every declared ChunkGroup must have physical Chunk members",
            ));
        }
        for (group_id, member_positions) in positions {
            if member_positions.len() < 2
                || member_positions
                    .windows(2)
                    .any(|pair| pair[1] != pair[0] + 1)
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidGroupOrdering,
                    format!("ChunkGroup {group_id} must contain one contiguous physical run"),
                ));
            }
            let group = &self.content_store.chunk_groups[&group_id];
            let lookback = usize::try_from(group.max_lookback).map_err(|_| {
                Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::ResourceLimit,
                    "ChunkGroup lookback exceeds usize",
                )
            })?;
            let mut maximum = 0_u64;
            for (member_index, position) in member_positions.iter().enumerate() {
                let first = member_index.saturating_sub(lookback);
                let preceding = member_positions[first..member_index].iter().try_fold(
                    0_u64,
                    |total, preceding_position| {
                        let chunk_id = self.content_store.physical_order[*preceding_position];
                        total
                            .checked_add(self.content_store.chunks[&chunk_id].logical_len)
                            .ok_or_else(|| {
                                Diagnostic::new(
                                    OutcomeClass::PolicyRefused,
                                    ReasonCode::ResourceLimit,
                                    "ChunkGroup access byte count exceeds u64",
                                )
                            })
                    },
                )?;
                maximum = maximum.max(preceding);
                if *position != member_positions[0] + member_index {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::InvalidGroupOrdering,
                        group_id.to_string(),
                    ));
                }
            }
            if maximum != group.max_preceding_bytes {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::AccessCostMismatch,
                    format!(
                        "ChunkGroup {group_id} declares {} preceding bytes but requires {maximum}",
                        group.max_preceding_bytes
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Derives total logical bytes without introducing a duplicate size authority.
    pub fn total_logical_size(&self) -> Result<u64> {
        let mut total = 0_u64;
        for entry in self.entry_set.entries() {
            let EntryData::File {
                content: ContentRef::Internal(digest),
            } = entry.data()
            else {
                continue;
            };
            let object = self.content_store.objects.get(digest).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownContentObject,
                    entry.path().to_string(),
                )
            })?;
            for chunk_ref in &object.chunks {
                let chunk = self
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
                total = total.checked_add(chunk.logical_len).ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::PolicyRefused,
                        ReasonCode::ResourceLimit,
                        "total logical size exceeds u64",
                    )
                })?;
            }
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eam::{
        ArchiveDescriptor, ArchiveRole, ContentStore, DecodeRequirements, Digest, DigestAlgorithm,
        EntryIdentity, FeatureSet, FidelityReport, IdentityProfile, Index, Layout, LogicalPath,
        MetadataSet, ResourceBudget, WindowsReparsePoint,
    };

    fn directory(path: &[&str]) -> Entry {
        Entry::new(
            LogicalPath::from_utf8(path).unwrap(),
            EntryData::Directory,
            MetadataSet::default(),
            EntryIdentity::default(),
        )
    }

    fn file(path: &[&str]) -> Entry {
        Entry::new(
            LogicalPath::from_utf8(path).unwrap(),
            EntryData::File {
                content: ContentRef::Internal(Digest::ZERO),
            },
            MetadataSet::default(),
            EntryIdentity::default(),
        )
    }

    #[test]
    fn duplicate_paths_are_rejected() {
        let error = EntrySet::new(vec![file(&["a"]), file(&["a"])]).unwrap_err();
        assert_eq!(error.code(), ReasonCode::DuplicateLogicalPath);
    }

    #[test]
    fn file_prefix_conflicts_are_rejected() {
        let error = EntrySet::new(vec![file(&["a"]), file(&["a", "b"])]).unwrap_err();
        assert_eq!(error.code(), ReasonCode::FileAsAncestor);
    }

    #[test]
    fn every_ancestor_must_exist_explicitly() {
        let error = EntrySet::new(vec![file(&["a", "b"])]).unwrap_err();
        assert_eq!(error.code(), ReasonCode::MissingAncestor);
    }

    #[test]
    fn ordering_is_independent_of_enumeration_order() {
        let first =
            EntrySet::new(vec![file(&["a", "b"]), directory(&["a"]), file(&["z"])]).unwrap();
        let second =
            EntrySet::new(vec![file(&["z"]), file(&["a", "b"]), directory(&["a"])]).unwrap();
        let first_paths = first
            .entries()
            .iter()
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        let second_paths = second
            .entries()
            .iter()
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        assert_eq!(first_paths, second_paths);
    }

    fn reparse_archive(features: u64) -> Archive {
        Archive {
            descriptor: ArchiveDescriptor {
                format_major: 0,
                format_minor: 1,
                format_namespace: crate::ecf::FORMAT_NAMESPACE.to_owned(),
                features: FeatureSet {
                    incompat: features,
                    ..FeatureSet::default()
                },
                layout: Layout::Indexed,
                role: ArchiveRole::Complete,
                budget_declared: true,
                stream_dedup_window: 0,
                budget: ResourceBudget {
                    entry_count: 1,
                    max_path_depth: 1,
                    max_metadata_bytes: 1024,
                    max_expansion_ratio_milli: 1000,
                    ..ResourceBudget::default()
                },
                decode: DecodeRequirements::default(),
                identity_profile: IdentityProfile::IdentityV1,
                digest_algorithm: DigestAlgorithm::Sha256,
                planner_id: "test".to_owned(),
                chunker_id: "test".to_owned(),
                lai: Digest::ZERO,
                pcr: Digest::ZERO,
                aux: Digest::ZERO,
                pci: None,
            },
            entry_set: EntrySet::new(vec![Entry::new(
                LogicalPath::from_utf8(["opaque"]).unwrap(),
                EntryData::ReparsePoint {
                    value: WindowsReparsePoint::new(0x8000_001b, b"opaque".to_vec()).unwrap(),
                },
                MetadataSet::default(),
                EntryIdentity::default(),
            )])
            .unwrap(),
            content_store: ContentStore::default(),
            transform_plans: Box::default(),
            fidelity: FidelityReport::default(),
            conversion: None,
            preservation: None,
            index: Index::default(),
        }
    }

    #[test]
    fn platform_security_feature_requires_posix_and_v3_semantics() {
        let both = crate::ecf::FEATURE_POSIX_METADATA_V1
            | crate::ecf::FEATURE_PLATFORM_SECURITY_METADATA_V1;
        reparse_archive(both).validate().unwrap();
        assert_eq!(
            reparse_archive(crate::ecf::FEATURE_PLATFORM_SECURITY_METADATA_V1)
                .validate()
                .unwrap_err()
                .code(),
            ReasonCode::UnsupportedRequiredFeature
        );
        assert_eq!(
            reparse_archive(crate::ecf::FEATURE_POSIX_METADATA_V1)
                .validate()
                .unwrap_err()
                .code(),
            ReasonCode::UnsupportedRequiredFeature
        );
    }
}
