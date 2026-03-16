use std::io::Cursor;

use cloak_core::{CloakError, EmbedOptions};
use image::{ImageFormat, RgbImage, RgbaImage};

fn make_png(width: u32, height: u32) -> Vec<u8> {
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

fn make_bmp(width: u32, height: u32) -> Vec<u8> {
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

fn make_jpeg(width: u32, height: u32) -> Vec<u8> {
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

fn make_webp(width: u32, height: u32) -> Vec<u8> {
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

fn default_opts() -> EmbedOptions {
    EmbedOptions {
        bit_depth: 1,
        randomized: false,
    }
}

const PASSPHRASE: &str = "test-passphrase-123";

#[test]
fn png_roundtrip() {
    let cover = make_png(64, 64);
    let payload = b"PNG end-to-end roundtrip";
    let opts = default_opts();

    let stego = cloak_core::embed(&cover, payload, PASSPHRASE, None, &opts).unwrap();
    let extracted = cloak_core::extract(&stego, PASSPHRASE, None, &opts).unwrap();

    assert_eq!(extracted, payload);
}

#[test]
fn bmp_roundtrip() {
    let cover = make_bmp(64, 64);
    let payload = b"BMP end-to-end roundtrip";
    let opts = default_opts();

    let stego = cloak_core::embed(&cover, payload, PASSPHRASE, Some("cover.bmp"), &opts).unwrap();
    let extracted = cloak_core::extract(&stego, PASSPHRASE, Some("stego.bmp"), &opts).unwrap();

    assert_eq!(extracted, payload);
}

#[test]
fn jpeg_to_png_roundtrip() {
    let cover = make_jpeg(64, 64);
    let payload = b"JPEG cover -> PNG stego";
    let opts = default_opts();

    // Embed into JPEG cover (produces PNG stego)
    let stego = cloak_core::embed(&cover, payload, PASSPHRASE, Some("photo.jpg"), &opts).unwrap();

    // Extract from the PNG output
    let extracted = cloak_core::extract(&stego, PASSPHRASE, None, &opts).unwrap();

    assert_eq!(extracted, payload);
}

#[test]
fn webp_to_png_roundtrip() {
    let cover = make_webp(64, 64);
    let payload = b"WebP cover -> PNG stego";
    let opts = default_opts();

    let stego = cloak_core::embed(&cover, payload, PASSPHRASE, Some("image.webp"), &opts).unwrap();

    // Stego output is PNG
    let extracted = cloak_core::extract(&stego, PASSPHRASE, None, &opts).unwrap();

    assert_eq!(extracted, payload);
}

#[test]
fn multi_bit_roundtrip_depth_2() {
    let cover = make_png(64, 64);
    let payload = b"multi-bit depth 2 test payload";
    let opts = EmbedOptions {
        bit_depth: 2,
        randomized: false,
    };

    let stego = cloak_core::embed(&cover, payload, PASSPHRASE, None, &opts).unwrap();
    let extracted = cloak_core::extract(&stego, PASSPHRASE, None, &opts).unwrap();

    assert_eq!(extracted, payload);
}

#[test]
fn multi_bit_roundtrip_depth_4() {
    let cover = make_png(64, 64);
    let payload = b"multi-bit depth 4 test payload with more data for good measure";
    let opts = EmbedOptions {
        bit_depth: 4,
        randomized: false,
    };

    let stego = cloak_core::embed(&cover, payload, PASSPHRASE, None, &opts).unwrap();
    let extracted = cloak_core::extract(&stego, PASSPHRASE, None, &opts).unwrap();

    assert_eq!(extracted, payload);
}

#[test]
fn randomized_roundtrip() {
    let cover = make_png(64, 64);
    let payload = b"randomized embedding test";
    let opts = EmbedOptions {
        bit_depth: 1,
        randomized: true,
    };

    let stego = cloak_core::embed(&cover, payload, PASSPHRASE, None, &opts).unwrap();
    let extracted = cloak_core::extract(&stego, PASSPHRASE, None, &opts).unwrap();

    assert_eq!(extracted, payload);
}

#[test]
fn randomized_multi_bit_roundtrip() {
    let cover = make_png(64, 64);
    let payload = b"randomized + multi-bit combined";
    let opts = EmbedOptions {
        bit_depth: 3,
        randomized: true,
    };

    let stego = cloak_core::embed(&cover, payload, PASSPHRASE, None, &opts).unwrap();
    let extracted = cloak_core::extract(&stego, PASSPHRASE, None, &opts).unwrap();

    assert_eq!(extracted, payload);
}

#[test]
fn wrong_passphrase() {
    let cover = make_png(64, 64);
    let payload = b"secret data";
    let opts = default_opts();

    let stego = cloak_core::embed(&cover, payload, PASSPHRASE, None, &opts).unwrap();
    let result = cloak_core::extract(&stego, "wrong-passphrase", None, &opts);

    assert!(
        matches!(
            result,
            Err(CloakError::InvalidPassphrase | CloakError::CorruptedData(_))
        ),
        "expected InvalidPassphrase or CorruptedData, got: {result:?}"
    );
}

#[test]
fn wrong_bit_depth_extract() {
    let cover = make_png(64, 64);
    let payload = b"bit depth mismatch test";
    let embed_opts = EmbedOptions {
        bit_depth: 2,
        randomized: false,
    };
    let extract_opts = EmbedOptions {
        bit_depth: 1,
        randomized: false,
    };

    let stego = cloak_core::embed(&cover, payload, PASSPHRASE, None, &embed_opts).unwrap();
    let result = cloak_core::extract(&stego, PASSPHRASE, None, &extract_opts);

    // Should fail to decrypt or produce garbage
    match result {
        Ok(data) => assert_ne!(data, payload),
        Err(_) => {} // Also acceptable
    }
}

#[test]
fn wrong_randomize_flag() {
    let cover = make_png(64, 64);
    let payload = b"randomize mismatch test";
    let embed_opts = EmbedOptions {
        bit_depth: 1,
        randomized: true,
    };
    let extract_opts = EmbedOptions {
        bit_depth: 1,
        randomized: false,
    };

    let stego = cloak_core::embed(&cover, payload, PASSPHRASE, None, &embed_opts).unwrap();
    let result = cloak_core::extract(&stego, PASSPHRASE, None, &extract_opts);

    // Should fail to decrypt or produce garbage
    match result {
        Ok(data) => assert_ne!(data, payload),
        Err(_) => {} // Also acceptable
    }
}

#[test]
fn capacity_check() {
    let cover = make_png(64, 64);
    let opts = default_opts();

    let cap = cloak_core::capacity(&cover, None, &opts).unwrap();
    assert!(cap > 0);

    // Payload at exact capacity should work
    let payload: Vec<u8> = (0..cap).map(|i| (i % 256) as u8).collect();
    let stego = cloak_core::embed(&cover, &payload, PASSPHRASE, None, &opts).unwrap();
    let extracted = cloak_core::extract(&stego, PASSPHRASE, None, &opts).unwrap();

    assert_eq!(extracted, payload);
}

#[test]
fn oversize_payload() {
    let cover = make_png(16, 16);
    let opts = default_opts();

    let cap = cloak_core::capacity(&cover, None, &opts).unwrap();
    let payload = vec![0xAA; cap + 1];

    let result = cloak_core::embed(&cover, &payload, PASSPHRASE, None, &opts);
    assert!(
        matches!(result, Err(CloakError::PayloadTooLarge { .. })),
        "expected PayloadTooLarge, got: {result:?}"
    );
}
