use image::RgbaImage;

use crate::{CloakError, Result};

/// Number of color channels used for embedding (R, G, B).
const CHANNELS: usize = 3;

/// Parameters controlling LSB embedding behavior.
#[derive(Debug, Clone)]
pub struct LsbParams {
    /// Number of bits per channel to use (1-4).
    pub bit_depth: u8,
    /// Pixel traversal order.
    pub pixel_order: PixelOrder,
}

impl Default for LsbParams {
    fn default() -> Self {
        Self {
            bit_depth: 1,
            pixel_order: PixelOrder::default(),
        }
    }
}

/// Controls pixel traversal order during embed/extract.
#[derive(Debug, Clone, Default)]
pub enum PixelOrder {
    /// Sequential row-major order (default).
    #[default]
    Sequential,
    /// Randomized order derived from passphrase.
    Randomized(Vec<usize>),
}

/// Generate a deterministic permutation of `[0..n)` from a passphrase.
///
/// Uses Argon2id with a distinct salt to derive a seed, then ChaCha20Rng
/// for a Fisher-Yates shuffle.
pub fn generate_permutation(passphrase: &str, n: usize) -> Vec<usize> {
    use argon2::Argon2;
    use rand::seq::SliceRandom;
    use rand_chacha::ChaCha20Rng;
    use rand::SeedableRng;

    // Derive 32-byte seed from passphrase with a distinct salt
    let mut seed = [0u8; 32];
    Argon2::default()
        .hash_password_into(
            passphrase.as_bytes(),
            b"cloak-permutation",
            &mut seed,
        )
        .expect("argon2 key derivation failed");

    let mut rng = ChaCha20Rng::from_seed(seed);
    let mut indices: Vec<usize> = (0..n).collect();
    indices.shuffle(&mut rng);
    indices
}

/// Maximum payload bytes that can be embedded at a given bit depth.
pub fn max_payload_bytes(width: u32, height: u32, bit_depth: u8) -> usize {
    let total_bits = width as usize * height as usize * CHANNELS * bit_depth as usize;
    total_bits.saturating_sub(32) / 8
}

/// Embed payload into the low N bits of R, G, B channels of an RGBA image.
pub fn embed_lsb(rgba: &mut RgbaImage, payload: &[u8], params: &LsbParams) -> Result<()> {
    let bit_depth = params.bit_depth.max(1);
    let (width, height) = rgba.dimensions();
    let max = max_payload_bytes(width, height, bit_depth);

    if payload.len() > max {
        return Err(CloakError::PayloadTooLarge {
            needed: payload.len(),
            capacity: max,
        });
    }

    let len_bytes = (payload.len() as u32).to_be_bytes();
    let all_bytes: Vec<u8> = len_bytes.iter().chain(payload.iter()).copied().collect();

    let mask = !((1u8 << bit_depth) - 1);
    let total_pixels = (width * height) as usize;

    let mut bit_idx = 0usize;
    let total_bits = all_bytes.len() * 8;

    for i in 0..total_pixels {
        if bit_idx >= total_bits {
            break;
        }
        let pixel_idx = match &params.pixel_order {
            PixelOrder::Sequential => i,
            PixelOrder::Randomized(perm) => perm[i],
        };
        let x = (pixel_idx % width as usize) as u32;
        let y = (pixel_idx / width as usize) as u32;
        let pixel = rgba.get_pixel_mut(x, y);

        for channel in 0..3u8 {
            if bit_idx >= total_bits {
                break;
            }
            let mut val = 0u8;
            for b in 0..bit_depth {
                if bit_idx < total_bits {
                    let byte_pos = bit_idx / 8;
                    let bit_pos = 7 - (bit_idx % 8);
                    let bit = (all_bytes[byte_pos] >> bit_pos) & 1;
                    val |= bit << (bit_depth - 1 - b);
                    bit_idx += 1;
                }
            }
            pixel[channel as usize] = (pixel[channel as usize] & mask) | val;
        }
    }

    Ok(())
}

