use std::fmt;

use thiserror::Error;

pub use macros::{FromXml, ToXml};

#[doc(hidden)]
pub mod de;
mod impls;
pub use de::Deserializer;
#[doc(hidden)]
pub mod ser;
pub use ser::Serializer;

pub trait ToXml {
    fn to_xml(&self) -> Result<String, Error> {
        let mut output = String::new();
        let mut serializer = Serializer::new(&mut output);
        self.serialize(&mut serializer)?;
        Ok(output)
    }

    fn serialize<W: fmt::Write>(&self, serializer: &mut Serializer<W>) -> Result<(), Error>;
}

pub enum FieldAttribute<'xml> {
    Prefix(&'xml str),
    Namespace(&'xml str),
    Attribute,
}

pub trait FromXml<'xml>: Sized {
    fn deserialize(deserializer: &mut Deserializer<'xml>) -> Result<Self, Error>;

    // If the missing field is of type `Option<T>` then treat is as `None`,
    // otherwise it is an error.
    fn missing_value() -> Result<Self, Error> {
        Err(Error::MissingValue)
    }

    const KIND: Kind;
}

pub fn from_str<'xml, T: FromXml<'xml>>(input: &'xml str) -> Result<T, Error> {
    T::deserialize(&mut Deserializer::new(input))
}

pub enum Kind {
    Scalar,
    Element(Id<'static>),
}

impl Kind {
    pub const fn name<'a>(&self, field: Id<'static>) -> Id<'static> {
        match self {
            Kind::Scalar => field,
            Kind::Element(name) => *name,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Id<'a> {
    pub ns: &'a str,
    pub name: &'a str,
}

pub trait FromXmlOwned: for<'xml> FromXml<'xml> {}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum Error {
    #[error("format: {0}")]
    Format(#[from] fmt::Error),
    #[error("parse: {0}")]
    Parse(#[from] xmlparser::Error),
    #[error("other: {0}")]
    Other(std::string::String),
    #[error("unexpected end of stream")]
    UnexpectedEndOfStream,
    #[error("unexpected value")]
    UnexpectedValue,
    #[error("unexpected tag")]
    UnexpectedTag,
    #[error("missing tag")]
    MissingTag,
    #[error("missing value")]
    MissingValue,
    #[error("unexpected token")]
    UnexpectedToken,
    #[error("missing prefix")]
    MissingdPrefix,
    #[error("unexpected state")]
    UnexpectedState,
    #[error("wrong namespace")]
    WrongNamespace,
}
