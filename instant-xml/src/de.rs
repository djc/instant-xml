use std::collections::HashMap;
use std::iter::Peekable;

use super::Error;
use xmlparser::{ElementEnd, Token, Tokenizer};

pub struct Deserializer<'xml> {
    parser: Peekable<XmlParser<'xml>>,
    def_namespaces: HashMap<&'xml str, &'xml str>,
    parser_namespaces: HashMap<&'xml str, &'xml str>,
    def_default_namespace: &'xml str,
    parser_default_namespace: &'xml str,
    tag_attributes: Vec<(&'xml str, &'xml str)>,
    next_type: EntityType,
}

impl<'xml> Deserializer<'xml> {
    pub fn new(input: &'xml str) -> Self {
        Self {
            parser: XmlParser::new(input).peekable(),
            def_namespaces: std::collections::HashMap::new(),
            parser_namespaces: std::collections::HashMap::new(),
            def_default_namespace: "",
            parser_default_namespace: "",
            tag_attributes: Vec::new(),
            next_type: EntityType::Element,
        }
    }

    pub fn peek_next_tag(&mut self) -> Result<Option<Node<'xml>>, Error> {
        let record = match self.parser.peek() {
            Some(Ok(record)) => record,
            Some(Err(err)) => return Err(err.clone()),
            None => return Ok(None),
        };

        Ok(Some(match record {
            XmlRecord::Open(TagData {
                key, ns, prefix, ..
            }) => {
                let ns = match (ns, prefix) {
                    (_, Some(prefix)) => match self.parser_namespaces.get(prefix) {
                        Some(ns) => ns,
                        None => return Err(Error::WrongNamespace),
                    },
                    (Some(ns), None) => ns,
                    (None, None) => self.parser_default_namespace,
                };

                Node::Open { ns, name: key }
            }
            XmlRecord::Element(text) => Node::Text { text },
            XmlRecord::Close(name) => Node::Close { name },
        }))
    }

    // Check if defined and gotten namespaces equals for each field
    pub fn compare_namespace(
        &self,
        expected: &Option<&str>,
        actual: Option<&str>,
    ) -> Result<(), Error> {
        match (expected, actual) {
            (Some(expected), Some(actual)) => {
                match self.parser_namespaces.get(expected) == self.def_namespaces.get(actual) {
                    true => Ok(()),
                    false => Err(Error::WrongNamespace),
                }
            }
            (Some(_), None) | (None, Some(_)) => Err(Error::WrongNamespace),
            (None, None) => Ok(()),
        }
    }

    pub fn peek_next_attribute(&self) -> Option<&(&'xml str, &'xml str)> {
        self.tag_attributes.last()
    }

    pub fn deserialize_struct<V>(
        &mut self,
        visitor: V,
        name: &str,
        def_default_namespace: &'xml str,
        def_namespaces: &HashMap<&'xml str, &'xml str>,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'xml>,
    {
        // Saveing current defined default namespace
        let def_default_namespace_to_revert = self.def_default_namespace;
        self.def_default_namespace = def_default_namespace;

        // Adding struct defined namespaces
        let new_def_namespaces = def_namespaces
            .iter()
            .filter(|(k, v)| self.def_namespaces.insert(k, v).is_none())
            .collect::<Vec<_>>();

        // Process open tag
        let tag_data = match self.parser.next() {
            Some(Ok(XmlRecord::Open(item))) if item.key == name => item,
            _ => return Err(Error::UnexpectedValue),
        };

        // Set current attributes
        self.tag_attributes = tag_data.attributes;

        // Saveing current parser default namespace
        let parser_default_namespace_to_revert = self.parser_default_namespace;

        // Set parser default namespace
        match tag_data.ns {
            Some(namespace) => {
                self.parser_default_namespace = namespace;
            }
            None => {
                // If there is no default namespace in the tag, check if parent default namespace equals the current one
                if def_default_namespace_to_revert != self.def_default_namespace {
                    return Err(Error::WrongNamespace);
                }
            }
        }

        // Compare parser namespace with defined one
        if self.parser_default_namespace != self.def_default_namespace {
            return Err(Error::WrongNamespace);
        }

        // Adding parser namespaces
        let new_parser_namespaces = tag_data
            .prefixes
            .iter()
            .filter(|(k, v)| self.parser_namespaces.insert(k, v).is_none())
            .collect::<Vec<_>>();

        let ret = visitor.visit_struct(self)?;

        // Process close tag
        let item = match self.parser.next() {
            Some(item) => item?,
            None => return Err(Error::MissingTag),
        };

        match item {
            XmlRecord::Close(v) if v == name => {}
            _ => return Err(Error::UnexpectedTag),
        }

        // Removing parser namespaces
        let _ = new_parser_namespaces
            .iter()
            .map(|(k, _)| self.parser_namespaces.remove(*k));

        // Removing struct defined namespaces
        let _ = new_def_namespaces
            .iter()
            .map(|(k, _)| self.def_namespaces.remove(*k));

        // Retriving old defined namespace
        self.def_default_namespace = def_default_namespace_to_revert;

        // Retriving old parser namespace
        self.parser_default_namespace = parser_default_namespace_to_revert;
        Ok(ret)
    }

    pub fn set_next_type_as_attribute(&mut self) -> Result<(), Error> {
        if self.next_type == EntityType::Attribute {
            return Err(Error::UnexpectedState);
        }

        self.next_type = EntityType::Attribute;
        Ok(())
    }

    pub fn consume_next_type(&mut self) -> EntityType {
        let ret = self.next_type.clone();
        self.next_type = EntityType::Element;
        ret
    }

    pub(crate) fn deserialize_element<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'xml>,
    {
        // Process open tag
        match self.parser.next() {
            Some(Ok(XmlRecord::Open(_))) => {}
            _ => return Err(Error::UnexpectedValue),
        };

        match self.parser.next() {
            Some(Ok(XmlRecord::Element(v))) => {
                let ret = visitor.visit_str(v);
                self.parser.next();
                ret
            }
            _ => Err(Error::UnexpectedValue),
        }
    }

    pub(crate) fn deserialize_attribute<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'xml>,
    {
        match self.tag_attributes.pop() {
            Some((_, value)) => visitor.visit_str(value),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }
}

pub struct XmlParser<'xml> {
    stack: Vec<&'xml str>,
    iter: Peekable<Tokenizer<'xml>>,
}

