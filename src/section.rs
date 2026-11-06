use crate::entry::Entry;

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
}
