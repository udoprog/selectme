use std::ops;

use proc_macro::{Delimiter, Span, TokenTree};

use crate::error::Error;
use crate::into_tokens::{bracketed, from_fn, parens, string, IntoTokens};
use crate::tok::S;
use crate::token_stream::TokenStream;

#[derive(Debug, Clone, Copy)]
pub(crate) enum EntryKind {
    Main,
    Test,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SupportsThreading {
    Supported,
    NotSupported,
}

impl EntryKind {
    /// The name of the attribute used as the entry kind.
    pub(crate) fn name(&self) -> &str {
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
pub(crate) struct Config {
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
    pub(crate) fn new(kind: EntryKind, supports_threading: SupportsThreading) -> Self {
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

    pub(crate) fn validate(&self, kind: EntryKind, errors: &mut Vec<Error>) {
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
pub(crate) struct ItemOutput {
    tokens: Vec<TokenTree>,
    async_keyword: Option<usize>,
    fn_name: Option<usize>,
    signature: Option<ops::Range<usize>>,
    block: Option<usize>,
}

impl ItemOutput {
    pub(crate) fn new(
        tokens: Vec<TokenTree>,
        async_keyword: Option<usize>,
        fn_name: Option<usize>,
        signature: Option<ops::Range<usize>>,
        block: Option<usize>,
    ) -> Self {
        Self {
            tokens,
            async_keyword,
            fn_name,
            signature,
            block,
        }
    }

    /// Validate the parsed item.
    pub(crate) fn validate(&self, kind: EntryKind, errors: &mut Vec<Error>) {
        if self.async_keyword.is_none() {
            let span = self
                .signature
                .as_ref()
                .and_then(|s| self.tokens.get(s.clone())?.first())
                .map(|tt| tt.span())
                .unwrap_or_else(Span::call_site);

            errors.push(Error::new(
                span,
                format!("functions marked with `#[{}]` must be `async`", kind.name()),
            ));
        }
    }

    pub(crate) fn block_span(&self) -> Option<Span> {
        let block = *self.block.as_ref()?;
        Some(self.tokens.get(block)?.span())
    }

    /// Expand into a function item.
    pub(crate) fn expand_item(self, kind: EntryKind, config: Config) -> impl IntoTokens {
        from_fn(move |s| {
            if let Some(item) = self.expand_if_present(kind, config) {
                s.write(item);
            } else if let Some(index) = self.async_keyword {
                s.write(expand_without_index(&self.tokens[..], index));
            } else {
                s.write(&self.tokens[..]);
            }
        })
    }

    /// Expands the function item if all prerequisites are present.
    fn expand_if_present(&self, kind: EntryKind, config: Config) -> Option<impl IntoTokens + '_> {
        let signature = self.signature.as_ref()?.clone();
        let signature = self.tokens.get(signature)?;
        let block = self.tokens.get(self.block?)?;
        let fn_name = self.tokens.get(self.fn_name?)?;
        let async_keyword = self.async_keyword?;

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

        let item_fn = (signature, block.clone());
        let item_body = (
            (build, '.', "block_on"),
            parens((fn_name.clone(), parens(()))),
        );

        Some((
            self.entry_kind_attribute(kind),
            expand_without_index(signature, async_keyword),
            group_with_span(Delimiter::Brace, (item_fn, item_body), block.span()),
        ))
    }

    /// Generate attribute associated with entry kind.
    fn entry_kind_attribute(&self, kind: EntryKind) -> impl IntoTokens {
        from_fn(move |s| {
            if let EntryKind::Test = kind {
                s.write((
                    '#',
                    bracketed((S, "core", S, "prelude", S, "v1", S, "test")),
                ))
            }
        })
    }
}

/// Expand the given token tree while skipping the given index.
fn expand_without_index(tokens: &[TokenTree], index: usize) -> impl IntoTokens + '_ {
    from_fn(move |s| {
        for (n, tt) in tokens.iter().enumerate() {
            if n != index {
                s.write(tt.clone());
            }
        }
    })
}

/// Construct a custom group  with a custom span that is not inherited by its
/// children.
pub(crate) fn group_with_span<T>(delimiter: Delimiter, inner: T, span: Span) -> impl IntoTokens
where
    T: IntoTokens,
{
    GroupWithSpan(delimiter, inner, span)
}

struct GroupWithSpan<T>(Delimiter, T, Span);

impl<T> IntoTokens for GroupWithSpan<T>
where
    T: IntoTokens,
{
    fn into_tokens(self, stream: &mut TokenStream, span: Span) {
        let checkpoint = stream.checkpoint();
        self.1.into_tokens(stream, span);
        stream.group(self.2, self.0, checkpoint);
    }
}
