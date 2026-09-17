//! End-to-end archive measurements using entrybound's production API:
//! `capture -> chunk -> plan -> encode -> write -> open -> verify`, with
//! exact section-level byte accounting derived by walking the encoded
//! container through production parsing APIs, plus logical counts, per-codec
//! stored/logical bytes, planner id, per-phase wall-clock, and this
//! process's peak memory. See `README.md` for the full picture and how this
//! crate's four binaries (`ebr-pack`, `ebr-unpack`, `ebr-verify`,
//! `ebr-inspect-bytes`) plug into an `ebr` runner spec.
//!
//! # Module map
//!
//! - [`pack`]: the production `planning`/`encoding` pack pipeline
//!   (INDEXED, STREAM, and encrypted-INDEXED) with phase-scoped memory,
//!   plus a deterministic-repeat check.
//! - [`bytes`]: the exact section-level byte-accounting walk. Granular
//!   (down to individual Chunk frame headers/payloads, attributed by codec
//!   and transform) for plaintext INDEXED archives; a coarser, still
//!   exactly-summed accounting for STREAM and encrypted archives, whose
//!   production APIs do not expose an equivalent physical section
//!   directory (see that module's docs for why).
//! - [`counts`]: a `Serialize`-able mirror of the logical counts and
//!   per-codec statistics `entrybound::archive::inspect` reports (that
//!   type itself does not derive `Serialize`).
//! - [`encrypt`]: fresh, ephemeral test-only X-Wing recipient and password
//!   generation for `--encrypt` runs.
//! - [`scratch`]: peak on-disk scratch-directory byte-usage sampling for
//!   `ebr-unpack`.
//! - [`unpack`] / [`verify`]: standalone measurements over an
//!   already-encoded archive file, independent of a fresh pack.
//!
//! Nothing in this crate changes production behavior or archive bytes; it
//! only calls entrybound's public API (`research-internals` is enabled so
//! [`entrybound::research::ecf::parse_chunk_frame_header`] is reachable for
//! the byte-accounting walk -- the only `entrybound::research` item this
//! crate uses).

pub mod bytes;
pub mod counts;
pub mod encrypt;
pub mod pack;
pub mod scratch;
pub mod unpack;
pub mod verify;

use entrybound::eam::Layout;

/// Carried in every measurement row this crate's binaries write.
pub const BUILD_NOTE: &str = "research harness build: entrybound with research-internals \
    and research/harness/Cargo.lock; timing is not a production-CLI timing";
use entrybound::planner::CompressionProfile;

/// Parses `--profile fast|balanced|dense|extreme` (default `balanced`).
pub fn parse_profile(name: &str) -> Result<CompressionProfile, String> {
    match name {
        "fast" => Ok(CompressionProfile::Fast),
        "balanced" => Ok(CompressionProfile::Balanced),
        "dense" => Ok(CompressionProfile::Dense),
        "extreme" => Ok(CompressionProfile::Extreme),
        other => Err(format!(
            "unknown --profile {other:?} (expected fast, balanced, dense, or extreme)"
        )),
    }
}

/// Parses `--layout indexed|stream` (default `indexed`).
pub fn parse_layout(name: &str) -> Result<Layout, String> {
    match name {
        "indexed" => Ok(Layout::Indexed),
        "stream" => Ok(Layout::Stream),
        other => Err(format!(
            "unknown --layout {other:?} (expected indexed or stream)"
        )),
    }
}

/// The `--layout` spelling that produced a given [`Layout`], for JSON rows.
#[must_use]
pub fn layout_str(layout: Layout) -> &'static str {
    match layout {
        Layout::Indexed => "indexed",
        Layout::Stream => "stream",
    }
}

/// Parses a `--flag true|false`-shaped boolean. `ebr_common::cli::Args`
/// requires every flag to carry a value, so this crate's binaries spell a
/// boolean flag `--encrypt true` rather than a bare `--encrypt`.
pub fn parse_bool_flag(value: Option<&str>, default: bool) -> Result<bool, String> {
    match value {
        None => Ok(default),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(other) => Err(format!("expected true or false, got {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_known_profile() {
        assert_eq!(parse_profile("fast").unwrap(), CompressionProfile::Fast);
        assert_eq!(
            parse_profile("balanced").unwrap(),
            CompressionProfile::Balanced
        );
        assert_eq!(parse_profile("dense").unwrap(), CompressionProfile::Dense);
        assert_eq!(
            parse_profile("extreme").unwrap(),
            CompressionProfile::Extreme
        );
        assert!(parse_profile("bogus").is_err());
    }

    #[test]
    fn parses_every_known_layout_and_round_trips_its_spelling() {
        assert_eq!(parse_layout("indexed").unwrap(), Layout::Indexed);
        assert_eq!(parse_layout("stream").unwrap(), Layout::Stream);
        assert!(parse_layout("bogus").is_err());
        assert_eq!(layout_str(Layout::Indexed), "indexed");
        assert_eq!(layout_str(Layout::Stream), "stream");
    }

    #[test]
    fn parses_bool_flag_with_default_and_rejects_garbage() {
        assert!(!parse_bool_flag(None, false).unwrap());
        assert!(parse_bool_flag(None, true).unwrap());
        assert!(parse_bool_flag(Some("true"), false).unwrap());
        assert!(!parse_bool_flag(Some("false"), true).unwrap());
        assert!(parse_bool_flag(Some("yes"), false).is_err());
    }
}
