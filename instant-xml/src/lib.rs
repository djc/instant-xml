use std::collections::{BTreeSet, HashMap};
use std::fmt;

use crate::xmlparser::Tokenizer;
use thiserror::Error;
pub use xmlparser;

pub use macros::{FromXml, ToXml};
use std::iter::Peekable;
use std::str::FromStr;

#[doc(hidden)]
pub mod parse;

pub trait ToXml {
    fn write_xml<W: fmt::Write>(
        &self,
        write: &mut W,
        parent_prefixes: Option<&mut BTreeSet<&str>>,
    ) -> Result<(), Error>;

    fn to_xml(&self, parent_prefixes: Option<&mut BTreeSet<&str>>) -> Result<String, Error> {
        let mut out = String::new();
        self.write_xml(&mut out, parent_prefixes)?;
        Ok(out)
    }
}

macro_rules! to_xml_for_type {
    ($typ:ty) => {
        impl ToXml for $typ {
            fn write_xml<W: fmt::Write>(
                &self,
                _write: &mut W,
                _parent_prefixes: Option<&mut BTreeSet<&str>>,
            ) -> Result<(), Error> {
                Ok(())
            }

            fn to_xml(
                &self,
                parent_prefixes: Option<&mut BTreeSet<&str>>,
            ) -> Result<String, Error> {
                let mut out = self.to_string();
                self.write_xml(&mut out, parent_prefixes)?;
                Ok(out)
            }
        }
    };
}

to_xml_for_type!(bool);
to_xml_for_type!(i8);
to_xml_for_type!(i16);
to_xml_for_type!(i32);
to_xml_for_type!(String);

pub trait FromXml<'xml>: Sized {
    fn from_xml<'a>(
        input: &'a str,
        iter: Option<&mut Peekable<Tokenizer<'a>>>,
        scalar_type_value: Option<String>,
    ) -> Result<Self, Error>;
}

impl<'xml> FromXml<'xml> for bool {
    fn from_xml<'a>(
        _input: &'a str,
        _parent_iter: Option<&mut Peekable<Tokenizer<'a>>>,
        scalar_type_value: Option<String>,
    ) -> Result<Self, Error> {
        match scalar_type_value {
            Some(v) => Ok(bool::from_str(v.as_str()).expect("Proper bool value")),
            _ => panic!("missing value"),
        }
    }
}

pub trait FromXmlOwned: for<'xml> FromXml<'xml> {}

#[allow(dead_code)]
struct State<'a> {
    prefix: HashMap<&'a str, &'a str>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("format: {0}")]
    Format(#[from] fmt::Error),
    #[error("parse: {0}")]
    Parse(#[from] xmlparser::Error),
    #[error("unexpected end of stream")]
    UnexpectedEndOfStream,
    #[error("unexpected value")]
    UnexpectedValue,
}
