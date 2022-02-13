use crate::token_stream::TokenStream;

pub trait ToTokens {
    /// Convert into tokens.
    fn to_tokens(&self, stream: &mut TokenStream);
}
