use instant_xml::{FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Unit;

#[derive(Debug, Eq, PartialEq, ToXml)]
struct StructWithNamedFields {
    flag: bool,
    string: String,
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
        "<StructWithNamedFields><flag>true</flag><string>test</string><number>1</number></StructWithNamedFields>"
    );
}
