use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Literal,
    Identifier,
    String,

    Newline,
    Comma,
    Equal,
    LBracket,
    RBracket,
    DoubleQuote,
    Backslash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub span: Span<'a>,
}

impl<'a> Token<'a> {
    #[must_use]
    pub const fn new(kind: TokenKind, span: Span<'a>) -> Self {
        Self { kind, span }
    }

    pub fn view(&self) -> &str {
        self.span.view()
    }
}
