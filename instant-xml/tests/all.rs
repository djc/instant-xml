use instant_xml::{Error, FromXml, ToXml};

#[derive(Debug, Eq, PartialEq, ToXml, FromXml)]
#[xml(namespace("URI", bar = "BAZ"))]
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
        "<StructWithCustomField xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><Nested xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></Nested></StructWithCustomField>"

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

    // Different prefixes then in definition
    assert_eq!(
        StructWithCustomFieldFromXml::from_xml("<StructWithCustomFieldFromXml flag_attribute=\"true\" xmlns=\"URI\" xmlns:grr=\"BAZ\" xmlns:foo=\"BAR\"><grr:flag>false</grr:flag><Nested><flag>true</flag></Nested></StructWithCustomFieldFromXml>").unwrap(),
        StructWithCustomFieldFromXml {
            flag: false,
            flag_attribute: true,
            test: Nested { flag: true }
        }
    );

    assert_eq!(
        Nested::from_xml(
            "<Nested xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></Nested>"
        )
        .unwrap(),
        Nested { flag: true }
    );
}

#[derive(Debug, Eq, PartialEq, ToXml, FromXml)]
struct NestedWrongNamespace {
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(namespace("URI", bar = "BAZ"))]
struct StructWithCorrectNestedNamespace {
    test: Nested,
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
        Nested::from_xml(
            "<Nested xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></Nested>"
        )
        .unwrap(),
        Nested { flag: true }
    );

    // Default namespace not-nested - wrong namespace
    assert_eq!(
        Nested::from_xml(
            "<Nested xmlns=\"WRONG\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></Nested>"
        )
        .unwrap_err(),
        Error::WrongNamespace
    );

    // Correct child namespace
    assert_eq!(
        StructWithCorrectNestedNamespace::from_xml("<StructWithCorrectNestedNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><Nested xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></Nested></StructWithCorrectNestedNamespace>").unwrap(),
        StructWithCorrectNestedNamespace {
            test: Nested { flag: true }
        }
    );

    // Correct child namespace - without child redefinition
    assert_eq!(
        StructWithCorrectNestedNamespace::from_xml("<StructWithCorrectNestedNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><Nested><bar:flag>true</bar:flag></Nested></StructWithCorrectNestedNamespace>").unwrap(),
        StructWithCorrectNestedNamespace {
            test: Nested { flag: true }
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
        Error::UnexpectedPrefix
    );

    // Other namespace not-nested - wrong parser namespace
    assert_eq!(
        NestedOtherNamespace::from_xml(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"WRONG\"><bar:flag>true</bar:flag></NestedOtherNamespace>"
        )
        .unwrap_err(),
        Error::MissingdPrefix
    );

    // Other namespace not-nested - missing parser prefix
    assert_eq!(
        NestedOtherNamespace::from_xml(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAR\"><flag>true</flag></NestedOtherNamespace>"
        )
        .unwrap_err(),
        Error::MissingdPrefix
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
        Error::UnexpectedPrefix
    );
}
