use similar_asserts::assert_eq;

use instant_xml::{from_str, Error, FromXml};

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
        from_str(
            "<StructDirectNamespace xmlns=\"URI\"><flag xmlns=\"BAZ\">true</flag></StructDirectNamespace>"
        ),
        Ok(StructDirectNamespace { flag: true })
    );

    // Wrong direct namespace
    assert_eq!(
        from_str(
            "<StructDirectNamespace xmlns=\"URI\"><flag xmlns=\"WRONG\">true</flag></StructDirectNamespace>"
        ),
        Err::<StructDirectNamespace, _>(Error::MissingValue("StructDirectNamespace::flag"))
    );

    // Wrong direct namespace - missing namespace
    assert_eq!(
        from_str("<StructDirectNamespace xmlns=\"URI\"><flag>true</flag></StructDirectNamespace>"),
        Err::<StructDirectNamespace, _>(Error::MissingValue("StructDirectNamespace::flag"))
    );
}
