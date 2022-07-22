use crate::{Error, Result};
pub use crate::{TagData, XmlRecord};
use std::collections::HashMap;
use std::iter::Peekable;
use xmlparser::{ElementEnd, Token, Tokenizer};

pub struct XmlParser<'a> {
    stack: Vec<String>,
    internal_iter: Peekable<Tokenizer<'a>>,
}

impl<'a> XmlParser<'a> {
    pub fn new(input: &'a str) -> XmlParser<'a> {
        XmlParser {
            stack: Vec::new(),
            internal_iter: Tokenizer::from(input).peekable(),
        }
    }

    fn parse_next(&mut self) -> Result<Option<XmlRecord>> {
        let mut key = String::new();
        let mut prefix_ret = None;
        let mut default_namespace = None;
        let mut namespaces: HashMap<String, String> = HashMap::new();
        let mut attributes: HashMap<String, String> = HashMap::new();

        loop {
            let item = match self.internal_iter.next() {
                Some(v) => v,
                None => return Ok(None),
            };

            println!("{:?}", &item);
            match item {
                Ok(Token::ElementStart { prefix, local, .. }) => {
                    key = local.to_string();
                    prefix_ret = Some(prefix.to_string());
                }
                Ok(Token::ElementEnd { end, .. }) => match end {
                    ElementEnd::Open => {
                        self.stack.push(key.to_owned());
                        println!(
                            "Stack size after push: {}, top: {:?}",
                            self.stack.len(),
                            &key
                        );

                        return Ok(Some(XmlRecord::Open(TagData {
                            key,
                            attributes: Some(attributes),
                            default_namespace,
                            namespaces: Some(namespaces),
                            prefix: prefix_ret,
                        })));
                    }
                    ElementEnd::Close(_, v) => match self.stack.pop() {
                        Some(last) if last == v.as_str() => {
                            println!("Stack size after pop: {}", self.stack.len());
                            return Ok(Some(XmlRecord::Close(last)));
                        }
                        _ => return Err(Error::UnexpectedValue),
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
                        default_namespace = Some(value.to_string());
                    } else if prefix.as_str() == "xmlns" {
                        // Namespaces
                        namespaces.insert(local.to_string(), value.to_string());
                    } else if prefix.is_empty() {
                        // Other attributes
                        attributes.insert(local.to_string(), value.to_string());
                    } else {
                        // TODO: Can the attributes have the prefix?
                        todo!();
                    }
                }
                Ok(Token::Text { text }) => {
                    return Ok(Some(XmlRecord::Element(text.to_string())));
                }
                Ok(_) => return Err(Error::UnexpectedToken),
                Err(e) => return Err(Error::Parse(e)),
            }
        }
    }

    pub fn peek_next_tag(&mut self) -> Result<Option<XmlRecord>> {
        let item = match self.internal_iter.peek() {
            Some(v) => v,
            None => return Ok(None),
        };

        println!("peek: {:?}", &item);
        match item {
            Ok(Token::ElementStart {
                prefix: _, local, ..
            }) => Ok(Some(XmlRecord::Open(TagData {
                key: local.to_string(),
                attributes: None,
                default_namespace: None,
                namespaces: None,
                prefix: None,
            }))),
            Ok(Token::ElementEnd { end, .. }) => {
                if let ElementEnd::Close(..) = end {
                    if self.stack.is_empty() {
                        return Err(Error::UnexpectedEndOfStream);
                    }

                    return Ok(Some(XmlRecord::Close(
                        self.stack.last().unwrap().to_string(),
                    )));
                }
                Err(Error::UnexpectedToken)
            }
            Ok(_) => Err(Error::UnexpectedToken),
            Err(e) => Err(Error::Parse(*e)),
        }
    }
}

impl<'a> Iterator for XmlParser<'a> {
    type Item = Result<XmlRecord>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.parse_next() {
            Ok(Some(v)) => Some(Ok(v)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}
