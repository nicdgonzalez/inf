#![warn(
    clippy::correctness,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::style,
    clippy::pedantic
)]

use std::collections::HashMap;
use std::{error, fmt};

/// Represents an entry in a [Section].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// Key-value pair
    Item(String, Value),
    /// Standalone value
    Value(Value),
}

/// Represents a value in an [Entry].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// String value
    Raw(String),
    /// Comma-separated list of values
    List(Vec<String>),
}

// "setup information (INF) file"

/// Implemented based on the rules in Microsoft's [General Syntax Rules for INF Files].
///
/// [General Syntax Rules for INF Files]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/general-syntax-rules-for-inf-files
#[derive(Debug)]
pub struct Inf {
    // TODO: Section names are supposed to be case-insensitive.
    // The HashMap and Key probably need custom hash implementations to abstract away this detail.
    sections: HashMap<String, Vec<Entry>>,
}

impl Inf {
    pub fn parse(mut buffer: &[u8]) -> Result<Self, ParserError> {
        let mut sections = HashMap::<String, Vec<Entry>>::with_capacity(16);

        while let Some((&b, data)) = buffer.split_first() {
            if b.is_ascii_whitespace() {
                buffer = data;
                continue;
            }

            match b {
                b';' => {
                    // TODO: This was just for my own sanity during development; in the final
                    // version, don't waste time keeping the comment, just return the `data`.
                    let (comment, data) = parse_comment(buffer);
                    println!("Comment: {:?}", String::from_utf8_lossy(comment));
                    buffer = data;
                }
                b'[' => {
                    // TODO: See if there is a performance gain if I first group each of
                    // the sections into chunks, and then process them in parallel.
                    let (section_name, data) = parse_section_name(buffer)?;
                    buffer = data;
                    println!("Section name: {:?}", String::from_utf8_lossy(section_name));

                    // TODO: Parse entries
                    //
                    // If we go line by line, the line can:
                    //
                    // Note: It would be good to have unit tests for all of these cases.
                    //
                    // - be empty
                    // - end with a `\`, making it a multiline value
                    // - be a comment
                    // - be `a \
                    //       really \
                    //       weird \
                    //       key = value1,"PathValue\"\ ; with a comment
                    //       ,value3`
                    // - be `key = value`
                    // - be `key = "value"`
                    // - be `key=value`
                    // - be `"value"`
                    // - be `"value","value",,"value"`
                    // - be `value,,,value,value`
                    // - be `long value with spaces and no quotes`
                    //
                    // First, we should separate out the line (take a slice of start to newline) to
                    // constrain our searches to within the currently-being-parsed line.
                    //
                    // Then, search within the sub-slice for an equal sign (=).
                    //
                    // If it is a quoted value, to escape a double quote ("), you need to use
                    // two double quotes (e.g., `"Display an ""example"" string"` will become
                    // `Display an "example" string`).
                    //
                    // ---
                    //
                    // Create sub-slice: start..newline.
                    //
                    // Create another sub-slice from start to `=`. If found, this becomes the key.
                    // (Note: Keep in mind that the line can end with a backslash, making it a
                    // multi-line key)
                    //
                    // For the value, search for commas.
                    //
                    // ---
                    //
                    // The maximum length of an INF file "field" is 4096 before string
                    // substitution.
                    //
                    // The maximum length of an individual string *after* string substitution is
                    // 4096.
                    //
                    // Allocate `n` threads, with pre-fixed buffers of 4096 for field and then
                    // later for the expanded string value.

                    // TODO: while !data.trim().starts_with(b'[') || EOF -> parse each entry
                    let (entry, data) = parse_section_entry(buffer);
                    buffer = data;
                    println!("Entry: {entry:?}");

                    // TODO:
                    // check if sections[section_name] already exists
                    //  if exists: get mut ref to existing -> extend existing
                    //  else: insert new vec -> parse and insert entries
                }
                _ => {
                    println!("No match: {}", char::from(b));
                    buffer = data;
                }
            }
        }

        Ok(Self { sections })
    }
}

type Comment<'a> = &'a [u8];
type Data<'a> = &'a [u8];

fn parse_comment(mut buffer: &[u8]) -> (Comment<'_>, Data<'_>) {
    buffer = &buffer[1..]; // Skip the ';'

    let start = buffer
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(0);
    let stop = buffer
        .iter()
        .position(|&b| b == b'\n')
        .unwrap_or(buffer.len());
    let comment = &buffer[start..stop];

    let data = if stop + 1 < buffer.len() {
        &buffer[stop + 1..]
    } else {
        &[]
    };

    (comment, data)
}

type SectionName<'a> = &'a [u8];

fn parse_section_name(mut buffer: &[u8]) -> Result<(SectionName<'_>, Data<'_>), ParserError> {
    buffer = &buffer[1..]; // Skip the '['

    // TODO: Make sure chaining take_while and position together like this does what I expect it
    // to do... which is to create a sub-slice until the newline, and find the closing bracket
    // *within that sub-slice*.
    let r_bracket = buffer
        .iter()
        .take_while(|&&b| b != b'\n')
        .position(|&b| b == b']')
        .ok_or(ParserError::InvalidSyntax)?;
    let section_name = &buffer[..r_bracket];

    if section_name.is_empty() {
        return Err(ParserError::SectionNameEmpty);
    } else if section_name.len() > 255 {
        return Err(ParserError::SectionNameTooLong);
    }

    let data = &buffer[r_bracket + 1..];

    if !data
        .iter()
        .take_while(|&&b| b != b'\n')
        .all(u8::is_ascii_whitespace)
    {
        return Err(ParserError::InvalidSyntax);
    }

    Ok((section_name, data))
}

