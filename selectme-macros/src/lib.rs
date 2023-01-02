//! [<img alt="github" src="https://img.shields.io/badge/github-udoprog/selectme-8da0cb?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/selectme)
//! [<img alt="crates.io" src="https://img.shields.io/crates/v/selectme-macros.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/selectme-macros)
//! [<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-selectme--macros-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/selectme-macros)
//! [<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/udoprog/selectme/ci.yml?branch=main&style=for-the-badge" height="20">](https://github.com/udoprog/selectme/actions?query=branch%3Amain)
//!
//! Macros for [selectme].
//!
//! [selectme]: https://docs.rs/selectme

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(rust_2018_idioms)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(unreachable_pub)]

use proc_macro::TokenStream;

mod entry;
mod error;
mod into_tokens;
mod parsing;
mod select;
mod tok;
mod token_stream;

#[allow(missing_docs)]
#[proc_macro]
pub fn select(input: TokenStream) -> TokenStream {
    select::build(input, select::Mode::Default)
}

#[allow(missing_docs)]
#[proc_macro]
pub fn inline(input: TokenStream) -> TokenStream {
    select::build(input, select::Mode::Inline)
}

/// Marks async function to be executed by the selected runtime. This macro
/// helps set up a `Runtime` without requiring the user to use [Runtime] or
/// [Builder] directly.
///
/// Note: This macro is designed to be simplistic and targets applications that
/// do not require a complex setup. If the provided functionality is not
/// sufficient, you may be interested in using [Builder], which provides a more
/// powerful interface.
///
/// Note: This macro can be used on any function and not just the `main`
/// function. Using it on a non-main function makes the function behave as if it
/// was synchronous by starting a new runtime each time it is called. If the
/// function is called often, it is preferable to create the runtime using the
/// runtime builder so the runtime can be reused across calls.
///
/// # Multi-threaded runtime
///
/// To use the multi-threaded runtime, the macro can be configured using
///
/// ```
/// #[selectme::main(flavor = "multi_thread", worker_threads = 10)]
/// # async fn main() {}
/// ```
///
/// The `worker_threads` option configures the number of worker threads, and
/// defaults to the number of cpus on the system. This is the default flavor.
///
/// Note: The multi-threaded runtime requires the `rt-multi-thread` feature
/// flag.
///
/// # Current thread runtime
///
/// To use the single-threaded runtime known as the `current_thread` runtime,
/// the macro can be configured using
///
/// ```
/// #[selectme::main(flavor = "current_thread")]
/// # async fn main() {}
/// ```
///
/// ## Function arguments:
///
/// Arguments are allowed for any functions aside from `main` which is special
///
/// ## Usage
///
/// ### Using the multi-thread runtime
///
/// ```rust
/// #[selectme::main]
/// async fn main() {
///     println!("Hello world");
/// }
/// ```
///
/// Equivalent code not using `#[selectme::main]`
///
/// ```rust
/// fn main() {
///     async fn main() {
///         println!("Hello world");
///     }
///
///     tokio::runtime::Builder::new_multi_thread()
///         .enable_all()
///         .build()
///         .unwrap()
///         .block_on(main())
/// }
/// ```
///
/// ### Using current thread runtime
///
/// The basic scheduler is single-threaded.
///
/// ```rust
/// #[selectme::main(flavor = "current_thread")]
/// async fn main() {
///     println!("Hello world");
/// }
/// ```
///
/// Equivalent code not using `#[selectme::main]`
///
/// ```rust
/// fn main() {
///     tokio::runtime::Builder::new_current_thread()
///         .enable_all()
///         .build()
///         .unwrap()
///         .block_on(async {
///             println!("Hello world");
///         })
/// }
/// ```
///
/// ### Set number of worker threads
///
/// ```rust
/// #[selectme::main(worker_threads = 2)]
/// async fn main() {
///     println!("Hello world");
/// }
/// ```
///
/// Equivalent code not using `#[selectme::main]`
///
/// ```rust
/// fn main() {
///     async fn main() {
///         println!("Hello world");
///     }
///
///     tokio::runtime::Builder::new_multi_thread()
///         .worker_threads(2)
///         .enable_all()
///         .build()
///         .unwrap()
///         .block_on(main())
/// }
/// ```
///
/// ### Configure the runtime to start with time paused
///
/// ```rust
/// #[selectme::main(flavor = "current_thread", start_paused = true)]
/// async fn main() {
///     println!("Hello world");
/// }
/// ```
///
/// Equivalent code not using `#[selectme::main]`
///
/// ```rust
/// fn main() {
///     tokio::runtime::Builder::new_current_thread()
///         .enable_all()
///         .start_paused(true)
///         .build()
///         .unwrap()
///         .block_on(async {
///             println!("Hello world");
///         })
/// }
/// ```
///
/// Note that `start_paused` requires the `test-util` feature to be enabled.
///
/// ### NOTE:
///
/// If you rename the Tokio crate in your dependencies this macro will not work.
/// If you must rename the current version of Tokio because you're also using an
/// older version of Tokio, you _must_ make the current version of Tokio
/// available as `tokio` in the module where this macro is expanded.
///
/// [Runtime]: https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html
/// [Builder]: https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html
#[proc_macro_attribute]
pub fn main(args: TokenStream, item_stream: TokenStream) -> TokenStream {
    crate::entry::build(
        crate::entry::EntryKind::Main,
        crate::entry::SupportsThreading::Supported,
        args,
        item_stream,
    )
}

