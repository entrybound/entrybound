//! Bounded staging for plaintext produced by the sequential reader.
//!
//! A sequential reader learns a Chunk's semantic destination only after the
//! Manifest record that references it, so decoded plaintext must be held
//! somewhere between the `CHUNK_FRAME` item and the record. Holding it all in
//! RAM would make memory scale with archive size, so this store keeps a bounded
//! resident working set and spills the remainder to a private temporary file.
//!
//! The temporary file is the reader's own scratch space, not the archive. The
//! "no `Seek`" guarantee applies to the archive source; seeking inside a file
//! this process created and owns is unrelated to it. The file is created with
//! `create_new`, is never reachable through a caller-supplied path, and is
//! removed when the store is dropped, including on failure and truncation.

use std::collections::{BTreeMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::Digest;

/// Caller-owned limits for sequential staging.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingLimits {
    /// Maximum plaintext held in memory before spilling to temporary storage.
    pub memory_bytes: u64,
    /// Maximum plaintext staged in total, across memory and temporary storage.
    pub total_bytes: u64,
}

impl StagingLimits {
    /// Limits that never spill, for callers that forbid temporary files.
    #[must_use]
    pub const fn memory_only(memory_bytes: u64) -> Self {
        Self {
            memory_bytes,
            total_bytes: memory_bytes,
        }
    }
}

static NEXT_STAGING_ID: AtomicU64 = AtomicU64::new(0);

/// A bounded plaintext store addressed by Chunk identity.
#[derive(Debug)]
pub(crate) struct ChunkStaging {
    limits: StagingLimits,
    resident: BTreeMap<Digest, Vec<u8>>,
    resident_order: VecDeque<Digest>,
    resident_bytes: u64,
    spilled: BTreeMap<Digest, SpillLocation>,
    spill: Option<SpillFile>,
    spill_len: u64,
    staged_bytes: u64,
    peak_resident_bytes: u64,
    spilled_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SpillLocation {
    offset: u64,
    len: u64,
}

#[derive(Debug)]
struct SpillFile {
    file: File,
    path: PathBuf,
}

impl Drop for SpillFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl ChunkStaging {
    /// Creates an empty store. No temporary file exists until one is needed.
    pub(crate) const fn new(limits: StagingLimits) -> Self {
        Self {
            limits,
            resident: BTreeMap::new(),
            resident_order: VecDeque::new(),
            resident_bytes: 0,
            spilled: BTreeMap::new(),
            spill: None,
            spill_len: 0,
            staged_bytes: 0,
            peak_resident_bytes: 0,
            spilled_bytes: 0,
        }
    }

    /// Largest resident working set observed so far.
    pub(crate) const fn peak_resident_bytes(&self) -> u64 {
        self.peak_resident_bytes
    }

    /// Bytes written to temporary backing storage so far.
    pub(crate) const fn spilled_bytes(&self) -> u64 {
        self.spilled_bytes
    }

    /// Number of Chunks currently retained.
    pub(crate) fn len(&self) -> usize {
        self.resident.len() + self.spilled.len()
    }

    /// Whether the store currently retains this Chunk.
    pub(crate) fn contains(&self, chunk_id: &Digest) -> bool {
        self.resident.contains_key(chunk_id) || self.spilled.contains_key(chunk_id)
    }

