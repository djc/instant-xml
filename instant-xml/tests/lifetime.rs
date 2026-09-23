use std::borrow::Cow;

use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Foo {
    bar: Bar<'static>,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Bar<'a> {
    baz: Cow<'a, str>,
}

#[test]
fn lifetime() {
    let v = Foo {
        bar: Bar {
            baz: Cow::Borrowed("hello"),
        },
    };
    let xml = r#"<Foo><Bar><baz>hello</baz></Bar></Foo>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}

#[derive(Debug, Eq, FromXml, PartialEq)]
struct Item<'a> {
    #[xml(borrow)]
    name: Cow<'a, str>,
}

#[derive(Debug, Eq, FromXml, PartialEq)]
struct Wrapper<'a> {
    // `borrow` adds the `'xml: 'a` bound that `Item<'a>` requires
    #[xml(borrow)]
    items: Vec<Item<'a>>,
}

#[test]
fn borrow_nested() {
    let xml = "<Wrapper><Item><name>a</name></Item><Item><name>b</name></Item></Wrapper>";
    let de = from_str::<Wrapper<'_>>(xml).unwrap();
    assert!(matches!(
        de.items.as_slice(),
        [
            Item {
                name: Cow::Borrowed("a")
            },
            Item {
                name: Cow::Borrowed("b")
            },
        ]
    ));
}
