use std::error::Error;
use std::fmt;

use crate::section::Section;
use crate::value::Value;

/// Substitutes [`strkey`] variables in `value` with those defined in the `Strings` section.
///
/// # Errors
///
/// Returns an error if a `strkey` variable is not terminated, the `Strings` section contains a
/// non-[`Entry::Item`][crate::entry::Entry::Item] value, or the `strkey` variable is not defined.
///
/// # Examples
///
/// ```
/// use inf::{Entry, Section, Value};
///
/// let input = "Hello, %name%!";
/// let strings = Section::new(
///     "Strings".to_owned(),
///     vec![Entry::Item("name".to_owned(), Value::Raw("Nic".to_owned()))]
/// )?;
/// let expanded = inf::expand(input, &strings)?;
///
/// assert_eq!(expanded, "Hello, Nic!".to_owned());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// [`strkey`]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/inf-strings-section
pub fn expand(value: &str, strings: &Section) -> Result<String, ExpandError> {
    let mut expanded = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('%', Some('%')) => {
                chars.next();
                expanded.push('%');
            }
            ('%', _) => {
                let mut key = String::new();

                loop {
                    match chars.next() {
                        Some('%') => break,
                        Some(c) => key.push(c),
                        None => return Err(ExpandError::Unterminated),
                    }
                }

                let replacement = match strings.get_entry(key.as_ref()) {
                    Some(Value::Raw(value)) => value,
                    Some(Value::List(_)) => return Err(ExpandError::InvalidValue { key }),
                    None => return Err(ExpandError::NotFound { key }),
                };

                expanded.push_str(replacement);
            }
            _ => expanded.push(c),
        }
    }

    Ok(expanded)
}

/// Errors that can occur while expanding `strkey` variables.
#[derive(Debug)]
#[non_exhaustive]
pub enum ExpandError {
    /// strkey not found.
    NotFound {
        /// Missing key.
        key: String,
    },
    /// Value kind must be [`Value::Raw`].
    InvalidValue {
        /// Key containing an invalid [`Value`].
        key: String,
    },
    /// Missing closing percent sign for `%strkey%`.
    Unterminated,
}

impl Error for ExpandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl fmt::Display for ExpandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NotFound { ref key } => write!(f, "key not found in Strings section: {key}"),
            Self::InvalidValue { ref key } => {
                write!(f, "invalid value kind for key in Strings: {key}")
            }
            Self::Unterminated => "unterminated strkey sequence".fmt(f),
        }
    }
}
