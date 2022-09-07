use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, PartialEq, FromXml, ToXml)]
#[xml(rename = "renamed")]
struct Renamed {
    #[xml(attribute, rename = "renamed")]
    flag: bool,
}

#[test]
fn renamed() {
    assert_eq!(
        from_str::<Renamed>("<renamed renamed=\"true\"></renamed>"),
        Ok(Renamed { flag: true })
    );

    assert_eq!(
        to_string(&Renamed { flag: true }).unwrap(),
        "<renamed renamed=\"true\"></renamed>"
    );
}
