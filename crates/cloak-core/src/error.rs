use thiserror::Error;

#[derive(Debug, Error)]
pub enum CloakError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unsupported image format: {0}")]
    UnsupportedFormat(String),

    #[error("payload too large: need {needed} bytes, capacity is {capacity} bytes")]
    PayloadTooLarge { needed: usize, capacity: usize },

    #[error("invalid passphrase")]
    InvalidPassphrase,

    #[error("corrupted data: {0}")]
    CorruptedData(String),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
}
