use std::borrow::Cow;
use std::marker::PhantomData;
use std::str::FromStr;

use crate::{Deserializer, EntityType, Error, FromXml, TagName, Visitor};

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
    const TAG_NAME: TagName<'xml> = TagName::FieldName;

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, Error> {
        match deserializer.consume_next_type() {
            EntityType::Element => deserializer.deserialize_element(BoolVisitor),
            EntityType::Attribute => deserializer.deserialize_attribute(BoolVisitor),
        }
    }
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
            const TAG_NAME: TagName<'xml> = TagName::FieldName;

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
    const TAG_NAME: TagName<'xml> = TagName::FieldName;

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
    const TAG_NAME: TagName<'xml> = TagName::FieldName;

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
    const TAG_NAME: TagName<'xml> = TagName::FieldName;

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
    const TAG_NAME: TagName<'xml> = <&str>::TAG_NAME;

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
    const TAG_NAME: TagName<'xml> = <T>::TAG_NAME;

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
