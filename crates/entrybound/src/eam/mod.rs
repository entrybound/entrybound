//! The supported Entrybound Archive Model subset.

mod logical_path;
mod model;
mod validate;

pub use logical_path::{LogicalPath, PathComponent, PathEncoding};
pub use model::{
    Archive, ArchiveDescriptor, ArchiveRole, Chunk, ChunkGroup, ChunkLocation, ChunkRef,
    ContentObject, ContentRef, ContentStore, ConversionProvenance, ConversionResolution,
    Criticality, DecodeRequirements, Dictionary, Digest, DigestAlgorithm, Entry, EntryData,
    EntryIdentity, EntryKind, EntrySet, FeatureSet, FidelityIssue, FidelityReport, IdentityProfile,
    Index, Layout, LegacyPreservation, MetadataItem, MetadataName, MetadataSet, MetadataValue,
    PreservedLegacyAuthority, PreservedLegacyConflict, PreservedLegacyLocation,
    PreservedLegacyObservation, PreservedLegacyResolution, PreservedLegacyValidity,
    PreservedLegacyValue, ReconstructionAudit, ReconstructionAuditReason,
    ReconstructionAuditTarget, ReconstructionData, ReconstructionFallbackReason,
    ReconstructionRegion, RegionAccessCost, ResourceBudget, Restorability, Timestamp,
    TimestampPrecision, TransformPlan, TransformStep,
};
