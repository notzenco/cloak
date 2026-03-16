# Changelog

## 0.4.0

- Versioned wire format: `WireVersion` enum, `detect_version()` function, `UnsupportedVersion` error variant, decrypt dispatches by version
- Parallel embedding via rayon: `embed_lsb_parallel()` with `--parallel` CLI flag, optional `parallel` feature in cloak-core, falls back to sequential for images < 64x64
- Enhanced capacity reporting: `CapacityBreakdown` struct with detailed metrics, `capacity_report()` API, CLI capacity command shows full breakdown
- GIF-specific steganalysis: palette anomaly detection, EzStego detection, Gifshuffle detection, palette chi-square test, integrated into CLI analyze command
- Property-based testing with proptest: 21 tests across 5 files covering roundtrip, analysis, capacity, crypto, and format detection
- Fuzz targets with cargo-fuzz: 6 fuzz harnesses for extract, embed, analyze, decrypt, format detection, and wire format parsing

## 0.3.0

- Fix `expect()` panics in public API — now returns proper `CloakError` variants
- Unified `LsbCodec` replaces per-format codec structs, reducing code duplication
- Avoid double image decoding in embed/extract paths (decode once, reuse)
- Encrypt payload length header with passphrase-derived XOR mask
- Streaming LSB extraction eliminates intermediate bit vector allocation
- Payload padding to 64-byte block boundaries for steganalysis resistance
- GIF and TIFF format support
- Passphrase strength warning in CLI for passphrases under 8 characters
- Criterion benchmarks for embed/extract at various image sizes and bit depths

## 0.2.0

- JPEG and WebP cover image support (stego output is PNG)
- RS analysis, sample pairs analysis, and Shannon entropy analysis
- Multi-bit LSB embedding (1-4 bits per channel, `--bit-depth`)
- Randomized pixel traversal (`--randomize`)
- Batch embed/extract subcommands (`batch-embed`, `batch-extract`)
- Stdin/stdout piping (use `-` as path)
- Glob pattern support for analyze and capacity commands
- Shared LSB engine refactor (unified `lsb.rs` across all formats)

## 0.1.0

Initial release.

- PNG and BMP LSB steganography
- Mandatory ChaCha20-Poly1305 encryption with Argon2 key derivation
- Format auto-detection from magic bytes
- Steganalysis module (chi-square, sample pairs, RS analysis)
- CLI with embed, extract, analyze, capacity, and inspect commands
- TUI analysis dashboard
