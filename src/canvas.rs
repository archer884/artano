use std::io;

use image::{self, imageops, DynamicImage, GenericImageView, ImageFormat, RgbaImage};
use rusttype::Font;

use crate::annotation::Annotation;
use crate::outline;
use crate::Result;
use crate::AA_FACTOR;

/// One rendered (text + outline) buffer plus where it belongs in the
/// base image. Produced by `Canvas::add_annotation` and consumed by
/// `Canvas::render`.
struct RenderedLayer {
    /// RgbaImage of size `(text_width * AA_FACTOR + 2*pad, text_height *
    /// AA_FACTOR + 2*pad)` containing text + outline. After Lanczos3
    /// resize-by-3 it becomes `(text_width + 2*pad/AA_FACTOR, text_height +
    /// 2*pad/AA_FACTOR)`.
    buffer: RgbaImage,
    /// Top-left of the text bbox in base image coordinates.
    x: u32,
    y: u32,
    /// Padding on each side of the small buffer, measured in 3x pixels.
    /// Needed to compute the composite offset.
    pad: u32,
}

pub struct Canvas {
    base: DynamicImage,
    layers: Vec<RenderedLayer>,
    width: u32,
    height: u32,
}

impl Canvas {
    /// Creates a new canvas based on a buffer of bytes.
    pub fn read_from_buffer(buf: &[u8]) -> Result<Canvas> {
        let base = image::load_from_memory(buf)?;
        let (width, height) = base.dimensions();
        Ok(Canvas {
            base,
            layers: Vec::new(),
            width,
            height,
        })
    }

    /// Adds an annotation to the canvas.
    ///
    /// Each annotation is rendered into its own small buffer (sized to
    /// the text bbox × `AA_FACTOR` plus a margin for the outline +
    /// Lanczos3 kernel). `Canvas::render` is the step that actually
    /// composites the rendered layers onto the base image.
    pub fn add_annotation(&mut self, annotation: &Annotation, font: &Font, scale_multiplier: f32) {
        // Font scale is, in fact, the height in pixels of each glyph. Here we set that to be
        // one tenth the height of the image itself modified by the scale multiplier provided
        // by the user. The multiplier serves to allow us to shrink or expand text to fit images
        // that are either too tall or too small for a given annotation.
        let scale_factor = (self.height as f32 / 10.0) * scale_multiplier;
        let lines = annotation.layout(font, scale_factor, self.width, self.height);

        for line in lines {
            let (buffer, pad) = render_line_to_small_buffer(
                &line.text,
                font,
                scale_factor,
                line.width,
                line.height,
            );
            self.layers.push(RenderedLayer {
                buffer,
                x: line.x,
                y: line.y,
                pad,
            });
        }
    }

