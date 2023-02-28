use std::borrow::Cow;

use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, Error, FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Wrapper {
    inline: Inline<'static>,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(transparent)]
struct Inline<'a> {
    foo: Foo,
    bar: Bar<'a>,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Foo {
    i: u8,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Bar<'a> {
    s: Cow<'a, str>,
}

#[test]
fn inline() {
    let v = Wrapper {
        inline: Inline {
            foo: Foo { i: 42 },
            bar: Bar { s: "hello".into() },
        },
    };

    let xml = r#"<Wrapper><Foo><i>42</i></Foo><Bar><s>hello</s></Bar></Wrapper>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());

    assert_eq!(
        from_str::<Wrapper>("<Wrapper><Foo><i>42</i><Bar><s>hello</s></Bar></Foo></Wrapper>")
            .unwrap_err(),
        Error::MissingValue("Inline::bar")
    );
}
