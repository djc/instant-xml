use std::borrow::Cow;
use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;
use std::{any::type_name, marker::PhantomData};

#[cfg(feature = "chrono")]
use chrono::{DateTime, NaiveDate, Utc};

use crate::{Accumulate, Deserializer, Error, FromXml, Id, Kind, Serializer, ToXml};

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
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'_, 'xml>,
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
                "unable to parse {} from `{value}` for {field}",
                type_name::<T>()
            ))),
        }
    }

    type Accumulator = Option<FromXmlStr<T>>;
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
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
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
                    "unable to parse bool from '{val}' for {field}"
                )))
            }
        };

        *into = Some(value);
        Ok(())
    }

    type Accumulator = Option<bool>;
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
                into: &mut Self::Accumulator,
                field: &'static str,
                deserializer: &mut Deserializer<'cx, 'xml>,
            ) -> Result<(), Error> {
                if into.is_some() {
                    return Err(Error::DuplicateValue);
                }

                let mut value = None;
                FromXmlStr::<Self>::deserialize(&mut value, field, deserializer)?;
                if let Some(value) = value {
                    *into = Some(value.0);
                }

                Ok(())
            }

            type Accumulator = Option<Self>;
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
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        let mut value = None;
        FromXmlStr::<Self>::deserialize(&mut value, field, deserializer)?;
        if let Some(value) = value {
            *into = Some(value.0);
        }

        Ok(())
    }

    type Accumulator = Option<Self>;
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
        into: &mut Self::Accumulator,
        _: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        match deserializer.take_str()? {
            Some(value) => *into = Some(decode(value)?.into_owned()),
            None => return Ok(()),
        }

        Ok(())
    }

    type Accumulator = Option<String>;
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
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        let value = match deserializer.take_str()? {
            Some(value) => value,
            None => return Ok(()),
        };

        match decode(value)? {
            Cow::Borrowed(str) => *into = Some(str),
            Cow::Owned(_) => {
                return Err(Error::UnexpectedValue(format!(
                    "string with escape characters cannot be deserialized as &str for {field}: '{value}'",
                )))
            }
        }

        Ok(())
    }

    type Accumulator = Option<&'xml str>;
    const KIND: Kind = Kind::Scalar;
}

impl<'xml, 'a> FromXml<'xml> for Cow<'a, str> {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        match field {
            Some(field) => id == field,
            None => false,
        }
    }

    fn deserialize(
        into: &mut Self::Accumulator,
        _: &'static str,
        deserializer: &mut Deserializer<'_, 'xml>,
    ) -> Result<(), Error> {
        if into.inner.is_some() {
            return Err(Error::DuplicateValue);
        }

        let value = match deserializer.take_str()? {
            Some(value) => value,
            None => return Ok(()),
        };

        into.inner = Some(decode(value)?.into_owned().into());
        Ok(())
    }

    type Accumulator = CowStrAccumulator<'xml, 'a>;
    const KIND: Kind = Kind::Scalar;
}

#[derive(Default)]
pub struct CowStrAccumulator<'xml, 'a> {
    pub(crate) inner: Option<Cow<'a, str>>,
    marker: PhantomData<&'xml str>,
}

impl<'xml, 'a> Accumulate<Cow<'a, str>> for CowStrAccumulator<'xml, 'a> {
    fn try_done(self, field: &'static str) -> Result<Cow<'a, str>, Error> {
        match self.inner {
            Some(inner) => Ok(inner),
            None => Err(Error::MissingValue(field)),
        }
    }
}

// The `FromXml` implementation for `Cow<'a, [T]>` always builds a `Cow::Owned`:
// it is not possible to deserialize into a `Cow::Borrowed` because there's no
// place to store the originating slice (length only known at run-time).
impl<'xml, 'a, T: FromXml<'xml>> FromXml<'xml> for Cow<'a, [T]>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        T::matches(id, field)
    }

    fn deserialize(
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'_, 'xml>,
    ) -> Result<(), Error> {
        let mut value = T::Accumulator::default();
        T::deserialize(&mut value, field, deserializer)?;
        into.push(value.try_done(field)?);
        Ok(())
    }

    type Accumulator = Vec<T>;
    const KIND: Kind = Kind::Scalar;
}

impl<'xml, T: FromXml<'xml>> FromXml<'xml> for Option<T> {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        T::matches(id, field)
    }

    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        <T>::deserialize(&mut into.value, field, deserializer)?;
        Ok(())
    }

    type Accumulator = OptionAccumulator<T, T::Accumulator>;
    const KIND: Kind = <T>::KIND;
}

pub struct OptionAccumulator<T, A: Accumulate<T>> {
    value: A,
    marker: PhantomData<T>,
}

impl<T, A: Accumulate<T>> OptionAccumulator<T, A> {
    pub fn get_mut(&mut self) -> &mut A {
        &mut self.value
    }
}

impl<T, A: Accumulate<T>> Default for OptionAccumulator<T, A> {
    fn default() -> Self {
        Self {
            value: A::default(),
            marker: PhantomData,
        }
    }
}

impl<T, A: Accumulate<T>> Accumulate<Option<T>> for OptionAccumulator<T, A> {
    fn try_done(self, field: &'static str) -> Result<Option<T>, Error> {
        match self.value.try_done(field) {
            Ok(value) => Ok(Some(value)),
            Err(_) => Ok(None),
        }
    }
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

impl ToXml for str {
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

