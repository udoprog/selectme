use core::slice::SliceIndex;

use proc_macro::TokenTree;

const BUF: usize = 2;

/// Parser base.
pub struct Parser {
    it: proc_macro::token_stream::IntoIter,
    buf: [Option<TokenTree>; BUF],
    head: usize,
    tail: usize,
    tokens: Vec<TokenTree>,
}

impl Parser {
    pub fn new(stream: proc_macro::TokenStream) -> Self {
        Self {
            it: stream.into_iter(),
            buf: [None, None],
            head: 0,
            tail: 0,
            tokens: Vec::new(),
        }
    }

    /// Push a single token onto the token buffer.
    pub fn push(&mut self, tt: TokenTree) {
        self.tokens.push(tt);
    }

    /// Look at the last token in the tokens buffer.
    pub fn last(&mut self) -> Option<&TokenTree> {
        self.tokens.last()
    }

    /// Get the given range of tokens.
    pub fn get<I>(&self, range: I) -> Option<&I::Output>
    where
        I: SliceIndex<[TokenTree]>,
    {
        self.tokens.get(range)
    }

    /// Extend the tokens recorded with the given iterator.
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = TokenTree>,
    {
        self.tokens.extend(iter);
    }

    /// The current length in number of tokens recorded.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Access the token at the given offset.
    pub fn nth(&mut self, n: usize) -> Option<&TokenTree> {
        while (self.head - self.tail) <= n {
            self.buf[self.head % BUF] = Some(self.it.next()?);
            self.head += 1;
        }

        self.buf.get((self.tail + n) % BUF)?.as_ref()
    }

    /// Bump the last token.
    pub fn bump(&mut self) -> Option<TokenTree> {
        if let Some(head) = self.buf.get_mut(self.tail % BUF).and_then(|s| s.take()) {
            self.tail += 1;
            return Some(head);
        }

        self.it.next()
    }

    /// Step over the given number of tokens.
    pub fn step(&mut self, n: usize) {
        for _ in 0..n {
            self.bump();
        }
    }

    /// Convert the current parser into a collection of tokens it has retained.
    pub fn into_tokens(self) -> Vec<TokenTree> {
        self.tokens
    }
}
