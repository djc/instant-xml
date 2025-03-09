use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

const NS: &str = "\\Some\\Namespace";
#[derive(ToXml, FromXml, PartialEq, Eq, Debug)]
#[xml(scalar, ns(NS))]
enum NestedEnum {
    Foo,
    Bar,
}

#[derive(ToXml, FromXml, PartialEq, Eq, Debug)]
#[xml(ns(NS))]
struct StructWithNamespacedEnumAttr {
    #[xml(attribute)]
    scalar: NestedEnum,
}

#[test]
fn toxml_parentstruct_test() {
    let xml_string = format!(
        r##"<StructWithNamespacedEnumAttr xmlns="{}" scalar="Foo" />"##,
        NS
    );
    let parent = StructWithNamespacedEnumAttr {
        scalar: NestedEnum::Foo,
    };
    let parent_string = to_string(&parent).unwrap();

    assert_eq!(parent_string, xml_string);
}

#[test]
fn fromxml_parentstruct_test() {
    let xml_string = format!(
        r##"<StructWithNamespacedEnumAttr xmlns="{}" scalar="Foo" />"##,
        NS
    );
    let parent = StructWithNamespacedEnumAttr {
        scalar: NestedEnum::Foo,
    };
    let parent_roundtrip = from_str::<StructWithNamespacedEnumAttr>(&xml_string).unwrap();

    assert_eq!(parent_roundtrip, parent); //this fails
}
