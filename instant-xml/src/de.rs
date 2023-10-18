use std::borrow::Cow;
use std::collections::{BTreeMap, VecDeque};

use xmlparser::{ElementEnd, Token, Tokenizer};

use crate::impls::{decode, CowStrAccumulator};
use crate::{Error, Id};

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

    pub fn take_str(&mut self) -> Result<Option<&'xml str>, Error> {
        loop {
            match self.next() {
                Some(Ok(Node::AttributeValue(s))) => return Ok(Some(s)),
                Some(Ok(Node::Text(s))) => return Ok(Some(s)),
                Some(Ok(Node::Attribute(_))) => continue,
                Some(Ok(node)) => return Err(Error::ExpectedScalar(format!("{node:?}"))),
                Some(Err(e)) => return Err(e),
                None => return Ok(None),
            }
        }
    }

    pub fn nested<'a>(&'a mut self, element: Element<'xml>) -> Deserializer<'a, 'xml>
    where
        'cx: 'a,
    {
        Deserializer::new(element, self.context)
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

    pub fn for_node<'a>(&'a mut self, node: Node<'xml>) -> Deserializer<'a, 'xml>
    where
        'cx: 'a,
    {
        self.context.records.push_front(node);
        Deserializer {
            local: self.local,
            prefix: self.prefix,
            level: self.level,
            done: self.done,
            context: self.context,
        }
    }

    pub fn parent(&self) -> Id<'xml> {
        Id {
            ns: match self.prefix {
                Some(ns) => self.context.lookup(ns).unwrap(),
                None => self.context.default_ns(),
            },
            name: self.local,
        }
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
                (_, Some(prefix)) => match element.level.prefixes.get(prefix) {
                    Some(ns) => ns,
                    None => match self.lookup(prefix) {
                        Some(ns) => ns,
                        None => return Err(Error::UnknownPrefix(prefix.to_owned())),
                    },
                },
                (Some(ns), None) => ns,
                (None, None) => self.default_ns(),
            },
            name: element.local,
        })
    }

    fn attribute_id(&self, attr: &Attribute<'xml>) -> Result<Id<'xml>, Error> {
        Ok(Id {
            ns: match attr.prefix {
                Some(ns) => self
                    .lookup(ns)
                    .ok_or_else(|| Error::UnknownPrefix(ns.to_owned()))?,
                None => "",
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
                        prefix: match prefix.is_empty() {
                            true => None,
                            false => Some(prefix),
                        },
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

                        let prefix = match prefix.is_empty() {
                            true => None,
                            false => Some(prefix.as_str()),
                        };

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
                        self.records.push_back(Node::Attribute(Attribute {
                            prefix: match prefix.is_empty() {
                                true => None,
                                false => Some(prefix.as_str()),
                            },
                            local: local.as_str(),
                            value: value.as_str(),
                        }));
                    }
                }
                Ok(Token::Text { text }) => {
                    return Some(Ok(Node::Text(text.as_str())));
                }
                Ok(Token::Cdata { text, .. }) => {
                    return Some(Ok(Node::Text(text.as_str())));
                }
                Ok(Token::Declaration { .. }) => match self.stack.is_empty() {
                    false => return Some(Err(Error::UnexpectedToken(format!("{token:?}")))),
                    true => {}
                },
                Ok(token) => return Some(Err(Error::UnexpectedToken(format!("{token:?}")))),
                Err(e) => return Some(Err(Error::Parse(e))),
            }
        }
    }
}

pub fn borrow_cow_str<'a, 'xml: 'a>(
    into: &mut CowStrAccumulator<'xml, 'a>,
    _: &'static str,
    deserializer: &mut Deserializer<'_, 'xml>,
) -> Result<(), Error> {
    if into.inner.is_some() {
        return Err(Error::DuplicateValue);
    }

    let value = match deserializer.take_str()? {
        Some(value) => value,
        None => return Ok(()),
    };

    into.inner = Some(decode(value)?);
    deserializer.ignore()?;
    Ok(())
}

pub fn borrow_cow_slice_u8<'xml>(
    into: &mut Option<Cow<'xml, [u8]>>,
    _: &'static str,
    deserializer: &mut Deserializer<'_, 'xml>,
) -> Result<(), Error> {
    if into.is_some() {
        return Err(Error::DuplicateValue);
    }

    if let Some(value) = deserializer.take_str()? {
        *into = Some(match decode(value)? {
            Cow::Borrowed(v) => Cow::Borrowed(v.as_bytes()),
            Cow::Owned(v) => Cow::Owned(v.into_bytes()),
        });
    }

    deserializer.ignore()?;
    Ok(())
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
