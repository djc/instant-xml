use std::str::FromStr;

use crate::{Deserializer, EntityType, Error, FromXml, Visitor, XMLTagName};

struct BoolVisitor;

impl<'de> Visitor<'de> for BoolVisitor {
    type Value = bool;

    fn visit_str<'a>(self, value: &str) -> Result<Self::Value, Error> {
        Ok(FromStr::from_str(value).unwrap())
    }
}

impl<'xml> FromXml<'xml> for bool {
    const TAG_NAME: XMLTagName<'xml> = XMLTagName::FieldName;

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, Error> {
        match deserializer.consume_next_kind()? {
            EntityType::Element => deserializer.deserialize_bool(BoolVisitor),
            EntityType::Attribute => deserializer.deserialize_attribute(BoolVisitor),
        }
    }
}
