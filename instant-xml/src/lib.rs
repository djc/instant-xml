use std::collections::HashMap;
use std::fmt;
use std::fmt::Write;

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
        self.serialize(&mut serializer)?;
        Ok(output)
    }

    fn serialize<W: fmt::Write>(&self, serializer: &mut Serializer<W>) -> Result<(), Error>;
}

pub struct Serializer<'xml, W: fmt::Write> {
    // For parent namespaces the key is the namespace and the value is the prefix. We are adding to map
    // only if the namespaces do not exist, if it does exist then we are using an already defined parent prefix.
    #[doc(hidden)]
    pub parent_namespaces: HashMap<&'xml str, &'xml str>,
    #[doc(hidden)]
    pub output: &'xml mut W,

    parent_default_namespace: &'xml str,
    parent_default_namespace_to_revert: &'xml str,
    current_attributes: String,
    next_field_context: Option<FieldContext<'xml>>,
}

impl<'xml, W: fmt::Write> Serializer<'xml, W> {
    pub fn new(output: &'xml mut W) -> Self {
        Self {
            parent_namespaces: HashMap::new(),
            output,
            parent_default_namespace: "",
            parent_default_namespace_to_revert: "",
            next_field_context: None,
            current_attributes: String::new(),
        }
    }

    pub fn consume_current_attributes(&mut self) -> Result<(), Error> {
        self.output.write_str(&self.current_attributes)?;
        self.current_attributes.clear();
        Ok(())
    }

    pub fn add_attribute_key(&mut self, attr_key: &impl fmt::Display) -> Result<(), Error> {
        self.current_attributes.push(' ');
        write!(self.current_attributes, "{}", attr_key)?;
        self.current_attributes.push('=');
        Ok(())
    }

    pub fn add_attribute_value(&mut self, attr_value: &impl fmt::Display) -> Result<(), Error> {
        self.current_attributes.push('"');
        write!(self.current_attributes, "{}", attr_value)?;
        self.current_attributes.push('"');
        Ok(())
    }

    pub fn set_field_context(&mut self, field_context: FieldContext<'xml>) -> Result<(), Error> {
        if self.next_field_context.is_some() {
            return Err(Error::UnexpectedState);
        };

        self.next_field_context = Some(field_context);
        Ok(())
    }

    pub fn consume_field_context(&mut self) -> Option<FieldContext<'xml>> {
        self.next_field_context.take()
    }

    pub fn set_parent_default_namespace(&mut self, namespace: &'xml str) -> Result<(), Error> {
        self.parent_default_namespace = namespace;
        Ok(())
    }

    pub fn parent_default_namespace(&self) -> &'xml str {
        self.parent_default_namespace
    }

    pub fn update_parent_default_namespace(&mut self, namespace: &'xml str) {
        self.parent_default_namespace_to_revert = self.parent_default_namespace;
        self.parent_default_namespace = namespace;
    }

    pub fn retrieve_parent_default_namespace(&mut self) {
        self.parent_default_namespace = self.parent_default_namespace_to_revert;
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
            Some(FieldAttribute::Namespace(namespace))
                if self.parent_default_namespace != namespace =>
            {
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

    fn add_close_tag(&mut self, field_context: FieldContext) -> Result<(), Error> {
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
    Attribute,
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

pub enum TagName {
    FieldName,
    Custom(&'static str),
}

pub trait FromXml<'xml>: Sized {
    const TAG_NAME: TagName;

    fn from_xml(input: &'xml str) -> Result<Self, Error> {
        let mut deserializer = Deserializer::new(input);
        Self::deserialize(&mut deserializer)
    }

    fn deserialize(deserializer: &mut Deserializer<'xml>) -> Result<Self, Error>;

    // If the missing field is of type `Option<T>` then treat is as `None`,
    // otherwise it is an error.
    fn missing_value() -> Result<Self, Error> {
        Err(Error::MissingValue)
    }
}

pub trait Visitor<'xml>: Sized {
    type Value;

    fn visit_str(self, _value: &'xml str) -> Result<Self::Value, Error> {
        unimplemented!();
    }

    fn visit_struct(&self, _deserializer: &mut Deserializer<'xml>) -> Result<Self::Value, Error> {
        unimplemented!();
    }
}

pub struct Deserializer<'xml> {
    parser: XmlParser<'xml>,
    def_namespaces: HashMap<&'xml str, &'xml str>,
    parser_namespaces: HashMap<&'xml str, &'xml str>,
    def_default_namespace: &'xml str,
    parser_default_namespace: &'xml str,
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
            def_default_namespace: "",
            parser_default_namespace: "",
            tag_attributes: Vec::new(),
            next_type: EntityType::Element,
            next_def_namespace: None,
        }
    }

    pub fn peek_next_tag(&mut self) -> Result<Option<XmlRecord<'xml>>, Error> {
        self.parser.peek_next_tag()
    }

    // Check if defined and gotten namespaces equals for each field
    pub fn compare_namespace(
        &self,
        expected: &Option<&str>,
        actual: Option<&str>,
    ) -> Result<(), Error> {
        match (expected, actual) {
            (Some(expected), Some(actual)) => {
                match self.parser_namespaces.get(expected) == self.def_namespaces.get(actual) {
                    true => Ok(()),
                    false => Err(Error::WrongNamespace),
                }
            }
            (Some(_), None) | (None, Some(_)) => Err(Error::WrongNamespace),
            (None, None) => Ok(()),
        }
    }

    pub fn compare_parser_and_def_default_namespaces(&self) -> bool {
        self.parser_default_namespace == self.def_default_namespace
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
        // Saveing current defined default namespace
        let def_default_namespace_to_revert = self.def_default_namespace;
        self.def_default_namespace = def_default_namespace;

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

        // Saveing current parser default namespace
        let parser_default_namespace_to_revert = self.parser_default_namespace;

        // Set parser default namespace
        match tag_data.default_namespace {
            Some(namespace) => {
                self.parser_default_namespace = namespace;
            }
            None => {
                // If there is no default namespace in the tag, check if parent default namespace equals the current one
                if def_default_namespace_to_revert != self.def_default_namespace {
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

        let ret = visitor.visit_struct(self)?;

        // Process close tag
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
        self.def_default_namespace = def_default_namespace_to_revert;

        // Retriving old parser namespace
        self.parser_default_namespace = parser_default_namespace_to_revert;
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

    fn deserialize_element<V>(&mut self, visitor: V) -> Result<V::Value, Error>
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
    #[error("unexpected state")]
    UnexpectedState,
    #[error("wrong namespace")]
    WrongNamespace,
}
