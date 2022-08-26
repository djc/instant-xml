use std::str::FromStr;

use crate::FieldAttribute;
use crate::FieldContext;
use crate::Serializer;
use crate::ToXml;
use crate::{Deserializer, EntityType, Error, FromXml, TagName, Visitor};

struct BoolVisitor;

impl<'de> Visitor<'de> for BoolVisitor {
    type Value = bool;

    fn visit_str<'a>(self, value: &str) -> Result<Self::Value, Error> {
        match FromStr::from_str(value) {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::Other(e.to_string())),
        }
    }
}

impl<'xml> FromXml<'xml> for bool {
    const TAG_NAME: TagName<'xml> = TagName::FieldName;

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, Error> {
        match deserializer.consume_next_type() {
            EntityType::Element => deserializer.deserialize_bool(BoolVisitor),
            EntityType::Attribute => deserializer.deserialize_attribute(BoolVisitor),
        }
    }
}

// #[derive(Debug, Eq, PartialEq)]
// #[xml(namespace("URI", bar = "BAZ"))]
// struct Nested {
//     #[xml(namespace(bar))]
//     flag: bool,
// }

// impl ToXml for Nested {
//     fn serialize<W>(
//         &self,
//         serializer: &mut Serializer<W>,
//     ) -> Result<(), Error>
//     where
//         W: std::fmt::Write,
//     {
//         let mut field_context = FieldContext {
//             name: "Nested",
//             attribute: None,
//         };
//         let current_default_namespace = "URI";
//         let mut to_remove: Vec<&str> = Vec::new();
//         if serializer.parent_namespaces.insert("bar", "BAZ") {
//             to_remove.push("bar");
//         }
//         serializer.output.write_char('<')?;
//         serializer.output.write_str(field_context.name)?;
//         if serializer.parent_default_namespace != "URI" {
//             serializer.output.write_str(" xmlns=\"")?;
//             serializer.output.write_str("URI")?;
//             serializer.output.write_char('\"')?;
//         }
//         serializer.parent_default_namespace = "URI";
//         match serializer.parent_namespaces.get("bar") {
//             Some(val) if val == &"BAZ" => panic!("not yet implemented"),
//             _ => {
//                 serializer.output.write_str(" xmlns:")?;
//                 serializer.output.write_str("bar")?;
//                 serializer.output.write_str("=\"")?;
//                 serializer.output.write_str("BAZ")?;
//                 serializer.output.write_char('\"')?;
//             }
//         }
//         serializer.output.write_char('>')?;
//         let mut field = FieldContext {
//             name: "flag",
//             attribute: None,
//         };
//         field.attribute = Some(FieldAttribute::Prefix("bar"));
//         serializer.set_field_context(field)?;
//         self.flag.serialize(serializer)?;
//         serializer.output.write_str("</")?;
//         serializer.output.write_str("Nested")?;
//         serializer.output.write_char('>')?;
//         for it in to_remove {
//             serializer.parent_namespaces.remove(it);
//         }
//         Ok(())
//     }
// }
