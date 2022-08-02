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

    fn write_xml(&self, serializer: &mut Serializer, field_data: &FieldData) -> Result<(), Error> {
        // Open tag
        match field_data.field_attribute {
            Some(FieldAttribute::Prefix(prefix)) => {
                serializer.output.push('<');
                serializer.output.push_str(prefix);
                serializer.output.push(':');
                serializer.output.push_str(field_data.field_name);
                serializer.output.push('>');
            }
            Some(FieldAttribute::Namespace(namespace)) => {
                serializer.output.push('<');
                serializer.output.push_str(field_data.field_name);
                serializer.output.push_str(" xmlns=\"");
                serializer.output.push_str(namespace);
                serializer.output.push_str("\">");
            }
            _ => {
                serializer.output.push('<');
                serializer.output.push_str(field_data.field_name);
                serializer.output.push('>');
            }
        }

        // Value
        serializer.output.push_str(&field_data.value);

        // Close tag
        match field_data.field_attribute {
            Some(FieldAttribute::Prefix(prefix)) => {
                serializer.output.push_str("</");
                serializer.output.push_str(prefix);
                serializer.output.push(':');
                serializer.output.push_str(field_data.field_name);
                serializer.output.push('>');
            }
            _ => {
                serializer.output.push_str("</");
                serializer.output.push_str(field_data.field_name);
                serializer.output.push('>');
            }
        }

        Ok(())
    }
}

macro_rules! to_xml_for_number {
    ($typ:ty) => {
        impl ToXml for $typ {
            fn serialize(
                &self,
                serializer: &mut Serializer,
                field_data: Option<&mut FieldData>,
            ) -> Result<(), Error> {
                match field_data {
                    Some(field_data) => {
                        field_data.value = self.to_string();
                        self.write_xml(serializer, field_data)?;
                    }
                    None => {
                        let field_data = FieldData {
                            field_name: stringify!($typ),
                            field_attribute: None,
                            value: self.to_string(),
                        };
                        self.write_xml(serializer, &field_data)?;
                    }
                }

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

pub struct Serializer<'xml> {
    pub parent_prefixes: &'xml mut BTreeSet<&'xml str>,
    pub output: &'xml mut String,
}

pub enum FieldAttribute<'xml> {
    Prefix(&'xml str),
    Namespace(&'xml str),
}

pub struct FieldData<'xml> {
    pub field_name: &'xml str,
    pub field_attribute: Option<FieldAttribute<'xml>>,
    pub value: String,
}

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

        match field_data {
            Some(field_data) => {
                field_data.value = self.to_string();
                self.write_xml(serializer, field_data)?;
            }
            None => {
                let field_data = FieldData {
                    field_name: "bool",
                    field_attribute: None,
                    value: value.to_owned(),
                };
                self.write_xml(serializer, &field_data)?;
            }
        }

        Ok(())
    }
}

impl ToXml for String {
    fn serialize<'xml>(
        &self,
        serializer: &mut Serializer,
        field_data: Option<&mut FieldData>,
    ) -> Result<(), Error> {
        match field_data {
            Some(field_data) => {
                field_data.value = (*self).clone(); // TODO: Is it possible to skip this clone?
                self.write_xml(serializer, field_data)?;
                Ok(())
            }
            None => {
                let field_data = FieldData {
                    field_name: "String",
                    field_attribute: None,
                    value: (*self).clone(),
                };
                self.write_xml(serializer, &field_data)?;
                Ok(())
            }
        }
    }
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
}
