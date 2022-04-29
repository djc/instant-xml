use std::fmt;

use thiserror::Error;

pub use macros::ToXml;

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

#[derive(Debug, Error)]
pub enum Error {
    #[error("format: {0}")]
    Format(#[from] fmt::Error),
}
