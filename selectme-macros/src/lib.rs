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
use crate::token_stream::TokenStream;

#[proc_macro]
pub fn select(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let p = select::Parser::new(input);

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
    let p = select::Parser::new(input);

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
pub fn main(args: proc_macro::TokenStream, _: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let p = entry::Parser::new(args);

    let mut stream = TokenStream::default();

    match p.parse() {
        Ok(output) => {
            output
                .expand_main()
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
pub fn test(args: proc_macro::TokenStream, _: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let p = entry::Parser::new(args);

    let mut stream = TokenStream::default();

    match p.parse() {
        Ok(output) => {
            output
                .expand_test()
                .to_tokens(&mut stream, Span::mixed_site());
        }
        Err(errors) => {
            format_errors(errors).to_tokens(&mut stream, Span::mixed_site());
        }
    }

    stream.into_token_stream()
}

fn format_errors(errors: Vec<Error>) -> impl ToTokens {
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
