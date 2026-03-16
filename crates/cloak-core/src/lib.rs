pub mod analysis;
pub mod crypto;
pub mod error;
pub mod formats;
pub mod traits;

pub use error::CloakError;
pub use formats::ImageFormat;
pub use formats::LsbCodec;
pub use formats::lsb::LsbParams;
pub use traits::{Capacity, Decoder, Encoder};

pub type Result<T> = std::result::Result<T, CloakError>;

/// Options controlling embedding behavior.
#[derive(Debug, Clone, Default)]
pub struct EmbedOptions {
    /// Number of bits per channel (1-4). Default: 1.
    pub bit_depth: u8,
    /// Use randomized pixel traversal order. Default: false.
    pub randomized: bool,
}

impl EmbedOptions {
    fn lsb_params(&self, passphrase: Option<&str>, pixel_count: usize) -> Result<LsbParams> {
        let pixel_order = if self.randomized {
            let passphrase = passphrase.ok_or(CloakError::MissingPassphrase)?;
            formats::lsb::PixelOrder::Randomized(formats::lsb::generate_permutation(
                passphrase,
                pixel_count,
            )?)
        } else {
            formats::lsb::PixelOrder::Sequential
        };
        Ok(LsbParams {
            bit_depth: self.bit_depth.max(1),
            pixel_order,
            ..Default::default()
        })
    }

    fn lsb_params_no_rand(&self) -> LsbParams {
        LsbParams {
            bit_depth: self.bit_depth.max(1),
            pixel_order: formats::lsb::PixelOrder::Sequential,
            ..Default::default()
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

    // Decode once; the codec reuses the decoded image directly.
    let img = image::load_from_memory(cover)?;
    let pixel_count = (img.width() * img.height()) as usize;
    let mut params = options.lsb_params(Some(passphrase), pixel_count)?;
    params.length_mask = formats::lsb::derive_length_mask(passphrase);

    let output_format = format.output_format();
    formats::LsbCodec::new(params, output_format).encode_image(&img, &encrypted)
}

/// Extract and decrypt payload from a stego image.
pub fn extract(
    stego: &[u8],
    passphrase: &str,
    path: Option<&str>,
    options: &EmbedOptions,
) -> Result<Vec<u8>> {
    let format = ImageFormat::detect(stego, path)?;

    match format {
        ImageFormat::Jpeg | ImageFormat::WebP | ImageFormat::Gif => {
            return Err(CloakError::UnsupportedFormat(
                "stego images from lossy covers are PNG — extract from the PNG output".into(),
            ));
        }
        _ => {}
    }

    // Decode once; the codec reuses the decoded image directly.
    let img = image::load_from_memory(stego)?;
    let pixel_count = (img.width() * img.height()) as usize;
    let mut params = options.lsb_params(Some(passphrase), pixel_count)?;
    params.length_mask = formats::lsb::derive_length_mask(passphrase);

    let encrypted = formats::LsbCodec::new(params, format).decode_image(&img)?;
    crypto::decrypt(&encrypted, passphrase)
}

/// Get the maximum payload capacity in bytes (after encryption overhead).
pub fn capacity(cover: &[u8], path: Option<&str>, options: &EmbedOptions) -> Result<usize> {
    let format = ImageFormat::detect(cover, path)?;
    // Capacity doesn't need randomization
    let params = options.lsb_params_no_rand();
    let raw_capacity = formats::LsbCodec::new(params, format).capacity(cover)?;
    Ok(raw_capacity.saturating_sub(crypto::overhead()))
}
