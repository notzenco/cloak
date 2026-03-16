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
    /// XOR mask applied to the 4-byte length header.
    /// Defaults to `[0; 4]` (no masking) for backwards compatibility.
    pub length_mask: [u8; 4],
}

impl Default for LsbParams {
    fn default() -> Self {
        Self {
            bit_depth: 1,
            pixel_order: PixelOrder::default(),
            length_mask: [0; 4],
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
pub fn generate_permutation(passphrase: &str, n: usize) -> crate::Result<Vec<usize>> {
    use argon2::Argon2;
    use rand::SeedableRng;
    use rand::seq::SliceRandom;
    use rand_chacha::ChaCha20Rng;

    let mut seed = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), b"cloak-permutation", &mut seed)
        .map_err(|e| {
            crate::CloakError::CorruptedData(format!("argon2 key derivation failed: {e}"))
        })?;

    let mut rng = ChaCha20Rng::from_seed(seed);
    let mut indices: Vec<usize> = (0..n).collect();
    indices.shuffle(&mut rng);
    Ok(indices)
}

/// Derive a 4-byte XOR mask for the length header from a passphrase.
///
/// Uses Argon2id with a distinct salt so the mask is deterministic for a given
/// passphrase but independent of the permutation seed.
pub fn derive_length_mask(passphrase: &str) -> [u8; 4] {
    use argon2::Argon2;

    let mut buf = [0u8; 32];
    let _ =
        Argon2::default().hash_password_into(passphrase.as_bytes(), b"cloak-len-mask!!", &mut buf);
    [buf[0], buf[1], buf[2], buf[3]]
}

/// Block size for payload padding (bytes).
///
/// Payload is padded with random bytes to a multiple of this block size,
/// making the exact data boundary harder to detect via steganalysis.
const PAD_BLOCK: usize = 64;

/// Maximum payload bytes that can be embedded at a given bit depth.
pub fn max_payload_bytes(width: u32, height: u32, bit_depth: u8) -> usize {
    let total_bits = width as usize * height as usize * CHANNELS * bit_depth as usize;
    // Subtract 32 bits for the length header. The remaining capacity is for
    // the padded payload, but we report the usable (pre-padding) amount.
    let raw = total_bits.saturating_sub(32) / 8;
    // Usable capacity is the largest payload that, once padded, still fits.
    // padded_len = payload_len rounded up to PAD_BLOCK.
    // We want the largest payload_len such that round_up(payload_len) <= raw.
    let full_blocks = raw / PAD_BLOCK;
    if full_blocks == 0 {
        // Image is too small for even one block of padding; fall back to raw.
        raw
    } else {
        full_blocks * PAD_BLOCK
    }
}

/// Pad payload to a multiple of `PAD_BLOCK` with random bytes.
fn pad_payload(payload: &[u8]) -> Vec<u8> {
    use rand::RngCore;
    let padded_len = if payload.is_empty() {
        0
    } else {
        payload.len().div_ceil(PAD_BLOCK) * PAD_BLOCK
    };
    let mut padded = Vec::with_capacity(padded_len);
    padded.extend_from_slice(payload);
    if padded_len > payload.len() {
        let pad_count = padded_len - payload.len();
        let mut random_bytes = vec![0u8; pad_count];
        rand::thread_rng().fill_bytes(&mut random_bytes);
        padded.extend_from_slice(&random_bytes);
    }
    padded
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

    // The length header stores the real payload length (before padding).
    let raw_len = (payload.len() as u32).to_be_bytes();
    let len_bytes = [
        raw_len[0] ^ params.length_mask[0],
        raw_len[1] ^ params.length_mask[1],
        raw_len[2] ^ params.length_mask[2],
        raw_len[3] ^ params.length_mask[3],
    ];
    let padded = pad_payload(payload);
    let all_bytes: Vec<u8> = len_bytes.iter().chain(padded.iter()).copied().collect();

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

/// Read a single bit from the image at the given bit position.
///
/// This avoids materialising a full bit vector; callers advance `bit_idx`
/// themselves.
#[inline]
fn read_bit(rgba: &RgbaImage, params: &LsbParams, width: u32, bit_idx: usize) -> u8 {
    let bit_depth = params.bit_depth.max(1) as usize;
    let bits_per_pixel = CHANNELS * bit_depth;
    let value_mask = (1u8 << bit_depth) - 1;

    let pixel_i = bit_idx / bits_per_pixel;
    let within_pixel = bit_idx % bits_per_pixel;
    let channel = within_pixel / bit_depth;
    let bit_in_channel = within_pixel % bit_depth;

    let pixel_idx = match &params.pixel_order {
        PixelOrder::Sequential => pixel_i,
        PixelOrder::Randomized(perm) => perm[pixel_i],
    };
    let x = (pixel_idx % width as usize) as u32;
    let y = (pixel_idx / width as usize) as u32;
    let pixel = rgba.get_pixel(x, y);
    let low_bits = pixel[channel] & value_mask;
    (low_bits >> (bit_depth - 1 - bit_in_channel)) & 1
}

/// Extract payload from the low N bits of R, G, B channels of an RGBA image.
///
/// Unlike the previous implementation this extracts bytes on the fly without
/// building an intermediate `Vec<u8>` of individual bits, significantly
/// reducing memory usage for large images.
pub fn extract_lsb(rgba: &RgbaImage, params: &LsbParams) -> Result<Vec<u8>> {
    let bit_depth = params.bit_depth.max(1);
    let (width, height) = rgba.dimensions();
    let total_pixels = width as usize * height as usize;
    let bits_per_pixel = CHANNELS * bit_depth as usize;
    let total_bits = total_pixels * bits_per_pixel;

    if total_bits < 32 {
        return Err(CloakError::CorruptedData(
            "image too small to contain data".into(),
        ));
    }

    // --- Extract the 4-byte (32-bit) masked length header ---
    let mut bit_idx = 0usize;
    let mut accumulated = 0u32;
    for _ in 0..32 {
        let bit = read_bit(rgba, params, width, bit_idx);
        accumulated = (accumulated << 1) | bit as u32;
        bit_idx += 1;
    }

    // Unmask the length
    let masked_len = accumulated.to_be_bytes();
    let unmasked = [
        masked_len[0] ^ params.length_mask[0],
        masked_len[1] ^ params.length_mask[1],
        masked_len[2] ^ params.length_mask[2],
        masked_len[3] ^ params.length_mask[3],
    ];
    let length = u32::from_be_bytes(unmasked) as usize;

    let needed_bits = 32 + length * 8;
    if needed_bits > total_bits {
        return Err(CloakError::CorruptedData(format!(
            "claimed payload length {length} exceeds image capacity"
        )));
    }

    // --- Extract payload bytes directly ---
    let mut payload = Vec::with_capacity(length);
    for _ in 0..length {
        let mut byte = 0u8;
        for _ in 0..8 {
            let bit = read_bit(rgba, params, width, bit_idx);
            byte = (byte << 1) | bit;
            bit_idx += 1;
        }
        payload.push(byte);
    }

    Ok(payload)
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
        let p1 = generate_permutation("test", 100).unwrap();
        let p2 = generate_permutation("test", 100).unwrap();
        assert_eq!(p1, p2);
    }

    #[test]
    fn permutation_differs_for_different_passphrase() {
        let p1 = generate_permutation("pass1", 100).unwrap();
        let p2 = generate_permutation("pass2", 100).unwrap();
        assert_ne!(p1, p2);
    }

    #[test]
    fn randomized_roundtrip() {
        let mut rgba = make_test_rgba(32, 32);
        let perm = generate_permutation("secret", 32 * 32).unwrap();
        let params = LsbParams {
            bit_depth: 1,
            pixel_order: PixelOrder::Randomized(perm.clone()),
            ..Default::default()
        };
        let payload = b"randomized embedding test!";

        embed_lsb(&mut rgba, payload, &params).unwrap();
        let extracted = extract_lsb(&rgba, &params).unwrap();
        assert_eq!(extracted, payload);
    }

    #[test]
    fn wrong_passphrase_wrong_result() {
        let mut rgba = make_test_rgba(32, 32);
        let perm = generate_permutation("correct", 32 * 32).unwrap();
        let params = LsbParams {
            bit_depth: 1,
            pixel_order: PixelOrder::Randomized(perm),
            ..Default::default()
        };
        let payload = b"secret data";

        embed_lsb(&mut rgba, payload, &params).unwrap();

        // Try extracting with wrong permutation
        let wrong_perm = generate_permutation("wrong", 32 * 32).unwrap();
        let wrong_params = LsbParams {
            bit_depth: 1,
            pixel_order: PixelOrder::Randomized(wrong_perm),
            ..Default::default()
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

        let perm = generate_permutation("password", 32 * 32).unwrap();
        let rand_params = LsbParams {
            bit_depth: 1,
            pixel_order: PixelOrder::Randomized(perm),
            ..Default::default()
        };
        embed_lsb(&mut rgba_rand, payload, &rand_params).unwrap();

        // The raw pixel data should differ
        assert_ne!(rgba_seq.as_raw(), rgba_rand.as_raw());
    }
}
