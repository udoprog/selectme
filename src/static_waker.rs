use crate::atomic_waker::AtomicWaker;
use crate::set::Set;

/// A static waker associated with a select loop.
pub struct StaticWaker {
    /// The current waker.
    pub(crate) parent: AtomicWaker,
    /// The bitset which is passed to tasks.
    pub(crate) set: Set,
}

impl StaticWaker {
    /// Construct a new static waker.
    pub const fn new() -> Self {
        Self {
            parent: AtomicWaker::new(),
            set: Set::empty(),
        }
    }

    /// Reset the current static waker.
    pub fn reset(&self, snapshot: u64) {
        self.set.reset(snapshot);
    }
}
