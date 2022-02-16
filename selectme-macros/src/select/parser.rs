use core::ops;

use proc_macro::{Delimiter, Ident, Span, TokenTree};

use crate::error::Error;
use crate::parsing::{BaseParser, Buf, Punct};
use crate::parsing::{COMMA, EQ, ROCKET};
use crate::select::output::{Output, SelectKind};

enum Segment {
    Branch(Branch),
    Else(Else),
}

pub enum Block {
    Group(ops::Range<usize>),
    Expr(ops::Range<usize>),
}

impl Block {
    /// Indicate if this block is an expression or not.
    pub fn is_expr(&self) -> bool {
        matches!(self, Block::Expr(..))
    }
}

/// A parser for the `select!` macro.
pub struct Parser<'a> {
    base: BaseParser<'a>,
    errors: Vec<Error>,
}

impl<'a> Parser<'a> {
    /// Construct a new parser around the given token stream.
    pub(crate) fn new(stream: proc_macro::TokenStream, buf: &'a mut Buf) -> Self {
        Self {
            base: BaseParser::new(stream, buf),
            errors: Vec::new(),
        }
    }

    /// Parse and produce the corresponding token stream.
    pub(crate) fn parse(mut self) -> Result<Output, Vec<Error>> {
        let mut branches = Vec::new();
        let mut else_branch = None;
        let mut index = 0;

        if let Err(span) = self.parse_until_reserved(COMMA) {
            self.errors.push(Error::new(span, "expected `,`"));
            return Err(self.errors);
        }

        let mut biased = false;
        let mut select_kind = None::<(Span, SelectKind)>;

        // Parse options.
        while matches!(self.base.peek2(), Some((TokenTree::Ident(..), TokenTree::Punct(p))) if p.as_char() == ';')
        {
            match self.base.bump() {
                Some(TokenTree::Ident(ident)) => match self.base.buf.display_as_str(&ident) {
                    "biased" => {
                        biased = true;
                    }
                    "static" => {
                        if let Some((span, _)) = &select_kind {
                            self.errors.push(Error::new(
                                ident.span(),
                                "option `static` should only be specified once",
                            ));

                            self.errors.push(Error::new(
                                span.clone(),
                                "option `static` previously specified here",
                            ));
                        } else {
                            select_kind = Some((ident.span(), SelectKind::StaticSelect));
                        }
                    }
                    other => {
                        self.errors.push(Error::new(
                            ident.span(),
                            format!("unsupported option `{}`", other),
                        ));
                    }
                },
                tt => {
                    let span = tt.map(|tt| tt.span()).unwrap_or_else(Span::call_site);
                    self.errors
                        .push(Error::new(span, "expected identifier as option"));
                }
            }

            let _ = self.base.bump();
        }

        let krate = 0..self.base.len();

        while self.base.nth(0).is_some() {
            let mut is_expr = false;

            if let Some(segment) = self.parse_segment(index) {
                match segment {
                    Segment::Branch(b) => {
                        is_expr = b.block.is_expr();
                        branches.push(b);
                    }
                    Segment::Else(e) => {
                        is_expr = e.block.is_expr();
                        else_branch = Some(e);
                    }
                }
            }

            if !self.base.skip_punct(COMMA) && is_expr {
                break;
            }

            index += 1;
        }

        if let Some(tt) = self.base.nth(0) {
            self.errors.push(Error::new(tt.span(), "trailing token"));
        }

        if !self.errors.is_empty() {
            return Err(self.errors);
        }

        if branches.is_empty() && else_branch.is_none() {
            self.errors.push(Error::new(Span::call_site(), "`select!` must not be empty, consider replacing with `future::pending::<()>().await` instead!"));
            return Err(self.errors);
        }

        Ok(Output::new(
            self.base.into_tokens(),
            krate,
            branches,
            else_branch,
            biased,
            select_kind
                .map(|(_, kind)| kind)
                .unwrap_or(SelectKind::Select),
        ))
    }

    /// Parse a condition up until the `=>` token. Implements basic error
    /// recovery by winding to a group (or a comma).
    fn parse_condition(&mut self, ident: Ident) -> Option<(usize, usize)> {
        let start = self.base.len();

        if let Err(span) = self.parse_until_reserved(ROCKET) {
            self.errors.push(Error::new(span, "expected `=>`"));
            self.recover_to_group();
            return None;
        }

        if start != self.base.len() {
            return Some((start, self.base.len()));
        }

        self.errors.push(Error::new(
            ident.span(),
            "expected expression following `if`",
        ));
        self.recover_to_group();
        return None;
    }

    fn parse_expr(
        &mut self,
        binding: usize,
    ) -> Option<(ops::Range<usize>, Option<ops::Range<usize>>)> {
        let start = self.base.len();

        loop {
            match self.base.peek_punct() {
                Some(p @ Punct { chars: ROCKET, .. }) => {
                    self.base.step(p.len());
                    return Some((start..self.base.len(), None));
                }
                Some(p @ Punct { chars: COMMA, .. }) => {
                    self.base.step(p.len());

                    let span = match self.base.bump() {
                        Some(TokenTree::Ident(ident))
                            if self.base.buf.display_as_str(&ident) == "if" =>
                        {
                            let (expr, len) = self.parse_condition(ident)?;
                            return Some((start..expr, Some(expr..len)));
                        }
                        Some(tt) => tt.span(),
                        None => Span::call_site(),
                    };

                    self.errors.push(Error::new(
                        span,
                        "expected `if` followed by branch condition",
                    ));
                    self.recover_to_group();
                    return None;
                }
                _ => {}
            }

            match self.base.bump() {
                Some(TokenTree::Ident(ident)) if self.base.buf.display_as_str(&ident) == "if" => {
                    let (expr, len) = self.parse_condition(ident)?;
                    return Some((start..expr, Some(expr..len)));
                }
                Some(tt) => {
                    self.base.push(tt);
                    continue;
                }
                _ => {}
            }

            let span = if let Some(tt) = self.base.get(binding..).and_then(|tree| tree.last()) {
                tt.span()
            } else {
                Span::call_site()
            };

            self.errors.push(Error::new(
                span,
                "expected branch expression followed by `=>`",
            ));
            return None;
        }
    }

