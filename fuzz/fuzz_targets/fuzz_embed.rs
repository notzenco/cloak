#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    width: u8,
    height: u8,
    payload: Vec<u8>,
}

fuzz_target!(|input: FuzzInput| {
    let w = (input.width as u32).clamp(4, 64);
    let h = (input.height as u32).clamp(4, 64);

    let img = image::RgbaImage::from_fn(w, h, |x, y| {
        let r = ((x * 17 + y * 31) % 256) as u8;
        let g = ((x * 41 + y * 13) % 256) as u8;
        let b = ((x * 7 + y * 53) % 256) as u8;
        image::Rgba([r, g, b, 255])
    });
    let mut buf = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Png,
    )
    .unwrap();

    let opts = cloak_core::EmbedOptions::default();
    let _ = cloak_core::embed(&buf, &input.payload, "fuzz-passphrase", None, &opts);
});
