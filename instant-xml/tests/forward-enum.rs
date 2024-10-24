use std::borrow::Cow;

use instant_xml::{from_str, to_string, FromXml, ToXml};
use similar_asserts::assert_eq;

#[derive(Debug, FromXml, PartialEq, ToXml)]
#[xml(forward)]
enum Foo {
    Bar(Bar),
    Baz(Baz),
}

#[derive(Debug, FromXml, PartialEq, ToXml)]
struct Bar {
    bar: u8,
}

#[derive(Debug, FromXml, PartialEq, ToXml)]
struct Baz {
    baz: String,
}

#[test]
fn wrapped_enum() {
    let v = Foo::Bar(Bar { bar: 42 });
    let xml = r#"<Bar><bar>42</bar></Bar>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}

#[derive(Debug, FromXml, PartialEq, ToXml)]
#[xml(forward)]
enum FooCow<'a> {
    Bar(Cow<'a, [BarBorrowed<'a>]>),
    Baz(Cow<'a, [BazBorrowed<'a>]>),
}

#[derive(Clone, Debug, FromXml, PartialEq, ToXml)]
#[xml(rename = "Bar")]
struct BarBorrowed<'a> {
    bar: Cow<'a, str>,
}

#[derive(Clone, Debug, FromXml, PartialEq, ToXml)]
#[xml(rename = "Baz")]
struct BazBorrowed<'a> {
    baz: Cow<'a, str>,
}

#[test]
fn with_cow_accumulator() {
    let v = FooCow::Bar(Cow::Borrowed(&[BarBorrowed {
        bar: Cow::Borrowed("test"),
    }]));
    let xml = r#"<Bar><bar>test</bar></Bar>"#;

    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}
