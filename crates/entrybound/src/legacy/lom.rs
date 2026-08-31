//! The format-neutral Legacy Observation Model (LOM).

use crate::eam::Digest;

/// One independently observed foreign authority.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LegacyAuthority {
    pub format: String,
    pub structure: String,
    pub instance: u64,
}

/// Exact source-byte location supporting an observation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LegacyEvidenceLocation {
    pub offset: u64,
    pub length: u64,
}

/// Parser-level state of one observation, before reconciliation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ObservationValidity {
    Valid,
    Invalid,
    Uninterpreted,
}

/// Type-erased values used in the common report while format adapters retain
/// richer internal types for reconciliation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LegacyObservedValue {
    Bytes(Box<[u8]>),
    Unsigned(u64),
    Signed(i64),
    Text(String),
    Boolean(bool),
}

impl LegacyObservedValue {
    #[must_use]
    pub fn display_compact(&self) -> String {
        match self {
            Self::Bytes(value) => format!("{} bytes", value.len()),
            Self::Unsigned(value) => value.to_string(),
            Self::Signed(value) => value.to_string(),
            Self::Text(value) => value.clone(),
            Self::Boolean(value) => value.to_string(),
        }
    }
}

/// One raw and optionally interpreted claim from one foreign authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyFieldObservation<T> {
    pub semantic_field: String,
    pub authority: LegacyAuthority,
    pub raw_value: Box<[u8]>,
    pub interpreted_value: Option<T>,
    pub evidence: LegacyEvidenceLocation,
    pub validity: ObservationValidity,
}

/// All observations associated with one foreign entry identity/ordinal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyEntryObservation {
    pub ordinal: u64,
    pub fields: Box<[LegacyFieldObservation<LegacyObservedValue>]>,
}

/// The four frozen conflict classes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConflictClass {
    Omission,
    Refinement,
    Divergence,
    Irreconcilable,
}

impl ConflictClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Omission => "omission",
            Self::Refinement => "refinement",
            Self::Divergence => "divergence",
            Self::Irreconcilable => "irreconcilable",
        }
    }
}

/// A policy decision made after observation and classification.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LegacyResolution {
    pub action: String,
    pub selected_authority: Option<LegacyAuthority>,
}

/// Competing evidence about one prospective semantic fact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyConflict {
    pub semantic_field: String,
    pub authorities: Box<[LegacyAuthority]>,
    pub observed_values: Box<[LegacyObservedValue]>,
    pub evidence: Box<[LegacyEvidenceLocation]>,
    pub classification: ConflictClass,
    pub resolution: Option<LegacyResolution>,
}

/// Complete format-neutral evidence emitted by one adapter invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyArchiveObservation {
    pub source_format: String,
    pub source_digest: Digest,
    pub archive_fields: Box<[LegacyFieldObservation<LegacyObservedValue>]>,
    pub entries: Box<[LegacyEntryObservation]>,
    pub conflicts: Box<[LegacyConflict]>,
}

impl LegacyArchiveObservation {
    #[must_use]
    pub fn observation_count(&self) -> u64 {
        self.archive_fields
            .len()
            .saturating_add(
                self.entries
                    .iter()
                    .map(|entry| entry.fields.len())
                    .sum::<usize>(),
            )
            .try_into()
            .unwrap_or(u64::MAX)
    }

    #[must_use]
    pub fn conflict_count(&self, class: ConflictClass) -> u64 {
        self.conflicts
            .iter()
            .filter(|conflict| conflict.classification == class)
            .count()
            .try_into()
            .unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_classes_are_stable() {
        assert_eq!(ConflictClass::Omission.as_str(), "omission");
        assert_eq!(ConflictClass::Refinement.as_str(), "refinement");
        assert_eq!(ConflictClass::Divergence.as_str(), "divergence");
        assert_eq!(ConflictClass::Irreconcilable.as_str(), "irreconcilable");
    }
}
