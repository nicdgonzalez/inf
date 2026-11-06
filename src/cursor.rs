use crate::lexer::Tokens;
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone)]
pub struct Cursor<'a> {
    tokens: Tokens<'a>,
    current: Option<Token<'a>>,
}

impl<'a> Cursor<'a> {
    #[must_use]
    pub fn new(mut tokens: Tokens<'a>) -> Self {
        let current = tokens.next();

        Self { tokens, current }
    }

    pub const fn current(&self) -> Option<&Token<'a>> {
        self.current.as_ref()
    }

    pub fn advance(&mut self) {
        self.current = self.tokens.next();
    }

    pub fn take_current_if(&mut self, expected: TokenKind) -> Option<Token<'a>> {
        if self.current.as_ref().is_none_or(|t| t.kind != expected) {
            return None;
        }

        let current = self.current.take().unwrap();
        self.advance();
        Some(current)
    }
}
