//! Identity and integrity types for the native format.
//!
//! Hash computation is intentionally not implemented by the language migration
//! itself. The types preserve the required separation so ECF code cannot
//! accidentally substitute one identity for another.

use crate::eam::Digest;

/// The logically stable identity descriptor result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogicalArchiveIdentity(pub Digest);

/// The chunking-dependent physical content root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalContentRoot(pub Digest);

/// The non-identity metadata, fidelity, and provenance root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuxiliaryRoot(pub Digest);

/// The digest of every exact serialized container byte.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalContainerIdentity(pub Digest);

/// The four identities exposed by an opened archive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentitySet {
    pub lai: LogicalArchiveIdentity,
    pub pcr: PhysicalContentRoot,
    pub aux: AuxiliaryRoot,
    pub pci: PhysicalContainerIdentity,
}

/// A reader must carry verification state rather than invite inference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationState {
    /// Bytes were checked against their plaintext Chunk digest.
    ChunkVerified,
    /// Bytes were checked against a Merkle slice rooted in PCR.
    PhysicalRootVerified,
    /// Bytes were returned without an available integrity proof.
    Unverified,
}
