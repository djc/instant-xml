use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

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

#[derive(ToXml)]
#[xml(ns(bar = "BAR"))]
struct NoPrefixAttrNs {
    #[xml(attribute, ns(bar))]
    flag: bool,
}

#[test]
fn no_prefix_attr_ns() {
    assert_eq!(
        to_string(&NoPrefixAttrNs { flag: true }).unwrap(),
        "<NoPrefixAttrNs xmlns:bar=\"BAR\" bar:flag=\"true\" />"
    );
}
