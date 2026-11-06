use std::error::Error;
use std::fmt;

use crate::Inf;
use crate::cursor::Cursor;
use crate::entry::Entry;
use crate::lexer::tokenize;
use crate::section::Section;
use crate::token::{Token, TokenKind};
use crate::value::Value;

#[derive(Debug, Clone)]
pub struct Parser<'a> {
    cursor: Cursor<'a>,
    line: u64,
}

impl<'a> Parser<'a> {
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        let tokens = tokenize(input);
        let cursor = Cursor::new(tokens);

        Self { cursor, line: 1 }
    }
}

impl Parser<'_> {
    pub fn parse(mut self) -> Result<Inf, ParseError> {
        let mut sections = Vec::<Section>::new();

        while let Some(token) = self.cursor.current() {
            match token.kind {
                TokenKind::Newline => {
                    self.cursor.advance();
                    self.line += 1;
                }
                TokenKind::LBracket => {
                    let section = self.parse_section()?;
                    sections.push(section);
                }
                _ => {
                    return Err(ParseError {
                        kind: ErrorKind::UnexpectedToken {
                            expected: TokenKind::LBracket,
                            actual: Some(token.kind),
                        },
                        line: self.line,
                    });
                }
            }
        }

        Ok(Inf { sections })
    }
}

impl<'a> Parser<'a> {
    fn expect_current(&mut self, expected: TokenKind) -> Result<Token<'a>, ParseError> {
        let actual = self.cursor.current().copied().map(|token| token.kind);