/// Extract payload from the low N bits of R, G, B channels of an RGBA image.
pub fn extract_lsb(rgba: &RgbaImage, params: &LsbParams) -> Result<Vec<u8>> {
    let bit_depth = params.bit_depth.max(1);
    let (width, height) = rgba.dimensions();
    let total_pixels = width as usize * height as usize;
    let bits_per_pixel = CHANNELS * bit_depth as usize;

    if total_pixels * bits_per_pixel < 32 {
        return Err(CloakError::CorruptedData(
            "image too small to contain data".into(),
        ));
    }

    let value_mask = (1u8 << bit_depth) - 1;

    let mut bits = Vec::with_capacity(total_pixels * bits_per_pixel);
    for i in 0..total_pixels {
        let pixel_idx = match &params.pixel_order {
            PixelOrder::Sequential => i,
            PixelOrder::Randomized(perm) => perm[i],
        };
        let x = (pixel_idx % width as usize) as u32;
        let y = (pixel_idx / width as usize) as u32;
        let pixel = rgba.get_pixel(x, y);

        for channel in 0..3 {
            let low_bits = pixel[channel] & value_mask;
            for b in (0..bit_depth).rev() {
                bits.push((low_bits >> b) & 1);
            }
        }
    }

    let length = bits_to_u32(&bits[..32]) as usize;

    let needed_bits = 32 + length * 8;
    if needed_bits > bits.len() {
        return Err(CloakError::CorruptedData(format!(
            "claimed payload length {length} exceeds image capacity"
        )));
    }

    let mut payload = Vec::with_capacity(length);
    for i in 0..length {
        let start = 32 + i * 8;
        let byte = bits_to_byte(&bits[start..start + 8]);
        payload.push(byte);
    }

    Ok(payload)
}

pub fn bits_to_u32(bits: &[u8]) -> u32 {
    let mut val = 0u32;
    for &bit in bits.iter().take(32) {
        val = (val << 1) | bit as u32;
    }
    val
}

pub fn bits_to_byte(bits: &[u8]) -> u8 {
    let mut val = 0u8;
    for &bit in bits.iter().take(8) {
        val = (val << 1) | bit;
    }
    val
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    fn make_test_rgba(width: u32, height: u32) -> RgbaImage {
        RgbaImage::from_fn(width, height, |x, y| {
            let r = ((x * 17 + y * 31) % 256) as u8;
            let g = ((x * 41 + y * 13) % 256) as u8;
            let b = ((x * 7 + y * 53) % 256) as u8;
            image::Rgba([r, g, b, 255])
        })
    }

    #[test]
    fn permutation_is_deterministic() {
        let p1 = generate_permutation("test", 100);
        let p2 = generate_permutation("test", 100);
        assert_eq!(p1, p2);
    }

    #[test]
    fn permutation_differs_for_different_passphrase() {
        let p1 = generate_permutation("pass1", 100);
        let p2 = generate_permutation("pass2", 100);
        assert_ne!(p1, p2);
    }

    #[test]
    fn randomized_roundtrip() {
        let mut rgba = make_test_rgba(32, 32);
        let perm = generate_permutation("secret", 32 * 32);
        let params = LsbParams {
            bit_depth: 1,
            pixel_order: PixelOrder::Randomized(perm.clone()),
        };
        let payload = b"randomized embedding test!";

        embed_lsb(&mut rgba, payload, &params).unwrap();
        let extracted = extract_lsb(&rgba, &params).unwrap();
        assert_eq!(extracted, payload);
    }

    #[test]
    fn wrong_passphrase_wrong_result() {
        let mut rgba = make_test_rgba(32, 32);
        let perm = generate_permutation("correct", 32 * 32);
        let params = LsbParams {
            bit_depth: 1,
            pixel_order: PixelOrder::Randomized(perm),
        };
        let payload = b"secret data";

        embed_lsb(&mut rgba, payload, &params).unwrap();

        // Try extracting with wrong permutation
        let wrong_perm = generate_permutation("wrong", 32 * 32);
        let wrong_params = LsbParams {
            bit_depth: 1,
            pixel_order: PixelOrder::Randomized(wrong_perm),
        };
        let result = extract_lsb(&rgba, &wrong_params);
        match result {
            Ok(data) => assert_ne!(data, payload),
            Err(_) => {} // Also acceptable (corrupted length)
        }
    }

    #[test]
    fn randomized_differs_from_sequential() {
        let mut rgba_seq = make_test_rgba(32, 32);
        let mut rgba_rand = rgba_seq.clone();

        let payload = b"test data for comparison";

        let seq_params = LsbParams::default();
        embed_lsb(&mut rgba_seq, payload, &seq_params).unwrap();

        let perm = generate_permutation("password", 32 * 32);
        let rand_params = LsbParams {
            bit_depth: 1,
            pixel_order: PixelOrder::Randomized(perm),
        };
        embed_lsb(&mut rgba_rand, payload, &rand_params).unwrap();

        // The raw pixel data should differ
        assert_ne!(rgba_seq.as_raw(), rgba_rand.as_raw());
    }
}
