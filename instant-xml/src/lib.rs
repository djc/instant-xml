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
    pub attributes: Vec<(String, String)>,

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
    fn to_xml(&self) -> Result<String> {
        let mut output = String::new();
        let mut serializer = Serializer::new(&mut output);
        self.serialize(&mut serializer, None)?;
        Ok(output)
    }

    fn serialize<W>(
        &self,
        serializer: &mut Serializer<W>,
        field_context: Option<&FieldContext>,
    ) -> Result<()>
    where
        W: fmt::Write;
}

macro_rules! to_xml_for_number {
    ($typ:ty) => {
        impl ToXml for $typ {
            fn serialize<W>(
                &self,
                serializer: &mut Serializer<W>,
                field_context: Option<&FieldContext>,
            ) -> Result<()>
            where
                W: fmt::Write,
            {
                match field_context {
                    Some(field_context) => {
                        serializer.add_open_tag(field_context)?;
                        write!(serializer.output, "{}", &self)?;
                        serializer.add_close_tag(field_context)?;
                        Ok(())
                    }
                    None => Err(Error::UnexpectedValue),
                }
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

impl ToXml for bool {
    fn serialize<W>(
        &self,
        serializer: &mut Serializer<W>,
        field_context: Option<&FieldContext>,
    ) -> Result<()>
    where
        W: fmt::Write,
    {
        let value = match self {
            true => "true",
            false => "false",
        };

        match field_context {
            Some(field_context) => {
                serializer.add_open_tag(field_context)?;
                serializer.output.write_str(value)?;
                serializer.add_close_tag(field_context)?;
                Ok(())
            }
            None => Err(Error::UnexpectedValue),
        }
    }
}

impl ToXml for String {
    fn serialize<W>(
        &self,
        serializer: &mut Serializer<W>,
        field_context: Option<&FieldContext>,
    ) -> Result<()>
    where
        W: fmt::Write,
    {
        match field_context {
            Some(field_context) => {
                serializer.add_open_tag(field_context)?;
                serializer.output.write_str(self)?;
                serializer.add_close_tag(field_context)?;
                Ok(())
            }
            None => Err(Error::UnexpectedValue),
        }
    }
}

pub struct Serializer<'xml, W>
where
    W: fmt::Write,
{
    #[doc(hidden)]
    pub parent_prefixes: BTreeSet<&'xml str>,
    #[doc(hidden)]
    pub output: &'xml mut W,
}

impl<'xml, W: std::fmt::Write> Serializer<'xml, W> {
    pub fn new(output: &'xml mut W) -> Self {
        Self {
            parent_prefixes: BTreeSet::new(),
            output,
        }
    }

    fn add_open_tag(&mut self, field_context: &FieldContext) -> Result<()> {
        match field_context.attribute {
            Some(FieldAttribute::Prefix(prefix)) => {
                self.output.write_char('<')?;
                self.output.write_str(prefix)?;
                self.output.write_char(':')?;
                self.output.write_str(field_context.name)?;
                self.output.write_char('>')?;
            }
            Some(FieldAttribute::Namespace(namespace)) => {
                self.output.write_char('<')?;
                self.output.write_str(field_context.name)?;
                self.output.write_str(" xmlns=\"")?;
                self.output.write_str(namespace)?;
                self.output.write_str("\">")?;
            }
            _ => {
                self.output.write_char('<')?;
                self.output.write_str(field_context.name)?;
                self.output.write_char('>')?;
            }
        }
        Ok(())
    }

    fn add_close_tag(&mut self, field_context: &FieldContext) -> Result<()> {
        match field_context.attribute {
            Some(FieldAttribute::Prefix(prefix)) => {
                self.output.write_str("</")?;
                self.output.write_str(prefix)?;
                self.output.write_char(':')?;
                self.output.write_str(field_context.name)?;
                self.output.write_char('>')?;
            }
            _ => {
                self.output.write_str("</")?;
                self.output.write_str(field_context.name)?;
                self.output.write_char('>')?;
            }
        }
        Ok(())
    }
}

pub enum FieldAttribute<'xml> {
    Prefix(&'xml str),
    Namespace(&'xml str),
}

pub struct FieldContext<'xml> {
    #[doc(hidden)]
    pub name: &'xml str,
    #[doc(hidden)]
    pub attribute: Option<FieldAttribute<'xml>>,
}

pub enum EntityType {
    Element,
    Attribute,
}

pub enum XMLTagName<'xml> {
    FieldName,
    Custom(&'xml str),
}

pub trait FromXml<'xml>: Sized {
    const TAG_NAME: XMLTagName<'xml>;

    fn from_xml(_input: &str) -> Result<Self> {
        unimplemented!();
    }

    fn deserialize(deserializer: &mut Deserializer, kind: EntityType) -> Result<Self>;
}

pub trait Visitor<'xml>: Sized {
    type Value;

    fn visit_str(self, _value: &str) -> Result<Self::Value> {
        unimplemented!();
    }

    fn visit_struct<'a>(&self, _deserializer: &'a mut Deserializer) -> Result<Self::Value> {
        unimplemented!();
    }
}

pub struct Deserializer<'xml> {
    #[doc(hidden)]
    pub parser: &'xml mut XmlParser<'xml>,
    #[doc(hidden)]
    pub namespaces: HashMap<&'xml str, &'xml str>,
    #[doc(hidden)]
    pub tag_attributes: Vec<(String, String)>,
}

