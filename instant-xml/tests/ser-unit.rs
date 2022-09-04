use similar_asserts::assert_eq;

use instant_xml::ToXml;

#[derive(Debug, Eq, PartialEq, ToXml)]
struct Unit;

#[test]
fn unit() {
    assert_eq!(Unit.to_xml().unwrap(), "<Unit></Unit>");
    //assert_eq!(Unit::from_xml("<Unit/>").unwrap(), Unit);
}
