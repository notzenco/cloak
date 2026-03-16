#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::ImageFormat;

    use crate::formats::lsb::LsbParams;
    use crate::formats::{ImageFormat as CloakFormat, LsbCodec};
    use crate::traits::{Capacity, Decoder, Encoder};
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

    fn codec() -> LsbCodec {
        LsbCodec::new(LsbParams::default(), CloakFormat::Png)
    }

    #[test]
    fn roundtrip() {
        let cover = make_test_png(64, 64);
        let payload = b"Hello, steganography!";
        let codec = codec();
        let stego = codec.encode(&cover, payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();
        assert_eq!(extracted, payload);
    }

    #[test]
    fn roundtrip_empty_payload() {
        let cover = make_test_png(16, 16);
        let codec = codec();
        let stego = codec.encode(&cover, b"").unwrap();
        let extracted = codec.decode(&stego).unwrap();
        assert!(extracted.is_empty());
    }

    #[test]
    fn capacity_check() {
        let cover = make_test_png(10, 10);
        let codec = codec();
        let cap = codec.capacity(&cover).unwrap();
        assert_eq!(cap, 33);
    }

    #[test]
    fn payload_too_large() {
        let cover = make_test_png(4, 4);
        let codec = codec();
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
        let cover = make_test_png(32, 32);
        let codec = codec();
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
        let codec = codec();
        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();
        assert_eq!(extracted, payload);
    }

    #[test]
    fn multi_bit_roundtrip_2() {
        let cover = make_test_png(32, 32);
        let codec = LsbCodec::new(
            LsbParams {
                bit_depth: 2,
                ..Default::default()
            },
            CloakFormat::Png,
        );
        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap.min(200)).map(|i| (i % 256) as u8).collect();
        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();
        assert_eq!(extracted, payload);
    }

    #[test]
    fn multi_bit_roundtrip_3() {
        let cover = make_test_png(32, 32);
        let codec = LsbCodec::new(
            LsbParams {
                bit_depth: 3,
                ..Default::default()
            },
            CloakFormat::Png,
        );
        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap.min(300)).map(|i| (i % 256) as u8).collect();
        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();
        assert_eq!(extracted, payload);
    }

    #[test]
    fn multi_bit_roundtrip_4() {
        let cover = make_test_png(32, 32);
        let codec = LsbCodec::new(
            LsbParams {
                bit_depth: 4,
                ..Default::default()
            },
            CloakFormat::Png,
        );
        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap.min(400)).map(|i| (i % 256) as u8).collect();
        let stego = codec.encode(&cover, &payload).unwrap();
        let extracted = codec.decode(&stego).unwrap();
        assert_eq!(extracted, payload);
    }

    #[test]
    fn capacity_scales_with_bit_depth() {
        let cover = make_test_png(10, 10);
        let cap1 = LsbCodec::new(
            LsbParams {
                bit_depth: 1,
                ..Default::default()
            },
            CloakFormat::Png,
        )
        .capacity(&cover)
        .unwrap();
        let cap2 = LsbCodec::new(
            LsbParams {
                bit_depth: 2,
                ..Default::default()
            },
            CloakFormat::Png,
        )
        .capacity(&cover)
        .unwrap();
        let cap4 = LsbCodec::new(
            LsbParams {
                bit_depth: 4,
                ..Default::default()
            },
            CloakFormat::Png,
        )
        .capacity(&cover)
        .unwrap();
        assert!(cap2 > cap1);
        assert!(cap4 > cap2);
    }

    #[test]
    fn wrong_bit_depth_fails_extract() {
        let cover = make_test_png(32, 32);
        let embed_codec = LsbCodec::new(
            LsbParams {
                bit_depth: 2,
                ..Default::default()
            },
            CloakFormat::Png,
        );
        let extract_codec = LsbCodec::new(
            LsbParams {
                bit_depth: 1,
                ..Default::default()
            },
            CloakFormat::Png,
        );
        let payload = b"multi-bit test";
        let stego = embed_codec.encode(&cover, payload).unwrap();
        let result = extract_codec.decode(&stego);
        match result {
            Ok(data) => assert_ne!(data, payload),
            Err(_) => {}
        }
    }
}