impl<'xml> Deserializer<'xml> {
    pub fn deserialize_bool<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'xml>,
    {
        self.parser.next();
        if let Some(item) = self.parser.next() {
            match item? {
                XmlRecord::Element(v) => {
                    let ret = visitor.visit_str(v.as_str());
                    self.parser.next();
                    ret
                }
                _ => Err(Error::UnexpectedTag),
            }
        } else {
            Err(Error::MissingValue)
        }
    }

    pub fn deserialize_attribute<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'xml>,
    {
        visitor.visit_str(&self.pop_next_attribute_value()?)
    }

    pub fn deserialize_struct<V>(
        &mut self,
        visitor: V,
        name: &str,
        namespaces: &HashMap<&'xml str, &'xml str>,
    ) -> Result<V::Value>
    where
        V: Visitor<'xml>,
    {
        let new_namespaces = namespaces
            .iter()
            .filter(|(k, v)| self.namespaces.insert(k, v).is_none())
            .collect::<Vec<_>>();

        self.process_open_tag(name, namespaces)?;
        let ret = visitor.visit_struct(self)?;

        self.check_close_tag(name)?;
        let _ = new_namespaces
            .iter()
            .map(|(k, _)| self.namespaces.remove(*k));

        Ok(ret)
    }

    pub fn peek_next_tag(&mut self) -> Result<Option<XmlRecord>> {
        self.parser.peek_next_tag()
    }

    pub fn verify_namespace(&self, namespace_to_verify: &str) -> bool {
        self.namespaces.get(namespace_to_verify).is_some()
    }

    pub fn peek_next_attribute(&self) -> Option<&(String, String)> {
        self.tag_attributes.last()
    }

    pub fn pop_next_attribute_value(&mut self) -> Result<String> {
        match self.tag_attributes.pop() {
            Some((_, value)) => Ok(value),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }

    fn process_open_tag(
        &mut self,
        name: &str,
        namespaces: &HashMap<&'xml str, &'xml str>,
    ) -> Result<()> {
        if let Some(item) = self.parser.next() {
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

                    self.tag_attributes = v.attributes;
                    Ok(())
                }
                _ => Err(Error::UnexpectedValue),
            }
        } else {
            Err(Error::UnexpectedTag)
        }
    }

    fn check_close_tag(&mut self, name: &str) -> Result<()> {
        // Close tag
        if let Some(item) = self.parser.next() {
            match item? {
                XmlRecord::Close(v) if v == name => Ok(()),
                _ => Err(Error::UnexpectedTag),
            }
        } else {
            Err(Error::MissingTag)
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
    #[error("missing prefix")]
    MissingdPrefix,
    #[error("unexpected prefix")]
    UnexpectedPrefix,
}
