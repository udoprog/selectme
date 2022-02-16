#[cfg(feature = "tokio-entry")]
mod entry;
mod error;
mod parsing;
mod select;
mod to_tokens;
mod tok;
mod token_stream;

use proc_macro::{Delimiter, Span};
use to_tokens::{from_fn, ToTokens};

use crate::error::Error;
use crate::parsing::Buf;
use crate::token_stream::TokenStream;

#[proc_macro]
pub fn select(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut buf = Buf::new();
    let p = select::Parser::new(input, &mut buf);

    let mut stream = TokenStream::default();

    match p.parse() {
        Ok(output) => {
            output.expand().to_tokens(&mut stream, Span::mixed_site());
        }
        Err(errors) => {
            format_errors(errors).to_tokens(&mut stream, Span::mixed_site());
        }
    }

    stream.into_token_stream()
}

#[proc_macro]
pub fn inline(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut buf = Buf::new();
    let p = select::Parser::new(input, &mut buf);

    let mut stream = TokenStream::default();

    match p.parse() {
        Ok(output) => {
            output
                .expand_inline()
                .to_tokens(&mut stream, Span::mixed_site());
        }
        Err(errors) => {
            format_errors(errors).to_tokens(&mut stream, Span::mixed_site());
        }
    }

    stream.into_token_stream()
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

fn format_errors<I>(errors: I) -> impl ToTokens
where
    I: IntoIterator<Item = Error>,
    I::IntoIter: DoubleEndedIterator,
{
    from_fn(move |s| {
        let start = s.checkpoint();
        let mut it = errors.into_iter();

        if let Some(last) = it.next_back() {
            for error in it {
                s.write((error, ';'));
            }

            s.write(last);
        }

        s.group(Delimiter::Brace, start);
    })
}
