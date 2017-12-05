extern crate conv;
extern crate image;
extern crate imageproc;
extern crate rusttype;

mod annotation;
mod canvas;
mod draw;
mod error;

pub use annotation::*;
pub use canvas::*;
pub use error::{Result, Error, ErrorKind};

const AA_FACTOR: u32 = 3;
const AA_FACTOR_FLOAT: f32 = 3.0;

/// A font to be used with artano's text rendering functions.
///
/// Artano is perfectly happy to accept `Font<'a>` provided that `'a` is a sufficient lifetime,
/// but I find the lifetime annoying to work with, so I'm kind of aliasing it away...
pub type Typeface = rusttype::Font<'static>;

/// Create a typeface based on font data.
///
/// This data will be found in a file like `Impact.ttf` somewhere on the host system. In theory,
/// any TrueType font will work for this purpose, but some ttf files contain multiple fonts, in
/// which case this function may fail, since it won't know which one to choose. In that event,
/// it may be necessary to load the font yourself using `rusttype`.
pub fn load_typeface<R: std::io::Read>(stream: &mut R) -> Result<Typeface> {
    use rusttype::FontCollection;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|e| {
        Error::io(e, "Unable to read font stream")
    })?;

    FontCollection::from_bytes(buf).into_font().ok_or_else(|| {
        Error::font("Unable to find a font in font data")
    })
}
