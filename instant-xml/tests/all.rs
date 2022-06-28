use instant_xml::{FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Unit;

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithNamedFields {
    flag: bool,
    #[xml(namespace("bar"))]
    string: String,
    #[xml(namespace("typo"))]
    number: i32,
}

#[test]
fn unit() {
    assert_eq!(Unit.to_xml().unwrap(), "<Unit></Unit>");
    assert_eq!(Unit::from_xml("<Unit/>").unwrap(), Unit);
}

#[test]
fn struct_with_named_fields() {
    assert_eq!(
        StructWithNamedFields {
            flag: true,
            string: "test".to_string(),
            number: 1,
        }
        .to_xml()
        .unwrap(),
        "<StructWithNamedFields xmlns=\"URI\">>true</flag><string xmlns=\"BAZ\">test</string><number xmlns=\"URI\">1</number></StructWithNamedFields>"
    );
}
