use draw;
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

            pub fn middle(
                width: u32,
                height: u32,
                text_width: u32,
                text_height: u32,
            ) -> (u32, u32) {
                let x = (width / 2) - (text_width / 2);
                let y = (height / 2) - (text_height / 2);

                (x, y)
            }

            pub fn bottom(
                width: u32,
                height: u32,
                text_width: u32,
                text_height: u32,
            ) -> (u32, u32) {
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

    pub fn render_text(
        &self,
        pixels: &mut DynamicImage,
        font: &Font,
        scale_factor: f32,
        c_width: u32,
        c_height: u32,
    ) {
        let scale = Scale::uniform(scale_factor);
        let text_width = calculate_text_width(&self.text, font, scale);
        let font_height = font_height(font, scale);

        // We don't want text extending the full breadth of the image, but we cannot split
        // without a space.
        if (text_width as f32 * 1.2) as u32 > c_width && self.text.contains(" ") {
            let (left, right) = split_text(&self.text);
            let line_offset = font_height as i32;

            // This should be all the evidence you require that we have not selected the
            // appropriate level of abstraction.
            //
            // The most important thing to bear in mind here is that the canvas begins in the 
            // TOP LEFT CORNER at 0,0.
            match self.position {
                Position::Top => {
                    let text_width = calculate_text_width(left, font, scale);
                    let position = self.position(c_width, c_height, text_width, font_height);
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
                    let position = self.position(c_width, c_height, text_width, font_height);
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
                    let position = self.position(c_width, c_height, text_width, font_height);
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
                    let position = self.position(c_width, c_height, text_width, font_height);
                    render_line(
                        right,
                        (line_offset / 2),
                        position,
                        (text_width, font_height),
                        scale_factor,
                        font,
                        pixels,
                    );
                }

                Position::Bottom => {
                    let text_width = calculate_text_width(left, font, scale);
                    let position = self.position(c_width, c_height, text_width, font_height);
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
                    let position = self.position(c_width, c_height, text_width, font_height);
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
            let position = self.position(c_width, c_height, text_width, font_height);
            render_line(
                &self.text,
                0,
                position,
                (text_width, font_height),
                scale_factor,
                font,
                pixels,
            );
        }
    }
}

fn render_line(
    text: &str,
    y_offset: i32,
    root_position: (u32, u32),
    text_dimensions: (u32, u32),
    scale_factor: f32,
    font: &Font,
    pixels: &mut DynamicImage,
) {
    use AA_FACTOR;
    use AA_FACTOR_FLOAT;

    // The final value in the array here is the *opacity* of the pixel. Not the transparency.
    // Apparently, this is not CSS...
    const WHITE_PIXEL: Rgba<u8> = Rgba { data: [255, 255, 255, 255] };
    const BLACK_PIXEL: Rgba<u8> = Rgba { data: [0, 0, 0, 255] };

    let (text_width, text_height) = text_dimensions;
    let scale = Scale::uniform(scale_factor * AA_FACTOR_FLOAT);

    // To reduce the janky jagginess of the black border around each letter, we want to render
    // the words themselves at 16x resolution and then paste that on top of the existing
    // image.
    let (x, y) = root_position;
    let x = x * AA_FACTOR;
    let y = (y as i32 + y_offset) as u32 * AA_FACTOR;

    let edge_canvas_width = text_width * AA_FACTOR;
    let mut edge_rendering = ImageBuffer::from_pixel(
        edge_canvas_width,
        text_height * AA_FACTOR,
        Luma([0u8]),
    );
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

    let rect_size = (0.1 * scale_factor * 2.2) as u32;
    let offset = (rect_size / 2) as i32;
    for (x, y) in edge_pixels {
        let rect = Rect::at(x as i32 - offset, y as i32 - offset).of_size(rect_size, rect_size);
        drawing::draw_hollow_rect_mut(pixels, rect, BLACK_PIXEL);
    }

    draw::text(pixels, WHITE_PIXEL, x, y, scale, font, text);
}

fn font_height(font: &Font, scale: Scale) -> u32 {
    use rusttype::VMetrics;

    let VMetrics { ascent, descent, .. } = font.v_metrics(scale);
    (ascent - descent) as u32
}

fn calculate_text_width(s: &str, font: &Font, scale: Scale) -> u32 {
    // Padding of two is intended to aid in edge detection--mostly beacuse ! does not seem to
    // have an appropriate advance width.
    2 +
        font.glyphs_for(s.chars())
            .map(|glyph| glyph.scaled(scale).h_metrics().advance_width)
            .sum::<f32>() as u32
}

fn split_text(s: &str) -> (&str, &str) {
    let middle_index = s.len() / 2;
    let space_indexen = s.char_indices().filter(|idx| idx.1 == ' ').map(|idx| idx.0);

    let mut split_index = None;
    for idx in space_indexen {
        match split_index {
            None => split_index = Some(idx),
            Some(s_idx) => {
                // I wrote this but did not read it, so I hope it's correct.
                // Edit: of course it's correct. There's a test. Hush, dammit.
                if (middle_index as i32 - s_idx as i32).abs() >
                    (middle_index as i32 - idx as i32).abs()
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
    let split_index = split_index.expect(
        "Wtf, bro?You weren't supposed to call this function if you didn't have a space.",
    );
    (&s[..split_index], &s[(split_index + 1)..])
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
