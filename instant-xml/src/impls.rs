use std::borrow::Cow;
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
struct DisplayToXml<'a, T: fmt::Display>(pub &'a T);

impl<'a, T> ToXml for DisplayToXml<'a, T>
where
    T: fmt::Display,
{
    fn serialize<W>(&self, serializer: &mut Serializer<W>) -> Result<(), Error>
    where
        W: fmt::Write,
    {
        let field_context = match serializer.consume_field_context() {
            Some(field_context) => field_context,
            None => return Err(Error::UnexpectedValue),
        };

        match field_context.attribute {
            Some(FieldAttribute::Attribute) => {
                serializer.add_attribute_value(&self.0)?;
            }
            _ => {
                serializer.add_open_tag(&field_context)?;
                write!(serializer.output, "{}", self.0)?;
                serializer.add_close_tag(field_context)?;
            }
        }
        Ok(())
    }
}

macro_rules! to_xml_for_number {
    ($typ:ty) => {
        impl ToXml for $typ {
            fn serialize<W: fmt::Write>(
                &self,
                serializer: &mut Serializer<W>,
            ) -> Result<(), Error> {
                DisplayToXml(self).serialize(serializer)
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
    fn serialize<W: fmt::Write>(&self, serializer: &mut Serializer<W>) -> Result<(), Error> {
        let value = match self {
            true => "true",
            false => "false",
        };

        DisplayToXml(&value).serialize(serializer)
    }
}

impl ToXml for String {
    fn serialize<W: fmt::Write>(&self, serializer: &mut Serializer<W>) -> Result<(), Error> {
        DisplayToXml(&escape(self)?).serialize(serializer)
    }
}

impl ToXml for char {
    fn serialize<W: fmt::Write>(&self, serializer: &mut Serializer<W>) -> Result<(), Error> {
        let mut tmp = [0u8; 4];
        DisplayToXml(&escape(&*self.encode_utf8(&mut tmp))?).serialize(serializer)
    }
}

impl ToXml for &str {
    fn serialize<W: fmt::Write>(&self, serializer: &mut Serializer<W>) -> Result<(), Error> {
        DisplayToXml(&escape(self)?).serialize(serializer)
    }
}

impl ToXml for Cow<'_, str> {
    fn serialize<W: fmt::Write>(&self, serializer: &mut Serializer<W>) -> Result<(), Error> {
        DisplayToXml(&escape(self)?).serialize(serializer)
    }
}

impl<T: ToXml> ToXml for Option<T> {
    fn serialize<W: fmt::Write>(&self, serializer: &mut Serializer<W>) -> Result<(), Error> {
        match self {
            Some(v) => v.serialize(serializer),
            None => Ok(()),
        }
    }
}

fn escape(input: &str) -> Result<Cow<'_, str>, Error> {
    let mut result = String::with_capacity(input.len());
    let mut last_end = 0;
    for (start, c) in input.chars().enumerate() {
        let to = match c {
            '&' => "&amp;",
            '"' => "&quot;",
            '<' => "&lt;",
            '>' => "&gt;",
            '\'' => "&apos;",
            _ => continue,
        };
        result.push_str(input.get(last_end..start).unwrap());
        result.push_str(to);
        last_end = start + 1;
    }

    if result.is_empty() {
        return Ok(Cow::Borrowed(input));
    }

    result.push_str(input.get(last_end..input.len()).unwrap());
    Ok(Cow::Owned(result))
}
