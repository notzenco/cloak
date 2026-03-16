pub mod analysis;
pub mod crypto;
pub mod error;
pub mod formats;
pub mod traits;

pub use analysis::{
    EzStegoResult, GifAnalysisResult, GifShuffleResult, PaletteAnomalyResult,
    PaletteChiSquareResult,
};
pub use error::CloakError;
pub use formats::ImageFormat;
pub use formats::LsbCodec;
pub use formats::lsb::{CapacityBreakdown, LsbParams};
pub use traits::{Capacity, Decoder, Encoder};

pub type Result<T> = std::result::Result<T, CloakError>;

/// Options controlling embedding behavior.
#[derive(Debug, Clone, Default)]
pub struct EmbedOptions {
    /// Number of bits per channel (1-4). Default: 1.
    pub bit_depth: u8,
    /// Use randomized pixel traversal order. Default: false.
    pub randomized: bool,
    /// Use parallel embedding (requires `parallel` feature). Default: false.
    pub parallel: bool,
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
    let codec = formats::LsbCodec::new(params, output_format);
    if options.parallel {
        codec.encode_image_parallel(&img, &encrypted)
    } else {
        codec.encode_image(&img, &encrypted)
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

/// Detailed capacity report for an image.
#[derive(Debug, Clone)]
pub struct CapacityReport {
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub bit_depth: u8,
    pub breakdown: CapacityBreakdown,
    pub crypto_overhead: usize,
    pub usable_bytes: usize,
}

/// Compute a detailed capacity report for a cover image.
pub fn capacity_report(
    cover: &[u8],
    path: Option<&str>,
    options: &EmbedOptions,
) -> Result<CapacityReport> {
    let format = ImageFormat::detect(cover, path)?;
    let img = image::load_from_memory(cover)?;
    let (width, height) = (img.width(), img.height());
    let bit_depth = options.bit_depth.max(1);
    let breakdown = formats::lsb::capacity_breakdown(width, height, bit_depth);
    let crypto_overhead = crypto::overhead();
    let usable_bytes = breakdown.usable_bytes.saturating_sub(crypto_overhead);
    Ok(CapacityReport {
        width,
        height,
        format,
        bit_depth,
        breakdown,
        crypto_overhead,
        usable_bytes,
    })
}

/// Get the maximum payload capacity in bytes (after encryption overhead).
pub fn capacity(cover: &[u8], path: Option<&str>, options: &EmbedOptions) -> Result<usize> {
    Ok(capacity_report(cover, path, options)?.usable_bytes)
}
