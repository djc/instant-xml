use instant_xml::{FromXml, ToXml};

#[derive(Debug, Eq, PartialEq, ToXml)]
struct Unit;

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithCustomField {
    test: Nested,
}

#[derive(Debug, Eq, PartialEq, ToXml, FromXml, Clone)]
struct Nested {
    #[xml(namespace(bar))]
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithCustomFieldWrongPrefix {
    test: NestedWrongPrefix,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
struct NestedWrongPrefix {
    #[xml(namespace(dar))]
    flag: bool,
}

#[derive(Debug, Eq, PartialEq, ToXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithNamedFields {
    flag: bool,
    #[xml(namespace(bar))]
    string: String,
    #[xml(namespace("typo"))]
    number: i32,
}

#[derive(Debug, Eq, PartialEq, FromXml, ToXml)]
#[xml(namespace("URI", bar = "BAZ", foo = "BAR"))]
struct StructWithCustomFieldFromXml {
    flag: bool,
    test: Nested,
}

#[test]
fn unit() {
    assert_eq!(Unit.to_xml(None).unwrap(), "<Unit></Unit>");
    //assert_eq!(Unit::from_xml("<Unit/>").unwrap(), Unit);
}

#[test]
fn struct_with_named_fields() {
    assert_eq!(
        StructWithNamedFields {
            flag: true,
            string: "test".to_string(),
            number: 1,
        }
        .to_xml(None)
        .unwrap(),
        "<StructWithNamedFields xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><flag>true</flag><bar:string>test</bar:string><number xmlns=\"typo\">1</number></StructWithNamedFields>"
    );
}

#[test]
fn struct_with_custom_field() {
    assert_eq!(
        StructWithCustomField {
            test: Nested {
                flag: true,
            },
        }
        .to_xml(None)
        .unwrap(),
        "<StructWithCustomField xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><test><Nested><bar:flag>true</bar:flag></Nested></test></StructWithCustomField>"
    );
}

#[test]
#[should_panic]
fn struct_with_custom_field_wrong_prefix() {
    assert_eq!(
        StructWithCustomFieldWrongPrefix {
            test: NestedWrongPrefix { flag: true },
        }
        .to_xml(None)
        .unwrap(),
        ""
    );
}

#[test]
fn struct_with_custom_field_from_xml() {
    assert_eq!(
        StructWithCustomFieldFromXml::from_xml("<StructWithCustomFieldFromXml xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><flag>false</flag><Nested><flag>true</flag></Nested></StructWithCustomFieldFromXml>").unwrap(),
        StructWithCustomFieldFromXml {
            flag: false,
            test: Nested { flag: true }
        }
    );
    // Different order
    assert_eq!(
        StructWithCustomFieldFromXml::from_xml("<StructWithCustomFieldFromXml xmlns=\"URI\" xmlns:bar=\"BAZ\" xmlns:foo=\"BAR\"><Nested><flag>true</flag></Nested><flag>false</flag></StructWithCustomFieldFromXml>").unwrap(),
        StructWithCustomFieldFromXml {
            flag: false,
            test: Nested { flag: true }
        }
    );
    assert_eq!(
        Nested::from_xml("<Nested><flag>true</flag></Nested>").unwrap(),
        Nested { flag: true }
    );
}

/* Example impl
struct StructWithCustomFieldFromXml {
    flag: bool,
    test: Nested,
}

struct Nested {
    flag: bool,
}

impl<'xml> FromXml<'xml> for StructWithCustomFieldFromXml {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, Error>
    where
        D: DeserializeXml<'xml>
    {
        //  1. Sprawdzenie czy next równa się typowi z T
        //  2. Sprawdzenie czy ten typ juz byl

        // Jezeli 1 i 2 spelnione to dalej

        enum __Field {
            Field0,
            Field1,
            Ignore,
        }

        fn get_type<'a>(value: &str) -> __Field {
            match value {
                "flag" => __Field::Field0,
                "test" => __Field::Field1,
                _ => __Field::Ignore,
            }
        }

        struct StructWithCustomFieldFromXmlVisitor;
        impl<'xml> Visitor<'xml> for StructWithCustomFieldFromXmlVisitor {
            type Value = StructWithCustomFieldFromXml;

            fn visit_struct<'a>(&self, deserializer: &mut Deserializer) -> Result<Self::Value, Error>
            {
                let mut field0: Option<bool> = None;
                let mut field1: Option<Nested> = None;
                while let Some(item) = &deserializer.iter.next() {
                    match item {
                        XmlRecord::Open(item) => {
                            match get_type(&item.key.as_ref().unwrap()) {
                                __Field::Field0 => {
                                    field0 = Some(bool::deserialize(deserializer).unwrap());
                                },
                                __Field::Field1 => {
                                    //field1 = Some(Nested::deserialize(deserializer).unwrap());
                                },
                                // __Field::__field2 => {
                                //     field2 = Some(Nested::deserialize(deserializer).unwrap());
                                // },
                                _ => (),
                            }
                        },
                        XmlRecord::Close(tag) => (),
                        XmlRecord::Element(_) => panic!("Unexpected element"),
                    }
                }

                Ok(Self::Value {
                    flag: field0.unwrap(),
                    test: field1.unwrap(),
                })
            }
        }

        Ok(deserializer.deserialize_struct(StructWithCustomFieldFromXmlVisitor{}, "StructWithCustomFieldFromXml")?)
    }
}
*/
