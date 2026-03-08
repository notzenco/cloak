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
4. Commit with a clear message
5. Open a pull request against `main`

## Project Structure

- `crates/cloak-core` — Core library (stego algorithms, encryption, format handling)
- `crates/cloak-cli` — CLI binary
- `crates/cloak-tui` — TUI analysis dashboard

## Code Style

- Follow `cargo fmt` and `cargo clippy` conventions
- Use `thiserror` for errors in `cloak-core`, `anyhow` in binaries
- Add tests for new functionality
