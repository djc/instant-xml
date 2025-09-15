use similar_asserts::assert_eq;

use instant_xml::{from_str, from_str_with_namespaces, Error, FromXml};

use std::collections::BTreeMap;

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

    // Default namespace not-nested - with xml:lang
    assert_eq!(
        from_str("<NestedDe xml:lang=\"en\" xmlns=\"URI\" xmlns:bar=\"BAZ\"><bar:flag>true</bar:flag></NestedDe>"),
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
#[xml(ns("URI", da_sh.ed-ns = "dashed"))]
struct DashedNs {
    #[xml(ns("dashed"))]
    element: String,
}

#[test]
fn dashed_ns() {
    assert_eq!(
        from_str("<DashedNs xmlns=\"URI\" xmlns:da_sh.ed-ns=\"dashed\"><da_sh.ed-ns:element>hello</da_sh.ed-ns:element></DashedNs>"),
        Ok(DashedNs { element: "hello".to_owned() })
    );
}

#[test]
fn namespace_extraction_basic() {
    let xml = r#"<NestedDe xmlns="URI" xmlns:bar="BAZ"><bar:flag>true</bar:flag></NestedDe>"#;

    let (result, namespaces) = from_str_with_namespaces::<NestedDe>(xml).unwrap();

    assert_eq!(result, NestedDe { flag: true });

    let mut expected = BTreeMap::new();
    expected.insert("".to_string(), "URI".to_string()); // default namespace
    expected.insert("bar".to_string(), "BAZ".to_string());
    expected.insert(
        "xml".to_string(),
        "http://www.w3.org/XML/1998/namespace".to_string(),
    );

    assert_eq!(namespaces, expected);
}

#[test]
fn namespace_extraction_with_xml_lang() {
    let xml = r#"<NestedDe xml:lang="en" xmlns="URI" xmlns:bar="BAZ"><bar:flag>true</bar:flag></NestedDe>"#;

    let (result, namespaces) = from_str_with_namespaces::<NestedDe>(xml).unwrap();

    assert_eq!(result, NestedDe { flag: true });

    let mut expected = BTreeMap::new();
    expected.insert("".to_string(), "URI".to_string());
    expected.insert("bar".to_string(), "BAZ".to_string());
    expected.insert(
        "xml".to_string(),
        "http://www.w3.org/XML/1998/namespace".to_string(),
    );

    assert_eq!(namespaces, expected);
}

#[test]
fn namespace_extraction_nested_declarations() {
    let xml = r#"<StructWithCorrectNestedNamespace xmlns="URI" xmlns:bar="BAZ">
        <NestedDe xmlns="URI" xmlns:bar="BAZ">
            <bar:flag>true</bar:flag>
        </NestedDe>
    </StructWithCorrectNestedNamespace>"#;

    let (result, namespaces) =
        from_str_with_namespaces::<StructWithCorrectNestedNamespace>(xml).unwrap();

    assert_eq!(
        result,
        StructWithCorrectNestedNamespace {
            test: NestedDe { flag: true }
        }
    );

    let mut expected = BTreeMap::new();
    expected.insert("".to_string(), "URI".to_string());
    expected.insert("bar".to_string(), "BAZ".to_string());
    expected.insert(
        "xml".to_string(),
        "http://www.w3.org/XML/1998/namespace".to_string(),
    );

    assert_eq!(namespaces, expected);
}

#[test]
fn namespace_extraction_multiple_prefixes() {
    let xml = r#"<Root xmlns="DEFAULT" xmlns:foo="ROOT_PREFIXED_1" xmlns:bar="ROOT_PREFIXED_2">
        <child xmlns:baz="CHILD_PREFIXED_1">content</child>
    </Root>"#;

    #[derive(Debug, Eq, PartialEq, FromXml)]
    #[xml(ns("DEFAULT"))]
    struct Root {
        child: String,
    }

    let (result, namespaces) = from_str_with_namespaces::<Root>(xml).unwrap();

    assert_eq!(
        result,
        Root {
            child: "content".to_string()
        }
    );

    let mut expected = BTreeMap::new();
    expected.insert("".to_string(), "DEFAULT".to_string());
    expected.insert("foo".to_string(), "ROOT_PREFIXED_1".to_string());
    expected.insert("bar".to_string(), "ROOT_PREFIXED_2".to_string());
    expected.insert("baz".to_string(), "CHILD_PREFIXED_1".to_string());
    expected.insert(
        "xml".to_string(),
        "http://www.w3.org/XML/1998/namespace".to_string(),
    );

    assert_eq!(namespaces, expected);
}

#[test]
fn namespace_extraction_dashed_prefixes() {
    let xml = r#"<DashedNs xmlns="URI" xmlns:da_sh.ed-ns="dashed"><da_sh.ed-ns:element>hello</da_sh.ed-ns:element></DashedNs>"#;

    let (result, namespaces) = from_str_with_namespaces::<DashedNs>(xml).unwrap();

    assert_eq!(
        result,
        DashedNs {
            element: "hello".to_owned()
        }
    );

    let mut expected = BTreeMap::new();
    expected.insert("".to_string(), "URI".to_string());
    expected.insert("da_sh.ed-ns".to_string(), "dashed".to_string());
    expected.insert(
        "xml".to_string(),
        "http://www.w3.org/XML/1998/namespace".to_string(),
    );

    assert_eq!(namespaces, expected);
}

#[test]
fn namespace_extraction_no_namespaces() {
    let xml = r#"<Root><child>content</child></Root>"#;

    #[derive(Debug, Eq, PartialEq, FromXml)]
    struct Root {
        child: String,
    }

    let (result, namespaces) = from_str_with_namespaces::<Root>(xml).unwrap();

    assert_eq!(
        result,
        Root {
            child: "content".to_string()
        }
    );

    let mut expected = BTreeMap::new();
    expected.insert(
        "xml".to_string(),
        "http://www.w3.org/XML/1998/namespace".to_string(),
    );

    assert_eq!(namespaces, expected);
}

#[test]
fn namespace_extraction_only_default_namespace() {
    let xml = r#"<Root xmlns="DEFAULT"><child>content</child></Root>"#;

    #[derive(Debug, Eq, PartialEq, FromXml)]
    #[xml(ns("DEFAULT"))]
    struct Root {
        child: String,
    }

    let (result, namespaces) = from_str_with_namespaces::<Root>(xml).unwrap();

    assert_eq!(
        result,
        Root {
            child: "content".to_string()
        }
    );

    let mut expected = BTreeMap::new();
    expected.insert("".to_string(), "DEFAULT".to_string());
    expected.insert(
        "xml".to_string(),
        "http://www.w3.org/XML/1998/namespace".to_string(),
    );

    assert_eq!(namespaces, expected);
}

//still need to figure out why this is not working
// on first look: It seems like the deserializer cannot deal with the case where the child element is a namespaced element?
#[test]
fn namespace_extraction_only_prefixed_namespaces() {
    let xml = r#"<Root xmlns:foo="ROOT_PREFIXED_1" xmlns:bar="ROOT_PREFIXED_2"><foo:child>content</foo:child></Root>"#;

    #[derive(Debug, Eq, PartialEq, FromXml)]
    struct Root {
        child: String,
    }

    let (result, namespaces) = from_str_with_namespaces::<Root>(xml).unwrap();

    assert_eq!(
        result,
        Root {
            child: "content".to_string()
        }
    );

    let mut expected = BTreeMap::new();
    expected.insert("foo".to_string(), "ROOT_PREFIXED_1".to_string());
    expected.insert("bar".to_string(), "ROOT_PREFIXED_2".to_string());
    expected.insert(
        "xml".to_string(),
        "http://www.w3.org/XML/1998/namespace".to_string(),
    );

    assert_eq!(namespaces, expected);
}

#[test]
fn namespace_extraction_empty_default_namespace() {
    let xml = r#"<Root xmlns="" xmlns:foo="ROOT_PREFIXED_1"><child>content</child></Root>"#;

    #[derive(Debug, Eq, PartialEq, FromXml)]
    struct Root {
        child: String,
    }

    let (result, namespaces) = from_str_with_namespaces::<Root>(xml).unwrap();

    assert_eq!(
        result,
        Root {
            child: "content".to_string()
        }
    );

    let mut expected = BTreeMap::new();
    expected.insert("".to_string(), "".to_string()); // empty default namespace
    expected.insert("foo".to_string(), "ROOT_PREFIXED_1".to_string());
    expected.insert(
        "xml".to_string(),
        "http://www.w3.org/XML/1998/namespace".to_string(),
    );

    assert_eq!(namespaces, expected);
}

#[test]
fn namespace_extraction_error_cases() {
    let xml =
        r#"<WrongElement xmlns="URI" xmlns:bar="BAZ"><bar:flag>true</bar:flag></WrongElement>"#;

    let result = from_str_with_namespaces::<NestedDe>(xml);

    assert!(result.is_err());

    let xml_correct =
        r#"<NestedDe xmlns="URI" xmlns:bar="BAZ"><bar:flag>true</bar:flag></NestedDe>"#;
    let (result, namespaces) = from_str_with_namespaces::<NestedDe>(xml_correct).unwrap();

    assert_eq!(result, NestedDe { flag: true });

    let mut expected = BTreeMap::new();
    expected.insert("".to_string(), "URI".to_string());
    expected.insert("bar".to_string(), "BAZ".to_string());
    expected.insert(
        "xml".to_string(),
        "http://www.w3.org/XML/1998/namespace".to_string(),
    );

    assert_eq!(namespaces, expected);
}

//still need to figure out why this is not working yet.
//is this the same issue as above?
#[test]
fn namespace_extraction_complex_nested() {
    let xml = r#"<Root xmlns="http://root.com" xmlns:level1="http://level1.com">
        <level1:Child xmlns:level2="http://level2.com">
            <level2:Grandchild xmlns:level3="http://level3.com">
                <level3:content>text</level3:content>
            </level2:Grandchild>
        </level1:Child>
    </Root>"#;

    #[derive(Debug, Eq, PartialEq, FromXml)]
    #[xml(ns("http://root.com", level1 = "http://level1.com"))]
    struct Root {
        child: Child,
    }

    #[derive(Debug, Eq, PartialEq, FromXml)]
    #[xml(ns("http://level1.com", level2 = "http://level2.com"))]
    struct Child {
        grandchild: Grandchild,
    }

    #[derive(Debug, Eq, PartialEq, FromXml)]
    #[xml(ns("http://level2.com", level3 = "http://level3.com"))]
    struct Grandchild {
        content: String,
    }

    let (result, namespaces) = from_str_with_namespaces::<Root>(xml).unwrap();

    assert_eq!(
        result,
        Root {
            child: Child {
                grandchild: Grandchild {
                    content: "text".to_string()
                }
            }
        }
    );

    let mut expected = BTreeMap::new();
    expected.insert("".to_string(), "http://root.com".to_string());
    expected.insert("level1".to_string(), "http://level1.com".to_string());
    expected.insert("level2".to_string(), "http://level2.com".to_string());
    expected.insert("level3".to_string(), "http://level3.com".to_string());
    expected.insert(
        "xml".to_string(),
        "http://www.w3.org/XML/1998/namespace".to_string(),
    );

    assert_eq!(namespaces, expected);
}
