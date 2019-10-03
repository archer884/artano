use std::borrow::Cow;
use std::error;
use std::fmt;
use std::result;

pub type Result<T, E = Error> = result::Result<T, E>;
pub type Cause = Option<Box<dyn error::Error + 'static>>;
pub type Description = Cow<'static, str>;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    description: Description,
    cause: Cause,
}

#[derive(Debug)]
pub enum ErrorKind {
    Font,
    ImageFormat,
    IO,
}

impl Error {
    pub fn io<D, E>(e: E, description: D) -> Error
    where
        D: Into<Description>,
        E: error::Error + 'static,
    {
        Error {
            kind: ErrorKind::IO,
            description: description.into(),
            cause: Some(Box::new(e)),
        }
    }

    pub fn font<D, E>(e: E, description: D) -> Error
    where
        D: Into<Description>,
        E: error::Error + 'static,
    {
        Error {
            kind: ErrorKind::Font,
            description: description.into(),
            cause: Some(Box::new(e)),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.cause {
            None => None,
            Some(ref error) => Some(error.as_ref()),
        }
    }
}

impl From<image::ImageError> for Error {
    fn from(error: image::ImageError) -> Error {
        Error {
            kind: ErrorKind::ImageFormat,
            description: Cow::from("Sorry, we couldn't read this image"),
            cause: Some(Box::new(error)),
        }
    }
}
