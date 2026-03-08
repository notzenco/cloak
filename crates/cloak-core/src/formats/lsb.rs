use image::RgbaImage;

use crate::{CloakError, Result};

/// Bits embedded per pixel (R, G, B LSBs).
const BITS_PER_PIXEL: usize = 3;

/// Maximum payload bytes that can be embedded in an image of the given dimensions.
pub fn max_payload_bytes(width: u32, height: u32) -> usize {
    let total_bits = width as usize * height as usize * BITS_PER_PIXEL;
    total_bits.saturating_sub(32) / 8
}

/// Embed payload into the LSBs of an RGBA image.
///
/// Writes a 32-bit big-endian length prefix followed by payload data
/// into the least significant bit of R, G, B channels.
pub fn embed_lsb(rgba: &mut RgbaImage, payload: &[u8]) -> Result<()> {
    let (width, height) = rgba.dimensions();
    let max = max_payload_bytes(width, height);

    if payload.len() > max {
        return Err(CloakError::PayloadTooLarge {
            needed: payload.len(),
            capacity: max,
        });
    }

    let len_bytes = (payload.len() as u32).to_be_bytes();
    let all_bytes: Vec<u8> = len_bytes.iter().chain(payload.iter()).copied().collect();

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
                let byte_pos = bit_idx / 8;
                let bit_pos = 7 - (bit_idx % 8); // MSB first
                let bit = (all_bytes[byte_pos] >> bit_pos) & 1;
                pixel[channel as usize] = (pixel[channel as usize] & 0xFE) | bit;
                bit_idx += 1;
            }
        }
    }

    Ok(())
}

/// Extract payload from the LSBs of an RGBA image.
///
/// Reads a 32-bit big-endian length prefix, then extracts that many payload bytes.
pub fn extract_lsb(rgba: &RgbaImage) -> Result<Vec<u8>> {
    let (width, height) = rgba.dimensions();
    let total_pixels = width as usize * height as usize;

    if total_pixels * BITS_PER_PIXEL < 32 {
        return Err(CloakError::CorruptedData(
            "image too small to contain data".into(),
        ));
    }

    let mut bits = Vec::with_capacity(total_pixels * BITS_PER_PIXEL);
    for y in 0..height {
        for x in 0..width {
            let pixel = rgba.get_pixel(x, y);
            for channel in 0..3 {
                bits.push(pixel[channel] & 1);
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
