use image::error::ImageError;
use std::io::Error as IOError;
use thiserror::Error;

/// A simple error wrapping the different errors that can occur during the run
#[derive(Debug, Error)]
pub enum LowPolyError {
    #[error("error encoding/decoding image: {0}")]
    ImageError(#[from] ImageError),
    #[error("io error: {0}")]
    IOError(#[from] IOError),

    /// This one is raised if the user passes more than one option regarding the number of points
    /// used, thereby giving mixed signals. I could not figure out how to have structopt check for
    /// this automatically.
    #[error("only one of --points, --points-relative and --points-pixel-relative can be set")]
    CLIError,
}

pub type LowPolyResult<T> = Result<T, LowPolyError>;
