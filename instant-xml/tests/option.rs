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
    let xml = r#"<Bar maybe="a" />"#;

    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());

    let v = Bar { maybe: None };
    let xml = r#"<Bar />"#;

    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}

#[test]
fn option_borrow_attribute() {
    let de = from_str::<Bar<'_>>(r#"<Bar maybe="a" />"#).unwrap();
    assert!(matches!(de.maybe, Some(Cow::Borrowed("a"))));

    let de = from_str::<Bar<'_>>(r#"<Bar maybe="a&amp;b" />"#).unwrap();
    assert!(matches!(de.maybe, Some(Cow::Owned(s)) if s == "a&b"));

    let de = from_str::<Bar<'_>>("<Bar />").unwrap();
    assert_eq!(de.maybe, None);
}

#[derive(Debug, Eq, FromXml, PartialEq)]
struct Baz<'a> {
    #[xml(borrow)]
    maybe: Option<Cow<'a, str>>,
}

#[test]
fn option_borrow_element() {
    let de = from_str::<Baz<'_>>("<Baz><maybe>a</maybe></Baz>").unwrap();
    assert!(matches!(de.maybe, Some(Cow::Borrowed("a"))));

    let de = from_str::<Baz<'_>>("<Baz><maybe>a&amp;b</maybe></Baz>").unwrap();
    assert!(matches!(de.maybe, Some(Cow::Owned(s)) if s == "a&b"));

    let de = from_str::<Baz<'_>>("<Baz><maybe/></Baz>").unwrap();
    assert!(matches!(de.maybe, Some(Cow::Borrowed(""))));

    let de = from_str::<Baz<'_>>("<Baz />").unwrap();
    assert_eq!(de.maybe, None);
}

#[derive(Debug, Eq, FromXml, PartialEq)]
struct Qux {
    maybe: Option<String>,
}

#[test]
fn option_borrow_empty_element() {
    // Same result as `Option<String>`
    let borrowed = from_str::<Baz<'_>>("<Baz><maybe/></Baz>").unwrap();
    let owned = from_str::<Qux>("<Qux><maybe/></Qux>").unwrap();
    assert_eq!(borrowed.maybe.as_deref(), owned.maybe.as_deref());
    assert_eq!(owned.maybe.as_deref(), Some(""));

    let borrowed = from_str::<Baz<'_>>("<Baz><maybe>test</maybe></Baz>").unwrap();
    let owned = from_str::<Qux>("<Qux><maybe>test</maybe></Qux>").unwrap();
    assert_eq!(borrowed.maybe.as_deref(), owned.maybe.as_deref());
    assert_eq!(owned.maybe.as_deref(), Some("test"));
}
