use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use similar_asserts::assert_eq;

use instant_xml::{from_str, FromXml, ToXml};

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, FromXml, ToXml)]
#[xml(ns("URI"))]
struct NestedLifetimes<'a> {
    flag: bool,
    str_type_a: Cow<'a, str>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, FromXml, ToXml)]
#[xml(ns("URI"))]
struct StructDeserializerScalars<'a, 'b> {
    bool_type: bool,
    i8_type: i8,
    u32_type: u32,
    string_type: String,
    str_type_a: Cow<'a, str>,
    str_type_b: Cow<'b, str>,
    char_type: char,
    f32_type: f32,
    nested: NestedLifetimes<'a>,
    cow: Cow<'a, str>,
    option: Option<Cow<'a, str>>,
    slice: Cow<'a, [u8]>,
}

#[test]
fn scalars() {
    assert_eq!(
        from_str(
            "<StructDeserializerScalars xmlns=\"URI\"><bool_type>true</bool_type><i8_type>1</i8_type><u32_type>42</u32_type><string_type>string</string_type><str_type_a>lifetime a</str_type_a><str_type_b>lifetime b</str_type_b><char_type>c</char_type><f32_type>1.20</f32_type><NestedLifetimes><flag>true</flag><str_type_a>asd</str_type_a></NestedLifetimes><cow>123</cow><slice>1</slice><slice>2</slice><slice>3</slice></StructDeserializerScalars>"
        ),
        Ok(StructDeserializerScalars{
            bool_type: true,
            i8_type: 1,
            u32_type: 42,
            string_type: "string".to_string(),
            str_type_a: "lifetime a".into(),
            str_type_b: "lifetime b".into(),
            char_type: 'c',
            f32_type: 1.20,
            nested: NestedLifetimes {
                flag: true,
                str_type_a: "asd".into()
            },
            cow: Cow::from("123"),
            option: None,
            slice: Cow::Borrowed(&[1, 2, 3]),
        })
    );

    // Option none
    assert_eq!(
        from_str(
            "<StructDeserializerScalars xmlns=\"URI\"><bool_type>true</bool_type><i8_type>1</i8_type><u32_type>42</u32_type><string_type>string</string_type><str_type_a>lifetime a</str_type_a><str_type_b>lifetime b</str_type_b><char_type>c</char_type><f32_type>1.2</f32_type><NestedLifetimes><flag>true</flag><str_type_a>asd</str_type_a></NestedLifetimes><cow>123</cow><option>asd</option><slice>1</slice><slice>2</slice><slice>3</slice></StructDeserializerScalars>"
        ),
        Ok(StructDeserializerScalars{
            bool_type: true,
            i8_type: 1,
            u32_type: 42,
            string_type: "string".to_string(),
            str_type_a: "lifetime a".into(),
            str_type_b: "lifetime b".into(),
            char_type: 'c',
            f32_type: 1.20,
            nested: NestedLifetimes {
                flag: true,
                str_type_a: "asd".into(),
            },
            cow: Cow::from("123"),
            option: Some("asd".into()),
            slice: Cow::Borrowed(&[1, 2, 3]),
        })
    );
}

#[derive(Debug, FromXml, PartialEq)]
struct ScalarElementAttr {
    s: String,
}

#[test]
fn scalar_element_attr() {
    assert_eq!(
        from_str::<ScalarElementAttr>(
            "<ScalarElementAttr><s lang=\"en\">hello</s></ScalarElementAttr>"
        )
        .unwrap(),
        ScalarElementAttr {
            s: "hello".to_string(),
        }
    );
}
