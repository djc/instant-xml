use std::borrow::Cow;

use instant_xml::{from_str, to_string, FromXml, ToXml};
use similar_asserts::assert_eq;

#[derive(Debug, PartialEq, Eq, FromXml, ToXml)]
#[xml(ns("URI"))]
struct StructSpecialEntities<'a> {
    string: String,
    #[xml(borrow)]
    cow: Cow<'a, str>,
}

#[test]
fn escape_back() {
    assert_eq!(
        from_str(
            "<StructSpecialEntities xmlns=\"URI\"><string>&lt;&gt;&amp;&quot;&apos;adsad&quot;</string><cow>str&amp;</cow></StructSpecialEntities>"
        ),
        Ok(StructSpecialEntities {
            string: String::from("<>&\"'adsad\""),
            cow: Cow::Owned("str&".to_string()),
        })
    );

    // Borrowed
    let escape_back = from_str::<StructSpecialEntities>(
        "<StructSpecialEntities xmlns=\"URI\"><string>&lt;&gt;&amp;&quot;&apos;adsad&quot;</string><cow>str</cow></StructSpecialEntities>"
    )
    .unwrap();

    if let Cow::Owned(_) = escape_back.cow {
        panic!("Should be Borrowed")
    }

    // Owned
    let escape_back = from_str::<StructSpecialEntities>(
            "<StructSpecialEntities xmlns=\"URI\"><string>&lt;&gt;&amp;&quot;&apos;adsad&quot;</string><cow>str&amp;</cow></StructSpecialEntities>"
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
            cow: Cow::from("&\"<>\'cc"),
        }).unwrap(),
        "<StructSpecialEntities xmlns=\"URI\"><string>&amp;&quot;&lt;&gt;&apos;aa</string><cow>&amp;&quot;&lt;&gt;&apos;cc</cow></StructSpecialEntities>",
    );
}

#[derive(Debug, PartialEq, Eq, FromXml, ToXml)]
struct SimpleCData<'a> {
    #[xml(borrow)]
    foo: Cow<'a, str>,
}

#[test]
fn simple_cdata() {
    assert_eq!(
        from_str::<SimpleCData>("<SimpleCData><foo><![CDATA[<fo&amp;o>]]></foo></SimpleCData>")
            .unwrap(),
        SimpleCData {
            foo: Cow::Borrowed("<fo&amp;o>")
        }
    );

    assert_eq!(
        to_string(&SimpleCData {
            foo: Cow::Borrowed("<foo>")
        })
        .unwrap(),
        "<SimpleCData><foo>&lt;foo&gt;</foo></SimpleCData>",
    );
}
