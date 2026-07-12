use image::{imageops, DynamicImage, GenericImageView, ImageBuffer, Luma, Rgba, RgbaImage};
use imageproc::{drawing, edges};
use rusttype::{Font, Scale};

use crate::draw;
use crate::Result;

pub fn render_line(
    text: &str,
    y_offset: i32,
    root_position: (u32, u32),
    text_dimensions: (u32, u32),
    scale_factor: f32,
    font: &Font,
    pixels: &mut DynamicImage,
) {
    use crate::{AA_FACTOR, AA_FACTOR_FLOAT};

    // The final value in the array here is the *opacity* of the pixel. Not the transparency.
    // Apparently, this is not CSS...
    const WHITE_PIXEL: Rgba<u8> = Rgba([255, 255, 255, 255]);
    const BLACK_PIXEL: Rgba<u8> = Rgba([0, 0, 0, 255]);

    let (text_width, text_height) = text_dimensions;
    let scale = Scale::uniform(scale_factor * AA_FACTOR_FLOAT);

    // To reduce the janky jagginess of the black border around each letter, we want to render
    // the words themselves at 16x resolution and then paste that on top of the existing
    // image.
    let (x, y) = root_position;
    let x = x * AA_FACTOR;
    let y = (y as i32 + y_offset) as u32 * AA_FACTOR;

    let edge_canvas_width = text_width * AA_FACTOR;
    let mut edge_rendering =
        ImageBuffer::from_pixel(edge_canvas_width, text_height * AA_FACTOR, Luma([0u8]));
    draw::text(&mut edge_rendering, Luma([255u8]), 0, 0, scale, font, text);

    let edge_rendering = edges::canny(&edge_rendering, 255.0, 255.0);
    let edge_pixels = edge_rendering
        .pixels()
        .enumerate()
        .filter(|px| *px.1 == Luma([255u8]))
        .map(|(idx, _)| {
            let idx = idx as u32;
            let x = idx % edge_canvas_width + x;
            let y = idx / edge_canvas_width + y;
            (x, y)
        });

    let radius = (0.09 * scale_factor) as i32;
    for (x, y) in edge_pixels {
        drawing::draw_hollow_circle_mut(pixels, (x as i32, y as i32), radius, BLACK_PIXEL);
    }

    draw::text(pixels, WHITE_PIXEL, x, y, scale, font, text);
}

pub fn font_height(font: &Font, scale: Scale) -> u32 {
    use rusttype::VMetrics;

    let VMetrics {
        ascent, descent, ..
    } = font.v_metrics(scale);
    ((ascent - descent) * 1.1) as u32
}

pub fn calculate_text_width(s: &str, font: &Font, scale: Scale) -> u32 {
    // Padding of two is intended to aid in edge detection--mostly beacuse ! does not seem to
    // have an appropriate advance width.
    2 + font
        .glyphs_for(s.chars())
        .map(|glyph| glyph.scaled(scale).h_metrics().advance_width)
        .sum::<f32>() as u32
}

pub fn split_text(s: &str) -> (&str, &str) {
    let middle_index = s.len() / 2;
    let space_indexen = s.char_indices().filter(|idx| idx.1 == ' ').map(|idx| idx.0);

    let mut split_index = None;
    for idx in space_indexen {
        match split_index {
            None => split_index = Some(idx),
            Some(s_idx) => {
                // I wrote this but did not read it, so I hope it's correct.
                // Edit: of course it's correct. There's a test. Hush, dammit.
                if (middle_index as i32 - s_idx as i32).abs()
                    > (middle_index as i32 - idx as i32).abs()
                {
                    split_index = Some(idx);
                } else {
                    break;
                }
            }
        }
    }

    // The following split behavior is unbelievably egregious when splitting an annotation without
    // spaces, but let's just get this working, ok? (For those of you who don't grok what's going
    // on, this throws away the middlemost character in the event that we have not located a
    // middlemost space.)
    //
    // Edit: see "solution" below. KABOOM.
    let split_index = split_index
        .expect("Wtf, bro? You weren't supposed to call this function if you didn't have a space.");

    (&s[..split_index], &s[(split_index + 1)..])
}

