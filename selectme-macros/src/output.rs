use proc_macro::{Delimiter, Span, TokenTree};

use crate::parser::Branch;
use crate::to_tokens::{braced, bracketed, from_fn, parens, string, ToTokens};
use crate::tok::{self, S};
use crate::token_stream::TokenStream;

fn clean_pattern<'a>(tree: impl Iterator<Item = TokenTree> + 'a) -> impl ToTokens + 'a {
    from_fn(move |stream, span| {
        for tt in tree {
            match tt {
                TokenTree::Group(g) => {
                    let checkpoint = stream.checkpoint();
                    clean_pattern(g.stream().into_iter()).to_tokens(stream, span);
                    stream.group(span, g.delimiter(), checkpoint);
                }
                TokenTree::Ident(i) => {
                    if i.to_string() == "mut" {
                        continue;
                    }

                    stream.push(TokenTree::Ident(i));
                }
                TokenTree::Punct(p) => {
                    if p.as_char() == '&' {
                        continue;
                    }

                    stream.push(TokenTree::Punct(p));
                }
                tt => {
                    stream.push(tt);
                }
            }
        }
    })
}

fn generics(branches: &[Branch]) -> impl ToTokens + '_ {
    from_fn(move |stream, span| {
        stream.tokens(span, '<');

        for b in branches {
            stream.tokens(span, (b.generic.as_ref(), ','));
        }

        stream.tokens(span, '>');
    })
}

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
            let poller = (self.prefix, "PollerWaker");
            let static_ = (self.prefix, "StaticWaker");

            let private_mod = braced(from_fn(|stream, _| {
                stream.tokens(
                    span,
                    (
                        ("pub", "static", "WAKER", ':', static_),
                        '=',
                        (static_, S, "new", parens(())),
                        ';',
                    ),
                );

                for b in &self.branches {
                    stream.tokens(
                        span,
                        (
                            ("pub", "static", b.waker.as_ref(), ':', poller),
                            '=',
                            (poller, S, "new", parens(('&', "WAKER", ',', b.index)), ';'),
                        ),
                    );
                }

                let enum_body = from_fn(|stream, span| {
                    for b in &self.branches {
                        stream.tokens(span, (b.variant.as_ref(), parens(b.generic.as_ref()), ','));
                    }
                });

                stream.tokens(
                    span,
                    (
                        "pub",
                        "enum",
                        "Output",
                        generics(&self.branches),
                        braced(enum_body),
                    ),
                );
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
            for b in &self.branches {
                let fut = (
                    "unsafe",
                    braced((
                        "Pin",
                        S,
                        "map_unchecked_mut",
                        parens((
                            tok::as_mut("__fut"),
                            ',',
                            (tok::piped("f"), '&', "mut", "f", '.', b.index),
                        )),
                    )),
                );

                stream.tokens(span, (b.index, tok::ROCKET));

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

                    stream.tokens(span, braced((assign, poll)));
                } else {
                    let assign = ("let", "__fut", '=', fut, ';');
                    let poll = self.poll(b, None);
                    stream.tokens(span, braced((assign, poll)));
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
        let inner_poll = ("Future", S, "poll", parens(("__fut", ',', "cx")));

        (
            ("if", "let", tok::Poll::Ready("out")),
            '=',
            (
                self.prefix,
                "poll_by_ref",
                parens((
                    '&',
                    "private",
                    S,
                    b.waker.as_ref(),
                    ',',
                    tok::piped("cx"),
                    inner_poll,
                )),
            ),
            braced((
                unset.map(|var| (var, '.', "set", parens(tok::Option::<()>::None), ';')),
                ('#', bracketed(("allow", parens("unused_variables")))),
                (
                    "if",
                    "let",
                    clean_pattern(self.tokens[b.binding.clone()].iter().cloned()),
                ),
                '=',
                ('&', "out"),
                braced((
                    "return",
                    tok::Poll::Ready((
                        "private",
                        S,
                        "Output",
                        S,
                        b.variant.as_ref(),
                        parens("out"),
                    )),
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

        for b in &self.branches {
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
                    reset_base += 1 << b.index;
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

        let loop_item = ("match", "index", braced(self.match_body()));
        let body = braced((loop_item, tok::Poll::<()>::Pending));

        let select_args = parens((
            ('&', "private", S, "WAKER"),
            ',',
            "__fut",
            ',',
            tok::piped(("cx", ',', "mut", "__fut", ',', "index")),
            body,
        ));

        let body = from_fn(|stream, span| {
            for b in &self.branches {
                stream.tokens(span, ("private", S, "Output", S, b.variant.as_ref()));
                stream.tokens(span, parens(&self.tokens[b.binding.clone()]));
                stream.tokens(span, tok::ROCKET);
                stream.tokens(span, &self.tokens[b.branch.clone()]);
            }

            let panic_ = (
                "unreachable",
                '!',
                parens(string("branch cannot be reached")),
            );

            stream.tokens(span, ("_", tok::ROCKET, braced(panic_)));
        });

        stream.tokens(
            span,
            ("match", select, select_args, '.', "await", braced(body)),
        );
        stream.group(span, Delimiter::Brace, start);
    }
}
