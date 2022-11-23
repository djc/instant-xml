use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(wrapped)]
enum Foo {
    Bar(Bar),
    Baz(Baz),
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Bar {
    bar: u8,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Baz {
    baz: String,
}

#[test]
fn wrapped_enum() {
    let v = Foo::Bar(Bar { bar: 42 });
    let xml = r#"<Foo><Bar><bar>42</bar></Bar></Foo>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}
