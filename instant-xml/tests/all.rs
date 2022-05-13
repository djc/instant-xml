use instant_xml::{FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Unit;

#[test]
fn unit() {
    assert_eq!(Unit.to_xml().unwrap(), "<Unit/>");
    assert_eq!(Unit::from_xml("<Unit/>").unwrap(), Unit);
}
