use std::borrow::Cow;
use std::fmt;

use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, Error, FromXml, Id, Serializer, ToXml};

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
    let escape_back = from_str::<StructSpecialEntities<'_>>(
        "<StructSpecialEntities xmlns=\"URI\"><string>&lt;&gt;&amp;&quot;&apos;adsad&quot;</string><cow>str</cow></StructSpecialEntities>"
    )
    .unwrap();

    if let Cow::Owned(_) = escape_back.cow {
        panic!("Should be Borrowed")
    }

    // Owned
    let escape_back = from_str::<StructSpecialEntities<'_>>(
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
        from_str::<SimpleCData<'_>>("<SimpleCData><foo><![CDATA[<fo&amp;o>]]></foo></SimpleCData>")
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

#[derive(Debug, PartialEq, Eq, FromXml, ToXml)]
struct Whitespace {
    #[xml(attribute)]
    attr: String,
    text: String,
}

#[test]
fn whitespace() {
    let value = Whitespace {
        attr: "a\r\nb\rc\td".to_string(),
        text: "a\r\nb\rc\td".to_string(),
    };

    let xml = to_string(&value).unwrap();
    assert_eq!(
        xml,
        "<Whitespace attr=\"a&#xD;&#xA;b&#xD;c&#x9;d\"><text>a&#xD;\nb&#xD;c\td</text></Whitespace>",
    );
    assert_eq!(from_str::<Whitespace>(&xml).unwrap(), value);
}

#[derive(Debug, PartialEq, Eq, ToXml)]
struct WhitespaceList {
    #[xml(attribute)]
    list: List,
}

#[derive(Debug, PartialEq, Eq)]
struct List(Vec<String>);

impl ToXml for List {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        _: Option<Id<'_>>,
        serializer: &mut Serializer<'_, W>,
    ) -> Result<(), Error> {
        for (i, item) in self.0.iter().enumerate() {
            if i > 0 {
                serializer.write_str(" ")?;
            }
            item.serialize(None, serializer)?;
        }
        Ok(())
    }
}

#[test]
fn whitespace_list() {
    let value = WhitespaceList {
        list: List(vec!["a\nb".to_string(), "c\td".to_string()]),
    };

    assert_eq!(
        to_string(&value).unwrap(),
        "<WhitespaceList list=\"a&#xA;b c&#x9;d\" />",
    );
}
