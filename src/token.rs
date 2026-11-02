use std::ops::Range;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'a> {
    pub input: &'a str,
    pub kind: TokenKind,
    pub span: Range<usize>,
}

impl<'a> Token<'a> {
    #[must_use]
    pub const fn new(input: &'a str, kind: TokenKind, span: Range<usize>) -> Self {
        Self { input, kind, span }
    }
}
