use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, PartialEq, Eq, ToXml, FromXml)]
#[xml(ns(CORE, NE = NESTED), rename = "top")]
struct TopLevel {
    #[xml(attribute)]
    attr: u32,
    nested: NestedLevel1ForcePrefix,
}

#[derive(Debug, PartialEq, Eq, FromXml)]
#[xml(ns(NESTED), rename = "nestedLevel1")]
struct NestedLevel1ForcePrefix {
    #[xml(attribute)]
    attr: u32,

    nested_collection_1: Vec<NestedLevel2ForcePrefix>,

    nested_collection_2: Vec<NestedLevel2NotForcedPrefix>,
}

impl ToXml for NestedLevel1ForcePrefix {
    fn serialize<W: std::fmt::Write + ?Sized>(
        &self,
        field: Option<instant_xml::Id<'_>>,
        serializer: &mut instant_xml::Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        let (element, name) = match field {
            Some(id) => {
                let prefix = serializer.write_start(
                    "nestedLevel1",
                    NESTED,
                    Some(instant_xml::ser::Context {
                        default_ns: CORE,
                        prefixes: [instant_xml::ser::Prefix {
                            prefix: "NE",
                            ns: NESTED,
                        }],
                    }),
                    true,
                )?;
                serializer.write_attr("attr", CORE, &self.attr)?;

                serializer.end_start()?;
                (prefix, "nestedLevel1")
            }
            None => {
                let element = serializer.write_start(
                    "nestedLevel1",
                    NESTED,
                    Some(instant_xml::ser::Context {
                        default_ns: NESTED,
                        prefixes: [instant_xml::ser::Prefix {
                            prefix: "NE",
                            ns: NESTED,
                        }],
                    }),
                    true,
                )?;

                serializer.write_attr("attr", NESTED, &self.attr)?;

                serializer.end_start()?;
                (element, "nestedLevel1")
            }
        };

        for nested1 in &self.nested_collection_1 {
            nested1.serialize(field, serializer)?;
        }

        for nested2 in &self.nested_collection_2 {
            nested2.serialize(field, serializer)?;
        }

        serializer.write_close(element)?;

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, FromXml)]
#[xml(ns(NESTED), rename = "nestedLevel2ForcePrefix")]
struct NestedLevel2ForcePrefix {
    #[xml(attribute)]
    attr: u32,
}

impl ToXml for NestedLevel2ForcePrefix {
    fn serialize<W: std::fmt::Write + ?Sized>(
        &self,
        field: Option<instant_xml::Id<'_>>,
        serializer: &mut instant_xml::Serializer<W>,
    ) -> Result<(), instant_xml::Error> {
        let (element, name) = match field {
            Some(id) => {
                let prefix = serializer.write_start(
                    "nestedLevel2ForcePrefix",
                    NESTED,
                    Some(instant_xml::ser::Context {
                        default_ns: CORE,
                        prefixes: [instant_xml::ser::Prefix {
                            prefix: "NE",
                            ns: NESTED,
                        }],
                    }),
                    true,
                )?;
                serializer.write_attr("attr", CORE, &self.attr)?;

                serializer.end_start()?;
                (prefix, "nestedLevel2ForcePrefix")
            }
            None => {
                let element = serializer.write_start(
                    "nestedLevel2ForcePrefix",
                    NESTED,
                    Some(instant_xml::ser::Context {
                        default_ns: NESTED,
                        prefixes: [instant_xml::ser::Prefix {
                            prefix: "NE",
                            ns: NESTED,
                        }],
                    }),
                    true,
                )?;

                serializer.write_attr("attr", NESTED, &self.attr)?;

                serializer.end_start()?;
                (element, "nestedLevel2ForcePrefix")
            }
        };

        serializer.write_close(element)?;

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, ToXml, FromXml)]
#[xml(ns(NESTED), rename = "nestedLevel2NotForcePrefix")]
struct NestedLevel2NotForcedPrefix {
    #[xml(attribute)]
    attr: u32,
    nested: NestedLevel3NotForcedPrefix,
}

#[derive(Debug, PartialEq, Eq, ToXml, FromXml)]
#[xml(ns(NESTED), rename = "nestedLevel3NotForcePrefix")]
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

    let xml_string = r##"<top xmlns="CORE" xmlns:NE="NESTED" attr="0"><NE:nestedLevel1 attr="1"><NE:nestedLevel2ForcePrefix attr="2"></NE:nestedLevel2ForcePrefix><NE:nestedLevel2ForcePrefix attr="3"></NE:nestedLevel2ForcePrefix><NE:nestedLevel2NotForcePrefix xmlns="NESTED" attr="4"><nestedLevel3NotForcePrefix attr="5" /></NE:nestedLevel2NotForcePrefix></NE:nestedLevel1></top>"##;

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

    let xml_string = r##"<top xmlns="CORE" xmlns:NE="NESTED" attr="0"><NE:nestedLevel1 attr="1"><NE:nestedLevel2ForcePrefix attr="2" /><NE:nestedLevel2ForcePrefix attr="3" /><NE:nestedLevel2NotForcePrefix attr="4"><NE:nestedLevel3NotForcePrefix attr="5" /></NE:nestedLevel2NotForcePrefix></NE:nestedLevel1></top>"##;

    let core_from_xml = from_str::<TopLevel>(xml_string).unwrap();

    assert_eq!(core_from_xml, core);
}

#[test]
fn test_fromxml_with_redefined_ns() {
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

    let xml_string = r##"<top xmlns="CORE" xmlns:NE="NESTED" attr="0"><NE:nestedLevel1 xmlns="NESTED" attr="1"><nestedLevel2ForcePrefix attr="2" /><nestedLevel2ForcePrefix attr="3" /><nestedLevel2NotForcePrefix xmlns="NESTED" attr="4"><nestedLevel3NotForcePrefix attr="5" /></nestedLevel2NotForcePrefix></NE:nestedLevel1></top>"##;

    let core_from_xml = from_str::<TopLevel>(xml_string).unwrap();

    assert_eq!(core_from_xml, core);
}
