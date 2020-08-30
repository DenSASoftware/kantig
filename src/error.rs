use image::error::ImageError;
use std::io::Error as IOError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LowPolyError {
    #[error("error encoding/decoding image: {0}")]
    ImageError(#[from] ImageError),
    #[error("io error: {0}")]
    IOError(#[from] IOError),
}

pub type LowPolyResult<T> = Result<T, LowPolyError>;
