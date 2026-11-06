#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Raw(String),
    List(Vec<String>),
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
