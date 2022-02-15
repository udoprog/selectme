use core::ops;

use proc_macro::TokenTree;

use crate::select::parser::{Block, Branch, Else};
use crate::to_tokens::{
    braced, bracketed, from_fn, group, parens, string, SpannedStream, ToTokens,
};
use crate::tok::{self, S};

/// The name of the output enum.
const OUT: &str = "Out";
/// The private module in use.
const PRIVATE: &str = "__private";

/// Expansion mode.
#[derive(Debug, Clone, Copy)]
enum Mode {
    Default,
    Inline,
}

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
        let private_mod = braced(self.out_enum());
        ("mod", PRIVATE, private_mod)
    }

    fn futures(&self) -> impl ToTokens + '_ {
        let init = from_fn(|s| {
            for b in &self.branches {
                if let Some(c) = &b.condition {
                    s.write(tok::if_else(
                        c.var.as_ref(),
                        tok::option_some(&self.tokens[b.expr.clone()]),
                        tok::OPTION_NONE,
                    ));
                } else {
                    s.write(&self.tokens[b.expr.clone()]);
                }

                s.write(',');
            }
        });

        ("let", "mut", "__fut", '=', parens(init))
    }

    /// Render the else branch.
    fn else_branch<'a>(&'a self, mode: Mode, e: &'a Else) -> impl ToTokens + 'a {
        from_fn(move |s| match mode {
            Mode::Default => {
                s.write((PRIVATE, S, OUT, S, "Disabled"));
            }
            Mode::Inline => {
                s.write(self.block(&e.block));
            }
        })
    }

    fn matches(&self, mode: Mode) -> impl ToTokens + '_ {
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
                    let poll = self.poll_body(b, Some(b.pin.as_ref()), mode);

                    let poll = (
                        ("if", "let", tok::option_some("__fut"), '='),
                        ("Option", S, "as_pin_mut"),
                        parens(tok::pin_as_mut(b.pin.as_ref())),
                        braced(poll),
                    );

                    s.write(braced((assign, poll)));
                } else {
                    let assign = ("let", "__fut", '=', fut, ';');
                    let poll = self.poll_body(b, None, mode);
                    s.write(braced((assign, poll)));
                }
            }

            if let Some(e) = &self.else_branch {
                let body = ("return", tok::poll_ready(self.else_branch(mode, e)), ';');
                s.write(((self.support(), "DISABLED"), tok::ROCKET, braced(body)));
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
    fn match_branch<'a>(&'a self, b: &'a Branch, mode: Mode) -> impl ToTokens + 'a {
        from_fn(move |s| match mode {
            Mode::Default => {
                let pat = clean_pattern(self.tokens[b.binding.clone()].iter().cloned());

                let body = ((PRIVATE, S, OUT, S, b.variant.as_ref()), parens("out"));

                s.write((
                    ('#', bracketed(("allow", parens("unused_variables")))),
                    (
                        "if",
                        "let",
                        pat,
                        '=',
                        '&',
                        "out",
                        braced(("return", tok::poll_ready(body), ';')),
                    ),
                ));
            }
            Mode::Inline => {
                let pat = &self.tokens[b.binding.clone()];
                s.write((
                    "if",
                    "let",
                    pat,
                    '=',
                    "out",
                    braced(("return", tok::poll_ready(self.block(&b.block)), ';')),
                ));
            }
        })
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
    fn out_else<'a>(&'a self, e: &'a Else) -> impl ToTokens + 'a {
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

    /// Expand the poll expression.
    fn poll_body<'a>(
        &'a self,
        b: &'a Branch,
        unset: Option<&'a str>,
        mode: Mode,
    ) -> impl ToTokens + 'a {
        let future_poll = ("Future", S, "poll", parens(("__fut", ',', "cx")));

        (
            ("if", "let", tok::poll_ready("out"), '='),
            future_poll,
            braced((
                unset.map(|var| (var, '.', "set", parens(tok::OPTION_NONE), ';')),
                // Unset the current branch in the mask, since it completed.
                ("mask", '.', "clear", parens(b.index), ';'),
                self.match_branch(b, mode),
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

    /// Generate imports.
    fn imports(&self) -> impl ToTokens + '_ {
        (
            "use",
            self.support(),
            braced(("Future", ',', "Pin", ',', "Poll")),
            ';',
        )
    }

    /// Setup the poll declaration.
    fn poll_decl(&self, reset_base: usize, mode: Mode) -> impl ToTokens + '_ {
        let match_body = ("match", "index", braced(self.matches(mode)));
        let fallback = ("Poll", S, "Pending");

        let poll_body = (
            tok::piped(("cx", ',', "mask", ',', "index")),
            braced((match_body, fallback)),
        );

        (
            self.support(),
            "poll_fn",
            parens((self.mask_expr(reset_base), ',', poll_body)),
            ('.', "await"),
        )
    }

    /// Expand a select which is awaited immediately.
    pub fn expand(self) -> impl ToTokens {
        braced(from_fn(move |s| {
            s.write(self.imports());
            s.write(self.private_mod());

            let reset_base = self.conditions(s);

            let output_body = from_fn(|s| {
                for b in &self.branches {
                    s.write(self.out_branch(b));
                }

                if let Some(e) = &self.else_branch {
                    s.write(self.out_else(e));
                }

                let panic_ = (
                    ("unreachable", '!'),
                    parens(string("branch cannot be reached")),
                );

                s.write(("_", tok::ROCKET, braced(panic_)));
            });

            let futures_decl = (self.futures(), ';');

            s.write((
                "match",
                braced((futures_decl, self.poll_decl(reset_base, Mode::Default))),
                braced(output_body),
            ));
        }))
    }

    /// Expand a select which is awaited immediately.
    pub fn expand_inline(self) -> impl ToTokens {
        braced(from_fn(move |s| {
            s.write(self.imports());
            s.write(self.private_mod());

            let reset_base = self.conditions(s);
            let futures_decl = (self.futures(), ';');

            s.write(braced((
                futures_decl,
                self.poll_decl(reset_base, Mode::Inline),
            )));
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