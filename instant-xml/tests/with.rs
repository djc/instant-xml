use std::fmt;

use similar_asserts::assert_eq;

use instant_xml::{Deserializer, Error, FromXml, Serializer, ToXml, from_str, to_string};

#[derive(ToXml)]
struct Foo {
    #[xml(serialize_with = "serialize_foo")]
    foo: u8,
}

fn serialize_foo<W: fmt::Write + ?Sized>(
    value: &u8,
    serializer: &mut Serializer<'_, W>,
) -> Result<(), Error> {
    serializer.write_str(&format_args!("foo: {value}"))
}

#[test]
fn serialize_with() {
    let v = Foo { foo: 42 };
    let xml = r#"<Foo>foo: 42</Foo>"#;
    assert_eq!(xml, to_string(&v).unwrap());
}

#[derive(Debug, FromXml, PartialEq)]
struct Bar {
    #[xml(attribute, deserialize_with = "deserialize_flag")]
    flag: bool,
}

fn deserialize_flag(
    into: &mut Option<bool>,
    field: &'static str,
    deserializer: &mut Deserializer<'_, '_>,
) -> Result<(), Error> {
    if into.is_some() {
        return Err(Error::DuplicateValue(field));
    }

    *into = match deserializer.take_str()?.as_deref() {
        Some("yes") => Some(true),
        Some("no") => Some(false),
        other => return Err(Error::UnexpectedValue(format!("{other:?}"))),
    };

    Ok(())
}

#[test]
fn deserialize_with_attribute() {
    assert_eq!(from_str(r#"<Bar flag="yes" />"#), Ok(Bar { flag: true }));
    assert_eq!(from_str(r#"<Bar flag="no" />"#), Ok(Bar { flag: false }));
}

#[derive(Debug, FromXml, PartialEq)]
struct Flags {
    #[xml(attribute, deserialize_with = "deserialize_flag_and_ignore")]
    first: bool,
    #[xml(attribute, deserialize_with = "deserialize_flag_and_ignore")]
    second: bool,
    #[xml(deserialize_with = "deserialize_flag_and_ignore")]
    third: bool,
    fourth: Option<String>,
}

fn deserialize_flag_and_ignore(
    into: &mut Option<bool>,
    field: &'static str,
    deserializer: &mut Deserializer<'_, '_>,
) -> Result<(), Error> {
    deserialize_flag(into, field, deserializer)?;
    deserializer.ignore()
}

#[test]
fn deserialize_with_ignore() {
    assert_eq!(
        from_str(
            r#"<Flags first="yes" second="no"><third>yes<x/></third><fourth>a</fourth></Flags>"#
        ),
        Ok(Flags {
            first: true,
            second: false,
            third: true,
            fourth: Some("a".to_owned()),
        })
    );
}
