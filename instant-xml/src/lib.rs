//! A serde-like library for rigorous XML (de)serialization.
//!
//! instant-xml provides traits and derive macros for mapping XML to Rust types,
//! with full support for XML namespaces and zero-copy deserialization.
//!
//! # Quick Start
//!
//! ```
//! # use instant_xml::{FromXml, ToXml, from_str, to_string};
//! #[derive(Debug, PartialEq, FromXml, ToXml)]
//! struct Person {
//!     name: String,
//!     #[xml(attribute)]
//!     age: u32,
//! }
//!
//! let person = Person {
//!     name: "Alice".to_string(),
//!     age: 30,
//! };
//!
//! let xml = to_string(&person).unwrap();
//! assert_eq!(xml, r#"<Person age="30"><name>Alice</name></Person>"#);
//!
//! let deserialized: Person = from_str(&xml).unwrap();
//! assert_eq!(person, deserialized);
//! ```
//!
//! # `#[xml(...)]` attribute reference
//!
//! The `#[xml(...)]` attribute configures serialization and deserialization behavior
//! for the [`ToXml`] and [`FromXml`] derive macros.
//!
//! ## Container attributes
//!
//! Applied to structs and enums using `#[xml(...)]`:
//!
//! - **`rename = "name"`** - renames the root element
//!
//!   ```
//!   # use instant_xml::{ToXml, to_string};
//!   #[derive(ToXml)]
//!   #[xml(rename = "custom-name")]
//!   struct MyStruct { }
//!
//!   assert_eq!(to_string(&MyStruct {}).unwrap(), "<custom-name />");
//!   ```
//!
//! - **`rename_all = "case"`** - transforms all field/variant names.
//!
//!   Supported cases: `"lowercase"`, `"UPPERCASE"`, `"PascalCase"`, `"camelCase"`,
//!   `"snake_case"`, `"SCREAMING_SNAKE_CASE"`, `"kebab-case"`, `"SCREAMING-KEBAB-CASE"`.
//!
//!   ```
//!   # use instant_xml::{ToXml, to_string};
//!   #[derive(ToXml)]
//!   #[xml(rename_all = "camelCase")]
//!   struct MyStruct {
//!       field_one: String,
//!   }
//!
//!   let s = MyStruct { field_one: "value".to_string() };
//!   assert_eq!(to_string(&s).unwrap(), "<MyStruct><fieldOne>value</fieldOne></MyStruct>");
//!   ```
//!
//! - **`ns("uri")` or `ns("uri", prefix = "namespace")`** - configures XML namespaces
//!
//!   Namespace URIs can be string literals or paths to constants. Prefixes may contain
//!   dashes and dots: `#[xml(ns(my-ns.v1 = "uri"))]`.
//!
//!   ```
//!   # use instant_xml::{ToXml, to_string};
//!   #[derive(ToXml)]
//!   #[xml(ns("http://example.com"))]
//!   struct Root { }
//!
//!   assert_eq!(to_string(&Root {}).unwrap(), r#"<Root xmlns="http://example.com" />"#);
//!
//!   #[derive(ToXml)]
//!   #[xml(ns("http://example.com", xsi = XSI))]
//!   struct WithPrefix { }
//!
//!   assert_eq!(
//!       to_string(&WithPrefix {}).unwrap(),
//!       r#"<WithPrefix xmlns="http://example.com" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" />"#
//!   );
//!
//!   const XSI: &'static str = "http://www.w3.org/2001/XMLSchema-instance";
//!   ```
//!
//! - **`transparent`** *(structs only)* - inlines fields without wrapper element
//!
//!   ```
//!   # use instant_xml::{ToXml, to_string};
//!   #[derive(ToXml)]
//!   #[xml(transparent)]
//!   struct Inline {
//!       foo: Foo,
//!       bar: Bar,
//!   }
//!
//!   #[derive(ToXml)]
//!   struct Foo { }
//!
//!   #[derive(ToXml)]
//!   struct Bar { }
//!
//!   let inline = Inline { foo: Foo {}, bar: Bar {} };
//!   assert_eq!(to_string(&inline).unwrap(), "<Foo /><Bar />");
//!   ```
//!
//! - **`scalar`** *(enums only)* - serializes variants as text content.
//!
//!   The enum must only have unit variants.
//!
//!   ```
//!   # use instant_xml::{ToXml, to_string};
//!
//!   #[derive(ToXml)]
//!   struct Container {
//!       status: Status,
//!   }
//!
//!   #[derive(ToXml)]
//!   #[xml(scalar)]
//!   enum Status {
//!       Active,
//!       Inactive,
//!   }
//!
//!   let c = Container { status: Status::Active };
//!   assert_eq!(to_string(&c).unwrap(), "<Container><status>Active</status></Container>");
//!   ```
//!
//!   Variants can use `#[xml(rename = "...")]` or string/integer discriminants.
//!
//! - **`forward`** *(enums only)* - forwards to inner type's element name.
//!
//!   Each variant must contain exactly one unnamed field.
//!
//!   ```
//!   # use instant_xml::{ToXml, to_string};
//!
//!   #[derive(ToXml)]
//!   #[xml(forward)]
//!   enum Message {
//!       Request(Request),
//!       Response(Response),
//!   }
//!
//!   #[derive(ToXml)]
//!   struct Request { }
//!
//!   #[derive(ToXml)]
//!   struct Response { }
//!
//!   let msg = Message::Request(Request {});
//!   assert_eq!(to_string(&msg).unwrap(), "<Request />");
//!   ```
//!
//! ## Field attributes
//!
//! Applied to struct fields using `#[xml(...)]`:
//!
//! - **`attribute`** - (de)serializes as XML attribute instead of child element
//!
//!   ```
//!   # use instant_xml::{ToXml, to_string};
//!   #[derive(ToXml)]
//!   struct Element {
//!       #[xml(attribute)]
//!       id: String,
//!   }
//!
//!   let elem = Element { id: "abc123".to_string() };
//!   assert_eq!(to_string(&elem).unwrap(), r#"<Element id="abc123" />"#);
//!   ```
//!
//! - **`direct`** - field contains element's direct text content
//!
//!   ```
//!   # use instant_xml::{ToXml, to_string};
//!   #[derive(ToXml)]
//!   struct Paragraph {
//!       #[xml(attribute)]
//!       lang: String,
//!       #[xml(direct)]
//!       text: String,
//!   }
//!
//!   let p = Paragraph { lang: "en".to_string(), text: "Hello".to_string() };
//!   assert_eq!(to_string(&p).unwrap(), r#"<Paragraph lang="en">Hello</Paragraph>"#);
//!   ```
//!
//! - **`rename = "name"`** - renames the field's element or attribute name
//!
//! - **`ns("uri")`** - sets namespace for this specific field. Like the container-level
//!   attribute, this supports both string literals and constant paths.
//!
//! - **`serialize_with = "path"`** - custom serialization function with signature:
//!
//!   ```
//!   # use instant_xml::{Error, Serializer, ToXml, to_string};
//!   # use std::fmt;
//!   #[derive(ToXml)]
//!   struct Config {
//!       #[xml(serialize_with = "serialize_custom")]
//!       count: u32,
//!   }
//!
//!   fn serialize_custom<W: fmt::Write + ?Sized>(
//!       value: &u32,
//!       serializer: &mut Serializer<'_, W>,
//!   ) -> Result<(), Error> {
//!       serializer.write_str(&format!("value: {}", value))?;
//!       Ok(())
//!   }
//!
//!   let config = Config { count: 42 };
//!   assert_eq!(to_string(&config).unwrap(), "<Config>value: 42</Config>");
//!   ```
//!
//! - **`deserialize_with = "path"`** - custom deserialization function with signature:
//!
//!   ```
//!   # use instant_xml::{Deserializer, Error, FromXml, from_str};
//!   #[derive(FromXml, PartialEq, Debug)]
//!   struct Config {
//!       #[xml(deserialize_with = "deserialize_bool")]
//!       enabled: bool,
//!   }
//!
//!   fn deserialize_bool<'xml>(
//!       accumulator: &mut <bool as FromXml<'xml>>::Accumulator,
//!       field: &'static str,
//!       deserializer: &mut Deserializer<'_, 'xml>,
//!   ) -> Result<(), Error> {
//!       if accumulator.is_some() {
//!           return Err(Error::DuplicateValue(field));
//!       }
//!
//!       let Some(s) = deserializer.take_str()? else {
//!           return Ok(());
//!       };
//!
//!       *accumulator = Some(match s.as_ref() {
//!           "yes" => true,
//!           "no" => false,
//!           other => return Err(Error::UnexpectedValue(
//!               format!("expected 'yes' or 'no', got '{}'", other)
//!           )),
//!       });
//!
//!       deserializer.ignore()?;
//!       Ok(())
//!   }
//!
//!   let xml = "<Config><enabled>yes</enabled></Config>";
//!   let config = from_str::<Config>(xml).unwrap();
//!   assert_eq!(config.enabled, true);
//!   ```
//!
//! - **`borrow`** - Borrows from input during deserialization. Automatically applies to
//!   top-level `&str` and `&[u8]` fields. Useful for `Cow<str>` and similar types.
//!
//!   ```
//!   # use instant_xml::{FromXml, from_str};
//!   # use std::borrow::Cow;
//!   #[derive(FromXml, PartialEq, Debug)]
//!   struct Borrowed<'a> {
//!       #[xml(borrow)]
//!       text: Cow<'a, str>,
//!   }
//!
//!   let xml = "<Borrowed><text>Hello</text></Borrowed>";
//!   let parsed = from_str::<Borrowed>(xml).unwrap();
//!   assert_eq!(parsed.text, "Hello");
//!   ```

