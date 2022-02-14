//! Token helpers.

use proc_macro::Span;

use crate::to_tokens::ToTokens;
use crate::to_tokens::{braced, from_fn, parens};
use crate::token_stream::TokenStream;

/// `::`
pub const S: [char; 2] = [':', ':'];
/// `=>`
pub const ROCKET: [char; 2] = ['=', '>'];

/// A piped expression.
pub fn piped(tt: impl ToTokens) -> impl ToTokens {
    from_fn(move |stream| {
        stream.write(('|', tt, '|'));
    })
}

pub fn as_mut(tt: impl ToTokens) -> impl ToTokens {
    from_fn(move |stream| {
        stream.write(("Pin", S, "as_mut", parens(('&', "mut", tt))));
    })
}

/// Generate an `if <cond> { <then> } else { <else_then> }` expression.
pub fn if_else(
    cond: impl ToTokens,
    then: impl ToTokens,
    else_then: impl ToTokens,
) -> impl ToTokens {
    ("if", cond, braced(then), "else", braced(else_then))
}

pub enum Option<T> {
    Some(T),
    None,
}

impl<T> ToTokens for Option<T>
where
    T: ToTokens,
{
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        match self {
            Option::Some(tt) => stream.write(span, ("Option", S, "Some", parens(tt))),
            Option::None => stream.write(span, ("Option", S, "None")),
        }
    }
}

pub enum Poll<T> {
    Ready(T),
}

impl<T> ToTokens for Poll<T>
where
    T: ToTokens,
{
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        match self {
            Poll::Ready(tt) => stream.write(span, ("Poll", S, "Ready", parens(tt))),
        }
    }
}
