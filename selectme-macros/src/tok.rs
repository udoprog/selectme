//! Token helpers.

use crate::into_tokens::IntoTokens;
use crate::into_tokens::{braced, from_fn, parens};

/// `::`
pub(crate) const S: [char; 2] = [':', ':'];
/// `=>`
pub(crate) const ROCKET: [char; 2] = ['=', '>'];

/// `|<tt>|`.
pub(crate) fn piped(tt: impl IntoTokens) -> impl IntoTokens {
    from_fn(move |stream| {
        stream.write(('|', tt, '|'));
    })
}

/// `Pin::as_mut(<tt>)`.
pub(crate) fn pin_as_mut(tt: impl IntoTokens) -> impl IntoTokens {
    from_fn(move |stream| {
        stream.write(("Pin", S, "as_mut", parens(('&', "mut", tt))));
    })
}

/// `if <cond> { <then> } else { <else_then> }`.
pub(crate) fn if_else(
    cond: impl IntoTokens,
    then: impl IntoTokens,
    else_then: impl IntoTokens,
) -> impl IntoTokens {
    ("if", cond, braced(then), "else", braced(else_then))
}

/// `Option::Some(<tt>)`.
pub(crate) fn option_some(tt: impl IntoTokens) -> impl IntoTokens {
    ("Option", S, "Some", parens(tt))
}

/// `Option::None`.
pub(crate) const OPTION_NONE: (&str, [char; 2], &str) = ("Option", S, "None");

/// `Poll::Ready(<tt>)`.
pub(crate) fn poll_ready(tt: impl IntoTokens) -> impl IntoTokens {
    ("Poll", S, "Ready", parens(tt))
}
