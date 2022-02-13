use std::iter::once;

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

use crate::to_tokens::ToTokens;
use crate::token_stream;

/// An error that can be raised during parsing which is associated with a span.
#[derive(Debug)]
pub struct Error {
    span: Span,
    message: &'static str,
}

impl Error {
    pub(crate) fn new(span: Span, message: &'static str) -> Self {
        Self { span, message }
    }
}

impl ToTokens for Error {
    fn to_tokens(&self, stream: &mut token_stream::TokenStream) {
        stream.push(Ident::new("compile_error", self.span));
        let mut exclamation = Punct::new('!', Spacing::Alone);
        exclamation.set_span(self.span);
        stream.push(exclamation);

        let mut message = Literal::string(self.message);
        message.set_span(self.span);

        let message = TokenStream::from_iter(once(TokenTree::Literal(message)));
        let mut group = Group::new(Delimiter::Brace, message);
        group.set_span(self.span);

        stream.push(group);
    }
}
