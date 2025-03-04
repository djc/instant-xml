use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(ToXml, FromXml, Debug, PartialEq, Eq)]
struct Foo {
    #[xml(attribute)]
    attribute: Option<String>,
    #[xml(direct)]
    direct: Option<String>,
}


#[test]
fn serde_direct_no_value_test() {
    let v = Foo {
        attribute: Some("Attribute text".to_string()),
        direct: None,
    };
    let xml = r#"<Foo attribute="Attribute text"/>"#;

    assert_eq!(xml, to_string(&v).unwrap()); //this fails because the serializer still writes "<Foo attribute=\"Attribute text\"></Foo>"
    assert_eq!(from_str::<Foo>(&xml).unwrap(), v); //this fails because the serializer still writes Some("") to direct
}