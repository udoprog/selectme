use core::ops;

use proc_macro::TokenTree;

use crate::select::parser::{Block, Branch, Else};
use crate::to_tokens::{
    braced, bracketed, from_fn, group, parens, string, SpannedStream, ToTokens,
};
use crate::tok::{self, S};

/// Limit to the number of branches we support.
pub const BRANCH_LIMIT: usize = u128::BITS as usize;

/// The name of the output enum.
const OUT: &str = "Out";
/// The private module in use.
const PRIVATE: &str = "private";
// Note the lack of convoluted naming. These are not visible in the
// corresponding branch scopes.
const MAYBE_FUT: &str = "maybe_fut";
const FUT: &str = "fut";
const CX: &str = "cx";
const STATE: &str = "state";
const MASK: &str = "mask";

/// Expansion mode.
#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Default,
    Inline,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SelectKind {
    Select,
    StaticSelect,
}

/// The parsed output.
pub struct Output {
    tokens: Vec<TokenTree>,
    mode: Mode,
    krate: ops::Range<usize>,
    branches: Vec<Branch>,
    else_branch: Option<Else>,
    biased: bool,
    select_kind: SelectKind,
}

impl Output {
    /// Construct new output.
    pub(crate) fn new(
        tokens: Vec<TokenTree>,
        mode: Mode,
        krate: ops::Range<usize>,
        branches: Vec<Branch>,
        else_branch: Option<Else>,
        biased: bool,
        select_kind: SelectKind,
    ) -> Self {
        Self {
            tokens,
            mode,
            krate,
            branches,
            else_branch,
            biased,
            select_kind,
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

    fn state(&self) -> impl ToTokens + '_ {
        parens(from_fn(|s| {
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
        }))
    }

    /// Render the else branch.
    fn else_branch<'a>(&'a self, e: &'a Else) -> impl ToTokens + 'a {
        from_fn(move |s| match self.mode {
            Mode::Default => {
                s.write((PRIVATE, S, OUT, S, "Disabled"));
            }
            Mode::Inline => {
                s.write(self.block(&e.block));
            }
        })
    }

    fn allow_unreachable_code(&self) -> impl ToTokens {
        ('#', bracketed(("allow", parens("unreachable_code"))))
    }

    fn matches(&self) -> impl ToTokens + '_ {
        from_fn(move |s| {
            for b in &self.branches {
                // We need to allow unreachable cause the expression that
                // generates the index might not be expressed.
                s.write(self.allow_unreachable_code());
                s.write((b.index, tok::ROCKET));

                let fut = (
                    "unsafe",
                    braced((
                        ("Pin", S, "map_unchecked_mut"),
                        parens((STATE, ',', tok::piped("f"), '&', "mut", "f", '.', b.index)),
                    )),
                );

                if b.condition.is_some() {
                    let assign = ("let", "mut", MAYBE_FUT, '=', fut, ';');
                    let poll = self.poll_body(b, Some(MAYBE_FUT));

                    let poll = (
                        ("if", "let", tok::option_some(FUT), '='),
                        ("Option", S, "as_pin_mut"),
                        parens(tok::pin_as_mut(MAYBE_FUT)),
                        braced(poll),
                    );

                    s.write(braced((assign, poll)));
                } else {
                    let assign = ("let", FUT, '=', fut, ';');
                    let poll = self.poll_body(b, None);
                    s.write(braced((assign, poll)));
                }
            }

            if let Some(e) = &self.else_branch {
                let body = ("return", tok::poll_ready(self.else_branch(e)), ';');
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
    fn match_branch<'a>(&'a self, b: &'a Branch) -> impl ToTokens + 'a {
        from_fn(move |s| match self.mode {
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
    fn poll_body<'a>(&'a self, b: &'a Branch, unset: Option<&'a str>) -> impl ToTokens + 'a {
        let future_poll = ("Future", S, "poll", parens((FUT, ',', CX)));

        (
            ("if", "let", tok::poll_ready("out"), '='),
            future_poll,
            braced((
                unset.map(|var| (var, '.', "set", parens(tok::OPTION_NONE), ';')),
                // Unset the current branch in the mask, since it completed.
                (MASK, '.', "clear", parens(b.index), ';'),
                self.match_branch(b),
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

    /// The type required to fit the given number of branches.
    fn mask_type(&self) -> impl ToTokens {
        match usize::BITS - self.branches.len().saturating_sub(1).leading_zeros() {
            7 => "u128",
            6 => "u64",
            5 => "u32",
            4 => "u16",
            _ => "u8",
        }
    }

    /// Generates the expression that should initially be used as a mask. This
    /// ensures that disabled branches stay disabled even if woken up..
    fn mask_expr(&self, reset_base: usize) -> impl ToTokens + '_ {
        let mask_expr = from_fn(move |s| {
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
        });

        braced((
            ("let", MASK, ':', self.mask_type(), '=', mask_expr, ';'),
            MASK,
        ))
    }

    /// Generate bias.
    fn bias(&self) -> impl ToTokens + '_ {
        if self.biased {
            (self.support(), "unbiased", parens(()))
        } else {
            (self.support(), "random", parens(()))
        }
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
    fn poll_decl(&self, reset_base: usize) -> impl ToTokens + '_ {
        let match_body = ("match", "index", braced(self.matches()));
        let fallback = ("Poll", S, "Pending");

        let poll_body = (
            tok::piped((CX, ',', STATE, ',', MASK, ',', "index")),
            braced((match_body, fallback)),
        );

        (
            self.support(),
            match (self.mode, self.select_kind) {
                // Default mode doesn't require anything to be captured, since
                // the branches are evaluated outside of the poller
                // implementation. While this will probably be optimized out
                // *anyways* we can instead use a `static_select` ahead of time.
                (Mode::Default, _) | (_, SelectKind::StaticSelect) => "static_select",
                _ => "select",
            },
            parens((
                self.mask_expr(reset_base),
                ',',
                self.bias(),
                ',',
                self.state(),
                ',',
                poll_body,
            )),
        )
    }

    /// Expand a select which is awaited immediately.
    pub fn expand(self) -> impl ToTokens {
        from_fn(move |s| match self.mode {
            Mode::Default => {
                s.write(braced(from_fn(move |s| {
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

                    s.write((
                        "match",
                        (self.poll_decl(reset_base), '.', "await"),
                        braced(output_body),
                    ));
                })));
            }
            Mode::Inline => {
                s.write(braced(from_fn(move |s| {
                    s.write(self.imports());
                    let reset_base = self.conditions(s);
                    s.write(self.poll_decl(reset_base));
                })));
            }
        })
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
