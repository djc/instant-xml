use similar_asserts::assert_eq;

use instant_xml::{Error, FromXml};

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI"))]
struct StructDirectNamespace {
    #[xml(ns("BAZ"))]
    flag: bool,
}

#[test]
fn direct_namespaces() {
    // Correct direct namespace
    assert_eq!(
        StructDirectNamespace::from_xml(
            "<StructDirectNamespace xmlns=\"URI\"><flag xmlns=\"BAZ\">true</flag></StructDirectNamespace>"
        )
        .unwrap(),
        StructDirectNamespace { flag: true }
    );

    // Wrong direct namespace
    assert_eq!(
        StructDirectNamespace::from_xml(
            "<StructDirectNamespace xmlns=\"URI\"><flag xmlns=\"WRONG\">true</flag></StructDirectNamespace>"
        )
        .unwrap_err(),
        Error::MissingValue
    );

    // Wrong direct namespace - missing namespace
    assert_eq!(
        StructDirectNamespace::from_xml(
            "<StructDirectNamespace xmlns=\"URI\"><flag>true</flag></StructDirectNamespace>"
        )
        .unwrap_err(),
        Error::MissingValue
    );
}
