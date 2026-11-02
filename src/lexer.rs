#![allow(dead_code)]

use crate::token::{Token, TokenKind};

#[must_use]
pub fn tokenize(input: &str) -> Tokens<'_> {
    Tokens {
        input,
        position: 0,
        mode: Mode::Normal,
    }
}

pub struct Tokens<'a> {
    input: &'a str,
    position: usize,
    mode: Mode,
}

enum Mode {
    Normal,
    Section,
    Comment,
    String,
}

enum LexResult<'a> {
    Token(Token<'a>),
    Skip,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.position < self.input.len() {
            let result = match self.mode {
                Mode::Normal => self.normal(),
                Mode::Section => self.section(),
                Mode::Comment => self.comment(),
                Mode::String => self.string(),
            };

            match result {
                LexResult::Token(token) => return Some(token),
                LexResult::Skip => {}
            }
        }

        None
    }
}

impl<'a> Tokens<'a> {
    fn bytes(&self) -> &[u8] {
        self.input.as_bytes()
    }

    fn current(&self) -> u8 {
        debug_assert!(self.position < self.input.len());
        self.bytes()[self.position]
    }

    fn peek(&self) -> Option<&u8> {
        self.bytes().get(self.position + 1)
    }

    fn char_len_at(&self, position: usize) -> usize {
        self.input[position..]
            .chars()
            .next()
            .map_or(0, char::len_utf8)
    }

    fn is_delimiter(byte: u8) -> bool {
        matches!(byte, b',' | b'=' | b';') || byte.is_ascii_whitespace()
    }

    fn normal(&mut self) -> LexResult<'a> {
        let (kind, offset) = match self.current() {
            b'\r' => (
                TokenKind::Newline,
                1 + usize::from(self.peek().is_some_and(|&b| b == b'\n')),
            ),
            b'\n' => (TokenKind::Newline, 1),
            b',' => (TokenKind::Comma, 1),
            b'=' => (TokenKind::Equal, 1),
            b'[' => {
                self.mode = Mode::Section;
                (TokenKind::LBracket, 1)
            }
            b']' => {
                self.mode = Mode::Normal;
                (TokenKind::RBracket, 1)
            }
            b';' => {
                self.position += 1;
                self.mode = Mode::Comment;
                return LexResult::Skip; // Comment-related tokens are skipped to simplify parsing.
            }
            b'"' => {
                self.mode = Mode::String;
                (TokenKind::DoubleQuote, 1)
            }
            b'\\' => (TokenKind::Backslash, 1),
            b if b.is_ascii_whitespace() => {
                self.position += self.char_len_at(self.position);
                return LexResult::Skip;
            }
            _ => {
                let offset = self
                    .bytes()
                    .iter()
                    .skip(self.position)
                    .position(|&b| Self::is_delimiter(b))
                    .unwrap_or(self.input.len() - self.position);

                (TokenKind::Literal, offset)
            }
        };

        let start = self.position;
        debug_assert!(offset > 0);
        self.position += offset;

