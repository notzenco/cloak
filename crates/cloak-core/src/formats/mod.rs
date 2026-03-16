pub mod bmp;
pub mod gif;
pub mod jpeg;
pub mod lsb;
pub mod png;
pub mod tiff;
pub mod webp;

use std::io::Cursor;

use crate::CloakError;
use crate::Result;
use crate::traits::{Capacity, Decoder, Encoder};

/// Supported image formats for steganography.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Bmp,
    Jpeg,
    WebP,
    Gif,
    Tiff,
}

impl ImageFormat {
    /// Detect format from magic bytes, falling back to file extension.
    pub fn detect(data: &[u8], path: Option<&str>) -> crate::Result<Self> {
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
        if data.len() >= 6 && &data[..6] == b"GIF87a" || data.len() >= 6 && &data[..6] == b"GIF89a"
        {
            return Ok(Self::Gif);
        }
        if data.len() >= 4
            && ((data[0] == 0x49 && data[1] == 0x49 && data[2] == 0x2A && data[3] == 0x00)
                || (data[0] == 0x4D && data[1] == 0x4D && data[2] == 0x00 && data[3] == 0x2A))
        {
            return Ok(Self::Tiff);
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
            if lower.ends_with(".gif") {
                return Ok(Self::Gif);
            }
            if lower.ends_with(".tif") || lower.ends_with(".tiff") {
                return Ok(Self::Tiff);
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
            Self::Tiff => Self::Tiff,
            Self::Jpeg | Self::WebP | Self::Gif => Self::Png,
        }
    }

    /// File extension for this format.
    pub fn extension(&self) -> &str {
        match self {
            Self::Png => ".png",
            Self::Bmp => ".bmp",
            Self::Jpeg => ".jpg",
            Self::WebP => ".webp",
            Self::Gif => ".gif",
            Self::Tiff => ".tiff",
        }
    }

    /// Whether this format is lossy (stego output will differ from input format).
    pub fn is_lossy(&self) -> bool {
        matches!(self, Self::Jpeg | Self::WebP | Self::Gif)
    }
}

/// Unified LSB steganography codec for all image formats.
pub struct LsbCodec {
    pub params: lsb::LsbParams,
    /// The image format to use for encoding the output.
    pub output_format: ImageFormat,
}

impl LsbCodec {
    pub fn new(params: lsb::LsbParams, output_format: ImageFormat) -> Self {
        Self {
            params,
            output_format,
        }
    }

    fn image_format(&self) -> image::ImageFormat {
        match self.output_format {
            ImageFormat::Png => image::ImageFormat::Png,
            ImageFormat::Bmp => image::ImageFormat::Bmp,
            ImageFormat::Jpeg => image::ImageFormat::Jpeg,
            ImageFormat::WebP => image::ImageFormat::WebP,
            ImageFormat::Gif => image::ImageFormat::Gif,
            ImageFormat::Tiff => image::ImageFormat::Tiff,
        }
    }

    /// Embed payload into a pre-decoded image (avoids double decoding).
    pub fn encode_image(&self, img: &image::DynamicImage, payload: &[u8]) -> Result<Vec<u8>> {
        let mut rgba = img.to_rgba8();
        lsb::embed_lsb(&mut rgba, payload, &self.params)?;
        let mut output = Vec::new();
        rgba.write_to(&mut Cursor::new(&mut output), self.image_format())?;
        Ok(output)
    }

    /// Extract payload from a pre-decoded image (avoids double decoding).
    pub fn decode_image(&self, img: &image::DynamicImage) -> Result<Vec<u8>> {
        let rgba = img.to_rgba8();
        lsb::extract_lsb(&rgba, &self.params)
    }
}

impl Capacity for LsbCodec {
    fn capacity(&self, cover: &[u8]) -> Result<usize> {
        let img = image::load_from_memory(cover)?;
        Ok(lsb::max_payload_bytes(
            img.width(),
            img.height(),
            self.params.bit_depth,
        ))
    }
}

impl Encoder for LsbCodec {
    fn encode(&self, cover: &[u8], payload: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(cover)?;
        self.encode_image(&img, payload)
    }
}

impl Decoder for LsbCodec {
    fn decode(&self, stego: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(stego)?;
        self.decode_image(&img)
    }
}
