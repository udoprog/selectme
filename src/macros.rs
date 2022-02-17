/// Waits on multiple concurrent branches, returning when the **first** branch
/// completes, cancelling the remaining branches.
///
/// The `select!` macro must be used inside of async functions, closures, and
/// blocks.
///
/// The `select!` macro accepts one or more branches with the following pattern:
///
/// ```text
/// <pattern> = <async expression> (if <precondition>)? => <handler>,
/// ```
///
/// Additionally, the `select!` macro may include a single, optional `else`
/// branch, which evaluates if none of the other branches match their patterns:
///
/// ```text
/// else => <expression>
/// ```
///
/// The macro aggregates all `<async expression>` expressions and runs them
/// concurrently on the **current** task. Once the **first** expression
/// completes with a value that matches its `<pattern>`, the `select!` macro
/// returns the result of evaluating the completed branch's `<handler>`
/// expression.
///
/// Additionally, each branch may include an optional `if` precondition. If the
/// precondition returns `false`, then the branch is disabled. The provided
/// `<async expression>` is still evaluated but the resulting future is never
/// polled. This capability is useful when using `select!` within a loop.
///
/// The complete lifecycle of a `select!` expression is as follows:
///
/// 1. Evaluate all provided `<precondition>` expressions. If the precondition
///    returns `false`, disable the branch for the remainder of the current call
///    to `select!`. Re-entering `select!` due to a loop clears the "disabled"
///    state.
/// 2. Aggregate the `<async expression>`s from each branch, excluding the
///    disabled ones. If the branch is disabled, `<async expression>` is not
///    evaluated.
/// 3. Concurrently await on the results for all remaining `<async
///    expression>`s.
/// 4. Once an `<async expression>` returns a value, attempt to apply the value
///    to the provided `<pattern>` if the pattern matches, evaluate `<handler>`
///    and return. If the pattern **does not** match, disable the current branch
///    and for the remainder of the current call to `select!`. Continue from
///    step 3.
/// 5. If **all** branches are disabled, evaluate the `else` expression. If no
///    else branch is provided, panic.
///
/// # Runtime characteristics
///
/// By running all async expressions on the current task, the expressions are
/// able to run **concurrently** but not in **parallel**. This means all
/// expressions are run on the same thread and if one branch blocks the thread,
/// all other expressions will be unable to continue. If parallelism is
/// required, spawn each async expression using [`tokio::spawn`] and pass the
/// join handle to `select!`.
///
/// [`tokio::spawn`]: https://docs.rs/tokio/latest/tokio/fn.spawn.html
///
/// # Cancellation safety
///
/// When using `select!` in a loop to receive messages from multiple sources,
/// you should make sure that the receive call is cancellation safe to avoid
/// losing messages. This section goes through various common methods and
/// describes whether they are cancel safe.  The lists in this section are not
/// exhaustive.
///
/// # Examples
///
/// Basic select with two branches.
///
/// ```
/// async fn do_stuff_async() {
///     // async work
/// }
///
/// async fn more_async_work() {
///     // more here
/// }
///
/// # #[tokio::main] async fn main() {
/// selectme::select! {
///     _ = do_stuff_async() => {
///         println!("do_stuff_async() completed first")
///     }
///     _ = more_async_work() => {
///         println!("more_async_work() completed first")
///     }
/// };
/// # }
/// ```
///
/// Basic stream selecting.
///
/// ```
/// use tokio_stream::{self as stream, StreamExt};
///
/// # #[tokio::main] async fn main() {
/// let mut stream1 = stream::iter(vec![1, 2, 3]);
/// let mut stream2 = stream::iter(vec![4, 5, 6]);
///
/// let next = selectme::select! {
///     Some(v) = stream1.next() => v,
///     Some(v) = stream2.next() => v,
/// };
///
/// assert!(next == 1 || next == 4);
/// # }
/// ```
///
/// Collect the contents of two streams. In this example, we rely on pattern
/// matching and the fact that `stream::iter` is "fused", i.e. once the stream
/// is complete, all calls to `next()` return `None`.
///
/// ```
/// use tokio_stream::{self as stream, StreamExt};
///
/// # #[tokio::main] async fn main() {
/// let mut stream1 = stream::iter(vec![1, 2, 3]);
/// let mut stream2 = stream::iter(vec![4, 5, 6]);
///
/// let mut values = vec![];
///
/// loop {
///     selectme::select! {
///         Some(v) = stream1.next() => values.push(v),
///         Some(v) = stream2.next() => values.push(v),
///         else => break,
///     }
/// }
///
/// values.sort();
/// assert_eq!(&[1, 2, 3, 4, 5, 6], &values[..]);
/// # }
/// ```
///
/// Using the same future in multiple `select!` expressions can be done by passing
/// a reference to the future. Doing so requires the future to be [`Unpin`]. A
/// future can be made [`Unpin`] by either using [`Box::pin`] or stack pinning.
///
/// [`Unpin`]: std::marker::Unpin
/// [`Box::pin`]: std::boxed::Box::pin
///
/// Here, a stream is consumed for at most 1 second.
///
/// ```
/// use tokio_stream::{self as stream, StreamExt};
/// use tokio::time::{self, Duration};
///
/// # #[tokio::main] async fn main() {
/// let mut stream = stream::iter(vec![1, 2, 3]);
/// let sleep = time::sleep(Duration::from_secs(1));
/// tokio::pin!(sleep);
///
/// loop {
///     selectme::select! {
///         maybe_v = stream.next() => {
///             if let Some(v) = maybe_v {
///                 println!("got = {}", v);
///             } else {
///                 break;
///             }
///         }
///         _ = &mut sleep => {
///             println!("timeout");
///             break;
///         }
///     }
/// }
/// # }
/// ```
///
/// Joining two values using `select!`.
///
/// ```
/// use tokio::sync::oneshot;
///
/// # #[tokio::main] async fn main() {
/// let (tx1, mut rx1) = oneshot::channel();
/// let (tx2, mut rx2) = oneshot::channel();
///
/// tokio::spawn(async move {
///     tx1.send("first").unwrap();
/// });
///
/// tokio::spawn(async move {
///     tx2.send("second").unwrap();
/// });
///
/// let mut a = None;
/// let mut b = None;
///
/// while a.is_none() || b.is_none() {
///     selectme::select! {
///         v1 = (&mut rx1) if a.is_none() => a = Some(v1.unwrap()),
///         v2 = (&mut rx2) if b.is_none() => b = Some(v2.unwrap()),
///     }
/// }
///
/// let res = (a.unwrap(), b.unwrap());
///
/// assert_eq!(res.0, "first");
/// assert_eq!(res.1, "second");
/// # }
/// ```
///
/// Showing how `select!` has a deterministic order by default. This is known as
/// the [`biased` option in Tokio].
///
/// ```
/// # #[tokio::main] async fn main() {
/// let mut count = 0u8;
///
/// loop {
///     selectme::select! {
///         biased;
///
///         _ = async {} if count < 1 => {
///             count += 1;
///             assert_eq!(count, 1);
///         }
///         _ = async {} if count < 2 => {
///             count += 1;
///             assert_eq!(count, 2);
///         }
///         _ = async {} if count < 3 => {
///             count += 1;
///             assert_eq!(count, 3);
///         }
///         _ = async {} if count < 4 => {
///             count += 1;
///             assert_eq!(count, 4);
///         }
///
///         else => {
///             break;
///         }
///     };
/// }
/// # }
/// ```
///
/// ## Avoid racy `if` preconditions
///
/// Given that `if` preconditions are used to disable `select!` branches, some
/// caution must be used to avoid missing values.
///
/// For example, here is **incorrect** usage of `sleep` with `if`. The objective
/// is to repeatedly run an asynchronous task for up to 50 milliseconds.
/// However, there is a potential for the `sleep` completion to be missed.
///
/// ```no_run,should_panic
/// use tokio::time::{self, Duration};
///
/// async fn some_async_work() {
///     // do work
/// }
///
/// # #[tokio::main] async fn main() {
/// let sleep = time::sleep(Duration::from_millis(50));
/// tokio::pin!(sleep);
///
/// while !sleep.is_elapsed() {
///     selectme::select! {
///         _ = &mut sleep if !sleep.is_elapsed() => {
///             println!("operation timed out");
///         }
///         _ = some_async_work() => {
///             println!("operation completed");
///         }
///     }
/// }
///
/// panic!("This example shows how not to do it!");
/// # }
/// ```
///
/// In the above example, `sleep.is_elapsed()` may return `true` even if
/// `sleep.poll()` never returned `Ready`. This opens up a potential race
/// condition where `sleep` expires between the `while !sleep.is_elapsed()`
/// check and the call to `select!` resulting in the `some_async_work()` call to
/// run uninterrupted despite the sleep having elapsed.
///
/// One way to write the above example without the race would be:
///
/// ```
/// use tokio::time::{self, Duration};
///
/// async fn some_async_work() {
/// # time::sleep(Duration::from_millis(10)).await;
///     // do work
/// }
///
/// # #[tokio::main] async fn main() {
/// let sleep = time::sleep(Duration::from_millis(50));
/// tokio::pin!(sleep);
///
/// loop {
///     selectme::select! {
///         _ = &mut sleep => {
///             println!("operation timed out");
///             break;
///         }
///         _ = some_async_work() => {
///             println!("operation completed");
///         }
///     }
/// }
/// # }
/// ```
///
/// [`biased` option in Tokio]: https://docs.rs/tokio/latest/tokio/macro.select.html#fairness
#[macro_export]
macro_rules! select {
    ($($tt:tt)*) => {{
        $crate::__support::select!($crate, $($tt)*)
    }};
}

