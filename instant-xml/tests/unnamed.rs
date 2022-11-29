use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct OneNumber(i32);

#[test]
fn one_number() {
    let v = OneNumber(42);
    let xml = r#"<OneNumber>42</OneNumber>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct OneString(String);

#[test]
fn one_string() {
    let v = OneString("f42".to_owned());
    let xml = r#"<OneString>f42</OneString>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct StringElement(String, Foo);

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Foo;

#[test]
fn string_element() {
    let v = StringElement("f42".to_owned(), Foo);
    let xml = r#"<StringElement>f42<Foo></Foo></StringElement>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}
