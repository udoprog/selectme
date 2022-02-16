use proc_macro::{Delimiter, Group, Literal, Span, TokenTree};

use crate::entry::output::{
    Config, EntryKind, ItemOutput, RuntimeFlavor, SupportsThreading, TailState,
};
use crate::error::Error;
use crate::parsing::{BaseParser, Buf};
use crate::parsing::{Punct, COMMA, EQ};

/// A parser for the arguments provided to an entry macro.
pub struct ConfigParser<'a> {
    base: BaseParser<'a>,
    errors: &'a mut Vec<Error>,
}

impl<'a> ConfigParser<'a> {
    /// Construct a new parser around the given token stream.
    pub(crate) fn new(
        stream: proc_macro::TokenStream,
        buf: &'a mut Buf,
        errors: &'a mut Vec<Error>,
    ) -> Self {
        Self {
            base: BaseParser::new(stream, buf),
            errors,
        }
    }

    /// Parse and produce the corresponding token stream.
    pub(crate) fn parse(
        mut self,
        kind: EntryKind,
        supports_threading: SupportsThreading,
    ) -> Config {
        let mut config = Config::new(kind, supports_threading);

        while self.base.nth(0).is_some() {
            let _ = self.parse_option(&mut config);

            if !self.base.skip_punct(COMMA) {
                break;
            }
        }

        if let Some(tt) = self.base.nth(0) {
            self.errors.push(Error::new(tt.span(), "trailing token"));
        }

        config
    }

    fn parse_option(&mut self, config: &mut Config) -> Option<()> {
        match self.base.bump() {
            Some(TokenTree::Ident(ident)) => match self.base.buf.display_as_str(&ident) {
                "worker_threads" => {
                    self.parse_eq()?;
                    config.worker_threads = Some(self.parse_token()?);
                    Some(())
                }
                "start_paused" => {
                    self.parse_eq()?;
                    config.start_paused = Some(self.parse_token()?);
                    Some(())
                }
                "flavor" => {
                    self.parse_eq()?;
                    let literal = self.parse_literal()?;

                    let flavor =
                        match RuntimeFlavor::from_literal(self.base.buf.display_as_str(&literal)) {
                            Ok(flavor) => flavor,
                            Err(error) => {
                                self.errors.push(Error::new(literal.span(), error));
                                return None;
                            }
                        };

                    if matches!(
                        (flavor, config.supports_threading),
                        (RuntimeFlavor::Threaded, SupportsThreading::NotSupported)
                    ) {
                        self.errors.push(Error::new(
                            ident.span(),
                            "The runtime flavor `multi_thread` requires the `rt-multi-thread` feature",
                        ));
                    }

                    if let Some((existing, _)) = &config.flavor {
                        self.errors.push(Error::new(
                            ident.span(),
                            "`flavor` must only be defined once",
                        ));
                        self.errors
                            .push(Error::new(existing.clone(), "`flavor` first defined here"));
                    }

                    config.flavor = Some((ident.span(), flavor));
                    Some(())
                }
                "core_threads" => {
                    self.parse_eq()?;
                    self.parse_literal()?;
                    self.errors.push(Error::new(
                        ident.span(),
                        "Attribute `core_threads` is renamed to `worker_threads`",
                    ));
                    None
                }
                _ => {
                    self.errors.push(Error::new(ident.span(), "Unknown attribute specified; expected one of: `flavor`, `worker_threads`, `start_paused`"));
                    self.parse_eq()?;
                    self.parse_literal()?;
                    None
                }
            },
            tt => {
                let span = tt.map(|tt| tt.span()).unwrap_or_else(Span::call_site);
                self.errors.push(Error::new(span, "expected identifier"));
                None
            }
        }
    }

    /// Parse the next element as a literal value.
    fn parse_literal(&mut self) -> Option<Literal> {
        match self.base.bump() {
            Some(TokenTree::Literal(literal)) => Some(literal),
            tt => {
                let span = tt.map(|tt| tt.span()).unwrap_or_else(Span::call_site);
                self.errors.push(Error::new(span, "expected literal"));
                None
            }
        }
    }

    /// Parse a token.
    fn parse_token(&mut self) -> Option<TokenTree> {
        match self.base.bump() {
            Some(t) => Some(t),
            tt => {
                let span = tt.map(|tt| tt.span()).unwrap_or_else(Span::call_site);
                self.errors.push(Error::new(span, "expected token"));
                None
            }
        }
    }

    /// Parse the next element as an `=` punctuation.
    fn parse_eq(&mut self) -> Option<()> {
        match self.base.peek_punct()? {
            p @ Punct { chars: EQ, .. } => {
                self.base.step(p.len());
                Some(())
            }
            p => {
                self.errors.push(Error::new(p.span, "expected `=`"));
                None
            }
        }
    }
}

/// A parser for the item annotated with an entry macro.
pub struct ItemParser<'a> {
    base: BaseParser<'a>,
}

impl<'a> ItemParser<'a> {
    /// Construct a new parser around the given token stream.
    pub(crate) fn new(stream: proc_macro::TokenStream, buf: &'a mut Buf) -> Self {
        Self {
            base: BaseParser::new(stream, buf),
        }
    }

    /// Parse and produce the corresponding token stream.
    pub(crate) fn parse(mut self) -> ItemOutput {
        let start = self.base.len();
        let mut signature = None;
        let mut block = None;

        let mut has_async = false;

        let mut tail_state = TailState::default();

        while let Some(tt) = self.base.bump() {
            match tt {
                TokenTree::Ident(ident) if self.base.buf.display_as_str(&ident) == "async" => {
                    // NB: intentionally skip over this token.
                    has_async = true;
                }
                TokenTree::Group(g) if matches!(g.delimiter(), Delimiter::Brace) => {
                    signature = Some(start..self.base.len());
                    let start = self.base.len();
                    tail_state.block = Some(g.span());
                    self.find_last_stmt_range(&g, &mut tail_state);
                    self.base.push(TokenTree::Group(g));
                    block = Some(start..self.base.len());
                }
                tt => {
                    self.base.push(tt);
                }
            }
        }

        let tokens = self.base.into_tokens();

        ItemOutput::new(tokens, has_async, signature, block, tail_state)
    }

    /// Find the range of spans that is defined by the last statement in the
    /// block so that they can be used for the generated expression.
    ///
    /// This in turn improves upon diagnostics when return types do not match.
    #[cfg(feature = "tokio-diagnostics")]
    pub fn find_last_stmt_range(&mut self, g: &Group, tail_state: &mut TailState) {
        let mut update = true;

        for tt in g.stream() {
            let span = tt.span();
            tail_state.end = Some(span);

            match tt {
                TokenTree::Punct(p) if p.as_char() == ';' => {
                    update = true;
                }
                tt => {
                    if std::mem::take(&mut update) {
                        tail_state.return_ = matches!(&tt, TokenTree::Ident(ident) if self.base.buf.display_as_str(ident) == "return");
                        tail_state.start = Some(span);
                    }
                }
            }
        }
    }

    #[cfg(not(feature = "tokio-diagnostics"))]
    pub fn find_last_stmt_range(&self, _: &Group, _: &mut TailState) {}
}
