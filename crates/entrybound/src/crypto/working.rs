//! Managed variable-capacity crypto work; not an all-heap or RSS bound.
//!
//! The fixed shared counter and diagnostic control plane are outside this
//! counter. Allocator request/capacity behavior still requires qualification
//! for each supported toolchain/target before claiming the aggregate bound.

use std::alloc::Layout;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use zeroize::Zeroize as _;

use super::{CryptoPolicy, resource_refused};
use crate::diagnostics::Result;

struct BudgetState {
    limit: u64,
    live: AtomicU64,
}

/// One immutable limit shared by nested work. Only this fixed control plane
/// may allocate outside the managed variable-capacity counter.
#[derive(Clone)]
pub(crate) struct WorkBudget {
    state: Arc<BudgetState>,
}

impl WorkBudget {
    pub(crate) fn new(limit: u64) -> Self {
        Self {
            state: Arc::new(BudgetState {
                limit,
                live: AtomicU64::new(0),
            }),
        }
    }

    pub(crate) fn reserve(&self, bytes: u64) -> Result<WorkLease> {
        let mut live = self.state.live.load(Ordering::Acquire);
        loop {
            let next = live
                .checked_add(bytes)
                .filter(|next| *next <= self.state.limit)
                .ok_or_else(|| {
                    resource_refused("managed crypto working capacity exceeds caller policy")
                })?;
            match self.state.live.compare_exchange_weak(
                live,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Ok(WorkLease {
                        budget: self.clone(),
                        bytes,
                    });
                }
                Err(observed) => live = observed,
            }
        }
    }

    pub(crate) fn available_bytes(&self) -> u64 {
        self.state
            .limit
            .saturating_sub(self.state.live.load(Ordering::Acquire))
    }

    #[cfg(test)]
    pub(crate) fn live_bytes(&self) -> u64 {
        self.state.live.load(Ordering::Acquire)
    }
}

/// The original caller policy and one counter for the complete crypto operation.
/// Clones share the counter; nested helpers never construct a new default budget.
#[derive(Clone)]
pub(crate) struct CryptoWorkContext {
    policy: CryptoPolicy,
    budget: WorkBudget,
}

impl CryptoWorkContext {
    pub(crate) fn new(policy: CryptoPolicy) -> Self {
        Self {
            policy,
            budget: WorkBudget::new(policy.max_working_memory_bytes),
        }
    }

    pub(crate) fn policy(&self) -> CryptoPolicy {
        self.policy
    }

    pub(crate) fn budget(&self) -> &WorkBudget {
        &self.budget
    }
}

/// A reservation is not Clone. Allocation owners retain it until backing free.
pub(crate) struct WorkLease {
    budget: WorkBudget,
    bytes: u64,
}

impl Drop for WorkLease {
    fn drop(&mut self) {
        let previous = self
            .budget
            .state
            .live
            .fetch_sub(self.bytes, Ordering::AcqRel);
        debug_assert!(previous >= self.bytes, "working reservation underflow");
    }
}

fn requested_bytes<T>(capacity: usize) -> Result<u64> {
    let layout = Layout::array::<T>(capacity)
        .map_err(|_| resource_refused("managed crypto capacity layout overflows"))?;
    u64::try_from(layout.size())
        .map_err(|_| resource_refused("managed crypto capacity exceeds u64"))
}

fn allocate<T>(budget: &WorkBudget, capacity: usize) -> Result<(Vec<T>, WorkLease)> {
    let lease = budget.reserve(requested_bytes::<T>(capacity)?)?;
    let mut storage = Vec::new();
    storage
        .try_reserve_exact(capacity)
        .map_err(|_| resource_refused("cannot allocate managed crypto backing"))?;
    // This is an adoption check, not a proof that an oversized allocator
    // allocation could never have happened. Runtime qualification is separate.
    if std::mem::size_of::<T>() != 0 && storage.capacity() != capacity {
        return Err(resource_refused(
            "allocator returned unqualified crypto backing capacity",
        ));
    }
    Ok((storage, lease))
}

/// A fixed-capacity, fully initialized secret Argon2 matrix.
/// Only a borrowed block slice is exposed, with no growth or Vec conversion.
pub(crate) struct WorkBlocks {
    storage: Vec<argon2::Block>,
    lease: Option<WorkLease>,
    #[cfg(test)]
    before_free: Option<fn(&[argon2::Block], &WorkBudget)>,
}

impl WorkBlocks {
    /// Early creation preflight only. The actual owner reserves again before
    /// allocation; this check cannot admit later work against a stale counter.
    pub(crate) fn check_fit(budget: &WorkBudget, count: usize) -> Result<()> {
        drop(budget.reserve(requested_bytes::<argon2::Block>(count)?)?);
        Ok(())
    }

    pub(crate) fn zeroed(budget: &WorkBudget, count: usize) -> Result<Self> {
        let (mut storage, lease) = allocate(budget, count)?;
        storage.resize_with(count, argon2::Block::new);
        Ok(Self {
            storage,
            lease: Some(lease),
            #[cfg(test)]
            before_free: None,
        })
    }

