use similar_asserts::assert_eq;

use instant_xml::FromXml;

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI", bar = "BAZ"))]
struct NestedDe {
    #[xml(ns(bar))]
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithCustomFieldFromXml {
    #[xml(ns(bar))]
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
