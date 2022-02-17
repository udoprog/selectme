mod output;
pub(crate) use self::output::Mode;

mod parser;

use crate::error::Error;
use crate::into_tokens::{from_fn, IntoTokens};
use crate::parsing::Buf;
use crate::token_stream::TokenStream;
use proc_macro::{Delimiter, Span};

pub(crate) fn build(input: proc_macro::TokenStream, mode: Mode) -> proc_macro::TokenStream {
    let mut buf = Buf::new();
    let p = parser::Parser::new(input, &mut buf);

    let mut stream = TokenStream::default();

    match p.parse(mode) {
        Ok(output) => {
            output.expand().into_tokens(&mut stream, Span::mixed_site());
        }
        Err(errors) => {
            format_errors(errors).into_tokens(&mut stream, Span::mixed_site());
        }
    }

    stream.into_token_stream()
}

fn format_errors<I>(errors: I) -> impl IntoTokens
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
