//! Archive-operation policy boundaries.
//!
//! Callers construct policy and archive bytes never mutate it.

use crate::eam::{DecodeRequirements, ResourceBudget};

mod filesystem;
mod inspection;

pub use filesystem::{
    ExtractionReport, PackOptions, default_pack_output, default_unpack_destination, pack_directory,
    unpack,
};
pub use inspection::{
    ArchiveInspection, ChunkStatistics, CodecUsage, CompressionExplanation, CrossFileInspection,
    ListedEntry, PlanInspection, explain, inspect, list,
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
/// V3 archives add stored Dictionary residency and bounded group access bytes
/// to the 4 MiB Zstandard working set. Applications may provide narrower
/// caller-owned policy.
#[must_use]
pub const fn bootstrap_decode_policy() -> DecodeRequirements {
    DecodeRequirements {
        window_bytes: 1024 * 1024,
        working_set_bytes: 64 * 1024 * 1024,
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
}

impl ExtractionPolicy {
    /// Constructs policy without consulting archive-controlled data.
    #[must_use]
    pub const fn new(collision: CollisionPolicy, budget: ResourceBudget) -> Self {
        Self {
            collision,
            budget,
            decode: bootstrap_decode_policy(),
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
