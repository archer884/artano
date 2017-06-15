use annotation::Annotation;
use error::Result;
use image::{self, DynamicImage, FilterType, GenericImage, ImageFormat, imageops, RgbaImage};
use rusttype::Font;
use std::io;

use AA_FACTOR;

pub struct Canvas {
    base: DynamicImage,
    overlay: DynamicImage,
    width: u32,
    height: u32,
}

impl Canvas {
    pub fn read_from_buffer(buf: &[u8]) -> Result<Canvas> {
        let base = image::load_from_memory(buf)?;
        let (width, height) = base.dimensions();
        Ok(Canvas {
            base,
            overlay: DynamicImage::ImageRgba8(
                RgbaImage::new(width * AA_FACTOR, height * AA_FACTOR),
            ),
            width,
            height,
        })
    }

    pub fn add_annotation<'a>(
        &mut self,
        annotation: &Annotation,
        font: &Font<'a>,
        scale_multiplier: f32,
    ) {
        let scale_factor = (self.height as f32 / 10.0) * scale_multiplier;
        annotation.render_text(&mut self.overlay, font, scale_factor, self.width, self.height);
    }

    pub fn render(&mut self) {
        let downsampled_text = imageops::resize(
            &self.overlay,
            self.width,
            self.height,
            FilterType::CatmullRom,
        );

        let image = &DynamicImage::ImageRgba8(downsampled_text);
        imageops::overlay(&mut self.base, image, 0, 0);
    }

    pub fn save_jpg<W: io::Write>(&self, stream: &mut W) -> io::Result<()> {
        self.base.save(stream, ImageFormat::JPEG).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, e)
        })
    }

    pub fn save_png<W: io::Write>(&self, stream: &mut W) -> io::Result<()> {
        self.base.save(stream, ImageFormat::PNG).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, e)
        })
    }
}
