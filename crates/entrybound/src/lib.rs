//! Entrybound's native semantic and format foundations.
//!
//! The Entrybound Archive Model (EAM) is authoritative. ECF is only an
//! encoding of that model, indexes are caches, and the CLI is a consumer of
//! this library rather than a second implementation of archive semantics.

pub mod archive;
mod canonical;
pub mod chunker;
mod codec;
pub mod diagnostics;
pub mod eam;
pub mod ecf;
pub mod identity;
pub mod planner;
pub mod similarity;
mod transform;
