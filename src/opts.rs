use std::fmt::{Display, Formatter, Result as FmtResult};
use std::num::ParseFloatError;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug)]
enum FloatParsingError {
    Native(ParseFloatError),
    NonNormal,
    Negative,
}

impl Display for FloatParsingError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
        match self {
            FloatParsingError::Native(err) => err.fmt(fmt),
            FloatParsingError::NonNormal => write!(fmt, "number is inf or NaN"),
            FloatParsingError::Negative => write!(fmt, "number is negative"),
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

#[derive(Debug, StructOpt)]
pub struct Options {
    #[structopt(long, parse(try_from_str = parse_positive_float), default_value = "10.0")]
    pub canny_lower: f32,

    #[structopt(long, parse(try_from_str = parse_positive_float), default_value = "15.0")]
    pub canny_upper: f32,

    #[structopt(short, long, default_value = "10000")]
    pub points: usize,

    #[structopt(long, parse(try_from_str = parse_positive_float), default_value = "4")]
    pub points_min_distance: f32,

    #[structopt(long)]
    pub no_antialiasing: bool,

    #[structopt(long)]
    pub rng_seed: Option<u128>,

    #[structopt(long, short, parse(from_os_str))]
    pub output: Option<PathBuf>,

    #[structopt(parse(from_os_str))]
    pub input: Option<PathBuf>,
}
