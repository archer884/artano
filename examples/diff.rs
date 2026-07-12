// Compares two rendering paths pixel-by-pixel.
//
// Both paths use the same code (the public `Canvas` API) initially, so
// the diff should be 0. When the refactor lands, switch `render_b` to the
// new path to see the visual change.
//
// Run with: cargo run --release --example diff
//
// Diff visualizations (|a - b| * 16, clamped to 255) are written to
// target/diff/<case>.png. Black = no diff.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use artano::{load_font, Annotation, Canvas, Font};
use image::{DynamicImage, Rgba, RgbaImage};

struct Case {
    name: &'static str,
    width: u32,
    height: u32,
    annotations: Vec<Annotation>,
}

struct DiffResult {
    max_channel_diff: u8,
    mean_channel_diff: f64,
    pixels_with_diff: usize,
    pixels_with_diff_gt_5: usize,
    pixels_with_diff_gt_32: usize,
    total_pixels: usize,
    time_a: Duration,
    time_b: Duration,
}

fn main() {
    let font = load_font("Impact").expect("Impact font must be installed");

    let cases = vec![
        Case {
            name: "hd_short_top",
            width: 1920,
            height: 1080,
            annotations: vec![Annotation::top("HELLO WORLD")],
        },
        Case {
            name: "hd_long_split",
            width: 1920,
            height: 1080,
            annotations: vec![Annotation::top(
                "THIS IS A VERY LONG CAPTION THAT SHOULD DEFINITELY BE SPLIT ACROSS TWO LINES",
            )],
        },
        Case {
            name: "hd_two_annotations",
            width: 1920,
            height: 1080,
            annotations: vec![
                Annotation::top("TOP CAPTION"),
                Annotation::bottom("BOTTOM CAPTION"),
            ],
        },
        Case {
            name: "4k_short_top",
            width: 3840,
            height: 2160,
            annotations: vec![Annotation::top("HELLO WORLD")],
        },
    ];

    let out_dir = PathBuf::from("target/diff");
    std::fs::create_dir_all(&out_dir).expect("create diff dir");

    println!("\n=== artano diff (method_a vs method_b) ===\n");
    println!(
        "{:<22} {:>9} {:>10} {:>9} {:>9} {:>8} {:>8} {:>11} {:>11}",
        "case", "max_diff", "mean_diff", "px_diff", "of_total", "px>5", "px>32", "time_a", "time_b"
    );
    println!("{}", "-".repeat(110));

    for case in &cases {
        let image_bytes = generate_test_image(case.width, case.height);

        let (img_a, time_a) = render_a(&image_bytes, &case.annotations, &font);
        let (img_b, time_b) = render_b(&image_bytes, &case.annotations, &font);
        let result = compute_diff(&img_a, &img_b, time_a, time_b);

        if let Err(e) =
            write_diff_image(&img_a, &img_b, &out_dir.join(format!("{}.png", case.name)))
        {
            eprintln!("failed to write diff image for {}: {}", case.name, e);
        }

        let pct = if result.total_pixels > 0 {
            100.0 * result.pixels_with_diff as f64 / result.total_pixels as f64
        } else {
            0.0
        };

        println!(
            "{:<22} {:>9} {:>10.4} {:>9} {:>8.3}% {:>8} {:>8} {:>11.2?} {:>11.2?}",
            case.name,
            result.max_channel_diff,
            result.mean_channel_diff,
            result.pixels_with_diff,
            pct,
            result.pixels_with_diff_gt_5,
            result.pixels_with_diff_gt_32,
            result.time_a,
            result.time_b,
        );
    }

    println!("\nDiff visualizations written to: {}", out_dir.display());
    println!("(method_a and method_b currently use the same code path; diff should be 0.)\n");
}

fn render_a(
    image_bytes: &[u8],
    annotations: &[Annotation],
    font: &Font,
) -> (DynamicImage, Duration) {
    let start = Instant::now();
    let mut canvas = Canvas::read_from_buffer(image_bytes).expect("decode");
    for ann in annotations {
        canvas.add_annotation(ann, font, 1.0);
    }
    canvas.render();
    let time = start.elapsed();
    (canvas_to_image(&canvas), time)
}

