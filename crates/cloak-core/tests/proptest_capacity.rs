use std::io::Cursor;

use image::{ImageFormat, RgbaImage};
use proptest::prelude::*;

fn make_cover(width: u32, height: u32) -> Vec<u8> {
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

const PASSPHRASE: &str = "proptest-pass";

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn capacity_matches_breakdown(
        w in 4u32..=128,
        h in 4u32..=128,
        bit_depth in 1u8..=4,
    ) {
        let cover = make_cover(w, h);
        let opts = cloak_core::EmbedOptions {
            bit_depth,
            ..Default::default()
        };
        let cap = cloak_core::capacity(&cover, None, &opts).unwrap();
        let report = cloak_core::capacity_report(&cover, None, &opts).unwrap();
        prop_assert_eq!(cap, report.usable_bytes);

        let breakdown = &report.breakdown;
        let expected = breakdown.usable_bytes.saturating_sub(cloak_core::crypto::overhead());
        prop_assert_eq!(cap, expected);
    }

    #[test]
    fn capacity_monotonic_with_bit_depth(
        w in 8u32..=128,
        h in 8u32..=128,
    ) {
        let cover = make_cover(w, h);
        let mut prev_cap = 0;
        for bd in 1u8..=4 {
            let opts = cloak_core::EmbedOptions {
                bit_depth: bd,
                ..Default::default()
            };
            let cap = cloak_core::capacity(&cover, None, &opts).unwrap();
            prop_assert!(cap >= prev_cap,
                "cap at bd={} ({}) < cap at bd={} ({})", bd, cap, bd - 1, prev_cap);
            prev_cap = cap;
        }
    }

    #[test]
    fn exact_capacity_embeds_and_roundtrips(
        w in 16u32..=64,
        h in 16u32..=64,
        bit_depth in 1u8..=4,
    ) {
        let cover = make_cover(w, h);
        let opts = cloak_core::EmbedOptions {
            bit_depth,
            ..Default::default()
        };
        let cap = cloak_core::capacity(&cover, None, &opts).unwrap();
        if cap == 0 { return Ok(()); }

        let payload: Vec<u8> = (0..cap).map(|i| (i % 256) as u8).collect();
        let stego = cloak_core::embed(&cover, &payload, PASSPHRASE, None, &opts).unwrap();
        let extracted = cloak_core::extract(&stego, PASSPHRASE, None, &opts).unwrap();
        prop_assert_eq!(extracted, payload);
    }

    #[test]
    fn max_payload_bounded_by_raw_pixels(
        w in 4u32..=128,
        h in 4u32..=128,
        bit_depth in 1u8..=4,
    ) {
        let raw_bytes = w as usize * h as usize * 3 * bit_depth as usize / 8;
        let max = cloak_core::formats::lsb::max_payload_bytes(w, h, bit_depth);
        prop_assert!(max <= raw_bytes,
            "max_payload {} > raw_bytes {} for {}x{} bd={}",
            max, raw_bytes, w, h, bit_depth);
    }
}
