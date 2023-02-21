use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Foo {
    #[xml(attribute)]
    flag: bool,
    #[xml(direct)]
    inner: String,
}

#[test]
fn direct() {
    let v = Foo {
        flag: true,
        inner: "cbdté".to_string(),
    };
    let xml = "<Foo flag=\"true\">cbdté</Foo>";

    assert_eq!(to_string(&v).unwrap(), xml);
    assert_eq!(from_str::<Foo>(xml), Ok(v));
}
