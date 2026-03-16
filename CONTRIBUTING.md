# Contributing to cloak

## Getting Started

1. Fork and clone the repository
2. Install Rust via [rustup](https://rustup.rs/)
3. Run `make check` to verify everything builds and passes

## Development Workflow

1. Create a branch from `main`
2. Make your changes
3. Run the full check suite:
   ```
   make check  # fmt-check + clippy + tests
   ```
4. Run benchmarks if touching performance-sensitive code:
   ```
   make bench
   ```
5. Commit with a clear message
6. Open a pull request against `main`

## Project Structure

```
crates/
  cloak-core/           Core library
    src/
      lib.rs            Public API: embed(), extract(), capacity(), capacity_report()
      crypto.rs         ChaCha20-Poly1305 encryption, Argon2id KDF, wire versioning
      error.rs          CloakError enum (incl. UnsupportedVersion)
      traits.rs         Encoder, Decoder, Capacity traits
      analysis.rs       Steganalysis (chi-square, RS, sample pairs, entropy, GIF analysis)
      formats/
        mod.rs          ImageFormat enum + unified LsbCodec
        lsb.rs          LSB embedding/extraction engine (sequential + parallel)
        png.rs          PNG tests
        bmp.rs          BMP tests
        jpeg.rs         JPEG tests
        webp.rs         WebP tests
        gif.rs          GIF tests
        tiff.rs         TIFF tests
    tests/
      integration.rs            End-to-end integration tests
      proptest_roundtrip.rs     Property-based roundtrip tests
      proptest_analysis.rs      Property-based analysis invariant tests
      proptest_capacity.rs      Property-based capacity tests
      proptest_crypto.rs        Property-based crypto tests
      proptest_format.rs        Property-based format detection tests
    benches/
      steganography.rs  Criterion benchmarks (incl. parallel)
  cloak-cli/            CLI binary
    src/main.rs         All CLI subcommands
  cloak-tui/            TUI analysis dashboard
fuzz/
  fuzz_targets/         Cargo-fuzz harnesses (extract, embed, analyze, decrypt, format, wire)
```

## Adding a New Image Format

With the unified `LsbCodec`, adding a new format requires:

1. Add a variant to `ImageFormat` in `formats/mod.rs`
2. Add magic byte detection in `ImageFormat::detect()`
3. Add extension detection in the same function
4. Update `output_format()`, `extension()`, `is_lossy()`
5. Add the `image::ImageFormat` mapping in `LsbCodec::image_format()`
6. Create a test file `formats/<name>.rs` with roundtrip tests
7. Add `pub mod <name>;` in `formats/mod.rs`
8. If lossy/palette-based, add to the extract rejection match in `lib.rs`

No trait implementations needed — `LsbCodec` handles everything.

## Code Style

- Follow `cargo fmt` and `cargo clippy -- -D warnings` conventions
- Use `thiserror` for errors in `cloak-core`, `anyhow` in binaries
- Add tests for new functionality
- Consider adding proptest properties for new invariants in `tests/proptest_*.rs`
- Keep the public API surface in `lib.rs` small — implementation details stay in submodules
- Optional features (e.g., `parallel`) should degrade gracefully when disabled
