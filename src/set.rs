use core::fmt;

/// A snapshot of a set that can be iterated over.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Set {
    state: u64,
}

impl Set {
    /// Construct a new snapshot with the specified `value`.
    #[inline]
    pub(crate) const fn new(state: u64) -> Self {
        Self { state }
    }

    /// Access the interior state of the set.
    #[inline]
    pub(crate) fn state(&self) -> u64 {
        self.state
    }

    /// Test if the set is empty.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.state == 0
    }

    /// Clear the given index.
    #[inline]
    pub fn clear(&mut self, index: usize) {
        self.state &= !(1u64 << index as u64);
    }

    /// Get the next index in the set.
    #[inline]
    pub fn next_index(&mut self) -> Option<u32> {
        if self.state == 0 {
            return None;
        }

        Some(self.state.trailing_zeros())
    }

    /// Construct an iterator over the snapshot.
    #[inline]
    pub(crate) fn iter(self) -> Iter {
        Iter { state: self.state }
    }
}

pub struct Iter {
    state: u64,
}

impl Iterator for Iter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state == 0 {
            return None;
        }

        let index = self.state.trailing_zeros();
        self.state &= !(1u64 << index);
        Some(index)
    }
}

impl fmt::Debug for Set {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}
