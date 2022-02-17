use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::bias::Bias;
use crate::set::{Number, Set};

/// Index which indicates that all branches have been disabled.
pub const DISABLED: u32 = u32::MAX;

/// This is the type produced by the [inline!][crate::inline!] macro unless the
/// `static;` option is enabled.
///
/// Note that second type parameter `T` in the [Select] type cannot be named. If
/// you want to embed a selection inside of another type have a look at
/// [StaticSelect][crate::StaticSelect].
///
/// See the [select!][crate::select!] and [inline!][crate::inline!] macros for
/// documentation on syntax and use.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use selectme::{Random, Select};
/// use tokio::time;
///
/// # #[tokio::main] pub async fn main() {
/// let s1 = time::sleep(Duration::from_millis(100));
/// let s2 = time::sleep(Duration::from_millis(200));
///
/// let mut inlined_var = false;
///
/// let output: Select<u8, _, Random, _> = selectme::inline! {
///     () = s1 => {
///         inlined_var = true;
///         Some(1)
///     }
///     _ = s2 => Some(2),
///     else => None,
/// };
///
/// tokio::pin!(output);
///
/// while let Some(output) = output.as_mut().next().await {
///     dbg!(output);
/// }
///
/// dbg!(inlined_var);
/// # }
/// ```

pub struct Select<Bits, S, B, T> {
    snapshot: Set<Bits>,
    state: S,
    bias: B,
    poll: T,
}

impl<Bits, S, B, T> Select<Bits, S, B, T> {
    pub(crate) fn new(snapshot: Set<Bits>, bias: B, state: S, poll: T) -> Self {
        Self {
            snapshot,
            state,
            bias,
            poll,
        }
    }
}

impl<Bits, S, B, T, O> Select<Bits, S, B, T>
where
    Bits: Number,
    B: Bias<Bits>,
    T: FnMut(&mut Context<'_>, Pin<&mut S>, &mut Set<Bits>, u32) -> Poll<O>,
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
            for index in this.bias.apply(this.snapshot) {
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

impl<Bits, S, B, T, O> Future for Select<Bits, S, B, T>
where
    Bits: Number,
    B: Bias<Bits>,
    T: FnMut(&mut Context<'_>, Pin<&mut S>, &mut Set<Bits>, u32) -> Poll<O>,
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.poll_next(cx)
    }
}

struct Next<'a, Bits, S, B, T> {
    this: Pin<&'a mut Select<Bits, S, B, T>>,
}

impl<Bits, S, B, T, O> Future for Next<'_, Bits, S, B, T>
where
    Bits: Number,
    B: Bias<Bits>,
    T: FnMut(&mut Context<'_>, Pin<&mut S>, &mut Set<Bits>, u32) -> Poll<O>,
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe { Pin::get_unchecked_mut(self).this.as_mut().poll_next(cx) }
    }
}
