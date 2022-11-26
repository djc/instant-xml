use std::collections::{BTreeMap, VecDeque};

use xmlparser::{ElementEnd, Token, Tokenizer};

use crate::{Error, Id, Kind};

pub struct Deserializer<'cx, 'xml> {
    pub(crate) local: &'xml str,
    prefix: Option<&'xml str>,
    level: usize,
    done: bool,
    context: &'cx mut Context<'xml>,
}

impl<'cx, 'xml> Deserializer<'cx, 'xml> {
    pub(crate) fn new(element: Element<'xml>, context: &'cx mut Context<'xml>) -> Self {
        let level = context.stack.len();
        if !element.empty {
            context.stack.push(element.level);
        }

        Self {
            local: element.local,
            prefix: element.prefix,
            level,
            done: false,
            context,
        }
    }

    pub fn take_str(&mut self) -> Result<&'xml str, Error> {
        let (value, element) = match self.next() {
            Some(Ok(Node::AttributeValue(s))) => (s, false),
            Some(Ok(Node::Text(s))) => (s, true),
            Some(Ok(_)) => return Err(Error::ExpectedScalar),
            Some(Err(e)) => return Err(e),
            None => return Err(Error::MissingValue(&Kind::Scalar)),
        };

        if element {
            match self.next() {
                Some(Ok(_)) => {
                    return Err(Error::UnexpectedState(
                        "found element while expecting scalar",
                    ))
                }
                Some(Err(e)) => return Err(e),
                _ => {}
            }
        }

        Ok(value)
    }

    pub fn nested<'a>(&'a mut self, element: Element<'xml>) -> Deserializer<'a, 'xml>
    where
        'cx: 'a,
    {
        Deserializer::new(element, self.context)
    }

    pub fn for_attr<'a>(&'a mut self, attr: Attribute<'xml>) -> Deserializer<'a, 'xml>
    where
        'cx: 'a,
    {
        self.context
            .records
            .push_front(Node::AttributeValue(attr.value));

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
                Some(Ok(Node::Open(element))) => {
                    let mut nested = self.nested(element);
                    nested.ignore()?;
                }
                Some(_) => continue,
                None => return Ok(()),
            }
        }
    }

    pub fn push_front(&mut self, node: Node<'xml>) {
        self.context.records.push_front(node);
    }

    #[inline]
    pub fn element_id(&self, element: &Element<'xml>) -> Result<Id<'xml>, Error> {
        self.context.element_id(element)
    }

    #[inline]
    pub fn attribute_id(&self, attr: &Attribute<'xml>) -> Result<Id<'xml>, Error> {
        self.context.attribute_id(attr)
    }
}

impl<'xml> Iterator for Deserializer<'_, 'xml> {
    type Item = Result<Node<'xml>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let (prefix, local) = match self.context.next() {
            Some(Ok(Node::Close { prefix, local })) => (prefix, local),
            item => return item,
        };

        if self.context.stack.len() == self.level && local == self.local && prefix == self.prefix {
            self.done = true;
            return None;
        }

        Some(Err(Error::UnexpectedState("close element mismatch")))
    }
}

pub(crate) struct Context<'xml> {
    parser: Tokenizer<'xml>,
    stack: Vec<Level<'xml>>,
    records: VecDeque<Node<'xml>>,
}

impl<'xml> Context<'xml> {
    pub(crate) fn new(input: &'xml str) -> Result<(Self, Element<'xml>), Error> {
        let mut new = Self {
            parser: Tokenizer::from(input),
            stack: Vec::new(),
            records: VecDeque::new(),
        };

        let root = match new.next() {
            Some(result) => match result? {
                Node::Open(element) => element,
                _ => return Err(Error::UnexpectedState("first node does not open element")),
            },
            None => return Err(Error::UnexpectedEndOfStream),
        };

        Ok((new, root))
    }