        self.cursor.take_current_if(expected).ok_or(ParseError {
            kind: ErrorKind::UnexpectedToken { expected, actual },
            line: self.line,
        })
    }

    fn current_is(&self, expected: TokenKind) -> bool {
        self.cursor
            .current()
            .is_some_and(|token| token.kind == expected)
    }

    fn parse_section(&mut self) -> Result<Section, ParseError> {
        _ = self.expect_current(TokenKind::LBracket)?;

        let token = self.expect_current(TokenKind::Identifier)?;
        let section_name = self.get_section_name(token)?;

        _ = self.expect_current(TokenKind::RBracket)?;

        let entries = if self.expect_newline()? {
            self.parse_entries()?
        } else {
            vec![]
        };

        Ok(Section::new(section_name, entries))
    }

    fn expect_newline(&mut self) -> Result<bool, ParseError> {
        match self.cursor.current().map(|token| token.kind) {
            Some(TokenKind::Newline) => {
                self.cursor.advance();
                self.line += 1;
                Ok(true)
            }
            // A newline is not required if this is the end of the buffer.
            None => Ok(false),
            _ => Err(ParseError {
                kind: ErrorKind::UnexpectedToken {
                    expected: TokenKind::Newline,
                    actual: None,
                },
                line: self.line,
            }),
        }
    }

    fn get_section_name(&self, token: Token<'a>) -> Result<String, ParseError> {
        let name = token.view();

        // Some Grapheme Clusters appear as a single character, but are handled as two
        // separate characters programatically (e.g., "y̆" is actually `y` and `U+0306`).
        // The character limit for section names is 255 characters, but depending on how
        // characters are interpreted changes how this function should be implemented.
        //
        // I'm going to assume 255 represents a buffer's capacity, and that it was chosen because
        // it is the maximum value of a `u8` (one byte). If 255 is the total number of bytes
        // a section name may occupy, then "y̆" counts as 2 slots, not 1.
        //
        // On Windows, there exists a tool called `InfVerif` which can be used to verify which
        // interpretation is correct, but I don't currently have a Windows machine to test with.
        //
        // https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/infverif
        match name.len() {
            0 => Err(ParseError {
                kind: ErrorKind::EmptySectionName,
                line: self.line,
            }),
            1..=255 => Ok(name.to_owned()),
            length => Err(ParseError {
                kind: ErrorKind::LongSectionName { length },
                line: self.line,
            }),
        }
    }

    fn parse_entries(&mut self) -> Result<Vec<Entry>, ParseError> {
        let mut entries = Vec::<Entry>::new();

        while let Some(token) = self.cursor.current() {
            match token.kind {
                TokenKind::LBracket => break,
                TokenKind::Newline => {
                    self.cursor.advance();
                    self.line += 1;
                }
                _ => {
                    let entry = self.parse_entry()?;
                    entries.push(entry);
                }
            }
        }

        Ok(entries)
    }

    fn parse_entry(&mut self) -> Result<Entry, ParseError> {
        let mut key = None::<String>;
        let mut values = Vec::<String>::new();
        let mut buffer = None::<String>;

        loop {
            if let Some(token) = self.cursor.current() {
                match token.kind {
                    TokenKind::DoubleQuote => {
                        let value = self.parse_string()?;

                        match buffer {
                            Some(ref mut b) => {
                                b.push(' ');
                                b.push_str(&value);
                            }
                            None => buffer = Some(value),
                        }
                    }
                    TokenKind::Literal => {
                        let value = token.view().to_owned();

                        match buffer {
                            // TODO: If two literals follow each other before reaching
                            // an equal sign, the key will have a space in it. Is this allowed?
                            Some(ref mut b) => {
                                b.push(' ');
                                b.push_str(&value);
                            }
                            None => buffer = Some(value),
                        }

                        self.cursor.advance();
                    }
                    TokenKind::Equal => {
                        if key.is_none() && buffer.is_some() {
                            key = buffer.take();
                        } else {
                            match buffer {
                                Some(ref mut b) => {
                                    b.push(' ');
                                    b.push('=');
                                }
                                None => buffer = Some("=".to_owned()),
                            }
                        }

                        self.cursor.advance();
                    }
                    TokenKind::Comma => {
                        values.push(buffer.take().unwrap_or_default());
                        self.cursor.advance();
                    }
                    TokenKind::Newline => {
                        values.push(buffer.take().unwrap_or_default());
                        self.line += 1;
                        self.cursor.advance();
                        break;
                    }
                    TokenKind::Backslash => {
                        self.cursor.advance();

                        // This is a Line Continuator; this entry continues onto the next line.
                        if self.current_is(TokenKind::Newline) {
                            self.line += 1;
                            self.cursor.advance();
                        }
                    }
                    actual => {
                        return Err(ParseError {
                            kind: ErrorKind::UnexpectedToken {
                                expected: TokenKind::Literal,
                                actual: Some(actual),
                            },
                            line: self.line,
                        });
                    }
                }
            } else {
                // Token stream has ended.
                if let Some(value) = buffer.take() {
                    values.push(value);
                }

                break;
            }
        }

        let value = match values.len() {
            1 => Value::Raw(values.pop().unwrap()),
            _ => Value::List(values),
        };

        Ok(match key {
            Some(key) => Entry::Item(key, value),
            None => Entry::ValueOnly(value),
        })
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        _ = self.expect_current(TokenKind::DoubleQuote)?;

        let token = self.expect_current(TokenKind::String)?;
        let string = Self::normalize_string(token);

        // TODO: Would an `ErrorKind::UnterminatedString` be better?
        _ = self.expect_current(TokenKind::DoubleQuote)?;

        Ok(string)
    }

    fn normalize_string(token: Token<'a>) -> String {
        let mut buffer = String::new();

        let mut chars = token.view().chars().peekable();

        while let Some(c) = chars.next() {
            match (c, chars.peek()) {
                ('"', Some('"')) => {
                    buffer.push('"');
                    chars.next();
                }
                ('\\', Some('\\')) => {
                    buffer.push('\\');
                    chars.next();
                }
                _ => buffer.push(c),
            }
        }

        buffer
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError {
    kind: ErrorKind,
    line: u64,
}

impl ParseError {
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    UnexpectedToken {
        expected: TokenKind,
        actual: Option<TokenKind>,
    },
    EmptySectionName,
    LongSectionName {
        length: usize,
    },
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{}: ", self.line)?;

        match self.kind {
            ErrorKind::UnexpectedToken { expected, actual } => {
                write!(f, "expected token {expected:?}, got {actual:?}")
            }
            ErrorKind::EmptySectionName => "empty section name".fmt(f),
            ErrorKind::LongSectionName { length } => {
                write!(f, "section name exceeds 255 characters (got {length}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unexpected_token() {
        let input = "]Section[";
        let error = Parser::new(input).parse().unwrap_err();

        assert!(matches!(
            error,
            ParseError {
                kind: ErrorKind::UnexpectedToken {
                    expected: TokenKind::LBracket,
                    actual: Some(TokenKind::RBracket),
                },
                line: 1
            }
        ));
    }

    #[test]
    fn empty_section_name() {
        let input = "\
            []\r\n\
            Signature = \"$CHICAGO$\"\
        ";
        let error = Parser::new(input).parse().unwrap_err();

        assert!(matches!(
            error,
            ParseError {
                kind: ErrorKind::EmptySectionName,
                line: 1
            }
        ));
    }

    #[test]
    fn section_name_is_too_long() {
        let input = "\
            [ThisIsASectionNameThatIsWayTooLongAndDefinitelyExceeds255CharactersThisIsASectionNameThatIsWayTooLongAndDefinitelyExceeds255CharactersThisIsASectionNameThatIsWayTooLongAndDefinitelyExceeds255CharactersThisIsASectionNameThatIsWayTooLongAndDefinitelyExceeds255Characters]\r\n\
            Signature = \"$CHICAGO$\"\
        ";
        let error = Parser::new(input).parse().unwrap_err();

        assert!(matches!(
            error,
            ParseError {
                kind: ErrorKind::LongSectionName { length: 268 },
                line: 1
            }
        ));
    }

    #[test]
    fn section_no_entries() {
        let input = "[Version]";
        let inf = Parser::new(input).parse().unwrap();

        assert_eq!(
            inf.sections,
            vec![Section::new("Version".to_owned(), vec![])]
        );
    }

    #[test]
    fn unterminated_string() {
        let input = "\
            [Version]\n\
            Signature = \"$CHICAGO$\
        ";
        let error = Parser::new(input).parse().unwrap_err();

        assert!(matches!(
            error,
            ParseError {
                kind: ErrorKind::UnexpectedToken {
                    expected: TokenKind::DoubleQuote,
                    actual: None,
                },
                line: 2
            }
        ));
    }

    #[test]
    fn starts_with_comment() {
        let input = "\
            ; File starts with a comment.\n\
            [Version]\n\
            Signature = \"$CHICAGO$\"\
        ";
        let inf = Parser::new(input).parse().unwrap();

        assert_eq!(
            inf.sections,
            vec![Section::new(
                "Version".to_owned(),
                vec![Entry::Item(
                    "Signature".to_owned(),
                    Value::Raw("$CHICAGO$".to_owned()),
                )]
            )]
        );
    }

    #[test]
    fn multiple_newlines_before_section() {
        let input = "\
            \n\
            \n\
            \n\
            \n\
            [Version]\n\
            Signature = \"$CHICAGO$\"\
        ";
        let inf = Parser::new(input).parse().unwrap();

        assert_eq!(
            inf.sections,
            vec![Section::new(
                "Version".to_owned(),
                vec![Entry::Item(
                    "Signature".to_owned(),
                    Value::Raw("$CHICAGO$".to_owned()),
                )]
            )]
        );
    }

    #[test]
    fn equal_sign_as_first_value() {
        let input = "\
            [Section]\n\
            =,\"Hello, World!\"\
        ";
        let inf = Parser::new(input).parse().unwrap();

        assert_eq!(
            inf.sections,
            vec![Section::new(
                "Section".to_owned(),
                vec![Entry::ValueOnly(Value::List(vec![
                    "=".to_owned(),
                    "Hello, World!".to_owned(),
                ]))]
            )]
        );
    }

    #[test]
    fn value_only_entry_quoted() {
        let input = "\
            [Scheme.Cur]\n\
            \"Default.ani\"
        ";
        let inf = Parser::new(input).parse().unwrap();

        assert_eq!(
            inf.sections,
            vec![Section::new(
                "Scheme.Cur".to_owned(),
                vec![Entry::ValueOnly(Value::Raw("Default.ani".to_owned()))],
            )]
        );
    }

    #[test]
    fn multiline_value_with_inline_comment() {
        let input = "\
            [Section]\n\
            key = value1,\"value2;not-a-comment\"\\ ; This is an inline comment.\n\
            ,value3,,value5
        ";
        let inf = Parser::new(input).parse().unwrap();

        assert_eq!(
            inf.sections,
            vec![Section::new(
                "Section".to_owned(),
                vec![Entry::Item(
                    "key".to_owned(),
                    Value::List(vec![
                        "value1".to_owned(),
                        "value2;not-a-comment".to_owned(),
                        "value3".to_owned(),
                        String::new(),
                        "value5".to_owned(),
                    ]),
                )],
            )]
        );
    }

    #[test]
    fn value_with_backslash_and_line_continuator() {
        let input = "\
            [DefaultInstall]\n\
            CopyFiles = \"SomeDirectory\\\"\\\n\
            ,SomeFile\
        ";
        let inf = Parser::new(input).parse().unwrap();

        assert_eq!(
            inf.sections,
            vec![Section::new(
                "DefaultInstall".to_owned(),
                vec![Entry::Item(
                    "CopyFiles".to_owned(),
                    Value::List(vec!["SomeDirectory\\".to_owned(), "SomeFile".to_owned()]),
                )],
            )]
        );
    }

    #[test]
    fn multiple_empty_sections() {
        let input = "\
            [Section1]\n\
            [Section2]\n\
            [Section3]\
        ";
        let inf = Parser::new(input).parse().unwrap();

        assert_eq!(
            inf.sections,
            vec![
                Section::new("Section1".to_owned(), vec![]),
                Section::new("Section2".to_owned(), vec![]),
                Section::new("Section3".to_owned(), vec![]),
            ]
        );
    }

    #[test]
    fn mixed_entry_kinds() {
        let input = "\
            [Section]\n\
            value\n\
            \"value1\",value2,,\"value4\\\"\n\
            key = value\
        ";
        let inf = Parser::new(input).parse().unwrap();

        assert_eq!(
            inf.sections,
            vec![Section::new(
                "Section".to_owned(),
                vec![
                    Entry::ValueOnly(Value::Raw("value".to_owned())),
                    Entry::ValueOnly(Value::List(vec![
                        "value1".to_owned(),
                        "value2".to_owned(),
                        String::new(),
                        "value4\\".to_owned()
                    ])),
                    Entry::Item("key".to_owned(), Value::Raw("value".to_owned())),
                ]
            )]
        );
    }
}
