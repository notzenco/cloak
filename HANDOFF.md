# Cloak v0.3.0 Handoff

This document describes all changes made in the v0.3.0 improvement cycle, the rationale behind each, current state of the codebase, and what remains for future work.

## Summary

Ten improvements were planned. Nine were implemented; one (property-based testing/fuzzing) was deferred. The codebase went from 73 passing tests to 88, gained two new image formats, and received security, performance, and code quality improvements throughout.

## Changes Made

### 1. Fix `expect()` Panics in Public API

**Problem:** Two `expect()` calls in library code could panic during normal usage instead of returning errors to the caller.

**Changes:**
- `error.rs` — Added `CloakError::MissingPassphrase` variant
- `lsb.rs:44` — `generate_permutation()` now returns `crate::Result<Vec<usize>>` instead of `Vec<usize>`. The internal Argon2 call uses `map_err` + `?` instead of `expect()`
- `lib.rs:25` — `EmbedOptions::lsb_params()` now returns `Result<LsbParams>`. Uses `ok_or(CloakError::MissingPassphrase)?` instead of `expect()`

**Impact:** The library never panics on user input. All error paths return typed `CloakError` variants.

### 2. Unified Format Codec

**Problem:** `png.rs`, `bmp.rs`, `jpeg.rs`, `webp.rs` each defined nearly identical codec structs (`PngCodec`, `BmpCodec`, etc.) with copy-pasted `Capacity`/`Encoder`/`Decoder` impls.

**Changes:**
- `formats/mod.rs` — Added `LsbCodec` struct with `params: LsbParams` and `output_format: ImageFormat` fields. Implements `Capacity`, `Encoder`, `Decoder` once for all formats.
- `lib.rs` — All embed/extract/capacity calls now go through `LsbCodec::new(params, format)`. Removed per-format codec dispatch matches.
- `png.rs`, `bmp.rs`, `jpeg.rs`, `webp.rs` — Stripped down to `#[cfg(test)] mod tests` only. Tests updated to use `LsbCodec`.
- `analysis.rs` — Three test functions updated from `PngCodec::default()` to `LsbCodec::new(...)`.
- `lib.rs` — Added `pub use formats::LsbCodec`.

**Impact:** Adding a new format no longer requires writing a codec struct or trait impls. Just add an `ImageFormat` variant and a test file.

### 3. Avoid Double Image Decoding

**Problem:** `lib.rs` decoded the cover image to get the pixel count, then `LsbCodec::encode()`/`decode()` decoded it again from raw bytes.

**Changes:**
- `formats/mod.rs` — Added `encode_image(&self, img: &DynamicImage, payload: &[u8])` and `decode_image(&self, img: &DynamicImage)` methods on `LsbCodec`. The raw-bytes `encode()`/`decode()` trait methods now delegate to these after a single decode.
- `lib.rs` — `embed()` and `extract()` call `load_from_memory()` once, then use `encode_image()`/`decode_image()` directly.

**Impact:** Embed and extract each do one image decode instead of two.

### 4. Encrypted Payload Length Header

**Problem:** The 4-byte payload length was stored in plaintext in the image's LSBs. An attacker reading the first 32 LSB bits would see a valid-looking length, confirming steganographic content.

**Changes:**
- `lsb.rs` — Added `length_mask: [u8; 4]` field to `LsbParams` (default `[0; 4]` for backwards compatibility at the low-level API). Added `derive_length_mask(passphrase: &str) -> [u8; 4]` using Argon2id with salt `b"cloak-len-mask!!"`.
- `lsb.rs:134-138` — `embed_lsb()` XORs length bytes with the mask before embedding.
- `lsb.rs:236-243` — `extract_lsb()` XORs extracted length bytes with the mask to recover the real length.
- `lib.rs:66,94` — `embed()` and `extract()` set `params.length_mask = derive_length_mask(passphrase)`.

**Wire format change:** Images embedded with v0.3.0 using the high-level `embed()`/`extract()` API are **not** extractable with v0.2.0, because the length header is now masked. The low-level `LsbCodec` with `length_mask: [0; 4]` remains backwards compatible.

### 5. Streaming LSB Extraction

**Problem:** `extract_lsb()` allocated a `Vec<u8>` containing one byte per bit for every pixel in the image. For a 4K image this was ~25 million entries.

