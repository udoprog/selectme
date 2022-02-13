mod error;
mod output;
mod parser;
mod to_tokens;
mod tok;
mod token_stream;

use proc_macro::{Delimiter, Span};
use to_tokens::ToTokens;

use crate::error::Error;
use crate::token_stream::TokenStream;

#[proc_macro]
pub fn select(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let p = parser::Parser::new(input);

    let mut stream = TokenStream::default();

    match p.parse() {
        Ok(output) => {
            output.expand(&mut stream, Span::mixed_site());
        }
        Err(errors) => {
            format_errors(errors, &mut stream, Span::mixed_site());
        }
    }

    stream.into_token_stream()
}

#[proc_macro]
pub fn inline(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let p = parser::Parser::new(input);

    let mut stream = TokenStream::default();

    match p.parse() {
        Ok(output) => {
            output.expand_inline(&mut stream, Span::mixed_site());
        }
        Err(errors) => {
            format_errors(errors, &mut stream, Span::mixed_site());
        }
    }

    stream.into_token_stream()
}

fn format_errors(errors: Vec<Error>, stream: &mut TokenStream, span: Span) {
    let start = stream.checkpoint();
    let mut it = errors.into_iter();

    if let Some(last) = it.next_back() {
        for error in it {
            error.to_tokens(stream, span);
            stream.tokens(span, ';');
        }

        last.to_tokens(stream, span);
    }

    stream.group(span, Delimiter::Brace, start);
}
