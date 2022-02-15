use proc_macro::TokenTree;

use crate::to_tokens::ToTokens;

/// The parsed output.
pub struct Output {
    #[allow(unused)]
    tokens: Vec<TokenTree>,
}

impl Output {
    pub(crate) fn new(tokens: Vec<TokenTree>) -> Self {
        Self { tokens }
    }

    /// Expand `main` macro.
    pub fn expand_main(&self) -> impl ToTokens + '_ {
        ()
    }

    /// Expand `test` macro.
    pub fn expand_test(&self) -> impl ToTokens + '_ {
        ()
    }
}
