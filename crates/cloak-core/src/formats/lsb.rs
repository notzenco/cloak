use image::RgbaImage;

use crate::{CloakError, Result};

/// Number of color channels used for embedding (R, G, B).
const CHANNELS: usize = 3;

/// Parameters controlling LSB embedding behavior.
#[derive(Debug, Clone)]
pub struct LsbParams {
    /// Number of bits per channel to use (1-4).
    pub bit_depth: u8,
}

impl Default for LsbParams {
    fn default() -> Self {
        Self { bit_depth: 1 }
    }
}

/// Maximum payload bytes that can be embedded at a given bit depth.
pub fn max_payload_bytes(width: u32, height: u32, bit_depth: u8) -> usize {
    let total_bits = width as usize * height as usize * CHANNELS * bit_depth as usize;
    // Reserve 32 bits for the length prefix
    total_bits.saturating_sub(32) / 8
}

/// Embed payload into the low N bits of R, G, B channels of an RGBA image.
///
/// Writes a 32-bit big-endian length prefix followed by payload data.
pub fn embed_lsb(rgba: &mut RgbaImage, payload: &[u8], params: &LsbParams) -> Result<()> {
    let bit_depth = params.bit_depth;
    assert!((1..=4).contains(&bit_depth), "bit_depth must be 1-4");

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

    let mask = !((1u8 << bit_depth) - 1); // e.g., 0xFE for 1-bit, 0xFC for 2-bit

    let mut bit_idx = 0usize;
    let total_bits = all_bytes.len() * 8;

    'outer: for y in 0..height {
        for x in 0..width {
            if bit_idx >= total_bits {
                break 'outer;
            }
            let pixel = rgba.get_pixel_mut(x, y);
            for channel in 0..3u8 {
                if bit_idx >= total_bits {
                    break 'outer;
                }
                // Extract `bit_depth` bits from the payload bit stream
                let mut val = 0u8;
                for b in 0..bit_depth {
                    if bit_idx < total_bits {
                        let byte_pos = bit_idx / 8;
                        let bit_pos = 7 - (bit_idx % 8); // MSB first
                        let bit = (all_bytes[byte_pos] >> bit_pos) & 1;
                        val |= bit << (bit_depth - 1 - b);
                        bit_idx += 1;
                    }
                }
                pixel[channel as usize] = (pixel[channel as usize] & mask) | val;
            }
        }
    }

    Ok(())
}

/// Extract payload from the low N bits of R, G, B channels of an RGBA image.
///
/// Reads a 32-bit big-endian length prefix, then extracts that many payload bytes.
pub fn extract_lsb(rgba: &RgbaImage, params: &LsbParams) -> Result<Vec<u8>> {
    let bit_depth = params.bit_depth;
    assert!((1..=4).contains(&bit_depth), "bit_depth must be 1-4");

    let (width, height) = rgba.dimensions();
    let total_pixels = width as usize * height as usize;
    let bits_per_pixel = CHANNELS * bit_depth as usize;

    if total_pixels * bits_per_pixel < 32 {
        return Err(CloakError::CorruptedData(
            "image too small to contain data".into(),
        ));
    }

    let value_mask = (1u8 << bit_depth) - 1; // e.g., 0x01 for 1-bit, 0x03 for 2-bit

    // Extract all low-order bits into a bit vector
    let mut bits = Vec::with_capacity(total_pixels * bits_per_pixel);
    for y in 0..height {
        for x in 0..width {
            let pixel = rgba.get_pixel(x, y);
            for channel in 0..3 {
                let low_bits = pixel[channel] & value_mask;
                // Unpack `bit_depth` bits MSB first
                for b in (0..bit_depth).rev() {
                    bits.push((low_bits >> b) & 1);
                }
            }
        }
    }

    // Read 32-bit length prefix
    let length = bits_to_u32(&bits[..32]) as usize;

    let needed_bits = 32 + length * 8;
    if needed_bits > bits.len() {
        return Err(CloakError::CorruptedData(format!(
            "claimed payload length {length} exceeds image capacity"
        )));
    }

    // Extract payload bytes
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
