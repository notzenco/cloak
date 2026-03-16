use std::io::Cursor;

use image::{ImageFormat, RgbaImage};
use proptest::prelude::*;

fn make_random_png(width: u32, height: u32, seed: u64) -> Vec<u8> {
    let img = RgbaImage::from_fn(width, height, |x, y| {
        // Deterministic pseudo-random based on seed, x, y
        let v = seed
            .wrapping_mul(x as u64 * 17 + y as u64 * 31 + 7)
            .wrapping_add(x as u64 * 41 + y as u64 * 13);
        let r = (v & 0xFF) as u8;
        let g = ((v >> 8) & 0xFF) as u8;
        let b = ((v >> 16) & 0xFF) as u8;
        image::Rgba([r, g, b, 255])
    });
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .unwrap();
    buf
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn entropy_in_valid_range(
        w in 8u32..=128,
        h in 8u32..=128,
        seed in any::<u64>(),
    ) {
        let data = make_random_png(w, h, seed);
        let result = cloak_core::analysis::analyze_image(&data).unwrap();
        let ent = result.entropy.unwrap();
        prop_assert!(ent.red >= 0.0 && ent.red <= 8.0, "red={}", ent.red);
        prop_assert!(ent.green >= 0.0 && ent.green <= 8.0, "green={}", ent.green);
        prop_assert!(ent.blue >= 0.0 && ent.blue <= 8.0, "blue={}", ent.blue);
        prop_assert!(ent.average >= 0.0 && ent.average <= 8.0, "avg={}", ent.average);
    }

    #[test]
    fn histogram_sum_equals_pixel_count_times_3(
        w in 4u32..=128,
        h in 4u32..=128,
        seed in any::<u64>(),
    ) {
        let data = make_random_png(w, h, seed);
        let result = cloak_core::analysis::analyze_image(&data).unwrap();
        let total: u64 = result.histogram.iter().sum();
        prop_assert_eq!(total, result.pixel_count * 3);
    }

    #[test]
    fn rs_rate_in_valid_range(
        w in 16u32..=128,
        h in 16u32..=128,
        seed in any::<u64>(),
    ) {
        let data = make_random_png(w, h, seed);
        let result = cloak_core::analysis::analyze_image(&data).unwrap();
        let rs = result.rs.unwrap();
        prop_assert!(rs.estimated_rate >= 0.0 && rs.estimated_rate <= 1.0,
            "rate={}", rs.estimated_rate);
    }

    #[test]
    fn sample_pairs_rate_in_valid_range(
        w in 16u32..=128,
        h in 16u32..=128,
        seed in any::<u64>(),
    ) {
        let data = make_random_png(w, h, seed);
        let result = cloak_core::analysis::analyze_image(&data).unwrap();
        let sp = result.sample_pairs.unwrap();
        prop_assert!(sp.estimated_rate >= 0.0 && sp.estimated_rate <= 1.0,
            "rate={}", sp.estimated_rate);
    }
}
