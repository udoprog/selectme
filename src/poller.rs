use core::future::Future;
use core::mem::take;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::set::Snapshot;
use crate::static_waker::StaticWaker;

/// Indicator index used when all futures have been disabled.
pub const DISABLED: usize = usize::MAX;

/// Helper type used to implement the [select!][crate::select] macro.
///
/// This keeps track of tasks that should be polled and interactions with the
/// [StaticWaker].
pub struct Poller {
    /// Mask of tasks which are active.
    mask: Snapshot,
    /// Indicates if a merge should be performed.
    merge: bool,
    /// Reference to the static waker associated with the poller.
    waker: &'static StaticWaker,
    /// A snapshot of the bitset for the current things that needs polling.
    snapshot: Snapshot,
}

impl Poller {
    pub(crate) fn new(waker: &'static StaticWaker, snapshot: Snapshot) -> Self {
        Self {
            mask: snapshot,
            merge: true,
            waker,
            snapshot,
        }
    }

    /// Clear the given index.
    pub fn clear(&mut self, index: usize) {
        self.mask.clear(index);
    }

    /// Wait for one of the select branches to complete in a [Select] which is
    /// [Unpin].
    pub async fn next(&mut self) -> usize
    where
        Self: Unpin,
    {
        Next { select: self }.await
    }

    /// Merge waker into current set.
    fn merge_from_shared(&mut self) {
        let snapshot = self.waker.set.take().mask(self.mask);
        self.snapshot.merge(snapshot);
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
    fn inner_poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<usize> {
        unsafe {
            let this = self.get_unchecked_mut();

            if this.mask.is_empty() {
                return Poll::Ready(DISABLED);
            }

            if take(&mut this.merge) && this.merge(cx) {
                return Poll::Pending;
            }

            while let Some(index) = this.snapshot.unset_next() {
                let index = index as usize;
                return Poll::Ready(index);
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

impl Future for Poller {
    type Output = usize;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner_poll(cx)
    }
}

/// The future implementation of [Select::next].
struct Next<'a> {
    select: &'a mut Poller,
}

impl Future for Next<'_> {
    type Output = usize;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: Type is safely Unpin since the only way to access it is
        // through [Select::next] which requires `Unpin`.
        unsafe { Pin::map_unchecked_mut(self, |f| f.select).inner_poll(cx) }
    }
}