use std::{borrow::Cow, fmt};

use thiserror::Error;

pub use macros::{FromXml, ToXml};

#[doc(hidden)]
pub mod de;
mod impls;
use de::Context;
pub use de::Deserializer;
pub use impls::{display_to_xml, from_xml_str, OptionAccumulator};
#[doc(hidden)]
pub mod ser;
pub use ser::Serializer;
mod any_element;
pub use any_element::{AnyAttribute, AnyElement};

/// Serialize a type to XML
pub trait ToXml {
    /// Serialize this value to XML using the provided serializer
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error>;

    /// Check if this value should be serialized
    ///
    /// Returns `false` for absent optional values, `true` otherwise.
    fn present(&self) -> bool {
        true
    }
}

impl<T: ToXml + ?Sized> ToXml for &T {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<W>,
    ) -> Result<(), Error> {
        (*self).serialize(field, serializer)
    }
}

/// Deserialize a type from XML
pub trait FromXml<'xml>: Sized {
    /// Check if an element or attribute matches this type
    fn matches(id: Id<'_>, field: Option<Id<'_>>) -> bool;

    /// Deserialize from XML into an accumulator
    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error>;

    /// The accumulator type used during deserialization
    type Accumulator: Accumulate<Self>;
    /// The kind of XML node this type represents
    const KIND: Kind;
}