        LexResult::Token(Token::new(self.input, kind, start..self.position))
    }

    fn section(&mut self) -> LexResult<'a> {
        let start = self.position;

        self.position += self
            .bytes()
            .iter()
            .skip(self.position)
            .position(|&b| b == b']')
            .unwrap_or(self.input.len() - self.position);

        self.mode = Mode::Normal;

        LexResult::Token(Token::new(
            self.input,
            TokenKind::Identifier,
            start..self.position,
        ))
    }

    fn comment(&mut self) -> LexResult<'a> {
        self.position += self
            .bytes()
            .iter()
            .skip(self.position)
            .position(|&b| b == b'\n')
            .unwrap_or(self.input.len() - self.position);

        self.mode = Mode::Normal;

        LexResult::Skip // Comment-related tokens are skipped to simplify parsing.
    }

    fn string(&mut self) -> LexResult<'a> {
        let start = self.position;

        match (self.current(), self.peek()) {
            (b'"', Some(b'"')) => {
                self.position += 2;
            }

            // Because strings share the same start and end delimiter, to reset the lexer's mode,
            // the DoubleQuote token must be returned from within String mode. Otherwise,
            // if Normal mode is set too soon, the next iteration will correctly return
            // the DoubleQuote token, but incorrectly set the mode back to String.
            (b'"', Some(_) | None) => {
                self.position += 1;
                self.mode = Mode::Normal;

                return LexResult::Token(Token::new(
                    self.input,
                    TokenKind::DoubleQuote,
                    start..self.position,
                ));
            }

            _ => {
                self.position += self.char_len_at(self.position);
            }
        }

        while self.position < self.input.len() {
            match self.current() {
                b'"' => {
                    if self.peek().is_some_and(|&b| b == b'"') {
                        self.position += 2;
                    } else {
                        break;
                    }
                }
                _ => {
                    self.position += self.char_len_at(self.position);
                }
            }
        }

        LexResult::Token(Token::new(
            self.input,
            TokenKind::String,
            start..self.position,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_section() {
        let input = "[Section]";
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(
            tokens,
            vec![
                TokenKind::LBracket,
                TokenKind::Identifier,
                TokenKind::RBracket,
            ]
        );
    }

    #[test]
    fn section_with_semicolon() {
        let input = "[; Std.Mfg ]";
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(
            tokens,
            vec![
                TokenKind::LBracket,
                TokenKind::Identifier,
                TokenKind::RBracket,
            ]
        );
    }

    #[test]
    fn comment() {
        let input = "; This is a comment";
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn string() {
        let input = r#""This is a string with a ""nested string"".""#;
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(
            tokens,
            vec![
                TokenKind::DoubleQuote,
                TokenKind::String,
                TokenKind::DoubleQuote,
            ]
        );
    }

    #[test]
    fn unterminated_string() {
        let input = "\"This is an unterminated string";
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(tokens, vec![TokenKind::DoubleQuote, TokenKind::String]);
    }

    #[test]
    fn item_raw_entry() {
        let input = r#"Signature = "$CHICAGO$""#;
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(
            tokens,
            vec![
                TokenKind::Literal,
                TokenKind::Equal,
                TokenKind::DoubleQuote,
                TokenKind::String,
                TokenKind::DoubleQuote,
            ]
        );
    }

    #[test]
    fn item_raw_entry_unquoted() {
        let input = r"Signature = $CHICAGO$";
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(
            tokens,
            vec![TokenKind::Literal, TokenKind::Equal, TokenKind::Literal]
        );
    }

    #[test]
    fn item_list_entry() {
        let input = r#"Scheme.Cur=10,"%CUR_DIR%""#;
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(
            tokens,
            vec![
                TokenKind::Literal,
                TokenKind::Equal,
                TokenKind::Literal,
                TokenKind::Comma,
                TokenKind::DoubleQuote,
                TokenKind::String,
                TokenKind::DoubleQuote,
            ]
        );
    }

    #[test]
    fn value_only_raw_entry() {
        let input = "Cursor Default.ani";
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(tokens, vec![TokenKind::Literal, TokenKind::Literal]);
    }

    #[test]
    fn value_only_raw_entries() {
        let input = "\
            Cursor Default.ani\n\
            Cursor Help.ani\n\
            Cursor Busy.ani\n\
            Cursor Text.ani\n\
            Cursor Unavailable.ani\
        ";
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(
            tokens,
            vec![
                TokenKind::Literal,
                TokenKind::Literal,
                TokenKind::Newline,
                TokenKind::Literal,
                TokenKind::Literal,
                TokenKind::Newline,
                TokenKind::Literal,
                TokenKind::Literal,
                TokenKind::Newline,
                TokenKind::Literal,
                TokenKind::Literal,
                TokenKind::Newline,
                TokenKind::Literal,
                TokenKind::Literal,
            ]
        );
    }

    #[test]
    fn value_only_list_entries() {
        let input =
            r#"HKCU,"Control Panel\Cursors\Schemes","%SCHEME_NAME%",,"%10%\%CUR_DIR%\%default%""#;
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(
            tokens,
            vec![
                TokenKind::Literal,
                TokenKind::Comma,
                TokenKind::DoubleQuote,
                TokenKind::String,
                TokenKind::DoubleQuote,
                TokenKind::Comma,
                TokenKind::DoubleQuote,
                TokenKind::String,
                TokenKind::DoubleQuote,
                TokenKind::Comma,
                TokenKind::Comma,
                TokenKind::DoubleQuote,
                TokenKind::String,
                TokenKind::DoubleQuote,
            ]
        );
    }

    #[test]
    fn mixed_entries() {
        let input = "\
            [DefaultInstall]\n\
            CopyFiles = 10,\"%CUR_DIR%\"\n\
            key=value\n\
            value1,value2,,\"value4\" ; Comment\
        ";
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(
            tokens,
            vec![
                TokenKind::LBracket,
                TokenKind::Identifier,
                TokenKind::RBracket,
                TokenKind::Newline,
                TokenKind::Literal,
                TokenKind::Equal,
                TokenKind::Literal,
                TokenKind::Comma,
                TokenKind::DoubleQuote,
                TokenKind::String,
                TokenKind::DoubleQuote,
                TokenKind::Newline,
                TokenKind::Literal,
                TokenKind::Equal,
                TokenKind::Literal,
                TokenKind::Newline,
                TokenKind::Literal,
                TokenKind::Comma,
                TokenKind::Literal,
                TokenKind::Comma,
                TokenKind::Comma,
                TokenKind::DoubleQuote,
                TokenKind::String,
                TokenKind::DoubleQuote,
            ]
        );
    }

    #[test]
    fn line_continuator_string() {
        let input = "\
            \"Hello, \" \\\n\
            \"World!\"\
        ";
        let tokens = tokenize(input).map(|token| token.kind).collect::<Vec<_>>();

        assert_eq!(
            tokens,
            vec![
                TokenKind::DoubleQuote,
                TokenKind::String,
                TokenKind::DoubleQuote,
                TokenKind::Backslash,
                TokenKind::Newline,
                TokenKind::DoubleQuote,
                TokenKind::String,
                TokenKind::DoubleQuote,
            ]
        );
    }
}
