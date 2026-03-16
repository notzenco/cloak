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

    #[error("passphrase required for randomized mode")]
    MissingPassphrase,

    #[error("corrupted data: {0}")]
    CorruptedData(String),

    #[error("unsupported wire format version: {0}")]
    UnsupportedVersion(u8),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
}
