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
    /// Returns the inner value(s) as a slice. If `self` is [`Value::Raw`], it returns
    /// the inner value as a slice with a length of 1.
    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        match self {
            Self::Raw(value) => slice::from_ref(value),
            Self::List(values) => values.as_slice(),
        }
    }

    /// Returns the inner value(s) as a mutable slice. If `self` is [`Value::Raw`], it returns
    /// the inner value as a slice with a length of 1.
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [String] {
        match self {
            Self::Raw(value) => slice::from_mut(value),
            Self::List(values) => values.as_mut_slice(),
        }
    }

    /// Returns an iterator over the slice.
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.into_iter()
    }

    /// Returns an iterator that allows modifying each value.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut String> {
        self.into_iter()
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

impl IntoIterator for Value {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Raw(value) => vec![value].into_iter(),
            Self::List(values) => values.into_iter(),
        }
    }
}

impl<'a> IntoIterator for &'a Value {
    type Item = &'a String;
    type IntoIter = slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'a> IntoIterator for &'a mut Value {
    type Item = &'a mut String;
    type IntoIter = slice::IterMut<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}
