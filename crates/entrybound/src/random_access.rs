//! Bounded, revision-pinned random byte sources used by INDEXED access.

use std::collections::{BTreeMap, VecDeque};
use std::fs::{File, Metadata};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use reqwest::blocking::Client;
use reqwest::header::{
    ACCEPT_ENCODING, ACCEPT_RANGES, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, ETAG,
    IF_MATCH, RANGE, TRANSFER_ENCODING,
};
use reqwest::{StatusCode, Url};

use crate::crypto::working::{WorkBudget, WorkBytes, WorkLease, WorkVec};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::identity::sha256_exact;

/// Stable identity observed for one random-read session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceRevision {
    Memory {
        length: u64,
        sha256: [u8; 32],
    },
    Local {
        length: u64,
        modified_nanos: Option<u128>,
    },
    Http {
        length: u64,
        strong_etag: String,
    },
}

/// A source capable of exact, position-independent reads.
pub trait RandomReadSource: Send + Sync {
    fn len(&self) -> Result<u64>;
    fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }
    fn read_exact_at(&self, offset: u64, len: u64) -> Result<Vec<u8>>;
    /// Fills caller-owned backing without allocating an intermediate range.
    /// Crypto sessions require this capability; old custom sources keep
    /// compiling but refuse until they implement a direct bounded fill.
    fn read_exact_at_into(&self, _offset: u64, _dst: &mut [u8]) -> Result<()> {
        Err(policy(
            "source does not implement caller-buffer bounded reads required by crypto policy",
        ))
    }
    fn revision(&self) -> Result<SourceRevision>;
}

impl<T: RandomReadSource + ?Sized> RandomReadSource for Box<T> {
    fn len(&self) -> Result<u64> {
        (**self).len()
    }

    fn read_exact_at(&self, offset: u64, len: u64) -> Result<Vec<u8>> {
        (**self).read_exact_at(offset, len)
    }

    fn read_exact_at_into(&self, offset: u64, dst: &mut [u8]) -> Result<()> {
        (**self).read_exact_at_into(offset, dst)
    }

    fn revision(&self) -> Result<SourceRevision> {
        (**self).revision()
    }
}

/// Immutable in-memory random source.
#[derive(Clone)]
pub struct MemoryRandomReadSource {
    bytes: Arc<[u8]>,
    revision: SourceRevision,
}

impl MemoryRandomReadSource {
    #[must_use]
    pub fn new(bytes: impl Into<Arc<[u8]>>) -> Self {
        let bytes = bytes.into();
        let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let sha256 = *sha256_exact(&bytes).as_bytes();
        Self {
            bytes,
            revision: SourceRevision::Memory { length, sha256 },
        }
    }
}

impl RandomReadSource for MemoryRandomReadSource {
    fn len(&self) -> Result<u64> {
        u64::try_from(self.bytes.len()).map_err(|_| policy("memory source length exceeds u64"))
    }

    fn read_exact_at(&self, offset: u64, len: u64) -> Result<Vec<u8>> {
        let end = offset
            .checked_add(len)
            .ok_or_else(|| policy("range extent overflows u64"))?;
        let start = usize::try_from(offset).map_err(|_| policy("range offset exceeds usize"))?;
        let end = usize::try_from(end).map_err(|_| policy("range end exceeds usize"))?;
        self.bytes.get(start..end).map_or_else(
            || {
                Err(Diagnostic::new(
                    OutcomeClass::Truncated,
                    ReasonCode::TruncatedStream,
                    "requested range extends beyond the memory source",
                ))
            },
            |bytes| Ok(bytes.to_vec()),
        )
    }

    fn revision(&self) -> Result<SourceRevision> {
        Ok(self.revision.clone())
    }

    fn read_exact_at_into(&self, offset: u64, dst: &mut [u8]) -> Result<()> {
        let start = usize::try_from(offset).map_err(|_| policy("range offset exceeds usize"))?;
        let end = start
            .checked_add(dst.len())
            .ok_or_else(|| policy("range end exceeds usize"))?;
        let source = self.bytes.get(start..end).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Truncated,
                ReasonCode::TruncatedStream,
                "requested range extends beyond the memory source",
            )
        })?;
        dst.copy_from_slice(source);
        Ok(())
    }
}

/// Random reads from one held local-file handle.
pub struct LocalFileRandomReadSource {
    file: Mutex<File>,
    initial: SourceRevision,
}

impl LocalFileRandomReadSource {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file =
            File::open(path.as_ref()).map_err(|error| io("open local range source", error))?;
        let metadata = file
            .metadata()
            .map_err(|error| io("inspect local range source", error))?;
        Ok(Self {
            file: Mutex::new(file),
            initial: local_revision(&metadata)?,
        })
    }
}

impl RandomReadSource for LocalFileRandomReadSource {
    fn len(&self) -> Result<u64> {
        match &self.initial {
            SourceRevision::Local { length, .. } => Ok(*length),
            _ => unreachable!("local source always has a local revision"),
        }
    }

    fn read_exact_at(&self, offset: u64, len: u64) -> Result<Vec<u8>> {
        let end = offset
            .checked_add(len)
            .ok_or_else(|| policy("range extent overflows u64"))?;
        if end > self.len()? {
            return Err(Diagnostic::new(
                OutcomeClass::Truncated,
                ReasonCode::TruncatedStream,
                "requested range extends beyond the local source",
            ));
        }
        let length = usize::try_from(len).map_err(|_| policy("range length exceeds usize"))?;
        let mut output = vec![0_u8; length];
        let mut file = self
            .file
            .lock()
            .map_err(|_| unstable("local range-source lock was poisoned"))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| io("seek local range source", error))?;
        file.read_exact(&mut output)
            .map_err(|error| io("read local range source", error))?;
        Ok(output)
    }

    fn revision(&self) -> Result<SourceRevision> {
        let file = self
            .file
            .lock()
            .map_err(|_| unstable("local range-source lock was poisoned"))?;
        let metadata = file
            .metadata()
            .map_err(|error| io("reinspect local range source", error))?;
        local_revision(&metadata)
    }

    fn read_exact_at_into(&self, offset: u64, dst: &mut [u8]) -> Result<()> {
        let length = u64::try_from(dst.len()).map_err(|_| policy("range length exceeds u64"))?;
        let end = offset
            .checked_add(length)
            .ok_or_else(|| policy("range extent overflows u64"))?;
        if end > self.len()? {
            return Err(Diagnostic::new(
                OutcomeClass::Truncated,
                ReasonCode::TruncatedStream,
                "requested range extends beyond the local source",
            ));
        }
        let mut file = self
            .file
            .lock()
            .map_err(|_| unstable("local range-source lock was poisoned"))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| io("seek local range source", error))?;
        file.read_exact(dst)
            .map_err(|error| io("read local range source", error))
    }
}

/// HTTPS/HTTP source pinned to one strong ETag and exact Content-Length.
pub struct HttpRangeSource {
    client: Client,
    url: Url,
    revision: SourceRevision,
}

