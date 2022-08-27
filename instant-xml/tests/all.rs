use instant_xml::{Error, FromXml, ToXml};

#[derive(Debug, Eq, PartialEq, ToXml)]
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

#[derive(Debug, Eq, PartialEq, ToXml, FromXml)]
#[xml(namespace("URI", bar = "BAZ"))]
struct NestedDe {
    #[xml(namespace(bar))]
    flag: bool,
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
    test: NestedDe,
}

#[test]
fn struct_with_custom_field_from_xml() {
    assert_eq!(
        StructWithCustomFieldFromXml::from_xml("<StructWithCustomFieldFromXml flag_attribute=\"true\" xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><bar:flag>false</bar:flag><NestedDe><bar:flag>true</bar:flag></NestedDe></StructWithCustomFieldFromXml>").unwrap(),
        StructWithCustomFieldFromXml {
            flag: false,
            flag_attribute: true,
            test: NestedDe { flag: true }
        }
    );
    // Different order
    assert_eq!(
        StructWithCustomFieldFromXml::from_xml("<StructWithCustomFieldFromXml xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\" flag_attribute=\"true\"><NestedDe><bar:flag>true</bar:flag></NestedDe><bar:flag>false</bar:flag></StructWithCustomFieldFromXml>").unwrap(),
        StructWithCustomFieldFromXml {
            flag: false,
            flag_attribute: true,
            test: NestedDe { flag: true }
        }
    );

    // Different prefixes then in definition
    assert_eq!(
        StructWithCustomFieldFromXml::from_xml("<StructWithCustomFieldFromXml flag_attribute=\"true\" xmlns=\"URI\" xmlns:grr=\"BAZ\" xmlns:foo=\"BAR\"><grr:flag>false</grr:flag><NestedDe><grr:flag>true</grr:flag></NestedDe></StructWithCustomFieldFromXml>").unwrap(),
        StructWithCustomFieldFromXml {
            flag: false,
            flag_attribute: true,
            test: NestedDe { flag: true }
        }
    );

    assert_eq!(
        NestedDe::from_xml(
            "<NestedDe xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedDe>"
        )
        .unwrap(),
        NestedDe { flag: true }
    );
}

#[derive(Debug, Eq, PartialEq, ToXml, FromXml)]
struct NestedWrongNamespace {
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(namespace("URI", bar = "BAZ"))]
struct StructWithCorrectNestedNamespace {
    test: NestedDe,
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(namespace("URI", bar = "BAZ"))]
struct StructWithWrongNestedNamespace {
    test: NestedWrongNamespace,
}

#[test]
fn default_namespaces() {
    // Default namespace not-nested
    assert_eq!(
        NestedDe::from_xml(
            "<NestedDe xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedDe>"
        )
        .unwrap(),
        NestedDe { flag: true }
    );

    // Default namespace not-nested - wrong namespace
    assert_eq!(
        NestedDe::from_xml(
            "<NestedDe xmlns=\"WRONG\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedDe>"
        )
        .unwrap_err(),
        Error::WrongNamespace
    );

    // Correct child namespace
    assert_eq!(
        StructWithCorrectNestedNamespace::from_xml("<StructWithCorrectNestedNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedDe xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedDe></StructWithCorrectNestedNamespace>").unwrap(),
        StructWithCorrectNestedNamespace {
            test: NestedDe { flag: true }
        }
    );

    // Correct child namespace - without child redefinition
    assert_eq!(
        StructWithCorrectNestedNamespace::from_xml("<StructWithCorrectNestedNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedDe><bar:flag>true</bar:flag></NestedDe></StructWithCorrectNestedNamespace>").unwrap(),
        StructWithCorrectNestedNamespace {
            test: NestedDe { flag: true }
        }
    );

    // Different child namespace
    assert_eq!(
        StructWithWrongNestedNamespace::from_xml("<StructWithWrongNestedNamespace xmlns=\"URI\" xmlns:dar=\"BAZ\"><NestedWrongNamespace xmlns=\"\"><flag>true</flag></NestedWrongNamespace></StructWithWrongNestedNamespace>").unwrap(),
        StructWithWrongNestedNamespace {
            test: NestedWrongNamespace {
                flag: true
            }
        }
    );

    // Wrong child namespace
    assert_eq!(
        StructWithWrongNestedNamespace::from_xml("<StructWithWrongNestedNamespace xmlns=\"URI\" xmlns:dar=\"BAZ\"><NestedWrongNamespace><flag>true</flag></NestedWrongNamespace></StructWithWrongNestedNamespace>").unwrap_err(),
        Error::WrongNamespace
    );
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(namespace("URI", bar = "BAZ"))]
struct NestedOtherNamespace {
    #[xml(namespace(bar))]
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(namespace("URI", bar = "BAZ"))]
struct StructOtherNamespace {
    test: NestedOtherNamespace,
}

