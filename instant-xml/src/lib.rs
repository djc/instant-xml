use std::collections::{BTreeSet, HashMap};
use std::fmt;

use thiserror::Error;
pub use xmlparser;

pub use macros::{FromXml, ToXml};
use parse::{XmlParser, XmlRecord};

#[doc(hidden)]
pub mod parse;
pub mod impls;

pub trait ToXml {
    fn write_xml<W: fmt::Write>(
        &self,
        write: &mut W,
        parent_prefixes: Option<&mut BTreeSet<&str>>,
    ) -> Result<(), Error>;

    fn to_xml(&self, parent_prefixes: Option<&mut BTreeSet<&str>>) -> Result<String, Error> {
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
            ) -> Result<(), Error> {
                Ok(())
            }

            fn to_xml(
                &self,
                parent_prefixes: Option<&mut BTreeSet<&str>>,
            ) -> Result<String, Error> {
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
    fn from_xml<'a>(input: &'a str) -> Result<Self, Error> 
    {
        let mut xml_parser = XmlParser::from_str(input);
        let mut deserializer = Deserializer {
            iter: &mut xml_parser,
        };

        deserializer.iter.next();
        Ok(Self::deserialize(&mut deserializer)?)
    }

    fn deserialize<D>(deserializer: &mut D) -> Result<Self, Error>
    where
        D: DeserializeXml<'xml>;
}

pub trait DeserializeXml<'xml>: Sized {
    fn deserialize<D>(deserializer: D) -> Result<Self, Error>
    where
        D: FromXml<'xml>;

    fn deserialize_bool<V>(&mut self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'xml> 
    {
        unimplemented!();
    }

    fn deserialize_struct<'a, V>(&mut self, _visitor: V, _name: &str) -> Result<V::Value, Error>
    where
        V: Visitor<'xml>,
    {
        unimplemented!();
    }
}

pub trait Visitor<'xml>: Sized {
    type Value;

    fn visit_str<'a>(self, _value: &str) -> Result<Self::Value, Error> {
        unimplemented!();
    }

    fn visit_struct<'a>(&self, _deserializer: &mut Deserializer) -> Result<Self::Value, Error>
    {
        unimplemented!();
    }
}

pub struct Deserializer<'a> {
    pub iter:  &'a mut XmlParser<'a>,
}

impl<'xml> FromXml<'xml> for Deserializer<'xml> {
    fn from_xml<'a>(_input: &'a str) -> Result<Self, Error> 
    {
        unimplemented!();
    }

    fn deserialize<D>(_deserializer: &mut D) -> Result<Self, Error>
    where
        D: DeserializeXml<'xml>,
    {
        unimplemented!();
    }
}

impl<'xml,'a> DeserializeXml<'xml> for &mut Deserializer<'a> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, Error>
    where
        D: FromXml<'xml> 
    {
        unimplemented!();
    }
}

impl<'xml,'a> DeserializeXml<'xml> for Deserializer<'a> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, Error>
    where
        D: FromXml<'xml> 
    {
        unimplemented!();
    }

    fn deserialize_bool<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'xml> 
    {
        if let Some(item) = self.iter.next() {
            match item {
                XmlRecord::Element(v) => visitor.visit_str(v.as_str()),
                _ => panic!("Wrong token type"),
            }
        } else {
            panic!("No element");
        }
    }

    // TODO: Validate if other types were already used, tab of &str
    fn deserialize_struct<'b, V>(&mut self, visitor: V, _name: &str) -> Result<V::Value, Error>
    where
        V: Visitor<'xml>,
    {
        // if let Some(item) = self.iter.next() {
        //     match item {
        //         XmlRecord::Open(item) if item.key.as_ref().unwrap() == name => {
        //             visitor.visit_struct(self)
        //         },
        //         _ => panic!("Wrong token type"),
        //     }
        // } else {
        //     panic!("No element");
        // }

        visitor.visit_struct(self)
    }
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
