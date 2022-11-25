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

    const KIND: Kind<'static>;
}

impl<'a, T: ToXml + ?Sized> ToXml for &'a T {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        (*self).serialize(serializer)
    }

    const KIND: Kind<'static> = T::KIND;
}

pub trait FromXml<'xml>: Sized {
    fn deserialize<'cx>(
        deserializer: &'cx mut Deserializer<'cx, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error>;

    // If the missing field is of type `Option<T>` then treat is as `None`,
    // otherwise it is an error.
    fn missing_value() -> Result<Self, Error> {
        Err(Error::MissingValue(&Self::KIND))
    }

    const KIND: Kind<'static>;
}

pub fn from_str<'xml, T: FromXml<'xml>>(input: &'xml str) -> Result<T, Error> {
    let (mut context, root) = Context::new(input)?;
    let id = context.element_id(&root)?;
    let expected = match T::KIND {
        Kind::Scalar => return Err(Error::UnexpectedState("found scalar as root")),
        Kind::Vec(_) => return Err(Error::UnexpectedState("found list as root")),
        Kind::Element(expected) => expected,
    };

    if id != expected {
        return Err(Error::UnexpectedValue);
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
    value.serialize(&mut Serializer::new(output))
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
    UnexpectedValue,
    #[error("unexpected tag")]
    UnexpectedTag,
    #[error("missing tag")]
    MissingTag,
    #[error("missing value")]
    MissingValue(&'static Kind<'static>),
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("missing prefix")]
    MissingdPrefix,
    #[error("unexpected node: {0}")]
    UnexpectedNode(String),
    #[error("unexpected state: {0}")]
    UnexpectedState(&'static str),
    #[error("expected scalar")]
    ExpectedScalar,
    #[error("wrong namespace")]
    WrongNamespace,
    #[error("duplicate value")]
    DuplicateValue,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Kind<'a> {
    Scalar,
    Element(Id<'a>),
    Vec(Id<'a>),
}

impl<'a> Kind<'a> {
    pub const fn element(&self) -> Id<'a> {
        match self {
            Kind::Element(id) => *id,
            _ => panic!("expected element kind"),
        }
    }

    pub const fn name(&self, field: Id<'a>) -> Id<'a> {
        match self {
            Kind::Scalar => field,
            Kind::Element(name) => *name,
            Kind::Vec(inner) => *inner,
        }
    }

    #[inline]
    pub fn matches(&self, id: Id<'_>, field: Id<'_>) -> bool {
        match self {
            Kind::Scalar => id == field,
            Kind::Element(name) => id == *name,
            Kind::Vec(inner) => id == *inner,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Id<'a> {
    pub ns: &'a str,
    pub name: &'a str,
}
