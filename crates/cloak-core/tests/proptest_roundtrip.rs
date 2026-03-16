use std::io::Cursor;

use image::{ImageFormat, RgbaImage};
use proptest::prelude::*;

fn make_cover(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let img = RgbaImage::from_fn(width, height, |x, y| {
        let r = ((x * 17 + y * 31) % 256) as u8;
        let g = ((x * 41 + y * 13) % 256) as u8;
        let b = ((x * 7 + y * 53) % 256) as u8;
        image::Rgba([r, g, b, 255])
    });
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), format).unwrap();
    buf
}

const PASSPHRASE: &str = "proptest-pass";

fn image_dim_strategy() -> impl Strategy<Value = (u32, u32)> {
    // Include prime dimensions for edge cases
    prop_oneof![
        Just((4, 4)),
        Just((7, 13)),
        Just((16, 16)),
        Just((32, 32)),
        Just((64, 64)),
        Just((128, 128)),
        (4u32..=128, 4u32..=128),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn roundtrip_random_payload(
        (w, h) in image_dim_strategy(),
        bit_depth in 1u8..=4,
        randomized in prop::bool::ANY,
    ) {
        let cover = make_cover(w, h, ImageFormat::Png);
        let opts = cloak_core::EmbedOptions {
            bit_depth,
            randomized,
            ..Default::default()
        };
        let cap = cloak_core::capacity(&cover, None, &opts).unwrap();
        if cap == 0 { return Ok(()); }

        let payload_len = cap.min(256); // Keep payloads small for speed
        let payload: Vec<u8> = (0..payload_len).map(|i| (i % 256) as u8).collect();

        let stego = cloak_core::embed(&cover, &payload, PASSPHRASE, None, &opts).unwrap();
        let extracted = cloak_core::extract(&stego, PASSPHRASE, None, &opts).unwrap();
        prop_assert_eq!(extracted, payload);
    }

    #[test]
    fn roundtrip_bmp(
        (w, h) in image_dim_strategy(),
        bit_depth in 1u8..=4,
    ) {
        let cover = make_cover(w, h, ImageFormat::Bmp);
        let opts = cloak_core::EmbedOptions {
            bit_depth,
            ..Default::default()
        };
        let cap = cloak_core::capacity(&cover, Some("test.bmp"), &opts).unwrap();
        if cap == 0 { return Ok(()); }

        let payload: Vec<u8> = (0..cap.min(128)).map(|i| (i % 256) as u8).collect();
        let stego = cloak_core::embed(&cover, &payload, PASSPHRASE, Some("test.bmp"), &opts).unwrap();
        let extracted = cloak_core::extract(&stego, PASSPHRASE, Some("out.bmp"), &opts).unwrap();
        prop_assert_eq!(extracted, payload);
    }

    #[test]
    fn roundtrip_tiff(
        (w, h) in image_dim_strategy(),
        bit_depth in 1u8..=4,
    ) {
        let cover = make_cover(w, h, ImageFormat::Tiff);
        let opts = cloak_core::EmbedOptions {
            bit_depth,
            ..Default::default()
        };
        let cap = cloak_core::capacity(&cover, Some("test.tiff"), &opts).unwrap();
        if cap == 0 { return Ok(()); }

        let payload: Vec<u8> = (0..cap.min(128)).map(|i| (i % 256) as u8).collect();
        let stego = cloak_core::embed(&cover, &payload, PASSPHRASE, Some("test.tiff"), &opts).unwrap();
        let extracted = cloak_core::extract(&stego, PASSPHRASE, Some("out.tiff"), &opts).unwrap();
        prop_assert_eq!(extracted, payload);
    }

    #[test]
    fn roundtrip_sequential_vs_randomized(
        (w, h) in image_dim_strategy(),
        bit_depth in 1u8..=4,
    ) {
        let cover = make_cover(w, h, ImageFormat::Png);
        let payload = b"consistency test payload data!";

        let seq_opts = cloak_core::EmbedOptions {
            bit_depth,
            randomized: false,
            ..Default::default()
        };
        let rand_opts = cloak_core::EmbedOptions {
            bit_depth,
            randomized: true,
            ..Default::default()
        };

        let seq_cap = cloak_core::capacity(&cover, None, &seq_opts).unwrap();
        let rand_cap = cloak_core::capacity(&cover, None, &rand_opts).unwrap();
        prop_assert_eq!(seq_cap, rand_cap);

        if seq_cap < payload.len() { return Ok(()); }

        let seq_stego = cloak_core::embed(&cover, payload, PASSPHRASE, None, &seq_opts).unwrap();
        let rand_stego = cloak_core::embed(&cover, payload, PASSPHRASE, None, &rand_opts).unwrap();

        let seq_out = cloak_core::extract(&seq_stego, PASSPHRASE, None, &seq_opts).unwrap();
        let rand_out = cloak_core::extract(&rand_stego, PASSPHRASE, None, &rand_opts).unwrap();

        prop_assert_eq!(&seq_out[..], payload);
        prop_assert_eq!(&rand_out[..], payload);
    }

    #[test]
    fn roundtrip_empty_payload(
        (w, h) in image_dim_strategy(),
        bit_depth in 1u8..=4,
    ) {
        let cover = make_cover(w, h, ImageFormat::Png);
        let opts = cloak_core::EmbedOptions {
            bit_depth,
            ..Default::default()
        };
        let cap = cloak_core::capacity(&cover, None, &opts).unwrap();
        if cap == 0 { return Ok(()); }

        let payload = b"";
        let stego = cloak_core::embed(&cover, payload, PASSPHRASE, None, &opts).unwrap();
        let extracted = cloak_core::extract(&stego, PASSPHRASE, None, &opts).unwrap();
        prop_assert!(extracted.is_empty());
    }
}