    pub(crate) fn as_mut_slice(&mut self) -> &mut [argon2::Block] {
        &mut self.storage
    }
}

impl Drop for WorkBlocks {
    fn drop(&mut self) {
        for block in &mut self.storage {
            block.zeroize();
        }
        #[cfg(test)]
        if let Some(observer) = self.before_free {
            observer(
                &self.storage,
                &self.lease.as_ref().expect("live matrix lease").budget,
            );
        }
        // Secret words are wiped and backing freed before the lease releases.
        drop(std::mem::take(&mut self.storage));
        drop(self.lease.take());
    }
}

/// A fixed-capacity, fully initialized secret-capable byte allocation.
/// No growable Vec or unmetered ownership conversion is exposed.
pub(crate) struct WorkBytes {
    storage: Vec<u8>,
    _lease: WorkLease,
}

impl WorkBytes {
    pub(crate) fn zeroed(budget: &WorkBudget, length: usize) -> Result<Self> {
        let (mut storage, lease) = allocate(budget, length)?;
        storage.resize(length, 0);
        Ok(Self {
            storage,
            _lease: lease,
        })
    }

    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.storage
    }

    pub(crate) fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.storage
    }
}

impl Deref for WorkBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl std::fmt::Debug for WorkBytes {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkBytes")
            .field("length", &self.storage.len())
            .finish_non_exhaustive()
    }
}

impl DerefMut for WorkBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl Drop for WorkBytes {
    fn drop(&mut self) {
        self.storage.as_mut_slice().zeroize();
        // Free backing before the field-owned lease is dropped.
        drop(std::mem::take(&mut self.storage));
    }
}

/// Nonsecret typed bookkeeping. Secret typed work needs a separate wiping,
/// fixed-capacity owner; this collection deliberately does not promise wiping.
pub(crate) struct WorkVec<T: Copy> {
    storage: Vec<T>,
    lease: WorkLease,
}

impl<T: Copy> WorkVec<T> {
    pub(crate) fn new(budget: &WorkBudget) -> Self {
        Self {
            storage: Vec::new(),
            lease: WorkLease {
                budget: budget.clone(),
                bytes: 0,
            },
        }
    }

    pub(crate) fn reserve(&mut self, additional: usize) -> Result<()> {
        self.reserve_with_headroom(additional, 0)
    }

    pub(crate) fn minimum_reservation_bytes(&self, additional: usize) -> Result<u64> {
        let required = self
            .storage
            .len()
            .checked_add(additional)
            .ok_or_else(|| resource_refused("managed crypto item count overflows"))?;
        if required <= self.storage.capacity() {
            Ok(0)
        } else {
            requested_bytes::<T>(required)
        }
    }

    pub(crate) fn reserve_with_headroom(&mut self, additional: usize, headroom: u64) -> Result<()> {
        let required = self
            .storage
            .len()
            .checked_add(additional)
            .ok_or_else(|| resource_refused("managed crypto item count overflows"))?;
        if required <= self.storage.capacity() {
            return Ok(());
        }
        let mut capacity = self
            .storage
            .capacity()
            .checked_mul(2)
            .unwrap_or(required)
            .max(required);
        if !requested_bytes::<T>(capacity).is_ok_and(|bytes| {
            bytes
                .checked_add(headroom)
                .is_some_and(|total| total <= self.lease.budget.available_bytes())
        }) {
            capacity = required;
        }
        if !requested_bytes::<T>(capacity)?
            .checked_add(headroom)
            .is_some_and(|total| total <= self.lease.budget.available_bytes())
        {
            return Err(resource_refused(
                "managed crypto replacement and required headroom exceed caller policy",
            ));
        }
        // The entire new allocation overlaps the still-live old allocation.
        let (mut replacement, lease) = allocate(&self.lease.budget, capacity)?;
        // Copy transfer must not invoke an independently implemented Clone.
        for &value in &self.storage {
            replacement.push(value);
        }
        let old_storage = std::mem::replace(&mut self.storage, replacement);
        let old_lease = std::mem::replace(&mut self.lease, lease);
        drop(old_storage);
        drop(old_lease);
        Ok(())
    }

    pub(crate) fn push(&mut self, value: T) -> Result<()> {
        self.reserve(1)?;
        self.storage.push(value);
        Ok(())
    }

    pub(crate) fn as_slice(&self) -> &[T] {
        &self.storage
    }
}

impl<T: Copy> Drop for WorkVec<T> {
    fn drop(&mut self) {
        drop(std::mem::take(&mut self.storage));
    }
}

#[cfg(test)]
mod tests {
    use super::{CryptoWorkContext, WorkBlocks, WorkBudget, WorkBytes, WorkVec, requested_bytes};
    use crate::crypto::CryptoPolicy;