    /// Unwind until we find a group and return a boolean indicating if the group was found.
    fn recover_to_group(&mut self) {
        while !matches!(self.base.nth(0), Some(TokenTree::Group(..)) | None) {
            let _ = self.base.bump();
        }

        let _ = self.base.bump();
    }

    /// Try to parse the `else` keyword and indicate if it was successful.
    fn try_parse_else(&mut self) -> bool {
        match self.base.nth(0) {
            Some(TokenTree::Ident(ident)) if ident.to_string() == "else" => {
                let _ = self.base.bump();
                true
            }
            _ => false,
        }
    }

    /// Test if the given punctuation is reserved.
    fn is_reserved_punct(&mut self, p: &Punct) -> bool {
        matches!(
            p,
            Punct {
                chars: ROCKET | COMMA | EQ,
                ..
            }
        )
    }

    /// Parse until the given token or EOF.
    fn parse_until_eof(&mut self, expected: [char; 2]) {
        loop {
            match self.base.peek_punct() {
                Some(p) if p.chars == expected => {
                    return;
                }
                _ => {}
            }

            if let Some(tt) = self.base.bump() {
                self.base.push(tt);
                continue;
            }

            return;
        }
    }

    /// Parse until we've found an '=' or another punctuation which causes us to
    /// error.
    fn parse_until_reserved(&mut self, expected: [char; 2]) -> Result<(), Span> {
        loop {
            match self.base.peek_punct() {
                Some(p) if p.chars == expected => {
                    self.base.step(p.len());
                    return Ok(());
                }
                Some(p) if self.is_reserved_punct(&p) => {
                    self.base.step(p.len());
                    return Err(p.span);
                }
                _ => {}
            }

            if let Some(tt) = self.base.bump() {
                if self
                    .base
                    .buf
                    .ident_matches(&tt, |ident| matches!(ident, "if" | "else"))
                {
                    return Err(tt.span());
                }

                self.base.push(tt);
                continue;
            }

            return Err(self
                .base
                .last()
                .map(|tt| tt.span())
                .unwrap_or_else(Span::call_site));
        }
    }

    /// Parse an else block.
    fn parse_else(&mut self) -> Option<Else> {
        let span = match self.base.peek_punct() {
            Some(p @ Punct { chars: ROCKET, .. }) => {
                self.base.step(p.len());
                let block = self.parse_block()?;
                return Some(Else { block });
            }
            Some(p) => p.span,
            _ => match self.base.nth(0) {
                Some(tt) => tt.span(),
                None => Span::call_site(),
            },
        };

        self.errors
            .push(Error::new(span, "expected `else` followed by `=>`"));
        self.recover_to_group();
        None
    }

    /// Parse the next block, if present.
    fn parse_segment(&mut self, index: usize) -> Option<Segment> {
        let start = self.base.len();

        if self.try_parse_else() {
            return Some(Segment::Else(self.parse_else()?));
        }

        let binding = match self.parse_until_reserved(EQ) {
            Ok(()) => start..self.base.len(),
            Err(span) => {
                self.errors
                    .push(Error::new(span, "binding must be followed by a `=`"));

                self.recover_to_group();
                return None;
            }
        };

        let (expr, condition) = self.parse_expr(binding.end)?;
        let block = self.parse_block()?;

        let condition = condition.map(|range| Condition {
            var: format!("__cond{}", index).into(),
            range,
        });

        let branch = Branch {
            index,
            binding,
            expr,
            block,
            waker: format!("WAKER{}", index).into(),
            generic: format!("T{}", index).into(),
            variant: format!("Branch{}", index).into(),
            condition,
        };

        Some(Segment::Branch(branch))
    }

    /// Parse the next group.
    fn parse_block(&mut self) -> Option<Block> {
        let start = self.base.len();

        match self.base.nth(0) {
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
                let tt = self.base.bump();
                self.base.extend(tt);
                Some(Block::Group(start..self.base.len()))
            }
            Some(..) => {
                // Either parse until a comma or an EOF.
                self.parse_until_eof(COMMA);
                Some(Block::Expr(start..self.base.len()))
            }
            None => {
                self.errors.push(Error::new(
                    Span::call_site(),
                    "expected braced group or expression followed by `,`",
                ));
                None
            }
        }
    }
}

/// A branch condition.
pub struct Condition {
    /// Condition variable.
    pub var: Box<str>,
    /// Token range of the condition.
    pub range: ops::Range<usize>,
}

/// A regular branch.
pub struct Branch {
    /// Branch index.
    pub index: usize,
    /// Range for the binding to use.
    pub binding: ops::Range<usize>,
    /// Range for the expression to be evaluated as a future.
    pub expr: ops::Range<usize>,
    /// Range for the branch.
    pub block: Block,
    /// The name of the child waker for this block.
    pub waker: Box<str>,
    /// The name of the generic used by the branch.
    pub generic: Box<str>,
    /// The name of the enum variant use by this branch.
    pub variant: Box<str>,
    /// Branch condition.
    pub condition: Option<Condition>,
}

/// Code for the else branch.
pub struct Else {
    /// Range for the branch.
    pub block: Block,
}
