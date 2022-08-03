use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::fmt::Write;

use thiserror::Error;
pub use xmlparser;

pub use macros::{FromXml, ToXml};

#[doc(hidden)]
pub mod parse;

pub trait ToXml {
    fn to_xml(&self) -> Result<String, Error> {
        let mut parent_prefixes = BTreeSet::new();
        let mut output = String::new();
        let mut serializer = Serializer {
            parent_prefixes: &mut parent_prefixes,
            output: &mut output,
        };
        self.serialize(&mut serializer, None)?;
        Ok(output)
    }

    fn serialize(
        &self,
        serializer: &mut Serializer,
        field_context: Option<&mut FieldContext>,
    ) -> Result<(), Error>;
}

macro_rules! to_xml_for_number {
    ($typ:ty) => {
        impl ToXml for $typ {
            fn serialize(
                &self,
                serializer: &mut Serializer,
                field_context: Option<&mut FieldContext>,
            ) -> Result<(), Error> {
                match field_context {
                    Some(field_context) => {
                        serializer.add_open_tag(field_context);
                        write!(serializer.output, "{}", &self)?;
                        serializer.add_close_tag(field_context);
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
    fn serialize(
        &self,
        serializer: &mut Serializer,
        field_context: Option<&mut FieldContext>,
    ) -> Result<(), Error> {
        let value = match self {
            true => "true",
            false => "false",
        };

        match field_context {
            Some(field_context) => {
                serializer.add_open_tag(field_context);
                serializer.output.push_str(value);
                serializer.add_close_tag(field_context);
                Ok(())
            }
            None => Err(Error::UnexpectedValue),
        }
    }
}

impl ToXml for String {
    fn serialize<'xml>(
        &self,
        serializer: &mut Serializer,
        field_context: Option<&mut FieldContext>,
    ) -> Result<(), Error> {
        match field_context {
            Some(field_context) => {
                serializer.add_open_tag(field_context);
                serializer.output.push_str(self);
                serializer.add_close_tag(field_context);
                Ok(())
            }
            None => Err(Error::UnexpectedValue),
        }
    }
}

pub struct Serializer<'xml> {
    pub parent_prefixes: &'xml mut BTreeSet<&'xml str>,
    pub output: &'xml mut String,
}

impl<'xml> Serializer<'xml> {
    fn add_open_tag(&mut self, field_context: &FieldContext) {
        match field_context.attribute {
            Some(FieldAttribute::Prefix(prefix)) => {
                self.output.push('<');
                self.output.push_str(prefix);
                self.output.push(':');
                self.output.push_str(field_context.name);
                self.output.push('>');
            }
            Some(FieldAttribute::Namespace(namespace)) => {
                self.output.push('<');
                self.output.push_str(field_context.name);
                self.output.push_str(" xmlns=\"");
                self.output.push_str(namespace);
                self.output.push_str("\">");
            }
            _ => {
                self.output.push('<');
                self.output.push_str(field_context.name);
                self.output.push('>');
            }
        }
    }

    fn add_close_tag(&mut self, field_context: &FieldContext) {
        match field_context.attribute {
            Some(FieldAttribute::Prefix(prefix)) => {
                self.output.push_str("</");
                self.output.push_str(prefix);
                self.output.push(':');
                self.output.push_str(field_context.name);
                self.output.push('>');
            }
            _ => {
                self.output.push_str("</");
                self.output.push_str(field_context.name);
                self.output.push('>');
            }
        }
    }
}

pub enum FieldAttribute<'xml> {
    Prefix(&'xml str),
    Namespace(&'xml str),
}

pub struct FieldContext<'xml> {
    pub name: &'xml str,
    pub attribute: Option<FieldAttribute<'xml>>,
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
    #[error("wrong prefix")]
    WrongPrefix,
}
