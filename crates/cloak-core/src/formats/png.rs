use std::io::Cursor;

#[cfg(test)]
use image::RgbaImage;
use image::{GenericImageView, ImageFormat};

use crate::traits::{Capacity, Decoder, Encoder};
use crate::{CloakError, Result};

/// LSB steganography for PNG images.
///
/// Embeds data in the least significant bit of R, G, B channels (skips alpha).
/// The first 32 bits encode the payload length as a big-endian u32.
pub struct PngCodec;

impl PngCodec {
    /// Usable bits per pixel (R, G, B LSBs = 3 bits per pixel).
    const BITS_PER_PIXEL: usize = 3;

    fn usable_bits(width: u32, height: u32) -> usize {
        width as usize * height as usize * Self::BITS_PER_PIXEL
    }

    fn max_payload_bytes(width: u32, height: u32) -> usize {
        let total_bits = Self::usable_bits(width, height);
        // Reserve 32 bits for the length prefix
        total_bits.saturating_sub(32) / 8
    }
}

impl Capacity for PngCodec {
    fn capacity(&self, cover: &[u8]) -> Result<usize> {
        let img = image::load_from_memory(cover)?;
        Ok(PngCodec::max_payload_bytes(img.width(), img.height()))
    }
}

impl Encoder for PngCodec {
    fn encode(&self, cover: &[u8], payload: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(cover)?;
        let (width, height) = img.dimensions();
        let max = PngCodec::max_payload_bytes(width, height);

        if payload.len() > max {
            return Err(CloakError::PayloadTooLarge {
                needed: payload.len(),
                capacity: max,
            });
        }

        let mut rgba = img.to_rgba8();

        // Build the bit stream: 32-bit length prefix + payload
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
                // Embed in R, G, B channels (indices 0, 1, 2)
                for channel in 0..3 {
                    if bit_idx >= total_bits {
                        break 'outer;
                    }
                    let byte_pos = bit_idx / 8;
                    let bit_pos = 7 - (bit_idx % 8); // MSB first
                    let bit = (all_bytes[byte_pos] >> bit_pos) & 1;

                    pixel[channel] = (pixel[channel] & 0xFE) | bit;
                    bit_idx += 1;
                }
            }
        }

        // Encode back to PNG
        let mut output = Vec::new();
        rgba.write_to(&mut Cursor::new(&mut output), ImageFormat::Png)?;

        Ok(output)
    }
}

impl Decoder for PngCodec {
    fn decode(&self, stego: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(stego)?;
        let (width, height) = img.dimensions();
        let rgba = img.to_rgba8();

        let total_pixels = width as usize * height as usize;
        if total_pixels * PngCodec::BITS_PER_PIXEL < 32 {
            return Err(CloakError::CorruptedData(
                "image too small to contain data".into(),
            ));
        }

        // Extract all LSBs into a bit vector
        let mut bits = Vec::with_capacity(total_pixels * PngCodec::BITS_PER_PIXEL);
        for y in 0..height {
            for x in 0..width {
                let pixel = rgba.get_pixel(x, y);
                for channel in 0..3 {
                    bits.push(pixel[channel] & 1);
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
}

fn bits_to_u32(bits: &[u8]) -> u32 {
    let mut val = 0u32;
    for &bit in bits.iter().take(32) {
        val = (val << 1) | bit as u32;
    }
    val
}

fn bits_to_byte(bits: &[u8]) -> u8 {
    let mut val = 0u8;
    for &bit in bits.iter().take(8) {
        val = (val << 1) | bit;
    }
    val
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a minimal RGBA PNG image in memory.
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
    fn roundtrip() {
        let cover = make_test_png(64, 64);
        let payload = b"Hello, steganography!";
        let codec = PngCodec;

        let stego = codec.encode(&cover, payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn roundtrip_empty_payload() {
        let cover = make_test_png(16, 16);
        let codec = PngCodec;

        let stego = codec.encode(&cover, b"").unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert!(extracted.is_empty());
    }

    #[test]
    fn capacity_check() {
        let cover = make_test_png(10, 10);
        let codec = PngCodec;

        let cap = codec.capacity(&cover).unwrap();
        // 10*10 pixels * 3 bits/pixel = 300 bits, minus 32 for length = 268 bits = 33 bytes
        assert_eq!(cap, 33);
    }

    #[test]
    fn payload_too_large() {
        let cover = make_test_png(4, 4);
        let codec = PngCodec;

        let cap = codec.capacity(&cover).unwrap();
        let payload = vec![0xAA; cap + 1];

        let result = codec.encode(&cover, &payload);
        assert!(matches!(result, Err(CloakError::PayloadTooLarge { .. })));
    }

    #[test]
    fn max_capacity_payload() {
        let cover = make_test_png(32, 32);
        let codec = PngCodec;

        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap).map(|i| (i % 256) as u8).collect();

        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn binary_data_roundtrip() {
        let cover = make_test_png(64, 64);
        let payload: Vec<u8> = (0..=255).collect();
        let codec = PngCodec;

        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }
}
