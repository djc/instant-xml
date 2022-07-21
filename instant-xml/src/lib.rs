use std::collections::{BTreeSet, HashMap};
use std::fmt;

use thiserror::Error;
pub use xmlparser;

pub use macros::{FromXml, ToXml};
use parse::XmlParser;

pub mod impls;
#[doc(hidden)]
pub mod parse;

pub struct TagData {
    pub attributes: Option<Vec<String>>,
    pub key: String,
}

pub enum XmlRecord {
    Open(TagData),
    Element(String),
    Close(String),
}

pub trait ToXml {
    fn write_xml<W: fmt::Write>(
        &self,
        write: &mut W,
        parent_prefixes: Option<&mut BTreeSet<&str>>,
    ) -> Result<()>;

    fn to_xml(&self, parent_prefixes: Option<&mut BTreeSet<&str>>) -> Result<String> {
        let mut out = String::new();
        self.write_xml(&mut out, parent_prefixes)?;
        Ok(out)
    }
}

macro_rules! to_xml_for_type {
    ($typ:ty) => {
        impl ToXml for $typ {
            fn write_xml<W: fmt::Write>(
                &self,
                _write: &mut W,
                _parent_prefixes: Option<&mut BTreeSet<&str>>,
            ) -> Result<()> {
                Ok(())
            }

            fn to_xml(&self, parent_prefixes: Option<&mut BTreeSet<&str>>) -> Result<String> {
                let mut out = self.to_string();
                self.write_xml(&mut out, parent_prefixes)?;
                Ok(out)
            }
        }
    };
}

to_xml_for_type!(bool);
to_xml_for_type!(i8);
to_xml_for_type!(i16);
to_xml_for_type!(i32);
to_xml_for_type!(String);

pub trait FromXml<'xml>: Sized {
    fn from_xml(input: &str) -> Result<Self> {
        let mut xml_parser = XmlParser::new(input);
        let mut deserializer = Deserializer {
            iter: &mut xml_parser,
        };

        Self::deserialize(&mut deserializer)
    }

    fn deserialize<D>(deserializer: &mut D) -> Result<Self>
    where
        D: DeserializeXml<'xml>;
}

pub trait DeserializeXml<'xml>: Sized {
    fn deserialize_bool<V>(&mut self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'xml>,
    {
        unimplemented!();
    }

    fn deserialize_struct<'a, V>(&mut self, _visitor: V, _name: &str) -> Result<V::Value>
    where
        V: Visitor<'xml>,
    {
        unimplemented!();
    }

    // TODO: Consider this with generic XmlRecord
    fn peek_next_tag(&mut self) -> Result<Option<XmlRecord>> {
        unimplemented!();
    }
}

pub trait Visitor<'xml>: Sized {
    type Value;

    fn visit_str(self, _value: &str) -> Result<Self::Value> {
        unimplemented!();
    }

    fn visit_struct<'a, D>(&self, _deserializer: &mut D) -> Result<Self::Value>
    where
        D: DeserializeXml<'xml>,
    {
        unimplemented!();
    }
}

pub struct Deserializer<'xml> {
    pub iter: &'xml mut XmlParser<'xml>,
}

impl<'xml> Deserializer<'xml> {
    fn check_open_tag(&mut self, name: &str) -> Result<()> {
        if let Some(item) = self.iter.next() {
            match item? {
                XmlRecord::Open(v) if v.key == name => Ok(()),
                _ => Err(Error::UnexpectedValue),
            }
        } else {
            Err(Error::UnexpectedTag)
        }
    }

    fn check_close_tag(&mut self, name: &str) -> Result<()> {
        // Close tag
        if let Some(item) = self.iter.next() {
            match item? {
                XmlRecord::Close(v) if v == name => Ok(()),
                _ => Err(Error::UnexpectedTag),
            }
        } else {
            Err(Error::MissingTag)
        }
    }
}

impl<'xml, 'a> DeserializeXml<'xml> for Deserializer<'a> {
    fn deserialize_bool<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'xml>,
    {
        self.iter.next();
        if let Some(item) = self.iter.next() {
            match item? {
                XmlRecord::Element(v) => {
                    let ret = visitor.visit_str(v.as_str());
                    self.iter.next();
                    ret
                }
                _ => Err(Error::UnexpectedTag),
            }
        } else {
            Err(Error::MissingValue)
        }
    }

    // TODO: Validate if other types were already used, tab of &str
    fn deserialize_struct<'b, V>(&mut self, visitor: V, name: &str) -> Result<V::Value>
    where
        V: Visitor<'xml>,
    {
        self.check_open_tag(name)?;
        let ret = visitor.visit_struct(self)?;
        self.check_close_tag(name)?;
        Ok(ret)
    }

    fn peek_next_tag(&mut self) -> Result<Option<XmlRecord>> {
        self.iter.peek_next_tag()
    }
}

pub trait FromXmlOwned: for<'xml> FromXml<'xml> {}

#[allow(dead_code)]
struct State<'a> {
    prefix: HashMap<&'a str, &'a str>,
}

pub type Result<T> = core::result::Result<T, Error>;

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
    #[error("unexpected tag")]
    UnexpectedTag,
    #[error("missing tag")]
    MissingTag,
    #[error("missing value")]
    MissingValue,
    #[error("unexpected token")]
    UnexpectedToken,
}
