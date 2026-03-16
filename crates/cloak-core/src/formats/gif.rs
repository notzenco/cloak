#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::ImageFormat;

    use crate::formats::lsb::LsbParams;
    use crate::formats::{ImageFormat as CloakFormat, LsbCodec};
    use crate::traits::{Capacity, Decoder, Encoder};
    use image::RgbaImage;

    fn make_test_gif(width: u32, height: u32) -> Vec<u8> {
        let img = RgbaImage::from_fn(width, height, |x, y| {
            let r = ((x * 17 + y * 31) % 256) as u8;
            let g = ((x * 41 + y * 13) % 256) as u8;
            let b = ((x * 7 + y * 53) % 256) as u8;
            image::Rgba([r, g, b, 255])
        });
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Gif)
            .unwrap();
        buf
    }

    #[test]
    fn gif_to_png_roundtrip() {
        let cover = make_test_gif(64, 64);
        let payload = b"GIF steganography!";
        let gif_codec = LsbCodec::new(LsbParams::default(), CloakFormat::Png);
        let png_codec = LsbCodec::new(LsbParams::default(), CloakFormat::Png);

        let stego = gif_codec.encode(&cover, payload).unwrap();
        let extracted = png_codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn gif_capacity() {
        let cover = make_test_gif(10, 10);
        let codec = LsbCodec::new(LsbParams::default(), CloakFormat::Png);

        let cap = codec.capacity(&cover).unwrap();
        assert_eq!(cap, 33);
    }

    #[test]
    fn gif_format_detection() {
        let cover = make_test_gif(4, 4);
        let format = CloakFormat::detect(&cover, None).unwrap();
        assert_eq!(format, CloakFormat::Gif);
    }

    #[test]
    fn gif_extension_detection() {
        let format = CloakFormat::detect(&[], Some("anim.gif")).unwrap();
        assert_eq!(format, CloakFormat::Gif);
    }

    #[test]
    fn payload_too_large() {
        let cover = make_test_gif(4, 4);
        let codec = LsbCodec::new(LsbParams::default(), CloakFormat::Png);

        let cap = codec.capacity(&cover).unwrap();
        let payload = vec![0xAA; cap + 1];

        let result = codec.encode(&cover, &payload);
        assert!(matches!(
            result,
            Err(crate::CloakError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn max_capacity_payload() {
        let cover = make_test_gif(32, 32);
        let gif_codec = LsbCodec::new(LsbParams::default(), CloakFormat::Png);
        let png_codec = LsbCodec::new(LsbParams::default(), CloakFormat::Png);

        let cap = gif_codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap).map(|i| (i % 256) as u8).collect();

        let stego = gif_codec.encode(&cover, &payload).unwrap();
        let extracted = png_codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn multi_bit_roundtrip() {
        let cover = make_test_gif(32, 32);
        let params = LsbParams {
            bit_depth: 2,
            ..Default::default()
        };
        let gif_codec = LsbCodec::new(params.clone(), CloakFormat::Png);
        let png_codec = LsbCodec::new(params, CloakFormat::Png);

        let cap = gif_codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap.min(200)).map(|i| (i % 256) as u8).collect();

        let stego = gif_codec.encode(&cover, &payload).unwrap();
        let extracted = png_codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }
}
