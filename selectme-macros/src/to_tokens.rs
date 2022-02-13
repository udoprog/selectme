use proc_macro::{Delimiter, Ident, Literal, Punct, Spacing, Span, TokenTree};

use crate::token_stream::TokenStream;

pub trait ToTokens {
    /// Convert into tokens.
    fn to_tokens(self, stream: &mut TokenStream, span: Span);
}

impl ToTokens for TokenStream {
    fn to_tokens(self, stream: &mut TokenStream, _: Span) {
        stream.extend(self);
    }
}

impl<T> ToTokens for Option<T>
where
    T: ToTokens,
{
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        if let Some(tt) = self {
            tt.to_tokens(stream, span);
        }
    }
}

impl ToTokens for &str {
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        let mut ident = Ident::new(self, span);
        ident.set_span(span);
        stream.push(TokenTree::Ident(ident));
    }
}

impl<const N: usize> ToTokens for [char; N] {
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        let mut it = self.into_iter();
        let last = it.next_back();

        while let Some(c) = it.next() {
            let mut punct = Punct::new(c, Spacing::Joint);
            punct.set_span(span);
            stream.push(TokenTree::Punct(punct));
        }

        if let Some(c) = last {
            let mut punct = Punct::new(c, Spacing::Alone);
            punct.set_span(span);
            stream.push(TokenTree::Punct(punct));
        }
    }
}

impl ToTokens for char {
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        let mut punct = Punct::new(self, Spacing::Alone);
        punct.set_span(span);
        stream.push(TokenTree::Punct(punct));
    }
}

impl ToTokens for usize {
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        let mut literal = Literal::usize_unsuffixed(self);
        literal.set_span(span);
        stream.push(TokenTree::Literal(literal));
    }
}

impl ToTokens for () {
    fn to_tokens(self, _: &mut TokenStream, _: Span) {}
}

macro_rules! tuple {
    ($($gen:ident $var:ident),*) => {
        impl<$($gen,)*> ToTokens for ($($gen,)*) where $($gen: ToTokens),* {
            fn to_tokens(self, stream: &mut TokenStream, span: Span) {
                let ($($var,)*) = self;
                $($var.to_tokens(stream, span);)*
            }
        }
    }
}

tuple!(A a);
tuple!(A a, B b);
tuple!(A a, B b, C c);
tuple!(A a, B b, C c, D d);
tuple!(A a, B b, C c, D d, E e);
tuple!(A a, B b, C c, D d, E e, F f);
tuple!(A a, B b, C c, D d, E e, F f, G g);
tuple!(A a, B b, C c, D d, E e, F f, G g, H h);
tuple!(A a, B b, C c, D d, E e, F f, G g, H h, I i);

struct Group<T>(Delimiter, T);

impl<T> ToTokens for Group<T>
where
    T: ToTokens,
{
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        let checkpoint = stream.checkpoint();
        self.1.to_tokens(stream, span);
        stream.group(span, self.0, checkpoint);
    }
}

/// Construct a parenthesized group.
pub fn parens<T>(inner: T) -> impl ToTokens
where
    T: ToTokens,
{
    Group(Delimiter::Parenthesis, inner)
}

/// Construct a braced group.
pub fn braced<T>(inner: T) -> impl ToTokens
where
    T: ToTokens,
{
    Group(Delimiter::Brace, inner)
}

/// Construct a bracketed group.
pub fn bracketed<T>(inner: T) -> impl ToTokens
where
    T: ToTokens,
{
    Group(Delimiter::Bracket, inner)
}

struct StringLiteral<'a>(&'a str);

impl ToTokens for StringLiteral<'_> {
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        let mut literal = Literal::string(self.0);
        literal.set_span(span);
        stream.push(TokenTree::Literal(literal));
    }
}

/// Construct a string literal.
pub fn string(s: &str) -> impl ToTokens + '_ {
    StringLiteral(s)
}

impl ToTokens for &[TokenTree] {
    fn to_tokens(self, stream: &mut TokenStream, _: Span) {
        for tt in self {
            stream.push(tt.clone());
        }
    }
}

struct FromFn<T>(T);

impl<T> ToTokens for FromFn<T>
where
    T: FnOnce(&mut TokenStream, Span),
{
    fn to_tokens(self, stream: &mut TokenStream, span: Span) {
        (self.0)(stream, span);
    }
}

/// Construct a [ToTokens] implementation from a callback function.
pub fn from_fn<T>(f: T) -> impl ToTokens
where
    T: FnOnce(&mut TokenStream, Span),
{
    FromFn(f)
}
