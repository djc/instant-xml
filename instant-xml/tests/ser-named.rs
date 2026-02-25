use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(ns(bar = "BAZ", foo = "BAR"))]
struct StructWithNamedFields {
    flag: bool,
    #[xml(ns("BAZ"))]
    string: String,
    #[xml(ns("typo"))]
    number: i32,
}

// Tests:
// - Empty default namespace
// - Prefix namespace
// - Direct namespace

#[test]
fn struct_with_named_fields() {
    assert_eq!(
        to_string(&StructWithNamedFields {
            flag: true,
            string: "test".to_string(),
            number: 1,
        })
        .unwrap(),
        "<StructWithNamedFields xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><flag>true</flag><bar:string>test</bar:string><number xmlns=\"typo\">1</number></StructWithNamedFields>"
    );
}

#[derive(Debug, FromXml, ToXml, PartialEq)]
#[xml(ns(b = "bar"))]
pub struct A {
    pub b: B,
}

#[derive(Debug, FromXml, ToXml, PartialEq)]
#[xml(ns("bar"))]
pub struct B {
    #[xml(ns("bar"))]
    pub b_prop: u32,
}

#[test]
fn prefixed_namespace() {
    let a = A {
        b: B { b_prop: 42 },
    };
    let xml = to_string(&a).unwrap();
    assert_eq!(
        xml,
        "<A xmlns:b=\"bar\"><b:B xmlns=\"bar\"><b_prop>42</b_prop></b:B></A>"
    );

    assert_eq!(from_str::<A>(&xml).unwrap(), a);
}