/// Render an image end-to-end using the original (pre-refactor) pipeline:
/// single 3x overlay, one big Lanczos3 resize at the end. This is what
/// the `Canvas` API used to do internally; it is preserved here so the
/// diff binary (and any external user) can reproduce the pre-refactor
/// output bit-for-bit.
pub fn render(
    image_bytes: &[u8],
    annotations: &[crate::annotation::Annotation],
    font: &Font,
) -> Result<DynamicImage> {
    use crate::AA_FACTOR;

    let mut base = image::load_from_memory(image_bytes)?;
    let (width, height) = base.dimensions();

    let mut overlay =
        DynamicImage::ImageRgba8(RgbaImage::new(width * AA_FACTOR, height * AA_FACTOR));

    for annotation in annotations {
        let scale_factor = height as f32 / 10.0;
        render_text(annotation, &mut overlay, font, scale_factor, width, height);
    }

    let downsampled = imageops::resize(&overlay, width, height, imageops::FilterType::Lanczos3);
    let downsampled = DynamicImage::ImageRgba8(downsampled);
    imageops::overlay(&mut base, &downsampled, 0, 0);

    Ok(base)
}

/// The original `Annotation::render_text` body, as a free function. Kept
/// for `legacy::render` and for any user that wants the old dispatch.
fn render_text(
    annotation: &crate::annotation::Annotation,
    pixels: &mut DynamicImage,
    font: &Font,
    scale_factor: f32,
    c_width: u32,
    c_height: u32,
) {
    use crate::annotation::Position;
    use rusttype::Scale;

    let scale = Scale::uniform(scale_factor);
    let text_width = calculate_text_width(&annotation.text, font, scale);
    let font_height = font_height(font, scale);

    // We don't want text extending the full breadth of the image, but we cannot split
    // without a space.
    if (text_width as f32 * 1.2) as u32 > c_width && annotation.text.contains(' ') {
        let (left, right) = split_text(&annotation.text);
        let line_offset = font_height as i32;

        // This should be all the evidence you require that we have not selected the
        // appropriate level of abstraction.
        //
        // The most important thing to bear in mind here is that the canvas begins in the
        // TOP LEFT CORNER at 0,0.
        match annotation.position {
            Position::Top => {
                let text_width = calculate_text_width(left, font, scale);
                let position = annotation.position(c_width, c_height, text_width, font_height);
                render_line(
                    left,
                    0,
                    position,
                    (text_width, font_height),
                    scale_factor,
                    font,
                    pixels,
                );

                let text_width = calculate_text_width(right, font, scale);
                let position = annotation.position(c_width, c_height, text_width, font_height);
                render_line(
                    right,
                    line_offset,
                    position,
                    (text_width, font_height),
                    scale_factor,
                    font,
                    pixels,
                );
            }

            Position::Middle => {
                let text_width = calculate_text_width(left, font, scale);
                let position = annotation.position(c_width, c_height, text_width, font_height);
                render_line(
                    left,
                    -(line_offset / 2),
                    position,
                    (text_width, font_height),
                    scale_factor,
                    font,
                    pixels,
                );

                let text_width = calculate_text_width(right, font, scale);
                let position = annotation.position(c_width, c_height, text_width, font_height);
                render_line(
                    right,
                    line_offset / 2,
                    position,
                    (text_width, font_height),
                    scale_factor,
                    font,
                    pixels,
                );
            }

            Position::Bottom => {
                let text_width = calculate_text_width(left, font, scale);
                let position = annotation.position(c_width, c_height, text_width, font_height);
                render_line(
                    left,
                    -line_offset,
                    position,
                    (text_width, font_height),
                    scale_factor,
                    font,
                    pixels,
                );

                let text_width = calculate_text_width(right, font, scale);
                let position = annotation.position(c_width, c_height, text_width, font_height);
                render_line(
                    right,
                    0,
                    position,
                    (text_width, font_height),
                    scale_factor,
                    font,
                    pixels,
                );
            }
        }
    } else {
        let position = annotation.position(c_width, c_height, text_width, font_height);
        render_line(
            &annotation.text,
            0,
            position,
            (text_width, font_height),
            scale_factor,
            font,
            pixels,
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn split_text() {
        let input = "text to be split";
        let expected = ("text to", "be split");
        assert_eq!(expected, super::split_text(input));
    }
}
