//! A fast and fair select! implementation for asynchronous programming.

mod set;

mod atomic_waker;

#[doc(inline)]
pub use selectme_macros::select;

#[doc(hidden)]
pub mod macros {
    pub use std::future::Future;
    pub use std::pin::Pin;
    pub use std::task::Poll;
}

use std::future::Future;
use std::marker;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use self::atomic_waker::AtomicWaker;
use self::set::{Set, Snapshot};

pub struct Select<T, F, O> {
    poller: Poller<O>,
    futures: F,
    poll: T,
}

impl<T, F, O> Select<T, F, O>
where
    T: FnMut(&mut Context<'_>, Pin<&mut F>, &mut Poller<O>) -> Poll<O>,
{
    /// Wait for one of the select branches to complete in a [Select] which is
    /// [Unpin].
    pub async fn next(&mut self) -> O
    where
        Self: Unpin,
    {
        Pin::new(self).next_pinned().await
    }

    /// Wait for one of the select branches to complete in a pinned select.
    pub fn next_pinned(self: Pin<&mut Self>) -> impl Future<Output = O> + '_ {
        Next { select: self }
    }

    /// Inner poll implementation.
    fn inner_poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<O> {
        unsafe {
            let this = self.get_unchecked_mut();
            this.poller.merge_from_shared();
            let futures = Pin::new_unchecked(&mut this.futures);
            (this.poll)(cx, futures, &mut this.poller)
        }
    }
}

impl<T, F, O> Future for Select<T, F, O>
where
    T: FnMut(&mut Context<'_>, Pin<&mut F>, &mut Poller<O>) -> Poll<O>,
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner_poll(cx)
    }
}

/// The future implementation of [Select::next].
pub struct Next<'a, T> {
    select: Pin<&'a mut T>,
}

impl<T, F, O> Future for Next<'_, Select<T, F, O>>
where
    T: FnMut(&mut Context<'_>, Pin<&mut F>, &mut Poller<O>) -> Poll<O>,
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: Type is safely Unpin since the only way to access it is
        // through [Select::next] which requires `Unpin`.
        unsafe {
            let this = self.get_unchecked_mut();
            this.select.as_mut().inner_poll(cx)
        }
    }
}

pub struct Poller<O> {
    /// Reference to the static waker associated with the poller.
    waker: &'static StaticWaker,
    /// A snapshot of the bitset for the current things that needs polling.
    snapshot: Snapshot,
    /// Marker indicating the output type.
    _marker: marker::PhantomData<O>,
}

impl<O> Poller<O> {
    /// Construct a new empty poller.
    fn new(waker: &'static StaticWaker) -> Self {
        let snapshot = waker.set.take();

        Self {
            waker,
            snapshot,
            _marker: marker::PhantomData,
        }
    }

    /// Merge waker into current set.
    fn merge_from_shared(&mut self) {
        let snapshot = self.waker.set.take();
        self.snapshot.merge(snapshot);
    }

    /// Iterate over the bits that are set.
    pub fn next(&mut self) -> Option<u32> {
        self.snapshot.next()
    }

    /// Poll the current poller.
    pub fn poll<T, U>(
        &mut self,
        cx: &mut Context<'_>,
        waker: &'static PollerWaker,
        poll: T,
    ) -> Poll<U>
    where
        T: FnOnce(&mut Context<'_>) -> Poll<U>,
    {
        self.waker.parent.register(cx.waker());

        let output = {
            let waker = unsafe {
                let waker = RawWaker::new(waker as *const _ as *const (), POLLER_WAKER_VTABLE);
                Waker::from_raw(waker)
            };

            let mut cx = Context::from_waker(&waker);
            poll(&mut cx)
        };

        output
    }
}

impl<O> Drop for Poller<O> {
    fn drop(&mut self) {
        self.waker.set.merge(self.snapshot);
    }
}

/// Construct a new polling context from a custom function.
pub fn select<T, F, O>(waker: &'static StaticWaker, futures: F, poll: T) -> Select<T, F, O>
where
    T: FnMut(&mut Context<'_>, Pin<&mut F>, &mut Poller<O>) -> Poll<O>,
{
    Select {
        poller: Poller::new(waker),
        futures,
        poll,
    }
}

/// A static waker associated with a select loop.
pub struct StaticWaker {
    /// The current waker.
    parent: AtomicWaker,
    /// The bitset which is passed to tasks.
    set: Set,
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
    fn wake(&self) {
        self.waker.set.set(self.index);
        self.waker.parent.wake();
    }
}

static POLLER_WAKER_VTABLE: &RawWakerVTable = &RawWakerVTable::new(
    |this| RawWaker::new(this, POLLER_WAKER_VTABLE),
    |this| unsafe { (*(this as *const PollerWaker)).wake() },
    |this| unsafe { (*(this as *const PollerWaker)).wake() },
    |_| {},
);
