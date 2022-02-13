use proc_macro::Span;
use to_tokens::ToTokens;

use crate::token_stream::TokenStream;

mod error;
mod output;
mod parser;
mod to_tokens;
mod tok;
mod token_stream;

#[proc_macro]
pub fn select(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let p = parser::Parser::new(input);

    let mut stream = TokenStream::default();

    match p.parse() {
        Ok(output) => {
            output.expand_deferred(&mut stream, Span::mixed_site());
        }
        Err(errors) => {
            for error in errors {
                error.to_tokens(&mut stream, Span::mixed_site());
            }
        }
    }

    stream.into_token_stream()
}

#[proc_macro]
pub fn immediate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let p = parser::Parser::new(input);

    let mut stream = TokenStream::default();

    match p.parse() {
        Ok(output) => {
            output.expand_immediate(&mut stream, Span::mixed_site());
        }
        Err(errors) => {
            for error in errors {
                error.to_tokens(&mut stream, Span::mixed_site());
            }
        }
    }

    stream.into_token_stream()
}
