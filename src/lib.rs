//! [<img alt="github" src="https://img.shields.io/badge/github-udoprog/selectme?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/selectme)
//! [<img alt="crates.io" src="https://img.shields.io/crates/v/selectme.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/selectme)
//! [<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-selectme?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/selectme)
//! [<img alt="build status" src="https://img.shields.io/github/workflow/status/udoprog/selectme/CI/main?style=for-the-badge" height="20">](https://github.com/udoprog/selectme/actions?query=branch%3Amain)
//!
//! A fast and fair select! implementation for asynchronous programming.
//!
//! See the [select!] or [inline!] macros for documentation.
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! selectme = "0.2.5"
//! ```
//!
//! ## Examples
//!
//! The following is a simple example showcasing two branches being polled
//! concurrently. For more documentation see [select!].
//!
//! ```
//! async fn do_stuff_async() {
//!     // async work
//! }
//!
//! async fn more_async_work() {
//!     // more here
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     selectme::select! {
//!         _ = do_stuff_async() => {
//!             println!("do_stuff_async() completed first")
//!         }
//!         _ = more_async_work() => {
//!             println!("more_async_work() completed first")
//!         }
//!     };
//! }
//! ```
//!
//! [select!]: https://docs.rs/selectme/latest/selectme/macro.select.html
//! [inline!]: https://docs.rs/selectme/latest/selectme/macro.inline.html

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

mod select;
pub use crate::select::Select;

mod set;

#[macro_use]
mod macros;

/// Hidden support module used by macros.
#[doc(hidden)]
pub mod __support {
    pub use crate::select::DISABLED;
    pub use core::future::Future;
    pub use core::pin::Pin;
    pub use core::task::Poll;
    pub use selectme_macros::{inline, select};

    use core::task::Context;

    use crate::select::Select;
    use crate::set::Set;

    /// Perform a poll with the initial mask.
    #[inline]
    pub fn select<T, S, O>(mask: u64, state: S, poll: T) -> Select<T, S>
    where
        T: FnMut(&mut Context<'_>, Pin<&mut S>, &mut Set, u32) -> Poll<O>,
    {
        Select::new(Set::new(mask), state, poll)
    }
}
