use std::collections::VecDeque;
use std::ops;

use proc_macro::{Delimiter, Ident, Spacing, Span, TokenTree};

use crate::error::Error;
use crate::output::{Output, Prefix};

// Punctuations that we look for.
const COMMA: [char; 2] = [',', '\0'];
const EQ: [char; 2] = ['=', '\0'];
const ROCKET: [char; 2] = ['=', '>'];

/// A parser for the `select!` macro.
pub struct Parser {
    it: proc_macro::token_stream::IntoIter,
    buf: VecDeque<TokenTree>,
    tokens: Vec<TokenTree>,
    errors: Vec<Error>,
}

impl Parser {
    /// Construct a new parser around the given token stream.
    pub(crate) fn new(stream: proc_macro::TokenStream) -> Self {
        Self {
            it: stream.into_iter(),
            buf: VecDeque::new(),
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Parse and produce the corresponding token stream.
    pub(crate) fn parse(mut self) -> Result<Output, Vec<Error>> {
        let mut branches = Vec::new();
        let mut n = 0;

        while self.nth(0).is_some() {
            if let Some(b) = self.parse_segment(n) {
                branches.push(b);
            }

            self.skip_punct(COMMA);
            n += 1;
        }

        if !self.errors.is_empty() {
            return Err(self.errors);
        }

        Ok(Output::new(self.tokens, branches, Prefix::SelectMe))
    }

    /// Skip one of the specified punctuations.
    fn skip_punct(&mut self, expected: [char; 2]) {
        if let Some(p) = self.peek_punct() {
            if p.chars == expected {
                self.step(p.len());
            }
        }
    }

    /// Parse until the given punctuation.
    fn parse_until(&mut self, expected: [char; 2]) -> bool {
        loop {
            match self.peek_punct() {
                Some(p) if p.chars == expected => {
                    self.step(p.len());
                    return true;
                }
                _ => {}
            }

            if let Some(tt) = self.bump() {
                self.tokens.push(tt);
                continue;
            }

            return false;
        }
    }

    /// Parse a condition up until the `=>` token. Implements basic error
    /// recovery by winding to a group (or a comma).
    fn parse_condition(&mut self, ident: Ident) -> Option<(usize, usize)> {
        let start = self.tokens.len();

        if self.parse_until(ROCKET) {
            if start != self.tokens.len() {
                return Some((start, self.tokens.len()));
            }
        }

        let end = self.recover_failed_condition(ident.span())?;
        Some((start, end))
    }

    fn parse_expr(
        &mut self,
        binding: usize,
    ) -> Option<(ops::Range<usize>, Option<ops::Range<usize>>)> {
        let start = self.tokens.len();

        loop {
            match self.peek_punct() {
                Some(p @ Punct { chars: ROCKET, .. }) => {
                    self.step(p.len());
                    return Some((start..self.tokens.len(), None));
                }
                #[cfg(feature = "tokio-compat")]
                Some(p @ Punct { chars: COMMA, .. }) => {
                    self.step(p.len());

                    let (expr, len) = match self.bump() {
                        Some(TokenTree::Ident(ident)) if ident.to_string() == "if" => {
                            self.parse_condition(ident)?
                        }
                        _ => (self.tokens.len(), self.recover_failed_condition(p.span)?),
                    };

                    return Some((start..expr, Some(expr..len)));
                }
                _ => {}
            }

            match self.bump() {
                Some(TokenTree::Ident(ident)) if ident.to_string() == "if" => {
                    let (expr, len) = self.parse_condition(ident)?;
                    return Some((start..expr, Some(expr..len)));
                }
                Some(tt) => {
                    self.tokens.push(tt);
                    continue;
                }
                _ => {}
            }

            let span = if let Some(tt) = self.tokens.get(binding..).and_then(|tree| tree.last()) {
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

    /// Consume until we've reached the end or encountered a braced block.
    fn recover_failed_condition(&mut self, span: Span) -> Option<usize> {
        loop {
            match self.nth(0) {
                Some(TokenTree::Group(..)) => {
                    self.errors.push(Error::new(
                        span,
                        "expected condition expression followed by `=>`",
                    ));
                    return Some(self.tokens.len());
                }
                Some(..) => match self.peek_punct() {
                    Some(p) => match p.chars {
                        ROCKET => {
                            self.step(p.len());
                            self.errors
                                .push(Error::new(span, "expected condition expression"));
                            return Some(self.tokens.len());
                        }
                        _ => {}
                    },
                    _ => {}
                },
                None => {
                    self.errors.push(Error::new(
                        span,
                        "expected condition expression (unexpected end)",
                    ));
                    return None;
                }
            }

            let _ = self.bump();
        }
    }

    /// Parse the next block, if present.
    fn parse_segment(&mut self, index: usize) -> Option<Branch> {
        let start = self.tokens.len();

        let binding = if self.parse_until(EQ) {
            start..self.tokens.len()
        } else {
            let span = self
                .tokens
                .last()
                .map(|tt| tt.span())
                .unwrap_or_else(Span::call_site);
            self.errors
                .push(Error::new(span, "expected binding followed by `=`"));
            return None;
        };

        let (expr, condition) = self.parse_expr(binding.end)?;
        let branch = self.parse_branch()?;

        let condition = condition.map(|range| Condition {
            var: format!("__cond{}", index).into(),
            range,
        });
        let branch = Branch {
            index,
            fuse: true,
            binding,
            expr,
            branch,
            waker: format!("WAKER{}", index).into(),
            pin: format!("__fut{}", index).into(),
            condition,
        };

        Some(branch)
    }

    /// Process a punctuation.
    fn peek_punct(&mut self) -> Option<Punct> {
        let mut out = [None; 2];

        for (n, o) in out.iter_mut().enumerate() {
            match (n, self.nth(n)) {
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
    fn parse_branch(&mut self) -> Option<ops::Range<usize>> {
        let start = self.tokens.len();

        let span = match self.bump() {
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
                self.tokens.push(TokenTree::Group(group));
                return Some(start..self.tokens.len());
            }
            Some(tt) => tt.span(),
            None => Span::call_site(),
        };

        self.errors.push(Error::new(span, "expected braced group"));
        None
    }

    /// Access the token at the given offset.
    fn nth(&mut self, n: usize) -> Option<&TokenTree> {
        while self.buf.len() <= n {
            self.buf.push_back(self.it.next()?);
        }

        self.buf.get(n)
    }

    /// Bump the last token.
    fn bump(&mut self) -> Option<TokenTree> {
        if let Some(head) = self.buf.pop_front() {
            return Some(head);
        }

        self.it.next()
    }

    /// Step over the given number of tokens.
    fn step(&mut self, n: usize) {
        for _ in 0..n {
            self.bump();
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

pub struct Branch {
    /// Branch index.
    pub index: usize,
    /// If the branch should automatically fuse.
    pub fuse: bool,
    pub binding: ops::Range<usize>,
    pub expr: ops::Range<usize>,
    pub branch: ops::Range<usize>,
    /// The name of the child waker for this block.
    pub waker: Box<str>,
    /// The name of the pin variable.
    pub pin: Box<str>,
    /// Branch condition.
    pub condition: Option<Condition>,
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