fn render_b(
    image_bytes: &[u8],
    annotations: &[Annotation],
    font: &Font,
) -> (DynamicImage, Duration) {
    // Placeholder for the second rendering path. Currently identical to
    // render_a so the diff is 0 (sanity check that the pipeline is
    // consistent with itself). Swap in a different rendering path here
    // to compare against it in the future.
    render_a(image_bytes, annotations, font)
}

fn canvas_to_image(canvas: &Canvas) -> DynamicImage {
    let mut buf = std::io::Cursor::new(Vec::new());
    canvas.save_png(&mut buf).expect("encode png");
    image::load_from_memory(&buf.into_inner()).expect("decode png")
}

fn compute_diff(
    img_a: &DynamicImage,
    img_b: &DynamicImage,
    time_a: Duration,
    time_b: Duration,
) -> DiffResult {
    let a = img_a.to_rgba8();
    let b = img_b.to_rgba8();
    let (w, h) = a.dimensions();
    let total = (w * h) as usize;

    let mut max_diff: u8 = 0;
    let mut sum_diff: u64 = 0;
    let mut with_diff: usize = 0;
    let mut with_diff_gt_5: usize = 0;
    let mut with_diff_gt_32: usize = 0;

    for (pa, pb) in a.pixels().zip(b.pixels()) {
        let mut any = false;
        let mut any_5 = false;
        let mut any_32 = false;
        for i in 0..4 {
            let d = pa[i].abs_diff(pb[i]);
            max_diff = max_diff.max(d);
            sum_diff += d as u64;
            if d > 0 {
                any = true;
            }
            if d > 5 {
                any_5 = true;
            }
            if d > 32 {
                any_32 = true;
            }
        }
        if any {
            with_diff += 1;
        }
        if any_5 {
            with_diff_gt_5 += 1;
        }
        if any_32 {
            with_diff_gt_32 += 1;
        }
    }

    DiffResult {
        max_channel_diff: max_diff,
        mean_channel_diff: sum_diff as f64 / (total * 4) as f64,
        pixels_with_diff: with_diff,
        pixels_with_diff_gt_5: with_diff_gt_5,
        pixels_with_diff_gt_32: with_diff_gt_32,
        total_pixels: total,
        time_a,
        time_b,
    }
}

fn write_diff_image(
    img_a: &DynamicImage,
    img_b: &DynamicImage,
    path: &Path,
) -> image::ImageResult<()> {
    let a = img_a.to_rgba8();
    let b = img_b.to_rgba8();
    let (w, h) = a.dimensions();
    let mut out = RgbaImage::new(w, h);
    for (x, y, pa) in a.enumerate_pixels() {
        let pb = b.get_pixel(x, y);
        out.put_pixel(
            x,
            y,
            Rgba([
                pa[0].abs_diff(pb[0]).saturating_mul(16),
                pa[1].abs_diff(pb[1]).saturating_mul(16),
                pa[2].abs_diff(pb[2]).saturating_mul(16),
                pa[3].abs_diff(pb[3]).saturating_mul(16),
            ]),
        );
    }
    DynamicImage::ImageRgba8(out).save(path)
}

fn generate_test_image(width: u32, height: u32) -> Vec<u8> {
    let img = image::ImageBuffer::from_fn(width, height, |x, y| {
        let r = ((x * 255) / width.max(1)) as u8;
        let g = ((y * 255) / height.max(1)) as u8;
        let b = (((x.wrapping_add(y)) * 128) / (width + height).max(1)) as u8;
        Rgba([r, g, b, 255])
    });
    let dynamic = image::DynamicImage::ImageRgba8(img);
    let mut buf = std::io::Cursor::new(Vec::new());
    dynamic
        .write_to(&mut buf, image::ImageFormat::Png)
        .expect("encode png");
    buf.into_inner()
}
