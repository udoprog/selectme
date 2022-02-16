use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::set::Set;

/// Index which indicates that all branches have been disabled.
pub const DISABLED: u32 = u32::MAX;

/// The implementation used by the [select!][crate::select!] macro internally
/// and returned by the [inline!][crate::inline!] macro.
pub struct Select<S, T> {
    snapshot: Set,
    state: S,
    poll: T,
}

impl<S, T> Select<S, T> {
    pub(crate) fn new(snapshot: Set, state: S, poll: T) -> Self {
        Self {
            snapshot,
            state,
            poll,
        }
    }
}

impl<S, T, O> Select<S, T>
where
    T: FnMut(&mut Context<'_>, Pin<&mut S>, &mut Set, u32) -> Poll<O>,
{
    /// Get the next element from this select when pinned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// use tokio::time;
    ///
    /// #[tokio::main]
    /// pub async fn main() {
    ///     let s1 = time::sleep(Duration::from_millis(100));
    ///     let s2 = time::sleep(Duration::from_millis(200));
    ///
    ///     let output = selectme::inline! {
    ///         () = s1 => Some(1),
    ///         _ = s2 => Some(2),
    ///         else => None,
    ///     };
    ///
    ///     tokio::pin!(output);
    ///
    ///     let mut values = Vec::new();
    ///
    ///     while let Some(output) = output.as_mut().next().await {
    ///         values.push(output);
    ///     }
    ///
    ///     assert_eq!(values, &[1, 2]);
    /// }
    /// ```
    pub async fn next(self: Pin<&mut Self>) -> O {
        Next { this: self }.await
    }

    /// Poll for the next branch to resolve in this [Select].
    pub fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<O> {
        // SAFETY: Select is safely pinned.
        unsafe {
            let this = Pin::get_unchecked_mut(self);
            let mut state = Pin::new_unchecked(&mut this.state);

            // All branches are disabled.
            for index in this.snapshot.iter() {
                if let Poll::Ready(output) =
                    (this.poll)(cx, state.as_mut(), &mut this.snapshot, index)
                {
                    return Poll::Ready(output);
                }
            }

            // We've polled through all branches (and they have been disabled
            // through pattern matching).
            if this.snapshot.is_empty() {
                return (this.poll)(cx, state.as_mut(), &mut this.snapshot, DISABLED);
            }

            Poll::Pending
        }
    }
}

impl<S, T, O> Future for Select<S, T>
where
    T: FnMut(&mut Context<'_>, Pin<&mut S>, &mut Set, u32) -> Poll<O>,
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.poll_next(cx)
    }
}

struct Next<'a, S, T> {
    this: Pin<&'a mut Select<S, T>>,
}

impl<S, T, O> Future for Next<'_, S, T>
where
    T: FnMut(&mut Context<'_>, Pin<&mut S>, &mut Set, u32) -> Poll<O>,
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe { Pin::get_unchecked_mut(self).this.as_mut().poll_next(cx) }
    }
}
