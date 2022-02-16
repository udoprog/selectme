use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::select::DISABLED;
use crate::set::Set;

/// The type of a static poller function. This is produced when
/// [inline!][crate::inline!] is used with the `static` option.
type StaticPoll<S, O> = fn(&mut Context<'_>, Pin<&mut S>, &mut Set, u32) -> Poll<O>;

/// The implementation used by the [select!][crate::select!] macro internally
/// and returned by the [inline!][crate::inline!] macro when the `static;`
/// option is enabled.
///
/// See the [select!][crate::select!] and [inline!][crate::inline!] macros for
/// documentation on syntax and use.
///
/// # Examples
///
/// ```
/// use std::future::Future;
/// use std::pin::Pin;
/// use std::task::{Context, Poll};
/// use std::time::Duration;
///
/// use pin_project::pin_project;
/// use selectme::StaticSelect;
/// use tokio::time::{self, Sleep};
///
/// #[pin_project]
/// struct MyFuture {
///     #[pin]
///     select: StaticSelect<(Sleep, Sleep), Option<u32>>,
/// }
///
/// impl Future for MyFuture {
///     type Output = Option<u32>;
///
///     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
///         let this = self.project();
///         this.select.poll_next(cx)
///     }
/// }
///
/// # #[tokio::main] pub async fn main() {
/// let s1 = time::sleep(Duration::from_millis(100));
/// let s2 = time::sleep(Duration::from_millis(200));
///
/// let my_future = MyFuture {
///     select: selectme::inline! {
///         static;
///
///         () = s1 => Some(1),
///         _ = s2 => Some(2),
///         else => None,
///     }
/// };
///
/// assert_eq!(my_future.await, Some(1));
/// # }
/// ```
pub struct StaticSelect<S, O> {
    snapshot: Set,
    state: S,
    poll: StaticPoll<S, O>,
}

impl<S, O> StaticSelect<S, O> {
    pub(crate) fn new(snapshot: Set, state: S, poll: StaticPoll<S, O>) -> Self {
        Self {
            snapshot,
            state,
            poll,
        }
    }
}

impl<S, O> StaticSelect<S, O> {
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
    ///         static;
    ///
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

    /// Poll for the next branch to resolve in this [StaticSelect].
    pub fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<O> {
        // SAFETY: StaticSelect is safely pinned.
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

impl<S, O> Future for StaticSelect<S, O> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.poll_next(cx)
    }
}

struct Next<'a, S, O> {
    this: Pin<&'a mut StaticSelect<S, O>>,
}

impl<S, O> Future for Next<'_, S, O> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe { Pin::get_unchecked_mut(self).this.as_mut().poll_next(cx) }
    }
}
