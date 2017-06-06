use annotation::Annotation;
use error::Result;
use image::{self, DynamicImage, FilterType, GenericImage, ImageBuffer, imageops, Luma, Rgba, RgbaImage};
use std::borrow::Borrow;

// The intent of this Image type is to provide an interface to consumers of this library that is 
// not dependent on the (sometimes confusing) types provided by the image and imageproc libraries.
// Fill this with data and provide it to my API and then it will spit out the right thinga at the 
// right time.

pub struct Canvas(DynamicImage);

impl Canvas {
    pub fn read_from_buffer(buf: &[u8]) -> Result<Canvas> {
        Ok(Canvas(image::load_from_memory(buf)?))
    }

    pub fn annotate<T: Borrow<Annotation>, I: IntoIterator<Item = T>>(&self, annotations: I) -> Vec<u8> {
        let mut pixels = self.0.clone();
        let (width, height) = pixels.dimensions();
        let mut text_rendering = DynamicImage::ImageRgba8(RgbaImage::new(width * 4, height * 4));

        for annotation in annotations.into_iter() {
            annotation.borrow().render_text(&mut text_rendering, font, scale_factor, width, height);
        }

        let downsampled_text = imageops::resize(&text_rendering, width, height, FilterType::CatmullRom);
        imageops::overlay(&mut pixels, &mut DynamicImage::ImageRgba8(downsampled_text), 0, 0);
        pixels.raw_pixels()
    }
}
