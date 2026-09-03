//! Archive-operation policy boundaries.
//!
//! Callers construct policy and archive bytes never mutate it.

use crate::eam::{DecodeRequirements, ResourceBudget};

mod filesystem;
mod inspection;
mod tooling;

pub(crate) use filesystem::{
    plan_directory_encrypted, plan_observed_archive, replan_archive_encrypted,
};

pub use filesystem::{
    ExtractionReport, PackOptions, default_pack_output, default_unpack_destination, pack_directory,
    pack_directory_stream, plan_directory, replan_archive, unpack, unpack_opened, unpack_stream,
};
pub use inspection::{
    ArchiveInspection, ChunkStatistics, CodecUsage, CompressionExplanation, CrossFileInspection,
    ListedEntry, PlanInspection, ReconstructionInspection, TransformUsage, WholeObjectInspection,
    explain, inspect, list,
};
pub use tooling::{
    ArchiveDiffReport, DiffChange, DiffIdentityStatus, DiffTier, EvidenceClass, ExplanationFact,
    IndexPolicy, InspectionSecurity, InspectionViews, PhysicalDiffSummary, PreparedRepack,
    RepackAnalysis, RepackMode, RepackOptions, StructuredExplanation, archive_diff,
    archive_metadata_diff, inspection_json, inspection_json_with_security, prepare_repack,
    random_inspection_json, structured_explain,
};

/// Explicit, deliberately generous limits for the experimental bootstrap CLI.
/// Applications should construct narrower limits for their own environment.
#[must_use]
pub const fn bootstrap_resource_policy() -> ResourceBudget {
    ResourceBudget {
        entry_count: 1_000_000,
        total_logical_bytes: 64 * 1024 * 1024 * 1024,
        max_single_entry_logical_bytes: 16 * 1024 * 1024 * 1024,
        max_expansion_ratio_milli: 64_000_000,
        chunk_count: 4_000_000,
        max_path_depth: 1_024,
        max_metadata_bytes: 1024 * 1024 * 1024,
        max_key_derivation_cost: 0,
    }
}

/// Decoder memory ceilings used by the experimental CLI.
///
/// V6 permits a bounded 256 MiB JPEG/JPEG XL reconstruction working set.
/// Applications may provide a narrower caller-owned policy.
#[must_use]
pub const fn bootstrap_decode_policy() -> DecodeRequirements {
    DecodeRequirements {
        window_bytes: 8 * 1024 * 1024,
        working_set_bytes: 384 * 1024 * 1024,
        flags: 0,
    }
}

/// Caller-owned handling for an extraction collision.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CollisionPolicy {
    /// Safest default: do not touch an existing destination object.
    #[default]
    Refuse,
}

/// Caller-owned policy for materializing archived symbolic links.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SymlinkPolicy {
    /// Refuse every symbolic link.
    Refuse,
    /// Permit only relative targets whose lexical resolution stays beneath the extraction root.
    #[default]
    Safe,
    /// Restore exact targets, including absolute and escaping targets.
    All,
}

/// Caller-owned ownership restoration policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OwnershipPolicy {
    #[default]
    Ignore,
    Restore,
}

/// Caller-owned extended-attribute restoration policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum XAttrPolicy {
    #[default]
    Ignore,
    Restore,
}

