use std::collections::{BTreeSet, HashMap};
use std::fmt;

use thiserror::Error;
pub use xmlparser;

pub use macros::{FromXml, ToXml};
use parse::XmlParser;

pub mod impls;
#[doc(hidden)]
pub mod parse;

pub struct TagData<'xml> {
    pub key: &'xml str,
    pub attributes: Vec<(&'xml str, &'xml str)>,

    // TODO: handle default namespace
    pub default_namespace: Option<&'xml str>,

    pub namespaces: Option<HashMap<&'xml str, &'xml str>>,
    pub prefix: Option<&'xml str>,
}

pub enum XmlRecord<'xml> {
    Open(TagData<'xml>),
    Element(&'xml str),
    Close(&'xml str),
}

pub trait ToXml {
    fn to_xml(&self) -> Result<String, Error> {
        let mut output = String::new();
        let mut serializer = Serializer::new(&mut output);
        self.serialize(&mut serializer, None)?;
        Ok(output)
    }

    fn serialize<W>(
        &self,
        serializer: &mut Serializer<W>,
        field_context: Option<&FieldContext>,
    ) -> Result<(), Error>
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
            ) -> Result<(), Error>
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
    ) -> Result<(), Error>
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
    ) -> Result<(), Error>
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

    fn add_open_tag(&mut self, field_context: &FieldContext) -> Result<(), Error> {
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

    fn add_close_tag(&mut self, field_context: &FieldContext) -> Result<(), Error> {
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

    fn from_xml(_input: &str) -> Result<Self, Error> {
        unimplemented!();
    }

    fn deserialize(deserializer: &mut Deserializer, kind: EntityType) -> Result<Self, Error>;
}

pub trait Visitor<'xml>: Sized {
    type Value;

    fn visit_str(self, _value: &str) -> Result<Self::Value, Error> {
        unimplemented!();
    }

    fn visit_struct<'a>(&self, _deserializer: &'a mut Deserializer) -> Result<Self::Value, Error> {
        unimplemented!();
    }
}

pub struct Deserializer<'xml> {
    parser: XmlParser<'xml>,
    namespaces: HashMap<&'xml str, &'xml str>,
    tag_attributes: Vec<(&'xml str, &'xml str)>,
}

impl<'xml> Deserializer<'xml> {
    pub fn new(input: &'xml str) -> Self {
        Self {
            parser: XmlParser::new(input),
            namespaces: std::collections::HashMap::new(),
            tag_attributes: Vec::new(),
        }
    }

    pub fn peek_next_tag(&mut self) -> Result<Option<XmlRecord>, Error> {
        self.parser.peek_next_tag()
    }

    pub fn verify_namespace(&self, namespace_to_verify: &str) -> bool {
        self.namespaces.get(namespace_to_verify).is_some()
    }

    pub fn peek_next_attribute(&self) -> Option<&(&'xml str, &'xml str)> {
        self.tag_attributes.last()
    }

    pub fn deserialize_struct<V>(
        &mut self,
        visitor: V,
        name: &str,
        namespaces: &HashMap<&'xml str, &'xml str>,
    ) -> Result<V::Value, Error>
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

    fn deserialize_bool<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'xml>,
    {
        self.parser.next();
        if let Some(Ok(XmlRecord::Element(v))) = self.parser.next() {
            let ret = visitor.visit_str(v);
            self.parser.next();
            ret
        } else {
            Err(Error::UnexpectedValue)
        }
    }

    fn deserialize_attribute<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'xml>,
    {
        match self.tag_attributes.pop() {
            Some((_, value)) => visitor.visit_str(&value),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }

    fn process_open_tag(
        &mut self,
        name: &str,
        namespaces: &HashMap<&'xml str, &'xml str>,
    ) -> Result<(), Error> {
        if let Some(Ok(XmlRecord::Open(item))) = self.parser.next() {
            if item.key == name {
                for (k, v) in item.namespaces.unwrap() {
                    if let Some(item) = namespaces.get(k) {
                        if *item != v {
                            return Err(Error::UnexpectedPrefix);
                        }
                    } else {
                        return Err(Error::MissingdPrefix);
                    }
                }

                self.tag_attributes = item.attributes;
                return Ok(());
            }
        }

        Err(Error::UnexpectedValue)
    }

    fn check_close_tag(&mut self, name: &str) -> Result<(), Error> {
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
