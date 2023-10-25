use std::borrow::Cow;

use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Foo {
    inner: Option<Vec<String>>,
}

#[test]
fn option_vec() {
    let v = Foo {
        inner: Some(vec!["a".to_string(), "b".to_string()]),
    };
    let xml = r#"<Foo><inner>a</inner><inner>b</inner></Foo>"#;

    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Bar<'a> {
    #[xml(attribute, borrow)]
    maybe: Option<Cow<'a, str>>,
}

#[test]
fn option_borrow() {
    let v = Bar {
        maybe: Some("a".into()),
    };
    let xml = r#"<Bar maybe="a"></Bar>"#;

    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());

    let v = Bar { maybe: None };
    let xml = r#"<Bar></Bar>"#;

    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}
