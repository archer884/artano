use image::{DynamicImage, ImageBuffer, Luma, Rgba};
use imageproc::{drawing, edges};
use imageproc::rect::Rect;
use rusttype::{Font, Scale};

#[derive(Debug)]
pub struct Annotation {
    pub text: String,
    pub position: Position,
}

#[derive(Debug, Copy, Clone)]
pub enum Position {
    Top,
    Middle,
    Bottom,
}

impl Annotation {
    pub fn top<T: Into<String>>(text: T) -> Annotation {
        Annotation {
            text: text.into(),
            position: Position::Top,
        }
    }

    pub fn middle<T: Into<String>>(text: T) -> Annotation {
        Annotation {
            text: text.into(),
            position: Position::Middle,
        }
    }

    pub fn bottom<T: Into<String>>(text: T) -> Annotation {
        Annotation {
            text: text.into(),
            position: Position::Bottom,
        }
    }

    fn position(&self, width: u32, height: u32, text_width: u32, text_height: u32) -> (u32, u32) {
        mod position {
            pub fn top(width: u32, _height: u32, text_width: u32, text_height: u32) -> (u32, u32) {
                let x = (width / 2) - (text_width / 2);
                let y = {
                    let text_height = text_height as f32;
                    (text_height * 0.2) as u32
                };

                (x, y)
            }

            pub fn middle(width: u32, height: u32, text_width: u32, text_height: u32) -> (u32, u32) {
                let x = (width / 2) - (text_width / 2);
                let y = (height / 2) - (text_height / 2);

                (x, y)
            }

            pub fn bottom(width: u32, height: u32, text_width: u32, text_height: u32) -> (u32, u32) {
                let x = (width / 2) - (text_width / 2);
                let y = {
                    let height = height as f32;
                    let text_height = text_height as f32;
                    (height - (text_height * 1.2)) as u32
                };

                (x, y)
            }
        }

        match self.position {
            Position::Top => position::top(width, height, text_width, text_height),
            Position::Middle => position::middle(width, height, text_width, text_height),
            Position::Bottom => position::bottom(width, height, text_width, text_height),
        }
    }

    pub fn render_text<'a>(&self, 
                       pixels: &'a mut DynamicImage,
                       font: &'a Font<'a>,
                       scale_factor: f32,
                       c_width: u32,
                       c_height: u32) {

        use AA_FACTOR;
        use AA_FACTOR_FLOAT;

        // The final value in the array here is the *opacity* of the pixel. Not the transparency.
        // Apparently, this is not CSS...
        let white_pixel = Rgba([255, 255, 255, 255]);
        let black_pixel = Rgba([0, 0, 0, 255]);
        
        let scale = Scale::uniform(scale_factor);
        let scale_aa = Scale::uniform(scale_factor * AA_FACTOR_FLOAT);
        let (text_width, text_height) = text_size(&self.text, font, scale);

        // To reduce the janky jagginess of the black border around each letter, we want to render the 
        // words themselves at 16x resolution and then paste that on top of the existing image.
        let (x, y) = self.position(c_width, c_height, text_width, text_height);
        let x = x * AA_FACTOR;
        let y = y * AA_FACTOR;

        let mut edge_rendering = ImageBuffer::from_pixel(text_width * AA_FACTOR, text_height * AA_FACTOR, Luma([0u8]));
        drawing::draw_text_with_font_mut(&mut edge_rendering, Luma([255u8]), 0, 0, scale_aa, &font, &self.text);

        let edge_rendering = edges::canny(&edge_rendering, 255.0, 255.0);
        let edge_pixels = edge_rendering.pixels().enumerate()
            .filter(|&(_, &px)| Luma([255u8]) == px)
            .map(|(idx, _)| {
                let idx = idx as u32;
                let x = idx % (text_width * AA_FACTOR) + x;
                let y = idx / (text_width * AA_FACTOR) + y;
                (x, y)
            });

        // I wonder how long this ends up taking. Seems like this would just have to be the slowest
        // part of the process. Would be great to parallelize this somehow, but it would probably be
        // pretty difficult to allow multiple mutable aliases, too...
        let rect_size = (0.1 * scale_factor * 2.2) as u32;
        let offset = (rect_size / 2) as i32;
        for (x, y) in edge_pixels {
            let rect = Rect::at(x as i32 - offset, y as i32 - offset).of_size(rect_size, rect_size);
            drawing::draw_hollow_rect_mut(pixels, rect, black_pixel);
        }

        drawing::draw_text_with_font_mut(pixels, white_pixel, x, y, scale_aa, &font, &self.text);

    }
}

/// Calculate the dimensions of the bounding box for a given string, font, and scale.
///
/// This works by summing the "advance width" of each glyph in the text, entirely ignoring
/// kerning as each character is considered in isolation. Because this is used just to center
/// text in the image, it's close enough for government work.
fn text_size<'a>(s: &'a str, font: &'a Font<'a>, scale: Scale) -> (u32, u32) {
    use rusttype::VMetrics;

    let text_width = font.glyphs_for(s.chars())
        .map(|glyph| glyph.scaled(scale).h_metrics().advance_width)
        .sum::<f32>();

    // The "v-metrics" for any given letter in a font are the same for a given scale, so we don't
    // need to check this for each glyph.
    let text_height = {
        let VMetrics { ascent, descent, ..} = font.v_metrics(scale);
        (ascent - descent) as u32
    };

    // I know I'm truncating the length and this is probably wrong, but it's not wrong by enough
    // to be noticeable when you print it to an image.
    //
    // The padding you see below is added to aid in edge detection, specifically because the
    // exclamation point doesn't seem to have enough advance width. -.-
    (text_width as u32 + 2, text_height)
}
