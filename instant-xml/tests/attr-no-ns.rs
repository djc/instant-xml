use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, PartialEq, FromXml, ToXml)]
struct Basic {
    #[xml(attribute)]
    flag: bool,
}

#[test]
fn basic() {
    assert_eq!(
        from_str::<Basic>("<Basic flag=\"true\"></Basic>"),
        Ok(Basic { flag: true })
    );

    assert_eq!(
        to_string(&Basic { flag: true }).unwrap(),
        "<Basic flag=\"true\"></Basic>"
    );
}
