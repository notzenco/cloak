#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::ImageFormat;

    use crate::formats::lsb::LsbParams;
    use crate::formats::{ImageFormat as CloakFormat, LsbCodec};
    use crate::traits::{Capacity, Decoder, Encoder};
    use image::RgbaImage;

    fn make_test_tiff(width: u32, height: u32) -> Vec<u8> {
        let img = RgbaImage::from_fn(width, height, |x, y| {
            let r = ((x * 17 + y * 31) % 256) as u8;
            let g = ((x * 41 + y * 13) % 256) as u8;
            let b = ((x * 7 + y * 53) % 256) as u8;
            image::Rgba([r, g, b, 255])
        });
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Tiff)
            .unwrap();
        buf
    }

    #[test]
    fn tiff_roundtrip() {
        let cover = make_test_tiff(64, 64);
        let payload = b"TIFF steganography!";
        let codec = LsbCodec::new(LsbParams::default(), CloakFormat::Tiff);

        let stego = codec.encode(&cover, payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn tiff_capacity() {
        let cover = make_test_tiff(10, 10);
        let codec = LsbCodec::new(LsbParams::default(), CloakFormat::Tiff);

        let cap = codec.capacity(&cover).unwrap();
        assert_eq!(cap, 33);
    }

    #[test]
    fn tiff_format_detection() {
        let cover = make_test_tiff(4, 4);
        let format = CloakFormat::detect(&cover, None).unwrap();
        assert_eq!(format, CloakFormat::Tiff);
    }

    #[test]
    fn tiff_extension_detection() {
        let format = CloakFormat::detect(&[], Some("photo.tiff")).unwrap();
        assert_eq!(format, CloakFormat::Tiff);

        let format = CloakFormat::detect(&[], Some("photo.tif")).unwrap();
        assert_eq!(format, CloakFormat::Tiff);
    }

    #[test]
    fn payload_too_large() {
        let cover = make_test_tiff(4, 4);
        let codec = LsbCodec::new(LsbParams::default(), CloakFormat::Tiff);

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
        let cover = make_test_tiff(32, 32);
        let codec = LsbCodec::new(LsbParams::default(), CloakFormat::Tiff);

        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap).map(|i| (i % 256) as u8).collect();

        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn multi_bit_roundtrip() {
        let cover = make_test_tiff(32, 32);
        let params = LsbParams {
            bit_depth: 2,
            ..Default::default()
        };
        let codec = LsbCodec::new(params, CloakFormat::Tiff);

        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap.min(200)).map(|i| (i % 256) as u8).collect();

        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn capacity_scales_with_bit_depth() {
        let cover = make_test_tiff(10, 10);
        let cap1 = LsbCodec::new(
            LsbParams {
                bit_depth: 1,
                ..Default::default()
            },
            CloakFormat::Tiff,
        )
        .capacity(&cover)
        .unwrap();
        let cap2 = LsbCodec::new(
            LsbParams {
                bit_depth: 2,
                ..Default::default()
            },
            CloakFormat::Tiff,
        )
        .capacity(&cover)
        .unwrap();
        let cap4 = LsbCodec::new(
            LsbParams {
                bit_depth: 4,
                ..Default::default()
            },
            CloakFormat::Tiff,
        )
        .capacity(&cover)
        .unwrap();

        assert!(cap2 > cap1);
        assert!(cap4 > cap2);
    }
}
