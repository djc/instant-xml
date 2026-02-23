use similar_asserts::assert_eq;

use instant_xml::{from_str, AnyElement, FromXml};

#[test]
fn standalone_element() {
    let xml = r#"<item xmlns="http://example.com">hello</item>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.name, "item");
    assert_eq!(elem.ns, "http://example.com");
    assert_eq!(elem.text.as_deref(), Some("hello"));
    assert!(elem.attributes.is_empty());
    assert!(elem.children.is_empty());
}

#[test]
fn element_with_attributes() {
    let xml = r#"<item xmlns="http://example.com" key="val" count="3">text</item>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.name, "item");
    assert_eq!(elem.attributes.len(), 2);
    assert_eq!(elem.attributes[0].ns, "");
    assert_eq!(elem.attributes[0].name, "key");
    assert_eq!(elem.attributes[0].value, "val");
    assert_eq!(elem.attributes[1].ns, "");
    assert_eq!(elem.attributes[1].name, "count");
    assert_eq!(elem.attributes[1].value, "3");
    assert_eq!(elem.text.as_deref(), Some("text"));
}

#[test]
fn nested_children() {
    let xml = r#"<parent xmlns="http://example.com"><child1>a</child1><child2>b</child2></parent>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.name, "parent");
    assert_eq!(elem.children.len(), 2);

    assert_eq!(elem.children[0].name, "child1");
    assert_eq!(elem.children[0].ns, "http://example.com");
    assert_eq!(elem.children[0].text.as_deref(), Some("a"));

    assert_eq!(elem.children[1].name, "child2");
    assert_eq!(elem.children[1].ns, "http://example.com");
    assert_eq!(elem.children[1].text.as_deref(), Some("b"));
}

#[test]
fn deeply_nested() {
    let xml = r#"<root xmlns="http://example.com"><level1><level2><level3>deep</level3></level2></level1></root>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.name, "root");
    assert_eq!(elem.children.len(), 1);
    assert_eq!(elem.children[0].name, "level1");
    assert_eq!(elem.children[0].children[0].name, "level2");
    assert_eq!(elem.children[0].children[0].children[0].name, "level3");
    assert_eq!(
        elem.children[0].children[0].children[0].text.as_deref(),
        Some("deep")
    );
}

#[test]
fn self_closing_element() {
    let xml = r#"<item xmlns="http://example.com" status="ok" />"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.name, "item");
    assert_eq!(elem.attributes[0].ns, "");
    assert_eq!(elem.attributes[0].name, "status");
    assert_eq!(elem.attributes[0].value, "ok");
    assert!(elem.text.is_none());
    assert!(elem.children.is_empty());
}

#[test]
fn prefixed_namespace() {
    let xml = r#"<ns:item xmlns:ns="http://other.com">text</ns:item>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.name, "item");
    assert_eq!(elem.ns, "http://other.com");
    assert_eq!(elem.text.as_deref(), Some("text"));
}

#[test]
fn prefixed_attribute() {
    let xml = r#"<item xmlns="http://example.com" xmlns:s="http://schema.com" s:type="string">val</item>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.name, "item");
    assert_eq!(elem.attributes.len(), 1);
    assert_eq!(elem.attributes[0].ns, "http://schema.com");
    assert_eq!(elem.attributes[0].name, "type");
    assert_eq!(elem.attributes[0].value, "string");
}

#[test]
fn child_with_different_namespace() {
    let xml = r#"<root xmlns="http://example.com" xmlns:other="http://other.com"><other:child>text</other:child></root>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.name, "root");
    assert_eq!(elem.ns, "http://example.com");
    assert_eq!(elem.children.len(), 1);
    assert_eq!(elem.children[0].name, "child");
    assert_eq!(elem.children[0].ns, "http://other.com");
    assert_eq!(elem.children[0].text.as_deref(), Some("text"));
}

#[test]
fn empty_text() {
    let xml = r#"<item xmlns="http://example.com"></item>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.name, "item");
    assert!(elem.text.is_none());
}

#[derive(Debug, FromXml, PartialEq)]
#[xml(ns("http://example.com"))]
struct Wrapper<'a> {
    #[xml(borrow)]
    inner: AnyElement<'a>,
}

#[test]
fn inside_derived_struct() {
    let xml =
        r#"<Wrapper xmlns="http://example.com"><anything key="val">hello</anything></Wrapper>"#;
    let parsed = from_str::<Wrapper<'_>>(xml).unwrap();

    assert_eq!(parsed.inner.name, "anything");
    assert_eq!(parsed.inner.ns, "http://example.com");
    assert_eq!(parsed.inner.text.as_deref(), Some("hello"));
    assert_eq!(parsed.inner.attributes[0].ns, "");
    assert_eq!(parsed.inner.attributes[0].name, "key");
    assert_eq!(parsed.inner.attributes[0].value, "val");
}

