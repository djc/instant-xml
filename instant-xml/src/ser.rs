use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{self};
use std::mem;

use super::Error;
use crate::ToXml;

/// XML serializer for writing structured XML output
pub struct Serializer<'xml, W: fmt::Write + ?Sized> {
    output: &'xml mut W,
    /// Map namespace keys to prefixes.
    ///
    /// The prefix map is updated using `Context` types that are held on the stack in the relevant
    /// `ToXml` implementation. If a prefix is already defined for a given namespace, we don't
    /// update the set with the new prefix.
    prefixes: HashMap<&'static str, &'static str>,
    default_ns: &'static str,
    state: State,
}

impl<'xml, W: fmt::Write + ?Sized> Serializer<'xml, W> {
    /// Create a new serializer writing to the given output
    pub fn new(output: &'xml mut W) -> Self {
        Self {
            output,
            prefixes: HashMap::new(),
            default_ns: "",
            state: State::Element,
        }
    }

    /// Write the opening tag for an element
    ///
    /// Returns the namespace prefix if one was used.
    ///
    /// The `cx` parameter can be used to specify namespace declarations for the element. When
    /// passing in `None`, you'll probably need to specify `None::<Context<0>>`.
    pub fn write_start<'a, const N: usize>(
        &mut self,
        name: &'a str,
        ns: &str,
        cx: Option<Context<N>>,
    ) -> Result<Element<'a, N>, Error> {
        if self.state != State::Element {
            return Err(Error::UnexpectedState("invalid state for element start"));
        }

        let prefix = match (ns == self.default_ns, self.prefixes.get(ns)) {
            (true, _) => {
                self.output.write_fmt(format_args!("<{name}"))?;
                None
            }
            (false, Some(prefix)) => {
                self.output.write_fmt(format_args!("<{prefix}:{name}"))?;
                Some(*prefix)
            }
            _ => {
                self.output
                    .write_fmt(format_args!("<{name} xmlns=\"{ns}\""))?;
                None
            }
        };

        self.state = State::Attribute;
        let Some(cx) = cx else {
            return Ok(Element {
                prefix,
                name,
                parent: None,
            });
        };

        let mut old = Context::default();
        let prev = mem::replace(&mut self.default_ns, cx.default_ns);
        let _ = mem::replace(&mut old.default_ns, prev);

        let mut used = 0;
        for prefix in cx.prefixes.into_iter() {
            if prefix.prefix.is_empty() {
                continue;
            }

            if self.prefixes.contains_key(prefix.ns) {
                continue;
            }

            self.output
                .write_fmt(format_args!(" xmlns:{}=\"{}\"", prefix.prefix, prefix.ns))?;

            let prev = match self.prefixes.entry(prefix.ns) {
                Entry::Occupied(mut entry) => mem::replace(entry.get_mut(), prefix.prefix),
                Entry::Vacant(entry) => {
                    entry.insert(prefix.prefix);
                    ""
                }
            };

            old.prefixes[used] = Prefix {
                ns: prefix.ns,
                prefix: prev,
            };
            used += 1;
        }

        Ok(Element {
            prefix,
            name,
            parent: Some(old),
        })
    }

    /// Write an attribute with the given name and value
    pub fn write_attr<V: ToXml + ?Sized>(
        &mut self,
        name: &str,
        ns: &str,
        value: &V,
    ) -> Result<(), Error> {
        if self.state != State::Attribute {
            return Err(Error::UnexpectedState("invalid state for attribute"));
        }

        match ns == self.default_ns {
            true => self.output.write_fmt(format_args!(" {name}=\""))?,
            false => {
                let prefix = self
                    .prefixes
                    .get(ns)
                    .ok_or(Error::UnexpectedState("unknown prefix"))?;
                self.output.write_fmt(format_args!(" {prefix}:{name}=\""))?;
            }
        }

        self.state = State::Scalar;
        value.serialize(None, self)?;
        self.state = State::Attribute;
        self.output.write_char('"')?;
        Ok(())
    }

    /// Write a string value (text content or attribute value)
    pub fn write_str<V: fmt::Display + ?Sized>(&mut self, value: &V) -> Result<(), Error> {
        if !matches!(self.state, State::Element | State::Scalar) {
            return Err(Error::UnexpectedState("invalid state for scalar"));
        }

        self.output.write_fmt(format_args!("{value}"))?;
        self.state = State::Element;
        Ok(())
    }

    /// Complete the opening tag and transition to element content
    pub fn end_start(&mut self) -> Result<(), Error> {
        if self.state != State::Attribute {
            return Err(Error::UnexpectedState("invalid state for element end"));
        }

        self.output.write_char('>')?;
        self.state = State::Element;
        Ok(())
    }

    /// Close an empty element (self-closing tag)
    pub fn end_empty(&mut self) -> Result<(), Error> {
        if self.state != State::Attribute {
            return Err(Error::UnexpectedState("invalid state for element end"));
        }

        self.output.write_str(" />")?;
        self.state = State::Element;
        Ok(())
    }

    /// Write the closing tag for an element
    pub fn write_close<const N: usize>(&mut self, element: Element<'_, N>) -> Result<(), Error> {
        if self.state != State::Element {
            return Err(Error::UnexpectedState("invalid state for close element"));
        }

        match element.prefix {
            Some(prefix) => self
                .output
                .write_fmt(format_args!("</{prefix}:{}>", element.name))?,
            None => self.output.write_fmt(format_args!("</{}>", element.name))?,
        }

        let Some(old) = element.parent else {
            return Ok(());
        };

        let _ = mem::replace(&mut self.default_ns, old.default_ns);
        for prefix in old.prefixes.into_iter() {
            if prefix.ns.is_empty() && prefix.prefix.is_empty() {
                continue;
            }

            let mut entry = match self.prefixes.entry(prefix.ns) {
                Entry::Occupied(entry) => entry,
                Entry::Vacant(_) => unreachable!(),
            };

            match prefix.prefix {
                "" => {
                    entry.remove();
                }
                prev => {
                    let _ = mem::replace(entry.get_mut(), prev);
                }
            }
        }

        Ok(())
    }

    /// Get the prefix for a namespace URI, if any
    pub fn prefix(&self, ns: &str) -> Option<&'static str> {
        self.prefixes.get(ns).copied()
    }

    /// Get the current default namespace URI
    pub fn default_ns(&self) -> &'static str {
        self.default_ns
    }
}

/// An element being serialized, used for tracking namespace context
#[non_exhaustive]
pub struct Element<'a, const N: usize> {
    /// Prefix of the element, if any
    pub prefix: Option<&'static str>,
    /// Local name of the element
    pub name: &'a str,
    /// Namespace context of the parent element, if any
    pub parent: Option<Context<N>>,
}

/// Namespace context for serialization
#[derive(Debug)]
pub struct Context<const N: usize> {
    /// The default namespace URI
    pub default_ns: &'static str,
    /// Array of namespace prefix mappings
    pub prefixes: [Prefix; N],
}

impl<const N: usize> Default for Context<N> {
    fn default() -> Self {
        Self {
            default_ns: Default::default(),
            prefixes: [Prefix { prefix: "", ns: "" }; N],
        }
    }
}

/// A namespace prefix mapping
#[derive(Clone, Copy, Debug, Default)]
pub struct Prefix {
    /// The namespace prefix
    pub prefix: &'static str,
    /// The namespace URI
    pub ns: &'static str,
}

#[derive(Debug, Eq, PartialEq)]
enum State {
    Attribute,
    Element,
    Scalar,
}
