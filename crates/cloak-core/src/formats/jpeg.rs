use std::io::Cursor;

use image::ImageFormat;

use super::lsb::{self, LsbParams};
use crate::traits::{Capacity, Encoder};
use crate::Result;

/// JPEG cover image support.
///
/// JPEG is lossy, so the stego output is PNG (lossless) to preserve LSBs.
#[derive(Default)]
pub struct JpegCodec {
    pub params: LsbParams,
}

impl JpegCodec {
    pub fn new(params: LsbParams) -> Self {
        Self { params }
    }
}

impl Capacity for JpegCodec {
    fn capacity(&self, cover: &[u8]) -> Result<usize> {
        let img = image::load_from_memory(cover)?;
        Ok(lsb::max_payload_bytes(
            img.width(),
            img.height(),
            self.params.bit_depth,
        ))
    }
}

impl Encoder for JpegCodec {
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
    use image::RgbImage;

    fn make_test_jpeg(width: u32, height: u32) -> Vec<u8> {
        let img = RgbImage::from_fn(width, height, |x, y| {
            let r = ((x * 17 + y * 31) % 256) as u8;
            let g = ((x * 41 + y * 13) % 256) as u8;
            let b = ((x * 7 + y * 53) % 256) as u8;
            image::Rgb([r, g, b])
        });
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
            .unwrap();
        buf
    }

    #[test]
    fn jpeg_to_png_roundtrip() {
        let cover = make_test_jpeg(64, 64);
        let payload = b"JPEG steganography!";
        let jpeg_codec = JpegCodec::default();
        let png_codec = PngCodec::default();

        let stego = jpeg_codec.encode(&cover, payload).unwrap();
        let extracted = png_codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn jpeg_capacity() {
        let cover = make_test_jpeg(10, 10);
        let codec = JpegCodec::default();

        let cap = codec.capacity(&cover).unwrap();
        assert_eq!(cap, 33);
    }

    #[test]
    fn jpeg_format_detection() {
        let cover = make_test_jpeg(4, 4);
        assert_eq!(cover[0], 0xFF);
        assert_eq!(cover[1], 0xD8);
        assert_eq!(cover[2], 0xFF);

        let format = crate::formats::ImageFormat::detect(&cover, None).unwrap();
        assert_eq!(format, crate::formats::ImageFormat::Jpeg);
    }

    #[test]
    fn jpeg_extension_detection() {
        let format = crate::formats::ImageFormat::detect(&[], Some("photo.jpg")).unwrap();
        assert_eq!(format, crate::formats::ImageFormat::Jpeg);

        let format = crate::formats::ImageFormat::detect(&[], Some("photo.jpeg")).unwrap();
        assert_eq!(format, crate::formats::ImageFormat::Jpeg);
    }
}
