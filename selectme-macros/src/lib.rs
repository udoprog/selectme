use proc_macro::{TokenStream, TokenTree};
use to_tokens::ToTokens;

mod error;
mod parser;
mod to_tokens;
mod token_stream;

#[proc_macro]
pub fn select(input: TokenStream) -> TokenStream {
    let p = parser::Parser::new(input);

    match p.parse() {
        Ok(..) => TokenStream::from_iter(std::iter::empty::<TokenTree>()),
        Err(errors) => {
            let mut stream = token_stream::TokenStream::default();

            for error in errors {
                error.to_tokens(&mut stream);
            }

            stream.into_token_stream()
        }
    }
}