impl HttpRangeSource {
    pub fn open(url: &str) -> Result<Self> {
        let url = Url::parse(url).map_err(|error| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::HttpRangeInvalid,
                format!("invalid HTTP range URL: {error}"),
            )
        })?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::HttpRangeUnsupported,
                "random HTTP access supports only http and https URLs",
            ));
        }
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(3))
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .build()
            .map_err(|error| http("build HTTP client", error))?;
        let response = client
            .head(url.clone())
            .header(ACCEPT_ENCODING, "identity")
            .send()
            .map_err(|error| http("initialize HTTP range source", error))?;
        if !response.status().is_success() {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::HttpRangeUnsupported,
                format!("HTTP metadata request returned {}", response.status()),
            ));
        }
        reject_content_encoding(response.headers())?;
        let length = parse_content_length(response.headers())?;
        let etag = parse_strong_etag(response.headers())?;
        let ranges = response
            .headers()
            .get(ACCEPT_RANGES)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if !ranges.split(',').any(|value| value.trim() == "bytes") {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::HttpRangeUnsupported,
                "HTTP source did not advertise byte-range support",
            ));
        }
        Ok(Self {
            client,
            url,
            revision: SourceRevision::Http {
                length,
                strong_etag: etag,
            },
        })
    }

    fn http_revision(&self) -> Result<SourceRevision> {
        let response = self
            .client
            .head(self.url.clone())
            .header(ACCEPT_ENCODING, "identity")
            .send()
            .map_err(|error| http("revalidate HTTP source", error))?;
        if !response.status().is_success() {
            return Err(unstable(format!(
                "HTTP source revalidation returned {}",
                response.status()
            )));
        }
        reject_content_encoding(response.headers())?;
        Ok(SourceRevision::Http {
            length: parse_content_length(response.headers())?,
            strong_etag: parse_strong_etag(response.headers())?,
        })
    }
}

impl HttpRangeSource {
    fn range_response(&self, offset: u64, len: u64) -> Result<reqwest::blocking::Response> {
        let end_exclusive = offset
            .checked_add(len)
            .ok_or_else(|| policy("HTTP range extent overflows u64"))?;
        if end_exclusive > self.len()? {
            return Err(Diagnostic::new(
                OutcomeClass::Truncated,
                ReasonCode::TruncatedStream,
                "requested HTTP range extends beyond Content-Length",
            ));
        }
        let strong_etag = match &self.revision {
            SourceRevision::Http { strong_etag, .. } => strong_etag,
            _ => unreachable!("HTTP source always has an HTTP revision"),
        };
        let end = end_exclusive - 1;
        let response = self
            .client
            .get(self.url.clone())
            .header(ACCEPT_ENCODING, "identity")
            .header(IF_MATCH, strong_etag)
            .header(RANGE, format!("bytes={offset}-{end}"))
            .send()
            .map_err(|error| http("fetch HTTP range", error))?;
        if response.status() == StatusCode::PRECONDITION_FAILED {
            return Err(unstable("HTTP source ETag changed during random access"));
        }
        if response.status() != StatusCode::PARTIAL_CONTENT {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::HttpRangeUnsupported,
                format!(
                    "HTTP source ignored or rejected Range (status {})",
                    response.status()
                ),
            ));
        }
        reject_content_encoding(response.headers())?;
        let response_etag = parse_strong_etag(response.headers())?;
        if &response_etag != strong_etag {
            return Err(unstable("HTTP response ETag changed during random access"));
        }
        let content_length = parse_content_length(response.headers())?;
        if content_length != len {
            return Err(invalid_http(format!(
                "range Content-Length {content_length} does not match requested {len}"
            )));
        }
        let expected_range = format!("bytes {offset}-{end}/{}", self.len()?);
        let actual_range = response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| invalid_http("206 response lacks Content-Range"))?;
        if actual_range != expected_range {
            return Err(invalid_http(format!(
                "Content-Range {actual_range:?} does not equal {expected_range:?}"
            )));
        }
        Ok(response)
    }
}

impl RandomReadSource for HttpRangeSource {
    fn len(&self) -> Result<u64> {
        match &self.revision {
            SourceRevision::Http { length, .. } => Ok(*length),
            _ => unreachable!("HTTP source always has an HTTP revision"),
        }
    }

    fn read_exact_at(&self, offset: u64, len: u64) -> Result<Vec<u8>> {
        if len == 0 {
            return Ok(Vec::new());
        }
        let mut response = self.range_response(offset, len)?;
        let length = usize::try_from(len).map_err(|_| policy("HTTP range exceeds usize"))?;
        let mut output = vec![0_u8; length];
        read_http_body_into(&mut response, &mut output)?;
        Ok(output)
    }

    fn read_exact_at_into(&self, offset: u64, dst: &mut [u8]) -> Result<()> {
        let len = u64::try_from(dst.len()).map_err(|_| policy("HTTP range exceeds u64"))?;
        if len == 0 {
            return Ok(());
        }
        let mut response = self.range_response(offset, len)?;
        read_http_body_into(&mut response, dst)
    }

    fn revision(&self) -> Result<SourceRevision> {
        self.http_revision()
    }
}

// Exact body fill never grows destination storage; the overrun probe is fixed.
fn read_http_body_into(reader: &mut impl Read, dst: &mut [u8]) -> Result<()> {
    reader
        .read_exact(dst)
        .map_err(|error| invalid_http(format!("cannot read exact HTTP range body: {error}")))?;
    let mut probe = [0_u8; 1];
    loop {
        match reader.read(&mut probe) {
            Ok(0) => return Ok(()),
            Ok(_) => {
                return Err(Diagnostic::new(
                    OutcomeClass::Truncated,
                    ReasonCode::TruncatedStream,
                    "HTTP range body exceeds the exact requested length",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => {
                return Err(invalid_http(format!(
                    "cannot finish exact HTTP range body: {error}"
                )));
            }
        }
    }
}

/// Why a byte range was fetched.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessPurpose {
    Footer,
    Preamble,
    SectionHeader,
    Descriptor,
    Manifest,
    Index,
    ChunkHeader,
    Chunk,
    Dictionary,
    Lookback,
    Reconstruction,
    EncryptedControl,
    EncryptedPayload,
}

/// One bounded diagnostic access event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccessTraceEntry {
    pub offset: u64,
    pub length: u64,
    pub purpose: AccessPurpose,
    pub cache_hit: bool,
}

/// Caller-owned limits for a random-access session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomAccessPolicy {
    pub max_individual_range_bytes: u64,
    pub max_total_bytes_fetched: u64,
    pub max_range_requests: u64,
    pub max_metadata_bytes: u64,
    pub max_section_count: u64,
    pub max_chunk_frames_scanned: u64,
    pub max_dependency_chunks: u64,
    pub max_decoded_logical_bytes: u64,
    pub max_cached_bytes: u64,
    pub max_encrypted_segment_header_walk: u64,
    pub max_trace_entries: u64,
    pub coalesce_gap_bytes: u64,
    pub resource_policy: crate::eam::ResourceBudget,
    pub decode_policy: crate::eam::DecodeRequirements,
}

