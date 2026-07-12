use std::time::{Duration, Instant};

use artano::{load_font, Annotation, Canvas, Font};
use image::{ImageBuffer, Rgba};

struct Case {
    name: &'static str,
    width: u32,
    height: u32,
    annotations: Vec<Annotation>,
}

struct Timing {
    read: Duration,
    annotate: Duration,
    render: Duration,
    total: Duration,
}

impl Timing {
    fn zero() -> Self {
        Self {
            read: Duration::ZERO,
            annotate: Duration::ZERO,
            render: Duration::ZERO,
            total: Duration::ZERO,
        }
    }

    fn add_assign(&mut self, other: &Timing) {
        self.read += other.read;
        self.annotate += other.annotate;
        self.render += other.render;
        self.total += other.total;
    }

    fn div(&self, n: u32) -> Self {
        Self {
            read: self.read / n,
            annotate: self.annotate / n,
            render: self.render / n,
            total: self.total / n,
        }
    }
}

fn main() {
    let font = load_font("Impact").expect("Impact font must be installed for the benchmark");

    let cases = vec![
        Case {
            name: "sd_short_top",
            width: 854,
            height: 480,
            annotations: vec![Annotation::top("HELLO WORLD")],
        },
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

    println!("\n=== artano benchmark (current implementation) ===\n");
    println!(
        "{:<22} {:>10} {:>10} {:>10} {:>10}",
        "case", "read", "annotate", "render", "total"
    );
    println!("{}", "-".repeat(66));

    let warmup = 3;
    let iterations = 10;

    for case in &cases {
        let image_bytes = generate_test_image(case.width, case.height);

        for _ in 0..warmup {
            run(&image_bytes, &case.annotations, &font);
        }

        let mut sum = Timing::zero();
        for _ in 0..iterations {
            sum.add_assign(&run(&image_bytes, &case.annotations, &font));
        }
        let avg = sum.div(iterations);

        println!(
            "{:<22} {:>10.2?} {:>10.2?} {:>10.2?} {:>10.2?}",
            case.name, avg.read, avg.annotate, avg.render, avg.total
        );
    }

    println!();
}

fn run(image_bytes: &[u8], annotations: &[Annotation], font: &Font) -> Timing {
    let t_total_start = Instant::now();

    let mut canvas = Canvas::read_from_buffer(image_bytes).expect("decode");
    let t_read = t_total_start.elapsed();

    let t_annotate_start = Instant::now();
    for ann in annotations {
        canvas.add_annotation(ann, font, 1.0);
    }
    let t_annotate = t_annotate_start.elapsed();

    let t_render_start = Instant::now();
    canvas.render();
    let t_render = t_render_start.elapsed();

    Timing {
        read: t_read,
        annotate: t_annotate,
        render: t_render,
        total: t_total_start.elapsed(),
    }
}

fn generate_test_image(width: u32, height: u32) -> Vec<u8> {
    let img = ImageBuffer::from_fn(width, height, |x, y| {
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
