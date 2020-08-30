use image::io::Reader as ImageReader;
use image::Pixel;
use image::{DynamicImage, ImageFormat, RgbImage};
use imageproc::drawing::{draw_antialiased_line_segment_mut, draw_convex_polygon_mut, Point};
use imageproc::edges::canny;
use imageproc::pixelops::interpolate;
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use rtriangulate::{triangulate, Triangle, TriangulationPoint};
use std::fs::File;
use std::io::{stdin, stdout, BufReader, Cursor, Read, Write};
use std::process::{Command, Stdio, exit};
use structopt::StructOpt;

use std::error::Error;
use error::LowPolyResult;
use opts::{Options, PixelUnit};

mod error;
mod opts;

fn distance(p1: &TriangulationPoint<f32>, p2: &TriangulationPoint<f32>) -> f32 {
    ((p1.x - p2.x).powi(2) + (p1.y - p2.y).powi(2)).sqrt()
}

fn load_image(opts: &Options) -> LowPolyResult<DynamicImage> {
    match &opts.input {
        Some(filename) => Ok(ImageReader::new(BufReader::new(File::open(filename)?))
            .with_guessed_format()?
            .decode()?),
        None => {
            let mut buffer = Vec::new();
            stdin().lock().read_to_end(&mut buffer)?;

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

    let limit = match opts.edge_number()? {
        PixelUnit::Absolute(abs) => abs,
        PixelUnit::Relative(rel) => (points.len() as f32 * rel) as usize,
        PixelUnit::PixelRelative(rel) => ((edges.width() * edges.height()) as f32 * rel) as usize,
    };
    points.truncate(limit);

    if opts.points_min_distance > 0. {
        remove_close_points(&mut points, opts.points_min_distance);
    }

    let width = edges.width() as f32;
    let height = edges.height() as f32;
    points.push(TriangulationPoint::new(0., 0.));
    points.push(TriangulationPoint::new(width, 0.));
    points.push(TriangulationPoint::new(0., height));
    points.push(TriangulationPoint::new(width, height));

    Ok(points)
}

fn remove_close_points(points: &mut Vec<TriangulationPoint<f32>>, min_distance: f32) {
    let mut i = 0;
    while i < points.len() {
        let mut j = i + 1;
        while j < points.len() {
            if distance(&points[i], &points[j]) < min_distance {
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
) -> LowPolyResult<RgbImage> {
    let mut img = RgbImage::new(original.width(), original.height());
    let mut tri_buf = [Point::new(0, 0); 3];
    for tri in triangulation {
        let a = points[tri.0];
        let b = points[tri.1];
        let c = points[tri.2];

        let center = ((a.x + b.x + c.x) as u32 / 3, (a.y + b.y + c.y) as u32 / 3);
        tri_buf[0] = Point::new(a.x as i32, a.y as i32);
        tri_buf[1] = Point::new(b.x as i32, b.y as i32);
        tri_buf[2] = Point::new(c.x as i32, c.y as i32);

        let mut color = original.get_pixel(center.0, center.1).to_rgb();
        if let Some(cmd) = &opts.color_mapper {
            color = get_color_from_command(&cmd, color, &[a, b, c], (img.width(), img.height()))?;
        }

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

    Ok(img)
}

fn get_color_from_command(
    cmd: &str,
    default_color: image::Rgb<u8>,
    triangle_coords: &[TriangulationPoint<f32>; 3],
    img_size: (u32, u32),
) -> LowPolyResult<image::Rgb<u8>> {
    let mut proc = if cfg!(target_os = "windows") {
        let mut tmp = Command::new("cmd");
        tmp.args(&["/C", cmd]);

        tmp
    } else {
        let mut tmp = Command::new("sh");
        tmp.args(&["-c", cmd]);

        tmp
    };

    proc.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut proc = proc.spawn().expect("could not spawn child process");
    let stdin = proc.stdin.as_mut().expect("could not open process stdin");
    let text = format!(
        "{} {} {}\n{} {} {} {} {} {}\n{} {}",
        default_color.0[0],
        default_color.0[1],
        default_color.0[2],
        triangle_coords[0].x,
        triangle_coords[0].y,
        triangle_coords[1].x,
        triangle_coords[1].y,
        triangle_coords[2].x,
        triangle_coords[2].y,
        img_size.0,
        img_size.1
    );
    stdin.write_all(text.as_bytes())?;
    stdin.flush()?;

    let output = proc.wait_with_output()?;
    let output = String::from_utf8_lossy(&output.stdout);
    let line = output.lines().next().unwrap();

    let nums = line
        .trim()
        .split_whitespace()
        .map(|s| s.parse::<u8>())
        .collect::<Result<Vec<u8>, _>>()
        .unwrap();
    match *nums.as_slice() {
        [r, g, b] => Ok([r, g, b].into()),
        _ => unimplemented!(),
    }
}

fn save_image(img: RgbImage, opts: &Options) -> LowPolyResult<()> {
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
    }?;

    Ok(())
}

fn simple_unwrap<T, E: Error>(res: Result<T, E>, action: &str) -> T {
    match res {
        Ok(val) => val,
        Err(err) => {
            eprintln!("during {} an error occured: {}", action, err);
            exit(1);
        }
    }
}

fn main() {
    let opts = Options::from_args();

    let image = simple_unwrap(load_image(&opts), "loading the image");
    let points = simple_unwrap(edge_points(&image, &opts), "calculating the edges");
    let image = image.to_rgb();

    let triangles = triangulate(&points).unwrap();

    let img = simple_unwrap(create_low_poly(&image, &points, &triangles, &opts), "creating the low-poly image");
    simple_unwrap(save_image(img, &opts), "saving the image");
}
