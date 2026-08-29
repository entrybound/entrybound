//! Archive-operation policy boundaries.
//!
//! Packing, opening, verification, and extraction will be implemented here;
//! callers construct policy and archive bytes never mutate it.

use crate::eam::ResourceBudget;

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
}

impl ExtractionPolicy {
    /// Constructs policy without consulting archive-controlled data.
    #[must_use]
    pub const fn new(collision: CollisionPolicy, budget: ResourceBudget) -> Self {
        Self { collision, budget }
    }

    #[must_use]
    pub const fn collision(self) -> CollisionPolicy {
        self.collision
    }

    #[must_use]
    pub const fn budget(self) -> ResourceBudget {
        self.budget
    }
}

impl Default for ExtractionPolicy {
    fn default() -> Self {
        Self::new(CollisionPolicy::Refuse, ResourceBudget::default())
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
