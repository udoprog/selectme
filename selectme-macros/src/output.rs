use proc_macro::{Delimiter, Span, TokenTree};

use crate::parser::Branch;
use crate::to_tokens::{braced, from_fn, parens, string, ToTokens};
use crate::tok::{self, S};
use crate::token_stream::TokenStream;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Prefix {
    SelectMe,
}

impl ToTokens for Prefix {
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        match self {
            Prefix::SelectMe => {
                stream.tokens(span, (S, "selectme", S));
            }
        }
    }
}

/// The parsed output.
pub struct Output {
    tokens: Vec<TokenTree>,
    branches: Vec<Branch>,
    prefix: Prefix,
}

impl Output {
    /// Construct new output.
    pub(crate) fn new(tokens: Vec<TokenTree>, branches: Vec<Branch>, prefix: Prefix) -> Self {
        Self {
            tokens,
            branches,
            prefix,
        }
    }

    /// Private module declaration.
    fn private_mod(&self) -> impl ToTokens + '_ {
        from_fn(move |stream, span| {
            let poller_waker = (self.prefix, "PollerWaker");
            let static_waker = (self.prefix, "StaticWaker");

            let private_mod = braced(from_fn(|stream, _| {
                stream.tokens(
                    span,
                    (
                        ("pub", "static", "WAKER", ':', static_waker),
                        '=',
                        (static_waker, S, "new", parens(())),
                        ';',
                    ),
                );

                for (n, block) in self.branches.iter().enumerate() {
                    stream.tokens(
                        span,
                        (
                            ("pub", "static", block.waker.as_ref(), ':', poller_waker),
                            '=',
                            (poller_waker, S, "new", parens(('&', "WAKER", ',', n)), ';'),
                        ),
                    );
                }
            }));

            stream.tokens(span, ("mod", "private", private_mod));
        })
    }

    fn futures(&self) -> impl ToTokens + '_ {
        let init = from_fn(|stream, span| {
            for b in &self.branches {
                if let Some(c) = &b.condition {
                    stream.tokens(
                        span,
                        tok::if_else(
                            c.var.as_ref(),
                            tok::Option::Some(&self.tokens[b.expr.clone()]),
                            tok::Option::<()>::None,
                        ),
                    );
                } else if b.fuse {
                    stream.tokens(span, tok::Option::Some(&self.tokens[b.expr.clone()]));
                } else {
                    stream.tokens(span, &self.tokens[b.expr.clone()]);
                }

                stream.tokens(span, ',');
            }
        });

        ("let", "__fut", '=', parens(init), ';')
    }

    fn match_body(&self) -> impl ToTokens + '_ {
        from_fn(|stream, span| {
            for (n, b) in self.branches.iter().enumerate() {
                let fut = (
                    "unsafe",
                    braced((
                        "Pin",
                        S,
                        "map_unchecked_mut",
                        parens((
                            tok::as_mut("__fut"),
                            ',',
                            tok::piped("f"),
                            '&',
                            "mut",
                            "f",
                            '.',
                            n,
                        )),
                    )),
                );

                if b.condition.is_some() || b.fuse {
                    let assign = ("let", "mut", b.pin.as_ref(), '=', fut, ';');
                    let poll = self.poll(b, Some(b.pin.as_ref()));

                    let poll = (
                        ("if", "let", tok::Option::Some("__fut")),
                        '=',
                        (
                            "Option",
                            S,
                            "as_pin_mut",
                            parens(tok::as_mut(b.pin.as_ref())),
                        ),
                        braced(poll),
                    );

                    stream.tokens(span, (n, tok::ROCKET, braced((assign, poll))));
                } else {
                    let assign = ("let", "__fut", '=', fut, ';');
                    let poll = self.poll(b, None);
                    stream.tokens(span, (n, tok::ROCKET, braced((assign, poll))));
                }
            }

            let panic_branch = (
                "panic",
                '!',
                parens((string("no branch with index `{}`"), ',', "n")),
            );

            stream.tokens(span, ("n", tok::ROCKET, braced(panic_branch)));
        })
    }

    fn poll<'a>(&'a self, b: &'a Branch, unset: Option<&'a str>) -> impl ToTokens + 'a {
        (
            (
                "if",
                "let",
                tok::Poll::Ready(&self.tokens[b.binding.clone()]),
            ),
            '=',
            (
                "poller",
                '.',
                "poll",
                parens((
                    "cx",
                    ',',
                    ('&', "private", S, b.waker.as_ref()),
                    ',',
                    (
                        tok::piped("cx"),
                        ("Future", S, "poll", parens(("__fut", ',', "cx"))),
                    ),
                )),
            ),
            braced((
                unset.map(|var| (var, '.', "set", parens(tok::Option::<()>::None), ';')),
                "return",
                tok::Poll::Ready((
                    parens((tok::piped(()), &self.tokens[b.branch.clone()])),
                    parens(()),
                )),
            )),
        )
    }
}

impl ToTokens for Output {
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        let start = stream.checkpoint();

        let private = ("private", S);
        let select = (self.prefix, "select");

        let imports = (
            self.prefix,
            "macros",
            S,
            braced(("Future", ',', "Pin", ',', "Poll")),
        );

        stream.tokens(span, ("use", imports, ';'));
        stream.tokens(span, self.private_mod());

        let mut reset_base = 0;

        for (n, b) in self.branches.iter().enumerate() {
            match &b.condition {
                Some(c) => {
                    stream.tokens(
                        span,
                        (
                            "let",
                            c.var.as_ref(),
                            '=',
                            &self.tokens[c.range.clone()],
                            ';',
                        ),
                    );
                }
                None => {
                    reset_base += 1 << n;
                }
            }
        }

        stream.tokens(span, self.futures());

        let args = from_fn(|stream, span| {
            let mut it = self
                .branches
                .iter()
                .filter_map(|b| b.condition.as_ref().map(|c| (b, c.var.as_ref())));

            if let Some((b, var)) = it.next_back() {
                if reset_base != 0 {
                    stream.tokens(span, reset_base);
                    stream.tokens(span, '+');
                }

                while let Some((b, var)) = it.next() {
                    stream.tokens(span, tok::if_else(var, 1 << b.index, 0));
                    stream.tokens(span, '+');
                }

                stream.tokens(span, tok::if_else(var, 1 << b.index, 0));
            } else {
                stream.tokens(span, reset_base);
            }
        });

        stream.tokens(span, (private, "WAKER", '.', "reset", parens(args), ';'));

        let loop_item = (
            ("while", "let", tok::Option::Some("index")),
            '=',
            ("poller", '.', "next", parens(())),
            braced(("match", "index", braced(self.match_body()))),
        );

        let body = braced((loop_item, tok::Poll::<()>::Pending));

        let select_args = parens((
            ('&', "private", S, "WAKER"),
            ',',
            "__fut",
            ',',
            tok::piped(("cx", ',', "mut", "__fut", ',', "poller")),
            body,
        ));

        stream.tokens(span, (select, select_args));
        stream.group(span, Delimiter::Brace, start);
    }
}
