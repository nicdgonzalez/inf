use crate::value::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Item(String, Value),
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
