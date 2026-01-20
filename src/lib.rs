#![warn(
    clippy::correctness,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::style,
    clippy::pedantic
)]

mod error;
mod section;

use std::char;
use std::io::Read;
use std::iter::Peekable;

pub use error::ParseError;
pub use section::{Entry, Section, Value};

/// The Byte Order Mark (BOM) is used to signal the endianness of an encoding.
/// The order `FF FE` strongly suggests that the data is encoded using little-endian byte order.
///
/// <https://en.wikipedia.org/wiki/Byte_order_mark>
const BOM_LE: [u8; 2] = [0xFF, 0xFE];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inf {
    // Using `Vec` instead of `HashMap` to preserve ordering.
    sections: Vec<Section>,
}

impl Inf {
    pub fn from_reader<R>(reader: &mut R) -> Result<Self, ParseError>
    where
        R: Read,
    {
        let mut buffer = Vec::new();
        reader
            .read_to_end(&mut buffer)
            .map_err(|err| ParseError::ReadFailure { source: err })?;

        Self::try_from(buffer.as_slice())
    }

    pub fn from_bytes(buffer: &[u8]) -> Result<Self, ParseError> {
        Self::try_from(buffer)
    }

    #[must_use]
    pub fn sections(&self) -> &[Section] {
        &self.sections
    }
}

impl TryFrom<&[u8]> for Inf {
    type Error = ParseError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let text = decode_data(data);
        let mut chars = text.chars().peekable();
        let mut sections = Vec::<Section>::with_capacity(16);

        while let Some(c) = chars.next() {
            match c {
                ';' => skip_comment(&mut chars),
                '[' => parse_section(&mut chars, &mut sections)?,
                _ => {}
            }
        }

        Ok(Self { sections })
    }
}