/// Caller-owned sparse-file restoration policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SparsePolicy {
    /// Materialize the complete logical byte sequence.
    #[default]
    Logical,
    /// Recreate declared data/hole extents where supported.
    Restore,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AclPolicy {
    #[default]
    Ignore,
    Restore,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WindowsSecurityPolicy {
    #[default]
    Ignore,
    Restore,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReparsePolicy {
    #[default]
    Refuse,
    KnownSafe,
    All,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PlatformMetadataPolicy {
    #[default]
    Ignore,
    Restore,
}

/// The containment guarantee an extractor actually achieved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfinementMode {
    /// Component resolution is confined by the operating-system kernel.
    KernelEnforced,
    /// A weaker platform fallback was used and must be reported.
    WeakerReported,
}

/// Immutable, caller-constructed extraction policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtractionPolicy {
    collision: CollisionPolicy,
    budget: ResourceBudget,
    decode: DecodeRequirements,
    symlinks: SymlinkPolicy,
    ownership: OwnershipPolicy,
    xattrs: XAttrPolicy,
    sparse: SparsePolicy,
    acls: AclPolicy,
    windows_security: WindowsSecurityPolicy,
    reparse: ReparsePolicy,
    platform_metadata: PlatformMetadataPolicy,
}

impl ExtractionPolicy {
    /// Constructs policy without consulting archive-controlled data.
    #[must_use]
    pub const fn new(collision: CollisionPolicy, budget: ResourceBudget) -> Self {
        Self {
            collision,
            budget,
            decode: bootstrap_decode_policy(),
            symlinks: SymlinkPolicy::Safe,
            ownership: OwnershipPolicy::Ignore,
            xattrs: XAttrPolicy::Ignore,
            sparse: SparsePolicy::Logical,
            acls: AclPolicy::Ignore,
            windows_security: WindowsSecurityPolicy::Ignore,
            reparse: ReparsePolicy::Refuse,
            platform_metadata: PlatformMetadataPolicy::Ignore,
        }
    }

    /// Constructs policy with explicit archive-size and decoder-memory limits.
    #[must_use]
    pub const fn new_with_decode(
        collision: CollisionPolicy,
        budget: ResourceBudget,
        decode: DecodeRequirements,
    ) -> Self {
        Self {
            collision,
            budget,
            decode,
            symlinks: SymlinkPolicy::Safe,
            ownership: OwnershipPolicy::Ignore,
            xattrs: XAttrPolicy::Ignore,
            sparse: SparsePolicy::Logical,
            acls: AclPolicy::Ignore,
            windows_security: WindowsSecurityPolicy::Ignore,
            reparse: ReparsePolicy::Refuse,
            platform_metadata: PlatformMetadataPolicy::Ignore,
        }
    }

    #[must_use]
    pub const fn collision(self) -> CollisionPolicy {
        self.collision
    }

    #[must_use]
    pub const fn budget(self) -> ResourceBudget {
        self.budget
    }

    #[must_use]
    pub const fn decode(self) -> DecodeRequirements {
        self.decode
    }

    #[must_use]
    pub const fn with_symlinks(mut self, value: SymlinkPolicy) -> Self {
        self.symlinks = value;
        self
    }

    #[must_use]
    pub const fn with_ownership(mut self, value: OwnershipPolicy) -> Self {
        self.ownership = value;
        self
    }

    #[must_use]
    pub const fn with_xattrs(mut self, value: XAttrPolicy) -> Self {
        self.xattrs = value;
        self
    }

    #[must_use]
    pub const fn with_sparse(mut self, value: SparsePolicy) -> Self {
        self.sparse = value;
        self
    }

    #[must_use]
    pub const fn with_acls(mut self, value: AclPolicy) -> Self {
        self.acls = value;
        self
    }

    #[must_use]
    pub const fn with_windows_security(mut self, value: WindowsSecurityPolicy) -> Self {
        self.windows_security = value;
        self
    }

    #[must_use]
    pub const fn with_reparse(mut self, value: ReparsePolicy) -> Self {
        self.reparse = value;
        self
    }

    #[must_use]
    pub const fn with_platform_metadata(mut self, value: PlatformMetadataPolicy) -> Self {
        self.platform_metadata = value;
        self
    }

    #[must_use]
    pub const fn symlinks(self) -> SymlinkPolicy {
        self.symlinks
    }

    #[must_use]
    pub const fn ownership(self) -> OwnershipPolicy {
        self.ownership
    }

    #[must_use]
    pub const fn xattrs(self) -> XAttrPolicy {
        self.xattrs
    }

    #[must_use]
    pub const fn sparse(self) -> SparsePolicy {
        self.sparse
    }

    #[must_use]
    pub const fn acls(self) -> AclPolicy {
        self.acls
    }

    #[must_use]
    pub const fn windows_security(self) -> WindowsSecurityPolicy {
        self.windows_security
    }

    #[must_use]
    pub const fn reparse(self) -> ReparsePolicy {
        self.reparse
    }

    #[must_use]
    pub const fn platform_metadata(self) -> PlatformMetadataPolicy {
        self.platform_metadata
    }
}

impl Default for ExtractionPolicy {
    fn default() -> Self {
        Self::new(CollisionPolicy::Refuse, bootstrap_resource_policy())
    }
}

/// Complexity exposed by an Entry cursor instead of hidden behind one API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryCursorComplexity {
    Sequential,
    RandomAccess,
}

#[cfg(test)]
mod tests {
    use super::{CollisionPolicy, ExtractionPolicy};

    #[test]
    fn extraction_refuses_collisions_by_default() {
        assert_eq!(
            ExtractionPolicy::default().collision(),
            CollisionPolicy::Refuse
        );
    }
}
