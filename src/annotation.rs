use rusttype::{Font, Scale, VMetrics};

#[derive(Clone, Debug)]
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
    pub fn top(text: impl Into<String>) -> Annotation {
        Annotation {
            text: text.into(),
            position: Position::Top,
        }
    }

    pub fn middle(text: impl Into<String>) -> Annotation {
        Annotation {
            text: text.into(),
            position: Position::Middle,
        }
    }

    pub fn bottom(text: impl Into<String>) -> Annotation {
        Annotation {
            text: text.into(),
            position: Position::Bottom,
        }
    }

    pub fn position(
        &self,
        width: u32,
        height: u32,
        text_width: u32,
        text_height: u32,
    ) -> (u32, u32) {
        mod position {
            pub fn top(width: u32, _height: u32, text_width: u32, text_height: u32) -> (u32, u32) {
                let x = (width / 2).saturating_sub(text_width / 2);
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
                let x = (width / 2).saturating_sub(text_width / 2);
                let y = (height / 2) - (text_height / 2);

                (x, y)
            }

            pub fn bottom(
                width: u32,
                height: u32,
                text_width: u32,
                text_height: u32,
            ) -> (u32, u32) {
                let x = (width / 2).saturating_sub(text_width / 2);
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

    /// Compute placement for the annotation's text without rendering.
    /// Returns one `PlacedLine` per visual line (1 or 2, depending on
    /// whether the text is wide enough to require splitting).
    pub fn layout(
        &self,
        font: &Font,
        scale_factor: f32,
        c_width: u32,
        c_height: u32,
    ) -> Vec<PlacedLine> {
        let scale = Scale::uniform(scale_factor);
        let text_width = calculate_text_width(&self.text, font, scale);
        let font_height = font_height(font, scale);

        // We don't want text extending the full breadth of the image, but we cannot split
        // without a space.
        if (text_width as f32 * 1.2) as u32 > c_width && self.text.contains(' ') {
            let (left, right) = split_text(&self.text);
            let line_offset = font_height as i32;

            // This should be all the evidence you require that we have not selected the
            // appropriate level of abstraction.
            //
            // The most important thing to bear in mind here is that the canvas begins in the
            // TOP LEFT CORNER at 0,0.
            match self.position {
                Position::Top => {
                    let left_width = calculate_text_width(left, font, scale);
                    let left_pos = self.position(c_width, c_height, left_width, font_height);

                    let right_width = calculate_text_width(right, font, scale);
                    let right_pos = self.position(c_width, c_height, right_width, font_height);

                    vec![
                        PlacedLine {
                            text: left.to_string(),
                            x: left_pos.0,
                            y: left_pos.1,
                            width: left_width,
                            height: font_height,
                        },
                        PlacedLine {
                            text: right.to_string(),
                            x: right_pos.0,
                            y: (right_pos.1 as i32 + line_offset).max(0) as u32,
                            width: right_width,
                            height: font_height,
                        },
                    ]
                }

                Position::Middle => {
                    let left_width = calculate_text_width(left, font, scale);
                    let left_pos = self.position(c_width, c_height, left_width, font_height);

                    let right_width = calculate_text_width(right, font, scale);
                    let right_pos = self.position(c_width, c_height, right_width, font_height);

                    vec![
                        PlacedLine {
                            text: left.to_string(),
                            x: left_pos.0,
                            y: (left_pos.1 as i32 - line_offset / 2).max(0) as u32,
                            width: left_width,
                            height: font_height,
                        },
                        PlacedLine {
                            text: right.to_string(),
                            x: right_pos.0,
                            y: (right_pos.1 as i32 + line_offset / 2).max(0) as u32,
                            width: right_width,
                            height: font_height,
                        },
                    ]
                }

                Position::Bottom => {
                    let left_width = calculate_text_width(left, font, scale);
                    let left_pos = self.position(c_width, c_height, left_width, font_height);

                    let right_width = calculate_text_width(right, font, scale);
                    let right_pos = self.position(c_width, c_height, right_width, font_height);

                    vec![
                        PlacedLine {
                            text: left.to_string(),
                            x: left_pos.0,
                            y: (left_pos.1 as i32 - line_offset).max(0) as u32,
                            width: left_width,
                            height: font_height,
                        },
                        PlacedLine {
                            text: right.to_string(),
                            x: right_pos.0,
                            y: right_pos.1,
                            width: right_width,
                            height: font_height,
                        },
                    ]
                }
            }
        } else {
            let pos = self.position(c_width, c_height, text_width, font_height);
            vec![PlacedLine {
                text: self.text.clone(),
                x: pos.0,
                y: pos.1,
                width: text_width,
                height: font_height,
            }]
        }
    }
}

/// Where a single line of caption text should be drawn, in base image
/// coordinates. Returned by `Annotation::layout` and consumed by
/// `Canvas::add_annotation`.
#[derive(Clone, Debug)]
pub struct PlacedLine {
    pub text: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

fn font_height(font: &Font, scale: Scale) -> u32 {
    let VMetrics {
        ascent, descent, ..
    } = font.v_metrics(scale);
    ((ascent - descent) * 1.1) as u32
}

fn calculate_text_width(s: &str, font: &Font, scale: Scale) -> u32 {
    // Padding of two is intended to aid in edge detection--mostly beacuse ! does not seem to
    // have an appropriate advance width.
    2 + font
        .glyphs_for(s.chars())
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

#[cfg(test)]
mod tests {
    #[test]
    fn split_text() {
        let input = "text to be split";
        let expected = ("text to", "be split");
        assert_eq!(expected, super::split_text(input));
    }
}
