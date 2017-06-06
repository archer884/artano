use image;
use std::borrow::Cow;
use std::error;
use std::result;

pub type Result<T> = result::Result<T, Error>;

pub type Cause = Option<Box<error::Error>>;

pub struct Error {
    kind: ErrorKind,
    description: Cow<'static, str>,
    cause: Cause,
}

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
