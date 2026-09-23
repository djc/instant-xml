use std::fmt;

use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, Deserializer, Error, FromXml, Serializer, ToXml};

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
