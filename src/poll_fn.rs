use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::set::Set;

/// Index which indicates that all branches have been disabled.
pub const DISABLED: u32 = u32::MAX;

/// The implementation returned from [poll_fn].
pub(crate) struct PollFn<T> {
    snapshot: Set,
    poll: T,
}

impl<T> PollFn<T> {
    pub(crate) fn new(snapshot: Set, poll: T) -> Self {
        Self { snapshot, poll }
    }
}

impl<T, O> Future for PollFn<T>
where
    T: FnMut(&mut Context<'_>, &mut Set, u32) -> Poll<O>,
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            let this = Pin::get_unchecked_mut(self);

            // All branches are disabled.
            for index in this.snapshot.iter() {
                if let Poll::Ready(output) = (this.poll)(cx, &mut this.snapshot, index) {
                    return Poll::Ready(output);
                }
            }

            // We've polled through all branches (and they have been disabled
            // through pattern matching).
            if this.snapshot.is_empty() {
                return (this.poll)(cx, &mut this.snapshot, DISABLED);
            }

            Poll::Pending
        }
    }
}
