use std::sync::atomic::{AtomicU64, Ordering};

/// An atomic set indicating wakeup interest.
#[repr(transparent)]
pub struct Set(AtomicU64);

impl Set {
    /// Construct a new empty set.
    pub const fn empty() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Fill the set marking all branches that should be initially polled.
    pub(crate) fn reset(&self, snapshot: u64) {
        self.0.store(snapshot, Ordering::SeqCst);
    }

    /// Take the current set and replace with an empty set returning the old set.
    pub(crate) fn take(&self) -> Snapshot {
        Snapshot(self.0.swap(0, Ordering::SeqCst))
    }

    /// Merge the current set with the given snapshot.
    pub(crate) fn merge(&self, snapshot: Snapshot) {
        self.0.fetch_or(snapshot.0, Ordering::SeqCst);
    }

    /// Set the given bit in the set.
    pub(crate) fn set(&self, index: usize) {
        assert!(index < 64);
        let bit = 1u64 << index as u64;
        self.0.fetch_or(bit, Ordering::SeqCst);
    }
}

/// A snapshot of a set that can be iterated over.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub(crate) struct Snapshot(u64);

impl Snapshot {
    /// Unset the given bit and return if it was set or not.
    pub(crate) fn unset(&mut self, n: usize) -> bool {
        let bit = 1u64 << n as u64;

        if self.0 & bit == 0 {
            false
        } else {
            self.0 &= !bit;
            true
        }
    }

    /// Merge this snapshot with another snapshot.
    pub(crate) fn merge(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl Iterator for Snapshot {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            return None;
        }

        let index = self.0.trailing_zeros();
        self.0 &= !(1u64 << index as u64);
        Some(index)
    }
}
