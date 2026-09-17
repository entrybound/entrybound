//! Random/remote access and streaming/memory measurements, over
//! entrybound's public production API.
//!
//! - [`measure_random_access`] (`src/random_access.rs`): INDEXED
//!   metadata-first random access (`entrybound::ecf::open_indexed_random`)
//!   over any `entrybound::random_access::RandomReadSource` (Memory,
//!   LocalFile, or Http today), with first-entry latency, a seeded random
//!   entry sample, random byte-range reads within large entries, an eager
//!   full-consumption pass, and every read's full
//!   `RandomAccessVerificationReport` (byte/range accounting, dependency
//!   Chunk count, and every verification flag).
//! - [`measure_stream`] (`src/stream.rs`): STREAM sequential
//!   encode -> open (verify + implicit list cost) -> verify -> list ->
//!   unpack (`entrybound::ecf::{encode_stream, open_stream, verify_stream}`,
//!   `entrybound::archive::{list, unpack_stream}`), with each pass's
//!   `StreamReport` memory/scratch fields
//!   (`peak_retained_chunks`/`peak_resident_staging_bytes`/
//!   `spilled_staging_bytes`) surfaced.
//! - [`probe`]: the streaming memory probe mode -- a synthetic,
//!   generated-on-the-fly input of a configurable size, run through
//!   pack/verify/unpack (and, opt-in, repack/export), sampling this
//!   process's own memory at fixed intervals throughout each stage.
//!
//! Every measurement here also attaches
//! [`ebr_common::measure::peak_memory`] where it is a single-shot reading is
//! useful, so a caller can compare INDEXED random access's metadata-first
//! footprint against STREAM's necessarily-more-sequential one, and both
//! against the streaming probe's interval samples.

mod probe;
mod random_access;
mod rng;
mod stream;

pub use probe::{
    MemorySample, ProbeConfig, ProbeError, ProbeMeasurement, ProbeStage, StageMeasurement,
    SyntheticGenerator, default_stages, run_probe, write_synthetic_file,
};
pub use random_access::{
    ByteRangeReadMeasurement, RandomAccessConfig, RandomAccessMeasurement, RandomReadMeasurement,
    VerificationFlags, measure_random_access,
};
pub use stream::{StreamMeasureError, StreamMeasurement, StreamPassStats, measure_stream};
