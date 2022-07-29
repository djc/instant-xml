use std::collections::{BTreeSet, HashMap};
use std::fmt;

use thiserror::Error;
pub use xmlparser;

pub use macros::{FromXml, ToXml};

#[doc(hidden)]
pub mod parse;

pub trait ToXml {
    fn to_xml(&self) -> Result<String, Error> {
        let mut parent_prefixes = BTreeSet::new();
        let mut serializer = Serializer {
            parent_prefixes: &mut parent_prefixes,
        };
        self.serialize(&mut serializer)
    }

    fn serialize(&self, serializer: &mut Serializer) -> Result<String, Error>;

    fn write_xml<W: fmt::Write>(
        &self,
        _write: &mut W,
        _serializer: &mut Serializer,
    ) -> Result<(), Error> {
        unimplemented!();
    }
}

macro_rules! to_xml_for_number {
    ($typ:ty) => {
        impl ToXml for $typ {
            fn serialize(&self, _serializer: &mut Serializer) -> Result<String, Error> {
                Ok(self.to_string())
            }
        }
    };
}

to_xml_for_number!(i8);
to_xml_for_number!(i16);
to_xml_for_number!(i32);
to_xml_for_number!(i64);
to_xml_for_number!(u8);
to_xml_for_number!(u16);
to_xml_for_number!(u32);
to_xml_for_number!(u64);

pub struct Serializer<'xml> {
    pub parent_prefixes: &'xml mut BTreeSet<&'xml str>,
}

impl ToXml for bool {
    fn serialize(&self, _serializer: &mut Serializer) -> Result<String, Error> {
        let value = match self {
            true => "true",
            false => "false",
        };
        Ok(value.to_string())
    }
}

impl ToXml for String {
    fn serialize(&self, _serializer: &mut Serializer) -> Result<String, Error> {
        Ok((*self).clone())
    }
}

pub trait FromXml<'xml>: Sized {
    fn from_xml(input: &str) -> Result<Self, Error>;
}

pub trait FromXmlOwned: for<'xml> FromXml<'xml> {}

#[allow(dead_code)]
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
