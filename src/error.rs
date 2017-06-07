use image;
use std::borrow::Cow;
use std::error;
use std::fmt;
use std::result;

pub type Result<T> = result::Result<T, Error>;
pub type Cause = Option<Box<error::Error>>;
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
    pub fn io<D: Into<Description>, E: error::Error + 'static>(e: E, description: D) -> Error {
        Error {
            kind: ErrorKind::IO,
            description: description.into(),
            cause: Some(Box::new(e)),
        }
    }

    pub fn font<D: Into<Description>>(description: D) -> Error {
        Error {
            kind: ErrorKind::Font,
            description: description.into(),
            cause: None,
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        &self.description
    }

    fn cause(&self) -> Option<&error::Error> {
        match self.cause {
            None => None,
            Some(ref error) => Some(error.as_ref()),
        }
    }
}
