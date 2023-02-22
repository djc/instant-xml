use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Wrapper {
    inline: Inline,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(transparent)]
struct Inline {
    foo: Foo,
    bar: Bar,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Foo {
    i: u8,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Bar {
    s: String,
}

#[test]
fn inline() {
    let v = Wrapper {
        inline: Inline {
            foo: Foo { i: 42 },
            bar: Bar {
                s: "hello".to_string(),
            },
        },
    };

    let xml = r#"<Wrapper><Foo><i>42</i></Foo><Bar><s>hello</s></Bar></Wrapper>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}
