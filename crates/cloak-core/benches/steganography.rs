use std::io::Cursor;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use image::{ImageFormat, RgbaImage};

use cloak_core::formats::lsb::{LsbParams, derive_length_mask, generate_permutation};
use cloak_core::formats::{ImageFormat as CloakFormat, LsbCodec};
use cloak_core::traits::{Capacity, Decoder, Encoder};

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

fn bench_embed(c: &mut Criterion) {
    let mut group = c.benchmark_group("embed");

    for &size in &[64u32, 256, 512] {
        let cover = make_png(size, size);
        let codec = LsbCodec::new(LsbParams::default(), CloakFormat::Png);
        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap.min(1024)).map(|i| (i % 256) as u8).collect();

        group.bench_with_input(
            BenchmarkId::new("sequential", format!("{size}x{size}")),
            &size,
            |b, _| {
                b.iter(|| codec.encode(&cover, &payload).unwrap());
            },
        );
    }

    group.finish();
}

fn bench_extract(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract");

    for &size in &[64u32, 256, 512] {
        let cover = make_png(size, size);
        let codec = LsbCodec::new(LsbParams::default(), CloakFormat::Png);
        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap.min(1024)).map(|i| (i % 256) as u8).collect();
        let stego = codec.encode(&cover, &payload).unwrap();

        group.bench_with_input(
            BenchmarkId::new("sequential", format!("{size}x{size}")),
            &size,
            |b, _| {
                b.iter(|| codec.decode(&stego).unwrap());
            },
        );
    }

    group.finish();
}

fn bench_bit_depths(c: &mut Criterion) {
    let mut group = c.benchmark_group("bit_depth");
    let cover = make_png(256, 256);

    for bit_depth in [1u8, 2, 3, 4] {
        let codec = LsbCodec::new(
            LsbParams {
                bit_depth,
                ..Default::default()
            },
            CloakFormat::Png,
        );
        let cap = codec.capacity(&cover).unwrap();
        let payload: Vec<u8> = (0..cap.min(2048)).map(|i| (i % 256) as u8).collect();

        group.bench_with_input(BenchmarkId::new("embed", bit_depth), &bit_depth, |b, _| {
            b.iter(|| codec.encode(&cover, &payload).unwrap());
        });
    }

    group.finish();
}

fn bench_randomized(c: &mut Criterion) {
    let mut group = c.benchmark_group("randomized");
    let cover = make_png(256, 256);
    let pixel_count = 256 * 256;

    let perm = generate_permutation("benchmark-passphrase", pixel_count).unwrap();
    let mask = derive_length_mask("benchmark-passphrase");
    let params = LsbParams {
        bit_depth: 1,
        pixel_order: cloak_core::formats::lsb::PixelOrder::Randomized(perm),
        length_mask: mask,
    };
    let codec = LsbCodec::new(params, CloakFormat::Png);
    let cap = codec.capacity(&cover).unwrap();
    let payload: Vec<u8> = (0..cap.min(1024)).map(|i| (i % 256) as u8).collect();
    let stego = codec.encode(&cover, &payload).unwrap();

    group.bench_function("embed_256x256", |b| {
        b.iter(|| codec.encode(&cover, &payload).unwrap());
    });

    group.bench_function("extract_256x256", |b| {
        b.iter(|| codec.decode(&stego).unwrap());
    });

    group.finish();
}

fn bench_e2e(c: &mut Criterion) {
    let mut group = c.benchmark_group("e2e");
    let cover = make_png(256, 256);
    let passphrase = "benchmark-passphrase";
    let opts = cloak_core::EmbedOptions {
        bit_depth: 1,
        randomized: false,
    };
    let payload = b"end-to-end benchmark payload data for testing";

    let stego = cloak_core::embed(&cover, payload, passphrase, None, &opts).unwrap();

    group.bench_function("embed_256x256", |b| {
        b.iter(|| cloak_core::embed(&cover, payload, passphrase, None, &opts).unwrap());
    });

    group.bench_function("extract_256x256", |b| {
        b.iter(|| cloak_core::extract(&stego, passphrase, None, &opts).unwrap());
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_embed,
    bench_extract,
    bench_bit_depths,
    bench_randomized,
    bench_e2e,
);
criterion_main!(benches);
