use std::fmt;
use std::collections::HashMap;

pub use xmlparser as xmlparser;
use thiserror::Error;

pub use macros::{FromXml, ToXml};

#[doc(hidden)]
pub mod parse;

pub trait ToXml {
    fn write_xml<W: fmt::Write>(&self, write: &mut W) -> Result<(), Error>;

    fn to_xml(&self) -> Result<String, Error> {
        let mut out = String::new();
        self.write_xml(&mut out)?;
        Ok(out)
    }
}

pub trait FromXml<'xml>: Sized {
    fn from_xml(input: &str) -> Result<Self, Error>;
}

pub trait FromXmlOwned: for<'xml> FromXml<'xml> {}

struct State<'a> {
    prefix: HashMap<&'a str, &'a str>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("format: {0}")]
    Format(#[from] fmt::Error),
    #[error("parse: {0}")]
    Parse(#[from] xmlparser::Error),
    #[error("unexpected end of stream")]
    UnexpectedEndOfStream,
    #[error("unexpected value")]
    UnexpectedValue,
}
