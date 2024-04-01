use similar_asserts::assert_eq;

use instant_xml::{from_str, Error, FromXml};

#[derive(Debug, Eq, PartialEq, FromXml)]
struct NestedWrongNamespace {
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI", bar = "BAZ"))]
struct NestedDe {
    #[xml(ns("BAZ"))]
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI", bar = "BAZ"))]
struct StructWithCorrectNestedNamespace {
    test: NestedDe,
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI", bar = "BAZ"))]
struct StructWithWrongNestedNamespace {
    test: NestedWrongNamespace,
}

#[test]
fn default_namespaces() {
    // Default namespace not-nested
    assert_eq!(
        from_str("<NestedDe xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedDe>"),
        Ok(NestedDe { flag: true })
    );

    // Default namespace not-nested - wrong namespace
    assert_eq!(
        from_str(
            "<NestedDe xmlns=\"WRONG\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedDe>"
        ),
        Err::<NestedDe, _>(Error::UnexpectedValue(
            "unexpected root element \"NestedDe\" in namespace \"WRONG\"".to_owned()
        ))
    );

    // Correct child namespace
    assert_eq!(
        from_str("<StructWithCorrectNestedNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedDe xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedDe></StructWithCorrectNestedNamespace>"),
        Ok(StructWithCorrectNestedNamespace {
            test: NestedDe { flag: true }
        })
    );

    // Correct child namespace - without child redefinition
    assert_eq!(
        from_str("<StructWithCorrectNestedNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedDe><bar:flag>true</bar:flag></NestedDe></StructWithCorrectNestedNamespace>"),
        Ok(StructWithCorrectNestedNamespace {
            test: NestedDe { flag: true }
        })
    );

    // Different child namespace
    assert_eq!(
        from_str("<StructWithWrongNestedNamespace xmlns=\"URI\" xmlns:dar=\"BAZ\"><NestedWrongNamespace xmlns=\"\"><flag>true</flag></NestedWrongNamespace></StructWithWrongNestedNamespace>"),
        Ok(StructWithWrongNestedNamespace {
            test: NestedWrongNamespace {
                flag: true
            }
        })
    );

    // Wrong child namespace
    assert_eq!(
        from_str("<StructWithWrongNestedNamespace xmlns=\"URI\" xmlns:dar=\"BAZ\"><NestedWrongNamespace><flag>true</flag></NestedWrongNamespace></StructWithWrongNestedNamespace>"),
        Err::<StructWithWrongNestedNamespace, _>(
            Error::MissingValue("StructWithWrongNestedNamespace::test")
        )
    );
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI", bar = "BAZ"))]
struct NestedOtherNamespace {
    #[xml(ns("BAZ"))]
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI", bar = "BAZ"))]
struct StructOtherNamespace {
    test: NestedOtherNamespace,
}

#[test]
fn other_namespaces() {
    // Other namespace not-nested
    assert_eq!(
        from_str(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedOtherNamespace>"
        ),
        Ok(NestedOtherNamespace { flag: true })
    );

    // Other namespace not-nested - wrong defined namespace
    assert_eq!(
        from_str(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><wrong:flag>true</wrong:flag></NestedOtherNamespace>"
        ),
        Err::<NestedOtherNamespace, _>(Error::UnknownPrefix("wrong".to_owned()))
    );

    // Other namespace not-nested - wrong parser namespace
    assert_eq!(
        from_str(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"WRONG\"><bar:flag>true</bar:flag></NestedOtherNamespace>"
        ),
        Err::<NestedOtherNamespace, _>(Error::MissingValue("NestedOtherNamespace::flag"))
    );

    // Other namespace not-nested - missing parser prefix
    assert_eq!(
        from_str(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAR\"><flag>true</flag></NestedOtherNamespace>"
        ),
        Err::<NestedOtherNamespace, _>(Error::MissingValue("NestedOtherNamespace::flag"))
    );

    // Correct child other namespace
    assert_eq!(
        from_str(
            "<StructOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedOtherNamespace></StructOtherNamespace>"
        ),
        Ok(StructOtherNamespace {
            test: NestedOtherNamespace {
                flag: true,
            }
        })
    );

    // Correct child other namespace - without child redefinition
    assert_eq!(
        from_str(
            "<StructOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedOtherNamespace><bar:flag>true</bar:flag></NestedOtherNamespace></StructOtherNamespace>"
        ),
        Ok(StructOtherNamespace {
            test: NestedOtherNamespace {
                flag: true,
            }
        })
    );

    // Wrong child other namespace - without child redefinition
    assert_eq!(
        from_str(
            "<StructOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedOtherNamespace><wrong:flag>true</wrong:flag></NestedOtherNamespace></StructOtherNamespace>"
        ),
        Err::<StructOtherNamespace, _>(Error::UnknownPrefix("wrong".to_owned()))
    );
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI", dashed-ns = "dashed"))]
struct DashedNs {
    #[xml(ns("dashed"))]
    element: String,
}

#[test]
fn dashed_ns() {
    assert_eq!(
        from_str("<DashedNs xmlns=\"URI\" xmlns:dashed-ns=\"dashed\"><dashed-ns:element>hello</dashed-ns:element></DashedNs>"),
        Ok(DashedNs { element: "hello".to_owned() })
    );
}
