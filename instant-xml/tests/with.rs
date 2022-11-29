use std::fmt;

use similar_asserts::assert_eq;

use instant_xml::{to_string, Error, Serializer, ToXml};

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
