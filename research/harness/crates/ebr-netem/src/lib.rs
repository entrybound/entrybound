//! Network emulation proxy and range origin for remote-access research.
//!
//! **This crate is a skeleton only.** It is scaffolded as part of the
//! research harness workspace skeleton (Phase B2) so its Cargo.toml, lints,
//! and dependency on [`ebr_common`] are in place, but its actual
//! implementation is built out by a separate agent. See `README.md` for what
//! it needs to become and why it does not exist yet.
//!
//! `PROGRESS.md`'s execution-environment notes are the reason this crate
//! exists at all: `sch_netem` is absent in both WSL2 and Docker on this
//! host, so network emulation for remote-access experiments (latency,
//! packet loss, bandwidth caps against `entrybound::random_access`'s
//! `HttpRangeSource`) has to be application-level, not kernel-level. That is
//! a documented threat to validity this crate's eventual README/experiment
//! writeups must carry forward, not something this skeleton can paper over.

/// A placeholder so this crate compiles as a real library today. The actual
/// proxy/origin implementation replaces this; nothing outside this crate
/// should depend on `SKELETON_ONLY` remaining exactly this shape.
pub const SKELETON_ONLY: bool = true;
