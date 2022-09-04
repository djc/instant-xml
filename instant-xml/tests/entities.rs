use std::borrow::Cow;

use similar_asserts::assert_eq;

use instant_xml::{Error, FromXml, ToXml};

#[derive(Debug, PartialEq, Eq, FromXml, ToXml)]
#[xml(ns("URI"))]
struct StructSpecialEntities<'a> {
    string: String,
    str: &'a str,
    cow: Cow<'a, str>,
}

#[test]
fn escape_back() {
    assert_eq!(
        StructSpecialEntities::from_xml(
            "<StructSpecialEntities xmlns=\"URI\"><string>&lt;&gt;&amp;&quot;&apos;adsad&quot;</string><str>str</str><cow>str&amp;</cow></StructSpecialEntities>"
        )
        .unwrap(),
        StructSpecialEntities {
            string: String::from("<>&\"'adsad\""),
            str: "str",
            cow: Cow::Owned("str&".to_string()),
        }
    );

    // Wrong str char
    assert_eq!(
        StructSpecialEntities::from_xml(
            "<StructSpecialEntities xmlns=\"URI\"><string>&lt;&gt;&amp;&quot;&apos;adsad&quot;</string><str>str&amp;</str></StructSpecialEntities>"
        )
        .unwrap_err(),
        Error::Other("Unsupported char: str&".to_string())
    );

    // Borrowed
    let escape_back = StructSpecialEntities::from_xml(
        "<StructSpecialEntities xmlns=\"URI\"><string>&lt;&gt;&amp;&quot;&apos;adsad&quot;</string><str>str</str><cow>str</cow></StructSpecialEntities>"
    )
    .unwrap();

    if let Cow::Owned(_) = escape_back.cow {
        panic!("Should be Borrowed")
    }

    // Owned
    let escape_back = StructSpecialEntities::from_xml(
            "<StructSpecialEntities xmlns=\"URI\"><string>&lt;&gt;&amp;&quot;&apos;adsad&quot;</string><str>str</str><cow>str&amp;</cow></StructSpecialEntities>"
        )
        .unwrap();

    if let Cow::Borrowed(_) = escape_back.cow {
        panic!("Should be Owned")
    }
}

#[test]
fn special_entities() {
    assert_eq!(
        StructSpecialEntities{
            string: "&\"<>\'aa".to_string(),
            str: "&\"<>\'bb",
            cow: Cow::from("&\"<>\'cc"),
        }
        .to_xml()
        .unwrap(),
        "<StructSpecialEntities xmlns=\"URI\"><string>&amp;&quot;&lt;&gt;&apos;aa</string><str>&amp;&quot;&lt;&gt;&apos;bb</str><cow>&amp;&quot;&lt;&gt;&apos;cc</cow></StructSpecialEntities>"
    );
}