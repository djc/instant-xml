use instant_xml::{from_str, FromXml};
use similar_asserts::assert_eq;

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI", bar = BAR))]
struct NestedDe {
    #[xml(ns(BAR))]
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithCustomFieldFromXml {
    #[xml(ns(BAR))]
    r#flag: bool,
    #[xml(attribute)]
    flag_attribute: bool,
    test: NestedDe,
}

const BAR: &str = "BAZ";

#[test]
fn struct_with_custom_field_from_xml() {
    assert_eq!(
        from_str::<StructWithCustomFieldFromXml>("<StructWithCustomFieldFromXml flag_attribute=\"true\" xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><bar:flag>false</bar:flag><NestedDe><bar:flag>true</bar:flag></NestedDe></StructWithCustomFieldFromXml>").unwrap(),
        StructWithCustomFieldFromXml {
            flag: false,
            flag_attribute: true,
            test: NestedDe { flag: true }
        }
    );
    // Different order
    assert_eq!(
        from_str::<StructWithCustomFieldFromXml>("<StructWithCustomFieldFromXml xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\" flag_attribute=\"true\"><NestedDe><bar:flag>true</bar:flag></NestedDe><bar:flag>false</bar:flag></StructWithCustomFieldFromXml>").unwrap(),
        StructWithCustomFieldFromXml {
            flag: false,
            flag_attribute: true,
            test: NestedDe { flag: true }
        }
    );

    // Different prefixes then in definition
    assert_eq!(
        from_str::<StructWithCustomFieldFromXml>("<StructWithCustomFieldFromXml flag_attribute=\"true\" xmlns=\"URI\" xmlns:grr=\"BAZ\" xmlns:foo=\"BAR\"><grr:flag>false</grr:flag><NestedDe><grr:flag>true</grr:flag></NestedDe></StructWithCustomFieldFromXml>").unwrap(),
        StructWithCustomFieldFromXml {
            flag: false,
            flag_attribute: true,
            test: NestedDe { flag: true }
        }
    );

    assert_eq!(
        from_str::<NestedDe>(
            "<NestedDe xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedDe>"
        )
        .unwrap(),
        NestedDe { flag: true }
    );
}
