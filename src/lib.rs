//! [<img alt="github" src="https://img.shields.io/badge/github-udoprog/selectme?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/selectme)
//! [<img alt="crates.io" src="https://img.shields.io/crates/v/selectme.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/selectme)
//! [<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-selectme?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/selectme)
//! [<img alt="build status" src="https://img.shields.io/github/workflow/status/udoprog/selectme/CI/main?style=for-the-badge" height="20">](https://github.com/udoprog/selectme/actions?query=branch%3Amain)
//!
//! A fast and fair select! implementation for asynchronous programming.
//!
//! See the [select!] or [inline!] macros for documentation.
//!
//! <br>
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! selectme = "0.5.0"
//! ```
//!
//! <br>
//!
//! ## Examples
//!
//! The following is a simple example showcasing two branches being polled
//! concurrently. For more documentation see [select!].
//!
//! ```
//! async fn do_stuff_async() {
//!     // work here
//! }
//!
//! async fn more_async_work() {
//!     // work here
//! }
//!
//! # #[tokio::main] async fn main() {
//! selectme::select! {
//!     _ = do_stuff_async() => {
//!         println!("do_stuff_async() completed first")
//!     }
//!     _ = more_async_work() => {
//!         println!("more_async_work() completed first")
//!     }
//! };
//! # }
//! ```
//!
//! <br>
//!
//! # The `inline!` macro
//!
//! The [inline!] macro provides an *inlined* variant of the [select!] macro.
//!
//! Instead of awaiting directly it evaluates to an instance of the [Select] or
//! [StaticSelect] allowing for more efficient multiplexing and complex control
//! flow.
//!
//! When combined with the `static;` option it performs the least amount of
//! magic possible to multiplex multiple asynchronous operations making it
//! suitable for efficient and custom abstractions.
//!
//! ```
//! use std::time::Duration;
//! use tokio::time;
//!
//! async fn async_operation() -> u32 {
//!     // work here
//! # 42
//! }
//!
//! # #[tokio::main]
//! # pub async fn main() {
//! let output = selectme::inline! {
//!     output = async_operation() => Some(output),
//!     () = time::sleep(Duration::from_secs(5)) => None,
//! }.await;
//!
//! match output {
//!     Some(output) => {
//!         assert_eq!(output, 42);
//!     }
//!     None => {
//!         panic!("operation timed out!")
//!     }
//! }
//! # }
//! ```
//!
//! The more interesting trick is producing a [StaticSelect] through the
//! `static;` option which can be properly named and used inside of another
//! future.
//!
//! ```
//! use std::future::Future;
//! use std::pin::Pin;
//! use std::task::{Context, Poll};
//! use std::time::Duration;
//!
//! use pin_project::pin_project;
//! use selectme::{Random, StaticSelect};
//! use tokio::time::{self, Sleep};
//!
//! #[pin_project]
//! struct MyFuture {
//!     #[pin]
//!     select: StaticSelect<u8, (Sleep, Sleep), Random, Option<u32>>,
//! }
//!
//! impl Future for MyFuture {
//!     type Output = Option<u32>;
//!
//!     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//!         let this = self.project();
//!         this.select.poll_next(cx)
//!     }
//! }
//!
//! # #[tokio::main] pub async fn main() {
//! let s1 = time::sleep(Duration::from_millis(100));
//! let s2 = time::sleep(Duration::from_millis(200));
//!
//! let my_future = MyFuture {
//!     select: selectme::inline! {
//!         static;
//!
//!         () = s1 => Some(1),
//!         _ = s2 => Some(2),
//!         else => None,
//!     }
//! };
//!
//! assert_eq!(my_future.await, Some(1));
//! # }
//! ```
//!
//! [select!]: https://docs.rs/selectme/latest/selectme/macro.select.html
//! [inline!]: https://docs.rs/selectme/latest/selectme/macro.inline.html
//! [Select]: https://docs.rs/selectme/latest/selectme/struct.Select.html
//! [StaticSelect]: https://docs.rs/selectme/latest/selectme/struct.StaticSelect.html

// This project contains code and documentation licensed under the MIT license
// from the futures-rs project.
//
// See: https://github.com/rust-lang/futures-rs/blob/c3d3e08/LICENSE-MIT

// This project contains code and documentation licensed under the MIT license
// from the Tokio project.
//
// See: https://github.com/tokio-rs/tokio/blob/986b88b/LICENSE

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![cfg_attr(not(feature = "std"), no_std)]

mod bias;
pub use self::bias::{Random, Unbiased};

#[cfg(feature = "random")]
mod rand;

mod select;
pub use crate::select::Select;

mod static_select;
pub use crate::static_select::StaticSelect;

mod set;

#[macro_use]
mod macros;

#[cfg(feature = "tokio-entry")]
pub use ::selectme_macros::{main, main_rt, test, test_rt};

/// Hidden support module used by macros.
#[doc(hidden)]
pub mod __support {
    pub use crate::bias::{Bias, Random, Unbiased};
    pub use crate::select::DISABLED;
    pub use core::future::Future;
    pub use core::pin::Pin;
    pub use core::task::Poll;
    pub use selectme_macros::{inline, select};

    use core::task::Context;

    use crate::select::Select;
    use crate::set::{Number, Set};
    use crate::static_select::StaticSelect;

    /// Construct a random bias.
    #[inline]
    #[cfg(feature = "random")]
    pub fn random() -> Random {
        Random::new(crate::rand::thread_rng_n(64))
    }

    /// Construct an unbiased bias.
    #[inline]
    pub const fn unbiased() -> Unbiased {
        Unbiased
    }

    /// Setup a [Select] with a dynamic function used to poll.
    #[inline]
    pub fn select<Bits, S, B, T, O>(mask: Bits, bias: B, state: S, poll: T) -> Select<Bits, S, B, T>
    where
        Bits: Number,
        B: Bias<Bits>,
        T: FnMut(&mut Context<'_>, Pin<&mut S>, &mut Set<Bits>, u32) -> Poll<O>,
    {
        Select::new(Set::new(mask), bias, state, poll)
    }

    /// Setup a [Select] with a static function used to poll.
    #[inline]
    pub fn static_select<Bits, S, B, O>(
        mask: Bits,
        bias: B,
        state: S,
        poll: fn(&mut Context<'_>, Pin<&mut S>, &mut Set<Bits>, u32) -> Poll<O>,
    ) -> StaticSelect<Bits, S, B, O>
    where
        Bits: Number,
        B: Bias<Bits>,
    {
        StaticSelect::new(Set::new(mask), bias, state, poll)
    }
}
