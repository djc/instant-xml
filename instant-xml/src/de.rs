use std::collections::{BTreeMap, VecDeque};

use super::{Error, Id};
use xmlparser::{ElementEnd, Token, Tokenizer};

pub struct Deserializer<'cx, 'xml> {
    pub(crate) local: &'xml str,
    prefix: Option<&'xml str>,
    level: usize,
    done: bool,
    context: &'cx mut Context<'xml>,
}

impl<'cx, 'xml> Deserializer<'cx, 'xml> {
    pub(crate) fn new(data: TagData<'xml>, context: &'cx mut Context<'xml>) -> Self {
        let level = context.stack.len();
        context.stack.push(data.level);

        Self {
            local: data.key,
            prefix: data.prefix,
            level,
            done: false,
            context,
        }
    }

    pub fn nested<'a>(&'a mut self, data: TagData<'xml>) -> Deserializer<'a, 'xml>
    where
        'cx: 'a,
    {
        Deserializer::new(data, self.context)
    }

    pub fn for_attr<'a>(&'a mut self, attr: Attribute<'xml>) -> Deserializer<'a, 'xml>
    where
        'cx: 'a,
    {
        self.context
            .records
            .push_front(XmlRecord::AttributeValue(attr.value));

        Deserializer {
            local: self.local,
            prefix: self.prefix,
            level: self.level,
            done: self.done,
            context: self.context,
        }
    }

    pub fn ignore(&mut self) -> Result<(), Error> {
        loop {
            match self.next() {
                Some(Err(e)) => return Err(e),
                Some(Ok(XmlRecord::Open(data))) => {
                    let mut nested = self.nested(data);
                    nested.ignore()?;
                }
                Some(_) => continue,
                None => return Ok(()),
            }
        }
    }

    #[inline]
    pub fn element_id(&self, item: &TagData<'xml>) -> Result<Id<'xml>, Error> {
        self.context.element_id(item)
    }

    #[inline]
    pub fn attribute_id(&self, attr: &Attribute<'xml>) -> Result<Id<'xml>, Error> {
        self.context.attribute_id(attr)
    }
}

impl<'xml> Iterator for Deserializer<'_, 'xml> {
    type Item = Result<XmlRecord<'xml>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let (prefix, local) = match self.context.next() {
            Some(Ok(XmlRecord::Close { prefix, local })) => (prefix, local),
            item => return item,
        };

        if self.context.stack.len() == self.level && local == self.local && prefix == self.prefix {
            self.done = true;
            return None;
        }

        Some(Err(Error::UnexpectedState))
    }
}

pub(crate) struct Context<'xml> {
    parser: Tokenizer<'xml>,
    stack: Vec<Level<'xml>>,
    records: VecDeque<XmlRecord<'xml>>,
}

impl<'xml> Context<'xml> {
    pub(crate) fn new(input: &'xml str) -> Result<(Self, TagData<'xml>), Error> {
        let mut new = Self {
            parser: Tokenizer::from(input),
            stack: Vec::new(),
            records: VecDeque::new(),
        };

        let root = match new.next() {
            Some(result) => match result? {
                XmlRecord::Open(data) => data,
                _ => return Err(Error::UnexpectedState),
            },
            None => return Err(Error::UnexpectedEndOfStream),
        };

        Ok((new, root))
    }

    pub(crate) fn element_id(&self, item: &TagData<'xml>) -> Result<Id<'xml>, Error> {
        let ns = match (item.ns, item.prefix) {
            (_, Some(prefix)) => match self.lookup(prefix) {
                Some(ns) => ns,
                None => return Err(Error::WrongNamespace),
            },
            (Some(ns), None) => ns,
            (None, None) => self.default_ns(),
        };

        Ok(Id {
            ns,
            name: &item.key,
        })
    }

    fn attribute_id(&self, attr: &Attribute<'xml>) -> Result<Id<'xml>, Error> {
        let ns = match attr.prefix {
            Some(ns) => match self.lookup(ns) {
                Some(ns) => ns,
                None => return Err(Error::WrongNamespace),
            },
            None => self.default_ns(),
        };

        Ok(Id {
            ns,
            name: attr.local,
        })
    }

    fn default_ns(&self) -> &'xml str {
        self.stack
            .iter()
            .rev()
            .find_map(|level| level.default_ns)
            .unwrap_or("")
    }

    fn lookup(&self, prefix: &str) -> Option<&'xml str> {
        self.stack
            .iter()
            .rev()
            .find_map(|level| level.prefixes.get(prefix).copied())
    }
}