// TODO: Return an Entry instead of raw bytes.
fn parse_section_entry(mut buffer: &[u8]) -> (Entry, &[u8]) {
    let mut chunk = Vec::<u8>::with_capacity(4096);

    // TODO: Create helper function to read one line at a time.
    let newline = buffer
        .iter()
        .position(|&b| b == b'\n')
        .unwrap_or(buffer.len());
    let mut current_line = &buffer[..newline];
    println!("Current line: {:?}", String::from_utf8_lossy(current_line));

    buffer = if newline + 1 < buffer.len() {
        &buffer[newline + 1..]
    } else {
        &[]
    };

    // Concatenate current line + next line if line ends with a backslash.
    while current_line.ends_with(b"\\") {
        chunk.extend(&current_line[..current_line.len() - 1]);

        let newline = buffer
            .iter()
            .position(|&b| b == b'\n')
            .unwrap_or(buffer.len());
        current_line = &buffer[..newline];

        buffer = if newline + 1 < buffer.len() {
            &buffer[newline + 1..]
        } else {
            &[]
        };
    }

    chunk.extend(current_line);
    println!("Chunk: {:?}", String::from_utf8_lossy(chunk.as_slice()));

    let entry = if chunk.contains(&b'=') {
        let equal = chunk.iter().position(|&b| b == b'=').unwrap();
        let key = &chunk[..equal];
        let value = &chunk[equal + 1..];

        Entry::Item(
            String::from_utf8_lossy(key).to_string(),
            Value::Raw(
                String::from_utf8_lossy(value)
                    .trim()
                    .trim_matches('"')
                    .to_string(),
            ),
        )
    } else {
        Entry::Value(Value::Raw(
            String::from_utf8_lossy(chunk.as_slice())
                .trim()
                .trim_matches('"')
                .to_string(),
        ))
    };

    (entry, buffer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserError {
    SectionNameEmpty,
    SectionNameTooLong,
    InvalidSyntax,
}

impl error::Error for ParserError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::SectionNameEmpty => "section name cannot be empty".fmt(f),
            Self::SectionNameTooLong => "section name exceeds 255 character limit".fmt(f),
            Self::InvalidSyntax => "invalid syntax".fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_before_sections() {
        let buffer = b"; Hello, World!\n\n[Version]";

        assert_eq!(
            parse_comment(buffer.as_slice()),
            (b"Hello, World!".as_slice(), b"\n[Version]".as_slice())
        );
    }

    #[test]
    fn comment() {
        let buffer = b"; This is a comment.";

        assert_eq!(
            parse_comment(buffer),
            (b"This is a comment.".as_slice(), b"".as_slice())
        );
    }

    #[test]
    fn section_name() {
        let buffer = b"[Version]";

        assert_eq!(
            parse_section_name(buffer).expect("failed to parse hardcoded section name"),
            (b"Version".as_slice(), b"".as_slice())
        );
    }

    #[test]
    fn section_name_double_closing_bracket() {
        let buffer = b"[Version]]";

        assert_eq!(parse_section_name(buffer), Err(ParserError::InvalidSyntax));
    }

    #[test]
    fn section_name_valid_quoted() {
        let buffer = b"[;; Std.Mfg ]";

        assert_eq!(
            parse_section_name(buffer).expect("failed to parse hardcoded section name"),
            (b";; Std.Mfg ".as_slice(), b"".as_slice())
        );
    }

    #[test]
    fn section_name_trailing_whitespace() {
        let buffer = b"[Version]     ";

        assert_eq!(
            parse_section_name(buffer).expect("failed to parse hardcoded section name"),
            (b"Version".as_slice(), b"     ".as_slice())
        );
    }

    #[test]
    fn section_name_newline_in_middle() {
        let buffer = b"[Vers\nion]";

        assert_eq!(parse_section_name(buffer), Err(ParserError::InvalidSyntax));
    }

    #[test]
    fn section_name_empty() {
        // Note: I'm not sure if this is actually disallowed, but I'm disallowing is just in case.
        let buffer = b"[]";

        assert_eq!(
            parse_section_name(buffer),
            Err(ParserError::SectionNameEmpty)
        );
    }

    #[test]
    fn multiple_sections() {
        let buffer = b"\
            [Version]\n\
            Signature=\"$CHICAGO$\"\n\
            \n\
            [DefaultInstall]\
        ";

        assert_eq!(
            parse_section_name(buffer),
            Ok((
                b"Version".as_slice(),
                b"\nSignature=\"$CHICAGO$\"\n\n[DefaultInstall]".as_slice()
            ))
        );
    }

    #[test]
    fn section_entry() {
        let buffer = b"Signature=\"$CHICAGO$\"";
        let (entry, data) = parse_section_entry(buffer);

        assert_eq!(
            entry,
            Entry::Item("Signature".to_owned(), Value::Raw("$CHICAGO$".to_owned()))
        );
        assert_eq!(data, b"");
    }

    #[test]
    fn section_entry_multiline() {
        let buffer = b"Signature=\\\n\"$CHICAGO$\"";
        let (entry, data) = parse_section_entry(buffer);

        assert_eq!(
            entry,
            Entry::Item("Signature".to_owned(), Value::Raw("$CHICAGO$".to_owned()))
        );
        assert_eq!(data, b"");
    }

    // test multi-line value with comment in the middle
    //
    // ```inf
    // [SectionName]
    // key = value1,"Value2\"\ ; example comment
    // ,value3,,
    // ```
}
