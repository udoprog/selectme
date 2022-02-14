use std::ops;

use proc_macro::{Delimiter, Span, TokenTree};

use crate::parser::{Block, Branch, Else};
use crate::to_tokens::{braced, bracketed, from_fn, parens, string, ToTokens};
use crate::tok::{self, S};
use crate::token_stream::TokenStream;

/// The name of the output enum.
const OUT: &str = "Out";
/// The private module in use.
const PRIVATE: &str = "__private";

/// The parsed output.
pub struct Output {
    tokens: Vec<TokenTree>,
    krate: ops::Range<usize>,
    branches: Vec<Branch>,
    else_branch: Option<Else>,
}

impl Output {
    /// Construct new output.
    pub(crate) fn new(
        tokens: Vec<TokenTree>,
        krate: ops::Range<usize>,
        branches: Vec<Branch>,
        else_branch: Option<Else>,
    ) -> Self {
        Self {
            tokens,
            krate,
            branches,
            else_branch,
        }
    }

    /// Render the support module.
    fn support(&self) -> impl ToTokens + Copy + '_ {
        let toks = &self.tokens[self.krate.clone()];
        (toks, S, "__support", S)
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
    fn private_mod(&self) -> impl ToTokens + '_ {
        from_fn(move |stream, span| {
            let poller = (self.support(), "PollerWaker");
            let static_ = (self.support(), "StaticWaker");

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

                stream.tokens(span, self.out_enum());
            }));

            stream.tokens(span, ("mod", PRIVATE, private_mod));
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

        ("let", "mut", "__fut", '=', parens(init), ';')
    }

    fn matches(&self) -> impl ToTokens + '_ {
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

            if let Some(e) = &self.else_branch {
                if self.else_branch.is_some() {
                    let body = ("break", (PRIVATE, S, OUT, S, "Disabled"), ';');
                    stream.tokens(
                        span,
                        ((self.support(), "DISABLED"), tok::ROCKET, braced(body)),
                    );
                } else {
                    let body = ("break", self.block(&e.block), ';');
                    stream.tokens(
                        span,
                        ((self.support(), "DISABLED"), tok::ROCKET, braced(body)),
                    );
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
                    (clean_pattern(self.tokens[b.binding.clone()].iter().cloned())),
                    ('=', '&', "out"),
                ),
            );

            stream.tokens(
                span,
                braced((
                    "break",
                    (PRIVATE, S, OUT, S, b.variant.as_ref(), parens("out")),
                    ';',
                )),
            );
        })
    }

    fn out_branch<'a>(&'a self, b: &'a Branch) -> impl ToTokens + 'a {
        (
            (PRIVATE, S, OUT, S, b.variant.as_ref()),
            parens(&self.tokens[b.binding.clone()]),
            (tok::ROCKET, self.block(&b.block)),
        )
    }

    fn else_branch<'a>(&'a self, e: &'a Else) -> impl ToTokens + 'a {
        (
            (PRIVATE, S, OUT, S, "Disabled"),
            (tok::ROCKET, self.block(&e.block)),
        )
    }

    /// Render a parsed block.
    fn block<'a>(&'a self, block: &'a Block) -> impl ToTokens + 'a {
        from_fn(move |stream, span| match block {
            Block::Group(range) => {
                stream.tokens(span, &self.tokens[range.clone()]);
            }
            Block::Expr(range) => {
                let checkpoint = stream.checkpoint();
                stream.tokens(span, &self.tokens[range.clone()]);
                stream.group(span, Delimiter::Brace, checkpoint);
            }
        })
    }

    fn poll<'a>(&'a self, b: &'a Branch, unset: Option<&'a str>) -> impl ToTokens + 'a {
        let inner_poll = ("Future", S, "poll", parens(("__fut", ',', "cx")));

        (
            ("if", "let", tok::Poll::Ready("out")),
            '=',
            (
                self.support(),
                "poll_by_ref",
                parens((
                    '&',
                    PRIVATE,
                    S,
                    b.waker.as_ref(),
                    ',',
                    tok::piped("cx"),
                    inner_poll,
                )),
            ),
            braced((
                unset.map(|var| (var, '.', "set", parens(tok::Option::<()>::None), ';')),
                ("__select", '.', "clear", parens(b.index), ';'),
                from_fn(move |stream, span| {
                    stream.tokens(span, self.immediate_match(b));
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

        (PRIVATE, S, "WAKER", '.', "reset", parens(args), ';')
    }

    /// Expand a select which is awaited immediately.
    pub fn expand(self, stream: &mut TokenStream, span: Span) {
        let start = stream.checkpoint();

        let imports = (self.support(), braced(("Future", ',', "Pin", ',', "Poll")));

        stream.tokens(span, ("use", imports, ';'));
        stream.tokens(span, self.private_mod());

        let reset_base = self.conditions(stream, span);
        stream.tokens(span, self.futures());
        stream.tokens(span, self.reset(reset_base));

        let output_body = from_fn(|stream, span| {
            for b in &self.branches {
                stream.tokens(span, self.out_branch(b));
            }

            if let Some(e) = &self.else_branch {
                stream.tokens(span, self.else_branch(e));
            }

            let panic_ = (
                ("unreachable", '!'),
                parens(string("branch cannot be reached")),
            );

            stream.tokens(span, ("_", tok::ROCKET, braced(panic_)));
        });

        let select_decl = (
            ("let", "mut", "__select"),
            '=',
            (self.support(), "select", parens(('&', PRIVATE, S, "WAKER"))),
            ';',
        );

        let loop_decl = (
            "loop",
            braced((
                ("match", "__select", '.', "next", parens(()), '.', "await"),
                braced(self.matches()),
            )),
        );

        stream.tokens(
            span,
            (
                "match",
                braced((select_decl, loop_decl)),
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
