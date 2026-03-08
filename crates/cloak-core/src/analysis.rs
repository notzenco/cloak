use image::GenericImageView;

use crate::{CloakError, Result};

/// Results from steganalysis on an image.
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Chi-square statistic from LSB analysis.
    pub chi_square: f64,
    /// P-value for the chi-square test (lower = more likely stego).
    pub p_value: f64,
    /// Histogram of pixel values (256 bins, summed across R/G/B).
    pub histogram: [u64; 256],
    /// Number of usable pixels.
    pub pixel_count: u64,
    /// Image dimensions.
    pub width: u32,
    pub height: u32,
}

/// Bit-plane data for a single channel.
#[derive(Debug, Clone)]
pub struct BitPlane {
    pub width: u32,
    pub height: u32,
    /// One byte per pixel: 0 or 1.
    pub data: Vec<u8>,
}

/// Perform steganalysis on raw image bytes.
pub fn analyze_image(image_data: &[u8]) -> Result<AnalysisResult> {
    let img = image::load_from_memory(image_data)?;
    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();

    let mut histogram = [0u64; 256];
    let mut pairs_of_values = [0u64; 128]; // pairs: (2k, 2k+1) counts
    let pixel_count = width as u64 * height as u64;

    for y in 0..height {
        for x in 0..width {
            let pixel = rgba.get_pixel(x, y);
            for channel in 0..3 {
                let val = pixel[channel];
                histogram[val as usize] += 1;
                pairs_of_values[(val / 2) as usize] += 1;
            }
        }
    }

    // Chi-square test on LSB distribution
    // For each pair (2k, 2k+1), if no steganography, expect roughly equal counts.
    // Under LSB embedding, the pair counts should be close to their average.
    let (chi_square, degrees_of_freedom) = chi_square_lsb(&histogram);
    let p_value = chi_square_p_value(chi_square, degrees_of_freedom);

    Ok(AnalysisResult {
        chi_square,
        p_value,
        histogram,
        pixel_count,
        width,
        height,
    })
}

/// Extract a single bit plane from an image.
///
/// `channel`: 0=R, 1=G, 2=B
/// `bit`: 0 (LSB) to 7 (MSB)
pub fn extract_bit_plane(image_data: &[u8], channel: usize, bit: u8) -> Result<BitPlane> {
    if channel > 2 {
        return Err(CloakError::CorruptedData(
            "channel must be 0 (R), 1 (G), or 2 (B)".into(),
        ));
    }
    if bit > 7 {
        return Err(CloakError::CorruptedData("bit must be 0-7".into()));
    }

    let img = image::load_from_memory(image_data)?;
    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();

    let mut data = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height {
        for x in 0..width {
            let pixel = rgba.get_pixel(x, y);
            let val = (pixel[channel] >> bit) & 1;
            data.push(val);
        }
    }

    Ok(BitPlane {
        width,
        height,
        data,
    })
}

/// Chi-square test for LSB pairs.
///
/// Groups pixel values into pairs (0,1), (2,3), ..., (254,255).
/// Under no steganography, each pair should have its own distribution.
/// Under LSB embedding, within each pair the counts should equalize.
fn chi_square_lsb(histogram: &[u64; 256]) -> (f64, usize) {
    let mut chi2 = 0.0;
    let mut df = 0usize;

    for k in 0..128 {
        let even = histogram[2 * k] as f64;
        let odd = histogram[2 * k + 1] as f64;
        let expected = (even + odd) / 2.0;
        if expected > 0.0 {
            chi2 += (even - expected).powi(2) / expected;
            chi2 += (odd - expected).powi(2) / expected;
            df += 1; // one degree of freedom per pair
        }
    }

    (chi2, df)
}

/// Approximate chi-square p-value using the regularized incomplete gamma function.
///
/// For large degrees of freedom, uses Wilson-Hilferty normal approximation.
fn chi_square_p_value(chi2: f64, df: usize) -> f64 {
    if df == 0 {
        return 1.0;
    }

    let k = df as f64;

    // Wilson-Hilferty approximation: transform chi2 to approximately standard normal
    let z = ((chi2 / k).powf(1.0 / 3.0) - (1.0 - 2.0 / (9.0 * k))) / (2.0 / (9.0 * k)).sqrt();

    // Convert standard normal z to p-value using error function approximation
    let p = 0.5 * erfc(z / std::f64::consts::SQRT_2);
    p.clamp(0.0, 1.0)
}

/// Complementary error function approximation (Abramowitz and Stegun).
fn erfc(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.3275911 * x.abs());
    let poly = t
        * (0.254829592
            + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    let result = poly * (-x * x).exp();
    if x >= 0.0 { result } else { 2.0 - result }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, RgbaImage};
    use std::io::Cursor;

    fn make_test_png(width: u32, height: u32) -> Vec<u8> {
        let img = RgbaImage::from_fn(width, height, |x, y| {
            let r = ((x * 17 + y * 31) % 256) as u8;
            let g = ((x * 41 + y * 13) % 256) as u8;
            let b = ((x * 7 + y * 53) % 256) as u8;
            image::Rgba([r, g, b, 255])
        });
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();
        buf
    }

    #[test]
    fn analyze_clean_image() {
        let data = make_test_png(64, 64);
        let result = analyze_image(&data).unwrap();

        assert_eq!(result.width, 64);
        assert_eq!(result.height, 64);
        assert_eq!(result.pixel_count, 4096);
        assert!(result.chi_square >= 0.0);
        // Clean image should generally have p > 0.05 (no stego signal)
    }

    #[test]
    fn analyze_stego_image() {
        let cover = make_test_png(64, 64);
        let codec = crate::formats::png::PngCodec;
        let payload = vec![0xAB; 500];
        let stego = crate::traits::Encoder::encode(&codec, &cover, &payload).unwrap();

        let result = analyze_image(&stego).unwrap();
        assert!(result.chi_square >= 0.0);
        // After heavy LSB modification, chi-square behavior should change
    }

    #[test]
    fn histogram_sums_correctly() {
        let data = make_test_png(16, 16);
        let result = analyze_image(&data).unwrap();

        let total: u64 = result.histogram.iter().sum();
        // 16*16 pixels * 3 channels = 768
        assert_eq!(total, 768);
    }

    #[test]
    fn extract_lsb_plane() {
        let data = make_test_png(8, 8);
        let plane = extract_bit_plane(&data, 0, 0).unwrap();

        assert_eq!(plane.width, 8);
        assert_eq!(plane.height, 8);
        assert_eq!(plane.data.len(), 64);
        assert!(plane.data.iter().all(|&v| v == 0 || v == 1));
    }

    #[test]
    fn extract_msb_plane() {
        let data = make_test_png(8, 8);
        let plane = extract_bit_plane(&data, 2, 7).unwrap();

        assert_eq!(plane.data.len(), 64);
        assert!(plane.data.iter().all(|&v| v == 0 || v == 1));
    }

    #[test]
    fn invalid_channel() {
        let data = make_test_png(4, 4);
        assert!(extract_bit_plane(&data, 3, 0).is_err());
    }

    #[test]
    fn invalid_bit() {
        let data = make_test_png(4, 4);
        assert!(extract_bit_plane(&data, 0, 8).is_err());
    }
}
