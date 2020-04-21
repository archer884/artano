use font_kit::error::SelectionError;
use std::{error, fmt, io};

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    Image(image::ImageError),
    FontSelection(SelectionError),
    Font(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IO(e) => e.fmt(f),
            Error::Image(e) => e.fmt(f),
            Error::FontSelection(e) => e.fmt(f),
            Error::Font(name) => write!(f, "Font {:?} not found", name),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::IO(e) => Some(e),
            Error::Image(e) => Some(e),
            Error::FontSelection(e) => Some(e),

            _ => None,
        }
    }
}

impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Self {
        Error::Image(e)
    }
}

impl From<SelectionError> for Error {
    fn from(e: SelectionError) -> Self {
        Error::FontSelection(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(e)
    }
}
