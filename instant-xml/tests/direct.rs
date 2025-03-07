use std::borrow::Cow;

use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, Error, FromXml, ToXml};

#[derive(Clone, Debug, Eq, FromXml, PartialEq, ToXml)]
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
    assert_eq!(from_str::<Foo>(xml), Ok(v.clone()));

    let xml = "<Foo flag=\"true\"><!--comment-->cbdté</Foo>";
    assert_eq!(from_str::<Foo>(xml), Ok(v.clone()));

    let xml = "<Foo flag=\"true\"><!--comment--><!--comment-->cbdté</Foo>";
    assert_eq!(from_str::<Foo>(xml), Ok(v.clone()));

    let xml = "<!--comment--><Foo flag=\"true\"><!--comment-->cbdté</Foo><!--comment-->";
    assert_eq!(from_str::<Foo>(xml), Ok(v));
}

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

#[derive(Debug, Eq, PartialEq, FromXml)]
struct DirectString {
    s: String,
}

#[test]
fn direct_string() {
    assert_eq!(
        from_str("<DirectString><s>hello</s></DirectString>"),
        Ok(DirectString {
            s: "hello".to_string()
        })
    );
}

#[derive(Debug, Eq, PartialEq, FromXml)]
struct DirectStr<'a> {
    s: Cow<'a, str>,
}

#[test]
fn direct_empty_str() {
    assert_eq!(
        from_str("<DirectStr><s></s></DirectStr>"),
        Ok(DirectStr { s: "".into() })
    );
}

#[test]
fn direct_missing_string() {
    assert_eq!(
        from_str("<DirectString></DirectString>"),
        Err::<DirectString, _>(Error::MissingValue("DirectString::s"))
    );
}

#[derive(Debug, PartialEq, FromXml)]
struct ArtUri {
    #[xml(direct)]
    uri: String,
}

#[derive(Debug, PartialEq, FromXml)]
struct Container {
    art: Option<ArtUri>,
}

#[test]
fn container_empty_string() {
    assert_eq!(
        from_str("<Container><ArtUri></ArtUri></Container>"),
        Ok(Container {
            art: Some(ArtUri {
                uri: "".to_string()
            })
        })
    );
    assert_eq!(
        from_str("<Container><ArtUri/></Container>"),
        Ok(Container {
            art: Some(ArtUri {
                uri: "".to_string()
            })
        })
    );
}

#[derive(ToXml, FromXml, Debug, PartialEq, Eq)]
struct Options {
    #[xml(attribute)]
    attribute: Option<String>,
    #[xml(direct)]
    direct: Option<String>,
}

#[test]
fn direct_options() {
    let v = Options {
        attribute: Some("Attribute text".to_string()),
        direct: None,
    };
    let xml = r#"<Options attribute="Attribute text" />"#;

    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(from_str::<Options>(xml).unwrap(), v);
}