/// Accumulate values during deserialization
///
/// A type implementing `Accumulate<T>` is used to accumulate a value of type `T`.
pub trait Accumulate<T>: Default {
    /// Convert the accumulator into the final value, or return an error
    fn try_done(self, field: &'static str) -> Result<T, Error>;
}

impl<T> Accumulate<T> for Option<T> {
    fn try_done(self, field: &'static str) -> Result<T, Error> {
        self.ok_or(Error::MissingValue(field))
    }
}

impl<T> Accumulate<Vec<T>> for Vec<T> {
    fn try_done(self, _: &'static str) -> Result<Vec<T>, Error> {
        Ok(self)
    }
}

impl<'a, T> Accumulate<Cow<'a, [T]>> for Vec<T>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    fn try_done(self, _: &'static str) -> Result<Cow<'a, [T]>, Error> {
        Ok(Cow::Owned(self))
    }
}

impl<T> Accumulate<Option<T>> for Option<T> {
    fn try_done(self, _: &'static str) -> Result<Option<T>, Error> {
        Ok(self)
    }
}

/// Deserialize a type from an XML string
pub fn from_str<'xml, T: FromXml<'xml>>(input: &'xml str) -> Result<T, Error> {
    let (mut context, root) = Context::new(input)?;
    let id = context.element_id(&root)?;

    if !T::matches(id, None) {
        return Err(Error::UnexpectedValue(match id.ns.is_empty() {
            true => format!("unexpected root element {:?}", id.name),
            false => format!(
                "unexpected root element {:?} in namespace {:?}",
                id.name, id.ns
            ),
        }));
    }

    let mut value = T::Accumulator::default();
    T::deserialize(
        &mut value,
        "<root element>",
        &mut Deserializer::new(root, &mut context),
    )?;
    value.try_done("<root element>")
}

/// Serialize a value to an XML string
pub fn to_string(value: &(impl ToXml + ?Sized)) -> Result<String, Error> {
    let mut output = String::new();
    to_writer(value, &mut output)?;
    Ok(output)
}

/// Serialize a value to an XML writer
pub fn to_writer(
    value: &(impl ToXml + ?Sized),
    output: &mut (impl fmt::Write + ?Sized),
) -> Result<(), Error> {
    value.serialize(None, &mut Serializer::new(output))
}

/// Marker trait for types that can be deserialized with any lifetime
pub trait FromXmlOwned: for<'xml> FromXml<'xml> {}

impl<T> FromXmlOwned for T where T: for<'xml> FromXml<'xml> {}

/// Errors that can occur during XML serialization and deserialization
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum Error {
    /// Error formatting output
    #[error("format: {0}")]
    Format(#[from] fmt::Error),
    /// Invalid XML entity encountered
    #[error("invalid entity: {0}")]
    InvalidEntity(String),
    /// Error parsing XML
    #[error("parse: {0}")]
    Parse(#[from] xmlparser::Error),
    /// Other error
    #[error("other: {0}")]
    Other(std::string::String),
    /// Unexpected end of XML stream
    #[error("unexpected end of stream")]
    UnexpectedEndOfStream,
    /// Unexpected value encountered
    #[error("unexpected value: '{0}'")]
    UnexpectedValue(String),
    /// Unexpected XML tag
    #[error("unexpected tag: {0}")]
    UnexpectedTag(String),
    /// Expected tag but none found
    #[error("missing tag")]
    MissingTag,
    /// Required field has no value
    #[error("missing value: {0}")]
    MissingValue(&'static str),
    /// Unexpected XML token
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    /// Unknown namespace prefix
    #[error("unknown prefix: {0}")]
    UnknownPrefix(String),
    /// Unexpected XML node type
    #[error("unexpected node: {0}")]
    UnexpectedNode(String),
    /// Internal state error
    #[error("unexpected state: {0}")]
    UnexpectedState(&'static str),
    /// Expected a scalar value but found an element
    #[error("expected scalar, found {0}")]
    ExpectedScalar(String),
    /// Field value appears more than once
    #[error("duplicate value for {0}")]
    DuplicateValue(&'static str),
}

/// The kind of XML node a type represents
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Kind {
    /// A scalar value (text content or attribute)
    Scalar,
    /// An XML element
    Element,
}

/// Identifier for an XML element or attribute with namespace
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Id<'a> {
    /// The namespace URI
    pub ns: &'a str,
    /// The local name
    pub name: &'a str,
}
