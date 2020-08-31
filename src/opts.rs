use crate::error::{LowPolyError, LowPolyResult};
use image::ImageFormat;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::num::ParseFloatError;
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

/// An error for parsing floats from the command line. Aside from the default parsing errors the
/// passed value might not satisfy certain constraints, e.g. being between 0 and 1 or not being
/// NaN.
#[derive(Debug)]
enum FloatParsingError {
    Native(ParseFloatError),
    NonNormal,
    Negative,
    BiggerThanOne,
}

impl Display for FloatParsingError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
        match self {
            FloatParsingError::Native(err) => err.fmt(fmt),
            FloatParsingError::NonNormal => write!(fmt, "number is inf or NaN"),
            FloatParsingError::Negative => write!(fmt, "number is negative"),
            FloatParsingError::BiggerThanOne => write!(fmt, "number is bigger than one"),
        }
    }
}

/// Parse a float from `src` and return `Ok(f)` if the value is greater than or equal to 0,
/// excluding NaN and +/- infinite. -0 is allowed.
fn parse_positive_float(src: &str) -> Result<f32, FloatParsingError> {
    let num = match src.parse::<f32>() {
        Ok(num) => num,
        Err(err) => return Err(FloatParsingError::Native(err)),
    };

    match num {
        _ if num.is_infinite() || num.is_nan() => Err(FloatParsingError::NonNormal),
        _ if num < 0. => Err(FloatParsingError::Negative),
        _ => Ok(num),
    }
}

/// Same as [parse_positive_float](fn.parse_positive_float.html), but also requires the value to be
/// in the range [0.0, 1.0].
fn parse_float_between_one_zero(src: &str) -> Result<f32, FloatParsingError> {
    let num = parse_positive_float(src)?;
    match num {
        _ if num > 1. => Err(FloatParsingError::BiggerThanOne),
        _ => Ok(num),
    }
}

/// This error will be returned when the user passes an image extension name that cannot be mapped
/// to an image type.
#[derive(Debug, Error)]
enum ImageFormatError {
    #[error("unsupported image format {0}")]
    Unsupported(String),
}

/// Search the different image formats supported by the image crate and return the one that lists
/// `src` as one of its file extension names.
fn parse_image_format(src: &str) -> Result<ImageFormat, ImageFormatError> {
    use ImageFormat::*;
    let formats = [
        Png, Jpeg, Gif, WebP, Pnm, Tiff, Tga, Dds, Bmp, Ico, Hdr, Farbfeld,
    ];

    formats
        .iter()
        .find(|format| format.extensions_str().into_iter().any(|ext| *ext == src))
        .cloned()
        .ok_or_else(|| ImageFormatError::Unsupported(src.into()))
}

#[derive(Debug, StructOpt)]
#[structopt(name = "kantig")]
/// Create low-poly images
///
/// Transform an image into a low-poly image. This program applies an edge-detection-algorithm,
/// selects a few of the resulting points and uses the delaunay-algorithm to create a mesh of
/// triangles that will be turned into the final image.
pub struct Options {
    /// the lower bound for the edge detectioin algorithm
    #[structopt(long, parse(try_from_str = parse_positive_float), default_value = "10.0")]
    pub canny_lower: f32,

    /// the upper bound for the edge detection algorithm
    #[structopt(long, parse(try_from_str = parse_positive_float), default_value = "15.0")]
    pub canny_upper: f32,

    /// the number of points to be picked from the edges
    #[structopt(short, long)]
    pub points: Option<usize>,

    /// pick an amount of low-poly points relative to the number of edge-points, 0 means none and 1
    /// means all
    #[structopt(long, parse(try_from_str = parse_float_between_one_zero))]
    pub points_relative: Option<f32>,

    /// pick an amount of low-poly points relative to the number of pixels in the image, 0 means
    /// none and 1 means all
    #[structopt(long, parse(try_from_str = parse_float_between_one_zero))]
    pub points_pixel_relative: Option<f32>,

    /// enforce a minimum distance between low-poly points
    #[structopt(long, parse(try_from_str = parse_positive_float), default_value = "4")]
    pub points_min_distance: f32,

    /// a shell command to map color values in the final image
    #[structopt(long)]
    pub color_mapper: Option<String>,

    /// do not use antialiasing when drawing polygons
    #[structopt(long)]
    pub no_antialiasing: bool,

    /// the seed for the random number generator, values from 0 to 2^16-1 are acceptable
    #[structopt(long)]
    pub rng_seed: Option<u128>,

    /// the output file name, defaults to stdout
    #[structopt(long, short, parse(from_os_str))]
    pub output: Option<PathBuf>,

    /// the output format, overrides the file format detected in the output file name
    #[structopt(long, parse(try_from_str = parse_image_format))]
    pub output_format: Option<ImageFormat>,

    /// the input file name, defaults to stdin
    #[structopt(parse(from_os_str))]
    pub input: Option<PathBuf>,
}

/// An enum specifying how many points should be used for triangulation
pub enum PixelUnit {
    /// Use an absolute number of points
    Absolute(usize),
    /// Use a fraction of the number of edge points
    Relative(f32),
    /// Use a fraction of the number of pixels in the image
    PixelRelative(f32),
}

impl Options {
    /// Check the cli options and return an object describing how many points should be used for
    /// triangulation. Fail if more than one option is set.
    pub fn edge_number(&self) -> LowPolyResult<PixelUnit> {
        match (
            self.points,
            self.points_relative,
            self.points_pixel_relative,
        ) {
            (None, None, None) => Ok(PixelUnit::Absolute(10000)),
            (Some(abs), None, None) => Ok(PixelUnit::Absolute(abs)),
            (None, Some(rel), None) => Ok(PixelUnit::Relative(rel)),
            (None, None, Some(rel)) => Ok(PixelUnit::PixelRelative(rel)),
            _ => Err(LowPolyError::CLIError),
        }
    }
}
