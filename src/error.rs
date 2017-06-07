use image;
use std::borrow::Cow;
use std::error;
use std::fmt;
use std::result;

pub type Result<T> = result::Result<T, Error>;

pub type Cause = Option<Box<error::Error>>;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    description: Cow<'static, str>,
    cause: Cause,
}

#[derive(Debug)]
pub enum ErrorKind {
    /// Couldn't interpret this data as an image.
    ImageFormat,
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
