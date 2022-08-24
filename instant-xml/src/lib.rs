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
    pub default_namespace: Option<&'xml str>,
    pub namespaces: HashMap<&'xml str, &'xml str>,
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

#[derive(Clone, PartialEq, Eq)]
pub enum EntityType {
    Element,
    Attribute,
}

pub enum TagName<'xml> {
    FieldName,
    Custom(&'xml str),
}

pub trait FromXml<'xml>: Sized {
    const TAG_NAME: TagName<'xml>;

    fn from_xml(input: &str) -> Result<Self, Error> {
        let mut deserializer = Deserializer::new(input);
        Self::deserialize(&mut deserializer)
    }

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, Error>;
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
    def_namespaces: HashMap<&'xml str, &'xml str>,
    parser_namespaces: HashMap<&'xml str, &'xml str>,
    def_defualt_namespace: &'xml str,
    parser_defualt_namespace: &'xml str,
    tag_attributes: Vec<(&'xml str, &'xml str)>,
    next_type: EntityType,
    next_def_namespace: Option<&'xml str>,
}

impl<'xml> Deserializer<'xml> {
    pub fn new(input: &'xml str) -> Self {
        Self {
            parser: XmlParser::new(input),
            def_namespaces: std::collections::HashMap::new(),
            parser_namespaces: std::collections::HashMap::new(),
            def_defualt_namespace: "",
            parser_defualt_namespace: "",
            tag_attributes: Vec::new(),
            next_type: EntityType::Element,
            next_def_namespace: None,
        }
    }

    pub fn peek_next_tag(&mut self) -> Result<Option<XmlRecord>, Error> {
        self.parser.peek_next_tag()
    }

    pub fn get_def_namespace(&self, prefix: &str) -> Option<&&str> {
        self.def_namespaces.get(prefix)
    }

    pub fn get_parser_namespace(&self, prefix: &str) -> Option<&&str> {
        self.parser_namespaces.get(prefix)
    }

    pub fn compare_parser_and_def_default_namespaces(&self) -> bool {
        self.parser_defualt_namespace == self.def_defualt_namespace
    }

    pub fn peek_next_attribute(&self) -> Option<&(&'xml str, &'xml str)> {
        self.tag_attributes.last()
    }

    pub fn deserialize_struct<V>(
        &mut self,
        visitor: V,
        name: &str,
        def_default_namespace: &'xml str,
        def_namespaces: &HashMap<&'xml str, &'xml str>,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'xml>,
    {
        println!("deserialize_struct");
        // Setting current defined default namespace
        let def_defualt_namespace_to_revert = self.def_defualt_namespace;
        self.def_defualt_namespace = def_default_namespace;

        // Adding struct defined namespaces
        let new_def_namespaces = def_namespaces
            .iter()
            .filter(|(k, v)| self.def_namespaces.insert(k, v).is_none())
            .collect::<Vec<_>>();

        // Process open tag
        let tag_data = match self.parser.next() {
            Some(Ok(XmlRecord::Open(item))) if item.key == name => item,
            _ => return Err(Error::UnexpectedValue),
        };

        // Set current attributes
        self.tag_attributes = tag_data.attributes;

        // Setting current parser default namespace
        let parser_defualt_namespace_to_revert = self.parser_defualt_namespace;

        // Set parser default namespace
        match tag_data.default_namespace {
            Some(namespace) => {
                self.parser_defualt_namespace = namespace;
            }
            None => {
                // If there is no default namespace in the tag, check if parent default namespace equals the current one
                if def_defualt_namespace_to_revert != self.def_defualt_namespace {
                    return Err(Error::WrongNamespace);
                }
            }
        }

        // Compare parser namespace with defined one
        if !self.compare_parser_and_def_default_namespaces() {
            return Err(Error::WrongNamespace);
        }

        // Adding parser namespaces
        let new_parser_namespaces = tag_data
            .namespaces
            .iter()
            .filter(|(k, v)| self.parser_namespaces.insert(k, v).is_none())
            .collect::<Vec<_>>();

        // Check if namespace is defined, regardless of its key.
        for v in self.parser_namespaces.values() {
            match self.def_namespaces.iter().find(|(_, def_v)| def_v == &v) {
                Some(_) => (),
                None => return Err(Error::MissingdPrefix),
            }
        }

        let ret = visitor.visit_struct(self)?;
        self.check_close_tag(name)?;

        // Removing parser namespaces
        let _ = new_parser_namespaces
            .iter()
            .map(|(k, _)| self.parser_namespaces.remove(*k));

        // Removing struct defined namespaces
        let _ = new_def_namespaces
            .iter()
            .map(|(k, _)| self.def_namespaces.remove(*k));

        // Retriving old defined namespace
        self.def_defualt_namespace = def_defualt_namespace_to_revert;

        // Retriving old parser namespace
        self.parser_defualt_namespace = parser_defualt_namespace_to_revert;
        Ok(ret)
    }

    pub fn set_next_type_as_attribute(&mut self) -> Result<(), Error> {
        if self.next_type == EntityType::Attribute {
            return Err(Error::UnexpectedState);
        }

        self.next_type = EntityType::Attribute;
        Ok(())
    }

    pub fn consume_next_type(&mut self) -> EntityType {
        let ret = self.next_type.clone();
        self.next_type = EntityType::Element;
        ret
    }

    pub fn set_next_def_namespace(&mut self, namespace: Option<&'xml str>) -> Result<(), Error> {
        if self.next_def_namespace.is_some() {
            return Err(Error::UnexpectedState);
        }

        self.next_def_namespace = namespace;
        Ok(())
    }

    pub fn consume_next_def_namespace(&mut self) -> Option<&'xml str> {
        let ret = self.next_def_namespace;
        self.next_def_namespace = None;
        ret
    }

    fn deserialize_bool<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'xml>,
    {
        // Process open tag
        let tag_data = match self.parser.next() {
            Some(Ok(XmlRecord::Open(item))) => item,
            _ => return Err(Error::UnexpectedValue),
        };

        if tag_data.default_namespace != self.consume_next_def_namespace() {
            return Err(Error::WrongNamespace);
        }

        match self.parser.next() {
            Some(Ok(XmlRecord::Element(v))) => {
                let ret = visitor.visit_str(v);
                self.parser.next();
                ret
            }
            _ => Err(Error::UnexpectedValue),
        }
    }

    fn deserialize_attribute<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'xml>,
    {
        match self.tag_attributes.pop() {
            Some((_, value)) => visitor.visit_str(value),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }

    fn check_close_tag(&mut self, name: &str) -> Result<(), Error> {
        let item = match self.parser.next() {
            Some(item) => item?,
            None => return Err(Error::MissingTag),
        };

        match item {
            XmlRecord::Close(v) if v == name => Ok(()),
            _ => Err(Error::UnexpectedTag),
        }
    }
}

pub trait FromXmlOwned: for<'xml> FromXml<'xml> {}

#[allow(dead_code)]
struct State<'a> {
    prefix: HashMap<&'a str, &'a str>,
}

#[derive(Debug, Error, PartialEq, Eq)]
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
    #[error("unexpected prefix")]
    UnexpectedPrefix,
    #[error("unexpected state")]
    UnexpectedState,
    #[error("wrong namespace")]
    WrongNamespace,
}
