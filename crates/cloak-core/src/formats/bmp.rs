use std::io::Cursor;

use image::ImageFormat;

use super::lsb;
use crate::traits::{Capacity, Decoder, Encoder};
use crate::Result;

/// LSB steganography for BMP images.
pub struct BmpCodec;

impl Capacity for BmpCodec {
    fn capacity(&self, cover: &[u8]) -> Result<usize> {
        let img = image::load_from_memory(cover)?;
        Ok(lsb::max_payload_bytes(img.width(), img.height()))
    }
}

impl Encoder for BmpCodec {
    fn encode(&self, cover: &[u8], payload: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(cover)?;
        let mut rgba = img.to_rgba8();

        lsb::embed_lsb(&mut rgba, payload)?;

        let mut output = Vec::new();
        rgba.write_to(&mut Cursor::new(&mut output), ImageFormat::Bmp)?;
        Ok(output)
    }
}

impl Decoder for BmpCodec {
    fn decode(&self, stego: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(stego)?;
        let rgba = img.to_rgba8();
        lsb::extract_lsb(&rgba)
    }
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
        assert!(matches!(result, Err(crate::CloakError::PayloadTooLarge { .. })));
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
