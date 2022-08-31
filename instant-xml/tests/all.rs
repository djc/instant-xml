use std::borrow::Cow;

use instant_xml::{Error, FromXml, ToXml};

//TODO: Add compile time errors check?

#[derive(Debug, Eq, PartialEq, ToXml)]
struct Unit;

#[test]
fn unit() {
    assert_eq!(Unit.to_xml().unwrap(), "<Unit></Unit>");
    //assert_eq!(Unit::from_xml("<Unit/>").unwrap(), Unit);
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace(bar = "BAZ", foo = "BAR"))]
struct StructWithNamedFields {
    flag: bool,
    #[xml(namespace(bar))]
    string: String,
    #[xml(namespace("typo"))]
    number: i32,
}

// Tests:
// - Empty default namespace
// - Prefix namespace
// - Direct namespace

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
        "<StructWithNamedFields xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><flag>true</flag><bar:string>test</bar:string><number xmlns=\"typo\">1</number></StructWithNamedFields>"
    );
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace("URI", dar = "BAZ", internal = "INTERNAL"))]
struct Nested {
    #[xml(namespace(dar))]
    flag_parent_prefix: bool,
    #[xml(namespace(internal))]
    flag_internal_prefix: bool,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithCustomField {
    #[xml(attribute)]
    int_attribute: i32,
    #[xml(namespace("BAZ"))]
    flag_direct_namespace_same_the_same_as_prefix: bool,
    #[xml(namespace(bar))]
    flag_prefix: bool,
    #[xml(namespace("DIFFERENT"))]
    flag_direct_namespace: bool,
    test: Nested,
}

// Tests:
// - The same direct namespace as the one from prefix
// - Attribute handling
// - Omitting redeclared child default namespace
// - Omitting redeclared child namespace with different prefix
// - Unique direct namespace
// - Child unique prefix
// - Child repeated prefix
// - Child default namespace the same as parent
#[test]
fn struct_with_custom_field() {
    assert_eq!(
        StructWithCustomField {
            int_attribute: 42,
            flag_direct_namespace_same_the_same_as_prefix: true,
            flag_prefix: false,
            flag_direct_namespace: true,
            test: Nested {
                flag_parent_prefix: true,
                flag_internal_prefix: false,
            },
        }
        .to_xml()
        .unwrap(),
        "<StructWithCustomField xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\" int_attribute=\"42\"><flag_direct_namespace_same_the_same_as_prefix xmlns=\"BAZ\">true</flag_direct_namespace_same_the_same_as_prefix><bar:flag_prefix>false</bar:flag_prefix><flag_direct_namespace xmlns=\"DIFFERENT\">true</flag_direct_namespace><Nested xmlns:internal=\"INTERNAL\"><bar:flag_parent_prefix>true</bar:flag_parent_prefix><internal:flag_internal_prefix>false</internal:flag_internal_prefix></Nested></StructWithCustomField>"
    );
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace(dar = "BAZ", internal = "INTERNAL"))]
struct NestedDifferentNamespace {
    #[xml(namespace(dar))]
    flag_parent_prefix: bool,
    #[xml(namespace(internal))]
    flag_internal_prefix: bool,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructChildNamespaces {
    different_child_namespace: NestedDifferentNamespace,
    same_child_namespace: Nested,
}

// Tests:
// - Different child namespace
// - The same child namespace
#[test]
fn struct_child_namespaces() {
    assert_eq!(
        StructChildNamespaces {
            different_child_namespace: NestedDifferentNamespace {
                flag_parent_prefix: true,
                flag_internal_prefix: false,
            },
            same_child_namespace: Nested {
                flag_parent_prefix: true,
                flag_internal_prefix: false,
            },
        }
        .to_xml()
        .unwrap(),
        "<StructChildNamespaces xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><NestedDifferentNamespace xmlns=\"\" xmlns:internal=\"INTERNAL\"><bar:flag_parent_prefix>true</bar:flag_parent_prefix><internal:flag_internal_prefix>false</internal:flag_internal_prefix></NestedDifferentNamespace><Nested xmlns:internal=\"INTERNAL\"><bar:flag_parent_prefix>true</bar:flag_parent_prefix><internal:flag_internal_prefix>false</internal:flag_internal_prefix></Nested></StructChildNamespaces>"
    );
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(namespace("URI", bar = "BAZ"))]
struct NestedDe {
    #[xml(namespace(bar))]
    flag: bool,
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

#[derive(Debug, Eq, PartialEq, FromXml)]
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

#[derive(Debug, PartialEq, ToXml)]
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
    cow: Cow<'a, str>,
    option: Option<&'a str>,
}

#[test]
fn scalars() {
    // Option some
    assert_eq!(
        StructDeserializerScalars{
            bool_type: true,
            i8_type: 1,
            u32_type: 42,
            string_type: "string".to_string(),
            str_type_a: "lifetime a",
            str_type_b: "lifetime b",
            char_type: 'c',
            f32_type: 1.20,
            cow: Cow::from("123"),
            option: Some("asd"),
        }
        .to_xml()
        .unwrap(),
        "<StructDeserializerScalars xmlns=\"URI\"><bool_type>true</bool_type><i8_type>1</i8_type><u32_type>42</u32_type><string_type>string</string_type><str_type_a>lifetime a</str_type_a><str_type_b>lifetime b</str_type_b><char_type>c</char_type><f32_type>1.2</f32_type><cow>123</cow><option>asd</option></StructDeserializerScalars>"
    );

    // Option none
    assert_eq!(
        StructDeserializerScalars{
            bool_type: true,
            i8_type: 1,
            u32_type: 42,
            string_type: "string".to_string(),
            str_type_a: "lifetime a",
            str_type_b: "lifetime b",
            char_type: 'c',
            f32_type: 1.20,
            cow: Cow::from("123"),
            option: None,
        }
        .to_xml()
        .unwrap(),
        "<StructDeserializerScalars xmlns=\"URI\"><bool_type>true</bool_type><i8_type>1</i8_type><u32_type>42</u32_type><string_type>string</string_type><str_type_a>lifetime a</str_type_a><str_type_b>lifetime b</str_type_b><char_type>c</char_type><f32_type>1.2</f32_type><cow>123</cow></StructDeserializerScalars>"
    );
}

#[derive(Debug, PartialEq, Eq, ToXml)]
#[xml(namespace("URI"))]
struct StructSpecialEntities<'a> {
    string_type: String,
    str_type_a: &'a str,
    cow: Cow<'a, str>,
}

#[test]
fn special_entities() {
    assert_eq!(
        StructSpecialEntities{
            string_type: "&\"<>\'aa".to_string(),
            str_type_a: "&\"<>\'bb",
            cow: Cow::from("&\"<>\'cc"),
        }
        .to_xml()
        .unwrap(),
        "<StructSpecialEntities xmlns=\"URI\"><string_type>&amp;&quot;&lt;&gt;&apos;aa</string_type><str_type_a>&amp;&quot;&lt;&gt;&apos;bb</str_type_a><cow>&amp;&quot;&lt;&gt;&apos;cc</cow></StructSpecialEntities>"
    );
}
