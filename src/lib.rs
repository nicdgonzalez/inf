#![doc = include_str!("../README.md")]
#![warn(
    clippy::correctness,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::style,
    clippy::pedantic
)]
#![allow(
    clippy::missing_errors_doc,
    reason = "to avoid breaking CI; future commit will add documentation"
)]

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;
use std::string::FromUtf16Error;
use std::sync::LazyLock;
use std::{fmt, io};

use encoding_rs::WINDOWS_1252;

pub use crate::entry::Entry;
use crate::parser::{ParseError, Parser};
pub use crate::section::Section;
pub use crate::value::Value;

mod cursor;
mod entry;
mod lexer;
mod parser;
mod section;
mod span;
mod token;
mod value;

/// Byte Order Mark (BOM) is used to signal the endianness of an encoding. The order `FF FE`
/// suggests that the data was encoded using the little-endian byte order.
///
/// <https://en.wikipedia.org/wiki/Byte_order_mark>
const BOM_LE: [u8; 2] = [0xFF, 0xFE];

/// Parsed INF-formatted buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inf {
    sections: Vec<Section>,
}

impl Inf {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, InfError> {
        File::open(path.as_ref())
            .map_err(|source| InfError::Open { source })
            .and_then(Self::from_reader)
    }

    pub fn from_reader(mut reader: impl Read) -> Result<Self, InfError> {
        let mut buffer = Vec::new();
        _ = reader
            .read_to_end(&mut buffer)
            .map_err(|source| InfError::Read { source })?;

        Self::from_bytes(&buffer)
    }

    pub fn from_bytes(value: &[u8]) -> Result<Self, InfError> {
        if value.starts_with(&BOM_LE) {
            Self::from_utf16le_lossy(value)
        } else {
            Self::from_windows_1252_lossy(value)
        }
    }

    pub fn from_utf16le(data: &[u8]) -> Result<Self, InfError> {
        let value = data.strip_prefix(&BOM_LE).unwrap_or(data);
        String::from_utf16le(value)
            .map_err(|source| InfError::InvalidUtf16Le { source })?
            .parse()
    }

    pub fn from_utf16le_lossy(data: &[u8]) -> Result<Self, InfError> {
        let value = data.strip_prefix(&BOM_LE).unwrap_or(data);
        String::from_utf16le_lossy(value).parse()
    }

    pub fn from_windows_1252(data: &[u8]) -> Result<Self, InfError> {
        // encoding_rs follows the WHATWG Encoding Standard, which maps Windows 1252's undefined
        // code points to C1 control code points rather than treating them as decoding errors.
        // Because this parser enforces strict input validation, these bytes must be explicitly
        // rejected prior to being passed to encoding_rs for decoding.
        //
        // The INF file format specification does not define how these bytes should be handled;
        // rejecting these bytes is a choice made by this particular parser implementation.
        //
        // https://encoding.spec.whatwg.org/windows-1252.html
        // https://en.wikipedia.org/wiki/C0_and_C1_control_codes#C1_controls
        if Self::contains_undefined_windows_1252_byte(data) {
            return Err(InfError::InvalidWindows1252);
        }

        #[expect(clippy::missing_panics_doc, reason = "irrelevant to caller")]
        WINDOWS_1252
            .decode_without_bom_handling_and_without_replacement(data)
            .expect("operation is infallible for this encoding")
            .parse()
    }

    fn contains_undefined_windows_1252_byte(data: &[u8]) -> bool {
        data.iter()
            .copied()
            .any(Self::is_undefined_windows_1252_code_point)
    }

    fn is_undefined_windows_1252_code_point(byte: u8) -> bool {
        // The Windows 1252 code page has five undefined code points.
        // https://en.wikipedia.org/wiki/Windows-1252#Code_page_layout
        matches!(byte, 0x81 | 0x8D | 0x8F | 0x90 | 0x9D)
    }

    pub fn from_windows_1252_lossy(data: &[u8]) -> Result<Self, InfError> {
        WINDOWS_1252.decode_without_bom_handling(data).0.parse()
    }

    #[must_use]
    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    #[must_use]
    pub fn get_section(&self, name: &str) -> Option<&Section> {
        self.sections
            .iter()
            .find(|section| section.name().eq_ignore_ascii_case(name))
    }

    pub fn get_section_mut(&mut self, name: &str) -> Option<&mut Section> {
        self.sections
            .iter_mut()
            .find(|section| section.name().eq_ignore_ascii_case(name))
    }

    pub fn add_section(&mut self, section: Section) -> &mut Section {
        self.sections.push_mut(section)
    }

    #[must_use]
    pub fn strings(&self) -> &Section {
        static EMPTY_STRINGS: LazyLock<Section> =
            LazyLock::new(|| Section::new("Strings".to_owned(), vec![]));

        self.get_section("Strings").unwrap_or(&*EMPTY_STRINGS)
    }
}

impl FromStr for Inf {
    type Err = InfError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Parser::new(s)
            .parse()
            .map_err(|source| InfError::Parse { source })
    }
}

/// Errors that can occur while parsing an INF-formatted buffer.
#[derive(Debug)]
pub enum InfError {
    Open { source: io::Error },
    Read { source: io::Error },
    InvalidUtf16Le { source: FromUtf16Error },
    InvalidWindows1252,
    Parse { source: ParseError },
}

impl Error for InfError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            Self::Open { ref source } | Self::Read { ref source } => Some(source),
            Self::InvalidUtf16Le { ref source } => Some(source),
            Self::InvalidWindows1252 => None,
            Self::Parse { ref source } => Some(source),
        }
    }
}

impl fmt::Display for InfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Open { source: _ } => "failed to open file".fmt(f),
            Self::Read { source: _ } => "failed to read data".fmt(f),
            Self::InvalidUtf16Le { source: _ } => {
                "data contains invalid UTF-16 LE code points".fmt(f)
            }
            Self::InvalidWindows1252 => "data contains invalid Windows 1252 code points".fmt(f),
            Self::Parse { source: _ } => "failed to parse buffer".fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Converts a list of u16 code points to Little-Endian bytes.
    ///
    /// This helps make tests more readable, as code points are typically described
    /// in Big-Endian format.
    fn to_le_bytes(code_points: &[u16]) -> Vec<u8> {
        code_points
            .iter()
            .copied()
            .flat_map(u16::to_le_bytes)
            .collect()
    }

    #[test]
    fn invalid_utf16le() {
        // U+D800 is a lone high surrogate (missing its low partner)
        let data = to_le_bytes(&[0xD800, 0x0000]);
        let error = Inf::from_utf16le(&data).unwrap_err();
        assert!(matches!(error, InfError::InvalidUtf16Le { .. }));
    }

    #[test]
    fn invalid_windows_1252() {
        // The Windows 1252 code page has 5 undefined code points.
        // https://en.wikipedia.org/wiki/Windows-1252#Code_page_layout
        let undefined_code_points = [[0x81], [0x8D], [0x8F], [0x90], [0x9D]];

        for data in undefined_code_points {
            let error = Inf::from_windows_1252(&data).unwrap_err();
            assert!(matches!(error, InfError::InvalidWindows1252));
        }
    }
}
