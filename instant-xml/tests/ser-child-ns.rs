use similar_asserts::assert_eq;

use instant_xml::{to_string, ToXml};

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(ns(dar = "BAZ", internal = INTERNAL))]
struct NestedDifferentNamespace {
    #[xml(ns(INTERNAL))]
    flag_internal_prefix: bool,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(ns("URI", bar = "BAZ", foo = "BAR"))]
struct StructChildNamespaces {
    different_child_namespace: NestedDifferentNamespace,
    same_child_namespace: Nested,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(ns("URI", dar = DAR, internal = INTERNAL))]
struct Nested {
    #[xml(ns(DAR))]
    flag_parent_prefix: bool,
    #[xml(ns(INTERNAL))]
    flag_internal_prefix: bool,
}

const DAR: &str = "BAZ";
const INTERNAL: &str = "INTERNAL";

// Tests:
// - Different child namespace
// - The same child namespace
#[test]
fn struct_child_namespaces() {
    assert_eq!(
        to_string(&StructChildNamespaces {
            different_child_namespace: NestedDifferentNamespace {
                flag_internal_prefix: false,
            },
            same_child_namespace: Nested {
                flag_parent_prefix: true,
                flag_internal_prefix: false,
            },
        })
        .unwrap(),
        "<StructChildNamespaces xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><NestedDifferentNamespace xmlns=\"\" xmlns:internal=\"INTERNAL\"><internal:flag_internal_prefix>false</internal:flag_internal_prefix></NestedDifferentNamespace><Nested xmlns:internal=\"INTERNAL\"><bar:flag_parent_prefix>true</bar:flag_parent_prefix><internal:flag_internal_prefix>false</internal:flag_internal_prefix></Nested></StructChildNamespaces>"
    );
}
