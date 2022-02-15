use core::ops;

use proc_macro::{Delimiter, Ident, Spacing, Span, TokenTree};

use crate::error::Error;
use crate::parsing;
use crate::select::output::Output;

enum Segment {
    Branch(Branch),
    Else(Else),
}

pub enum Block {
    Group(ops::Range<usize>),
    Expr(ops::Range<usize>),
}

// Punctuations that we look for.
const COMMA: [char; 2] = [',', '\0'];
const EQ: [char; 2] = ['=', '\0'];
const ROCKET: [char; 2] = ['=', '>'];

/// A parser for the `select!` macro.
pub struct Parser {
    base: parsing::Parser,
    errors: Vec<Error>,
    // Re-usable string buffer.
    string_buf: String,
}

impl Parser {
    /// Construct a new parser around the given token stream.
    pub(crate) fn new(stream: proc_macro::TokenStream) -> Self {
        Self {
            base: parsing::Parser::new(stream),
            errors: Vec::new(),
            string_buf: String::new(),
        }
    }

    /// Parse and produce the corresponding token stream.
    pub(crate) fn parse(mut self) -> Result<Output, Vec<Error>> {
        let mut branches = Vec::new();
        let mut else_branch = None;
        let mut n = 0;

        if let Err(span) = self.parse_until_reserved(COMMA) {
            self.errors.push(Error::new(span, "expected `,`"));
            return Err(self.errors);
        }

        let krate = 0..self.base.len();

        while self.base.nth(0).is_some() {
            if let Some(segment) = self.parse_segment(n) {
                match segment {
                    Segment::Branch(b) => {
                        branches.push(b);
                    }
                    Segment::Else(e) => {
                        else_branch = Some(e);
                    }
                }
            }

            self.skip_punct(COMMA);
            n += 1;
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
        ))
    }

    /// Skip one of the specified punctuations.
    fn skip_punct(&mut self, expected: [char; 2]) {
        if let Some(p) = self.peek_punct() {
            if p.chars == expected {
                self.base.step(p.len());
            }
        }
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
            match self.peek_punct() {
                Some(p @ Punct { chars: ROCKET, .. }) => {
                    self.base.step(p.len());
                    return Some((start..self.base.len(), None));
                }
                Some(p @ Punct { chars: COMMA, .. }) => {
                    self.base.step(p.len());

                    let span = match self.base.bump() {
                        Some(TokenTree::Ident(ident)) if self.is_ident(&ident, "if") => {
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
                Some(TokenTree::Ident(ident)) if self.is_ident(&ident, "if") => {
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

    /// Test if the given ident matches the condition.
    fn is_ident(&mut self, ident: &Ident, m: &str) -> bool {
        use std::fmt::Write;

        self.string_buf.clear();
        let _ = write!(&mut self.string_buf, "{}", ident);
        self.string_buf.as_str() == m
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

    /// Test if the given [TokenTree] represented a reserved identifier.
    fn is_reserved_ident(&mut self, tt: &TokenTree) -> bool {
        use std::fmt::Write;

        if let TokenTree::Ident(ident) = tt {
            self.string_buf.clear();
            let _ = write!(&mut self.string_buf, "{}", ident);
            return matches!(self.string_buf.as_str(), "if" | "else");
        }

        false
    }

    /// Parse until the given token or EOF.
    fn parse_until_eof(&mut self, expected: [char; 2]) {
        loop {
            match self.peek_punct() {
                Some(p) if p.chars == expected => {
                    self.base.step(p.len());
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
            match self.peek_punct() {
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
                if self.is_reserved_ident(&tt) {
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
        let span = match self.peek_punct() {
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

    /// Process a punctuation.
    fn peek_punct(&mut self) -> Option<Punct> {
        let mut out = [None; 2];

        for (n, o) in out.iter_mut().enumerate() {
            match (n, self.base.nth(n)) {
                (_, Some(TokenTree::Punct(punct))) => {
                    *o = Some((punct.span(), punct.as_char()));

                    if !matches!(punct.spacing(), Spacing::Joint) {
                        break;
                    }
                }
                _ => {
                    break;
                }
            }
        }

        match out {
            [Some((span, head)), tail] => Some(Punct {
                span,
                chars: [head, tail.map(|(_, c)| c).unwrap_or('\0')],
            }),
            _ => None,
        }
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

/// A complete punctuation.
#[derive(Debug)]
struct Punct {
    span: Span,
    chars: [char; 2],
}

impl Punct {
    fn len(&self) -> usize {
        self.chars.iter().take_while(|c| **c != '\0').count()
    }
}
