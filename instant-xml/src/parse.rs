use std::collections::HashMap;
use std::iter::Peekable;

use xmlparser::{ElementEnd, Token, Tokenizer};

use crate::Error;
pub use crate::{TagData, XmlRecord};

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

    pub fn peek_next_tag(&mut self) -> Result<Option<XmlRecord>, Error> {
        let item = match self.iter.peek() {
            Some(v) => v,
            None => return Ok(None),
        };

        println!("peek: {:?}", &item);
        match item {
            Ok(Token::ElementStart { prefix, local, .. }) => {
                let prefix = match prefix.is_empty() {
                    true => None,
                    false => Some(prefix.as_str()),
                };

                Ok(Some(XmlRecord::Open(TagData {
                    key: local,
                    attributes: Vec::new(),
                    default_namespace: "",
                    namespaces: HashMap::new(),
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
        let mut default_namespace = "";
        let mut namespaces = HashMap::new();
        let mut attributes = Vec::new();

        loop {
            let item = match self.iter.next() {
                Some(v) => v,
                None => return None,
            };

            println!("{:?}", &item);
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
                        println!(
                            "Stack size after push: {}, top: {:?}",
                            self.stack.len(),
                            key
                        );

                        return Some(Ok(XmlRecord::Open(TagData {
                            key: key.unwrap(),
                            attributes,
                            default_namespace,
                            namespaces: namespaces,
                            prefix: prefix_ret,
                        })));
                    }
                    ElementEnd::Close(_, v) => match self.stack.pop() {
                        Some(last) if last == v.as_str() => {
                            println!("Stack size after pop: {}", self.stack.len());
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
                        default_namespace = value.as_str();
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