    #[test]
    fn argon2_blocks_share_the_original_context_and_charge_complete_layout() {
        assert_eq!(std::mem::size_of::<argon2::Block>(), argon2::Block::SIZE);
        assert_eq!(std::mem::align_of::<argon2::Block>(), 64);
        let mut policy = CryptoPolicy {
            max_working_memory_bytes: 2 * 1024,
            ..CryptoPolicy::default()
        };
        let work = CryptoWorkContext::new(policy);
        let shared = work.clone();
        policy.max_working_memory_bytes = 1;
        assert_eq!(policy.max_working_memory_bytes, 1);
        assert_eq!(work.policy().max_working_memory_bytes, 2048);
        let retained = WorkBytes::zeroed(work.budget(), 1).unwrap();
        assert!(WorkBlocks::zeroed(shared.budget(), 2).is_err());
        assert_eq!(shared.budget().live_bytes(), 1);
        drop(retained);
        let mut blocks = WorkBlocks::zeroed(shared.budget(), 2).unwrap();
        assert_eq!(work.budget().live_bytes(), 2048);
        assert!(
            blocks
                .as_mut_slice()
                .iter()
                .all(|block| block.as_ref().iter().all(|word| *word == 0))
        );
        drop(blocks);
        assert_eq!(work.budget().live_bytes(), 0);
        assert!(WorkBlocks::zeroed(work.budget(), usize::MAX).is_err());
    }

    #[test]
    fn argon2_blocks_wipe_before_free_and_release_on_error_and_unwind() {
        fn before_free(blocks: &[argon2::Block], budget: &WorkBudget) {
            assert_eq!(blocks.len(), 2);
            assert!(
                blocks
                    .iter()
                    .all(|block| block.as_ref().iter().all(|word| *word == 0))
            );
            assert_eq!(budget.live_bytes(), 2048);
        }
        for unwind in [false, true] {
            let budget = WorkBudget::new(2048);
            let result = std::panic::catch_unwind(|| -> crate::diagnostics::Result<()> {
                let mut blocks = WorkBlocks::zeroed(&budget, 2)?;
                blocks.before_free = Some(before_free);
                for block in blocks.as_mut_slice() {
                    block.as_mut().fill(0xDEAD_BEEF);
                }
                if unwind {
                    panic!("synthetic KDF unwind");
                }
                Err(super::resource_refused("synthetic KDF error"))
            });
            assert_eq!(result.is_err(), unwind);
            if !unwind {
                assert!(result.unwrap().is_err());
            }
            assert_eq!(budget.live_bytes(), 0);
        }
    }

    #[test]
    fn checked_reservations_are_shared_and_released() {
        let budget = WorkBudget::new(16);
        let lease = budget.reserve(16).unwrap();
        assert!(budget.clone().reserve(1).is_err());
        assert_eq!(budget.live_bytes(), 16);
        drop(lease);
        assert_eq!(budget.live_bytes(), 0);
        let bytes = WorkBytes::zeroed(&budget, 16).unwrap();
        assert_eq!(bytes.as_slice(), [0; 16]);
        drop(bytes);
        assert_eq!(budget.live_bytes(), 0);
        assert!(requested_bytes::<u64>(usize::MAX).is_err());
        assert_eq!(requested_bytes::<()>(usize::MAX).unwrap(), 0);
    }

    #[test]
    fn replacement_charges_old_and_full_new_backing() {
        for limit in [47, 48] {
            let budget = WorkBudget::new(limit);
            let mut values = WorkVec::<u8>::new(&budget);
            values.reserve(16).unwrap();
            for value in 0..16 {
                values.push(value).unwrap();
            }
            let result = values.reserve(16);
            assert_eq!(result.is_ok(), limit == 48);
            assert_eq!(values.as_slice(), (0..16).collect::<Vec<_>>());
            assert_eq!(budget.live_bytes(), if limit == 48 { 32 } else { 16 });
            drop(values);
            assert_eq!(budget.live_bytes(), 0);
        }
    }

    #[test]
    fn tight_budget_uses_exact_required_capacity_and_never_calls_clone() {
        #[derive(Copy)]
        struct Plain(u8);
        #[expect(
            clippy::non_canonical_clone_impl,
            reason = "Deliberate Clone panic verifies Copy transfer never invokes Clone"
        )]
        impl Clone for Plain {
            fn clone(&self) -> Self {
                panic!("Copy transfer must not call Clone")
            }
        }
        let budget = WorkBudget::new(33);
        let mut values = WorkVec::new(&budget);
        values.reserve(16).unwrap();
        for value in 0..16 {
            values.push(Plain(value)).unwrap();
        }
        values.reserve(1).unwrap();
        assert_eq!(budget.live_bytes(), 17);
        assert_eq!(values.as_slice()[15].0, 15);
    }

    #[test]
    fn zero_sized_items_and_unwind_release_reservations() {
        let budget = WorkBudget::new(0);
        let mut values = WorkVec::new(&budget);
        values.push(()).unwrap();
        assert_eq!(values.as_slice(), [()]);
        let budget = WorkBudget::new(16);
        let result = std::panic::catch_unwind(|| {
            let mut bytes = WorkBytes::zeroed(&budget, 16).unwrap();
            bytes.as_mut_slice().fill(0xAA);
            panic!("synthetic failure");
        });
        assert!(result.is_err());
        assert_eq!(budget.live_bytes(), 0);
    }
}
