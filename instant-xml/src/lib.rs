use std::collections::{BTreeSet, HashMap};
use std::fmt;

use thiserror::Error;
pub use xmlparser;

pub use macros::{FromXml, ToXml};
use parse::XmlParser;

pub mod impls;
#[doc(hidden)]
pub mod parse;

pub enum Attribute<T> {
    Value(T),
}

pub struct TagData {
    pub key: String,
    pub attributes: Option<HashMap<String, String>>,

    // TODO: handle default namespace
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
    fn from_xml(_input: &str) -> Result<Self> {
        unimplemented!();
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

    fn deserialize_struct<V>(
        &mut self,
        _visitor: V,
        _name: &str,
        _prefixes: &HashMap<&'xml str, &'xml str>,
    ) -> Result<V::Value>
    where
        V: Visitor<'xml>,
    {
        unimplemented!();
    }

    fn deserialize_attribute<V>(&mut self, _visitor: V) -> Result<V::Value>
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
    fn set_current_attribute(&mut self, attr: &str);
}

pub struct Deserializer<'xml> {
    pub iter: &'xml mut XmlParser<'xml>,
    pub prefixes: &'xml mut BTreeSet<&'xml str>,

    // TODO: Think of some more clever way to pass this
    pub current_attribute: &'xml mut String,
}

impl<'xml> Deserializer<'xml> {
    fn process_open_tag(
        &mut self,
        name: &str,
        namespaces: &HashMap<&'xml str, &'xml str>,
    ) -> Result<Option<HashMap<String, String>>> {
        if let Some(item) = self.iter.next() {
            match item? {
                XmlRecord::Open(v) if v.key == name => {
                    // Check if namespaces from parser are the same as defined in the struct
                    for (k, v) in v.namespaces.unwrap() {
                        if let Some(item) = namespaces.get(k.as_str()) {
                            if *item != v.as_str() {
                                return Err(Error::UnexpectedPrefix);
                            }
                        } else {
                            return Err(Error::MissingdPrefix);
                        }
                    }

                    Ok(v.attributes)
                }
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

impl<'xml> DeserializeXml<'xml> for Deserializer<'xml> {
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

    fn deserialize_attribute<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'xml>,
    {
        let ret = visitor.visit_str(self.current_attribute);
        self.current_attribute.clear();
        ret
    }

    // TODO: Validate if other types were already used, tab of &str
    fn deserialize_struct<V>(
        &mut self,
        visitor: V,
        name: &str,
        namespaces: &HashMap<&'xml str, &'xml str>,
    ) -> Result<V::Value>
    where
        V: Visitor<'xml>,
    {
        let new_prefixes = namespaces
            .keys()
            .filter(|v| self.prefixes.insert(v))
            .collect::<Vec<_>>();

        let attributes = self.process_open_tag(name, namespaces)?;
        if attributes.is_some() {
            for (v, k) in attributes.as_ref().unwrap().iter() {
                println!("attr : {}, {}", v, k);
            }
        }

        let ret = visitor.visit_struct(self, attributes.as_ref())?;

        self.check_close_tag(name)?;
        let _ = new_prefixes.iter().map(|v| self.prefixes.remove(*v));

        Ok(ret)
    }
}

impl<'xml, 'a> AccessorXml<'xml> for Deserializer<'a> {
    fn peek_next_tag(&mut self) -> Result<Option<XmlRecord>> {
        self.iter.peek_next_tag()
    }

    fn verify_prefix(&self, prefix_to_verify: &str) -> bool {
        self.prefixes.get(prefix_to_verify).is_some()
    }

    fn set_current_attribute(&mut self, attr: &str) {
        *self.current_attribute = attr.to_string();
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
    #[error("missing prefix")]
    MissingdPrefix,
    #[error("unexpected prefix")]
    UnexpectedPrefix,
}
