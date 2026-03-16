use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, PartialEq, Eq, ToXml, FromXml)]
#[xml(ns(CORE, NE = NESTED), rename = "top")]
struct TopLevel {
    #[xml(attribute)]
    attr: u32,
    nested: NestedLevel1ForcePrefix,
}

#[derive(Debug, PartialEq, Eq, ToXml, FromXml)]
#[xml(ns(NESTED), rename = "nestedLevel1", force_prefix)]
struct NestedLevel1ForcePrefix {
    #[xml(attribute)]
    attr: u32,

    nested_collection_1: Vec<NestedLevel2ForcePrefix>,

    nested_collection_2: Vec<NestedLevel2NotForcedPrefix>,
}

#[derive(Debug, PartialEq, Eq, ToXml, FromXml)]
#[xml(ns(NESTED), rename = "nestedLevel2ForcePrefix", force_prefix)]
struct NestedLevel2ForcePrefix {
    #[xml(attribute)]
    attr: u32,
}

#[derive(Debug, PartialEq, Eq, ToXml, FromXml)]
#[xml(ns(NESTED), rename = "nestedLevel2NotForcePrefix")]
struct NestedLevel2NotForcedPrefix {
    #[xml(attribute)]
    attr: u32,
    nested: NestedLevel3NotForcedPrefix,
}

#[derive(Debug, PartialEq, Eq, ToXml, FromXml)]
#[xml(ns(NESTED), rename = "nestedLevel3ForcePrefix", force_prefix)]
struct NestedLevel3NotForcedPrefix {
    #[xml(attribute)]
    attr: u32,
}

const CORE: &str = "CORE";
const NESTED: &str = "NESTED";

#[test]
fn test_toxml_core() {
    let core = TopLevel {
        attr: 0,
        nested: NestedLevel1ForcePrefix {
            attr: 1,
            nested_collection_1: vec![
                NestedLevel2ForcePrefix { attr: 2 },
                NestedLevel2ForcePrefix { attr: 3 },
            ],
            nested_collection_2: vec![NestedLevel2NotForcedPrefix {
                attr: 4,
                nested: NestedLevel3NotForcedPrefix { attr: 5 },
            }],
        },
    };

    let core_string = to_string(&core).unwrap();

    let xml_string = r##"<top xmlns="CORE" xmlns:NE="NESTED" attr="0"><NE:nestedLevel1 attr="1"><NE:nestedLevel2ForcePrefix attr="2" /><NE:nestedLevel2ForcePrefix attr="3" /><nestedLevel2NotForcePrefix xmlns="NESTED" attr="4"><NE:nestedLevel3ForcePrefix attr="5" /></nestedLevel2NotForcePrefix></NE:nestedLevel1></top>"##;

    assert_eq!(xml_string, core_string);
}

#[test]
fn test_fromxml_with_parent_ns_only() {
    let core = TopLevel {
        attr: 0,
        nested: NestedLevel1ForcePrefix {
            attr: 1,
            nested_collection_1: vec![
                NestedLevel2ForcePrefix { attr: 2 },
                NestedLevel2ForcePrefix { attr: 3 },
            ],
            nested_collection_2: vec![NestedLevel2NotForcedPrefix {
                attr: 4,
                nested: NestedLevel3NotForcedPrefix { attr: 5 },
            }],
        },
    };

    let xml_string = r##"<top xmlns="CORE" xmlns:NE="NESTED" attr="0"><NE:nestedLevel1 attr="1"><NE:nestedLevel2ForcePrefix attr="2" /><NE:nestedLevel2ForcePrefix attr="3" /><NE:nestedLevel2NotForcePrefix attr="4"><NE:nestedLevel3ForcePrefix attr="5" /></NE:nestedLevel2NotForcePrefix></NE:nestedLevel1></top>"##;

    let core_from_xml = from_str::<TopLevel>(xml_string).unwrap();

    assert_eq!(core_from_xml, core);
}

#[test]
fn test_fromxml_with_mixed_ns_handling() {
    let core = TopLevel {
        attr: 0,
        nested: NestedLevel1ForcePrefix {
            attr: 1,
            nested_collection_1: vec![
                NestedLevel2ForcePrefix { attr: 2 },
                NestedLevel2ForcePrefix { attr: 3 },
            ],
            nested_collection_2: vec![NestedLevel2NotForcedPrefix {
                attr: 4,
                nested: NestedLevel3NotForcedPrefix { attr: 5 },
            }],
        },
    };

    let xml_string = r##"<top xmlns="CORE" xmlns:NE="NESTED" attr="0"><NE:nestedLevel1 attr="1"><NE:nestedLevel2ForcePrefix attr="2" /><NE:nestedLevel2ForcePrefix attr="3" /><nestedLevel2NotForcePrefix xmlns="NESTED" attr="4"><NE:nestedLevel3ForcePrefix attr="5" /></nestedLevel2NotForcePrefix></NE:nestedLevel1></top>"##;

    let core_from_xml = from_str::<TopLevel>(xml_string).unwrap();

    assert_eq!(core_from_xml, core);
}
