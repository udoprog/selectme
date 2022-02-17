use std::ops;

use proc_macro::{Delimiter, Span, TokenTree};

use crate::error::Error;
use crate::to_tokens::{bracketed, from_fn, parens, string, ToTokens};
use crate::tok::S;
use crate::token_stream::TokenStream;

#[derive(Default)]
pub struct TailState {
    pub block: Option<Span>,
    pub start: Option<Span>,
    pub end: Option<Span>,
    /// Indicates if last expression is a return.
    pub return_: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum EntryKind {
    Main,
    Test,
}

#[derive(Debug, Clone, Copy)]
pub enum SupportsThreading {
    Supported,
    NotSupported,
}

impl EntryKind {
    /// The name of the attribute used as the entry kind.
    pub fn name(&self) -> &str {
        match self {
            EntryKind::Main => "tokio::main",
            EntryKind::Test => "tokio::test",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RuntimeFlavor {
    CurrentThread,
    Threaded,
}

impl RuntimeFlavor {
    /// Parse a literal (as it appears in Rust code) as a runtime flavor. This
    /// means that it includes quotes.
    pub(crate) fn from_literal(s: &str) -> Result<RuntimeFlavor, &'static str> {
        match s {
            "\"current_thread\"" => Ok(RuntimeFlavor::CurrentThread),
            "\"multi_thread\"" => Ok(RuntimeFlavor::Threaded),
            "\"single_thread\"" => Err("the single threaded runtime flavor is called \"current_thread\""),
            "\"basic_scheduler\"" => Err("the \"basic_scheduler\" runtime flavor has been renamed to \"current_thread\""),
            "\"threaded_scheduler\"" => Err("the \"threaded_scheduler\" runtime flavor has been renamed to \"multi_thread\""),
            _ => Err("no such runtime flavor, the runtime flavors are: \"current_thread\", \"multi_thread\""),
        }
    }
}

/// The parsed arguments output.
#[derive(Debug)]
pub struct Config {
    pub(crate) supports_threading: SupportsThreading,
    /// The default runtime flavor to use if left unspecified.
    default_flavor: RuntimeFlavor,
    /// The runtime flavor to use.
    pub(crate) flavor: Option<(Span, RuntimeFlavor)>,
    /// The number of worker threads to configure.
    pub(crate) worker_threads: Option<TokenTree>,
    /// If the runtime should start paused.
    pub(crate) start_paused: Option<TokenTree>,
}

impl Config {
    pub fn new(kind: EntryKind, supports_threading: SupportsThreading) -> Self {
        Self {
            supports_threading,
            default_flavor: match (kind, supports_threading) {
                (EntryKind::Main, SupportsThreading::Supported) => RuntimeFlavor::Threaded,
                (EntryKind::Main, SupportsThreading::NotSupported) => RuntimeFlavor::CurrentThread,
                (EntryKind::Test, _) => RuntimeFlavor::CurrentThread,
            },
            flavor: None,
            worker_threads: None,
            start_paused: None,
        }
    }

    pub fn validate(&self, kind: EntryKind, errors: &mut Vec<Error>) {
        match (self.flavor(), &self.start_paused) {
            (RuntimeFlavor::Threaded, Some(tt)) => {
                if tt.to_string() == "true" {
                    errors.push(Error::new(tt.span(), format!("the `start_paused` option requires the \"current_thread\" runtime flavor. Use `#[{}(flavor = \"current_thread\")]`", kind.name())));
                }
            }
            _ => {}
        }

        match (self.flavor(), &self.worker_threads) {
            (RuntimeFlavor::CurrentThread, Some(tt)) => {
                errors.push(Error::new(tt.span(), format!("the `worker_threads` option requires the \"multi_thread\" runtime flavor. Use `#[{}(flavor = \"multi_thread\")]`", kind.name())));
            }
            _ => {}
        }
    }

    /// Get the runtime flavor to use.
    fn flavor(&self) -> RuntimeFlavor {
        match &self.flavor {
            Some((_, flavor)) => *flavor,
            None => self.default_flavor,
        }
    }
}

/// The parsed item output.
pub struct ItemOutput {
    tokens: Vec<TokenTree>,
    pub has_async: bool,
    signature: Option<ops::Range<usize>>,
    block: Option<ops::Range<usize>>,
    tail_state: TailState,
}

impl ItemOutput {
    pub(crate) fn new(
        tokens: Vec<TokenTree>,
        has_async: bool,
        signature: Option<ops::Range<usize>>,
        block: Option<ops::Range<usize>>,
        tail_state: TailState,
    ) -> Self {
        Self {
            tokens,
            has_async,
            signature,
            block,
            tail_state,
        }
    }

    /// Validate the parsed item.
    pub fn validate(&self, kind: EntryKind, errors: &mut Vec<Error>) {
        if !self.has_async {
            let span = self
                .signature
                .as_ref()
                .and_then(|s| self.tokens.get(s.clone()))
                .and_then(|t| t.first())
                .map(|tt| tt.span())
                .unwrap_or_else(Span::call_site);

            errors.push(Error::new(
                span,
                format!("functions marked with `#[{}]` must be `async`", kind.name()),
            ));
        }
    }

    pub fn block_spans(&self) -> (Span, Span) {
        let start = self
            .tail_state
            .start
            .or(self.tail_state.block)
            .unwrap_or_else(Span::call_site);
        let end = self
            .tail_state
            .end
            .or(self.tail_state.block)
            .unwrap_or_else(Span::call_site);
        (start, end)
    }

    /// Expand into a function item.
    pub fn expand_item(&self, kind: EntryKind, config: Config, start: Span) -> impl ToTokens + '_ {
        from_fn(move |s| {
            if let (Some(signature), Some(block)) = (self.signature.clone(), self.block.clone()) {
                let block_span = self.tail_state.block.unwrap_or_else(Span::call_site);

                s.write((
                    self.entry_kind_attribute(kind),
                    &self.tokens[signature],
                    group_with_span(
                        Delimiter::Brace,
                        self.item_body(config, block, start),
                        block_span,
                    ),
                ))
            } else {
                s.write(&self.tokens[..]);
            }
        })
    }

    /// Generate attribute associated with entry kind.
    fn entry_kind_attribute(&self, kind: EntryKind) -> impl ToTokens {
        from_fn(move |s| {
            if let EntryKind::Test = kind {
                s.write((
                    '#',
                    bracketed((S, "core", S, "prelude", S, "v1", S, "test")),
                ))
            }
        })
    }

    /// Expanded item body.
    fn item_body(
        &self,
        config: Config,
        block: ops::Range<usize>,
        start: Span,
    ) -> impl ToTokens + '_ {
        // NB: override the first generated part with the detected start span.
        let rt = ("tokio", S, "runtime", S, "Builder");

        let rt = from_fn(move |s| {
            s.write(rt);

            match config.flavor() {
                RuntimeFlavor::CurrentThread => {
                    s.write((S, "new_current_thread", parens(())));
                }
                RuntimeFlavor::Threaded => {
                    s.write((S, "new_multi_thread", parens(())));
                }
            }

            if let Some(start_paused) = config.start_paused {
                s.write(('.', "start_paused", parens(start_paused)));
            }

            if let Some(worker_threads) = config.worker_threads {
                s.write(('.', "worker_threads", parens(worker_threads)));
            }
        });

        let build = (
            (rt, '.', "enable_all", parens(()), '.', "build", parens(())),
            '.',
            "expect",
            parens(string("Failed building the Runtime")),
        );

        from_fn(move |s| {
            if self.tail_state.return_ {
                s.write((
                    with_span(("return", build, '.', "block_on"), start),
                    parens(("async", &self.tokens[block])),
                    ';',
                ));
            } else {
                s.write((
                    with_span((build, '.', "block_on"), start),
                    parens(("async", &self.tokens[block])),
                ));
            }
        })
    }
}

/// Insert the given tokens with a custom span.
pub fn with_span<T>(inner: T, span: Span) -> impl ToTokens
where
    T: ToTokens,
{
    WithSpan(inner, span)
}

struct WithSpan<T>(T, Span);

impl<T> ToTokens for WithSpan<T>
where
    T: ToTokens,
{
    fn to_tokens(self, stream: &mut TokenStream, _: Span) {
        self.0.to_tokens(stream, self.1);
    }
}

/// Construct a custom group  with a custom span that is not inherited by its
/// children.
pub fn group_with_span<T>(delimiter: Delimiter, inner: T, span: Span) -> impl ToTokens
where
    T: ToTokens,
{
    GroupWithSpan(delimiter, inner, span)
}

struct GroupWithSpan<T>(Delimiter, T, Span);

impl<T> ToTokens for GroupWithSpan<T>
where
    T: ToTokens,
{
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        let checkpoint = stream.checkpoint();
        self.1.to_tokens(stream, span);
        stream.group(self.2, self.0, checkpoint);
    }
}
