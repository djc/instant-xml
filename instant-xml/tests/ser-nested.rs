use similar_asserts::assert_eq;

use instant_xml::{to_string, ToXml};

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(ns("URI", dar = "BAZ", internal = "INTERNAL"))]
struct Nested {
    #[xml(ns(dar))]
    flag_parent_prefix: bool,
    #[xml(ns(internal))]
    flag_internal_prefix: bool,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(ns("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithCustomField {
    #[xml(attribute)]
    int_attribute: i32,
    #[xml(ns("BAZ"))]
    flag_direct_namespace_same_the_same_as_prefix: bool,
    #[xml(ns(bar))]
    flag_prefix: bool,
    #[xml(ns("DIFFERENT"))]
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
        to_string(&StructWithCustomField {
            int_attribute: 42,
            flag_direct_namespace_same_the_same_as_prefix: true,
            flag_prefix: false,
            flag_direct_namespace: true,
            test: Nested {
                flag_parent_prefix: true,
                flag_internal_prefix: false,
            },
        })
        .unwrap(),
        "<StructWithCustomField xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\" int_attribute=\"42\"><flag_direct_namespace_same_the_same_as_prefix xmlns=\"BAZ\">true</flag_direct_namespace_same_the_same_as_prefix><bar:flag_prefix>false</bar:flag_prefix><flag_direct_namespace xmlns=\"DIFFERENT\">true</flag_direct_namespace><Nested xmlns:internal=\"INTERNAL\"><bar:flag_parent_prefix>true</bar:flag_parent_prefix><internal:flag_internal_prefix>false</internal:flag_internal_prefix></Nested></StructWithCustomField>"
    );
}
