//! Entrybound's native semantic and format foundations.
//!
//! The Entrybound Archive Model (EAM) is authoritative. ECF is only an
//! encoding of that model, indexes are caches, and the CLI is a consumer of
//! this library rather than a second implementation of archive semantics.

pub mod archive;
mod canonical;
pub mod chunker;
mod codec;
pub mod crypto;
pub mod diagnostics;
pub mod eam;
pub mod ecf;
pub mod identity;
mod jpeg_reconstruction;
pub mod legacy;
pub mod planner;
pub mod random_access;
mod reconstruction;
pub mod similarity;
mod transform;

/// Default-off, read-only research access to crate internals.
///
/// Compiled only with the `research-internals` cargo feature. Every item is a
/// thin wrapper over, or an enumeration built from, the exact private functions
/// production uses; nothing here alters production behavior or archive bytes.
/// These APIs are not stable: promoting any of them to a production API requires
/// an accepted Decision Ledger entry.
#[cfg(feature = "research-internals")]
pub mod research {
    pub use crate::codec::research as codec;
    pub use crate::ecf::research as ecf;
    pub use crate::jpeg_reconstruction::research as jpeg_reconstruction;
    pub use crate::planner::research as planner;
    pub use crate::reconstruction::research as reconstruction;
    pub use crate::similarity::research as similarity;
    pub use crate::transform::research as transform;
}
