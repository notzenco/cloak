pub mod bmp;
pub mod jpeg;
pub mod lsb;
pub mod png;
pub mod webp;

use crate::CloakError;

/// Supported image formats for steganography.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Bmp,
    Jpeg,
    WebP,
}

impl ImageFormat {
    /// Detect format from magic bytes, falling back to file extension.
    pub fn detect(data: &[u8], path: Option<&str>) -> Result<Self, CloakError> {
        // Check magic bytes first
        if data.len() >= 8 && &data[..8] == b"\x89PNG\r\n\x1a\n" {
            return Ok(Self::Png);
        }
        if data.len() >= 2 && &data[..2] == b"BM" {
            return Ok(Self::Bmp);
        }
        if data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
            return Ok(Self::Jpeg);
        }
        if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
            return Ok(Self::WebP);
        }

        // Fall back to extension
        if let Some(path) = path {
            let lower = path.to_lowercase();
            if lower.ends_with(".png") {
                return Ok(Self::Png);
            }
            if lower.ends_with(".bmp") {
                return Ok(Self::Bmp);
            }
            if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
                return Ok(Self::Jpeg);
            }
            if lower.ends_with(".webp") {
                return Ok(Self::WebP);
            }
        }

        Err(CloakError::UnsupportedFormat(
            path.unwrap_or("unknown").to_string(),
        ))
    }

    /// The output format after embedding (lossy inputs produce PNG output).
    pub fn output_format(&self) -> Self {
        match self {
            Self::Png => Self::Png,
            Self::Bmp => Self::Bmp,
            Self::Jpeg | Self::WebP => Self::Png,
        }
    }

    /// File extension for this format.
    pub fn extension(&self) -> &str {
        match self {
            Self::Png => ".png",
            Self::Bmp => ".bmp",
            Self::Jpeg => ".jpg",
            Self::WebP => ".webp",
        }
    }

    /// Whether this format is lossy (stego output will differ from input format).
    pub fn is_lossy(&self) -> bool {
        matches!(self, Self::Jpeg | Self::WebP)
    }
}