#[test]
fn other_namespaces() {
    // Other namespace not-nested
    assert_eq!(
        NestedOtherNamespace::from_xml(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedOtherNamespace>"
        )
        .unwrap(),
        NestedOtherNamespace { flag: true }
    );

    // Other namespace not-nested - wrong defined namespace
    assert_eq!(
        NestedOtherNamespace::from_xml(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><wrong:flag>true</wrong:flag></NestedOtherNamespace>"
        )
        .unwrap_err(),
        Error::WrongNamespace
    );

    // Other namespace not-nested - wrong parser namespace
    assert_eq!(
        NestedOtherNamespace::from_xml(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"WRONG\"><bar:flag>true</bar:flag></NestedOtherNamespace>"
        )
        .unwrap_err(),
        Error::WrongNamespace
    );

    // Other namespace not-nested - missing parser prefix
    assert_eq!(
        NestedOtherNamespace::from_xml(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAR\"><flag>true</flag></NestedOtherNamespace>"
        )
        .unwrap_err(),
        Error::WrongNamespace
    );

    // Correct child other namespace
    assert_eq!(
        StructOtherNamespace::from_xml(
            "<StructOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedOtherNamespace></StructOtherNamespace>"
        )
        .unwrap(),
        StructOtherNamespace {
            test: NestedOtherNamespace {
                flag: true,
            }
        }
    );

    // Correct child other namespace - without child redefinition
    assert_eq!(
        StructOtherNamespace::from_xml(
            "<StructOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedOtherNamespace><bar:flag>true</bar:flag></NestedOtherNamespace></StructOtherNamespace>"
        )
        .unwrap(),
        StructOtherNamespace {
            test: NestedOtherNamespace {
                flag: true,
            }
        }
    );

    // Wrong child other namespace - without child redefinition
    assert_eq!(
        StructOtherNamespace::from_xml(
            "<StructOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedOtherNamespace><wrong:flag>true</wrong:flag></NestedOtherNamespace></StructOtherNamespace>"
        )
        .unwrap_err(),
        Error::WrongNamespace
    );
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(namespace("URI"))]
struct StructDirectNamespace {
    #[xml(namespace("BAZ"))]
    flag: bool,
}

#[test]
fn direct_namespaces() {
    // Correct direct namespace
    assert_eq!(
        StructDirectNamespace::from_xml(
            "<StructDirectNamespace xmlns=\"URI\"><flag xmlns=\"BAZ\">true</flag></StructDirectNamespace>"
        )
        .unwrap(),
        StructDirectNamespace { flag: true }
    );

    // Wrong direct namespace
    assert_eq!(
        StructDirectNamespace::from_xml(
            "<StructDirectNamespace xmlns=\"URI\"><flag xmlns=\"WRONG\">true</flag></StructDirectNamespace>"
        )
        .unwrap_err(),
        Error::WrongNamespace
    );

    // Wrong direct namespace - missing namespace
    assert_eq!(
        StructDirectNamespace::from_xml(
            "<StructDirectNamespace xmlns=\"URI\"><flag>true</flag></StructDirectNamespace>"
        )
        .unwrap_err(),
        Error::WrongNamespace
    );
}

#[derive(Debug, PartialEq, FromXml)]
#[xml(namespace("URI"))]
struct StructDeserializerScalars<'a, 'b> {
    bool_type: bool,
    i8_type: i8,
    u32_type: u32,
    string_type: String,
    str_type_a: &'a str,
    str_type_b: &'b str,
    char_type: char,
    f32_type: f32,
}

#[test]
fn scalars() {
    assert_eq!(
        StructDeserializerScalars::from_xml(
            "<StructDeserializerScalars xmlns=\"URI\"><bool_type>true</bool_type><i8_type>1</i8_type><u32_type>42</u32_type><string_type>string</string_type><str_type_a>lifetime a</str_type_a><str_type_b>lifetime b</str_type_b><char_type>c</char_type><f32_type>1.20</f32_type></StructDeserializerScalars>"
        )
        .unwrap(),
        StructDeserializerScalars{
            bool_type: true,
            i8_type: 1,
            u32_type: 42,
            string_type: "string".to_string(),
            str_type_a: "lifetime a",
            str_type_b: "lifetime b",
            char_type: 'c',
            f32_type: 1.20,
        }
    );
}
