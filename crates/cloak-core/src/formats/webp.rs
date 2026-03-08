use std::io::Cursor;

use image::ImageFormat;

use super::lsb::{self, LsbParams};
use crate::traits::{Capacity, Encoder};
use crate::Result;

/// WebP cover image support.
///
/// WebP is lossy, so the stego output is PNG (lossless) to preserve LSBs.
#[derive(Default)]
pub struct WebpCodec {
    pub params: LsbParams,
}

impl WebpCodec {
    pub fn new(params: LsbParams) -> Self {
        Self { params }
    }
}

impl Capacity for WebpCodec {
    fn capacity(&self, cover: &[u8]) -> Result<usize> {
        let img = image::load_from_memory(cover)?;
        Ok(lsb::max_payload_bytes(
            img.width(),
            img.height(),
            self.params.bit_depth,
        ))
    }
}

impl Encoder for WebpCodec {
    fn encode(&self, cover: &[u8], payload: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(cover)?;
        let mut rgba = img.to_rgba8();

        lsb::embed_lsb(&mut rgba, payload, &self.params)?;

        let mut output = Vec::new();
        rgba.write_to(&mut Cursor::new(&mut output), ImageFormat::Png)?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::png::PngCodec;
    use crate::traits::Decoder;
    use image::RgbaImage;

    fn make_test_webp(width: u32, height: u32) -> Vec<u8> {
        let img = RgbaImage::from_fn(width, height, |x, y| {
            let r = ((x * 17 + y * 31) % 256) as u8;
            let g = ((x * 41 + y * 13) % 256) as u8;
            let b = ((x * 7 + y * 53) % 256) as u8;
            image::Rgba([r, g, b, 255])
        });
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::WebP)
            .unwrap();
        buf
    }

    #[test]
    fn webp_to_png_roundtrip() {
        let cover = make_test_webp(64, 64);
        let payload = b"WebP steganography!";
        let webp_codec = WebpCodec::default();
        let png_codec = PngCodec::default();

        let stego = webp_codec.encode(&cover, payload).unwrap();
        let extracted = png_codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn webp_capacity() {
        let cover = make_test_webp(10, 10);
        let codec = WebpCodec::default();

        let cap = codec.capacity(&cover).unwrap();
        assert_eq!(cap, 33);
    }

    #[test]
    fn webp_format_detection() {
        let cover = make_test_webp(4, 4);
        assert_eq!(&cover[..4], b"RIFF");
        assert_eq!(&cover[8..12], b"WEBP");

        let format = crate::formats::ImageFormat::detect(&cover, None).unwrap();
        assert_eq!(format, crate::formats::ImageFormat::WebP);
    }

    #[test]
    fn webp_extension_detection() {
        let format = crate::formats::ImageFormat::detect(&[], Some("photo.webp")).unwrap();
        assert_eq!(format, crate::formats::ImageFormat::WebP);
    }
}
