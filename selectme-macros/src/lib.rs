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

#[cfg(feature = "tokio-entry")]
use crate::entry::{EntryKind, SupportsThreading};
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

#[cfg(feature = "tokio-entry")]
fn entry(
    kind: EntryKind,
    supports_threading: SupportsThreading,
    args: proc_macro::TokenStream,
    item_stream: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut buf = Buf::new();
    let mut errors = Vec::new();

    let config = entry::ConfigParser::new(args, &mut buf, &mut errors);
    let config = config.parse(kind, supports_threading);

    config.validate(kind, &mut errors);

    let item = entry::ItemParser::new(item_stream.clone(), &mut buf);
    let item = item.parse();

    if !item.has_async {
        errors.push(Error::new(
            Span::call_site(),
            format!(
                "function marked with `#[{}]` must be an `async` fn",
                kind.name()
            ),
        ));
    }

    let mut stream = TokenStream::default();

    item.expand_item(kind, config)
        .to_tokens(&mut stream, Span::mixed_site());
    format_item_errors(errors).to_tokens(&mut stream, Span::mixed_site());

    stream.into_token_stream()
}

#[proc_macro_attribute]
#[cfg(feature = "tokio-entry")]
pub fn main(
    args: proc_macro::TokenStream,
    item_stream: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    entry(
        entry::EntryKind::Main,
        SupportsThreading::Supported,
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
    entry(
        EntryKind::Test,
        SupportsThreading::Supported,
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
    entry(
        entry::EntryKind::Main,
        SupportsThreading::NotSupported,
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
    entry(
        entry::EntryKind::Test,
        SupportsThreading::NotSupported,
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

#[cfg(feature = "tokio-entry")]
fn format_item_errors<I>(errors: I) -> impl ToTokens
where
    I: IntoIterator<Item = Error>,
{
    from_fn(move |s| {
        for error in errors {
            s.write(error);
        }
    })
}
