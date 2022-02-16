#[cfg(feature = "tokio-entry")]
mod entry;
mod error;
mod parsing;
mod select;
mod to_tokens;
mod tok;
mod token_stream;

#[proc_macro]
pub fn select(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    select::build(input, select::Mode::Default)
}

#[proc_macro]
pub fn inline(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    select::build(input, select::Mode::Inline)
}

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
