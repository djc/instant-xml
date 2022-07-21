use crate::Error;
pub use crate::{TagData, XmlRecord};
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

    fn parse_next(&mut self) -> Result<Option<XmlRecord>, Error> {
        let mut attributes = None;
        let mut key = String::new();

        loop {
            let item = match self.internal_iter.next() {
                Some(v) => v,
                None => return Ok(None),
            };

            println!("{:?}", &item);
            match item {
                Ok(Token::ElementStart {
                    prefix: _, local, ..
                }) => {
                    key = local.to_string();
                }
                Ok(Token::ElementEnd { end, .. }) => {
                    match end {
                        ElementEnd::Open => {
                            self.stack.push(key.to_owned());
                            println!(
                                "Stack size after push: {}, top: {:?}",
                                self.stack.len(),
                                &key
                            );

                            return Ok(Some(XmlRecord::Open(TagData { attributes, key })));
                        }
                        ElementEnd::Close(..) => {
                            // TODO: Check if close tag equal to tag in top of stack
                            let last = self.stack.pop();

                            println!("Stack size after pop: {}", self.stack.len());
                            return Ok(Some(XmlRecord::Close(last.unwrap())));
                        }
                        ElementEnd::Empty => {
                            todo!();
                        }
                    }
                }
                Ok(Token::Attribute { prefix: _, .. }) => {
                    // TODO: Add to attributes map
                    attributes = Some(Vec::new());
                }
                Ok(Token::Text { text }) => {
                    return Ok(Some(XmlRecord::Element(text.to_string())));
                }
                _ => (), // Todo
            }
        }
    }

    pub fn peek_next_tag(&mut self) -> Result<Option<XmlRecord>, Error> {
        let item = match self.internal_iter.peek() {
            Some(v) => v,
            None => return Ok(None),
        };

        println!("peek: {:?}", &item);
        match item {
            Ok(Token::ElementStart {
                prefix: _, local, ..
            }) => Ok(Some(XmlRecord::Open(TagData {
                attributes: None,
                key: local.to_string(),
            }))),
            Ok(Token::ElementEnd { end, .. }) => {
                if let ElementEnd::Close(..) = end {
                    return Ok(Some(XmlRecord::Close(
                        self.stack.last().unwrap().to_string(),
                    )));
                }
                panic!("Wrong end type")
            }
            _ => panic!("Wrong token, expected Start or End"),
        }
    }
}

impl<'a> Iterator for XmlParser<'a> {
    type Item = XmlRecord;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.parse_next() {
            Ok(Some(v)) => Some(v),
            Ok(None) => None,
            Err(_) => todo!(),
        }
    }
}
