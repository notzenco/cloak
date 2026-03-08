use std::io::Cursor;

use image::ImageFormat;

use super::lsb;
use crate::traits::{Capacity, Encoder};
use crate::Result;

/// WebP cover image support.
///
/// WebP is lossy, so the stego output is PNG (lossless) to preserve LSBs.
/// Extraction uses `PngCodec` since the output format is PNG.
pub struct WebpCodec;

impl Capacity for WebpCodec {
    fn capacity(&self, cover: &[u8]) -> Result<usize> {
        let img = image::load_from_memory(cover)?;
        Ok(lsb::max_payload_bytes(img.width(), img.height()))
    }
}

impl Encoder for WebpCodec {
    fn encode(&self, cover: &[u8], payload: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(cover)?;
        let mut rgba = img.to_rgba8();

        lsb::embed_lsb(&mut rgba, payload)?;

        // Encode as PNG to preserve LSBs (WebP would destroy them)
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
        let webp_codec = WebpCodec;
        let png_codec = PngCodec;

        let stego = webp_codec.encode(&cover, payload).unwrap();
        let extracted = png_codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn webp_capacity() {
        let cover = make_test_webp(10, 10);
        let codec = WebpCodec;

        let cap = codec.capacity(&cover).unwrap();
        assert_eq!(cap, 33);
    }

    #[test]
    fn webp_format_detection() {
        let cover = make_test_webp(4, 4);
        // WebP magic: RIFF at offset 0, WEBP at offset 8
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