    /// Add several annotations at once, rendering them in parallel with rayon.
    /// Each annotation's layout + per-line rendering is fully independent
    /// (it allocates its own small buffer, so the only shared state is
    /// the read-only `font`, `self.width`, and `self.height`), so the
    /// work fans out across the thread pool. The final `render()` step
    /// stays serial because all the produced layers need to be composited
    /// onto the same base image.
    ///
    /// For a single annotation, prefer `add_annotation` — it skips rayon's
    /// work-distribution overhead.
    pub fn add_annotations(
        &mut self,
        annotations: &[Annotation],
        font: &Font,
        scale_multiplier: f32,
    ) {
        use rayon::prelude::*;

        let scale_factor = (self.height as f32 / 10.0) * scale_multiplier;
        let (canvas_w, canvas_h) = (self.width, self.height);

        let new_layers: Vec<RenderedLayer> = annotations
            .par_iter()
            .map(|annotation| {
                let lines = annotation.layout(font, scale_factor, canvas_w, canvas_h);
                lines
                    .into_iter()
                    .map(|line| {
                        let (buffer, pad) = render_line_to_small_buffer(
                            &line.text,
                            font,
                            scale_factor,
                            line.width,
                            line.height,
                        );
                        RenderedLayer {
                            buffer,
                            x: line.x,
                            y: line.y,
                            pad,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect();

        self.layers.extend(new_layers);
    }

    /// Produces the final rendering of the canvas.
    ///
    /// Each layer is Lanczos3-resized by `AA_FACTOR` (only over the
    /// text-bbox-sized region it covers, not the whole image) and
    /// composited onto the base at the annotation's position.
    ///
    /// The resize step is parallelized with rayon — every layer's resize
    /// is independent. The composite step stays serial because every
    /// layer's overlay writes to the same `self.base`.
    pub fn render(&mut self) {
        use rayon::prelude::*;

        let layers = std::mem::take(&mut self.layers);

        // Parallel resize: each layer is resized independently.
        let resized: Vec<(RgbaImage, u32, u32, u32)> = layers
            .par_iter()
            .map(|layer| {
                let (w, h) = layer.buffer.dimensions();
                let downsampled = imageops::resize(
                    &layer.buffer,
                    w / AA_FACTOR,
                    h / AA_FACTOR,
                    imageops::FilterType::Lanczos3,
                );
                (downsampled, layer.x, layer.y, layer.pad)
            })
            .collect();

        // Serial composite: each overlay writes to self.base, so this has
        // to happen on the main thread in the original layer order.
        for (downsampled, x, y, pad) in resized {
            let downsampled = DynamicImage::ImageRgba8(downsampled);

            // The text sits at (pad, pad) inside the small buffer, so after
            // the 3x resize it sits at (pad/AA_FACTOR, pad/AA_FACTOR) in the
            // downsampled image. Subtract that to align the text at (x, y).
            let offset = (pad / AA_FACTOR) as i64;
            let cx = (x as i64 - offset).max(0);
            let cy = (y as i64 - offset).max(0);
            imageops::overlay(&mut self.base, &downsampled, cx, cy);
        }
    }

    pub fn save_jpg(&self, stream: &mut (impl io::Write + io::Seek)) -> io::Result<()> {
        self.base
            .write_to(stream, ImageFormat::Jpeg)
            .map_err(io::Error::other)
    }

    pub fn save_png(&self, stream: &mut (impl io::Write + io::Seek)) -> io::Result<()> {
        self.base
            .write_to(stream, ImageFormat::Png)
            .map_err(io::Error::other)
    }
}

/// Render one line of text + outline into a small buffer sized to
/// `(text_width * AA_FACTOR + 2*pad, text_height * AA_FACTOR + 2*pad)`.
///
/// The `pad` is the per-side margin in 3x pixels covering two things:
///   * the outline radius (`(0.09 * scale_factor) as i32` at 3x), so
///     the hollow-circle outline isn't clipped at the buffer edge, and
///   * the Lanczos3 kernel radius at 3x→1x (3 output px × 3 = 9 input px),
///     so the resize doesn't sample off the edge into undefined pixels.
fn render_line_to_small_buffer(
    text: &str,
    font: &Font,
    scale_factor: f32,
    text_width: u32,
    text_height: u32,
) -> (RgbaImage, u32) {
    let radius_3x = (0.09 * scale_factor) as i32;
    let kernel_3x: i32 = 9;
    let raw_pad = (radius_3x + kernel_3x).max(0) as u32;
    // Round pad up to a multiple of AA_FACTOR so the 3x resize produces
    // a clean output dimension and the composite offset is exact.
    let pad = raw_pad.div_ceil(AA_FACTOR) * AA_FACTOR;

    let buf_w = text_width * AA_FACTOR + 2 * pad;
    let buf_h = text_height * AA_FACTOR + 2 * pad;

    let mut buffer = DynamicImage::ImageRgba8(RgbaImage::new(buf_w, buf_h));

    outline::render_line(
        text,
        0,
        (pad / AA_FACTOR, pad / AA_FACTOR),
        (text_width, text_height),
        scale_factor,
        font,
        &mut buffer,
    );

    (buffer.into_rgba8(), pad)
}
