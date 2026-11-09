use std::error::Error;
use std::fmt;

use crate::entry::Entry;
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SectionError {
    Empty,
    TooLong { length: usize },
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
