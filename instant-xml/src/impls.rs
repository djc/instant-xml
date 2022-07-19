use crate::{DeserializeXml, Error, FromXml, Visitor};
use std::str::FromStr;

struct BoolVisitor;

impl<'de> Visitor<'de> for BoolVisitor {
    type Value = bool;

    fn visit_str<'a>(self, value: &str) -> Result<Self::Value, Error> {
        Ok(FromStr::from_str(value).unwrap())
    }
}

impl<'xml> FromXml<'xml> for bool {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, Error>
    where
        D: DeserializeXml<'xml>,
    {
        deserializer.deserialize_bool(BoolVisitor)
    }
}
