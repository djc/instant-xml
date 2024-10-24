use std::borrow::Cow;

use instant_xml::{from_str, to_string, FromXml, ToXml};
use similar_asserts::assert_eq;

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Foo {
    bar: Bar<'static>,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Bar<'a> {
    baz: Cow<'a, str>,
}

#[test]
fn lifetime() {
    let v = Foo {
        bar: Bar {
            baz: Cow::Borrowed("hello"),
        },
    };
    let xml = r#"<Foo><Bar><baz>hello</baz></Bar></Foo>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}
