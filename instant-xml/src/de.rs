use std::borrow::Cow;
use std::collections::{BTreeMap, VecDeque};
use std::str::{self, FromStr};

use xmlparser::{ElementEnd, Token, Tokenizer};

use crate::impls::CowStrAccumulator;
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

    pub fn take_str(&mut self) -> Result<Option<Cow<'xml, str>>, Error> {
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
        // The prefix xml is by definition bound to the namespace
        // name http://www.w3.org/XML/1998/namespace
        // See https://www.w3.org/TR/xml-names/#ns-decl
        if prefix == "xml" {
            return Some("http://www.w3.org/XML/1998/namespace");
        }

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
                        let value = match decode(value.as_str()) {
                            Ok(value) => value,
                            Err(e) => return Some(Err(e)),
                        };

                        self.records.push_back(Node::Attribute(Attribute {
                            prefix: match prefix.is_empty() {
                                true => None,
                                false => Some(prefix.as_str()),
                            },
                            local: local.as_str(),
                            value,
                        }));
                    }
                }
                Ok(Token::Text { text }) => {
                    return Some(decode(text.as_str()).map(Node::Text));
                }
                Ok(Token::Cdata { text, .. }) => {
                    return Some(Ok(Node::Text(Cow::Borrowed(text.as_str()))));
                }
                Ok(Token::Declaration { .. }) => match self.stack.is_empty() {
                    false => return Some(Err(Error::UnexpectedToken(format!("{token:?}")))),
                    true => {}
                },
                Ok(Token::Comment { .. }) => continue,
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

    match deserializer.take_str()? {
        Some(value) => into.inner = Some(value),
        None => return Ok(()),
    };

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
        *into = Some(match value {
            Cow::Borrowed(v) => Cow::Borrowed(v.as_bytes()),
            Cow::Owned(v) => Cow::Owned(v.into_bytes()),
        });
    }

    deserializer.ignore()?;
    Ok(())
}

fn decode(input: &str) -> Result<Cow<'_, str>, Error> {
    let mut result = String::with_capacity(input.len());
    let (mut state, mut last_end) = (DecodeState::Normal, 0);
    for (i, &b) in input.as_bytes().iter().enumerate() {
        // use a state machine to find entities
        state = match (state, b) {
            (DecodeState::Normal, b'&') => DecodeState::Entity([0; 6], 0),
            (DecodeState::Normal, _) => DecodeState::Normal,
            (DecodeState::Entity(chars, len), b';') => {
                let decoded = match &chars[..len] {
                    [b'a', b'm', b'p'] => '&',
                    [b'a', b'p', b'o', b's'] => '\'',
                    [b'g', b't'] => '>',
                    [b'l', b't'] => '<',
                    [b'q', b'u', b'o', b't'] => '"',
                    [b'#', b'x' | b'X', hex @ ..] => {
                        // Hexadecimal character reference e.g. "&#x007c;" -> '|'
                        str::from_utf8(hex)
                            .ok()
                            .and_then(|hex_str| u32::from_str_radix(hex_str, 16).ok())
                            .and_then(char::from_u32)
                            .filter(valid_xml_character)
                            .ok_or_else(|| {
                                Error::InvalidEntity(
                                    String::from_utf8_lossy(&chars[..len]).into_owned(),
                                )
                            })?
                    }
                    [b'#', decimal @ ..] => {
                        // Decimal character reference e.g. "&#1234;" -> 'Ӓ'
                        str::from_utf8(decimal)
                            .ok()
                            .and_then(|decimal_str| u32::from_str(decimal_str).ok())
                            .and_then(char::from_u32)
                            .filter(valid_xml_character)
                            .ok_or_else(|| {
                                Error::InvalidEntity(
                                    String::from_utf8_lossy(&chars[..len]).into_owned(),
                                )
                            })?
                    }
                    _ => {
                        return Err(Error::InvalidEntity(
                            String::from_utf8_lossy(&chars[..len]).into_owned(),
                        ))
                    }
                };

                let start = i - (len + 1); // current position - (length of entity characters + 1 for '&')
                if last_end < start {
                    // Unwrap should be safe: `last_end` and `start` must be at character boundaries.
                    result.push_str(input.get(last_end..start).unwrap());
                }

                last_end = i + 1;
                result.push(decoded);
                DecodeState::Normal
            }
            (DecodeState::Entity(mut chars, len), b) => {
                if len >= 6 {
                    let mut bytes = Vec::with_capacity(7);
                    bytes.extend(&chars[..len]);
                    bytes.push(b);
                    return Err(Error::InvalidEntity(
                        String::from_utf8_lossy(&bytes).into_owned(),
                    ));
                }

                chars[len] = b;
                DecodeState::Entity(chars, len + 1)
            }
        };
    }

    // Unterminated entity (& without ;) at end of input
    if let DecodeState::Entity(chars, len) = state {
        return Err(Error::InvalidEntity(
            String::from_utf8_lossy(&chars[..len]).into_owned(),
        ));
    }

    Ok(match result.is_empty() {
        true => Cow::Borrowed(input),
        false => {
            // Unwrap should be safe: `last_end` and `input.len()` must be at character boundaries.
            result.push_str(input.get(last_end..input.len()).unwrap());
            Cow::Owned(result)
        }
    })
}

