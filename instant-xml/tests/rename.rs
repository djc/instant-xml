use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, PartialEq, FromXml, ToXml)]
#[xml(rename = "renamed")]
struct Renamed {
    #[xml(attribute, rename = "renamed")]
    flag: bool,
}

#[test]
fn renamed() {
    assert_eq!(
        from_str::<Renamed>("<renamed renamed=\"true\"></renamed>"),
        Ok(Renamed { flag: true })
    );

    assert_eq!(
        to_string(&Renamed { flag: true }).unwrap(),
        "<renamed renamed=\"true\"></renamed>"
    );
}

#[test]
fn rename_all_struct() {
    #[derive(Debug, PartialEq, Eq, ToXml, FromXml)]
    #[xml(rename_all = "UPPERCASE")]
    pub struct TestStruct {
        field_1: String,
        #[xml(attribute)]
        field_2: bool,
    }

    let serialized = r#"<TestStruct FIELD_2="true"><FIELD_1>value</FIELD_1></TestStruct>"#;
    let instance = TestStruct {
        field_1: "value".into(),
        field_2: true,
    };

    assert_eq!(to_string(&instance).unwrap(), serialized);
    assert_eq!(from_str::<TestStruct>(serialized), Ok(instance));
}
