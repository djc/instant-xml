use std::collections::HashMap;
use std::fmt::{self, Write};

use super::Error;

pub struct Serializer<'xml, W: fmt::Write + ?Sized> {
    // For parent namespaces the key is the namespace and the value is the prefix. We are adding to map
    // only if the namespaces do not exist, if it does exist then we are using an already defined parent prefix.
    #[doc(hidden)]
    pub parent_namespaces: HashMap<&'xml str, &'xml str>,
    #[doc(hidden)]
    pub output: &'xml mut W,

    parent_default_namespace: &'xml str,
    parent_default_namespace_to_revert: &'xml str,
    current_attributes: String,
    next_field_context: Option<FieldContext<'xml>>,
}

impl<'xml, W: fmt::Write + ?Sized> Serializer<'xml, W> {
    pub fn new(output: &'xml mut W) -> Self {
        Self {
            parent_namespaces: HashMap::new(),
            output,
            parent_default_namespace: "",
            parent_default_namespace_to_revert: "",
            next_field_context: None,
            current_attributes: String::new(),
        }
    }

    pub fn consume_current_attributes(&mut self) -> Result<(), Error> {
        self.output.write_str(&self.current_attributes)?;
        self.current_attributes.clear();
        Ok(())
    }

    pub fn add_attribute_key(&mut self, attr_key: &impl fmt::Display) -> Result<(), Error> {
        self.current_attributes.push(' ');
        write!(self.current_attributes, "{}", attr_key)?;
        self.current_attributes.push('=');
        Ok(())
    }

    pub fn add_attribute_value(&mut self, attr_value: &impl fmt::Display) -> Result<(), Error> {
        self.current_attributes.push('"');
        write!(self.current_attributes, "{}", attr_value)?;
        self.current_attributes.push('"');
        Ok(())
    }

    pub fn set_field_context(&mut self, field_context: FieldContext<'xml>) -> Result<(), Error> {
        if self.next_field_context.is_some() {
            return Err(Error::UnexpectedState);
        };

        self.next_field_context = Some(field_context);
        Ok(())
    }

    pub fn consume_field_context(&mut self) -> Option<FieldContext<'xml>> {
        self.next_field_context.take()
    }

    pub fn set_parent_default_namespace(&mut self, namespace: &'xml str) -> Result<(), Error> {
        self.parent_default_namespace = namespace;
        Ok(())
    }

    pub fn parent_default_namespace(&self) -> &'xml str {
        self.parent_default_namespace
    }

    pub fn update_parent_default_namespace(&mut self, namespace: &'xml str) {
        self.parent_default_namespace_to_revert = self.parent_default_namespace;
        self.parent_default_namespace = namespace;
    }

    pub fn retrieve_parent_default_namespace(&mut self) {
        self.parent_default_namespace = self.parent_default_namespace_to_revert;
    }

    pub fn add_open_tag(&mut self, field_context: &FieldContext) -> Result<(), Error> {
        match field_context.attribute {
            Some(FieldAttribute::Prefix(prefix)) => {
                self.output.write_char('<')?;
                self.output.write_str(prefix)?;
                self.output.write_char(':')?;
                self.output.write_str(field_context.name)?;
                self.output.write_char('>')?;
            }
            Some(FieldAttribute::Namespace(namespace))
                if self.parent_default_namespace != namespace =>
            {
                self.output.write_char('<')?;
                self.output.write_str(field_context.name)?;
                self.output.write_str(" xmlns=\"")?;
                self.output.write_str(namespace)?;
                self.output.write_str("\">")?;
            }
            _ => {
                self.output.write_char('<')?;
                self.output.write_str(field_context.name)?;
                self.output.write_char('>')?;
            }
        }
        Ok(())
    }

    pub fn add_close_tag(&mut self, field_context: FieldContext) -> Result<(), Error> {
        match field_context.attribute {
            Some(FieldAttribute::Prefix(prefix)) => {
                self.output.write_str("</")?;
                self.output.write_str(prefix)?;
                self.output.write_char(':')?;
                self.output.write_str(field_context.name)?;
                self.output.write_char('>')?;
            }
            _ => {
                self.output.write_str("</")?;
                self.output.write_str(field_context.name)?;
                self.output.write_char('>')?;
            }
        }
        Ok(())
    }
}

pub struct FieldContext<'xml> {
    #[doc(hidden)]
    pub name: &'xml str,
    #[doc(hidden)]
    pub attribute: Option<FieldAttribute<'xml>>,
}

pub enum FieldAttribute<'xml> {
    Prefix(&'xml str),
    Namespace(&'xml str),
    Attribute,
}