/// Converts a slice of bytes into a UTF-8 string that we can iterate over.
fn decode_data(data: &[u8]) -> String {
    // INF files must be saved with UTF-16 LE or ANSI file encodings. Because ANSI is a subset
    // of UTF-8 and endianness is irrelevant to UTF-8, the BOM being present strongly suggests
    // that the data was encoded with UTF-16 LE.
    if data.starts_with(&BOM_LE) {
        let utf16 = data[BOM_LE.len()..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<u16>>();

        char::decode_utf16(utf16)
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect::<String>()
    } else {
        String::from_utf8_lossy(data).to_string()
    }
}

fn skip_comment<C>(chars: &mut C)
where
    C: Iterator<Item = char>,
{
    _ = chars.find(|&c| c == '\n');
}

fn parse_section<C>(chars: &mut Peekable<C>, sections: &mut Vec<Section>) -> Result<(), ParseError>
where
    C: Iterator<Item = char>,
{
    let section_name = parse_section_name(chars)?;

    // Duplicate section names are allowed; the specification states we should merge their entries.
    let entries = if let Some(i) = sections
        .iter()
        .position(|section| section_name == section.name())
    {
        // If a section with the same name already exists, extend it.
        sections.get_mut(i).unwrap()
    } else {
        // Otherwise, create a new section.
        sections.push(Section::new(section_name, Vec::with_capacity(32)));
        sections.last_mut().unwrap()
    };

    while chars.peek().is_some_and(|&c| c != '[') {
        if let Some(line) = get_next_entry(chars)? {
            let entry = parse_section_entry(&line)?;
            entries.push(entry);
        }
    }

    Ok(())
}

fn parse_section_name<C>(chars: &mut Peekable<C>) -> Result<String, ParseError>
where
    C: Iterator<Item = char>,
{
    let section_name = chars.take_while(|&c| c != ']').collect::<String>();

    if section_name.is_empty() {
        return Err(ParseError::SectionNameEmpty);
    } else if section_name.len() > 255 {
        return Err(ParseError::SectionNameTooLong);
    }

    // Strip excess whitespace and inline comments; break the loop after consuming the newline.
    while let Some(c) = chars.next() {
        match c {
            ';' => {
                skip_comment(chars);
                break;
            }
            '\n' => break, // Will consume any Carriage Returns (\r) also.
            c if c.is_ascii_whitespace() => {
                assert_ne!(c, '\n', r"\n should have been handled separately");
            }
            c => return Err(ParseError::UnexpectedCharacter { c }),
        }
    }

    Ok(section_name)
}

/// Reads the next entry while stripping Line Continuators (\) and inline comments to return
/// a single, uninterrupted line.
fn get_next_entry<C>(chars: &mut Peekable<C>) -> Result<Option<String>, ParseError>
where
    C: Iterator<Item = char>,
{
    let mut line = String::with_capacity(4096);
    let mut within_quotes = false;

    loop {
        let current = chars.take_while(|&c| c != '\n').collect::<String>();
        let mut current = current
            .strip_suffix('\r')
            .unwrap_or(current.as_str())
            .trim_end();

        // Strip inline comments.
        for (i, c) in current.char_indices() {
            match c {
                '"' => within_quotes = !within_quotes,
                ';' if !within_quotes => {
                    current = current[..i].trim_end();
                    break;
                }
                _ => {}
            }
        }

        if within_quotes {
            return Err(ParseError::UnterminatedString);
        }

        // If the line ends with a Line Continuator, strip it and continue to next line.
        if let Some(s) = current.strip_suffix('\\') {
            line.push_str(s);
            continue;
        }

        line.push_str(current);
        break;
    }

    if line.is_empty() {
        Ok(None)
    } else {
        Ok(Some(line))
    }
}

fn parse_section_entry(line: &str) -> Result<Entry, ParseError> {
    assert!(!line.is_empty());
    assert!(!line.ends_with('\\'));
    assert!(!line.contains('\r'));
    assert!(!line.contains('\n'));

    let mut values = Vec::<String>::new();
    let mut within_quotes = false;
    let mut key = None::<String>;
    let mut start = 0;

    for (i, c) in line.char_indices().skip(1) {
        match c {
            '"' => within_quotes = !within_quotes,
            ',' if !within_quotes => {
                if key.is_some() {
                    assert_ne!(start, 0, "expected start to be after the equal sign");
                }

                let value = normalize_value(&line[start..i])?;
                values.push(value);
                start = i + 1;
            }
            '=' if !within_quotes => {
                if !values.is_empty() {
                    // I'm not sure if unquoted equal signs should be allowed, but since it's not
                    // mentioned anywhere in the specification, we'll assume it is for now.
                    continue;
                }

                key = Some(line[start..i].trim().to_owned());
                start = i + 1;
            }
            _ => {}
        }
    }

    // Normalize the final value
    let last = normalize_value(line[start..].trim())?;
    values.push(last);

    let value = if values.len() == 1 {
        values.remove(0).into()
    } else {
        values.into()
    };

    Ok(if let Some(k) = key {
        Entry::Item(k, value)
    } else {
        Entry::ValueOnly(value)
    })
}

fn normalize_value(mut value: &str) -> Result<String, ParseError> {
    value = value.trim();
    value = match (value.starts_with('"'), value.ends_with('"')) {
        (true, true) => &value[1..value.len() - 1],
        (false, false) => value,
        // TODO: I feel like we should already know this based on previous `within_quotes` check.
        _ => return Err(ParseError::UnterminatedString),
    };
    let value = value.replace("\"\"", "\"").replace("\\\\", "\\");

    // Note: We do not un-escape percent signs here since it will become ambiguous later whether
    // they were supposed to be for string substitution or simply escaped percent signs.

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hoshimachi_inf() {
        // Uses quotes strings in section entries.
        let buffer = include_bytes!("../Hoshimachi.inf");
        let inf = Inf::from_bytes(buffer).expect("failed to parse Hoshimachi.inf");
        dbg!(&inf.sections);
    }

    #[test]
    fn novella_inf() {
        // Uses Windows' CRLF
        let buffer = include_bytes!("../Novella.inf");
        let inf = Inf::from_bytes(buffer).expect("failed to parse Novella.inf");
        dbg!(&inf.sections);
    }

    #[test]
    fn hornet_inf() {
        // Uses unquotes strings in section entries.
        let buffer = include_bytes!("../Hornet.inf");
        let inf = Inf::from_bytes(buffer).expect("failed to parse Hornet.inf");
        dbg!(&inf.sections);
    }
}