impl<'xml> Iterator for Context<'xml> {
    type Item = Result<XmlRecord<'xml>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(record) = self.records.pop_front() {
            return Some(Ok(record));
        }

        let mut current = None;
        loop {
            let token = match self.parser.next() {
                Some(v) => v,
                None => return None,
            };

            match token {
                Ok(Token::ElementStart { prefix, local, .. }) => {
                    let prefix = prefix.as_str();
                    current = Some(Level {
                        local: local.as_str(),
                        prefix: (!prefix.is_empty()).then_some(prefix),
                        default_ns: None,
                        prefixes: BTreeMap::new(),
                    });
                }
                Ok(Token::ElementEnd { end, .. }) => match end {
                    ElementEnd::Open => {
                        let level = match current {
                            Some(level) => level,
                            None => return Some(Err(Error::UnexpectedState)),
                        };

                        let data = TagData {
                            key: level.local,
                            prefix: level.prefix,
                            ns: level.default_ns,
                            level,
                        };

                        return Some(Ok(XmlRecord::Open(data)));
                    }
                    ElementEnd::Close(prefix, v) => {
                        let level = match self.stack.pop() {
                            Some(level) => level,
                            None => return Some(Err(Error::UnexpectedState)),
                        };

                        let prefix = (!prefix.is_empty()).then_some(prefix.as_str());
                        match v.as_str() == level.local && prefix == level.prefix {
                            true => {
                                return Some(Ok(XmlRecord::Close {
                                    prefix,
                                    local: level.local,
                                }))
                            }
                            false => return Some(Err(Error::UnexpectedState)),
                        }
                    }
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
                        match &mut current {
                            Some(level) => level.default_ns = Some(value.as_str()),
                            None => return Some(Err(Error::UnexpectedState)),
                        }
                    } else if prefix.as_str() == "xmlns" {
                        match &mut current {
                            Some(level) => {
                                level.prefixes.insert(local.as_str(), value.as_str());
                            }
                            None => return Some(Err(Error::UnexpectedState)),
                        }
                    } else {
                        let prefix = (!prefix.is_empty()).then_some(prefix.as_str());
                        self.records.push_back(XmlRecord::Attribute(Attribute {
                            prefix,
                            local: local.as_str(),
                            value: value.as_str(),
                        }));
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

    fn visit_str(_value: &'xml str) -> Result<Self::Value, Error> {
        unimplemented!();
    }

    fn visit_struct<'cx>(
        _deserializer: &'cx mut Deserializer<'cx, 'xml>,
    ) -> Result<Self::Value, Error> {
        unimplemented!();
    }
}

#[derive(Debug)]
pub enum XmlRecord<'xml> {
    Attribute(Attribute<'xml>),
    AttributeValue(&'xml str),
    Close {
        prefix: Option<&'xml str>,
        local: &'xml str,
    },
    Element(&'xml str),
    Open(TagData<'xml>),
}

#[derive(Debug)]
pub struct TagData<'xml> {
    key: &'xml str,
    ns: Option<&'xml str>,
    prefix: Option<&'xml str>,
    level: Level<'xml>,
}

#[derive(Debug)]
struct Level<'xml> {
    local: &'xml str,
    prefix: Option<&'xml str>,
    default_ns: Option<&'xml str>,
    prefixes: BTreeMap<&'xml str, &'xml str>,
}

#[derive(Debug)]
pub struct Attribute<'xml> {
    pub prefix: Option<&'xml str>,
    pub local: &'xml str,
    pub value: &'xml str,
}
