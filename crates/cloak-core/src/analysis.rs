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
    /// RS analysis result.
    pub rs: Option<RsAnalysisResult>,
    /// Sample pairs analysis result.
    pub sample_pairs: Option<SamplePairsResult>,
    /// Shannon entropy per channel.
    pub entropy: Option<EntropyResult>,
}

/// RS (Regular-Singular) analysis result.
#[derive(Debug, Clone)]
pub struct RsAnalysisResult {
    /// Regular group count for mask m.
    pub r_m: f64,
    /// Singular group count for mask m.
    pub s_m: f64,
    /// Regular group count for mask -m.
    pub r_neg_m: f64,
    /// Singular group count for mask -m.
    pub s_neg_m: f64,
    /// Estimated embedding rate (0.0 = clean, 1.0 = fully embedded).
    pub estimated_rate: f64,
}

/// Sample pairs analysis result.
#[derive(Debug, Clone)]
pub struct SamplePairsResult {
    /// Estimated embedding rate.
    pub estimated_rate: f64,
    /// Total pixel pairs analyzed.
    pub total_pairs: u64,
    /// Pairs with close values (differ by <= 1).
    pub close_pairs: u64,
}

/// Shannon entropy analysis result.
#[derive(Debug, Clone)]
pub struct EntropyResult {
    /// Entropy of the red channel (0.0–8.0 bits).
    pub red: f64,
    /// Entropy of the green channel (0.0–8.0 bits).
    pub green: f64,
    /// Entropy of the blue channel (0.0–8.0 bits).
    pub blue: f64,
    /// Average entropy across all channels.
    pub average: f64,
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
    let pixel_count = width as u64 * height as u64;

    // Collect pixel data for all analyses
    let mut pixel_values: Vec<[u8; 3]> = Vec::with_capacity(pixel_count as usize);

    for y in 0..height {
        for x in 0..width {
            let pixel = rgba.get_pixel(x, y);
            for channel in 0..3 {
                let val = pixel[channel];
                histogram[val as usize] += 1;
            }
            pixel_values.push([pixel[0], pixel[1], pixel[2]]);
        }
    }

    let (chi_square, degrees_of_freedom) = chi_square_lsb(&histogram);
    let p_value = chi_square_p_value(chi_square, degrees_of_freedom);

    let rs = rs_analysis_from_pixels(&pixel_values, width);
    let sample_pairs = sample_pairs_from_pixels(&pixel_values);
    let entropy = compute_entropy(&pixel_values, pixel_count as usize);

