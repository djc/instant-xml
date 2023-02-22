use std::borrow::Cow;

use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, Error, FromXml, ToXml};

#[derive(Debug, PartialEq, Eq, FromXml, ToXml)]
#[xml(ns("URI"))]
struct StructSpecialEntities<'a> {
    string: String,
    str: &'a str,
    #[xml(borrow)]
    cow: Cow<'a, str>,
}

#[test]
fn escape_back() {
    assert_eq!(
        from_str(
            "<StructSpecialEntities xmlns=\"URI\"><string>&lt;&gt;&amp;&quot;&apos;adsad&quot;</string><str>str</str><cow>str&amp;</cow></StructSpecialEntities>"
        ),
        Ok(StructSpecialEntities {
            string: String::from("<>&\"'adsad\""),
            str: "str",
            cow: Cow::Owned("str&".to_string()),
        })
    );

    // Wrong str char
    assert_eq!(
        from_str(
            "<StructSpecialEntities xmlns=\"URI\"><string>&lt;&gt;&amp;&quot;&apos;adsad&quot;</string><str>str&amp;</str></StructSpecialEntities>"
        ),
        Err::<StructSpecialEntities, _>(Error::UnexpectedValue("string with escape characters cannot be deserialized as &str for StructSpecialEntities::str: 'str&amp;'".to_owned()))
    );

    // Borrowed
    let escape_back = from_str::<StructSpecialEntities>(
        "<StructSpecialEntities xmlns=\"URI\"><string>&lt;&gt;&amp;&quot;&apos;adsad&quot;</string><str>str</str><cow>str</cow></StructSpecialEntities>"
    )
    .unwrap();

    if let Cow::Owned(_) = escape_back.cow {
        panic!("Should be Borrowed")
    }

    // Owned
    let escape_back = from_str::<StructSpecialEntities>(
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
        to_string(&StructSpecialEntities{
            string: "&\"<>\'aa".to_string(),
            str: "&\"<>\'bb",
            cow: Cow::from("&\"<>\'cc"),
        }).unwrap(),
        "<StructSpecialEntities xmlns=\"URI\"><string>&amp;&quot;&lt;&gt;&apos;aa</string><str>&amp;&quot;&lt;&gt;&apos;bb</str><cow>&amp;&quot;&lt;&gt;&apos;cc</cow></StructSpecialEntities>",
    );
}
