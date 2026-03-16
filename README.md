# cloak

A modern steganography toolkit written in Rust. Embed encrypted data within images using LSB steganography with mandatory ChaCha20-Poly1305 encryption.

## Features

- **Mandatory encryption** — All payloads encrypted with ChaCha20-Poly1305 (Argon2id KDF)
- **6 image formats** — PNG, BMP, JPEG, WebP, GIF, and TIFF cover images
- **Multi-bit LSB** — Embed using 1-4 bits per channel (`--bit-depth`) for higher capacity
- **Randomized embedding** — Passphrase-derived pixel traversal order (`--randomize`) for improved security
- **Parallel embedding** — Optional rayon-based parallelism (`--parallel`) for large images
- **Versioned wire format** — Forward-compatible encryption format with version detection
- **Encrypted length header** — Payload length XOR-masked with a passphrase-derived key to resist steganalysis
- **Payload padding** — Random padding to 64-byte block boundaries hides exact data boundaries
- **Steganalysis suite** — Chi-square testing, RS analysis, sample pairs analysis, Shannon entropy, histogram analysis, bit-plane extraction
- **GIF-specific steganalysis** — Palette anomaly detection, EzStego detection, Gifshuffle detection, palette chi-square testing
- **Detailed capacity reporting** — Full breakdown of pixel bits, padding overhead, and crypto overhead
- **Batch processing** — Embed/extract across entire directories with `batch-embed` / `batch-extract`
- **Stdin/stdout piping** — Use `-` as path for pipeline integration
- **TUI dashboard** — Interactive terminal-based image analysis
- **CLI** — Full-featured command-line interface with passphrase strength warnings
- **Property-based testing** — Proptest roundtrip and invariant tests across randomized inputs
- **Fuzz targets** — Cargo-fuzz harnesses for all untrusted input paths

## Format Support

| Format | Cover input | Stego output | Lossless roundtrip |
|--------|-------------|--------------|-------------------|
| PNG    | Yes         | Yes          | Yes               |
| BMP    | Yes         | Yes          | Yes               |
| TIFF   | Yes         | Yes          | Yes               |
| JPEG   | Yes         | PNG          | No (lossy input)  |
| WebP   | Yes         | PNG          | No (lossy input)  |
| GIF    | Yes         | PNG          | No (palette-based)|

Lossy and palette-based formats are decoded to RGBA for embedding, and the stego output is saved as lossless PNG to preserve the embedded data.

## Usage

### Embed and extract

```bash
# Embed secret data into an image
cloak embed -i cover.png -d secret.txt -o output.png

# Extract hidden data
cloak extract -i output.png -o recovered.txt

# With higher bit depth for more capacity
cloak embed -i cover.png -d secret.txt -o output.png --bit-depth 2

# With randomized pixel traversal (must use same flag to extract)
cloak embed -i cover.png -d secret.txt -o output.png --randomize
cloak extract -i output.png -o recovered.txt --randomize

# Combine both
cloak embed -i cover.png -d secret.txt -o output.png --bit-depth 4 --randomize
cloak extract -i output.png -o recovered.txt --bit-depth 4 --randomize

# Use parallel embedding for large images
cloak embed -i large.png -d secret.txt -o output.png --parallel
```

### Lossy and palette-based covers

JPEG, WebP, and GIF images can be used as covers. The stego output is always PNG (lossless) to preserve embedded data:

```bash
cloak embed -i photo.jpg -d secret.txt -o stego.png
cloak embed -i image.webp -d secret.txt -o stego.png
cloak embed -i animation.gif -d secret.txt -o stego.png

# Extract from the PNG output
cloak extract -i stego.png -o recovered.txt
```

TIFF is lossless and supports direct roundtrip:

```bash
cloak embed -i scan.tiff -d secret.txt -o stego.tiff
cloak extract -i stego.tiff -o recovered.txt
```

### Stdin/stdout piping

Use `-` as a path for stdin or stdout:

```bash
# Pipe cover from stdin, stego to stdout
cat cover.png | cloak embed -i - -d secret.txt -o - > stego.png

# Pipe secret data from stdin
echo "secret" | cloak embed -i cover.png -d - -o stego.png

# Pipe extracted data to stdout
cloak extract -i stego.png -o - | less
```

### Batch processing

```bash
# Embed matching files from two directories
cloak batch-embed --input-dir covers/ --data-dir secrets/ --output-dir stego/

# Extract all stego images in a directory
cloak batch-extract --input-dir stego/ --output-dir recovered/
```

### Analysis

```bash
# Analyze a single image
cloak analyze -i suspicious.png

# Analyze multiple images with a glob pattern
cloak analyze -i "photos/*.png"

# Check embedding capacity (shows detailed breakdown)
cloak capacity -i cover.png
cloak capacity -i cover.png --bit-depth 2

# Glob pattern for capacity
cloak capacity -i "covers/*.png"

# Launch interactive TUI
cloak inspect -i image.png
```

## Building

```bash
cargo build --workspace
cargo test --workspace
```

## Benchmarks

```bash
make bench
# or
cargo bench --package cloak-core
```

Criterion benchmarks cover embed/extract at multiple image sizes (64x64 to 512x512), all bit depths (1-4), randomized mode, parallel embedding, and full end-to-end encrypt+embed/extract+decrypt cycles.

## Fuzzing

Fuzz targets require nightly Rust:

```bash
cargo +nightly fuzz run fuzz_extract -- -max_total_time=60
cargo +nightly fuzz run fuzz_decrypt -- -max_total_time=60
cargo +nightly fuzz run fuzz_analyze -- -max_total_time=60
```

Six fuzz targets cover: extract, embed, analyze, decrypt, format detection, and wire format parsing.

## Architecture

```
crates/
  cloak-core/    Core library: encryption, LSB engine, format codecs, steganalysis
  cloak-cli/     CLI binary (cloak)
  cloak-tui/     TUI analysis dashboard
fuzz/            Cargo-fuzz targets
```

`cloak-core` is the reusable library. All steganography operations go through the public API: `embed()`, `extract()`, `capacity()`, and `capacity_report()`. Internally, a unified `LsbCodec` handles all six image formats with a single implementation. GIF-specific steganalysis is available via `analysis::analyze_gif()`.

## Security Model

- **Encryption is mandatory** — There is no way to embed unencrypted data. Every payload is wrapped with ChaCha20-Poly1305 authenticated encryption before embedding.
- **Key derivation** — Argon2id derives the encryption key, the pixel permutation seed, and the length header mask from the passphrase, each with distinct salts.
- **Authenticated encryption** — Poly1305 authentication tags detect tampering or wrong passphrases.
- **Versioned wire format** — Version byte allows forward-compatible format evolution; unknown versions are rejected with a clear error.
- **Anti-steganalysis** — Randomized pixel traversal, encrypted length headers, and block-aligned padding reduce statistical detectability.

## License

MIT