    pub(crate) fn element_id(&self, element: &Element<'xml>) -> Result<Id<'xml>, Error> {
        Ok(Id {
            ns: match (element.default_ns, element.prefix) {
                (_, Some(prefix)) => self.lookup(prefix).ok_or(Error::WrongNamespace)?,
                (Some(ns), None) => ns,
                (None, None) => self.default_ns(),
            },
            name: element.local,
        })
    }

    fn attribute_id(&self, attr: &Attribute<'xml>) -> Result<Id<'xml>, Error> {
        Ok(Id {
            ns: match attr.prefix {
                Some(ns) => self.lookup(ns).ok_or(Error::WrongNamespace)?,
                None => self.default_ns(),
            },
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
    type Item = Result<Node<'xml>, Error>;

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
                            None => {
                                return Some(Err(Error::UnexpectedState(
                                    "opening element with no parent",
                                )))
                            }
                        };

                        let element = Element {
                            local: level.local,
                            prefix: level.prefix,
                            default_ns: level.default_ns,
                            level,
                            empty: false,
                        };

                        return Some(Ok(Node::Open(element)));
                    }
                    ElementEnd::Close(prefix, v) => {
                        let level = match self.stack.pop() {
                            Some(level) => level,
                            None => {
                                return Some(Err(Error::UnexpectedState(
                                    "closing element without parent",
                                )))
                            }
                        };

                        let prefix = (!prefix.is_empty()).then_some(prefix.as_str());
                        match v.as_str() == level.local && prefix == level.prefix {
                            true => {
                                return Some(Ok(Node::Close {
                                    prefix,
                                    local: level.local,
                                }))
                            }
                            false => {
                                return Some(Err(Error::UnexpectedState("close element mismatch")))
                            }
                        }
                    }
                    ElementEnd::Empty => {
                        let level = match current {
                            Some(level) => level,
                            None => {
                                return Some(Err(Error::UnexpectedState(
                                    "opening element with no parent",
                                )))
                            }
                        };

                        self.records.push_back(Node::Close {
                            prefix: level.prefix,
                            local: level.local,
                        });

                        let element = Element {
                            local: level.local,
                            prefix: level.prefix,
                            default_ns: level.default_ns,
                            level,
                            empty: true,
                        };

                        return Some(Ok(Node::Open(element)));
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
                            None => {
                                return Some(Err(Error::UnexpectedState(
                                    "attribute without element context",
                                )))
                            }
                        }
                    } else if prefix.as_str() == "xmlns" {
                        match &mut current {
                            Some(level) => {
                                level.prefixes.insert(local.as_str(), value.as_str());
                            }
                            None => {
                                return Some(Err(Error::UnexpectedState(
                                    "attribute without element context",
                                )))
                            }
                        }
                    } else {
                        let prefix = (!prefix.is_empty()).then_some(prefix.as_str());
                        self.records.push_back(Node::Attribute(Attribute {
                            prefix,
                            local: local.as_str(),
                            value: value.as_str(),
                        }));
                    }
                }
                Ok(Token::Text { text }) => {
                    return Some(Ok(Node::Text(text.as_str())));
                }
                Ok(Token::Declaration { .. }) => match self.stack.is_empty() {
                    false => return Some(Err(Error::UnexpectedToken(format!("{:?}", token)))),
                    true => {}
                },
                Ok(token) => return Some(Err(Error::UnexpectedToken(format!("{:?}", token)))),
                Err(e) => return Some(Err(Error::Parse(e))),
            }
        }
    }
}

#[derive(Debug)]
pub enum Node<'xml> {
    Attribute(Attribute<'xml>),
    AttributeValue(&'xml str),
    Close {
        prefix: Option<&'xml str>,
        local: &'xml str,
    },
    Text(&'xml str),
    Open(Element<'xml>),
}

#[derive(Debug)]
pub struct Element<'xml> {
    local: &'xml str,
    default_ns: Option<&'xml str>,
    prefix: Option<&'xml str>,
    level: Level<'xml>,
    empty: bool,
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
