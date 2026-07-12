use image::{DynamicImage, GenericImage, GrayImage, ImageBuffer, Luma, Rgba};
use imageproc::morphology::{grayscale_dilate, Mask};
use rusttype::{Font, Scale};

use crate::draw;

// The final value in the array here is the *opacity* of the pixel. Not the transparency.
// Apparently, this is not CSS...
const WHITE_PIXEL: Rgba<u8> = Rgba([255, 255, 255, 255]);
const BLACK_PIXEL: Rgba<u8> = Rgba([0, 0, 0, 255]);

/// Render one line of caption text with a black outline into `pixels`.
///
/// The outline is produced by a single morphology dilate with a 1-pixel
/// ring structuring element at the outline radius, instead of the
/// `imageproc::edges::canny` + `draw_hollow_circle_mut`-per-edge-pixel
/// approach used by the pre-refactor implementation.
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

    let (text_width, text_height) = text_dimensions;
    let scale = Scale::uniform(scale_factor * AA_FACTOR_FLOAT);

    // To reduce the janky jagginess of the black border around each letter, we want to render
    // the words themselves at 16x resolution and then paste that on top of the existing
    // image.
    let (x, y) = root_position;
    let x = x * AA_FACTOR;
    let y = (y as i32 + y_offset) as u32 * AA_FACTOR;

    let mask_w = text_width * AA_FACTOR;
    let mask_h = text_height * AA_FACTOR;

    // 1. Draw the text mask (white text on black background) at 3x.
    let mut text_mask: GrayImage = ImageBuffer::from_pixel(mask_w, mask_h, Luma([0u8]));
    draw::text(&mut text_mask, Luma([255u8]), 0, 0, scale, font, text);

    // 2. Build a 1-pixel-thick ring mask at the outline radius.
    let radius = (0.09 * scale_factor) as i32;
    let ring_dim = (2 * radius + 1) as u32;
    let mut ring_img: GrayImage = ImageBuffer::from_pixel(ring_dim, ring_dim, Luma([0u8]));
    for ry in 0..ring_dim {
        for rx in 0..ring_dim {
            let dx = rx as i32 - radius;
            let dy = ry as i32 - radius;
            let dist = ((dx * dx + dy * dy) as f64).sqrt();
            if (dist - radius as f64).abs() < 0.5 {
                ring_img.put_pixel(rx, ry, Luma([255u8]));
            }
        }
    }
    let ring_mask = Mask::from_image(&ring_img, radius as u8, radius as u8);

    // 3. Dilate the text mask with the ring mask -> 1-pixel ring at distance `radius`.
    let outline = grayscale_dilate(&text_mask, &ring_mask);

    // 4. Stamp the outline as opaque black.
    let (max_x, max_y) = (pixels.width() as i32, pixels.height() as i32);
    for (idx, outline_px) in outline.pixels().enumerate() {
        if outline_px[0] > 0 {
            let idx = idx as u32;
            let px = idx % mask_w + x;
            let py = idx / mask_w + y;
            let pxi = px as i32;
            let pyi = py as i32;
            if pxi >= 0 && pxi < max_x && pyi >= 0 && pyi < max_y {
                pixels.put_pixel(px, py, BLACK_PIXEL);
            }
        }
    }

    // 5. Draw the white text on top (weighted blend so the AA edges transition
    //    smoothly out of the black outline).
    draw::text(pixels, WHITE_PIXEL, x, y, scale, font, text);
}
