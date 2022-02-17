//! Macros for [selectme].
//!
//! [selectme]: https://docs.rs/selectme

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(rust_2018_idioms)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(unreachable_pub)]

#[cfg(feature = "tokio-entry")]
mod entry;
mod error;
mod parsing;
mod select;
mod to_tokens;
mod tok;
mod token_stream;

#[allow(missing_docs)]
#[proc_macro]
pub fn select(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    select::build(input, select::Mode::Default)
}

#[allow(missing_docs)]
#[proc_macro]
pub fn inline(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    select::build(input, select::Mode::Inline)
}

#[allow(missing_docs)]
#[proc_macro_attribute]
#[cfg(feature = "tokio-entry")]
pub fn main(
    args: proc_macro::TokenStream,
    item_stream: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    crate::entry::build(
        crate::entry::EntryKind::Main,
        crate::entry::SupportsThreading::Supported,
        args,
        item_stream,
    )
}

#[allow(missing_docs)]
#[proc_macro_attribute]
#[cfg(feature = "tokio-entry")]
pub fn test(
    args: proc_macro::TokenStream,
    item_stream: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    crate::entry::build(
        crate::entry::EntryKind::Test,
        crate::entry::SupportsThreading::Supported,
        args,
        item_stream,
    )
}

#[allow(missing_docs)]
#[proc_macro_attribute]
#[cfg(feature = "tokio-entry")]
pub fn main_rt(
    args: proc_macro::TokenStream,
    item_stream: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    crate::entry::build(
        crate::entry::EntryKind::Main,
        crate::entry::SupportsThreading::NotSupported,
        args,
        item_stream,
    )
}

#[allow(missing_docs)]
#[proc_macro_attribute]
#[cfg(feature = "tokio-entry")]
pub fn test_rt(
    args: proc_macro::TokenStream,
    item_stream: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    crate::entry::build(
        crate::entry::EntryKind::Test,
        crate::entry::SupportsThreading::NotSupported,
        args,
        item_stream,
    )
}
