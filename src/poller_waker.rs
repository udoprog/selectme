use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use crate::static_waker::StaticWaker;

/// A waker for a single poller.
pub struct PollerWaker {
    waker: &'static StaticWaker,
    index: usize,
}

impl PollerWaker {
    /// Construct a new static poller waker.
    pub const fn new(waker: &'static StaticWaker, index: usize) -> Self {
        Self { waker, index }
    }

    /// Mark the current set as woken.
    fn wake_by_ref(&self) {
        self.waker.set.set(self.index);
        self.waker.parent.wake_by_ref();
    }
}

// Note: Since all resources associated with the waker are statically allocated,
// there's no need to free anything on calls to `wake` which otherwise takes
// ownership of the waker. There is also nothing to drop and no work that needs
// to happen when cloning.
static VTABLE: &RawWakerVTable = &RawWakerVTable::new(
    |this| RawWaker::new(this, VTABLE),
    |this| unsafe { (*(this as *const PollerWaker)).wake_by_ref() },
    |this| unsafe { (*(this as *const PollerWaker)).wake_by_ref() },
    |_| {},
);

/// Poll the given task using the given waker.
#[doc(hidden)]
pub fn poll_by_ref<T, O>(waker: &'static PollerWaker, f: T) -> Poll<O>
where
    T: FnOnce(&mut Context<'_>) -> Poll<O>,
{
    unsafe {
        let waker = RawWaker::new(waker as *const _ as *const (), VTABLE);
        let waker = Waker::from_raw(waker);
        let mut cx = Context::from_waker(&waker);
        f(&mut cx)
    }
}
