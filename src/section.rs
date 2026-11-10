use std::error::Error;
use std::fmt;
use std::str::FromStr;

use crate::entry::Entry;
use crate::parser::{ParseError, Parser};
use crate::value::Value;

/// Group of entries in an [`Inf`][crate::Inf].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    name: String,
    entries: Vec<Entry>,
}

impl Section {
    /// Creates a new [`Section`].
    ///
    /// # Errors
    ///
    /// Returns an error if `name` is empty or exceeds 255 bytes.
    pub fn new(name: String, entries: Vec<Entry>) -> Result<Self, SectionError> {
        Self::validate_name(&name)?;
        Ok(Self { name, entries })
    }

    pub(crate) fn validate_name(name: &str) -> Result<(), SectionError> {
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
            0 => Err(SectionError::Empty),
            1..256 => Ok(()),
            length => Err(SectionError::TooLong { length }),
        }
    }

    /// Returns a reference to this section's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Updates the name of this section.
    ///
    /// # Errors
    ///
    /// Returns an error if `name` is empty or exceeds 255 bytes.
    pub fn rename(&mut self, name: String) -> Result<(), SectionError> {
        Self::validate_name(&name)?;
        self.name = name;
        Ok(())
    }

    /// Returns a reference to this section's entries.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Appends all of the entries from `other` to this section in order.
    pub fn merge(&mut self, other: Section) {
        self.entries.extend(other.entries);
    }

    /// Appends `entry` to the section and returns a mutable reference to it.
    pub fn add_entry(&mut self, entry: Entry) -> &mut Entry {
        self.entries.push_mut(entry)
    }

    /// Searches for an entry using the provided `query`. Queries can be indices or key names.
    /// Indices match any entry, while key names match only [`Entry::Item`] entries.
    pub fn get_entry<'a>(&self, query: impl Into<Query<'a>>) -> Option<&Value> {
        match query.into() {
            Query::Name(name) => self.get_entry_by_name(name),
            Query::Index(index) => self.get_entry_by_index(index),
        }
    }

    fn get_entry_by_name(&self, name: &str) -> Option<&Value> {
        self.entries
            .iter()
            .find(|&entry| matches!(entry, Entry::Item(key, _) if key.eq_ignore_ascii_case(name)))
            .and_then(|entry| match entry {
                Entry::Item(_, value) => Some(value),
                Entry::ValueOnly(..) => None,
            })
    }

    fn get_entry_by_index(&self, index: usize) -> Option<&Value> {
        self.entries.get(index).map(|entry| match entry {
            Entry::Item(_, value) | Entry::ValueOnly(value) => value,
        })
    }

    /// Converts this section into a collection of validated [`AddRegistryEntry`] values.
    ///
    /// # Errors
    ///
    /// Returns an error if an entry has an unexpected kind or contains fewer than five elements.
    ///
    /// Each entry must be a `Value::List` containing at least five elements, corresponding to
    /// the registry root, subkey, entry name, flags, and value, respectively. Remaining elements
    /// are preserved as additional arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::FromStr as _;
    /// use inf::{Section, InvalidAddRegistryEntry};
    ///
    /// let section = Section::from_str(r#"
    ///     [Section]
    ///     HKCU,"Control Panel\Cursors\Schemes","%SCHEME_NAME%",,"%10%\%CUR_DIR%\Default.ani"
    /// "#)?;
    ///
    /// let entries = section.as_add_registry_section().collect::<Result<Vec<_>, InvalidAddRegistryEntry>>()?;
    /// let entry = entries.first().unwrap();
    ///
    /// assert_eq!(entry.root, "HKCU");
    /// assert_eq!(entry.subkey, r"Control Panel\Cursors\Schemes");
    /// assert_eq!(entry.entry_name, "%SCHEME_NAME%");
    /// assert_eq!(entry.flags, "");
    /// assert_eq!(entry.value, r"%10%\%CUR_DIR%\Default.ani");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn as_add_registry_section(
        &self,
    ) -> impl Iterator<Item = Result<AddRegistryEntry<'_>, InvalidAddRegistryEntry>> {
        self.entries.iter().enumerate().map(|(index, entry)| {
            let Entry::ValueOnly(Value::List(values)) = entry else {
                return Err(InvalidAddRegistryEntry::EntryKind { index });
            };

            let [root, subkey, entry_name, flags, value, additional @ ..] = values.as_slice()
            else {
                return Err(InvalidAddRegistryEntry::NotEnoughArguments);
            };

            Ok(AddRegistryEntry {
                root,
                subkey,
                entry_name,
                flags,
                value,
                additional,
            })
        })
    }
}

impl FromStr for Section {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Parser::new(s).parse_section()
    }
}

/// Errors that can occur while creating a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SectionError {
    /// Section name cannot be empty.
    Empty,
    /// Section name exceeds 255 bytes.
    TooLong {
        /// Length of section name.
        length: usize,
    },
}

impl Error for SectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl fmt::Display for SectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Empty => "section name cannot be empty".fmt(f),
            Self::TooLong { length } => {
                write!(f, "section name exceeds 255 bytes ({length} > 255)")
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Query<'a> {
    Name(&'a str),
    Index(usize),
}

impl<'a> From<&'a str> for Query<'a> {
    fn from(value: &'a str) -> Self {
        Self::Name(value)
    }
}

impl From<usize> for Query<'_> {
    fn from(value: usize) -> Self {
        Self::Index(value)
    }
}

/// Entry under an `AddReg`-specified section that is known to be valid.
///
/// Sections specified by the `AddReg` directive must follow a particular structure, allowing us to
/// make assumptions about their entries. This is a convenience type that simplifies working with
/// such entries.
#[expect(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddRegistryEntry<'a> {
    pub root: &'a str,
    pub subkey: &'a str,
    pub entry_name: &'a str,
    pub flags: &'a str,
    pub value: &'a str,
    pub additional: &'a [String],
}

/// Errors that can occur while validating entries in an `AddReg` section.
#[derive(Debug)]
#[non_exhaustive]
pub enum InvalidAddRegistryEntry {
    /// Entry must be [`Entry::Item`].
    EntryKind {
        /// Entry index.
        index: usize,
    },
    /// Entries must have 5 or more values.
    NotEnoughArguments,
}

impl Error for InvalidAddRegistryEntry {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl fmt::Display for InvalidAddRegistryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::EntryKind { index } => write!(f, "entry #{index} has an invalid entry kind"),
            Self::NotEnoughArguments => "expected 5 or more comma-delimited values".fmt(f),
        }
    }
}
