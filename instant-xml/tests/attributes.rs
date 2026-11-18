use std::borrow::Cow;

use similar_asserts::assert_eq;

use instant_xml::{FromXml, ToXml, from_str, to_string};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Basic {
    #[xml(attribute)]
    flag: bool,
}

#[test]
fn basic() {
    assert_eq!(
        from_str::<Basic>("<Basic flag=\"true\" />"),
        Ok(Basic { flag: true })
    );

    assert_eq!(
        to_string(&Basic { flag: true }).unwrap(),
        "<Basic flag=\"true\" />"
    );
}

#[derive(Debug, Eq, FromXml, PartialEq)]
struct Empty;

#[test]
fn empty() {
    assert_eq!(
        from_str::<Empty>("<?xml version=\"1.0\" ?><Empty />"),
        Ok(Empty)
    );
}

#[derive(FromXml, ToXml, PartialEq)]
#[xml(ns(bar = BAR))]
struct NoPrefixAttrNs {
    #[xml(attribute, ns(BAR))]
    flag: bool,
}

const BAR: &str = "BAR";

#[test]
fn no_prefix_attr_ns() {
    let v = NoPrefixAttrNs { flag: true };
    let xml = "<NoPrefixAttrNs xmlns:bar=\"BAR\" bar:flag=\"true\" />";
    assert_eq!(to_string(&v).unwrap(), xml);
    assert_eq!(from_str::<NoPrefixAttrNs>(xml).unwrap(), v);
}

#[derive(Debug, Eq, FromXml, PartialEq)]
struct BorrowedAttributes<'a> {
    #[xml(attribute, borrow)]
    first: Cow<'a, str>,
    #[xml(attribute, borrow)]
    second: Option<Cow<'a, str>>,
    #[xml(attribute, borrow)]
    third: Cow<'a, str>,
    #[xml(attribute, borrow)]
    fourth: Cow<'a, str>,
    child: Option<String>,
}

#[test]
fn multiple_borrowed_attributes() {
    let xml = r#"<BorrowedAttributes first="a" second="b" third="c" fourth="d&amp;e" />"#;
    let de = from_str::<BorrowedAttributes<'_>>(xml).unwrap();
    assert!(matches!(de.first, Cow::Borrowed("a")));
    assert!(matches!(de.second, Some(Cow::Borrowed("b"))));
    assert!(matches!(de.third, Cow::Borrowed("c")));
    assert!(matches!(&de.fourth, Cow::Owned(s) if s == "d&e"));
    assert_eq!(de.child, None);

    let xml = r#"<BorrowedAttributes fourth="d" third="c" first="a"><child>f</child></BorrowedAttributes>"#;
    let de = from_str::<BorrowedAttributes<'_>>(xml).unwrap();
    assert!(matches!(de.first, Cow::Borrowed("a")));
    assert_eq!(de.second, None);
    assert!(matches!(de.third, Cow::Borrowed("c")));
    assert!(matches!(de.fourth, Cow::Borrowed("d")));
    assert_eq!(de.child.as_deref(), Some("f"));
}

#[derive(Debug, Eq, FromXml, PartialEq)]
struct BorrowedAttributesParent<'a> {
    #[xml(borrow)]
    inner: BorrowedAttributes<'a>,
    after: String,
}

#[test]
fn borrowed_attributes_nested() {
    let xml = r#"<BorrowedAttributesParent><BorrowedAttributes first="a" second="b" third="c" fourth="d"><child>e</child></BorrowedAttributes><after>f</after></BorrowedAttributesParent>"#;
    let de = from_str::<BorrowedAttributesParent<'_>>(xml).unwrap();
    assert_eq!(de.inner.first, "a");
    assert_eq!(de.inner.second.as_deref(), Some("b"));
    assert_eq!(de.inner.third, "c");
    assert_eq!(de.inner.fourth, "d");
    assert_eq!(de.inner.child.as_deref(), Some("e"));
    assert_eq!(de.after, "f");
}