impl Default for RandomAccessPolicy {
    fn default() -> Self {
        Self {
            max_individual_range_bytes: 64 * 1024 * 1024,
            max_total_bytes_fetched: 512 * 1024 * 1024,
            max_range_requests: 100_000,
            max_metadata_bytes: 128 * 1024 * 1024,
            max_section_count: 64,
            max_chunk_frames_scanned: 4_000_000,
            max_dependency_chunks: 65_536,
            max_decoded_logical_bytes: 8 * 1024 * 1024 * 1024,
            max_cached_bytes: 128 * 1024 * 1024,
            max_encrypted_segment_header_walk: 4_000_000,
            max_trace_entries: 100_000,
            coalesce_gap_bytes: 4096,
            resource_policy: crate::archive::bootstrap_resource_policy(),
            decode_policy: crate::archive::bootstrap_decode_policy(),
        }
    }
}

#[derive(Clone, Debug)]
struct CacheEntry {
    bytes: Vec<u8>,
}

struct CryptoRangeState {
    budget: WorkBudget,
    trace: WorkVec<AccessTraceEntry>,
    _revision_lease: WorkLease,
    cache: [Option<CryptoCacheEntry>; 16],
    cache_next: usize,
    cache_bytes: u64,
}

struct CryptoCacheEntry {
    offset: u64,
    bytes: WorkBytes,
}

impl CryptoRangeState {
    fn cached_range(&self, offset: u64, end: u64) -> Option<usize> {
        self.cache.iter().position(|slot| {
            slot.as_ref().is_some_and(|entry| {
                entry.offset <= offset
                    && entry
                        .offset
                        .checked_add(entry.bytes.len() as u64)
                        .is_some_and(|cached_end| end <= cached_end)
            })
        })
    }

    fn evict(&mut self, slot: usize) {
        if let Some(entry) = self.cache[slot].take() {
            self.cache_bytes = self
                .cache_bytes
                .checked_sub(entry.bytes.len() as u64)
                .expect("owned cache capacity is in the cache ledger");
            drop(entry); // WorkBytes wipes/frees before releasing its lease.
        }
    }

    fn evict_oldest(&mut self, keep: Option<usize>) -> bool {
        for step in 0..self.cache.len() {
            let slot = (self.cache_next + step) % self.cache.len();
            if Some(slot) != keep && self.cache[slot].is_some() {
                self.evict(slot);
                return true;
            }
        }
        false
    }

    fn make_room(&mut self, required: u64, keep: Option<usize>) -> bool {
        while self.budget.available_bytes() < required && self.evict_oldest(keep) {}
        self.budget.available_bytes() >= required
    }

    fn retain(&mut self, offset: u64, bytes: &[u8], max_cached: u64, revision_headroom: u64) {
        let length = bytes.len() as u64;
        if length == 0 || length > max_cached {
            return;
        }
        while !self
            .cache_bytes
            .checked_add(length)
            .is_some_and(|sum| sum <= max_cached)
        {
            if !self.evict_oldest(None) {
                return;
            }
        }
        let slot = self.cache_next;
        self.evict(slot);
        let Some(required) = length.checked_add(revision_headroom) else {
            return;
        };
        if !self.make_room(required, None) {
            return;
        }
        // Optional cache admission must not fail an otherwise successful read.
        let Ok(mut backing) = WorkBytes::zeroed(&self.budget, bytes.len()) else {
            return;
        };
        backing.as_mut_slice().copy_from_slice(bytes);
        self.cache[slot] = Some(CryptoCacheEntry {
            offset,
            bytes: backing,
        });
        self.cache_bytes = self
            .cache_bytes
            .checked_add(length)
            .expect("cache sum checked before admission");
        self.cache_next = (slot + 1) % self.cache.len();
    }
}

/// One revision-pinned, policy-accounted read session.
pub(crate) struct RangeSession {
    source: Box<dyn RandomReadSource>,
    initial_revision: SourceRevision,
    policy: RandomAccessPolicy,
    cache: BTreeMap<u64, CacheEntry>,
    cache_order: VecDeque<u64>,
    cache_bytes: u64,
    bytes_fetched: u64,
    range_requests: u64,
    metadata_bytes: u64,
    trace: Vec<AccessTraceEntry>,
    crypto: Option<CryptoRangeState>,
}

impl RangeSession {
    pub(crate) fn new(
        source: Box<dyn RandomReadSource>,
        policy: RandomAccessPolicy,
    ) -> Result<Self> {
        let initial_revision = source.revision()?;
        if source.len()? != revision_len(&initial_revision) {
            return Err(unstable("source length and source revision disagree"));
        }
        Ok(Self {
            source,
            initial_revision,
            policy,
            cache: BTreeMap::new(),
            cache_order: VecDeque::new(),
            cache_bytes: 0,
            bytes_fetched: 0,
            range_requests: 0,
            metadata_bytes: 0,
            trace: Vec::new(),
            crypto: None,
        })
    }

    /// Uses caller-filled, lease-carrying buffers and a separately charged cache.
    /// The budget is shared with every nested crypto operation by the caller.
    pub(crate) fn new_crypto(
        source: Box<dyn RandomReadSource>,
        policy: RandomAccessPolicy,
        budget: WorkBudget,
    ) -> Result<Self> {
        let mut session = Self::new(source, policy)?;
        // The callback owns revision construction; charge its retained backing
        // before adopting it into the returned crypto session.
        let revision_lease = budget.reserve(revision_capacity(&session.initial_revision)?)?;
        session.crypto = Some(CryptoRangeState {
            trace: WorkVec::new(&budget),
            budget,
            _revision_lease: revision_lease,
            cache: std::array::from_fn(|_| None),
            cache_next: 0,
            cache_bytes: 0,
        });
        Ok(session)
    }

    pub(crate) fn len(&self) -> u64 {
        revision_len(&self.initial_revision)
    }

    pub(crate) fn policy(&self) -> &RandomAccessPolicy {
        &self.policy
    }

    pub(crate) fn read(
        &mut self,
        offset: u64,
        len: u64,
        purpose: AccessPurpose,
    ) -> Result<Vec<u8>> {
        self.read_internal(offset, len, purpose, true)
    }

    /// Fetches one full-verification range without retaining a second copy in
    /// the random-access cache. Existing cached metadata is released first.
    /// Transfer, request, trace, and revision limits remain in force.
    pub(crate) fn read_uncached(
        &mut self,
        offset: u64,
        len: u64,
        purpose: AccessPurpose,
    ) -> Result<Vec<u8>> {
        self.require_ordinary()?;
        self.check_stable()?;
        self.cache.clear();
        self.cache_order.clear();
        self.cache_bytes = 0;
        let bytes = self.read_internal(offset, len, purpose, false)?;
        self.check_stable()?;
        Ok(bytes)
    }