    fn present(&self) -> bool {
        self.is_some()
    }
}

fn encode(input: &str) -> Result<Cow<'_, str>, Error> {
    let mut result = String::with_capacity(input.len());
    let mut last_end = 0;
    for (start, c) in input.char_indices() {
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

pub(crate) fn decode(input: &str) -> Result<Cow<'_, str>, Error> {
    let mut result = String::with_capacity(input.len());
    let (mut state, mut last_end) = (DecodeState::Normal, 0);
    for (i, &b) in input.as_bytes().iter().enumerate() {
        // use a state machine to find entities
        state = match (state, b) {
            (DecodeState::Normal, b'&') => DecodeState::Entity([0; 4], 0),
            (DecodeState::Normal, _) => DecodeState::Normal,
            (DecodeState::Entity(chars, len), b';') => {
                let decoded = match chars[..len] {
                    [b'a', b'm', b'p'] => '&',
                    [b'a', b'p', b'o', b's'] => '\'',
                    [b'g', b't'] => '>',
                    [b'l', b't'] => '<',
                    [b'q', b'u', b'o', b't'] => '"',
                    _ => {
                        return Err(Error::InvalidEntity(
                            String::from_utf8_lossy(&chars[..len]).into_owned(),
                        ))
                    }
                };

                let start = i - (len + 1); // current position - (length of entity characters + 1 for '&')
                if last_end < start {
                    // Unwrap should be safe: `last_end` and `start` must be at character boundaries.
                    result.push_str(input.get(last_end..start).unwrap());
                }

                last_end = i + 1;
                result.push(decoded);
                DecodeState::Normal
            }
            (DecodeState::Entity(mut chars, len), b) => {
                if len >= 4 {
                    let mut bytes = Vec::with_capacity(5);
                    bytes.extend(&chars[..len]);
                    bytes.push(b);
                    return Err(Error::InvalidEntity(
                        String::from_utf8_lossy(&bytes).into_owned(),
                    ));
                }

                chars[len] = b;
                DecodeState::Entity(chars, len + 1)
            }
        };
    }

    // Unterminated entity (& without ;) at end of input
    if let DecodeState::Entity(chars, len) = state {
        return Err(Error::InvalidEntity(
            String::from_utf8_lossy(&chars[..len]).into_owned(),
        ));
    }

    Ok(match result.is_empty() {
        true => Cow::Borrowed(input),
        false => {
            // Unwrap should be safe: `last_end` and `input.len()` must be at character boundaries.
            result.push_str(input.get(last_end..input.len()).unwrap());
            Cow::Owned(result)
        }
    })
}

#[derive(Debug)]
enum DecodeState {
    Normal,
    Entity([u8; 4], usize),
}

impl<'xml, T: FromXml<'xml>> FromXml<'xml> for Vec<T> {
    #[inline]
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool {
        T::matches(id, field)
    }

    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        let mut value = T::Accumulator::default();
        T::deserialize(&mut value, field, deserializer)?;
        into.push(value.try_done(field)?);
        Ok(())
    }

    type Accumulator = Vec<T>;
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
        into: &mut Self::Accumulator,
        _: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
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

    type Accumulator = Option<Self>;
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
        into: &mut Self::Accumulator,
        _: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
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

    type Accumulator = Option<Self>;
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
        into: &mut Self::Accumulator,
        _: &'static str,
        _: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        *into = Some(());
        Ok(())
    }

    type Accumulator = Option<Self>;
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
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue);
        }

        let mut value = None;
        FromXmlStr::<Self>::deserialize(&mut value, field, deserializer)?;
        if let Some(value) = value {
            *into = Some(value.0);
        }

        Ok(())
    }

    type Accumulator = Option<Self>;
    const KIND: Kind = Kind::Scalar;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode() {
        assert_eq!(decode("foo").unwrap(), "foo");
        assert_eq!(decode("foo &amp; bar").unwrap(), "foo & bar");
        assert_eq!(decode("foo &lt; bar").unwrap(), "foo < bar");
        assert_eq!(decode("foo &gt; bar").unwrap(), "foo > bar");
        assert_eq!(decode("foo &quot; bar").unwrap(), "foo \" bar");
        assert_eq!(decode("foo &apos; bar").unwrap(), "foo ' bar");
        assert_eq!(decode("foo &amp;lt; bar").unwrap(), "foo &lt; bar");
        assert_eq!(decode("&amp; foo").unwrap(), "& foo");
        assert_eq!(decode("foo &amp;").unwrap(), "foo &");
        assert_eq!(decode("cbdtéda&amp;sü").unwrap(), "cbdtéda&sü");
        assert!(decode("&").is_err());
        assert!(decode("foo&").is_err());
        assert!(decode("&bar").is_err());
        assert!(decode("&foo;").is_err());
        assert!(decode("&foobar;").is_err());
        assert!(decode("cbdtéd&ampü").is_err());
    }

    #[test]
    fn encode_unicode() {
        let input = "Iñtërnâ&tiônàlizætiøn";
        assert_eq!(encode(input).unwrap(), "Iñtërnâ&amp;tiônàlizætiøn");
    }
}
