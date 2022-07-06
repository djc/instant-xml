use xmlparser::{ElementEnd, Token};

use super::Error;

pub enum Element {
    Scalar(String),
    Custom,
}

pub enum EndType {
    Open,
    Close,
    Empty,
}

pub enum StackEntity {
    Open,
    Close(EndType),
    Attribute,
    Element,
}

#[derive(Default)]
pub struct ParsingState {
    pub open_tag: (bool, Option<String>),
}

impl<'a> Parse for Result<xmlparser::Token<'a>, xmlparser::Error> {
    fn get_next_element(self, state: &mut ParsingState) -> Option<(StackEntity, String)> {
        match self {
            Ok(Token::ElementStart {
                prefix: _, local, ..
            }) => {
                if state.open_tag.0 {
                    panic!("Element inside open tag");
                }

                state.open_tag.1 = Some(local.to_string());

                // This is current key
                Some((StackEntity::Open, local.to_string()))
            }
            Ok(Token::ElementEnd { end, .. }) => {
                match end {
                    ElementEnd::Open => {
                        state.open_tag.0 = true;

                        // Push this to stack
                        return Some((
                            StackEntity::Close(EndType::Open),
                            state.open_tag.1.as_ref().unwrap().to_string(),
                        ));
                    }
                    ElementEnd::Close(_, v) => {
                        state.open_tag.0 = false;

                        // Check and pop from stack
                        Some((StackEntity::Close(EndType::Close), v.to_string()))
                    }
                    ElementEnd::Empty => {
                        state.open_tag.0 = false;

                        // Do nothing
                        return Some((
                            StackEntity::Close(EndType::Empty),
                            state.open_tag.1.as_ref().unwrap().to_string(),
                        ));
                    }
                }
            }
            Ok(Token::Attribute {
                prefix: _, local, ..
            }) => {
                if state.open_tag.0 || state.open_tag.1.is_none() {
                    panic!("Element outside open tag");
                }

                Some((StackEntity::Attribute, local.to_string()))
            }
            Ok(Token::Text { text }) => {
                // This is current value
                Some((StackEntity::Element, text.to_string()))
            }
            _ => None, //todo!(),
        }
    }
    fn element_start(self, ns: Option<&str>, tag: &str) -> Result<(), Error> {
        match self {
            Ok(Token::ElementStart { prefix, local, .. }) => {
                let prefix_ns = prefix.as_str();
                let (has_prefix, expect_prefix) = (!prefix_ns.is_empty(), ns.is_some());
                if has_prefix != expect_prefix {
                    return dbg!(Err(Error::UnexpectedValue));
                }

                if has_prefix && Some(prefix_ns) != ns {
                    return dbg!(Err(Error::UnexpectedValue));
                }

                if local.as_str() != tag {
                    return dbg!(Err(Error::UnexpectedValue));
                }

                Ok(())
            }
            Ok(_) => Err(Error::UnexpectedValue),
            Err(err) => Err(err.into()),
        }
    }

    fn element_end(self, _: Option<&str>, _: &str) -> Result<(), Error> {
        match self {
            Ok(Token::ElementEnd { end, .. }) => match end {
                ElementEnd::Open => todo!(),
                ElementEnd::Close(_, _) => todo!(),
                ElementEnd::Empty => Ok(()),
            },
            Ok(_) => Err(Error::UnexpectedValue),
            Err(err) => Err(err.into()),
        }
    }
}

pub trait Parse {
    fn get_next_element(self, state: &mut ParsingState) -> Option<(StackEntity, String)>;
    fn element_start(self, ns: Option<&str>, tag: &str) -> Result<(), Error>;
    fn element_end(self, ns: Option<&str>, tag: &str) -> Result<(), Error>;
}
