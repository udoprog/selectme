use std::ops;

use proc_macro::{Span, TokenTree};

use crate::error::Error;
use crate::to_tokens::{braced, bracketed, from_fn, parens, string, ToTokens};
use crate::tok::S;

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
            "\"single_thread\"" => Err("The single threaded runtime flavor is called `current_thread`"),
            "\"basic_scheduler\"" => Err("The `basic_scheduler` runtime flavor has been renamed to `current_thread`"),
            "\"threaded_scheduler\"" => Err("The `threaded_scheduler` runtime flavor has been renamed to `multi_thread`"),
            _ => Err("No such runtime flavor. The runtime flavors are `current_thread` and `multi_thread`"),
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
                    errors.push(Error::new(tt.span(), format!("The `start_paused` option requires the `current_thread` runtime flavor. Use `#[{}(flavor = \"current_thread\")]`", kind.name())));
                }
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
}

impl ItemOutput {
    pub(crate) fn new(
        tokens: Vec<TokenTree>,
        has_async: bool,
        signature: Option<ops::Range<usize>>,
        block: Option<ops::Range<usize>>,
    ) -> Self {
        Self {
            tokens,
            has_async,
            signature,
            block,
        }
    }

    /// Expand into a function item.
    pub fn expand_item(&self, kind: EntryKind, config: Config) -> impl ToTokens + '_ {
        from_fn(move |s| {
            if let (Some(signature), Some(block)) = (self.signature.clone(), self.block.clone()) {
                s.write((
                    self.entry_kind_attribute(kind),
                    &self.tokens[signature],
                    braced(self.item_body(config, block)),
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
    fn item_body(&self, config: Config, block: ops::Range<usize>) -> impl ToTokens + '_ {
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

        (
            build,
            '.',
            "block_on",
            parens(("async", &self.tokens[block])),
        )
    }
}
