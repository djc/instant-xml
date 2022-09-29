use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use crate::{de::Node, Deserializer, Error, FromXml, Kind, Serializer, ToXml};

// Deserializer
struct FromXmlStr<T: FromStr>(Option<T>);

impl<'xml, T: FromStr> FromXml<'xml> for FromXmlStr<T> {
    fn deserialize(deserializer: &mut Deserializer<'_, 'xml>) -> Result<Self, Error> {
        let value = deserializer.take_str()?;
        match T::from_str(value) {
            Ok(value) => Ok(Self(Some(value))),
            Err(_) => Err(Error::UnexpectedValue),
        }
    }

    const KIND: Kind = Kind::Scalar;
}

impl<'xml> FromXml<'xml> for bool {
    fn deserialize<'cx>(deserializer: &'cx mut Deserializer<'cx, 'xml>) -> Result<Self, Error> {
        FromXmlStr::<Self>::deserialize(deserializer)?
            .0
            .ok_or(Error::MissingValue)
    }

    const KIND: Kind = Kind::Scalar;
}

// Serializer
struct DisplayToXml<'a, T: fmt::Display>(pub &'a T);

impl<'a, T> ToXml for DisplayToXml<'a, T>
where
    T: fmt::Display,
{
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        serializer.write_str(self.0)
    }

    const KIND: Kind = Kind::Scalar;
}

macro_rules! to_xml_for_number {
    ($typ:ty) => {
        impl ToXml for $typ {
            fn serialize<W: fmt::Write + ?Sized>(
                &self,
                serializer: &mut Serializer<W>,
            ) -> Result<(), Error> {
                DisplayToXml(self).serialize(serializer)
            }

            const KIND: Kind = DisplayToXml::<Self>::KIND;
        }
    };
}

macro_rules! from_xml_for_number {
    ($typ:ty) => {
        impl<'xml> FromXml<'xml> for $typ {
            fn deserialize<'cx>(
                deserializer: &'cx mut Deserializer<'cx, 'xml>,
            ) -> Result<Self, Error> {
                FromXmlStr::<Self>::deserialize(deserializer)?
                    .0
                    .ok_or(Error::MissingValue)
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

impl<'xml> FromXml<'xml> for char {
    fn deserialize<'cx>(deserializer: &'cx mut Deserializer<'cx, 'xml>) -> Result<Self, Error> {
        FromXmlStr::<Self>::deserialize(deserializer)?
            .0
            .ok_or(Error::MissingValue)
    }

    const KIND: Kind = Kind::Scalar;
}

impl<'xml> FromXml<'xml> for String {
    fn deserialize<'cx>(deserializer: &'cx mut Deserializer<'cx, 'xml>) -> Result<Self, Error> {
        Ok(<Cow<'xml, str> as FromXml<'xml>>::deserialize(deserializer)?.into_owned())
    }

    const KIND: Kind = Kind::Scalar;
}

impl<'xml> FromXml<'xml> for &'xml str {
    fn deserialize<'cx>(deserializer: &'cx mut Deserializer<'cx, 'xml>) -> Result<Self, Error> {
        Ok(
            match <Cow<'xml, str> as FromXml<'xml>>::deserialize(deserializer)? {
                Cow::Borrowed(s) => s,
                Cow::Owned(_) => return Err(Error::UnexpectedValue),
            },
        )
    }

    const KIND: Kind = Kind::Scalar;
}

impl<'xml> FromXml<'xml> for Cow<'xml, str> {
    fn deserialize(deserializer: &mut Deserializer<'_, 'xml>) -> Result<Self, Error> {
        let value = deserializer.take_str()?;
        Ok(decode(value))
    }

    const KIND: Kind = Kind::Scalar;
}

impl<'xml, T> FromXml<'xml> for Option<T>
where
    T: FromXml<'xml>,
{
    fn deserialize<'cx>(deserializer: &'cx mut Deserializer<'cx, 'xml>) -> Result<Self, Error> {
        match <T>::deserialize(deserializer) {
            Ok(v) => Ok(Some(v)),
            Err(e) => Err(e),
        }
    }

    fn missing_value() -> Result<Self, Error> {
        Ok(None)
    }

    const KIND: Kind = <T>::KIND;
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
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        let value = match self {
            true => "true",
            false => "false",
        };

        DisplayToXml(&value).serialize(serializer)
    }

    const KIND: Kind = DisplayToXml::<Self>::KIND;
}

impl ToXml for String {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        DisplayToXml(&encode(self)?).serialize(serializer)
    }

    const KIND: Kind = DisplayToXml::<Self>::KIND;
}

impl ToXml for char {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        let mut tmp = [0u8; 4];
        DisplayToXml(&encode(&*self.encode_utf8(&mut tmp))?).serialize(serializer)
    }

    const KIND: Kind = DisplayToXml::<Self>::KIND;
}

impl ToXml for &str {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        DisplayToXml(&encode(self)?).serialize(serializer)
    }

    const KIND: Kind = DisplayToXml::<Self>::KIND;
}

impl ToXml for Cow<'_, str> {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        DisplayToXml(&encode(self)?).serialize(serializer)
    }

    const KIND: Kind = DisplayToXml::<Self>::KIND;
}

impl<T: ToXml> ToXml for Option<T> {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        match self {
            Some(v) => v.serialize(serializer),
            None => Ok(()),
        }
    }

    const KIND: Kind = T::KIND;
}

fn encode(input: &str) -> Result<Cow<'_, str>, Error> {
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

fn decode(input: &str) -> Cow<'_, str> {
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

const VEC_ELEMENT_TAG: &str = "element";

impl<'xml, T> FromXml<'xml> for Vec<T>
where
    T: FromXml<'xml>,
{
    fn deserialize<'cx>(deserializer: &'cx mut Deserializer<'cx, 'xml>) -> Result<Self, Error> {
        let mut result = Self::new();

        while let Some(Ok(node)) = deserializer.next() {
            match node {
                Node::Open(data) => {
                    let id = deserializer.element_id(&data)?;

                    match id.name {
                        VEC_ELEMENT_TAG => {
                            let mut nested = deserializer.nested(data);
                            result.push(<T as FromXml<'xml>>::deserialize(&mut nested)?)
                        }
                        _ => return Err(Error::UnexpectedState),
                    }
                }
                _ => return Err(Error::UnexpectedState),
            }
        }

        Ok(result)
    }

    const KIND: Kind = Kind::Vec;
}

impl<T> ToXml for Vec<T>
where
    T: ToXml,
{
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        for i in self {
            let prefix = serializer.write_start(VEC_ELEMENT_TAG, "", false)?;
            serializer.end_start()?;
            i.serialize(serializer)?;
            serializer.write_close(prefix, VEC_ELEMENT_TAG)?;
        }

        Ok(())
    }

    const KIND: Kind = Kind::Vec;
}
