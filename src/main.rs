use image::io::Reader as ImageReader;
use image::Pixel;
use image::{DynamicImage, ImageFormat, RgbImage};
use imageproc::drawing::{draw_antialiased_line_segment_mut, draw_convex_polygon_mut, Point};
use imageproc::edges::canny;
use imageproc::pixelops::interpolate;
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use rtriangulate::{triangulate, Triangle, TriangulationPoint};
use std::fs::File;
use std::io::{stdin, stdout, BufReader, Cursor, Read};
use structopt::StructOpt;

use error::LowPolyResult;
use opts::Options;

mod error;
mod opts;

fn load_image(opts: &Options) -> LowPolyResult<DynamicImage> {
    match &opts.input {
        Some(filename) => Ok(ImageReader::new(BufReader::new(File::open(filename)?))
            .with_guessed_format()?
            .decode()?),
        None => {
            let mut buffer = Vec::new();
            stdin().read_to_end(&mut buffer)?;

            Ok(ImageReader::new(Cursor::new(buffer))
                .with_guessed_format()?
                .decode()?)
        }
    }
}

fn edge_points(img: &DynamicImage, opts: &Options) -> LowPolyResult<Vec<TriangulationPoint<f32>>> {
    let edges = canny(&img.to_luma(), opts.canny_lower, opts.canny_upper);

    let mut points = Vec::new();
    const WHITE: [u8; 1] = [255u8; 1];
    for (x, y, p) in edges.enumerate_pixels() {
        if p.0 == WHITE {
            points.push(TriangulationPoint::new(x as f32, y as f32));
        }
    }
    let mut rng = match opts.rng_seed {
        Some(seed) => SmallRng::from_seed(seed.to_le_bytes()),
        None => SmallRng::from_entropy(),
    };
    points.shuffle(&mut rng);
    points.truncate(opts.points);

    remove_close_points(&mut points, opts.points_min_distance);

    let width = edges.width() as f32;
    let height = edges.height() as f32;
    points.push(TriangulationPoint::new(0., 0.));
    points.push(TriangulationPoint::new(width, 0.));
    points.push(TriangulationPoint::new(0., height));
    points.push(TriangulationPoint::new(width, height));

    Ok(points)
}

fn remove_close_points(points: &mut Vec<TriangulationPoint<f32>>, distance: f32) {
    let mut i = 0;
    while i < points.len() {
        let mut j = i + 1;
        while j < points.len() {
            if ((points[i].x - points[j].x).powi(2) + (points[i].y - points[j].y).powi(2)).sqrt()
                < distance
            {
                points.remove(j);
            } else {
                j += 1;
            }
        }

        i += 1;
    }
}

fn create_low_poly(
    original: &RgbImage,
    points: &[TriangulationPoint<f32>],
    triangulation: &[Triangle],
    opts: &Options,
) -> RgbImage {
    let mut img = RgbImage::new(original.width(), original.height());
    let mut tri_buf = [Point::new(0, 0); 3];
    for tri in triangulation {
        let a = points[tri.0];
        let b = points[tri.1];
        let c = points[tri.2];

        let center = ((a.x + b.x + c.x) as u32 / 3, (a.y + b.y + c.y) as u32 / 3);
        let color = original.get_pixel(center.0, center.1).to_rgb();
        tri_buf[0] = Point::new(a.x as i32, a.y as i32);
        tri_buf[1] = Point::new(b.x as i32, b.y as i32);
        tri_buf[2] = Point::new(c.x as i32, c.y as i32);

        draw_convex_polygon_mut(&mut img, &tri_buf, color);

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

    img
}

fn main() {
    let opts = Options::from_args();

    let image = load_image(&opts).unwrap();
    let points = edge_points(&image, &opts).unwrap();
    let image = image.to_rgb();

    let triangles = triangulate(&points).unwrap();

    let img = create_low_poly(&image, &points, &triangles, &opts);

    let output_format = opts
        .output_format
        .or_else(|| {
            opts.output
                .as_ref()
                .and_then(|out| ImageFormat::from_path(out).ok())
        })
        .unwrap_or(ImageFormat::Png);

    match &opts.output {
        Some(out) => img.save_with_format(out, output_format),
        None => DynamicImage::ImageRgb8(img).write_to(&mut stdout().lock(), output_format),
    }
    .unwrap();
}
