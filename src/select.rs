use std::future::Future;
use std::marker;
use std::mem::take;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::set::Snapshot;
use crate::static_waker::StaticWaker;

/// The type produced by the [select!] macro.
pub struct Select<T, F, O> {
    /// Mask of tasks which are active.
    mask: Snapshot,
    /// Indicates if a merge should be performed.
    merge: bool,
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
    pub(crate) fn new(waker: &'static StaticWaker, futures: F, poll: T) -> Self {
        let snapshot = waker.set.take();

        Self {
            mask: snapshot,
            merge: true,
            waker,
            snapshot,
            futures,
            poll,
            _marker: marker::PhantomData,
        }
    }

    /// Merge waker into current set.
    fn merge_from_shared(&mut self) {
        let snapshot = self.waker.set.take().mask(self.mask);
        self.snapshot.merge(snapshot);
    }
}

impl<T, F, O> Select<T, F, O>
where
    T: FnMut(&mut F, &mut Snapshot, usize) -> Poll<O>,
{
    /// Wait for one of the select branches to complete in a [Select] which is
    /// [Unpin].
    pub async fn next(&mut self) -> O
    where
        Self: Unpin,
    {
        Pin::new(self).next_pinned().await
    }

    /// Wait for one of the select branches to complete in a [Select] which is
    /// [Unpin] in a pinned context.
    pub fn next_pinned(self: Pin<&mut Self>) -> impl Future<Output = O> + '_ {
        Next { select: self }
    }

    /// Merge and return a boolean indicating if we should yield as pending or
    /// not.
    fn merge(&mut self, cx: &mut Context<'_>) -> bool {
        // NB: Oppurtunistically take things which have been marked without
        // paying the cost of registering a new waker.
        self.merge_from_shared();

        if !self.snapshot.is_empty() {
            return false;
        }

        // NB: perform a more costly registration of a waker and merge again.
        self.waker.parent.register(cx.waker());
        self.merge_from_shared();

        self.snapshot.is_empty()
    }

    /// Inner poll implementation.
    fn inner_poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<O> {
        unsafe {
            let this = self.get_unchecked_mut();

            if this.mask.is_empty() {
                if let Poll::Ready(output) = (this.poll)(
                    &mut this.futures,
                    &mut this.mask,
                    crate::__support::DISABLED,
                ) {
                    return Poll::Ready(output);
                }
            }

            if take(&mut this.merge) && this.merge(cx) {
                return Poll::Pending;
            }

            while let Some(index) = this.snapshot.unset_next() {
                let index = index as usize;

                if let Poll::Ready(output) = (this.poll)(&mut this.futures, &mut this.mask, index) {
                    if this.snapshot.is_empty() {
                        this.merge = true;
                    }

                    cx.waker().wake_by_ref();
                    return Poll::Ready(output);
                }
            }

            // We have drained the current snapshot, so we must perform another
            // merge. But we're also obligated to yield at this point to be good
            // citizens.
            this.merge = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

impl<T, F, O> Future for Select<T, F, O>
where
    T: FnMut(&mut F, &mut Snapshot, usize) -> Poll<O>,
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner_poll(cx)
    }
}

/// The future implementation of [Select::next].
struct Next<'a, T> {
    select: Pin<&'a mut T>,
}

impl<T, F, O> Future for Next<'_, Select<T, F, O>>
where
    T: FnMut(&mut F, &mut Snapshot, usize) -> Poll<O>,
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
