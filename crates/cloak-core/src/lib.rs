pub mod analysis;
pub mod crypto;
pub mod error;
pub mod formats;
pub mod traits;

pub use error::CloakError;
pub use formats::ImageFormat;
pub use formats::lsb::LsbParams;
pub use traits::{Capacity, Decoder, Encoder};

pub type Result<T> = std::result::Result<T, CloakError>;

/// Options controlling embedding behavior.
#[derive(Debug, Clone)]
pub struct EmbedOptions {
    /// Number of bits per channel (1-4). Default: 1.
    pub bit_depth: u8,
}

impl Default for EmbedOptions {
    fn default() -> Self {
        Self { bit_depth: 1 }
    }
}

impl EmbedOptions {
    fn lsb_params(&self) -> LsbParams {
        LsbParams {
            bit_depth: self.bit_depth,
        }
    }
}

/// Embed encrypted payload into a cover image.
pub fn embed(
    cover: &[u8],
    data: &[u8],
    passphrase: &str,
    path: Option<&str>,
    options: &EmbedOptions,
) -> Result<Vec<u8>> {
    let encrypted = crypto::encrypt(data, passphrase)?;
    let format = ImageFormat::detect(cover, path)?;
    let params = options.lsb_params();
    match format {
        ImageFormat::Png => formats::png::PngCodec::new(params).encode(cover, &encrypted),
        ImageFormat::Bmp => formats::bmp::BmpCodec::new(params).encode(cover, &encrypted),
        ImageFormat::Jpeg => formats::jpeg::JpegCodec::new(params).encode(cover, &encrypted),
        ImageFormat::WebP => formats::webp::WebpCodec::new(params).encode(cover, &encrypted),
    }
}

/// Extract and decrypt payload from a stego image.
pub fn extract(
    stego: &[u8],
    passphrase: &str,
    path: Option<&str>,
    options: &EmbedOptions,
) -> Result<Vec<u8>> {
    let format = ImageFormat::detect(stego, path)?;
    let params = options.lsb_params();
    let encrypted = match format {
        ImageFormat::Png => formats::png::PngCodec::new(params).decode(stego)?,
        ImageFormat::Bmp => formats::bmp::BmpCodec::new(params).decode(stego)?,
        ImageFormat::Jpeg | ImageFormat::WebP => {
            return Err(CloakError::UnsupportedFormat(
                "stego images from lossy covers are PNG — extract from the PNG output".into(),
            ));
        }
    };
    crypto::decrypt(&encrypted, passphrase)
}

/// Get the maximum payload capacity in bytes (after encryption overhead).
pub fn capacity(cover: &[u8], path: Option<&str>, options: &EmbedOptions) -> Result<usize> {
    let format = ImageFormat::detect(cover, path)?;
    let params = options.lsb_params();
    let raw_capacity = match format {
        ImageFormat::Png => formats::png::PngCodec::new(params).capacity(cover)?,
        ImageFormat::Bmp => formats::bmp::BmpCodec::new(params).capacity(cover)?,
        ImageFormat::Jpeg => formats::jpeg::JpegCodec::new(params).capacity(cover)?,
        ImageFormat::WebP => formats::webp::WebpCodec::new(params).capacity(cover)?,
    };
    Ok(raw_capacity.saturating_sub(crypto::overhead()))
}
