use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct OneNumber(i32);

#[test]
fn one_number() {
    let v = OneNumber(42);
    let xml = r#"<OneNumber>42</OneNumber>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}