/// Marks async function to be executed by runtime, suitable to test environment
///
/// ## Usage
///
/// ### Multi-thread runtime
///
/// ```no_run
/// #[selectme::test(flavor = "multi_thread", worker_threads = 1)]
/// async fn my_test() {
///     assert!(true);
/// }
/// ```
///
/// ### Using default
///
/// The default test runtime is single-threaded.
///
/// ```no_run
/// #[selectme::test]
/// async fn my_test() {
///     assert!(true);
/// }
/// ```
///
/// ### Configure the runtime to start with time paused
///
/// ```no_run
/// #[selectme::test(start_paused = true)]
/// async fn my_test() {
///     assert!(true);
/// }
/// ```
///
/// Note that `start_paused` requires the `test-util` feature to be enabled.
///
/// ### NOTE:
///
/// If you rename the Tokio crate in your dependencies this macro will not work.
/// If you must rename the current version of Tokio because you're also using an
/// older version of Tokio, you _must_ make the current version of Tokio
/// available as `tokio` in the module where this macro is expanded.
#[proc_macro_attribute]
pub fn test(args: TokenStream, item_stream: TokenStream) -> TokenStream {
    crate::entry::build(
        crate::entry::EntryKind::Test,
        crate::entry::SupportsThreading::Supported,
        args,
        item_stream,
    )
}

#[cfg(feature = "tokio-entry")]
#[allow(missing_docs)]
#[proc_macro_attribute]
pub fn main_rt(args: TokenStream, item_stream: TokenStream) -> TokenStream {
    crate::entry::build(
        crate::entry::EntryKind::Main,
        crate::entry::SupportsThreading::NotSupported,
        args,
        item_stream,
    )
}

#[cfg(feature = "tokio-entry")]
#[allow(missing_docs)]
#[proc_macro_attribute]
pub fn test_rt(args: TokenStream, item_stream: TokenStream) -> TokenStream {
    crate::entry::build(
        crate::entry::EntryKind::Test,
        crate::entry::SupportsThreading::NotSupported,
        args,
        item_stream,
    )
}

/// Always fails with the error message below.
/// ```text
/// The #[selectme::main] macro requires rt or rt-multi-thread.
/// ```
#[cfg(feature = "tokio-entry")]
#[proc_macro_attribute]
pub fn main_fail(_args: TokenStream, _item: TokenStream) -> TokenStream {
    error::expand("the #[selectme::main] macro requires rt or rt-multi-thread")
}

/// Always fails with the error message below.
/// ```text
/// The #[selectme::test] macro requires rt or rt-multi-thread.
/// ```
#[cfg(feature = "tokio-entry")]
#[proc_macro_attribute]
pub fn test_fail(_args: TokenStream, _item: TokenStream) -> TokenStream {
    error::expand("the #[selectme::test] macro requires rt or rt-multi-thread")
}
