use instant_xml::{to_string, ToXml};
use similar_asserts::assert_eq;

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
        "<NoPrefixAttrNs xmlns:bar=\"BAR\" bar:flag=\"true\"></NoPrefixAttrNs>"
    );
}
