use std::fmt;

use similar_asserts::assert_eq;

use instant_xml::{Error, FromXml, Serializer, ToXml, from_str, to_string};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Item {
    #[xml(attribute, skip_serializing_if = "String::is_empty")]
    label: String,
    #[xml(skip_serializing_if = "is_zero")]
    count: u32,
    #[xml(skip_serializing_if = "is_default")]
    total: u64,
}

#[test]
fn skip_serializing_if() {
    let v = Item {
        label: "a".to_owned(),
        count: 1,
        total: 2,
    };
    let xml = r#"<Item label="a"><count>1</count><total>2</total></Item>"#;
    assert_eq!(to_string(&v).unwrap(), xml);
    assert_eq!(from_str::<Item>(xml).unwrap(), v);

    let v = Item {
        label: String::new(),
        count: 0,
        total: 0,
    };
    assert_eq!(to_string(&v).unwrap(), "<Item></Item>");
}

#[derive(ToXml)]
struct With {
    #[xml(serialize_with = "serialize_count", skip_serializing_if = "is_zero")]
    count: u32,
}

#[test]
fn skip_serializing_if_with() {
    assert_eq!(
        to_string(&With { count: 1 }).unwrap(),
        "<With>count: 1</With>"
    );
    assert_eq!(to_string(&With { count: 0 }).unwrap(), "<With></With>");
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Direct {
    #[xml(attribute)]
    flag: bool,
    #[xml(direct, skip_serializing_if = "String::is_empty")]
    text: String,
}

#[test]
fn skip_serializing_if_direct() {
    let v = Direct {
        flag: true,
        text: "a".to_owned(),
    };
    let xml = r#"<Direct flag="true">a</Direct>"#;
    assert_eq!(to_string(&v).unwrap(), xml);
    assert_eq!(from_str::<Direct>(xml).unwrap(), v);

    let v = Direct {
        flag: true,
        text: String::new(),
    };
    let xml = r#"<Direct flag="true" />"#;
    assert_eq!(to_string(&v).unwrap(), xml);
    assert_eq!(from_str::<Direct>(xml).unwrap(), v);
}

#[derive(ToXml)]
struct Outer {
    inner: Inline,
}

#[derive(ToXml)]
#[xml(transparent)]
struct Inline {
    #[xml(skip_serializing_if = "is_zero")]
    count: u32,
    name: String,
}

#[test]
fn skip_serializing_if_transparent() {
    let v = Outer {
        inner: Inline {
            count: 1,
            name: "a".to_owned(),
        },
    };
    assert_eq!(
        to_string(&v).unwrap(),
        "<Outer><count>1</count><name>a</name></Outer>"
    );

    let v = Outer {
        inner: Inline {
            count: 0,
            name: "a".to_owned(),
        },
    };
    assert_eq!(to_string(&v).unwrap(), "<Outer><name>a</name></Outer>");
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

fn serialize_count<W: fmt::Write + ?Sized>(
    value: &u32,
    serializer: &mut Serializer<'_, W>,
) -> Result<(), Error> {
    serializer.write_str(&format_args!("count: {value}"))
}
