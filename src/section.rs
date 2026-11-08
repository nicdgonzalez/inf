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
    #[must_use]
    pub const fn new(name: String, entries: Vec<Entry>) -> Self {
        Self { name, entries }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn merge(&mut self, other: Section) {
        self.entries.extend(other.entries);
    }

    pub fn rename(&mut self, name: String) -> Result<(), RenameError> {
        match name.len() {
            0 => Err(RenameError::Empty),
            1..256 => {
                self.name = name;
                Ok(())
            }
            256.. => Err(RenameError::TooLong),
        }
    }

    pub fn add_entry(&mut self, entry: Entry) -> &mut Entry {
        self.entries.push_mut(entry)
    }

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

#[derive(Debug)]
pub enum RenameError {
    Empty,
    TooLong,
}

impl Error for RenameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl fmt::Display for RenameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Empty => "section name cannot be empty".fmt(f),
            Self::TooLong => "section name exceeds 255 bytes".fmt(f),
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
