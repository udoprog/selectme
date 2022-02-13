//! A fast and fair select! implementation for asynchronous programming.

mod set;

mod atomic_waker;

mod poller_waker;
pub use self::poller_waker::{poll_by_ref, PollerWaker};

mod static_waker;
pub use self::static_waker::StaticWaker;

#[doc(inline)]
pub use selectme_macros::{inline, select};

#[doc(hidden)]
pub mod macros {
    pub use std::future::Future;
    pub use std::pin::Pin;
    pub use std::task::Poll;
}

use std::future::Future;
use std::marker;
use std::pin::Pin;
use std::task::{Context, Poll};

use self::set::Snapshot;

/// Indicator index used when all futures have been disabled.
#[doc(hidden)]
pub const DISABLED: usize = usize::MAX;

pub struct Select<T, F, O> {
    /// Mask of tasks which are active.
    mask: Snapshot,
    /// Reference to the static waker associated with the poller.
    waker: &'static StaticWaker,
    /// A snapshot of the bitset for the current things that needs polling.
    snapshot: Snapshot,
    /// Captured futures.
    futures: F,
    /// Polling function.
    poll: T,
    /// Marker indicating the output type.
    _marker: marker::PhantomData<O>,
}

impl<T, F, O> Select<T, F, O> {
    /// Merge waker into current set.
    fn merge_from_shared(&mut self) {
        let snapshot = self.waker.set.take();
        self.snapshot.merge(snapshot);
        self.snapshot.retain(self.mask);
    }
}

impl<T, F, O> Select<T, F, O>
where
    T: FnMut(&mut Context<'_>, &mut F, usize) -> Poll<O>,
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
            this.merge_from_shared();

            this.waker.parent.register(cx.waker());

            if this.mask.is_empty() {
                if let Poll::Ready(output) = (this.poll)(cx, &mut this.futures, DISABLED) {
                    return Poll::Ready(output);
                }
            }

            for index in this.snapshot.by_ref() {
                let index = index as usize;

                if let Poll::Ready(output) = (this.poll)(cx, &mut this.futures, index) {
                    this.mask.clear(index);
                    return Poll::Ready(output);
                }
            }

            Poll::Pending
        }
    }
}

impl<T, F, O> Future for Select<T, F, O>
where
    T: FnMut(&mut Context<'_>, &mut F, usize) -> Poll<O>,
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
    T: FnMut(&mut Context<'_>, &mut F, usize) -> Poll<O>,
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

impl<T, F, O> Drop for Select<T, F, O> {
    fn drop(&mut self) {
        self.waker.set.merge(self.snapshot);
    }
}

/// Construct a new polling context from a custom function.
#[doc(hidden)]
pub fn select<T, F, O>(waker: &'static StaticWaker, futures: F, poll: T) -> Select<T, F, O>
where
    T: FnMut(&mut Context<'_>, &mut F, usize) -> Poll<O>,
{
    let snapshot = waker.set.take();

    Select {
        mask: snapshot,
        waker,
        snapshot,
        futures,
        poll,
        _marker: marker::PhantomData,
    }
}
