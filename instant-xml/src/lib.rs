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
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error>;
}

impl<'a, T: ToXml + ?Sized> ToXml for &'a T {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        (*self).serialize(field, serializer)
    }
}

pub trait FromXml<'xml>: Sized {
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool;

    fn deserialize<'cx>(
        deserializer: &mut Deserializer<'cx, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error>;

    // If the missing field is of type `Option<T>` then treat is as `None`,
    // otherwise it is an error.
    fn missing_value() -> Result<Self, Error> {
        Err(Error::MissingValue(Self::KIND))
    }

    const KIND: Kind;
}

pub fn from_str<'xml, T: FromXml<'xml>>(input: &'xml str) -> Result<T, Error> {
    let (mut context, root) = Context::new(input)?;
    let id = context.element_id(&root)?;

    if !T::matches(id, None) {
        return Err(Error::UnexpectedValue("unexpected root"));
    }

    let mut value = None;
    T::deserialize(&mut Deserializer::new(root, &mut context), &mut value)?;
    match value {
        Some(value) => Ok(value),
        None => T::missing_value(),
    }
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
    value.serialize(None, &mut Serializer::new(output))
}

pub trait FromXmlOwned: for<'xml> FromXml<'xml> {}

impl<T> FromXmlOwned for T where T: for<'xml> FromXml<'xml> {}

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
    UnexpectedValue(&'static str),
    #[error("unexpected tag: {0}")]
    UnexpectedTag(String),
    #[error("missing tag")]
    MissingTag,
    #[error("missing value")]
    MissingValue(Kind),
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("unknown prefix: {0}")]
    UnknownPrefix(String),
    #[error("unexpected node: {0}")]
    UnexpectedNode(String),
    #[error("unexpected state: {0}")]
    UnexpectedState(&'static str),
    #[error("expected scalar, found {0}")]
    ExpectedScalar(String),
    #[error("duplicate value")]
    DuplicateValue,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Kind {
    Scalar,
    Element,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Id<'a> {
    pub ns: &'a str,
    pub name: &'a str,
}
