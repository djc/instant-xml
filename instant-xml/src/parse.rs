use xmlparser::{ElementEnd, Token};

use super::Error;

impl<'a> Parse for Option<Result<xmlparser::Token<'a>, xmlparser::Error>> {
    fn element_start(self, ns: Option<&str>, tag: &str) -> Result<(), Error> {
        match self {
            Some(Ok(Token::ElementStart { prefix, local, .. })) => {
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
            Some(Ok(_)) => Err(Error::UnexpectedValue),
            Some(Err(err)) => Err(err.into()),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }

    fn element_end(self, _: Option<&str>, _: &str) -> Result<(), Error> {
        match self {
            Some(Ok(Token::ElementEnd { end, .. })) => match end {
                ElementEnd::Open => todo!(),
                ElementEnd::Close(_, _) => todo!(),
                ElementEnd::Empty => return Ok(()),
            },
            Some(Ok(_)) => Err(Error::UnexpectedValue),
            Some(Err(err)) => Err(err.into()),
            None => Err(Error::UnexpectedEndOfStream),
        }
    }
}

pub trait Parse {
    fn element_start(self, ns: Option<&str>, tag: &str) -> Result<(), Error>;
    fn element_end(self, ns: Option<&str>, tag: &str) -> Result<(), Error>;
}
