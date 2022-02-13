use proc_macro::{Delimiter, Span, TokenTree};

use crate::parser::{Branch, Else};
use crate::to_tokens::{braced, bracketed, from_fn, parens, string, ToTokens};
use crate::tok::{self, S};
use crate::token_stream::TokenStream;

/// The name of the output enum.
const OUT: &str = "Out";

/// The parsed output.
pub struct Output {
    tokens: Vec<TokenTree>,
    branches: Vec<Branch>,
    else_branch: Option<Else>,
    prefix: Prefix,
}

impl Output {
    /// Construct new output.
    pub(crate) fn new(
        tokens: Vec<TokenTree>,
        branches: Vec<Branch>,
        else_branch: Option<Else>,
        prefix: Prefix,
    ) -> Self {
        Self {
            tokens,
            branches,
            else_branch,
            prefix,
        }
    }

    /// Output enumeration.
    fn out_enum(&self) -> impl ToTokens + '_ {
        (
            ("pub", "enum", OUT),
            branch_generics(&self.branches),
            braced(from_fn(|stream, span| {
                for b in &self.branches {
                    stream.tokens(span, (b.variant.as_ref(), parens(b.generic.as_ref()), ','));
                }

                if self.else_branch.is_some() {
                    stream.tokens(span, ("Disabled", ','));
                }
            })),
        )
    }

    /// Private module declaration.
    fn private_mod(&self, immediate: bool) -> impl ToTokens + '_ {
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

                if immediate {
                    stream.tokens(span, self.out_enum());
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

    fn match_body(&self, immediate: bool) -> impl ToTokens + '_ {
        from_fn(move |stream, span| {
            for b in &self.branches {
                let fut = (
                    "unsafe",
                    braced((
                        ("Pin", S, "new_unchecked"),
                        parens(('&', "mut", "__fut", '.', b.index)),
                    )),
                );

                stream.tokens(span, (b.index, tok::ROCKET));

                if b.condition.is_some() || b.fuse {
                    let assign = ("let", "mut", b.pin.as_ref(), '=', fut, ';');
                    let poll = self.poll(b, Some(b.pin.as_ref()), immediate);

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
                    let poll = self.poll(b, None, immediate);
                    stream.tokens(span, braced((assign, poll)));
                }
            }

            if let Some(e) = &self.else_branch {
                if immediate && self.else_branch.is_some() {
                    let body = (
                        "return",
                        tok::Poll::Ready(("private", S, OUT, S, "Disabled")),
                        ';',
                    );
                    stream.tokens(span, ("usize", S, "MAX", tok::ROCKET, braced(body)));
                } else {
                    let body = (
                        "return",
                        tok::Poll::Ready(&self.tokens[e.branch.clone()]),
                        ';',
                    );
                    stream.tokens(span, ("usize", S, "MAX", tok::ROCKET, braced(body)));
                }
            }

            let panic_branch = (
                ("panic", '!'),
                parens((string("no branch with index `{}`"), ',', "n")),
            );

            stream.tokens(span, ("n", tok::ROCKET, braced(panic_branch)));
        })
    }

    fn immediate_match<'a>(&'a self, b: &'a Branch) -> impl ToTokens + 'a {
        from_fn(|stream, span| {
            stream.tokens(
                span,
                ('#', bracketed(("allow", parens("unused_variables")))),
            );
            stream.tokens(
                span,
                (
                    ("if", "let"),
                    (clean_pattern(
                        self.tokens[b.binding.clone()].iter().cloned(),
                    ),),
                    ('=', '&', "out"),
                ),
            );
            stream.tokens(
                span,
                braced((
                    "return",
                    tok::Poll::Ready(("private", S, OUT, S, b.variant.as_ref(), parens("out"))),
                    ';',
                )),
            );
        })
    }

    fn immediate_out_branch<'a>(&'a self, b: &'a Branch) -> impl ToTokens + 'a {
        (
            ("private", S, OUT, S, b.variant.as_ref()),
            parens(&self.tokens[b.binding.clone()]),
            (tok::ROCKET, &self.tokens[b.branch.clone()]),
        )
    }

    fn immediate_else_branch<'a>(&'a self, e: &'a Else) -> impl ToTokens + 'a {
        (
            ("private", S, OUT, S, "Disabled"),
            (tok::ROCKET, &self.tokens[e.branch.clone()]),
        )
    }

    fn poll<'a>(
        &'a self,
        b: &'a Branch,
        unset: Option<&'a str>,
        immediate: bool,
    ) -> impl ToTokens + 'a {
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
                from_fn(move |stream, span| {
                    if immediate {
                        stream.tokens(span, self.immediate_match(b));
                    } else {
                        stream.tokens(
                            span,
                            (
                                "if",
                                "let",
                                &self.tokens[b.binding.clone()],
                                '=',
                                "out",
                                braced((
                                    "return",
                                    tok::Poll::Ready(&self.tokens[b.branch.clone()]),
                                    ';',
                                )),
                            ),
                        );
                    }
                }),
            )),
        )
    }

    fn conditions(&self, stream: &mut TokenStream, span: Span) -> usize {
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

        reset_base
    }

    fn reset(&self, reset_base: usize) -> impl ToTokens + '_ {
        let args = from_fn(move |stream, span| {
            let mut it = self
                .branches
                .iter()
                .filter_map(|b| b.condition.as_ref().map(|c| (b, c.var.as_ref())));

            if let Some((b, var)) = it.next_back() {
                if reset_base != 0 {
                    stream.tokens(span, reset_base);
                    stream.tokens(span, '+');
                }

                for (b, var) in it {
                    stream.tokens(span, tok::if_else(var, 1 << b.index, 0));
                    stream.tokens(span, '+');
                }

                stream.tokens(span, tok::if_else(var, 1 << b.index, 0));
            } else {
                stream.tokens(span, reset_base);
            }
        });

        ("private", S, "WAKER", '.', "reset", parens(args), ';')
    }

    /// Expand a select which is deferred.
    pub fn expand(self, stream: &mut TokenStream, span: Span) {
        let start = stream.checkpoint();

        let imports = (
            self.prefix,
            "macros",
            S,
            braced(("Future", ',', "Pin", ',', "Poll")),
        );

        stream.tokens(span, ("use", imports, ';'));
        stream.tokens(span, self.private_mod(false));

        let reset_base = self.conditions(stream, span);
        stream.tokens(span, self.futures());
        stream.tokens(span, self.reset(reset_base));

        let loop_item = ("match", "index", braced(self.match_body(false)));
        let body = braced((loop_item, tok::Poll::<()>::Pending));

        let select_args = parens((
            ('&', "private", S, "WAKER"),
            ',',
            "__fut",
            ',',
            tok::piped(("cx", ',', "mut", "__fut", ',', "index")),
            body,
        ));

        stream.tokens(span, (self.prefix, "select", select_args));
        stream.group(span, Delimiter::Brace, start);
    }

    /// Expand a select which is awaited immediately.
    pub fn expand_inline(self, stream: &mut TokenStream, span: Span) {
        let start = stream.checkpoint();

        let imports = (
            self.prefix,
            "macros",
            S,
            braced(("Future", ',', "Pin", ',', "Poll")),
        );

        stream.tokens(span, ("use", imports, ';'));
        stream.tokens(span, self.private_mod(true));

        let reset_base = self.conditions(stream, span);
        stream.tokens(span, self.futures());
        stream.tokens(span, self.reset(reset_base));

        let loop_item = ("match", "index", braced(self.match_body(true)));
        let body = braced((loop_item, tok::Poll::<()>::Pending));

        let select_args = parens((
            ('&', "private", S, "WAKER"),
            ',',
            "__fut",
            ',',
            tok::piped(("cx", ',', "mut", "__fut", ',', "index")),
            body,
        ));

        let output_body = from_fn(|stream, span| {
            for b in &self.branches {
                stream.tokens(span, self.immediate_out_branch(b));
            }

            if let Some(e) = &self.else_branch {
                stream.tokens(span, self.immediate_else_branch(e));
            }

            let panic_ = (
                ("unreachable", '!'),
                parens(string("branch cannot be reached")),
            );

            stream.tokens(span, ("_", tok::ROCKET, braced(panic_)));
        });

        stream.tokens(
            span,
            (
                "match",
                (self.prefix, "select", select_args, '.', "await"),
                braced(output_body),
            ),
        );

        stream.group(span, Delimiter::Brace, start);
    }
}

/// Clean up a pattern by skipping over any `mut` and `&` tokens.
fn clean_pattern(tree: impl Iterator<Item = TokenTree>) -> impl ToTokens {
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

fn branch_generics(branches: &[Branch]) -> impl ToTokens + '_ {
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
