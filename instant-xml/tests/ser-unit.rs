use instant_xml::{to_string, ToXml};
use similar_asserts::assert_eq;

#[derive(Debug, Eq, PartialEq, ToXml)]
struct Unit;

#[test]
fn unit() {
    assert_eq!(to_string(&Unit).unwrap(), "<Unit />");
}
