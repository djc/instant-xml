use std::any::type_name;
use std::borrow::Cow;
use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;

#[cfg(feature = "chrono")]
use chrono::{DateTime, NaiveDate, Utc};

use crate::{Deserializer, Error, FromXml, Id, Kind, Serializer, ToXml};

// Deserializer

pub fn from_xml_str<T: FromStr>(
    deserializer: &mut Deserializer<'_, '_>,
    into: &mut Option<T>,
) -> Result<(), Error> {
    if into.is_some() {
        return Err(Error::DuplicateValue);
    }

    let value = match deserializer.take_str()? {
        Some(value) => value,
        None => return Ok(()),
    };

    match T::from_str(value) {
        Ok(value) => {
            *into = Some(value);
            Ok(())
        }
        Err(_) => Err(Error::UnexpectedValue(format!(
            "unable to parse {} from `{value}`",
            type_name::<T>()
        ))),
    }
}

struct FromXmlStr<T: FromStr>(T);

impl<'xml, T: FromStr> FromXml<'xml> for FromXmlStr<T> {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => false,
        }
    }

    fn deserialize(
        deserializer: &mut Deserializer<'_, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        let value = match deserializer.take_str()? {
            Some(value) => value,
            None => return Ok(()),
        };

        match T::from_str(value) {
            Ok(value) => {
                *into = Some(FromXmlStr(value));
                Ok(())
            }
            Err(_) => Err(Error::UnexpectedValue(format!(
                "unable to parse {} from `{value}`",
                type_name::<T>()
            ))),
        }
    }

    const KIND: Kind = Kind::Scalar;
}

impl<'xml> FromXml<'xml> for bool {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => false,
        }
    }

    fn deserialize<'cx>(
        deserializer: &mut Deserializer<'cx, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        let value = match deserializer.take_str()? {
            Some(value) => value,
            None => return Ok(()),
        };

        let value = match value {
            "true" | "1" => true,
            "false" | "0" => false,
            val => {
                return Err(Error::UnexpectedValue(format!(
                    "unable to parse bool from '{val}'"
                )))
            }
        };

        *into = Some(value);
        Ok(())
    }

    const KIND: Kind = Kind::Scalar;
}

// Serializer

pub fn display_to_xml(
    value: &impl fmt::Display,
    field: Option<Id<'_>>,
    serializer: &mut Serializer<impl fmt::Write + ?Sized>,
) -> Result<(), Error> {
    DisplayToXml(value).serialize(field, serializer)
}

struct DisplayToXml<'a, T: fmt::Display>(pub &'a T);

impl<'a, T> ToXml for DisplayToXml<'a, T>
where
    T: fmt::Display,
{
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        let prefix = match field {
            Some(id) => {
                let prefix = serializer.write_start(id.name, id.ns)?;
                serializer.end_start()?;
                Some((prefix, id.name))
            }
            None => None,
        };

        serializer.write_str(self.0)?;
        if let Some((prefix, name)) = prefix {
            serializer.write_close(prefix, name)?;
        }

        Ok(())
    }
}

macro_rules! to_xml_for_number {
    ($typ:ty) => {
        impl ToXml for $typ {
            fn serialize<W: fmt::Write + ?Sized>(
                &self,
                field: Option<Id<'_>>,
                serializer: &mut Serializer<W>,
            ) -> Result<(), Error> {
                DisplayToXml(self).serialize(field, serializer)
            }
        }
    };
}

macro_rules! from_xml_for_number {
    ($typ:ty) => {
        impl<'xml> FromXml<'xml> for $typ {
            #[inline]
            fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
                match field {
                    Some(field) => id == field,
                    None => false,
                }
            }

            fn deserialize<'cx>(
                deserializer: &mut Deserializer<'cx, 'xml>,
                into: &mut Option<Self>,
            ) -> Result<(), Error> {
                if into.is_some() {
                    return Err(Error::DuplicateValue);
                }

                let mut value = None;
                FromXmlStr::<Self>::deserialize(deserializer, &mut value)?;
                if let Some(value) = value {
                    *into = Some(value.0);
                }

                Ok(())
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
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => false,
        }
    }

    fn deserialize<'cx>(
        deserializer: &mut Deserializer<'cx, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        let mut value = None;
        FromXmlStr::<Self>::deserialize(deserializer, &mut value)?;
        if let Some(value) = value {
            *into = Some(value.0);
        }

        Ok(())
    }

    const KIND: Kind = Kind::Scalar;
}

impl<'xml> FromXml<'xml> for String {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => false,
        }
    }

    fn deserialize<'cx>(
        deserializer: &mut Deserializer<'cx, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        match deserializer.take_str()? {
            Some(value) => *into = Some(decode(value).into_owned()),
            None => return Ok(()),
        }

        Ok(())
    }

    const KIND: Kind = Kind::Scalar;
}

impl<'xml> FromXml<'xml> for &'xml str {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => false,
        }
    }

    fn deserialize<'cx>(
        deserializer: &mut Deserializer<'cx, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        let value = match deserializer.take_str()? {
            Some(value) => value,
            None => return Ok(()),
        };

        match decode(value) {
            Cow::Borrowed(str) => *into = Some(str),
            Cow::Owned(_) => {
                return Err(Error::UnexpectedValue(format!(
                    "string with escape characters cannot be deserialized as &str: '{value}'",
                )))
            }
        }

        Ok(())
    }

    const KIND: Kind = Kind::Scalar;
}

