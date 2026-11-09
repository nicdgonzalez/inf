use std::slice;

/// Kinds of values in an [`Entry`][crate::Entry].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Contains a single value.
    Raw(String),
    /// Contains multiple values.
    List(Vec<String>),
}

impl Value {
    /// Returns the inner value(s) as a slice. If `self` is [`Value::Raw`], returns the inner value
    /// as a slice with a length of 1.
    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        match self {
            Self::Raw(value) => slice::from_ref(value),
            Self::List(values) => values.as_slice(),
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Raw(value)
    }
}

impl From<Vec<String>> for Value {
    fn from(value: Vec<String>) -> Self {
        Self::List(value)
    }
}
