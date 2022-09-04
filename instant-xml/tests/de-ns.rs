use similar_asserts::assert_eq;

use instant_xml::{Error, FromXml};

#[derive(Debug, Eq, PartialEq, FromXml)]
struct NestedWrongNamespace {
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI", bar = "BAZ"))]
struct NestedDe {
    #[xml(ns(bar))]
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
        NestedDe::from_xml(
            "<NestedDe xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedDe>"
        )
        .unwrap(),
        NestedDe { flag: true }
    );

    // Default namespace not-nested - wrong namespace
    assert_eq!(
        NestedDe::from_xml(
            "<NestedDe xmlns=\"WRONG\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedDe>"
        )
        .unwrap_err(),
        Error::WrongNamespace
    );

    // Correct child namespace
    assert_eq!(
        StructWithCorrectNestedNamespace::from_xml("<StructWithCorrectNestedNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedDe xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedDe></StructWithCorrectNestedNamespace>").unwrap(),
        StructWithCorrectNestedNamespace {
            test: NestedDe { flag: true }
        }
    );

    // Correct child namespace - without child redefinition
    assert_eq!(
        StructWithCorrectNestedNamespace::from_xml("<StructWithCorrectNestedNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedDe><bar:flag>true</bar:flag></NestedDe></StructWithCorrectNestedNamespace>").unwrap(),
        StructWithCorrectNestedNamespace {
            test: NestedDe { flag: true }
        }
    );

    // Different child namespace
    assert_eq!(
        StructWithWrongNestedNamespace::from_xml("<StructWithWrongNestedNamespace xmlns=\"URI\" xmlns:dar=\"BAZ\"><NestedWrongNamespace xmlns=\"\"><flag>true</flag></NestedWrongNamespace></StructWithWrongNestedNamespace>").unwrap(),
        StructWithWrongNestedNamespace {
            test: NestedWrongNamespace {
                flag: true
            }
        }
    );

    // Wrong child namespace
    assert_eq!(
        StructWithWrongNestedNamespace::from_xml("<StructWithWrongNestedNamespace xmlns=\"URI\" xmlns:dar=\"BAZ\"><NestedWrongNamespace><flag>true</flag></NestedWrongNamespace></StructWithWrongNestedNamespace>").unwrap_err(),
        Error::MissingValue
    );
}

#[derive(Debug, Eq, PartialEq, FromXml)]
#[xml(ns("URI", bar = "BAZ"))]
struct NestedOtherNamespace {
    #[xml(ns(bar))]
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
        NestedOtherNamespace::from_xml(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedOtherNamespace>"
        )
        .unwrap(),
        NestedOtherNamespace { flag: true }
    );

    // Other namespace not-nested - wrong defined namespace
    assert_eq!(
        NestedOtherNamespace::from_xml(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><wrong:flag>true</wrong:flag></NestedOtherNamespace>"
        )
        .unwrap_err(),
        Error::WrongNamespace
    );

    // Other namespace not-nested - wrong parser namespace
    assert_eq!(
        NestedOtherNamespace::from_xml(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"WRONG\"><bar:flag>true</bar:flag></NestedOtherNamespace>"
        )
        .unwrap_err(),
        Error::MissingValue
    );

    // Other namespace not-nested - missing parser prefix
    assert_eq!(
        NestedOtherNamespace::from_xml(
            "<NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAR\"><flag>true</flag></NestedOtherNamespace>"
        )
        .unwrap_err(),
        Error::MissingValue
    );

    // Correct child other namespace
    assert_eq!(
        StructOtherNamespace::from_xml(
            "<StructOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedOtherNamespace></StructOtherNamespace>"
        )
        .unwrap(),
        StructOtherNamespace {
            test: NestedOtherNamespace {
                flag: true,
            }
        }
    );

    // Correct child other namespace - without child redefinition
    assert_eq!(
        StructOtherNamespace::from_xml(
            "<StructOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedOtherNamespace><bar:flag>true</bar:flag></NestedOtherNamespace></StructOtherNamespace>"
        )
        .unwrap(),
        StructOtherNamespace {
            test: NestedOtherNamespace {
                flag: true,
            }
        }
    );

    // Wrong child other namespace - without child redefinition
    assert_eq!(
        StructOtherNamespace::from_xml(
            "<StructOtherNamespace xmlns=\"URI\" xmlns:bar=\"BAZ\"><NestedOtherNamespace><wrong:flag>true</wrong:flag></NestedOtherNamespace></StructOtherNamespace>"
        )
        .unwrap_err(),
        Error::WrongNamespace
    );
}