    Ok(AnalysisResult {
        chi_square,
        p_value,
        histogram,
        pixel_count,
        width,
        height,
        rs: Some(rs),
        sample_pairs: Some(sample_pairs),
        entropy: Some(entropy),
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

/// RS analysis on pre-extracted pixel data.
fn rs_analysis_from_pixels(pixels: &[[u8; 3]], width: u32) -> RsAnalysisResult {
    let group_size = 4usize;
    let mut r_m = 0.0f64;
    let mut s_m = 0.0f64;
    let mut r_neg_m = 0.0f64;
    let mut s_neg_m = 0.0f64;
    let mut total_groups = 0.0f64;

    // Process each channel independently
    for channel in 0..3 {
        let values: Vec<u8> = pixels.iter().map(|p| p[channel]).collect();

        // Process in groups along rows
        let height = pixels.len() / width as usize;
        for y in 0..height {
            let row_start = y * width as usize;
            let row_end = row_start + width as usize;
            if row_end > values.len() {
                break;
            }
            let row = &values[row_start..row_end];

            let mut i = 0;
            while i + group_size <= row.len() {
                let group = &row[i..i + group_size];
                let original_smoothness = smoothness(group);

                // Apply mask m = [0, 1, 1, 0]
                let flipped_m = apply_mask(group, &[0, 1, 1, 0]);
                let smooth_m = smoothness(&flipped_m);

                if smooth_m > original_smoothness {
                    r_m += 1.0;
                } else if smooth_m < original_smoothness {
                    s_m += 1.0;
                }

                // Apply mask -m = [0, -1, -1, 0]
                let flipped_neg_m = apply_mask(group, &[0, -1, -1, 0]);
                let smooth_neg_m = smoothness(&flipped_neg_m);

                if smooth_neg_m > original_smoothness {
                    r_neg_m += 1.0;
                } else if smooth_neg_m < original_smoothness {
                    s_neg_m += 1.0;
                }

                total_groups += 1.0;
                i += group_size;
            }
        }
    }

    // Normalize
    if total_groups > 0.0 {
        r_m /= total_groups;
        s_m /= total_groups;
        r_neg_m /= total_groups;
        s_neg_m /= total_groups;
    }

    // Estimate embedding rate using quadratic formula (Fridrich et al.)
    let estimated_rate = estimate_rs_rate(r_m, s_m, r_neg_m, s_neg_m);

    RsAnalysisResult {
        r_m,
        s_m,
        r_neg_m,
        s_neg_m,
        estimated_rate,
    }
}

/// Smoothness of a group: sum of absolute differences between adjacent values.
fn smoothness(group: &[u8]) -> f64 {
    let mut s = 0.0;
    for i in 1..group.len() {
        s += (group[i] as f64 - group[i - 1] as f64).abs();
    }
    s
}

/// Apply RS flipping mask to a pixel group.
/// mask values: 0 = no change, 1 = flip LSB (positive), -1 = flip LSB (negative/inverse)
fn apply_mask(group: &[u8], mask: &[i8]) -> Vec<u8> {
    group
        .iter()
        .zip(mask.iter())
        .map(|(&val, &m)| match m {
            1 => flip_lsb(val),
            -1 => flip_lsb_neg(val),
            _ => val,
        })
        .collect()
}

/// Positive LSB flip: even→odd+1, odd→even-1 (toggle LSB)
fn flip_lsb(val: u8) -> u8 {
    val ^ 1
}

/// Negative LSB flip: swap 2k↔2k+1 but in reverse direction
/// For RS analysis: -1 operation is (val + 1) ^ 1 - 1 when val is even,
/// which gives the inverse permutation.
fn flip_lsb_neg(val: u8) -> u8 {
    if val == 255 {
        254
    } else if val == 0 {
        1
    } else {
        // -1 flipping: shift by 1 then flip
        ((val as i16 + 1) ^ 1).saturating_sub(1) as u8
    }
}

/// Estimate RS embedding rate from group statistics.
fn estimate_rs_rate(r_m: f64, s_m: f64, r_neg_m: f64, s_neg_m: f64) -> f64 {
    // Quadratic equation from Fridrich et al.
    // d0 = R_m - S_m, d1 = R_{-m} - S_{-m}
    let d0 = r_m - s_m;
    let d1 = r_neg_m - s_neg_m;

    // If d0 and d1 are close, low embedding
    if (d0 - d1).abs() < 1e-10 {
        return 0.0;
    }

    // Simplified estimation: ratio-based
    // When d0 ≈ d1, no embedding. When d0 → 0, full embedding.
    let rate = if d1.abs() < 1e-10 {
        0.0
    } else {
        // Use the relationship between R-S differences
        let x = (d0 / d1 - 1.0).abs();
        // Approximate: x ≈ 2p where p is embedding rate
        (x / 2.0).clamp(0.0, 1.0)
    };

    rate.clamp(0.0, 1.0)
}

/// Sample pairs analysis on pre-extracted pixel data.
fn sample_pairs_from_pixels(pixels: &[[u8; 3]]) -> SamplePairsResult {
    let mut total_pairs = 0u64;
    let mut close_pairs = 0u64;

    // Dumitrescu-Wu-Wang framework
    // Analyze consecutive pixel pairs per channel
    let mut p_even_even = 0u64; // both even
    let mut p_even_odd = 0u64; // first even, second odd
    let mut p_odd_even = 0u64; // first odd, second even
    let mut p_odd_odd = 0u64; // both odd

    for channel in 0..3 {
        for i in 0..pixels.len().saturating_sub(1) {
            let v1 = pixels[i][channel];
            let v2 = pixels[i + 1][channel];

            let diff = (v1 as i16 - v2 as i16).unsigned_abs();
            total_pairs += 1;
            if diff <= 1 {
                close_pairs += 1;
            }

            let e1 = v1.is_multiple_of(2);
            let e2 = v2.is_multiple_of(2);
            match (e1, e2) {
                (true, true) => p_even_even += 1,
                (true, false) => p_even_odd += 1,
                (false, true) => p_odd_even += 1,
                (false, false) => p_odd_odd += 1,
            }
        }
    }

    // Estimate embedding rate from LSB pair statistics
    // In a clean image: p(even,odd) ≈ p(odd,even) and similar for p(even,even) ≈ p(odd,odd)
    // LSB embedding equalizes these distributions
    let estimated_rate = if total_pairs == 0 {
        0.0
    } else {
        let n = total_pairs as f64;
        let ee = p_even_even as f64 / n;
        let eo = p_even_odd as f64 / n;
        let oe = p_odd_even as f64 / n;
        let oo = p_odd_odd as f64 / n;

        // Trace statistic: measures how much the LSB pair distribution
        // deviates from the embedded distribution
        let diag_diff = (ee - oo).abs();
        let off_diff = (eo - oe).abs();

        // Combined asymmetry measure
        // Clean images have natural asymmetry; embedded images tend toward symmetry
        let asymmetry = diag_diff + off_diff;

        // Under full embedding, asymmetry → 0
        // Under no embedding, asymmetry is larger
        // We invert: lower asymmetry means higher embedding rate
        // Normalize against a typical clean-image asymmetry (~0.1-0.3)
        let rate = 1.0 - (asymmetry * 10.0).min(1.0);
        rate.clamp(0.0, 1.0)
    };

    SamplePairsResult {
        estimated_rate,
        total_pairs,
        close_pairs,
    }
}

/// Compute Shannon entropy per channel.
fn compute_entropy(pixels: &[[u8; 3]], pixel_count: usize) -> EntropyResult {
    if pixel_count == 0 {
        return EntropyResult {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            average: 0.0,
        };
    }

    let mut histograms = [[0u64; 256]; 3];

    for pixel in pixels {
        for channel in 0..3 {
            histograms[channel][pixel[channel] as usize] += 1;
        }
    }

    let n = pixel_count as f64;
    let mut entropies = [0.0f64; 3];

    for (ch, hist) in histograms.iter().enumerate() {
        let mut h = 0.0;
        for &count in hist {
            if count > 0 {
                let p = count as f64 / n;
                h -= p * p.log2();
            }
        }
        entropies[ch] = h;
    }

    EntropyResult {
        red: entropies[0],
        green: entropies[1],
        blue: entropies[2],
        average: (entropies[0] + entropies[1] + entropies[2]) / 3.0,
    }
}

/// Chi-square test for LSB pairs.
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
            df += 1;
        }
    }

