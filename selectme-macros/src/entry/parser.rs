use crate::entry::output::Output;
use crate::error::Error;
use crate::parsing;

/// A parser for the `select!` macro.
pub struct Parser {
    base: parsing::Parser,
    errors: Vec<Error>,
}

impl Parser {
    /// Construct a new parser around the given token stream.
    pub(crate) fn new(stream: proc_macro::TokenStream) -> Self {
        Self {
            base: parsing::Parser::new(stream),
            errors: Vec::new(),
        }
    }

    /// Parse and produce the corresponding token stream.
    pub(crate) fn parse(self) -> Result<Output, Vec<Error>> {
        if true {
            return Err(self.errors);
        }

        Ok(Output::new(self.base.into_tokens()))
    }
}