**Changes:**
- `lsb.rs:187-206` — Added `read_bit()` inline helper that reads a single bit from the image by computing pixel coordinates, channel, and bit position on the fly.
- `lsb.rs:213-265` — Rewrote `extract_lsb()` to use `read_bit()` in a loop. Extracts the 32-bit length header, then each payload byte directly. No intermediate allocation.
- `lsb.rs` — Removed now-unused `bits_to_u32()` and `bits_to_byte()` functions.

**Impact:** Extraction memory is O(payload_size) instead of O(image_pixels).

### 6. Passphrase Strength Warning

**Problem:** No feedback for weak passphrases. Argon2id helps, but 1-3 character passphrases are still trivially brute-forceable.

**Changes:**
- `cloak-cli/src/main.rs` — `get_passphrase()` prints a warning to stderr when the passphrase is non-empty but shorter than 8 characters.

**Impact:** Users see `Warning: passphrase is short (N chars) — consider using 8+ characters for stronger security`. This is a warning only; short passphrases are still accepted.

### 7. GIF and TIFF Format Support

**Problem:** Only PNG, BMP, JPEG, and WebP were supported. GIF and TIFF are common and natural additions.

**Changes:**
- `formats/mod.rs` — Added `Gif` and `Tiff` variants to `ImageFormat`. Added magic byte detection (GIF87a/GIF89a, TIFF II/MM byte order marks). Added `.gif`/`.tif`/`.tiff` extension detection. GIF is treated as lossy (palette-based; stego output is PNG). TIFF is lossless (direct roundtrip).
- `formats/gif.rs` — New file with 7 tests (roundtrip via PNG, capacity, format detection, extension detection, payload too large, max capacity, multi-bit).
- `formats/tiff.rs` — New file with 8 tests (direct roundtrip, capacity, format detection, extension detection, payload too large, max capacity, multi-bit, capacity scaling).
- `lib.rs:82` — GIF added to extract rejection for lossy formats.
- `cloak-cli/src/main.rs` — `.gif` added to lossy output path correction.

**Impact:** 6 formats supported. 15 new tests.

### 8. Property-Based Testing and Fuzzing (Deferred)

**Not implemented.** Adding `proptest`/`quickcheck` and `cargo-fuzz` requires significant infrastructure (fuzz corpus management, CI integration for continuous fuzzing). The existing 88 hand-written tests provide good coverage. This is the best candidate for a future improvement cycle.

### 9. Criterion Benchmarks

**Problem:** No benchmarks existed. Performance regressions could go unnoticed.

**Changes:**
- `cloak-core/Cargo.toml` — Added `criterion` dev-dependency and `[[bench]]` section.
- `cloak-core/benches/steganography.rs` — Five benchmark groups:
  - `embed` — Sequential embed at 64x64, 256x256, 512x512
  - `extract` — Sequential extract at the same sizes
  - `bit_depth` — Embed at bit depths 1-4 on 256x256
  - `randomized` — Randomized embed/extract on 256x256
  - `e2e` — Full encrypt+embed and extract+decrypt on 256x256
- `Makefile` — Added `bench` target.

**Impact:** Run `make bench` or `cargo bench --package cloak-core`. HTML reports in `target/criterion/`.

### 10. Payload Padding

**Problem:** Payload size was exactly encoded. Chi-square analysis could detect exactly where embedded data ended, revealing the payload boundary.

**Changes:**
- `lsb.rs:80` — Added `PAD_BLOCK` constant (64 bytes).
- `lsb.rs:101-117` — Added `pad_payload()` function. Pads with `rand::thread_rng()` random bytes to the next 64-byte boundary. Empty payloads are not padded.
- `lsb.rs:83-97` — `max_payload_bytes()` updated to report usable capacity after accounting for padding overhead (rounds down to full blocks).
- `lsb.rs:140` — `embed_lsb()` pads the payload before embedding. The length header still records the real (unpadded) length.
- Extraction is unchanged — it reads only `length` bytes from the header, naturally ignoring the padding.

**Impact:** An observer reading LSBs sees data that extends to a block boundary, not the exact payload end. Capacity is slightly reduced (up to 63 bytes per image, depending on alignment).

## Current State

