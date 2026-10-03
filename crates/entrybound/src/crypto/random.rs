//! Authenticated, range-backed access to crypto-v1 INDEXED archives.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest as _, Sha256};
use subtle::ConstantTimeEq as _;

use super::working::{CryptoWorkContext, WorkBudget, WorkBytes, WorkVec};
use super::{
    BoundaryMode, EncryptedOpenOptions, KeyHierarchy, PaddingMode, aead_open,
    public_crypto_context_digest, public_crypto_context_work, wire,
};
use crate::canonical::decode_record_view as decode_record;
use crate::codec::{PlanMode, aggregate_archive_decode_requirements, plan_mode, validate_plans};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    ArchiveDescriptor, ArchiveRole, Chunk, ChunkGroup, ContentObject, ContentRef, Dictionary,
    Digest, DigestAlgorithm, EntryData, EntrySet, FeatureSet, IdentityProfile, Layout,
    ReconstructionData, ReconstructionRegion, ResourceBudget, TransformPlan,
};
use crate::ecf::container::{
    ChunkFrameHeader, chunk_frame_header_len, decode_frame_payload, decode_preamble,
    enforce_chunk_bounds, has_codec_transform_feature, has_cross_file_feature,
    has_reconstructive_feature, has_whole_object_feature, parse_chunk_frame_header,
    physical_prefix_from_slices, reconstruct_region_members,
};
use crate::ecf::records::{
    decode_chunk_groups, decode_descriptor, decode_dictionaries, decode_fidelity, decode_manifest,
    decode_reconstruction_regions, decode_reconstruction_section, decode_transform_plans,
    decode_transform_plans_v2, decode_transform_plans_v3,
};
use crate::ecf::{
    IdentityVerificationStatus, RandomAccessIndexStatus, RandomAccessMetadata, RandomAccessRead,
    RandomAccessSection, RandomAccessVerificationReport,
};
use crate::identity::{
    chunk_root_from_leaves, sha256_exact, verify_metadata_aux, verify_metadata_lai,
};
use crate::random_access::{
    AccessPurpose, AccessTraceEntry, RandomAccessPolicy, RandomReadSource, RangeSession,
    SourceRevision,
};
use zeroize::Zeroize as _;

/// A conservative source-size screen for the slice-based legacy full verifier.
/// This is not a proved aggregate live-allocation reservation: container growth,
/// source-return capacity and overlapping transient buffers still need a
/// checked allocation ledger. Ordinary ECF model/decode allocations remain
/// independently governed by ResourceBudget and DecodeRequirements.
fn legacy_crypto_reservation(source_len: u64, policy: super::CryptoPolicy) -> Result<()> {
    // Refuse large legacy full-source materialization before fetching it.
    // Qualification must not interpret this provisional factor as a bound on
    // all coexisting capacities or allocator/container overhead.
    const CRYPTO_RESERVATION_MULTIPLIER: u64 = 64;
    let reserved = source_len
        .checked_mul(CRYPTO_RESERVATION_MULTIPLIER)
        .ok_or_else(|| crypto_policy("legacy full verification reservation overflows u64"))?;
    if reserved > policy.max_working_memory_bytes {
        return Err(crypto_policy(
            "legacy full verification exceeds aggregate crypto working-memory policy",
        ));
    }
    Ok(())
}

const ENCRYPTED_FOOTER_LEN: u64 = 192;
const SECTION_HEADER_LEN: u64 = 64;
const SEGMENT_HEADER_LEN: u64 = 64;
const PROTECTED_HEADER_LEN: u64 = 32;
const SEGMENT_CONTROL: u8 = 1;
const SEGMENT_PAYLOAD: u8 = 2;
const RECORD_DATA: u8 = 1;
const RECORD_END: u8 = 2;

#[derive(Clone, Copy)]
struct Footer {
    total_len: u64,
    envelope_offset: u64,
    envelope_len: u64,
    segments_offset: u64,
    segments_len: u64,
    terminal_offset: u64,
    archive_final_offset: u64,
    preamble_digest: [u8; 32],
    public_context_digest: [u8; 32],
}

#[derive(Clone, Copy)]
struct SegmentLocator {
    offset: u64,
    extent: u64,
    class: u8,
    salt: [u8; 16],
    count: u32,
    header: [u8; 64],
    left: usize,
    right: usize,
    parent: usize,
    height: u32,
}

const NO_SEGMENT_SLOT: usize = usize::MAX;

#[derive(Clone, Copy)]
struct SegmentVacant {
    parent: usize,
    left: bool,
}

// Public framing stays in physical ordinal order. Rotations change only links.
struct SegmentTable {
    locators: WorkVec<SegmentLocator>,
    root: usize,
    capacity: usize,
}

#[cfg(test)]
std::thread_local! {
    static SEGMENT_TABLE_BUDGET_OBSERVER: std::cell::Cell<Option<fn(&WorkBudget)>> =
        const { std::cell::Cell::new(None) };
}

impl SegmentTable {
    fn new(budget: &WorkBudget) -> Self {
        #[cfg(test)]
        SEGMENT_TABLE_BUDGET_OBSERVER.with(|observer| {
            if let Some(probe) = observer.get() {
                probe(budget);
            }
        });
        Self {
            locators: WorkVec::new(budget),
            root: NO_SEGMENT_SLOT,
            capacity: 0,
        }
    }

    fn len(&self) -> usize {
        self.locators.as_slice().len()
    }

    fn as_slice(&self) -> &[SegmentLocator] {
        self.locators.as_slice()
    }

    fn next_capacity(capacity: usize) -> Result<usize> {
        let target = if capacity == 0 {
            1
        } else {
            capacity
                .checked_mul(2)
                .ok_or_else(|| crypto_policy("Segment table capacity overflows"))?
        };
        if target == NO_SEGMENT_SLOT {
            return Err(crypto_policy(
                "Segment table capacity reaches its index sentinel",
            ));
        }
        Ok(target)
    }

    fn reservation_bytes(capacity: usize) -> Result<u64> {
        if capacity == NO_SEGMENT_SLOT {
            return Err(crypto_policy(
                "Segment table capacity reaches its index sentinel",
            ));
        }
        let layout = std::alloc::Layout::array::<SegmentLocator>(capacity)
            .map_err(|_| crypto_policy("Segment table allocation layout overflows"))?;
        u64::try_from(layout.size())
            .map_err(|_| crypto_policy("Segment table allocation exceeds u64"))
    }

    fn probe(&self, class: u8, salt: &[u8; 16]) -> Option<SegmentVacant> {
        let mut index = self.root;
        let mut parent = NO_SEGMENT_SLOT;
        let mut left = false;
        while index != NO_SEGMENT_SLOT {
            parent = index;
            let slot = &self.locators.as_slice()[index];
            match class.cmp(&slot.class).then_with(|| salt.cmp(&slot.salt)) {
                std::cmp::Ordering::Equal => return None,
                std::cmp::Ordering::Less => {
                    left = true;
                    index = slot.left;
                }
                std::cmp::Ordering::Greater => {
                    left = false;
                    index = slot.right;
                }
            }
        }
        Some(SegmentVacant { parent, left })
    }

    fn height(&self, index: usize) -> u32 {
        if index == NO_SEGMENT_SLOT {
            0
        } else {
            self.locators.as_slice()[index].height
        }
    }

    fn next_height(height: u32) -> Result<u32> {
        height
            .checked_add(1)
            .ok_or_else(|| crypto_policy("Segment table height overflows"))
    }

    fn next_index(index: usize) -> Result<usize> {
        index
            .checked_add(1)
            .filter(|required| *required < NO_SEGMENT_SLOT)
            .ok_or_else(|| crypto_policy("Segment table index reaches its sentinel"))
    }

    fn refresh_height(&mut self, index: usize) -> Result<()> {
        let slot = self.locators.as_slice()[index];
        let height = Self::next_height(self.height(slot.left).max(self.height(slot.right)))?;
        self.locators.as_mut_slice()[index].height = height;
        Ok(())
    }

    fn replace_parent_child(&mut self, parent: usize, old: usize, new: usize) {
        if parent == NO_SEGMENT_SLOT {
            self.root = new;
        } else if self.locators.as_slice()[parent].left == old {
            self.locators.as_mut_slice()[parent].left = new;
        } else {
            debug_assert_eq!(self.locators.as_slice()[parent].right, old);
            self.locators.as_mut_slice()[parent].right = new;
        }
    }

    fn rotate_left(&mut self, x: usize) -> Result<usize> {
        let y = self.locators.as_slice()[x].right;
        let beta = self.locators.as_slice()[y].left;
        let parent = self.locators.as_slice()[x].parent;
        self.replace_parent_child(parent, x, y);
        let slots = self.locators.as_mut_slice();
        slots[y].parent = parent;
        slots[y].left = x;
        slots[x].parent = y;
        slots[x].right = beta;
        if beta != NO_SEGMENT_SLOT {
            slots[beta].parent = x;
        }
        self.refresh_height(x)?;
        self.refresh_height(y)?;
        Ok(y)
    }

    fn rotate_right(&mut self, x: usize) -> Result<usize> {
        let y = self.locators.as_slice()[x].left;
        let beta = self.locators.as_slice()[y].right;
        let parent = self.locators.as_slice()[x].parent;
        self.replace_parent_child(parent, x, y);
        let slots = self.locators.as_mut_slice();
        slots[y].parent = parent;
        slots[y].right = x;
        slots[x].parent = y;
        slots[x].left = beta;
        if beta != NO_SEGMENT_SLOT {
            slots[beta].parent = x;
        }
        self.refresh_height(x)?;
        self.refresh_height(y)?;
        Ok(y)
    }

    fn append_unique(&mut self, mut locator: SegmentLocator, vacant: SegmentVacant) -> Result<()> {
        let index = self.len();
        let required = Self::next_index(index)?;
        // Valid insertion increases root height at most once; guard it before growth.
        Self::next_height(self.height(self.root))?;
        debug_assert!(index <= self.capacity);
        if index == self.capacity {
            let target = Self::next_capacity(self.capacity)?;
            Self::reservation_bytes(target)?;
            let additional = target
                .checked_sub(index)
                .ok_or_else(|| crypto_policy("Segment table capacity is below its length"))?;
            // At a full transition required == preferred == target. No smaller retry.
            self.locators.reserve(additional)?;
            self.capacity = target;
        }
        debug_assert!(required <= self.capacity);
        locator.left = NO_SEGMENT_SLOT;
        locator.right = NO_SEGMENT_SLOT;
        locator.parent = vacant.parent;
        locator.height = 1;
        // The explicit reserve covers this slot; push cannot grow the backing.
        self.locators.push(locator)?;
        if vacant.parent == NO_SEGMENT_SLOT {
            self.root = index;
        } else if vacant.left {
            self.locators.as_mut_slice()[vacant.parent].left = index;
        } else {
            self.locators.as_mut_slice()[vacant.parent].right = index;
        }
        let mut ancestor = vacant.parent;
        while ancestor != NO_SEGMENT_SLOT {
            self.refresh_height(ancestor)?;
            let slot = self.locators.as_slice()[ancestor];
            let left_height = self.height(slot.left);
            let right_height = self.height(slot.right);
            let balanced = if left_height.saturating_sub(right_height) > 1 {
                let left = self.locators.as_slice()[slot.left];
                if self.height(left.right) > self.height(left.left) {
                    self.rotate_left(slot.left)?;
                }
                self.rotate_right(ancestor)?
            } else if right_height.saturating_sub(left_height) > 1 {
                let right = self.locators.as_slice()[slot.right];
                if self.height(right.left) > self.height(right.right) {
                    self.rotate_right(slot.right)?;
                }
                self.rotate_left(ancestor)?
            } else {
                ancestor
            };
            ancestor = self.locators.as_slice()[balanced].parent;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct EncryptedChunkLocator {
    segment_ordinal: u64,
    fragment_count: u32,
}

// This fixed owner never grows after initializing a secret Slot. Generic
// WorkVec growth is only used once, while empty; this wrapper owns wiping.
const NO_CHUNK_SLOT: usize = usize::MAX;

#[derive(Clone, Copy)]
struct ChunkSlot {
    digest: [u8; 32],
    locator: EncryptedChunkLocator,
    left: usize,
    right: usize,
    parent: usize,
    height: u32,
}

struct ChunkDirectory {
    slots: WorkVec<ChunkSlot>,
    bound: usize,
    reserved: bool,
    root: usize,
    sorted: bool,
    #[cfg(test)]
    budget: WorkBudget,
    #[cfg(test)]
    before_free: Option<fn(&[ChunkSlot], &WorkBudget)>,
}

impl ChunkDirectory {
    fn new(budget: &WorkBudget, bound: usize, sorted: bool) -> Self {
        Self {
            slots: WorkVec::new(budget),
            bound,
            reserved: false,
            root: NO_CHUNK_SLOT,
            sorted,
            #[cfg(test)]
            budget: budget.clone(),
            #[cfg(test)]
            before_free: None,
        }
    }

    fn reservation_bytes(bound: usize) -> Result<u64> {
        if bound == NO_CHUNK_SLOT {
            return Err(crypto_policy(
                "Chunk directory bound reaches its index sentinel",
            ));
        }
        let layout = std::alloc::Layout::array::<ChunkSlot>(bound)
            .map_err(|_| crypto_policy("Chunk directory allocation layout overflows"))?;
        u64::try_from(layout.size())
            .map_err(|_| crypto_policy("Chunk directory allocation exceeds u64"))
    }

    fn reserve_once(&mut self) -> Result<()> {
        if !self.reserved {
            debug_assert_eq!(self.len(), 0);
            Self::reservation_bytes(self.bound)?;
            self.slots.reserve(self.bound)?;
            self.reserved = true;
        }
        Ok(())
    }

    fn len(&self) -> usize {
        self.slots.as_slice().len()
    }

    fn append(
        &mut self,
        digest: Digest,
        locator: EncryptedChunkLocator,
        parent: usize,
    ) -> Result<usize> {
        if self.len() >= self.bound {
            return Err(crypto_policy(
                "Chunk directory fixed slot bound is exhausted",
            ));
        }
        self.reserve_once()?;
        let index = self.len();
        // The sole reserve was for bound slots; push cannot grow this owner.
        self.slots.push(ChunkSlot {
            digest: *digest.as_bytes(),
            locator,
            left: NO_CHUNK_SLOT,
            right: NO_CHUNK_SLOT,
            parent,
            height: 1,
        })?;
        Ok(index)
    }

    fn append_index(&mut self, digest: Digest, locator: EncryptedChunkLocator) -> Result<bool> {
        debug_assert!(self.sorted);
        if self.get(&digest).is_some() {
            return Ok(true);
        }
        self.append(digest, locator, NO_CHUNK_SLOT)?;
        Ok(false)
    }

    fn height(&self, index: usize) -> u32 {
        if index == NO_CHUNK_SLOT {
            0
        } else {
            self.slots.as_slice()[index].height
        }
    }

    fn refresh_height(&mut self, index: usize) {
        let slot = self.slots.as_slice()[index];
        self.slots.as_mut_slice()[index].height =
            1 + self.height(slot.left).max(self.height(slot.right));
    }

    fn replace_parent_child(&mut self, parent: usize, old: usize, new: usize) {
        if parent == NO_CHUNK_SLOT {
            self.root = new;
        } else if self.slots.as_slice()[parent].left == old {
            self.slots.as_mut_slice()[parent].left = new;
        } else {
            debug_assert_eq!(self.slots.as_slice()[parent].right, old);
            self.slots.as_mut_slice()[parent].right = new;
        }
    }

    fn rotate_left(&mut self, x: usize) -> usize {
        let y = self.slots.as_slice()[x].right;
        let beta = self.slots.as_slice()[y].left;
        let parent = self.slots.as_slice()[x].parent;
        self.replace_parent_child(parent, x, y);
        let slots = self.slots.as_mut_slice();
        slots[y].parent = parent;
        slots[y].left = x;
        slots[x].parent = y;
        slots[x].right = beta;
        if beta != NO_CHUNK_SLOT {
            slots[beta].parent = x;
        }
        self.refresh_height(x);
        self.refresh_height(y);
        y
    }

    fn rotate_right(&mut self, x: usize) -> usize {
        let y = self.slots.as_slice()[x].left;
        let beta = self.slots.as_slice()[y].right;
        let parent = self.slots.as_slice()[x].parent;
        self.replace_parent_child(parent, x, y);
        let slots = self.slots.as_mut_slice();
        slots[y].parent = parent;
        slots[y].right = x;
        slots[x].parent = y;
        slots[x].left = beta;
        if beta != NO_CHUNK_SLOT {
            slots[beta].parent = x;
        }
        self.refresh_height(x);
        self.refresh_height(y);
        y
    }

    fn insert_fallback(&mut self, digest: Digest, locator: EncryptedChunkLocator) -> Result<()> {
        debug_assert!(!self.sorted);
        let mut index = self.root;
        let mut parent = NO_CHUNK_SLOT;
        while index != NO_CHUNK_SLOT {
            parent = index;
            let slot = &self.slots.as_slice()[index];
            match digest.as_bytes().cmp(&slot.digest) {
                std::cmp::Ordering::Equal => {
                    self.slots.as_mut_slice()[index].locator = locator;
                    return Ok(());
                }
                std::cmp::Ordering::Less => index = slot.left,
                std::cmp::Ordering::Greater => index = slot.right,
            }
        }
        let inserted = self.append(digest, locator, parent)?;
        if parent == NO_CHUNK_SLOT {
            self.root = inserted;
        } else if digest.as_bytes() < &self.slots.as_slice()[parent].digest {
            self.slots.as_mut_slice()[parent].left = inserted;
        } else {
            self.slots.as_mut_slice()[parent].right = inserted;
        }
        let mut ancestor = parent;
        while ancestor != NO_CHUNK_SLOT {
            self.refresh_height(ancestor);
            let slot = self.slots.as_slice()[ancestor];
            let left_height = self.height(slot.left);
            let right_height = self.height(slot.right);
            let balanced_root = if left_height > right_height + 1 {
                let left = self.slots.as_slice()[slot.left];
                if self.height(left.right) > self.height(left.left) {
                    self.rotate_left(slot.left);
                }
                self.rotate_right(ancestor)
            } else if right_height > left_height + 1 {
                let right = self.slots.as_slice()[slot.right];
                if self.height(right.left) > self.height(right.right) {
                    self.rotate_right(slot.right);
                }
                self.rotate_left(ancestor)
            } else {
                ancestor
            };
            ancestor = self.slots.as_slice()[balanced_root].parent;
        }
        Ok(())
    }

    fn sift_slots(slots: &mut [ChunkSlot], mut root: usize, end: usize) {
        // end <= admitted bound < usize::MAX. An internal node satisfies
        // root < end/2, so 2*root+1 < end and left+1 <= end cannot overflow.
        while root < end / 2 {
            let left = root * 2 + 1;
            let right = left + 1;
            let child = if right < end && slots[left].digest < slots[right].digest {
                right
            } else {
                left
            };
            if slots[root].digest >= slots[child].digest {
                break;
            }
            slots.swap(root, child);
            root = child;
        }
    }

    fn sort_slots(slots: &mut [ChunkSlot]) {
        // Bottom-up max heap, then a shrinking heap and ascending suffix.
        // Only fixed scalar indices and same-slice swaps are used: no second
        // backing, allocator call, recursion or heap path stack is reachable.
        for root in (0..slots.len() / 2).rev() {
            Self::sift_slots(slots, root, slots.len());
        }
        for end in (1..slots.len()).rev() {
            slots.swap(0, end);
            Self::sift_slots(slots, 0, end);
        }
    }

    fn finish(&mut self) {
        Self::sort_slots(self.slots.as_mut_slice());
        for slot in self.slots.as_mut_slice() {
            slot.left = NO_CHUNK_SLOT;
            slot.right = NO_CHUNK_SLOT;
            slot.parent = NO_CHUNK_SLOT;
            slot.height = 0;
        }
        self.root = NO_CHUNK_SLOT;
        self.sorted = true;
    }

    fn get(&self, digest: &Digest) -> Option<&EncryptedChunkLocator> {
        debug_assert!(self.sorted);
        self.slots
            .as_slice()
            .binary_search_by(|slot| slot.digest.cmp(digest.as_bytes()))
            .ok()
            .map(|index| &self.slots.as_slice()[index].locator)
    }

    fn iter(&self) -> impl Iterator<Item = (&[u8; 32], &EncryptedChunkLocator)> {
        self.slots
            .as_slice()
            .iter()
            .map(|slot| (&slot.digest, &slot.locator))
    }

    fn values(&self) -> impl Iterator<Item = &EncryptedChunkLocator> {
        self.slots.as_slice().iter().map(|slot| &slot.locator)
    }
}

impl Drop for ChunkDirectory {
    fn drop(&mut self) {
        for slot in self.slots.as_mut_slice() {
            slot.digest.zeroize();
            slot.locator.segment_ordinal.zeroize();
            slot.locator.fragment_count.zeroize();
            slot.left.zeroize();
            slot.right.zeroize();
            slot.parent.zeroize();
            slot.height.zeroize();
        }
        #[cfg(test)]
        if let Some(probe) = self.before_free {
            probe(self.slots.as_slice(), &self.budget);
        }
        // WorkVec subsequently frees backing before releasing its original lease.
    }
}

// The initialized length remains K after compaction: duplicate tail ordinals
// still belong to this secret owner and are wiped before its lease is released.
struct KnownChunkSegments {
    slots: WorkVec<u64>,
    bound: usize,
    unique_len: usize,
    #[cfg(test)]
    budget: WorkBudget,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KnownChunkPhase {
    BeforeReserve,
    Ready,
    BeforeFree,
}

#[cfg(test)]
type KnownChunkObserver = fn(KnownChunkPhase, &WorkBudget, &[u64], usize);

#[cfg(test)]
std::thread_local! {
    static KNOWN_CHUNK_OBSERVER: std::cell::Cell<Option<KnownChunkObserver>> =
        const { std::cell::Cell::new(None) };
}

impl KnownChunkSegments {
    fn reservation_bytes(bound: usize) -> Result<u64> {
        let layout = std::alloc::Layout::array::<u64>(bound)
            .map_err(|_| crypto_policy("known Chunk ordinal allocation layout overflows"))?;
        u64::try_from(layout.size())
            .map_err(|_| crypto_policy("known Chunk ordinal allocation exceeds u64"))
    }

    fn reserve_empty(budget: &WorkBudget, bound: usize) -> Result<Self> {
        // Construct the wiping wrapper before either allocation or initialization.
        let mut owner = Self {
            slots: WorkVec::new(budget),
            bound,
            unique_len: 0,
            #[cfg(test)]
            budget: budget.clone(),
        };
        Self::reservation_bytes(bound)?;
        #[cfg(test)]
        owner.observe(KnownChunkPhase::BeforeReserve);
        if bound != 0 {
            // The only reserve is while empty, for the complete physical K.
            owner.slots.reserve(bound)?;
        }
        Ok(owner)
    }

    fn new(budget: &WorkBudget, index: &ChunkDirectory) -> Result<Self> {
        let mut owner = Self::reserve_empty(budget, index.len())?;
        for locator in index.values() {
            owner.push(locator.segment_ordinal)?;
        }
        owner.finish();
        #[cfg(test)]
        owner.observe(KnownChunkPhase::Ready);
        Ok(owner)
    }

    fn push(&mut self, ordinal: u64) -> Result<()> {
        if self.slots.as_slice().len() >= self.bound {
            return Err(crypto_policy(
                "known Chunk ordinal fixed bound is exhausted",
            ));
        }
        // WorkVec's exact capacity adoption check established K spare slots.
        // This bounded push cannot grow or copy an initialized secret owner.
        self.slots.push(ordinal)
    }

    fn sift(slots: &mut [u64], mut root: usize, end: usize) {
        // root < end/2 proves 2*root+1 < end; right <= end cannot overflow.
        while root < end / 2 {
            let left = root * 2 + 1;
            let right = left + 1;
            let child = if right < end && slots[right] > slots[left] {
                right
            } else {
                left
            };
            if slots[root] >= slots[child] {
                break;
            }
            slots.swap(root, child);
            root = child;
        }
    }

    fn finish(&mut self) {
        let slots = self.slots.as_mut_slice();
        let len = slots.len();
        for root in (0..len / 2).rev() {
            Self::sift(slots, root, len);
        }
        for end in (1..len).rev() {
            slots.swap(0, end);
            Self::sift(slots, 0, end);
        }
        let mut unique_len = 0;
        for read in 0..len {
            if unique_len == 0 || slots[read] != slots[unique_len - 1] {
                slots[unique_len] = slots[read];
                unique_len += 1;
            }
        }
        self.unique_len = unique_len;
    }

    fn contains(&self, ordinal: &u64) -> bool {
        let slots = self.slots.as_slice();
        let (mut low, mut high) = (0, self.unique_len);
        while low < high {
            let middle = low + (high - low) / 2;
            match slots[middle].cmp(ordinal) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => return true,
            }
        }
        false
    }

    #[cfg(test)]
    fn observe(&self, phase: KnownChunkPhase) {
        KNOWN_CHUNK_OBSERVER.with(|cell| {
            if let Some(observer) = cell.get() {
                observer(phase, &self.budget, self.slots.as_slice(), self.bound);
            }
        });
    }
}

impl Drop for KnownChunkSegments {
    fn drop(&mut self) {
        for ordinal in self.slots.as_mut_slice() {
            ordinal.zeroize();
        }
        #[cfg(test)]
        self.observe(KnownChunkPhase::BeforeFree);
        // WorkVec then frees backing before releasing the original budget lease.
    }
}

#[derive(Clone, Copy)]
struct PhysicalPrerequisiteSlot {
    ordinal: u64,
    digest: [u8; 32],
}

// Each owner reserves its entire physical bound once, before filling. The
// initialized fields are secret; generic WorkVec growth must not copy them.
struct PhysicalPrerequisites {
    slots: WorkVec<PhysicalPrerequisiteSlot>,
    bound: usize,
    #[cfg(test)]
    budget: WorkBudget,
}

struct PredecessorDigests {
    slots: WorkVec<[u8; 32]>,
    bound: usize,
    #[cfg(test)]
    budget: WorkBudget,
}

struct PrefixSliceSlots<'a> {
    slots: WorkVec<&'a [u8]>,
    bound: usize,
    #[cfg(test)]
    budget: WorkBudget,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PrerequisiteKind {
    Physical,
    Predecessors,
    Slices,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PrerequisitePhase {
    BeforeReserve,
    Reserved,
    Ready,
    BeforeFree,
}

#[cfg(test)]
#[derive(Clone, Copy)]
enum PrerequisiteSlots<'a> {
    Physical(&'a [PhysicalPrerequisiteSlot]),
    Predecessors(&'a [[u8; 32]]),
    Slices(&'a [&'a [u8]]),
}

#[cfg(test)]
type PrerequisiteObserver =
    fn(PrerequisiteKind, PrerequisitePhase, &WorkBudget, usize, PrerequisiteSlots<'_>);

#[cfg(test)]
std::thread_local! {
    static PREREQUISITE_OBSERVER: std::cell::Cell<Option<PrerequisiteObserver>> =
        const { std::cell::Cell::new(None) };
}

fn prerequisite_reservation_bytes<T>(bound: usize) -> Result<u64> {
    let layout = std::alloc::Layout::array::<T>(bound)
        .map_err(|_| crypto_policy("physical prerequisite allocation layout overflows"))?;
    u64::try_from(layout.size())
        .map_err(|_| crypto_policy("physical prerequisite allocation exceeds u64"))
}

fn physical_prerequisite_first(position: usize, lookback: u32) -> usize {
    position.saturating_sub(usize::try_from(lookback).unwrap_or(usize::MAX))
}

fn prerequisite_logical_total(total: u64, logical_len: u64) -> Result<u64> {
    total
        .checked_add(logical_len)
        .ok_or_else(|| dependency("lookback byte count overflow"))
}

impl PhysicalPrerequisites {
    fn reservation_bytes(bound: usize) -> Result<u64> {
        prerequisite_reservation_bytes::<PhysicalPrerequisiteSlot>(bound)
    }

    fn reserve_empty(budget: &WorkBudget, bound: usize) -> Result<Self> {
        let mut owner = Self {
            slots: WorkVec::new(budget),
            bound,
            #[cfg(test)]
            budget: budget.clone(),
        };
        Self::reservation_bytes(bound)?;
        #[cfg(test)]
        owner.observe(PrerequisitePhase::BeforeReserve);
        if bound != 0 {
            owner.slots.reserve(bound)?;
        }
        #[cfg(test)]
        owner.observe(PrerequisitePhase::Reserved);
        Ok(owner)
    }

    fn new(budget: &WorkBudget, index: &ChunkDirectory) -> Result<Self> {
        let mut owner = Self::reserve_empty(budget, index.len())?;
        for (digest, locator) in index.iter() {
            owner.push(locator.segment_ordinal, *digest)?;
        }
        owner.sort();
        #[cfg(test)]
        owner.observe(PrerequisitePhase::Ready);
        Ok(owner)
    }

    fn push(&mut self, ordinal: u64, digest: [u8; 32]) -> Result<()> {
        if self.slots.as_slice().len() >= self.bound {
            return Err(crypto_policy(
                "physical prerequisite fixed bound is exhausted",
            ));
        }
        // The empty reserve established the full capacity. No growth is possible.
        self.slots
            .push(PhysicalPrerequisiteSlot { ordinal, digest })
    }

    fn compare(a: &PhysicalPrerequisiteSlot, b: &PhysicalPrerequisiteSlot) -> std::cmp::Ordering {
        a.ordinal
            .cmp(&b.ordinal)
            .then_with(|| a.digest.cmp(&b.digest))
    }

    fn sift(slots: &mut [PhysicalPrerequisiteSlot], mut root: usize, end: usize) {
        // root < end/2 proves left < end and right <= end without overflow.
        while root < end / 2 {
            let left = root * 2 + 1;
            let right = left + 1;
            let child = if right < end && Self::compare(&slots[right], &slots[left]).is_gt() {
                right
            } else {
                left
            };
            if !Self::compare(&slots[root], &slots[child]).is_lt() {
                break;
            }
            slots.swap(root, child);
            root = child;
        }
    }

    fn sort(&mut self) {
        let slots = self.slots.as_mut_slice();
        let len = slots.len();
        for root in (0..len / 2).rev() {
            Self::sift(slots, root, len);
        }
        for end in (1..len).rev() {
            slots.swap(0, end);
            Self::sift(slots, 0, end);
        }
        // No deduplication: physical positions include every initialized slot.
    }

    fn as_slice(&self) -> &[PhysicalPrerequisiteSlot] {
        self.slots.as_slice()
    }

    #[cfg(test)]
    fn observe(&self, phase: PrerequisitePhase) {
        PREREQUISITE_OBSERVER.with(|cell| {
            if let Some(observer) = cell.get() {
                observer(
                    PrerequisiteKind::Physical,
                    phase,
                    &self.budget,
                    self.bound,
                    PrerequisiteSlots::Physical(self.as_slice()),
                );
            }
        });
    }
}

impl Drop for PhysicalPrerequisites {
    fn drop(&mut self) {
        for slot in self.slots.as_mut_slice() {
            slot.ordinal.zeroize();
            slot.digest.zeroize();
        }
        #[cfg(test)]
        self.observe(PrerequisitePhase::BeforeFree);
        // WorkVec frees backing before releasing its field-owned original lease.
    }
}

impl PredecessorDigests {
    fn empty(budget: &WorkBudget) -> Self {
        Self {
            slots: WorkVec::new(budget),
            bound: 0,
            #[cfg(test)]
            budget: budget.clone(),
        }
    }

    fn reservation_bytes(bound: usize) -> Result<u64> {
        prerequisite_reservation_bytes::<[u8; 32]>(bound)
    }

    fn reserve_empty(budget: &WorkBudget, bound: usize) -> Result<Self> {
        let mut owner = Self::empty(budget);
        owner.bound = bound;
        Self::reservation_bytes(bound)?;
        #[cfg(test)]
        owner.observe(PrerequisitePhase::BeforeReserve);
        if bound != 0 {
            owner.slots.reserve(bound)?;
        }
        #[cfg(test)]
        owner.observe(PrerequisitePhase::Reserved);
        Ok(owner)
    }

    fn push(&mut self, digest: Digest) -> Result<()> {
        if self.len() >= self.bound {
            return Err(crypto_policy("predecessor digest fixed bound is exhausted"));
        }
        self.slots.push(*digest.as_bytes())
    }

    fn len(&self) -> usize {
        self.slots.as_slice().len()
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn iter(&self) -> impl Iterator<Item = Digest> + '_ {
        self.slots
            .as_slice()
            .iter()
            .map(|bytes| Digest::from_bytes(*bytes))
    }

    #[cfg(test)]
    fn observe(&self, phase: PrerequisitePhase) {
        PREREQUISITE_OBSERVER.with(|cell| {
            if let Some(observer) = cell.get() {
                observer(
                    PrerequisiteKind::Predecessors,
                    phase,
                    &self.budget,
                    self.bound,
                    PrerequisiteSlots::Predecessors(self.slots.as_slice()),
                );
            }
        });
    }
}

impl Drop for PredecessorDigests {
    fn drop(&mut self) {
        for digest in self.slots.as_mut_slice() {
            digest.zeroize();
        }
        #[cfg(test)]
        self.observe(PrerequisitePhase::BeforeFree);
    }
}

impl<'a> PrefixSliceSlots<'a> {
    fn reservation_bytes(bound: usize) -> Result<u64> {
        prerequisite_reservation_bytes::<&[u8]>(bound)
    }

    fn reserve_empty(budget: &WorkBudget, bound: usize) -> Result<Self> {
        let mut owner = Self {
            slots: WorkVec::new(budget),
            bound,
            #[cfg(test)]
            budget: budget.clone(),
        };
        Self::reservation_bytes(bound)?;
        #[cfg(test)]
        owner.observe(PrerequisitePhase::BeforeReserve);
        if bound != 0 {
            owner.slots.reserve(bound)?;
        }
        #[cfg(test)]
        owner.observe(PrerequisitePhase::Reserved);
        Ok(owner)
    }

    fn push(&mut self, bytes: &'a [u8]) -> Result<()> {
        if self.slots.as_slice().len() >= self.bound {
            return Err(crypto_policy("prefix reference fixed bound is exhausted"));
        }
        self.slots.push(bytes)
    }

    fn as_slice(&self) -> &[&'a [u8]] {
        self.slots.as_slice()
    }

    #[cfg(test)]
    fn observe(&self, phase: PrerequisitePhase) {
        PREREQUISITE_OBSERVER.with(|cell| {
            if let Some(observer) = cell.get() {
                observer(
                    PrerequisiteKind::Slices,
                    phase,
                    &self.budget,
                    self.bound,
                    PrerequisiteSlots::Slices(self.as_slice()),
                );
            }
        });
    }
}

impl Drop for PrefixSliceSlots<'_> {
    fn drop(&mut self) {
        for slot in self.slots.as_mut_slice() {
            // Empty the reference metadata; ordinary pointed plaintext is unchanged.
            *slot = &[];
        }
        #[cfg(test)]
        self.observe(PrerequisitePhase::BeforeFree);
    }
}

fn fallback_slot_bound(segments: &[SegmentLocator], cap: u64) -> Result<usize> {
    // Only the current traversal is bounded here: each PAYLOAD segment is
    // considered once and its one returned WorkObject can insert at most once.
    // This is not a general object-per-segment wire packing requirement.
    let payloads = segments
        .iter()
        .filter(|segment| segment.class == SEGMENT_PAYLOAD)
        .count();
    let physical =
        u64::try_from(payloads).map_err(|_| crypto_policy("PAYLOAD segment count exceeds u64"))?;
    if cap < physical {
        usize::try_from(cap).map_err(|_| crypto_policy("Chunk scan cap exceeds usize"))
    } else {
        Ok(payloads)
    }
}

fn discover_chunk(
    directory: &mut ChunkDirectory,
    payload: &[u8],
    cap: u64,
    extended: bool,
    whole_object: bool,
    locator: EncryptedChunkLocator,
) -> Result<()> {
    if u64::try_from(directory.len()).unwrap_or(u64::MAX) >= cap {
        return Err(access_policy(
            "encrypted Chunk-object scan exceeds caller policy",
        ));
    }
    let header_len = usize::try_from(chunk_frame_header_len(extended))
        .map_err(|_| access_policy("Chunk header length exceeds usize"))?;
    let header = parse_chunk_frame_header(
        payload
            .get(..header_len)
            .ok_or_else(|| truncated("encrypted Chunk frame is truncated"))?,
        extended,
        whole_object,
    )?;
    // A conservative physical upper capacity can refuse a support-heavy scan
    // that incremental storage could admit. This truthful policy refusal is
    // fatal; it neither retries with a new budget nor rebuilds an invalid cache.
    directory.insert_fallback(header.chunk_id, locator)
}

#[derive(Default)]
struct ControlObjects {
    descriptor: Option<wire::WorkObject>,
    descriptor_id: Option<[u8; 32]>,
    plans: Option<wire::WorkSequence>,
    groups: Option<wire::WorkSequence>,
    manifest: Option<wire::WorkSequence>,
    manifest_id: Option<[u8; 32]>,
    fidelity: Option<wire::WorkObject>,
    index: Option<wire::WorkSequence>,
    index_invalid: bool,
    final_value: Option<ArchiveFinal>,
}

struct SupportObjects {
    dictionaries: BTreeMap<Digest, Dictionary>,
    reconstruction_data: BTreeMap<Digest, ReconstructionData>,
    regions: BTreeMap<Digest, ReconstructionRegion>,
}

#[derive(Clone, Copy)]
struct ArchiveFinal {
    segment_count: u64,
    entry_count: u64,
    total_logical: u64,
    chunk_count: u64,
    lai: [u8; 32],
    pcr: [u8; 32],
    aux: [u8; 32],
    recipient_set: [u8; 32],
    footer_core: [u8; 32],
    descriptor_id: [u8; 32],
    manifest_id: [u8; 32],
}

/// Public, range-backed inspection of encrypted framing. This does not unlock
/// or authenticate private archive semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptedRandomPublicInspection {
    pub public: super::PublicCryptoInspection,
    pub source_revision: SourceRevision,
    pub required_features: u64,
    pub bytes_fetched: u64,
    pub range_request_count: u64,
    pub access_trace: Box<[AccessTraceEntry]>,
    pub whole_archive_verified: bool,
}

/// An unlocked encrypted INDEXED archive whose unrelated PAYLOAD segments have
/// not been read.
pub struct EncryptedRandomAccessArchive {
    session: RangeSession,
    metadata: RandomAccessMetadata,
    envelope: wire::WorkEnvelope,
    keys: KeyHierarchy,
    padding: PaddingMode,
    work: CryptoWorkContext,
    legacy_whole_archive_verified: bool,
    segments: SegmentTable,
    index: ChunkDirectory,
    index_status: RandomAccessIndexStatus,
    plans: Box<[TransformPlan]>,
    groups: BTreeMap<Digest, ChunkGroup>,
    support: Option<SupportObjects>,
    extended: bool,
    whole_object: bool,
}

/// Opens encrypted INDEXED metadata through authenticated CONTROL records.
pub fn open_indexed_random_encrypted(
    source: impl RandomReadSource + 'static,
    policy: RandomAccessPolicy,
    options: EncryptedOpenOptions<'_>,
) -> Result<EncryptedRandomAccessArchive> {
    EncryptedRandomAccessArchive::open(Box::new(source), policy, options)
}

/// Inspects only public crypto-v1 framing over bounded byte ranges.
pub fn inspect_indexed_random_encrypted_public(
    source: impl RandomReadSource + 'static,
    policy: RandomAccessPolicy,
    crypto_limits: super::CryptoPolicy,
) -> Result<EncryptedRandomPublicInspection> {
    let work = CryptoWorkContext::new(crypto_limits);
    let mut session = RangeSession::new_crypto(Box::new(source), policy, work.budget().clone())?;
    if session.policy().max_section_count < 2 {
        return Err(access_policy(
            "encrypted INDEXED framing requires two physical sections",
        ));
    }
    if session.len() < crate::ecf::PREAMBLE_LEN + ENCRYPTED_FOOTER_LEN {
        return Err(truncated("encrypted INDEXED footer is missing"));
    }
    let footer_offset = session.len() - ENCRYPTED_FOOTER_LEN;
    let footer_bytes =
        session.read_crypto(footer_offset, ENCRYPTED_FOOTER_LEN, AccessPurpose::Footer)?;
    let footer = parse_footer(&footer_bytes, session.len())?;
    let preamble_bytes =
        session.read_crypto(0, crate::ecf::PREAMBLE_LEN, AccessPurpose::Preamble)?;
    let preamble = decode_preamble(&preamble_bytes)?;
    if preamble.layout != Layout::Indexed
        || preamble.features.incompat & super::FEATURE_ENCRYPTED_INDEXED_V1 == 0
        || preamble.features.incompat & super::FEATURE_PAYLOAD_SUITE_V1 == 0
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::RandomAccessNotIndexed,
            "source is not a crypto-v1 INDEXED archive",
        ));
    }
    if footer.total_len != session.len()
        || footer.preamble_digest != *sha256_exact(&preamble_bytes).as_bytes()
    {
        return Err(crypto_integrity(
            ReasonCode::FooterBindingMismatch,
            "encrypted footer length/preamble binding mismatch",
        ));
    }
    if footer.envelope_offset != crate::ecf::PREAMBLE_LEN
        || footer.envelope_offset.checked_add(footer.envelope_len) != Some(footer.segments_offset)
        || footer.segments_offset.checked_add(footer.segments_len) != Some(footer_offset)
    {
        return Err(segment_invalid(
            "encrypted section extents are not canonical",
        ));
    }
    if footer.envelope_len > crypto_limits.max_envelope_bytes {
        return Err(crypto_policy("public CryptoEnvelope exceeds caller policy"));
    }
    let envelope_owner = read_envelope(
        &mut session,
        footer.envelope_offset,
        footer.envelope_len,
        &work,
    )?;
    let envelope = envelope_owner.view();
    if envelope.stanzas.len() as u32 > crypto_limits.max_stanzas {
        return Err(crypto_policy("public CryptoEnvelope exceeds caller policy"));
    }
    super::container::validate_envelope_policy(&envelope, preamble.features.incompat)?;
    let padding = PaddingMode::try_from(envelope.padding_mode)?;
    let boundary = BoundaryMode::try_from(envelope.boundary_mode)?;
    let context = public_crypto_context_digest(
        &envelope.archive_id,
        preamble.features.incompat,
        padding,
        boundary,
    )?;
    if !constant_time_eq(&context, &footer.public_context_digest) {
        return Err(crypto_integrity(
            ReasonCode::FooterBindingMismatch,
            "encrypted public-context/footer binding mismatch",
        ));
    }
    let options = EncryptedOpenOptions {
        unlock: None,
        crypto_policy: crypto_limits,
        resource_policy: session.policy().resource_policy,
        decode_policy: session.policy().decode_policy,
    };
    let segments = walk_segments(&mut session, &footer, options, &work)?;
    session.check_stable()?;
    Ok(EncryptedRandomPublicInspection {
        public: super::container::public_inspection(
            &envelope,
            session.len(),
            Some(u64::try_from(segments.len()).unwrap_or(u64::MAX)),
        )?,
        source_revision: session.initial_revision().clone(),
        required_features: preamble.features.incompat,
        bytes_fetched: session.bytes_fetched(),
        range_request_count: session.range_requests(),
        access_trace: session.trace().to_vec().into_boxed_slice(),
        whole_archive_verified: false,
    })
}

impl EncryptedRandomAccessArchive {
    fn open(
        source: Box<dyn RandomReadSource>,
        mut policy: RandomAccessPolicy,
        mut options: EncryptedOpenOptions<'_>,
    ) -> Result<Self> {
        // Neither caller-owned policy can grant permission denied by the other.
        policy.resource_policy = crate::ecf::container::intersect_resource_policies(
            policy.resource_policy,
            options.resource_policy,
        );
        policy.decode_policy = crate::ecf::container::intersect_decode_policies(
            policy.decode_policy,
            options.decode_policy,
        );
        options.resource_policy = policy.resource_policy;
        options.decode_policy = policy.decode_policy;
        let work = CryptoWorkContext::new(options.crypto_policy);
        let mut session = RangeSession::new_crypto(source, policy, work.budget().clone())?;
        if session.policy().max_section_count < 2 {
            return Err(access_policy(
                "encrypted INDEXED framing requires two physical sections",
            ));
        }
        if session.len() < crate::ecf::PREAMBLE_LEN + ENCRYPTED_FOOTER_LEN {
            return Err(truncated("encrypted INDEXED footer is missing"));
        }
        let footer_offset = session.len() - ENCRYPTED_FOOTER_LEN;
        let footer_bytes =
            session.read_crypto(footer_offset, ENCRYPTED_FOOTER_LEN, AccessPurpose::Footer)?;
        let footer = parse_footer(&footer_bytes, session.len())?;
        let preamble_bytes =
            session.read_crypto(0, crate::ecf::PREAMBLE_LEN, AccessPurpose::Preamble)?;
        let preamble = decode_preamble(&preamble_bytes)?;
        if preamble.layout != Layout::Indexed
            || preamble.features.incompat & super::FEATURE_ENCRYPTED_INDEXED_V1 == 0
            || preamble.features.incompat & super::FEATURE_PAYLOAD_SUITE_V1 == 0
        {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::RandomAccessNotIndexed,
                "source is not a crypto-v1 INDEXED archive",
            ));
        }
        if preamble.features.incompat & super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1 == 0 {
            // The missing feature is the public Descriptor-v1 signal. Charge
            // the complete fallback before envelope/control parsing and any
            // variable-size crypto allocation in this session.
            legacy_crypto_reservation(session.len(), options.crypto_policy)?;
        }
        if footer.total_len != session.len()
            || footer.preamble_digest != *sha256_exact(&preamble_bytes).as_bytes()
        {
            return Err(crypto_integrity(
                ReasonCode::FooterBindingMismatch,
                "encrypted footer length/preamble binding mismatch",
            ));
        }
        if footer.envelope_offset != crate::ecf::PREAMBLE_LEN
            || footer.envelope_offset.checked_add(footer.envelope_len)
                != Some(footer.segments_offset)
            || footer.segments_offset.checked_add(footer.segments_len) != Some(footer_offset)
        {
            return Err(segment_invalid(
                "encrypted section extents are not canonical",
            ));
        }
        if footer.envelope_len > options.crypto_policy.max_envelope_bytes {
            return Err(crypto_policy("CryptoEnvelope exceeds caller policy"));
        }
        let envelope_owner = read_envelope(
            &mut session,
            footer.envelope_offset,
            footer.envelope_len,
            &work,
        )?;
        let envelope = envelope_owner.view();
        if envelope.stanzas.len() as u32 > options.crypto_policy.max_stanzas {
            return Err(crypto_policy(
                "recipient stanza count exceeds caller policy",
            ));
        }
        for stanza in envelope.stanzas.iter() {
            if u64::try_from(stanza.exact_bytes().len()).unwrap_or(u64::MAX)
                > options.crypto_policy.max_stanza_bytes
            {
                return Err(crypto_policy("recipient stanza exceeds caller policy"));
            }
        }
        super::container::validate_envelope_policy(&envelope, preamble.features.incompat)?;
        let padding = PaddingMode::try_from(envelope.padding_mode)?;
        let boundary = BoundaryMode::try_from(envelope.boundary_mode)?;
        let context = public_crypto_context_work(
            &envelope.archive_id,
            preamble.features.incompat,
            padding,
            boundary,
            &work,
        )?;
        if !constant_time_eq(
            &Sha256::digest(context.as_slice()),
            &footer.public_context_digest,
        ) {
            return Err(crypto_integrity(
                ReasonCode::FooterBindingMismatch,
                "public crypto context digest mismatch",
            ));
        }
        let unlock = options.unlock.ok_or_else(super::no_recipient)?;
        let (afk, keys) = super::container::unlock_envelope(
            &envelope,
            preamble.features.incompat,
            unlock,
            &work,
        )?;
        drop(afk);
        let segments = walk_segments(&mut session, &footer, options, &work)?;
        let mut controls = ControlObjects::default();
        let mut control_private_bytes = 0_u64;
        for (ordinal, locator) in segments.as_slice().iter().enumerate() {
            if locator.class != SEGMENT_CONTROL {
                continue;
            }
            let object = decrypt_segment(
                &mut session,
                locator,
                u64::try_from(ordinal).unwrap_or(u64::MAX),
                &envelope,
                &keys,
                padding,
                options,
                AccessPurpose::EncryptedControl,
                &work,
            )?;
            control_private_bytes = control_private_bytes
                .checked_add(u64::try_from(object.exact_bytes().len()).unwrap_or(u64::MAX))
                .ok_or_else(|| crypto_policy("CONTROL private byte total overflow"))?;
            if control_private_bytes > options.crypto_policy.max_working_memory_bytes {
                return Err(crypto_policy(
                    "aggregate CONTROL private bytes exceed caller policy",
                ));
            }
            dispatch_control(&mut controls, object, preamble.features.incompat, &work)?;
            if ordinal == 0 {
                let (_, descriptor) = controls
                    .descriptor
                    .as_ref()
                    .ok_or_else(|| private_invalid("Descriptor is not the first private object"))?
                    .view()?;
                let body = decode_descriptor(descriptor)?;
                let corrected = preamble.features.incompat
                    & super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1
                    != 0;
                if corrected != body.declarations.is_some() {
                    return Err(private_invalid(
                        "Descriptor version and private-resource feature disagree",
                    ));
                }
                if let Some(declarations) = body.declarations {
                    crate::ecf::enforce_caller_policy(
                        declarations.budget,
                        options.resource_policy,
                    )?;
                    crate::ecf::enforce_decode_policy(declarations.decode, options.decode_policy)?;
                    crate::ecf::enforce_caller_policy(
                        declarations.budget,
                        session.policy().resource_policy,
                    )?;
                    crate::ecf::enforce_decode_policy(
                        declarations.decode,
                        session.policy().decode_policy,
                    )?;
                }
            }
        }
        let final_value = controls
            .final_value
            .ok_or_else(|| truncated("authenticated ArchiveFinal is missing"))?;
        validate_terminal_bindings(
            &controls,
            &final_value,
            &footer,
            &envelope,
            preamble.features.incompat,
            &context,
            segments.as_slice(),
        )?;
        let (_, descriptor_bytes) = controls
            .descriptor
            .as_ref()
            .ok_or_else(|| private_invalid("encrypted Descriptor is missing"))?
            .view()?;
        let descriptor_body = decode_descriptor(descriptor_bytes)?;
        let manifest_items = controls
            .manifest
            .as_ref()
            .ok_or_else(|| private_invalid("encrypted Manifest is missing"))?;
        let manifest_bytes = manifest_items.concatenate_work(&work)?;
        let (entries, content_objects) = decode_manifest(&manifest_bytes)?;
        drop(manifest_bytes);
        let (_, fidelity_bytes) = controls
            .fidelity
            .as_ref()
            .ok_or_else(|| private_invalid("encrypted Fidelity is missing"))?
            .view()?;
        let fidelity = decode_fidelity(fidelity_bytes)?;
        verify_metadata_lai(
            &entries,
            final_value.total_logical,
            Digest::from_bytes(final_value.lai),
        )?;
        verify_metadata_aux(&entries, &fidelity, Digest::from_bytes(final_value.aux))?;
        if final_value.entry_count != u64::try_from(entries.len()).unwrap_or(u64::MAX)
            || descriptor_body.lai != Digest::from_bytes(final_value.lai)
            || descriptor_body.pcr != Digest::from_bytes(final_value.pcr)
            || descriptor_body.aux != Digest::from_bytes(final_value.aux)
        {
            return Err(segment_invalid(
                "ArchiveFinal counts/identities disagree with authenticated metadata",
            ));
        }
        verify_manifest_references(&entries, &content_objects)?;
        let ordinary_features = FeatureSet {
            incompat: preamble.features.incompat & !super::CRYPTO_FEATURES,
            read_only_compat: preamble.features.read_only_compat,
            compat: preamble.features.compat,
        };
        let extended = has_cross_file_feature(ordinary_features);
        let reconstructive = has_reconstructive_feature(ordinary_features);
        let whole_object = has_whole_object_feature(ordinary_features);
        let plan_items = controls
            .plans
            .as_ref()
            .ok_or_else(|| private_invalid("encrypted TransformPlans are missing"))?;
        let plan_bytes = plan_items.concatenate_work(&work)?;
        let plans = if whole_object {
            decode_transform_plans_v3(&plan_bytes)?
        } else if reconstructive {
            decode_transform_plans_v2(&plan_bytes)?
        } else {
            decode_transform_plans(&plan_bytes, has_codec_transform_feature(ordinary_features))?
        };
        drop(plan_bytes);
        validate_plans(&plans)?;
        let groups = controls.groups.as_ref().map_or_else(
            || Ok(BTreeMap::new()),
            |items| decode_chunk_groups(&items.concatenate_work(&work)?),
        )?;
        let (budget_declared, mut budget, mut decode) = descriptor_body.declarations.map_or(
            (
                false,
                ResourceBudget::default(),
                crate::eam::DecodeRequirements::default(),
            ),
            |value| (true, value.budget, value.decode),
        );
        let legacy_whole_archive_verified = descriptor_body.declarations.is_none();
        if legacy_whole_archive_verified {
            // Descriptor v1 lacks authoritative bounds for unread payloads.
            // Verify the complete source under all caller-owned range limits,
            // with one charged source buffer and the existing unlocked keys.
            let length = session.len();
            let bytes = session.read_crypto(0, length, AccessPurpose::EncryptedPayload)?;
            let fully_opened = super::container::open_encrypted_with_unlocked_keys(
                &bytes, options, &envelope, &keys, &work,
            )?;
            budget = fully_opened.archive.descriptor.budget;
            decode = fully_opened.archive.descriptor.decode;
            session.check_stable()?;
        }
        let descriptor = ArchiveDescriptor {
            format_major: preamble.version.major,
            format_minor: preamble.version.minor,
            format_namespace: descriptor_body.namespace,
            features: ordinary_features,
            layout: Layout::Indexed,
            role: ArchiveRole::Complete,
            budget_declared,
            stream_dedup_window: 0,
            budget,
            decode,
            identity_profile: IdentityProfile::IdentityV1,
            digest_algorithm: DigestAlgorithm::Sha256,
            planner_id: descriptor_body.planner_id,
            chunker_id: descriptor_body.chunker_id,
            lai: descriptor_body.lai,
            pcr: descriptor_body.pcr,
            aux: descriptor_body.aux,
            pci: None,
        };
        let (index, index_status) = if controls.index_invalid {
            (
                ChunkDirectory::new(work.budget(), 0, true),
                RandomAccessIndexStatus::RebuiltInvalid,
            )
        } else if let Some(items) = controls.index {
            decode_index_or_rebuild(&items, segments.as_slice(), final_value.chunk_count, &work)?
        } else {
            (
                ChunkDirectory::new(work.budget(), 0, true),
                RandomAccessIndexStatus::RebuiltAbsent,
            )
        };
        let metadata = RandomAccessMetadata {
            descriptor,
            entries,
            content_objects,
            source_length: session.len(),
            source_revision: session.initial_revision().clone(),
            section_count: 2,
            section_directory: vec![
                RandomAccessSection {
                    kind: "CryptoEnvelope".to_owned(),
                    offset: footer.envelope_offset,
                    payload_length: footer.envelope_len - SECTION_HEADER_LEN,
                },
                RandomAccessSection {
                    kind: "EncryptedSegments".to_owned(),
                    offset: footer.segments_offset,
                    payload_length: footer.segments_len - SECTION_HEADER_LEN,
                },
            ]
            .into_boxed_slice(),
            encrypted_segment_count: Some(u64::try_from(segments.len()).unwrap_or(u64::MAX)),
            encrypted: true,
        };
        session.check_stable()?;
        Ok(Self {
            session,
            metadata,
            envelope: envelope_owner,
            keys,
            padding,
            work,
            legacy_whole_archive_verified,
            segments,
            index,
            index_status,
            plans,
            groups,
            support: None,
            extended,
            whole_object,
        })
    }

    #[must_use]
    pub fn metadata(&self) -> &RandomAccessMetadata {
        &self.metadata
    }

    pub fn metadata_report(&self) -> Result<RandomAccessVerificationReport> {
        self.session.check_stable()?;
        Ok(self.report(None, false, 0, 0, false, false, false))
    }

    pub fn read_entry(&mut self, path: &crate::eam::LogicalPath) -> Result<RandomAccessRead> {
        self.load_support_and_index_fallback()?;
        let entry = self
            .metadata
            .entries
            .entries()
            .iter()
            .find(|entry| entry.path() == path)
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::RandomAccessEntryNotFound,
                    path.to_string(),
                )
            })?;
        let content_id = match entry.data() {
            EntryData::File {
                content: ContentRef::Internal(value),
            } => *value,
            EntryData::Directory | EntryData::Symlink { .. } | EntryData::ReparsePoint { .. } => {
                return Err(Diagnostic::new(
                    OutcomeClass::Unsupported,
                    ReasonCode::RandomAccessEntryNotFile,
                    path.to_string(),
                ));
            }
        };
        let sparse_map = entry.metadata().sparse_map().cloned();
        let object = self
            .metadata
            .content_objects
            .get(&content_id)
            .cloned()
            .ok_or_else(|| dependency("Entry references unknown ContentObject"))?;
        let requested = object
            .chunks
            .iter()
            .map(|value| value.chunk_id)
            .collect::<BTreeSet<_>>();
        if u64::try_from(requested.len()).unwrap_or(u64::MAX)
            > self.session.policy().max_dependency_chunks
        {
            return Err(access_policy(
                "requested Chunk closure exceeds caller policy",
            ));
        }
        let mut decoded = BTreeMap::<Digest, Chunk>::new();
        let mut dependencies = 0_u64;
        let mut reconstruction_verified = false;
        for chunk_id in requested {
            let header = self.chunk_header(chunk_id)?;
            if header.region_owned {
                decoded.insert(
                    chunk_id,
                    Chunk {
                        chunk_id,
                        logical_len: header.logical_len,
                        plan_ref: header.plan_ref,
                        group_ref: None,
                        plaintext: Box::new([]),
                    },
                );
                continue;
            }
            let plan = self
                .plans
                .iter()
                .find(|plan| plan.plan_id == header.plan_ref)
                .ok_or_else(|| dependency(format!("unknown TransformPlan {}", header.plan_ref)))?;
            reconstruction_verified |= plan
                .transforms
                .iter()
                .any(|step| step.reconstruction_ref.is_some());
            let predecessors = self.group_prerequisites(chunk_id, &header)?;
            dependencies = dependencies
                .checked_add(u64::try_from(predecessors.len()).unwrap_or(u64::MAX))
                .ok_or_else(|| access_policy("dependency count overflow"))?;
            if dependencies > self.session.policy().max_dependency_chunks {
                return Err(access_policy("lookback closure exceeds caller policy"));
            }
            for predecessor in predecessors.iter() {
                self.decode_chunk(predecessor, &mut decoded)?;
            }
            // The former consumed Vec released its backing at the loop end.
            drop(predecessors);
            self.decode_chunk(chunk_id, &mut decoded)?;
        }
        if self.whole_object {
            let owned_plans = self.plans.to_vec();
            let regions = self
                .support
                .as_ref()
                .expect("support loaded")
                .regions
                .values()
                .filter(|value| value.content_object == content_id)
                .cloned()
                .collect::<Vec<_>>();
            let requested_region_bytes = regions.iter().try_fold(0_u64, |total, region| {
                total
                    .checked_add(region.logical_bytes)
                    .ok_or_else(|| access_policy("region logical byte total overflow"))
            })?;
            if requested_region_bytes > self.session.policy().max_decoded_logical_bytes {
                return Err(access_policy(
                    "aggregate region access exceeds caller decoded-byte policy",
                ));
            }
            for region in &regions {
                reconstruction_verified = true;
                if region.access.logical_bytes != region.logical_bytes
                    || region.access.logical_chunks != region.chunk_count
                    || region.access.worst_reconstructed_bytes != region.logical_bytes
                {
                    return Err(crypto_integrity(
                        ReasonCode::InvalidRegionAccess,
                        region.region_id.to_string(),
                    ));
                }
                let start = usize::try_from(region.start_chunk_index)
                    .map_err(|_| dependency("region start exceeds usize"))?;
                let end = start
                    .checked_add(
                        usize::try_from(region.chunk_count)
                            .map_err(|_| dependency("region count exceeds usize"))?,
                    )
                    .ok_or_else(|| dependency("region range overflow"))?;
                let members = object
                    .chunks
                    .get(start..end)
                    .ok_or_else(|| dependency("region exceeds ContentObject"))?;
                for member in members {
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        decoded.entry(member.chunk_id)
                    {
                        let header = self.chunk_header(member.chunk_id)?;
                        entry.insert(Chunk {
                            chunk_id: member.chunk_id,
                            logical_len: header.logical_len,
                            plan_ref: header.plan_ref,
                            group_ref: header.group_ref,
                            plaintext: Box::new([]),
                        });
                    }
                }
                let lengths = members
                    .iter()
                    .map(|member| Ok(decoded[&member.chunk_id].logical_len))
                    .collect::<Result<Vec<_>>>()?;
                let plans = owned_plans
                    .iter()
                    .map(|plan| (plan.plan_id, plan))
                    .collect::<BTreeMap<_, _>>();
                for (chunk_id, bytes) in
                    reconstruct_region_members(region, &object, &plans, &lengths)?
                {
                    decoded
                        .get_mut(&chunk_id)
                        .expect("declared member")
                        .plaintext = bytes.into_boxed_slice();
                }
            }
        }
        let mut output = Vec::new();
        let mut leaves = Vec::new();
        for reference in &object.chunks {
            let chunk = decoded
                .get(&reference.chunk_id)
                .ok_or_else(|| dependency("requested Chunk was not decoded"))?;
            if sha256_exact(&chunk.plaintext) != chunk.chunk_id {
                return Err(crypto_integrity(
                    ReasonCode::ChunkDigestMismatch,
                    chunk.chunk_id.to_string(),
                ));
            }
            leaves.push((chunk.chunk_id, chunk.logical_len));
            output.extend_from_slice(&chunk.plaintext);
            if u64::try_from(output.len()).unwrap_or(u64::MAX)
                > self.session.policy().max_decoded_logical_bytes
            {
                return Err(access_policy("decoded logical bytes exceed caller policy"));
            }
        }
        if chunk_root_from_leaves(&leaves) != object.chunk_root {
            return Err(crypto_integrity(
                ReasonCode::ChunkRootMismatch,
                object.logical_digest.to_string(),
            ));
        }
        if sha256_exact(&output) != object.logical_digest {
            return Err(crypto_integrity(
                ReasonCode::ContentDigestMismatch,
                object.logical_digest.to_string(),
            ));
        }
        if let Some(map) = sparse_map {
            map.validate_plaintext(&output)?;
        }
        self.session.check_stable()?;
        Ok(RandomAccessRead {
            bytes: output.into_boxed_slice(),
            report: self.report(
                Some(path.to_string()),
                true,
                u64::try_from(object.chunks.len()).unwrap_or(u64::MAX),
                dependencies,
                self.extended,
                self.extended,
                reconstruction_verified,
            ),
        })
    }

    fn load_support_and_index_fallback(&mut self) -> Result<()> {
        if self.support.is_some() {
            return Ok(());
        }
        let known_chunk_segments = KnownChunkSegments::new(self.work.budget(), &self.index)?;
        let mut dictionary_items = None;
        let mut reconstruction_items = None;
        let mut region_items = None;
        let bound = fallback_slot_bound(
            self.segments.as_slice(),
            self.session.policy().max_chunk_frames_scanned,
        )?;
        let mut discovered = ChunkDirectory::new(self.work.budget(), bound, false);
        let mut retained_private_bytes = 0_u64;
        for ordinal in 0..self.segments.len() {
            let ordinal_u64 = u64::try_from(ordinal).unwrap_or(u64::MAX);
            if self.segments.as_slice()[ordinal].class != SEGMENT_PAYLOAD {
                continue;
            }
            if self.index_status == RandomAccessIndexStatus::PresentValid
                && known_chunk_segments.contains(&ordinal_u64)
            {
                continue;
            }
            let locator = self.segments.as_slice()[ordinal];
            let resource_policy = self.session.policy().resource_policy;
            let decode_policy = self.session.policy().decode_policy;
            let envelope = self.envelope.view();
            let object = decrypt_segment(
                &mut self.session,
                &locator,
                ordinal_u64,
                &envelope,
                &self.keys,
                self.padding,
                EncryptedOpenOptions {
                    unlock: None,
                    crypto_policy: self.work.policy(),
                    resource_policy,
                    decode_policy,
                },
                AccessPurpose::EncryptedPayload,
                &self.work,
            )?;
            let (kind, payload) = object.view()?;
            match kind {
                wire::PRIVATE_OBJECT_CHUNK => {
                    discover_chunk(
                        &mut discovered,
                        payload,
                        self.session.policy().max_chunk_frames_scanned,
                        self.extended,
                        self.whole_object,
                        EncryptedChunkLocator {
                            segment_ordinal: ordinal_u64,
                            fragment_count: locator.count,
                        },
                    )?;
                    // Retain the authenticated locator, not decrypted Chunk
                    // bytes. A fallback scan must not create a plaintext cache
                    // that grows across otherwise bounded range reads.
                    if self.index_status == RandomAccessIndexStatus::PresentValid {
                        self.index_status = RandomAccessIndexStatus::RebuiltInvalid;
                    }
                }
                wire::PRIVATE_OBJECT_SEQUENCE => {
                    retained_private_bytes = retained_private_bytes
                        .checked_add(u64::try_from(payload.len()).unwrap_or(u64::MAX))
                        .ok_or_else(|| crypto_policy("PAYLOAD support byte total overflow"))?;
                    if retained_private_bytes > self.work.policy().max_working_memory_bytes {
                        return Err(crypto_policy(
                            "retained PAYLOAD support exceeds caller crypto memory policy",
                        ));
                    }
                    let items = wire::WorkSequence::decode_with_context(object, &self.work)?;
                    match items.kind() {
                        wire::COLLECTION_DICTIONARIES => {
                            set_once(&mut dictionary_items, items, "Dictionaries")?
                        }
                        wire::COLLECTION_RECONSTRUCTION_DATA => {
                            set_once(&mut reconstruction_items, items, "ReconstructionData")?
                        }
                        wire::COLLECTION_RECONSTRUCTION_REGIONS => {
                            set_once(&mut region_items, items, "ReconstructionRegions")?
                        }
                        _ => return Err(private_invalid("unexpected PAYLOAD collection")),
                    }
                }
                _ => return Err(private_invalid("unexpected PAYLOAD private object")),
            }
        }
        if self.index_status != RandomAccessIndexStatus::PresentValid {
            discovered.finish();
            self.index = discovered;
            if u64::try_from(self.index.len()).unwrap_or(u64::MAX)
                != self.metadata.descriptor.budget.chunk_count
                && self.metadata.descriptor.budget_declared
            {
                return Err(segment_invalid("rebuilt encrypted Chunk count mismatch"));
            }
        }
        let dictionaries = dictionary_items.map_or_else(
            || Ok(BTreeMap::new()),
            |items: wire::WorkSequence| decode_dictionaries(&items.concatenate_work(&self.work)?),
        )?;
        let reconstruction_data = reconstruction_items.map_or_else(
            || Ok(BTreeMap::new()),
            |items: wire::WorkSequence| {
                decode_reconstruction_section(&items.concatenate_work(&self.work)?)
                    .map(|value| value.0)
            },
        )?;
        let regions = region_items.map_or_else(
            || Ok(BTreeMap::new()),
            |items: wire::WorkSequence| {
                decode_reconstruction_regions(&items.concatenate_work(&self.work)?)
                    .map(|value| value.0)
            },
        )?;
        let actual_decode =
            aggregate_archive_decode_requirements(&self.plans, &dictionaries, &self.groups)?;
        if self.metadata.descriptor.budget_declared
            && actual_decode != self.metadata.descriptor.decode
        {
            return Err(private_invalid(
                "Descriptor decode declaration disagrees with authenticated dependencies",
            ));
        }
        crate::ecf::enforce_decode_policy(actual_decode, self.session.policy().decode_policy)?;
        self.support = Some(SupportObjects {
            dictionaries,
            reconstruction_data,
            regions,
        });
        Ok(())
    }

    fn chunk_frame(&mut self, chunk_id: Digest) -> Result<wire::WorkObject> {
        // A rebuilt Index gets one retry. Decrypted frames are caller-owned
        // temporaries and never retained by this archive between requests.
        for attempt in 0..2 {
            if let Some(frame) = self.indexed_chunk_frame(chunk_id)? {
                return Ok(frame);
            }
            if attempt == 1 {
                break;
            }
            self.index_status = RandomAccessIndexStatus::RebuiltInvalid;
            self.support = None;
            self.load_support_and_index_fallback()?;
        }
        Err(dependency(format!(
            "rebuilt Index lacks valid Chunk {chunk_id}"
        )))
    }

    fn indexed_chunk_frame(&mut self, chunk_id: Digest) -> Result<Option<wire::WorkObject>> {
        let locator = self
            .index
            .get(&chunk_id)
            .copied()
            .ok_or_else(|| dependency(format!("encrypted Index lacks Chunk {chunk_id}")))?;
        let segment_index = usize::try_from(locator.segment_ordinal)
            .map_err(|_| dependency("segment ordinal exceeds usize"))?;
        let segment = self
            .segments
            .as_slice()
            .get(segment_index)
            .copied()
            .ok_or_else(|| dependency("encrypted Index references absent segment"))?;
        if segment.class != SEGMENT_PAYLOAD || segment.count != locator.fragment_count {
            return Ok(None);
        }
        let resource_policy = self.session.policy().resource_policy;
        let decode_policy = self.session.policy().decode_policy;
        let envelope = self.envelope.view();
        let object = decrypt_segment(
            &mut self.session,
            &segment,
            locator.segment_ordinal,
            &envelope,
            &self.keys,
            self.padding,
            EncryptedOpenOptions {
                unlock: None,
                crypto_policy: self.work.policy(),
                resource_policy,
                decode_policy,
            },
            AccessPurpose::EncryptedPayload,
            &self.work,
        )?;
        let (kind, payload) = object.view()?;
        if kind != wire::PRIVATE_OBJECT_CHUNK {
            return Ok(None);
        }
        let header_len = usize::try_from(chunk_frame_header_len(self.extended))
            .map_err(|_| access_policy("Chunk header exceeds usize"))?;
        let header = parse_chunk_frame_header(
            payload
                .get(..header_len)
                .ok_or_else(|| truncated("encrypted Chunk frame is truncated"))?,
            self.extended,
            self.whole_object,
        )?;
        if header.chunk_id != chunk_id {
            return Ok(None);
        }
        Ok(Some(object))
    }

    fn chunk_header(&mut self, chunk_id: Digest) -> Result<ChunkFrameHeader> {
        let object = self.chunk_frame(chunk_id)?;
        let (_, frame) = object.view()?;
        let header_len = usize::try_from(chunk_frame_header_len(self.extended))
            .map_err(|_| access_policy("Chunk header exceeds usize"))?;
        let header =
            parse_chunk_frame_header(&frame[..header_len], self.extended, self.whole_object)?;
        let budget = if self.metadata.descriptor.budget_declared {
            self.metadata.descriptor.budget
        } else {
            self.session.policy().resource_policy
        };
        enforce_chunk_bounds(&header, budget)?;
        if frame.len()
            != header_len
                .checked_add(
                    usize::try_from(header.stored_len)
                        .map_err(|_| access_policy("Chunk stored length exceeds usize"))?,
                )
                .ok_or_else(|| dependency("Chunk frame extent overflow"))?
        {
            return Err(dependency("encrypted Chunk frame stored length mismatch"));
        }
        Ok(header)
    }

    fn group_prerequisites(
        &mut self,
        chunk_id: Digest,
        header: &ChunkFrameHeader,
    ) -> Result<PredecessorDigests> {
        let plan = self
            .plans
            .iter()
            .find(|plan| plan.plan_id == header.plan_ref)
            .ok_or_else(|| dependency("Chunk references unknown TransformPlan"))?;
        let PlanMode::Prefix { lookback } = plan_mode(plan)? else {
            return Ok(PredecessorDigests::empty(self.work.budget()));
        };
        let group_id = header
            .group_ref
            .ok_or_else(|| dependency("prefix-coded Chunk lacks group_ref"))?;
        let group = self
            .groups
            .get(&group_id)
            .ok_or_else(|| dependency("prefix-coded Chunk references unknown group"))?
            .clone();
        if group.max_lookback != lookback {
            return Err(dependency("group lookback and plan disagree"));
        }
        let physical = PhysicalPrerequisites::new(self.work.budget(), &self.index)?;
        let position = physical
            .as_slice()
            .iter()
            .position(|slot| slot.digest == *chunk_id.as_bytes())
            .ok_or_else(|| dependency("Chunk has no encrypted physical position"))?;
        let first = physical_prerequisite_first(position, lookback);
        // The whole physical window is charged before filtering or header reads.
        let mut predecessors =
            PredecessorDigests::reserve_empty(self.work.budget(), position - first)?;
        let mut logical_bytes = 0_u64;
        for slot in &physical.as_slice()[first..position] {
            let predecessor = Digest::from_bytes(slot.digest);
            let predecessor_header = self.chunk_header(predecessor)?;
            if predecessor_header.group_ref == Some(group_id) {
                logical_bytes =
                    prerequisite_logical_total(logical_bytes, predecessor_header.logical_len)?;
                predecessors.push(predecessor)?;
            }
        }
        if logical_bytes > group.max_preceding_bytes {
            return Err(crypto_integrity(
                ReasonCode::AccessCostMismatch,
                group_id.to_string(),
            ));
        }
        #[cfg(test)]
        predecessors.observe(PrerequisitePhase::Ready);
        Ok(predecessors)
    }

    fn decode_chunk(
        &mut self,
        chunk_id: Digest,
        decoded: &mut BTreeMap<Digest, Chunk>,
    ) -> Result<()> {
        if decoded.contains_key(&chunk_id) {
            return Ok(());
        }
        let object = self.chunk_frame(chunk_id)?;
        let (_, frame) = object.view()?;
        let header_len = usize::try_from(chunk_frame_header_len(self.extended))
            .map_err(|_| access_policy("Chunk header exceeds usize"))?;
        let header =
            parse_chunk_frame_header(&frame[..header_len], self.extended, self.whole_object)?;
        if header.region_owned {
            return Err(dependency(
                "region member has no independent representation",
            ));
        }
        let plan = self
            .plans
            .iter()
            .find(|plan| plan.plan_id == header.plan_ref)
            .ok_or_else(|| dependency("Chunk references unknown TransformPlan"))?
            .clone();
        let prefix = if let PlanMode::Prefix { lookback } = plan_mode(&plan)? {
            let predecessors = self.group_prerequisites(chunk_id, &header)?;
            for predecessor in predecessors.iter() {
                if !decoded.contains_key(&predecessor) {
                    self.decode_chunk(predecessor, decoded)?;
                }
            }
            let mut slices =
                PrefixSliceSlots::reserve_empty(self.work.budget(), predecessors.len())?;
            for digest in predecessors.iter() {
                slices.push(decoded[&digest].plaintext.as_ref())?;
            }
            #[cfg(test)]
            slices.observe(PrerequisitePhase::Ready);
            Some(physical_prefix_from_slices(slices.as_slice(), lookback)?)
        } else {
            None
        };
        let support = self.support.as_ref().expect("support metadata loaded");
        let plaintext = decode_frame_payload(
            &plan,
            &frame[header_len..],
            header.logical_len,
            &support.dictionaries,
            &support.reconstruction_data,
            prefix.as_deref(),
        )?;
        if sha256_exact(&plaintext) != chunk_id {
            return Err(crypto_integrity(
                ReasonCode::ChunkDigestMismatch,
                chunk_id.to_string(),
            ));
        }
        let retained = decoded.values().try_fold(0_u64, |total, chunk| {
            total
                .checked_add(u64::try_from(chunk.plaintext.len()).unwrap_or(u64::MAX))
                .ok_or_else(|| access_policy("decoded dependency byte total overflow"))
        })?;
        if retained.saturating_add(header.logical_len)
            > self.session.policy().max_decoded_logical_bytes
        {
            return Err(access_policy(
                "decoded dependency bytes exceed caller policy",
            ));
        }
        decoded.insert(
            chunk_id,
            Chunk {
                chunk_id,
                logical_len: header.logical_len,
                plan_ref: header.plan_ref,
                group_ref: header.group_ref,
                plaintext: plaintext.into_boxed_slice(),
            },
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn report(
        &self,
        requested_path: Option<String>,
        content_verified: bool,
        chunks: u64,
        dependencies: u64,
        dictionaries: bool,
        groups: bool,
        reconstruction: bool,
    ) -> RandomAccessVerificationReport {
        RandomAccessVerificationReport {
            source_revision_stable: true,
            preamble_footer_verified: true,
            section_structure_verified: true,
            semantic_metadata_sections_verified: true,
            index_status: self.index_status,
            requested_path,
            content_object_digest_verified: content_verified,
            chunk_count_verified: chunks,
            dependency_chunk_count: dependencies,
            dictionaries_verified: dictionaries,
            groups_verified: groups,
            reconstruction_verified: reconstruction,
            bytes_fetched: self.session.bytes_fetched(),
            range_request_count: self.session.range_requests(),
            lai: IdentityVerificationStatus::Verified,
            aux: IdentityVerificationStatus::Verified,
            pcr: if self.legacy_whole_archive_verified {
                IdentityVerificationStatus::Verified
            } else {
                IdentityVerificationStatus::DeclaredNotFullyVerified
            },
            pci: if self.legacy_whole_archive_verified {
                IdentityVerificationStatus::Verified
            } else {
                IdentityVerificationStatus::NotComputed
            },
            whole_archive_verified: self.legacy_whole_archive_verified,
            access_trace: self.session.trace().to_vec().into_boxed_slice(),
        }
    }
}

fn read_envelope(
    session: &mut RangeSession,
    offset: u64,
    extent: u64,
    work: &CryptoWorkContext,
) -> Result<wire::WorkEnvelope> {
    let crypto_limits = work.policy();
    if extent < SECTION_HEADER_LEN
        || extent > session.policy().max_metadata_bytes
        || extent > crypto_limits.max_envelope_bytes
    {
        return Err(crypto_policy("CryptoEnvelope extent exceeds caller policy"));
    }
    let header = session.read_crypto(offset, SECTION_HEADER_LEN, AccessPurpose::SectionHeader)?;
    validate_section_header(&header, 32)?;
    let payload_len = be_u64(&header[16..24])?;
    if SECTION_HEADER_LEN.checked_add(payload_len) != Some(extent) {
        return Err(segment_invalid(
            "CryptoEnvelope footer/header extent mismatch",
        ));
    }
    let payload = session.read_crypto(
        offset + SECTION_HEADER_LEN,
        payload_len,
        AccessPurpose::EncryptedControl,
    )?;
    if sha256_exact(&payload).as_bytes() != &header[24..56] {
        return Err(crypto_integrity(
            ReasonCode::SectionDigestMismatch,
            "CryptoEnvelope section digest mismatch",
        ));
    }
    wire::WorkEnvelope::decode_with_context(payload, work)
}

fn walk_segments(
    session: &mut RangeSession,
    footer: &Footer,
    options: EncryptedOpenOptions<'_>,
    work: &CryptoWorkContext,
) -> Result<SegmentTable> {
    let section = session.read_crypto(
        footer.segments_offset,
        SECTION_HEADER_LEN,
        AccessPurpose::SectionHeader,
    )?;
    validate_section_header(&section, 33)?;
    let payload_len = be_u64(&section[16..24])?;
    if SECTION_HEADER_LEN.checked_add(payload_len) != Some(footer.segments_len) {
        return Err(segment_invalid("ENCRYPTED_SEGMENTS extent mismatch"));
    }
    let end = footer
        .segments_offset
        .checked_add(footer.segments_len)
        .ok_or_else(|| segment_invalid("ENCRYPTED_SEGMENTS extent overflow"))?;
    let mut cursor = footer.segments_offset + SECTION_HEADER_LEN;
    let mut locators = SegmentTable::new(work.budget());
    while cursor < end {
        let ordinal = u64::try_from(locators.len()).unwrap_or(u64::MAX);
        if ordinal >= options.crypto_policy.max_segments
            || ordinal >= session.policy().max_encrypted_segment_header_walk
        {
            return Err(access_policy(
                "encrypted SegmentHeader walk exceeds caller policy",
            ));
        }
        let bytes =
            session.read_crypto(cursor, SEGMENT_HEADER_LEN, AccessPurpose::SectionHeader)?;
        let header: [u8; 64] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| truncated("SegmentHeader is truncated"))?;
        let (class, salt, count, extent) = parse_segment_header(&header, ordinal)?;
        if count > options.crypto_policy.max_messages_per_segment
            || extent > options.crypto_policy.max_working_memory_bytes
        {
            return Err(crypto_policy(
                "encrypted segment declaration exceeds caller policy",
            ));
        }
        let vacant = locators
            .probe(class, &salt)
            .ok_or_else(|| segment_invalid("segment salt is reused in one key domain"))?;
        let next = cursor
            .checked_add(extent)
            .ok_or_else(|| segment_invalid("segment extent overflow"))?;
        if next > end {
            return Err(truncated("segment extent exceeds ENCRYPTED_SEGMENTS"));
        }
        locators.append_unique(
            SegmentLocator {
                offset: cursor,
                extent,
                class,
                salt,
                count,
                header,
                left: NO_SEGMENT_SLOT,
                right: NO_SEGMENT_SLOT,
                parent: NO_SEGMENT_SLOT,
                height: 1,
            },
            vacant,
        )?;
        cursor = next;
    }
    if cursor != end || locators.len() == 0 {
        return Err(truncated("encrypted segment sequence is incomplete"));
    }
    let terminal = locators.as_slice().last().expect("not empty");
    if terminal.offset != footer.terminal_offset
        || footer.archive_final_offset != terminal.offset + SEGMENT_HEADER_LEN
        || terminal.class != SEGMENT_CONTROL
        || terminal.count != 1
    {
        return Err(segment_invalid(
            "terminal segment/footer locators are invalid",
        ));
    }
    Ok(locators)
}

fn reserve_segment_object(
    fragment: &wire::PrivateFragmentRef<'_>,
    segment_extent: u64,
    work: &CryptoWorkContext,
) -> Result<WorkBytes> {
    if fragment.total_len < u64::try_from(fragment.bytes.len()).unwrap_or(u64::MAX)
        || u64::from(fragment.count) > fragment.total_len
    {
        return Err(private_invalid(
            "fragment total cannot contain its declared bytes and count",
        ));
    }
    if fragment.total_len > segment_extent
        || fragment.total_len > work.policy().max_working_memory_bytes
    {
        return Err(crypto_policy(
            "encrypted object exceeds physical extent or caller crypto memory policy",
        ));
    }
    WorkBytes::zeroed(
        work.budget(),
        usize::try_from(fragment.total_len)
            .map_err(|_| crypto_policy("encrypted object exceeds usize"))?,
    )
}

#[allow(clippy::too_many_arguments)]
fn decrypt_segment(
    session: &mut RangeSession,
    locator: &SegmentLocator,
    ordinal: u64,
    envelope: &wire::EnvelopeRef<'_>,
    keys: &KeyHierarchy,
    padding: PaddingMode,
    options: EncryptedOpenOptions<'_>,
    purpose: AccessPurpose,
    work: &CryptoWorkContext,
) -> Result<wire::WorkObject> {
    preflight_protected_records(session, locator, ordinal, options)?;
    // Crypto source/cache/trace buffers and complete assembled objects share
    // the caller's original counter. AEAD/AD/transcript temporaries and the
    // other engine bookkeeping still need their separate ownership conversion.
    let bytes = session.read_crypto(locator.offset, locator.extent, purpose)?;
    if bytes.get(..64) != Some(locator.header.as_slice()) {
        return Err(segment_invalid(
            "fetched SegmentHeader changed within the session",
        ));
    }
    let key = super::container::segment_key(
        keys,
        &envelope.archive_id,
        locator.class,
        ordinal,
        &locator.salt,
    )?;
    let mut cursor = usize::try_from(SEGMENT_HEADER_LEN).unwrap_or(64);
    let mut private_total = 0_u64;
    let mut ciphertext_total = 0_u64;
    let mut partial_id = None;
    let mut partial_total = 0_u64;
    let mut partial_count = 0_u32;
    let mut partial_next = 0_u32;
    let mut object = None;
    let mut written = 0_usize;
    for counter in 0..locator.count {
        let (protected, ciphertext, next) = parse_protected(
            &bytes,
            cursor,
            RECORD_DATA,
            ordinal,
            u64::from(counter),
            options,
        )?;
        let private = decrypt_private_record(
            &key.0,
            &envelope.archive_id,
            &locator.header,
            protected,
            ciphertext,
            locator.class,
            padding,
            false,
        )?;
        if u64::try_from(private.len()).unwrap_or(u64::MAX)
            > options.crypto_policy.max_private_record_bytes
        {
            return Err(crypto_policy(
                "protected private record exceeds caller policy",
            ));
        }
        private_total = private_total
            .checked_add(u64::try_from(private.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| segment_invalid("private byte total overflow"))?;
        ciphertext_total = ciphertext_total
            .checked_add(u64::try_from(ciphertext.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| segment_invalid("ciphertext byte total overflow"))?;
        let fragment = wire::decode_private_fragment_ref(&private)?;
        if counter == 0 {
            if fragment.index != 0 || fragment.offset != 0 {
                return Err(private_invalid("first object fragment is not initial"));
            }
            partial_id = Some(fragment.object_id);
            partial_total = fragment.total_len;
            partial_count = fragment.count;
            object = Some(reserve_segment_object(&fragment, locator.extent, work)?);
        }
        if Some(fragment.object_id) != partial_id
            || fragment.total_len != partial_total
            || fragment.count != partial_count
            || fragment.index != partial_next
            || fragment.offset != u64::try_from(written).unwrap_or(u64::MAX)
        {
            return Err(private_invalid(
                "private fragments are not contiguous and exact",
            ));
        }
        let next_written = written
            .checked_add(fragment.bytes.len())
            .ok_or_else(|| private_invalid("private fragment extent overflows"))?;
        let destination = object
            .as_mut()
            .and_then(|bytes: &mut WorkBytes| bytes.get_mut(written..next_written))
            .ok_or_else(|| {
                private_invalid("private fragment exceeds its declared object extent")
            })?;
        destination.copy_from_slice(fragment.bytes);
        written = next_written;
        partial_next = partial_next
            .checked_add(1)
            .ok_or_else(|| private_invalid("private fragment count overflows"))?;
        cursor = next;
    }
    let data_end = cursor;
    if partial_next != partial_count || u64::try_from(written).unwrap_or(u64::MAX) != partial_total
    {
        return Err(truncated("encrypted object fragments are incomplete"));
    }
    let (end_header, end_ciphertext, next) = parse_protected(
        &bytes,
        cursor,
        RECORD_END,
        ordinal,
        u64::from(locator.count),
        options,
    )?;
    if next != bytes.len() {
        return Err(segment_invalid("bytes follow authenticated segment END"));
    }
    let end_private = decrypt_private_record(
        &key.0,
        &envelope.archive_id,
        &locator.header,
        end_header,
        end_ciphertext,
        locator.class,
        padding,
        true,
    )?;
    if u64::try_from(end_private.len()).unwrap_or(u64::MAX)
        > options.crypto_policy.max_private_record_bytes
    {
        return Err(crypto_policy(
            "segment END private record exceeds caller policy",
        ));
    }
    let data_digest = segment_data_digest(&locator.header, locator.count, &bytes[64..data_end])?;
    validate_segment_end(
        &end_private,
        ordinal,
        locator.class,
        locator.count,
        private_total,
        ciphertext_total,
        &data_digest,
    )?;
    let object_id = partial_id.ok_or_else(|| truncated("segment has no private object"))?;
    wire::WorkObject::verified(
        object.ok_or_else(|| truncated("segment has no private object"))?,
        object_id,
        work,
    )
}

fn dispatch_control(
    controls: &mut ControlObjects,
    object: wire::WorkObject,
    features: u64,
    work: &CryptoWorkContext,
) -> Result<()> {
    let object_id = object.object_id();
    let (kind, payload) = object.view()?;
    match kind {
        wire::PRIVATE_OBJECT_RECORD => match wire::record_kind(payload)? {
            1 => {
                set_once(&mut controls.descriptor, object, "Descriptor")?;
                controls.descriptor_id = Some(object_id);
            }
            5 => set_once(&mut controls.fidelity, object, "Fidelity")?,
            wire::RECORD_ARCHIVE_FINAL => {
                set_once(
                    &mut controls.final_value,
                    decode_archive_final(payload)?,
                    "ArchiveFinal",
                )?;
            }
            _ => return Err(private_invalid("forbidden singleton CONTROL object")),
        },
        wire::PRIVATE_OBJECT_SEQUENCE => {
            let peek_index = payload.len() >= 8
                && &payload[..4] == b"EBCS"
                && u16::from_be_bytes(payload[6..8].try_into().unwrap()) == wire::COLLECTION_INDEX;
            let decoded = wire::WorkSequence::decode_with_context(object, work);
            let items = match decoded {
                Ok(value) => value,
                Err(error) if error.class() == OutcomeClass::PolicyRefused => return Err(error),
                Err(_) if peek_index => {
                    controls.index_invalid = true;
                    return Ok(());
                }
                Err(error) => return Err(error),
            };
            match items.kind() {
                wire::COLLECTION_TRANSFORM_PLANS => {
                    set_once(&mut controls.plans, items, "TransformPlans")?
                }
                wire::COLLECTION_CHUNK_GROUPS => {
                    set_once(&mut controls.groups, items, "ChunkGroups")?
                }
                wire::COLLECTION_MANIFEST => {
                    set_once(&mut controls.manifest, items, "Manifest")?;
                    controls.manifest_id = Some(object_id);
                }
                wire::COLLECTION_INDEX => set_once(&mut controls.index, items, "Index")?,
                wire::COLLECTION_RECIPIENT_DIRECTORY | wire::COLLECTION_SIGNATURES => {}
                _ => {
                    return Err(private_invalid(
                        "PAYLOAD collection appeared in CONTROL segment",
                    ));
                }
            }
        }
        _ => return Err(private_invalid("Chunk object appeared in CONTROL segment")),
    }
    if features & super::FEATURE_SIGNATURE_ED25519_V1 == 0 {
        // Signature collection parsing above remains authenticated but is not
        // needed for random file retrieval.
    }
    Ok(())
}

fn validate_terminal_bindings(
    controls: &ControlObjects,
    final_value: &ArchiveFinal,
    footer: &Footer,
    envelope: &wire::EnvelopeRef<'_>,
    features: u64,
    public_context: &[u8],
    segments: &[SegmentLocator],
) -> Result<()> {
    if final_value.segment_count != u64::try_from(segments.len()).unwrap_or(u64::MAX)
        || controls.descriptor_id != Some(final_value.descriptor_id)
        || controls.manifest_id != Some(final_value.manifest_id)
    {
        return Err(segment_invalid(
            "ArchiveFinal object/count binding mismatch",
        ));
    }
    let recipient_set = super::t1_digest(
        "entrybound/recipient-set/v1",
        &[envelope.stanzas.exact_bytes()],
    )?;
    if !constant_time_eq(&recipient_set, &final_value.recipient_set) {
        return Err(crypto_integrity(
            ReasonCode::CryptoEnvelopeAuthFailed,
            "ArchiveFinal recipient-set binding mismatch",
        ));
    }
    let context_digest: [u8; 32] = Sha256::digest(public_context).into();
    let core = wire::t1(
        "entrybound/encrypted-footer-core/v1",
        &[
            &2_u16.to_be_bytes(),
            &footer.total_len.to_be_bytes(),
            &footer.envelope_offset.to_be_bytes(),
            &footer.envelope_len.to_be_bytes(),
            &footer.segments_offset.to_be_bytes(),
            &footer.segments_len.to_be_bytes(),
            &footer.terminal_offset.to_be_bytes(),
            &footer.archive_final_offset.to_be_bytes(),
            &footer.preamble_digest,
            &context_digest,
        ],
    )?;
    if !constant_time_eq(&Sha256::digest(core), &final_value.footer_core) {
        return Err(crypto_integrity(
            ReasonCode::FooterBindingMismatch,
            "authenticated footer-core mismatch",
        ));
    }
    if features & super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1 == 0 {
        // Accepted only as the explicitly supported pre-correction form.
    }
    Ok(())
}

fn decode_index_or_rebuild(
    items: &wire::WorkSequence,
    segments: &[SegmentLocator],
    expected_count: u64,
    work: &CryptoWorkContext,
) -> Result<(ChunkDirectory, RandomAccessIndexStatus)> {
    match decode_encrypted_index(items, segments, expected_count, work) {
        Ok(value) => Ok((value, RandomAccessIndexStatus::PresentValid)),
        Err(error) if error.class() == OutcomeClass::PolicyRefused => Err(error),
        Err(_) => Ok((
            ChunkDirectory::new(work.budget(), 0, true),
            RandomAccessIndexStatus::RebuiltInvalid,
        )),
    }
}

fn decode_encrypted_index(
    items: &wire::WorkSequence,
    segments: &[SegmentLocator],
    expected_count: u64,
    work: &CryptoWorkContext,
) -> Result<ChunkDirectory> {
    let mut output = ChunkDirectory::new(work.budget(), items.len(), true);
    output.reserve_once()?;
    for item in items.items() {
        decode_encrypted_index_item(item, segments, &mut output)?;
    }
    if u64::try_from(output.len()).unwrap_or(u64::MAX) != expected_count {
        return Err(private_invalid("encrypted Index Chunk count mismatch"));
    }
    Ok(output)
}

fn decode_encrypted_index_item(
    item: &[u8],
    segments: &[SegmentLocator],
    output: &mut ChunkDirectory,
) -> Result<()> {
    let (record, consumed) = decode_record(item)?;
    if consumed != item.len() || record.kind != wire::RECORD_ENCRYPTED_INDEX {
        return Err(private_invalid("encrypted Index contains a wrong record"));
    }
    record.expect_tags(&[1, 2, 3, 4], &[])?;
    let digest = Digest::from_bytes(
        record
            .field(1)?
            .as_bytes()?
            .try_into()
            .map_err(|_| private_invalid("encrypted Index digest length"))?,
    );
    let ordinal = record.field(2)?.as_u64()?;
    let first_counter = record.field(3)?.as_u64()?;
    let fragments = record.field(4)?.as_u32()?;
    let segment = segments
        .get(usize::try_from(ordinal).map_err(|_| private_invalid("Index ordinal exceeds usize"))?)
        .ok_or_else(|| private_invalid("Index references absent segment"))?;
    if first_counter != 0
        || fragments == 0
        || segment.class != SEGMENT_PAYLOAD
        || segment.count != fragments
        || output.append_index(
            digest,
            EncryptedChunkLocator {
                segment_ordinal: ordinal,
                fragment_count: fragments,
            },
        )?
    {
        return Err(private_invalid(
            "encrypted Index locator is invalid or duplicate",
        ));
    }
    Ok(())
}

fn parse_footer(bytes: &[u8], source_len: u64) -> Result<Footer> {
    if bytes.len() != usize::try_from(ENCRYPTED_FOOTER_LEN).unwrap_or(192)
        || &bytes[..8] != b"\x8eEBF\r\n\x1a\n"
    {
        return Err(truncated("encrypted footer magic is missing"));
    }
    if be_u16(&bytes[8..10])? != 2
        || be_u16(&bytes[10..12])? != 192
        || bytes[12..16].iter().any(|byte| *byte != 0)
        || bytes[168..].iter().any(|byte| *byte != 0)
    {
        return Err(private_invalid("encrypted footer is noncanonical"));
    }
    let total_len = be_u64(&bytes[16..24])?;
    if total_len != source_len {
        return Err(Diagnostic::new(
            if total_len > source_len {
                OutcomeClass::Truncated
            } else {
                OutcomeClass::Corrupt
            },
            ReasonCode::IncorrectTotalLength,
            "encrypted footer total length mismatch",
        ));
    }
    Ok(Footer {
        total_len,
        envelope_offset: be_u64(&bytes[24..32])?,
        envelope_len: be_u64(&bytes[32..40])?,
        segments_offset: be_u64(&bytes[40..48])?,
        segments_len: be_u64(&bytes[48..56])?,
        terminal_offset: be_u64(&bytes[56..64])?,
        archive_final_offset: be_u64(&bytes[64..72])?,
        preamble_digest: bytes[72..104].try_into().unwrap(),
        public_context_digest: bytes[136..168].try_into().unwrap(),
    })
}

fn validate_section_header(bytes: &[u8], kind: u16) -> Result<()> {
    if bytes.len() != 64
        || &bytes[..4] != b"EBS1"
        || be_u16(&bytes[4..6])? != kind
        || be_u16(&bytes[6..8])? != 1
        || bytes[8..16].iter().any(|byte| *byte != 0)
        || bytes[56..64].iter().any(|byte| *byte != 0)
    {
        return Err(segment_invalid("encrypted section header is noncanonical"));
    }
    Ok(())
}

fn parse_segment_header(header: &[u8; 64], ordinal: u64) -> Result<(u8, [u8; 16], u32, u64)> {
    if &header[..4] != b"EBSG"
        || be_u16(&header[4..6])? != 1
        || !matches!(header[6], SEGMENT_CONTROL | SEGMENT_PAYLOAD)
        || header[7] != 0
        || be_u64(&header[8..16])? != ordinal
        || header[36..48].iter().any(|byte| *byte != 0)
        || header[56..64].iter().any(|byte| *byte != 0)
    {
        return Err(segment_invalid(
            "SegmentHeader is noncanonical or reordered",
        ));
    }
    let count = u32::from_be_bytes(header[32..36].try_into().unwrap());
    let extent = be_u64(&header[48..56])?;
    if count > (1 << 20) - 1 || extent < SEGMENT_HEADER_LEN + PROTECTED_HEADER_LEN + 16 {
        return Err(segment_invalid("SegmentHeader count/extent is invalid"));
    }
    Ok((header[6], header[16..32].try_into().unwrap(), count, extent))
}

fn parse_protected<'a>(
    segment: &'a [u8],
    cursor: usize,
    class: u8,
    ordinal: u64,
    counter: u64,
    options: EncryptedOpenOptions<'_>,
) -> Result<(&'a [u8], &'a [u8], usize)> {
    let header_end = cursor
        .checked_add(usize::try_from(PROTECTED_HEADER_LEN).unwrap_or(32))
        .ok_or_else(|| segment_invalid("protected header offset overflow"))?;
    let header = segment
        .get(cursor..header_end)
        .ok_or_else(|| truncated("protected header is truncated"))?;
    let len = protected_ciphertext_len(header, class, ordinal, counter, options)?;
    let end = header_end
        .checked_add(usize::try_from(len).map_err(|_| crypto_policy("ciphertext exceeds usize"))?)
        .ok_or_else(|| segment_invalid("ciphertext extent overflow"))?;
    let ciphertext = segment
        .get(header_end..end)
        .ok_or_else(|| truncated("protected ciphertext is truncated"))?;
    Ok((header, ciphertext, end))
}

fn protected_ciphertext_len(
    header: &[u8],
    class: u8,
    ordinal: u64,
    counter: u64,
    options: EncryptedOpenOptions<'_>,
) -> Result<u64> {
    if header.len() != PROTECTED_HEADER_LEN as usize
        || &header[..4] != b"EBC1"
        || be_u16(&header[4..6])? != 1
        || header[6] != class
        || header[7] != 0
        || be_u64(&header[8..16])? != ordinal
        || be_u64(&header[16..24])? != counter
    {
        return Err(segment_invalid(
            "protected record is reordered or noncanonical",
        ));
    }
    let len = be_u64(&header[24..32])?;
    if len < 16 || len > options.crypto_policy.max_ciphertext_record_bytes {
        return Err(crypto_policy("protected ciphertext exceeds caller policy"));
    }
    Ok(len)
}

/// Validate caller-lowerable record extents using only the public headers.
/// This must precede the whole-segment fetch, allocation, and AEAD work.
fn preflight_protected_records(
    session: &mut RangeSession,
    locator: &SegmentLocator,
    ordinal: u64,
    options: EncryptedOpenOptions<'_>,
) -> Result<()> {
    let end = locator
        .offset
        .checked_add(locator.extent)
        .ok_or_else(|| segment_invalid("segment extent overflow"))?;
    let mut cursor = locator
        .offset
        .checked_add(SEGMENT_HEADER_LEN)
        .ok_or_else(|| segment_invalid("protected record offset overflow"))?;
    for counter in 0..=u64::from(locator.count) {
        let header_end = cursor
            .checked_add(PROTECTED_HEADER_LEN)
            .filter(|value| *value <= end)
            .ok_or_else(|| truncated("protected header exceeds segment extent"))?;
        let header =
            session.read_crypto(cursor, PROTECTED_HEADER_LEN, AccessPurpose::SectionHeader)?;
        let class = if counter == u64::from(locator.count) {
            RECORD_END
        } else {
            RECORD_DATA
        };
        let len = protected_ciphertext_len(&header, class, ordinal, counter, options)?;
        cursor = header_end
            .checked_add(len)
            .filter(|value| *value <= end)
            .ok_or_else(|| truncated("protected ciphertext exceeds segment extent"))?;
    }
    if cursor != end {
        return Err(segment_invalid("bytes follow segment END record"));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn decrypt_private_record(
    key: &[u8; 32],
    archive_id: &[u8; 32],
    segment_header: &[u8],
    protected: &[u8],
    ciphertext: &[u8],
    segment_class: u8,
    padding: PaddingMode,
    end: bool,
) -> Result<Vec<u8>> {
    let counter = be_u64(&protected[16..24])?;
    let nonce = if end {
        super::container::end_nonce(counter)
    } else {
        super::container::data_nonce(counter)
    };
    let plaintext = aead_open(
        key,
        &nonce,
        &super::container::record_ad(archive_id, segment_header, protected)?,
        ciphertext,
    )?;
    if plaintext.len() < 8 {
        return Err(private_invalid("authenticated private record is too short"));
    }
    let private_len = usize::try_from(be_u64(&plaintext[..8])?)
        .map_err(|_| crypto_policy("private length exceeds usize"))?;
    let private_end = 8_usize
        .checked_add(private_len)
        .ok_or_else(|| crypto_policy("private length overflow"))?;
    let private = plaintext
        .get(8..private_end)
        .ok_or_else(|| private_invalid("private length exceeds authenticated plaintext"))?;
    let class = if end { SEGMENT_CONTROL } else { segment_class };
    if plaintext.len() != padded_len(8 + private_len, class, padding)? {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::CryptoPaddingInvalid,
            "authenticated padding is noncanonical",
        ));
    }
    Ok(private.to_vec())
}

fn padded_len(unpadded: usize, class: u8, mode: PaddingMode) -> Result<usize> {
    let maximum = if class == SEGMENT_CONTROL {
        1 << 20
    } else {
        64 << 20
    };
    if unpadded > maximum {
        return Err(crypto_policy("private record exceeds class capacity"));
    }
    match mode {
        PaddingMode::None => Ok(unpadded),
        PaddingMode::Maximum => Ok(maximum),
        PaddingMode::Bucketed => {
            let (first, last) = if class == SEGMENT_CONTROL {
                (8_u32, 20_u32)
            } else {
                (12_u32, 26_u32)
            };
            let mut buckets = BTreeSet::new();
            for power in first..=last {
                buckets.insert(1_usize << power);
                buckets.insert(5 * (1_usize << (power - 2)));
                buckets.insert(3 * (1_usize << (power - 1)));
                buckets.insert(7 * (1_usize << (power - 2)));
            }
            buckets
                .into_iter()
                .filter(|value| *value >= 1_usize << first && *value <= 1_usize << last)
                .find(|value| *value >= unpadded)
                .ok_or_else(|| crypto_policy("private record has no padding bucket"))
        }
    }
}

fn segment_data_digest(header: &[u8; 64], count: u32, exact_records: &[u8]) -> Result<[u8; 32]> {
    // The first pass sizes the exact existing sequence field; neither pass
    // owns a transcript. Production has already authenticated these records.
    let mut sequence_len = 8_u64;
    let mut cursor = 0_usize;
    while cursor < exact_records.len() {
        let header_end = cursor
            .checked_add(32)
            .ok_or_else(|| segment_invalid("DATA transcript extent overflow"))?;
        let protected = exact_records
            .get(cursor..header_end)
            .ok_or_else(|| segment_invalid("DATA transcript header is truncated"))?;
        let cipher_len = usize::try_from(be_u64(&protected[24..32])?)
            .map_err(|_| crypto_policy("DATA transcript length exceeds usize"))?;
        let extent = 32_usize
            .checked_add(cipher_len)
            .ok_or_else(|| segment_invalid("DATA transcript extent overflow"))?;
        let end = cursor
            .checked_add(extent)
            .ok_or_else(|| segment_invalid("DATA transcript extent overflow"))?;
        exact_records
            .get(cursor..end)
            .ok_or_else(|| segment_invalid("DATA transcript record is truncated"))?;
        let extent = u64::try_from(extent)
            .map_err(|_| crypto_policy("DATA transcript length exceeds u64"))?;
        sequence_len = sequence_len
            .checked_add(8)
            .and_then(|length| length.checked_add(extent))
            .ok_or_else(|| segment_invalid("DATA transcript length overflow"))?;
        cursor = end;
    }

    let mut digest = Sha256::new();
    digest.update(26_u16.to_be_bytes());
    digest.update(b"entrybound/segment-data/v1");
    digest.update(2_u16.to_be_bytes());
    digest.update(1_u16.to_be_bytes());
    digest.update(64_u64.to_be_bytes());
    digest.update(header);
    digest.update(2_u16.to_be_bytes());
    digest.update(sequence_len.to_be_bytes());
    digest.update(u64::from(count).to_be_bytes());
    cursor = 0;
    while cursor < exact_records.len() {
        let header_end = cursor
            .checked_add(32)
            .ok_or_else(|| segment_invalid("DATA transcript extent overflow"))?;
        let protected = exact_records
            .get(cursor..header_end)
            .ok_or_else(|| segment_invalid("DATA transcript header is truncated"))?;
        let cipher_len = usize::try_from(be_u64(&protected[24..32])?)
            .map_err(|_| crypto_policy("DATA transcript length exceeds usize"))?;
        let extent = 32_usize
            .checked_add(cipher_len)
            .ok_or_else(|| segment_invalid("DATA transcript extent overflow"))?;
        let end = cursor
            .checked_add(extent)
            .ok_or_else(|| segment_invalid("DATA transcript extent overflow"))?;
        let record = exact_records
            .get(cursor..end)
            .ok_or_else(|| segment_invalid("DATA transcript record is truncated"))?;
        digest.update(
            u64::try_from(extent)
                .map_err(|_| crypto_policy("DATA transcript length exceeds u64"))?
                .to_be_bytes(),
        );
        digest.update(record);
        cursor = end;
    }
    Ok(digest.finalize().into())
}

#[cfg(test)]
fn data_sequence(count: u32, exact_records: &[u8]) -> Result<Vec<u8>> {
    let mut output = u64::from(count).to_be_bytes().to_vec();
    let mut cursor = 0_usize;
    while cursor < exact_records.len() {
        let header = exact_records
            .get(cursor..cursor + 32)
            .ok_or_else(|| segment_invalid("DATA transcript header is truncated"))?;
        let cipher_len = usize::try_from(be_u64(&header[24..32])?)
            .map_err(|_| crypto_policy("DATA transcript length exceeds usize"))?;
        let extent = 32_usize
            .checked_add(cipher_len)
            .ok_or_else(|| segment_invalid("DATA transcript extent overflow"))?;
        let record = exact_records
            .get(cursor..cursor + extent)
            .ok_or_else(|| segment_invalid("DATA transcript record is truncated"))?;
        output.extend_from_slice(&u64::try_from(extent).unwrap_or(u64::MAX).to_be_bytes());
        output.extend_from_slice(record);
        cursor += extent;
    }
    Ok(output)
}

fn validate_segment_end(
    bytes: &[u8],
    ordinal: u64,
    class: u8,
    count: u32,
    private_bytes: u64,
    ciphertext_bytes: u64,
    digest: &[u8; 32],
) -> Result<()> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != wire::RECORD_SEGMENT_END {
        return Err(segment_invalid("END plaintext is not SegmentEndV1"));
    }
    record.expect_tags(&[1, 2, 3, 4, 5, 6], &[])?;
    if record.field(1)?.as_u64()? != ordinal
        || record.field(2)?.as_u8()? != class
        || record.field(3)?.as_u32()? != count
        || record.field(4)?.as_u64()? != private_bytes
        || record.field(5)?.as_u64()? != ciphertext_bytes
        || !constant_time_eq(record.field(6)?.as_bytes()?, digest)
    {
        return Err(segment_invalid("SegmentEnd totals/digest mismatch"));
    }
    Ok(())
}

fn decode_archive_final(bytes: &[u8]) -> Result<ArchiveFinal> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != wire::RECORD_ARCHIVE_FINAL {
        return Err(segment_invalid("terminal object is not ArchiveFinalV1"));
    }
    record.expect_tags(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], &[])?;
    Ok(ArchiveFinal {
        segment_count: record.field(1)?.as_u64()?,
        entry_count: record.field(2)?.as_u64()?,
        total_logical: record.field(3)?.as_u64()?,
        chunk_count: record.field(4)?.as_u64()?,
        lai: exact(record.field(5)?.as_bytes()?)?,
        pcr: exact(record.field(6)?.as_bytes()?)?,
        aux: exact(record.field(7)?.as_bytes()?)?,
        recipient_set: exact(record.field(8)?.as_bytes()?)?,
        footer_core: exact(record.field(10)?.as_bytes()?)?,
        descriptor_id: exact(record.field(11)?.as_bytes()?)?,
        manifest_id: exact(record.field(12)?.as_bytes()?)?,
    })
}

fn verify_manifest_references(
    entries: &EntrySet,
    objects: &BTreeMap<Digest, ContentObject>,
) -> Result<()> {
    for entry in entries.entries() {
        if let EntryData::File {
            content: ContentRef::Internal(digest),
        } = entry.data()
            && !objects.contains_key(digest)
        {
            return Err(dependency(format!(
                "Entry references unknown ContentObject {digest}"
            )));
        }
    }
    Ok(())
}

fn set_once<T>(slot: &mut Option<T>, value: T, name: &str) -> Result<()> {
    if slot.replace(value).is_some() {
        return Err(private_invalid(format!(
            "duplicate encrypted {name} object"
        )));
    }
    Ok(())
}

fn be_u16(bytes: &[u8]) -> Result<u16> {
    Ok(u16::from_be_bytes(
        bytes
            .try_into()
            .map_err(|_| segment_invalid("expected u16"))?,
    ))
}

fn be_u64(bytes: &[u8]) -> Result<u64> {
    Ok(u64::from_be_bytes(
        bytes
            .try_into()
            .map_err(|_| segment_invalid("expected u64"))?,
    ))
}

fn exact<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| private_invalid(format!("expected {N} bytes")))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && bool::from(left.ct_eq(right))
}

fn segment_invalid(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::CryptoSegmentStructureInvalid,
        detail,
    )
}

fn private_invalid(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::CryptoPrivateObjectInvalid,
        detail,
    )
}

fn crypto_integrity(code: ReasonCode, detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, code, detail)
}

fn crypto_policy(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::CryptoResourcePolicyRefused,
        detail,
    )
}

fn access_policy(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::RandomAccessPolicyRefused,
        detail,
    )
}

fn dependency(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::RandomAccessDependencyInvalid,
        detail,
    )
}

fn truncated(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Truncated, ReasonCode::TruncatedStream, detail)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

    use super::{PROTECTED_HEADER_LEN, SEGMENT_HEADER_LEN, open_indexed_random_encrypted};
    use crate::archive::{PackOptions, plan_directory};
    use crate::crypto::{
        EncryptedOpenOptions, EncryptedWriteOptions, PaddingMode, Unlock, XWingIdentity,
        encrypt_archive,
    };
    use crate::eam::{ContentRef, EntryData, LogicalPath};
    use crate::random_access::{
        MemoryRandomReadSource, RandomAccessPolicy, RandomReadSource, SourceRevision,
    };

    type PrerequisiteEvent = (
        super::PrerequisiteKind,
        super::PrerequisitePhase,
        usize,
        usize,
        u64,
    );

    #[derive(Default)]
    struct PrerequisiteState {
        events: Vec<PrerequisiteEvent>,
        active: std::collections::BTreeMap<(u8, usize), u64>,
        pressure_kind: Option<super::PrerequisiteKind>,
        pressure_occurrence: usize,
        matching_reserves: usize,
        short: bool,
        held: Option<super::WorkBytes>,
        budget: Option<super::WorkBudget>,
        hit: bool,
        release_reserved: bool,
        nested_predecessor_charge: u64,
        reads: Vec<(u64, u64)>,
        corrupt_at: Option<u64>,
        requested_read: Option<(u64, u64)>,
        requested_hits: usize,
        outer_drop_observed: bool,
    }

    std::thread_local! {
        static PREREQUISITE_STATE: std::cell::RefCell<Option<Arc<Mutex<PrerequisiteState>>>> =
            const { std::cell::RefCell::new(None) };
    }

    fn prerequisite_charge(kind: super::PrerequisiteKind, bound: usize) -> u64 {
        match kind {
            super::PrerequisiteKind::Physical => {
                super::PhysicalPrerequisites::reservation_bytes(bound).unwrap()
            }
            super::PrerequisiteKind::Predecessors => {
                super::PredecessorDigests::reservation_bytes(bound).unwrap()
            }
            super::PrerequisiteKind::Slices => {
                super::PrefixSliceSlots::reservation_bytes(bound).unwrap()
            }
        }
    }

    fn observe_prerequisites(
        kind: super::PrerequisiteKind,
        phase: super::PrerequisitePhase,
        budget: &super::WorkBudget,
        bound: usize,
        slots: super::PrerequisiteSlots<'_>,
    ) {
        use super::{PrerequisiteKind as K, PrerequisitePhase as P, PrerequisiteSlots as S};
        let (id, len, pointer, cleared) = match slots {
            S::Physical(v) => (
                0,
                v.len(),
                v.as_ptr() as usize,
                v.iter().all(|s| s.ordinal == 0 && s.digest == [0; 32]),
            ),
            S::Predecessors(v) => (
                1,
                v.len(),
                v.as_ptr() as usize,
                v.iter().all(|s| *s == [0; 32]),
            ),
            S::Slices(v) => (
                2,
                v.len(),
                v.as_ptr() as usize,
                v.iter().all(|s| s.is_empty()),
            ),
        };
        PREREQUISITE_STATE.with(|cell| {
            let shared = cell.borrow().as_ref().unwrap().clone();
            let mut state = shared.lock().unwrap_or_else(|e| e.into_inner());
            state.budget = Some(budget.clone());
            assert!(len <= bound);
            if phase == P::BeforeReserve {
                assert_eq!(len, 0);
                if kind == K::Physical {
                    state.nested_predecessor_charge = state.nested_predecessor_charge.max(
                        state
                            .active
                            .iter()
                            .filter(|((k, _), _)| *k == 1)
                            .map(|(_, bytes)| bytes)
                            .sum(),
                    );
                }
                if state.pressure_kind == Some(kind) && bound != 0 {
                    state.matching_reserves += 1;
                    if state.matching_reserves == state.pressure_occurrence {
                        let leave = prerequisite_charge(kind, bound) - u64::from(state.short);
                        let hold = budget.available_bytes().checked_sub(leave).unwrap();
                        let pressure = super::WorkBytes::zeroed(budget, hold as usize).unwrap();
                        assert!(pressure.belongs_to(budget));
                        state.held = Some(pressure);
                        state.hit = true;
                        assert_eq!(budget.available_bytes(), leave);
                    }
                }
            }
            if phase == P::Reserved && bound != 0 {
                assert!(
                    state
                        .active
                        .insert((id, pointer), prerequisite_charge(kind, bound))
                        .is_none()
                );
                if state.pressure_kind == Some(kind)
                    && state.hit
                    && state.release_reserved
                    && state.held.is_some()
                {
                    assert_eq!(budget.available_bytes(), 0);
                    drop(state.held.take());
                }
            }
            if phase == P::Ready && kind != K::Predecessors {
                assert_eq!(len, bound);
            }
            if phase == P::BeforeFree {
                assert!(
                    cleared,
                    "only initialized fields/reference slots are inspected"
                );
                // The observer runs before WorkVec backing free and lease release.
                let owners_live: u64 = state.active.values().sum();
                assert!(budget.live_bytes() >= owners_live);
                state.active.remove(&(id, pointer));
            }
            state
                .events
                .push((kind, phase, bound, len, budget.live_bytes()));
        });
    }

    struct PrerequisiteObserverGuard {
        previous: Option<super::PrerequisiteObserver>,
        previous_state: Option<Arc<Mutex<PrerequisiteState>>>,
    }

    impl PrerequisiteObserverGuard {
        fn install(state: &Arc<Mutex<PrerequisiteState>>) -> Self {
            Self {
                previous: super::PREREQUISITE_OBSERVER
                    .with(|c| c.replace(Some(observe_prerequisites))),
                previous_state: PREREQUISITE_STATE.with(|c| c.replace(Some(Arc::clone(state)))),
            }
        }
    }

    impl Drop for PrerequisiteObserverGuard {
        fn drop(&mut self) {
            super::PREREQUISITE_OBSERVER.with(|c| c.set(self.previous));
            PREREQUISITE_STATE.with(|c| {
                c.replace(self.previous_state.take());
            });
        }
    }

    struct PrerequisiteSource {
        source: MemoryRandomReadSource,
        state: Arc<Mutex<PrerequisiteState>>,
    }

    impl RandomReadSource for PrerequisiteSource {
        fn len(&self) -> crate::diagnostics::Result<u64> {
            self.source.len()
        }
        fn revision(&self) -> crate::diagnostics::Result<SourceRevision> {
            self.source.revision()
        }
        fn read_exact_at(&self, _: u64, _: u64) -> crate::diagnostics::Result<Vec<u8>> {
            panic!("prerequisite fixtures require bounded source fills")
        }
        fn read_exact_at_into(
            &self,
            offset: u64,
            dst: &mut [u8],
        ) -> crate::diagnostics::Result<()> {
            self.source.read_exact_at_into(offset, dst)?;
            let mut state = self.state.lock().unwrap();
            state.reads.push((offset, dst.len() as u64));
            if state.requested_read == Some((offset, dst.len() as u64)) {
                state.requested_hits += 1;
                if state.requested_hits == 2 {
                    assert!(
                        state.active.keys().all(|(kind, _)| *kind != 1),
                        "outer predecessor backing must be freed before requested frame decode"
                    );
                    state.outer_drop_observed = true;
                }
            }
            if let Some(byte) = state.corrupt_at
                && let Some(relative) = byte.checked_sub(offset)
                && relative < dst.len() as u64
            {
                dst[relative as usize] ^= 1;
            }
            Ok(())
        }
    }

    struct PrerequisiteFixture {
        bytes: Vec<u8>,
        identity: XWingIdentity,
        target: super::Digest,
        plaintexts: std::collections::BTreeMap<super::Digest, Vec<u8>>,
        physical_order: Vec<super::Digest>,
        target_path: LogicalPath,
        target_entry_bytes: Vec<u8>,
    }

    fn prerequisite_fixture(include_index: bool) -> PrerequisiteFixture {
        // Same proven Dense lookback input shape as the retained recursive test.
        let directory = TestDir::new();
        let mut state = 0xbb67_ae85_84ca_a73b_u64;
        let base = (0..64 * 1024)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect::<Vec<_>>();
        for index in 0..12 {
            let mut bytes = base.clone();
            let first = 128 + (index * 977) % (bytes.len() - 512);
            for (offset, byte) in bytes[first..first + 256].iter_mut().enumerate() {
                *byte = (index as u8).wrapping_mul(31).wrapping_add(offset as u8);
            }
            std::fs::write(
                directory.path().join(format!("prefix-{index:03}.bin")),
                bytes,
            )
            .unwrap();
        }
        let archive = plan_directory(
            directory.path(),
            PackOptions {
                profile: crate::planner::CompressionProfile::Dense,
                ..PackOptions::default()
            },
        )
        .unwrap();
        let target = archive
            .content_store
            .physical_order
            .iter()
            .rev()
            .find(|id| archive.content_store.chunks[id].group_ref.is_some())
            .copied()
            .expect("Dense fixture must select real prefix-coded chunks");
        let plaintexts = archive
            .content_store
            .chunks
            .iter()
            .map(|(id, chunk)| (*id, chunk.plaintext.to_vec()))
            .collect();
        let physical_order = archive.content_store.physical_order.to_vec();
        let entry = archive
            .entry_set
            .entries()
            .iter()
            .find(|entry| {
                if let EntryData::File {
                    content: ContentRef::Internal(id),
                    ..
                } = entry.data()
                {
                    archive.content_store.objects[id]
                        .chunks
                        .iter()
                        .any(|part| part.chunk_id == target)
                } else {
                    false
                }
            })
            .unwrap();
        let target_path = entry.path().clone();
        let EntryData::File {
            content: ContentRef::Internal(content_id),
            ..
        } = entry.data()
        else {
            unreachable!()
        };
        let target_entry_bytes = archive.content_store.objects[content_id]
            .chunks
            .iter()
            .flat_map(|part| {
                archive.content_store.chunks[&part.chunk_id]
                    .plaintext
                    .iter()
                    .copied()
            })
            .collect();
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: &[recipient],
                padding: PaddingMode::None,
                include_index,
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        PrerequisiteFixture {
            bytes: encrypted.bytes,
            identity,
            target,
            plaintexts,
            physical_order,
            target_path,
            target_entry_bytes,
        }
    }

    fn prerequisite_open(
        fixture: &PrerequisiteFixture,
        state: &Arc<Mutex<PrerequisiteState>>,
    ) -> super::EncryptedRandomAccessArchive {
        let mut options = EncryptedOpenOptions::new(Some(Unlock::Identity(&fixture.identity)));
        options.crypto_policy.max_working_memory_bytes = SEGMENT_FINITE_LIMIT;
        let mut reader = open_indexed_random_encrypted(
            PrerequisiteSource {
                source: MemoryRandomReadSource::new(fixture.bytes.clone()),
                state: Arc::clone(state),
            },
            segment_access_policy(),
            options,
        )
        .unwrap();
        reader.load_support_and_index_fallback().unwrap();
        reader
    }

    fn prerequisite_original_order(
        reader: &super::EncryptedRandomAccessArchive,
    ) -> Vec<(u64, super::Digest)> {
        let mut physical = reader
            .index
            .iter()
            .map(|(digest, locator)| (locator.segment_ordinal, super::Digest::from_bytes(*digest)))
            .collect::<Vec<_>>();
        physical.sort();
        physical
    }

    // Test-only original tuple/window/Vec algorithm. Its source fetches and
    // ordinary decode calls are an independent oracle for the replacement.
    fn original_prerequisite_decode(
        reader: &mut super::EncryptedRandomAccessArchive,
        id: super::Digest,
        decoded: &mut super::BTreeMap<super::Digest, super::Chunk>,
    ) {
        if decoded.contains_key(&id) {
            return;
        }
        let object = reader.chunk_frame(id).unwrap();
        let (_, frame) = object.view().unwrap();
        let header_len = super::chunk_frame_header_len(reader.extended) as usize;
        let header = super::parse_chunk_frame_header(
            &frame[..header_len],
            reader.extended,
            reader.whole_object,
        )
        .unwrap();
        assert!(!header.region_owned);
        let plan = reader
            .plans
            .iter()
            .find(|p| p.plan_id == header.plan_ref)
            .unwrap()
            .clone();
        let prefix = if let super::PlanMode::Prefix { lookback } = super::plan_mode(&plan).unwrap()
        {
            let group_id = header.group_ref.unwrap();
            let group = reader.groups[&group_id].clone();
            assert_eq!(group.max_lookback, lookback);
            let physical = prerequisite_original_order(reader);
            let position = physical.iter().position(|(_, d)| *d == id).unwrap();
            // Original saturating conversion, before filtering any group.
            let first = position.saturating_sub(usize::try_from(lookback).unwrap_or(usize::MAX));
            let mut predecessors = Vec::new();
            let mut logical = 0_u64;
            for (_, predecessor) in &physical[first..position] {
                let h = reader.chunk_header(*predecessor).unwrap();
                if h.group_ref == Some(group_id) {
                    logical = logical.checked_add(h.logical_len).unwrap();
                    predecessors.push(*predecessor);
                }
            }
            assert!(logical <= group.max_preceding_bytes);
            for predecessor in &predecessors {
                if !decoded.contains_key(predecessor) {
                    original_prerequisite_decode(reader, *predecessor, decoded);
                }
            }
            let slices = predecessors
                .iter()
                .map(|d| decoded[d].plaintext.as_ref())
                .collect::<Vec<_>>();
            Some(super::physical_prefix_from_slices(&slices, lookback).unwrap())
        } else {
            None
        };
        let support = reader.support.as_ref().unwrap();
        let plaintext = super::decode_frame_payload(
            &plan,
            &frame[header_len..],
            header.logical_len,
            &support.dictionaries,
            &support.reconstruction_data,
            prefix.as_deref(),
        )
        .unwrap();
        assert_eq!(super::sha256_exact(&plaintext), id);
        decoded.insert(
            id,
            super::Chunk {
                chunk_id: id,
                logical_len: header.logical_len,
                plan_ref: header.plan_ref,
                group_ref: header.group_ref,
                plaintext: plaintext.into_boxed_slice(),
            },
        );
    }

    fn prerequisite_teardown(
        reader: super::EncryptedRandomAccessArchive,
        state: &Arc<Mutex<PrerequisiteState>>,
    ) {
        let budget = reader.work.budget().clone();
        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
        assert!(state.active.is_empty());
        drop(state.held.take());
        drop(state);
        drop(reader);
        assert_eq!(budget.live_bytes(), 0);
    }

    fn authenticated_prerequisite_test_segment(
        ordinal: u64,
        fragments: &[super::wire::PrivateFragmentRef<'_>],
        archive_id: &[u8; 32],
        keys: &super::KeyHierarchy,
        wrong_end: bool,
    ) -> (Vec<u8>, super::SegmentLocator) {
        use crate::canonical::RecordBuilder;
        use sha2::Digest as _;

        let count = u32::try_from(fragments.len()).unwrap();
        let class = super::SEGMENT_PAYLOAD;
        let mut salt = [9; 16];
        salt[8..].copy_from_slice(&ordinal.to_be_bytes());
        let private = fragments
            .iter()
            .map(|fragment| {
                super::wire::encode_private_fragment(
                    &fragment.object_id,
                    fragment.total_len,
                    fragment.index,
                    fragment.count,
                    fragment.offset,
                    fragment.bytes,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let private_total = private.iter().map(|value| value.len() as u64).sum::<u64>();
        let ciphertext_total = private
            .iter()
            .map(|value| (8 + value.len() + 16) as u64)
            .sum::<u64>();
        let end_record = |digest: &[u8; 32]| {
            let mut record = RecordBuilder::new(super::wire::RECORD_SEGMENT_END);
            record
                .u64(1, ordinal)
                .unwrap()
                .u8(2, class)
                .unwrap()
                .u32(3, count)
                .unwrap()
                .u64(4, private_total)
                .unwrap()
                .u64(5, ciphertext_total)
                .unwrap()
                .bytes(6, digest)
                .unwrap();
            record.finish().unwrap()
        };
        let extent = 64
            + u64::from(count) * 32
            + ciphertext_total
            + 32
            + 8
            + end_record(&[0; 32]).len() as u64
            + 16;
        let mut header = [0; 64];
        header[..4].copy_from_slice(b"EBSG");
        header[4..6].copy_from_slice(&1_u16.to_be_bytes());
        header[6] = class;
        header[8..16].copy_from_slice(&ordinal.to_be_bytes());
        header[16..32].copy_from_slice(&salt);
        header[32..36].copy_from_slice(&count.to_be_bytes());
        header[48..56].copy_from_slice(&extent.to_be_bytes());
        assert_eq!(
            super::parse_segment_header(&header, ordinal).unwrap(),
            (class, salt, count, extent)
        );
        let key =
            super::super::container::segment_key(keys, archive_id, class, ordinal, &salt).unwrap();
        let seal = |class: u8, counter: u64, value: &[u8], nonce: &[u8; 12]| {
            let mut protected = [0; 32];
            protected[..4].copy_from_slice(b"EBC1");
            protected[4..6].copy_from_slice(&1_u16.to_be_bytes());
            protected[6] = class;
            protected[8..16].copy_from_slice(&ordinal.to_be_bytes());
            protected[16..24].copy_from_slice(&counter.to_be_bytes());
            protected[24..32].copy_from_slice(&((8 + value.len() + 16) as u64).to_be_bytes());
            let mut padded = (value.len() as u64).to_be_bytes().to_vec();
            padded.extend_from_slice(value);
            let ciphertext = super::super::aead_seal(
                &key.0,
                nonce,
                &super::super::container::record_ad(archive_id, &header, &protected).unwrap(),
                &padded,
            )
            .unwrap();
            let mut record = protected.to_vec();
            record.extend_from_slice(&ciphertext);
            record
        };
        let mut data = Vec::new();
        for (index, value) in private.iter().enumerate() {
            let counter = index as u64;
            data.extend_from_slice(&seal(
                super::RECORD_DATA,
                counter,
                value,
                &super::super::container::data_nonce(counter),
            ));
        }
        let mut digest: [u8; 32] = super::Sha256::digest(
            super::wire::t1(
                "entrybound/segment-data/v1",
                &[&header, &super::data_sequence(count, &data).unwrap()],
            )
            .unwrap(),
        )
        .into();
        if wrong_end {
            digest[0] ^= 1;
        }
        let mut segment = header.to_vec();
        segment.extend_from_slice(&data);
        segment.extend_from_slice(&seal(
            super::RECORD_END,
            u64::from(count),
            &end_record(&digest),
            &super::super::container::end_nonce(u64::from(count)),
        ));
        assert_eq!(segment.len() as u64, extent);
        (
            segment,
            super::SegmentLocator {
                offset: 0,
                extent,
                class,
                salt,
                count,
                header,
                left: super::NO_SEGMENT_SLOT,
                right: super::NO_SEGMENT_SLOT,
                parent: super::NO_SEGMENT_SLOT,
                height: 1,
            },
        )
    }

    #[test]
    fn physical_prerequisite_heap_matches_original_tuple_positions_without_dedup() {
        let inputs = [
            vec![],
            vec![(0, [0; 32])],
            vec![(u64::MAX, [255; 32]), (0, [0; 32]), (u64::MAX, [0; 32])],
            (0..257)
                .map(|i| (i % 7, *slot_digest(i).as_bytes()))
                .collect(),
            (0..257)
                .rev()
                .map(|i| (i % 7, *slot_digest(i).as_bytes()))
                .collect(),
            vec![(4, [8; 32]), (4, [7; 32]), (4, [8; 32]), (0, [255; 32])],
        ];
        for input in inputs {
            let charge = super::PhysicalPrerequisites::reservation_bytes(input.len()).unwrap();
            let work = test_work(charge);
            let mut owner =
                super::PhysicalPrerequisites::reserve_empty(work.budget(), input.len()).unwrap();
            let pointer = owner.as_slice().as_ptr();
            for &(ordinal, digest) in &input {
                owner.push(ordinal, digest).unwrap();
            }
            let mut oracle = input
                .iter()
                .map(|(o, d)| (*o, super::Digest::from_bytes(*d)))
                .collect::<Vec<_>>();
            oracle.sort();
            owner.sort();
            assert_eq!(owner.as_slice().as_ptr(), pointer);
            assert_eq!(
                owner
                    .as_slice()
                    .iter()
                    .map(|s| (s.ordinal, super::Digest::from_bytes(s.digest)))
                    .collect::<Vec<_>>(),
                oracle
            );
            assert_eq!(owner.as_slice().len(), input.len());
            for (_, id) in &oracle {
                assert_eq!(
                    owner
                        .as_slice()
                        .iter()
                        .position(|s| s.digest == *id.as_bytes()),
                    oracle.iter().position(|(_, d)| d == id)
                );
            }
            let absent = [0x51; 32];
            assert_eq!(
                owner.as_slice().iter().position(|s| s.digest == absent),
                oracle.iter().position(|(_, d)| *d.as_bytes() == absent)
            );
            assert_eq!(work.budget().live_bytes(), charge);
            assert_eq!(work.budget().available_bytes(), 0);
            drop(owner);
            assert_eq!(work.budget().live_bytes(), 0);
        }
    }

    #[test]
    fn prerequisite_fixed_layout_capacity_spares_and_original_counter_are_exact() {
        use super::PrerequisiteKind as K;
        for bound in [0, 1, 4, 17] {
            for kind in [K::Physical, K::Predecessors, K::Slices] {
                let expected = match kind {
                    K::Physical => {
                        std::alloc::Layout::array::<super::PhysicalPrerequisiteSlot>(bound)
                            .unwrap()
                            .size()
                    }
                    K::Predecessors => std::alloc::Layout::array::<[u8; 32]>(bound).unwrap().size(),
                    K::Slices => std::alloc::Layout::array::<&[u8]>(bound).unwrap().size(),
                } as u64;
                assert_eq!(prerequisite_charge(kind, bound), expected);
            }
            let charge = prerequisite_charge(K::Physical, bound)
                + prerequisite_charge(K::Predecessors, bound)
                + prerequisite_charge(K::Slices, bound);
            let work = test_work(charge + 13);
            let caller = super::WorkBytes::zeroed(work.budget(), 13).unwrap();
            let mut physical =
                super::PhysicalPrerequisites::reserve_empty(work.budget(), bound).unwrap();
            let mut predecessors =
                super::PredecessorDigests::reserve_empty(work.budget(), bound).unwrap();
            let mut slices = super::PrefixSliceSlots::reserve_empty(work.budget(), bound).unwrap();
            let pointers = (
                physical.as_slice().as_ptr(),
                predecessors.slots.as_slice().as_ptr(),
                slices.as_slice().as_ptr(),
            );
            for i in 0..bound {
                physical.push(i as u64, [7; 32]).unwrap();
                // Fill only one matching digest: every unused physical B slot stays charged.
                if i == 0 {
                    predecessors.push(slot_digest(7)).unwrap();
                }
                slices.push(b"ordinary plaintext").unwrap();
            }
            assert_eq!(work.budget().live_bytes(), charge + 13);
            assert_eq!(work.budget().available_bytes(), 0);
            assert_eq!(
                pointers,
                (
                    physical.as_slice().as_ptr(),
                    predecessors.slots.as_slice().as_ptr(),
                    slices.as_slice().as_ptr()
                )
            );
            assert!(physical.push(0, [1; 32]).is_err());
            assert!(slices.push(b"overflow").is_err());
            while predecessors.len() < bound {
                predecessors.push(slot_digest(8)).unwrap();
            }
            assert!(predecessors.push(slot_digest(9)).is_err());
            drop(slices);
            drop(predecessors);
            drop(physical);
            assert_eq!(work.budget().live_bytes(), 13);
            drop(caller);
            assert_eq!(work.budget().live_bytes(), 0);
        }
        for bytes in [
            super::PhysicalPrerequisites::reservation_bytes(usize::MAX),
            super::PredecessorDigests::reservation_bytes(usize::MAX),
            super::PrefixSliceSlots::reservation_bytes(usize::MAX),
        ] {
            let e = bytes.unwrap_err();
            assert_eq!(e.class(), super::OutcomeClass::PolicyRefused);
            assert_eq!(e.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        }
        let work = test_work(31);
        assert!(super::PredecessorDigests::reserve_empty(work.budget(), 1).is_err());
        assert_eq!(work.budget().live_bytes(), 0);
    }

    #[test]
    fn prerequisite_window_before_group_filter_preserves_order_and_full_physical_bound() {
        for position in [0, 1, 2, 17, usize::MAX] {
            for lookback in [0, 1, 8, u32::MAX] {
                let expected =
                    position.saturating_sub(usize::try_from(lookback).unwrap_or(usize::MAX));
                assert_eq!(
                    super::physical_prerequisite_first(position, lookback),
                    expected
                );
            }
        }
        let fixture = prerequisite_fixture(true);
        let state = Arc::new(Mutex::new(PrerequisiteState::default()));
        let mut reader = prerequisite_open(&fixture, &state);
        let physical = prerequisite_original_order(&reader);
        assert_eq!(
            physical.iter().map(|(_, id)| *id).collect::<Vec<_>>(),
            fixture.physical_order
        );
        let header = reader.chunk_header(fixture.target).unwrap();
        let plan = reader
            .plans
            .iter()
            .find(|p| p.plan_id == header.plan_ref)
            .unwrap();
        let super::PlanMode::Prefix { lookback } = super::plan_mode(plan).unwrap() else {
            panic!("real prefix plan")
        };
        let position = physical
            .iter()
            .position(|(_, d)| *d == fixture.target)
            .unwrap();
        let first = super::physical_prerequisite_first(position, lookback);
        let mut oracle = Vec::new();
        let mut expected_reads = Vec::new();
        for (_, id) in &physical[first..position] {
            state.lock().unwrap().reads.clear();
            let h = reader.chunk_header(*id).unwrap();
            expected_reads.extend_from_slice(&state.lock().unwrap().reads);
            if h.group_ref == header.group_ref {
                oracle.push(*id);
            }
        }
        state.lock().unwrap().reads.clear();
        {
            let _observer = PrerequisiteObserverGuard::install(&state);
            let predecessors = reader.group_prerequisites(fixture.target, &header).unwrap();
            assert_eq!(predecessors.bound, position - first);
            assert_eq!(predecessors.iter().collect::<Vec<_>>(), oracle);
            assert_eq!(state.lock().unwrap().reads, expected_reads);
            drop(predecessors);
            // A declared group with no matching authenticated candidate headers
            // exercises N=0/B>0 without changing the physical window first.
            let group_id = slot_digest(u64::MAX);
            let mut other_header = header;
            other_header.group_ref = Some(group_id);
            reader.groups.insert(
                group_id,
                super::ChunkGroup {
                    group_id,
                    max_lookback: lookback,
                    max_preceding_bytes: u64::MAX,
                },
            );
            let empty = reader
                .group_prerequisites(fixture.target, &other_header)
                .unwrap();
            assert_eq!(empty.len(), 0);
            assert_eq!(empty.bound, position - first);
            assert!(empty.bound > 0);
            assert!(state.lock().unwrap().events.iter().any(|e| e.0
                == super::PrerequisiteKind::Predecessors
                && e.1 == super::PrerequisitePhase::Ready
                && e.2 > 0
                && e.3 == 0));
        }
        prerequisite_teardown(reader, &state);
        // Finite actual-key authenticated candidate frames deliberately alternate
        // groups. The requested header/plan is a helper declaration; this proves
        // physical-window selection, not publication of a changed whole archive.
        let state = Arc::new(Mutex::new(PrerequisiteState::default()));
        let mut reader = prerequisite_open(&fixture, &state);
        assert!(reader.extended);
        let mut requested = reader.chunk_header(fixture.target).unwrap();
        let plan = crate::codec::zstd_prefix_plan(5, 4).unwrap();
        requested.plan_ref = plan.plan_id;
        reader.plans = vec![plan].into_boxed_slice();
        let matching_group = requested.group_ref.unwrap();
        let other_group = slot_digest(u64::MAX);
        assert_ne!(matching_group, other_group);
        reader.groups.insert(
            matching_group,
            super::ChunkGroup {
                group_id: matching_group,
                max_lookback: 4,
                max_preceding_bytes: u64::MAX,
            },
        );
        let candidate_ids = prerequisite_original_order(&reader)
            .into_iter()
            .map(|(_, id)| id)
            .filter(|id| *id != fixture.target)
            .take(4)
            .collect::<Vec<_>>();
        assert_eq!(candidate_ids.len(), 4);
        let mut bytes = Vec::new();
        let mut segments = super::SegmentTable::new(reader.work.budget());
        for (ordinal, id) in candidate_ids.iter().enumerate() {
            let mut frame = {
                let object = reader.chunk_frame(*id).unwrap();
                object.view().unwrap().1.to_vec()
            };
            let group = if ordinal % 2 == 0 {
                matching_group
            } else {
                other_group
            };
            frame[64..96].copy_from_slice(group.as_bytes());
            let raw =
                super::wire::private_object(super::wire::PRIVATE_OBJECT_CHUNK, &frame).unwrap();
            let fragment = super::wire::PrivateFragmentRef {
                object_id: super::wire::encrypted_object_id(&raw).unwrap(),
                total_len: raw.len() as u64,
                index: 0,
                count: 1,
                offset: 0,
                bytes: &raw,
            };
            let (segment, mut locator) = authenticated_prerequisite_test_segment(
                ordinal as u64,
                &[fragment],
                &reader.envelope.view().archive_id,
                &reader.keys,
                false,
            );
            locator.offset = bytes.len() as u64;
            append_segment(&mut segments, locator);
            bytes.extend_from_slice(&segment);
        }
        reader.session = super::RangeSession::new_crypto(
            Box::new(PrerequisiteSource {
                source: MemoryRandomReadSource::new(bytes),
                state: Arc::clone(&state),
            }),
            segment_access_policy(),
            reader.work.budget().clone(),
        )
        .unwrap();
        reader.segments = segments;
        let mut entries = candidate_ids
            .iter()
            .enumerate()
            .map(|(ordinal, id)| (*id, ordinal as u64))
            .collect::<Vec<_>>();
        entries.push((fixture.target, 4)); // Requested position is never fetched here.
        entries.sort_by_key(|(id, _)| *id);
        let mut index = super::ChunkDirectory::new(reader.work.budget(), entries.len(), true);
        for (id, ordinal) in entries {
            index
                .append_index(
                    id,
                    super::EncryptedChunkLocator {
                        segment_ordinal: ordinal,
                        fragment_count: 1,
                    },
                )
                .unwrap();
        }
        reader.index = index;
        let physical = prerequisite_original_order(&reader);
        let position = physical
            .iter()
            .position(|(_, id)| *id == fixture.target)
            .unwrap();
        let first = position.saturating_sub(4);
        assert_eq!(position - first, 4);
        let mut expected = Vec::new();
        let mut candidate_groups = Vec::new();
        let mut expected_reads = Vec::new();
        for (_, id) in &physical[first..position] {
            state.lock().unwrap().reads.clear();
            let header = reader.chunk_header(*id).unwrap();
            expected_reads.extend_from_slice(&state.lock().unwrap().reads);
            candidate_groups.push(header.group_ref);
            if header.group_ref == requested.group_ref {
                expected.push(*id);
            }
        }
        assert_eq!(
            candidate_groups,
            vec![
                Some(matching_group),
                Some(other_group),
                Some(matching_group),
                Some(other_group)
            ]
        );
        assert_eq!(expected, vec![candidate_ids[0], candidate_ids[2]]);
        assert!(!expected.is_empty() && expected.len() < position - first);
        state.lock().unwrap().reads.clear();
        {
            let _observer = PrerequisiteObserverGuard::install(&state);
            let predecessors = reader
                .group_prerequisites(fixture.target, &requested)
                .unwrap();
            assert_eq!(predecessors.iter().collect::<Vec<_>>(), expected);
            assert_eq!(predecessors.len(), 2);
            assert_eq!(predecessors.bound, 4);
            let state = state.lock().unwrap();
            assert_eq!(state.reads, expected_reads);
            let full_charge = super::PredecessorDigests::reservation_bytes(4).unwrap();
            assert_eq!(state.active.values().copied().sum::<u64>(), full_charge);
            assert!(state.events.iter().any(|event| {
                event.0 == super::PrerequisiteKind::Predecessors
                    && event.1 == super::PrerequisitePhase::Ready
                    && event.2 == 4
                    && event.3 == 2
                    && event.4 >= full_charge
            }));
            drop(state);
            drop(predecessors);
        }
        prerequisite_teardown(reader, &state);
    }

    #[test]
    fn prerequisite_initialized_fields_and_reference_slots_clear_before_free_on_all_exits() {
        let work = test_work(8192);
        let baseline_owner = super::WorkBytes::zeroed(work.budget(), 19).unwrap();
        let ordinary = b"ordinary plaintext must survive metadata drop".to_vec();
        let state = Arc::new(Mutex::new(PrerequisiteState::default()));
        {
            let _observer = PrerequisiteObserverGuard::install(&state);
            for fill in [0, 1, 4] {
                for panic_after_fill in [false, true] {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let mut k =
                            super::PhysicalPrerequisites::reserve_empty(work.budget(), 4).unwrap();
                        let mut b =
                            super::PredecessorDigests::reserve_empty(work.budget(), 4).unwrap();
                        let mut s =
                            super::PrefixSliceSlots::reserve_empty(work.budget(), 4).unwrap();
                        for i in 0..fill {
                            k.push(i as u64 + 1, [255; 32]).unwrap();
                            b.push(slot_digest(i as u64 + 1)).unwrap();
                            s.push(&ordinary).unwrap();
                        }
                        if panic_after_fill {
                            panic!("generated partial-owner unwind");
                        }
                    }));
                    assert_eq!(result.is_err(), panic_after_fill);
                    assert_eq!(work.budget().live_bytes(), 19);
                    assert_eq!(
                        ordinary.as_slice(),
                        b"ordinary plaintext must survive metadata drop"
                    );
                    assert!(state.lock().unwrap().active.is_empty());
                }
            }
        }
        let state = state.lock().unwrap();
        assert_eq!(
            state
                .events
                .iter()
                .filter(|e| e.1 == super::PrerequisitePhase::BeforeFree)
                .count(),
            18
        );
        assert!(
            state
                .events
                .iter()
                .any(|e| e.1 == super::PrerequisitePhase::BeforeFree && e.3 == 1)
        );
        drop(state);
        drop(baseline_owner);
        assert_eq!(work.budget().live_bytes(), 0);
    }

    #[test]
    fn authenticated_prerequisite_exact_fit_and_short_refusals_keep_original_read_stops() {
        use super::{PrerequisiteKind as K, PrerequisitePhase as P};
        let fixture = prerequisite_fixture(true);
        for kind in [K::Physical, K::Predecessors, K::Slices] {
            for short in [false, true] {
                let state = Arc::new(Mutex::new(PrerequisiteState::default()));
                let mut reader = prerequisite_open(&fixture, &state);
                assert_eq!(
                    reader.index_status,
                    super::RandomAccessIndexStatus::PresentValid
                );
                let header = reader.chunk_header(fixture.target).unwrap();
                state.lock().unwrap().reads.clear();
                {
                    let mut state = state.lock().unwrap();
                    state.pressure_kind = Some(kind);
                    state.pressure_occurrence = 1;
                    state.short = short;
                    state.release_reserved = !short;
                }
                let mut decoded = super::BTreeMap::new();
                {
                    let _observer = PrerequisiteObserverGuard::install(&state);
                    let result = if kind == K::Slices {
                        reader.decode_chunk(fixture.target, &mut decoded)
                    } else {
                        reader
                            .group_prerequisites(fixture.target, &header)
                            .map(drop)
                    };
                    if short {
                        let error = result.unwrap_err();
                        assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
                        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
                        assert_eq!(
                            reader.index_status,
                            super::RandomAccessIndexStatus::PresentValid
                        );
                        assert!(!decoded.contains_key(&fixture.target));
                        if kind == K::Slices {
                            assert!(
                                !decoded.is_empty(),
                                "actual recursive plaintext fills precede S refusal"
                            );
                        } else {
                            assert!(
                                state.lock().unwrap().reads.is_empty(),
                                "K/B refusal precedes every candidate header read"
                            );
                        }
                    } else {
                        result.unwrap();
                        if kind == K::Slices {
                            assert_eq!(
                                decoded[&fixture.target].plaintext.as_ref(),
                                fixture.plaintexts[&fixture.target]
                            );
                        }
                    }
                }
                {
                    let mut state = state.lock().unwrap();
                    assert!(state.hit);
                    assert!(state.active.is_empty());
                    assert!(
                        state
                            .events
                            .iter()
                            .any(|e| e.0 == kind && e.1 == P::BeforeReserve && e.2 > 0)
                    );
                    assert_eq!(
                        state
                            .events
                            .iter()
                            .filter(|e| e.0 == kind && e.1 == P::Reserved && e.2 > 0)
                            .count()
                            > 0,
                        !short
                    );
                    state.pressure_kind = None;
                    drop(state.held.take());
                }
                reader.decode_chunk(fixture.target, &mut decoded).unwrap();
                assert_eq!(
                    decoded[&fixture.target].plaintext.as_ref(),
                    fixture.plaintexts[&fixture.target]
                );
                prerequisite_teardown(reader, &state);
            }
        }
    }

    #[test]
    fn authenticated_prerequisite_selection_refetch_and_decoded_skip_match_old_behavior() {
        let fixture = prerequisite_fixture(true);
        let oracle_state = Arc::new(Mutex::new(PrerequisiteState::default()));
        let mut oracle_reader = prerequisite_open(&fixture, &oracle_state);
        oracle_state.lock().unwrap().reads.clear();
        let mut oracle_decoded = super::BTreeMap::new();
        original_prerequisite_decode(&mut oracle_reader, fixture.target, &mut oracle_decoded);
        let oracle_reads = oracle_state.lock().unwrap().reads.clone();
        let oracle_values = oracle_decoded
            .iter()
            .map(|(id, c)| (*id, c.plaintext.to_vec()))
            .collect::<Vec<_>>();
        prerequisite_teardown(oracle_reader, &oracle_state);
        let state = Arc::new(Mutex::new(PrerequisiteState::default()));
        let mut reader = prerequisite_open(&fixture, &state);
        state.lock().unwrap().reads.clear();
        let mut decoded = super::BTreeMap::new();
        let expected;
        {
            let _observer = PrerequisiteObserverGuard::install(&state);
            reader.decode_chunk(fixture.target, &mut decoded).unwrap();
            assert!(decoded.len() > 1);
            for (id, chunk) in &decoded {
                assert_eq!(chunk.plaintext.as_ref(), fixture.plaintexts[id]);
            }
            expected = decoded
                .iter()
                .map(|(id, c)| (*id, c.plaintext.to_vec()))
                .collect::<Vec<_>>();
            assert_eq!(expected, oracle_values);
            assert_eq!(state.lock().unwrap().reads, oracle_reads);
            let old_events = state.lock().unwrap().events.len();
            let old_reads = state.lock().unwrap().reads.len();
            reader.decode_chunk(fixture.target, &mut decoded).unwrap();
            assert_eq!(state.lock().unwrap().events.len(), old_events);
            assert_eq!(state.lock().unwrap().reads.len(), old_reads);
            decoded.clear();
            state.lock().unwrap().reads.clear();
            reader.decode_chunk(fixture.target, &mut decoded).unwrap();
            let first_reads = state.lock().unwrap().reads.clone();
            decoded.clear();
            state.lock().unwrap().reads.clear();
            reader.decode_chunk(fixture.target, &mut decoded).unwrap();
            assert_eq!(state.lock().unwrap().reads, first_reads);
            assert_eq!(
                decoded
                    .iter()
                    .map(|(id, c)| (*id, c.plaintext.to_vec()))
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(reader.session.policy().max_cached_bytes, 0);
            let header = reader.chunk_header(fixture.target).unwrap();
            let mut plain_header = header;
            let store = crate::codec::store_plan();
            plain_header.plan_ref = store.plan_id;
            if !reader.plans.iter().any(|p| p.plan_id == store.plan_id) {
                let mut plans = reader.plans.to_vec();
                plans.push(store);
                reader.plans = plans.into_boxed_slice();
            }
            let before = state.lock().unwrap().events.len();
            let empty = reader
                .group_prerequisites(fixture.target, &plain_header)
                .unwrap();
            assert!(empty.is_empty());
            assert_eq!(state.lock().unwrap().events.len(), before);
            drop(empty);
        }
        prerequisite_teardown(reader, &state);
    }

    #[test]
    fn prerequisite_diagnostic_precedence_and_authenticated_header_failures_are_preserved() {
        let fixture = prerequisite_fixture(true);
        let state = Arc::new(Mutex::new(PrerequisiteState::default()));
        let mut reader = prerequisite_open(&fixture, &state);
        let header = reader.chunk_header(fixture.target).unwrap();
        let group = reader.groups[&header.group_ref.unwrap()].clone();
        {
            let _observer = PrerequisiteObserverGuard::install(&state);
            for (case, detail) in [
                (0, "Chunk references unknown TransformPlan"),
                (1, "prefix-coded Chunk lacks group_ref"),
                (2, "prefix-coded Chunk references unknown group"),
                (3, "group lookback and plan disagree"),
                (4, "Chunk has no encrypted physical position"),
            ] {
                let mut h = header;
                let id = if case == 4 {
                    slot_digest(u64::MAX)
                } else {
                    fixture.target
                };
                match case {
                    0 => h.plan_ref = u64::MAX,
                    1 => h.group_ref = None,
                    2 => h.group_ref = Some(slot_digest(u64::MAX)),
                    3 => {
                        reader.groups.get_mut(&group.group_id).unwrap().max_lookback =
                            group.max_lookback + 1
                    }
                    _ => {}
                }
                state.lock().unwrap().reads.clear();
                let error = reader.group_prerequisites(id, &h).err().unwrap();
                assert_eq!(error.class(), super::OutcomeClass::Corrupt);
                assert_eq!(
                    error.code(),
                    super::ReasonCode::RandomAccessDependencyInvalid
                );
                assert_eq!(error.detail(), detail);
                assert!(state.lock().unwrap().reads.is_empty());
                reader.groups.insert(group.group_id, group.clone());
                assert!(state.lock().unwrap().active.is_empty());
            }
            reader
                .groups
                .get_mut(&group.group_id)
                .unwrap()
                .max_preceding_bytes = 0;
            let error = reader
                .group_prerequisites(fixture.target, &header)
                .err()
                .unwrap();
            assert_eq!(error.code(), super::ReasonCode::AccessCostMismatch);
            assert!(!state.lock().unwrap().reads.is_empty());
            reader.groups.insert(group.group_id, group);
            let overflow = super::prerequisite_logical_total(u64::MAX, 1).unwrap_err();
            assert_eq!(overflow.detail(), "lookback byte count overflow");
            assert_eq!(
                overflow.code(),
                super::ReasonCode::RandomAccessDependencyInvalid
            );
            let physical = prerequisite_original_order(&reader);
            let target_position = physical
                .iter()
                .position(|(_, id)| *id == fixture.target)
                .unwrap();
            let predecessor = physical[target_position - 1].1;
            let locator = reader.index.get(&predecessor).unwrap();
            let segment = reader.segments.as_slice()[locator.segment_ordinal as usize];
            state.lock().unwrap().corrupt_at = Some(segment.offset + 96);
            let error = reader
                .group_prerequisites(fixture.target, &header)
                .err()
                .unwrap();
            assert_eq!(error.code(), super::ReasonCode::CryptoAeadAuthFailed);
            assert!(state.lock().unwrap().active.is_empty());
            state.lock().unwrap().corrupt_at = None;
            reader.group_prerequisites(fixture.target, &header).unwrap();
        }
        prerequisite_teardown(reader, &state);
        // Authenticate malformed private headers with the actual retained key
        // hierarchy. This finite two-position helper fixture has one real
        // candidate PAYLOAD; the requested position is not fetched on error.
        for case in 0..4 {
            let state = Arc::new(Mutex::new(PrerequisiteState::default()));
            let mut reader = prerequisite_open(&fixture, &state);
            let requested = reader.chunk_header(fixture.target).unwrap();
            let physical = prerequisite_original_order(&reader);
            let position = physical
                .iter()
                .position(|(_, d)| *d == fixture.target)
                .unwrap();
            let candidate = physical[position - 1].1;
            let mut frame = {
                let object = reader.chunk_frame(candidate).unwrap();
                object.view().unwrap().1.to_vec()
            };
            let expected = match case {
                0 => {
                    frame[0] ^= 1;
                    (
                        super::OutcomeClass::Corrupt,
                        super::ReasonCode::SectionStructure,
                        "Chunk frame magic mismatch",
                    )
                }
                1 => {
                    frame[6..8].copy_from_slice(&9_u16.to_be_bytes());
                    (
                        super::OutcomeClass::Nonconforming,
                        super::ReasonCode::NoncanonicalEncoding,
                        "Chunk frame version or flags are not canonical",
                    )
                }
                2 => {
                    frame[48..56].copy_from_slice(&u64::MAX.to_be_bytes());
                    (
                        super::OutcomeClass::Corrupt,
                        super::ReasonCode::ResourceLimit,
                        "declared logical bound",
                    )
                }
                _ => {
                    let old = u64::from_be_bytes(frame[8..16].try_into().unwrap());
                    frame[8..16].copy_from_slice(&(old + 1).to_be_bytes());
                    (
                        super::OutcomeClass::Corrupt,
                        super::ReasonCode::RandomAccessDependencyInvalid,
                        "encrypted Chunk frame stored length mismatch",
                    )
                }
            };
            let raw =
                super::wire::private_object(super::wire::PRIVATE_OBJECT_CHUNK, &frame).unwrap();
            let fragment = super::wire::PrivateFragmentRef {
                object_id: super::wire::encrypted_object_id(&raw).unwrap(),
                total_len: raw.len() as u64,
                index: 0,
                count: 1,
                offset: 0,
                bytes: &raw,
            };
            let (bytes, locator) = authenticated_test_segment(
                &[fragment],
                &reader.envelope.view().archive_id,
                &reader.keys,
                false,
            );
            let end_header = 96 + u64::from_be_bytes(bytes[88..96].try_into().unwrap());
            let expected_reads = vec![(64, 32), (end_header, 32), (0, locator.extent)];
            reader.session = super::RangeSession::new_crypto(
                Box::new(PrerequisiteSource {
                    source: MemoryRandomReadSource::new(bytes),
                    state: Arc::clone(&state),
                }),
                segment_access_policy(),
                reader.work.budget().clone(),
            )
            .unwrap();
            let mut segments = super::SegmentTable::new(reader.work.budget());
            append_segment(&mut segments, locator);
            reader.segments = segments;
            let mut index = super::ChunkDirectory::new(reader.work.budget(), 2, true);
            let mut slots = vec![(candidate, 0), (fixture.target, 1)];
            slots.sort_by_key(|(id, _)| *id);
            for (id, ordinal) in slots {
                index
                    .append_index(
                        id,
                        super::EncryptedChunkLocator {
                            segment_ordinal: ordinal,
                            fragment_count: 1,
                        },
                    )
                    .unwrap();
            }
            reader.index = index;
            state.lock().unwrap().reads.clear();
            {
                let _observer = PrerequisiteObserverGuard::install(&state);
                let error = reader
                    .group_prerequisites(fixture.target, &requested)
                    .err()
                    .unwrap();
                assert_eq!(error.class(), expected.0);
                assert_eq!(error.code(), expected.1);
                assert!(error.detail().contains(expected.2));
                assert_eq!(state.lock().unwrap().reads, expected_reads);
            }
            prerequisite_teardown(reader, &state);
        }
    }

    #[test]
    fn prerequisite_copied_physical_owner_allows_authenticated_index_replacement() {
        for include_index in [false, true] {
            let fixture = prerequisite_fixture(include_index);
            let state = Arc::new(Mutex::new(PrerequisiteState::default()));
            let mut reader = prerequisite_open(&fixture, &state);
            assert_eq!(
                reader.index_status,
                if include_index {
                    super::RandomAccessIndexStatus::PresentValid
                } else {
                    super::RandomAccessIndexStatus::RebuiltAbsent
                }
            );
            let header = reader.chunk_header(fixture.target).unwrap();
            let physical = prerequisite_original_order(&reader);
            let position = physical
                .iter()
                .position(|(_, id)| *id == fixture.target)
                .unwrap();
            let lookback = reader.groups[&header.group_ref.unwrap()].max_lookback;
            let first = super::physical_prerequisite_first(position, lookback);
            let predecessor = physical[first].1;
            let mut expected_predecessors = Vec::new();
            for (_, id) in &physical[first..position] {
                if reader.chunk_header(*id).unwrap().group_ref == header.group_ref {
                    expected_predecessors.push(*id);
                }
            }
            let original = reader
                .index
                .iter()
                .map(|(d, l)| (super::Digest::from_bytes(*d), *l))
                .collect::<Vec<_>>();
            let mut changed =
                super::ChunkDirectory::new(reader.work.budget(), original.len(), true);
            for (digest, mut locator) in original {
                if digest == predecessor {
                    locator.fragment_count += 1;
                }
                changed.append_index(digest, locator).unwrap();
            }
            reader.index = changed;
            let before = reader.index.slots.as_slice().as_ptr();
            state.lock().unwrap().reads.clear();
            {
                let _observer = PrerequisiteObserverGuard::install(&state);
                let known = Arc::new(Mutex::new(KnownState::default()));
                known.lock().unwrap().reserve_leave = Some(
                    super::KnownChunkSegments::reservation_bytes(reader.index.len()).unwrap() - 1,
                );
                {
                    let _known_observer = KnownObserverGuard::install(&known);
                    let error = reader
                        .group_prerequisites(fixture.target, &header)
                        .err()
                        .unwrap();
                    assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
                    assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
                    // Genuine fragment-count invalidity was observed first;
                    // the fallback's PolicyRefused propagates without a retry.
                    assert_eq!(
                        reader.index_status,
                        super::RandomAccessIndexStatus::RebuiltInvalid
                    );
                    assert_eq!(reader.index.slots.as_slice().as_ptr(), before);
                    assert!(reader.support.is_none());
                    assert!(state.lock().unwrap().reads.is_empty());
                    assert!(state.lock().unwrap().active.is_empty());
                    let mut known = known.lock().unwrap();
                    assert!(
                        known
                            .held
                            .as_ref()
                            .unwrap()
                            .belongs_to(reader.work.budget())
                    );
                    drop(known.held.take());
                }
                let predecessors = reader.group_prerequisites(fixture.target, &header).unwrap();
                assert_eq!(
                    predecessors.iter().collect::<Vec<_>>(),
                    expected_predecessors
                );
                assert_eq!(
                    reader.index_status,
                    super::RandomAccessIndexStatus::RebuiltInvalid
                );
                assert_ne!(reader.index.slots.as_slice().as_ptr(), before);
                assert!(
                    state
                        .lock()
                        .unwrap()
                        .events
                        .iter()
                        .any(|e| e.0 == super::PrerequisiteKind::Physical
                            && e.1 == super::PrerequisitePhase::Ready)
                );
                assert!(!state.lock().unwrap().reads.is_empty());
                drop(predecessors);
                let mut decoded = super::BTreeMap::new();
                reader.decode_chunk(fixture.target, &mut decoded).unwrap();
                assert_eq!(
                    decoded[&fixture.target].plaintext.as_ref(),
                    fixture.plaintexts[&fixture.target]
                );
            }
            prerequisite_teardown(reader, &state);
        }
    }

    #[test]
    fn recursive_prerequisite_owners_overlap_on_original_budget_and_unwind_without_detaching() {
        use super::PrerequisiteKind as K;
        let fixture = prerequisite_fixture(true);
        let state = Arc::new(Mutex::new(PrerequisiteState::default()));
        let mut reader = prerequisite_open(&fixture, &state);
        {
            let mut state = state.lock().unwrap();
            state.pressure_kind = Some(K::Physical);
            state.pressure_occurrence = 2; // First K is outer; second is actual recursive decode.
            state.short = true;
        }
        let mut decoded = super::BTreeMap::new();
        {
            let _observer = PrerequisiteObserverGuard::install(&state);
            let error = reader
                .decode_chunk(fixture.target, &mut decoded)
                .unwrap_err();
            assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
            assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
            assert!(!decoded.contains_key(&fixture.target));
            let mut state = state.lock().unwrap();
            assert!(state.hit);
            assert!(
                state.nested_predecessor_charge > 0,
                "real outer B is charged during recursive K"
            );
            assert!(state.active.is_empty());
            assert!(
                state.reads.len() > 2,
                "real outer/candidate/nested encrypted frames were fetched"
            );
            state.pressure_kind = None;
            drop(state.held.take());
            drop(state);
            reader.decode_chunk(fixture.target, &mut decoded).unwrap();
            assert!(decoded.len() > 1);
            assert_eq!(
                decoded[&fixture.target].plaintext.as_ref(),
                fixture.plaintexts[&fixture.target]
            );
        }
        prerequisite_teardown(reader, &state);
    }

    #[test]
    fn prefix_reference_slots_remain_charged_through_builder_then_preserve_plaintext() {
        use super::{PrerequisiteKind as K, PrerequisitePhase as P};
        let work = test_work(4096);
        let a = b"first ordinary chunk".to_vec();
        let b = b"second ordinary chunk".to_vec();
        let state = Arc::new(Mutex::new(PrerequisiteState::default()));
        {
            let _observer = PrerequisiteObserverGuard::install(&state);
            let mut predecessors =
                super::PredecessorDigests::reserve_empty(work.budget(), 4).unwrap();
            predecessors.push(slot_digest(1)).unwrap();
            predecessors.push(slot_digest(2)).unwrap();
            let mut slices =
                super::PrefixSliceSlots::reserve_empty(work.budget(), predecessors.len()).unwrap();
            slices.push(&a).unwrap();
            slices.push(&b).unwrap();
            let charge = super::PredecessorDigests::reservation_bytes(4).unwrap()
                + super::PrefixSliceSlots::reservation_bytes(2).unwrap();
            let prefix = super::physical_prefix_from_slices(slices.as_slice(), 2).unwrap();
            assert_eq!(prefix, [a.as_slice(), b.as_slice()].concat());
            assert_eq!(work.budget().live_bytes(), charge);
            drop(slices);
            assert_eq!(
                work.budget().live_bytes(),
                super::PredecessorDigests::reservation_bytes(4).unwrap()
            );
            assert_eq!(a, b"first ordinary chunk");
            assert_eq!(b, b"second ordinary chunk");
            drop(predecessors);
            assert_eq!(prefix, b"first ordinary chunksecond ordinary chunk");
        }
        assert_eq!(work.budget().live_bytes(), 0);
        let fixture = prerequisite_fixture(true);
        let state = Arc::new(Mutex::new(PrerequisiteState::default()));
        let mut reader = prerequisite_open(&fixture, &state);
        {
            let _observer = PrerequisiteObserverGuard::install(&state);
            let mut decoded = super::BTreeMap::new();
            reader.decode_chunk(fixture.target, &mut decoded).unwrap();
            assert_eq!(
                decoded[&fixture.target].plaintext.as_ref(),
                fixture.plaintexts[&fixture.target]
            );
            let state = state.lock().unwrap();
            for (index, event) in state.events.iter().enumerate() {
                if event.0 == K::Slices && event.1 == P::Ready && event.2 > 0 {
                    assert_eq!(state.events[index + 1].0, K::Slices);
                    assert_eq!(state.events[index + 1].1, P::BeforeFree);
                    assert_eq!(state.events[index + 2].0, K::Predecessors);
                    assert_eq!(state.events[index + 2].1, P::BeforeFree);
                }
            }
            assert!(
                state
                    .events
                    .iter()
                    .any(|e| e.0 == K::Slices && e.1 == P::Ready && e.2 > 0)
            );
        }
        {
            let locator = reader.index.get(&fixture.target).unwrap();
            let segment = reader.segments.as_slice()[locator.segment_ordinal as usize];
            let mut state = state.lock().unwrap();
            state.requested_read = Some((segment.offset, segment.extent));
            state.requested_hits = 0;
            state.outer_drop_observed = false;
        }
        {
            let _observer = PrerequisiteObserverGuard::install(&state);
            let entry = reader.read_entry(&fixture.target_path).unwrap();
            assert_eq!(&*entry.bytes, fixture.target_entry_bytes);
            assert!(entry.report.content_object_digest_verified);
            assert!(state.lock().unwrap().outer_drop_observed);
        }
        prerequisite_teardown(reader, &state);
    }

    fn segment_slot(value: u64, class: u8) -> super::SegmentLocator {
        let mut slot = test_segment(class, 1);
        slot.offset = value * 128;
        slot.extent = 128;
        slot.salt[8..].copy_from_slice(&value.to_be_bytes());
        slot.header[..8].copy_from_slice(&value.to_be_bytes());
        slot
    }

    #[derive(Default)]
    struct KnownState {
        budget: Option<super::WorkBudget>,
        reserve_leave: Option<u64>,
        release_ready: bool,
        held: Option<super::WorkBytes>,
        ready_live: u64,
        ready_available: u64,
        events: Vec<(super::KnownChunkPhase, usize, usize, u64)>,
        target: Option<(u64, u64)>,
        fill_leave: Option<u64>,
        fill_live: u64,
        fill_available: u64,
        hits: usize,
        reads: Vec<(u64, u64)>,
    }

    std::thread_local! {
        static KNOWN_STATE: std::cell::RefCell<Option<Arc<Mutex<KnownState>>>> =
            const { std::cell::RefCell::new(None) };
    }

    fn observe_known(
        phase: super::KnownChunkPhase,
        budget: &super::WorkBudget,
        slots: &[u64],
        bound: usize,
    ) {
        KNOWN_STATE.with(|cell| {
            let shared = cell.borrow().as_ref().unwrap().clone();
            let mut state = shared.lock().unwrap_or_else(|e| e.into_inner());
            match phase {
                super::KnownChunkPhase::BeforeReserve => {
                    assert!(slots.is_empty());
                    state.budget = Some(budget.clone());
                    if let Some(leave) = state.reserve_leave {
                        let held = budget.available_bytes().checked_sub(leave).unwrap();
                        let pressure = super::WorkBytes::zeroed(budget, held as usize).unwrap();
                        assert!(pressure.belongs_to(budget));
                        state.held = Some(pressure);
                        assert_eq!(budget.available_bytes(), leave);
                    }
                }
                super::KnownChunkPhase::Ready => {
                    assert_eq!(slots.len(), bound);
                    state.ready_live = budget.live_bytes();
                    state.ready_available = budget.available_bytes();
                    if state.release_ready {
                        drop(state.held.take());
                    }
                }
                super::KnownChunkPhase::BeforeFree => {
                    assert!(slots.iter().all(|value| *value == 0));
                    if !slots.is_empty() {
                        assert!(
                            budget.live_bytes()
                                >= super::KnownChunkSegments::reservation_bytes(bound).unwrap()
                        );
                    }
                }
            }
            state
                .events
                .push((phase, bound, slots.len(), budget.live_bytes()));
        });
    }

    struct KnownObserverGuard {
        previous: Option<super::KnownChunkObserver>,
        previous_state: Option<Arc<Mutex<KnownState>>>,
    }

    impl KnownObserverGuard {
        fn install(state: &Arc<Mutex<KnownState>>) -> Self {
            Self {
                previous: super::KNOWN_CHUNK_OBSERVER
                    .with(|cell| cell.replace(Some(observe_known))),
                previous_state: KNOWN_STATE.with(|cell| cell.replace(Some(Arc::clone(state)))),
            }
        }
    }

    impl Drop for KnownObserverGuard {
        fn drop(&mut self) {
            super::KNOWN_CHUNK_OBSERVER.with(|cell| cell.set(self.previous));
            KNOWN_STATE.with(|cell| *cell.borrow_mut() = self.previous_state.take());
        }
    }

    struct KnownSource {
        source: MemoryRandomReadSource,
        state: Arc<Mutex<KnownState>>,
    }

    impl RandomReadSource for KnownSource {
        fn len(&self) -> crate::diagnostics::Result<u64> {
            self.source.len()
        }
        fn revision(&self) -> crate::diagnostics::Result<SourceRevision> {
            self.source.revision()
        }
        fn read_exact_at(&self, _: u64, _: u64) -> crate::diagnostics::Result<Vec<u8>> {
            panic!("known ordinal scan must use bounded source fill")
        }
        fn read_exact_at_into(
            &self,
            offset: u64,
            dst: &mut [u8],
        ) -> crate::diagnostics::Result<()> {
            let mut state = self.state.lock().unwrap();
            state.reads.push((offset, dst.len() as u64));
            if state.target == Some((offset, dst.len() as u64)) {
                state.hits += 1;
                if state.hits == 1 {
                    let budget = state.budget.as_ref().unwrap().clone();
                    if let Some(leave) = state.fill_leave {
                        let held = budget.available_bytes().checked_sub(leave).unwrap();
                        let pressure = super::WorkBytes::zeroed(&budget, held as usize)?;
                        assert!(pressure.belongs_to(&budget));
                        state.held = Some(pressure);
                    }
                    state.fill_live = budget.live_bytes();
                    state.fill_available = budget.available_bytes();
                }
            }
            drop(state);
            self.source.read_exact_at_into(offset, dst)
        }
    }

    fn known_source(bytes: Vec<u8>, state: &Arc<Mutex<KnownState>>) -> KnownSource {
        KnownSource {
            source: MemoryRandomReadSource::new(bytes),
            state: Arc::clone(state),
        }
    }

    fn known_fixture(include_index: bool) -> SegmentFixture {
        let directory = TestDir::new();
        let expected = (0_u64..12)
            .map(|i| {
                let name = format!("known{i:02}");
                let bytes = (i + 1).to_be_bytes();
                std::fs::write(directory.path().join(&name), bytes).unwrap();
                (name, bytes)
            })
            .collect::<Vec<_>>();
        let mut archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        archive.descriptor.features.incompat |= crate::ecf::FEATURE_CROSS_FILE_COMPRESSION_V1
            | crate::ecf::FEATURE_CODEC_TRANSFORM_V1
            | crate::ecf::FEATURE_RECONSTRUCTIVE_TRANSFORM_V1
            | crate::ecf::FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1;
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: &[recipient],
                padding: PaddingMode::None,
                include_index,
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let mut options = EncryptedOpenOptions::new(Some(Unlock::Identity(&identity)));
        options.crypto_policy.max_working_memory_bytes = SEGMENT_FINITE_LIMIT;
        let opened = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(encrypted.bytes.clone()),
            segment_access_policy(),
            options,
        )
        .unwrap();
        let segments = opened.segments.as_slice().to_vec();
        let footer = super::parse_footer(
            &encrypted.bytes[encrypted.bytes.len() - 192..],
            encrypted.bytes.len() as u64,
        )
        .unwrap();
        SegmentFixture {
            bytes: encrypted.bytes,
            identity,
            expected,
            footer,
            segments,
        }
    }

    fn known_open(
        fixture: &SegmentFixture,
        state: &Arc<Mutex<KnownState>>,
        policy: RandomAccessPolicy,
    ) -> super::EncryptedRandomAccessArchive {
        let mut options = EncryptedOpenOptions::new(Some(Unlock::Identity(&fixture.identity)));
        options.crypto_policy.max_working_memory_bytes = SEGMENT_FINITE_LIMIT;
        open_indexed_random_encrypted(known_source(fixture.bytes.clone(), state), policy, options)
            .unwrap()
    }

    // Derive the complete expected fetches directly from immutable fixture bytes.
    // This oracle does not call protected-record preflight or decrypt_segment.
    fn known_read_oracle(
        fixture: &SegmentFixture,
        ordinals: impl IntoIterator<Item = usize>,
    ) -> Vec<(u64, u64)> {
        let mut reads = Vec::new();
        for ordinal in ordinals {
            let segment = fixture.segments[ordinal];
            let end = segment.offset.checked_add(segment.extent).unwrap();
            assert!(end <= u64::try_from(fixture.bytes.len()).unwrap());
            let mut cursor = segment.offset.checked_add(64).unwrap();
            assert!(cursor <= end);
            for counter in 0..=u64::from(segment.count) {
                let header_end = cursor.checked_add(32).unwrap();
                assert!(header_end <= end);
                let header = fixture
                    .bytes
                    .get(usize::try_from(cursor).unwrap()..usize::try_from(header_end).unwrap())
                    .unwrap();
                assert_eq!(&header[..4], b"EBC1");
                assert_eq!(u16::from_be_bytes(header[4..6].try_into().unwrap()), 1);
                let class = if counter == u64::from(segment.count) {
                    2
                } else {
                    1
                };
                assert_eq!(header[6], class);
                assert_eq!(header[7], 0);
                assert_eq!(
                    u64::from_be_bytes(header[8..16].try_into().unwrap()),
                    u64::try_from(ordinal).unwrap()
                );
                assert_eq!(
                    u64::from_be_bytes(header[16..24].try_into().unwrap()),
                    counter
                );
                let ciphertext_len = u64::from_be_bytes(header[24..32].try_into().unwrap());
                assert!(ciphertext_len >= 16);
                reads.push((cursor, 32));
                cursor = header_end.checked_add(ciphertext_len).unwrap();
                assert!(cursor <= end);
            }
            assert_eq!(cursor, end);
            reads.push((segment.offset, segment.extent));
        }
        reads
    }

    fn known_index(work: &super::CryptoWorkContext, ordinals: &[u64]) -> super::ChunkDirectory {
        let mut index = super::ChunkDirectory::new(work.budget(), ordinals.len(), true);
        for (i, ordinal) in ordinals.iter().enumerate() {
            index
                .append_index(slot_digest(i as u64), slot_locator(*ordinal))
                .unwrap();
        }
        index
    }

    #[test]
    fn known_ordinals_heap_compaction_and_lookup_match_independent_set() {
        let cases = [
            vec![],
            vec![u64::MAX],
            (0..129).collect(),
            (0..129).rev().collect(),
            (0..65).flat_map(|i| [i, 128 - i]).collect(),
            vec![5, 2, 5, 0, u64::MAX, 2, u64::MAX - 1, 0, u64::MAX],
            vec![17; 129],
        ];
        for ordinals in cases {
            let work = test_work(128 * 1024);
            let index = known_index(&work, &ordinals);
            let oracle = ordinals
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            let baseline = work.budget().live_bytes();
            let owner = super::KnownChunkSegments::new(work.budget(), &index).unwrap();
            assert_eq!(owner.slots.as_slice().len(), ordinals.len());
            assert_eq!(owner.unique_len, oracle.len());
            assert_eq!(
                &owner.slots.as_slice()[..owner.unique_len],
                oracle.iter().copied().collect::<Vec<_>>()
            );
            assert_eq!(
                work.budget().live_bytes(),
                baseline + ordinals.len() as u64 * 8
            );
            for value in (0..135).chain([u64::MAX - 1, u64::MAX]) {
                assert_eq!(owner.contains(&value), oracle.contains(&value));
            }
            drop(owner);
            assert_eq!(work.budget().live_bytes(), baseline);
        }
    }

    #[test]
    fn known_ordinals_checked_layout_fixed_fill_and_no_growth_at_zero_headroom() {
        assert_eq!(super::KnownChunkSegments::reservation_bytes(0).unwrap(), 0);
        for count in [usize::MAX, isize::MAX as usize / 8 + 1] {
            let error = super::KnownChunkSegments::reservation_bytes(count).unwrap_err();
            assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
            assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        }
        let zero = test_work(0);
        let owner = super::KnownChunkSegments::reserve_empty(zero.budget(), 0).unwrap();
        assert_eq!(zero.budget().live_bytes(), 0);
        drop(owner);
        let k = 17;
        let charge = super::KnownChunkSegments::reservation_bytes(k).unwrap();
        assert_eq!(
            charge,
            std::alloc::Layout::array::<u64>(k).unwrap().size() as u64
        );
        let work = test_work(charge + 19);
        let caller = super::WorkBytes::zeroed(work.budget(), 19).unwrap();
        let mut owner = super::KnownChunkSegments::reserve_empty(work.budget(), k).unwrap();
        let pointer = owner.slots.as_slice().as_ptr();
        assert_eq!(owner.slots.minimum_reservation_bytes(k).unwrap(), 0);
        assert_eq!(
            owner.slots.minimum_reservation_bytes(k + 1).unwrap(),
            (k + 1) as u64 * 8
        );
        assert_eq!(work.budget().available_bytes(), 0);
        for i in (0..k).rev() {
            owner.push(i as u64).unwrap();
            assert_eq!(owner.slots.as_slice().as_ptr(), pointer);
            assert_eq!(work.budget().live_bytes(), charge + 19);
        }
        let error = owner.push(99).unwrap_err();
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        owner.finish();
        for value in [0, 8, 16, 17, u64::MAX] {
            assert_eq!(owner.contains(&value), value < k as u64);
        }
        assert_eq!(owner.slots.as_slice().as_ptr(), pointer);
        assert_eq!(owner.slots.as_slice().len(), k);
        assert_eq!(work.budget().available_bytes(), 0);
        drop(owner);
        assert_eq!(work.budget().live_bytes(), 19);
        drop(caller);
        assert_eq!(work.budget().live_bytes(), 0);
        let short = test_work(charge - 1);
        assert!(super::KnownChunkSegments::reserve_empty(short.budget(), k).is_err());
        assert_eq!(short.budget().live_bytes(), 0);
    }

    #[test]
    fn known_ordinals_wipe_full_duplicate_tail_and_partial_prefix_on_every_exit() {
        let state = Arc::new(Mutex::new(KnownState::default()));
        let work = test_work(7 * 8 + 13);
        let caller = super::WorkBytes::zeroed(work.budget(), 13).unwrap();
        for (mode, initialized) in [(0, 7), (1, 3), (2, 7), (3, 3)] {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _observer = KnownObserverGuard::install(&state);
                let mut owner = super::KnownChunkSegments::reserve_empty(work.budget(), 7).unwrap();
                for _ in 0..initialized {
                    owner.push(u64::MAX).unwrap();
                }
                owner.finish();
                assert_eq!(owner.unique_len, 1);
                assert_eq!(owner.slots.as_slice().len(), initialized);
                assert!(owner.slots.as_slice()[1..].iter().all(|v| *v == u64::MAX));
                if mode == 2 || mode == 3 {
                    panic!("generated owner unwind");
                }
                if mode == 1 {
                    return Err(super::crypto_policy("generated partial error"));
                }
                Ok(())
            }));
            assert_eq!(outcome.is_err(), mode >= 2);
            if mode == 1 {
                assert!(outcome.unwrap().is_err());
            }
            assert_eq!(work.budget().live_bytes(), 13);
            assert!(super::KNOWN_CHUNK_OBSERVER.with(|cell| cell.get().is_none()));
            assert!(KNOWN_STATE.with(|cell| cell.borrow().is_none()));
        }
        let state = state.lock().unwrap();
        let wipes = state
            .events
            .iter()
            .filter(|e| e.0 == super::KnownChunkPhase::BeforeFree)
            .map(|e| (e.1, e.2, e.3))
            .collect::<Vec<_>>();
        assert_eq!(wipes, vec![(7, 7, 69), (7, 3, 69), (7, 7, 69), (7, 3, 69)]);
        drop(state);
        drop(caller);
        assert_eq!(work.budget().live_bytes(), 0);
    }

    #[test]
    fn known_ordinals_authenticated_exact_fit_and_one_byte_refusal_precede_payload() {
        let fixture = known_fixture(true);
        for short in [false, true] {
            let state = Arc::new(Mutex::new(KnownState::default()));
            let mut reader = known_open(&fixture, &state, segment_access_policy());
            assert_eq!(
                reader.index_status,
                super::RandomAccessIndexStatus::PresentValid
            );
            let k = reader.index.len();
            assert_eq!(k, fixture.expected.len());
            let charge = super::KnownChunkSegments::reservation_bytes(k).unwrap();
            let baseline = reader.work.budget().live_bytes();
            let reads_before = state.lock().unwrap().reads.len();
            {
                let mut state = state.lock().unwrap();
                state.reserve_leave = Some(charge - u64::from(short));
                state.release_ready = true;
            }
            {
                let _observer = KnownObserverGuard::install(&state);
                let result = reader.load_support_and_index_fallback();
                if short {
                    let error = result.unwrap_err();
                    assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
                    assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
                    assert_eq!(state.lock().unwrap().reads.len(), reads_before);
                    assert_eq!(
                        reader.index_status,
                        super::RandomAccessIndexStatus::PresentValid
                    );
                    assert_eq!(reader.index.len(), k);
                    assert!(reader.support.is_none());
                } else {
                    result.unwrap();
                    assert!(reader.support.is_some());
                    assert_eq!(reader.index.len(), k);
                    assert_eq!(
                        reader.index_status,
                        super::RandomAccessIndexStatus::PresentValid
                    );
                    let count = state.lock().unwrap().events.len();
                    reader.load_support_and_index_fallback().unwrap();
                    assert_eq!(state.lock().unwrap().events.len(), count);
                    for (name, bytes) in &fixture.expected {
                        let entry = reader
                            .read_entry(&LogicalPath::from_utf8([name.as_str()]).unwrap())
                            .unwrap();
                        assert_eq!(&*entry.bytes, bytes);
                    }
                }
            }
            {
                let mut state = state.lock().unwrap();
                let ready = state
                    .events
                    .iter()
                    .filter(|e| e.0 == super::KnownChunkPhase::Ready)
                    .count();
                assert_eq!(ready, usize::from(!short));
                if !short {
                    assert_eq!(state.ready_available, 0);
                    assert_eq!(state.ready_live, SEGMENT_FINITE_LIMIT);
                }
                assert!(
                    state
                        .events
                        .iter()
                        .any(|e| e.0 == super::KnownChunkPhase::BeforeFree
                            && e.2 == if short { 0 } else { k })
                );
                drop(state.held.take());
            }
            if short {
                assert_eq!(reader.work.budget().live_bytes(), baseline);
                reader.load_support_and_index_fallback().unwrap();
                assert!(reader.support.is_some());
            }
            let budget = reader.work.budget().clone();
            drop(reader);
            assert_eq!(budget.live_bytes(), 0);
        }
    }

    #[test]
    fn known_ordinals_authenticated_scan_skips_known_chunks_but_reads_real_support() {
        let fixture = known_fixture(true);
        let state = Arc::new(Mutex::new(KnownState::default()));
        let mut reader = known_open(&fixture, &state, segment_access_policy());
        let revision = reader.session.initial_revision().clone();
        let known = reader
            .index
            .values()
            .map(|l| l.segment_ordinal)
            .collect::<std::collections::BTreeSet<_>>();
        let before = state.lock().unwrap().reads.len();
        let trace_before = reader.session.trace().len();
        let requests_before = reader.session.range_requests();
        let fetched_before = reader.session.bytes_fetched();
        {
            let _observer = KnownObserverGuard::install(&state);
            reader.load_support_and_index_fallback().unwrap();
        }
        let reads = state.lock().unwrap().reads[before..].to_vec();
        let support_ordinals = fixture
            .segments
            .iter()
            .enumerate()
            .filter(|(i, s)| s.class == super::SEGMENT_PAYLOAD && !known.contains(&(*i as u64)))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        assert_eq!(support_ordinals.len(), 3);
        let expected = known_read_oracle(&fixture, support_ordinals);
        assert_eq!(expected.len(), 9);
        assert_eq!(reads, expected);
        assert_eq!(reader.session.trace().len() - trace_before, expected.len());
        assert_eq!(
            reader.session.range_requests() - requests_before,
            expected.len() as u64
        );
        assert_eq!(
            reader.session.bytes_fetched() - fetched_before,
            expected.iter().map(|r| r.1).sum::<u64>()
        );
        assert_eq!(
            reader.index_status,
            super::RandomAccessIndexStatus::PresentValid
        );
        assert!(reader.support.is_some());
        assert_eq!(reader.session.initial_revision(), &revision);
        assert_eq!(
            state.lock().unwrap().events.last().unwrap().0,
            super::KnownChunkPhase::BeforeFree
        );
        for (name, bytes) in &fixture.expected {
            let entry = reader
                .read_entry(&LogicalPath::from_utf8([name.as_str()]).unwrap())
                .unwrap();
            assert_eq!(&*entry.bytes, bytes);
        }
    }

    #[test]
    fn known_ordinals_physical_membership_and_missing_invalid_index_rebuild() {
        for include_index in [true, false] {
            let fixture = known_fixture(include_index);
            for invalid in [false, true] {
                let state = Arc::new(Mutex::new(KnownState::default()));
                let mut reader = known_open(&fixture, &state, segment_access_policy());
                if invalid {
                    // Authenticate a deliberately invalid canonical Index body
                    // with the archive's real keys, then feed its rejected state
                    // into the unchanged archive's live PAYLOAD fallback.
                    let records = [index_record(slot_digest(0).as_bytes(), u64::MAX, 0, 1)];
                    let bytes =
                        super::wire::sequence_container(super::wire::COLLECTION_INDEX, &records)
                            .unwrap();
                    let exact =
                        super::wire::private_object(super::wire::PRIVATE_OBJECT_SEQUENCE, &bytes)
                            .unwrap();
                    let fragment = super::wire::PrivateFragmentRef {
                        object_id: super::wire::encrypted_object_id(&exact).unwrap(),
                        total_len: exact.len() as u64,
                        index: 0,
                        count: 1,
                        offset: 0,
                        bytes: &exact,
                    };
                    let (source, locator) = authenticated_test_segment(
                        &[fragment],
                        &reader.envelope.view().archive_id,
                        &reader.keys,
                        false,
                    );
                    let mut session = super::RangeSession::new_crypto(
                        Box::new(MemoryRandomReadSource::new(source)),
                        segment_access_policy(),
                        reader.work.budget().clone(),
                    )
                    .unwrap();
                    let mut options = EncryptedOpenOptions::new(None);
                    options.crypto_policy = reader.work.policy();
                    let object = super::decrypt_segment(
                        &mut session,
                        &locator,
                        0,
                        &reader.envelope.view(),
                        &reader.keys,
                        reader.padding,
                        options,
                        super::AccessPurpose::EncryptedPayload,
                        &reader.work,
                    )
                    .unwrap();
                    let sequence =
                        super::wire::WorkSequence::decode_with_context(object, &reader.work)
                            .unwrap();
                    let (index, status) = super::decode_index_or_rebuild(
                        &sequence,
                        reader.segments.as_slice(),
                        reader.metadata.descriptor.budget.chunk_count,
                        &reader.work,
                    )
                    .unwrap();
                    assert_eq!(status, super::RandomAccessIndexStatus::RebuiltInvalid);
                    reader.index = index;
                    reader.index_status = status;
                } else if include_index {
                    // Exercise physical ordinals independently of digest order:
                    // repeated known PAYLOAD, a known CONTROL and a nonexistent
                    // ordinal cannot suppress other real PAYLOAD reads.
                    let ordinal = reader
                        .index
                        .values()
                        .map(|l| l.segment_ordinal)
                        .min()
                        .unwrap();
                    let control = reader
                        .segments
                        .as_slice()
                        .iter()
                        .position(|s| s.class == super::SEGMENT_CONTROL)
                        .unwrap() as u64;
                    reader.index =
                        known_index(&reader.work, &[ordinal, control, ordinal, u64::MAX]);
                }
                let expected_skip = if include_index && !invalid {
                    reader.index.values().next().unwrap().segment_ordinal
                } else {
                    u64::MAX
                };
                let before = state.lock().unwrap().reads.len();
                {
                    let _observer = KnownObserverGuard::install(&state);
                    let result = reader.load_support_and_index_fallback();
                    if include_index && !invalid {
                        let error = result.unwrap_err();
                        assert_eq!(error.class(), super::OutcomeClass::Nonconforming);
                        assert_eq!(error.detail(), "rebuilt encrypted Chunk count mismatch");
                    } else {
                        result.unwrap();
                    }
                }
                let reads = state.lock().unwrap().reads[before..].to_vec();
                let expected_reads = known_read_oracle(
                    &fixture,
                    fixture
                        .segments
                        .iter()
                        .enumerate()
                        .filter(|(i, s)| {
                            s.class == super::SEGMENT_PAYLOAD && (*i as u64 != expected_skip)
                        })
                        .map(|(i, _)| i),
                );
                assert_eq!(reads, expected_reads);
                assert_eq!(
                    reader.index.len(),
                    fixture.expected.len() - usize::from(include_index && !invalid)
                );
                if include_index || invalid {
                    assert_eq!(
                        reader.index_status,
                        super::RandomAccessIndexStatus::RebuiltInvalid
                    );
                } else {
                    assert_eq!(
                        reader.index_status,
                        super::RandomAccessIndexStatus::RebuiltAbsent
                    );
                }
                assert_eq!(reader.support.is_some(), invalid || !include_index);
                // Do not claim the deliberately suppressed Chunk is usable;
                // genuine absent/invalid rebuilds recover every entry.
                if invalid || !include_index {
                    for (name, bytes) in &fixture.expected {
                        assert_eq!(
                            &*reader
                                .read_entry(&LogicalPath::from_utf8([name.as_str()]).unwrap())
                                .unwrap()
                                .bytes,
                            bytes
                        );
                    }
                }
            }
        }
    }

    fn known_object(
        reader: &mut super::EncryptedRandomAccessArchive,
        ordinal: usize,
    ) -> super::wire::WorkObject {
        let locator = reader.segments.as_slice()[ordinal];
        let mut options = EncryptedOpenOptions::new(None);
        options.crypto_policy = reader.work.policy();
        options.resource_policy = reader.session.policy().resource_policy;
        options.decode_policy = reader.session.policy().decode_policy;
        super::decrypt_segment(
            &mut reader.session,
            &locator,
            ordinal as u64,
            &reader.envelope.view(),
            &reader.keys,
            reader.padding,
            options,
            super::AccessPurpose::EncryptedPayload,
            &reader.work,
        )
        .unwrap()
    }

    fn miss_first_known_ordinal(reader: &mut super::EncryptedRandomAccessArchive) {
        let first = reader
            .index
            .values()
            .map(|l| l.segment_ordinal)
            .min()
            .unwrap();
        let mut index = super::ChunkDirectory::new(reader.work.budget(), reader.index.len(), true);
        for (digest, locator) in reader.index.iter() {
            let mut locator = *locator;
            if locator.segment_ordinal == first {
                locator.segment_ordinal = u64::MAX;
            }
            index
                .append_index(super::Digest::from_bytes(*digest), locator)
                .unwrap();
        }
        reader.index = index;
    }

    #[test]
    fn known_ordinals_remain_live_through_raw_discovered_and_retained_support_overlap() {
        let fixture = known_fixture(true);
        let calibration = Arc::new(Mutex::new(KnownState::default()));
        let mut probe = known_open(&fixture, &calibration, segment_access_policy());
        let payloads = probe
            .segments
            .as_slice()
            .iter()
            .enumerate()
            .filter(|(_, s)| s.class == super::SEGMENT_PAYLOAD)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        assert_eq!(payloads.len(), fixture.expected.len() + 3);
        let mut support_kinds = std::collections::BTreeSet::new();
        let mut support_collections = Vec::new();
        let mut support_owned = 0_u64;
        for i in &payloads[..3] {
            let object = known_object(&mut probe, *i);
            assert_eq!(
                object.view().unwrap().0,
                super::wire::PRIVATE_OBJECT_SEQUENCE
            );
            let raw_len = u64::try_from(object.exact_bytes().len()).unwrap();
            // The raw object and any source trace capacity are already charged.
            // Only this decode adds the full WorkSequence span-table backing.
            let before_decode = probe.work.budget().live_bytes();
            let without_object = before_decode.checked_sub(raw_len).unwrap();
            let sequence =
                super::wire::WorkSequence::decode_with_context(object, &probe.work).unwrap();
            let span_bytes = probe
                .work
                .budget()
                .live_bytes()
                .checked_sub(before_decode)
                .unwrap();
            assert_eq!(span_bytes > 0, sequence.len() != 0);
            assert!(support_kinds.insert(sequence.kind()));
            support_collections.push((sequence.kind(), sequence.len()));
            let owned_bytes = raw_len.checked_add(span_bytes).unwrap();
            support_owned = support_owned.checked_add(owned_bytes).unwrap();
            drop(sequence);
            // Trace capacity stays in without_object; release only raw+spans.
            assert_eq!(probe.work.budget().live_bytes(), without_object);
        }
        assert_eq!(
            support_kinds,
            std::collections::BTreeSet::from([
                super::wire::COLLECTION_DICTIONARIES,
                super::wire::COLLECTION_RECONSTRUCTION_DATA,
                super::wire::COLLECTION_RECONSTRUCTION_REGIONS,
            ])
        );
        assert_eq!(support_collections.len(), 3);
        let first_chunk = probe.segments.as_slice()[payloads[3]];
        let raw_len = known_object(&mut probe, payloads[3]).exact_bytes().len() as u64;
        let bound = super::fallback_slot_bound(probe.segments.as_slice(), u64::MAX).unwrap();
        let directory_bytes = super::ChunkDirectory::reservation_bytes(bound).unwrap();
        assert!(directory_bytes > first_chunk.extent + 64);
        drop(probe);
        for pressure in [true, false] {
            let state = Arc::new(Mutex::new(KnownState::default()));
            let mut reader = known_open(&fixture, &state, segment_access_policy());
            miss_first_known_ordinal(&mut reader);
            let k = reader.index.len();
            let known_bytes = super::KnownChunkSegments::reservation_bytes(k).unwrap();
            let baseline = reader.work.budget().live_bytes();
            {
                let mut state = state.lock().unwrap();
                state.target = Some((first_chunk.offset, first_chunk.extent));
                if pressure {
                    state.fill_leave = Some(directory_bytes - 1 - first_chunk.extent + raw_len);
                }
            }
            {
                let _observer = KnownObserverGuard::install(&state);
                let result = reader.load_support_and_index_fallback();
                let state_guard = state.lock().unwrap();
                assert_eq!(state_guard.hits, 1);
                assert!(
                    state_guard.fill_live
                        >= baseline + known_bytes + support_owned + first_chunk.extent
                );
                drop(state_guard);
                if pressure {
                    let error = result.unwrap_err();
                    assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
                    assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
                    assert_eq!(
                        error.detail(),
                        "managed crypto replacement and required headroom exceed caller policy"
                    );
                    assert_eq!(
                        reader.index_status,
                        super::RandomAccessIndexStatus::PresentValid
                    );
                    assert_eq!(reader.index.len(), k);
                    assert!(reader.support.is_none());
                    let state_guard = state.lock().unwrap();
                    assert_eq!(
                        state_guard.fill_available + first_chunk.extent - raw_len,
                        directory_bytes - 1
                    );
                    assert_eq!(
                        state_guard.reads.last(),
                        Some(&(first_chunk.offset, first_chunk.extent))
                    );
                    assert_eq!(
                        reader.work.budget().live_bytes(),
                        baseline + state_guard.held.as_ref().unwrap().len() as u64
                    );
                } else {
                    result.unwrap();
                    assert_eq!(
                        reader.index_status,
                        super::RandomAccessIndexStatus::RebuiltInvalid
                    );
                    assert_eq!(reader.index.len(), fixture.expected.len());
                    assert!(reader.support.is_some());
                }
            }
            {
                let mut state = state.lock().unwrap();
                assert_eq!(
                    state.events.last().unwrap().0,
                    super::KnownChunkPhase::BeforeFree
                );
                assert_eq!(state.events.last().unwrap().2, k);
                drop(state.held.take());
                state.target = None;
            }
            if pressure {
                reader.load_support_and_index_fallback().unwrap();
                assert!(reader.support.is_some());
                assert_eq!(
                    reader.index_status,
                    super::RandomAccessIndexStatus::RebuiltInvalid
                );
            }
            for (name, bytes) in &fixture.expected {
                assert_eq!(
                    &*reader
                        .read_entry(&LogicalPath::from_utf8([name.as_str()]).unwrap())
                        .unwrap()
                        .bytes,
                    bytes
                );
            }
            let budget = reader.work.budget().clone();
            drop(reader);
            assert_eq!(budget.live_bytes(), 0);
        }
    }

    #[test]
    fn known_ordinals_scan_cap_authentication_and_late_count_keep_read_stops() {
        // Membership ownership does not change cap-before-header or
        // duplicate-at-cap precedence in the existing Chunk producer.
        let work = test_work(4096);
        let index = known_index(&work, &[7, 7, u64::MAX]);
        let owner = super::KnownChunkSegments::new(work.budget(), &index).unwrap();
        let held_charge = work.budget().live_bytes();
        let mut discovered = super::ChunkDirectory::new(work.budget(), 2, false);
        let bad_header = [0_u8; 96];
        let error =
            super::discover_chunk(&mut discovered, &bad_header, 0, true, true, slot_locator(0))
                .unwrap_err();
        assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
        assert_eq!(error.code(), super::ReasonCode::RandomAccessPolicyRefused);
        let error = super::discover_chunk(
            &mut discovered,
            &bad_header,
            u64::MAX,
            true,
            true,
            slot_locator(0),
        )
        .unwrap_err();
        assert_eq!(error.class(), super::OutcomeClass::Corrupt);
        assert_eq!(error.code(), super::ReasonCode::SectionStructure);
        assert_eq!(work.budget().live_bytes(), held_charge);
        discovered
            .insert_fallback(slot_digest(7), slot_locator(7))
            .unwrap();
        let error =
            super::discover_chunk(&mut discovered, &bad_header, 1, true, true, slot_locator(7))
                .unwrap_err();
        assert_eq!(error.code(), super::ReasonCode::RandomAccessPolicyRefused);
        assert!(owner.contains(&7));
        assert_eq!(owner.slots.as_slice().len(), 3);
        let fixture = known_fixture(false);
        for cap in [0, 1] {
            let state = Arc::new(Mutex::new(KnownState::default()));
            let mut reader = known_open(
                &fixture,
                &state,
                RandomAccessPolicy {
                    max_chunk_frames_scanned: cap,
                    ..segment_access_policy()
                },
            );
            let payloads = fixture
                .segments
                .iter()
                .enumerate()
                .filter(|(_, s)| s.class == super::SEGMENT_PAYLOAD)
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            let before = state.lock().unwrap().reads.len();
            {
                let _observer = KnownObserverGuard::install(&state);
                let error = reader.load_support_and_index_fallback().unwrap_err();
                assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
                assert_eq!(error.code(), super::ReasonCode::RandomAccessPolicyRefused);
                assert_eq!(
                    error.detail(),
                    "encrypted Chunk-object scan exceeds caller policy"
                );
            }
            let reads = state.lock().unwrap().reads[before..].to_vec();
            assert_eq!(
                reads,
                known_read_oracle(&fixture, payloads[..4 + cap as usize].iter().copied())
            );
            assert!(reader.support.is_none());
            assert_eq!(reader.index.len(), 0);
            assert_eq!(
                reader.index_status,
                super::RandomAccessIndexStatus::RebuiltAbsent
            );
        }
        let state = Arc::new(Mutex::new(KnownState::default()));
        let mut reader = known_open(&fixture, &state, segment_access_policy());
        reader.metadata.descriptor.budget.chunk_count += 1;
        let before = state.lock().unwrap().reads.len();
        {
            let _observer = KnownObserverGuard::install(&state);
            let error = reader.load_support_and_index_fallback().unwrap_err();
            assert_eq!(error.class(), super::OutcomeClass::Nonconforming);
            assert_eq!(error.detail(), "rebuilt encrypted Chunk count mismatch");
        }
        assert_eq!(reader.index.len(), fixture.expected.len());
        assert!(reader.support.is_none());
        assert_eq!(
            state.lock().unwrap().reads[before..],
            known_read_oracle(
                &fixture,
                fixture
                    .segments
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.class == super::SEGMENT_PAYLOAD)
                    .map(|(i, _)| i)
            )
        );
        let mut corrupted = fixture.bytes.clone();
        let support_ordinal = fixture
            .segments
            .iter()
            .position(|s| s.class == super::SEGMENT_PAYLOAD)
            .unwrap();
        let support = fixture.segments[support_ordinal];
        corrupted[(support.offset + super::SEGMENT_HEADER_LEN + super::PROTECTED_HEADER_LEN)
            as usize] ^= 1;
        // CONTROL and terminal authentication are untouched; the support failure
        // is encountered at its original authenticated PAYLOAD read position.
        let corrupted_fixture = SegmentFixture {
            bytes: corrupted,
            ..fixture
        };
        let state = Arc::new(Mutex::new(KnownState::default()));
        let mut reader = known_open(&corrupted_fixture, &state, segment_access_policy());
        let before = state.lock().unwrap().reads.len();
        {
            let _observer = KnownObserverGuard::install(&state);
            let error = reader.load_support_and_index_fallback().unwrap_err();
            assert_eq!(error.class(), super::OutcomeClass::Corrupt);
            assert_eq!(error.code(), super::ReasonCode::CryptoAeadAuthFailed);
        }
        assert_eq!(
            state.lock().unwrap().reads[before..],
            known_read_oracle(&corrupted_fixture, [support_ordinal])
        );
        assert!(reader.support.is_none());
    }

    #[test]
    fn known_ordinals_authenticated_sequence_concat_keeps_original_owner_charge() {
        // This component proof uses a nonempty authenticated EBCS owner to
        // exercise concatenation allocation; the live archive scan/assignment
        // and its retained support owners are covered by the overlap test.
        let fixture = known_fixture(true);
        let state = Arc::new(Mutex::new(KnownState::default()));
        let reader = known_open(&fixture, &state, segment_access_policy());
        let budget = reader.work.budget().clone();
        let baseline = budget.live_bytes();
        let _observer = KnownObserverGuard::install(&state);
        let owner = super::KnownChunkSegments::new(&budget, &reader.index).unwrap();
        let known_bytes = super::KnownChunkSegments::reservation_bytes(reader.index.len()).unwrap();
        assert_eq!(budget.live_bytes(), baseline + known_bytes);
        let locator = reader.index.values().next().unwrap();
        let records = [index_record(
            slot_digest(0).as_bytes(),
            locator.segment_ordinal,
            0,
            locator.fragment_count,
        )];
        let sequence =
            super::wire::sequence_container(super::wire::COLLECTION_INDEX, &records).unwrap();
        let exact =
            super::wire::private_object(super::wire::PRIVATE_OBJECT_SEQUENCE, &sequence).unwrap();
        let fragment = super::wire::PrivateFragmentRef {
            object_id: super::wire::encrypted_object_id(&exact).unwrap(),
            total_len: exact.len() as u64,
            index: 0,
            count: 1,
            offset: 0,
            bytes: &exact,
        };
        let (bytes, segment) = authenticated_test_segment(
            &[fragment],
            &reader.envelope.view().archive_id,
            &reader.keys,
            false,
        );
        let mut session = super::RangeSession::new_crypto(
            Box::new(MemoryRandomReadSource::new(bytes)),
            segment_access_policy(),
            budget.clone(),
        )
        .unwrap();
        let mut options = EncryptedOpenOptions::new(None);
        options.crypto_policy = reader.work.policy();
        let object = super::decrypt_segment(
            &mut session,
            &segment,
            0,
            &reader.envelope.view(),
            &reader.keys,
            reader.padding,
            options,
            super::AccessPurpose::EncryptedPayload,
            &reader.work,
        )
        .unwrap();
        let items = super::wire::WorkSequence::decode_with_context(object, &reader.work).unwrap();
        let retained = budget.live_bytes();
        assert!(retained >= baseline + known_bytes + exact.len() as u64);
        let length = records[0].len() as u64;
        let pressure =
            super::WorkBytes::zeroed(&budget, (budget.available_bytes() - length + 1) as usize)
                .unwrap();
        assert!(pressure.belongs_to(&budget));
        let before = budget.live_bytes();
        let error = items.concatenate_work(&reader.work).unwrap_err();
        assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        assert_eq!(budget.live_bytes(), before);
        assert!(owner.contains(&locator.segment_ordinal));
        assert_eq!(owner.slots.as_slice().len(), reader.index.len());
        drop(pressure);
        let concatenated = items.concatenate_work(&reader.work).unwrap();
        assert_eq!(&*concatenated, records[0]);
        assert_eq!(budget.live_bytes(), retained + length);
        drop(concatenated);
        drop(items);
        drop(session);
        assert_eq!(budget.live_bytes(), baseline + known_bytes);
        drop(owner);
        assert_eq!(budget.live_bytes(), baseline);
        drop(reader);
        assert_eq!(budget.live_bytes(), 0);
    }

    fn append_segment(table: &mut super::SegmentTable, slot: super::SegmentLocator) {
        let vacant = table.probe(slot.class, &slot.salt).unwrap();
        table.append_unique(slot, vacant).unwrap();
    }

    type SegmentSnapshot = (u64, u64, u8, [u8; 16], u32, [u8; 64]);

    fn segment_snapshot(table: &super::SegmentTable) -> Vec<SegmentSnapshot> {
        table
            .as_slice()
            .iter()
            .map(|s| (s.offset, s.extent, s.class, s.salt, s.count, s.header))
            .collect()
    }

    fn assert_segment_capacity(table: &super::SegmentTable) {
        assert_eq!(
            table
                .locators
                .minimum_reservation_bytes(table.capacity - table.len())
                .unwrap(),
            0
        );
        assert_eq!(
            table
                .locators
                .minimum_reservation_bytes(table.capacity - table.len() + 1)
                .unwrap(),
            super::SegmentTable::reservation_bytes(table.capacity + 1).unwrap()
        );
    }

    #[test]
    fn segment_geometric_capacity_preserves_physical_order_and_charges_spares() {
        let work = test_work(64 * 1024);
        let mut table = super::SegmentTable::new(work.budget());
        assert_eq!(work.budget().live_bytes(), 0);
        let mut copied = 0;
        let mut expected = Vec::new();
        for value in 0..17 {
            let previous = table.capacity;
            let pointer = table.as_slice().as_ptr();
            let slot = segment_slot(value, super::SEGMENT_PAYLOAD);
            expected.push((
                slot.offset,
                slot.extent,
                slot.class,
                slot.salt,
                slot.count,
                slot.header,
            ));
            append_segment(&mut table, slot);
            if previous != table.capacity {
                copied += previous;
                assert_eq!(table.capacity, if previous == 0 { 1 } else { previous * 2 });
            } else {
                assert_eq!(table.as_slice().as_ptr(), pointer);
            }
            assert_segment_capacity(&table);
            assert_eq!(
                work.budget().live_bytes(),
                super::SegmentTable::reservation_bytes(table.capacity).unwrap()
            );
            assert_eq!(segment_snapshot(&table), expected);
            assert_eq!(
                table
                    .as_slice()
                    .iter()
                    .map(|s| s.offset / 128)
                    .collect::<Vec<_>>(),
                (0..=value).collect::<Vec<_>>()
            );
            assert!(copied < 2 * table.len());
            if table.len() == 3 {
                assert_eq!(table.capacity, 4);
                assert_eq!(
                    work.budget().live_bytes(),
                    super::SegmentTable::reservation_bytes(4).unwrap()
                );
            }
        }
        assert_eq!(table.capacity, 32);
        assert!(std::mem::size_of::<super::SegmentLocator>() > 8 + 8 + 1 + 16 + 4 + 64);
        drop(table);
        assert_eq!(work.budget().live_bytes(), 0);
    }

    #[test]
    fn segment_growth_requires_full_old_plus_new_original_budget_overlap() {
        let old = super::SegmentTable::reservation_bytes(4).unwrap();
        let new = super::SegmentTable::reservation_bytes(8).unwrap();
        for extra in [0, 1] {
            let work = test_work(37 + old + new - 1 + extra);
            let caller = super::WorkBytes::zeroed(work.budget(), 37).unwrap();
            let shared = work.clone();
            let mut table = super::SegmentTable::new(shared.budget());
            for i in 0..4 {
                append_segment(&mut table, segment_slot(i, 2));
            }
            let before = segment_snapshot(&table);
            let links = table
                .as_slice()
                .iter()
                .map(|s| (s.left, s.right, s.parent, s.height))
                .collect::<Vec<_>>();
            let root = table.root;
            let slot = segment_slot(4, 2);
            let result = table.append_unique(slot, table.probe(slot.class, &slot.salt).unwrap());
            assert_eq!(result.is_ok(), extra == 1);
            assert!(caller.belongs_to(shared.budget()));
            if extra == 0 {
                let error = result.unwrap_err();
                assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
                assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
                assert_eq!(segment_snapshot(&table), before);
                assert_eq!(
                    table
                        .as_slice()
                        .iter()
                        .map(|s| (s.left, s.right, s.parent, s.height))
                        .collect::<Vec<_>>(),
                    links
                );
                assert_eq!(table.root, root);
                assert_eq!(table.capacity, 4);
            } else {
                assert_eq!(&segment_snapshot(&table)[..4], before);
                assert_eq!(table.capacity, 8);
            }
            assert_segment_capacity(&table);
            assert_eq!(
                work.budget().live_bytes(),
                37 + if extra == 0 { old } else { new }
            );
            drop(table);
            assert_eq!(work.budget().live_bytes(), 37);
            drop(caller);
            assert_eq!(work.budget().live_bytes(), 0);
        }
    }

    #[test]
    fn segment_growth_never_retries_an_exact_one_capacity() {
        let old = super::SegmentTable::reservation_bytes(4).unwrap();
        let exact_one = super::SegmentTable::reservation_bytes(5).unwrap();
        assert!(exact_one < super::SegmentTable::reservation_bytes(8).unwrap());
        let work = test_work(old + exact_one);
        let mut table = super::SegmentTable::new(work.budget());
        for i in 0..4 {
            append_segment(&mut table, segment_slot(i, 2));
        }
        let before = segment_snapshot(&table);
        let slot = segment_slot(4, 2);
        let error = table
            .append_unique(slot, table.probe(slot.class, &slot.salt).unwrap())
            .unwrap_err();
        assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        assert_eq!(
            super::SegmentTable::next_capacity(table.capacity).unwrap(),
            8
        );
        assert_eq!(segment_snapshot(&table), before);
        assert_eq!(table.capacity, 4);
        assert_eq!(work.budget().live_bytes(), old);
    }

    fn assert_segment_avl(table: &super::SegmentTable) {
        fn visit(
            table: &super::SegmentTable,
            i: usize,
            parent: usize,
            low: Option<(u8, [u8; 16])>,
            high: Option<(u8, [u8; 16])>,
        ) -> (u32, usize) {
            if i == super::NO_SEGMENT_SLOT {
                return (0, 0);
            }
            assert!(i < table.len());
            let s = table.as_slice()[i];
            let key = (s.class, s.salt);
            assert_eq!(s.parent, parent);
            assert!(low.is_none_or(|v| v < key));
            assert!(high.is_none_or(|v| key < v));
            let (lh, ln) = visit(table, s.left, i, low, Some(key));
            let (rh, rn) = visit(table, s.right, i, Some(key), high);
            assert!(lh.abs_diff(rh) <= 1);
            assert_eq!(s.height, 1 + lh.max(rh));
            (s.height, ln + rn + 1)
        }
        let (height, count) = visit(table, table.root, super::NO_SEGMENT_SLOT, None, None);
        assert_eq!(count, table.len());
        let (mut previous, mut minimum) = (0_usize, 1_usize);
        for _ in 1..height {
            (previous, minimum) = (minimum, 1 + minimum + previous);
        }
        if height != 0 {
            assert!(count >= minimum);
        }
    }

    #[test]
    fn segment_avl_all_rotations_and_probe_match_independent_membership() {
        let orders = [
            vec![3, 2, 1],
            vec![1, 2, 3],
            vec![3, 1, 2],
            vec![1, 3, 2],
            (0..129).collect(),
            (0..129).rev().collect(),
            (0..65).flat_map(|i| [i, 128 - i]).collect(),
        ];
        for order in orders {
            let work = test_work(128 * 1024);
            let mut table = super::SegmentTable::new(work.budget());
            let mut oracle = std::collections::BTreeSet::new();
            let mut inserted = Vec::new();
            for value in order {
                let slot = segment_slot(value, 2);
                let key = (slot.class, slot.salt);
                assert_eq!(
                    table.probe(slot.class, &slot.salt).is_some(),
                    !oracle.contains(&key)
                );
                if oracle.insert(key) {
                    append_segment(&mut table, slot);
                    inserted.push(value);
                }
                assert_segment_avl(&table);
                assert_eq!(
                    table
                        .as_slice()
                        .iter()
                        .map(|s| s.offset / 128)
                        .collect::<Vec<_>>(),
                    inserted
                );
            }
            let held =
                super::WorkBytes::zeroed(work.budget(), work.budget().available_bytes() as usize)
                    .unwrap();
            let pointer = table.as_slice().as_ptr();
            let before = segment_snapshot(&table);
            for value in 0..135 {
                for class in [1, 2] {
                    let slot = segment_slot(value, class);
                    assert_eq!(
                        table.probe(class, &slot.salt).is_none(),
                        oracle.contains(&(class, slot.salt))
                    );
                }
            }
            assert_eq!(table.as_slice().as_ptr(), pointer);
            assert_eq!(segment_snapshot(&table), before);
            assert_eq!(work.budget().available_bytes(), 0);
            assert_segment_capacity(&table);
            drop(held);
        }
    }

    struct SegmentFixture {
        bytes: Vec<u8>,
        identity: XWingIdentity,
        expected: Vec<(String, [u8; 8])>,
        footer: super::Footer,
        segments: Vec<super::SegmentLocator>,
    }

    fn segment_fixture(include_index: bool) -> SegmentFixture {
        let directory = TestDir::new();
        let expected = (0_u64..64)
            .map(|i| {
                let name = format!("slot{i:02}");
                let bytes = (i + 1).to_be_bytes();
                std::fs::write(directory.path().join(&name), bytes).unwrap();
                (name, bytes)
            })
            .collect::<Vec<_>>();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: &[recipient],
                padding: PaddingMode::None,
                include_index,
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let opened = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(encrypted.bytes.clone()),
            segment_access_policy(),
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
        )
        .unwrap();
        let segments = opened.segments.as_slice().to_vec();
        let footer = super::parse_footer(
            &encrypted.bytes[encrypted.bytes.len() - 192..],
            encrypted.bytes.len() as u64,
        )
        .unwrap();
        assert_eq!(segments.last().unwrap().offset, footer.terminal_offset);
        SegmentFixture {
            bytes: encrypted.bytes,
            identity,
            expected,
            footer,
            segments,
        }
    }

    fn segment_access_policy() -> RandomAccessPolicy {
        RandomAccessPolicy {
            max_cached_bytes: 0,
            ..RandomAccessPolicy::default()
        }
    }

    const SEGMENT_FINITE_LIMIT: u64 = 8 * 1024 * 1024;

    #[derive(Default)]
    struct SegmentPressure {
        budget: Option<super::WorkBudget>,
        target: Option<(u64, u64)>,
        leave: Option<u64>,
        held: Option<super::WorkBytes>,
        release_at_next_fill: bool,
        available_before_release: Option<u64>,
        original_live: u64,
        live_during_fill: u64,
        available_during_fill: u64,
        hits: usize,
        reads: Vec<(u64, u64)>,
    }

    std::thread_local! {
        static SEGMENT_PRESSURE_STATE: std::cell::RefCell<Option<Arc<Mutex<SegmentPressure>>>> =
            const { std::cell::RefCell::new(None) };
    }

    fn observe_segment_budget(budget: &super::WorkBudget) {
        SEGMENT_PRESSURE_STATE.with(|state| {
            let shared = state.borrow().as_ref().unwrap().clone();
            let mut state = shared.lock().unwrap();
            assert!(state.budget.is_none());
            state.budget = Some(budget.clone());
        });
    }

    struct SegmentObserverGuard {
        previous: Option<fn(&super::WorkBudget)>,
        previous_state: Option<Arc<Mutex<SegmentPressure>>>,
    }

    impl SegmentObserverGuard {
        fn install(state: Arc<Mutex<SegmentPressure>>) -> Self {
            Self {
                previous: super::SEGMENT_TABLE_BUDGET_OBSERVER
                    .with(|cell| cell.replace(Some(observe_segment_budget))),
                previous_state: SEGMENT_PRESSURE_STATE.with(|cell| cell.replace(Some(state))),
            }
        }
    }

    impl Drop for SegmentObserverGuard {
        fn drop(&mut self) {
            super::SEGMENT_TABLE_BUDGET_OBSERVER.with(|cell| cell.set(self.previous));
            SEGMENT_PRESSURE_STATE.with(|cell| *cell.borrow_mut() = self.previous_state.take());
        }
    }

    struct SegmentSource {
        source: MemoryRandomReadSource,
        state: Arc<Mutex<SegmentPressure>>,
    }

    impl RandomReadSource for SegmentSource {
        fn len(&self) -> crate::diagnostics::Result<u64> {
            self.source.len()
        }
        fn revision(&self) -> crate::diagnostics::Result<SourceRevision> {
            self.source.revision()
        }
        fn read_exact_at(&self, _: u64, _: u64) -> crate::diagnostics::Result<Vec<u8>> {
            panic!("segment caller must use bounded fill");
        }
        fn read_exact_at_into(
            &self,
            offset: u64,
            dst: &mut [u8],
        ) -> crate::diagnostics::Result<()> {
            let mut state = self.state.lock().unwrap();
            state.reads.push((offset, dst.len() as u64));
            if state.hits == 1
                && state.release_at_next_fill
                && state.target != Some((offset, dst.len() as u64))
            {
                state.available_before_release =
                    Some(state.budget.as_ref().unwrap().available_bytes());
                drop(state.held.take());
                state.release_at_next_fill = false;
            }
            if state.target == Some((offset, dst.len() as u64)) {
                state.hits += 1;
                let budget = state.budget.as_ref().unwrap().clone();
                state.original_live = budget.live_bytes();
                if let Some(leave) = state.leave {
                    let held = budget
                        .available_bytes()
                        .checked_sub(leave)
                        .expect("finite cap admits all earlier owners");
                    state.held = Some(super::WorkBytes::zeroed(&budget, held as usize)?);
                    assert!(state.held.as_ref().unwrap().belongs_to(&budget));
                }
                state.live_during_fill = budget.live_bytes();
                state.available_during_fill = budget.available_bytes();
            }
            drop(state);
            self.source.read_exact_at_into(offset, dst)
        }
    }

    fn segment_source(bytes: Vec<u8>, state: &Arc<Mutex<SegmentPressure>>) -> SegmentSource {
        SegmentSource {
            source: MemoryRandomReadSource::new(bytes),
            state: Arc::clone(state),
        }
    }

    fn segment_target(fixture: &SegmentFixture) -> usize {
        let target = [8_usize, 16, 32]
            .into_iter()
            .find(|i| *i + 1 < fixture.segments.len())
            .unwrap();
        assert!(target.is_power_of_two());
        target
    }

    fn segment_inspect(
        bytes: Vec<u8>,
        state: &Arc<Mutex<SegmentPressure>>,
        policy: RandomAccessPolicy,
        crypto: crate::crypto::CryptoPolicy,
    ) -> crate::diagnostics::Result<super::EncryptedRandomPublicInspection> {
        let _observer = SegmentObserverGuard::install(Arc::clone(state));
        super::inspect_indexed_random_encrypted_public(segment_source(bytes, state), policy, crypto)
    }

    fn assert_segment_pressure_teardown(state: &Arc<Mutex<SegmentPressure>>) {
        let mut state = state.lock().unwrap();
        let budget = state.budget.as_ref().unwrap().clone();
        let held = state.held.as_ref().map_or(0, |h| h.len() as u64);
        assert_eq!(budget.live_bytes(), held);
        drop(state.held.take());
        assert_eq!(budget.live_bytes(), 0);
        assert!(super::SEGMENT_TABLE_BUDGET_OBSERVER.with(|cell| cell.get().is_none()));
        assert!(SEGMENT_PRESSURE_STATE.with(|cell| cell.borrow().is_none()));
    }

    #[test]
    fn segment_class_domains_and_public_walk_duplicate_rules() {
        let work = test_work(4096);
        let mut table = super::SegmentTable::new(work.budget());
        append_segment(&mut table, segment_slot(5, 1));
        append_segment(&mut table, segment_slot(5, 2));
        assert!(table.probe(1, &segment_slot(5, 1).salt).is_none());
        assert!(table.probe(2, &segment_slot(5, 2).salt).is_none());
        let f = segment_fixture(true);
        let control = f.segments.iter().find(|s| s.class == 1).unwrap();
        let payload = f.segments.iter().find(|s| s.class == 2).unwrap();
        let mut changed = f.bytes.clone();
        changed[payload.offset as usize + 16..payload.offset as usize + 32]
            .copy_from_slice(&control.salt);
        let state = Arc::new(Mutex::new(SegmentPressure::default()));
        assert!(
            segment_inspect(
                changed,
                &state,
                segment_access_policy(),
                crate::crypto::CryptoPolicy::default()
            )
            .is_ok()
        );
        assert_segment_pressure_teardown(&state);
        let mut changed = f.bytes;
        let two = f
            .segments
            .iter()
            .filter(|s| s.class == 2)
            .take(2)
            .collect::<Vec<_>>();
        changed[two[1].offset as usize + 16..two[1].offset as usize + 32]
            .copy_from_slice(&two[0].salt);
        let state = Arc::new(Mutex::new(SegmentPressure::default()));
        let error = segment_inspect(
            changed,
            &state,
            segment_access_policy(),
            crate::crypto::CryptoPolicy::default(),
        )
        .unwrap_err();
        assert_eq!(error.class(), super::OutcomeClass::Nonconforming);
        assert_eq!(
            error.code(),
            super::ReasonCode::CryptoSegmentStructureInvalid
        );
        assert_eq!(error.detail(), "segment salt is reused in one key domain");
        assert_eq!(
            state.lock().unwrap().reads.last(),
            Some(&(two[1].offset, 64))
        );
        assert_segment_pressure_teardown(&state);
    }

    #[test]
    fn segment_precedence_matrix_keeps_exact_diagnostic_and_read_stop() {
        let f = segment_fixture(true);
        let target = segment_target(&f);
        let slot = f.segments[target];
        let previous = f.segments[..target]
            .iter()
            .find(|s| s.class == slot.class)
            .unwrap();
        let crossing = f.footer.segments_offset + f.footer.segments_len - slot.offset + 1;
        assert!(crossing < SEGMENT_FINITE_LIMIT);
        let new = super::SegmentTable::reservation_bytes(target * 2).unwrap();
        for case in 0..7 {
            let mut bytes = f.bytes.clone();
            let mut crypto = crate::crypto::CryptoPolicy {
                max_working_memory_bytes: SEGMENT_FINITE_LIMIT,
                ..crate::crypto::CryptoPolicy::default()
            };
            let header = &mut bytes[slot.offset as usize..slot.offset as usize + 64];
            if matches!(case, 0..=3 | 5) {
                header[16..32].copy_from_slice(&previous.salt);
            }
            let (code, detail) = match case {
                0 => {
                    header[7] = 1;
                    (
                        super::ReasonCode::CryptoSegmentStructureInvalid,
                        "SegmentHeader is noncanonical or reordered",
                    )
                }
                1 => {
                    let cap = f.segments[..target]
                        .iter()
                        .map(|s| s.count)
                        .max()
                        .unwrap()
                        .max(1);
                    crypto.max_messages_per_segment = cap;
                    header[32..36].copy_from_slice(&(cap + 1).to_be_bytes());
                    (
                        super::ReasonCode::CryptoResourcePolicyRefused,
                        "encrypted segment declaration exceeds caller policy",
                    )
                }
                2 => {
                    header[48..56].copy_from_slice(&(SEGMENT_FINITE_LIMIT + 1).to_be_bytes());
                    (
                        super::ReasonCode::CryptoResourcePolicyRefused,
                        "encrypted segment declaration exceeds caller policy",
                    )
                }
                3..=4 => {
                    header[48..56].copy_from_slice(&crossing.to_be_bytes());
                    if case == 3 {
                        (
                            super::ReasonCode::CryptoSegmentStructureInvalid,
                            "segment salt is reused in one key domain",
                        )
                    } else {
                        (
                            super::ReasonCode::TruncatedStream,
                            "segment extent exceeds ENCRYPTED_SEGMENTS",
                        )
                    }
                }
                5 => (
                    super::ReasonCode::CryptoSegmentStructureInvalid,
                    "segment salt is reused in one key domain",
                ),
                _ => (
                    super::ReasonCode::CryptoResourcePolicyRefused,
                    "managed crypto replacement and required headroom exceed caller policy",
                ),
            };
            let state = Arc::new(Mutex::new(SegmentPressure {
                target: Some((slot.offset, 64)),
                leave: if case >= 5 { Some(new - 1) } else { None },
                ..SegmentPressure::default()
            }));
            let error =
                segment_inspect(bytes, &state, segment_access_policy(), crypto).unwrap_err();
            let class = match case {
                0 | 3 | 5 => super::OutcomeClass::Nonconforming,
                4 => super::OutcomeClass::Truncated,
                _ => super::OutcomeClass::PolicyRefused,
            };
            assert_eq!(error.class(), class, "case{case}");
            assert_eq!(error.code(), code, "case{case}");
            assert_eq!(error.detail(), detail, "case{case}");
            let state_ref = state.lock().unwrap();
            assert_eq!(state_ref.hits, 1);
            assert_eq!(state_ref.reads.last(), Some(&(slot.offset, 64)));
            drop(state_ref);
            assert_segment_pressure_teardown(&state);
        }
    }

    #[test]
    fn segment_both_caller_ceilings_refuse_before_first_disallowed_header() {
        let f = segment_fixture(true);
        let n = f.segments.len();
        for crypto_ceiling in [false, true] {
            for cap in [0, 1, 3, n - 1, n] {
                let state = Arc::new(Mutex::new(SegmentPressure::default()));
                let mut policy = segment_access_policy();
                let mut crypto = crate::crypto::CryptoPolicy::default();
                if crypto_ceiling {
                    crypto.max_segments = cap as u64;
                } else {
                    policy.max_encrypted_segment_header_walk = cap as u64;
                }
                let result = segment_inspect(f.bytes.clone(), &state, policy, crypto);
                assert_eq!(result.is_ok(), cap == n);
                if cap != n {
                    let error = result.unwrap_err();
                    assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
                    assert_eq!(error.code(), super::ReasonCode::RandomAccessPolicyRefused);
                    assert_eq!(
                        error.detail(),
                        "encrypted SegmentHeader walk exceeds caller policy"
                    );
                }
                let reads = state.lock().unwrap().reads.clone();
                let headers = reads
                    .iter()
                    .filter(|(o, l)| *l == 64 && f.segments.iter().any(|s| s.offset == *o))
                    .copied()
                    .collect::<Vec<_>>();
                assert_eq!(
                    headers,
                    f.segments[..cap]
                        .iter()
                        .map(|s| (s.offset, 64))
                        .collect::<Vec<_>>()
                );
                assert!(reads.contains(&(f.footer.segments_offset, 64)));
                assert_segment_pressure_teardown(&state);
            }
        }
    }

    #[test]
    fn segment_authenticated_finite_memory_inspection_uses_original_counter() {
        let f = segment_fixture(true);
        let target = segment_target(&f);
        let new = super::SegmentTable::reservation_bytes(target * 2).unwrap();
        let ample = Arc::new(Mutex::new(SegmentPressure {
            target: Some((f.segments[target].offset, 64)),
            ..SegmentPressure::default()
        }));
        let report = segment_inspect(
            f.bytes.clone(),
            &ample,
            segment_access_policy(),
            crate::crypto::CryptoPolicy {
                max_working_memory_bytes: SEGMENT_FINITE_LIMIT,
                ..crate::crypto::CryptoPolicy::default()
            },
        )
        .unwrap();
        assert!(!report.whole_archive_verified);
        assert_eq!(report.public.segment_count, Some(f.segments.len() as u64));
        assert_eq!(
            report.source_revision,
            MemoryRandomReadSource::new(f.bytes.clone())
                .revision()
                .unwrap()
        );
        let prefix = {
            let s = ample.lock().unwrap();
            let position = s
                .reads
                .iter()
                .position(|r| *r == (f.segments[target].offset, 64))
                .unwrap();
            s.reads[..=position].to_vec()
        };
        assert_segment_pressure_teardown(&ample);
        let state = Arc::new(Mutex::new(SegmentPressure {
            target: Some((f.segments[target].offset, 64)),
            leave: Some(new - 1),
            ..SegmentPressure::default()
        }));
        let crypto = crate::crypto::CryptoPolicy {
            max_working_memory_bytes: SEGMENT_FINITE_LIMIT,
            ..crate::crypto::CryptoPolicy::default()
        };
        let error =
            segment_inspect(f.bytes.clone(), &state, segment_access_policy(), crypto).unwrap_err();
        assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        assert_eq!(
            error.detail(),
            "managed crypto replacement and required headroom exceed caller policy"
        );
        let s = state.lock().unwrap();
        assert_eq!(s.available_during_fill, new - 1);
        assert_eq!(s.live_during_fill, SEGMENT_FINITE_LIMIT - (new - 1));
        assert!(s.original_live >= super::SegmentTable::reservation_bytes(target).unwrap() + 128);
        assert_eq!(s.reads, prefix);
        println!(
            "segment inspect immutable_M={SEGMENT_FINITE_LIMIT} ordinal={target} old={} new={new} live_at_fill={} available={} revision_headroom=0",
            super::SegmentTable::reservation_bytes(target).unwrap(),
            s.live_during_fill,
            s.available_during_fill
        );
        drop(s);
        assert_segment_pressure_teardown(&state);
        // A separate immutable-cap session leaves exactly the full new owner.
        // Release the caller only at the following read: reaching that callback
        // proves target growth succeeded while the held lease was still live.
        let positive = Arc::new(Mutex::new(SegmentPressure {
            target: Some((f.segments[target].offset, 64)),
            leave: Some(new),
            release_at_next_fill: true,
            ..SegmentPressure::default()
        }));
        let exact = segment_inspect(f.bytes, &positive, segment_access_policy(), crypto).unwrap();
        assert_eq!(exact, report);
        assert_eq!(
            positive.lock().unwrap().available_before_release,
            Some(super::SegmentTable::reservation_bytes(target).unwrap())
        );
        assert_segment_pressure_teardown(&positive);
    }

    #[test]
    fn segment_authenticated_finite_memory_open_stops_before_control() {
        for include_index in [true, false] {
            let f = segment_fixture(include_index);
            let target = segment_target(&f);
            let new = super::SegmentTable::reservation_bytes(target * 2).unwrap();
            let state = Arc::new(Mutex::new(SegmentPressure {
                target: Some((f.segments[target].offset, 64)),
                leave: Some(new - 1),
                ..SegmentPressure::default()
            }));
            let error = {
                let _observer = SegmentObserverGuard::install(Arc::clone(&state));
                let mut options = EncryptedOpenOptions::new(Some(Unlock::Identity(&f.identity)));
                options.crypto_policy.max_working_memory_bytes = SEGMENT_FINITE_LIMIT;
                match open_indexed_random_encrypted(
                    segment_source(f.bytes.clone(), &state),
                    segment_access_policy(),
                    options,
                ) {
                    Ok(_) => panic!("original-counter full replacement must refuse"),
                    Err(e) => e,
                }
            };
            assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
            assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
            assert_eq!(
                error.detail(),
                "managed crypto replacement and required headroom exceed caller policy"
            );
            {
                let s = state.lock().unwrap();
                assert_eq!(s.hits, 1);
                assert_eq!(s.reads.last(), Some(&(f.segments[target].offset, 64)));
                assert_eq!(s.available_during_fill, new - 1);
                assert!(s.reads.iter().all(|(o, l)| {
                    !f.segments
                        .iter()
                        .any(|segment| segment.offset == *o && segment.extent == *l)
                }));
                println!(
                    "segment open index={include_index} immutable_M={SEGMENT_FINITE_LIMIT} ordinal={target} old={} new={new} live_at_fill={} available={} revision_headroom=0",
                    super::SegmentTable::reservation_bytes(target).unwrap(),
                    s.live_during_fill,
                    s.available_during_fill
                );
            }
            assert_segment_pressure_teardown(&state);
            let positive = Arc::new(Mutex::new(SegmentPressure {
                target: Some((f.segments[target].offset, 64)),
                leave: Some(new),
                release_at_next_fill: true,
                ..SegmentPressure::default()
            }));
            let mut opened = {
                let _observer = SegmentObserverGuard::install(Arc::clone(&positive));
                let mut options = EncryptedOpenOptions::new(Some(Unlock::Identity(&f.identity)));
                options.crypto_policy.max_working_memory_bytes = SEGMENT_FINITE_LIMIT;
                open_indexed_random_encrypted(
                    segment_source(f.bytes, &positive),
                    segment_access_policy(),
                    options,
                )
                .unwrap()
            };
            assert_eq!(
                positive.lock().unwrap().available_before_release,
                Some(super::SegmentTable::reservation_bytes(target).unwrap())
            );
            let pointer = opened.segments.as_slice().as_ptr();
            for (name, bytes) in f.expected {
                let read = opened
                    .read_entry(&LogicalPath::from_utf8([name.as_str()]).unwrap())
                    .unwrap();
                assert_eq!(&*read.bytes, bytes.as_slice());
                assert_eq!(opened.segments.as_slice().as_ptr(), pointer);
                assert_eq!(
                    read.report.index_status,
                    if include_index {
                        super::RandomAccessIndexStatus::PresentValid
                    } else {
                        super::RandomAccessIndexStatus::RebuiltAbsent
                    }
                );
            }
            drop(opened);
            assert_segment_pressure_teardown(&positive);
        }
    }

    #[test]
    fn segment_retained_archive_borrowers_keep_one_owner_and_release_to_caller() {
        let f = segment_fixture(true);
        let state = Arc::new(Mutex::new(SegmentPressure::default()));
        let mut opened = {
            let _observer = SegmentObserverGuard::install(Arc::clone(&state));
            open_indexed_random_encrypted(
                segment_source(f.bytes, &state),
                segment_access_policy(),
                EncryptedOpenOptions::new(Some(Unlock::Identity(&f.identity))),
            )
            .unwrap()
        };
        let budget = state.lock().unwrap().budget.as_ref().unwrap().clone();
        let caller = super::WorkBytes::zeroed(&budget, 29).unwrap();
        let pointer = opened.segments.as_slice().as_ptr();
        let metadata = opened
            .segments
            .as_slice()
            .iter()
            .map(|s| (s.offset, s.extent, s.header))
            .collect::<Vec<_>>();
        assert_eq!(
            metadata,
            f.segments
                .iter()
                .map(|s| (s.offset, s.extent, s.header))
                .collect::<Vec<_>>()
        );
        assert_segment_capacity(&opened.segments);
        let table_charge =
            super::SegmentTable::reservation_bytes(opened.segments.capacity).unwrap();
        assert!(budget.live_bytes() >= table_charge + 29);
        opened.load_support_and_index_fallback().unwrap();
        for (name, bytes) in f.expected {
            let read = opened
                .read_entry(&LogicalPath::from_utf8([name.as_str()]).unwrap())
                .unwrap();
            assert_eq!(&*read.bytes, bytes.as_slice());
            assert_eq!(opened.segments.as_slice().as_ptr(), pointer);
            assert!(budget.live_bytes() >= table_charge + 29);
        }
        drop(opened);
        assert_eq!(budget.live_bytes(), 29);
        drop(caller);
        assert_eq!(budget.live_bytes(), 0);
    }

    #[test]
    fn segment_checked_boundaries_refuse_before_allocation_and_preserve_lifetime() {
        let work = test_work(31);
        let caller = super::WorkBytes::zeroed(work.budget(), 31).unwrap();
        for capacity in [usize::MAX, usize::MAX / 2 + 1] {
            let error = super::SegmentTable::next_capacity(capacity).unwrap_err();
            assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
            assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        }
        for capacity in [
            usize::MAX,
            usize::MAX / std::mem::size_of::<super::SegmentLocator>(),
        ] {
            let error = super::SegmentTable::reservation_bytes(capacity).unwrap_err();
            assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
            assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        }
        for error in [
            super::SegmentTable::next_height(u32::MAX).unwrap_err(),
            super::SegmentTable::next_index(usize::MAX).unwrap_err(),
            super::SegmentTable::next_index(usize::MAX - 1).unwrap_err(),
        ] {
            assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
            assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        }
        assert_eq!(super::SegmentTable::next_index(0).unwrap(), 1);
        assert_eq!(work.budget().live_bytes(), 31);
        drop(caller);
        let work = test_work(4096);
        let caller = super::WorkBytes::zeroed(work.budget(), 31).unwrap();
        let unwind = std::panic::catch_unwind(|| {
            let mut table = super::SegmentTable::new(work.budget());
            for i in 0..5 {
                append_segment(&mut table, segment_slot(i, 2));
            }
            panic!("segment table owned unwind");
        });
        assert!(unwind.is_err());
        assert_eq!(work.budget().live_bytes(), 31);
        drop(caller);
        assert_eq!(work.budget().live_bytes(), 0);
        assert!(super::SEGMENT_TABLE_BUDGET_OBSERVER.with(|cell| cell.get().is_none()));
        let state = Arc::new(Mutex::new(SegmentPressure::default()));
        let unwind = std::panic::catch_unwind(|| {
            let _guard = SegmentObserverGuard::install(Arc::clone(&state));
            let work = test_work(0);
            let _table = super::SegmentTable::new(work.budget());
            panic!("observer owned unwind");
        });
        assert!(unwind.is_err());
        assert!(super::SEGMENT_TABLE_BUDGET_OBSERVER.with(|cell| cell.get().is_none()));
        assert!(SEGMENT_PRESSURE_STATE.with(|cell| cell.borrow().is_none()));
        assert_segment_pressure_teardown(&state);
    }

    #[test]
    fn segment_framing_errors_keep_terminal_and_section_validation_order() {
        let f = segment_fixture(true);
        for case in 0..8 {
            let mut footer = f.footer;
            let mut bytes = f.bytes.clone();
            let expected = match case {
                0 => {
                    footer.segments_len += 1;
                    "ENCRYPTED_SEGMENTS extent mismatch"
                }
                1 => {
                    footer.segments_len = 64;
                    bytes[footer.segments_offset as usize + 16
                        ..footer.segments_offset as usize + 24]
                        .copy_from_slice(&0_u64.to_be_bytes());
                    "encrypted segment sequence is incomplete"
                }
                2 => {
                    bytes[f.segments[0].offset as usize + 48..f.segments[0].offset as usize + 56]
                        .copy_from_slice(&111_u64.to_be_bytes());
                    "SegmentHeader count/extent is invalid"
                }
                3 => {
                    footer.terminal_offset += 1;
                    "terminal segment/footer locators are invalid"
                }
                4 => {
                    footer.archive_final_offset += 1;
                    "terminal segment/footer locators are invalid"
                }
                5 => {
                    bytes[f.footer.terminal_offset as usize + 32
                        ..f.footer.terminal_offset as usize + 36]
                        .copy_from_slice(&2_u32.to_be_bytes());
                    "terminal segment/footer locators are invalid"
                }
                6 => {
                    bytes[f.segments[0].offset as usize + 32..f.segments[0].offset as usize + 36]
                        .copy_from_slice(&(1_u32 << 20).to_be_bytes());
                    "SegmentHeader count/extent is invalid"
                }
                _ => {
                    bytes[f.footer.terminal_offset as usize + 6] = super::SEGMENT_PAYLOAD;
                    "terminal segment/footer locators are invalid"
                }
            };
            let work = test_work(SEGMENT_FINITE_LIMIT);
            let state = Arc::new(Mutex::new(SegmentPressure::default()));
            let mut session = super::RangeSession::new_crypto(
                Box::new(segment_source(bytes, &state)),
                segment_access_policy(),
                work.budget().clone(),
            )
            .unwrap();
            let error = match super::walk_segments(
                &mut session,
                &footer,
                EncryptedOpenOptions::new(None),
                &work,
            ) {
                Ok(_) => panic!("damaged framing must refuse"),
                Err(e) => e,
            };
            let (class, code) = if case == 1 {
                (
                    super::OutcomeClass::Truncated,
                    super::ReasonCode::TruncatedStream,
                )
            } else {
                (
                    super::OutcomeClass::Nonconforming,
                    super::ReasonCode::CryptoSegmentStructureInvalid,
                )
            };
            assert_eq!(error.class(), class, "case{case}");
            assert_eq!(error.code(), code, "case{case}");
            assert_eq!(error.detail(), expected, "case{case}");
            let reads = state.lock().unwrap().reads.clone();
            if case <= 1 {
                assert_eq!(reads, [(f.footer.segments_offset, 64)]);
            }
            if matches!(case, 2 | 6) {
                assert_eq!(
                    reads,
                    [(f.footer.segments_offset, 64), (f.segments[0].offset, 64)]
                );
            }
            drop(session);
            assert_eq!(work.budget().live_bytes(), 0);
        }
    }

    #[test]
    fn segment_sparse_address_overflow_refuses_at_original_checked_positions() {
        struct SparseSource {
            offset: u64,
            section: [u8; 64],
            segment: [u8; 64],
            reads: Arc<Mutex<Vec<(u64, u64)>>>,
        }
        impl RandomReadSource for SparseSource {
            fn len(&self) -> crate::diagnostics::Result<u64> {
                Ok(u64::MAX)
            }
            fn revision(&self) -> crate::diagnostics::Result<SourceRevision> {
                Ok(SourceRevision::Memory {
                    length: u64::MAX,
                    sha256: [0; 32],
                })
            }
            fn read_exact_at(&self, _: u64, _: u64) -> crate::diagnostics::Result<Vec<u8>> {
                panic!("sparse framing fixture requires bounded fill");
            }
            fn read_exact_at_into(
                &self,
                offset: u64,
                dst: &mut [u8],
            ) -> crate::diagnostics::Result<()> {
                self.reads.lock().unwrap().push((offset, dst.len() as u64));
                assert_eq!(dst.len(), 64);
                if offset == self.offset {
                    dst.copy_from_slice(&self.section);
                } else {
                    assert_eq!(offset, self.offset + 64);
                    dst.copy_from_slice(&self.segment);
                }
                Ok(())
            }
        }
        let f = segment_fixture(true);
        for section_overflow in [true, false] {
            let mut section: [u8; 64] = f.bytes
                [f.footer.segments_offset as usize..f.footer.segments_offset as usize + 64]
                .try_into()
                .unwrap();
            section[16..24].copy_from_slice(&128_u64.to_be_bytes());
            let mut segment = f.segments[0].header;
            segment[48..56].copy_from_slice(&256_u64.to_be_bytes());
            let offset = if section_overflow {
                u64::MAX - 128
            } else {
                u64::MAX - 256
            };
            let mut footer = f.footer;
            footer.segments_offset = offset;
            footer.segments_len = 192;
            let reads = Arc::new(Mutex::new(Vec::new()));
            let work = test_work(SEGMENT_FINITE_LIMIT);
            let source = SparseSource {
                offset,
                section,
                segment,
                reads: Arc::clone(&reads),
            };
            let mut session = super::RangeSession::new_crypto(
                Box::new(source),
                segment_access_policy(),
                work.budget().clone(),
            )
            .unwrap();
            let error = match super::walk_segments(
                &mut session,
                &footer,
                EncryptedOpenOptions::new(None),
                &work,
            ) {
                Ok(_) => panic!("sparse extent must overflow"),
                Err(e) => e,
            };
            assert_eq!(error.class(), super::OutcomeClass::Nonconforming);
            assert_eq!(
                error.code(),
                super::ReasonCode::CryptoSegmentStructureInvalid
            );
            assert_eq!(
                error.detail(),
                if section_overflow {
                    "ENCRYPTED_SEGMENTS extent overflow"
                } else {
                    "segment extent overflow"
                }
            );
            assert_eq!(
                reads.lock().unwrap().len(),
                if section_overflow { 1 } else { 2 }
            );
            drop(session);
            assert_eq!(work.budget().live_bytes(), 0);
        }
    }

    fn slot_digest(value: u64) -> super::Digest {
        let mut digest = [0; 32];
        digest[24..].copy_from_slice(&value.to_be_bytes());
        super::Digest::from_bytes(digest)
    }

    fn slot_locator(value: u64) -> super::EncryptedChunkLocator {
        super::EncryptedChunkLocator {
            segment_ordinal: value,
            fragment_count: 1,
        }
    }

    fn test_segment(class: u8, count: u32) -> super::SegmentLocator {
        super::SegmentLocator {
            offset: 0,
            extent: 0,
            class,
            salt: [0; 16],
            count,
            header: [0; 64],
            left: super::NO_SEGMENT_SLOT,
            right: super::NO_SEGMENT_SLOT,
            parent: super::NO_SEGMENT_SLOT,
            height: 1,
        }
    }

    fn test_work(limit: u64) -> super::CryptoWorkContext {
        super::CryptoWorkContext::new(crate::crypto::CryptoPolicy {
            max_working_memory_bytes: limit,
            ..crate::crypto::CryptoPolicy::default()
        })
    }

    fn chunk_header(digest: super::Digest) -> [u8; 64] {
        let mut header = [0; 64];
        header[..4].copy_from_slice(b"EBCH");
        header[4..6].copy_from_slice(&1_u16.to_be_bytes());
        header[16..48].copy_from_slice(digest.as_bytes());
        header
    }

    fn index_record(digest: &[u8], ordinal: u64, counter: u64, fragments: u32) -> Vec<u8> {
        let mut record = crate::canonical::RecordBuilder::new(super::wire::RECORD_ENCRYPTED_INDEX);
        record
            .bytes(1, digest)
            .unwrap()
            .u64(2, ordinal)
            .unwrap()
            .u64(3, counter)
            .unwrap()
            .u32(4, fragments)
            .unwrap();
        record.finish().unwrap()
    }

    fn index_sequence(
        records: &[Vec<u8>],
        work: &super::CryptoWorkContext,
    ) -> super::wire::WorkSequence {
        let sequence =
            super::wire::sequence_container(super::wire::COLLECTION_INDEX, records).unwrap();
        let exact =
            super::wire::private_object(super::wire::PRIVATE_OBJECT_SEQUENCE, &sequence).unwrap();
        let mut raw = super::WorkBytes::zeroed(work.budget(), exact.len()).unwrap();
        raw.copy_from_slice(&exact);
        let object = super::wire::WorkObject::verified(
            raw,
            super::wire::encrypted_object_id(&exact).unwrap(),
            work,
        )
        .unwrap();
        super::wire::WorkSequence::decode_with_context(object, work).unwrap()
    }

    #[test]
    fn chunk_slot_layout_bounds_and_lazy_empty_owners() {
        assert_eq!(super::ChunkDirectory::reservation_bytes(0).unwrap(), 0);
        for bound in [
            usize::MAX,
            usize::MAX / std::mem::size_of::<super::ChunkSlot>(),
        ] {
            let error = super::ChunkDirectory::reservation_bytes(bound).unwrap_err();
            assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        }
        let work = test_work(0);
        let segments = [
            test_segment(super::SEGMENT_CONTROL, 1),
            test_segment(super::SEGMENT_PAYLOAD, 1),
            test_segment(super::SEGMENT_PAYLOAD, 1),
            test_segment(super::SEGMENT_PAYLOAD, 1),
        ];
        for (cap, expected) in [(0, 0), (1, 1), (2, 2), (3, 3), (u64::MAX, 3)] {
            assert_eq!(
                super::fallback_slot_bound(&segments, cap).unwrap(),
                expected
            );
        }
        assert_eq!(super::fallback_slot_bound(&[], u64::MAX).unwrap(), 0);
        // Support-only/PresentValid skipped segments never call insertion. Even
        // an unaffordable conservative bound holds no lease before that point.
        let empty = super::ChunkDirectory::new(work.budget(), 3, false);
        assert!(!empty.reserved);
        assert_eq!(work.budget().live_bytes(), 0);
        drop(empty);
        let mut zero = super::ChunkDirectory::new(work.budget(), 0, true);
        zero.reserve_once().unwrap();
        assert_eq!(zero.len(), 0);
        assert_eq!(work.budget().live_bytes(), 0);
    }

    #[test]
    fn fixed_chunk_slots_charge_padding_unused_capacity_and_original_counter() {
        let bytes = super::ChunkDirectory::reservation_bytes(7).unwrap();
        assert_eq!(
            bytes,
            std::alloc::Layout::array::<super::ChunkSlot>(7)
                .unwrap()
                .size() as u64
        );
        assert!(std::mem::size_of::<super::ChunkSlot>() > 32 + 8 + 4);
        let work = test_work(bytes + 19);
        let other = super::WorkBytes::zeroed(work.budget(), 19).unwrap();
        let shared = work.clone();
        let mut table = super::ChunkDirectory::new(shared.budget(), 7, false);
        table
            .insert_fallback(slot_digest(2), slot_locator(2))
            .unwrap();
        assert_eq!(table.slots.as_slice().len(), 1);
        assert_eq!(work.budget().live_bytes(), bytes + 19);
        let backing = table.slots.as_slice().as_ptr();
        for i in 3..9 {
            table
                .insert_fallback(slot_digest(i), slot_locator(i))
                .unwrap();
        }
        assert_eq!(table.len(), 7);
        assert_eq!(table.slots.as_slice().as_ptr(), backing);
        let error = table
            .insert_fallback(slot_digest(9), slot_locator(9))
            .unwrap_err();
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        assert_eq!(table.slots.as_slice().as_ptr(), backing);
        assert_eq!(work.budget().live_bytes(), bytes + 19);
        // Move, duplicate replacement and finishing retain one backing/lease.
        let mut moved = table;
        moved
            .insert_fallback(slot_digest(2), slot_locator(99))
            .unwrap();
        moved.finish();
        assert_eq!(moved.slots.as_slice().as_ptr(), backing);
        assert_eq!(moved.get(&slot_digest(2)).unwrap().segment_ordinal, 99);
        assert_eq!(work.budget().live_bytes(), bytes + 19);
        drop(moved);
        assert_eq!(work.budget().live_bytes(), 19);
        drop(other);
        assert_eq!(work.budget().live_bytes(), 0);
        let short = test_work(bytes - 1);
        let mut refused = super::ChunkDirectory::new(short.budget(), 7, false);
        let error = refused
            .insert_fallback(slot_digest(0), slot_locator(0))
            .unwrap_err();
        assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
        assert_eq!(refused.len(), 0);
        assert!(!refused.reserved);
        assert_eq!(short.budget().live_bytes(), 0);
    }

    fn assert_avl(table: &super::ChunkDirectory) {
        fn walk(
            table: &super::ChunkDirectory,
            index: usize,
            parent: usize,
            low: Option<[u8; 32]>,
            high: Option<[u8; 32]>,
        ) -> (u32, usize) {
            if index == super::NO_CHUNK_SLOT {
                return (0, 0);
            }
            let slot = table.slots.as_slice()[index];
            assert_eq!(slot.parent, parent);
            assert!(low.is_none_or(|bound| bound < slot.digest));
            assert!(high.is_none_or(|bound| slot.digest < bound));
            let (left_height, left_count) = walk(table, slot.left, index, low, Some(slot.digest));
            let (right_height, right_count) =
                walk(table, slot.right, index, Some(slot.digest), high);
            assert!(left_height.abs_diff(right_height) <= 1);
            assert_eq!(slot.height, 1 + left_height.max(right_height));
            (slot.height, left_count + right_count + 1)
        }
        let (_, reachable) = walk(table, table.root, super::NO_CHUNK_SLOT, None, None);
        assert_eq!(reachable, table.len());
    }

    #[test]
    fn chunk_avl_rotations_sort_and_last_locator_match_independent_map() {
        // Empty and singleton finishing exercise both zero-iteration bounds.
        let work = test_work(4096);
        for count in [0, 1] {
            let mut table = super::ChunkDirectory::new(work.budget(), count, false);
            if count == 1 {
                table
                    .insert_fallback(slot_digest(1), slot_locator(1))
                    .unwrap();
            }
            let pointer = table.slots.as_slice().as_ptr();
            let live = work.budget().live_bytes();
            table.finish();
            assert_eq!(table.len(), count);
            assert_eq!(table.slots.as_slice().as_ptr(), pointer);
            assert_eq!(work.budget().live_bytes(), live);
        }
        const COUNT: u64 = 257;
        for order in [
            (0..COUNT).collect::<Vec<_>>(),
            (0..COUNT).rev().collect(),
            (0..COUNT).map(|i| (i * 131) % COUNT).collect(),
        ] {
            let work = test_work(super::ChunkDirectory::reservation_bytes(COUNT as usize).unwrap());
            let mut table = super::ChunkDirectory::new(work.budget(), COUNT as usize, false);
            let mut oracle = std::collections::BTreeMap::new();
            for i in order {
                table
                    .insert_fallback(slot_digest(i), slot_locator(i))
                    .unwrap();
                oracle.insert(slot_digest(i), i);
                assert_avl(&table);
            }
            for i in (0..COUNT).step_by(3) {
                table
                    .insert_fallback(slot_digest(i), slot_locator(COUNT + i))
                    .unwrap();
                oracle.insert(slot_digest(i), COUNT + i);
                assert_avl(&table);
            }
            let pointer = table.slots.as_slice().as_ptr();
            let charge = work.budget().live_bytes();
            table.finish();
            assert_eq!(table.slots.as_slice().as_ptr(), pointer);
            assert_eq!(work.budget().live_bytes(), charge);
            assert!(
                table
                    .slots
                    .as_slice()
                    .iter()
                    .all(|slot| slot.left == super::NO_CHUNK_SLOT
                        && slot.right == super::NO_CHUNK_SLOT
                        && slot.parent == super::NO_CHUNK_SLOT
                        && slot.height == 0)
            );
            let actual = table
                .iter()
                .map(|(digest, locator)| {
                    (super::Digest::from_bytes(*digest), locator.segment_ordinal)
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, oracle.into_iter().collect::<Vec<_>>());
            for (digest, ordinal) in actual {
                assert_eq!(table.get(&digest).unwrap().segment_ordinal, ordinal);
            }
            assert!(table.get(&slot_digest(COUNT)).is_none());
        }
    }

    thread_local! { static WIPED_SLOT_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

    fn observe_wiped_slots(slots: &[super::ChunkSlot], budget: &super::WorkBudget) {
        assert!(slots.iter().all(|slot| slot.digest == [0; 32]
            && slot.locator.segment_ordinal == 0
            && slot.locator.fragment_count == 0
            && slot.left == 0
            && slot.right == 0
            && slot.parent == 0
            && slot.height == 0));
        assert_eq!(
            budget.live_bytes(),
            super::ChunkDirectory::reservation_bytes(5).unwrap() + 13
        );
        WIPED_SLOT_COUNT.with(|count| count.set(count.get() + slots.len()));
    }

    #[test]
    fn chunk_directory_wipes_fields_before_free_and_original_lease_release() {
        WIPED_SLOT_COUNT.with(|count| count.set(0));
        let bytes = super::ChunkDirectory::reservation_bytes(5).unwrap();
        let work = test_work(bytes + 13);
        let retained = super::WorkBytes::zeroed(work.budget(), 13).unwrap();
        for mode in 0..3 {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || -> crate::diagnostics::Result<()> {
                    let mut table = super::ChunkDirectory::new(work.budget(), 5, false);
                    table.before_free = Some(observe_wiped_slots);
                    for i in 1..4 {
                        table
                            .insert_fallback(slot_digest(i), slot_locator(i))
                            .unwrap();
                    }
                    if mode == 1 {
                        return Err(super::private_invalid("post-initialization decode failure"));
                    }
                    if mode == 2 {
                        panic!("owned directory unwind");
                    }
                    table.finish();
                    Ok(())
                },
            ));
            assert_eq!(work.budget().live_bytes(), 13);
            match mode {
                0 => assert!(result.unwrap().is_ok()),
                1 => assert!(result.unwrap().is_err()),
                _ => assert!(result.is_err()),
            }
        }
        WIPED_SLOT_COUNT.with(|count| assert_eq!(count.get(), 9));
        drop(retained);
        assert_eq!(work.budget().live_bytes(), 0);
    }

    #[test]
    fn fallback_cap_and_header_errors_precede_lazy_table_reservation() {
        let work = test_work(super::ChunkDirectory::reservation_bytes(3).unwrap());
        let mut zero = super::ChunkDirectory::new(work.budget(), 0, false);
        let error =
            super::discover_chunk(&mut zero, &[], 0, false, false, slot_locator(0)).unwrap_err();
        assert_eq!(error.code(), super::ReasonCode::RandomAccessPolicyRefused);
        assert_eq!(work.budget().live_bytes(), 0);
        let mut table = super::ChunkDirectory::new(work.budget(), 3, false);
        assert_eq!(
            super::discover_chunk(&mut table, &[], 3, false, false, slot_locator(0))
                .unwrap_err()
                .class(),
            super::OutcomeClass::Truncated
        );
        let mut malformed = chunk_header(slot_digest(1));
        malformed[0] = 0;
        assert_eq!(
            super::discover_chunk(&mut table, &malformed, 3, false, false, slot_locator(0))
                .unwrap_err()
                .class(),
            super::OutcomeClass::Corrupt
        );
        assert!(!table.reserved);
        assert_eq!(work.budget().live_bytes(), 0);
        for ordinal in [1, 9] {
            super::discover_chunk(
                &mut table,
                &chunk_header(slot_digest(1)),
                3,
                false,
                false,
                slot_locator(ordinal),
            )
            .unwrap();
        }
        assert_eq!(table.len(), 1); // Duplicate below cap keeps its last locator.
        // At cap, even a duplicate or malformed/truncated header is refused
        // at the original guard. Later data does not change that position.
        for payload in [&chunk_header(slot_digest(1))[..], &malformed[..], &[][..]] {
            assert_eq!(
                super::discover_chunk(&mut table, payload, 1, false, false, slot_locator(77))
                    .unwrap_err()
                    .code(),
                super::ReasonCode::RandomAccessPolicyRefused
            );
        }
        table.finish();
        assert_eq!(table.get(&slot_digest(1)).unwrap().segment_ordinal, 9);
    }

    #[test]
    fn support_heavy_upper_capacity_refusal_keeps_all_shared_owners_live() {
        let slot_bytes = super::ChunkDirectory::reservation_bytes(1).unwrap();
        let upper_bytes = super::ChunkDirectory::reservation_bytes(6).unwrap();
        let work = test_work(upper_bytes + 32 + 47 + 59 + 71 - 1);
        let mut old_index = super::ChunkDirectory::new(work.budget(), 1, true);
        old_index.reserve_once().unwrap();
        old_index
            .append_index(slot_digest(7), slot_locator(7))
            .unwrap();
        let fetched_cipher = super::WorkBytes::zeroed(work.budget(), 32).unwrap();
        let raw_object = super::WorkBytes::zeroed(work.budget(), 47).unwrap();
        let raw_support = super::WorkBytes::zeroed(work.budget(), 59).unwrap();
        let other_support = super::WorkBytes::zeroed(work.budget(), 71).unwrap();
        let live = work.budget().live_bytes();
        assert!(work.budget().available_bytes() >= slot_bytes);
        assert!(work.budget().available_bytes() < upper_bytes);
        let segments = (0..6)
            .map(|_| test_segment(super::SEGMENT_PAYLOAD, 1))
            .collect::<Vec<_>>();
        let bound = super::fallback_slot_bound(&segments, u64::MAX).unwrap();
        let mut builder = super::ChunkDirectory::new(work.budget(), bound, false);
        let error = super::discover_chunk(
            &mut builder,
            &chunk_header(slot_digest(2)),
            u64::MAX,
            false,
            false,
            slot_locator(2),
        )
        .unwrap_err();
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        assert_eq!(work.budget().live_bytes(), live);
        assert_eq!(old_index.get(&slot_digest(7)).unwrap().segment_ordinal, 7);
        assert_eq!(builder.len(), 0);
        assert!(!builder.reserved);
        drop((
            builder,
            old_index,
            fetched_cipher,
            raw_object,
            raw_support,
            other_support,
        ));
        assert_eq!(work.budget().live_bytes(), 0);
    }

    #[test]
    fn authenticated_support_scan_refuses_upper_slots_after_first_valid_chunk() {
        #[derive(Default)]
        struct Pressure {
            budget: Option<super::WorkBudget>,
            target: Option<(u64, u64)>,
            raw_len: u64,
            upper_bytes: u64,
            available_during_fill: u64,
            held: Option<super::WorkBytes>,
            hits: usize,
        }
        struct PressureSource {
            source: MemoryRandomReadSource,
            pressure: Arc<Mutex<Pressure>>,
        }
        impl RandomReadSource for PressureSource {
            fn len(&self) -> crate::diagnostics::Result<u64> {
                self.source.len()
            }
            fn revision(&self) -> crate::diagnostics::Result<SourceRevision> {
                self.source.revision()
            }
            fn read_exact_at(&self, _: u64, _: u64) -> crate::diagnostics::Result<Vec<u8>> {
                panic!("encrypted range producer requires a bounded source fill")
            }
            fn read_exact_at_into(
                &self,
                offset: u64,
                dst: &mut [u8],
            ) -> crate::diagnostics::Result<()> {
                let mut pressure = self.pressure.lock().unwrap();
                if pressure.target == Some((offset, dst.len() as u64)) {
                    pressure.hits += 1;
                    if pressure.hits == 1 {
                        let budget = pressure.budget.as_ref().unwrap().clone();
                        // The actual cipher and trace slot are already reserved.
                        // Leave room for the complete raw object and revision
                        // checks. After cipher free, B Slots are one byte short.
                        let leave = pressure.upper_bytes - 1 - dst.len() as u64 + pressure.raw_len;
                        assert!(leave > pressure.raw_len + 64);
                        pressure.held = Some(super::WorkBytes::zeroed(
                            &budget,
                            (budget.available_bytes() - leave) as usize,
                        )?);
                        pressure.available_during_fill = budget.available_bytes();
                    }
                }
                drop(pressure);
                self.source.read_exact_at_into(offset, dst)
            }
        }
        let directory = TestDir::new();
        let mut expected = Vec::new();
        for i in 0_u64..12 {
            let bytes = (i + 1).to_be_bytes();
            let name = format!("item{i:02}");
            std::fs::write(directory.path().join(&name), bytes).unwrap();
            expected.push((name, bytes));
        }
        let mut archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        // These supported declarations deliberately exercise all three empty
        // support collections. The writer validates/recomputes identities and
        // the positive read below establishes a readable generated archive.
        archive.descriptor.features.incompat |= crate::ecf::FEATURE_CROSS_FILE_COMPRESSION_V1
            | crate::ecf::FEATURE_CODEC_TRANSFORM_V1
            | crate::ecf::FEATURE_RECONSTRUCTIVE_TRANSFORM_V1
            | crate::ecf::FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1;
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: &[recipient],
                padding: PaddingMode::None,
                include_index: false,
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let mut opened = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(encrypted.bytes.clone()),
            RandomAccessPolicy {
                max_cached_bytes: 0,
                ..RandomAccessPolicy::default()
            },
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
        )
        .unwrap();
        let count = archive.content_store.chunks.len();
        assert_eq!(count, 12);
        let bound = super::fallback_slot_bound(opened.segments.as_slice(), u64::MAX).unwrap();
        assert_eq!(bound, count + 3);
        let first_chunk = *opened
            .segments
            .as_slice()
            .iter()
            .filter(|segment| segment.class == super::SEGMENT_PAYLOAD)
            .nth(3)
            .unwrap();
        let (_, first_raw) = {
            let object = super::decrypt_segment(
                &mut opened.session,
                &first_chunk,
                opened
                    .segments
                    .as_slice()
                    .iter()
                    .position(|segment| segment.offset == first_chunk.offset)
                    .unwrap() as u64,
                &opened.envelope.view(),
                &opened.keys,
                opened.padding,
                EncryptedOpenOptions::new(None),
                super::AccessPurpose::EncryptedPayload,
                &opened.work,
            )
            .unwrap();
            let (kind, payload) = object.view().unwrap();
            assert_eq!(kind, super::wire::PRIVATE_OBJECT_CHUNK);
            super::parse_chunk_frame_header(&payload[..96], true, true).unwrap();
            (kind, object.exact_bytes().len() as u64)
        };
        let upper_bytes = super::ChunkDirectory::reservation_bytes(bound).unwrap();
        assert!(upper_bytes > first_chunk.extent + 64);
        opened.load_support_and_index_fallback().unwrap();
        assert_eq!(opened.index.len(), count);
        assert_eq!(
            opened.index_status,
            super::RandomAccessIndexStatus::RebuiltAbsent
        );
        // Probe session is separate; the tested session gets one finite cap
        // before open, then pressure uses that original shared counter.
        drop(opened);
        for cap in [0, u64::MAX] {
            let pressure = Arc::new(Mutex::new(Pressure::default()));
            let mut options = EncryptedOpenOptions::new(Some(Unlock::Identity(&identity)));
            options.crypto_policy.max_working_memory_bytes = 64 * 1024;
            let mut reader = open_indexed_random_encrypted(
                PressureSource {
                    source: MemoryRandomReadSource::new(encrypted.bytes.clone()),
                    pressure: Arc::clone(&pressure),
                },
                RandomAccessPolicy {
                    max_cached_bytes: 0,
                    max_chunk_frames_scanned: cap,
                    ..RandomAccessPolicy::default()
                },
                options,
            )
            .unwrap();
            {
                let mut state = pressure.lock().unwrap();
                state.budget = Some(reader.work.budget().clone());
                state.target = Some((first_chunk.offset, first_chunk.extent));
                state.raw_len = first_raw;
                state.upper_bytes = upper_bytes;
            }
            let error = reader.load_support_and_index_fallback().unwrap_err();
            if cap == 0 {
                assert_eq!(error.code(), super::ReasonCode::RandomAccessPolicyRefused);
                assert_eq!(
                    error.detail(),
                    "encrypted Chunk-object scan exceeds caller policy"
                );
            } else {
                assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
                assert_eq!(
                    error.detail(),
                    "managed crypto replacement and required headroom exceed caller policy"
                );
            }
            assert_eq!(
                reader.index_status,
                super::RandomAccessIndexStatus::RebuiltAbsent
            );
            assert_eq!(reader.index.len(), 0);
            assert!(reader.support.is_none());
            {
                let mut state = pressure.lock().unwrap();
                assert_eq!(state.hits, 1);
                // No charged allocation occurs between the fill's reserved
                // trace and the first Slot admission except this raw object.
                assert_eq!(
                    state.available_during_fill + first_chunk.extent - first_raw,
                    upper_bytes - 1
                );
                assert!(upper_bytes > super::ChunkDirectory::reservation_bytes(count).unwrap());
                println!(
                    "authenticated fallback cap={cap}: physical={bound} unique={count} cipher={} raw={first_raw} upper={upper_bytes} first_slot_available={}",
                    first_chunk.extent,
                    upper_bytes - 1
                );
                state.target = None;
                state.held = None;
            }
            if cap == u64::MAX {
                for (name, bytes) in &expected {
                    let read = reader
                        .read_entry(&LogicalPath::from_utf8([name.as_str()]).unwrap())
                        .unwrap();
                    assert_eq!(&*read.bytes, bytes);
                }
                assert_eq!(reader.index.len(), count);
            }
        }
    }

    #[test]
    fn validated_index_count_drives_admission_with_raw_and_spans_live() {
        let work = test_work(4096);
        let records = vec![
            index_record(slot_digest(1).as_bytes(), 0, 0, 1),
            index_record(slot_digest(2).as_bytes(), 1, 0, 1),
        ];
        let items = index_sequence(&records, &work);
        assert_eq!(items.len(), 2);
        let raw_and_spans = work.budget().live_bytes();
        let table_bytes = super::ChunkDirectory::reservation_bytes(2).unwrap();
        let segments = [
            test_segment(super::SEGMENT_PAYLOAD, 1),
            test_segment(super::SEGMENT_PAYLOAD, 1),
        ];
        let table = super::decode_encrypted_index(&items, &segments, 2, &work).unwrap();
        assert_eq!(work.budget().live_bytes(), raw_and_spans + table_bytes);
        drop(table);
        // Expected semantic count is never allocation authority: with enough
        // space for physical N, an enormous expected count is a late mismatch.
        let error = super::decode_encrypted_index(&items, &segments, u64::MAX, &work)
            .err()
            .unwrap();
        assert_eq!(error.code(), super::ReasonCode::CryptoPrivateObjectInvalid);
        assert!(error.to_string().contains("Chunk count mismatch"));
        assert_eq!(work.budget().live_bytes(), raw_and_spans);
        let (rebuilt, status) =
            super::decode_index_or_rebuild(&items, &segments, u64::MAX, &work).unwrap();
        assert_eq!(status, super::RandomAccessIndexStatus::RebuiltInvalid);
        assert_eq!(rebuilt.len(), 0);
        drop(rebuilt);
        let held = super::WorkBytes::zeroed(
            work.budget(),
            (work.budget().available_bytes() - table_bytes + 1) as usize,
        )
        .unwrap();
        let before = work.budget().live_bytes();
        let error = super::decode_index_or_rebuild(&items, &segments, 2, &work)
            .err()
            .unwrap();
        assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        assert_eq!(work.budget().live_bytes(), before);
        drop((held, items));
        assert_eq!(work.budget().live_bytes(), 0);
        let empty = index_sequence(&[], &work);
        let live = work.budget().live_bytes();
        let table = super::decode_encrypted_index(&empty, &[], 0, &work).unwrap();
        assert_eq!(table.len(), 0);
        assert_eq!(work.budget().live_bytes(), live);
    }

    #[test]
    fn encrypted_index_item_defenses_keep_field_and_late_duplicate_positions() {
        let good = index_record(slot_digest(1).as_bytes(), 0, 0, 1);
        let mut wrong_kind = good.clone();
        wrong_kind[..2].copy_from_slice(&1_u16.to_be_bytes());
        let mut trailing = good.clone();
        trailing.push(0);
        let mut missing = crate::canonical::RecordBuilder::new(super::wire::RECORD_ENCRYPTED_INDEX);
        missing
            .bytes(1, slot_digest(1).as_bytes())
            .unwrap()
            .u64(2, 0)
            .unwrap()
            .u64(3, 0)
            .unwrap();
        let mut extra = crate::canonical::RecordBuilder::new(super::wire::RECORD_ENCRYPTED_INDEX);
        extra
            .bytes(1, slot_digest(1).as_bytes())
            .unwrap()
            .u64(2, 0)
            .unwrap()
            .u64(3, 0)
            .unwrap()
            .u32(4, 1)
            .unwrap()
            .u8(5, 1)
            .unwrap();
        let mut wrong_digest_type =
            crate::canonical::RecordBuilder::new(super::wire::RECORD_ENCRYPTED_INDEX);
        wrong_digest_type
            .u64(1, 0)
            .unwrap()
            .u64(2, 0)
            .unwrap()
            .u64(3, 0)
            .unwrap()
            .u32(4, 1)
            .unwrap();
        let mut cases = vec![
            ("kind", wrong_kind),
            ("trailing", trailing),
            ("missing", missing.finish().unwrap()),
            ("extra", extra.finish().unwrap()),
            ("digest_type", wrong_digest_type.finish().unwrap()),
            ("digest_width", index_record(&[1; 31], 0, 0, 1)),
            (
                "absent_ordinal",
                index_record(slot_digest(1).as_bytes(), 99, 0, 1),
            ),
            (
                "ordinal_conversion_or_absence",
                index_record(slot_digest(1).as_bytes(), u64::MAX, 0, 1),
            ),
            (
                "first_counter",
                index_record(slot_digest(1).as_bytes(), 0, 1, 1),
            ),
            (
                "zero_fragments",
                index_record(slot_digest(1).as_bytes(), 0, 0, 0),
            ),
            (
                "segment_count",
                index_record(slot_digest(1).as_bytes(), 0, 0, 2),
            ),
        ];
        for tag in 2..=4 {
            let mut wrong_type =
                crate::canonical::RecordBuilder::new(super::wire::RECORD_ENCRYPTED_INDEX);
            wrong_type.bytes(1, slot_digest(1).as_bytes()).unwrap();
            for field in 2..=4 {
                if field == tag {
                    wrong_type.bytes(field, &[0]).unwrap();
                } else if field == 4 {
                    wrong_type.u32(field, 1).unwrap();
                } else {
                    wrong_type.u64(field, 0).unwrap();
                }
            }
            cases.push(("integer_type", wrong_type.finish().unwrap()));
        }
        let work = test_work(4096);
        for (name, record) in cases {
            let mut table = super::ChunkDirectory::new(work.budget(), 2, true);
            table.reserve_once().unwrap();
            let error = super::decode_encrypted_index_item(
                &record,
                &[test_segment(super::SEGMENT_PAYLOAD, 1)],
                &mut table,
            )
            .unwrap_err();
            assert_eq!(
                error.class(),
                super::OutcomeClass::Nonconforming,
                "{name}: {error}"
            );
            assert_eq!(table.len(), 0, "{name}");
        }
        let mut table = super::ChunkDirectory::new(work.budget(), 2, true);
        table.reserve_once().unwrap();
        assert!(
            super::decode_encrypted_index_item(
                &good,
                &[test_segment(super::SEGMENT_CONTROL, 1)],
                &mut table
            )
            .is_err()
        );
        super::decode_encrypted_index_item(
            &good,
            &[test_segment(super::SEGMENT_PAYLOAD, 1)],
            &mut table,
        )
        .unwrap();
        // Locator validation still wins before late duplicate detection.
        let invalid_duplicate = index_record(slot_digest(1).as_bytes(), 99, 0, 1);
        let error = super::decode_encrypted_index_item(
            &invalid_duplicate,
            &[test_segment(super::SEGMENT_PAYLOAD, 1)],
            &mut table,
        )
        .unwrap_err();
        assert!(error.to_string().contains("absent segment"));
        let error = super::decode_encrypted_index_item(
            &good,
            &[test_segment(super::SEGMENT_PAYLOAD, 1)],
            &mut table,
        )
        .unwrap_err();
        assert!(error.to_string().contains("invalid or duplicate"));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn borrowed_slot_consumers_preserve_digest_lookup_and_physical_order() {
        let work = test_work(4096);
        let mut table = super::ChunkDirectory::new(work.budget(), 3, false);
        for (digest, ordinal) in [(3, 1), (1, 8), (2, 1)] {
            table
                .insert_fallback(slot_digest(digest), slot_locator(ordinal))
                .unwrap();
        }
        table.finish();
        let borrowed = table.get(&slot_digest(2)).unwrap();
        assert_eq!(borrowed.segment_ordinal, 1);
        let mut physical = table
            .iter()
            .map(|(digest, locator)| (locator.segment_ordinal, super::Digest::from_bytes(*digest)))
            .collect::<Vec<_>>();
        physical.sort();
        assert_eq!(
            physical,
            [
                (1, slot_digest(2)),
                (1, slot_digest(3)),
                (8, slot_digest(1))
            ]
        );
        assert_eq!(
            table
                .values()
                .map(|locator| locator.segment_ordinal)
                .collect::<Vec<_>>(),
            [8, 1, 1]
        );
        assert_eq!(borrowed.segment_ordinal, 1);
    }

    struct CountingSource {
        source: MemoryRandomReadSource,
        reads: Arc<Mutex<Vec<(u64, u64)>>>,
    }

    impl RandomReadSource for CountingSource {
        fn len(&self) -> crate::diagnostics::Result<u64> {
            self.source.len()
        }

        fn revision(&self) -> crate::diagnostics::Result<SourceRevision> {
            self.source.revision()
        }

        fn read_exact_at(&self, offset: u64, len: u64) -> crate::diagnostics::Result<Vec<u8>> {
            self.reads.lock().unwrap().push((offset, len));
            self.source.read_exact_at(offset, len)
        }

        fn read_exact_at_into(
            &self,
            offset: u64,
            dst: &mut [u8],
        ) -> crate::diagnostics::Result<()> {
            self.reads
                .lock()
                .unwrap()
                .push((offset, u64::try_from(dst.len()).unwrap()));
            self.source.read_exact_at_into(offset, dst)
        }
    }

    #[test]
    fn encrypted_callers_refuse_sources_without_a_bounded_fill_capability() {
        struct AllocatingOnlySource(MemoryRandomReadSource);
        impl RandomReadSource for AllocatingOnlySource {
            fn len(&self) -> crate::diagnostics::Result<u64> {
                self.0.len()
            }
            fn revision(&self) -> crate::diagnostics::Result<SourceRevision> {
                self.0.revision()
            }
            fn read_exact_at(&self, _: u64, _: u64) -> crate::diagnostics::Result<Vec<u8>> {
                panic!("encrypted callers must never invoke the allocating source callback")
            }
        }
        let directory = TestDir::new();
        std::fs::write(directory.path().join("file"), b"bounded fill fixture").unwrap();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: std::slice::from_ref(&recipient),
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let public_error = super::inspect_indexed_random_encrypted_public(
            Box::new(AllocatingOnlySource(MemoryRandomReadSource::new(
                encrypted.bytes.clone(),
            ))),
            RandomAccessPolicy::default(),
            crate::crypto::CryptoPolicy::default(),
        )
        .unwrap_err();
        let open_error = open_indexed_random_encrypted(
            Box::new(AllocatingOnlySource(MemoryRandomReadSource::new(
                encrypted.bytes,
            ))),
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
        )
        .err()
        .expect("missing bounded capability must refuse encrypted open");
        for error in [public_error, open_error] {
            assert_eq!(
                error.class(),
                crate::diagnostics::OutcomeClass::PolicyRefused
            );
            assert_eq!(error.code(), super::ReasonCode::RandomAccessPolicyRefused);
        }
    }

    #[test]
    fn record_policy_refuses_before_fetching_a_whole_segment() {
        let directory = TestDir::new();
        let file = (0_u32..512)
            .flat_map(|value| *super::sha256_exact(&value.to_be_bytes()).as_bytes())
            .collect::<Vec<_>>();
        std::fs::write(directory.path().join("file"), file).unwrap();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        for include_index in [true, false] {
            let encrypted = encrypt_archive(
                &archive,
                EncryptedWriteOptions {
                    recipients: std::slice::from_ref(&recipient),
                    padding: PaddingMode::None,
                    include_index,
                    ..EncryptedWriteOptions::default()
                },
            )
            .unwrap();
            let reads = Arc::new(Mutex::new(Vec::new()));
            let mut options = EncryptedOpenOptions::new(Some(Unlock::Identity(&identity)));
            // Admit the small CONTROL records while refusing the larger PAYLOAD
            // record under the original immutable caller policy.
            options.crypto_policy.max_ciphertext_record_bytes = 4096;
            let mut opened = open_indexed_random_encrypted(
                CountingSource {
                    source: MemoryRandomReadSource::new(encrypted.bytes),
                    reads: Arc::clone(&reads),
                },
                RandomAccessPolicy::default(),
                options,
            )
            .unwrap();
            let refused = *opened
                .segments
                .as_slice()
                .iter()
                .filter(|segment| segment.class == super::SEGMENT_PAYLOAD)
                .max_by_key(|segment| segment.extent)
                .expect("fixture has a PAYLOAD segment");
            assert!(refused.extent > 4096 + SEGMENT_HEADER_LEN + PROTECTED_HEADER_LEN);
            reads.lock().unwrap().clear();
            let error = opened
                .read_entry(&LogicalPath::from_utf8(["file"]).unwrap())
                .unwrap_err();
            assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
            let requests = reads.lock().unwrap();
            let protected_header = (refused.offset + SEGMENT_HEADER_LEN, PROTECTED_HEADER_LEN);
            assert!(requests.contains(&protected_header));
            // Unrelated small PAYLOAD support objects can be admitted. For the
            // refused large segment, only its public protected header is read.
            assert!(
                requests
                    .iter()
                    .filter(|(offset, len)| {
                        *offset < refused.offset + refused.extent && *offset + *len > refused.offset
                    })
                    .all(|request| *request == protected_header),
                "requests intersecting refused segment: {requests:?}"
            );
        }
    }

    #[test]
    fn decrypted_frames_are_not_retained_across_bounded_entry_reads() {
        let directory = TestDir::new();
        let first = (0_u32..512)
            .flat_map(|value| *super::sha256_exact(&value.to_be_bytes()).as_bytes())
            .collect::<Vec<_>>();
        let second = (512_u32..1024)
            .flat_map(|value| *super::sha256_exact(&value.to_be_bytes()).as_bytes())
            .collect::<Vec<_>>();
        std::fs::write(directory.path().join("first"), &first).unwrap();
        std::fs::write(directory.path().join("second"), &second).unwrap();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let policy = RandomAccessPolicy {
            max_cached_bytes: 0,
            ..RandomAccessPolicy::default()
        };
        for include_index in [true, false] {
            let encrypted = encrypt_archive(
                &archive,
                EncryptedWriteOptions {
                    recipients: std::slice::from_ref(&recipient),
                    padding: PaddingMode::None,
                    include_index,
                    ..EncryptedWriteOptions::default()
                },
            )
            .unwrap();
            // Calibrate this generated fixture's real managed owners under a
            // separate probe. Its final retained counter includes envelope,
            // context and trace capacity after the entry-read/refetch sequence
            // and the later pressure/refetch attempts.
            let mut probe = open_indexed_random_encrypted(
                MemoryRandomReadSource::new(encrypted.bytes.clone()),
                policy.clone(),
                EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
            )
            .unwrap();
            probe.load_support_and_index_fallback().unwrap();
            let ids = probe
                .index
                .iter()
                .map(|(digest, _)| super::Digest::from_bytes(*digest))
                .collect::<Vec<_>>();
            assert_eq!(ids.len(), 2);
            for name in ["first", "second", "first", "second", "first"] {
                probe
                    .read_entry(&LogicalPath::from_utf8([name]).unwrap())
                    .unwrap();
            }
            let first_owner = probe.chunk_frame(ids[0]).unwrap();
            let second_owner = probe.chunk_frame(ids[1]).unwrap();
            let first_length = first_owner.exact_bytes().len() as u64;
            let second_length = second_owner.exact_bytes().len() as u64;
            let two_owners_live = probe.work.budget().live_bytes();
            drop(second_owner);
            drop(first_owner);
            let retained = probe.work.budget().live_bytes();
            assert_eq!(two_owners_live, retained + first_length + second_length);
            let largest_payload = probe
                .segments
                .as_slice()
                .iter()
                .filter(|value| value.class == super::SEGMENT_PAYLOAD)
                .map(|value| value.extent)
                .max()
                .unwrap();
            let largest_object = first_length.max(second_length);
            let smallest_object = first_length.min(second_length);
            // Complete-object assembly overlaps retained owners with fetched
            // ciphertext and one raw Chunk owner. This is a fixture-derived
            // capacity, not an engine encoded-size coefficient or RSS claim.
            let single_object_peak = retained
                .checked_add(largest_payload)
                .and_then(|value| value.checked_add(largest_object))
                .unwrap();
            let two_object_fetch_peak = two_owners_live.checked_add(largest_payload).unwrap();
            assert!(retained < smallest_object);
            assert!(single_object_peak < two_object_fetch_peak);
            assert!(single_object_peak < probe.work.policy().max_working_memory_bytes);
            println!(
                "generated cache fixture include_index={include_index}: retained={retained} ciphertext={largest_payload} object={largest_object} cap={single_object_peak} two_owner_fetch_peak={two_object_fetch_peak}"
            );
            drop(probe);
            let reads = Arc::new(Mutex::new(Vec::new()));
            let mut opened = open_indexed_random_encrypted(
                CountingSource {
                    source: MemoryRandomReadSource::new(encrypted.bytes),
                    reads: Arc::clone(&reads),
                },
                policy.clone(),
                {
                    let mut options = EncryptedOpenOptions::new(Some(Unlock::Identity(&identity)));
                    options.crypto_policy.max_working_memory_bytes = single_object_peak;
                    options
                },
            )
            .unwrap();
            let largest_payload = opened
                .segments
                .as_slice()
                .iter()
                .filter(|value| value.class == super::SEGMENT_PAYLOAD)
                .map(|value| value.extent)
                .max()
                .unwrap();
            // This immutable cap includes simultaneous ciphertext/object and
            // retained control/trace ownership. Refetching still proves that
            // decrypted frames are not cached across independent entry reads.
            assert!(largest_payload < opened.work.policy().max_working_memory_bytes);
            for (name, expected) in [("first", &first), ("second", &second)] {
                let read = opened
                    .read_entry(&LogicalPath::from_utf8([name]).unwrap())
                    .unwrap();
                assert_eq!(&*read.bytes, expected);
            }
            reads.lock().unwrap().clear();
            let _ = opened
                .read_entry(&LogicalPath::from_utf8(["first"]).unwrap())
                .unwrap();
            assert!(reads.lock().unwrap().iter().any(|(offset, len)| {
                opened.segments.as_slice().iter().any(|segment| {
                    segment.class == super::SEGMENT_PAYLOAD
                        && segment.offset == *offset
                        && segment.extent == *len
                })
            }));
            // An extra real complete Chunk owner prevents the next assembly
            // from fitting. Releasing it admits the same authorized read.
            let owner = opened.chunk_frame(ids[0]).unwrap();
            let error = opened
                .read_entry(&LogicalPath::from_utf8(["second"]).unwrap())
                .unwrap_err();
            assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
            assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
            drop(owner);
            let read = opened
                .read_entry(&LogicalPath::from_utf8(["second"]).unwrap())
                .unwrap();
            assert_eq!(&*read.bytes, &second);
        }
    }

    #[test]
    fn forged_segment_object_total_is_refused_before_reserve() {
        let work = super::CryptoWorkContext::new(crate::crypto::CryptoPolicy::default());
        let fragment = super::wire::PrivateFragmentRef {
            object_id: [0; 32],
            total_len: u64::MAX,
            index: 0,
            count: 1,
            offset: 0,
            bytes: &[1],
        };
        let error = super::reserve_segment_object(&fragment, 256, &work).unwrap_err();
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        assert_eq!(work.budget().live_bytes(), 0);
    }

    #[test]
    fn segment_object_reservation_counts_other_live_crypto_owners() {
        let work = super::CryptoWorkContext::new(crate::crypto::CryptoPolicy {
            max_working_memory_bytes: 32,
            ..crate::crypto::CryptoPolicy::default()
        });
        let retained = super::WorkBytes::zeroed(work.budget(), 17).unwrap();
        let fragment = super::wire::PrivateFragmentRef {
            object_id: [0; 32],
            total_len: 16,
            index: 0,
            count: 1,
            offset: 0,
            bytes: &[1],
        };
        let error = super::reserve_segment_object(&fragment, 32, &work).unwrap_err();
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        assert_eq!(work.budget().live_bytes(), 17);
        drop(retained);
        let object = super::reserve_segment_object(&fragment, 32, &work).unwrap();
        assert_eq!(object.len(), 16);
        assert_eq!(work.budget().live_bytes(), 16);
        drop(object);
        assert_eq!(work.budget().live_bytes(), 0);
    }

    #[test]
    fn real_index_span_admission_refusal_is_not_a_rebuildable_index() {
        use crate::canonical::RecordBuilder;

        let mut record = RecordBuilder::new(super::wire::RECORD_ENCRYPTED_INDEX);
        record
            .bytes(1, &[7; 32])
            .unwrap()
            .u64(2, 0)
            .unwrap()
            .u64(3, 0)
            .unwrap()
            .u32(4, 1)
            .unwrap();
        let sequence = super::wire::sequence_container(
            super::wire::COLLECTION_INDEX,
            &[record.finish().unwrap()],
        )
        .unwrap();
        let exact =
            super::wire::private_object(super::wire::PRIVATE_OBJECT_SEQUENCE, &sequence).unwrap();
        let work = super::CryptoWorkContext::new(crate::crypto::CryptoPolicy {
            max_working_memory_bytes: 4096,
            ..crate::crypto::CryptoPolicy::default()
        });
        let object = |bytes: &[u8]| {
            let mut raw = super::WorkBytes::zeroed(work.budget(), bytes.len()).unwrap();
            raw.copy_from_slice(bytes);
            super::wire::WorkObject::verified(
                raw,
                super::wire::encrypted_object_id(bytes).unwrap(),
                &work,
            )
            .unwrap()
        };
        let charged_object = object(&exact);
        // Leave no room for a nonempty span table, while the real authenticated
        // object already fits. This is the production Count admission path.
        let retained = super::WorkBytes::zeroed(
            work.budget(),
            usize::try_from(work.budget().available_bytes()).unwrap(),
        )
        .unwrap();
        let mut controls = super::ControlObjects::default();
        let error = super::dispatch_control(&mut controls, charged_object, 0, &work).unwrap_err();
        assert_eq!(
            error.class(),
            crate::diagnostics::OutcomeClass::PolicyRefused
        );
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        assert!(!controls.index_invalid);
        assert!(controls.index.is_none());
        drop(retained);
        assert_eq!(work.budget().live_bytes(), 0);

        // A semantic Index defect still takes the specified rebuild route.
        let mut malformed = exact;
        malformed[12 + 8] = 1; // Nonzero EBCS flags, after its kind is peeked.
        super::dispatch_control(&mut controls, object(&malformed), 0, &work).unwrap();
        assert!(controls.index_invalid);
        assert!(controls.index.is_none());
        assert_eq!(work.budget().live_bytes(), 0);
    }

    #[test]
    fn borrowed_chunk_frame_remains_charged_during_a_nested_frame_read() {
        let directory = TestDir::new();
        std::fs::write(directory.path().join("first"), b"first distinct Chunk").unwrap();
        std::fs::write(directory.path().join("second"), b"second distinct Chunk").unwrap();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: &[recipient],
                padding: PaddingMode::None,
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let mut opened = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(encrypted.bytes),
            RandomAccessPolicy {
                max_cached_bytes: 0,
                ..RandomAccessPolicy::default()
            },
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
        )
        .unwrap();
        let ids = opened
            .index
            .iter()
            .map(|(digest, _)| super::Digest::from_bytes(*digest))
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 2);
        let first = opened.chunk_frame(ids[0]).unwrap();
        let (_, borrowed) = first.view().unwrap();
        let digest = super::sha256_exact(borrowed);
        let first_length = first.exact_bytes().len() as u64;
        let second = opened.chunk_frame(ids[1]).unwrap();
        let second_length = second.exact_bytes().len() as u64;
        assert_eq!(super::sha256_exact(borrowed), digest);
        // The same relationship used by recursive decode_chunk keeps the outer
        // object's backing alive while a nested mutable archive call runs.
        let both_live = opened.work.budget().live_bytes();
        assert!(both_live >= first_length + second_length);
        drop(second);
        assert_eq!(opened.work.budget().live_bytes(), both_live - second_length);
        drop(first);
        assert_eq!(
            opened.work.budget().live_bytes(),
            both_live - second_length - first_length
        );
    }

    fn data_test_envelope() -> Vec<u8> {
        use super::wire::{CryptoEnvelope, RecipientStanza};

        CryptoEnvelope {
            archive_id: [4; 32],
            commitment: [5; 32],
            protection_policy: 2,
            padding_mode: 0,
            boundary_mode: 1,
            envelope_mac: [6; 32],
            stanzas: vec![RecipientStanza {
                stanza_type: 2,
                protection_class: 2,
                stanza_id: [1; 16],
                recipient_hint: [0; 16],
                method_parameters: vec![0; super::wire::PASSWORD_METHOD_LEN],
                encapsulation: Vec::new(),
                wrap_nonce: [2; 12],
                wrapped_afk: [3; 48],
            }],
        }
        .encode()
        .unwrap()
    }

    fn data_test_raw_records(lengths: &[usize]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (index, length) in lengths.iter().copied().enumerate() {
            let mut header = [index as u8 + 1; 32];
            header[24..32].copy_from_slice(&u64::try_from(length).unwrap().to_be_bytes());
            bytes.extend_from_slice(&header);
            bytes.extend((0..length).map(|i| (i as u8).wrapping_add(index as u8)));
        }
        bytes
    }

    // This oracle reads raw fixture extents, not the production preflight.
    fn data_test_reads(bytes: &[u8], locator: &super::SegmentLocator) -> (Vec<(u64, u64)>, usize) {
        let mut cursor = usize::try_from(locator.offset + 64).unwrap();
        let mut reads = Vec::new();
        let mut data_end = cursor;
        for counter in 0..=locator.count {
            let header = &bytes[cursor..cursor + 32];
            assert_eq!(&header[..4], b"EBC1");
            assert_eq!(
                u64::from_be_bytes(header[16..24].try_into().unwrap()),
                u64::from(counter)
            );
            reads.push((cursor as u64, 32));
            let length =
                usize::try_from(u64::from_be_bytes(header[24..32].try_into().unwrap())).unwrap();
            cursor = cursor.checked_add(32).unwrap().checked_add(length).unwrap();
            if counter < locator.count {
                data_end = cursor;
            }
        }
        assert_eq!(cursor as u64, locator.offset + locator.extent);
        reads.push((locator.offset, locator.extent));
        (reads, data_end)
    }

    fn data_test_run(
        bytes: Vec<u8>,
        locator: &super::SegmentLocator,
        envelope: &super::wire::EnvelopeRef<'_>,
        keys: &super::KeyHierarchy,
        options: EncryptedOpenOptions<'_>,
        work: &super::CryptoWorkContext,
    ) -> (
        crate::diagnostics::Result<super::wire::WorkObject>,
        Vec<(u64, u64)>,
    ) {
        let reads = Arc::new(Mutex::new(Vec::new()));
        let mut session = super::RangeSession::new_crypto(
            Box::new(CountingSource {
                source: MemoryRandomReadSource::new(bytes),
                reads: Arc::clone(&reads),
            }),
            RandomAccessPolicy {
                max_cached_bytes: 0,
                ..RandomAccessPolicy::default()
            },
            work.budget().clone(),
        )
        .unwrap();
        let result = super::decrypt_segment(
            &mut session,
            locator,
            0,
            envelope,
            keys,
            PaddingMode::None,
            options,
            super::AccessPurpose::EncryptedPayload,
            work,
        );
        drop(session);
        let observed = reads.lock().unwrap().clone();
        (result, observed)
    }

    fn data_test_reseal_end(
        bytes: &mut [u8],
        locator: &super::SegmentLocator,
        envelope: &super::wire::EnvelopeRef<'_>,
        keys: &super::KeyHierarchy,
        change: impl FnOnce(&mut [u8]),
    ) {
        let (_, end) = data_test_reads(bytes, locator);
        let protected = &bytes[end..end + 32];
        let key = super::super::container::segment_key(
            keys,
            &envelope.archive_id,
            locator.class,
            0,
            &locator.salt,
        )
        .unwrap();
        let nonce = super::super::container::end_nonce(u64::from(locator.count));
        let ad =
            super::super::container::record_ad(&envelope.archive_id, &locator.header, protected)
                .unwrap();
        let mut padded = super::super::aead_open(&key.0, &nonce, &ad, &bytes[end + 32..]).unwrap();
        change(&mut padded);
        let ciphertext = super::super::aead_seal(&key.0, &nonce, &ad, &padded).unwrap();
        assert_eq!(ciphertext.len(), bytes.len() - end - 32);
        bytes[end + 32..].copy_from_slice(&ciphertext);
    }

    fn data_test_change_end(padded: &mut [u8], case: &str) {
        use crate::canonical::RecordBuilder;

        let private_len = u64::from_be_bytes(padded[..8].try_into().unwrap());
        if case == "end-prefix" || case == "end-padding" {
            let length = if case == "end-prefix" {
                private_len + 1
            } else {
                private_len - 1
            };
            padded[..8].copy_from_slice(&length.to_be_bytes());
            return;
        }
        let (record, consumed) = super::decode_record(&padded[8..]).unwrap();
        assert_eq!(consumed, padded.len() - 8);
        let mut digest: [u8; 32] = record
            .field(6)
            .unwrap()
            .as_bytes()
            .unwrap()
            .try_into()
            .unwrap();
        if case == "end-digest" {
            digest[0] ^= 1;
        }
        let mut changed = RecordBuilder::new(if case == "end-kind" {
            super::wire::RECORD_SEGMENT_END ^ 1
        } else {
            super::wire::RECORD_SEGMENT_END
        });
        changed
            .u64(
                1,
                record.field(1).unwrap().as_u64().unwrap() + u64::from(case == "end-ordinal"),
            )
            .unwrap()
            .u8(
                2,
                record.field(2).unwrap().as_u8().unwrap() + u8::from(case == "end-class"),
            )
            .unwrap()
            .u32(
                3,
                record.field(3).unwrap().as_u32().unwrap() + u32::from(case == "end-count"),
            )
            .unwrap()
            .u64(
                4,
                record.field(4).unwrap().as_u64().unwrap() + u64::from(case == "end-private-total"),
            )
            .unwrap()
            .u64(
                5,
                record.field(5).unwrap().as_u64().unwrap() + u64::from(case == "end-cipher-total"),
            )
            .unwrap()
            .bytes(6, &digest)
            .unwrap();
        let changed = changed.finish().unwrap();
        assert_eq!(changed.len(), padded.len() - 8);
        padded[8..].copy_from_slice(&changed);
    }

    #[test]
    fn segment_data_digest_matches_allocated_t1_and_exact_framing() {
        use sha2::Digest as _;

        let header = [0x5a; 64];
        for (lengths, count) in [
            (&[][..], 0),
            (&[0][..], 1),
            (&[16][..], 1),
            (&[1, 16, 257][..], 3),
            (&[65536, 17][..], 7),
        ] {
            let raw = data_test_raw_records(lengths);
            let sequence = super::data_sequence(count, &raw).unwrap();
            let expected: [u8; 32] = super::Sha256::digest(
                super::wire::t1("entrybound/segment-data/v1", &[&header, &sequence]).unwrap(),
            )
            .into();
            assert_eq!(
                super::segment_data_digest(&header, count, &raw).unwrap(),
                expected
            );
        }
        let raw = data_test_raw_records(&[16]);
        let digest = super::segment_data_digest(&header, 1, &raw).unwrap();
        assert_ne!(
            super::segment_data_digest(&header, 2, &raw).unwrap(),
            digest
        );
        for byte in 0..header.len() {
            let mut changed = header;
            changed[byte] ^= 1;
            assert_ne!(
                super::segment_data_digest(&changed, 1, &raw).unwrap(),
                digest
            );
        }
        for byte in 0..raw.len() {
            let mut changed = raw.clone();
            changed[byte] ^= 1;
            if (24..32).contains(&byte) {
                assert!(super::segment_data_digest(&header, 1, &changed).is_err());
            } else {
                assert_ne!(
                    super::segment_data_digest(&header, 1, &changed).unwrap(),
                    digest
                );
            }
        }
        let mut wrong_length = super::data_sequence(1, &raw).unwrap();
        wrong_length[15] ^= 1;
        let wrong: [u8; 32] = super::Sha256::digest(
            super::wire::t1("entrybound/segment-data/v1", &[&header, &wrong_length]).unwrap(),
        )
        .into();
        assert_ne!(wrong, digest);
        let ordered = data_test_raw_records(&[16, 17]);
        let reordered = [&ordered[48..], &ordered[..48]].concat();
        assert_ne!(
            super::segment_data_digest(&header, 2, &ordered).unwrap(),
            super::segment_data_digest(&header, 2, &reordered).unwrap()
        );
    }

    #[test]
    fn segment_data_digest_checked_grammar_preserves_returned_diagnostics() {
        let header = [0; 64];
        let mut short_record = data_test_raw_records(&[16]);
        short_record.pop();
        let mut overflowing = vec![0; 32];
        overflowing[24..32].copy_from_slice(&u64::MAX.to_be_bytes());
        for malformed in [vec![0; 31], short_record, overflowing] {
            let old = super::data_sequence(1, &malformed).unwrap_err();
            let new = super::segment_data_digest(&header, 1, &malformed).unwrap_err();
            assert_eq!(
                (new.class(), new.code(), new.detail()),
                (old.class(), old.code(), old.detail())
            );
        }
        #[cfg(target_pointer_width = "64")]
        {
            // The retained old isolated oracle can panic on cursor + extent;
            // it is not executed here or reported as a returned diagnostic.
            let mut malformed = vec![0; 64];
            malformed[56..64].copy_from_slice(&(u64::MAX - 32).to_be_bytes());
            let error = super::segment_data_digest(&header, 2, &malformed).unwrap_err();
            assert_eq!(error.class(), super::OutcomeClass::Nonconforming);
            assert_eq!(
                error.code(),
                super::ReasonCode::CryptoSegmentStructureInvalid
            );
            assert_eq!(error.detail(), "DATA transcript extent overflow");
        }
    }

    #[test]
    fn segment_data_digest_authentic_objects_and_reads_match() {
        use super::wire::PrivateFragmentRef;
        use sha2::Digest as _;

        let envelope_bytes = data_test_envelope();
        let policy = crate::crypto::CryptoPolicy::default();
        let envelope =
            super::wire::EnvelopeRef::decode_with_policy(&envelope_bytes, policy).unwrap();
        let keys = super::KeyHierarchy::derive(&[3; 32], &envelope.archive_id).unwrap();
        let exact =
            super::wire::private_object(super::wire::PRIVATE_OBJECT_CHUNK, b"streamed DATA object")
                .unwrap();
        let id = super::wire::encrypted_object_id(&exact).unwrap();
        for split in [exact.len(), 9] {
            let mut fragments = vec![PrivateFragmentRef {
                object_id: id,
                total_len: exact.len() as u64,
                index: 0,
                count: if split == exact.len() { 1 } else { 2 },
                offset: 0,
                bytes: &exact[..split],
            }];
            if split < exact.len() {
                fragments.push(PrivateFragmentRef {
                    index: 1,
                    offset: split as u64,
                    bytes: &exact[split..],
                    ..fragments[0].clone()
                });
            }
            let (bytes, locator) =
                authenticated_test_segment(&fragments, &envelope.archive_id, &keys, false);
            let (expected_reads, data_end) = data_test_reads(&bytes, &locator);
            let expected: [u8; 32] = super::Sha256::digest(
                super::wire::t1(
                    "entrybound/segment-data/v1",
                    &[
                        &locator.header,
                        &super::data_sequence(locator.count, &bytes[64..data_end]).unwrap(),
                    ],
                )
                .unwrap(),
            )
            .into();
            assert_eq!(
                super::segment_data_digest(&locator.header, locator.count, &bytes[64..data_end])
                    .unwrap(),
                expected
            );
            let mut changed_end = bytes.clone();
            changed_end[data_end + 32] ^= 1;
            assert_eq!(
                super::segment_data_digest(
                    &locator.header,
                    locator.count,
                    &changed_end[64..data_end],
                )
                .unwrap(),
                expected
            );
            let work = super::CryptoWorkContext::new(policy);
            let retained = super::WorkBytes::zeroed(work.budget(), 19).unwrap();
            let (result, observed) = data_test_run(
                bytes,
                &locator,
                &envelope,
                &keys,
                EncryptedOpenOptions::new(None),
                &work,
            );
            assert_eq!(observed, expected_reads);
            let object = result.unwrap();
            assert_eq!(object.exact_bytes(), exact);
            assert_eq!(object.object_id(), id);
            assert!(work.budget().live_bytes() >= 19 + exact.len() as u64);
            drop(object);
            assert_eq!(work.budget().live_bytes(), 19);
            drop(retained);
            assert_eq!(work.budget().live_bytes(), 0);
        }
        for include_index in [true, false] {
            let fixture = known_fixture(include_index);
            let state = Arc::new(Mutex::new(KnownState::default()));
            let mut reader = known_open(&fixture, &state, segment_access_policy());
            let budget = reader.work.budget().clone();
            for (name, expected) in &fixture.expected {
                let read = reader
                    .read_entry(&LogicalPath::from_utf8([name.as_str()]).unwrap())
                    .unwrap();
                assert_eq!(&*read.bytes, expected);
            }
            drop(reader);
            assert_eq!(budget.live_bytes(), 0);
        }
    }

    #[test]
    fn segment_data_digest_authentication_end_and_fragment_precedence() {
        use super::wire::PrivateFragmentRef;

        let envelope_bytes = data_test_envelope();
        let policy = crate::crypto::CryptoPolicy::default();
        let envelope =
            super::wire::EnvelopeRef::decode_with_policy(&envelope_bytes, policy).unwrap();
        let keys = super::KeyHierarchy::derive(&[3; 32], &envelope.archive_id).unwrap();
        let exact =
            super::wire::private_object(super::wire::PRIVATE_OBJECT_CHUNK, b"abcdef").unwrap();
        let first = PrivateFragmentRef {
            object_id: super::wire::encrypted_object_id(&exact).unwrap(),
            total_len: exact.len() as u64,
            index: 0,
            count: 2,
            offset: 0,
            bytes: &exact[..9],
        };
        let second = PrivateFragmentRef {
            index: 1,
            offset: 9,
            bytes: &exact[9..],
            ..first.clone()
        };
        for case in [
            "data-tag",
            "overrun",
            "count",
            "index",
            "offset",
            "incomplete",
            "id",
            "id-with-end",
            "end-tag",
            "end-prefix",
            "end-padding",
            "end-cap",
            "end-kind",
            "end-ordinal",
            "end-class",
            "end-count",
            "end-private-total",
            "end-cipher-total",
            "end-digest",
        ] {
            let mut fragments = vec![first.clone(), second.clone()];
            if case == "end-cap" {
                // Two-byte fragments leave their canonical private records
                // smaller than the authentic six-field END private record.
                fragments = exact
                    .chunks(2)
                    .enumerate()
                    .map(|(index, bytes)| PrivateFragmentRef {
                        object_id: first.object_id,
                        total_len: exact.len() as u64,
                        index: index as u32,
                        count: exact.len().div_ceil(2) as u32,
                        offset: (index * 2) as u64,
                        bytes,
                    })
                    .collect();
            }
            match case {
                "overrun" => fragments.iter_mut().for_each(|f| f.total_len -= 1),
                "count" => fragments[1].count += 1,
                "index" => fragments[1].index = 0,
                "offset" => fragments[1].offset -= 1,
                "incomplete" => fragments.iter_mut().for_each(|f| f.total_len += 1),
                "id" | "id-with-end" => fragments.iter_mut().for_each(|f| f.object_id[0] ^= 1),
                _ => {}
            }
            let competing_end = !case.starts_with("end-") && case != "id"
                || matches!(case, "end-tag" | "end-prefix" | "end-padding" | "end-cap");
            let (mut bytes, locator) =
                authenticated_test_segment(&fragments, &envelope.archive_id, &keys, competing_end);
            let (expected_reads, end) = data_test_reads(&bytes, &locator);
            if case == "data-tag" {
                bytes[96] ^= 1;
            } else if case == "end-tag" {
                bytes[end + 32] ^= 1;
            } else if case.starts_with("end-") && case != "end-cap" {
                data_test_reseal_end(&mut bytes, &locator, &envelope, &keys, |padded| {
                    data_test_change_end(padded, case);
                });
            }
            let mut options = EncryptedOpenOptions::new(None);
            if case == "end-cap" {
                let data_private_max = fragments
                    .iter()
                    .map(|f| {
                        super::wire::encode_private_fragment(
                            &f.object_id,
                            f.total_len,
                            f.index,
                            f.count,
                            f.offset,
                            f.bytes,
                        )
                        .unwrap()
                        .len()
                    })
                    .max()
                    .unwrap();
                let end_private_len = bytes.len() - end - 32 - 16 - 8;
                assert!(end_private_len > data_private_max);
                options.crypto_policy.max_private_record_bytes = data_private_max as u64;
            }
            let work = super::CryptoWorkContext::new(options.crypto_policy);
            let (result, observed) =
                data_test_run(bytes, &locator, &envelope, &keys, options, &work);
            assert_eq!(observed, expected_reads, "{case}");
            let error = result.unwrap_err();
            let (class, code, detail) = match case {
                "data-tag" | "end-tag" => (
                    super::OutcomeClass::Corrupt,
                    super::ReasonCode::CryptoAeadAuthFailed,
                    "encrypted record authentication failed",
                ),
                "overrun" => (
                    super::OutcomeClass::Nonconforming,
                    super::ReasonCode::CryptoPrivateObjectInvalid,
                    "private fragment exceeds its declared object extent",
                ),
                "count" | "index" | "offset" => (
                    super::OutcomeClass::Nonconforming,
                    super::ReasonCode::CryptoPrivateObjectInvalid,
                    "private fragments are not contiguous and exact",
                ),
                "incomplete" => (
                    super::OutcomeClass::Truncated,
                    super::ReasonCode::TruncatedStream,
                    "encrypted object fragments are incomplete",
                ),
                "id" => (
                    super::OutcomeClass::Corrupt,
                    super::ReasonCode::CryptoPrivateObjectInvalid,
                    "encrypted object identity mismatch",
                ),
                "end-prefix" => (
                    super::OutcomeClass::Nonconforming,
                    super::ReasonCode::CryptoPrivateObjectInvalid,
                    "private length exceeds authenticated plaintext",
                ),
                "end-padding" => (
                    super::OutcomeClass::Nonconforming,
                    super::ReasonCode::CryptoPaddingInvalid,
                    "authenticated padding is noncanonical",
                ),
                "end-cap" => (
                    super::OutcomeClass::PolicyRefused,
                    super::ReasonCode::CryptoResourcePolicyRefused,
                    "segment END private record exceeds caller policy",
                ),
                "end-kind" => (
                    super::OutcomeClass::Nonconforming,
                    super::ReasonCode::CryptoSegmentStructureInvalid,
                    "END plaintext is not SegmentEndV1",
                ),
                _ => (
                    super::OutcomeClass::Nonconforming,
                    super::ReasonCode::CryptoSegmentStructureInvalid,
                    "SegmentEnd totals/digest mismatch",
                ),
            };
            assert_eq!(
                (error.class(), error.code(), error.detail()),
                (class, code, detail),
                "{case}"
            );
            assert_eq!(work.budget().live_bytes(), 0, "{case}");
        }
    }

    #[test]
    fn segment_data_digest_original_owner_pressure_and_teardown() {
        use super::wire::PrivateFragmentRef;
        use super::{PrerequisiteKind as K, PrerequisitePhase as P};

        let envelope_bytes = data_test_envelope();
        let work = test_work(512 * 1024);
        let envelope =
            super::wire::EnvelopeRef::decode_with_policy(&envelope_bytes, work.policy()).unwrap();
        let keys = super::KeyHierarchy::derive(&[3; 32], &envelope.archive_id).unwrap();
        let exact =
            super::wire::private_object(super::wire::PRIVATE_OBJECT_CHUNK, b"budget DATA").unwrap();
        let fragment = PrivateFragmentRef {
            object_id: super::wire::encrypted_object_id(&exact).unwrap(),
            total_len: exact.len() as u64,
            index: 0,
            count: 1,
            offset: 0,
            bytes: &exact,
        };
        let caller = super::WorkBytes::zeroed(work.budget(), 19).unwrap();
        for short in [false, true] {
            let state = Arc::new(Mutex::new(PrerequisiteState {
                pressure_kind: Some(K::Physical),
                pressure_occurrence: 1,
                short,
                ..PrerequisiteState::default()
            }));
            {
                let _observer = PrerequisiteObserverGuard::install(&state);
                let result = super::PhysicalPrerequisites::reserve_empty(work.budget(), 4);
                if short {
                    let error = result.err().expect("one-byte-short owner must refuse");
                    assert_eq!(
                        error.detail(),
                        "managed crypto replacement and required headroom exceed caller policy"
                    );
                } else {
                    drop(result.unwrap());
                }
            }
            let mut state = state.lock().unwrap();
            assert!(state.hit);
            assert_eq!(
                state
                    .events
                    .iter()
                    .filter(|e| e.0 == K::Physical && e.1 == P::BeforeReserve)
                    .count(),
                1
            );
            assert_eq!(
                state
                    .events
                    .iter()
                    .filter(|e| e.0 == K::Physical && e.1 == P::Reserved)
                    .count(),
                if short { 0 } else { 1 }
            );
            assert!(state.active.is_empty());
            drop(state.held.take());
            assert_eq!(work.budget().live_bytes(), 19);
        }
        for short in [false, true] {
            let (bytes, locator) = authenticated_test_segment(
                std::slice::from_ref(&fragment),
                &envelope.archive_id,
                &keys,
                false,
            );
            let expected_reads = data_test_reads(&bytes, &locator).0;
            // This is a source-fill pressure seam, not an allocator callback.
            // The original object WorkBytes reservation reaches allocate's
            // budget.reserve before Vec::new/try_reserve_exact.
            let state = Arc::new(Mutex::new(KnownState {
                budget: Some(work.budget().clone()),
                target: Some((0, locator.extent)),
                fill_leave: Some(exact.len() as u64 - u64::from(short)),
                ..KnownState::default()
            }));
            let mut session = super::RangeSession::new_crypto(
                Box::new(known_source(bytes, &state)),
                segment_access_policy(),
                work.budget().clone(),
            )
            .unwrap();
            let mut options = EncryptedOpenOptions::new(None);
            options.crypto_policy = work.policy();
            let result = super::decrypt_segment(
                &mut session,
                &locator,
                0,
                &envelope,
                &keys,
                PaddingMode::None,
                options,
                super::AccessPurpose::EncryptedPayload,
                &work,
            );
            assert_eq!(state.lock().unwrap().reads, expected_reads);
            assert_eq!(state.lock().unwrap().hits, 1);
            assert_eq!(
                state.lock().unwrap().fill_available,
                exact.len() as u64 - u64::from(short)
            );
            if short {
                let error = result.unwrap_err();
                assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
                assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
                assert_eq!(
                    error.detail(),
                    "managed crypto working capacity exceeds caller policy"
                );
            } else {
                let object = result.unwrap();
                assert_eq!(object.exact_bytes(), exact);
                drop(object);
            }
            drop(session);
            drop(state.lock().unwrap().held.take());
            assert_eq!(work.budget().live_bytes(), 19);
        }
        for mode in ["normal", "error", "unwind"] {
            let (bytes, locator) = authenticated_test_segment(
                std::slice::from_ref(&fragment),
                &envelope.archive_id,
                &keys,
                mode == "error",
            );
            let mut options = EncryptedOpenOptions::new(None);
            options.crypto_policy = work.policy();
            let completed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let (result, _) = data_test_run(bytes, &locator, &envelope, &keys, options, &work);
                if mode == "error" {
                    assert_eq!(
                        result.unwrap_err().detail(),
                        "SegmentEnd totals/digest mismatch"
                    );
                } else {
                    let object = result.unwrap();
                    assert_eq!(object.exact_bytes(), exact);
                    if mode == "unwind" {
                        panic!("generated DATA object unwind");
                    }
                    drop(object);
                }
            }));
            assert_eq!(completed.is_err(), mode == "unwind");
            assert_eq!(work.budget().live_bytes(), 19);
        }
        drop(caller);
        assert_eq!(work.budget().live_bytes(), 0);
    }

    #[test]
    fn segment_data_digest_source_refusals_and_revision_stop() {
        use super::wire::PrivateFragmentRef;

        let envelope_bytes = data_test_envelope();
        let policy = crate::crypto::CryptoPolicy::default();
        let envelope =
            super::wire::EnvelopeRef::decode_with_policy(&envelope_bytes, policy).unwrap();
        let keys = super::KeyHierarchy::derive(&[3; 32], &envelope.archive_id).unwrap();
        let exact =
            super::wire::private_object(super::wire::PRIVATE_OBJECT_CHUNK, b"stable DATA").unwrap();
        let fragment = PrivateFragmentRef {
            object_id: super::wire::encrypted_object_id(&exact).unwrap(),
            total_len: exact.len() as u64,
            index: 0,
            count: 1,
            offset: 0,
            bytes: &exact,
        };
        let (bytes, locator) =
            authenticated_test_segment(&[fragment], &envelope.archive_id, &keys, false);
        let expected_reads = data_test_reads(&bytes, &locator).0;
        let mut options = EncryptedOpenOptions::new(None);
        options.crypto_policy.max_ciphertext_record_bytes =
            u64::from_be_bytes(bytes[88..96].try_into().unwrap()) - 1;
        let work = super::CryptoWorkContext::new(options.crypto_policy);
        let (result, observed) =
            data_test_run(bytes.clone(), &locator, &envelope, &keys, options, &work);
        let error = result.unwrap_err();
        assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        assert_eq!(error.detail(), "protected ciphertext exceeds caller policy");
        assert_eq!(observed, [(64, 32)]);
        assert_eq!(work.budget().live_bytes(), 0);

        struct ChangingSource {
            source: MemoryRandomReadSource,
            reads: Arc<Mutex<Vec<(u64, u64)>>>,
            changed: AtomicU64,
            full_extent: u64,
        }
        impl RandomReadSource for ChangingSource {
            fn len(&self) -> crate::diagnostics::Result<u64> {
                self.source.len()
            }
            fn revision(&self) -> crate::diagnostics::Result<SourceRevision> {
                let mut revision = self.source.revision()?;
                if self.changed.load(Ordering::Acquire) != 0
                    && let SourceRevision::Memory { sha256, .. } = &mut revision
                {
                    sha256[0] ^= 1;
                }
                Ok(revision)
            }
            fn read_exact_at(&self, _: u64, _: u64) -> crate::diagnostics::Result<Vec<u8>> {
                panic!("DATA tests require bounded fills")
            }
            fn read_exact_at_into(
                &self,
                offset: u64,
                dst: &mut [u8],
            ) -> crate::diagnostics::Result<()> {
                self.reads.lock().unwrap().push((offset, dst.len() as u64));
                self.source.read_exact_at_into(offset, dst)?;
                if offset == 0 && dst.len() as u64 == self.full_extent {
                    self.changed.store(1, Ordering::Release);
                }
                Ok(())
            }
        }
        let reads = Arc::new(Mutex::new(Vec::new()));
        let work = super::CryptoWorkContext::new(policy);
        let mut session = super::RangeSession::new_crypto(
            Box::new(ChangingSource {
                source: MemoryRandomReadSource::new(bytes),
                reads: Arc::clone(&reads),
                changed: AtomicU64::new(0),
                full_extent: locator.extent,
            }),
            segment_access_policy(),
            work.budget().clone(),
        )
        .unwrap();
        let error = super::decrypt_segment(
            &mut session,
            &locator,
            0,
            &envelope,
            &keys,
            PaddingMode::None,
            EncryptedOpenOptions::new(None),
            super::AccessPurpose::EncryptedPayload,
            &work,
        )
        .unwrap_err();
        assert_eq!(error.class(), super::OutcomeClass::Corrupt);
        assert_eq!(error.code(), super::ReasonCode::SourceUnstable);
        assert_eq!(
            error.detail(),
            "source revision changed during random access"
        );
        assert_eq!(*reads.lock().unwrap(), expected_reads);
        drop(session);
        assert_eq!(work.budget().live_bytes(), 0);
    }

    fn authenticated_test_segment(
        fragments: &[super::wire::PrivateFragmentRef<'_>],
        archive_id: &[u8; 32],
        keys: &super::KeyHierarchy,
        wrong_end: bool,
    ) -> (Vec<u8>, super::SegmentLocator) {
        use crate::canonical::RecordBuilder;
        use sha2::Digest as _;

        let count = u32::try_from(fragments.len()).unwrap();
        let class = super::SEGMENT_PAYLOAD;
        let salt = [9; 16];
        let private = fragments
            .iter()
            .map(|fragment| {
                super::wire::encode_private_fragment(
                    &fragment.object_id,
                    fragment.total_len,
                    fragment.index,
                    fragment.count,
                    fragment.offset,
                    fragment.bytes,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let private_total = private.iter().map(|value| value.len() as u64).sum::<u64>();
        let ciphertext_total = private
            .iter()
            .map(|value| (8 + value.len() + 16) as u64)
            .sum::<u64>();
        let end_record = |digest: &[u8; 32]| {
            let mut record = RecordBuilder::new(super::wire::RECORD_SEGMENT_END);
            record
                .u64(1, 0)
                .unwrap()
                .u8(2, class)
                .unwrap()
                .u32(3, count)
                .unwrap()
                .u64(4, private_total)
                .unwrap()
                .u64(5, ciphertext_total)
                .unwrap()
                .bytes(6, digest)
                .unwrap();
            record.finish().unwrap()
        };
        let extent = 64
            + u64::from(count) * 32
            + ciphertext_total
            + 32
            + 8
            + end_record(&[0; 32]).len() as u64
            + 16;
        let mut header = [0; 64];
        header[..4].copy_from_slice(b"EBSG");
        header[4..6].copy_from_slice(&1_u16.to_be_bytes());
        header[6] = class;
        header[16..32].copy_from_slice(&salt);
        header[32..36].copy_from_slice(&count.to_be_bytes());
        header[48..56].copy_from_slice(&extent.to_be_bytes());
        assert_eq!(
            super::parse_segment_header(&header, 0).unwrap(),
            (class, salt, count, extent)
        );
        let key = super::super::container::segment_key(keys, archive_id, class, 0, &salt).unwrap();
        let seal = |class: u8, counter: u64, value: &[u8], nonce: &[u8; 12]| {
            let mut protected = [0; 32];
            protected[..4].copy_from_slice(b"EBC1");
            protected[4..6].copy_from_slice(&1_u16.to_be_bytes());
            protected[6] = class;
            protected[16..24].copy_from_slice(&counter.to_be_bytes());
            protected[24..32].copy_from_slice(&((8 + value.len() + 16) as u64).to_be_bytes());
            let mut padded = (value.len() as u64).to_be_bytes().to_vec();
            padded.extend_from_slice(value);
            let ciphertext = super::super::aead_seal(
                &key.0,
                nonce,
                &super::super::container::record_ad(archive_id, &header, &protected).unwrap(),
                &padded,
            )
            .unwrap();
            let mut record = protected.to_vec();
            record.extend_from_slice(&ciphertext);
            record
        };
        let mut data = Vec::new();
        for (index, value) in private.iter().enumerate() {
            let counter = index as u64;
            data.extend_from_slice(&seal(
                super::RECORD_DATA,
                counter,
                value,
                &super::super::container::data_nonce(counter),
            ));
        }
        let mut digest: [u8; 32] = super::Sha256::digest(
            super::wire::t1(
                "entrybound/segment-data/v1",
                &[&header, &super::data_sequence(count, &data).unwrap()],
            )
            .unwrap(),
        )
        .into();
        if wrong_end {
            digest[0] ^= 1;
        }
        let mut segment = header.to_vec();
        segment.extend_from_slice(&data);
        segment.extend_from_slice(&seal(
            super::RECORD_END,
            u64::from(count),
            &end_record(&digest),
            &super::super::container::end_nonce(u64::from(count)),
        ));
        assert_eq!(segment.len() as u64, extent);
        (
            segment,
            super::SegmentLocator {
                offset: 0,
                extent,
                class,
                salt,
                count,
                header,
                left: super::NO_SEGMENT_SLOT,
                right: super::NO_SEGMENT_SLOT,
                parent: super::NO_SEGMENT_SLOT,
                height: 1,
            },
        )
    }

    #[test]
    fn authenticated_fragment_cursor_checks_overrun_and_complete_authentication() {
        use super::wire::{CryptoEnvelope, PrivateFragmentRef, RecipientStanza};

        let policy = crate::crypto::CryptoPolicy::default();
        let envelope_bytes = CryptoEnvelope {
            archive_id: [4; 32],
            commitment: [5; 32],
            protection_policy: 2,
            padding_mode: 0,
            boundary_mode: 1,
            envelope_mac: [6; 32],
            stanzas: vec![RecipientStanza {
                stanza_type: 2,
                protection_class: 2,
                stanza_id: [1; 16],
                recipient_hint: [0; 16],
                method_parameters: vec![0; super::wire::PASSWORD_METHOD_LEN],
                encapsulation: Vec::new(),
                wrap_nonce: [2; 12],
                wrapped_afk: [3; 48],
            }],
        }
        .encode()
        .unwrap();
        let envelope =
            super::wire::EnvelopeRef::decode_with_policy(&envelope_bytes, policy).unwrap();
        let keys = super::KeyHierarchy::derive(&[3; 32], &envelope.archive_id).unwrap();
        let exact =
            super::wire::private_object(super::wire::PRIVATE_OBJECT_CHUNK, b"abcdef").unwrap();
        let id = super::wire::encrypted_object_id(&exact).unwrap();
        let first = PrivateFragmentRef {
            object_id: id,
            total_len: exact.len() as u64,
            index: 0,
            count: 2,
            offset: 0,
            bytes: &exact[..9],
        };
        let second = PrivateFragmentRef {
            index: 1,
            offset: 9,
            bytes: &exact[9..],
            ..first.clone()
        };
        for case in [
            "valid",
            "overrun",
            "id",
            "count",
            "index",
            "offset",
            "incomplete",
            "end",
            "tag",
        ] {
            let mut fragments = [first.clone(), second.clone()];
            match case {
                "overrun" => {
                    for fragment in &mut fragments {
                        fragment.total_len -= 1;
                    }
                }
                "id" => {
                    for fragment in &mut fragments {
                        fragment.object_id[0] ^= 1;
                    }
                }
                "count" => fragments[1].count = 3,
                "index" => fragments[1].index = 0,
                "offset" => fragments[1].offset = 8,
                "incomplete" => {
                    for fragment in &mut fragments {
                        fragment.total_len += 1;
                    }
                }
                _ => {}
            }
            let (mut bytes, locator) =
                authenticated_test_segment(&fragments, &envelope.archive_id, &keys, case == "end");
            if case == "tag" {
                bytes[64 + 32] ^= 1;
            }
            let work = super::CryptoWorkContext::new(policy);
            let mut session = super::RangeSession::new_crypto(
                Box::new(MemoryRandomReadSource::new(bytes)),
                RandomAccessPolicy {
                    max_cached_bytes: 0,
                    ..RandomAccessPolicy::default()
                },
                work.budget().clone(),
            )
            .unwrap();
            let result = super::decrypt_segment(
                &mut session,
                &locator,
                0,
                &envelope,
                &keys,
                PaddingMode::None,
                EncryptedOpenOptions::new(None),
                super::AccessPurpose::EncryptedPayload,
                &work,
            );
            if case == "valid" {
                let object = result.unwrap();
                assert_eq!(object.exact_bytes(), exact);
                assert_eq!(object.object_id(), id);
                drop(object);
            } else {
                let error = result.unwrap_err();
                let (class, code) = match case {
                    "id" => (
                        super::OutcomeClass::Corrupt,
                        super::ReasonCode::CryptoPrivateObjectInvalid,
                    ),
                    "incomplete" => (
                        super::OutcomeClass::Truncated,
                        super::ReasonCode::TruncatedStream,
                    ),
                    "end" => (
                        super::OutcomeClass::Nonconforming,
                        super::ReasonCode::CryptoSegmentStructureInvalid,
                    ),
                    "tag" => (
                        super::OutcomeClass::Corrupt,
                        super::ReasonCode::CryptoAeadAuthFailed,
                    ),
                    _ => (
                        super::OutcomeClass::Nonconforming,
                        super::ReasonCode::CryptoPrivateObjectInvalid,
                    ),
                };
                assert_eq!((error.class(), error.code()), (class, code), "{case}");
                if case == "overrun" {
                    assert_eq!(
                        error.detail(),
                        "private fragment exceeds its declared object extent"
                    );
                }
            }
            drop(session);
            assert_eq!(work.budget().live_bytes(), 0, "{case}");
        }
    }

    #[test]
    fn recursive_prefix_decode_shares_live_object_budget_and_keeps_outer_frame() {
        use crate::crypto::working::WorkBudget;
        use crate::planner::CompressionProfile;

        #[derive(Default)]
        struct Pressure {
            budget: Option<WorkBudget>,
            target: Option<(u64, u64)>,
            matching_reads: usize,
            observed_live: u64,
            retained: Option<super::WorkBytes>,
        }
        struct PressureSource {
            source: MemoryRandomReadSource,
            pressure: Arc<Mutex<Pressure>>,
        }
        impl RandomReadSource for PressureSource {
            fn len(&self) -> crate::diagnostics::Result<u64> {
                self.source.len()
            }
            fn revision(&self) -> crate::diagnostics::Result<SourceRevision> {
                self.source.revision()
            }
            fn read_exact_at(&self, _: u64, _: u64) -> crate::diagnostics::Result<Vec<u8>> {
                panic!("recursive encrypted reads must use the bounded source callback")
            }
            fn read_exact_at_into(
                &self,
                offset: u64,
                dst: &mut [u8],
            ) -> crate::diagnostics::Result<()> {
                let mut pressure = self.pressure.lock().unwrap();
                if pressure.target == Some((offset, dst.len() as u64)) {
                    pressure.matching_reads += 1;
                    // The first fetch is the prerequisite header check. The
                    // second is the real recursive decode_chunk producer.
                    if pressure.matching_reads == 2 {
                        let budget = pressure.budget.as_ref().unwrap();
                        pressure.observed_live = budget.live_bytes();
                        let budget = pressure.budget.as_ref().unwrap();
                        pressure.retained = Some(super::WorkBytes::zeroed(
                            budget,
                            usize::try_from(budget.available_bytes()).unwrap(),
                        )?);
                    }
                }
                drop(pressure);
                self.source.read_exact_at_into(offset, dst)
            }
        }

        // This is the existing generated Dense lookback fixture, encrypted
        // here so the actual range decoder enters its recursive prefix path.
        let directory = TestDir::new();
        let mut state = 0xbb67_ae85_84ca_a73b_u64;
        let base = (0..64 * 1024)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect::<Vec<_>>();
        for index in 0..12 {
            let mut bytes = base.clone();
            let first = 128 + (index * 977) % (bytes.len() - 512);
            for (offset, byte) in bytes[first..first + 256].iter_mut().enumerate() {
                *byte = (index as u8).wrapping_mul(31).wrapping_add(offset as u8);
            }
            std::fs::write(
                directory.path().join(format!("sample-{index:03}.bin")),
                bytes,
            )
            .unwrap();
        }
        let archive = plan_directory(
            directory.path(),
            PackOptions {
                profile: CompressionProfile::Dense,
                ..PackOptions::default()
            },
        )
        .unwrap();
        let target = archive
            .content_store
            .physical_order
            .iter()
            .rev()
            .find(|id| archive.content_store.chunks[id].group_ref.is_some())
            .copied()
            .expect("generated fixture selects a prefix-coded Chunk");
        let expected = &archive.content_store.chunks[&target].plaintext;
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: &[recipient],
                padding: PaddingMode::None,
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let pressure = Arc::new(Mutex::new(Pressure::default()));
        let mut options = EncryptedOpenOptions::new(Some(Unlock::Identity(&identity)));
        // A finite generated-input-sized caller cap, not an engine coefficient.
        options.crypto_policy.max_working_memory_bytes = 12 * base.len() as u64;
        let mut opened = open_indexed_random_encrypted(
            PressureSource {
                source: MemoryRandomReadSource::new(encrypted.bytes),
                pressure: Arc::clone(&pressure),
            },
            RandomAccessPolicy {
                max_cached_bytes: 0,
                ..RandomAccessPolicy::default()
            },
            options,
        )
        .unwrap();
        opened.load_support_and_index_fallback().unwrap();
        let header = opened.chunk_header(target).unwrap();
        let predecessors = opened.group_prerequisites(target, &header).unwrap();
        assert!(!predecessors.is_empty());
        let ordinal = opened
            .index
            .get(&predecessors.iter().next().unwrap())
            .unwrap()
            .segment_ordinal as usize;
        let predecessor = &opened.segments.as_slice()[ordinal];
        {
            let mut pressure = pressure.lock().unwrap();
            pressure.budget = Some(opened.work.budget().clone());
            pressure.target = Some((predecessor.offset, predecessor.extent));
        }
        let mut decoded = super::BTreeMap::new();
        let error = opened.decode_chunk(target, &mut decoded).unwrap_err();
        assert_eq!(error.class(), super::OutcomeClass::PolicyRefused);
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        assert!(decoded.is_empty());
        let retained = {
            let mut pressure = pressure.lock().unwrap();
            assert_eq!(pressure.matching_reads, 2);
            assert!(pressure.observed_live > 0);
            pressure.target = None;
            pressure.retained.take().unwrap()
        };
        let live = opened.work.budget().live_bytes();
        let retained_len = retained.len() as u64;
        assert!(retained_len > 0);
        drop(retained);
        assert_eq!(opened.work.budget().live_bytes(), live - retained_len);
        opened.decode_chunk(target, &mut decoded).unwrap();
        assert!(
            decoded.len() > 1,
            "actual recursive prerequisite decoding occurred"
        );
        assert_eq!(decoded[&target].plaintext.as_ref(), expected.as_ref());
    }

    struct TestDir(PathBuf);
    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "entrybound-crypto-random-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn hybrid_random_access_authenticates_control_terminal_and_requested_payload() {
        let directory = TestDir::new();
        std::fs::write(
            directory.path().join("wanted.bin"),
            b"authenticated range bytes",
        )
        .unwrap();
        std::fs::write(
            directory.path().join("unread.bin"),
            vec![91_u8; 2 * 1024 * 1024],
        )
        .unwrap();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let recipients = [recipient];
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: &recipients,
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let source = MemoryRandomReadSource::new(encrypted.bytes);
        let mut opened = open_indexed_random_encrypted(
            source,
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
        )
        .unwrap();
        let path = LogicalPath::from_utf8(["wanted.bin"]).unwrap();
        let read = opened.read_entry(&path).unwrap();
        assert_eq!(&*read.bytes, b"authenticated range bytes");
        assert!(read.report.content_object_digest_verified);
        assert!(!read.report.whole_archive_verified);
        assert!(read.report.bytes_fetched < opened.metadata().source_length);
    }

    #[test]
    fn encrypted_missing_index_rebuilds_authenticated_payload_objects() {
        let directory = TestDir::new();
        std::fs::write(directory.path().join("wanted.bin"), b"encrypted fallback").unwrap();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: &[recipient],
                include_index: false,
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let mut opened = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(encrypted.bytes),
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
        )
        .unwrap();
        let read = opened
            .read_entry(&LogicalPath::from_utf8(["wanted.bin"]).unwrap())
            .unwrap();
        assert_eq!(&*read.bytes, b"encrypted fallback");
        assert_eq!(
            read.report.index_status,
            crate::ecf::RandomAccessIndexStatus::RebuiltAbsent
        );
    }

    #[test]
    fn password_random_access_uses_the_existing_policy_checked_unlock() {
        let directory = TestDir::new();
        std::fs::write(directory.path().join("wanted.bin"), b"password range bytes").unwrap();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let password = b"high entropy password for range testing";
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                password: Some(password),
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let mut matrix_only = EncryptedOpenOptions::new(Some(Unlock::Password(password)));
        matrix_only.crypto_policy.max_working_memory_bytes = 262_144 * 1024;
        let error = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(encrypted.bytes.clone()),
            RandomAccessPolicy::default(),
            matrix_only,
        )
        .err()
        .expect("live range buffers and Argon2 must share the same budget");
        assert_eq!(error.code(), super::ReasonCode::CryptoResourcePolicyRefused);
        let mut opened = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(encrypted.bytes),
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(Unlock::Password(password))),
        )
        .unwrap();
        let read = opened
            .read_entry(&LogicalPath::from_utf8(["wanted.bin"]).unwrap())
            .unwrap();
        assert_eq!(&*read.bytes, b"password range bytes");
        assert!(!read.report.whole_archive_verified);
    }

    #[test]
    fn unread_encrypted_payload_corruption_does_not_become_a_whole_archive_claim() {
        let directory = TestDir::new();
        std::fs::write(
            directory.path().join("wanted.bin"),
            b"authenticated range bytes",
        )
        .unwrap();
        std::fs::write(
            directory.path().join("unread.bin"),
            vec![17_u8; 1024 * 1024],
        )
        .unwrap();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let unread = archive
            .entry_set
            .entries()
            .iter()
            .find(|entry| entry.path().to_string() == "unread.bin")
            .and_then(|entry| match entry.data() {
                EntryData::File {
                    content: ContentRef::Internal(content),
                } => archive.content_store.objects.get(content),
                EntryData::Directory
                | EntryData::Symlink { .. }
                | EntryData::ReparsePoint { .. } => None,
            })
            .unwrap()
            .chunks[0]
            .chunk_id;
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: &[recipient],
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let probe = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(encrypted.bytes.clone()),
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
        )
        .unwrap();
        let ordinal = probe.index.get(&unread).unwrap().segment_ordinal;
        let segment = &probe.segments.as_slice()[usize::try_from(ordinal).unwrap()];
        let mutation =
            usize::try_from(segment.offset + SEGMENT_HEADER_LEN + PROTECTED_HEADER_LEN).unwrap();
        let mut corrupted = encrypted.bytes;
        corrupted[mutation] ^= 0x40;
        let mut opened = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(corrupted),
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
        )
        .unwrap();
        let read = opened
            .read_entry(&LogicalPath::from_utf8(["wanted.bin"]).unwrap())
            .unwrap();
        assert_eq!(&*read.bytes, b"authenticated range bytes");
        assert!(!read.report.whole_archive_verified);
        assert_eq!(
            read.report.pci,
            crate::ecf::IdentityVerificationStatus::NotComputed
        );
    }
}
