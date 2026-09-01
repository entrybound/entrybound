//! Format-neutral legacy observation and reconciliation boundary.
//!
//! Adapters in this module describe foreign evidence. They never make foreign
//! container structures authoritative EAM state.

pub mod import;
pub mod lom;
pub mod sevenz;
pub mod stream;
pub mod tar;
pub mod zip;

pub use lom::{
    ConflictClass, LegacyArchiveObservation, LegacyAuthority, LegacyConflict,
    LegacyEntryObservation, LegacyEvidenceLocation, LegacyFieldObservation, LegacyObservedValue,
    LegacyResolution, ObservationValidity,
};