```
cargo test --workspace    # 88 tests pass (75 unit + 13 integration)
cargo clippy --workspace -- -D warnings   # 0 warnings
cargo fmt --all --check   # Clean
```

### Test Counts by Module

| Module | Tests |
|--------|-------|
| analysis | 11 |
| crypto | 8 |
| formats::png | 11 |
| formats::bmp | 8 |
| formats::jpeg | 7 |
| formats::webp | 7 |
| formats::gif | 7 |
| formats::tiff | 8 |
| formats::lsb | 5 |
| integration | 13 |
| **Total** | **88** |

### File Inventory

**Modified files:**
- `crates/cloak-core/src/lib.rs` — Public API, double-decode fix, length mask wiring
- `crates/cloak-core/src/error.rs` — `MissingPassphrase` variant
- `crates/cloak-core/src/formats/mod.rs` — `LsbCodec`, GIF/TIFF variants, `encode_image`/`decode_image`
- `crates/cloak-core/src/formats/lsb.rs` — Length mask, padding, streaming extraction, `generate_permutation` returns Result
- `crates/cloak-core/src/formats/png.rs` — Tests only (uses `LsbCodec`)
- `crates/cloak-core/src/formats/bmp.rs` — Tests only (uses `LsbCodec`)
- `crates/cloak-core/src/formats/jpeg.rs` — Tests only (uses `LsbCodec`)
- `crates/cloak-core/src/formats/webp.rs` — Tests only (uses `LsbCodec`)
- `crates/cloak-core/src/analysis.rs` — Test updates for `LsbCodec`
- `crates/cloak-core/Cargo.toml` — criterion dev-dependency
- `crates/cloak-cli/src/main.rs` — Passphrase warning, GIF output path correction
- `Cargo.toml` — No changes (workspace deps unchanged)
- `Makefile` — Added `bench` target
- `CHANGELOG.md` — v0.3.0 entry
- `README.md` — Updated for 6 formats, security model, benchmarks, architecture
- `CONTRIBUTING.md` — Updated project structure, "adding a format" guide
- `SECURITY.md` — Updated supported versions, security architecture section

**New files:**
- `crates/cloak-core/src/formats/gif.rs` — GIF tests
- `crates/cloak-core/src/formats/tiff.rs` — TIFF tests
- `crates/cloak-core/benches/steganography.rs` — Criterion benchmarks

## Breaking Changes

### Wire format (v0.3.0 vs v0.2.0)

The high-level `embed()`/`extract()` API now applies a passphrase-derived XOR mask to the 4-byte length header and pads the payload. **Images embedded with v0.3.0 cannot be extracted with v0.2.0, and vice versa.**

The low-level `LsbCodec` API is backwards compatible if `length_mask` is left at `[0; 4]` and the caller does not use padding (by calling `embed_lsb` directly with unpadded data). However, this bypasses the security improvements.

### API changes

- `generate_permutation()` now returns `Result<Vec<usize>>` instead of `Vec<usize>`.
- `EmbedOptions::lsb_params()` now returns `Result<LsbParams>` instead of `LsbParams`.
- `LsbParams` has a new `length_mask: [u8; 4]` field (defaults to `[0; 4]`).
- `PngCodec`, `BmpCodec`, `JpegCodec`, `WebpCodec` are removed. Use `LsbCodec` instead.
- `CloakError` has a new `MissingPassphrase` variant.
- `ImageFormat` has new `Gif` and `Tiff` variants.
- `bits_to_u32()` and `bits_to_byte()` are removed (were public but only used internally).

## Future Work (v0.3.0)

All six items identified in v0.3.0 were implemented in v0.4.0:

1. **Property-based testing** — Implemented with proptest (21 tests across 5 files).
2. **Cargo-fuzz targets** — 6 fuzz harnesses covering extract, embed, analyze, decrypt, format detection, and wire format.
3. **Capacity reporting with padding** — `CapacityBreakdown` struct and `capacity_report()` API with detailed CLI output.
4. **Versioned wire format** — `WireVersion` enum, `detect_version()`, `UnsupportedVersion` error, decrypt dispatches by version.
5. **Parallel embedding** — `embed_lsb_parallel()` with rayon, `--parallel` CLI flag, optional `parallel` feature.
6. **GIF-specific analysis** — Palette anomaly detection, EzStego, Gifshuffle, palette chi-square.
