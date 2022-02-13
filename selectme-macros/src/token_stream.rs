use proc_macro::{Delimiter, Group, Span, TokenTree};

use crate::to_tokens::ToTokens;

/// A checkpoint of the current location in the stream.
#[repr(transparent)]
pub struct Checkpoint(usize);

/// A token stream that can be modified by this crate.
#[derive(Default)]
pub struct TokenStream {
    inner: Vec<TokenTree>,
}

impl TokenStream {
    /// Push a single token tree.
    pub fn push(&mut self, tt: TokenTree) {
        self.inner.push(tt);
    }

    /// Push the given sequence of tokens.
    pub fn tokens<T>(&mut self, span: Span, tt: T)
    where
        T: ToTokens,
    {
        tt.to_tokens(self, span);
    }

    /// Push the given stream as a group.
    pub fn group(&mut self, span: Span, delimiter: Delimiter, Checkpoint(start): Checkpoint) {
        let it = self.inner.drain(start..);
        let mut group = Group::new(delimiter, proc_macro::TokenStream::from_iter(it));
        group.set_span(span);
        self.push(TokenTree::Group(group));
    }

    /// Coerce into a token stream.
    pub fn into_token_stream(self) -> proc_macro::TokenStream {
        proc_macro::TokenStream::from_iter(self.inner)
    }

    /// Extend the current stream from another.
    pub fn extend(&mut self, mut other: Self) {
        self.inner.append(&mut other.inner);
    }

    /// Get a checkpoint of the current location in the tree.
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint(self.inner.len())
    }
}
