mod annotation;
mod canvas;
mod draw;
mod error;

pub use annotation::*;
pub use canvas::*;
pub use error::Error;
use rusttype::Font;

const AA_FACTOR: u32 = 3;
const AA_FACTOR_FLOAT: f32 = 3.0;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Load a font face by name.
///
/// This will work only if your system actually has a font of the given name. Unfortunately,
/// licensing restrictions on fonts pretty much require this kind of nonsense.
pub fn load_font<'a>(name: &str) -> Result<Font<'a>> {
    use font_kit::{handle::Handle, source::SystemSource};

    let font = SystemSource::new().select_by_postscript_name(name)?;

    // I have a sneaking suspicion that only one of these paths will ever be exercised, but I have
    // no way of knowing that for sure. Thank God for documentation, right? On Windows, the Path
    // variant is definitely the one exercised.
    let font = match font {
        Handle::Path { path, font_index } => load_from_bytes(std::fs::read(path)?, font_index),
        Handle::Memory { bytes, font_index } => {
            // Sharing font data sucks.
            let font_data: Vec<_> = bytes.iter().cloned().collect();
            load_from_bytes(font_data, font_index)
        }
    };

    font.ok_or_else(|| Error::Font(name.into()))
}

fn load_from_bytes<'a>(bytes: Vec<u8>, idx: u32) -> Option<Font<'a>> {
    Font::try_from_vec_and_index(bytes, idx)
}

#[test]
fn test_impact() {
    let _font = load_font("Impact").unwrap();
}
