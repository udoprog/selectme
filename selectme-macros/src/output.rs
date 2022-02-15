use std::ops;

use proc_macro::TokenTree;

use crate::parser::{Block, Branch, Else};
use crate::to_tokens::{
    braced, bracketed, from_fn, group, parens, string, SpannedStream, ToTokens,
};
use crate::tok::{self, S};

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
            braced(from_fn(|s| {
                for b in &self.branches {
                    s.write((b.variant.as_ref(), parens(b.generic.as_ref()), ','));
                }

                if self.else_branch.is_some() {
                    s.write(("Disabled", ','));
                }
            })),
        )
    }

    /// Private module declaration.
    fn private_mod(&self) -> impl ToTokens + '_ {
        let poller = (self.support(), "PollerWaker");
        let static_ = (self.support(), "StaticWaker");

        let private_mod = braced(from_fn(move |s| {
            s.write((
                ("pub", "static", "WAKER", ':', static_, '='),
                (static_, S, "new", parens(()), ';'),
            ));

            for b in &self.branches {
                s.write((
                    ("pub", "static", b.waker.as_ref(), ':', poller, '='),
                    (poller, S, "new", parens(('&', "WAKER", ',', b.index)), ';'),
                ));
            }

            s.write(self.out_enum());
        }));

        ("mod", PRIVATE, private_mod)
    }

    fn futures(&self) -> impl ToTokens + '_ {
        let init = from_fn(|s| {
            for b in &self.branches {
                if let Some(c) = &b.condition {
                    s.write(tok::if_else(
                        c.var.as_ref(),
                        tok::Option::Some(&self.tokens[b.expr.clone()]),
                        tok::Option::<()>::None,
                    ));
                } else {
                    s.write(&self.tokens[b.expr.clone()]);
                }

                s.write(',');
            }
        });

        ("let", "mut", "__fut", '=', parens(init))
    }

    fn matches(&self) -> impl ToTokens + '_ {
        from_fn(move |s| {
            for b in &self.branches {
                s.write((b.index, tok::ROCKET));

                let fut = (
                    "unsafe",
                    braced((
                        ("Pin", S, "new_unchecked"),
                        parens(('&', "mut", "__fut", '.', b.index)),
                    )),
                );

                if b.condition.is_some() {
                    let assign = ("let", "mut", b.pin.as_ref(), '=', fut, ';');
                    let poll = self.poll_body(b, Some(b.pin.as_ref()));

                    let poll = (
                        ("if", "let", tok::Option::Some("__fut"), '='),
                        ("Option", S, "as_pin_mut"),
                        parens(tok::as_mut(b.pin.as_ref())),
                        braced(poll),
                    );

                    s.write(braced((assign, poll)));
                } else {
                    let assign = ("let", "__fut", '=', fut, ';');
                    let poll = self.poll_body(b, None);
                    s.write(braced((assign, poll)));
                }
            }

            if let Some(e) = &self.else_branch {
                if self.else_branch.is_some() {
                    let body = ("break", (PRIVATE, S, OUT, S, "Disabled"), ';');
                    s.write(((self.support(), "DISABLED"), tok::ROCKET, braced(body)));
                } else {
                    let body = ("break", self.block(&e.block), ';');
                    s.write(((self.support(), "DISABLED"), tok::ROCKET, braced(body)));
                }
            }

            let panic_branch = (
                ("panic", '!'),
                parens((string("no branch with index `{}`"), ',', "n")),
            );

            s.write(("n", tok::ROCKET, braced(panic_branch)));
        })
    }

    /// Generate the immediate match which performs a borrowing match over the
    /// pattern supplied by the user to determine whether we should break out of
    /// the loop with a value or not.
    fn immediate_match<'a>(&'a self, b: &'a Branch) -> impl ToTokens + 'a {
        let pat = clean_pattern(self.tokens[b.binding.clone()].iter().cloned());

        let body = (
            "break",
            (PRIVATE, S, OUT, S, b.variant.as_ref()),
            parens("out"),
        );

        (
            ('#', bracketed(("allow", parens("unused_variables")))),
            ("if", "let", pat, '=', '&', "out", braced((body, ';'))),
        )
    }

    /// Generate the output matching branch.
    fn out_branch<'a>(&'a self, b: &'a Branch) -> impl ToTokens + 'a {
        (
            (PRIVATE, S, OUT, S, b.variant.as_ref()),
            parens(&self.tokens[b.binding.clone()]),
            (tok::ROCKET, self.block(&b.block)),
        )
    }

    /// Generate the output matching "else" branch.
    fn else_branch<'a>(&'a self, e: &'a Else) -> impl ToTokens + 'a {
        (
            (PRIVATE, S, OUT, S, "Disabled"),
            (tok::ROCKET, self.block(&e.block)),
        )
    }

    /// Render a parsed block.
    fn block<'a>(&'a self, block: &'a Block) -> impl ToTokens + 'a {
        from_fn(move |s| match block {
            Block::Group(range) => {
                s.write(&self.tokens[range.clone()]);
            }
            Block::Expr(range) => {
                s.write(braced(&self.tokens[range.clone()]));
            }
        })
    }

    fn poll_body<'a>(&'a self, b: &'a Branch, unset: Option<&'a str>) -> impl ToTokens + 'a {
        let future_poll = ("Future", S, "poll", parens(("__fut", ',', "cx")));

        (
            ("if", "let", tok::Poll::Ready("out")),
            '=',
            (
                (self.support(), "poll_by_ref"),
                parens((
                    ('&', PRIVATE, S, b.waker.as_ref(), ','),
                    (tok::piped("cx"), future_poll),
                )),
            ),
            braced((
                unset.map(|var| (var, '.', "set", parens(tok::Option::<()>::None), ';')),
                ("__select", '.', "clear", parens(b.index), ';'),
                self.immediate_match(b),
            )),
        )
    }

    fn conditions(&self, s: &mut SpannedStream<'_>) -> usize {
        let mut reset_base = 0;

        for b in &self.branches {
            match &b.condition {
                Some(c) => {
                    s.write((
                        "let",
                        c.var.as_ref(),
                        '=',
                        &self.tokens[c.range.clone()],
                        ';',
                    ));
                }
                None => {
                    reset_base += 1 << b.index;
                }
            }
        }

        reset_base
    }

    /// Generates the expression that should initially be used as a mask. This
    /// ensures that disabled branches stay disabled even if woken up..
    fn mask_expr(&self, reset_base: usize) -> impl ToTokens + '_ {
        from_fn(move |s| {
            let mut it = self
                .branches
                .iter()
                .filter_map(|b| b.condition.as_ref().map(|c| (b, c.var.as_ref())));

            if let Some((b, var)) = it.next_back() {
                if reset_base != 0 {
                    s.write((reset_base, '+'));
                }

                for (b, var) in it {
                    s.write((tok::if_else(var, 1 << b.index, 0), '+'));
                }

                s.write(tok::if_else(var, 1 << b.index, 0));
            } else {
                s.write(reset_base);
            }
        })
    }

    /// Expand a select which is awaited immediately.
    pub fn expand(self) -> impl ToTokens {
        braced(from_fn(move |s| {
            let imports = (self.support(), braced(("Future", ',', "Pin", ',', "Poll")));

            s.write(("use", imports, ';'));
            s.write(self.private_mod());

            let reset_base = self.conditions(s);

            let output_body = from_fn(|s| {
                for b in &self.branches {
                    s.write(self.out_branch(b));
                }

                if let Some(e) = &self.else_branch {
                    s.write(self.else_branch(e));
                }

                let panic_ = (
                    ("unreachable", '!'),
                    parens(string("branch cannot be reached")),
                );

                s.write(("_", tok::ROCKET, braced(panic_)));
            });

            let futures_decl = (self.futures(), ';');

            let select_decl = (
                ("let", "mut", "__select", '=', "unsafe"),
                braced((
                    (self.support(), "poller"),
                    parens((('&', PRIVATE, S, "WAKER"), ',', self.mask_expr(reset_base))),
                )),
                ';',
            );

            let loop_decl = (
                "loop",
                braced((
                    ("match", "__select", '.', "next", parens(()), '.', "await"),
                    braced(self.matches()),
                )),
            );

            s.write((
                "match",
                braced((futures_decl, select_decl, loop_decl)),
                braced(output_body),
            ));
        }))
    }
}

/// Clean up a pattern by skipping over any `mut` and `&` tokens.
fn clean_pattern(tree: impl Iterator<Item = TokenTree>) -> impl ToTokens {
    from_fn(move |s| {
        for tt in tree {
            match tt {
                TokenTree::Group(g) => {
                    s.write(group(g.delimiter(), clean_pattern(g.stream().into_iter())));
                }
                TokenTree::Ident(i) => {
                    if i.to_string() == "mut" {
                        continue;
                    }

                    s.push(TokenTree::Ident(i));
                }
                TokenTree::Punct(p) => {
                    if p.as_char() == '&' {
                        continue;
                    }

                    s.push(TokenTree::Punct(p));
                }
                tt => {
                    s.push(tt);
                }
            }
        }
    })
}

fn branch_generics(branches: &[Branch]) -> impl ToTokens + '_ {
    from_fn(move |s| {
        s.write('<');

        for b in branches {
            s.write((b.generic.as_ref(), ','));
        }

        s.write('>');
    })
}
