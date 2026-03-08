# cloak

A modern steganography toolkit written in Rust. Embed encrypted data within images using LSB steganography with mandatory ChaCha20-Poly1305 encryption.

## Features

- **Mandatory encryption** — All embedded data is encrypted with ChaCha20-Poly1305 (Argon2id KDF)
- **Multiple formats** — PNG, BMP support (JPEG/WebP planned)
- **Steganalysis** — Chi-square testing, histogram analysis, bit-plane extraction
- **TUI dashboard** — Interactive terminal-based image analysis
- **CLI** — Simple command-line interface for embedding and extracting

## Usage

```bash
# Embed secret data into an image
cloak embed -i cover.png -d secret.txt -o output.png

# Extract hidden data
cloak extract -i output.png -o recovered.txt

# Analyze an image for steganographic content
cloak analyze -i suspicious.png

# Check embedding capacity
cloak capacity -i cover.png

# Launch interactive TUI
cloak inspect -i image.png
```

## Building

```bash
cargo build --workspace
cargo test --workspace
```

## License

MIT
