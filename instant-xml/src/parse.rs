use xmlparser::{ElementEnd, Token, Tokenizer};

#[derive(Debug)]
pub struct TagData {
    pub attributes: Option<Vec<String>>,
    pub key: Option<String>, // TODO: Not an option
}

pub enum XmlRecord {
    Open(TagData),
    Element(String),
    Close(String),
}

pub struct XmlParser<'a> {
    stack: Vec<String>,
    internal_iter: Tokenizer<'a>,
}

impl<'a> XmlParser<'a> {
    pub fn from_str(input: &'a str) -> XmlParser<'a> {
        XmlParser {
            stack: Vec::new(),
            internal_iter: Tokenizer::from(input),
        }
    }

    fn parse_next(&mut self) -> Result<Option<XmlRecord>, ()> {
        let mut attributes = None;
        let mut key = None;

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
                    key = Some(local.to_string());
                }
                Ok(Token::ElementEnd { end, .. }) => {
                    match end {
                        ElementEnd::Open => {
                            self.stack.push(key.clone().unwrap());
                            println!("Stack size after push: {}, top: {:?}", self.stack.len(), &key);

                            return Ok(Some(XmlRecord::Open(TagData {
                                attributes,
                                key
                            })));
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