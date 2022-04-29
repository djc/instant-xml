use instant_xml::ToXml;

#[derive(ToXml)]
struct Unit;

#[test]
fn unit() {
    assert_eq!(Unit.to_xml().unwrap(), "<Unit/>");
}