    fn read_internal(
        &mut self,
        offset: u64,
        len: u64,
        purpose: AccessPurpose,
        use_cache: bool,
    ) -> Result<Vec<u8>> {
        self.require_ordinary()?;
        let end = offset
            .checked_add(len)
            .ok_or_else(|| policy("range extent overflows u64"))?;
        if end > self.len() {
            return Err(Diagnostic::new(
                OutcomeClass::Truncated,
                ReasonCode::TruncatedStream,
                "requested range exceeds the declared source length",
            ));
        }
        if u64::try_from(self.trace.len()).unwrap_or(u64::MAX) >= self.policy.max_trace_entries {
            return Err(policy("access trace entry limit exceeded"));
        }
        if len == 0 {
            self.push_trace(offset, 0, purpose, use_cache)?;
            return Ok(Vec::new());
        }
        if len > self.policy.max_individual_range_bytes {
            return Err(policy(format!(
                "range length {len} exceeds caller limit {}",
                self.policy.max_individual_range_bytes
            )));
        }
        if use_cache && let Some(bytes) = self.cached_slice(offset, end) {
            self.push_trace(offset, len, purpose, true)?;
            return Ok(bytes);
        }
        let next_requests = self
            .range_requests
            .checked_add(1)
            .ok_or_else(|| policy("range request count overflow"))?;
        let next_bytes = self
            .bytes_fetched
            .checked_add(len)
            .ok_or_else(|| policy("fetched byte count overflow"))?;
        if next_requests > self.policy.max_range_requests
            || next_bytes > self.policy.max_total_bytes_fetched
        {
            return Err(policy("random-access transfer budget would be exceeded"));
        }
        if is_metadata(purpose) {
            let next_metadata = self
                .metadata_bytes
                .checked_add(len)
                .ok_or_else(|| policy("metadata byte count overflow"))?;
            if next_metadata > self.policy.max_metadata_bytes {
                return Err(policy("random-access metadata budget would be exceeded"));
            }
            self.metadata_bytes = next_metadata;
        }
        let bytes = self.source.read_exact_at(offset, len)?;
        if u64::try_from(bytes.len()).map_err(|_| policy("range result exceeds u64"))? != len {
            return Err(Diagnostic::new(
                OutcomeClass::Truncated,
                ReasonCode::TruncatedStream,
                "random source returned a short range",
            ));
        }
        self.range_requests = next_requests;
        self.bytes_fetched = next_bytes;
        self.push_trace(offset, len, purpose, false)?;
        if use_cache {
            self.insert_cache(offset, bytes.clone())?;
        }
        Ok(bytes)
    }

    /// Reserves the full destination and a trace slot before invoking a source.
    /// Returned backing remains charged until its owner wipes and frees it.
    pub(crate) fn read_crypto(
        &mut self,
        offset: u64,
        len: u64,
        purpose: AccessPurpose,
    ) -> Result<WorkBytes> {
        if self.crypto.is_none() {
            return Err(policy(
                "caller-buffer crypto read requires a shared working budget",
            ));
        }
        let end = offset
            .checked_add(len)
            .ok_or_else(|| policy("range extent overflows u64"))?;
        if end > self.len() {
            return Err(Diagnostic::new(
                OutcomeClass::Truncated,
                ReasonCode::TruncatedStream,
                "requested range exceeds the declared source length",
            ));
        }
        if u64::try_from(self.trace().len()).unwrap_or(u64::MAX) >= self.policy.max_trace_entries {
            return Err(policy("access trace entry limit exceeded"));
        }
        if len > self.policy.max_individual_range_bytes {
            return Err(policy("range length exceeds caller limit"));
        }
        let length = usize::try_from(len).map_err(|_| policy("range length exceeds usize"))?;
        let revision_headroom = revision_capacity(&self.initial_revision)?;
        let headroom = len
            .checked_add(revision_headroom)
            .ok_or_else(|| policy("crypto read headroom overflows"))?;
        let state = self.crypto.as_mut().expect("crypto mode checked above");
        let mut hit = if len == 0 {
            None
        } else {
            state.cached_range(offset, end)
        };
        let required = state
            .trace
            .minimum_reservation_bytes(1)?
            .checked_add(headroom)
            .ok_or_else(|| policy("crypto trace and destination capacity overflows"))?;
        if !state.make_room(required, hit)
            && let Some(slot) = hit.take()
        {
            // If cache-plus-copy overlap cannot fit, free the cached backing
            // before allocating a direct source destination instead.
            state.evict(slot);
            state.make_room(required, None);
        }
        self.check_stable()?;
        let next_requests = self
            .range_requests
            .checked_add(u64::from(len != 0 && hit.is_none()))
            .ok_or_else(|| policy("range request count overflow"))?;
        let next_bytes = if hit.is_some() {
            self.bytes_fetched
        } else {
            self.bytes_fetched
                .checked_add(len)
                .ok_or_else(|| policy("fetched byte count overflow"))?
        };
        let next_metadata = if hit.is_none() && is_metadata(purpose) {
            self.metadata_bytes
                .checked_add(len)
                .ok_or_else(|| policy("metadata byte count overflow"))?
        } else {
            self.metadata_bytes
        };
        if next_requests > self.policy.max_range_requests
            || next_bytes > self.policy.max_total_bytes_fetched
        {
            return Err(policy("random-access transfer budget would be exceeded"));
        }
        if next_metadata > self.policy.max_metadata_bytes {
            return Err(policy("random-access metadata budget would be exceeded"));
        }
        let state = self.crypto.as_mut().expect("crypto mode checked above");
        state.trace.reserve_with_headroom(1, headroom)?;
        let mut bytes = WorkBytes::zeroed(&state.budget, length)?;
        if let Some(slot) = hit {
            let entry = state.cache[slot]
                .as_ref()
                .expect("hit slot was preserved while making room");
            let start = usize::try_from(offset - entry.offset)
                .map_err(|_| policy("cached range offset exceeds usize"))?;
            let cached_end = start
                .checked_add(length)
                .ok_or_else(|| policy("cached range end exceeds usize"))?;
            let cached = entry
                .bytes
                .as_slice()
                .get(start..cached_end)
                .ok_or_else(|| policy("cached range does not contain the requested extent"))?;
            bytes.as_mut_slice().copy_from_slice(cached);
        } else if len != 0 {
            self.source
                .read_exact_at_into(offset, bytes.as_mut_slice())?;
        }
        self.check_stable()?;
        self.range_requests = next_requests;
        self.bytes_fetched = next_bytes;
        self.metadata_bytes = next_metadata;
        self.push_trace(offset, len, purpose, hit.is_some())?;
        if hit.is_none() {
            self.crypto
                .as_mut()
                .expect("crypto mode checked above")
                .retain(
                    offset,
                    bytes.as_slice(),
                    self.policy.max_cached_bytes,
                    revision_headroom,
                );
        }
        Ok(bytes)
    }

    pub(crate) fn prefetch(&mut self, ranges: &[(u64, u64, AccessPurpose)]) -> Result<()> {
        self.require_ordinary()?;
        let mut ranges = ranges
            .iter()
            .copied()
            .filter(|(_, length, _)| *length != 0)
            .collect::<Vec<_>>();
        ranges.sort_by_key(|range| range.0);
        let mut cursor = 0;
        while cursor < ranges.len() {
            let (start, len, purpose) = ranges[cursor];
            let mut end = start
                .checked_add(len)
                .ok_or_else(|| policy("prefetch range overflow"))?;
            let mut next = cursor + 1;
            while next < ranges.len() {
                let (candidate, candidate_len, candidate_purpose) = ranges[next];
                if candidate_purpose != purpose {
                    break;
                }
                if candidate > end.saturating_add(self.policy.coalesce_gap_bytes) {
                    break;
                }
                let candidate_end = candidate
                    .checked_add(candidate_len)
                    .ok_or_else(|| policy("prefetch range overflow"))?;
                let merged_end = end.max(candidate_end);
                if merged_end - start > self.policy.max_individual_range_bytes {
                    break;
                }
                end = merged_end;
                next += 1;
            }
            let _ = self.read(start, end - start, purpose)?;
            cursor = next;
        }
        Ok(())
    }

