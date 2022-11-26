use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Foo {
    bar: usize,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Bar {
    foo: Vec<Foo>,
    baz: Vec<String>,
}

#[test]
fn vec() {
    let val = Bar {
        foo: vec![],
        baz: vec![],
    };
    let xml = "<Bar></Bar>";
    assert_eq!(xml, to_string(&val).unwrap());
    assert_eq!(val, from_str(xml).unwrap());

    let val = Bar {
        foo: vec![Foo { bar: 42 }],
        baz: vec!["hello".to_owned()],
    };
    let xml = "<Bar><Foo><bar>42</bar></Foo><baz>hello</baz></Bar>";
    assert_eq!(xml, to_string(&val).unwrap());
    assert_eq!(val, from_str(xml).unwrap());

    let val = Bar {
        foo: vec![Foo { bar: 42 }, Foo { bar: 73 }],
        baz: vec!["hello".to_owned(), "world".to_owned()],
    };
    let xml = "<Bar><Foo><bar>42</bar></Foo><Foo><bar>73</bar></Foo><baz>hello</baz><baz>world</baz></Bar>";
    assert_eq!(xml, to_string(&val).unwrap());
    assert_eq!(val, from_str(xml).unwrap());
}
