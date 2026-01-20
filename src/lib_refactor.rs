#![warn(
    clippy::correctness,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::style,
    clippy::pedantic
)]

use std::collections::HashMap;
use std::iter::Peekable;
use std::{char, error, fmt};

/// Byte Order Mark (BOM) is used to signal the endianness of an encoding. The order `0xFF 0xFE`
/// strongly suggests that the encoding is using little-endian byte order.
///
/// <https://en.wikipedia.org/wiki/Byte_order_mark>
const BOM_LE: &[u8] = &[0xFF, 0xFE];

#[derive(Debug)]
pub struct Inf {
    pub sections: HashMap<String, Vec<Entry>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Item(String, Value),
    ValueOnly(Value),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Raw(String),
    List(Vec<String>),
}

impl Inf {
    pub fn parse(buffer: &[u8]) -> Result<Self, ParseError> {
        let text = decode_data(buffer);
        let mut chars = text.chars().peekable();
        let mut sections = HashMap::<String, Vec<Entry>>::with_capacity(14);

        // TODO: Parse the [Strings[.*]] section first, and do string substitution while parsing
        // entries.
        //
        // Or, clarify that string substitution has not occurred yet, and provide a helper util
        // expand() function or something.
        //
        // Experiment with splitting the text into chunks at each section (adds way more complexity)
        // -> text.find("[Strings")
        //
        // The Strings section is more complex than it seems, so make sure to read this while
        // implementing it:
        //
        // https://learn.microsoft.com/en-us/windows-hardware/drivers/install/inf-strings-section
        while let Some(c) = chars.next() {
            match c {
                ';' => skip_comment(&mut chars),
                '[' => parse_section(&mut chars, &mut sections)?,
                // TODO: Handle CRLF, not only LF. Some sections appear as Raw("") because CR does
                // not count as the LF.
                c if c.is_ascii_whitespace() => {}
                c => return Err(ParseError::Syntax { character: c }),
            }
        }

        Ok(Self { sections })
    }
}

