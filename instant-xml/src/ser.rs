use std::collections::HashMap;
use std::fmt::{self};

use super::Error;
use crate::ToXml;

pub struct Serializer<'xml, W: fmt::Write + ?Sized> {
    // For parent namespaces the key is the namespace and the value is the prefix. We are adding to map
    // only if the namespaces do not exist, if it does exist then we are using an already defined parent prefix.
    #[doc(hidden)]
    pub parent_namespaces: HashMap<&'xml str, &'xml str>,
    output: &'xml mut W,
    parent_default_namespace: &'xml str,
    parent_default_namespace_to_revert: &'xml str,
    state: State,
}

impl<'xml, W: fmt::Write + ?Sized> Serializer<'xml, W> {
    pub fn new(output: &'xml mut W) -> Self {
        Self {
            parent_namespaces: HashMap::new(),
            output,
            parent_default_namespace: "",
            parent_default_namespace_to_revert: "",
            state: State::Element,
        }
    }

    pub fn write_start(
        &mut self,
        prefix: Option<&str>,
        name: &str,
        ns: Option<&str>,
    ) -> Result<(), Error> {
        if self.state != State::Element {
            return Err(Error::UnexpectedState);
        }

        match prefix {
            Some(prefix) => self.output.write_fmt(format_args!("<{prefix}:{name}"))?,
            None => match ns {
                Some(ns) => self
                    .output
                    .write_fmt(format_args!("<{name} xmlns=\"{ns}\""))?,
                None => self.output.write_fmt(format_args!("<{name}"))?,
            },
        }

        self.state = State::Attribute;
        Ok(())
    }

    pub fn write_attr<V: ToXml + ?Sized>(&mut self, name: &str, value: &V) -> Result<(), Error> {
        if self.state != State::Attribute {
            return Err(Error::UnexpectedState);
        }

        self.output.write_fmt(format_args!(" {}=\"", name))?;
        self.state = State::Scalar;
        value.serialize(self)?;
        self.state = State::Attribute;
        self.output.write_char('"')?;
        Ok(())
    }

    pub fn write_prefix(&mut self, prefix: &str, ns: &str) -> Result<(), Error> {
        if self.state != State::Attribute {
            return Err(Error::UnexpectedState);
        }

        self.output
            .write_fmt(format_args!(" xmlns:{prefix}=\"{ns}\""))?;
        Ok(())
    }

    pub fn write_str<V: fmt::Display + ?Sized>(&mut self, value: &V) -> Result<(), Error> {
        if !matches!(self.state, State::Element | State::Scalar) {
            return Err(Error::UnexpectedState);
        }

        self.output.write_fmt(format_args!("{}", value))?;
        self.state = State::Element;
        Ok(())
    }

    pub fn end_start(&mut self) -> Result<(), Error> {
        if self.state != State::Attribute {
            return Err(Error::UnexpectedState);
        }

        self.output.write_char('>')?;
        self.state = State::Element;
        Ok(())
    }

    pub fn write_close(&mut self, prefix: Option<&str>, name: &str) -> Result<(), Error> {
        if self.state != State::Element {
            return Err(Error::UnexpectedState);
        }

        match prefix {
            Some(prefix) => self.output.write_fmt(format_args!("</{prefix}:{name}>"))?,
            None => self.output.write_fmt(format_args!("</{name}>"))?,
        }

        Ok(())
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
}

#[derive(Debug, Eq, PartialEq)]
enum State {
    Attribute,
    Element,
    Scalar,
}
