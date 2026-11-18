use similar_asserts::assert_eq;

use instant_xml::{ToXml, to_string};

#[derive(Debug, Eq, PartialEq, ToXml)]
struct Unit;

#[test]
fn unit() {
    assert_eq!(to_string(&Unit).unwrap(), "<Unit />");
}