    (chi2, df)
}

/// Approximate chi-square p-value using Wilson-Hilferty normal approximation.
fn chi_square_p_value(chi2: f64, df: usize) -> f64 {
    if df == 0 {
        return 1.0;
    }

    let k = df as f64;
    let z = ((chi2 / k).powf(1.0 / 3.0) - (1.0 - 2.0 / (9.0 * k))) / (2.0 / (9.0 * k)).sqrt();
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
    }

    #[test]
    fn analyze_stego_image() {
        let cover = make_test_png(64, 64);
        let codec = crate::formats::png::PngCodec::default();
        let payload = vec![0xAB; 500];
        let stego = crate::traits::Encoder::encode(&codec, &cover, &payload).unwrap();

        let result = analyze_image(&stego).unwrap();
        assert!(result.chi_square >= 0.0);
    }

    #[test]
    fn histogram_sums_correctly() {
        let data = make_test_png(16, 16);
        let result = analyze_image(&data).unwrap();

        let total: u64 = result.histogram.iter().sum();
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

    #[test]
    fn rs_analysis_clean_low_rate() {
        let data = make_test_png(64, 64);
        let result = analyze_image(&data).unwrap();
        let rs = result.rs.unwrap();
        // Clean image should have low estimated rate
        assert!(rs.estimated_rate < 0.5, "RS rate {}", rs.estimated_rate);
    }

    #[test]
    fn rs_analysis_stego_detects() {
        let cover = make_test_png(64, 64);
        let codec = crate::formats::png::PngCodec::default();
        // Embed near-max payload to maximize LSB modification
        let cap = crate::traits::Capacity::capacity(&codec, &cover).unwrap();
        let payload = vec![0xAB; cap];
        let stego = crate::traits::Encoder::encode(&codec, &cover, &payload).unwrap();

        let result = analyze_image(&stego).unwrap();
        let rs = result.rs.unwrap();
        // Stego image should have higher estimated rate
        assert!(rs.estimated_rate > 0.0, "RS rate {}", rs.estimated_rate);
    }

    #[test]
    fn sample_pairs_has_result() {
        let data = make_test_png(64, 64);
        let result = analyze_image(&data).unwrap();
        let sp = result.sample_pairs.unwrap();
        assert!(sp.total_pairs > 0);
        assert!(sp.estimated_rate >= 0.0 && sp.estimated_rate <= 1.0);
    }

    #[test]
    fn sample_pairs_stego() {
        let cover = make_test_png(64, 64);
        let codec = crate::formats::png::PngCodec::default();
        let cap = crate::traits::Capacity::capacity(&codec, &cover).unwrap();
        let payload = vec![0xAB; cap];
        let stego = crate::traits::Encoder::encode(&codec, &cover, &payload).unwrap();

        let result = analyze_image(&stego).unwrap();
        let sp = result.sample_pairs.unwrap();
        assert!(sp.total_pairs > 0);
    }

    #[test]
    fn entropy_uniform_image_zero() {
        // All pixels same color → entropy 0
        let img = RgbaImage::from_fn(16, 16, |_, _| image::Rgba([128, 128, 128, 255]));
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();

        let result = analyze_image(&buf).unwrap();
        let ent = result.entropy.unwrap();
        assert!((ent.red - 0.0).abs() < 0.001);
        assert!((ent.green - 0.0).abs() < 0.001);
        assert!((ent.blue - 0.0).abs() < 0.001);
    }

    #[test]
    fn entropy_natural_image_range() {
        let data = make_test_png(64, 64);
        let result = analyze_image(&data).unwrap();
        let ent = result.entropy.unwrap();
        // Pseudo-random pattern should have moderate-high entropy
        assert!(
            ent.average > 3.0 && ent.average <= 8.0,
            "entropy {} out of expected range",
            ent.average
        );
    }

    #[test]
    fn entropy_random_image_high() {
        // Random pixel values → entropy near 8.0
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let img = RgbaImage::from_fn(64, 64, |_, _| {
            let mut bytes = [0u8; 3];
            rng.fill_bytes(&mut bytes);
            image::Rgba([bytes[0], bytes[1], bytes[2], 255])
        });
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();

        let result = analyze_image(&buf).unwrap();
        let ent = result.entropy.unwrap();
        assert!(ent.average > 7.0, "entropy {} expected > 7.0", ent.average);
    }
}
