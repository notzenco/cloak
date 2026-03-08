pub mod bmp;
pub mod lsb;
pub mod png;

use crate::CloakError;

/// Supported image formats for steganography.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Bmp,
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

        // Fall back to extension
        if let Some(path) = path {
            let lower = path.to_lowercase();
            if lower.ends_with(".png") {
                return Ok(Self::Png);
            }
            if lower.ends_with(".bmp") {
                return Ok(Self::Bmp);
            }
        }

        Err(CloakError::UnsupportedFormat(
            path.unwrap_or("unknown").to_string(),
        ))
    }
}
