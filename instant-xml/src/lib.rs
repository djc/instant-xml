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
    pub key: String,
    pub attributes: Option<HashMap<String, String>>,
    pub default_namespace: Option<String>,
    pub namespaces: Option<HashMap<String, String>>,
    pub prefix: Option<String>,
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
        let mut prefixes_set = BTreeSet::new();
        let mut deserializer = Deserializer {
            iter: &mut xml_parser,
            prefixes: &mut prefixes_set,
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

    fn deserialize_struct<'b, V>(&mut self, _visitor: V, _name: &str, _prefixes: &mut BTreeSet<&str>)  -> Result<V::Value>
    where
        V: Visitor<'xml>,
    {
        unimplemented!();
    }
}

pub trait Visitor<'xml>: Sized {
    type Value;

    fn visit_str(self, _value: &str) -> Result<Self::Value> {
        unimplemented!();
    }

    fn visit_struct<'a, D>(
        &self,
        _deserializer: &mut D,
        _attributes: Option<&HashMap<String, String>>,
    ) -> Result<Self::Value>
    where
        D: DeserializeXml<'xml> + AccessorXml<'xml>,
    {
        unimplemented!();
    }
}

pub trait AccessorXml<'xml> {
    fn peek_next_tag(&mut self) -> Result<Option<XmlRecord>>;
    fn verify_prefix(&self, prefix_to_verify: &str) -> bool;
}

pub struct Deserializer<'xml> {
    pub iter: &'xml mut XmlParser<'xml>,
    prefixes: &'xml mut BTreeSet<&'xml str>,
}

impl<'xml> Deserializer<'xml> {
    fn process_open_tag(&mut self, name: &str) -> Result<Option<HashMap<String, String>>> {
        if let Some(item) = self.iter.next() {
            match item? {
                XmlRecord::Open(v) if v.key == name => Ok(v.attributes),
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
    fn deserialize_struct<'b, V>(&mut self, visitor: V, name: &str, prefixes: &mut BTreeSet<&str>) -> Result<V::Value>
    where
        V: Visitor<'xml>,
    {
        // Dodać prefixes do self.prefixes, po drodze zebrać te których nie było (missed prefixes).
        let attributes = self.process_open_tag(name)?; // atrybuty stąd
        
        // Przekazać atrybuty
        let ret = visitor.visit_struct(self, attributes.as_ref())?;
        
        self.check_close_tag(name)?;
        // usunąć missed prefixes.
        Ok(ret)
    }
}

impl<'xml, 'a> AccessorXml<'xml> for Deserializer<'a> {
    fn peek_next_tag(&mut self) -> Result<Option<XmlRecord>> {
        self.iter.peek_next_tag()
    }

    fn verify_prefix(&self, prefix_to_verify: &str) -> bool {
        match self.prefixes.get(prefix_to_verify) {
            Some(_) => true,
            None => false,
        }
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