#[derive(Debug, FromXml, PartialEq)]
#[xml(ns("http://example.com"))]
struct MultiWrapper<'a> {
    #[xml(borrow)]
    items: Vec<AnyElement<'a>>,
}

#[test]
fn vec_of_elements() {
    let xml = r#"<MultiWrapper xmlns="http://example.com"><a>1</a><b>2</b><c>3</c></MultiWrapper>"#;
    let parsed = from_str::<MultiWrapper<'_>>(xml).unwrap();

    assert_eq!(parsed.items.len(), 3);
    assert_eq!(parsed.items[0].name, "a");
    assert_eq!(parsed.items[0].text.as_deref(), Some("1"));
    assert_eq!(parsed.items[1].name, "b");
    assert_eq!(parsed.items[1].text.as_deref(), Some("2"));
    assert_eq!(parsed.items[2].name, "c");
    assert_eq!(parsed.items[2].text.as_deref(), Some("3"));
}

#[test]
fn mixed_children_and_attributes() {
    let xml = r#"<root xmlns="http://example.com" id="42"><child status="active">data</child><other /></root>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.name, "root");
    assert_eq!(elem.attributes[0].ns, "");
    assert_eq!(elem.attributes[0].name, "id");
    assert_eq!(elem.attributes[0].value, "42");
    assert_eq!(elem.children.len(), 2);

    assert_eq!(elem.children[0].name, "child");
    assert_eq!(elem.children[0].attributes[0].ns, "");
    assert_eq!(elem.children[0].attributes[0].name, "status");
    assert_eq!(elem.children[0].attributes[0].value, "active");
    assert_eq!(elem.children[0].text.as_deref(), Some("data"));

    assert_eq!(elem.children[1].name, "other");
    assert!(elem.children[1].text.is_none());
    assert!(elem.children[1].children.is_empty());
}

/// Namespace declarations (`xmlns:prefix="uri"`) are consumed by the parser
/// to resolve prefixes into the `ns` field. Children correctly inherit or
/// resolve their namespace without needing a separate `namespaces` store.
#[test]
fn namespace_resolution_on_children() {
    let xml = r#"<root xmlns="http://example.com" xmlns:x="http://x.com" xmlns:y="http://y.com"><x:a>1</x:a><y:b>2</y:b><c>3</c></root>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.ns, "http://example.com");
    assert_eq!(elem.children[0].name, "a");
    assert_eq!(elem.children[0].ns, "http://x.com");
    assert_eq!(elem.children[1].name, "b");
    assert_eq!(elem.children[1].ns, "http://y.com");
    // Unprefixed child inherits the default namespace
    assert_eq!(elem.children[2].name, "c");
    assert_eq!(elem.children[2].ns, "http://example.com");
}

#[test]
fn no_namespace() {
    let xml = r#"<item>hello</item>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.name, "item");
    assert_eq!(elem.ns, "");
    assert_eq!(elem.text.as_deref(), Some("hello"));
}

/// `into_owned()` decouples the element from the XML input lifetime,
/// similar to serde's `DeserializeOwned` pattern.
#[test]
fn into_owned() {
    fn parse_and_own(xml: &str) -> AnyElement<'static> {
        from_str::<AnyElement<'_>>(xml).unwrap().into_owned()
    }

    let elem = parse_and_own(
        r#"<root xmlns="http://example.com" xmlns:s="http://schema.com" id="1" s:type="str"><child>text</child></root>"#,
    );

    assert_eq!(elem.name, "root");
    assert_eq!(elem.ns, "http://example.com");
    assert_eq!(elem.text, None);

    assert_eq!(elem.attributes.len(), 2);
    assert_eq!(elem.attributes[0].ns, "");
    assert_eq!(elem.attributes[0].name, "id");
    assert_eq!(elem.attributes[0].value, "1");
    assert_eq!(elem.attributes[1].ns, "http://schema.com");
    assert_eq!(elem.attributes[1].name, "type");
    assert_eq!(elem.attributes[1].value, "str");

    assert_eq!(elem.children.len(), 1);
    assert_eq!(elem.children[0].name, "child");
    assert_eq!(elem.children[0].text.as_deref(), Some("text"));
}

#[test]
fn getters() {
    let xml = r#"<item xmlns="http://example.com">hello</item>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.name, "item");
    assert_eq!(elem.ns, "http://example.com");
    assert_eq!(elem.text.as_deref(), Some("hello"));

    let xml = r#"<item>hello</item>"#;
    let elem = from_str::<AnyElement<'_>>(xml).unwrap();

    assert_eq!(elem.ns, "");
}