impl<'a> XmlParser<'a> {
    pub fn new(input: &'a str) -> XmlParser<'a> {
        XmlParser {
            stack: Vec::new(),
            iter: Tokenizer::from(input).peekable(),
        }
    }

    pub fn peek_next_tag(&mut self) -> Result<Option<XmlRecord<'a>>, Error> {
        let item = match self.iter.peek() {
            Some(v) => v,
            None => return Ok(None),
        };

        match item {
            Ok(Token::ElementStart { prefix, local, .. }) => {
                let prefix = match prefix.is_empty() {
                    true => None,
                    false => Some(prefix.as_str()),
                };

                Ok(Some(XmlRecord::Open(TagData {
                    key: local.as_str(),
                    attributes: Vec::new(),
                    ns: Some(""),
                    prefixes: HashMap::new(),
                    prefix,
                })))
            }
            Ok(Token::ElementEnd {
                end: ElementEnd::Close(..),
                ..
            }) => {
                if self.stack.is_empty() {
                    return Err(Error::UnexpectedEndOfStream);
                }

                return Ok(Some(XmlRecord::Close(self.stack.last().unwrap())));
            }
            Ok(_) => Err(Error::UnexpectedToken),
            Err(e) => Err(Error::Parse(*e)),
        }
    }
}

impl<'xml> Iterator for XmlParser<'xml> {
    type Item = Result<XmlRecord<'xml>, Error>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut key: Option<&str> = None;
        let mut prefix_ret: Option<&str> = None;
        let mut default_namespace = None;
        let mut namespaces = HashMap::new();
        let mut attributes = Vec::new();

        loop {
            let item = match self.iter.next() {
                Some(v) => v,
                None => return None,
            };

            match item {
                Ok(Token::ElementStart { prefix, local, .. }) => {
                    key = Some(local.as_str());
                    prefix_ret = match prefix.is_empty() {
                        true => None,
                        false => Some(prefix.as_str()),
                    };
                }
                Ok(Token::ElementEnd { end, .. }) => match end {
                    ElementEnd::Open => {
                        self.stack.push(key.unwrap());

                        return Some(Ok(XmlRecord::Open(TagData {
                            key: key.unwrap(),
                            attributes,
                            ns: default_namespace,
                            prefixes: namespaces,
                            prefix: prefix_ret,
                        })));
                    }
                    ElementEnd::Close(_, v) => match self.stack.pop() {
                        Some(last) if last == v.as_str() => {
                            return Some(Ok(XmlRecord::Close(last)));
                        }
                        _ => return Some(Err(Error::UnexpectedValue)),
                    },
                    ElementEnd::Empty => {
                        todo!();
                    }
                },
                Ok(Token::Attribute {
                    prefix,
                    local,
                    value,
                    ..
                }) => {
                    if prefix.is_empty() && local.as_str() == "xmlns" {
                        // Default namespace
                        default_namespace = Some(value.as_str());
                    } else if prefix.as_str() == "xmlns" {
                        // Namespaces
                        namespaces.insert(local.as_str(), value.as_str());
                    } else if prefix.is_empty() {
                        // Other attributes
                        attributes.push((local.as_str(), value.as_str()));
                    } else {
                        // TODO: Can the attributes have the prefix?
                        todo!();
                    }
                }
                Ok(Token::Text { text }) => {
                    return Some(Ok(XmlRecord::Element(text.as_str())));
                }
                Ok(_) => return Some(Err(Error::UnexpectedToken)),
                Err(e) => return Some(Err(Error::Parse(e))),
            }
        }
    }
}

pub trait Visitor<'xml>: Sized {
    type Value;

    fn visit_str(self, _value: &'xml str) -> Result<Self::Value, Error> {
        unimplemented!();
    }

    fn visit_struct(&self, _deserializer: &mut Deserializer<'xml>) -> Result<Self::Value, Error> {
        unimplemented!();
    }
}

pub enum XmlRecord<'xml> {
    Open(TagData<'xml>),
    Element(&'xml str),
    Close(&'xml str),
}

pub struct TagData<'xml> {
    pub key: &'xml str,
    pub attributes: Vec<(&'xml str, &'xml str)>,
    pub ns: Option<&'xml str>,
    pub prefixes: HashMap<&'xml str, &'xml str>,
    pub prefix: Option<&'xml str>,
}

pub enum Node<'xml> {
    Open { ns: &'xml str, name: &'xml str },
    Close { name: &'xml str },
    Text { text: &'xml str },
}

#[derive(Clone, PartialEq, Eq)]
pub enum EntityType {
    Element,
    Attribute,
}
