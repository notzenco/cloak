use std::io::Cursor;

use image::ImageFormat;

use super::lsb::{self, LsbParams};
use crate::traits::{Capacity, Decoder, Encoder};
use crate::Result;

/// LSB steganography for PNG images.
#[derive(Default)]
pub struct PngCodec {
    pub params: LsbParams,
}

impl PngCodec {
    pub fn new(params: LsbParams) -> Self {
        Self { params }
    }
}

impl Capacity for PngCodec {
    fn capacity(&self, cover: &[u8]) -> Result<usize> {
        let img = image::load_from_memory(cover)?;
        Ok(lsb::max_payload_bytes(
            img.width(),
            img.height(),
            self.params.bit_depth,
        ))
    }
}

impl Encoder for PngCodec {
    fn encode(&self, cover: &[u8], payload: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(cover)?;
        let mut rgba = img.to_rgba8();

        lsb::embed_lsb(&mut rgba, payload, &self.params)?;

        let mut output = Vec::new();
        rgba.write_to(&mut Cursor::new(&mut output), ImageFormat::Png)?;
        Ok(output)
    }
}

impl Decoder for PngCodec {
    fn decode(&self, stego: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(stego)?;
        let rgba = img.to_rgba8();
        lsb::extract_lsb(&rgba, &self.params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

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
        let codec = PngCodec::default();

        let stego = codec.encode(&cover, payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn roundtrip_empty_payload() {
        let cover = make_test_png(16, 16);
        let codec = PngCodec::default();

        let stego = codec.encode(&cover, b"").unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert!(extracted.is_empty());
    }

    #[test]
    fn capacity_check() {
        let cover = make_test_png(10, 10);
        let codec = PngCodec::default();

        let cap = codec.capacity(&cover).unwrap();
        // 10*10 pixels * 3 bits/pixel = 300 bits, minus 32 for length = 268 bits = 33 bytes
        assert_eq!(cap, 33);
    }

    #[test]
    fn payload_too_large() {
        let cover = make_test_png(4, 4);
        let codec = PngCodec::default();

        let cap = codec.capacity(&cover).unwrap();
        let payload = vec![0xAA; cap + 1];

        let result = codec.encode(&cover, &payload);
        assert!(matches!(result, Err(crate::CloakError::PayloadTooLarge { .. })));
    }

    #[test]
    fn max_capacity_payload() {
        let cover = make_test_png(32, 32);
        let codec = PngCodec::default();

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
        let codec = PngCodec::default();

        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn multi_bit_roundtrip_2() {
        let cover = make_test_png(32, 32);
        let params = LsbParams { bit_depth: 2 };
        let codec = PngCodec::new(params);

        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap.min(200)).map(|i| (i % 256) as u8).collect();

        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn multi_bit_roundtrip_3() {
        let cover = make_test_png(32, 32);
        let params = LsbParams { bit_depth: 3 };
        let codec = PngCodec::new(params);

        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap.min(300)).map(|i| (i % 256) as u8).collect();

        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn multi_bit_roundtrip_4() {
        let cover = make_test_png(32, 32);
        let params = LsbParams { bit_depth: 4 };
        let codec = PngCodec::new(params);

        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap.min(400)).map(|i| (i % 256) as u8).collect();

        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn capacity_scales_with_bit_depth() {
        let cover = make_test_png(10, 10);
        let cap1 = PngCodec::new(LsbParams { bit_depth: 1 }).capacity(&cover).unwrap();
        let cap2 = PngCodec::new(LsbParams { bit_depth: 2 }).capacity(&cover).unwrap();
        let cap4 = PngCodec::new(LsbParams { bit_depth: 4 }).capacity(&cover).unwrap();

        assert!(cap2 > cap1);
        assert!(cap4 > cap2);
        // Roughly: cap2 ≈ 2*cap1, cap4 ≈ 4*cap1 (minus the fixed 32-bit header)
    }

    #[test]
    fn wrong_bit_depth_fails_extract() {
        let cover = make_test_png(32, 32);
        let embed_codec = PngCodec::new(LsbParams { bit_depth: 2 });
        let extract_codec = PngCodec::new(LsbParams { bit_depth: 1 });

        let payload = b"multi-bit test";
        let stego = embed_codec.encode(&cover, payload).unwrap();

        // Extracting with wrong bit depth should fail or return garbage
        let result = extract_codec.decode(&stego);
        match result {
            Ok(data) => assert_ne!(data, payload),
            Err(_) => {} // Also acceptable
        }
    }
}
