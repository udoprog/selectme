use proc_macro::TokenTree;

/// A token stream that can be modified by this crate.
#[derive(Default)]
pub struct TokenStream {
    inner: Vec<TokenTree>,
}

impl TokenStream {
    /// Push a single token tree.
    pub fn push(&mut self, tt: impl Into<TokenTree>) {
        self.inner.push(tt.into());
    }

    /// Coerce into a token stream.
    pub fn into_token_stream(self) -> proc_macro::TokenStream {
        proc_macro::TokenStream::from_iter(self.inner)
    }
}
