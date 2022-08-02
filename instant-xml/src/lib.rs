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
        field_data: Option<&mut FieldData>,
    ) -> Result<(), Error>;
}

macro_rules! to_xml_for_number {
    ($typ:ty) => {
        impl ToXml for $typ {
            fn serialize(
                &self,
                serializer: &mut Serializer,
                field_data: Option<&mut FieldData>,
            ) -> Result<(), Error> {
                let field_data = match field_data {
                    Some(field_data) => field_data,
                    None => &FieldData {
                        field_name: stringify!($typ),
                        field_attribute: None,
                    },
                };

                serializer.add_open_tag(field_data);
                serializer.output.push_str(&self.to_string());
                serializer.add_close_tag(field_data);

                Ok(())
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
        field_data: Option<&mut FieldData>,
    ) -> Result<(), Error> {
        let value = match self {
            true => "true",
            false => "false",
        };

        let field_data = match field_data {
            Some(field_data) => field_data,
            None => &FieldData {
                field_name: "bool",
                field_attribute: None,
            },
        };

        serializer.add_open_tag(field_data);
        serializer.output.push_str(value);
        serializer.add_close_tag(field_data);

        Ok(())
    }
}

impl ToXml for String {
    fn serialize<'xml>(
        &self,
        serializer: &mut Serializer,
        field_data: Option<&mut FieldData>,
    ) -> Result<(), Error> {
        let field_data = match field_data {
            Some(field_data) => field_data,
            None => &FieldData {
                field_name: "String",
                field_attribute: None,
            },
        };

        serializer.add_open_tag(field_data);
        serializer.output.push_str(self);
        serializer.add_close_tag(field_data);
        Ok(())
    }
}

pub struct Serializer<'xml> {
    pub parent_prefixes: &'xml mut BTreeSet<&'xml str>,
    pub output: &'xml mut String,
}

impl<'xml> Serializer<'xml> {
    fn add_open_tag(&mut self, field_data: &FieldData) {
        match field_data.field_attribute {
            Some(FieldAttribute::Prefix(prefix)) => {
                self.output.push('<');
                self.output.push_str(prefix);
                self.output.push(':');
                self.output.push_str(field_data.field_name);
                self.output.push('>');
            }
            Some(FieldAttribute::Namespace(namespace)) => {
                self.output.push('<');
                self.output.push_str(field_data.field_name);
                self.output.push_str(" xmlns=\"");
                self.output.push_str(namespace);
                self.output.push_str("\">");
            }
            _ => {
                self.output.push('<');
                self.output.push_str(field_data.field_name);
                self.output.push('>');
            }
        }
    }

    fn add_close_tag(&mut self, field_data: &FieldData) {
        match field_data.field_attribute {
            Some(FieldAttribute::Prefix(prefix)) => {
                self.output.push_str("</");
                self.output.push_str(prefix);
                self.output.push(':');
                self.output.push_str(field_data.field_name);
                self.output.push('>');
            }
            _ => {
                self.output.push_str("</");
                self.output.push_str(field_data.field_name);
                self.output.push('>');
            }
        }
    }
}

pub enum FieldAttribute<'xml> {
    Prefix(&'xml str),
    Namespace(&'xml str),
}

pub struct FieldData<'xml> {
    pub field_name: &'xml str,
    pub field_attribute: Option<FieldAttribute<'xml>>,
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
