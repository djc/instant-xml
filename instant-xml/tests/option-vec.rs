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