/// The `inline!` macro provides an *inlined* variant of the [select!] macro.
///
/// Instead of awaiting directly it evaluates to an instance of the
/// [Select][crate::Select] or [StaticSelect][crate::StaticSelect] allowing for
/// more efficient multiplexing and complex control flow.
///
/// When combined with the `static;` option it performs the least amount of
/// magic possible to multiplex multiple asynchronous operations making it
/// suitable for efficient and custom abstractions.
///
/// <br>
///
/// # Branches and state
///
/// The `inline!` macro differs from [select!] in that it allows for retaining
/// information on which branches have been disabled due to the future
/// completing without having to rely on a [branch condition].
///
/// In contrast to [select!] it *does not* allow for performing additional
/// asynchronous operations *in the branches*, nor affecting the control flow of
/// the surrounding context like through `break`.
///
/// The following would not compile, since the `break` below is not evaluated in
/// the context of the loop:
///
/// ```compile_fail
/// # #[tokio::main]
/// # pub(crate) async fn main() {
/// let s1 = std::future::ready(());
///
/// loop {
///     selectme::inline! {
///         () = s1 => 1,
///         else => break,
///     }.await;
/// }
/// # }
/// ```
///
/// While this seems like it would make it less useful, instead it means that it
/// can efficiently perform inline common inline operations like timeouts.
///
/// ```
/// use std::time::Duration;
/// use tokio::time;
///
/// async fn async_operation() -> u32 {
///     // work here
/// # 42
/// }
///
/// # #[tokio::main]
/// # pub(crate) async fn main() {
/// let output = selectme::inline! {
///     output = async_operation() => Some(output),
///     () = time::sleep(Duration::from_secs(5)) => None,
/// }.await;
///
/// match output {
///     Some(output) => {
///         assert_eq!(output, 42);
///     }
///     None => {
///         panic!("operation timed out!")
///     }
/// }
/// # }
/// ```
///
/// <br>
///
/// # Static selects
///
/// The `inline!` macro can also make use of the `static;` option, which allows
/// the select that is generated to be named. This can be useful if you want to
/// incorporate selection-like behaviour into another future.
///
/// Note that this option is not supported in [select!].
///
/// ```
/// use std::future::Future;
/// use std::pin::Pin;
/// use std::task::{Context, Poll};
/// use std::time::Duration;
///
/// use pin_project::pin_project;
/// use selectme::{Random, StaticSelect};
/// use tokio::time::{self, Sleep};
///
/// #[pin_project]
/// struct MyFuture {
///     #[pin]
///     select: StaticSelect<u8, (Sleep, Sleep), Random, Option<u32>>,
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
/// # #[tokio::main] pub(crate) async fn main() {
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
///
/// <br>
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use tokio::time;
///
/// # #[tokio::main] pub(crate) async fn main() {
/// let s1 = time::sleep(Duration::from_millis(100));
/// let s2 = time::sleep(Duration::from_millis(200));
///
/// let output = selectme::inline! {
///     () = s1 => Some(1),
///     _ = s2 => Some(2),
///     else => None,
/// };
///
/// tokio::pin!(output);
///
/// let mut values = Vec::new();
///
/// while let Some(output) = output.as_mut().next().await {
///     values.push(output);
/// }
///
/// assert_eq!(values, &[1, 2]);
/// # }
/// ```
#[macro_export]
macro_rules! inline {
    ($($tt:tt)*) => {{
        $crate::__support::inline!($crate, $($tt)*)
    }};
}