    pub(crate) fn check_stable(&self) -> Result<()> {
        let final_revision = self.source.revision()?;
        let lease = self
            .crypto
            .as_ref()
            .map(|state| state.budget.reserve(revision_capacity(&final_revision)?))
            .transpose()?;
        let stable = final_revision == self.initial_revision;
        drop(final_revision);
        drop(lease);
        if !stable {
            return Err(unstable("source revision changed during random access"));
        }
        Ok(())
    }

    pub(crate) fn initial_revision(&self) -> &SourceRevision {
        &self.initial_revision
    }

    pub(crate) fn bytes_fetched(&self) -> u64 {
        self.bytes_fetched
    }

    pub(crate) fn range_requests(&self) -> u64 {
        self.range_requests
    }

    pub(crate) fn trace(&self) -> &[AccessTraceEntry] {
        self.crypto
            .as_ref()
            .map_or(&self.trace, |state| state.trace.as_slice())
    }

    fn require_ordinary(&self) -> Result<()> {
        if self.crypto.is_some() {
            Err(policy(
                "crypto sessions require lease-carrying caller-buffer reads",
            ))
        } else {
            Ok(())
        }
    }

    fn cached_slice(&self, offset: u64, end: u64) -> Option<Vec<u8>> {
        let (&start, entry) = self.cache.range(..=offset).next_back()?;
        let entry_end = start.checked_add(u64::try_from(entry.bytes.len()).ok()?)?;
        if end > entry_end {
            return None;
        }
        let relative = usize::try_from(offset - start).ok()?;
        let length = usize::try_from(end - offset).ok()?;
        Some(entry.bytes[relative..relative + length].to_vec())
    }

    fn insert_cache(&mut self, offset: u64, bytes: Vec<u8>) -> Result<()> {
        let length = u64::try_from(bytes.len()).map_err(|_| policy("cache item exceeds u64"))?;
        if length > self.policy.max_cached_bytes {
            return Ok(());
        }
        while self.cache_bytes.saturating_add(length) > self.policy.max_cached_bytes {
            let Some(oldest) = self.cache_order.pop_front() else {
                break;
            };
            if let Some(removed) = self.cache.remove(&oldest) {
                self.cache_bytes = self
                    .cache_bytes
                    .saturating_sub(u64::try_from(removed.bytes.len()).unwrap_or(u64::MAX));
            }
        }
        if let Some(replaced) = self.cache.insert(offset, CacheEntry { bytes }) {
            self.cache_bytes = self
                .cache_bytes
                .saturating_sub(u64::try_from(replaced.bytes.len()).unwrap_or(u64::MAX));
            self.cache_order.retain(|value| *value != offset);
        }
        self.cache_order.push_back(offset);
        self.cache_bytes = self
            .cache_bytes
            .checked_add(length)
            .ok_or_else(|| policy("cache accounting overflow"))?;
        Ok(())
    }

    fn push_trace(
        &mut self,
        offset: u64,
        length: u64,
        purpose: AccessPurpose,
        cache_hit: bool,
    ) -> Result<()> {
        if u64::try_from(self.trace().len()).unwrap_or(u64::MAX) >= self.policy.max_trace_entries {
            return Err(policy("access trace entry limit exceeded"));
        }
        let entry = AccessTraceEntry {
            offset,
            length,
            purpose,
            cache_hit,
        };
        if let Some(state) = &mut self.crypto {
            state.trace.push(entry)?;
        } else {
            self.trace.push(entry);
        }
        Ok(())
    }
}

fn revision_capacity(revision: &SourceRevision) -> Result<u64> {
    match revision {
        SourceRevision::Http { strong_etag, .. } => u64::try_from(strong_etag.capacity())
            .map_err(|_| policy("source revision capacity exceeds u64")),
        _ => Ok(0),
    }
}

fn local_revision(metadata: &Metadata) -> Result<SourceRevision> {
    let modified_nanos = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos());
    Ok(SourceRevision::Local {
        length: metadata.len(),
        modified_nanos,
    })
}

const fn revision_len(revision: &SourceRevision) -> u64 {
    match revision {
        SourceRevision::Memory { length, .. }
        | SourceRevision::Local { length, .. }
        | SourceRevision::Http { length, .. } => *length,
    }
}

fn parse_content_length(headers: &reqwest::header::HeaderMap) -> Result<u64> {
    let mut values = headers.get_all(CONTENT_LENGTH).iter();
    let value = values
        .next()
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| invalid_http("HTTP source lacks one exact Content-Length"))?;
    if values.next().is_some() || value.contains(',') {
        return Err(invalid_http(
            "HTTP source contains multiple Content-Length values",
        ));
    }
    value
        .parse()
        .map_err(|_| invalid_http("HTTP source has an invalid Content-Length"))
}

fn parse_strong_etag(headers: &reqwest::header::HeaderMap) -> Result<String> {
    let mut values = headers.get_all(ETAG).iter();
    let etag = values
        .next()
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| invalid_http("HTTP source lacks a valid ETag"))?;
    let bytes = etag.as_bytes();
    let valid_opaque_tag = bytes.len() >= 2
        && bytes[0] == b'"'
        && bytes[bytes.len() - 1] == b'"'
        && bytes[1..bytes.len() - 1]
            .iter()
            .all(|byte| *byte == 0x21 || (0x23..=0x7e).contains(byte));
    if values.next().is_some() || etag.starts_with("W/") || !valid_opaque_tag {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::HttpRangeUnsupported,
            "HTTP random access requires a strong quoted ETag",
        ));
    }
    Ok(etag.to_owned())
}

fn reject_content_encoding(headers: &reqwest::header::HeaderMap) -> Result<()> {
    if let Some(value) = headers.get(CONTENT_ENCODING) {
        let value = value.to_str().unwrap_or_default();
        if !value.is_empty() && value != "identity" {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::HttpRangeUnsupported,
                "HTTP random access requires identity content encoding",
            ));
        }
    }
    if let Some(value) = headers.get(TRANSFER_ENCODING) {
        let value = value.to_str().unwrap_or_default();
        if !value.is_empty() && value != "identity" {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::HttpRangeUnsupported,
                "HTTP random access requires identity transfer encoding",
            ));
        }
    }
    Ok(())
}

const fn is_metadata(purpose: AccessPurpose) -> bool {
    matches!(
        purpose,
        AccessPurpose::Footer
            | AccessPurpose::Preamble
            | AccessPurpose::SectionHeader
            | AccessPurpose::Descriptor
            | AccessPurpose::Manifest
            | AccessPurpose::Index
            | AccessPurpose::ChunkHeader
            | AccessPurpose::EncryptedControl
    )
}

fn policy(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::RandomAccessPolicyRefused,
        detail,
    )
}

fn unstable(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, ReasonCode::SourceUnstable, detail)
}

fn invalid_http(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, ReasonCode::HttpRangeInvalid, detail)
}

