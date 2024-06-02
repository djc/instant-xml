use std::borrow::Cow;

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
