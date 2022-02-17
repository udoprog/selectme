//! Token helpers.

use crate::to_tokens::ToTokens;
use crate::to_tokens::{braced, from_fn, parens};

/// `::`
pub(crate) const S: [char; 2] = [':', ':'];
/// `=>`
pub(crate) const ROCKET: [char; 2] = ['=', '>'];

/// `|<tt>|`.
pub(crate) fn piped(tt: impl ToTokens) -> impl ToTokens {
    from_fn(move |stream| {
        stream.write(('|', tt, '|'));
    })
}

/// `Pin::as_mut(<tt>)`.
pub(crate) fn pin_as_mut(tt: impl ToTokens) -> impl ToTokens {
    from_fn(move |stream| {
        stream.write(("Pin", S, "as_mut", parens(('&', "mut", tt))));
    })
}

/// `if <cond> { <then> } else { <else_then> }`.
pub(crate) fn if_else(
    cond: impl ToTokens,
    then: impl ToTokens,
    else_then: impl ToTokens,
) -> impl ToTokens {
    ("if", cond, braced(then), "else", braced(else_then))
}

/// `Option::Some(<tt>)`.
pub(crate) fn option_some(tt: impl ToTokens) -> impl ToTokens {
    ("Option", S, "Some", parens(tt))
}

/// `Option::None`.
pub(crate) const OPTION_NONE: (&str, [char; 2], &str) = ("Option", S, "None");

/// `Poll::Ready(<tt>)`.
pub(crate) fn poll_ready(tt: impl ToTokens) -> impl ToTokens {
    ("Poll", S, "Ready", parens(tt))
}
