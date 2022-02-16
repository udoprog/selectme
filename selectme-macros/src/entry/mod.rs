mod output;
pub use self::output::{EntryKind, SupportsThreading};

mod parser;

use crate::error::Error;
use crate::parsing::Buf;
use crate::to_tokens::{from_fn, ToTokens};
use crate::token_stream::TokenStream;
use proc_macro::Span;

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

    let item = parser::ItemParser::new(item_stream.clone(), &mut buf);
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

    let fallback_span = item.find_block_span().unwrap_or_else(Span::call_site);
    let (start, end) = item
        .find_last_stmt_range()
        .unwrap_or_else(|| (fallback_span, fallback_span));

    item.expand_item(kind, config, start)
        .to_tokens(&mut stream, end);
    format_item_errors(errors).to_tokens(&mut stream, end);

    stream.into_token_stream()
}

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