fn http(context: &str, error: reqwest::Error) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::HttpRangeInvalid,
        format!("{context}: {error}"),
    )
}

fn io(context: &str, error: std::io::Error) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::Io,
        format!("{context}: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use crate::crypto::working::WorkBudget;

    use super::{
        AccessPurpose, HttpRangeSource, MemoryRandomReadSource, RandomAccessPolicy,
        RandomReadSource, RangeSession, SourceRevision,
    };

    struct BoundedProbe {
        calls: Arc<AtomicUsize>,
        changes_revision: bool,
        fails_read: bool,
    }

    impl RandomReadSource for BoundedProbe {
        fn len(&self) -> crate::diagnostics::Result<u64> {
            Ok(64)
        }

        fn read_exact_at(&self, _: u64, _: u64) -> crate::diagnostics::Result<Vec<u8>> {
            panic!("crypto must never invoke the allocating source callback")
        }

        fn read_exact_at_into(
            &self,
            offset: u64,
            dst: &mut [u8],
        ) -> crate::diagnostics::Result<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fails_read {
                return Err(super::policy("synthetic bounded source failure"));
            }
            for (index, byte) in dst.iter_mut().enumerate() {
                *byte = (offset as usize + index) as u8;
            }
            Ok(())
        }

        fn revision(&self) -> crate::diagnostics::Result<SourceRevision> {
            Ok(SourceRevision::Memory {
                length: 64,
                sha256: [if self.changes_revision {
                    self.calls.load(Ordering::SeqCst) as u8
                } else {
                    0
                }; 32],
            })
        }
    }

    #[test]
    fn bounded_crypto_counts_simultaneous_buffers_and_retained_trace_without_cache() {
        let budget = WorkBudget::new(512);
        let calls = Arc::new(AtomicUsize::new(0));
        let source = BoundedProbe {
            calls: calls.clone(),
            changes_revision: false,
            fails_read: false,
        };
        let mut session = RangeSession::new_crypto(
            Box::new(source),
            RandomAccessPolicy {
                max_cached_bytes: 0,
                ..RandomAccessPolicy::default()
            },
            budget.clone(),
        )
        .unwrap();
        let first = session
            .read_crypto(0, 16, AccessPurpose::EncryptedPayload)
            .unwrap();
        let second = session
            .read_crypto(0, 16, AccessPurpose::EncryptedPayload)
            .unwrap();
        assert_eq!(first.as_slice(), second.as_slice());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(session.cache.is_empty());
        assert!(session.cache_order.is_empty());
        assert_eq!(session.cache_bytes, 0);
        assert_eq!(session.bytes_fetched(), 32);
        assert_eq!(session.range_requests(), 2);
        assert!(session.trace().iter().all(|entry| !entry.cache_hit));
        let trace_bytes = (2 * std::mem::size_of::<super::AccessTraceEntry>()) as u64;
        assert_eq!(budget.live_bytes(), trace_bytes + 32);
        assert!(session.read(0, 1, AccessPurpose::EncryptedPayload).is_err());
        assert!(
            session
                .prefetch(&[(0, 1, AccessPurpose::EncryptedPayload)])
                .is_err()
        );
        drop(first);
        assert_eq!(budget.live_bytes(), trace_bytes + 16);
        drop(second);
        assert_eq!(budget.live_bytes(), trace_bytes);
        drop(session);
        assert_eq!(budget.live_bytes(), 0);
    }

    #[test]
    fn bounded_crypto_refuses_destination_and_transfer_before_callback() {
        let trace_bytes = std::mem::size_of::<super::AccessTraceEntry>() as u64;
        for limits in [
            RandomAccessPolicy::default(),
            RandomAccessPolicy {
                max_total_bytes_fetched: 15,
                ..RandomAccessPolicy::default()
            },
        ] {
            let budget = WorkBudget::new(trace_bytes + 15);
            let calls = Arc::new(AtomicUsize::new(0));
            let source = BoundedProbe {
                calls: calls.clone(),
                changes_revision: false,
                fails_read: false,
            };
            let mut session =
                RangeSession::new_crypto(Box::new(source), limits, budget.clone()).unwrap();
            assert!(
                session
                    .read_crypto(0, 16, AccessPurpose::EncryptedPayload)
                    .is_err()
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert_eq!(session.range_requests(), 0);
            assert_eq!(session.bytes_fetched(), 0);
            drop(session);
            assert_eq!(budget.live_bytes(), 0);
        }
    }

    #[test]
    fn bounded_crypto_callback_error_and_revision_change_release_destination() {
        for (fails_read, changes_revision) in [(true, false), (false, true)] {
            let budget = WorkBudget::new(512);
            let calls = Arc::new(AtomicUsize::new(0));
            let source = BoundedProbe {
                calls: calls.clone(),
                changes_revision,
                fails_read,
            };
            let mut session = RangeSession::new_crypto(
                Box::new(source),
                RandomAccessPolicy::default(),
                budget.clone(),
            )
            .unwrap();
            let error = session
                .read_crypto(0, 16, AccessPurpose::EncryptedPayload)
                .unwrap_err();
            if changes_revision {
                assert_eq!(error.code(), crate::diagnostics::ReasonCode::SourceUnstable);
            }
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert!(session.trace().is_empty());
            assert_eq!(
                budget.live_bytes(),
                std::mem::size_of::<super::AccessTraceEntry>() as u64
            );
            drop(session);
            assert_eq!(budget.live_bytes(), 0);
        }
    }

    #[test]
    fn bounded_crypto_default_capability_refuses_without_allocating_adapter() {
        struct OldSource;
        impl RandomReadSource for OldSource {
            fn len(&self) -> crate::diagnostics::Result<u64> {
                Ok(1)
            }
            fn read_exact_at(&self, _: u64, _: u64) -> crate::diagnostics::Result<Vec<u8>> {
                panic!("default bounded method must not allocate through the old callback")
            }
            fn revision(&self) -> crate::diagnostics::Result<SourceRevision> {
                Ok(SourceRevision::Memory {
                    length: 1,
                    sha256: [0; 32],
                })
            }
        }
        let budget = WorkBudget::new(512);
        let mut session = RangeSession::new_crypto(
            Box::new(OldSource),
            RandomAccessPolicy::default(),
            budget.clone(),
        )
        .unwrap();
        let error = session
            .read_crypto(0, 1, AccessPurpose::EncryptedPayload)
            .unwrap_err();
        assert_eq!(
            error.class(),
            crate::diagnostics::OutcomeClass::PolicyRefused
        );
        assert_eq!(
            error.code(),
            crate::diagnostics::ReasonCode::RandomAccessPolicyRefused
        );
        drop(session);
        assert_eq!(budget.live_bytes(), 0);
    }

    #[test]
    fn bounded_boxed_memory_and_local_sources_fill_exact_destination() {
        let memory: Box<dyn RandomReadSource> =
            Box::new(MemoryRandomReadSource::new(Vec::from(&b"0123456789"[..])));
        let mut dst = [0; 4];
        memory.read_exact_at_into(2, &mut dst).unwrap();
        assert_eq!(&dst, b"2345");
        assert!(memory.read_exact_at_into(8, &mut dst).is_err());
        let mut suffix = [0; 8];
        getrandom::fill(&mut suffix).unwrap();
        let name = suffix
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let path = std::env::temp_dir().join(format!("entrybound-bounded-source-{name}"));
        let mut file = std::fs::File::create_new(&path).unwrap();
        file.write_all(b"0123456789").unwrap();
        drop(file);
        let local: Box<dyn RandomReadSource> =
            Box::new(super::LocalFileRandomReadSource::open(&path).unwrap());
        local.read_exact_at_into(2, &mut dst).unwrap();
        assert_eq!(&dst, b"2345");
        assert!(local.read_exact_at_into(8, &mut dst).is_err());
        drop(local);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn bounded_http_body_fill_rejects_short_and_overlong_without_growth() {
        let mut dst = [0; 4];
        super::read_http_body_into(&mut std::io::Cursor::new(b"2345"), &mut dst).unwrap();
        assert_eq!(&dst, b"2345");
        assert!(super::read_http_body_into(&mut std::io::Cursor::new(b"234"), &mut dst).is_err());
        assert!(super::read_http_body_into(&mut std::io::Cursor::new(b"23456"), &mut dst).is_err());
    }

    #[test]
    fn bounded_crypto_cache_hit_keeps_separate_copy_charged_and_transfer_counts_unchanged() {
        let budget = WorkBudget::new(512);
        let calls = Arc::new(AtomicUsize::new(0));
        let source = BoundedProbe {
            calls: calls.clone(),
            changes_revision: false,
            fails_read: false,
        };
        let limits = RandomAccessPolicy {
            max_cached_bytes: 16,
            max_range_requests: 1,
            max_total_bytes_fetched: 16,
            max_metadata_bytes: 16,
            ..RandomAccessPolicy::default()
        };
        let mut session =
            RangeSession::new_crypto(Box::new(source), limits, budget.clone()).unwrap();
        let first = session
            .read_crypto(0, 16, AccessPurpose::Descriptor)
            .unwrap();
        let item_bytes = std::mem::size_of::<super::AccessTraceEntry>() as u64;
        assert_eq!(budget.live_bytes(), item_bytes + 16 + 16);
        drop(first);
        let hit = session
            .read_crypto(4, 8, AccessPurpose::Descriptor)
            .unwrap();
        assert_eq!(hit.as_slice(), [4, 5, 6, 7, 8, 9, 10, 11]);
        assert_eq!(budget.live_bytes(), 2 * item_bytes + 16 + 8);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(session.range_requests(), 1);
        assert_eq!(session.bytes_fetched(), 16);
        assert_eq!(session.metadata_bytes, 16);
        assert!(session.trace()[1].cache_hit);
        assert_eq!(session.crypto.as_ref().unwrap().cache_bytes, 16);
        drop(hit);
        assert_eq!(budget.live_bytes(), 2 * item_bytes + 16);
        drop(session);
        assert_eq!(budget.live_bytes(), 0);
    }

    #[test]
    fn bounded_crypto_tight_hit_overlap_evicts_then_fetches_instead_of_unmetered_copy() {
        let item_bytes = std::mem::size_of::<super::AccessTraceEntry>() as u64;
        // A trace replacement plus destination fits after freeing the hit;
        // retaining that hit during the same replacement does not fit.
        let budget = WorkBudget::new(3 * item_bytes + 16);
        let calls = Arc::new(AtomicUsize::new(0));
        let source = BoundedProbe {
            calls: calls.clone(),
            changes_revision: false,
            fails_read: false,
        };
        let mut session = RangeSession::new_crypto(
            Box::new(source),
            RandomAccessPolicy::default(),
            budget.clone(),
        )
        .unwrap();
        drop(
            session
                .read_crypto(0, 16, AccessPurpose::EncryptedPayload)
                .unwrap(),
        );
        assert_eq!(session.crypto.as_ref().unwrap().cache_bytes, 16);
        let second = session
            .read_crypto(0, 16, AccessPurpose::EncryptedPayload)
            .unwrap();
        assert_eq!(second.as_slice(), (0..16).collect::<Vec<_>>());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(!session.trace()[1].cache_hit);
        assert_eq!(session.range_requests(), 2);
        assert_eq!(session.bytes_fetched(), 32);
        assert!(budget.live_bytes() <= 3 * item_bytes + 16);
        drop(second);
        drop(session);
        assert_eq!(budget.live_bytes(), 0);
    }

    #[test]
    fn bounded_crypto_cache_byte_limit_zero_and_eviction_are_enforced() {
        for max_cached_bytes in [0, 8] {
            let budget = WorkBudget::new(512);
            let calls = Arc::new(AtomicUsize::new(0));
            let source = BoundedProbe {
                calls: calls.clone(),
                changes_revision: false,
                fails_read: false,
            };
            let limits = RandomAccessPolicy {
                max_cached_bytes,
                ..RandomAccessPolicy::default()
            };
            let mut session =
                RangeSession::new_crypto(Box::new(source), limits, budget.clone()).unwrap();
            for offset in [0, 16, 0] {
                drop(
                    session
                        .read_crypto(offset, 8, AccessPurpose::EncryptedPayload)
                        .unwrap(),
                );
                assert!(session.crypto.as_ref().unwrap().cache_bytes <= max_cached_bytes);
            }
            assert_eq!(calls.load(Ordering::SeqCst), 3);
            assert!(session.trace().iter().all(|entry| !entry.cache_hit));
            drop(session);
            assert_eq!(budget.live_bytes(), 0);
        }
    }

    #[test]
    fn bounded_crypto_optional_cache_never_refuses_required_read_under_tight_budget() {
        let item_bytes = std::mem::size_of::<super::AccessTraceEntry>() as u64;
        let budget = WorkBudget::new(item_bytes + 16);
        let calls = Arc::new(AtomicUsize::new(0));
        let source = BoundedProbe {
            calls: calls.clone(),
            changes_revision: false,
            fails_read: false,
        };
        let mut session = RangeSession::new_crypto(
            Box::new(source),
            RandomAccessPolicy::default(),
            budget.clone(),
        )
        .unwrap();
        let bytes = session
            .read_crypto(0, 16, AccessPurpose::EncryptedPayload)
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(budget.live_bytes(), item_bytes + 16);
        assert_eq!(session.crypto.as_ref().unwrap().cache_bytes, 0);
        drop(bytes);
        drop(session);
        assert_eq!(budget.live_bytes(), 0);
    }

    fn test_server(
        responses: Vec<&'static str>,
    ) -> (String, Arc<Mutex<Vec<String>>>, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let thread = std::thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0_u8; 1024];
                loop {
                    let count = stream.read(&mut buffer).unwrap();
                    if count == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..count]);
                    if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                captured
                    .lock()
                    .unwrap()
                    .push(String::from_utf8(bytes).unwrap());
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
        });
        (format!("http://{address}/archive.eb"), requests, thread)
    }

    #[test]
    fn memory_ranges_are_exact_cached_and_revision_pinned() {
        let source = MemoryRandomReadSource::new(Vec::from(&b"0123456789"[..]));
        let mut session =
            RangeSession::new(Box::new(source), RandomAccessPolicy::default()).unwrap();
        assert_eq!(session.read(2, 4, AccessPurpose::Chunk).unwrap(), b"2345");
        assert_eq!(session.read(3, 2, AccessPurpose::Chunk).unwrap(), b"34");
        assert_eq!(session.bytes_fetched(), 4);
        assert_eq!(session.range_requests(), 1);
        assert!(session.trace()[1].cache_hit);
        assert!(matches!(
            session.initial_revision(),
            SourceRevision::Memory { length: 10, .. }
        ));
        session.check_stable().unwrap();
    }

    #[test]
    fn uncached_full_read_drops_cache_and_keeps_range_limits() {
        let source = MemoryRandomReadSource::new(Vec::from(&b"0123456789"[..]));
        let policy = RandomAccessPolicy {
            max_individual_range_bytes: 10,
            max_total_bytes_fetched: 12,
            ..RandomAccessPolicy::default()
        };
        let mut session = RangeSession::new(Box::new(source), policy).unwrap();
        assert_eq!(session.read(0, 2, AccessPurpose::Preamble).unwrap(), b"01");
        assert_eq!(session.cache_bytes, 2);
        assert_eq!(
            session
                .read_uncached(0, 10, AccessPurpose::EncryptedPayload)
                .unwrap(),
            b"0123456789"
        );
        assert_eq!(session.cache_bytes, 0);
        assert!(session.cache.is_empty());
        assert_eq!(session.range_requests(), 2);
        assert_eq!(session.bytes_fetched(), 12);
        assert!(!session.trace().last().unwrap().cache_hit);
        assert!(
            session
                .read_uncached(0, 1, AccessPurpose::EncryptedPayload)
                .is_err()
        );
        assert_eq!(session.range_requests(), 2);

        let source = MemoryRandomReadSource::new(Vec::from(&b"0123456789"[..]));
        let policy = RandomAccessPolicy {
            max_trace_entries: 0,
            ..RandomAccessPolicy::default()
        };
        let mut no_trace = RangeSession::new(Box::new(source), policy).unwrap();
        assert!(
            no_trace
                .read_uncached(0, 10, AccessPurpose::EncryptedPayload)
                .is_err()
        );
        assert_eq!(no_trace.bytes_fetched(), 0);
    }

    #[test]
    fn policy_refuses_before_fetching() {
        let source = MemoryRandomReadSource::new(Vec::from(&b"0123456789"[..]));
        let policy = RandomAccessPolicy {
            max_individual_range_bytes: 2,
            ..RandomAccessPolicy::default()
        };
        let mut session = RangeSession::new(Box::new(source), policy).unwrap();
        assert!(session.read(0, 3, AccessPurpose::Chunk).is_err());
        assert_eq!(session.bytes_fetched(), 0);
    }

    #[test]
    fn http_ranges_require_strong_revision_and_exact_response() {
        let (url, requests, thread) = test_server(vec![
            "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nAccept-Ranges: bytes\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n",
            "HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 2-5/10\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n2345",
        ]);
        let source = HttpRangeSource::open(&url).unwrap();
        assert_eq!(source.read_exact_at(2, 4).unwrap(), b"2345");
        thread.join().unwrap();
        let requests = requests.lock().unwrap();
        assert!(requests[1].contains("range: bytes=2-5"));
        assert!(requests[1].contains("if-match: \"stable\""));
        assert!(requests[1].contains("accept-encoding: identity"));
    }

    #[test]
    fn http_rejects_weak_etag_before_any_range() {
        let (url, _, thread) = test_server(vec![
            "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nAccept-Ranges: bytes\r\nETag: W/\"weak\"\r\nConnection: close\r\n\r\n",
        ]);
        let error = HttpRangeSource::open(&url).err().unwrap();
        assert_eq!(
            error.code(),
            crate::diagnostics::ReasonCode::HttpRangeUnsupported
        );
        thread.join().unwrap();
    }

    #[test]
    fn http_never_falls_back_when_range_is_ignored() {
        let (url, _, thread) = test_server(vec![
            "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nAccept-Ranges: bytes\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n",
            "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n0123456789",
        ]);
        let source = HttpRangeSource::open(&url).unwrap();
        let error = source.read_exact_at(2, 4).unwrap_err();
        assert_eq!(
            error.code(),
            crate::diagnostics::ReasonCode::HttpRangeUnsupported
        );
        thread.join().unwrap();
    }

    #[test]
    fn http_detects_etag_change_on_range() {
        let (url, _, thread) = test_server(vec![
            "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nAccept-Ranges: bytes\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n",
            "HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 2-5/10\r\nETag: \"changed\"\r\nConnection: close\r\n\r\n2345",
        ]);
        let source = HttpRangeSource::open(&url).unwrap();
        let error = source.read_exact_at(2, 4).unwrap_err();
        assert_eq!(error.code(), crate::diagnostics::ReasonCode::SourceUnstable);
        thread.join().unwrap();
    }

    #[test]
    fn http_rejects_malformed_content_range() {
        let (url, _, thread) = test_server(vec![
            "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nAccept-Ranges: bytes\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n",
            "HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 2-6/10\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n2345",
        ]);
        let source = HttpRangeSource::open(&url).unwrap();
        let error = source.read_exact_at(2, 4).unwrap_err();
        assert_eq!(
            error.code(),
            crate::diagnostics::ReasonCode::HttpRangeInvalid
        );
        thread.join().unwrap();
    }

    #[test]
    fn http_final_revalidation_detects_changed_length() {
        let stable = "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nAccept-Ranges: bytes\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n";
        let (url, _, thread) = test_server(vec![
            stable,
            stable,
            "HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 2-5/10\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n2345",
            "HTTP/1.1 200 OK\r\nContent-Length: 11\r\nAccept-Ranges: bytes\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n",
        ]);
        let source = HttpRangeSource::open(&url).unwrap();
        let mut session =
            RangeSession::new(Box::new(source), RandomAccessPolicy::default()).unwrap();
        assert_eq!(session.read(2, 4, AccessPurpose::Chunk).unwrap(), b"2345");
        assert_eq!(
            session.check_stable().unwrap_err().code(),
            crate::diagnostics::ReasonCode::SourceUnstable
        );
        thread.join().unwrap();
    }

    #[test]
    fn http_rejects_transparent_content_encoding() {
        let (url, _, thread) = test_server(vec![
            "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nAccept-Ranges: bytes\r\nETag: \"stable\"\r\nContent-Encoding: gzip\r\nConnection: close\r\n\r\n",
        ]);
        assert_eq!(
            HttpRangeSource::open(&url).err().unwrap().code(),
            crate::diagnostics::ReasonCode::HttpRangeUnsupported
        );
        thread.join().unwrap();
    }

    #[test]
    fn http_rejects_a_short_range_body() {
        let (url, _, thread) = test_server(vec![
            "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nAccept-Ranges: bytes\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n",
            "HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 2-5/10\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n234",
        ]);
        let source = HttpRangeSource::open(&url).unwrap();
        assert_eq!(
            source.read_exact_at(2, 4).unwrap_err().code(),
            crate::diagnostics::ReasonCode::HttpRangeInvalid
        );
        thread.join().unwrap();
    }
}
