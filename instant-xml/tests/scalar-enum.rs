use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(scalar)]
enum Foo {
    A,
    B,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Container {
    foo: Foo,
}

#[test]
fn scalar_enum() {
    let v = Container { foo: Foo::A };
    let xml = r#"<Container><foo>A</foo></Container>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(scalar, ns("URI", x = "URI"))]
enum Bar {
    A,
    B,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(ns("OTHER", x = "URI"))]
struct NsContainer {
    bar: Bar,
}

#[test]
fn scalar_enum_ns() {
    let v = NsContainer { bar: Bar::A };
    let xml = r#"<NsContainer xmlns="OTHER" xmlns:x="URI"><x:bar>A</x:bar></NsContainer>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}
