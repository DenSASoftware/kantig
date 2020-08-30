use image::Pixel;
use imageproc::edges::canny;
use rand::{thread_rng, seq::SliceRandom};
use rtriangulate::{TriangulationPoint, triangulate};

fn main() {
    let orig = image::open(std::env::args().nth(1).unwrap()).unwrap();

    let c = canny(&orig.to_luma(), 10.0, 100.0);
    let orig = orig.to_rgb();

    let mut points = Vec::new();
    for (x, y, p) in c.enumerate_pixels() {
        if p.0 == [255u8; 1] {
            points.push(TriangulationPoint::new(x as f32, y as f32));
        }
    }
    points.shuffle(&mut thread_rng());
    points.truncate(1000);

    let mut i = 0;
    while i < points.len() {
        let mut j = i + 1;
        while j < points.len() {
            if ((points[i].x - points[j].x).powi(2) + (points[i].y - points[j].y).powi(2)).sqrt() < 2.5 {
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

    let mut img = image::RgbImage::new(c.width(), c.height());
    let mut tri_buf = [imageproc::drawing::Point::new(0, 0); 3];
    for tri in triangles {
        let a = points[tri.0];
        let b = points[tri.1];
        let c = points[tri.2];

        let center = (
            (a.x + b.x + c.x) as u32 / 3,
            (a.y + b.y + c.y) as u32 / 3,
        );
        let color = orig.get_pixel(center.0, center.1).to_rgb();
        tri_buf[0] = imageproc::drawing::Point::new(a.x as i32, a.y as i32);
        tri_buf[1] = imageproc::drawing::Point::new(b.x as i32, b.y as i32);
        tri_buf[2] = imageproc::drawing::Point::new(c.x as i32, c.y as i32);

        imageproc::drawing::draw_convex_polygon_mut(
            &mut img,
            &tri_buf,
            color
        );

        imageproc::drawing::draw_antialiased_line_segment_mut(
            &mut img,
            (a.x as i32, a.y as i32),
            (b.x as i32, b.y as i32),
            color,
            imageproc::pixelops::interpolate,
        );
        imageproc::drawing::draw_antialiased_line_segment_mut(
            &mut img,
            (a.x as i32, a.y as i32),
            (c.x as i32, c.y as i32),
            color,
            imageproc::pixelops::interpolate,
        );
        imageproc::drawing::draw_antialiased_line_segment_mut(
            &mut img,
            (c.x as i32, c.y as i32),
            (b.x as i32, b.y as i32),
            color,
            imageproc::pixelops::interpolate,
        );
    }

    img.save_with_format("/dev/stdout", image::ImageFormat::Png).unwrap();
}
