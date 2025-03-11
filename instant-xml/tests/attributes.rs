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
