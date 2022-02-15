use core::sync::atomic::{AtomicU64, Ordering};
use std::ops::Deref;

/// An atomic set indicating wakeup interest.
#[repr(transparent)]
pub struct Set(AtomicU64);

impl Set {
    /// Construct a new empty set.
    pub const fn empty() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Take the current set and replace with an empty set returning the old set.
    pub(crate) fn take(&self) -> Snapshot {
        Snapshot(self.0.swap(0, Ordering::SeqCst))
    }

    /// Set the given bit in the set.
    pub(crate) fn set(&self, index: usize) {
        assert!(index < u64::BITS as usize);
        let bit = 1u64 << index as u64;
        self.0.fetch_or(bit, Ordering::SeqCst);
    }

    /// Merge the global set with a snapshot.
    pub(crate) fn merge(&self, snapshot: Snapshot) {
        self.0.fetch_or(snapshot.0, Ordering::SeqCst);
    }
}

/// A snapshot of a set that can be iterated over.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Snapshot(u64);

impl Snapshot {
    /// Construct a new snapshot with the specified `value`.
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Construct an empty snapshot.
    pub(crate) const fn empty() -> Self {
        Self(0)
    }

    /// Take the current snapshot.
    pub(crate) fn take(&mut self) -> Self {
        Self(std::mem::take(&mut self.0))
    }

    /// Test if the snapshot is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Apply the given mask to the current snapshot.
    #[inline]
    pub(crate) fn mask(self, mask: Self) -> Self {
        Self(self.0 & mask.0)
    }

    /// Clear the given index.
    #[inline]
    pub fn clear(&mut self, index: usize) {
        self.0 &= !(1u64 << index as u64);
    }

    /// Merge this snapshot with another snapshot.
    #[inline]
    pub(crate) fn merge(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Unset the next index in the set and return it.
    #[inline]
    pub fn unset_next(&mut self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            let index = self.0.trailing_zeros() as u64;
            self.0 &= !(1u64 << index);
            Some(index as usize)
        }
    }
}

impl Deref for Snapshot {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
