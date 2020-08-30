use image::Pixel;
use image::io::Reader as ImageReader;
use image::RgbImage;
use std::io::Read;
use imageproc::edges::canny;
use imageproc::drawing::{
    draw_convex_polygon_mut,
    draw_antialiased_line_segment_mut,
    Point,
};
use imageproc::pixelops::interpolate;
use rand::{seq::SliceRandom, rngs::SmallRng, SeedableRng};
use rtriangulate::{TriangulationPoint, triangulate};
use structopt::StructOpt;
use std::path::PathBuf;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(long, default_value = "10.0")]
    canny_lower: f32,

    #[structopt(long, default_value = "100.0")]
    canny_upper: f32,

    #[structopt(short, long, default_value = "1000")]
    points: usize,

    #[structopt(long, default_value = "2.5")]
    points_min_distance: f32,

    #[structopt(long)]
    no_antialiasing: bool,

    #[structopt(long)]
    rng_seed: Option<u128>,

    #[structopt(long, short, parse(from_os_str))]
    output: Option<PathBuf>,

    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,
}

fn main() {
    let opts = Options::from_args();

    let mut buffer = Vec::new();
    match opts.input {
        Some(filename) => std::fs::File::open(filename).unwrap().read_to_end(&mut buffer).unwrap(),
        None => std::io::stdin().read_to_end(&mut buffer).unwrap(),
    };
    let orig = ImageReader::new(std::io::Cursor::new(buffer)).with_guessed_format().unwrap().decode().unwrap();

    let c = canny(&orig.to_luma(), opts.canny_lower, opts.canny_upper);
    let orig = orig.to_rgb();

    let mut points = Vec::new();
    for (x, y, p) in c.enumerate_pixels() {
        if p.0 == [255u8; 1] {
            points.push(TriangulationPoint::new(x as f32, y as f32));
        }
    }
    let mut rng = match opts.rng_seed {
        Some(seed) => SmallRng::from_seed(seed.to_le_bytes()),
        None => SmallRng::from_entropy(),
    };
    points.shuffle(&mut rng);
    points.truncate(opts.points);

    let mut i = 0;
    while i < points.len() {
        let mut j = i + 1;
        while j < points.len() {
            if ((points[i].x - points[j].x).powi(2) + (points[i].y - points[j].y).powi(2)).sqrt() < opts.points_min_distance {
                points.remove(j);
            } else {
                j += 1;
            }
        }

        i += 1;
    }

    let width = c.width() as f32;
    let height = c.height() as f32;
    points.push(TriangulationPoint::new(0., 0.));
    points.push(TriangulationPoint::new(width, 0.));
    points.push(TriangulationPoint::new(0., height));
    points.push(TriangulationPoint::new(width, height));

    let triangles = triangulate(&points).unwrap();

    let mut img = RgbImage::new(c.width(), c.height());
    let mut tri_buf = [Point::new(0, 0); 3];
    for tri in triangles {
        let a = points[tri.0];
        let b = points[tri.1];
        let c = points[tri.2];

        let center = (
            (a.x + b.x + c.x) as u32 / 3,
            (a.y + b.y + c.y) as u32 / 3,
        );
        let color = orig.get_pixel(center.0, center.1).to_rgb();
        tri_buf[0] = Point::new(a.x as i32, a.y as i32);
        tri_buf[1] = Point::new(b.x as i32, b.y as i32);
        tri_buf[2] = Point::new(c.x as i32, c.y as i32);

        draw_convex_polygon_mut(
            &mut img,
            &tri_buf,
            color
        );

        if !opts.no_antialiasing {
            let ps = [a, b, c];

            for i in 0..3 {
                let p1 = ps[i];
                let p2 = ps[(i + 1) % 3];

                draw_antialiased_line_segment_mut(
                    &mut img,
                    (p1.x as i32, p1.y as i32),
                    (p2.x as i32, p2.y as i32),
                    color,
                    interpolate,
                );
            }
        }
    }

    img.save_with_format("/dev/stdout", image::ImageFormat::Png).unwrap();
}
