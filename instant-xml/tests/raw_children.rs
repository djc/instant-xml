use instant_xml::de::RawToken;
use instant_xml::{Deserializer, Error, FromXml, Id, Kind, from_str};

/// Collects `<p v=".."/>` children of `<list>` through raw_children().
#[derive(Debug, PartialEq, Default)]
struct List(Vec<String>);

impl<'xml> FromXml<'xml> for List {
    fn matches(id: Id<'_>, _: Option<Id<'_>>) -> bool {
        id == Id {
            ns: "Foo",
            name: "list",
        }
    }
    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        _: &'static str,
        de: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        let mut out = Vec::new();
        for token in de.raw_children() {
            if let RawToken::Attribute {
                local: "v", value, ..
            } = token?
            {
                out.push(value.into_owned());
            }
        }
        *into = Some(Self(out));
        Ok(())
    }
    type Accumulator = Option<Self>;
    const KIND: Kind = Kind::Element;
}

#[derive(Debug, PartialEq, FromXml)]
#[xml(rename = "doc", ns("Foo", BAZ = "BAZ"))]
struct Doc {
    list: List,
    after: String,
    #[xml(ns("BAZ"))]
    tail: Tail,
}

#[derive(Debug, PartialEq, FromXml)]
#[xml(rename = "tail", ns("BAZ"))]
struct Tail {
    #[xml(attribute)]
    n: u32,
}

#[test]
fn raw_children_reads_section_and_restores_state() {
    let xml = r#"<doc xmlns="Foo" xmlns:BAZ="BAZ"><list kind="x"><p v="a"/><p v="b &amp; c"></p><!-- c --><p v="d"><deep/></p></list><after>ok</after><BAZ:tail n="7"/></doc>"#;
    let doc = from_str::<Doc>(xml).unwrap();
    assert_eq!(doc.list, List(vec!["a".into(), "b & c".into(), "d".into()]));
    assert_eq!(doc.after, "ok");
    assert_eq!(doc.tail, Tail { n: 7 });
}

#[test]
fn raw_children_on_empty_element() {
    let xml = r#"<doc xmlns="Foo" xmlns:BAZ="BAZ"><list/><after>ok</after><BAZ:tail n="1"/></doc>"#;
    let doc = from_str::<Doc>(xml).unwrap();
    assert_eq!(doc.list, List(vec![]));
    assert_eq!(doc.after, "ok");
}

#[test]
fn raw_children_reports_truncated_input() {
    let xml = r#"<doc xmlns="Foo"><list><p v="a"/>"#;
    assert!(from_str::<Doc>(xml).is_err());
}