#[derive(Debug)]
enum DecodeState {
    Normal,
    Entity([u8; 6], usize),
}

/// Valid character ranges per https://www.w3.org/TR/xml/#NT-Char
fn valid_xml_character(c: &char) -> bool {
    matches!(c, '\u{9}' | '\u{A}' | '\u{D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')
}

#[derive(Debug)]
pub enum Node<'xml> {
    Attribute(Attribute<'xml>),
    AttributeValue(Cow<'xml, str>),
    Close {
        prefix: Option<&'xml str>,
        local: &'xml str,
    },
    Text(Cow<'xml, str>),
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
    pub value: Cow<'xml, str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode() {
        decode_ok("foo", "foo");
        decode_ok("foo &amp; bar", "foo & bar");
        decode_ok("foo &lt; bar", "foo < bar");
        decode_ok("foo &gt; bar", "foo > bar");
        decode_ok("foo &quot; bar", "foo \" bar");
        decode_ok("foo &apos; bar", "foo ' bar");
        decode_ok("foo &amp;lt; bar", "foo &lt; bar");
        decode_ok("&amp; foo", "& foo");
        decode_ok("foo &amp;", "foo &");
        decode_ok("cbdtéda&amp;sü", "cbdtéda&sü");
        // Decimal character references
        decode_ok("&#1234;", "Ӓ");
        decode_ok("foo &#9; bar", "foo \t bar");
        decode_ok("foo &#124; bar", "foo | bar");
        decode_ok("foo &#1234; bar", "foo Ӓ bar");
        // Hexadecimal character references
        decode_ok("&#xc4;", "Ä");
        decode_ok("&#x00c4;", "Ä");
        decode_ok("foo &#x9; bar", "foo \t bar");
        decode_ok("foo &#x007c; bar", "foo | bar");
        decode_ok("foo &#xc4; bar", "foo Ä bar");
        decode_ok("foo &#x00c4; bar", "foo Ä bar");
        decode_ok("foo &#x10de; bar", "foo პ bar");

        decode_err("&");
        decode_err("&#");
        decode_err("&#;");
        decode_err("foo&");
        decode_err("&bar");
        decode_err("&foo;");
        decode_err("&foobar;");
        decode_err("cbdtéd&ampü");
    }

    fn decode_ok(input: &str, expected: &'static str) {
        assert_eq!(super::decode(input).unwrap(), expected, "{input:?}");
    }

    fn decode_err(input: &str) {
        assert!(super::decode(input).is_err(), "{input:?}");
    }
}
