use instant_xml::{FromXml, ToXml};

#[derive(Debug, Eq, PartialEq, ToXml, FromXml)]
struct Nested {
    #[xml(namespace(bar))]
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
struct NestedWrongPrefix {
    #[xml(namespace(dar))]
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
struct Unit;

#[test]
fn unit() {
    assert_eq!(Unit.to_xml().unwrap(), "<Unit></Unit>");
    //assert_eq!(Unit::from_xml("<Unit/>").unwrap(), Unit);
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
fn struct_with_named_fields() {
    assert_eq!(
        StructWithNamedFields {
            flag: true,
            string: "test".to_string(),
            number: 1,
        }
        .to_xml()
        .unwrap(),
        "<StructWithNamedFields xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><flag>true</flag><bar:string>test</bar:string><number xmlns=\"typo\">1</number></StructWithNamedFields>"
    );
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithCustomField {
    test: Nested,
}

#[test]
fn struct_with_custom_field() {
    assert_eq!(
        StructWithCustomField {
            test: Nested {
                flag: true,
            },
        }
        .to_xml()
        .unwrap(),
        "<StructWithCustomField xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><Nested><bar:flag>true</bar:flag></Nested></StructWithCustomField>"

    );
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithCustomFieldWrongPrefix {
    test: NestedWrongPrefix,
}

#[test]
#[should_panic]
fn struct_with_custom_field_wrong_prefix() {
    StructWithCustomFieldWrongPrefix {
        test: NestedWrongPrefix { flag: true },
    }
    .to_xml()
    .unwrap();
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithCustomFieldFromXml {
    #[xml(namespace(bar))]
    flag: bool,
    #[xml(attribute)]
    flag_attribute: bool,
    test: Nested,
}

#[test]
fn struct_with_custom_field_from_xml() {
    assert_eq!(
        StructWithCustomFieldFromXml::from_xml("<StructWithCustomFieldFromXml flag_attribute=\"true\" xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><bar:flag>false</bar:flag><Nested><flag>true</flag></Nested></StructWithCustomFieldFromXml>").unwrap(),
        StructWithCustomFieldFromXml {
            flag: false,
            flag_attribute: true,
            test: Nested { flag: true }
        }
    );
    // Different order
    assert_eq!(
        StructWithCustomFieldFromXml::from_xml("<StructWithCustomFieldFromXml xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\" flag_attribute=\"true\"><Nested><flag>true</flag></Nested><flag>false</flag></StructWithCustomFieldFromXml>").unwrap(),
        StructWithCustomFieldFromXml {
            flag: false,
            flag_attribute: true,
            test: Nested { flag: true }
        }
    );

    assert_eq!(
        Nested::from_xml("<Nested><flag>true</flag></Nested>").unwrap(),
        Nested { flag: true }
    );
}
