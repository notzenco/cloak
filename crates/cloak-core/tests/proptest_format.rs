use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn format_detect_never_panics(
        data in prop::collection::vec(any::<u8>(), 0..512),
    ) {
        // Should return Ok or Err, never panic
        let _ = cloak_core::ImageFormat::detect(&data, None);
    }

    #[test]
    fn format_detect_with_path_never_panics(
        data in prop::collection::vec(any::<u8>(), 0..512),
        ext in prop_oneof![
            Just("test.png"),
            Just("test.bmp"),
            Just("test.jpg"),
            Just("test.webp"),
            Just("test.gif"),
            Just("test.tiff"),
            Just("test.xyz"),
        ],
    ) {
        let _ = cloak_core::ImageFormat::detect(&data, Some(ext));
    }

    #[test]
    fn detect_version_never_panics(
        data in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        let _ = cloak_core::crypto::detect_version(&data);
    }
}

#[test]
fn valid_magic_bytes_detect_correctly() {
    let png_magic = b"\x89PNG\r\n\x1a\n";
    assert_eq!(
        cloak_core::ImageFormat::detect(png_magic, None).unwrap(),
        cloak_core::ImageFormat::Png
    );

    let bmp_magic = b"BM\x00\x00\x00\x00";
    assert_eq!(
        cloak_core::ImageFormat::detect(bmp_magic, None).unwrap(),
        cloak_core::ImageFormat::Bmp
    );

    let jpeg_magic = b"\xFF\xD8\xFF\xE0";
    assert_eq!(
        cloak_core::ImageFormat::detect(jpeg_magic, None).unwrap(),
        cloak_core::ImageFormat::Jpeg
    );

    let gif_magic = b"GIF89a";
    assert_eq!(
        cloak_core::ImageFormat::detect(gif_magic, None).unwrap(),
        cloak_core::ImageFormat::Gif
    );
}
