use std::fmt;
use std::str::FromStr;

use crate::{
    Deserializer, EntityType, Error, FieldAttribute, FromXml, Serializer, TagName, ToXml, Visitor,
};

struct BoolVisitor;

impl<'de> Visitor<'de> for BoolVisitor {
    type Value = bool;

    fn visit_str<'a>(self, value: &str) -> Result<Self::Value, Error> {
        match FromStr::from_str(value) {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::Other(e.to_string())),
        }
    }
}

impl<'xml> FromXml<'xml> for bool {
    const TAG_NAME: TagName<'xml> = TagName::FieldName;

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, Error> {
        match deserializer.consume_next_type() {
            EntityType::Element => deserializer.deserialize_bool(BoolVisitor),
            EntityType::Attribute => deserializer.deserialize_attribute(BoolVisitor),
        }
    }
}

// Serializer

macro_rules! to_xml_for_number {
    ($typ:ty) => {
        impl ToXml for $typ {
            fn serialize<W>(&self, serializer: &mut Serializer<W>) -> Result<(), Error>
            where
                W: fmt::Write,
            {
                match serializer.consume_field_context() {
                    Some(field_context) => {
                        match field_context.attribute {
                            Some(FieldAttribute::Attribute) => {
                                serializer.add_attribute_value(&self.to_string());
                            }
                            _ => {
                                serializer.add_open_tag(&field_context)?;
                                write!(serializer.output, "{}", &self)?;
                                serializer.add_close_tag(field_context)?;
                            }
                        }
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
to_xml_for_number!(isize);
to_xml_for_number!(u8);
to_xml_for_number!(u16);
to_xml_for_number!(u32);
to_xml_for_number!(u64);
to_xml_for_number!(usize);
to_xml_for_number!(f32);
to_xml_for_number!(f64);

impl ToXml for bool {
    fn serialize<W>(&self, serializer: &mut Serializer<W>) -> Result<(), Error>
    where
        W: fmt::Write,
    {
        let value = match self {
            true => "true",
            false => "false",
        };

        match serializer.consume_field_context() {
            Some(field_context) => {
                match field_context.attribute {
                    Some(FieldAttribute::Attribute) => {
                        serializer.add_attribute_value(value);
                    }
                    _ => {
                        serializer.add_open_tag(&field_context)?;
                        serializer.output.write_str(value)?;
                        serializer.add_close_tag(field_context)?;
                    }
                }
                Ok(())
            }
            None => Err(Error::UnexpectedValue),
        }
    }
}

impl ToXml for String {
    fn serialize<W>(&self, serializer: &mut Serializer<W>) -> Result<(), Error>
    where
        W: fmt::Write,
    {
        match serializer.consume_field_context() {
            Some(field_context) => {
                match field_context.attribute {
                    Some(FieldAttribute::Attribute) => {
                        serializer.add_attribute_value(self);
                    }
                    _ => {
                        serializer.add_open_tag(&field_context)?;
                        serializer.output.write_str(self)?;
                        serializer.add_close_tag(field_context)?;
                    }
                }
                Ok(())
            }
            None => Err(Error::UnexpectedValue),
        }
    }
}

impl ToXml for char {
    fn serialize<W>(&self, serializer: &mut Serializer<W>) -> Result<(), Error>
    where
        W: fmt::Write,
    {
        match serializer.consume_field_context() {
            Some(field_context) => {
                let mut tmp = [0u8; 4];
                let char_str = self.encode_utf8(&mut tmp);
                match field_context.attribute {
                    Some(FieldAttribute::Attribute) => {
                        serializer.add_attribute_value(char_str);
                    }
                    _ => {
                        serializer.add_open_tag(&field_context)?;
                        serializer.output.write_str(char_str)?;
                        serializer.add_close_tag(field_context)?;
                    }
                }
                Ok(())
            }
            None => Err(Error::UnexpectedValue),
        }
    }
}

impl ToXml for &str {
    fn serialize<W>(&self, serializer: &mut Serializer<W>) -> Result<(), Error>
    where
        W: fmt::Write,
    {
        match serializer.consume_field_context() {
            Some(field_context) => {
                match field_context.attribute {
                    Some(FieldAttribute::Attribute) => {
                        serializer.add_attribute_value(self);
                    }
                    _ => {
                        serializer.add_open_tag(&field_context)?;
                        serializer.output.write_str(self)?;
                        serializer.add_close_tag(field_context)?;
                    }
                }
                Ok(())
            }
            None => Err(Error::UnexpectedValue),
        }
    }
}
