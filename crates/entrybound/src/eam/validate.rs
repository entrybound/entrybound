use std::collections::{BTreeMap, BTreeSet};

use super::{
    Archive, ArchiveRole, ContentRef, Digest, Entry, EntryData, EntryKind, EntrySet, Layout,
};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::identity::sha256_exact;

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
                    Some(EntryKind::File) => {
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
                && !self.content_store.objects.contains_key(&digest)
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownContentObject,
                    entry.path().to_string(),
                ));
            }
        }
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
            let object = self.content_store.objects.get(&digest).ok_or_else(|| {
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
    use crate::eam::{Digest, EntryIdentity, LogicalPath, MetadataSet};

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
}