    /// Stages one verified plaintext, spilling older entries when needed.
    pub(crate) fn insert(&mut self, chunk_id: Digest, plaintext: Vec<u8>) -> Result<()> {
        if self.contains(&chunk_id) {
            return Ok(());
        }
        let len = u64::try_from(plaintext.len()).map_err(|_| resource("Chunk exceeds u64"))?;
        self.staged_bytes = self
            .staged_bytes
            .checked_add(len)
            .ok_or_else(|| resource("staged byte total exceeds u64"))?;
        if self.staged_bytes > self.limits.total_bytes {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                "sequential staging exceeds the caller's total staging limit",
            ));
        }
        while self.resident_bytes.saturating_add(len) > self.limits.memory_bytes
            && !self.resident_order.is_empty()
        {
            self.spill_oldest()?;
        }
        if self.resident_bytes.saturating_add(len) > self.limits.memory_bytes {
            self.write_spilled(chunk_id, &plaintext)?;
            return Ok(());
        }
        self.resident_bytes = self
            .resident_bytes
            .checked_add(len)
            .ok_or_else(|| resource("resident byte total exceeds u64"))?;
        self.peak_resident_bytes = self.peak_resident_bytes.max(self.resident_bytes);
        self.resident.insert(chunk_id, plaintext);
        self.resident_order.push_back(chunk_id);
        Ok(())
    }

    /// Returns a staged plaintext, reading from temporary storage when spilled.
    pub(crate) fn read(&mut self, chunk_id: &Digest) -> Result<Vec<u8>> {
        if let Some(plaintext) = self.resident.get(chunk_id) {
            return Ok(plaintext.clone());
        }
        let location = *self.spilled.get(chunk_id).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::UnknownChunk,
                format!("Chunk {chunk_id} is no longer staged"),
            )
        })?;
        let spill = self
            .spill
            .as_mut()
            .ok_or_else(|| staging_io("staged Chunk has no backing file"))?;
        let mut bytes = vec![
            0_u8;
            usize::try_from(location.len)
                .map_err(|_| resource("staged Chunk exceeds usize"))?
        ];
        spill
            .file
            .seek(SeekFrom::Start(location.offset))
            .map_err(|error| staging_io(format!("seek staged Chunk: {error}")))?;
        spill
            .file
            .read_exact(&mut bytes)
            .map_err(|error| staging_io(format!("read staged Chunk: {error}")))?;
        Ok(bytes)
    }

    /// Drops one Chunk from the store. Spilled bytes become unreferenced and
    /// are reclaimed when the temporary file is removed.
    pub(crate) fn release(&mut self, chunk_id: &Digest) {
        if let Some(plaintext) = self.resident.remove(chunk_id) {
            self.resident_bytes = self
                .resident_bytes
                .saturating_sub(u64::try_from(plaintext.len()).unwrap_or(u64::MAX));
            if let Some(position) = self
                .resident_order
                .iter()
                .position(|value| value == chunk_id)
            {
                self.resident_order.remove(position);
            }
        }
        self.spilled.remove(chunk_id);
    }

    fn spill_oldest(&mut self) -> Result<()> {
        let Some(chunk_id) = self.resident_order.pop_front() else {
            return Ok(());
        };
        let Some(plaintext) = self.resident.remove(&chunk_id) else {
            return Ok(());
        };
        self.resident_bytes = self
            .resident_bytes
            .saturating_sub(u64::try_from(plaintext.len()).unwrap_or(u64::MAX));
        self.write_spilled(chunk_id, &plaintext)
    }

    fn write_spilled(&mut self, chunk_id: Digest, plaintext: &[u8]) -> Result<()> {
        let len = u64::try_from(plaintext.len()).map_err(|_| resource("Chunk exceeds u64"))?;
        let offset = self.spill_len;
        let spill = match self.spill.as_mut() {
            Some(spill) => spill,
            None => {
                self.spill = Some(SpillFile::create()?);
                self.spill
                    .as_mut()
                    .ok_or_else(|| staging_io("temporary staging file is unavailable"))?
            }
        };
        spill
            .file
            .seek(SeekFrom::Start(offset))
            .map_err(|error| staging_io(format!("position staging file: {error}")))?;
        spill
            .file
            .write_all(plaintext)
            .map_err(|error| staging_io(format!("write staging file: {error}")))?;
        self.spill_len = offset
            .checked_add(len)
            .ok_or_else(|| resource("staging file offset exceeds u64"))?;
        self.spilled_bytes = self
            .spilled_bytes
            .checked_add(len)
            .ok_or_else(|| resource("spilled byte total exceeds u64"))?;
        self.spilled.insert(chunk_id, SpillLocation { offset, len });
        Ok(())
    }
}

impl SpillFile {
    fn create() -> Result<Self> {
        let nanoseconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.subsec_nanos())
            .unwrap_or_default();
        let base = std::env::temp_dir();
        for attempt in 0..64_u64 {
            let path = base.join(format!(
                "entrybound-stage-{}-{}-{}-{}.tmp",
                std::process::id(),
                NEXT_STAGING_ID.fetch_add(1, Ordering::Relaxed),
                nanoseconds,
                attempt
            ));
            match OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(file) => return Ok(Self { file, path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => {
                    return Err(staging_io(format!("create staging file: {error}")));
                }
            }
        }
        Err(staging_io("cannot create a unique staging file"))
    }
}

fn staging_io(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::PolicyRefused, ReasonCode::Io, detail)
}

fn resource(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ResourceLimit,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::{ChunkStaging, StagingLimits};
    use crate::identity::sha256_exact;

    #[test]
    fn staging_spills_beyond_its_memory_limit_and_still_reads_back() {
        let mut staging = ChunkStaging::new(StagingLimits {
            memory_bytes: 8,
            total_bytes: 1024,
        });
        let payloads: Vec<Vec<u8>> = (0..6_u8).map(|value| vec![value; 6]).collect();
        for payload in &payloads {
            staging
                .insert(sha256_exact(payload), payload.clone())
                .unwrap();
        }
        assert!(staging.spilled_bytes() > 0);
        assert!(staging.peak_resident_bytes() <= 8);
        for payload in &payloads {
            assert_eq!(staging.read(&sha256_exact(payload)).unwrap(), *payload);
        }
    }

    #[test]
    fn staging_refuses_to_exceed_its_total_limit() {
        let mut staging = ChunkStaging::new(StagingLimits::memory_only(4));
        staging
            .insert(sha256_exact(b"abcd"), b"abcd".to_vec())
            .unwrap();
        let error = staging
            .insert(sha256_exact(b"efgh"), b"efgh".to_vec())
            .unwrap_err();
        assert_eq!(error.code(), crate::diagnostics::ReasonCode::ResourceLimit);
    }

    #[test]
    fn released_chunks_are_no_longer_retained() {
        let mut staging = ChunkStaging::new(StagingLimits {
            memory_bytes: 1024,
            total_bytes: 1024,
        });
        let chunk_id = sha256_exact(b"payload");
        staging.insert(chunk_id, b"payload".to_vec()).unwrap();
        assert_eq!(staging.len(), 1);
        staging.release(&chunk_id);
        assert!(!staging.contains(&chunk_id));
        assert_eq!(staging.len(), 0);
    }

    #[test]
    fn dropping_the_store_removes_its_temporary_file() {
        let path;
        {
            let mut staging = ChunkStaging::new(StagingLimits {
                memory_bytes: 1,
                total_bytes: 1024,
            });
            staging
                .insert(sha256_exact(b"spill me"), b"spill me".to_vec())
                .unwrap();
            path = staging
                .spill
                .as_ref()
                .map(|spill| spill.path.clone())
                .unwrap();
            assert!(path.exists());
        }
        assert!(!path.exists());
    }
}
