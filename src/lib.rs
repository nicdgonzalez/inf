#![doc = include_str!("../README.md")]
#![warn(
    clippy::correctness,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::style,
    clippy::pedantic
)]

use std::str::FromStr;

pub use crate::entry::Entry;
use crate::parser::{ParseError, Parser};
pub use crate::section::Section;
pub use crate::value::Value;

mod cursor;
mod entry;
mod lexer;
mod parser;
mod section;
mod span;
mod token;
mod value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inf {
    sections: Vec<Section>,
}

impl FromStr for Inf {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Parser::new(s).parse()
    }
}
