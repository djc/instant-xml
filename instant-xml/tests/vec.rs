use std::borrow::Cow;

use similar_asserts::assert_eq;

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

#[derive(Debug, Eq, FromXml, PartialEq)]
struct Borrowed<'a> {
    #[xml(borrow)]
    baz: Vec<Cow<'a, str>>,
}

#[test]
fn vec_borrow() {
    let de =
        from_str::<Borrowed<'_>>("<Borrowed><baz>a</baz><baz>a&amp;b</baz></Borrowed>").unwrap();
    assert!(matches!(
        de.baz.as_slice(),
        [Cow::Borrowed("a"), Cow::Owned(s)] if s == "a&b"
    ));

    let de = from_str::<Borrowed<'_>>("<Borrowed />").unwrap();
    assert!(de.baz.is_empty());
}

#[derive(Debug, Eq, FromXml, PartialEq)]
struct Owned {
    baz: Vec<String>,
}

#[test]
fn vec_borrow_empty_element() {
    let xml = "<Borrowed><baz>a</baz><baz/></Borrowed>";
    let de = from_str::<Borrowed<'_>>(xml).unwrap();
    assert!(matches!(
        de.baz.as_slice(),
        [Cow::Borrowed("a"), Cow::Borrowed("")]
    ));

    // Same result as `Vec<String>`
    let owned = from_str::<Owned>("<Owned><baz>a</baz><baz/></Owned>").unwrap();
    assert_eq!(de.baz, owned.baz);
    assert_eq!(owned.baz, ["a", ""]);
}
