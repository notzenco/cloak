pub mod analysis;
pub mod crypto;
pub mod error;
pub mod formats;
pub mod traits;

pub use error::CloakError;
pub use formats::ImageFormat;
pub use traits::{Capacity, Decoder, Encoder};

pub type Result<T> = std::result::Result<T, CloakError>;

/// Embed encrypted payload into a cover image.
///
/// Detects format automatically. Returns the stego image bytes.
pub fn embed(cover: &[u8], data: &[u8], passphrase: &str, path: Option<&str>) -> Result<Vec<u8>> {
    let encrypted = crypto::encrypt(data, passphrase)?;
    let format = ImageFormat::detect(cover, path)?;
    match format {
        ImageFormat::Png => formats::png::PngCodec.encode(cover, &encrypted),
        ImageFormat::Bmp => formats::bmp::BmpCodec.encode(cover, &encrypted),
        ImageFormat::Jpeg => formats::jpeg::JpegCodec.encode(cover, &encrypted),
    }
}

/// Extract and decrypt payload from a stego image.
pub fn extract(stego: &[u8], passphrase: &str, path: Option<&str>) -> Result<Vec<u8>> {
    let format = ImageFormat::detect(stego, path)?;
    let encrypted = match format {
        ImageFormat::Png => formats::png::PngCodec.decode(stego)?,
        ImageFormat::Bmp => formats::bmp::BmpCodec.decode(stego)?,
        ImageFormat::Jpeg => {
            return Err(CloakError::UnsupportedFormat(
                "stego images from JPEG covers are PNG — extract from the PNG output".into(),
            ));
        }
    };
    crypto::decrypt(&encrypted, passphrase)
}

/// Get the maximum payload capacity in bytes (after encryption overhead).
pub fn capacity(cover: &[u8], path: Option<&str>) -> Result<usize> {
    let format = ImageFormat::detect(cover, path)?;
    let raw_capacity = match format {
        ImageFormat::Png => formats::png::PngCodec.capacity(cover)?,
        ImageFormat::Bmp => formats::bmp::BmpCodec.capacity(cover)?,
        ImageFormat::Jpeg => formats::jpeg::JpegCodec.capacity(cover)?,
    };
    Ok(raw_capacity.saturating_sub(crypto::overhead()))
}
