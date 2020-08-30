use crate::error::{LowPolyError, LowPolyResult};
use image::ImageFormat;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::num::ParseFloatError;
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

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

fn parse_float_between_one_zero(src: &str) -> Result<f32, FloatParsingError> {
    let num = parse_positive_float(src)?;
    match num {
        _ if num > 1. => Err(FloatParsingError::BiggerThanOne),
        _ => Ok(num),
    }
}

#[derive(Debug, Error)]
enum ImageFormatError {
    #[error("unsupported image format {0}")]
    Unsupported(String),
}

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
pub struct Options {
    #[structopt(long, parse(try_from_str = parse_positive_float), default_value = "10.0")]
    pub canny_lower: f32,

    #[structopt(long, parse(try_from_str = parse_positive_float), default_value = "15.0")]
    pub canny_upper: f32,

    #[structopt(short, long)]
    pub points: Option<usize>,

    #[structopt(long, parse(try_from_str = parse_float_between_one_zero))]
    pub points_relative: Option<f32>,

    #[structopt(long, parse(try_from_str = parse_float_between_one_zero))]
    pub points_pixel_relative: Option<f32>,

    #[structopt(long, parse(try_from_str = parse_positive_float), default_value = "4")]
    pub points_min_distance: f32,

    #[structopt(long)]
    pub no_antialiasing: bool,

    #[structopt(long)]
    pub rng_seed: Option<u128>,

    #[structopt(long, short, parse(from_os_str))]
    pub output: Option<PathBuf>,

    #[structopt(long, parse(try_from_str = parse_image_format))]
    pub output_format: Option<ImageFormat>,

    #[structopt(parse(from_os_str))]
    pub input: Option<PathBuf>,
}

pub enum PixelUnit {
    Absolute(usize),
    Relative(f32),
    PixelRelative(f32),
}

impl Options {
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