impl<'xml, 'a, T: ?Sized> FromXml<'xml> for Cow<'a, T>
where
    T: ToOwned,
    T::Owned: FromXml<'xml>,
{
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => false,
        }
    }

    fn deserialize(
        deserializer: &mut Deserializer<'_, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        let mut value = None;
        T::Owned::deserialize(deserializer, &mut value)?;
        if let Some(value) = value {
            *into = Some(Cow::Owned(value));
        }

        Ok(())
    }

    const KIND: Kind = Kind::Scalar;
}

impl<'xml, T: FromXml<'xml>> FromXml<'xml> for Option<T> {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        T::matches(id, field)
    }

    fn deserialize<'cx>(
        deserializer: &mut Deserializer<'cx, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error> {
        match into.as_mut() {
            Some(value) => {
                <T>::deserialize(deserializer, value)?;
            }
            None => {
                let mut value = None;
                <T>::deserialize(deserializer, &mut value)?;
                if let Some(value) = value {
                    *into = Some(Some(value));
                }
            }
        }

        Ok(())
    }

    fn missing(_: &'static str) -> Result<Self, Error> {
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
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        let value = match self {
            true => "true",
            false => "false",
        };

        DisplayToXml(&value).serialize(field, serializer)
    }
}

impl ToXml for String {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        DisplayToXml(&encode(self)?).serialize(field, serializer)
    }
}

impl ToXml for char {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        let mut tmp = [0u8; 4];
        DisplayToXml(&encode(&*self.encode_utf8(&mut tmp))?).serialize(field, serializer)
    }
}

impl ToXml for &str {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        DisplayToXml(&encode(self)?).serialize(field, serializer)
    }
}

impl ToXml for Cow<'_, str> {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        DisplayToXml(&encode(self)?).serialize(field, serializer)
    }
}

impl<T: ToXml> ToXml for Option<T> {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        match self {
            Some(v) => v.serialize(field, serializer),
            None => Ok(()),
        }
    }
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

pub(crate) fn decode(input: &str) -> Cow<'_, str> {
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

impl<'xml, T: FromXml<'xml>> FromXml<'xml> for Vec<T> {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        T::matches(id, field)
    }

    fn deserialize<'cx>(
        deserializer: &mut Deserializer<'cx, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error> {
        let mut value = None;
        T::deserialize(deserializer, &mut value)?;
        let dst = into.get_or_insert(Vec::new());
        if let Some(value) = value {
            dst.push(value);
        }

        Ok(())
    }

    fn missing(_: &'static str) -> Result<Self, Error> {
        Ok(Vec::new())
    }

    const KIND: Kind = T::KIND;
}

impl<T: ToXml> ToXml for Vec<T> {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        self.as_slice().serialize(field, serializer)
    }
}

impl<T: ToXml> ToXml for [T] {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        for i in self {
            i.serialize(field, serializer)?;
        }

        Ok(())
    }
}

#[cfg(feature = "chrono")]
impl ToXml for DateTime<Utc> {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        let prefix = match field {
            Some(id) => {
                let prefix = serializer.write_start(id.name, id.ns)?;
                serializer.end_start()?;
                Some((prefix, id.name))
            }
            None => None,
        };

        serializer.write_str(&self.to_rfc3339())?;
        if let Some((prefix, name)) = prefix {
            serializer.write_close(prefix, name)?;
        }

        Ok(())
    }
}

#[cfg(feature = "chrono")]
impl<'xml> FromXml<'xml> for DateTime<Utc> {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => false,
        }
    }

    fn deserialize<'cx>(
        deserializer: &mut Deserializer<'cx, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        let value = match deserializer.take_str()? {
            Some(value) => value,
            None => return Ok(()),
        };

        match DateTime::parse_from_rfc3339(value) {
            Ok(dt) if dt.timezone().utc_minus_local() == 0 => {
                *into = Some(dt.with_timezone(&Utc));
                Ok(())
            }
            _ => Err(Error::Other("invalid date/time".into())),
        }
    }

    const KIND: Kind = Kind::Scalar;
}

#[cfg(feature = "chrono")]
impl ToXml for NaiveDate {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        let prefix = match field {
            Some(id) => {
                let prefix = serializer.write_start(id.name, id.ns)?;
                serializer.end_start()?;
                Some((prefix, id.name))
            }
            None => None,
        };

        serializer.write_str(&self)?;
        if let Some((prefix, name)) = prefix {
            serializer.write_close(prefix, name)?;
        }

        Ok(())
    }
}

#[cfg(feature = "chrono")]
impl<'xml> FromXml<'xml> for NaiveDate {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => false,
        }
    }

    fn deserialize<'cx>(
        deserializer: &mut Deserializer<'cx, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        let value = match deserializer.take_str()? {
            Some(value) => value,
            None => return Ok(()),
        };

        match NaiveDate::parse_from_str(value, "%Y-%m-%d") {
            Ok(d) => {
                *into = Some(d);
                Ok(())
            }
            _ => Err(Error::Other("invalid date/time".into())),
        }
    }

    const KIND: Kind = Kind::Scalar;
}

impl<'xml> FromXml<'xml> for () {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => false,
        }
    }

    fn deserialize<'cx>(
        _: &mut Deserializer<'cx, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error> {
        *into = Some(());
        Ok(())
    }

    const KIND: Kind = Kind::Scalar;
}

impl ToXml for IpAddr {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        DisplayToXml(self).serialize(field, serializer)
    }
}

impl<'xml> FromXml<'xml> for IpAddr {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => false,
        }
    }

    fn deserialize<'cx>(
        deserializer: &mut Deserializer<'cx, 'xml>,
        into: &mut Option<Self>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        let mut value = None;
        FromXmlStr::<Self>::deserialize(deserializer, &mut value)?;
        if let Some(value) = value {
            *into = Some(value.0);
        }

        Ok(())
    }

    const KIND: Kind = Kind::Scalar;
}
