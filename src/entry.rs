use crate::value::Value;

/// Kinds of entries in a [`Section`][crate::Section].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// Key-value pair.
    Item(String, Value),
    /// Key-less entry.
    ValueOnly(Value),
}

impl Entry {
    #[must_use]
    pub fn key(&self) -> Option<&str> {
        match self {
            Self::Item(key, ..) => Some(key.as_str()),
            Self::ValueOnly(..) => None,
        }
    }

    #[must_use]
    pub fn value(&self) -> &Value {
        match self {
            Self::Item(_, value) | Self::ValueOnly(value) => value,
        }
    }
}

impl From<(String, Value)> for Entry {
    fn from((key, value): (String, Value)) -> Self {
        Self::Item(key, value)
    }
}

impl From<Value> for Entry {
    fn from(value: Value) -> Self {
        Self::ValueOnly(value)
    }
}
