use std::str::FromStr;

use crate::{Deserializer, EntityType, FromXml, Result, Visitor, XMLTagName};

struct BoolVisitor;

impl<'de> Visitor<'de> for BoolVisitor {
    type Value = bool;

    fn visit_str<'a>(self, value: &str) -> Result<Self::Value> {
        Ok(FromStr::from_str(value).unwrap())
    }
}

impl<'xml> FromXml<'xml> for bool {
    const TAG_NAME: XMLTagName<'xml> = XMLTagName::FieldName;

    fn deserialize(deserializer: &mut Deserializer, kind: EntityType) -> Result<Self> {
        match kind {
            EntityType::Element => deserializer.deserialize_bool(BoolVisitor),
            EntityType::Attribute => deserializer.deserialize_attribute(BoolVisitor),
        }
    }
}
