extern crate image;
extern crate imageproc;
extern crate rusttype;

mod annotation;
mod canvas;
mod error;

pub use annotation::*;
pub use canvas::*;
pub use error::{Error, ErrorKind};

const AA_FACTOR: u32 = 4;
const AA_FACTOR_FLOAT: f32 = 4.0;

/// A font to be used with artano's text rendering functions.
///
/// Artano is perfectly happy to accept `Font<'a>` provided that `'a` is a sufficient lifetime,
/// but I find the lifetime annoying to work with, so I'm kind of aliasing it away...
pub type Typeface = rusttype::Font<'static>;
