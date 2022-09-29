use std::fmt;

use thiserror::Error;

pub use macros::{FromXml, ToXml};

#[doc(hidden)]
pub mod de;
mod impls;
use de::Context;
pub use de::Deserializer;
#[doc(hidden)]
pub mod ser;
pub use ser::Serializer;

pub trait ToXml {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error>;

    const KIND: Kind;
}

pub trait FromXml<'xml>: Sized {
    fn deserialize<'cx>(deserializer: &'cx mut Deserializer<'cx, 'xml>) -> Result<Self, Error>;

    // If the missing field is of type `Option<T>` then treat is as `None`,
    // otherwise it is an error.
    fn missing_value() -> Result<Self, Error> {
        Err(Error::MissingValue)
    }

    const KIND: Kind;
}

pub fn from_str<'xml, T: FromXml<'xml>>(input: &'xml str) -> Result<T, Error> {
    let (mut context, root) = Context::new(input)?;
    let id = context.element_id(&root)?;
    let expected = match T::KIND {
        Kind::Scalar => return Err(Error::UnexpectedState),
        Kind::Vec => return Err(Error::UnexpectedState),
        Kind::Element(expected) => expected,
    };

    if id != expected {
        return Err(Error::UnexpectedValue);
    }

    T::deserialize(&mut Deserializer::new(root, &mut context))
}

pub fn to_string(value: &(impl ToXml + ?Sized)) -> Result<String, Error> {
    let mut output = String::new();
    to_writer(value, &mut output)?;
    Ok(output)
}

pub fn to_writer(
    value: &(impl ToXml + ?Sized),
    output: &mut (impl fmt::Write + ?Sized),
) -> Result<(), Error> {
    value.serialize(&mut Serializer::new(output))
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
    #[error("expected scalar")]
    ExpectedScalar,
    #[error("wrong namespace")]
    WrongNamespace,
    #[error("duplicate value")]
    DuplicateValue,
}

pub enum Kind {
    Scalar,
    Element(Id<'static>),
    Vec,
}

impl Kind {
    pub const fn name(&self, field: Id<'static>) -> Id<'static> {
        match self {
            Kind::Scalar => field,
            Kind::Element(name) => *name,
            Kind::Vec => field,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Id<'a> {
    pub ns: &'a str,
    pub name: &'a str,
}
