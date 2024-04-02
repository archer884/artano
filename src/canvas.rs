use std::io;

use image::{self, imageops, DynamicImage, GenericImageView, ImageFormat, RgbaImage};
use rusttype::Font;

use crate::{annotation::Annotation, Result, AA_FACTOR};

pub struct Canvas {
    base: DynamicImage,
    overlay: DynamicImage,
    width: u32,
    height: u32,
}

impl Canvas {
    /// Creates a new canvas based on a buffer of bytes.
    ///
    /// A canvas consists of both a base layer and an upscaled annotation layer (at 3x the
    /// original resolution? Depends on how we count that, I guess...). Text is rendered first
    /// at this upscaled size and then downsampled onto the background.
    pub fn read_from_buffer(buf: &[u8]) -> Result<Canvas> {
        let base = image::load_from_memory(buf)?;
        let (width, height) = base.dimensions();
        Ok(Canvas {
            base,
            overlay: DynamicImage::ImageRgba8(RgbaImage::new(
                width * AA_FACTOR,
                height * AA_FACTOR,
            )),
            width,
            height,
        })
    }

    /// Adds an annotation to the canvas.
    ///
    /// This renders the annotation to the upscaled layer of the canvas that will eventually be
    /// overlaid onto the canvas proper. Text is laid out and drawn at this stage, meaning each
    /// annotation is individually rendered.
    pub fn add_annotation<'a>(
        &mut self,
        annotation: &Annotation,
        font: &Font<'a>,
        scale_multiplier: f32,
    ) {
        // Font scale is, in fact, the height in pixels of each glyph. Here we set that to be
        // one tenth the height of the image itself modified by the scale multiplier provided
        // by the user. The multiplier serves to allow us to shrink or expand text to fit images
        // that are either too tall or too small for a given annotation.
        let scale = (self.height as f32 / 10.0) * scale_multiplier;

        annotation.render_text(&mut self.overlay, font, scale, self.width, self.height);
    }

    /// Produces the final rendering of the canvas.
    ///
    /// This rendering step applies the upscaled overlay to the base canvas, thereby adding the
    /// desired text to the image proper. This is done via resizing and then overlaying. It's not
    /// rocket surgery; the whole process is three lines of code.
    ///
    /// I've added this documentation just as a reminder of what's actually going on here.
    pub fn render(&mut self) {
        let downsampled_text = imageops::resize(
            &self.overlay,
            self.width,
            self.height,
            imageops::FilterType::Lanczos3,
        );

        let image = &DynamicImage::ImageRgba8(downsampled_text);
        imageops::overlay(&mut self.base, image, 0, 0);
    }

    pub fn save_jpg(&self, stream: &mut (impl io::Write + io::Seek)) -> io::Result<()> {
        self.base
            .write_to(stream, ImageFormat::Jpeg)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    pub fn save_png(&self, stream: &mut (impl io::Write + io::Seek)) -> io::Result<()> {
        self.base
            .write_to(stream, ImageFormat::Png)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}
