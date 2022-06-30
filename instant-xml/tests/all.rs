use instant_xml::{FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Unit;

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithCustomField {
    test: Nested,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
struct Nested {
    #[xml(namespace(bar))]
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithCustomFieldWrongPrefix {
    test: NestedWrongPrefix,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
struct NestedWrongPrefix {
    #[xml(namespace(dar))]
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithNamedFields {
    flag: bool,
    #[xml(namespace(bar))]
    string: String,
    #[xml(namespace("typo"))]
    number: i32,
}

#[test]
fn unit() {
    assert_eq!(Unit.to_xml(None).unwrap(), "<Unit></Unit>");
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
        .to_xml(None)
        .unwrap(),
        "<StructWithNamedFields xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><flag>true</flag><bar:string>test</bar:string><number xmlns=\"typo\">1</number></StructWithNamedFields>"
    );
}

#[test]
fn struct_with_custom_field() {
    assert_eq!(
        StructWithCustomField {
            test: Nested {
                flag: true,
            },
        }
        .to_xml(None)
        .unwrap(),
        "<StructWithCustomField xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><test><Nested><bar:flag>true</bar:flag></Nested></test></StructWithCustomField>"
    );
}

#[test]
#[should_panic]
fn struct_with_custom_field_wrong_prefix() {
    assert_eq!(
        StructWithCustomFieldWrongPrefix {
            test: NestedWrongPrefix { flag: true },
        }
        .to_xml(None)
        .unwrap(),
        ""
    );
}
