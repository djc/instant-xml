use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, FromXml, ToXml, PartialEq)]
struct Foo<T> {
    inner: T,
}

#[derive(Debug, Eq, FromXml, ToXml, PartialEq)]
struct Bar {
    bar: String,
}

#[allow(clippy::disallowed_names)]
#[test]
fn serialize_generics() {
    let foo = Foo {
        inner: Bar {
            bar: "Bar".to_owned(),
        },
    };

    let xml = "<Foo><Bar><bar>Bar</bar></Bar></Foo>";

    assert_eq!(to_string(&foo).unwrap(), xml);
    assert_eq!(from_str::<Foo<Bar>>(xml).unwrap(), foo);
}
