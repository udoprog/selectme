mod output;
use proc_macro::Span;

pub(crate) use self::output::{EntryKind, SupportsThreading};

mod parser;

use crate::error::Error;
use crate::into_tokens::{from_fn, IntoTokens};
use crate::parsing::Buf;
use crate::token_stream::TokenStream;

/// Configurable macro code to build entry.
pub(crate) fn build(
    kind: EntryKind,
    supports_threading: SupportsThreading,
    args: proc_macro::TokenStream,
    item_stream: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut buf = Buf::new();
    let mut errors = Vec::new();

    let config = parser::ConfigParser::new(args, &mut buf, &mut errors);
    let config = config.parse(kind, supports_threading);

    config.validate(kind, &mut errors);

    let item = parser::ItemParser::new(item_stream, &mut buf);
    let item = item.parse();

    item.validate(kind, &mut errors);

    let mut stream = TokenStream::default();

    let span = item.block_span().unwrap_or_else(Span::call_site);

    item.expand_item(kind, config)
        .into_tokens(&mut stream, span);
    format_item_errors(errors).into_tokens(&mut stream, span);

    stream.into_token_stream()
}

fn format_item_errors<I>(errors: I) -> impl IntoTokens
where
    I: IntoIterator<Item = Error>,
{
    from_fn(move |s| {
        for error in errors {
            s.write(error);
        }
    })
}
