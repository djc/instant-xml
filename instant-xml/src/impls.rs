use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use crate::de::{EntityType, Visitor};
use crate::{Deserializer, Error, FieldAttribute, FromXml, Kind, Serializer, ToXml};

// Deserializer
struct FromStrToVisitor<T: FromStr>(PhantomData<T>)
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display;

impl<'xml, T> Visitor<'xml> for FromStrToVisitor<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    type Value = T;
    fn visit_str(self, value: &str) -> Result<Self::Value, Error> {
        match FromStr::from_str(value) {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::Other(e.to_string())),
        }
    }
}

struct BoolVisitor;

impl<'xml> Visitor<'xml> for BoolVisitor {
    type Value = bool;

    fn visit_str(self, value: &str) -> Result<Self::Value, Error> {
        FromStrToVisitor(PhantomData::<Self::Value>).visit_str(value)
    }
}

impl<'xml> FromXml<'xml> for bool {
    const KIND: Kind = Kind::Scalar;

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, Error> {
        match deserializer.consume_next_type() {
            EntityType::Element => deserializer.deserialize_element(BoolVisitor),
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

struct NumberVisitor<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    marker: PhantomData<T>,
}

impl<'xml, T> Visitor<'xml> for NumberVisitor<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    type Value = T;

    fn visit_str(self, value: &str) -> Result<Self::Value, Error> {
        FromStrToVisitor(PhantomData::<Self::Value>).visit_str(value)
    }
}

macro_rules! from_xml_for_number {
    ($typ:ty) => {
        impl<'xml> FromXml<'xml> for $typ {
            fn deserialize(deserializer: &mut Deserializer) -> Result<Self, Error> {
                match deserializer.consume_next_type() {
                    EntityType::Element => deserializer.deserialize_element(NumberVisitor {
                        marker: PhantomData,
                    }),
                    EntityType::Attribute => deserializer.deserialize_attribute(NumberVisitor {
                        marker: PhantomData,
                    }),
                }
            }

            const KIND: Kind = Kind::Scalar;
        }
    };
}

from_xml_for_number!(i8);
from_xml_for_number!(i16);
from_xml_for_number!(i32);
from_xml_for_number!(i64);
from_xml_for_number!(isize);
from_xml_for_number!(u8);
from_xml_for_number!(u16);
from_xml_for_number!(u32);
from_xml_for_number!(u64);
from_xml_for_number!(usize);
from_xml_for_number!(f32);
from_xml_for_number!(f64);

struct StringVisitor;

impl<'xml> Visitor<'xml> for StringVisitor {
    type Value = String;

    fn visit_str(self, value: &str) -> Result<Self::Value, Error> {
        Ok(escape_back(value).into_owned())
    }
}

impl<'xml> FromXml<'xml> for String {
    const KIND: Kind = Kind::Scalar;

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, Error> {
        //<&'xml str>::deserialize(deserializer);
        match deserializer.consume_next_type() {
            EntityType::Element => deserializer.deserialize_element(StringVisitor),
            EntityType::Attribute => deserializer.deserialize_attribute(StringVisitor),
        }
    }
}

struct CharVisitor;

impl<'xml> Visitor<'xml> for CharVisitor {
    type Value = char;

    fn visit_str(self, value: &str) -> Result<Self::Value, Error> {
        match value.len() {
            1 => Ok(value.chars().next().expect("char type")),
            _ => Err(Error::Other("Expected char type".to_string())),
        }
    }
}

impl<'xml> FromXml<'xml> for char {
    const KIND: Kind = Kind::Scalar;

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, Error> {
        match deserializer.consume_next_type() {
            EntityType::Element => deserializer.deserialize_element(CharVisitor),
            EntityType::Attribute => deserializer.deserialize_attribute(CharVisitor),
        }
    }
}

struct StrVisitor;

impl<'a> Visitor<'a> for StrVisitor {
    type Value = &'a str;

    fn visit_str(self, value: &'a str) -> Result<Self::Value, Error> {
        match escape_back(value) {
            Cow::Owned(v) => Err(Error::Other(format!("Unsupported char: {}", v))),
            Cow::Borrowed(v) => Ok(v),
        }
    }
}

impl<'xml> FromXml<'xml> for &'xml str {
    const KIND: Kind = Kind::Scalar;

    fn deserialize(deserializer: &mut Deserializer<'xml>) -> Result<Self, Error> {
        match deserializer.consume_next_type() {
            EntityType::Element => deserializer.deserialize_element(StrVisitor),
            EntityType::Attribute => deserializer.deserialize_attribute(StrVisitor),
        }
    }
}

struct CowStrVisitor;

impl<'a> Visitor<'a> for CowStrVisitor {
    type Value = Cow<'a, str>;

    fn visit_str(self, value: &'a str) -> Result<Self::Value, Error> {
        Ok(escape_back(value))
    }
}

impl<'xml> FromXml<'xml> for Cow<'xml, str> {
    const KIND: Kind = <&str>::KIND;

    fn deserialize(deserializer: &mut Deserializer<'xml>) -> Result<Self, Error> {
        match deserializer.consume_next_type() {
            EntityType::Element => deserializer.deserialize_element(CowStrVisitor),
            EntityType::Attribute => deserializer.deserialize_attribute(CowStrVisitor),
        }
    }
}

impl<'xml, T> FromXml<'xml> for Option<T>
where
    T: FromXml<'xml>,
{
    const KIND: Kind = <T>::KIND;

    fn deserialize(deserializer: &mut Deserializer<'xml>) -> Result<Self, Error> {
        match <T>::deserialize(deserializer) {
            Ok(v) => Ok(Some(v)),
            Err(e) => Err(e),
        }
    }

    fn missing_value() -> Result<Self, Error> {
        Ok(None)
    }
}

fn escape_back(input: &str) -> Cow<'_, str> {
    let mut result = String::with_capacity(input.len());
    let input_len = input.len();

    let mut last_end = 0;
    while input_len - last_end >= 4 {
        match &input[last_end..(last_end + 4)] {
            "&lt;" => {
                result.push('<');
                last_end += 4;
                continue;
            }
            "&gt;" => {
                result.push('>');
                last_end += 4;
                continue;
            }
            _ => (),
        };

        if input_len - last_end >= 5 {
            if &input[last_end..(last_end + 5)] == "&amp;" {
                result.push('&');
                last_end += 5;
                continue;
            }

            if input_len - last_end >= 6 {
                match &input[last_end..(last_end + 6)] {
                    "&apos;" => {
                        result.push('\'');
                        last_end += 6;
                        continue;
                    }
                    "&quot;" => {
                        result.push('"');
                        last_end += 6;
                        continue;
                    }
                    _ => (),
                };
            }
        }

        result.push_str(input.get(last_end..last_end + 1).unwrap());
        last_end += 1;
    }

    result.push_str(input.get(last_end..).unwrap());
    if result.len() == input.len() {
        return Cow::Borrowed(input);
    }

    Cow::Owned(result)
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