// INF files must be saved with UTF-16 LE or ANSI file encoding.
// <https://learn.microsoft.com/en-us/windows-hardware/drivers/display/general-unicode-requirement>
fn decode_data(data: &[u8]) -> String {
    if data.starts_with(BOM_LE) {
        // Likely UTF-16 LE
        let utf16 = data[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<u16>>();

        char::decode_utf16(utf16)
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect::<String>()
    } else {
        // Otherwise, assume ANSI, which is a subset of UTF-8.
        String::from_utf8_lossy(data).to_string()
    }
}

fn skip_comment<C>(chars: &mut C)
where
    C: Iterator<Item = char>,
{
    _ = chars.find(|&c| c == '\n');
}

fn parse_section<C>(
    chars: &mut Peekable<C>,
    sections: &mut HashMap<String, Vec<Entry>>,
) -> Result<(), ParseError>
where
    C: Iterator<Item = char>,
{
    let section_name = parse_section_name(chars)?;
    let entries = sections
        .entry(section_name)
        .or_insert_with(|| Vec::with_capacity(32));

    // Parse each line until you reach a new section or EOF.
    while chars.peek().is_some_and(|&c| c != '[') {
        if let Some(line) = read_line(chars) {
            let entry = parse_section_entry(&line);
            entries.push(entry?);
        }
    }

    Ok(())
}

fn read_line<C>(chars: &mut Peekable<C>) -> Option<String>
where
    C: Iterator<Item = char>,
{
    let mut line = String::with_capacity(1024);
    let mut within_quotes = false;

    loop {
        let mut current = chars
            .by_ref()
            .take_while(|&c| c != '\n')
            .collect::<String>();

        strip_inline_comment(&mut current, &mut within_quotes);

        if !current.ends_with('\\') {
            line.push_str(&current);
            break;
        }

        line.push_str(current[..current.len() - 1].trim_end());
    }

    if line.is_empty() { None } else { Some(line) }
}

fn strip_inline_comment(line: &mut String, within_quotes: &mut bool) {
    for (i, c) in line.char_indices() {
        match c {
            '"' => *within_quotes = !*within_quotes,
            ';' if !*within_quotes => {
                *line = line[..i].trim_end().to_owned();
                break;
            }
            _ => {}
        }
    }
}

fn parse_section_name<C>(chars: &mut C) -> Result<String, ParseError>
where
    C: Iterator<Item = char>,
{
    let section_name = chars.by_ref().take_while(|&c| c != ']').collect::<String>();

    if section_name.is_empty() {
        return Err(ParseError::SectionNameEmpty);
    } else if section_name.len() > 255 {
        return Err(ParseError::SectionNameTooLong);
    }

    // Strip excess whitespace and comments; break the loop after consuming the newline.
    while let Some(d) = chars.next() {
        match d {
            ';' => {
                skip_comment(chars);
                break;
            }
            '\n' => break,
            c if c.is_ascii_whitespace() => {
                assert!(c != '\n', "newline should have been handled separately");
            }
            c => return Err(ParseError::Syntax { character: c }),
        }
    }

    Ok(section_name)
}

fn parse_section_entry(line: &str) -> Result<Entry, ParseError> {
    // TODO: There has to be a better way to ensure these things are true.
    assert!(!line.is_empty());
    assert!(!line.ends_with('\\'));
    assert!(!line.contains('\n'));

    if line.starts_with('"') {
        let mut values = Vec::<String>::new();
        let mut within_quotes = true;
        let mut start = 0usize;

        // Split at commas
        for (i, c) in line.trim().char_indices().skip(1) {
            match c {
                '"' => within_quotes = !within_quotes,
                ',' if !within_quotes => {
                    let value = &line[start..i];
                    let value = match (value.starts_with('"'), value.ends_with('"')) {
                        (true, true) => &value[1..value.len() - 1],
                        (false, false) => value,
                        // TODO: Ensure that every string contains an opening AND closing double quote.
                        // TODO: Either properly test this, or remove this error.
                        _ => {
                            println!("Debug: {value:?}");
                            return Err(ParseError::UnclosedString);
                        }
                    };
                    let value = value.replace("\"\"", "\"");
                    let value = value.replace("\\\\", "\\");
                    // TODO: If I collapse the double percent signs now, it will be ambiguous
                    // whether it is supposed to be string subsitution or an escaped percent sign.
                    // let value = value.replace("%%", "%");
                    values.push(value);
                    start = i + 1;
                }
                _ => {}
            }
        }

        // TODO: Process the value.
        let value = line[start..].trim();
        let value = match (value.starts_with('"'), value.ends_with('"')) {
            (true, true) => value[1..value.len() - 1].trim(),
            // TODO: Ensure that every string contains an opening AND closing double quote.
            // TODO: Either properly test this, or remove this error.
            (false, false) => value,
            _ => {
                println!("Debug: {value:?}");
                return Err(ParseError::UnclosedString);
            }
        };
        let value = value.replace("\"\"", "\"");
        let value = value.replace("\\\\", "\\");
        // TODO: If I collapse the double percent signs now, it will be ambiguous
        // whether it is supposed to be string subsitution or an escaped percent sign.
        // let value = value.replace("%%", "%");
        values.push(value);
        println!("{values:#?}");

        if values.len() > 1 {
            return Ok(Entry::ValueOnly(Value::List(values)));
        } else if let Some(value) = values.first() {
            return Ok(Entry::ValueOnly(Value::Raw(value.to_owned())));
        }
    }

    let mut within_quotes = false;
    let mut values = Vec::<String>::new();
    let mut key = None::<String>;
    let mut start = 0usize;

    for (i, c) in line.char_indices() {
        match c {
            '"' => within_quotes = !within_quotes,
            ',' if !within_quotes => {
                if key.is_some() {
                    assert_ne!(start, 0);
                }

                // Process value
                let value = line[start..i].trim();
                let value = match (value.starts_with('"'), value.ends_with('"')) {
                    (true, true) => value[1..value.len() - 1].trim(),
                    (false, false) => value,
                    // TODO: Ensure that every string contains an opening AND closing double quote.
                    // TODO: Either properly test this, or remove this error.
                    _ => {
                        println!("Debug: {value:?}");
                        return Err(ParseError::UnclosedString);
                    }
                };
                let value = value.replace("\"\"", "\"");
                let value = value.replace("\\\\", "\\");
                // TODO: If I collapse the double percent signs now, it will be ambiguous
                // whether it is supposed to be string subsitution or an escaped percent sign.
                // let value = value.replace("%%", "%");
                values.push(value);
                // TODO: Do I need to handle if this is the end of the line?
                //  value1,value2,,
                start = i + 1;
            }
            '=' if !within_quotes => {
                if !values.is_empty() {
                    // NOTE: I am unsure whether this should be allowed or not: `"1+1=2",==,value3`
                    continue;
                }
                key = Some(line[..i].trim().to_owned());
                start = i + 1;
            }
            _ => {}
        }
    }

    // TODO: Process the value.
    let value = line[start..].trim();
    let value = match (value.starts_with('"'), value.ends_with('"')) {
        (true, true) => value[1..value.len() - 1].trim(),
        // TODO: Ensure that every string contains an opening AND closing double quote.
        // TODO: Either properly test this, or remove this error.
        (false, false) => value,
        _ => {
            println!("Debug: {value:?}");
            return Err(ParseError::UnclosedString);
        }
    };
    let value = value.replace("\"\"", "\"");
    let value = value.replace("\\\\", "\\");
    // TODO: If I collapse the double percent signs now, it will be ambiguous
    // whether it is supposed to be string subsitution or an escaped percent sign.
    // let value = value.replace("%%", "%");
    values.push(value);
    println!("Entries: {values:#?}");

    assert!(!values.is_empty());

    let value = if values.len() == 1 {
        Value::Raw(values.remove(0))
    } else {
        Value::List(values)
    };

    Ok(if let Some(key) = key {
        Entry::Item(key, value)
    } else {
        Entry::ValueOnly(value)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    Syntax { character: char },
    SectionNameEmpty,
    SectionNameTooLong,
    UnclosedString,
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Syntax { character } => {
                write!(f, "invalid syntax: unexpected character: {character}")
            }
            Self::SectionNameEmpty => "section name cannot be empty".fmt(f),
            Self::SectionNameTooLong => "section name cannot exceed 255 characters".fmt(f),
            Self::UnclosedString => {
                "expected string value to have double quotes on both ends".fmt(f)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiline_value_with_inline_comment() {
        let buffer = br#"
; This is a comment

[Section]
key = value1,"value2;not-a-comment"\ ; This is an inline comment.
,value3,,value5
"#;

        let inf = Inf::parse(buffer).expect("failed to parse hardcoded INF file");

        assert_eq!(
            inf.sections.get("Section"),
            Some(&vec![Entry::Item(
                "key".to_owned(),
                Value::Raw("value1,\"value2;not-a-comment\",value3,,value5".to_owned())
            )])
        );
    }

    #[test]
    fn multiple_keys_and_values() {
        let buffer = br"
[Section]
key1 = value1
key2 = value2
key3 = value3
";

        let inf = Inf::parse(buffer).expect("failed to parse hardcoded INF file");

        assert_eq!(
            inf.sections.get("Section"),
            Some(&vec![
                Entry::Item("key1".to_owned(), Value::Raw("value1".to_owned())),
                Entry::Item("key2".to_owned(), Value::Raw("value2".to_owned())),
                Entry::Item("key3".to_owned(), Value::Raw("value3".to_owned())),
            ])
        );
    }

    #[test]
    fn multiple_sections() {
        let buffer = br#"
[Version] ; This section is typically required.
Signature = "$CHICAGO$"

[Section]
key = value
"#;

        let inf = Inf::parse(buffer).expect("failed to parse hardcoded INF file");

        assert_eq!(
            inf.sections.get("Version"),
            Some(&vec![Entry::Item(
                "Signature".to_owned(),
                Value::Raw("$CHICAGO$".to_owned())
            ),])
        );
        assert_eq!(
            inf.sections.get("Section"),
            Some(&vec![Entry::Item(
                "key".to_owned(),
                Value::Raw("value".to_owned())
            ),])
        );
    }

    #[test]
    fn quoted_value_with_equal() {
        let buffer = br#"
[Section]
"1+1=2"
"#;

        let inf = Inf::parse(buffer).expect("failed to parse hardcoded INF file");

        assert_eq!(
            inf.sections.get("Section"),
            Some(&vec![Entry::ValueOnly(Value::Raw("1+1=2".to_owned()))])
        );
    }

    #[test]
    fn various_entry_kinds() {
        parse_section_entry("\"1+1=2\",foo,,bar").expect("expected hardcoded value to pass");
        parse_section_entry("key = value").expect("expected hardcoded value to pass");
        parse_section_entry("key2=value2").expect("expected hardcoded value to pass");
        parse_section_entry("key3 = \"value3\"").expect("expected hardcoded value to pass");
    }

    #[test]
    fn hoshimachi_inf() {
        // Uses quotes strings in section entries.
        let buffer = include_bytes!("./Hoshimachi.inf");
        let inf = Inf::parse(buffer).expect("failed to parse Hoshimachi.inf");
        dbg!(&inf.sections);
    }

    #[test]
    fn novella_inf() {
        // Uses Windows' CRLF
        let buffer = include_bytes!("./Novella.inf");
        let inf = Inf::parse(buffer).expect("failed to parse Novella.inf");
        dbg!(&inf.sections);
    }

    #[test]
    fn hornet_inf() {
        // Uses unquotes strings in section entries.
        let buffer = include_bytes!("./Hornet.inf");
        let inf = Inf::parse(buffer).expect("failed to parse Hornet.inf");
        dbg!(&inf.sections);
    }
}
