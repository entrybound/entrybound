//! The supported Entrybound Archive Model subset.

mod logical_path;
mod model;
mod validate;

pub use logical_path::{LogicalPath, PathComponent, PathEncoding};
pub use model::{
    Archive, ArchiveDescriptor, ArchiveRole, Chunk, ChunkLocation, ChunkRef, ContentObject,
    ContentRef, ContentStore, Criticality, DecodeRequirements, Digest, DigestAlgorithm, Entry,
    EntryData, EntryIdentity, EntryKind, EntrySet, FeatureSet, FidelityIssue, FidelityReport,
    IdentityProfile, Index, Layout, MetadataItem, MetadataName, MetadataSet, MetadataValue,
    ResourceBudget, Restorability, Timestamp, TimestampPrecision, TransformPlan,
};
