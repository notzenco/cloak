use std::io::Cursor;

use image::{GenericImageView, ImageFormat};

use crate::traits::{Capacity, Decoder, Encoder};
use crate::{CloakError, Result};

/// LSB steganography for BMP images.
///
/// Same algorithm as PNG: embeds in LSB of R, G, B channels.
/// 32-bit big-endian length prefix followed by payload data.
pub struct BmpCodec;

impl BmpCodec {
    const BITS_PER_PIXEL: usize = 3;

    fn max_payload_bytes(width: u32, height: u32) -> usize {
        let total_bits = width as usize * height as usize * Self::BITS_PER_PIXEL;
        total_bits.saturating_sub(32) / 8
    }
}

impl Capacity for BmpCodec {
    fn capacity(&self, cover: &[u8]) -> Result<usize> {
        let img = image::load_from_memory(cover)?;
        Ok(BmpCodec::max_payload_bytes(img.width(), img.height()))
    }
}

impl Encoder for BmpCodec {
    fn encode(&self, cover: &[u8], payload: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(cover)?;
        let (width, height) = img.dimensions();
        let max = BmpCodec::max_payload_bytes(width, height);

        if payload.len() > max {
            return Err(CloakError::PayloadTooLarge {
                needed: payload.len(),
                capacity: max,
            });
        }

        let mut rgba = img.to_rgba8();

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
                for channel in 0..3 {
                    if bit_idx >= total_bits {
                        break 'outer;
                    }
                    let byte_pos = bit_idx / 8;
                    let bit_pos = 7 - (bit_idx % 8);
                    let bit = (all_bytes[byte_pos] >> bit_pos) & 1;
                    pixel[channel] = (pixel[channel] & 0xFE) | bit;
                    bit_idx += 1;
                }
            }
        }

        let mut output = Vec::new();
        rgba.write_to(&mut Cursor::new(&mut output), ImageFormat::Bmp)?;

        Ok(output)
    }
}

impl Decoder for BmpCodec {
    fn decode(&self, stego: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(stego)?;
        let (width, height) = img.dimensions();
        let rgba = img.to_rgba8();

        let total_pixels = width as usize * height as usize;
        if total_pixels * BmpCodec::BITS_PER_PIXEL < 32 {
            return Err(CloakError::CorruptedData(
                "image too small to contain data".into(),
            ));
        }

        let mut bits = Vec::with_capacity(total_pixels * BmpCodec::BITS_PER_PIXEL);
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
    use image::RgbaImage;

    fn make_test_bmp(width: u32, height: u32) -> Vec<u8> {
        let img = RgbaImage::from_fn(width, height, |x, y| {
            let r = ((x * 17 + y * 31) % 256) as u8;
            let g = ((x * 41 + y * 13) % 256) as u8;
            let b = ((x * 7 + y * 53) % 256) as u8;
            image::Rgba([r, g, b, 255])
        });
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Bmp)
            .unwrap();
        buf
    }

    #[test]
    fn roundtrip() {
        let cover = make_test_bmp(64, 64);
        let payload = b"BMP steganography works!";
        let codec = BmpCodec;

        let stego = codec.encode(&cover, payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn capacity_check() {
        let cover = make_test_bmp(10, 10);
        let codec = BmpCodec;

        let cap = codec.capacity(&cover).unwrap();
        assert_eq!(cap, 33);
    }

    #[test]
    fn payload_too_large() {
        let cover = make_test_bmp(4, 4);
        let codec = BmpCodec;

        let cap = codec.capacity(&cover).unwrap();
        let payload = vec![0xAA; cap + 1];

        let result = codec.encode(&cover, &payload);
        assert!(matches!(result, Err(CloakError::PayloadTooLarge { .. })));
    }

    #[test]
    fn max_capacity_payload() {
        let cover = make_test_bmp(32, 32);
        let codec = BmpCodec;

        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap).map(|i| (i % 256) as u8).collect();

        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }
}
